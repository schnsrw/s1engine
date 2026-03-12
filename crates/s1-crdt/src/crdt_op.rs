//! CRDT operation wrapper.
//!
//! [`CrdtOperation`] wraps an [`s1_ops::Operation`] with the causal metadata
//! needed for conflict-free replication: a unique [`OpId`], causal dependencies
//! via [`StateVector`], and CRDT-specific positioning hints.

use crate::op_id::OpId;
use crate::state_vector::StateVector;
use s1_ops::Operation;

/// A document operation wrapped with CRDT metadata for replication.
///
/// Each `CrdtOperation` is uniquely identified by its [`OpId`] and carries
/// causal dependency information in its [`StateVector`]. Text operations
/// additionally carry origin references for the Fugue-based text CRDT.
#[derive(Debug, Clone, PartialEq)]
pub struct CrdtOperation {
    /// Unique identifier for this operation.
    pub id: OpId,
    /// The causal dependencies: what operations this one has seen.
    pub deps: StateVector,
    /// The underlying document operation.
    pub operation: Operation,
    /// For text insert: the left neighbor at insertion time.
    pub origin_left: Option<OpId>,
    /// For text insert: the right neighbor at insertion time.
    pub origin_right: Option<OpId>,
    /// For tree insert/move: the operation that placed the parent.
    pub parent_op: Option<OpId>,
}

impl CrdtOperation {
    /// Create a new CRDT operation with just the required fields.
    pub fn new(id: OpId, deps: StateVector, operation: Operation) -> Self {
        Self {
            id,
            deps,
            operation,
            origin_left: None,
            origin_right: None,
            parent_op: None,
        }
    }

    /// Set text CRDT origin references (builder pattern).
    pub fn with_text_origins(mut self, left: Option<OpId>, right: Option<OpId>) -> Self {
        self.origin_left = left;
        self.origin_right = right;
        self
    }

    /// Set tree CRDT parent operation reference (builder pattern).
    pub fn with_parent_op(mut self, parent_op: Option<OpId>) -> Self {
        self.parent_op = parent_op;
        self
    }

    /// Returns the replica that issued this operation.
    pub fn replica(&self) -> u64 {
        self.id.replica
    }

    /// Returns the Lamport timestamp of this operation.
    pub fn lamport(&self) -> u64 {
        self.id.lamport
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::NodeId;

    #[test]
    fn create_crdt_op() {
        let id = OpId::new(1, 5);
        let deps = StateVector::new();
        let op = Operation::insert_text(NodeId::new(0, 3), 0, "hello");
        let crdt_op = CrdtOperation::new(id, deps.clone(), op);

        assert_eq!(crdt_op.id, id);
        assert_eq!(crdt_op.deps, deps);
        assert_eq!(crdt_op.replica(), 1);
        assert_eq!(crdt_op.lamport(), 5);
        assert!(crdt_op.origin_left.is_none());
        assert!(crdt_op.origin_right.is_none());
        assert!(crdt_op.parent_op.is_none());
    }

    #[test]
    fn crdt_op_with_text_origins() {
        let id = OpId::new(1, 5);
        let left = OpId::new(1, 3);
        let right = OpId::new(2, 4);
        let op = Operation::insert_text(NodeId::new(0, 3), 0, "x");
        let crdt_op = CrdtOperation::new(id, StateVector::new(), op)
            .with_text_origins(Some(left), Some(right));

        assert_eq!(crdt_op.origin_left, Some(left));
        assert_eq!(crdt_op.origin_right, Some(right));
    }

    #[test]
    fn crdt_op_with_parent_op() {
        let id = OpId::new(1, 5);
        let parent = OpId::new(1, 2);
        let op = Operation::insert_node(
            NodeId::new(0, 1),
            0,
            s1_model::Node::new(NodeId::new(1, 3), s1_model::NodeType::Paragraph),
        );
        let crdt_op = CrdtOperation::new(id, StateVector::new(), op).with_parent_op(Some(parent));

        assert_eq!(crdt_op.parent_op, Some(parent));
    }
}
