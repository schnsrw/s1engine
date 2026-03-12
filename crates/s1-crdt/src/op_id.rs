//! Operation identity for CRDT mutations.
//!
//! [`OpId`] uniquely identifies a single mutation event. Unlike [`NodeId`](s1_model::NodeId)
//! which identifies a node in the tree, an OpId identifies the *action* that created
//! or modified nodes. A single operation can affect many nodes.
//!
//! Total ordering: Lamport timestamp first, replica ID for tiebreaking.

use std::fmt;

/// A globally unique identifier for a CRDT operation.
///
/// Total ordering is defined as: compare by `lamport` first, then by `replica`
/// for tiebreaking. This guarantees a deterministic total order that is
/// consistent with causal ordering.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpId {
    /// The replica that issued this operation.
    pub replica: u64,
    /// The Lamport timestamp at the time of the operation.
    pub lamport: u64,
}

impl OpId {
    /// Create a new operation ID.
    pub const fn new(replica: u64, lamport: u64) -> Self {
        Self { replica, lamport }
    }
}

impl Ord for OpId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.lamport
            .cmp(&other.lamport)
            .then(self.replica.cmp(&other.replica))
    }
}

impl PartialOrd for OpId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for OpId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpId({}, {})", self.replica, self.lamport)
    }
}

impl fmt::Display for OpId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "op({}, {})", self.replica, self.lamport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_id_equality() {
        let a = OpId::new(1, 5);
        let b = OpId::new(1, 5);
        let c = OpId::new(2, 5);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn op_id_ordering_by_lamport() {
        let a = OpId::new(1, 3);
        let b = OpId::new(1, 5);
        assert!(a < b);
    }

    #[test]
    fn op_id_ordering_tiebreak_by_replica() {
        let a = OpId::new(1, 5);
        let b = OpId::new(2, 5);
        assert!(a < b);
    }

    #[test]
    fn op_id_total_order() {
        let mut ids = vec![
            OpId::new(2, 3),
            OpId::new(1, 1),
            OpId::new(1, 3),
            OpId::new(3, 2),
        ];
        ids.sort();
        assert_eq!(
            ids,
            vec![
                OpId::new(1, 1),
                OpId::new(3, 2),
                OpId::new(1, 3),
                OpId::new(2, 3),
            ]
        );
    }

    #[test]
    fn op_id_display() {
        let id = OpId::new(3, 42);
        assert_eq!(format!("{id}"), "op(3, 42)");
        assert_eq!(format!("{id:?}"), "OpId(3, 42)");
    }

    #[test]
    fn op_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(OpId::new(1, 1));
        set.insert(OpId::new(1, 1)); // duplicate
        set.insert(OpId::new(2, 1));
        assert_eq!(set.len(), 2);
    }
}
