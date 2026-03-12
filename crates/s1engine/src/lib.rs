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
}
