use std::{os::fd::AsRawFd, sync::Arc};

use backend::Backend;
use calloop::{generic::Generic, Interest, LoopHandle, LoopSignal, Mode, PostAction};
use clap::Parser;
use cli::AerugoArgs;
use shell::Graph;
use smithay::{
    input::SeatState,
    reexports::calloop::EventLoop,
    wayland::{compositor::CompositorState, shell::xdg::XdgShellState, socket::ListeningSocketSource},
};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use wayland_server::{Display, DisplayHandle};

mod backend;
mod buffer;
mod cli;
mod compositor;
mod seat;
mod shell;

#[derive(Debug)]
pub struct Aerugo {
    r#loop: LoopHandle<'static, Self>,
    signal: LoopSignal,
    display: Display<AerugoCompositor>,
    comp: AerugoCompositor,
}

impl Aerugo {
    pub fn new(r#loop: &EventLoop<'static, Self>, args: &AerugoArgs) -> Result<Self, ()> {
        let mut display = Display::new().expect("Failed to initialize Wayland display");

        let signal = r#loop.get_signal();
        let r#loop = r#loop.handle();

        // Register the display to the event loop to allow client requests to be processed.
        register_display_source(&mut display, &r#loop);

        // Register the listening socket so clients can connect
        register_listening_socket(&r#loop);

        let comp = AerugoCompositor::new(&r#loop, display.handle(), args);

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
    let listening_socket = ListeningSocketSource::new_auto(None).expect("Failed to bind a socket");

    let socket = listening_socket.socket_name().to_owned();
    tracing::info!("Bound Wayland socket: {:?}", socket);

    r#loop
        .insert_source(listening_socket, |client, _, state| {
            // TODO: Graceful error handling
            state
                .display
                .handle()
                .insert_client(client, Arc::new(()))
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
    shell: Graph,
    backend: Box<dyn Backend>,
}

impl AerugoCompositor {
    fn new(r#loop: &LoopHandle<'static, Aerugo>, display: DisplayHandle, args: &AerugoArgs) -> Self {
        let backend = backend::create_backend(r#loop, &display, args).expect("Failed to initialize backend");

        // Initialize common globals
        let seat_state = SeatState::new();
        let wl_compositor = CompositorState::new::<Self, _>(&display, None);
        let xdg_shell = XdgShellState::new::<Self, _>(&display, None);

        Self {
            display,
            wl_compositor,
            xdg_shell,
            seat_state,
            shell: Graph::default(),
            backend,
        }
    }
}

fn main() {
    let args = cli::AerugoArgs::parse();

    let subscriber = FmtSubscriber::builder().with_max_level(Level::TRACE).finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut r#loop = EventLoop::try_new_high_precision()
        .or_else(|_| {
            tracing::warn!("Failed to initialize high precision event loop, falling back to regular event loop");
            EventLoop::try_new()
        })
        .expect("Failed to create event loop");

    let mut state = Aerugo::new(&r#loop, &args).unwrap();
    r#loop
        .run(None, &mut state, |state| {
            state.check_shutdown();

            // Flush the display at the end of the idle callback to allow clients to process server events.
            state.flush_display();
        })
        .expect("Error while dispatching event loop");
}
