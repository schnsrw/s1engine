//! Transaction — groups multiple operations into a single undo step.
//!
//! A transaction is the user-visible unit of change. While [`Operation`] is the
//! atomic unit, transactions group related operations so they are applied and
//! undone together. For example, "make selection bold" might produce multiple
//! `SetAttributes` operations, but they appear as one undo step.

use crate::operation::{apply, Operation, OperationError};
use s1_model::DocumentModel;

/// A transaction groups multiple operations so they are applied/undone together.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    /// Human-readable label for the transaction (e.g., "Bold text").
    pub label: Option<String>,
    /// The operations in this transaction (in application order).
    pub operations: Vec<Operation>,
}

impl Transaction {
    /// Create an empty transaction.
    pub fn new() -> Self {
        Self {
            label: None,
            operations: Vec::new(),
        }
    }

    /// Create a transaction with a label.
    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            operations: Vec::new(),
        }
    }

    /// Add an operation to the transaction.
    pub fn push(&mut self, op: Operation) {
        self.operations.push(op);
    }

    /// Returns `true` if the transaction has no operations.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Number of operations.
    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply a transaction to the model. Returns the inverse transaction for undo.
///
/// If any operation fails, previously applied operations are rolled back.
pub fn apply_transaction(
    model: &mut DocumentModel,
    txn: &Transaction,
) -> Result<Transaction, OperationError> {
    let mut inverses = Vec::new();

    for op in &txn.operations {
        match apply(model, op) {
            Ok(inverse) => inverses.push(inverse),
            Err(e) => {
                // Rollback: apply collected inverses in reverse order.
                // Note: rollback errors are intentionally ignored — if rollback fails,
                // the document may be in an inconsistent state. This is a best-effort
                // recovery; the original error is reported to the caller.
                for inv in inverses.into_iter().rev() {
                    let _ = apply(model, &inv);
                }
                return Err(e);
            }
        }
    }

    // Inverse transaction has operations in reverse order
    inverses.reverse();
    Ok(Transaction {
        label: txn.label.clone(),
        operations: inverses,
    })
}

/// A builder for constructing transactions fluently.
pub struct TransactionBuilder {
    txn: Transaction,
}

impl TransactionBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            txn: Transaction::new(),
        }
    }

    /// Set the transaction label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.txn.label = Some(label.into());
        self
    }

    /// Add an operation.
    pub fn push(mut self, op: Operation) -> Self {
        self.txn.push(op);
        self
    }

    /// Build the transaction.
    pub fn build(self) -> Transaction {
        self.txn
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{Node, NodeId, NodeType};

    #[test]
    fn empty_transaction() {
        let txn = Transaction::new();
        assert!(txn.is_empty());
        assert_eq!(txn.len(), 0);
        assert!(txn.label.is_none());
    }

    #[test]
    fn transaction_with_label() {
        let txn = Transaction::with_label("Bold text");
        assert_eq!(txn.label.as_deref(), Some("Bold text"));
    }

    #[test]
    fn apply_single_op_transaction() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();

        let mut txn = Transaction::with_label("Add paragraph");
        txn.push(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ));

        let inverse = apply_transaction(&mut doc, &txn).unwrap();
        assert!(doc.node(para_id).is_some());
        assert_eq!(inverse.len(), 1);

        // Undo
        apply_transaction(&mut doc, &inverse).unwrap();
        assert!(doc.node(para_id).is_none());
    }

    #[test]
    fn apply_multi_op_transaction() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let txn = TransactionBuilder::new()
            .label("Add paragraph with text")
            .push(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .push(Operation::insert_node(
                para_id,
                0,
                Node::new(run_id, NodeType::Run),
            ))
            .push(Operation::insert_node(
                run_id,
                0,
                Node::text(text_id, "Hello"),
            ))
            .build();

        let inverse = apply_transaction(&mut doc, &txn).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello")
        );
        assert_eq!(inverse.len(), 3);

        // Undo: removes text, run, paragraph (inverse order)
        apply_transaction(&mut doc, &inverse).unwrap();
        assert!(doc.node(para_id).is_none());
        assert!(doc.node(run_id).is_none());
        assert!(doc.node(text_id).is_none());
    }

    #[test]
    fn transaction_rollback_on_failure() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();

        // First op: valid insert
        // Second op: invalid (Run directly under Body)
        let txn = TransactionBuilder::new()
            .push(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .push(Operation::insert_node(
                body_id,
                1,
                Node::new(NodeId::new(0, 99), NodeType::Run),
            ))
            .build();

        let result = apply_transaction(&mut doc, &txn);
        assert!(result.is_err());

        // The first op should have been rolled back
        assert!(doc.node(para_id).is_none());
    }

    #[test]
    fn builder_pattern() {
        let txn = TransactionBuilder::new()
            .label("test")
            .push(Operation::set_metadata("title", Some("Doc".into())))
            .push(Operation::set_metadata("creator", Some("User".into())))
            .build();

        assert_eq!(txn.label.as_deref(), Some("test"));
        assert_eq!(txn.len(), 2);
    }
}
