use std::ops::{Deref, DerefMut};

use slotmap::{new_key_type, SlotMap};

/// An error from using a [`Forest`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0:?} is not present in the forest")]
    NotPresent(Index),

    #[error("failed to insert because the forest would become cyclic")]
    Cycle,
}

#[derive(Debug)]
pub struct Forest<T> {
    inner: SlotMap<Index, Node<T>>,
}

impl<T> Forest<T> {
    pub fn new() -> Self {
        Self {
            inner: SlotMap::with_key(),
        }
    }

    /// Inserts a value into the forest, returning the index of the value.
    ///
    /// The value when inserted does not have any child or parent nodes.
    pub fn insert(&mut self, value: T) -> Index {
        self.insert_with(|_| value)
    }

    pub fn insert_with<F>(&mut self, f: F) -> Index
    where
        F: FnOnce(Index) -> T,
    {
        self.inner.insert_with_key(|index| Node {
            value: f(index),
            index,
            parent: None,
            prev: None,
            next: None,
            first_last_child: None,
        })
    }

    pub fn get(&self, index: Index) -> Option<&Node<T>> {
        self.inner.get(index)
    }

    pub fn get_mut(&mut self, index: Index) -> Option<&mut Node<T>> {
        self.inner.get_mut(index)
    }

    pub fn contains_index(&self, index: Index) -> bool {
        self.inner.contains_key(index)
    }

    /// Removes the index from the forest, returning the value stored with the index.
    pub fn remove(&mut self, index: Index) -> Result<T, Error> {
        // Detach the node before removing from the map.
        self.detach(index)?;
        // TODO: Detach children from the node.

        let node = self.inner.remove(index).unwrap();
        Ok(node.value)
    }

    /// Adds makes the `child` a child of the `index`.
    pub fn add_child(&mut self, index: Index, child: Index) -> Result<(), Error> {
        self.is_present(index)?;
        self.is_present(child)?;
        self.check_for_cycles(index, child)?;

        let parent = self.get_mut(index).unwrap();

        match parent.first_last_child {
            // Create a triangle relationship:
            //
            //         a
            //     /   |   \
            //    /    |    \
            // ... <-> x <-> y
            //
            // where x is the previous last child of the parent
            // and y is the next last child of the parent.
            Some((first_child, prev_last_child)) => {
                // Set the next last child of the parent node.
                parent.first_last_child.replace((first_child, child));

                // Update the previous child, ensuring it's next sibling is set to the new, next sibling.
                let prev_last_child_node = self.get_mut(prev_last_child).unwrap();
                prev_last_child_node.next = Some(child);

                // Update the child node, which is now the last child of the parent.
                let child_node = self.get_mut(child).unwrap();
                child_node.prev = Some(prev_last_child);
                child_node.parent = Some(index);
            }

            // If the parent is receiving it's first child node, a lot of code can be skipped.
            None => {
                parent.first_last_child = Some((child, child));

                let parent = parent.index;
                let child = self.get_mut(child).unwrap();
                child.parent = Some(parent);
            }
        }

        Ok(())
    }

    /// Detaches the node from it's parent and siblings.
    ///
    /// The children of the node are not detached.
    pub fn detach(&mut self, index: Index) -> Result<(), Error> {
        self.is_present(index)?;

        let node = self.get_mut(index).unwrap();
        let parent = Node::parent(node);
        node.parent.take();

        let prev_sibling = Node::prev_sibling(node);
        let next_sibling = Node::next_sibling(node);

        match (prev_sibling, next_sibling) {
            // If this node is the only child of it's parent we need to fully detach the parent.
            (None, None) => {
                if let Some(parent) = parent {
                    let node = self.get_mut(parent).unwrap();
                    node.first_last_child.take();
                }
            }

            // This node is the first child of the parent
            (None, Some(next)) => {
                if let Some(parent) = parent {
                    let node = self.get_mut(parent).unwrap();
                    let last_child = Node::last_child(node).unwrap();
                    node.first_last_child = Some((next, last_child))
                }
            }

            // This node is the last child of the parent
            (Some(prev), None) => {
                if let Some(parent) = parent {
                    let node = self.get_mut(parent).unwrap();
                    let first_child = Node::first_child(node).unwrap();
                    node.first_last_child = Some((first_child, prev))
                }
            }

            (Some(prev), Some(next)) => {
                // Detach the previous and next siblings, relinking those as needed.
                if let Some(prev) = self.get_mut(prev) {
                    prev.next = next_sibling;
                }

                if let Some(next) = self.get_mut(next) {
                    next.prev = prev_sibling;
                }
            }
        }

        Ok(())
    }

    pub fn preorder_traverse(&self, index: Index) -> Option<PreorderTraverse<'_, T>> {
        if !self.contains_index(index) {
            return None;
        }

        Some(PreorderTraverse {
            forest: self,
            root: index,
            next: Some(Edge::Start(index)),
        })
    }

    pub fn dfs_descend(&self, index: Index) -> Option<DfsDescend<'_, T>> {
        self.preorder_traverse(index).map(DfsDescend)
    }

    pub fn previous_siblings(&self, index: Index) -> Option<PreviousSiblings<'_, T>> {
        if !self.contains_index(index) {
            return None;
        }

        Some(PreviousSiblings {
            forest: self,
            next: Some(index),
        })
    }

    pub fn next_siblings(&self, index: Index) -> Option<NextSiblings<'_, T>> {
        if !self.contains_index(index) {
            return None;
        }

        Some(NextSiblings {
            forest: self,
            next: Some(index),
        })
    }

    pub fn children(&self, index: Index) -> Children<'_, T> {
        let (first_child, last_child) = self
            .get(index)
            .map(|node| (Node::first_child(node), Node::last_child(node)))
            .unzip();

        Children {
            forest: self,
            next: first_child.flatten(),
            last: last_child.flatten(),
        }
    }

    // TODO: Relation related methods
    // - Raise/lower node as child

    fn is_present(&self, index: Index) -> Result<(), Error> {
        if !self.contains_index(index) {
            return Err(Error::NotPresent(index));
        }

        Ok(())
    }

    fn check_for_cycles(&self, index: Index, inserting: Index) -> Result<(), Error> {
        // 1. If the two nodes are the same, then a cycle is guaranteed.
        if index == inserting {
            return Err(Error::Cycle);
        }

        // 2. If the node being inserted has no parents, siblings or children a cycle is impossible.
        let inserting_node = self.get(inserting).expect("Node needs to be present");

        if inserting_node.parent.is_none()
            && inserting_node.prev.is_none()
            && inserting_node.next.is_none()
            && inserting_node.first_last_child.is_none()
        {
            return Ok(());
        }

        // 3. Ensure the parent and child do not become cyclic.
        let parent_node = self.get(index).expect("Node needs to be present");

        if parent_node.parent == Some(inserting) {
            return Err(Error::Cycle);
        }

        // 4. Make sure the node being inserted does not appear in the parent's child hierarchy
        if self.dfs_descend(index).unwrap().any(|index| index == inserting) {
            return Err(Error::Cycle);
        }

        // 5. TODO: Make sure the node being inserted does not appear in the parent's parents and siblings.

        Ok(())
    }
}

new_key_type! {
    /// The index to a value in a [`Forest`].
    ///
    /// This type should be considered as raw, meaning that specific node types are given a special index type
    /// that wraps an instance of [`Index`].
    pub struct Index;
}

#[derive(Debug)]
pub struct Node<T> {
    value: T,
    index: Index,
    parent: Option<Index>,
    prev: Option<Index>,
    next: Option<Index>,
    /// The first and last children of the node (first, last).
    first_last_child: Option<(Index, Index)>,
}

impl<T> Node<T> {
    /// ```
    /// use aerugo_comp::forest::{Forest, Node};
    ///
    /// let mut forest = Forest::new();
    /// let index = forest.insert(());
    ///
    /// let node = forest.get(index).unwrap();
    /// assert_eq!(index, Node::index(node));
    /// ```
    pub fn index(self_: &Self) -> Index {
        self_.index
    }

    pub fn parent(self_: &Self) -> Option<Index> {
        self_.parent
    }

    pub fn prev_sibling(self_: &Self) -> Option<Index> {
        self_.prev
    }

    pub fn next_sibling(self_: &Self) -> Option<Index> {
        self_.next
    }

    pub fn first_child(self_: &Self) -> Option<Index> {
        self_.first_last_child.map(|(first, _)| first)
    }

    pub fn last_child(self_: &Self) -> Option<Index> {
        self_.first_last_child.map(|(_, last)| last)
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

/// A type to indicate if an index representing a node is starting or ending on a tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The start of a node.
    ///
    /// This may be treated like pushing a node onto a stack.
    Start(Index),

    /// The end of a node.
    ///
    /// This may be treated like popping a node off a stack.
    End(Index),
}

/// An pre-order depth first iterator over nodes in a [`Forest`].
///
/// This iterator yields some [`Edge`].
pub struct PreorderTraverse<'f, T> {
    forest: &'f Forest<T>,
    root: Index,
    next: Option<Edge>,
}

impl<T> PreorderTraverse<'_, T> {
    fn next_node(&self, next: Edge) -> Option<Edge> {
        match next {
            // A node was pushed onto the stack, meaning we can try to go further down the current branch.
            Edge::Start(index) => {
                // Check if there is any child node to further visit.
                match Node::first_child(self.forest.get(index).unwrap()) {
                    Some(first_child) => Some(Edge::Start(first_child)),
                    None => Some(Edge::End(index)),
                }
            }

            // If we are popping the node off the pseudo-stack, then try to select the next sibling if possible.
            Edge::End(index) => {
                // If the root is encountered then then the traversal is complete.
                if index == self.root {
                    return None;
                }

                let node = self.forest.get(index).unwrap();

                match Node::next_sibling(node) {
                    Some(next_sibling) => Some(Edge::Start(next_sibling)),
                    None => node.parent.map(Edge::End),
                }
            }
        }
    }
}

impl<T> Iterator for PreorderTraverse<'_, T> {
    type Item = Edge;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next.take()?;
        self.next = self.next_node(next);
        Some(next)
    }
}

impl<T> Clone for PreorderTraverse<'_, T> {
    fn clone(&self) -> Self {
        Self {
            forest: self.forest,
            root: self.root,
            next: self.next,
        }
    }
}

pub struct DfsDescend<'f, T>(PreorderTraverse<'f, T>);

impl<T> Iterator for DfsDescend<'_, T> {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.find_map(|edge| {
            match edge {
                Edge::Start(index) => Some(index),
                // Continue popping nodes off the stack.
                Edge::End(_) => None,
            }
        })
    }
}

impl<T> Clone for DfsDescend<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Clone)]
pub struct PreviousSiblings<'f, T> {
    forest: &'f Forest<T>,
    next: Option<Index>,
}

impl<T> Iterator for PreviousSiblings<'_, T> {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next.take()?;
        let node = self.forest.get(next).unwrap();
        self.next = Node::prev_sibling(node);
        Some(next)
    }
}

#[derive(Clone)]
pub struct NextSiblings<'f, T> {
    forest: &'f Forest<T>,
    next: Option<Index>,
}

impl<T> Iterator for NextSiblings<'_, T> {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next.take()?;
        let node = self.forest.get(next).unwrap();
        self.next = Node::next_sibling(node);
        Some(next)
    }
}

#[derive(Clone)]
pub struct Children<'f, T> {
    forest: &'f Forest<T>,
    next: Option<Index>,
    last: Option<Index>,
}

impl<T> Iterator for Children<'_, T> {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next.take()?;

        if Some(next) != self.last {
            self.next = Node::next_sibling(self.forest.get(next).unwrap());
        }

        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use crate::forest::Edge;

    use super::{Error, Forest, Node};

    /// Ensure a node cannot become it's own child.
    #[test]
    fn self_cyclic_node() {
        let mut forest = Forest::new();
        let a = forest.insert(());

        assert!(matches!(forest.add_child(a, a), Err(Error::Cycle)));
    }

    /// Ensure a node does not form a parent-child loop.
    #[test]
    fn parent_cycle() {
        let mut forest = Forest::new();
        let a = forest.insert(());
        let b = forest.insert(());
        // a -> b
        forest.add_child(a, b).unwrap();
        assert!(matches!(forest.add_child(b, a), Err(Error::Cycle)));
    }

    /// a -> b -> c
    #[test]
    fn preorder_traverse_line() {
        let mut forest = Forest::new();
        let a = forest.insert(0);
        let b = forest.insert(1);
        let c = forest.insert(2);

        forest.add_child(a, b).unwrap();
        forest.add_child(b, c).unwrap();

        let mut iter = forest.preorder_traverse(a).unwrap();
        assert_eq!(iter.next(), Some(Edge::Start(a)));
        assert_eq!(iter.next(), Some(Edge::Start(b)));
        assert_eq!(iter.next(), Some(Edge::Start(c)));
        assert_eq!(iter.next(), Some(Edge::End(c)));
        assert_eq!(iter.next(), Some(Edge::End(b)));
        assert_eq!(iter.next(), Some(Edge::End(a)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn triangle() {
        let mut forest = Forest::new();
        let a = forest.insert(0);
        let b = forest.insert(1);
        let c = forest.insert(2);

        //    a
        //  /   \
        // b <-> c
        forest.add_child(a, b).unwrap();
        forest.add_child(a, c).unwrap();

        let node_a = forest.get(a).unwrap();
        // a has two children, b and c. As such the first child should be b and last child c.
        assert_eq!(Node::first_child(node_a), Some(b));
        assert_eq!(Node::last_child(node_a), Some(c));

        // b's next sibling should be c.
        let node_b = forest.get(b).unwrap();
        assert_eq!(Node::parent(node_b), Some(a));
        assert_eq!(Node::prev_sibling(node_b), None);
        assert_eq!(Node::next_sibling(node_b), Some(c));

        // c's previous sibling should be b.
        let node_c = forest.get(c).unwrap();
        assert_eq!(Node::parent(node_b), Some(a));
        assert_eq!(Node::prev_sibling(node_c), Some(b));
        assert_eq!(Node::next_sibling(node_c), None);

        let mut iter = forest.preorder_traverse(a).unwrap();
        assert_eq!(iter.next(), Some(Edge::Start(a)));
        assert_eq!(iter.next(), Some(Edge::Start(b)));
        assert_eq!(iter.next(), Some(Edge::End(b)));
        assert_eq!(iter.next(), Some(Edge::Start(c)));
        assert_eq!(iter.next(), Some(Edge::End(c)));
        assert_eq!(iter.next(), Some(Edge::End(a)));
        assert_eq!(iter.next(), None);

        let mut prev_siblings = forest.previous_siblings(c).unwrap();
        assert_eq!(prev_siblings.next(), Some(c));
        assert_eq!(prev_siblings.next(), Some(b));
        assert_eq!(prev_siblings.next(), None);

        let mut next_siblings = forest.next_siblings(b).unwrap();
        assert_eq!(next_siblings.next(), Some(b));
        assert_eq!(next_siblings.next(), Some(c));
        assert_eq!(next_siblings.next(), None);

        let mut children = forest.children(a);
        assert_eq!(children.next(), Some(b));
        assert_eq!(children.next(), Some(c));
        assert_eq!(children.next(), None);
    }
}
