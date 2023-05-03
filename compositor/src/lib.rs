use std::{
    error::Error,
    fmt, io,
    os::fd::{AsRawFd, OwnedFd},
    sync::{
        mpsc::{self, SendError},
        Arc,
    },
    thread::{self, JoinHandle, Thread},
};

use bitflags::bitflags;
use calloop::{channel::SyncSender, generic::Generic, EventLoop, Interest, LoopHandle, LoopSignal, Mode, PostAction};

use backend::Backend;
use scene::Scene;
use shell::Shell;
use smithay::{
    input::SeatState,
    output::{Output, PhysicalProperties},
    wayland::{compositor::CompositorState, shell::xdg::XdgShellState, socket::ListeningSocketSource},
};
use wayland_server::{
    backend::{ClientId, DisconnectReason},
    Client, Display, DisplayHandle,
};

pub mod backend;
pub mod forest;
mod scene;
mod shell;
mod wayland;

type BackendConstructor = Box<
    dyn FnOnce(LoopHandle<'static, Aerugo>, DisplayHandle) -> Result<Box<dyn Backend>, Box<dyn Error>> + Send + 'static,
>;

/// Configuration used to create a server instance.
pub struct Configuration {
    backend_constructor: BackendConstructor,
}

impl Configuration {
    pub fn new<B>(b: B) -> Self
    where
        B: FnOnce(LoopHandle<'static, Aerugo>, DisplayHandle) -> Result<Box<dyn Backend>, Box<dyn Error>>
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
            let mut r#loop = EventLoop::try_new_high_precision()
                .or_else(|err| {
                    tracing::warn!(%err, "Failed to create high precision event loop, falling back.");
                    EventLoop::try_new()
                })
                .expect("Failed to create event loop");

            let signal = r#loop.get_signal();
            let (send_server, recv_server) = calloop::channel::sync_channel::<ExecutorMessage>(5);
            send.send((signal, send_server)).expect("Executor thread died");

            let mut aerugo = Aerugo::new(&r#loop, self.backend_constructor).expect("TODO: Error type");

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
pub struct Aerugo {
    r#loop: LoopHandle<'static, Self>,
    signal: LoopSignal,
    display: Display<AerugoCompositor>,
    comp: AerugoCompositor,
}

impl Aerugo {
    pub fn new(r#loop: &EventLoop<'static, Self>, backend: BackendConstructor) -> Result<Self, ()> {
        let mut display = Display::new().expect("Failed to initialize Wayland display");

        let signal = r#loop.get_signal();
        let r#loop = r#loop.handle();

        // Register the display to the event loop to allow client requests to be processed.
        register_display_source(&mut display, &r#loop);

        // Register the listening socket so clients can connect
        register_listening_socket(&r#loop);

        let backend = backend(r#loop.clone(), display.handle()).expect("TODO: Error type");

        let comp = AerugoCompositor::new(&r#loop, display.handle(), backend);

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

fn register_display_source(display: &mut Display<AerugoCompositor>, r#loop: &LoopHandle<'static, Aerugo>) {
    let poll_fd = display.backend().poll_fd().as_raw_fd();

    r#loop
        .insert_source(Generic::new(poll_fd, Interest::READ, Mode::Level), |_, _, state| {
            state.display.dispatch_clients(&mut state.comp).unwrap();
            Ok(PostAction::Continue)
        })
        .unwrap();
}

fn register_listening_socket(r#loop: &LoopHandle<'static, Aerugo>) {
    let listening_socket = ListeningSocketSource::new_auto().expect("Failed to bind a socket");

    let socket = listening_socket.socket_name().to_owned();
    tracing::info!("Bound Wayland socket: {:?}", socket);

    r#loop
        .insert_source(listening_socket, |client, _, state| {
            // TODO: Graceful error handling
            state
                .display
                .handle()
                .insert_client(
                    client,
                    Arc::new(ClientData {
                        // TODO: Limit the available globals
                        globals: PrivilegedGlobals::all(),
                    }),
                )
                .expect("Failed to register client");
        })
        .unwrap();
}

#[derive(Debug)]
pub struct AerugoCompositor {
    display: DisplayHandle,
    wl_compositor: CompositorState,
    xdg_shell: XdgShellState,
    seat_state: SeatState<Self>,
    shell: Shell,
    scene: Scene,
    // This is not what I want in the future, but is for testing.
    output: Output,
    backend: Box<dyn Backend>,
}

impl AerugoCompositor {
    fn new(_loop: &LoopHandle<'static, Aerugo>, display: DisplayHandle, backend: Box<dyn Backend>) -> Self {
        // Initialize common globals
        let seat_state = SeatState::new();
        let wl_compositor = CompositorState::new::<Self>(&display);
        let xdg_shell = XdgShellState::new::<Self>(&display);
        let output = Output::new(
            "Test output".into(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: smithay::output::Subpixel::Unknown,
                make: String::new(),
                model: String::new(),
            },
        );
        output.create_global::<Self>(&display);

        let mut scene = Scene::new();
        scene.create_output(output.clone());

        let shell = Shell::new();

        Self {
            display,
            wl_compositor,
            xdg_shell,
            seat_state,
            shell,
            scene,
            output,
            backend,
        }
    }
}

bitflags! {
    /// Bitflag to describe what globals are visible to clients.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct PrivilegedGlobals: u32 {
        /// Whether the `ext-foreign-toplevel-list-v1` global is available.
        const FOREIGN_TOPLEVEL_LIST = 0x01;

        /// Whether the `ext-foreign-toplevel-state-v1` global is available.
        ///
        /// This protocol is always enabled with the `ext-foreign-toplevel-list-v1` protocol.
        ///
        /// This is not enabled at the moment since the protocol is not yet done: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/196
        const FOREIGN_TOPLEVEL_STATE = 0x03;

        /// Whether the foreign toplevel management global is available.
        ///
        /// This protocol is always enabled with the `ext-foreign-toplevel-state-v1` protocol.
        const FOREIGN_TOPLEVEL_MANAGEMENT = 0x07;

        /// Whether the client is XWayland.
        ///
        /// This will enable the `xwayland-shell-v1` and `zwp_xwayland-keyboard-grab-v1` protocols.
        const XWAYLAND = 0x08;

        /// Whether the `ext-session-lock-v1` global is available.
        const SESSION_LOCK = 0x10;

        /// Whether the `zwlr-layer-shell-v1` protocol is available.
        ///
        /// This will also make the `ext-layer-shell-v1` protocol available when merged: https://gitlab.freedesktop.org/wayland/wayland-protocols/-/merge_requests/28
        const LAYER_SHELL = 0x20;

        /// Whether the `aerugo-shell-v1` protocol is available.
        const AERUGO_SHELL = 0x40;
    }
}

#[derive(Debug)]
pub struct ClientData {
    globals: PrivilegedGlobals,
}

impl ClientData {
    pub fn get_data(client: &Client) -> Option<&Self> {
        client.get_data()
    }

    pub fn is_visible(&self, global: PrivilegedGlobals) -> bool {
        self.globals.contains(global)
    }
}

impl wayland_server::backend::ClientData for ClientData {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}
