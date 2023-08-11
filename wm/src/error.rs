use std::{error::Error, fmt, io, ops::RangeInclusive};

/// Errors that can occur when creating a [`Wm`].
///
/// [`Wm`]: crate::Wm
#[derive(Debug)]
pub enum Setup {
    /// One or more globals are missing.
    ///
    /// This may indicate the compositor does not support the aerugo window management api.
    MissingGlobals(Vec<MissingGlobal>),

    /// An [`io::Error`].
    Io(io::Error),
}

impl fmt::Display for Setup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Setup::MissingGlobals(_) => write!(f, "some required globals are missing"),
            Setup::Io(ref io) => fmt::Display::fmt(io, f),
        }
    }
}

impl Error for Setup {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Setup::Io(ref io) => Some(io),
            _ => None,
        }
    }
}

/// A missing global.
#[derive(Debug)]
pub enum MissingGlobal {
    /// A global with the specified interface is not available.
    Missing(String),

    /// A global with the specified version is available, but a compatible version is not available.
    IncompatibleVersion {
        //// Name of the interface
        interface: String,

        /// Advertised version
        version: u32,

        /// The compatible versions
        compatible: RangeInclusive<u32>,
    },
}
