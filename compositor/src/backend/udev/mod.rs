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
    fn new(_logger: Logger) -> Self
    where
        Self: Sized,
    {
        UdevBackend
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        // This is kinda hacky but it should be fine.
        AutoSession::new(None).is_some()
    }

    fn setup_backend(&mut self, _handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>> {
        todo!("Udev backend is not implemented yet")
    }

    fn setup_globals(&mut self, _display: &mut Display) -> Result<(), Box<dyn Error>> {
        todo!("Udev backend is not implemented yet")
    }

    fn name(&self) -> &str {
        "udev"
    }
}
