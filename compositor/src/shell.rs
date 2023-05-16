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
    backend::renderer::utils::with_renderer_surface_state,
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

#[derive(Debug)]
pub struct Shell {
    // TODO: Remove surfaces that are never mapped and destroyed.
    /// Toplevel surfaces pending an initial commit.
    pub pending_toplevels: Vec<ToplevelSurface>,

    pub toplevels: FxHashMap<ToplevelId, Toplevel>,

    /// State related to instances of the foreign toplevel protocols and extension protocols.
    pub foreign_toplevel_instances: FxHashMap<ObjectId, ForeignToplevelInstance>,

    next_toplevel_id: ToplevelId,
}

#[derive(Debug)]
pub struct ForeignToplevelInstance {
    pub instance: ExtForeignToplevelListV1,
    pub stopped: bool,
}

/// The underlying surface.
#[derive(Debug)]
pub enum Surface {
    Toplevel(ToplevelSurface),
    XWayland(X11Surface),
}

/// A toplevel surface.
#[derive(Debug)]
pub struct Toplevel {
    /// The id of the toplevel.
    id: ToplevelId,

    /// Underlying surface.
    surface: Surface,

    /// Current state.
    current: State,

    /// The pending state.
    ///
    /// This is updated when the configure is acked.
    pending: Option<Mapped>,

    /// Foreign handles to this toplevel.
    handles: Vec<ExtForeignToplevelHandleV1>,
    // TODO: xdg-foreign id?
}

pub type ToplevelId = NonZeroU64;

impl Toplevel {
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

    pub fn wl_surface(&self) -> Option<WlSurface> {
        match &self.surface {
            Surface::Toplevel(toplevel) => Some(toplevel.wl_surface().clone()),
            Surface::XWayland(xwayland) => xwayland.wl_surface(),
        }
    }

    /// For client developers:
    ///
    /// The protocol discourages trying to use the identifier to guess things like what was last mapped:
    /// > How the generation value is used when generating the identifier is implementation dependent.
    pub fn make_identifier(&self, generation: u64) -> String {
        format!("{:016X}{:016X}", generation, self.id)
    }

    pub fn remove_handle(&mut self, id: ObjectId) {
        if let Some(index) = self.handles.iter().position(|handle| handle.id() == id) {
            self.handles.remove(index);
        }
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
        let has_buffer = with_renderer_surface_state(surface, |state| state.buffer().is_some());

        // If the surface is pending, tell the WM about the new window.
        if let Some(toplevel_index) = comp
            .shell
            .pending_toplevels
            .iter()
            .position(|toplevel| toplevel.wl_surface() == surface)
        {
            let toplevel = comp.shell.pending_toplevels.remove(toplevel_index);

            // Ensure the toplevel has no attached buffer during initial commit
            if has_buffer {
                todo!("Either add XdgSurface to ToplevelSurface or search")
                // TODO: Send UnconfiguredBuffer
            }

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
                Toplevel {
                    id,
                    surface: Surface::Toplevel(toplevel),
                    current: State::default(),
                    pending: None,
                    handles: Vec::new(),
                },
            );

            return;
        }

        // If the surface is mapped (which it will be if it is not pending) and a buffer is no longer attached
        // then unmap the surface and return it to pending.
        if !has_buffer {
            if let Some(key) = comp
                .shell
                .toplevels
                .iter()
                .find_map(|(key, toplevel)| (toplevel.wl_surface().as_ref() == Some(surface)).then_some(*key))
            {
                if let Some(toplevel) = comp.shell.toplevels.remove(&key) {
                    // TODO: Tell ext-foreign-toplevel objects the surface is closed.

                    match toplevel.surface {
                        Surface::Toplevel(surface) => comp.shell.pending_toplevels.push(surface),
                        Surface::XWayland(_) => todo!("How to handle xwayland?"),
                    }
                }
            }

            // Unmapped, propagate nothing more
            return;
        }

        // Apply updates to surface state
        // TODO
    }

    pub fn destroyed(comp: &mut Aerugo, surface: &WlSurface) {}

    pub fn get_state(&self, id: ToplevelId) -> Option<&Toplevel> {
        self.toplevels.get(&id)
    }

    pub fn get_state_mut(&mut self, id: ToplevelId) -> Option<&mut Toplevel> {
        self.toplevels.get_mut(&id)
    }
}
