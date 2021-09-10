pub mod winit;

use std::{error::Error, fmt};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

pub trait Backend: fmt::Debug {
    fn setup_backend(&mut self, handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>>;

    fn setup_globals(
        &mut self,
        display: &mut Display,
        logger: Logger,
    ) -> Result<(), Box<dyn Error>>;

    fn name(&self) -> &str;
}
