use std::{env, error::Error};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

use super::Backend;

#[derive(Debug)]
pub struct WaylandBackend;

impl Backend for WaylandBackend {
    fn new(_logger: Logger, _handle: LoopHandle<'_, State>, _display: &mut Display) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        Ok(WaylandBackend)
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        env::var("WAYLAND_DISPLAY").is_ok()
    }

    fn name(&self) -> &str {
        "wayland"
    }

    fn logger(&self) -> &Logger {
        todo!()
    }

    fn setup_outputs(&mut self, _display: &mut Display) {
        todo!()
    }
}
