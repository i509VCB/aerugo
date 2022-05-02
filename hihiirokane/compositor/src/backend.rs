//! Abstractions over input and output backends.

use std::{error::Error, fmt};

use downcast_rs::{impl_downcast, Downcast};
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    reexports::wayland_server::Display,
    wayland::dmabuf::{DmabufGlobal, ImportError},
};

use crate::{output::Output, state::State};

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("\"{0}\" is not supported")]
    NotSupported(&'static str),

    #[error(transparent)]
    Backend(Box<dyn Error>),
}

pub trait Backend: Downcast + fmt::Debug {
    /// Creates a new output and returns a handle to the newly created output.
    fn create_output(&mut self, _display: &mut Display<State>) -> Result<Output, BackendError> {
        Err(BackendError::NotSupported("create_output"))
    }

    /// Returns true if the backend handles the specified [`DmabufGlobal`].
    ///
    /// This must be checked before importing a [`Dmabuf`].
    fn can_handle_dmabuf_global(&self, _global: &DmabufGlobal) -> bool {
        false
    }

    /// Imports the [`Dmabuf`] into the backend.
    // TODO: How do we return a texture if applicable?
    fn dmabuf_import(&mut self, _dmabuf: Dmabuf) -> Result<(), ImportError> {
        // Each backend must instantiate it's own "DmabufGlobal" to advertise globals to a client.
        //
        // The default implementation is fine to return "Failed" because the implementation could not have
        // advertised a global in the first place.
        Err(ImportError::Failed)
    }
}

impl_downcast!(Backend);

/// Handle to a backend output type
pub trait BackendOutput: fmt::Debug {}

/// A headless backend.
///
/// The headless backend has no outputs and provides no inputs.
#[derive(Debug)]
pub struct Headless;

impl Backend for Headless {}

#[cfg(test)]
mod tests {
    use super::{Backend, Headless};

    /// Backends must be object safe since we use dynamic dispatch.
    #[test]
    fn object_safety() {
        let _: Box<dyn Backend> = Box::new(Headless);
    }
}
