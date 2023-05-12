mod x11;

use std::{error::Error, fmt};

use calloop::LoopHandle;
use downcast_rs::{impl_downcast, Downcast};
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    wayland::{
        dmabuf::{DmabufGlobal, DmabufState, ImportError},
        shm::ShmState,
    },
};
use wayland_server::DisplayHandle;

use crate::Loop;

pub trait Backend: fmt::Debug + Downcast {
    fn shm_state(&self) -> &ShmState;

    /// Return the delegate type for the dmabuf protocol state.
    ///
    /// This is managed by the backend since not every backend might support the dmabuf protocol.
    fn dmabuf_state(&mut self) -> &mut DmabufState;

    /// Import a client's dmabuf buffer into the backend.
    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, _dmabuf: Dmabuf) -> Result<(), ImportError>;

    /// Check if the backend is asking the compositor to shutdown.
    ///
    /// Outside of the windowed test backends, this should return [`false`]
    fn should_shutdown(&self) -> bool {
        false
    }

    // TODO: Outputs?
    // TODO: Seat?
}
impl_downcast!(Backend);

pub fn default_backend(
    r#loop: LoopHandle<'static, Loop>,
    display: DisplayHandle,
) -> Result<Box<dyn Backend>, Box<dyn Error>> {
    // TODO: X11 backend only exists right now, so the backend selection is ignored.
    Ok(Box::new(x11::Backend::new(r#loop, display).expect("TODO: Error type")))
}

#[cfg(test)]
mod tests {
    use crate::backend::Backend;

    /// Test that [`Backend`] is object safe.
    #[test]
    #[should_panic(expected = "Should panic if Backend is object safe, or compilation will fail")]
    fn dynamic_dispatch() {
        let _: Box<dyn Backend> = panic!("Should panic if Backend is object safe, or compilation will fail");
    }
}
