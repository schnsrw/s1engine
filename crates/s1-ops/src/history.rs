//! Undo/redo history.
//!
//! [`History`] maintains undo and redo stacks of [`Transaction`]s. When a
//! transaction is applied, its inverse is pushed onto the undo stack.
//! Undoing pops from the undo stack and pushes onto redo. Redo does the reverse.
//! Any new edit clears the redo stack.

use crate::operation::OperationError;
use crate::transaction::{apply_transaction, Transaction};
use s1_model::DocumentModel;

/// Undo/redo history for a document.
#[derive(Debug)]
pub struct History {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    /// Maximum number of undo steps. `0` means unlimited.
    max_undo: usize,
}

impl History {
    /// Create a new history with unlimited undo.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 0,
        }
    }

    /// Create history with a maximum undo depth.
    pub fn with_max_undo(max_undo: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo,
        }
    }

    /// Apply a transaction and push its inverse onto the undo stack.
    /// Clears the redo stack (new edits invalidate redo).
    pub fn apply(
        &mut self,
        model: &mut DocumentModel,
        txn: &Transaction,
    ) -> Result<(), OperationError> {
        let inverse = apply_transaction(model, txn)?;
        self.undo_stack.push(inverse);
        self.redo_stack.clear();

        // Trim undo stack if max is set
        if self.max_undo > 0 && self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }

        Ok(())
    }

    /// Undo the last transaction. Returns `true` if something was undone.
    pub fn undo(&mut self, model: &mut DocumentModel) -> Result<bool, OperationError> {
        let txn = match self.undo_stack.pop() {
            Some(t) => t,
            None => return Ok(false),
        };

        let inverse = apply_transaction(model, &txn)?;
        self.redo_stack.push(inverse);
        Ok(true)
    }

    /// Redo the last undone transaction. Returns `true` if something was redone.
    pub fn redo(&mut self, model: &mut DocumentModel) -> Result<bool, OperationError> {
        let txn = match self.redo_stack.pop() {
            Some(t) => t,
            None => return Ok(false),
        };

        let inverse = apply_transaction(model, &txn)?;
        self.undo_stack.push(inverse);
        Ok(true)
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Number of undo steps available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of redo steps available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Set the maximum undo depth. 0 means unlimited.
    ///
    /// If the current stack exceeds the new limit, oldest entries are trimmed.
    pub fn set_max_undo(&mut self, max: usize) {
        self.max_undo = max;
        if max > 0 {
            while self.undo_stack.len() > max {
                self.undo_stack.remove(0);
            }
        }
    }

    /// Get the current maximum undo depth (0 = unlimited).
    pub fn max_undo(&self) -> usize {
        self.max_undo
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::Operation;
    use s1_model::{Node, NodeType};

    fn make_insert_para_txn(doc: &mut DocumentModel) -> (Transaction, s1_model::NodeId) {
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();
        let mut txn = Transaction::with_label("Add paragraph");
        txn.push(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ));
        (txn, para_id)
    }

    #[test]
    fn undo_redo_basic() {
        let mut doc = DocumentModel::new();
        let mut history = History::new();

        let (txn, para_id) = make_insert_para_txn(&mut doc);
        history.apply(&mut doc, &txn).unwrap();
        assert!(doc.node(para_id).is_some());
        assert!(history.can_undo());
        assert!(!history.can_redo());

        // Undo
        assert!(history.undo(&mut doc).unwrap());
        assert!(doc.node(para_id).is_none());
        assert!(!history.can_undo());
        assert!(history.can_redo());

        // Redo
        assert!(history.redo(&mut doc).unwrap());
        assert!(doc.node(para_id).is_some());
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_empty() {
        let mut doc = DocumentModel::new();
        let mut history = History::new();
        assert!(!history.undo(&mut doc).unwrap());
    }

    #[test]
    fn redo_empty() {
        let mut doc = DocumentModel::new();
        let mut history = History::new();
        assert!(!history.redo(&mut doc).unwrap());
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut doc = DocumentModel::new();
        let mut history = History::new();

        let (txn1, _) = make_insert_para_txn(&mut doc);
        history.apply(&mut doc, &txn1).unwrap();

        // Undo
        history.undo(&mut doc).unwrap();
        assert!(history.can_redo());

        // New edit should clear redo
        let (txn2, _) = make_insert_para_txn(&mut doc);
        history.apply(&mut doc, &txn2).unwrap();
        assert!(!history.can_redo());
    }

    #[test]
    fn multiple_undo_redo() {
        let mut doc = DocumentModel::new();
        let mut history = History::new();

        // Apply 3 transactions
        for _ in 0..3 {
            let (txn, _) = make_insert_para_txn(&mut doc);
            history.apply(&mut doc, &txn).unwrap();
        }

        assert_eq!(history.undo_count(), 3);

        // Undo all
        history.undo(&mut doc).unwrap();
        history.undo(&mut doc).unwrap();
        history.undo(&mut doc).unwrap();
        assert_eq!(history.undo_count(), 0);
        assert_eq!(history.redo_count(), 3);

        // Redo all
        history.redo(&mut doc).unwrap();
        history.redo(&mut doc).unwrap();
        history.redo(&mut doc).unwrap();
        assert_eq!(history.undo_count(), 3);
        assert_eq!(history.redo_count(), 0);
    }

    #[test]
    fn max_undo_limit() {
        let mut doc = DocumentModel::new();
        let mut history = History::with_max_undo(2);

        for _ in 0..5 {
            let (txn, _) = make_insert_para_txn(&mut doc);
            history.apply(&mut doc, &txn).unwrap();
        }

        // Only 2 undo steps should remain
        assert_eq!(history.undo_count(), 2);
    }

    #[test]
    fn clear_history() {
        let mut doc = DocumentModel::new();
        let mut history = History::new();

        let (txn, _) = make_insert_para_txn(&mut doc);
        history.apply(&mut doc, &txn).unwrap();
        history.undo(&mut doc).unwrap();

        assert!(history.can_redo());
        history.clear();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn text_edit_undo_redo() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let mut history = History::new();

        // Build paragraph structure
        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let setup = {
            let mut txn = Transaction::new();
            txn.push(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ));
            txn.push(Operation::insert_node(
                para_id,
                0,
                Node::new(run_id, NodeType::Run),
            ));
            txn.push(Operation::insert_node(
                run_id,
                0,
                Node::text(text_id, "Hello"),
            ));
            txn
        };
        history.apply(&mut doc, &setup).unwrap();

        // Insert text
        let mut insert_txn = Transaction::with_label("Insert text");
        insert_txn.push(Operation::insert_text(text_id, 5, " World"));
        history.apply(&mut doc, &insert_txn).unwrap();

        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );

        // Undo insert
        history.undo(&mut doc).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello")
        );

        // Redo insert
        history.redo(&mut doc).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );
    }
}
