//! s1engine — A modern, modular document engine.
//!
//! Read, write, edit, and convert documents: DOCX, ODT, PDF, TXT.
//! Designed as an embeddable SDK for building document editors,
//! converters, and collaborative editing applications.
//!
//! # Quick Start
//!
//! ```no_run
//! use s1engine::{Engine, Format};
//!
//! let engine = Engine::new();
//!
//! // Open a DOCX file
//! let doc = engine.open_file("report.docx").unwrap();
//! println!("{}", doc.to_plain_text());
//!
//! // Export as TXT
//! let txt = doc.export(Format::Txt).unwrap();
//! ```
//!
//! # Architecture
//!
//! This crate is a facade over the internal `s1-*` crates:
//! - `s1-model` — Core document tree, nodes, attributes, styles
//! - `s1-ops` — Operations, transactions, undo/redo
//! - `s1-format-docx` — DOCX reader/writer (feature: `docx`)
//! - `s1-format-txt` — TXT reader/writer (feature: `txt`)
//!
//! All document mutations go through the operation system, enabling
//! undo/redo and (future) CRDT-based collaborative editing.

pub mod builder;
pub mod document;
pub mod engine;
pub mod error;
pub mod format;

// Re-export primary facade types.
pub use builder::{DocumentBuilder, ParagraphBuilder, RowBuilder, TableBuilder};
pub use document::Document;
pub use engine::Engine;
pub use error::Error;
pub use format::Format;

// Re-export key model types consumers will need.
pub use s1_model::{
    Alignment, AttributeKey, AttributeMap, AttributeValue, BorderSide, BorderStyle, Borders, Color,
    DocumentMetadata, DocumentModel, FieldType, HeaderFooterRef, HeaderFooterType, LineSpacing,
    ListFormat, ListInfo, MediaId, MediaStore, Node, NodeId, NodeType, PageOrientation,
    SectionBreakType, SectionProperties, Style, StyleType, TabAlignment, TabLeader, TabStop,
    TableWidth, UnderlineStyle, VerticalAlignment,
};

// Re-export operation types.
pub use s1_ops::{
    History, Operation, OperationError, Position, Selection, Transaction, TransactionBuilder,
};

/// Low-level access to the core document model types.
///
/// This module re-exports the full `s1-model` crate for advanced use cases
/// that need types not covered by the top-level re-exports. Types accessed
/// through this module are part of the public API and follow semver.
pub use s1_model as model;

/// Low-level access to the operations layer.
///
/// This module re-exports the full `s1-ops` crate for advanced use cases
/// that need direct operation construction beyond what [`Document`] provides.
/// Types accessed through this module are part of the public API and follow semver.
pub use s1_ops as ops;

// CRDT / collaboration support (feature-gated).
#[cfg(feature = "crdt")]
pub use s1_crdt as crdt;

#[cfg(feature = "crdt")]
pub use s1_crdt::{CollabDocument, CrdtError, CrdtOperation, OpId, StateVector};

// Layout engine support (feature-gated).
#[cfg(feature = "layout")]
pub use s1_layout as layout;

#[cfg(feature = "layout")]
pub use s1_layout::{
    layout_to_html, layout_to_html_with_options, GlyphRun, HtmlOptions, LayoutBlock,
    LayoutBlockKind, LayoutBookmark, LayoutCache, LayoutConfig, LayoutDocument, LayoutLine,
    LayoutPage, LayoutTableCell, LayoutTableRow, PageLayout, Rect,
};

#[cfg(feature = "layout")]
pub use s1_text as text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty_document() {
        let engine = Engine::new();
        let doc = engine.create();
        assert_eq!(doc.to_plain_text(), "");
        assert_eq!(doc.paragraph_count(), 0);
    }

    #[test]
    fn document_metadata() {
        let engine = Engine::new();
        let mut doc = engine.create();
        doc.metadata_mut().title = Some("Test".to_string());
        assert_eq!(doc.metadata().title.as_deref(), Some("Test"));
    }

    #[test]
    fn document_apply_and_undo() {
        let engine = Engine::new();
        let mut doc = engine.create();
        let body_id = doc.body_id().unwrap();

        // Insert a paragraph via transaction
        let para_id = doc.next_id();
        let mut txn = Transaction::with_label("Add paragraph");
        txn.push(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ));

        let run_id = doc.next_id();
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));

        let text_id = doc.next_id();
        txn.push(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, "Hello World"),
        ));

        doc.apply_transaction(&txn).unwrap();
        assert_eq!(doc.to_plain_text(), "Hello World");
        assert!(doc.can_undo());

        // Undo
        assert!(doc.undo().unwrap());
        assert_eq!(doc.to_plain_text(), "");
        assert!(doc.can_redo());

        // Redo
        assert!(doc.redo().unwrap());
        assert_eq!(doc.to_plain_text(), "Hello World");
    }

    #[test]
    fn document_paragraph_ids() {
        let engine = Engine::new();
        let mut doc = engine.create();
        let body_id = doc.body_id().unwrap();

        // Insert two paragraphs
        for i in 0..2 {
            let para_id = doc.next_id();
            doc.apply(Operation::insert_node(
                body_id,
                i,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap();
        }

        assert_eq!(doc.paragraph_count(), 2);
        assert_eq!(doc.paragraph_ids().len(), 2);
    }

    #[cfg(feature = "docx")]
    #[test]
    fn open_and_export_docx() {
        // Create a document, export as DOCX, re-open, verify text preserved
        let engine = Engine::new();
        let mut doc = engine.create();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.apply(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        let run_id = doc.next_id();
        doc.apply(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ))
        .unwrap();

        let text_id = doc.next_id();
        doc.apply(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, "Round-trip test"),
        ))
        .unwrap();

        let bytes = doc.export(Format::Docx).unwrap();

        let doc2 = engine.open(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Round-trip test");
    }

    #[cfg(feature = "txt")]
    #[test]
    fn open_and_export_txt() {
        let engine = Engine::new();

        // Open text bytes
        let doc = engine.open_as(b"Line one\nLine two", Format::Txt).unwrap();
        assert_eq!(doc.to_plain_text(), "Line one\nLine two");
        assert_eq!(doc.paragraph_count(), 2);

        // Export back to TXT
        let txt = doc.export_string(Format::Txt).unwrap();
        assert_eq!(txt, "Line one\nLine two");
    }

    #[cfg(feature = "odt")]
    #[test]
    fn open_and_export_odt() {
        let engine = Engine::new();
        let mut doc = engine.create();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.apply(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        let run_id = doc.next_id();
        doc.apply(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ))
        .unwrap();

        let text_id = doc.next_id();
        doc.apply(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, "ODT round-trip"),
        ))
        .unwrap();

        let bytes = doc.export(Format::Odt).unwrap();

        let doc2 = engine.open_as(&bytes, Format::Odt).unwrap();
        assert!(doc2.to_plain_text().contains("ODT round-trip"));
    }

    #[cfg(feature = "odt")]
    #[test]
    fn odt_builder_roundtrip() {
        let doc = DocumentBuilder::new()
            .heading(1, "Title")
            .paragraph(|p| p.text("Some text").bold("bold part"))
            .build();

        let bytes = doc.export(Format::Odt).unwrap();

        let engine = Engine::new();
        let doc2 = engine.open_as(&bytes, Format::Odt).unwrap();

        let text = doc2.to_plain_text();
        assert!(text.contains("Title"));
        assert!(text.contains("Some text"));
        assert!(text.contains("bold part"));
    }

    #[test]
    fn format_detection() {
        assert_eq!(Format::detect(b"PK\x03\x04word/doc"), Format::Docx);
        assert_eq!(Format::detect(b"%PDF-1.5"), Format::Pdf);
        assert_eq!(Format::detect(b"Hello"), Format::Txt);
    }

    #[test]
    fn unsupported_format_error() {
        let engine = Engine::new();
        let result = engine.open_as(b"dummy", Format::Pdf);
        assert!(result.is_err());
    }

    #[test]
    fn document_clear_history() {
        let engine = Engine::new();
        let mut doc = engine.create();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.apply(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        assert!(doc.can_undo());
        doc.clear_history();
        assert!(!doc.can_undo());
    }

    // ─── Layout Tests (feature-gated) ────────────────────────────

    #[cfg(feature = "layout")]
    #[test]
    fn layout_empty_document() {
        let doc = Document::new();
        let font_db = s1_text::FontDatabase::empty();
        let result = doc.layout(&font_db).unwrap();
        assert_eq!(result.pages.len(), 1, "empty doc should produce 1 page");
        assert!(result.pages[0].blocks.is_empty());
    }

    #[cfg(feature = "layout")]
    #[test]
    fn layout_single_paragraph() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("Hello World"))
            .build();
        let font_db = s1_text::FontDatabase::empty();
        let result = doc.layout(&font_db).unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Paragraph { lines, .. } => {
                assert!(!lines.is_empty(), "paragraph should have at least one line");
            }
            _ => panic!("expected a paragraph block"),
        }
    }

    #[cfg(feature = "layout")]
    #[test]
    fn layout_with_config() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("Hello"))
            .build();
        let font_db = s1_text::FontDatabase::empty();
        let config = LayoutConfig {
            default_page_layout: PageLayout::a4(),
            ..Default::default()
        };
        let result = doc.layout_with_config(&font_db, config).unwrap();
        let page = &result.pages[0];
        assert!(
            (page.width - 595.28).abs() < 0.01,
            "A4 page width should be ~595.28pt"
        );
        assert!(
            (page.height - 841.89).abs() < 0.01,
            "A4 page height should be ~841.89pt"
        );
    }

    #[cfg(feature = "layout")]
    #[test]
    fn layout_table() {
        let doc = DocumentBuilder::new()
            .table(|t| {
                t.row(|r| r.cell("R0C0").cell("R0C1"))
                    .row(|r| r.cell("R1C0").cell("R1C1"))
            })
            .build();
        let font_db = s1_text::FontDatabase::empty();
        let result = doc.layout(&font_db).unwrap();
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Table { rows, .. } => {
                assert_eq!(rows.len(), 2, "table should have 2 rows");
                assert_eq!(rows[0].cells.len(), 2, "row should have 2 cells");
            }
            _ => panic!("expected a table block"),
        }
    }

    #[cfg(feature = "layout")]
    #[test]
    fn layout_multiple_paragraphs() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("First paragraph"))
            .paragraph(|p| p.text("Second paragraph"))
            .paragraph(|p| p.text("Third paragraph"))
            .build();
        let font_db = s1_text::FontDatabase::empty();
        let result = doc.layout(&font_db).unwrap();
        assert_eq!(
            result.pages[0].blocks.len(),
            3,
            "should have 3 paragraph blocks"
        );
    }

    #[cfg(feature = "layout")]
    #[test]
    fn layout_returns_pages() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("Content"))
            .build();
        let font_db = s1_text::FontDatabase::empty();
        let result = doc.layout(&font_db).unwrap();
        assert!(!result.pages.is_empty(), "layout should return at least 1 page");
        let page = &result.pages[0];
        // Default letter size
        assert!((page.width - 612.0).abs() < 0.01);
        assert!((page.height - 792.0).abs() < 0.01);
        // Content area should be inside margins
        assert!(page.content_area.x >= 72.0);
        assert!(page.content_area.y >= 72.0);
        assert!(page.content_area.width > 0.0);
        assert!(page.content_area.height > 0.0);
    }

    // ─── Track Changes Accept/Reject Tests ──────────────────────

    /// Helper: create a document with a paragraph containing a run marked
    /// with a revision attribute. Returns (doc, run_node_id).
    fn make_tracked_doc(
        rev_type: &str,
        text: &str,
    ) -> (Document, NodeId) {
        let mut doc = Document::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let model = doc.model_mut();
        model
            .insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = model.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String(rev_type.to_string()),
        );
        run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Alice".to_string()),
        );
        run.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-03-13T10:00:00Z".to_string()),
        );
        run.attributes.set(
            AttributeKey::RevisionId,
            AttributeValue::Int(42),
        );
        model.insert_node(para_id, 0, run).unwrap();

        let text_id = model.next_id();
        model
            .insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        (doc, run_id)
    }

    #[test]
    fn test_accept_all_insertions() {
        let (mut doc, run_id) = make_tracked_doc("Insert", "inserted text");

        // Verify the tracked change exists
        let changes = doc.tracked_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1, "Insert");

        // Accept all changes
        doc.accept_all_changes().unwrap();

        // Revision attributes should be gone
        let node = doc.node(run_id).unwrap();
        assert!(node.attributes.get_string(&AttributeKey::RevisionType).is_none());
        assert!(node.attributes.get_string(&AttributeKey::RevisionAuthor).is_none());

        // Content should still be there
        assert!(doc.to_plain_text().contains("inserted text"));

        // No more tracked changes
        assert_eq!(doc.tracked_changes().len(), 0);
    }

    #[test]
    fn test_accept_all_deletions() {
        let (mut doc, run_id) = make_tracked_doc("Delete", "deleted text");

        // Content exists before accept
        assert!(doc.to_plain_text().contains("deleted text"));
        assert_eq!(doc.tracked_changes().len(), 1);

        // Accept all: deleted nodes should be removed
        doc.accept_all_changes().unwrap();

        // Node should be gone
        assert!(doc.node(run_id).is_none());

        // Text should be gone
        assert!(!doc.to_plain_text().contains("deleted text"));

        // No more tracked changes
        assert_eq!(doc.tracked_changes().len(), 0);
    }

    #[test]
    fn test_reject_all_insertions() {
        let (mut doc, run_id) = make_tracked_doc("Insert", "inserted text");

        assert_eq!(doc.tracked_changes().len(), 1);

        // Reject all: inserted nodes should be removed
        doc.reject_all_changes().unwrap();

        // Node should be gone
        assert!(doc.node(run_id).is_none());

        // Text should be gone
        assert!(!doc.to_plain_text().contains("inserted text"));

        // No more tracked changes
        assert_eq!(doc.tracked_changes().len(), 0);
    }

    #[test]
    fn test_reject_all_deletions() {
        let (mut doc, run_id) = make_tracked_doc("Delete", "deleted text");

        assert_eq!(doc.tracked_changes().len(), 1);

        // Reject all: deletions are un-deleted (revision attrs removed, content stays)
        doc.reject_all_changes().unwrap();

        // Node should still be there
        let node = doc.node(run_id).unwrap();
        assert!(node.attributes.get_string(&AttributeKey::RevisionType).is_none());
        assert!(node.attributes.get_string(&AttributeKey::RevisionAuthor).is_none());

        // Content should still be there (un-deleted)
        assert!(doc.to_plain_text().contains("deleted text"));

        // No more tracked changes
        assert_eq!(doc.tracked_changes().len(), 0);
    }

    #[test]
    fn test_accept_single_change() {
        let (mut doc, run_id1) = make_tracked_doc("Insert", "first insert");

        // Add a second tracked change
        let body_id = doc.body_id().unwrap();
        let para_id2 = doc.next_id();
        let model = doc.model_mut();
        model
            .insert_node(body_id, 1, Node::new(para_id2, NodeType::Paragraph))
            .unwrap();
        let run_id2 = model.next_id();
        let mut run2 = Node::new(run_id2, NodeType::Run);
        run2.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Insert".to_string()),
        );
        run2.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Bob".to_string()),
        );
        model.insert_node(para_id2, 0, run2).unwrap();
        let text_id2 = model.next_id();
        model
            .insert_node(run_id2, 0, Node::text(text_id2, "second insert"))
            .unwrap();

        // Two tracked changes
        assert_eq!(doc.tracked_changes().len(), 2);

        // Accept only the first change
        doc.accept_change(run_id1).unwrap();

        // First change accepted (attrs removed), second still tracked
        let changes = doc.tracked_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].0, run_id2);

        // First run has no revision attrs but content remains
        let node1 = doc.node(run_id1).unwrap();
        assert!(node1.attributes.get_string(&AttributeKey::RevisionType).is_none());
        assert!(doc.to_plain_text().contains("first insert"));
    }

    #[test]
    fn test_tracked_changes_list() {
        let mut doc = Document::new();
        let body_id = doc.body_id().unwrap();

        // No tracked changes initially
        assert_eq!(doc.tracked_changes().len(), 0);

        // Add three tracked changes of different types
        let rev_types = ["Insert", "Delete", "FormatChange"];
        let mut run_ids = Vec::new();

        for (i, rev_type) in rev_types.iter().enumerate() {
            let para_id = doc.next_id();
            let model = doc.model_mut();
            model
                .insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = model.next_id();
            let mut run = Node::new(run_id, NodeType::Run);
            run.attributes.set(
                AttributeKey::RevisionType,
                AttributeValue::String(rev_type.to_string()),
            );
            run.attributes.set(
                AttributeKey::RevisionAuthor,
                AttributeValue::String(format!("Author{}", i)),
            );
            run.attributes.set(
                AttributeKey::RevisionDate,
                AttributeValue::String(format!("2026-03-13T1{}:00:00Z", i)),
            );
            model.insert_node(para_id, 0, run).unwrap();
            let text_id = model.next_id();
            model
                .insert_node(run_id, 0, Node::text(text_id, format!("text{}", i)))
                .unwrap();
            run_ids.push(run_id);
        }

        // Should have 3 tracked changes
        let changes = doc.tracked_changes();
        assert_eq!(changes.len(), 3);

        // Verify types
        let types: Vec<&str> = changes.iter().map(|(_, t, _, _)| t.as_str()).collect();
        assert!(types.contains(&"Insert"));
        assert!(types.contains(&"Delete"));
        assert!(types.contains(&"FormatChange"));

        // Verify authors are present
        for (_, _, author, date) in &changes {
            assert!(author.is_some());
            assert!(date.is_some());
        }
    }
}
