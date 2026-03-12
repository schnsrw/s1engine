//! High-level document wrapper.
//!
//! [`Document`] wraps [`DocumentModel`] with undo/redo history and provides
//! a convenient API for reading, editing, and exporting documents.

use s1_model::{DocumentMetadata, DocumentModel, Node, NodeId, NodeType};
use s1_ops::{History, Operation, Transaction, TransactionBuilder};

use crate::error::Error;
use crate::format::Format;

/// A document with undo/redo history and high-level operations.
pub struct Document {
    model: DocumentModel,
    history: History,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            model: DocumentModel::new(),
            history: History::new(),
        }
    }

    /// Create a Document from an existing model (e.g., after reading a file).
    pub fn from_model(model: DocumentModel) -> Self {
        Self {
            model,
            history: History::new(),
        }
    }

    // ─── Model access ────────────────────────────────────────────────

    /// Get a read-only reference to the underlying document model.
    pub fn model(&self) -> &DocumentModel {
        &self.model
    }

    /// Get a mutable reference to the underlying document model.
    ///
    /// # Warning
    ///
    /// **This is an advanced escape hatch.** Direct mutation bypasses the
    /// operation system, which means:
    /// - Changes will NOT be recorded in undo/redo history
    /// - Changes will NOT generate CRDT operations for collaboration
    /// - The document may enter an inconsistent state
    ///
    /// Prefer [`apply`](Self::apply) or [`apply_transaction`](Self::apply_transaction)
    /// for all edits that should be undoable or collaborative.
    ///
    /// This method exists for cases where you need direct model access
    /// (e.g., bulk import, format reader integration, or testing).
    pub fn model_mut(&mut self) -> &mut DocumentModel {
        &mut self.model
    }

    /// Consume the Document and return the underlying model.
    pub fn into_model(self) -> DocumentModel {
        self.model
    }

    // ─── Metadata ────────────────────────────────────────────────────

    /// Get document metadata (title, author, etc.).
    pub fn metadata(&self) -> &DocumentMetadata {
        self.model.metadata()
    }

    /// Get mutable document metadata.
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        self.model.metadata_mut()
    }

    // ─── Content queries ─────────────────────────────────────────────

    /// Extract all text as a plain string. Paragraphs separated by newlines.
    pub fn to_plain_text(&self) -> String {
        self.model.to_plain_text()
    }

    /// Get the body node ID.
    pub fn body_id(&self) -> Option<NodeId> {
        self.model.body_id()
    }

    /// Get a node by ID.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.model.node(id)
    }

    /// Generate the next unique node ID.
    pub fn next_id(&mut self) -> NodeId {
        self.model.next_id()
    }

    /// Return top-level paragraph node IDs in document order.
    ///
    /// This returns only direct children of the document body that are
    /// paragraphs. Paragraphs nested inside tables, headers, footers,
    /// or other container elements are **not** included.
    ///
    /// To traverse all paragraphs (including nested ones), walk the
    /// document tree via [`model()`](Self::model) and
    /// [`DocumentModel::node()`].
    pub fn paragraph_ids(&self) -> Vec<NodeId> {
        let body_id = match self.model.body_id() {
            Some(id) => id,
            None => return vec![],
        };
        let body = match self.model.node(body_id) {
            Some(n) => n,
            None => return vec![],
        };
        body.children
            .iter()
            .copied()
            .filter(|id| {
                self.model
                    .node(*id)
                    .map(|n| n.node_type == NodeType::Paragraph)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Count top-level body paragraphs.
    ///
    /// Equivalent to `self.paragraph_ids().len()`. See
    /// [`paragraph_ids()`](Self::paragraph_ids) for semantics.
    pub fn paragraph_count(&self) -> usize {
        self.paragraph_ids().len()
    }

    // ─── Styles ──────────────────────────────────────────────────────

    /// Get all styles.
    pub fn styles(&self) -> &[s1_model::Style] {
        self.model.styles()
    }

    /// Get a style by ID.
    pub fn style_by_id(&self, id: &str) -> Option<&s1_model::Style> {
        self.model.style_by_id(id)
    }

    /// Get the numbering definitions.
    pub fn numbering(&self) -> &s1_model::NumberingDefinitions {
        self.model.numbering()
    }

    /// Get section properties.
    pub fn sections(&self) -> &[s1_model::SectionProperties] {
        self.model.sections()
    }

    // ─── Transactions ────────────────────────────────────────────────

    /// Begin building a new transaction.
    ///
    /// All operations within a transaction form a single undo unit.
    pub fn begin_transaction(label: &str) -> TransactionBuilder {
        TransactionBuilder::new().label(label)
    }

    /// Apply a transaction to the document.
    ///
    /// On success, the transaction is pushed onto the undo stack.
    /// On failure, all operations are rolled back.
    pub fn apply_transaction(&mut self, txn: &Transaction) -> Result<(), Error> {
        self.history.apply(&mut self.model, txn)?;
        Ok(())
    }

    /// Apply a single operation as a transaction.
    pub fn apply(&mut self, op: Operation) -> Result<(), Error> {
        let mut txn = Transaction::new();
        txn.push(op);
        self.apply_transaction(&txn)
    }

    // ─── Undo / Redo ─────────────────────────────────────────────────

    /// Undo the last transaction. Returns `true` if something was undone.
    pub fn undo(&mut self) -> Result<bool, Error> {
        Ok(self.history.undo(&mut self.model)?)
    }

    /// Redo the last undone transaction. Returns `true` if something was redone.
    pub fn redo(&mut self) -> Result<bool, Error> {
        Ok(self.history.redo(&mut self.model)?)
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Clear all undo/redo history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    // ─── Export ──────────────────────────────────────────────────────

    /// Export the document to bytes in the given format.
    pub fn export(&self, format: Format) -> Result<Vec<u8>, Error> {
        match format {
            #[cfg(feature = "docx")]
            Format::Docx => Ok(s1_format_docx::write(&self.model)?),
            #[cfg(feature = "odt")]
            Format::Odt => Ok(s1_format_odt::write(&self.model)?),
            #[cfg(feature = "txt")]
            Format::Txt => Ok(s1_format_txt::write(&self.model)),
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat(format!(
                "{:?} export not available (check feature flags)",
                format
            ))),
        }
    }

    /// Export the document as a string (useful for TXT format).
    pub fn export_string(&self, format: Format) -> Result<String, Error> {
        match format {
            #[cfg(feature = "txt")]
            Format::Txt => Ok(s1_format_txt::write_string(&self.model)),
            _ => {
                let bytes = self.export(format)?;
                String::from_utf8(bytes)
                    .map_err(|e| Error::Format(format!("Output is not valid UTF-8: {e}")))
            }
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
