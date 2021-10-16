use std::error::Error;

use slog::Logger;
use smithay::{
    backend::session::auto::AutoSession,
    reexports::{calloop::LoopHandle, wayland_server::Display},
};

use crate::state::State;

use super::Backend;

#[derive(Debug)]
pub struct UdevBackend;

impl Backend for UdevBackend {
    fn init(
        _logger: Logger,
        _handle: LoopHandle<'static, State>,
        _display: &mut Display,
    ) -> Result<Box<dyn Backend>, Box<dyn Error>>
    where
        Self: Sized,
    {
        Ok(Box::new(UdevBackend))
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        // This is kinda hacky but it should be fine.
        AutoSession::new(None).is_some()
    }

    fn name(&self) -> &str {
        "udev"
    }

    fn logger(&self) -> &Logger {
        todo!()
    }
}
