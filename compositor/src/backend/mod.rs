pub mod winit;

use std::{error::Error, fmt};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

#[derive(Debug)]
pub struct InvalidBackend(String);

impl fmt::Display for InvalidBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid backend: {}", self.0)
    }
}

impl Error for InvalidBackend {}

pub trait Backend: fmt::Debug {
    fn setup_backend(&mut self, handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>>;

    fn setup_globals(
        &mut self,
        display: &mut Display,
        logger: Logger,
    ) -> Result<(), Box<dyn Error>>;

    fn name(&self) -> &str;
}
