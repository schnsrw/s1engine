//! M8: Fidelity validation tests for the web bridge.
//!
//! These tests validate the open/edit/save/re-open round-trip
//! using committed fixture files.

use s1engine::{Engine, Format};
use s1_model::{NodeType, AttributeKey};
use std::path::Path;

fn engine() -> Engine {
    Engine::new()
}

fn load_fixture(name: &str) -> Vec<u8> {
    let path = Path::new("tests/fixtures").join(name);
    if !path.exists() {
        panic!(
            "Fixture {} not found. Run: cargo test -p s1engine --test create_fixtures -- --ignored",
            path.display()
        );
    }
    std::fs::read(path).unwrap()
}

// ──────────────────────────────────────────────────────────────────
// Text-only fixture round-trip
// ──────────────────────────────────────────────────────────────────

#[test]
fn fidelity_text_only_open() {
    let bytes = load_fixture("text-only.docx");
    let doc = engine().open(&bytes).unwrap();
    let text = doc.to_plain_text();
    assert!(text.contains("First paragraph"), "text={:?}", text);
    assert!(text.contains("Second paragraph"));
    assert!(text.contains("Third paragraph"));
}

#[test]
fn fidelity_text_only_paragraph_count() {
    let bytes = load_fixture("text-only.docx");
    let doc = engine().open(&bytes).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let para_count = body.children.iter()
        .filter(|id| model.node(**id).map_or(false, |n| n.node_type == NodeType::Paragraph))
        .count();
    assert!(para_count >= 3, "Expected 3+ paragraphs, got {}", para_count);
}

#[test]
fn fidelity_text_only_round_trip() {
    let bytes = load_fixture("text-only.docx");
    let doc = engine().open(&bytes).unwrap();
    let exported = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&exported).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("First paragraph"));
    assert!(text.contains("Second paragraph"));
    assert!(text.contains("Third paragraph"));
}

// ──────────────────────────────────────────────────────────────────
// Formatted fixture
// ──────────────────────────────────────────────────────────────────

#[test]
fn fidelity_formatted_open() {
    let bytes = load_fixture("formatted.docx");
    let doc = engine().open(&bytes).unwrap();
    let text = doc.to_plain_text();
    assert!(text.contains("Heading One"), "text={:?}", text);
    assert!(text.contains("Normal paragraph"));
    assert!(text.contains("Heading Two"));
    assert!(text.contains("Bold text"));
    assert!(text.contains("italic text"));
}

#[test]
fn fidelity_formatted_heading_style() {
    let bytes = load_fixture("formatted.docx");
    let doc = engine().open(&bytes).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();
    let first_para = model.node(body.children[0]).unwrap();
    let style = first_para.attributes.get_string(&AttributeKey::StyleId);
    assert!(style.is_some(), "First paragraph should have a heading style");
}

#[test]
fn fidelity_formatted_bold_exists() {
    let bytes = load_fixture("formatted.docx");
    let doc = engine().open(&bytes).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();

    let has_bold = body.children.iter().any(|pid| {
        let para = model.node(*pid).unwrap();
        para.children.iter().any(|rid| {
            model.node(*rid).map_or(false, |r| {
                r.node_type == NodeType::Run && r.attributes.get_bool(&AttributeKey::Bold) == Some(true)
            })
        })
    });
    assert!(has_bold, "Formatted fixture should contain bold text");
}

#[test]
fn fidelity_formatted_italic_exists() {
    let bytes = load_fixture("formatted.docx");
    let doc = engine().open(&bytes).unwrap();
    let model = doc.model();
    let body = model.node(model.body_id().unwrap()).unwrap();

    let has_italic = body.children.iter().any(|pid| {
        let para = model.node(*pid).unwrap();
        para.children.iter().any(|rid| {
            model.node(*rid).map_or(false, |r| {
                r.node_type == NodeType::Run && r.attributes.get_bool(&AttributeKey::Italic) == Some(true)
            })
        })
    });
    assert!(has_italic, "Formatted fixture should contain italic text");
}

#[test]
fn fidelity_formatted_round_trip() {
    let bytes = load_fixture("formatted.docx");
    let doc = engine().open(&bytes).unwrap();
    let exported = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&exported).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Heading One"));
    assert!(text.contains("Bold text"));
    assert!(text.contains("italic text"));
}

#[test]
fn fidelity_formatted_round_trip_preserves_bold() {
    let bytes = load_fixture("formatted.docx");
    let doc = engine().open(&bytes).unwrap();
    let exported = doc.export(Format::Docx).unwrap();
    let doc2 = engine().open(&exported).unwrap();
    let model = doc2.model();
    let body = model.node(model.body_id().unwrap()).unwrap();

    let has_bold = body.children.iter().any(|pid| {
        let para = model.node(*pid).unwrap();
        para.children.iter().any(|rid| {
            model.node(*rid).map_or(false, |r| {
                r.node_type == NodeType::Run && r.attributes.get_bool(&AttributeKey::Bold) == Some(true)
            })
        })
    });
    assert!(has_bold, "Bold should survive DOCX round-trip");
}
