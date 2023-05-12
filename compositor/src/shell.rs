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
    wayland::{
        compositor,
        shell::xdg::{ToplevelSurface, XdgToplevelSurfaceData},
    },
    xwayland::X11Surface,
};
use wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface, Client, DisplayHandle, Resource};

use crate::{
    wayland::protocols::{
        ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
    },
    Aerugo,
};

/// The underlying surface.
#[derive(Debug)]
pub enum Surface {
    Toplevel(ToplevelSurface),
    XWayland(X11Surface),
}

#[derive(Debug)]
pub struct ToplevelState {
    id: ToplevelId,

    /// Underlying surface of the toplevel.
    surface: Surface,

    /// Acknowledged state of the toplevel.
    current: State,

    /// The pending state of the toplevel.
    ///
    /// This is updated when the configure is acknowledged.
    pending: Option<Mapped>,

    /// Foreign handles to this toplevel.
    handles: Vec<ExtForeignToplevelHandleV1>,

    mapped_count: u64,
    // TODO: xdg-foreign id?
}

impl ToplevelState {
    pub fn create_handle(
        &mut self,
        identifier: &str,
        instance: &ExtForeignToplevelListV1,
        display: &DisplayHandle,
        client: &Client,
    ) {
        let handle = client
            .create_resource::<ExtForeignToplevelHandleV1, _, Aerugo>(display, 1, self.id)
            .unwrap();
        instance.toplevel(&handle);
        handle.identifier(identifier.into());

        match self.surface {
            Surface::Toplevel(ref surface) => compositor::with_states(surface.wl_surface(), |states| {
                let data = states.data_map.get::<XdgToplevelSurfaceData>().unwrap().lock().unwrap();

                if let Some(ref app_id) = data.app_id {
                    handle.app_id(app_id.into());
                }

                if let Some(ref title) = data.title {
                    handle.title(title.into());
                }
            }),
            Surface::XWayland(ref surface) => {
                handle.title(surface.title());
                // The class of the window is the X11 equivalent to app id.
                handle.app_id(surface.class());
            }
        };

        handle.done();
        self.handles.push(handle.clone());
    }

    /// For client developers:
    ///
    /// The protocol discourages trying to use the identifier to guess things like what was last mapped:
    /// > How the generation value is used when generating the identifier is implementation dependent.
    pub fn make_identifier(&self, generation: u64) -> String {
        let id = self.id();
        let mapped_count = self.map_count();
        format!("{generation:08X}{id:08X}{mapped_count:08X}")
    }

    pub fn id(&self) -> ToplevelId {
        self.id
    }

    pub fn map_count(&self) -> u64 {
        self.mapped_count
    }
}

/// The state of a toplevel.
#[derive(Debug, Default)]
enum State {
    /// The toplevel is not mapped.
    #[default]
    NotMapped,

    /// The toplevel is currently mapped.
    Mapped {
        /// The state of the toplevel.
        state: Mapped,
        /// The identifier.
        identifier: String,
    },
}

/// The state of a mapped toplevel.
#[derive(Debug)]
struct Mapped {
    /// Size of the window.
    ///
    /// If this is `0x0` then we don't care about the surface size.
    size: Size<i32, Logical>,

    /// The serial of this state.
    serial: Serial,
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

    pub fn commit(comp: &mut Aerugo, surface: &WlSurface) {
        // If the surface is pending, tell the WM about the new window.
        if let Some(toplevel_index) = comp
            .shell
            .pending_toplevels
            .iter()
            .position(|toplevel| toplevel.wl_surface() == surface)
        {
            let toplevel = comp.shell.pending_toplevels.remove(toplevel_index);

            // TODO: Remove this temporary configure and make the WM send the configure.
            toplevel.send_configure();

            let id = comp.shell.next_toplevel_id;
            comp.shell.next_toplevel_id = comp
                .shell
                .next_toplevel_id
                .checked_add(1)
                .expect("u64 overflow (unlikely)");

            comp.shell.toplevels.insert(
                id,
                ToplevelState {
                    id,
                    surface: Surface::Toplevel(toplevel),
                    current: State::default(),
                    pending: None,
                    handles: Vec::new(),
                    mapped_count: 0,
                },
            );

            return;
        }

        // TODO: Check if the surface is a toplevel and ack the state.
        //
        // TODO: Could store id in surface state and lookup id vs worst case O(1) list lookup
        let toplevel = comp
            .shell
            .toplevels
            .values_mut()
            .find(|state| match &state.surface {
                Surface::Toplevel(toplevel) => surface == toplevel.wl_surface(),
                Surface::XWayland(xwayland) => Some(surface) == xwayland.wl_surface().as_ref(),
            })
            .unwrap();

        let _mapped = match &mut toplevel.current {
            State::NotMapped => {
                let identifier = toplevel.make_identifier(comp.generation);

                // init foreign toplevel handle
                toplevel.current = State::Mapped {
                    state: Mapped {
                        size: (0, 0).into(), // TODO
                        serial: 0.into(),    // TODO
                    },
                    identifier: identifier.clone(),
                };

                for instance in comp.shell.foreign_toplevel_instances.values() {
                    let Some(client) = instance.instance.client() else {
                        continue;
                    };

                    toplevel.create_handle(&identifier, &instance.instance, &comp.display, &client);
                }

                let State::Mapped { ref mut state, .. } = toplevel.current else {
                    unreachable!()
                };

                state
            }
            State::Mapped { ref mut state, .. } => {
                // TODO: Handle null buffer attach (effectively unmapping)
                state
            }
        };

        // TODO: Apply pending state.
        if let Some(_pending) = toplevel.pending.take() {}
    }

    pub fn get_state(&self, id: ToplevelId) -> Option<&ToplevelState> {
        self.toplevels.get(&id)
    }
}
