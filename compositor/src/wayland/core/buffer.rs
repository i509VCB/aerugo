use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    wayland::{
        buffer::BufferHandler,
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
        shm::{ShmHandler, ShmState},
    },
};
use wayland_server::protocol::wl_buffer;

use crate::AerugoCompositor;

impl BufferHandler for AerugoCompositor {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for AerugoCompositor {
    fn shm_state(&self) -> &ShmState {
        self.backend.shm_state()
    }
}

smithay::delegate_shm!(AerugoCompositor);

impl DmabufHandler for AerugoCompositor {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        self.backend.dmabuf_state()
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf) -> Result<(), ImportError> {
        self.backend.dmabuf_imported(global, dmabuf)
    }
}

smithay::delegate_dmabuf!(AerugoCompositor);
