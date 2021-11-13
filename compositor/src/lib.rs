#![warn(missing_debug_implementations)]

pub mod backend;
pub mod shell;
pub mod state;
pub mod surface_data;
pub mod vulkan;

#[cfg(feature = "xwayland")]
mod xwayland;

use std::{cell::RefCell, error::Error, rc::Rc};

use backend::Backend;
use slog::Logger;
use smithay::reexports::{
    calloop::{EventLoop, LoopHandle},
    wayland_server::Display,
};

use crate::state::{Socket, State};

pub type CreateBackendFn =
    fn(Logger, LoopHandle<'static, State>, &mut Display) -> Result<Box<dyn Backend + 'static>, Box<dyn Error>>;

/// The main entrypoint of the compositor.
pub fn run(logger: Logger, backend: CreateBackendFn, socket: Socket) -> Result<(), Box<dyn Error>> {
    let display = Rc::new(RefCell::new(Display::new()));
    let mut event_loop = EventLoop::try_new()?;
    let mut state = State::new(logger, event_loop.handle(), display, socket, backend)?;

    // Signal used to shut down the event loop..
    let signal = event_loop.get_signal();

    #[cfg(feature = "xwayland")]
    state.start_xwayland();

    event_loop.run(None, &mut state, |state| {
        if !state.should_continue() {
            signal.stop();
            return;
        }

        let display = state.display.clone();
        // Send events to the client or else no new requests will ever come in.
        display.borrow_mut().flush_clients(state);
    })?;

    // TODO: Any relevant cleanup

    Ok(())
}
