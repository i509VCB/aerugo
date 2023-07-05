use std::{
    error::Error,
    io,
    os::fd::{AsRawFd, OwnedFd},
    sync::{
        mpsc::{self, SendError},
        Arc,
    },
    thread::{self, JoinHandle, Thread},
};

use calloop::{channel::SyncSender, generic::Generic, EventLoop, Interest, LoopHandle, LoopSignal, Mode, PostAction};

use backend::Backend;
use smithay::wayland::{compositor::CompositorClientState, socket::ListeningSocketSource};
use wayland_server::{Display, DisplayHandle};

pub mod backend;
pub mod forest;
mod scene;
mod shell;
mod state;
mod wayland;

pub use state::Aerugo;

use crate::state::{ClientData, PrivilegedGlobals};

type BackendConstructor = Box<
    dyn FnOnce(LoopHandle<'static, Loop>, DisplayHandle) -> Result<Box<dyn Backend>, Box<dyn Error>> + Send + 'static,
>;

/// Configuration used to create a server instance.
pub struct Configuration {
    backend_constructor: BackendConstructor,
}

impl Configuration {
    pub fn new<B>(b: B) -> Self
    where
        B: FnOnce(LoopHandle<'static, Loop>, DisplayHandle) -> Result<Box<dyn Backend>, Box<dyn Error>>
            + Send
            + 'static,
    {
        Self {
            backend_constructor: Box::new(b),
        }
    }

    // TODO: Socket creation here

    /// Creates a server using the configuration.
    ///
    /// This will start the server event loop and return a handle that may be used to stop the server and check
    /// on the
    pub fn create_server(self) -> io::Result<AerugoExecutor> {
        // TODO: io Error is wrong error type

        // In calloop EventLoop is !Send and !Sync, so we need to send the loop signal from the event loop
        // thread to the caller. We do this with a rendezvous channel (which is why the bound is 0).
        let (send, recv) = mpsc::sync_channel(0);

        let thread = thread::Builder::new().name("Aerugo event loop".into()).spawn(move || {
            // TODO: Proper typedef
            let mut r#loop = EventLoop::try_new().expect("Failed to create event loop");

            let signal = r#loop.get_signal();
            let (send_server, recv_server) = calloop::channel::sync_channel::<ExecutorMessage>(5);
            send.send((signal, send_server)).expect("Executor thread died");

            let mut aerugo = Loop::new(&r#loop, self.backend_constructor).expect("TODO: Error type");

            {
                let r#loop = r#loop.handle();
                r#loop
                    .insert_source(recv_server, |msg, _, _state| {
                        if let calloop::channel::Event::Msg(_msg) = msg {
                            todo!("Handle executor messages")
                        }
                    })
                    .unwrap();
            }

            r#loop
                .run(None, &mut aerugo, |state| {
                    // Flush any pending messages to ensure clients can respond to server events.
                    state.flush_display();
                    // Check the backend has met any internal shutdown conditions.
                    state.check_shutdown();
                })
                .unwrap();

            tracing::info!("Server shutting down");
        })?;

        // Get the signal from the oneshot channel so the executor can stop the server.
        //
        // There is no need to use try_recv since the event loop is either successfully created or the
        // thread fails to initialize the event loop
        let (signal, channel) = recv.recv().expect("TODO: Add error variant");

        Ok(AerugoExecutor {
            thread,
            signal,
            channel,
        })
    }
}

/// A handle to an instance of the display server.
///
/// Messages may be sent to the display server's event loop using this type.
pub struct AerugoExecutor {
    thread: JoinHandle<()>,
    signal: LoopSignal,
    channel: SyncSender<ExecutorMessage>,
}

impl AerugoExecutor {
    /// The thread which the event loop is running on.
    pub fn thread(&self) -> &Thread {
        self.thread.thread()
    }

    /// Creates a client using the specified file descriptor for the client socket.
    ///
    /// This function is primarily intended for allowing wlcs to create clients for testing.
    pub fn create_client(&self, fd: OwnedFd) -> Result<(), SendError<OwnedFd>> {
        self.channel
            .send(ExecutorMessage::CreateClient(fd))
            .map_err(|msg| match msg.0 {
                ExecutorMessage::CreateClient(fd) => SendError(fd),
            })
    }

    /// Stops the server event loop.
    pub fn stop(&self) {
        // Stopping the server is twofold, first we send the event loop to stop and then immediately wake the
        // event loop to immediately ask the event loop to shut down.
        self.signal.stop();
        self.signal.wakeup();
    }

    /// Wait for the server event loop to stop.
    pub fn join(self) -> thread::Result<()> {
        self.thread.join()
    }
}

enum ExecutorMessage {
    CreateClient(OwnedFd),
}

#[derive(Debug)]
pub struct Loop {
    r#loop: LoopHandle<'static, Self>,
    signal: LoopSignal,
    display: Display<Aerugo>,
    comp: Aerugo,
}

impl Loop {
    pub fn new(r#loop: &EventLoop<'static, Self>, backend: BackendConstructor) -> Result<Self, ()> {
        let mut display = Display::new().expect("Failed to initialize Wayland display");

        let signal = r#loop.get_signal();
        let r#loop = r#loop.handle();

        // Register the display to the event loop to allow client requests to be processed.
        register_display_source(&mut display, &r#loop);

        // Register the listening socket so clients can connect
        register_listening_socket(&r#loop);

        let backend = backend(r#loop.clone(), display.handle()).expect("TODO: Error type");

        let comp = Aerugo::new(&r#loop, display.handle(), backend);

        Ok(Self {
            r#loop,
            signal,
            display,
            comp,
        })
    }

    pub fn flush_display(&mut self) {
        self.display.flush_clients().expect("TODO: Error?");
    }

    pub fn check_shutdown(&mut self) {
        let shutdown =
            // Check if the backend has requested a shutdown
            self.comp.backend.should_shutdown();

        // TODO: Other shutdown check mechanisms, such as a logout.

        if shutdown {
            // Signal the event loop to stop
            self.signal.stop();
            // In order to terminate the event loop quickly after stopping it, we need to wake the event loop.
            self.signal.wakeup();
        }
    }
}

fn register_display_source(display: &mut Display<Aerugo>, r#loop: &LoopHandle<'static, Loop>) {
    let poll_fd = display.backend().poll_fd().as_raw_fd();

    r#loop
        .insert_source(Generic::new(poll_fd, Interest::READ, Mode::Level), |_, _, state| {
            state.display.dispatch_clients(&mut state.comp).unwrap();
            Ok(PostAction::Continue)
        })
        .unwrap();
}

fn register_listening_socket(r#loop: &LoopHandle<'static, Loop>) {
    let listening_socket = ListeningSocketSource::new_auto().expect("Failed to bind a socket");

    let socket = listening_socket.socket_name().to_owned();
    tracing::info!("Bound Wayland socket: {:?}", socket);

    r#loop
        .insert_source(listening_socket, |client, _, state| {
            let info = format!("{client:?}");

            // TODO: Graceful error handling
            if let Err(err) = state.display.handle().insert_client(
                client,
                Arc::new(ClientData {
                    // TODO: Limit the available globals
                    globals: PrivilegedGlobals::all(),
                    compositor: CompositorClientState::default(),
                }),
            ) {
                // TODO: Provide info about the socket (name)
                tracing::error!(%err, "Failed to register client with fd: {info}");
            }
        })
        .unwrap();
}
