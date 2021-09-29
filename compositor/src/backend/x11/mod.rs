use std::{env, error::Error};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

use super::Backend;

#[derive(Debug)]
pub struct X11Backend;

impl Backend for X11Backend {
    fn run(_logger: Logger) -> Result<(), Box<dyn Error>>
    where
        Self: Sized,
    {
        todo!()
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        env::var("DISPLAY").is_ok()
    }

    fn setup_backend(&mut self, _handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>> {
        todo!("X11 backend not implemented yet")
    }

    fn setup_globals(
        &mut self,
        _display: &mut Display,
        _logger: Logger,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn name(&self) -> &str {
        "x11"
    }
}
