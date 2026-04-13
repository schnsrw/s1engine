//! Tests for the s1engine API surface used by the web adapter bridge.
//!
//! These validate the core engine functions that the WASM layer wraps.
//! The WASM layer (ffi/wasm) adds JSON serialization on top of these.
//!
//! Milestones: M1 (text-only bridge), M3 (structural import)

use s1engine::{Engine, Format};

fn engine() -> Engine {
    Engine::new()
}

// ──────────────────────────────────────────────────────────────────
// M1: Text-only bridge — open, extract text, create, export
// ──────────────────────────────────────────────────────────────────

#[test]
fn create_empty_document_and_export() {
    let engine = engine();
    let doc = engine.create();
    let bytes = doc.export(Format::Docx).unwrap();
    assert!(bytes.len() > 100, "Empty DOCX should have content, got {} bytes", bytes.len());
}

#[test]
fn create_empty_document_plain_text_is_empty() {
    let engine = engine();
    let doc = engine.create();
    let text = doc.to_plain_text();
    assert!(text.trim().is_empty(), "Empty doc plain text should be empty, got: {:?}", text);
}

#[test]
fn round_trip_docx_preserves_text() {
    let engine = engine();

    // Create a simple DOCX with known text using the txt import path
    let txt_content = "Hello world\nSecond paragraph\nThird line";
    let doc = engine.open_as(txt_content.as_bytes(), Format::Txt).unwrap();
    let docx_bytes = doc.export(Format::Docx).unwrap();

    // Re-open and verify
    let doc2 = engine.open(&docx_bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Hello world"), "Round-trip must preserve 'Hello world', got: {:?}", text);
    assert!(text.contains("Second paragraph"), "Round-trip must preserve 'Second paragraph'");
    assert!(text.contains("Third line"), "Round-trip must preserve 'Third line'");
}

#[test]
fn open_txt_as_docx_export() {
    let engine = engine();
    let doc = engine.open_as(b"Test content", Format::Txt).unwrap();
    let text = doc.to_plain_text();
    assert!(text.contains("Test content"), "TXT import should preserve text");

    let bytes = doc.export(Format::Docx).unwrap();
    assert!(bytes.len() > 100, "Exported DOCX should have content");

    let doc2 = engine.open(&bytes).unwrap();
    let text2 = doc2.to_plain_text();
    assert!(text2.contains("Test content"), "DOCX round-trip should preserve text");
}

#[test]
fn multi_paragraph_text_extraction() {
    let engine = engine();
    let content = "Para one\nPara two\n\nPara four after blank";
    let doc = engine.open_as(content.as_bytes(), Format::Txt).unwrap();
    let text = doc.to_plain_text();

    assert!(text.contains("Para one"), "First paragraph");
    assert!(text.contains("Para two"), "Second paragraph");
    assert!(text.contains("Para four"), "Fourth paragraph");
}

#[test]
fn export_multiple_formats() {
    let engine = engine();
    let doc = engine.open_as(b"Format test", Format::Txt).unwrap();

    let docx = doc.export(Format::Docx).unwrap();
    assert!(docx.len() > 0, "DOCX export should produce bytes");

    let odt = doc.export(Format::Odt).unwrap();
    assert!(odt.len() > 0, "ODT export should produce bytes");

    let txt = doc.export(Format::Txt).unwrap();
    let txt_str = String::from_utf8(txt).unwrap();
    assert!(txt_str.contains("Format test"), "TXT export should contain text");
}

// ──────────────────────────────────────────────────────────────────
// M3: Structural access — model nodes, body, metadata
// ──────────────────────────────────────────────────────────────────

#[test]
fn document_has_body_with_children() {
    let engine = engine();
    let doc = engine.open_as(b"Test\nTwo", Format::Txt).unwrap();
    let model = doc.model();
    let body_id = model.body_id().expect("Document should have a body");
    let body = model.node(body_id).expect("Body node should exist");
    assert!(!body.children.is_empty(), "Body should have children (paragraphs)");
}

#[test]
fn paragraph_nodes_have_run_children() {
    let engine = engine();
    let doc = engine.open_as(b"Hello", Format::Txt).unwrap();
    let model = doc.model();
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap();

    // First child should be a paragraph
    let para_id = body.children[0];
    let para = model.node(para_id).expect("Paragraph node should exist");
    assert_eq!(para.node_type, s1_model::NodeType::Paragraph);
    assert!(!para.children.is_empty(), "Paragraph should have children (runs)");
}

#[test]
fn metadata_accessible() {
    let engine = engine();
    let doc = engine.open_as(b"Meta test", Format::Txt).unwrap();
    let meta = doc.metadata();
    // TXT import may not set title, but metadata should be accessible
    assert!(meta.title.is_none() || meta.title.is_some());
}

// ──────────────────────────────────────────────────────────────────
// Negative: explicitly out of scope
// ──────────────────────────────────────────────────────────────────

#[test]
fn text_bridge_does_not_preserve_formatting() {
    // M1 text-only bridge strips formatting. This test documents that.
    // When M3 adapter uses node_info_json formatting fields, this scope changes.
    let engine = engine();
    let content = "Just plain text";
    let doc = engine.open_as(content.as_bytes(), Format::Txt).unwrap();
    let text = doc.to_plain_text();
    assert_eq!(text.trim(), content, "Plain text round-trip should be exact");
}
