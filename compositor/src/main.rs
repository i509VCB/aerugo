use std::error::Error;

use clap::{ArgGroup, Clap};
use slog::{o, Drain, Logger};
use wayland_compositor::{
    backend::{winit::WinitBackend, Backend},
    run,
    state::Socket,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Initialize logger
    let logger = Logger::root(
        slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
        o!(),
    );

    let _guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().expect("Could not setup log backend");

    let backend = args.backend.new_backend(logger.clone());
    run(logger, backend, Socket::Auto)
}

#[derive(Debug, Clap)]
struct Args {
    #[clap(flatten)]
    backend: BackendSelection,
}

#[derive(Debug, Clap)]
#[clap(group = ArgGroup::new("backend").required(false).multiple(false))]
struct BackendSelection {
    /// Run the compositor in a window using winit.
    #[clap(long, group = "backend")]
    winit: bool,

    /// Run the compositor in a tty session.
    #[clap(long, group = "backend")]
    udev: bool,
}

impl BackendSelection {
    fn new_backend(self, logger: Logger) -> impl Backend + 'static {
        // We can ensure only a single boolean or none will be true because of the ArgGroup requirements.

        if self.winit {
            WinitBackend::new(logger)
        } else if self.udev {
            todo!()
        } else {
            // No explicit backend specified, determine which one should be used.
            // TODO: Fallback detection
            WinitBackend::new(logger)
        }
    }
}
