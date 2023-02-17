use std::{borrow::Borrow, collections::hash_map::Entry};

use rustc_hash::FxHashMap;
use slotmap::{new_key_type, SlotMap};
use smithay::utils::{Physical, Point};
use wayland_server::{backend::ObjectId, protocol::wl_surface, Resource};

mod xdg_shell;
// TODO: XWayland
// TODO: Layer shell
// TODO: Aerugo shell implementation

#[allow(dead_code)]
// TODO: Remove when used

/*
TODO: Transactions

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
#[derive(Debug, Default)]
pub struct Scene {
    /// Storage of all surface nodes.
    ///
    /// A SlotMap is used for stable indices.
    surfaces: SlotMap<SurfaceIndex, SurfaceNode>,

    /// Mapping from surface (object id) to a node.
    surface_to_node: FxHashMap<ObjectId, SurfaceIndex>,

    /// Storage of all graph nodes.
    ///
    /// A SlotMap is used for stable indices.
    graphs: SlotMap<GraphIndex, GraphNode>,
}

impl Scene {
    pub fn commit(&mut self, surface: &wl_surface::WlSurface) {
        // If the surface is not known, create a node for the surface.
        let entry = self.surface_to_node.entry(surface.id());
        let _surface_index = entry.or_insert_with(|| {
            let index = self.surfaces.insert_with_key(|index| SurfaceNode {
                index,
                surface: surface.clone(),
                offset: Point::default(),
                relations: NodeRelations::default(),
            });

            index
        });

        // TODO: Lower subsurface tree into the surface nodes
    }

    pub fn surface_destroyed(&mut self, surface: &wl_surface::WlSurface) {
        let Entry::Occupied(entry) = self.surface_to_node.entry(surface.id()) else {
            return;
        };

        let index = entry.remove();
        let _node = self.surfaces.remove(index).unwrap();

        todo!("graph rearrangement")
    }

    pub fn create_graph_node(&mut self) -> &mut GraphNode {
        let index = self.graphs.insert_with_key(|index| GraphNode {
            index,
            offset: Point::default(),
            relations: NodeRelations::default(),
        });

        self.get_graph_mut(index).expect("impossible to reach: just created")
    }

    pub fn destroy_graph_node(&mut self, index: GraphIndex) {
        let Some(_node) = self.graphs.remove(index) else {
            return
        };

        todo!("graph rearrangement")
    }

    pub fn get_with_surface(&self, surface: &wl_surface::WlSurface) -> Option<&SurfaceNode> {
        let index = self.surface_to_node.get(&surface.id())?;
        self.get_surface(*index)
    }

    pub fn get_with_surface_mut(&mut self, surface: &wl_surface::WlSurface) -> Option<&mut SurfaceNode> {
        let index = self.surface_to_node.get(&surface.id())?;
        self.get_surface_mut(*index)
    }

    pub fn get_surface(&self, index: SurfaceIndex) -> Option<&SurfaceNode> {
        self.surfaces.get(index)
    }

    pub fn get_surface_mut(&mut self, index: SurfaceIndex) -> Option<&mut SurfaceNode> {
        self.surfaces.get_mut(index)
    }

    pub fn get_graph(&self, index: GraphIndex) -> Option<&GraphNode> {
        self.graphs.get(index)
    }

    pub fn get_graph_mut(&mut self, index: GraphIndex) -> Option<&mut GraphNode> {
        self.graphs.get_mut(index)
    }
}

new_key_type! {
    /// A stable index to reference a [`SurfaceNode`].
    pub struct SurfaceIndex;

    /// A stable index to reference a [`GraphNode`]
    pub struct GraphIndex;
}

/// A stable index to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeIndex {
    /// The index references a surface.
    Surface(SurfaceIndex),

    /// The index references a graph.
    Graph(GraphIndex),
}

impl From<SurfaceIndex> for NodeIndex {
    fn from(value: SurfaceIndex) -> Self {
        Self::Surface(value)
    }
}

impl From<GraphIndex> for NodeIndex {
    fn from(value: GraphIndex) -> Self {
        Self::Graph(value)
    }
}

#[derive(Debug)]
pub struct SurfaceNode {
    index: SurfaceIndex,

    surface: wl_surface::WlSurface,

    /// Offset of this surface relative to a parent.
    offset: Point<i32, Physical>,

    relations: NodeRelations<NodeIndex, NodeIndex, SurfaceIndex>,
}

impl SurfaceNode {
    /// The index of this surface.
    ///
    /// This may be used to reference this surface in other nodes, such as a graph node.
    pub fn index(&self) -> SurfaceIndex {
        self.index
    }

    pub fn offset(&self) -> Point<i32, Physical> {
        self.offset
    }

    /// The underlying `wl_surface` of this node.
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }
}

impl Borrow<SurfaceIndex> for SurfaceNode {
    fn borrow(&self) -> &SurfaceIndex {
        &self.index
    }
}

#[derive(Debug)]
pub struct GraphNode {
    index: GraphIndex,

    /// Offset of this surface relative to a parent.
    offset: Point<i32, Physical>,

    relations: NodeRelations<GraphIndex, NodeIndex, NodeIndex>,
}

impl GraphNode {
    /// The index of this graph node.
    ///
    /// This may be used to reference this graph in other graph nodes or as the graph to be presented.
    pub fn index(&self) -> GraphIndex {
        self.index
    }

    pub fn offset(&self) -> Point<i32, Physical> {
        self.offset
    }
}

impl Borrow<GraphIndex> for GraphNode {
    fn borrow(&self) -> &GraphIndex {
        &self.index
    }
}

#[derive(Debug)]
struct NodeRelations<Parent, Sibling, Child> {
    /// Parent of this node.
    parent: Option<Parent>,

    /// The previous sibling of this node.
    ///
    /// If this is [`None`] but `next_sibling` is [`Some`], then this is the first child of the parent.
    prev_sibling: Option<Sibling>,

    /// The next sibling of this node.
    ///
    /// If this is [`None`] but `prev_sibling` is [`Some`], then this is the last child node of the parent.
    next_sibling: Option<Sibling>,

    /// First child of this node.
    first_child: Option<Child>,

    /// Last child of this node.
    last_child: Option<Child>,
}

// manual implementation of Default is needed since #[derive(Default)] requires Parent, Sibling and Child to
// also be Default.
impl<Parent, Sibling, Child> Default for NodeRelations<Parent, Sibling, Child> {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            prev_sibling: Default::default(),
            next_sibling: Default::default(),
            first_child: Default::default(),
            last_child: Default::default(),
        }
    }
}
