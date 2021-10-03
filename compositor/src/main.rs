use std::{error::Error, mem, process, thread, time::Duration};

use clap::{ArgGroup, Clap};
use slog::{error, o, Drain, Logger};
use wayland_compositor::{backend::Backend, run, state::Socket};

#[cfg(feature = "wayland_backend")]
use wayland_compositor::backend::wayland::WaylandBackend;
#[cfg(feature = "x11_backend")]
use wayland_compositor::backend::x11::X11Backend;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Initialize logger
    let logger = Logger::root(
        slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
        o!(),
    );

    let guard = slog_scope::set_global_logger(logger.clone());
    slog_stdlog::init().expect("Could not setup logging backend");
    // Leak the logger's scope for the entire span of the program.
    mem::forget(guard);

    // TODO: Configurable socket setup
    if let Err(err) = args.backend.run(logger.clone(), Socket::Auto) {
        match err {
            StartError::NoBackendAvailable => {
                error!(logger, "No backends available to start the compositor");
                thread::sleep(Duration::from_millis(50)); // Wait for the logger to flush async.
                process::exit(1)
            }

            StartError::Other(err) => Err(err),
        }
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Clap)]
struct Args {
    #[clap(flatten)]
    backend: BackendSelection,
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

#[derive(Debug, Clone, Copy, Clap)]
#[clap(group = ArgGroup::new("backend").required(false).multiple(false))]
struct BackendSelection {
    /// Run the compositor inside an existing session as a window.
    ///
    /// This option will automatically choose to run as a Wayland or X11 client depending on the current session.
    ///
    /// If you need to explicitly run the compositor as an X11 or Wayland client, use the "--x11" or "--wayland"
    /// flag.
    #[cfg(all(feature = "wayland_backend", feature = "x11_backend"))]
    #[clap(long, group = "backend")]
    windowed: bool,

    /// Run the compositor as a Wayland client.
    #[cfg(all(feature = "wayland_backend", not(feature = "x11_backend")))]
    #[clap(long, group = "backend")]
    windowed: bool,

    /// Run the compositor as an X11 client.
    #[cfg(all(feature = "x11_backend", not(feature = "wayland_backend")))]
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
                        return Ok(run(logger.clone(), WaylandBackend::new(logger), socket)?);
                    }
                }

                // Try X second if wayland is enabled
                #[cfg(feature = "x11_backend")]
                {
                    if X11Backend::available() {
                        return Ok(run(logger.clone(), X11Backend::new(logger), socket)?);
                    }
                }

                todo!("Wayland and X11 not available")
            }
        }

        #[cfg(feature = "wayland_backend")]
        {
            if self.wayland {
                return Ok(run(logger.clone(), WaylandBackend::new(logger), socket)?);
            }
        }

        #[cfg(feature = "x11_backend")]
        {
            if self.x11 {
                return Ok(run(logger.clone(), X11Backend::new(logger), socket)?);
            }
        }

        #[cfg(feature = "udev_backend")]
        {
            if self.udev {
                todo!("Udev backend implementation")
            }
        }

        // Auto-detect which backend to use.

        #[cfg(feature = "wayland_backend")]
        {
            // Try Wayland as first fallback if enabled
            if WaylandBackend::available() {
                return Ok(run(
                    logger.clone(),
                    WaylandBackend::new(logger.clone()),
                    socket,
                )?);
            }
        }

        #[cfg(feature = "x11_backend")]
        {
            // Then try X as fallback
            if X11Backend::available() {
                return Ok(run(logger.clone(), X11Backend::new(logger), socket)?);
            }
        }

        #[cfg(feature = "udev_backend")]
        {
            todo!("Check if udev is available")
        }

        Err(StartError::NoBackendAvailable)
    }
}
