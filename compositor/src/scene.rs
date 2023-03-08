//! The Aerugo scene graph
//!
//! TODO: Documentation

use rustc_hash::FxHashMap;
use slotmap::{new_key_type, SlotMap};
use smithay::{
    output::Output,
    utils::{Physical, Point},
};
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
    relations: Relation,
    offset: Point<i32, Physical>,
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
    relations: Relation,
    // TODO: Offset from parent?
}

#[derive(Debug)]
pub struct BranchNode {
    index: BranchIndex,
    relations: Relation,
    offset: Point<i32, Physical>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeIndex {
    SurfaceTree(SurfaceTreeIndex),
    Branch(BranchIndex),
}

impl PartialEq<SurfaceTreeIndex> for NodeIndex {
    fn eq(&self, other: &SurfaceTreeIndex) -> bool {
        Self::SurfaceTree(*other) == *self
    }
}

impl PartialEq<BranchIndex> for NodeIndex {
    fn eq(&self, other: &BranchIndex) -> bool {
        Self::Branch(*other) == *self
    }
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
    pub fn create_output(&mut self, output: Output) -> OutputIndex {
        let index = self.outputs.insert_with_key(|index| OutputNode {
            index,
            output: output.clone(),
            present: None,
        });

        self.output_to_node.insert(output, index);
        index
    }

    pub fn destroy_output(&mut self, output: &Output) {
        // Disassoicating the output from child surfaces needs to occur before we destroy the node.
        self.unset_output_root(output);

        if let Some(index) = self.output_to_node.remove(output) {
            let _ = self.outputs.remove(index);
        }
    }

    pub fn set_output_node(&mut self, output: &Output, node: NodeIndex) {
        self.unset_output_root(output);

        if let Some(index) = self.get_output_index(output) {
            let output_node = self.outputs.get_mut(index).unwrap();
            output_node.present = Some(node);
        }

        // TODO: Set the node and cause all child surfaces to enter the output.
    }

    pub fn get_output_index(&self, output: &Output) -> Option<OutputIndex> {
        self.output_to_node.get(output).cloned()
    }

    // TODO: Set node for presentation on output

    // TODO: Create surface tree

    pub fn get_surface_tree_index(&self, surface: wl_surface::WlSurface) -> Option<SurfaceTreeIndex> {
        self.surface_tree_to_node.get(&surface.id()).cloned()
    }

    // TODO: Handle surface tree commit

    pub fn get_surface_index(&self, surface: wl_surface::WlSurface) -> Option<SurfaceIndex> {
        self.surface_to_node.get(&surface.id()).cloned()
    }

    // TODO: Surface destroyed (for both tree and surface)

    pub fn create_branch(&mut self) -> BranchIndex {
        self.branches.insert_with_key(|index| BranchNode {
            index,
            relations: Relation::default(),
            offset: (0, 0).into(),
        })
    }

    pub fn branch_add_child(&mut self, branch: BranchIndex, index: NodeIndex) {
        if !self.is_node_index_valid(index) {
            todo!()
        }

        let Some(branch) = self.branches.get_mut(branch) else {
            return;
        };

        // Detect if adding the child node will result in a cycle.
        // First make sure we aren't make this branch a child of itself
        if index == branch.index {
            todo!()
        }

        // Next check if there is a path from the branch to the new child node.
    }

    pub fn destroy_branch(&mut self, index: BranchIndex) {
        if let Some(branch) = self.branches.remove(index) {
            let parent = branch.relations.parent;
            let parent = parent.map(|index| match index {
                Index::Branch(index) => index,
                _ => unreachable!("The parent of a branch can only be a branch"),
            });

            // TODO: Unparent the child nodes
        }
    }

    /// Sets the offset of the node relative to it's parent.
    pub fn set_node_offset(&mut self, index: NodeIndex, offset: Point<i32, Physical>) {
        match index {
            NodeIndex::SurfaceTree(index) => {
                if let Some(surface_tree) = self.surface_trees.get_mut(index) {
                    surface_tree.offset = offset;
                }
            }

            NodeIndex::Branch(index) => {
                if let Some(branch) = self.branches.get_mut(index) {
                    branch.offset = offset;
                }
            }
        }
    }

    /// Raise the node one node higher relative to the parent.
    ///
    /// This will cause the node to farther above the parent.
    pub fn raise_node(&mut self, index: NodeIndex) {
        todo!()
    }

    /// Raise the node to become child node placed highest above the parent.
    pub fn raise_node_to_top(&mut self, index: NodeIndex) {
        todo!()
    }

    /// Lower the node one node relative to other children of it's parent.
    ///
    /// This will cause the node to be closer but still above the parent node.
    pub fn lower_node(&mut self, index: NodeIndex) {
        todo!()
    }

    /// Lower the node to be the lowest node above it's parent.
    pub fn lower_node_to_bottom(&mut self, index: NodeIndex) {
        todo!()
    }

    /// Unsets the node which is the output root and sends leave events.
    fn unset_output_root(&mut self, output: &Output) {
        if let Some(index) = self.get_output_index(output) {
            let node = self.outputs.get_mut(index).unwrap();

            if let Some(_root) = node.present {
                // TODO: Send leave events
            }
        }
    }

    fn is_node_index_valid(&self, index: NodeIndex) -> bool {
        match index {
            NodeIndex::SurfaceTree(index) => self.surface_trees.contains_key(index),
            NodeIndex::Branch(index) => self.branches.contains_key(index),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Index {
    Branch(BranchIndex),
    SurfaceTree(SurfaceTreeIndex),
    Surface(SurfaceIndex),
}

impl From<BranchIndex> for Index {
    fn from(value: BranchIndex) -> Self {
        Self::Branch(value)
    }
}

impl From<SurfaceTreeIndex> for Index {
    fn from(value: SurfaceTreeIndex) -> Self {
        Self::SurfaceTree(value)
    }
}

impl From<SurfaceIndex> for Index {
    fn from(value: SurfaceIndex) -> Self {
        Self::Surface(value)
    }
}

impl From<NodeIndex> for Index {
    fn from(value: NodeIndex) -> Self {
        match value {
            NodeIndex::SurfaceTree(index) => index.into(),
            NodeIndex::Branch(index) => index.into(),
        }
    }
}

#[derive(Debug, Default)]
struct Relation {
    /// Parent of this node.
    parent: Option<Index>,

    /// The previous sibling of this node.
    ///
    /// If this is [`None`] but `next_sibling` is [`Some`], then this is the first child of the parent.
    prev_sibling: Option<Index>,

    /// The next sibling of this node.
    ///
    /// If this is [`None`] but `prev_sibling` is [`Some`], then this is the last child node of the parent.
    next_sibling: Option<Index>,

    /// The number of child nodes.
    child_count: u32,

    /// First child of this node.
    first_child: Option<Index>,
}
