use std::{
    error::Error,
    fmt::{self, Formatter},
    sync::Mutex,
};

use clap::{ArgGroup, Parser};
use slog::{error, o, Drain, Logger};
use wayland_compositor::{backend::Backend, run, state::Socket};

#[cfg(feature = "udev_backend")]
use wayland_compositor::backend::udev::UdevBackend;
#[cfg(feature = "wayland_backend")]
use wayland_compositor::backend::wayland::WaylandBackend;
#[cfg(feature = "x11_backend")]
use wayland_compositor::backend::x11::X11Backend;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let logger = if args.debug_logger {
        Logger::root(Mutex::new(slog_term::term_full().fuse()).fuse(), o!())
    } else {
        Logger::root(slog_async::Async::default(slog_term::term_full().fuse()).fuse(), o!())
    };

    let _guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().expect("Could not setup logging backend");

    // TODO: Configurable socket setup
    if let Err(err) = args.backend.run(logger.clone(), Socket::Auto) {
        match err {
            StartError::NoBackendAvailable => {
                error!(logger, "No backends available to start the compositor");
                Err(err.into())
            }

            StartError::Other(err) => Err(err),
        }
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Parser)]
struct Args {
    #[clap(flatten)]
    backend: BackendSelection,

    /// Whether the compositor should use the slower mutex logger over the async logger.
    #[clap(long)]
    debug_logger: bool,
}

#[derive(Debug)]
enum StartError {
    NoBackendAvailable,

    Other(Box<dyn Error>),
}

impl From<Box<dyn Error>> for StartError {
    fn from(err: Box<dyn Error>) -> Self {
        StartError::Other(err)
    }
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StartError::NoBackendAvailable => write!(f, "No suitable backend was available"),
            StartError::Other(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl Error for StartError {}

#[derive(Debug, Clone, Copy, Parser)]
#[clap(group = ArgGroup::new("backend").required(false).multiple(false))]
struct BackendSelection {
    #[cfg_attr(
        all(feature = "wayland_backend", not(feature = "x11_backend")),
        doc = "Run the compositor as a Wayland client."
    )]
    #[cfg_attr(
        all(feature = "x11_backend", not(feature = "wayland_backend")),
        doc = "Run the compositor as an X11 client."
    )]
    #[cfg_attr(
        all(feature = "wayland_backend", feature = "x11_backend"),
        doc = "Run the compositor inside an existing session as a window.\n\n",
        doc = "This option will automatically choose to run as a Wayland or X11 client depending on the current session.",
        doc = "If you need to explicitly run the compositor as an X11 or Wayland client, use the \"--x11\" or \"--wayland\" flag."
    )]
    #[cfg(any(feature = "wayland_backend", feature = "x11_backend"))]
    #[clap(long, group = "backend")]
    windowed: bool,

    /// Run the compositor as an X11 client.
    #[cfg(any(feature = "x11_backend"))]
    #[clap(long, group = "backend")]
    x11: bool,

    /// Run the compositor as a Wayland client.
    #[cfg(any(feature = "wayland_backend"))]
    #[clap(long, group = "backend")]
    wayland: bool,

    /// Run the compositor in a tty session.
    #[cfg(feature = "udev_backend")]
    #[clap(long, group = "backend")]
    udev: bool,
}

impl BackendSelection {
    fn run(self, logger: Logger, socket: Socket) -> Result<(), StartError> {
        // ArgGroup requirements mean only 1 boolean will be set

        #[cfg(any(feature = "wayland_backend", feature = "x11_backend"))]
        {
            // Select the session type.
            if self.windowed {
                // Try Wayland first if enabled
                #[cfg(feature = "wayland_backend")]
                {
                    if WaylandBackend::available() {
                        return Ok(run(logger.clone(), WaylandBackend::new, socket)?);
                    }
                }

                // Try X second if wayland is enabled
                #[cfg(feature = "x11_backend")]
                {
                    if X11Backend::available() {
                        return Ok(run(logger, X11Backend::init, socket)?);
                    }
                }

                return Err(StartError::NoBackendAvailable);
            }
        }

        #[cfg(feature = "wayland_backend")]
        {
            if self.wayland {
                return Ok(run(logger, WaylandBackend::new, socket)?);
            }
        }

        #[cfg(feature = "x11_backend")]
        {
            if self.x11 {
                return Ok(run(logger, X11Backend::init, socket)?);
            }
        }

        #[cfg(feature = "udev_backend")]
        {
            if self.udev {
                return Ok(run(logger, UdevBackend::init, socket)?);
            }
        }

        // Auto-detect which backend to use.

        #[cfg(feature = "wayland_backend")]
        {
            // Try Wayland as first fallback if enabled
            if WaylandBackend::available() {
                return Ok(run(logger, WaylandBackend::new, socket)?);
            }
        }

        #[cfg(feature = "x11_backend")]
        {
            // Then try X as fallback
            if X11Backend::available() {
                return Ok(run(logger, X11Backend::init, socket)?);
            }
        }

        #[cfg(feature = "udev_backend")]
        {
            if UdevBackend::available() {
                return Ok(run(logger, UdevBackend::init, socket)?);
            }
        }

        Err(StartError::NoBackendAvailable)
    }
}
