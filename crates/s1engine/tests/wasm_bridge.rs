//! Tests for the s1engine API surface used by the web adapter bridge.
//!
//! These validate the core engine functions that the WASM layer wraps.
//! Milestones: M1, M3 (structural import), M4 (structural export)

use s1engine::{Engine, Format};
use s1_model::{NodeType, AttributeKey};

fn engine() -> Engine {
    Engine::new()
}

// ──────────────────────────────────────────────────────────────────
// M1: Text-only bridge — open, extract text, create, export
// ──────────────────────────────────────────────────────────────────

#[test]
fn m1_create_empty_document_and_export() {
    let doc = engine().create();
    let bytes = doc.export(Format::Docx).unwrap();
    assert!(bytes.len() > 100, "Empty DOCX got {} bytes", bytes.len());
}

#[test]
fn m1_empty_document_plain_text_is_empty() {
    let doc = engine().create();
    assert!(doc.to_plain_text().trim().is_empty());
}

#[test]
fn m1_round_trip_docx_preserves_text() {
    let doc = engine().open_as(b"Hello world\nSecond paragraph\nThird line", Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Hello world"));
    assert!(text.contains("Second paragraph"));
    assert!(text.contains("Third line"));
}

#[test]
fn m1_multi_paragraph_extraction() {
    let doc = engine().open_as(b"Para one\nPara two\n\nPara four", Format::Txt).unwrap();
    let text = doc.to_plain_text();
    assert!(text.contains("Para one"));
    assert!(text.contains("Para two"));
    assert!(text.contains("Para four"));
}

#[test]
fn m1_export_multiple_formats() {
    let doc = engine().open_as(b"Format test", Format::Txt).unwrap();
    assert!(doc.export(Format::Docx).unwrap().len() > 0);
    assert!(doc.export(Format::Odt).unwrap().len() > 0);
    let txt = String::from_utf8(doc.export(Format::Txt).unwrap()).unwrap();
    assert!(txt.contains("Format test"));
}

// ──────────────────────────────────────────────────────────────────
// M3: Structural import — model nodes, body, formatting attributes
// ──────────────────────────────────────────────────────────────────

#[test]
fn m3_body_has_paragraph_children() {
    let doc = engine().open_as(b"Test\nTwo", Format::Txt).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    assert!(body.children.len() >= 2);
    let para = model.node(body.children[0]).unwrap();
    assert_eq!(para.node_type, NodeType::Paragraph);
}

#[test]
fn m3_paragraphs_have_run_children() {
    let doc = engine().open_as(b"Hello", Format::Txt).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();
    assert_eq!(para.node_type, NodeType::Paragraph);
    assert!(!para.children.is_empty(), "Paragraph should have run children");
    let run = model.node(para.children[0]).unwrap();
    assert_eq!(run.node_type, NodeType::Run);
}

#[test]
fn m3_run_has_text_children() {
    let doc = engine().open_as(b"Hello", Format::Txt).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();
    let run = model.node(para.children[0]).unwrap();
    assert!(!run.children.is_empty(), "Run should have text children");
    let text_node = model.node(run.children[0]).unwrap();
    assert_eq!(text_node.node_type, NodeType::Text);
    assert!(text_node.text_content.as_ref().unwrap().contains("Hello"));
}

#[test]
fn m3_node_info_formatting_attributes_exist() {
    // Verify the s1-model AttributeKey enum has the keys the adapter uses
    // These are checked by node_to_json in lib.rs
    let doc = engine().open_as(b"Test", Format::Txt).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();

    // Paragraph attributes the adapter reads
    let _ = para.attributes.get(&AttributeKey::Alignment);
    let _ = para.attributes.get(&AttributeKey::KeepLinesTogether);
    let _ = para.attributes.get(&AttributeKey::KeepWithNext);
    let _ = para.attributes.get(&AttributeKey::WidowControl);
    let _ = para.attributes.get(&AttributeKey::PageBreakBefore);

    let run = model.node(para.children[0]).unwrap();
    // Run attributes the adapter reads
    let _ = run.attributes.get(&AttributeKey::Bold);
    let _ = run.attributes.get(&AttributeKey::Italic);
    let _ = run.attributes.get(&AttributeKey::Underline);
    let _ = run.attributes.get(&AttributeKey::Strikethrough);
    let _ = run.attributes.get(&AttributeKey::FontSize);
    let _ = run.attributes.get(&AttributeKey::FontFamily);
    let _ = run.attributes.get(&AttributeKey::Color);
}

#[test]
fn m3_heading_has_style_id() {
    let doc = engine().open_as(b"# Heading One\n\nBody text", Format::Md).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();
    let style = para.attributes.get_string(&AttributeKey::StyleId);
    assert!(
        style.is_some(),
        "Heading paragraph should have StyleId attribute"
    );
}

// ──────────────────────────────────────────────────────────────────
// M4: Structural export — round-trip preservation
// ──────────────────────────────────────────────────────────────────

#[test]
fn m4_round_trip_paragraph_count() {
    let doc = engine().open_as(b"One\nTwo\nThree\nFour\nFive", Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&bytes).unwrap();
    let model = doc2.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para_count = body.children.iter()
        .filter(|id| model.node(**id).map_or(false, |n| n.node_type == NodeType::Paragraph))
        .count();
    assert!(para_count >= 5, "Round-trip should preserve at least 5 paragraphs, got {}", para_count);
}

#[test]
fn m4_round_trip_text_content() {
    let original = "First paragraph\nSecond paragraph\nThird paragraph";
    let doc = engine().open_as(original.as_bytes(), Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("First paragraph"));
    assert!(text.contains("Second paragraph"));
    assert!(text.contains("Third paragraph"));
}

#[test]
fn m4_heading_round_trip() {
    let doc = engine().open_as(b"# Title\n\nBody", Format::Md).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Title"), "Heading text should survive round-trip");
    assert!(text.contains("Body"), "Body text should survive round-trip");
}

#[test]
fn m4_metadata_accessible_after_export() {
    let doc = engine().open_as(b"Meta doc", Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&bytes).unwrap();
    let _ = doc2.metadata(); // should not panic
}

// ──────────────────────────────────────────────────────────────────
// M5: Run-level formatting, line breaks
// ──────────────────────────────────────────────────────────────────

#[test]
fn m5_bold_attribute_survives_round_trip() {
    let engine = engine();
    let doc = engine.open_as(b"**Bold text** normal", Format::Md).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine.open(&bytes).unwrap();
    let model = doc2.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();

    // Find a run with bold attribute
    let has_bold = para.children.iter().any(|cid| {
        model.node(*cid).map_or(false, |n| {
            n.node_type == NodeType::Run && n.attributes.get_bool(&AttributeKey::Bold) == Some(true)
        })
    });
    assert!(has_bold, "Bold attribute should survive DOCX round-trip");
}

#[test]
fn m5_italic_attribute_survives_round_trip() {
    let engine = engine();
    let doc = engine.open_as(b"*Italic text* normal", Format::Md).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine.open(&bytes).unwrap();
    let model = doc2.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();

    let has_italic = para.children.iter().any(|cid| {
        model.node(*cid).map_or(false, |n| {
            n.node_type == NodeType::Run && n.attributes.get_bool(&AttributeKey::Italic) == Some(true)
        })
    });
    assert!(has_italic, "Italic attribute should survive DOCX round-trip");
}

#[test]
fn m5_multiple_runs_in_paragraph() {
    let engine = engine();
    let doc = engine.open_as(b"Normal **bold** *italic*", Format::Md).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para = model.node(body.children[0]).unwrap();
    let run_count = para.children.iter()
        .filter(|cid| model.node(**cid).map_or(false, |n| n.node_type == NodeType::Run))
        .count();
    assert!(run_count >= 3, "Paragraph with mixed formatting should have multiple runs, got {}", run_count);
}

#[test]
fn m5_line_break_in_text() {
    let engine = engine();
    // Create a doc with text, export, check plain text has content
    let doc = engine.open_as(b"Line one\nLine two", Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine.open(&bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Line one"));
    assert!(text.contains("Line two"));
}

// ──────────────────────────────────────────────────────────────────
// Scope guards: explicitly document what is NOT supported yet
// ──────────────────────────────────────────────────────────────────

#[test]
fn scope_tables_not_yet_imported() {
    // Tables are M5 scope. This test documents the current limitation.
    let doc = engine().create();
    let text = doc.to_plain_text();
    assert!(text.trim().is_empty(), "Empty doc has no tables — M5 will add table support");
}

#[test]
fn scope_run_formatting_not_preserved_in_text_export() {
    // Run-level formatting (bold/italic/font) is not preserved in M4 save.
    // The save path extracts text only. M5 will add run-level export.
    let doc = engine().open_as(b"Bold and italic text", Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Bold and italic text"), "Text survives even without formatting");
}
