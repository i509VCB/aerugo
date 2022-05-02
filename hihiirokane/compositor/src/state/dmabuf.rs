use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_dmabuf,
    wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
};

use super::State;

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.protocols.dmabuf
    }

    fn dmabuf_imported(&mut self, global: &DmabufGlobal, dmabuf: Dmabuf) -> Result<(), ImportError> {
        if self.backend.can_handle_dmabuf_global(global) {
            self.backend.dmabuf_import(dmabuf)
        } else {
            Err(ImportError::Failed)
        }
    }
}

delegate_dmabuf!(State);
