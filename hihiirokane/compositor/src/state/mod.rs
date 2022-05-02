mod buffer;
mod compositor;
mod dmabuf;
mod output;
mod seat;
mod shm;

use smithay::{
    reexports::wayland_server::Display,
    wayland::{
        compositor::CompositorState, dmabuf::DmabufState, output::OutputManagerState, seat::SeatState, shm::ShmState,
    },
};

use crate::backend::{Backend, Headless};

/// The compositor state
///
/// The [`State`] manages all compositor systems, the shell and the backend.
#[derive(Debug)]
pub struct State {
    pub protocols: Protocols,
    pub shell: Shell,

    pub running: bool,

    /// The backend for the compositor.
    ///
    /// By default this is set to [`Headless`].
    pub backend: Box<dyn Backend>,
}

impl State {
    pub fn new(display: &mut Display<State>) -> State {
        State {
            protocols: Protocols::new(display),
            shell: Shell {},
            running: true,
            backend: Box::new(Headless),
        }
    }

    /// Returns true if the backend is [`Headless`].
    pub fn is_headless(&self) -> bool {
        self.backend.downcast_ref::<Headless>().is_some()
    }
}

/// Delegate types for protocol implementations.
#[derive(Debug)]
pub struct Protocols {
    pub compositor: CompositorState,
    pub seat: SeatState<State>,
    pub output_manager: OutputManagerState,
    // FIXME: Integrate shm into the backend?
    pub shm: ShmState,
    pub dmabuf: DmabufState,
}

impl Protocols {
    pub fn new(display: &mut Display<State>) -> Protocols {
        Protocols {
            compositor: CompositorState::new(display, None),
            seat: SeatState::new(),
            // TODO: With xdg output
            output_manager: OutputManagerState::new(),
            // TODO: More shm formats from renderer
            shm: ShmState::new(display, Vec::new(), None),
            dmabuf: DmabufState::new(),
        }
    }
}

/// Data associated with Wayland shell implementations.
#[derive(Debug)]
pub struct Shell {}
