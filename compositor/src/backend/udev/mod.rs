use std::error::Error;

use slog::Logger;
use smithay::{
    backend::session::auto::{AutoSession, AutoSessionNotifier},
    reexports::{calloop::LoopHandle, wayland_server::Display},
};

use crate::state::NameMe;

use super::Backend;

#[derive(Debug)]
pub struct UdevBackend {
    _session: AutoSession,
    _notifier: AutoSessionNotifier,
}

impl Backend for UdevBackend {
    fn new(logger: Logger, _handle: LoopHandle<'_, NameMe>, _display: &mut Display) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        let (session, notifier) = AutoSession::new(logger).unwrap();

        Ok(UdevBackend {
            _session: session,
            _notifier: notifier,
        })
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

    fn setup_outputs(&mut self, _display: &mut Display) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}
