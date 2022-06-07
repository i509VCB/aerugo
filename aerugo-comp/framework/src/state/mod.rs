mod dmabuf;
mod shm;

use smithay::{
    delegate_compositor, delegate_seat,
    reexports::wayland_server::{
        protocol::{wl_buffer, wl_surface},
        DisplayHandle,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorHandler, CompositorState},
        dmabuf::DmabufState,
        output::OutputManagerState,
        seat::{SeatHandler, SeatState},
        shm::ShmState,
    },
};

/// The compositor state
///
/// The [`State`] manages all compositor systems, the shell and the backend.
#[derive(Debug)]
pub struct Aerugo {
    pub protocols: Protocols,
    pub shell: Shell,

    pub running: bool,
}

impl Aerugo {
    pub fn new(dh: &DisplayHandle) -> Aerugo {
        Aerugo {
            protocols: Protocols::new(dh),
            shell: Shell {},
            running: true,
        }
    }
}

/// Delegate types for protocol implementations.
#[derive(Debug)]
pub struct Protocols {
    pub compositor: CompositorState,
    pub seat: SeatState<Aerugo>,
    pub output_manager: OutputManagerState,
    // FIXME: Integrate shm into the backend?
    pub shm: ShmState,
    pub dmabuf: DmabufState,
}

impl Protocols {
    pub fn new(dh: &DisplayHandle) -> Protocols {
        Protocols {
            compositor: CompositorState::new::<Aerugo, _>(dh, None),
            seat: SeatState::new(),
            // TODO: With xdg output
            output_manager: OutputManagerState::new(),
            // TODO: More shm formats from renderer
            shm: ShmState::new::<Aerugo, _>(dh, Vec::new(), None),
            dmabuf: DmabufState::new(),
        }
    }
}

/// Data associated with Wayland shell implementations.
#[derive(Debug)]
pub struct Shell {}

// Handler implementations

impl BufferHandler for Aerugo {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {
        todo!()
    }
}

impl CompositorHandler for Aerugo {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.protocols.compositor
    }

    fn commit(&mut self, _dh: &DisplayHandle, _surface: &wl_surface::WlSurface) {
        todo!()
    }
}

delegate_compositor!(Aerugo);

impl SeatHandler for Aerugo {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        todo!()
    }
}

delegate_seat!(Aerugo);
