use std::{env, error::Error};

use slog::Logger;
use smithay::reexports::{calloop::LoopHandle, wayland_server::Display};

use crate::state::State;

use super::Backend;

#[derive(Debug)]
pub struct WaylandBackend;

impl Backend for WaylandBackend {
    fn init(
        _logger: Logger,
        _handle: LoopHandle<'static, State>,
        _display: &mut Display,
    ) -> Result<Box<dyn Backend>, Box<dyn Error>>
    where
        Self: Sized,
    {
        Ok(Box::new(WaylandBackend))
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
}
