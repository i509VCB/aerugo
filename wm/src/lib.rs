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

mod aerugo_wm;
mod configure;
mod error;
mod event;
mod foreign_toplevel;
mod id;
mod node;
mod transaction;
mod wm;

use std::io;

pub use configure::*;
pub use error::*;
pub use event::*;
pub use transaction::*;

pub use euclid;

use wayland_client::{protocol::wl_surface::WlSurface, Connection, EventQueue};

pub struct AlreadyDestroyed;

/// A handle to the window management capabilities of the display server.
pub struct Wm {
    inner: wm::Inner,
    queue: EventQueue<wm::Inner>,
}

impl Wm {
    // TODO: Connection/Backend?
    pub fn new(conn: &Connection) -> Result<Self, Setup> {
        let (inner, queue) = wm::Inner::new(conn)?;
        Ok(Self { inner, queue })
    }

    pub fn blocking_dispatch(&mut self) -> io::Result<()> {
        self.queue
            .blocking_dispatch(&mut self.inner)
            .map_err(wm::map_dispatch)?;
        Ok(())
    }

    /// Read an event from the wm.
    ///
    /// Returns [`None`] if there are no more pending messages.
    pub fn read_event(&mut self) -> Option<Event> {
        self.inner.pop_event()
    }

    pub fn get_status(&self, _transaction: TransactionId) -> Status {
        todo!()
    }

    /// Return the identifier of the underlying foreign toplevel handle.
    ///
    /// This can be used to correlate a toplevel instance from this [`Wm`] elsewhere.
    pub fn get_toplevel_identifier(&self, _toplevel: ToplevelId) -> Option<&str> {
        todo!()
    }

    /// Release the toplevel's resources.
    ///
    /// Calling this function will result in the toplevel being unmapped and forgotten by the [`Wm`] instance
    /// if this is not called as a result of a [`ToplevelEvent::Closed`] event.
    pub fn release_toplevel(&mut self, toplevel: ToplevelId) -> Result<(), AlreadyDestroyed> {
        self.inner.release_toplevel(toplevel.0)
    }

    pub fn create_toplevel_node(&mut self, toplevel: ToplevelId) -> ToplevelNode {
        todo!()
    }

    pub fn create_transaction(&self) -> Transaction<'_> {
        todo!()
    }

    // TODO: Creating transactions

    // TODO: Cancelling transactions

    // TODO: Polling related stuff
}

/// id used to identify a toplevel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToplevelId(id::Toplevel);

/// id used to identify some submitted transaction.
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
}

mod private {
    use std::num::NonZeroU32;

    /// Crate private implementation details for node implementations.
    pub trait NodePrivate: Sized {
        fn generation(&self) -> NonZeroU32;

        // TODO: Anything generic over parameters should be delegated to here
    }
}
