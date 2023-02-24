use rustc_hash::FxHashMap;
use slotmap::{new_key_type, SlotMap};
use smithay::output::Output;
use wayland_server::{backend::ObjectId, protocol::wl_surface, Resource};

new_key_type! {
    /// A stable index to reference an [`OutputNode`].
    pub struct OutputIndex;

    /// A stable index to reference a [`SurfaceTreeNode`].
    pub struct SurfaceTreeIndex;

    /// A stable index to reference a [`SurfaceNode`].
    pub struct SurfaceIndex;

    /// A stable index to reference a [`BranchNode`]
    pub struct BranchIndex;
}

#[derive(Debug)]
pub struct OutputNode {
    index: OutputIndex,
    output: Output,
    present: Option<NodeIndex>,
}

impl OutputNode {
    pub fn index(&self) -> OutputIndex {
        self.index
    }

    pub fn output(&self) -> &Output {
        &self.output
    }
}

/// A node for a surface and it's subsurface tree.
#[derive(Debug)]
pub struct SurfaceTreeNode {
    index: SurfaceTreeIndex,
    root: SurfaceIndex,
    base: SurfaceIndex,
    // TODO: Node relations
}

impl SurfaceTreeNode {
    pub fn index(&self) -> SurfaceTreeIndex {
        self.index
    }

    /// The root surface is the parent of all subsurfaces in this subsurface tree.
    ///
    /// The root surface will typically be a surface with a role such as `xdg-toplevel`, `layer-shell` and
    /// `xwayland`.
    pub fn root(&self) -> SurfaceIndex {
        self.root
    }

    /// The base surface is the subsurface at the with the lowest z-index in a subsurface tree.
    ///
    /// If there are no subsurfaces below the root surface, then this will be the same as the root surface.
    pub fn base(&self) -> SurfaceIndex {
        self.base
    }
}

#[derive(Debug)]
pub struct SurfaceNode {
    index: SurfaceIndex,
    surface: wl_surface::WlSurface,
    // TODO: Node relations
}

#[derive(Debug)]
pub struct BranchNode {
    index: BranchIndex,
    // TODO: Node relations
}

#[derive(Debug)]
pub enum NodeIndex {
    Surface(SurfaceTreeIndex),
    Branch(BranchIndex),
}

#[derive(Debug, Default)]
pub struct Scene {
    output_to_node: FxHashMap<Output, OutputIndex>,
    outputs: SlotMap<OutputIndex, OutputNode>,
    surface_tree_to_node: FxHashMap<ObjectId, SurfaceTreeIndex>,
    surface_trees: SlotMap<SurfaceTreeIndex, SurfaceTreeNode>,
    surface_to_node: FxHashMap<ObjectId, SurfaceIndex>,
    surfaces: SlotMap<SurfaceIndex, SurfaceNode>,
    branches: SlotMap<BranchIndex, BranchNode>,
}

impl Scene {
    pub fn create_output(&mut self, output: Output) -> &mut OutputNode {
        let index = self.outputs.insert_with_key(|index| OutputNode {
            index,
            output: output.clone(),
            present: None,
        });

        self.output_to_node.insert(output, index);
        self.outputs.get_mut(index).expect("just created")
    }

    pub fn destroy_output(&mut self, output: &Output) {
        if let Some(index) = self.output_to_node.remove(output) {
            let node = self
                .outputs
                .remove(index)
                .expect("index was tracked, so the node must also be");

            // TODO: Traverse the tree, removing the surfaces from the output?
            // TODO: Remove this output as the parent of the node it will present.
            todo!()
        }
    }

    pub fn get_output_index(&self, output: &Output) -> Option<OutputIndex> {
        self.output_to_node.get(output).cloned()
    }

    pub fn get_output(&self, index: OutputIndex) -> Option<&OutputNode> {
        self.outputs.get(index)
    }

    pub fn get_output_mut(&mut self, index: OutputIndex) -> Option<&mut OutputNode> {
        self.outputs.get_mut(index)
    }

    // TODO: Set node for presentation on output

    // TODO: Create surface tree

    pub fn get_surface_tree_index(&self, surface: wl_surface::WlSurface) -> Option<SurfaceTreeIndex> {
        self.surface_tree_to_node.get(&surface.id()).cloned()
    }

    pub fn get_surface_tree(&self, index: SurfaceTreeIndex) -> Option<&SurfaceTreeNode> {
        self.surface_trees.get(index)
    }

    pub fn get_surface_tree_mut(&mut self, index: SurfaceTreeIndex) -> Option<&mut SurfaceTreeNode> {
        self.surface_trees.get_mut(index)
    }

    // TODO: Handle surface tree commit

    pub fn get_surface_index(&self, surface: wl_surface::WlSurface) -> Option<SurfaceIndex> {
        self.surface_to_node.get(&surface.id()).cloned()
    }

    pub fn get_surface(&self, index: SurfaceIndex) -> Option<&SurfaceNode> {
        self.surfaces.get(index)
    }

    pub fn get_surface_mut(&mut self, index: SurfaceIndex) -> Option<&mut SurfaceNode> {
        self.surfaces.get_mut(index)
    }

    // TODO: Surface destroyed (for both tree and surface)

    pub fn create_branch(&mut self) -> &mut BranchNode {
        let index = self.branches.insert_with_key(|index| BranchNode { index });

        self.branches.get_mut(index).unwrap()
    }

    // TODO: Destroy branch

    pub fn get_branch(&self, index: BranchIndex) -> Option<&BranchNode> {
        self.branches.get(index)
    }

    pub fn get_branch_mut(&mut self, index: BranchIndex) -> Option<&mut BranchNode> {
        self.branches.get_mut(index)
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
