use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_dmabuf,
    wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
};

use super::Hihiirokane;

impl DmabufHandler for Hihiirokane {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        todo!()
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, _dmabuf: Dmabuf) -> Result<(), ImportError> {
        todo!()
    }
}

delegate_dmabuf!(Hihiirokane);
