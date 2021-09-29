#[cfg(feature = "udev_backend")]
pub mod udev;

#[cfg(feature = "x11_backend")]
pub mod x11;

#[cfg(feature = "wayland_backend")]
pub mod wayland;

use std::{error::Error, fmt};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

pub trait Backend: fmt::Debug {
    fn run(logger: Logger) -> Result<(), Box<dyn Error>>
    where
        Self: Sized;

    fn available() -> bool
    where
        Self: Sized;

    fn setup_backend(&mut self, handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>>;

    fn setup_globals(
        &mut self,
        display: &mut Display,
        logger: Logger,
    ) -> Result<(), Box<dyn Error>>;

    fn name(&self) -> &str;
}
