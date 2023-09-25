// fn window<'a>(decorations: &'a Decorations, toplevel: &'a ToplevelId) -> Graph<'a> {
//     container((
//         decorations,
//         toplevel.offset(0, decorations.height()),
//     ))
// }

use std::iter::Chain;

use euclid::{UnknownUnit, Transform2D, Transform3D};

struct Shader;

struct Node;

impl Node {}

struct NodeUpdate;

impl NodeUpdate {
    pub fn new(serial: ConfigureSerial) -> Self {
        Self
    }

    pub fn crop(&mut self, rect: euclid::Rect<u32, UnknownUnit>) -> &mut Self {
        self
    }

    pub fn shader(&mut self, shader: Shader) -> &mut Self {
        self
    }

    // TODO: How do I actually want to describe this?
    pub fn shader_data(&mut self, data: &[u8]) -> &mut Self {
        self
    }

    pub fn transform(&mut self, transform: Transform3D<f32, UnknownUnit, UnknownUnit>) -> &mut Self {
        self
    }
}

struct Entry<'a> {
    serial: Option<ConfigureSerial>,
    node: &'a Node,
}

#[derive(Default)]
struct Tree<'a> {
    nodes: Vec<Entry<'a>>,
    trees: Vec<Tree<'a>>,
}

impl<'a> Tree<'a> {
    fn append_tree(&mut self, tree: Tree<'a>) -> &mut Self {
        self
    }

    fn append(&mut self, node: &'a Node) -> &mut Self {
        self
    }

    fn append_with_serial(&mut self, node: &'a Node, serial: ConfigureSerial) -> &mut Self {
        self
    }
}

pub struct Decorations(Node);

pub struct Toplevel(Node);

#[derive(Clone)]
struct ConfigureSerial;

// idea 2: barriers, have the tree only apply changes when configures finish
fn window<'a>(decorations: &'a Node, toplevel: &'a Node, pending: ConfigureSerial) -> Tree<'a> {
    let mut tree = Tree::default();
    tree.append(decorations)
        .append_with_serial(toplevel, pending);
    tree
}

fn main() {

}
