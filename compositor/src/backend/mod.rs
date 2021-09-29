#[cfg(feature = "udev_backend")]
pub mod udev;

#[cfg(feature = "x11_backend")]
pub mod x11;

#[cfg(feature = "wayland_backend")]
pub mod wayland;

use std::{error::Error, fmt};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::{Socket, State};

/// A trait specifying the implementation of a backend.
///
/// A backend may register calloop event sources and globals on the display in order to handle events.
///
/// Generally implementations of this trait will set up some way to handle device events and present what the
/// compositor has rendered.
///
/// ## Accessing data stored on a backend object
///
/// Data may be accessed in most places through [`State::backend`] or [`State::backend_mut`] inside of callbacks or
/// [`DispatchData`](smithay::reexports::wayland_server::DispatchData) and then downcast to the backend internal type.
pub trait Backend: fmt::Debug {
    /// Starts the compositor.
    ///
    /// In this function the backend should instantiate itself and invoke [`crate::run`] to start the compositor.
    fn run(logger: Logger, socket: Socket) -> Result<(), Box<dyn Error>>
    where
        Self: Sized;

    /// Returns true if the backend is available for use.
    ///
    /// If this function returns `false`, it is more than likely that creating the backend will fail.
    fn available() -> bool
    where
        Self: Sized;

    /// The backend should perform any required setup at this point.
    ///
    /// A backend may insert any event sources it needs into the event loop at this point using the `handle`.
    fn setup_backend(&mut self, handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>>;

    /// The backend should perform any setup needed on the Wayland Display at this point.
    ///
    /// A backend may instantiate any globals it needs at this point in order to receive requests from clients.
    fn setup_globals(&mut self, display: &mut Display) -> Result<(), Box<dyn Error>>;

    /// Returns the name of the backend.
    ///
    /// This should be a lowercase string.
    fn name(&self) -> &str;
}
