//! X11 input and output backend

use calloop::LoopHandle;
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    wayland::{
        dmabuf::{DmabufGlobal, DmabufState, ImportError},
        shm::ShmState,
    },
};
use wayland_server::DisplayHandle;

use crate::{
    cli::AerugoArgs,
    state::{Aerugo, AerugoCompositor},
};

#[derive(Debug)]
pub struct Backend {
    r#loop: LoopHandle<'static, Aerugo>,
    display: DisplayHandle,
    shm_state: ShmState,
}

impl Backend {
    // TODO: Error type
    pub fn new(r#loop: &LoopHandle<'static, Aerugo>, display: &DisplayHandle, _args: &AerugoArgs) -> Result<Self, ()> {
        Ok(Self {
            r#loop: r#loop.clone(),
            display: display.clone(),
            // TODO: Renderer shm formats
            shm_state: ShmState::new::<AerugoCompositor, _>(display, Vec::with_capacity(2), None),
        })
    }
}

impl crate::backend::Backend for Backend {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }

    fn dmabuf_state(&mut self) -> &mut DmabufState {
        todo!("X11 does not initialize the dmabuf global yet")
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, _dmabuf: Dmabuf) -> Result<(), ImportError> {
        todo!("X11 does not initialize the dmabuf global yet")
    }
}
