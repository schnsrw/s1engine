//! Error types for the CRDT subsystem.

use crate::op_id::OpId;
use std::fmt;

/// Errors that can occur during CRDT operations.
#[derive(Debug, Clone, PartialEq)]
pub enum CrdtError {
    /// An operation arrived before its causal dependencies.
    CausalityViolation { op_id: OpId, missing_replica: u64 },
    /// An operation with this ID has already been applied.
    DuplicateOperation(OpId),
    /// The operation is structurally invalid.
    InvalidOperation(String),
    /// A referenced node was not found in the model.
    NodeNotFound(s1_model::NodeId),
    /// Error from the underlying operation layer.
    OperationError(s1_ops::OperationError),
    /// A cycle would be created by a move operation.
    CycleDetected {
        node_id: s1_model::NodeId,
        target_parent: s1_model::NodeId,
    },
    /// The text position referenced by an OpId was not found.
    TextPositionNotFound(OpId),
}

impl fmt::Display for CrdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CausalityViolation {
                op_id,
                missing_replica,
            } => write!(
                f,
                "Causality violation: op {op_id:?} depends on unseen ops from replica {missing_replica}"
            ),
            Self::DuplicateOperation(id) => write!(f, "Duplicate operation: {id:?}"),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {msg}"),
            Self::NodeNotFound(id) => write!(f, "Node not found: {id}"),
            Self::OperationError(e) => write!(f, "Operation error: {e}"),
            Self::CycleDetected {
                node_id,
                target_parent,
            } => write!(
                f,
                "Cycle detected: moving {node_id} under {target_parent} would create a cycle"
            ),
            Self::TextPositionNotFound(id) => {
                write!(f, "Text position not found for op: {id:?}")
            }
        }
    }
}

impl std::error::Error for CrdtError {}

impl From<s1_ops::OperationError> for CrdtError {
    fn from(e: s1_ops::OperationError) -> Self {
        Self::OperationError(e)
    }
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
