mod buffer;
mod compositor;
mod dmabuf;
mod output;
mod seat;
mod shm;

use smithay::{
    reexports::wayland_server::Display,
    wayland::{compositor::CompositorState, output::OutputManagerState, seat::SeatState, shm::ShmState},
};

#[derive(Debug)]
pub struct Hihiirokane {
    pub protocols: Protocols,
    pub shell: ShellData,
}

/// Delegate types for protocol implementations.
#[derive(Debug)]
pub struct Protocols {
    pub compositor: CompositorState,
    pub shm: ShmState,
    pub seat: SeatState<Hihiirokane>,
    pub output_manager: OutputManagerState,
}

impl Protocols {
    pub fn new(display: &mut Display<Hihiirokane>) -> Protocols {
        Protocols {
            compositor: CompositorState::new(display, None),
            // TODO: More shm formats from renderer
            shm: ShmState::new(display, Vec::new(), None),
            seat: SeatState::new(),
            // TODO: With xdg output
            output_manager: OutputManagerState::new(),
        }
    }
}

/// Data associated with Wayland shell implementations.
#[derive(Debug)]
pub struct ShellData {}
