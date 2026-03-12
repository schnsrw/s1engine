//! Invariant tests -- test global correctness properties, not just individual features.
//!
//! These tests verify guarantees that must hold regardless of input:
//! - undo/redo reversibility
//! - tree integrity after operations
//! - cross-format text preservation
//! - builder output consistency

use s1engine::{DocumentBuilder, Engine, Format, Operation};

// ─── Undo/Redo Invariants ───────────────────────────────────────────

#[test]
fn undo_redo_preserves_text_across_multiple_edits() {
    let engine = Engine::new();
    let mut doc = engine.create();
    let body_id = doc.body_id().unwrap();

    // Build initial content via operations
    let para_id = doc.next_id();
    doc.apply(Operation::insert_node(
        body_id,
        0,
        s1engine::Node::new(para_id, s1engine::NodeType::Paragraph),
    ))
    .unwrap();

    let run_id = doc.next_id();
    doc.apply(Operation::insert_node(
        para_id,
        0,
        s1engine::Node::new(run_id, s1engine::NodeType::Run),
    ))
    .unwrap();

    let text_id = doc.next_id();
    doc.apply(Operation::insert_node(
        run_id,
        0,
        s1engine::Node::text(text_id, "Hello"),
    ))
    .unwrap();

    // Apply a series of text edits
    let snapshots: Vec<String> = vec![doc.to_plain_text()];

    doc.apply(Operation::insert_text(text_id, 5, " World"))
        .unwrap();
    let snap1 = doc.to_plain_text();

    doc.apply(Operation::insert_text(text_id, 11, "!"))
        .unwrap();
    let snap2 = doc.to_plain_text();

    doc.apply(Operation::delete_text(text_id, 0, 6)).unwrap();
    let snap3 = doc.to_plain_text();

    assert_eq!(snap1, "Hello World");
    assert_eq!(snap2, "Hello World!");
    assert_eq!(snap3, "World!");

    // Undo all three edits in reverse order
    doc.undo().unwrap();
    assert_eq!(doc.to_plain_text(), snap2);

    doc.undo().unwrap();
    assert_eq!(doc.to_plain_text(), snap1);

    doc.undo().unwrap();
    assert_eq!(doc.to_plain_text(), snapshots[0]);

    // Redo all three
    doc.redo().unwrap();
    assert_eq!(doc.to_plain_text(), snap1);

    doc.redo().unwrap();
    assert_eq!(doc.to_plain_text(), snap2);

    doc.redo().unwrap();
    assert_eq!(doc.to_plain_text(), snap3);
}

#[test]
fn undo_after_attribute_change_restores_exactly() {
    let mut doc = DocumentBuilder::new()
        .paragraph(|p| p.bold("Hello"))
        .build();

    let body_id = doc.body_id().unwrap();
    let body = doc.node(body_id).unwrap();
    let para_id = body.children[0];
    let para = doc.node(para_id).unwrap();
    let run_id = para.children[0];

    // Snapshot before
    let before = doc.node(run_id).unwrap().attributes.clone();

    // Set italic + change font size
    let attrs = s1engine::AttributeMap::new().italic(true).font_size(24.0);
    doc.apply(Operation::set_attributes(run_id, attrs)).unwrap();

    // Verify changed
    let changed = doc.node(run_id).unwrap();
    assert_eq!(
        changed.attributes.get_bool(&s1engine::AttributeKey::Italic),
        Some(true)
    );

    // Undo
    doc.undo().unwrap();
    let after = doc.node(run_id).unwrap().attributes.clone();
    assert_eq!(before, after, "attributes must be exactly restored after undo");
}

// ─── Cross-Format Text Preservation ─────────────────────────────────

#[test]
fn docx_roundtrip_preserves_text() {
    let doc = DocumentBuilder::new()
        .heading(1, "Title")
        .paragraph(|p| p.text("First paragraph with some text."))
        .paragraph(|p| p.bold("Bold ").italic("italic ").text("normal"))
        .bullet("Item A")
        .bullet("Item B")
        .build();

    let original_text = doc.to_plain_text();
    assert!(!original_text.is_empty());

    // Export to DOCX
    let docx_bytes = doc.export(Format::Docx).unwrap();
    assert!(!docx_bytes.is_empty());

    // Re-open
    let engine = Engine::new();
    let reopened = engine.open(&docx_bytes).unwrap();
    let roundtrip_text = reopened.to_plain_text();

    assert_eq!(original_text, roundtrip_text, "DOCX round-trip must preserve text");
}

#[test]
fn odt_roundtrip_preserves_text() {
    let doc = DocumentBuilder::new()
        .heading(1, "ODT Title")
        .paragraph(|p| p.text("Content here."))
        .build();

    let original_text = doc.to_plain_text();

    let odt_bytes = doc.export(Format::Odt).unwrap();
    let engine = Engine::new();
    let reopened = engine.open(&odt_bytes).unwrap();

    assert_eq!(original_text, reopened.to_plain_text(), "ODT round-trip must preserve text");
}

#[test]
fn txt_roundtrip_preserves_text() {
    let doc = DocumentBuilder::new()
        .text("Line one")
        .text("Line two")
        .text("Line three")
        .build();

    let original = doc.to_plain_text();
    let txt_bytes = doc.export(Format::Txt).unwrap();

    let engine = Engine::new();
    let reopened = engine.open_as(&txt_bytes, Format::Txt).unwrap();

    // TXT may add trailing newlines, so compare trimmed
    assert_eq!(
        original.trim(),
        reopened.to_plain_text().trim(),
        "TXT round-trip must preserve text"
    );
}

#[test]
fn cross_format_docx_to_odt_preserves_text() {
    let doc = DocumentBuilder::new()
        .heading(1, "Cross-Format Test")
        .paragraph(|p| p.text("This should survive DOCX -> ODT conversion."))
        .build();

    let original_text = doc.to_plain_text();

    // DOCX -> bytes -> model -> ODT -> bytes -> model
    let docx_bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let from_docx = engine.open(&docx_bytes).unwrap();
    let odt_bytes = from_docx.export(Format::Odt).unwrap();
    let from_odt = engine.open(&odt_bytes).unwrap();

    assert_eq!(
        original_text,
        from_odt.to_plain_text(),
        "DOCX -> ODT conversion must preserve text"
    );
}

// ─── Builder Output Invariants ──────────────────────────────────────

#[test]
fn builder_always_produces_valid_tree() {
    let doc = DocumentBuilder::new()
        .title("Test")
        .author("Author")
        .heading(1, "H1")
        .heading(2, "H2")
        .paragraph(|p| p.text("Normal"))
        .table(|t| {
            t.row(|r| r.cell("A").cell("B"))
                .row(|r| r.cell("C").cell("D"))
        })
        .bullet("Bullet")
        .numbered("Number")
        .build();

    let model = doc.model();

    // Root exists
    assert!(model.node(s1engine::NodeId::ROOT).is_some());

    // Body exists
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap();

    // All children have correct parent
    for &child_id in &body.children {
        let child = model.node(child_id).unwrap();
        assert_eq!(child.parent, Some(body_id));
    }

    // Paragraph count matches expected
    assert!(doc.paragraph_count() > 0);

    // Metadata is set
    assert_eq!(doc.metadata().title.as_deref(), Some("Test"));
    assert_eq!(doc.metadata().creator.as_deref(), Some("Author"));
}

#[test]
fn builder_output_exports_to_all_formats() {
    let doc = DocumentBuilder::new()
        .heading(1, "Export Test")
        .paragraph(|p| p.text("Content"))
        .build();

    // DOCX
    let docx = doc.export(Format::Docx);
    assert!(docx.is_ok(), "DOCX export must succeed");
    assert!(!docx.unwrap().is_empty());

    // ODT
    let odt = doc.export(Format::Odt);
    assert!(odt.is_ok(), "ODT export must succeed");
    assert!(!odt.unwrap().is_empty());

    // TXT
    let txt = doc.export(Format::Txt);
    assert!(txt.is_ok(), "TXT export must succeed");
    assert!(!txt.unwrap().is_empty());
}

// ─── Tree Integrity After Operations ────────────────────────────────

#[test]
fn tree_integrity_after_delete_undo_redo_cycle() {
    let mut doc = DocumentBuilder::new()
        .paragraph(|p| p.text("Para 1"))
        .paragraph(|p| p.text("Para 2"))
        .paragraph(|p| p.text("Para 3"))
        .build();

    let body_id = doc.body_id().unwrap();
    let para_ids = doc.paragraph_ids();
    assert_eq!(para_ids.len(), 3);

    let initial_count = doc.model().node_count();

    // Delete middle paragraph
    doc.apply(Operation::delete_node(para_ids[1])).unwrap();
    assert_eq!(doc.paragraph_ids().len(), 2);

    // Undo
    doc.undo().unwrap();
    assert_eq!(doc.paragraph_ids().len(), 3);
    assert_eq!(doc.model().node_count(), initial_count);

    // Verify parent-child consistency
    let body = doc.node(body_id).unwrap();
    for &child_id in &body.children {
        let child = doc.node(child_id).unwrap();
        assert_eq!(child.parent, Some(body_id));
    }

    // Redo
    doc.redo().unwrap();
    assert_eq!(doc.paragraph_ids().len(), 2);
}

// ─── Format Detection ───────────────────────────────────────────────

#[test]
fn format_detection_is_consistent() {
    let doc = DocumentBuilder::new()
        .paragraph(|p| p.text("Detect me"))
        .build();

    // DOCX bytes start with ZIP magic
    let docx = doc.export(Format::Docx).unwrap();
    assert_eq!(Format::detect(&docx), Format::Docx);

    // ODT bytes also start with ZIP magic, but detect should handle
    let odt = doc.export(Format::Odt).unwrap();
    let detected = Format::detect(&odt);
    // Both DOCX and ODT are ZIP files; detection may return either
    assert!(detected == Format::Docx || detected == Format::Odt);

    // TXT is detected as TXT
    let txt = doc.export(Format::Txt).unwrap();
    assert_eq!(Format::detect(&txt), Format::Txt);
}

// ─── Unicode Preservation ───────────────────────────────────────────

#[test]
fn unicode_text_survives_docx_roundtrip() {
    let doc = DocumentBuilder::new()
        .paragraph(|p| p.text("caf\u{00e9}"))
        .paragraph(|p| p.text("\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}")) // Arabic
        .paragraph(|p| p.text("\u{4e16}\u{754c}")) // CJK
        .paragraph(|p| p.text("na\u{00ef}ve r\u{00e9}sum\u{00e9}"))
        .build();

    let original = doc.to_plain_text();
    let docx = doc.export(Format::Docx).unwrap();

    let engine = Engine::new();
    let reopened = engine.open(&docx).unwrap();

    assert_eq!(original, reopened.to_plain_text(), "Unicode text must survive DOCX roundtrip");
}
