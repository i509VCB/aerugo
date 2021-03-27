use std::error::Error;

use wayland_compositor::{backend::winit::WinitBackend, run, state::Socket};
use slog::{o, Drain, Logger};

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger:
    let logger = Logger::root(
        slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
        o!(),
    );

    let _guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().expect("Could not setup log backend");

    // TODO: Arg parsing and other things.
    let backend = WinitBackend::new(logger.clone());

    run(logger.clone(), backend, Socket::Auto)
}
