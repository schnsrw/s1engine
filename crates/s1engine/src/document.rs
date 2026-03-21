//! High-level document wrapper.
//!
//! [`Document`] wraps [`DocumentModel`] with undo/redo history and provides
//! a convenient API for reading, editing, and exporting documents.

use s1_model::{
    AttributeKey, AttributeValue, DocumentMetadata, DocumentModel, Node, NodeId, NodeType,
};
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

    /// Set the maximum number of undo steps. 0 means unlimited.
    pub fn set_undo_cap(&mut self, max: usize) {
        self.history.set_max_undo(max);
    }

    /// Get the number of undo steps currently on the stack.
    pub fn undo_count(&self) -> usize {
        self.history.undo_count()
    }

    /// Merge the last `count` undo entries into a single undo step.
    ///
    /// Used by the batch operation API to group multiple operations
    /// into one undo unit.
    pub fn merge_undo_entries(&mut self, count: usize, label: &str) -> Result<(), Error> {
        self.history.merge_undo_entries(count, label);
        Ok(())
    }

    // ─── TOC ────────────────────────────────────────────────────────

    /// Update all Table of Contents entries in the document.
    ///
    /// Scans for heading paragraphs and regenerates the cached entry
    /// paragraphs inside each TOC node. Call this before exporting if
    /// content has changed since the TOC was inserted.
    pub fn update_toc(&mut self) {
        // First, find all TOC nodes and their max_level
        let body_id = match self.model.body_id() {
            Some(id) => id,
            None => return,
        };
        let toc_nodes: Vec<(NodeId, u8)> = self
            .find_toc_nodes(body_id)
            .into_iter()
            .map(|id| {
                let max_level = self
                    .model
                    .node(id)
                    .and_then(|n| n.attributes.get_i64(&AttributeKey::TocMaxLevel))
                    .unwrap_or(3) as u8;
                (id, max_level)
            })
            .collect();

        if toc_nodes.is_empty() {
            return;
        }

        // Collect headings (excluding any inside TOC nodes)
        let headings = self.model.collect_headings();

        for (toc_id, max_level) in toc_nodes {
            self.generate_toc_entries(toc_id, max_level, &headings);
        }
    }

    fn find_toc_nodes(&self, container_id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        if let Some(node) = self.model.node(container_id) {
            for &child_id in &node.children {
                if let Some(child) = self.model.node(child_id) {
                    if child.node_type == NodeType::TableOfContents {
                        result.push(child_id);
                    }
                }
            }
        }
        result
    }

    fn generate_toc_entries(
        &mut self,
        toc_id: NodeId,
        max_level: u8,
        headings: &[(NodeId, u8, String)],
    ) {
        // Remove existing children
        if let Some(toc) = self.model.node(toc_id) {
            let old_children: Vec<NodeId> = toc.children.clone();
            for child_id in old_children {
                let _ = self.model.remove_node(child_id);
            }
        }

        // Generate new entry paragraphs
        let mut child_index = 0;
        for (_heading_id, level, text) in headings {
            if *level > max_level {
                continue;
            }

            // Create paragraph for this TOC entry
            let para_id = self.model.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::StyleId,
                AttributeValue::String(format!("TOC{}", level)),
            );
            let _ = self.model.insert_node(toc_id, child_index, para);

            // Add a run with the heading text
            let run_id = self.model.next_id();
            let _ = self
                .model
                .insert_node(para_id, 0, Node::new(run_id, NodeType::Run));

            let text_id = self.model.next_id();
            let _ = self
                .model
                .insert_node(run_id, 0, Node::text(text_id, text.clone()));

            child_index += 1;
        }
    }

    // ─── Track Changes ───────────────────────────────────────────

    /// List all tracked changes in the document.
    ///
    /// Returns a list of tuples: `(node_id, revision_type, author, date)` for
    /// every node that carries a `RevisionType` attribute.
    pub fn tracked_changes(&self) -> Vec<(NodeId, String, Option<String>, Option<String>)> {
        let root_id = self.model.root_id();
        let mut result = Vec::new();
        for node in self.model.descendants(root_id) {
            if let Some(rev_type) = node.attributes.get_string(&AttributeKey::RevisionType) {
                let author = node
                    .attributes
                    .get_string(&AttributeKey::RevisionAuthor)
                    .map(|s| s.to_string());
                let date = node
                    .attributes
                    .get_string(&AttributeKey::RevisionDate)
                    .map(|s| s.to_string());
                result.push((node.id, rev_type.to_string(), author, date));
            }
        }
        result
    }

    /// Accept all tracked changes in the document.
    ///
    /// - **Insertions**: revision attributes are removed; the inserted content stays.
    /// - **Deletions**: the deleted nodes are removed from the tree entirely.
    /// - **Format changes**: revision attributes (including original formatting)
    ///   are removed; the current formatting is kept.
    ///
    /// This is a bulk transform that bypasses the undo/redo history.
    ///
    /// # Errors
    ///
    /// Returns an error if a node marked for deletion cannot be removed
    /// (e.g., it is the root node).
    pub fn accept_all_changes(&mut self) -> Result<(), Error> {
        let changes = self.tracked_changes();
        for (node_id, rev_type, _, _) in changes {
            self.accept_change_inner(node_id, &rev_type)?;
        }
        Ok(())
    }

    /// Reject all tracked changes in the document.
    ///
    /// - **Insertions**: the inserted nodes are removed from the tree entirely.
    /// - **Deletions**: revision attributes are removed; the content stays (un-deleted).
    /// - **Format changes**: original formatting is restored from
    ///   `RevisionOriginalFormatting`, and all revision attributes are removed.
    ///
    /// This is a bulk transform that bypasses the undo/redo history.
    ///
    /// # Errors
    ///
    /// Returns an error if a node marked for removal cannot be removed
    /// (e.g., it is the root node).
    pub fn reject_all_changes(&mut self) -> Result<(), Error> {
        let changes = self.tracked_changes();
        for (node_id, rev_type, _, _) in changes {
            self.reject_change_inner(node_id, &rev_type)?;
        }
        Ok(())
    }

    /// Accept a single tracked change by node ID.
    ///
    /// See [`accept_all_changes`](Self::accept_all_changes) for the semantics
    /// of accepting each revision type.
    ///
    /// # Errors
    ///
    /// Returns an error if the node does not exist, has no `RevisionType`
    /// attribute, or cannot be removed from the tree.
    pub fn accept_change(&mut self, node_id: NodeId) -> Result<(), Error> {
        let rev_type = self
            .model
            .node(node_id)
            .and_then(|n| {
                n.attributes
                    .get_string(&AttributeKey::RevisionType)
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| {
                Error::Format(format!(
                    "Node {node_id} does not exist or has no RevisionType attribute"
                ))
            })?;
        self.accept_change_inner(node_id, &rev_type)
    }

    /// Reject a single tracked change by node ID.
    ///
    /// See [`reject_all_changes`](Self::reject_all_changes) for the semantics
    /// of rejecting each revision type.
    ///
    /// # Errors
    ///
    /// Returns an error if the node does not exist, has no `RevisionType`
    /// attribute, or cannot be removed from the tree.
    pub fn reject_change(&mut self, node_id: NodeId) -> Result<(), Error> {
        let rev_type = self
            .model
            .node(node_id)
            .and_then(|n| {
                n.attributes
                    .get_string(&AttributeKey::RevisionType)
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| {
                Error::Format(format!(
                    "Node {node_id} does not exist or has no RevisionType attribute"
                ))
            })?;
        self.reject_change_inner(node_id, &rev_type)
    }

    /// Internal: accept a single change (shared by accept_change and accept_all_changes).
    fn accept_change_inner(&mut self, node_id: NodeId, rev_type: &str) -> Result<(), Error> {
        match rev_type {
            "Insert" | "FormatChange" => {
                // Content/formatting stays; just strip revision attributes.
                Self::strip_revision_attributes(self.model.node_mut(node_id));
            }
            "Delete" => {
                // Remove the node entirely.
                self.model.remove_node(node_id).map_err(|e| {
                    Error::Format(format!("Failed to remove deleted node {node_id}: {e}"))
                })?;
            }
            _ => {
                // Unknown revision type — strip attributes defensively.
                Self::strip_revision_attributes(self.model.node_mut(node_id));
            }
        }
        Ok(())
    }

    /// Internal: reject a single change (shared by reject_change and reject_all_changes).
    fn reject_change_inner(&mut self, node_id: NodeId, rev_type: &str) -> Result<(), Error> {
        match rev_type {
            "Insert" => {
                // Remove the inserted node entirely.
                self.model.remove_node(node_id).map_err(|e| {
                    Error::Format(format!("Failed to remove inserted node {node_id}: {e}"))
                })?;
            }
            "Delete" => {
                // Un-delete: content stays, strip revision attributes.
                Self::strip_revision_attributes(self.model.node_mut(node_id));
            }
            "FormatChange" => {
                // Restore original formatting if available, then strip revision attrs.
                if let Some(node) = self.model.node_mut(node_id) {
                    // If RevisionOriginalFormatting contains serialized attribute data,
                    // parse and restore it. The convention is a semicolon-separated list
                    // of "key=value" pairs, but for now we support the common case where
                    // original formatting attributes were stored alongside the revision
                    // attributes. The caller is responsible for setting appropriate
                    // original formatting attributes before calling reject.
                    //
                    // Remove all revision-related attributes.
                    node.attributes.remove(&AttributeKey::RevisionType);
                    node.attributes.remove(&AttributeKey::RevisionAuthor);
                    node.attributes.remove(&AttributeKey::RevisionDate);
                    node.attributes.remove(&AttributeKey::RevisionId);
                    node.attributes
                        .remove(&AttributeKey::RevisionOriginalFormatting);
                }
            }
            _ => {
                // Unknown revision type — strip attributes defensively.
                Self::strip_revision_attributes(self.model.node_mut(node_id));
            }
        }
        Ok(())
    }

    /// Remove all revision-tracking attributes from a node.
    fn strip_revision_attributes(node: Option<&mut Node>) {
        if let Some(node) = node {
            node.attributes.remove(&AttributeKey::RevisionType);
            node.attributes.remove(&AttributeKey::RevisionAuthor);
            node.attributes.remove(&AttributeKey::RevisionDate);
            node.attributes.remove(&AttributeKey::RevisionId);
            node.attributes
                .remove(&AttributeKey::RevisionOriginalFormatting);
        }
    }

    // ─── Layout ──────────────────────────────────────────────────

    /// Lay out the document using the default configuration.
    ///
    /// Requires the `layout` feature flag. The returned [`s1_layout::LayoutDocument`]
    /// contains pages with positioned blocks, lines, and glyph runs ready for
    /// rendering or PDF export.
    ///
    /// # Errors
    ///
    /// Returns an error if fonts cannot be resolved or text shaping fails.
    #[cfg(feature = "layout")]
    pub fn layout(
        &self,
        font_db: &s1_text::FontDatabase,
    ) -> Result<s1_layout::LayoutDocument, Error> {
        self.layout_with_config(font_db, s1_layout::LayoutConfig::default())
    }

    /// Lay out the document with a custom configuration.
    ///
    /// Requires the `layout` feature flag. Use this method when you need
    /// to control page dimensions, margins, or widow/orphan settings.
    ///
    /// # Errors
    ///
    /// Returns an error if fonts cannot be resolved or text shaping fails.
    #[cfg(feature = "layout")]
    pub fn layout_with_config(
        &self,
        font_db: &s1_text::FontDatabase,
        config: s1_layout::LayoutConfig,
    ) -> Result<s1_layout::LayoutDocument, Error> {
        let mut engine = s1_layout::LayoutEngine::new(&self.model, font_db, config);
        Ok(engine.layout()?)
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
            #[cfg(feature = "md")]
            Format::Md => Ok(s1_format_md::write_bytes(&self.model)),
            #[cfg(feature = "pdf")]
            Format::Pdf => {
                let font_db = s1_text::FontDatabase::empty();
                self.export_pdf(&font_db)
            }
            #[cfg(feature = "convert")]
            Format::Csv => {
                let csv_text = s1_convert::model_to_csv(&self.model);
                Ok(csv_text.into_bytes())
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat(format!(
                "{:?} export not available (check feature flags)",
                format
            ))),
        }
    }

    /// Export the document as PDF using the provided font database.
    ///
    /// Requires the `pdf` feature flag. Lays out the document with the
    /// given fonts and renders to PDF bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if layout or PDF generation fails.
    #[cfg(feature = "pdf")]
    pub fn export_pdf(&self, font_db: &s1_text::FontDatabase) -> Result<Vec<u8>, Error> {
        let layout = self.layout(font_db)?;
        let bytes = s1_format_pdf::write_pdf(&layout, font_db, Some(self.model.metadata()))?;
        Ok(bytes)
    }

    /// Export the document as PDF with a custom layout configuration.
    ///
    /// Requires the `pdf` feature flag. Use this method when you need to
    /// control page dimensions, margins, or other layout settings.
    ///
    /// # Errors
    ///
    /// Returns an error if layout or PDF generation fails.
    #[cfg(feature = "pdf")]
    pub fn export_pdf_with_config(
        &self,
        font_db: &s1_text::FontDatabase,
        config: s1_layout::LayoutConfig,
    ) -> Result<Vec<u8>, Error> {
        let layout = self.layout_with_config(font_db, config)?;
        let bytes = s1_format_pdf::write_pdf(&layout, font_db, Some(self.model.metadata()))?;
        Ok(bytes)
    }

    /// Export the document as PDF/A (archival-compliant PDF).
    ///
    /// PDF/A-1b includes an ICC color profile, XMP metadata, and output intent
    /// for long-term archival compliance.
    ///
    /// # Errors
    ///
    /// Returns an error if layout or PDF generation fails.
    #[cfg(feature = "pdf")]
    pub fn export_pdf_a(
        &self,
        font_db: &s1_text::FontDatabase,
        conformance: s1_format_pdf::PdfAConformance,
    ) -> Result<Vec<u8>, Error> {
        let layout = self.layout(font_db)?;
        let bytes =
            s1_format_pdf::write_pdf_a(&layout, font_db, Some(self.model.metadata()), conformance)?;
        Ok(bytes)
    }

    /// Export the document as a string (useful for TXT and Markdown formats).
    pub fn export_string(&self, format: Format) -> Result<String, Error> {
        match format {
            #[cfg(feature = "txt")]
            Format::Txt => Ok(s1_format_txt::write_string(&self.model)),
            #[cfg(feature = "md")]
            Format::Md => Ok(s1_format_md::write_string(&self.model)),
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
