use std::{env, error::Error};

use slog::{info, Logger};
use smithay::{
    backend::{
        self,
        x11::{Window, X11Event, X11Surface},
    },
    reexports::{calloop::LoopHandle, wayland_server::Display},
};

use crate::state::NameMe;

use super::Backend;

#[derive(Debug)]
pub struct X11Backend {
    logger: Logger,
    // TODO: Replace this with X11Handle when PR is merged.
    _window: Window,
    outputs: Vec<X11Output>,
}

impl Backend for X11Backend {
    fn new(logger: Logger, handle: LoopHandle<'_, NameMe>, _display: &mut Display) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        let backend = backend::x11::X11Backend::new(logger.clone())?;
        let window = backend.window();
        let logger = logger.new(slog::o!("backend" => "x11"));

        handle.insert_source(backend, |event, window, name_me| match event {
            X11Event::CloseRequested => {
                let backend = name_me.state.downcast_backend_mut::<Self>().unwrap();

                backend.outputs.retain(|output| &output.window != window);

                if backend.outputs.is_empty() {
                    info!(
                        name_me.state.backend().logger(),
                        "Quitting because all outputs are destroyed"
                    );
                    name_me.state.continue_loop = false;
                }
            }

            X11Event::Input(event) => name_me.state.handle_input(event),

            _ => (),
        })?;

        Ok(X11Backend {
            logger,
            _window: window,
            outputs: vec![],
        })
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

    fn setup_outputs(&mut self, _display: &mut Display) {
        // TODO: Pending multi-window support.
    }
}

#[derive(Debug)]
struct X11Output {
    window: Window,
    _surface: X11Surface,
}

impl Drop for X11Output {
    fn drop(&mut self) {
        // TODO: Destroy the global?
    }
}
