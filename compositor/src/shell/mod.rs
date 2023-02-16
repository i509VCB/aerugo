use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
};

use slotmap::{new_key_type, SlotMap};
use smithay::{
    output::Output,
    utils::{Physical, Point},
};
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
pub struct Graph {
    /// Storage of all surface nodes.
    ///
    /// A SlotMap is used for stable indices.
    surfaces: SlotMap<SurfaceIndex, SurfaceNode>,

    /// Mapping from surface (object id) to a node.
    surface_to_node: HashMap<ObjectId, SurfaceIndex, ahash::RandomState>,

    /// Storage of all surface nodes.
    ///
    /// A SlotMap is used for stable indices.
    graphs: SlotMap<GraphIndex, GraphNode>,

    /// Outputs which have a bound node.
    outputs: HashMap<Output, OutputInfo>,
}

impl Graph {
    pub fn commit(&mut self, surface: &wl_surface::WlSurface) {
        // If the surface is not known, create a node for the surface.
        let entry = self.surface_to_node.entry(surface.id());
        let _surface_index = entry.or_insert_with(|| {
            let index = self.surfaces.insert_with_key(|index| SurfaceNode {
                index,
                surface: surface.clone(),
                parent: None,
                children: Vec::new(),
            });

            index
        });

        // TODO: Lower subsurface tree into the surface nodes
    }

    pub fn surface_destroyed(&mut self, surface: &wl_surface::WlSurface) {
        if let Entry::Occupied(entry) = self.surface_to_node.entry(surface.id()) {
            let index = entry.remove();
            let node = self.surfaces.remove(index).unwrap();

            // Remove the node from it's parents and children
            if let Some(parent) = node.parent {
                match parent {
                    NodeIndex::Surface(parent) => {
                        if let Some(parent) = self.surfaces.get_mut(parent) {
                            parent.children.retain(|node| node.index == index);
                        }
                    }

                    NodeIndex::Graph(parent) => {
                        if let Some(parent) = self.graphs.get_mut(parent) {
                            parent.children.retain(
                                // bizzare rustfmt output...
                                |&ChildNode {
                                     index: parent_index, ..
                                 }| { parent_index != NodeIndex::Surface(index) },
                            );
                        }
                    }
                }
            }

            for child in node.children.iter() {
                if let Some(child) = self.surfaces.get_mut(child.index) {
                    child.parent.take();
                }
            }
        }
    }

    // TODO: map_output

    /// Outputs mapped in the shell.
    pub fn outputs(&self) -> impl ExactSizeIterator<Item = &Output> {
        self.outputs.keys()
    }

    // TODO: unmap_output

    // TODO: Traverse nodes mapped in an output.

    pub fn get_surface(&self, surface: &wl_surface::WlSurface) -> Option<&SurfaceNode> {
        let index = self.surface_to_node.get(&surface.id())?;
        self.surfaces.get(*index)
    }

    pub fn get_surface_mut(&mut self, surface: &wl_surface::WlSurface) -> Option<&mut SurfaceNode> {
        let index = self.surface_to_node.get(&surface.id())?;
        self.surfaces.get_mut(*index)
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

    /// The parent node of this surface.
    parent: Option<NodeIndex>,

    /// Subsurface children of this surface.
    ///
    /// A surface can only have subsurfaces as children.
    ///
    /// Although wl_surface already provides a way to get the subsurfaces, the indices of the subsurfaces
    /// are tracked to allow quickly building a graph.
    children: Vec<ChildNode<SurfaceIndex>>,
}

impl SurfaceNode {
    /// The index of this surface.
    ///
    /// This may be used to reference this surface in other nodes, such as a graph node.
    pub fn index(&self) -> SurfaceIndex {
        self.index
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

    /// A graph can only have another graph as it's parent.
    parent: Option<GraphIndex>,

    /// Children of this graph.
    children: Vec<ChildNode<NodeIndex>>,
}

impl GraphNode {
    /// The index of this graph node.
    ///
    /// This may be used to reference this graph in other graph nodes or as the graph to be presented.
    pub fn index(&self) -> GraphIndex {
        self.index
    }
}

impl Borrow<GraphIndex> for GraphNode {
    fn borrow(&self) -> &GraphIndex {
        &self.index
    }
}

#[derive(Debug)]
struct ChildNode<Index> {
    index: Index,

    /// The transform relative to the parent node.
    ///
    /// If there is no parent, then this is relative to the origin of the global coordinate space.
    transform: Point<i32, Physical>,
}

#[derive(Debug)]
struct OutputInfo {
    /// Index of the root of the graph to be presented
    ///
    /// This may be a single surface if in full screen, or a graph node for compositing.
    index: NodeIndex,
    // TODO: Damage?
}
