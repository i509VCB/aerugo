use std::{env, error::Error};

use slog::{info, Logger};
use smithay::{
    backend::{self, x11::X11Event},
    reexports::{calloop::LoopHandle, wayland_server::Display},
};

use crate::state::State;

use super::Backend;

#[derive(Debug)]
pub struct X11Backend {
    logger: Logger,
}

impl Backend for X11Backend {
    fn init(
        logger: Logger,
        handle: LoopHandle<'static, State>,
        _display: &mut Display,
    ) -> Result<Box<dyn Backend>, Box<dyn Error>>
    where
        Self: Sized,
    {
        let (backend, _surface) = backend::x11::X11Backend::new(logger.clone())?;
        let logger = logger.new(slog::o!("backend" => "x11"));

        #[allow(clippy::single_match)] // temporary
        handle.insert_source(backend, |event, _window, state| match event {
            X11Event::CloseRequested => {
                info!(state.backend().logger(), "Closing compositor");
                state.continue_loop = false;
            }

            _ => (),
        })?;

        Ok(Box::new(X11Backend { logger }))
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        env::var("DISPLAY").is_ok()
    }

    fn name(&self) -> &str {
        "x11"
    }

    fn logger(&self) -> &Logger {
        &self.logger
    }
}
