//! X11 input and output backend

use calloop::LoopHandle;
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    wayland::dmabuf::{DmabufGlobal, DmabufState, ImportError},
};
use wayland_server::DisplayHandle;

use crate::{cli::AerugoArgs, state::Aerugo};

#[derive(Debug)]
pub struct Backend {
    r#loop: LoopHandle<'static, Aerugo>,
    display: DisplayHandle,
}

impl Backend {
    // TODO: Error type
    pub fn new(r#loop: &LoopHandle<'static, Aerugo>, display: &DisplayHandle, _args: &AerugoArgs) -> Result<Self, ()> {
        Ok(Self {
            r#loop: r#loop.clone(),
            display: display.clone(),
        })
    }
}

impl crate::backend::Backend for Backend {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        todo!("X11 does not initialize the dmabuf global yet")
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, _dmabuf: Dmabuf) -> Result<(), ImportError> {
        todo!("X11 does not initialize the dmabuf global yet")
    }
}
