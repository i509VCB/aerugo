use smithay::{
    reexports::wayland_server::{protocol::wl_surface, Display, DisplayHandle},
    wayland::{
        compositor::{CompositorHandler, CompositorState},
        data_device::DataDeviceState,
        output::OutputManagerState,
        seat::{Seat, SeatState},
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{decoration::XdgDecorationState, XdgShellState},
        },
        shm::{ShmHandler, ShmState},
    },
};

mod backend;
mod client;
mod seat;
mod shell;

pub struct Aerugo {
    state: State,
    display: Display<State>,
}

impl Aerugo {
    pub fn new() -> Self {
        todo!()
    }
}

/// The compositor state.
#[derive(Debug)]
pub struct State {
    display: DisplayHandle,
    protocols: Protocols,
    seat: Seat<Self>,
    // TODO: Space
    // TODO: XWayland
}

/// Protocol state objects.
#[derive(Debug)]
pub struct Protocols {
    compositor: CompositorState,
    seat: SeatState<State>,
    data_device: DataDeviceState,
    output: OutputManagerState,
    xdg_shell: XdgShellState,
    xdg_decor: XdgDecorationState,
    layer_shell: WlrLayerShellState,
    shm: ShmState,
    // TODO: Dmabuf?
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.protocols.compositor
    }

    fn commit(&mut self, _dh: &DisplayHandle, _surface: &wl_surface::WlSurface) {
        todo!()
    }
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.protocols.shm
    }
}
