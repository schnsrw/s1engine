//! Error types for the CRDT subsystem.

use crate::op_id::OpId;

/// Errors that can occur during CRDT operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CrdtError {
    /// An operation arrived before its causal dependencies.
    #[error(
        "Causality violation: op {op_id:?} depends on unseen ops from replica {missing_replica}"
    )]
    CausalityViolation { op_id: OpId, missing_replica: u64 },
    /// An operation with this ID has already been applied.
    #[error("Duplicate operation: {0:?}")]
    DuplicateOperation(OpId),
    /// The operation is structurally invalid.
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    /// A referenced node was not found in the model.
    #[error("Node not found: {0}")]
    NodeNotFound(s1_model::NodeId),
    /// Error from the underlying operation layer.
    #[error("Operation error: {0}")]
    OperationError(#[from] s1_ops::OperationError),
    /// A cycle would be created by a move operation.
    #[error("Cycle detected: moving {node_id} under {target_parent} would create a cycle")]
    CycleDetected {
        node_id: s1_model::NodeId,
        target_parent: s1_model::NodeId,
    },
    /// The text position referenced by an OpId was not found.
    #[error("Text position not found for op: {0:?}")]
    TextPositionNotFound(OpId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = CrdtError::DuplicateOperation(OpId::new(1, 5));
        assert!(err.to_string().contains("Duplicate"));
    }

    #[test]
    fn error_from_operation_error() {
        let op_err = s1_ops::OperationError::Model(s1_model::ModelError::NodeNotFound(
            s1_model::NodeId::new(0, 99),
        ));
        let crdt_err: CrdtError = op_err.into();
        assert!(matches!(crdt_err, CrdtError::OperationError(_)));
    }
}
