use std::collections::{hash_map::Entry, HashMap};

use slotmap::{new_key_type, SlotMap};
use smithay::{
    output::Output,
    utils::{Physical, Point},
    wayland::{compositor, shell::xdg::ToplevelSurface},
};
use wayland_server::{backend::ObjectId, protocol::wl_surface, Resource};

mod xdg_shell;
// TODO: XWayland
// TODO: Layer shell
// TODO: Aerugo shell implementation

// TODO: Surfaces should be independently displayable from the graph. i.e. a surface can be in multiple graphs.

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

I guess I will need to seperate the scene graph used for rendering and then the WM state.

*/

// TODO: Maybe better to call it a scene graph? This is the representation post XDG shell and XWayland.
#[derive(Debug, Default)]
pub struct Shell {
    /// Shell graph nodes.
    ///
    /// A slotmap is used for stable keys and to reuse heap allocations that represent the nodes in the graph.
    nodes: SlotMap<NodeIndex, Node>,

    /// Outputs mapped in the shell.
    ///
    /// The value of the map is a node index, which references a node in the scene graph.
    outputs: HashMap<Output, NodeIndex>,

    /// Mapping from surface (object id) to a node.
    surface_to_node: HashMap<ObjectId, NodeIndex>,
}

impl Shell {
    pub fn commit(&mut self, surface: &wl_surface::WlSurface) {
        // If the surface is not known, create a node for the surface.
        let entry = self.surface_to_node.entry(surface.id());
        let _surface_index = entry.or_insert_with(|| {
            let index = self.nodes.insert_with_key(|index| Node {
                index,
                surface: surface.clone(),
                parent: None,
                children: Vec::new(),
            });

            index
        });
    }

    pub fn surface_destroyed(&mut self, surface: &wl_surface::WlSurface) {
        if let Entry::Occupied(entry) = self.surface_to_node.entry(surface.id()) {
            let index = entry.remove();
            let node = self.nodes.remove(index).unwrap();

            // Remove the node from it's parents and children
            if let Some(parent) = node.parent {
                if let Some(parent) = self.nodes.get_mut(parent) {
                    parent.children.retain(|node| node.index == index);
                }
            }

            for child in node.children.iter() {
                if let Some(child) = self.nodes.get_mut(child.index) {
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

    pub fn node(&self, surface: &wl_surface::WlSurface) -> Option<&Node> {
        let index = self.surface_to_node.get(&surface.id())?;
        self.nodes.get(*index)
    }

    pub fn node_mut(&mut self, surface: &wl_surface::WlSurface) -> Option<&mut Node> {
        let index = self.surface_to_node.get(&surface.id())?;
        self.nodes.get_mut(*index)
    }
}

new_key_type! { pub struct NodeIndex; }

#[derive(Debug)]
pub struct Node {
    index: NodeIndex,
    // TODO: Surface, etc...
    // TODO: Should an output be a valid node?
    //       That would allow for things for floating outputs which contain windows, but that is much more
    //       complicated. It would also be useful then to have a way to scale a surface for such a use.
    /// The surface which represents the contents of the node.
    surface: wl_surface::WlSurface,

    /// Parent of this node.
    parent: Option<NodeIndex>,

    /// Children of this node.
    children: Vec<ChildNode>,
}

impl Node {
    pub fn index(&self) -> NodeIndex {
        self.index
    }

    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }


    // TODO: Get surface tree of node?
    //       The other option would be to put the tree into the graph on behalf of the user and let the
    //       compositor manage the surface tree of subsurfacesw.
}

#[derive(Debug)]
struct ChildNode {
    index: NodeIndex,
    offset: Point<i32, Physical>,
}
