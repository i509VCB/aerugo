use std::{env, error::Error};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

use super::Backend;

#[derive(Debug)]
pub struct WaylandBackend;

impl Backend for WaylandBackend {
    fn new(_logger: Logger) -> Self
    where
        Self: Sized,
    {
        WaylandBackend
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        env::var("WAYLAND_DISPLAY").is_ok()
    }

    fn setup_backend(&mut self, _handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>> {
        todo!("Wayland backend not implemented yet")
    }

    fn setup_globals(&mut self, _display: &mut Display) -> Result<(), Box<dyn Error>> {
        todo!("Wayland backend not implemented yet")
    }

    fn name(&self) -> &str {
        "wayland"
    }
}
