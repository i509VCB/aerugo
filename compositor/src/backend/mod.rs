mod x11;

use std::fmt;

use calloop::LoopHandle;
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    wayland::dmabuf::{DmabufGlobal, DmabufState, ImportError},
};
use wayland_server::DisplayHandle;

use crate::{cli::AerugoArgs, state::Aerugo};

pub trait Backend: fmt::Debug {
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

pub fn create_backend(
    r#loop: &LoopHandle<'static, Aerugo>,
    display: &DisplayHandle,
    args: &AerugoArgs,
) -> Result<Box<dyn Backend>, ()> {
    // TODO: X11 backend only exists right now, so the backend selection is ignored.
    Ok(Box::new(x11::Backend::new(r#loop, display, args)?))
}

#[cfg(test)]
mod tests {
    use crate::backend::Backend;

    /// Test that [`Backend`] is object safe.
    #[test]
    #[should_panic(expected = "Should panic if Backend is object safe, or compilation will fail")]
    fn dynamic_dispatch() {
        let _: Box<dyn Backend> = panic!("Intentional panic");
    }
}
