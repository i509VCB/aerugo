use std::{error::Error, time::Duration};

use slog::Logger;
use smithay::{
    backend::{
        input::{InputBackend, InputEvent},
        winit::{self, WinitGraphicsBackend, WinitInputBackend},
    },
    reexports::calloop::{timer::Timer, LoopHandle},
};

use crate::{backend::Backend, state::State};

#[derive(Debug)]
pub struct WinitBackend {
    logger: Logger,
}

impl WinitBackend {
    pub fn new(logger: Logger) -> WinitBackend {
        WinitBackend { logger }
    }
}

impl Backend for WinitBackend {
    fn setup_backend(&mut self, handle: LoopHandle<'static, State>) -> Result<(), Box<dyn Error>> {
        let (renderer, input) = winit::init(self.logger.clone()).unwrap();
        let timer = Timer::new().unwrap();
        let timer_handle = timer.handle();

        handle.insert_source(
            timer,
            |(mut input, renderer): (WinitInputBackend, WinitGraphicsBackend), handle, _state| {
                #[allow(clippy::single_match)] // TODO: Not done yet
                match input.dispatch_new_events(|event| {
                    match event {
                        InputEvent::Special(special) => {
                            #[allow(clippy::single_match)] // TODO: Not done yet
                            match special {
                                // WinitEvent::Resized { .. } => (),
                                // WinitEvent::Refresh => todo!(),
                                _ => (),
                            }
                        }

                        _ => (),
                    }
                }) {
                    Ok(()) => {
                        // TODO: Schedule rendering
                        // TODO: Schedule timeout on current framerate and not a fixed 120
                        handle.add_timeout(Duration::from_millis(8), (input, renderer));
                    }

                    Err(_) => (),
                }
            },
        )?;

        timer_handle.add_timeout(Duration::ZERO, (input, renderer));

        Ok(())
    }

    fn setup_globals(
        &mut self,
        _display: &mut smithay::reexports::wayland_server::Display,
        _logger: Logger,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn name(&self) -> &str {
        "winit"
    }
}
