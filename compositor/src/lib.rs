#![warn(missing_debug_implementations)]

pub mod backend;
pub mod state;
pub mod vulkan;

#[cfg(feature = "xwayland")]
mod xwayland;

use std::{error::Error, time::Duration};

use backend::Backend;
use slog::{error, Logger};
use smithay::reexports::{
    calloop::{generic::Generic, EventLoop, Interest, LoopHandle, Mode, PostAction},
    wayland_server::Display,
};
use state::CompositorState;

use crate::state::{NameMe, Socket};

/// The main entrypoint of the compositor.
pub fn run<B>(logger: Logger, socket: Socket) -> Result<(), Box<dyn Error>>
where
    B: Backend,
{
    let mut display = Display::new();
    let mut event_loop = EventLoop::try_new()?;

    // Make sure we dispatch the display when there are pending requests.
    insert_wayland_readiness_source(event_loop.handle(), &display)?;

    let mut backend = Box::new(B::new(logger.clone(), event_loop.handle(), &mut display)?);
    // TODO: Renderer init
    // Create the outputs to start with.
    backend.setup_outputs(&mut display);

    let state = CompositorState::new(logger, &mut display, socket, backend as Box<_>)?;
    let mut name_me = NameMe { display, state };

    // Signal used to shut down the event loop..
    let signal = event_loop.get_signal();

    #[cfg(feature = "xwayland")]
    state.start_xwayland();

    event_loop.run(None, &mut name_me, |name_me| {
        if !name_me.state.should_continue() {
            signal.stop();
            return;
        }

        // Send events to the client or else no new requests will ever come in.
        name_me.display.flush_clients(&mut name_me.state);
    })?;

    // TODO: Any relevant cleanup

    Ok(())
}

/// Inserts an event source which will be activated when there are pending messages for the compositor to
/// process.
fn insert_wayland_readiness_source(handle: LoopHandle<'_, NameMe>, display: &Display) -> Result<(), Box<dyn Error>> {
    handle.insert_source(
        Generic::from_fd(
            // The file descriptor which indicates there are pending messages.
            display.get_poll_fd(),
            Interest::READ,
            Mode::Level,
        ),
        move |_, _, name_me: &mut NameMe| {
            let display = &mut name_me.display;
            let inner = &mut name_me.state;

            if let Err(err) = display.dispatch(Duration::ZERO, inner) {
                error!(name_me.state.logger, "Error while dispatching requests"; "error" => &err);
                name_me.state.continue_loop = false;
                Err(err)
            } else {
                Ok(PostAction::Continue)
            }
        },
    )?;

    Ok(())
}
