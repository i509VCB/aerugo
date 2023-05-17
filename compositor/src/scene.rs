//! The Aerugo scene graph
//!
//! TODO: Documentation

use std::ops::{Deref, DerefMut};

use rustc_hash::FxHashMap;
use smithay::{
    backend::renderer::{
        element::{AsRenderElements, Element, Id, RenderElement, UnderlyingStorage},
        utils::{CommitCounter, RendererSurfaceStateUserData},
        Frame, ImportAll, Renderer,
    },
    output::Output,
    utils::{Buffer, Physical, Point, Rectangle, Scale, Transform},
    wayland::compositor,
};
use wayland_server::{backend::ObjectId, protocol::wl_surface, Resource};

use crate::forest::{Error, Forest, Index};

/// A stable index to reference an [`OutputNode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputIndex(Index);

/// A stable index to reference an [`SurfaceTreeNode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceTreeIndex(Index);

/// A stable index to reference a [`SurfaceNode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceIndex(Index);

/// A stable index to reference a [`BranchNode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BranchIndex(Index);

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
    top: SurfaceIndex,
    /// The offset of the root surface from the parent.
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

    /// The top surface is the subsurface at the with the highest z-index in a subsurface tree.
    ///
    /// If there are no subsurfaces above the root surface, then this will be the same as the root surface.
    pub fn top(&self) -> SurfaceIndex {
        self.top
    }
}

#[derive(Debug)]
pub struct SurfaceNode {
    index: SurfaceIndex,
    surface: wl_surface::WlSurface,
    offset: Point<i32, Physical>,
}

#[derive(Debug)]
pub struct BranchNode {
    index: BranchIndex,
    offset: Point<i32, Physical>,
}

#[derive(Debug)]
pub struct Scene {
    outputs: FxHashMap<Output, OutputIndex>,
    surface_trees: FxHashMap<ObjectId, SurfaceTreeIndex>,
    surfaces: FxHashMap<ObjectId, SurfaceIndex>,
    forest: Forest<SceneNode>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            outputs: FxHashMap::default(),
            surface_trees: FxHashMap::default(),
            surfaces: FxHashMap::default(),
            forest: Forest::new(),
        }
    }

    pub fn create_output(&mut self, output: Output) -> OutputIndex {
        let index = OutputIndex(self.forest.insert_with(|index| {
            SceneNode::Output(OutputNode {
                index: OutputIndex(index),
                output: output.clone(),
                present: None,
            })
        }));

        self.outputs.insert(output, index);
        index
    }

    pub fn destroy_output(&mut self, output: &Output) {
        // Disassociating the output from child surfaces needs to occur before we destroy the node.
        self.unset_output_root(output);

        if let Some(OutputIndex(index)) = self.outputs.remove(output) {
            let _ = self.forest.remove(index);
        }
    }

    pub fn get_output_index(&self, output: &Output) -> Option<OutputIndex> {
        self.outputs.get(output).cloned()
    }

    pub fn get_output(&self, index: OutputIndex) -> Option<&OutputNode> {
        self.forest.get(index.0).map(|node| match node.deref() {
            SceneNode::Output(node) => node,
            _ => unreachable!(),
        })
    }

    pub fn get_output_mut(&mut self, index: OutputIndex) -> Option<&mut OutputNode> {
        self.forest.get_mut(index.0).map(|node| match node.deref_mut() {
            SceneNode::Output(node) => node,
            _ => unreachable!(),
        })
    }

    pub fn set_output_node(&mut self, output: &Output, node: NodeIndex) {
        self.unset_output_root(output);

        if let Some(index) = self.get_output_index(output) {
            let output_node = self.get_output_mut(index).unwrap();
            output_node.present = Some(node);
        }

        // TODO: Send enter and exit events
    }

    pub fn get_surface_tree_index(&self, surface: wl_surface::WlSurface) -> Option<SurfaceTreeIndex> {
        self.surface_trees.get(&surface.id()).cloned()
    }

    pub fn get_surface_tree(&mut self, index: SurfaceTreeIndex) -> Option<&mut SurfaceTreeNode> {
        self.forest.get_mut(index.0).map(|node| match node.deref_mut() {
            SceneNode::SurfaceTree(node) => node,
            _ => unreachable!(),
        })
    }

    pub fn create_surface_tree(&mut self, surface: wl_surface::WlSurface) -> SurfaceTreeIndex {
        // Create the surface node for this surface.
        let root = SurfaceIndex(self.forest.insert_with(|index| {
            SceneNode::Surface(SurfaceNode {
                index: SurfaceIndex(index),
                surface: surface.clone(),
                offset: Default::default(),
            })
        }));

        let index = SurfaceTreeIndex(self.forest.insert_with(|index| {
            SceneNode::SurfaceTree(SurfaceTreeNode {
                index: SurfaceTreeIndex(index),
                root,
                base: root,
                top: root,
                offset: Default::default(),
            })
        }));

        self.forest.add_child(index.0, root.0).unwrap();

        // Initialize the surface tree
        self.apply_surface_commit(&surface);
        index
    }

    pub fn get_surface_index(&self, surface: wl_surface::WlSurface) -> Option<SurfaceIndex> {
        self.surfaces.get(&surface.id()).cloned()
    }

    pub fn get_surface(&mut self, index: SurfaceIndex) -> Option<&mut SurfaceNode> {
        self.forest.get_mut(index.0).map(|node| match node.deref_mut() {
            SceneNode::Surface(node) => node,
            _ => unreachable!(),
        })
    }

    /// Applies the new surface state to the scene graph.
    ///
    /// If the surface has any subsurfaces, the subsurfaces will be adjusted.
    pub fn apply_surface_commit(&mut self, _surface: &wl_surface::WlSurface) {
        // TODO: Do we need a commit state to apply since we are transaction based?
    }

    // TODO: Surface destroyed (for both tree and surface)

    pub fn create_branch(&mut self) -> BranchIndex {
        BranchIndex(self.forest.insert_with(|index| {
            SceneNode::Branch(BranchNode {
                index: BranchIndex(index),
                offset: (0, 0).into(),
            })
        }))
    }

    pub fn get_branch(&mut self, index: BranchIndex) -> Option<&mut BranchNode> {
        self.forest.get_mut(index.0).map(|node| match node.deref_mut() {
            SceneNode::Branch(node) => node,
            _ => unreachable!(),
        })
    }

    pub fn branch_add_child(&mut self, branch: BranchIndex, index: NodeIndex) -> Result<(), Error> {
        self.forest.add_child(branch.into(), index.into())
    }

    pub fn destroy_branch(&mut self, index: BranchIndex) {
        let _ = self.forest.remove(index.into());
    }

    /// Sets the offset of the node relative to it's parent.
    pub fn set_node_offset(&mut self, index: NodeIndex, offset: Point<i32, Physical>) {
        match index {
            NodeIndex::SurfaceTree(index) => {
                if let Some(surface_tree) = self.get_surface_tree(index) {
                    surface_tree.offset = offset;
                }
            }

            NodeIndex::Branch(index) => {
                if let Some(branch) = self.get_branch(index) {
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

    pub fn get_graph(&self, output: &Output) -> Option<Hierarchy<'_>> {
        let output = self.get_output_index(output)?;
        let output = self.get_output(output).unwrap();
        Some(Hierarchy {
            scene: self,
            root: output.present?,
        })
    }

    /// Unsets the node which is the output root and sends leave events.
    fn unset_output_root(&mut self, output: &Output) {
        if let Some(index) = self.get_output_index(output) {
            let node = self.get_output(index).unwrap();

            if let Some(_root) = node.present {
                // TODO: Send leave events
            }
        }
    }
}

pub struct SceneGraphElement {
    id: Id,
    surface: wl_surface::WlSurface,
}

impl SceneGraphElement {}

impl Element for SceneGraphElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        compositor::with_states(&self.surface, |states| {
            let data = states.data_map.get::<RendererSurfaceStateUserData>();
            data.map(|d| d.borrow().current_commit())
        })
        .unwrap_or_default()
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        compositor::with_states(&self.surface, |states| {
            let data = states.data_map.get::<RendererSurfaceStateUserData>();
            if let Some(data) = data {
                let data = data.borrow();

                if let Some(view) = data.view() {
                    Some(view.src.to_buffer(
                        // TODO: Do not hardcode these
                        1.0,
                        Transform::Normal,
                        &data.buffer_size().unwrap().to_f64(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_default()
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let size = compositor::with_states(&self.surface, |states| {
            let data = states.data_map.get::<RendererSurfaceStateUserData>();
            data.and_then(|d| d.borrow().view()).map(|surface_view| {
                (surface_view.dst.to_f64().to_physical(1.0).to_point())
                    .to_i32_round()
                    .to_size()
            })
        })
        .unwrap_or_default();

        Rectangle::from_loc_and_size((0, 0), size)
    }
}

impl<R: Renderer + ImportAll> RenderElement<R> for SceneGraphElement
where
    R::TextureId: 'static,
{
    fn draw<'a>(
        &self,
        frame: &mut R::Frame<'a>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        compositor::with_states(&self.surface, |states| {
            let data = states.data_map.get::<RendererSurfaceStateUserData>();
            if let Some(data) = data {
                let data = data.borrow();

                if let Some(texture) = data.texture::<R>(frame.id()) {
                    // TODO: data.buffer_transform is private
                    frame.render_texture_from_to(texture, src, dst, damage, Transform::Normal, 1.0f32)?;
                } else {
                    dbg!("Not available");
                    // warn!("trying to render texture from different renderer");
                }
            }

            Ok(())
        })
    }

    fn underlying_storage(&self, _renderer: &mut R) -> Option<UnderlyingStorage> {
        compositor::with_states(&self.surface, |states| {
            let data = states.data_map.get::<RendererSurfaceStateUserData>();
            data.and_then(|d| d.borrow().buffer().cloned())
                .map(UnderlyingStorage::Wayland)
        })
    }
}

pub struct Hierarchy<'scene> {
    scene: &'scene Scene,
    root: NodeIndex,
}

impl<R: Renderer + ImportAll> AsRenderElements<R> for Hierarchy<'_>
where
    R::TextureId: 'static,
{
    type RenderElement = SceneGraphElement;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        _location: Point<i32, Physical>,
        _scale: Scale<f64>,
        _alpha: f32,
    ) -> Vec<C> {
        let Some(iter) = self.scene.forest.dfs_descend(self.root.into()) else {
            return Vec::new();
        };

        // Determine the final offset of the indices because smithay expects the render elements top to bottom.
        let final_offset: Point<i32, Physical> = iter.clone().fold((0, 0).into(), |mut offset, index| {
            match self.scene.forest.get(index).unwrap().deref() {
                SceneNode::Output(_) => unreachable!(),
                SceneNode::SurfaceTree(node) => offset += node.offset,
                SceneNode::Surface(node) => offset += node.offset,
                SceneNode::Branch(node) => offset += node.offset,
            }

            offset
        });

        // Collect all the surfaces, subtracting from the final offset to get the expected offset.
        let indices = iter.collect::<Vec<_>>();

        let mut offset = final_offset;
        indices
            .iter()
            .rev()
            .filter_map(|&index| {
                let node = self.scene.forest.get(index)?;

                match node.deref() {
                    SceneNode::Output(_) => unreachable!(),
                    SceneNode::SurfaceTree(node) => {
                        offset -= node.offset;
                        None
                    }

                    SceneNode::Surface(node) => {
                        smithay::backend::renderer::utils::import_surface_tree(renderer, &node.surface)
                            .expect("Failed to import");

                        let elem = SceneGraphElement {
                            id: Id::from_wayland_resource(&node.surface),
                            surface: node.surface.clone(),
                        };

                        offset -= node.offset;
                        Some(elem)
                    }

                    SceneNode::Branch(node) => {
                        offset -= node.offset;
                        None
                    }
                }
            })
            .map(C::from)
            .collect()
    }
}

#[derive(Debug)]
enum SceneNode {
    Output(OutputNode),
    SurfaceTree(SurfaceTreeNode),
    Surface(SurfaceNode),
    Branch(BranchNode),
}

impl From<BranchIndex> for Index {
    fn from(value: BranchIndex) -> Self {
        value.0
    }
}

impl From<SurfaceTreeIndex> for Index {
    fn from(value: SurfaceTreeIndex) -> Self {
        value.0
    }
}

impl From<SurfaceIndex> for Index {
    fn from(value: SurfaceIndex) -> Self {
        value.0
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
