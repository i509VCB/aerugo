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
mod node;
mod transaction;
mod wm;

pub use configure::*;
pub use error::*;
pub use event::*;
pub use transaction::*;
use wayland_client::protocol::wl_surface::WlSurface;

/// A handle to the window management capabilities of the display server.
pub struct Wm(wm::Inner);

impl Wm {
    // TODO: Connection/Backend?
    pub fn new() -> Result<Self, Setup> {
        wm::Inner::new().map(Wm)
    }

    pub fn get_status(&self, _transaction: TransactionId) -> Status {
        todo!()
    }

    // TODO: Creating transactions

    // TODO: Cancelling transactions

    // TODO: Polling related stuff
}

/// Id used to identify a toplevel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToplevelId(id::Toplevel);

/// Id used to identify some submitted transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(id::Transaction);

/// Status of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    Pending,

    Finished,

    Cancelled,
}

/// Marker trait used to parameterize the type of a node in functions.
pub trait Node: private::NodePrivate + Sized {}

/// A node backed by a surface.
///
/// This type of node allows the wm client to place it's own surfaces in the scene graph.
#[derive(Debug)]
pub struct SurfaceNode(node::Surface);
impl Node for SurfaceNode {}

impl SurfaceNode {
    pub fn surface(&self) -> &WlSurface {
        &self.0.wl_surface
    }
}

// TODO: Implement HasWindowHandle for SurfaceNode?

/// A node backed by a toplevel.
///
/// This effectively allows a toplevel to be placed in the scene graph with modifiers.
pub struct ToplevelNode(node::Toplevel);
impl Node for ToplevelNode {}

impl ToplevelNode {
    pub fn toplevel(&self) -> ToplevelId {
        self.0.toplevel
    }

    /// Return the identifier of the underlying toplevel handle.
    pub fn identifier(&self) -> &str {
        todo!()
    }
}

mod private {
    use std::num::NonZeroU32;

    /// Crate private implementation details for node implementations.
    pub trait NodePrivate: Sized {
        fn generation(&self) -> NonZeroU32;

        // TODO: Anything generic over parameters should be delegated to here
    }
}
