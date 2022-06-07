use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_dmabuf,
    reexports::wayland_server::DisplayHandle,
    wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
};

use super::Aerugo;

impl DmabufHandler for Aerugo {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.protocols.dmabuf
    }

    fn dmabuf_imported(
        &mut self,
        _dh: &DisplayHandle,
        _global: &DmabufGlobal,
        _dmabuf: Dmabuf,
    ) -> Result<(), ImportError> {
        todo!()
    }
}

delegate_dmabuf!(Aerugo);
