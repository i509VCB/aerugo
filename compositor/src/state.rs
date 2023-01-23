use std::{os::fd::AsRawFd, sync::Arc};

use calloop::{generic::Generic, EventLoop, Interest, LoopHandle, LoopSignal, Mode, PostAction};
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_compositor, delegate_dmabuf, delegate_shm, delegate_xdg_shell,
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorHandler, CompositorState},
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
        shell::xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};
use wayland_server::{
    protocol::{wl_buffer, wl_seat, wl_surface},
    Display, DisplayHandle,
};

use crate::{
    backend::{self, Backend},
    cli::AerugoArgs,
};

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
            backend,
        }
    }
}

impl BufferHandler for AerugoCompositor {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl CompositorHandler for AerugoCompositor {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.wl_compositor
    }

    fn commit(&mut self, _surface: &wl_surface::WlSurface) {}
}

delegate_compositor!(AerugoCompositor);

impl ShmHandler for AerugoCompositor {
    fn shm_state(&self) -> &ShmState {
        self.backend.shm_state()
    }
}

delegate_shm!(AerugoCompositor);

impl XdgShellHandler for AerugoCompositor {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell
    }

    fn new_toplevel(&mut self, _surface: ToplevelSurface) {}

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}
}

delegate_xdg_shell!(AerugoCompositor);

impl SeatHandler for AerugoCompositor {
    type KeyboardFocus = wl_surface::WlSurface;
    type PointerFocus = wl_surface::WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&Self::KeyboardFocus>) {}

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
}

impl DmabufHandler for AerugoCompositor {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        self.backend.dmabuf_state()
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf) -> Result<(), ImportError> {
        self.backend.dmabuf_imported(global, dmabuf)
    }
}

delegate_dmabuf!(AerugoCompositor);
