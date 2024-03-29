//! The shell implementation
//!
//! # Toplevel state machine
//!
//! When a toplevel (xdg_toplevel or xwayland surface) is mapped, the toplevel will go through three states:
//! new, not yet mapped and mapped. The reason for these states is to fulfill the requirements for
//! implementing the xdg-shell protocol.
//!
//! The state machine for a toplevel is shown below:
//!
//! ```text
//! /---> New ---> Possible to map ---> Mapped ---\
//! |              ^             |      ^    |    |
//! |              \-------------/      \----/    |
//! |                                             |
//! \---------------------------------------------/
//! ```
//!
//! A toplevel starts in the `New` state. A toplevel enters this state when it is created. To make a toplevel
//! possible to map, it must be transitioned to the next state. To make a toplevel possible to map, the client
//! must send an initial commit. During the initial commit the client may provide some hints about how the toplevel
//! should be configured. When a surface is new, the client cannot attach anything to present yet.
//!
//! After the initial commit the server will configure the toplevel. A configure describes the new state of the
//! toplevel. Before the client can apply this state, the client must acknowledge the configure. After the
//! configure is acknowledged the client can apply the configured state in the next commit. After the first
//! configure, the client may attach a buffer to map the toplevel. Otherwise the client and server could
//! continue to negotiate the current state. This means the client may perform a commit with no attached buffer
//! after the initial configure.
//!
//! After a buffer is attached the surface can be mapped. A mapped surface is not necessarily visible, but can
//! be made visible by the window management. For each future configure and or commit, the toplevel will stay
//! mapped. Only when a null buffer is attached, the toplevel is unmapped and becomes new again.
//!
//! The toplevel can also be destroyed at any state and if mapped, the surface will be unmapped.
//!
//! # Transactions
//!
//! **TODO**
//!
//! # Window management
//!
//! **TODO**

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

use std::{fmt, num::NonZeroU64, sync::Arc};

use rustc_hash::FxHashMap;
use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    utils::{Logical, Serial, Size},
    wayland::{
        compositor::{self, SurfaceAttributes, TraversalAction},
        shell::{
            wlr_layer,
            xdg::{ToplevelSurface, XdgToplevelSurfaceData},
        },
    },
    xwayland::X11Surface,
};
use wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface, Client, DisplayHandle, Resource};

use crate::{
    wayland::ext::foreign_toplevel::{
        ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
    },
    Aerugo,
};

/// A surface with some assigned role.
#[derive(Clone)]
pub struct AerugoSurface(Arc<SurfaceInner>);

impl AerugoSurface {}

impl fmt::Debug for AerugoSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AerugoSurface").finish()
    }
}

impl fmt::Display for AerugoSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl PartialEq for AerugoSurface {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

struct SurfaceInner {
    kind: SurfaceKind,
}

enum SurfaceKind {
    Toplevel(ToplevelSurface),
    XWayland(X11Surface),
    WlrLayer(wlr_layer::LayerSurface),
}

#[derive(Debug)]
pub struct Shell {
    // TODO: Remove surfaces that are never mapped and destroyed.
    /// Toplevel surfaces pending an initial commit.
    ///
    /// Toplevels in this state are effectively new.
    pub pending_toplevels: Vec<ToplevelSurface>,

    /// Toplevels that are able to or are mapped.
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

impl Surface {
    pub fn ensure_configured(&self) -> bool {
        match self {
            Surface::Toplevel(toplevel) => toplevel.ensure_configured(),
            // TODO: Xwayland?
            Surface::XWayland(_) => false,
        }
    }
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
    handles: FxHashMap<ObjectId, ToplevelHandles>,
    // TODO: xdg-foreign id?
}

#[derive(Debug)]
pub struct ToplevelHandles {
    pub handle: ExtForeignToplevelHandleV1,
}

pub type ToplevelId = NonZeroU64;

impl Toplevel {
    pub fn create_handle(
        &mut self,
        generation: u64,
        instance: &ExtForeignToplevelListV1,
        display: &DisplayHandle,
        client: &Client,
    ) -> ExtForeignToplevelHandleV1 {
        // An identifier is made of a 64-bit generation value created from a timestamp on startup and a 64-bit
        // monotonic counter. Aerugo coverts both of these into hex to create the identifier. Clients should
        // NOT rely on the behavior which Aerugo uses to allocate identifiers.
        let identifier = format!("{generation:016X}{:016X}", self.id);
        let handle = client
            .create_resource::<ExtForeignToplevelHandleV1, _, Aerugo>(display, 1, self.id)
            .unwrap();
        instance.toplevel(&handle);
        handle.identifier(identifier.into());
        self.handles
            .insert(handle.id(), ToplevelHandles { handle: handle.clone() });
        // Defer sending other information about the toplevel handles.
        handle
    }

    /// Initialize the state of a toplevel handle.
    pub fn initialize_handle(&self, handle: &ExtForeignToplevelHandleV1) {
        if let Some(title) = self.title() {
            handle.title(title);
        }

        if let Some(app_id) = self.app_id() {
            handle.app_id(app_id);
        }

        // Apply the current state of the toplevel handle.
        handle.done();
    }

    pub fn title(&self) -> Option<String> {
        match self.surface {
            Surface::Toplevel(ref toplevel) => compositor::with_states(&toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .title
                    .clone()
            }),
            Surface::XWayland(ref surface) => Some(surface.title()),
        }
    }

    pub fn app_id(&self) -> Option<String> {
        match self.surface {
            Surface::Toplevel(ref toplevel) => compositor::with_states(&toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .app_id
                    .clone()
            }),
            Surface::XWayland(ref surface) => Some(surface.class()),
        }
    }

    pub fn wl_surface(&self) -> Option<WlSurface> {
        match &self.surface {
            Surface::Toplevel(toplevel) => Some(toplevel.wl_surface().clone()),
            Surface::XWayland(xwayland) => xwayland.wl_surface(),
        }
    }

    pub fn update_state(&mut self) {
        todo!()
    }

    pub fn remove_handle(&mut self, id: ObjectId) {
        let _ = self.handles.remove(&id);
    }

    pub fn get_handles(&mut self, id: ObjectId) -> Option<&mut ToplevelHandles> {
        self.handles.get_mut(&id)
    }
}

/// The state of a toplevel.
#[derive(Debug, Default)]
enum State {
    /// The toplevel is not yet mapped, but can be mapped once acked.
    #[default]
    NotYetMapped,

    /// The toplevel is currently mapped.
    Mapped(Mapped),
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

struct AerugoToplevelData {
    toplevel_id: ToplevelId,
}

impl Shell {
    pub fn get_toplevel_id(surface: &WlSurface) -> Option<ToplevelId> {
        compositor::with_states(surface, |data| {
            data.data_map.get::<AerugoToplevelData>().map(|data| data.toplevel_id)
        })
    }

    pub fn new() -> Self {
        Shell {
            pending_toplevels: Vec::new(),
            toplevels: Default::default(),
            foreign_toplevel_instances: Default::default(),
            next_toplevel_id: NonZeroU64::new(1).unwrap(),
        }
    }

    pub fn commit(comp: &mut Aerugo, surface: &WlSurface) {
        // Handle commit for each type of role.
        Shell::toplevel_commit(comp, surface);
    }

    pub fn toplevel_commit(comp: &mut Aerugo, surface: &WlSurface) {
        let Some(id) = Shell::get_toplevel_id(surface) else {
            // If the surface is pending, then an initial commit has happened.
            if let Some(toplevel_index) = comp
                .shell
                .pending_toplevels
                .iter()
                .position(|toplevel| toplevel.wl_surface() == surface)
            {
                let toplevel = comp.shell.pending_toplevels.remove(toplevel_index);
            }

            return;
        };

        let toplevel = comp
            .shell
            .toplevels
            .get_mut(&id)
            .expect("invalid state: toplevel was unmapped before initial commit");

        let has_buffer = with_renderer_surface_state(surface, |state| state.buffer().is_some());

        // Toplevel was unmapped.
        if !has_buffer {
            // If the surface was never mapped do not unmap the toplevel since the client may have needed a
            // second commit to communicate all state.
            if !matches!(toplevel.current, State::NotYetMapped) {
                // TODO: Include app_id, remove toplevel debug impl
                tracing::debug!(?toplevel, "Unmap toplevel");
                let toplevel = comp.shell.toplevels.remove(&id).unwrap();

                // Notify clients the toplevel is being unmapped.
                for handle in toplevel.handles.values() {
                    handle.handle.closed();
                }

                match toplevel.surface {
                    Surface::Toplevel(surface) => comp.shell.pending_toplevels.push(surface),
                    Surface::XWayland(_) => todo!("How to handle xwayland?"),
                }

                return;
            }
        }

        // Make sure initial configure was acked.
        if has_buffer && !toplevel.surface.ensure_configured() {
            let id = toplevel.id;
            let app_id = toplevel.app_id().unwrap_or_default();
            tracing::warn!(%id, %app_id, "Killing client: toplevel not configured");
        }
    }

    // pub fn commit(comp: &mut Aerugo, surface: &WlSurface) {
    //     let has_buffer = with_renderer_surface_state(surface, |state| state.buffer().is_some());

    //     // If the surface is pending, tell the WM about the new window.
    //     if let Some(toplevel_index) = comp
    //         .shell
    //         .pending_toplevels
    //         .iter()
    //         .position(|toplevel| toplevel.wl_surface() == surface)
    //     {
    //         let toplevel = comp.shell.pending_toplevels.remove(toplevel_index);

    //         // Query some info about the toplevel for logging.
    //         let app_id = compositor::with_states(toplevel.wl_surface(), |states| {
    //             states
    //                 .data_map
    //                 .get::<XdgToplevelSurfaceData>()
    //                 .unwrap()
    //                 .lock()
    //                 .unwrap()
    //                 .app_id
    //                 .clone()
    //         })
    //         .unwrap_or_default();

    //         // Ensure the toplevel has no attached buffer during initial commit
    //         if has_buffer {
    //             tracing::warn!(%app_id, "Killing client: attached buffer during initial commit");
    //             todo!("Either add XdgSurface to ToplevelSurface or search")
    //             // TODO: Send UnconfiguredBuffer
    //         }

    //         // TODO: Remove this temporary configure and make the WM send the configure.
    //         toplevel.with_pending_state(|state| {
    //             // Set some size to make smithay and send a configure.
    //             //
    //             // FIXME: This seems broken as extension protocols have no way to force a configure to be sent.
    //             state.size = Some((0, 0).into());
    //         });
    //         toplevel.send_configure();

    //         let id = comp.shell.next_toplevel_id;

    //         tracing::debug!(%id, %app_id, "Initial commit of toplevel");

    //         comp.shell.next_toplevel_id = comp
    //             .shell
    //             .next_toplevel_id
    //             .checked_add(1)
    //             .expect("u64 overflow (unlikely)");

    //         let toplevel = comp.shell.toplevels.entry(id).or_insert(Toplevel {
    //             id,
    //             surface: Surface::Toplevel(toplevel),
    //             current: State::default(),
    //             pending: None,
    //             handles: FxHashMap::default(),
    //         });

    //         let mut new_instances = Vec::with_capacity(comp.shell.foreign_toplevel_instances.len());

    //         // Create the foreign toplevel handles
    //         for instance in comp.shell.foreign_toplevel_instances.values() {
    //             // Create all toplevel handle instances to ensure that extension protocols do not refer to handles
    //             // that were not yet created.
    //             if let Some(client) = instance.instance.client() {
    //                 new_instances.push(toplevel.create_handle(
    //                     comp.generation,
    //                     &instance.instance,
    //                     &comp.display,
    //                     &client,
    //                 ));
    //             }
    //         }

    //         // Describe the toplevel.
    //         for new in new_instances {
    //             toplevel.initialize_handle(&new);
    //         }

    //         return;
    //     }

    //     // If the surface is mapped (which it will be if it is not pending) and a buffer is no longer attached
    //     // then unmap the surface and return it to pending.
    //     if !has_buffer {
    //         if let Some(key) = comp
    //             .shell
    //             .toplevels
    //             .iter()
    //             .find_map(|(key, toplevel)| (toplevel.wl_surface().as_ref() == Some(surface)).then_some(*key))
    //         {
    //             match comp.shell.toplevels.entry(key) {
    //                 Entry::Occupied(entry) => {
    //                     // If the surface was never mapped assume a second initial commit was sent and apply
    //                     // the state.
    //                     let toplevel = entry.get();

    //                     if !matches!(toplevel.current, State::NotYetMapped) {
    //                         // TODO: Include app_id, remove toplevel debug impl
    //                         tracing::debug!(?toplevel, "Unmap toplevel");
    //                         let toplevel = entry.remove();

    //                         // Notify clients the toplevel is being unmapped.
    //                         for handle in toplevel.handles.values() {
    //                             handle.handle.closed();
    //                         }

    //                         match toplevel.surface {
    //                             Surface::Toplevel(surface) => comp.shell.pending_toplevels.push(surface),
    //                             Surface::XWayland(_) => todo!("How to handle xwayland?"),
    //                         }

    //                         return;
    //                     }
    //                 }

    //                 Entry::Vacant(_) => unreachable!("initial commit must have occurred"),
    //             }
    //         }
    //     }

    //     if let Some(toplevel) = comp
    //         .shell
    //         .toplevels
    //         .values_mut()
    //         .find(|toplevel| toplevel.wl_surface().as_ref() == Some(surface))
    //     {
    //         match &toplevel.surface {
    //             Surface::Toplevel(surface) => {
    //                 // Ensure the configure was acked before applying state.
    //                 if has_buffer && !surface.ensure_configured()  {
    //                     let id = toplevel.id;
    //                     let app_id = toplevel.app_id().unwrap_or_default();
    //                     tracing::warn!(%id, %app_id, "Killing client: toplevel not configured");
    //                 }

    //                 // Verify the toplevel's state is correct if some states were configured.
    //                 let current = surface.current_state();
    //                 let states = &current.states;
    //                 let _size = current.size;

    //                 if states.contains(xdg_toplevel::State::Maximized) {
    //                     // From xdg-shell:
    //                     // > The window geometry specified in the configure event must be obeyed by the client
    //                     //
    //                     // TODO: Check window geometry and compare to the committed size.
    //                 }

    //                 if states.contains(xdg_toplevel::State::Fullscreen) {
    //                     // From xdg-shell:
    //                     // > The window geometry specified in the configure event is a maximum; the client
    //                     // > cannot resize beyond it.
    //                     //
    //                     // This means the compositor can insert letterboxes if needed.
    //                     //
    //                     // TODO: Check that window geometry does not exceed the configured size.
    //                 }

    //                 if states.contains(xdg_toplevel::State::Resizing) {
    //                     // From xdg-shell:
    //                     // > The window geometry specified in the configure event is a maximum; the client
    //                     // > cannot resize beyond it. Clients that have aspect ratio or cell sizing configuration
    //                     // > can use a smaller size, however.
    //                     //
    //                     // TODO: Check that window geometry does not exceed the configured size.
    //                 }

    //                 // Activated and Tiled do not need to be checked here.

    //                 // TODO: Transaction setup
    //                 // FIXME: This is horrible.
    //                 let tree = comp.scene.create_surface_tree(surface.wl_surface().clone());
    //                 comp.scene.set_output_node(&comp.output, NodeIndex::SurfaceTree(tree));
    //                 send_frames_surface_tree(surface.wl_surface(), 0);
    //             }
    //             Surface::XWayland(_) => todo!("how to handle xwayland"),
    //         }
    //     }
    // }

    pub fn remove_toplevel(comp: &mut Aerugo, surface: &WlSurface) {
        // Remove toplevels that are pending
        comp.shell
            .pending_toplevels
            .retain(|toplevel| toplevel.wl_surface() != surface);

        if let Some(id) = comp.shell.toplevels.iter().find_map(|(key, toplevel)| {
            let remove = toplevel.wl_surface().as_ref() == Some(surface);
            remove.then_some(*key)
        }) {
            let toplevel = comp.shell.toplevels.remove(&id).unwrap();
            let app_id = toplevel.app_id();
            tracing::debug!(id, app_id, "Removed toplevel");
        }
    }

    pub fn get_state(&self, id: ToplevelId) -> Option<&Toplevel> {
        self.toplevels.get(&id)
    }

    pub fn get_state_mut(&mut self, id: ToplevelId) -> Option<&mut Toplevel> {
        self.toplevels.get_mut(&id)
    }
}

pub fn send_frames_surface_tree(surface: &WlSurface, time: u32) {
    compositor::with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            // the surface may not have any user_data if it is a subsurface and has not
            // yet been commited
            for callback in states
                .cached_state
                .current::<SurfaceAttributes>()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}
