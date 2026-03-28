//! Unique ID system for document nodes (CRDT-ready).
//!
//! Every node gets a globally unique [`NodeId`] composed of `(replica_id, counter)`.
//! For single-user mode, `replica_id` is always `0`. When CRDT collaboration is
//! enabled, each user gets a unique `replica_id`, ensuring IDs never collide.

use std::fmt;

/// A globally unique identifier for a document node.
///
/// Composed of a replica (site) ID and a monotonically increasing counter.
/// This design is compatible with Yjs, Automerge, and Diamond Types CRDT schemes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId {
    /// Replica/site identifier. `0` for single-user mode.
    pub replica: u64,
    /// Monotonically increasing counter per replica.
    pub counter: u64,
}

impl NodeId {
    /// The root document node ID.
    pub const ROOT: NodeId = NodeId {
        replica: 0,
        counter: 0,
    };

    /// Create a new NodeId.
    pub const fn new(replica: u64, counter: u64) -> Self {
        Self { replica, counter }
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({}, {})", self.replica, self.counter)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.replica, self.counter)
    }
}

/// Generates unique [`NodeId`]s for a given replica.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct IdGenerator {
    replica: u64,
    counter: u64,
}

impl IdGenerator {
    /// Create a new generator for the given replica ID.
    /// Counter starts at 1 (0 is reserved for ROOT).
    pub fn new(replica: u64) -> Self {
        Self {
            replica,
            counter: 1,
        }
    }

    /// Generate the next unique ID.
    pub fn next_id(&mut self) -> NodeId {
        let id = NodeId::new(self.replica, self.counter);
        self.counter += 1;
        id
    }

    /// Get the replica ID.
    pub fn replica(&self) -> u64 {
        self.replica
    }

    /// Get the current counter value (next ID to be issued).
    pub fn counter(&self) -> u64 {
        self.counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_id() {
        assert_eq!(NodeId::ROOT, NodeId::new(0, 0));
    }

    #[test]
    fn id_generator_sequential() {
        let mut gen = IdGenerator::new(0);
        let a = gen.next_id();
        let b = gen.next_id();
        let c = gen.next_id();
        assert_eq!(a, NodeId::new(0, 1));
        assert_eq!(b, NodeId::new(0, 2));
        assert_eq!(c, NodeId::new(0, 3));
    }

    #[test]
    fn id_generator_different_replicas() {
        let mut gen_a = IdGenerator::new(1);
        let mut gen_b = IdGenerator::new(2);
        let a = gen_a.next_id();
        let b = gen_b.next_id();
        assert_ne!(a, b);
        assert_eq!(a, NodeId::new(1, 1));
        assert_eq!(b, NodeId::new(2, 1));
    }

    #[test]
    fn id_ordering() {
        let a = NodeId::new(0, 1);
        let b = NodeId::new(0, 2);
        let c = NodeId::new(1, 1);
        assert!(a < b);
        assert!(b < c); // replica 1 > replica 0
    }

    #[test]
    fn id_display() {
        let id = NodeId::new(3, 42);
        assert_eq!(format!("{id}"), "(3, 42)");
        assert_eq!(format!("{id:?}"), "NodeId(3, 42)");
    }
}
