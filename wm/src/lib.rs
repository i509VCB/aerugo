//! Aerugo window management API
//!
//! The Aerugo Wayland compositor allows a special client to act as a window manager.
//!
//! This library provides an implementation of the `aerugo-wm-v1` protocol that manages the messiness involved
//! with implementing the protocol yourself with the [wayland-client] crate.
//!
//! # Overview
//!
//! This library is based around the [`Wm`] type. In order to handle incoming and outbound messages, you are
//! expected to use a [`Wm`] inside of an event loop. This library does not dictate what event loop library
//! to use and provides a mechanism to poll a [`Wm`].
//!
//! An [`Event`] is the primary way through which your window manager will get updates from the server. An
//! [`Event`] can indicate an update such as a window being closed.
//!
//! A toplevel is referenced by it's [`ToplevelId`].
//!
//! [wayland-client]: https://crates.io/crates/wayland-client

mod configure;
mod error;
mod event;
mod id;
mod transaction;
mod wm;

pub use configure::*;
pub use error::*;
pub use event::*;
pub use transaction::*;

/// A handle to the window management capabilities of the display server.
pub struct Wm(wm::Inner);

impl Wm {
    // TODO: Connection/Backend?
    pub fn new() -> Result<Self, Setup> {
        wm::Inner::new().map(Wm)
    }
}

/// An id used to identify a toplevel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToplevelId(id::Toplevel);

/// An id used to identify some submitted transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(id::Transaction);

/// A scene graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node(id::Node);
