//! Dependency tracking
//!
//! This module provides the [`DependencyTracker`] type to help manage transaction dependencies.

use std::mem;

use slotmap::SlotMap;

slotmap::new_key_type! {
    pub struct Id;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    #[default]
    Queued,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotPresent,

    CausesCycle,
}

#[derive(Default)]
pub struct DependencyTracker {
    nodes: SlotMap<Id, Node>,
    failed: Vec<Id>,
    finished: Vec<Id>,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            failed: Vec::new(),
            finished: Vec::new(),
        }
    }

    pub fn get_status(&self, id: Id) -> Option<Status> {
        self.nodes.get(id).map(|t| t.status)
    }

    pub fn create_id(&mut self) -> Id {
        self.nodes.insert(Node::default())
    }

    /// Add a dependency to the specified node.
    ///
    /// Returns [`Err`] if adding the dependency would cause a cycle.
    pub fn add_dependency(&mut self, id: Id, dependency: Id) -> Result<Status, Error> {
        if !self.nodes.contains_key(id) || !self.nodes.contains_key(dependency) {
            return Err(Error::NotPresent);
        }

        if id == dependency {
            return Err(Error::CausesCycle);
        }

        // Does id appear in the dependency's dependencies?
        {
            // Use a stack to iterate without recursion.
            let mut stack = self
                .nodes
                .get(dependency)
                .unwrap()
                .dependencies
                .iter()
                .copied()
                .collect::<Vec<_>>();

            while !stack.is_empty() {
                for dependency in mem::take(&mut stack) {
                    if dependency == id {
                        return Err(Error::CausesCycle);
                    }

                    let node = self.nodes.get(dependency).unwrap();
                    stack.extend(node.dependencies.iter());
                }
            }
        }

        // Does the dependency appear in the id's dependents?
        {
            // Use a stack to iterate without recursion.
            let mut stack = self
                .nodes
                .get(id)
                .unwrap()
                .dependents
                .iter()
                .copied()
                .collect::<Vec<_>>();

            while !stack.is_empty() {
                for dependent in mem::take(&mut stack) {
                    if dependent == id {
                        return Err(Error::CausesCycle);
                    }

                    let node = self.nodes.get(dependent).unwrap();
                    stack.extend(node.dependents.iter());
                }
            }
        }

        let [node, dependency_node] = self.nodes.get_disjoint_mut([id, dependency]).unwrap();

        // If the dependency is finished there is no reason to add it.
        if dependency_node.status == Status::Finished {
            return Ok(Status::Queued);
        } else if dependency_node.status == Status::Queued {
            node.dependencies.push(dependency);
            dependency_node.dependents.push(id);
            return Ok(Status::Queued);
        }

        // The dependency failed, so propagate the failure to dependents.
        self.fail(id);

        Ok(Status::Failed)
    }

    /// Changes the node status to failed.
    ///
    /// If a node fails, all dependent nodes will also fail.
    ///
    /// The list of nodes that failed as a result of this call be be obtained using [`DependencyTracker::drain_failed`].
    pub fn fail(&mut self, id: Id) {
        if !self.nodes.contains_key(id) {
            return;
        };

        // Use a stack to iterate without recursion.
        let mut stack = vec![id];

        while !stack.is_empty() {
            for dependent in mem::take(&mut stack) {
                let node = self.nodes.get_mut(dependent).unwrap();
                stack.extend(node.dependents.iter());

                self.failed.push(dependent);
                node.status = Status::Failed;
            }
        }
    }

    #[must_use]
    pub fn drain_failed(&mut self) -> Vec<Id> {
        mem::take(&mut self.failed)
    }

    /// Changes the node status to finished.
    ///
    /// If a node finishes, the node is removed from the dependencies of the dependents.
    ///
    /// The list of nodes that have had all dependencies finished as a result of this call be be obtained using
    /// [`DependencyTracker::drain_finished`].
    pub fn finish(&mut self, id: Id) {
        if !self.nodes.contains_key(id) {
            return;
        }

        // Use a stack to iterate without recursion.
        let mut stack = vec![id];

        while !stack.is_empty() {
            for id in mem::take(&mut stack) {
                let node = self.nodes.get_mut(id).unwrap();

                // If the node has unfinished dependencies, skip it.
                if !node.dependencies.is_empty() {
                    continue;
                }

                // Remove the dependency from each dependent
                let dependents = node.dependents.clone();

                for dependent in dependents {
                    let node = self.nodes.get_mut(dependent).unwrap();
                    node.dependencies.retain(|&dependency| dependency != id);
                    // queue the dependent for processing
                    stack.push(dependent);
                }

                let node = self.nodes.get_mut(id).unwrap();
                node.status = Status::Finished;
                self.finished.push(id);
            }
        }
    }

    #[must_use]
    pub fn drain_finished(&mut self) -> Vec<Id> {
        mem::take(&mut self.finished)
    }
}

#[derive(Default)]
struct Node {
    dependents: Vec<Id>,
    dependencies: Vec<Id>,
    status: Status,
}

#[cfg(test)]
mod tests {
    use slotmap::KeyData;

    use crate::transaction::{Error, Status};

    use super::{DependencyTracker, Id};

    #[test]
    fn add_missing() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        // Create some dummy value that isn't in the internal slot map.
        let missing = Id::from(KeyData::from_ffi(u64::MAX));

        assert_eq!(tracker.add_dependency(a, missing), Err(Error::NotPresent));
        assert_eq!(tracker.add_dependency(missing, a), Err(Error::NotPresent));
    }

    /// ```text
    /// A -> A
    /// ```
    #[test]
    fn self_dependency() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        assert_eq!(tracker.add_dependency(a, a), Err(Error::CausesCycle));
    }

    /// ```text
    /// B -> A -> B
    /// ```
    #[test]
    fn cyclic_dependency() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        assert_eq!(tracker.add_dependency(a, b), Ok(Status::Queued));
        assert_eq!(tracker.add_dependency(b, a), Err(Error::CausesCycle));
    }

    /// ```text
    /// A
    /// ```
    #[test]
    fn fail_one() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();

        tracker.fail(a);

        assert_eq!(tracker.get_status(a), Some(Status::Failed));

        let failed = tracker.drain_failed();
        assert!(failed.contains(&a));
        assert_eq!(failed.len(), 1);
    }

    /// ```text
    /// C -> B -> A
    /// ```
    #[test]
    fn fail_chain() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        assert!(tracker.add_dependency(a, b).is_ok());
        assert!(tracker.add_dependency(b, c).is_ok());

        tracker.fail(c);
        assert_eq!(tracker.get_status(a), Some(Status::Failed));
        assert_eq!(tracker.get_status(b), Some(Status::Failed));
        assert_eq!(tracker.get_status(c), Some(Status::Failed));

        let failed = tracker.drain_failed();
        assert!(failed.contains(&a));
        assert!(failed.contains(&b));
        assert!(failed.contains(&c));
        assert_eq!(failed.len(), 3);
    }

    /// ```text
    /// B -\
    ///     -> A
    /// C -/
    /// ```
    #[test]
    fn fail_merge() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        assert!(tracker.add_dependency(a, b).is_ok());
        assert!(tracker.add_dependency(a, c).is_ok());

        // B failed, C should still be queued but A must fail.
        tracker.fail(b);
        assert_eq!(tracker.get_status(a), Some(Status::Failed));
        assert_eq!(tracker.get_status(b), Some(Status::Failed));
        assert_eq!(tracker.get_status(c), Some(Status::Queued));

        let failed = tracker.drain_failed();
        assert!(failed.contains(&a));
        assert!(failed.contains(&b));
        assert!(!failed.contains(&c));
        assert_eq!(failed.len(), 2);
    }

    /// ```text
    ///     /--> A
    /// C --
    ///     \--> B
    /// ```
    #[test]
    fn fail_branch() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        assert!(tracker.add_dependency(a, c).is_ok());
        assert!(tracker.add_dependency(b, c).is_ok());

        // C failed, A and B must also fail.
        tracker.fail(c);
        assert_eq!(tracker.get_status(a), Some(Status::Failed));
        assert_eq!(tracker.get_status(b), Some(Status::Failed));
        assert_eq!(tracker.get_status(c), Some(Status::Failed));

        let failed = tracker.drain_failed();
        assert!(failed.contains(&a));
        assert!(failed.contains(&b));
        assert!(failed.contains(&c));
        assert_eq!(failed.len(), 3);
    }

    /// ```text
    /// A
    /// ```
    #[test]
    fn finish_one() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();

        tracker.finish(a);
        assert_eq!(tracker.get_status(a), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&a));
        assert_eq!(finished.len(), 1);
    }

    /// ```text
    /// B -> A
    /// ```
    #[test]
    fn finish_chain() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();

        assert!(tracker.add_dependency(a, b).is_ok());

        tracker.finish(b);
        assert_eq!(tracker.get_status(a), Some(Status::Finished));
        assert_eq!(tracker.get_status(b), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&a));
        assert!(finished.contains(&b));
    }

    /// ```text
    ///     /--> A
    /// C --
    ///     \--> B
    /// ```
    #[test]
    fn finish_branch() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        assert!(tracker.add_dependency(a, c).is_ok());
        assert!(tracker.add_dependency(b, c).is_ok());

        tracker.finish(c);
        assert_eq!(tracker.get_status(a), Some(Status::Finished));
        assert_eq!(tracker.get_status(b), Some(Status::Finished));
        assert_eq!(tracker.get_status(c), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&a));
        assert!(finished.contains(&b));
        assert!(finished.contains(&c));
    }

    /// ```text
    /// B -\
    ///     -> A
    /// C -/
    /// ```
    #[test]
    fn finish_merge() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        assert!(tracker.add_dependency(a, b).is_ok());
        assert!(tracker.add_dependency(a, c).is_ok());

        // B finished, C and A should still be queued.
        tracker.finish(b);
        assert_eq!(tracker.get_status(b), Some(Status::Finished));

        assert_eq!(tracker.get_status(c), Some(Status::Queued));
        assert_eq!(tracker.get_status(a), Some(Status::Queued));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&b));
        assert!(!finished.contains(&a));
        assert!(!finished.contains(&c));
        assert_eq!(finished.len(), 1);

        // C finished, so A must also finish
        tracker.finish(c);
        assert_eq!(tracker.get_status(c), Some(Status::Finished));
        assert_eq!(tracker.get_status(a), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&a));
        assert!(finished.contains(&c));
        assert!(!finished.contains(&b));
        assert_eq!(finished.len(), 2);
    }

    /// ```text
    /// D -> B -\
    ///          -> A
    /// C ------/
    /// ```
    #[test]
    fn finish_merge_chain() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        let d = tracker.create_id();
        assert!(tracker.add_dependency(a, b).is_ok());
        assert!(tracker.add_dependency(b, d).is_ok());
        assert!(tracker.add_dependency(a, c).is_ok());

        // D finished, B should be finished A and C should still be queued.
        tracker.finish(d);
        assert_eq!(tracker.get_status(d), Some(Status::Finished));
        assert_eq!(tracker.get_status(b), Some(Status::Finished));

        assert_eq!(tracker.get_status(a), Some(Status::Queued));
        assert_eq!(tracker.get_status(c), Some(Status::Queued));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&b));
        assert!(finished.contains(&d));
        assert!(!finished.contains(&a));
        assert!(!finished.contains(&c));
        assert_eq!(finished.len(), 2);

        // B finished, C and A should still be queued.
        tracker.finish(b);
        assert_eq!(tracker.get_status(b), Some(Status::Finished));

        assert_eq!(tracker.get_status(c), Some(Status::Queued));
        assert_eq!(tracker.get_status(a), Some(Status::Queued));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&b));
        assert!(!finished.contains(&a));
        assert!(!finished.contains(&c));
        assert_eq!(finished.len(), 1);

        // C finished, so A must also finish
        tracker.finish(c);
        assert_eq!(tracker.get_status(c), Some(Status::Finished));
        assert_eq!(tracker.get_status(a), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&a));
        assert!(finished.contains(&c));
        assert!(!finished.contains(&b));
        assert_eq!(finished.len(), 2);
    }

    /// ```text
    /// D -\      /-> A
    ///     -> C -
    /// E -/      \-> B
    /// ```
    #[test]
    fn finish_merge_branch() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();
        let d = tracker.create_id();
        let e = tracker.create_id();

        assert!(tracker.add_dependency(c, d).is_ok());
        assert!(tracker.add_dependency(c, e).is_ok());
        assert!(tracker.add_dependency(a, c).is_ok());
        assert!(tracker.add_dependency(b, c).is_ok());

        // D finished, everything else should be queued.
        tracker.finish(d);
        assert_eq!(tracker.get_status(d), Some(Status::Finished));

        assert_eq!(tracker.get_status(a), Some(Status::Queued));
        assert_eq!(tracker.get_status(b), Some(Status::Queued));
        assert_eq!(tracker.get_status(c), Some(Status::Queued));
        assert_eq!(tracker.get_status(e), Some(Status::Queued));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&d));
        assert!(!finished.contains(&a));
        assert!(!finished.contains(&b));
        assert!(!finished.contains(&c));
        assert!(!finished.contains(&e));
        assert_eq!(finished.len(), 1);

        // E finished, everything should finish
        tracker.finish(e);
        assert_eq!(tracker.get_status(a), Some(Status::Finished));
        assert_eq!(tracker.get_status(b), Some(Status::Finished));
        assert_eq!(tracker.get_status(c), Some(Status::Finished));
        assert_eq!(tracker.get_status(e), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(!finished.contains(&d));
        assert!(finished.contains(&a));
        assert!(finished.contains(&b));
        assert!(finished.contains(&c));
        assert!(finished.contains(&e));
        assert_eq!(finished.len(), 4);
    }

    /// ```text
    /// C -> B -> A
    /// ```
    #[test]
    fn finish_middle() {
        let mut tracker = DependencyTracker::new();
        let a = tracker.create_id();
        let b = tracker.create_id();
        let c = tracker.create_id();

        assert!(tracker.add_dependency(a, b).is_ok());
        assert!(tracker.add_dependency(b, c).is_ok());

        // Middle node was finished, so nothing is finished
        tracker.finish(b);
        assert_eq!(tracker.get_status(a), Some(Status::Queued));
        assert_eq!(tracker.get_status(b), Some(Status::Queued));
        assert_eq!(tracker.get_status(c), Some(Status::Queued));

        let finished = dbg!(tracker.drain_finished());
        assert!(finished.is_empty());

        // C finished, so all should finish
        tracker.finish(c);
        assert_eq!(tracker.get_status(a), Some(Status::Finished));
        assert_eq!(tracker.get_status(b), Some(Status::Finished));
        assert_eq!(tracker.get_status(c), Some(Status::Finished));

        let finished = tracker.drain_finished();
        assert!(finished.contains(&a));
        assert!(finished.contains(&b));
        assert!(finished.contains(&c));
        assert_eq!(finished.len(), 3);
    }
}
