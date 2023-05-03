#![allow(dead_code)]

// TODO: XWayland
// TODO: Layer shell
// TODO: Aerugo shell implementation

// TODO: Remove when used

/*
TODO: Transactions - move this to a higher level

The idea I have in mind is to make the application of window and WM state be atomically committed.

First the WM creates a graph to describe what is desired to be posted to a display. This graph is built of
nodes. The WM may need to change the state of a window however to apply this new state. However the surface
update may take some time. Furthermore the WM state applying before the surface state or vice versa would
cause issues. To solve this we ensure that changes to the WM state are commited once the window states have
been committed. (TODO: How do we handle windows which refuse to respond? We could ping the client to test for
that in the transaction).

If the clients fail to commit the previous transaction states, should the WM's next state override the current
client state, and cancel the previous transaction?
*/

use std::num::NonZeroU64;

use rustc_hash::FxHashMap;
use smithay::{
    utils::{Logical, Serial, Size},
    wayland::shell::xdg::ToplevelSurface,
    xwayland::X11Surface,
};
use wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface};

use crate::wayland::protocols::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;

/// The underlying surface.
#[derive(Debug)]
pub enum Surface {
    Toplevel(ToplevelSurface),
    XWayland(X11Surface),
}

#[derive(Debug)]
pub struct ToplevelState {
    /// Underlying surface of the toplevel.
    surface: Surface,

    // TODO: Attached texture.
    /// Acknowledged state of the toplevel.
    ///
    /// [`None`] if the state of the toplevel has not been acknowledged yet.
    acked: Option<State>,
}

#[derive(Debug)]
struct State {
    /// Size of the window.
    ///
    /// If this is `0x0` then we don't care about the surface size.
    pub size: Size<i32, Logical>,

    /// The serial of this state.
    pub serial: Serial,
}

pub type ToplevelId = NonZeroU64;

#[derive(Debug)]
pub struct Shell {
    /// Toplevel surfaces pending an initial commit.
    pub pending_toplevels: Vec<ToplevelSurface>,

    pub toplevels: FxHashMap<ToplevelId, ToplevelState>,

    /// State related to instances of the foreign toplevel protocols and extension protocols.
    pub foreign_toplevel_instances: FxHashMap<ObjectId, ForeignToplevelInstance>,

    next_toplevel_id: ToplevelId,
}

#[derive(Debug)]
pub struct ForeignToplevelInstance {
    pub instance: ExtForeignToplevelListV1,
    pub stopped: bool,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            pending_toplevels: Vec::new(),
            toplevels: Default::default(),
            foreign_toplevel_instances: Default::default(),
            next_toplevel_id: NonZeroU64::new(1).unwrap(),
        }
    }

    pub fn commit(&mut self, surface: &WlSurface) {
        // If the surface is pending, tell the WM about the new window.
        if let Some(toplevel_index) = self
            .pending_toplevels
            .iter()
            .position(|toplevel| toplevel.wl_surface() == surface)
        {
            let toplevel = self.pending_toplevels.remove(toplevel_index);

            // TODO: Remove this temporary configure and make the WM send the configure.
            toplevel.send_configure();

            let id = self.next_toplevel_id;
            self.next_toplevel_id = self.next_toplevel_id.checked_add(1).expect("u64 overflow (unlikely)");

            self.toplevels.insert(
                id,
                ToplevelState {
                    surface: Surface::Toplevel(toplevel),
                    acked: None,
                },
            );

            // TODO: Advertise the window using ext-foreign-toplevel-list-v1
            return;
        }

        // TODO: Check if the surface is a toplevel and ack the state.
        //
        // TODO: Could store id in surface state and lookup id vs worst case O(1) list lookup
        let state = self
            .toplevels
            .values_mut()
            .find(|state| match &state.surface {
                Surface::Toplevel(toplevel) => surface == toplevel.wl_surface(),
                Surface::XWayland(xwayland) => Some(surface) == xwayland.wl_surface().as_ref(),
            })
            .unwrap();

        match &state.surface {
            Surface::Toplevel(_toplevel) => {
                // TODO: Check for open transactions on this?
                //
            }

            Surface::XWayland(_) => todo!(),
        }
    }

    pub fn get_state(&self, id: ToplevelId) -> Option<&ToplevelState> {
        self.toplevels.get(&id)
    }
}
