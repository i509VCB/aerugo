#[cfg(feature = "udev_backend")]
pub mod udev;

#[cfg(feature = "x11_backend")]
pub mod x11;

#[cfg(feature = "wayland_backend")]
pub mod wayland;

use std::{error::Error, fmt};

use downcast_rs::{impl_downcast, Downcast};
use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::NameMe;

/// A trait specifying the implementation of a backend.
///
/// A backend may register calloop event sources and globals on the display in order to handle events.
///
/// Generally implementations of this trait will set up some way to handle device events and present what the
/// compositor has rendered.
pub trait Backend: fmt::Debug + Downcast {
    fn new(_logger: Logger, _handle: LoopHandle<'_, NameMe>, _display: &mut Display) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    /// Returns true if the backend is available for use.
    ///
    /// If this function returns `false`, it is more than likely that creating the backend will fail.
    fn available() -> bool
    where
        Self: Sized;

    /// Returns the name of the backend.
    ///
    /// This should be a lowercase string.
    fn name(&self) -> &str;

    /// Returns the logger for this backend.
    ///
    /// This logger may be used to log under the name of the module inside of a callback.
    fn logger(&self) -> &Logger;

    /// Perform initial output setup.
    fn setup_outputs(&mut self, _display: &mut Display) -> Result<(), Box<dyn Error>>;

    fn create_new_output(&mut self);
}

impl_downcast!(Backend);
