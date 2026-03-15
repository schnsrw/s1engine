//! Import/Export Fidelity Testing Suite (C.6)
//!
//! Comprehensive integration tests that exercise realistic document scenarios.
//! Each test creates a document programmatically, exports it, reimports it,
//! and verifies content/structure is preserved.

use s1engine::{
    AttributeKey, AttributeValue, Color, DocumentBuilder, Engine, Format, ListFormat, Node, NodeId,
    NodeType, SectionBreakType, SectionProperties,
};
use std::time::Instant;

// ─── Helper Functions ──────────────────────────────────────────────────────

/// Count all nodes of a given type anywhere in the document tree.
fn count_nodes_of_type(doc: &s1engine::Document, node_type: NodeType) -> usize {
    let model = doc.model();
    let root_id = model.root_id();
    count_nodes_recursive(model, root_id, node_type)
}

fn count_nodes_recursive(
    model: &s1engine::DocumentModel,
    node_id: NodeId,
    target: NodeType,
) -> usize {
    let mut count = 0;
    if let Some(node) = model.node(node_id) {
        if node.node_type == target {
            count += 1;
        }
        for &child_id in &node.children {
            count += count_nodes_recursive(model, child_id, target);
        }
    }
    count
}

/// Collect all nodes of a given type in tree order.
fn collect_nodes_of_type(doc: &s1engine::Document, node_type: NodeType) -> Vec<NodeId> {
    let model = doc.model();
    let root_id = model.root_id();
    let mut result = Vec::new();
    collect_nodes_recursive(model, root_id, node_type, &mut result);
    result
}

fn collect_nodes_recursive(
    model: &s1engine::DocumentModel,
    node_id: NodeId,
    target: NodeType,
    result: &mut Vec<NodeId>,
) {
    if let Some(node) = model.node(node_id) {
        if node.node_type == target {
            result.push(node_id);
        }
        for &child_id in &node.children {
            collect_nodes_recursive(model, child_id, target, result);
        }
    }
}

// ─── Test 1: Complex Formatting Round-Trip ─────────────────────────────────

#[test]
fn test_complex_formatting_roundtrip() {
    // Create a document with varied formatting
    let doc = DocumentBuilder::new()
        .heading(1, "Document Title")
        .paragraph(|p| {
            p.bold("Bold text")
                .italic(" italic text")
                .underline(" underlined text")
        })
        .paragraph(|p| {
            p.colored("Red text", Color::RED)
                .styled(" Custom font", "Courier New", 16.0)
        })
        .build();

    let original_text = doc.to_plain_text();
    assert!(original_text.contains("Document Title"));
    assert!(original_text.contains("Bold text"));
    assert!(original_text.contains("Red text"));

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text is fully preserved
    assert_eq!(
        doc2.to_plain_text(),
        original_text,
        "Text must survive DOCX round-trip"
    );

    // Verify heading style survived
    let body_id = doc2.body_id().unwrap();
    let body = doc2.node(body_id).unwrap();

    // First child is the heading paragraph
    let heading_para = doc2.node(body.children[0]).unwrap();
    assert_eq!(heading_para.node_type, NodeType::Paragraph);

    // Second child: paragraph with bold, italic, underline runs
    let format_para = doc2.node(body.children[1]).unwrap();
    assert!(
        format_para.children.len() >= 3,
        "Formatting paragraph should have at least 3 runs"
    );

    // Check bold on first run
    let bold_run = doc2.node(format_para.children[0]).unwrap();
    assert_eq!(
        bold_run.attributes.get_bool(&AttributeKey::Bold),
        Some(true),
        "Bold attribute must survive round-trip"
    );

    // Check italic on second run
    let italic_run = doc2.node(format_para.children[1]).unwrap();
    assert_eq!(
        italic_run.attributes.get_bool(&AttributeKey::Italic),
        Some(true),
        "Italic attribute must survive round-trip"
    );

    // Third child: paragraph with color + font
    let styled_para = doc2.node(body.children[2]).unwrap();
    let red_run = doc2.node(styled_para.children[0]).unwrap();
    assert_eq!(
        red_run.attributes.get_color(&AttributeKey::Color),
        Some(Color::RED),
        "Color attribute must survive round-trip"
    );
}

// ─── Test 2: Nested Table Round-Trip ───────────────────────────────────────

#[test]
fn test_nested_table_roundtrip() {
    let doc = DocumentBuilder::new()
        .table(|t| {
            t.row(|r| {
                r.rich_cell(|p| p.bold("Header 1"))
                    .rich_cell(|p| p.italic("Header 2"))
                    .cell("Header 3")
            })
            .row(|r| {
                r.cell("Row 1 Col 1")
                    .cell("Row 1 Col 2")
                    .cell("Row 1 Col 3")
            })
            .row(|r| {
                r.cell("Row 2 Col 1")
                    .cell("Row 2 Col 2")
                    .cell("Row 2 Col 3")
            })
        })
        .build();

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text content preserved
    let reimported_text = doc2.to_plain_text();
    assert!(
        reimported_text.contains("Header 1"),
        "Table header text must survive"
    );
    assert!(
        reimported_text.contains("Row 2 Col 3"),
        "Table cell text must survive"
    );

    // Verify table structure
    let tables = count_nodes_of_type(&doc2, NodeType::Table);
    assert_eq!(tables, 1, "Should have exactly 1 table");

    let rows = count_nodes_of_type(&doc2, NodeType::TableRow);
    assert_eq!(rows, 3, "Should have 3 rows");

    let cells = count_nodes_of_type(&doc2, NodeType::TableCell);
    assert_eq!(cells, 9, "Should have 9 cells (3x3)");

    // Verify rich cell formatting (bold in first cell)
    let body_id = doc2.body_id().unwrap();
    let body = doc2.node(body_id).unwrap();
    let table = doc2.node(body.children[0]).unwrap();
    let first_row = doc2.node(table.children[0]).unwrap();
    let first_cell = doc2.node(first_row.children[0]).unwrap();
    let cell_para = doc2.node(first_cell.children[0]).unwrap();
    let cell_run = doc2.node(cell_para.children[0]).unwrap();
    assert_eq!(
        cell_run.attributes.get_bool(&AttributeKey::Bold),
        Some(true),
        "Bold in table cell must survive round-trip"
    );
}

// ─── Test 3: Multi-Section Document ────────────────────────────────────────

#[test]
fn test_multi_section_document() {
    let mut section1 = SectionProperties::default();
    section1.page_width = 612.0; // Letter
    section1.page_height = 792.0;
    section1.margin_top = 90.0; // 1.25 inch
    section1.margin_left = 90.0;
    section1.break_type = Some(SectionBreakType::NextPage);

    let mut section2 = SectionProperties::default();
    section2.page_width = 842.0; // ~A4 width
    section2.page_height = 595.0; // landscape-like
    section2.margin_top = 54.0; // 0.75 inch

    let doc = DocumentBuilder::new()
        .paragraph(|p| p.text("Section 1 content, paragraph 1"))
        .paragraph(|p| p.text("Section 1 content, paragraph 2"))
        .section(section1)
        .paragraph(|p| p.text("Section 2 content, paragraph 1"))
        .paragraph(|p| p.text("Section 2 content, paragraph 2"))
        .section(section2)
        .build();

    assert_eq!(doc.sections().len(), 2);

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text preserved
    let text = doc2.to_plain_text();
    assert!(text.contains("Section 1 content, paragraph 1"));
    assert!(text.contains("Section 2 content, paragraph 2"));

    // Verify sections survived
    assert!(
        !doc2.sections().is_empty(),
        "Sections must survive DOCX round-trip"
    );

    // Verify section properties (margins, page sizes)
    let sections = doc2.sections();
    if sections.len() >= 2 {
        let s1 = &sections[0];
        assert!(
            (s1.margin_top - 90.0).abs() < 1.0,
            "Section 1 margin_top must survive; got {}",
            s1.margin_top
        );

        let s2 = &sections[1];
        assert!(
            (s2.margin_top - 54.0).abs() < 1.0,
            "Section 2 margin_top must survive; got {}",
            s2.margin_top
        );
    }
}

// ─── Test 4: All Heading Levels ────────────────────────────────────────────

#[test]
fn test_all_heading_levels() {
    let doc = DocumentBuilder::new()
        .heading(1, "Heading Level 1")
        .heading(2, "Heading Level 2")
        .heading(3, "Heading Level 3")
        .heading(4, "Heading Level 4")
        .heading(5, "Heading Level 5")
        .heading(6, "Heading Level 6")
        .build();

    let original_text = doc.to_plain_text();
    assert_eq!(doc.paragraph_count(), 6);

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify all heading text preserved
    assert_eq!(doc2.to_plain_text(), original_text);

    // Verify heading styles exist
    for level in 1..=6 {
        let style_id = format!("Heading{level}");
        assert!(
            doc2.style_by_id(&style_id).is_some(),
            "Heading{level} style must survive round-trip"
        );
    }

    // Verify each paragraph references its heading style
    let body_id = doc2.body_id().unwrap();
    let body = doc2.node(body_id).unwrap();
    for (i, &child_id) in body.children.iter().enumerate() {
        let para = doc2.node(child_id).unwrap();
        let style_id = para.attributes.get_string(&AttributeKey::StyleId);
        let expected = format!("Heading{}", i + 1);
        assert_eq!(
            style_id,
            Some(expected.as_str()),
            "Paragraph {} should reference {}",
            i,
            expected
        );
    }
}

// ─── Test 5: Comments Round-Trip ───────────────────────────────────────────

#[test]
fn test_comments_roundtrip() {
    // Comments aren't exposed via DocumentBuilder, so we build via model_mut
    let engine = Engine::new();
    let mut doc = engine.create();
    let model = doc.model_mut();

    let body_id = model.body_id().unwrap();

    // Create paragraph with comment range
    let para_id = model.next_id();
    model
        .insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
        .unwrap();

    // CommentStart
    let cs_id = model.next_id();
    let mut cs = Node::new(cs_id, NodeType::CommentStart);
    cs.attributes
        .set(AttributeKey::CommentId, AttributeValue::String("42".into()));
    model.insert_node(para_id, 0, cs).unwrap();

    // Run with text
    let run_id = model.next_id();
    model
        .insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
        .unwrap();
    let text_id = model.next_id();
    model
        .insert_node(run_id, 0, Node::text(text_id, "Commented text"))
        .unwrap();

    // CommentEnd
    let ce_id = model.next_id();
    let mut ce = Node::new(ce_id, NodeType::CommentEnd);
    ce.attributes
        .set(AttributeKey::CommentId, AttributeValue::String("42".into()));
    model.insert_node(para_id, 2, ce).unwrap();

    // Create CommentBody node on the document root
    let root_id = model.root_id();
    let root_children = model.node(root_id).unwrap().children.len();
    let cb_id = model.next_id();
    let mut cb = Node::new(cb_id, NodeType::CommentBody);
    cb.attributes
        .set(AttributeKey::CommentId, AttributeValue::String("42".into()));
    cb.attributes.set(
        AttributeKey::CommentAuthor,
        AttributeValue::String("Test Author".into()),
    );
    model.insert_node(root_id, root_children, cb).unwrap();

    let cp_id = model.next_id();
    model
        .insert_node(cb_id, 0, Node::new(cp_id, NodeType::Paragraph))
        .unwrap();
    let cr_id = model.next_id();
    model
        .insert_node(cp_id, 0, Node::new(cr_id, NodeType::Run))
        .unwrap();
    let ct_id = model.next_id();
    model
        .insert_node(cr_id, 0, Node::text(ct_id, "This is a comment"))
        .unwrap();

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify body text
    assert_eq!(doc2.to_plain_text(), "Commented text");

    // Verify comment start/end in body
    let comment_starts = count_nodes_of_type(&doc2, NodeType::CommentStart);
    assert!(comment_starts >= 1, "CommentStart must survive round-trip");

    let comment_ends = count_nodes_of_type(&doc2, NodeType::CommentEnd);
    assert!(comment_ends >= 1, "CommentEnd must survive round-trip");

    // Verify CommentBody
    let comment_bodies = count_nodes_of_type(&doc2, NodeType::CommentBody);
    assert!(comment_bodies >= 1, "CommentBody must survive round-trip");

    // Verify comment author
    let cb_nodes = collect_nodes_of_type(&doc2, NodeType::CommentBody);
    let cb_node = doc2.node(cb_nodes[0]).unwrap();
    assert_eq!(
        cb_node.attributes.get_string(&AttributeKey::CommentAuthor),
        Some("Test Author"),
        "Comment author must survive round-trip"
    );
}

// ─── Test 6: Large Document Performance ────────────────────────────────────

#[test]
fn test_large_document_performance() {
    let start = Instant::now();

    // Build a document with 100 paragraphs
    let mut builder = DocumentBuilder::new()
        .title("Performance Test Document")
        .heading(1, "Large Document");

    for i in 0..100 {
        let text = format!(
            "This is paragraph number {} with enough content to make it realistic. \
             Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod \
             tempor incididunt ut labore et dolore magna aliqua.",
            i
        );
        builder = builder.text(&text);
    }

    let doc = builder.build();
    let build_elapsed = start.elapsed();

    // Export to DOCX
    let export_start = Instant::now();
    let bytes = doc.export(Format::Docx).unwrap();
    let export_elapsed = export_start.elapsed();

    assert!(!bytes.is_empty(), "DOCX output must not be empty");

    // Reimport
    let import_start = Instant::now();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();
    let import_elapsed = import_start.elapsed();

    // Verify content
    let para_count = doc2.paragraph_count();
    assert!(
        para_count >= 100,
        "Should have at least 100 paragraphs; got {}",
        para_count
    );

    let total_elapsed = start.elapsed();
    assert!(
        total_elapsed.as_secs() < 5,
        "Total build+export+import for 100-paragraph doc must complete in < 5 seconds; took {:?}",
        total_elapsed
    );

    // Print timings for diagnostics (visible with --nocapture)
    eprintln!(
        "Performance: build={:?}, export={:?} ({} bytes), import={:?}, total={:?}",
        build_elapsed,
        export_elapsed,
        bytes.len(),
        import_elapsed,
        total_elapsed
    );
}

// ─── Test 7: Cross-Format DOCX to ODT ─────────────────────────────────────

#[test]
#[cfg(feature = "odt")]
fn test_cross_format_docx_to_odt() {
    let doc = DocumentBuilder::new()
        .heading(1, "Cross-Format Title")
        .paragraph(|p| p.text("First paragraph with plain text."))
        .paragraph(|p| p.bold("Bold ").italic("italic ").text("normal"))
        .table(|t| {
            t.row(|r| r.cell("Cell A").cell("Cell B"))
                .row(|r| r.cell("Cell C").cell("Cell D"))
        })
        .bullet("Bullet item")
        .numbered("Numbered item")
        .build();

    // Step 1: Export to DOCX
    let docx_bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let from_docx = engine.open(&docx_bytes).unwrap();

    // Step 2: Export to ODT
    let odt_bytes = from_docx.export(Format::Odt).unwrap();
    assert!(!odt_bytes.is_empty(), "ODT output must not be empty");

    // Step 3: Reimport from ODT
    let from_odt = engine.open_as(&odt_bytes, Format::Odt).unwrap();

    // Verify text content
    let odt_text = from_odt.to_plain_text();
    assert!(
        odt_text.contains("Cross-Format Title"),
        "Title must survive DOCX -> ODT; got: {}",
        odt_text
    );
    assert!(
        odt_text.contains("First paragraph with plain text."),
        "Paragraph text must survive DOCX -> ODT"
    );
    assert!(
        odt_text.contains("Bold"),
        "Bold text must survive DOCX -> ODT"
    );
    assert!(
        odt_text.contains("Cell A"),
        "Table content must survive DOCX -> ODT"
    );
    assert!(
        odt_text.contains("Bullet item"),
        "List text must survive DOCX -> ODT"
    );

    // Verify table structure in ODT
    let tables = count_nodes_of_type(&from_odt, NodeType::Table);
    assert!(tables >= 1, "Table must survive DOCX -> ODT conversion");
}

// ─── Test 8: Unicode Text Round-Trip ───────────────────────────────────────

#[test]
fn test_unicode_text_roundtrip() {
    let doc = DocumentBuilder::new()
        .paragraph(|p| p.text("\u{4e16}\u{754c}\u{4f60}\u{597d}")) // Chinese: 世界你好
        .paragraph(|p| p.text("\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}")) // Japanese: こんにちは
        .paragraph(|p| p.text("\u{c548}\u{b155}\u{d558}\u{c138}\u{c694}")) // Korean: 안녕하세요
        .paragraph(|p| p.text("\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}")) // Arabic: مرحبا
        .paragraph(|p| p.text("\u{1F600}\u{1F60D}\u{1F4DA}")) // Emoji: 😀😍📚
        .paragraph(|p| p.text("caf\u{00e9} na\u{00ef}ve r\u{00e9}sum\u{00e9}")) // Accented
        .paragraph(|p| p.text("\u{0410}\u{0411}\u{0412}\u{0413}")) // Cyrillic: АБВГ
        .build();

    let original_text = doc.to_plain_text();
    assert!(!original_text.is_empty());

    // DOCX round-trip
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    assert_eq!(
        doc2.to_plain_text(),
        original_text,
        "Unicode text must survive DOCX round-trip exactly"
    );

    // Verify specific substrings
    let text = doc2.to_plain_text();
    assert!(text.contains("\u{4e16}\u{754c}"), "CJK must survive");
    assert!(
        text.contains("\u{0645}\u{0631}\u{062d}"),
        "Arabic must survive"
    );
    assert!(text.contains("\u{1F600}"), "Emoji must survive");
    assert!(text.contains("caf\u{00e9}"), "Accented text must survive");
}

// ─── Test 9: Nested Lists Round-Trip ───────────────────────────────────────

#[test]
fn test_nested_lists_roundtrip() {
    let doc = DocumentBuilder::new()
        .bullet("First bullet")
        .bullet("Second bullet")
        .bullet("Third bullet")
        .numbered("First numbered")
        .numbered("Second numbered")
        .numbered("Third numbered")
        .build();

    let original_text = doc.to_plain_text();

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text preserved
    assert_eq!(doc2.to_plain_text(), original_text);

    // Verify numbering definitions survived
    assert!(
        !doc2.numbering().is_empty(),
        "Numbering definitions must survive DOCX round-trip"
    );

    // Verify ListInfo on paragraphs
    let body_id = doc2.body_id().unwrap();
    let body = doc2.node(body_id).unwrap();

    // Check first paragraph has bullet ListInfo
    let first_para = doc2.node(body.children[0]).unwrap();
    let list_info = first_para.attributes.get(&AttributeKey::ListInfo);
    assert!(
        list_info.is_some(),
        "First bullet paragraph must have ListInfo after round-trip"
    );

    if let Some(AttributeValue::ListInfo(info)) = list_info {
        assert_eq!(
            info.num_format,
            ListFormat::Bullet,
            "First list item must be a bullet"
        );
    }

    // Check fourth paragraph has numbered ListInfo
    let fourth_para = doc2.node(body.children[3]).unwrap();
    let list_info = fourth_para.attributes.get(&AttributeKey::ListInfo);
    assert!(
        list_info.is_some(),
        "First numbered paragraph must have ListInfo after round-trip"
    );

    if let Some(AttributeValue::ListInfo(info)) = list_info {
        assert_eq!(
            info.num_format,
            ListFormat::Decimal,
            "Fourth list item must be decimal numbered"
        );
    }
}

// ─── Test 10: Images Round-Trip ────────────────────────────────────────────

#[test]
fn test_images_roundtrip() {
    // Build a document with an inline image via model_mut (no builder for images)
    let engine = Engine::new();
    let mut doc = engine.create();
    let model = doc.model_mut();

    let body_id = model.body_id().unwrap();

    // Create paragraph
    let para_id = model.next_id();
    model
        .insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
        .unwrap();

    // Create a minimal valid 1x1 red PNG
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // 8-bit RGB
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, // compressed data
        0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, // ...
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let media_id =
        model
            .media_mut()
            .insert("image/png", png_bytes.clone(), Some("test.png".to_string()));

    // Create image node
    let img_id = model.next_id();
    let mut img = Node::new(img_id, NodeType::Image);
    img.attributes.set(
        AttributeKey::ImageMediaId,
        AttributeValue::MediaId(media_id),
    );
    img.attributes
        .set(AttributeKey::ImageWidth, AttributeValue::Float(100.0));
    img.attributes
        .set(AttributeKey::ImageHeight, AttributeValue::Float(100.0));
    img.attributes.set(
        AttributeKey::ImageAltText,
        AttributeValue::String("Test image".into()),
    );
    model.insert_node(para_id, 0, img).unwrap();

    // Also add a text run after the image
    let run_id = model.next_id();
    model
        .insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
        .unwrap();
    let text_id = model.next_id();
    model
        .insert_node(run_id, 0, Node::text(text_id, "Image caption"))
        .unwrap();

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text survived
    assert!(doc2.to_plain_text().contains("Image caption"));

    // Verify image node survived
    let images = count_nodes_of_type(&doc2, NodeType::Image);
    assert!(images >= 1, "Image node must survive DOCX round-trip");

    // Verify image has a media reference
    let image_nodes = collect_nodes_of_type(&doc2, NodeType::Image);
    let img_node = doc2.node(image_nodes[0]).unwrap();
    assert!(
        img_node
            .attributes
            .get(&AttributeKey::ImageMediaId)
            .is_some(),
        "Image must retain its media reference after round-trip"
    );
}

// ─── Test 11: Hyperlinks and Bookmarks Round-Trip ──────────────────────────

#[test]
fn test_hyperlinks_and_bookmarks_roundtrip() {
    let doc = DocumentBuilder::new()
        .paragraph(|p| {
            p.text("Visit ")
                .hyperlink("https://example.com", "Example Site")
                .text(" for more info.")
        })
        .paragraph(|p| {
            p.bookmark_start("section_one")
                .text("This is the bookmarked section.")
                .bookmark_end()
        })
        .paragraph(|p| {
            p.hyperlink("https://rust-lang.org", "Rust")
                .text(" is great.")
        })
        .build();

    let original_text = doc.to_plain_text();

    // Export to DOCX and reimport
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text preserved
    assert_eq!(doc2.to_plain_text(), original_text);

    // Verify hyperlinks
    let body_id = doc2.body_id().unwrap();
    let body = doc2.node(body_id).unwrap();

    // First paragraph: look for HyperlinkUrl attribute on a run
    let para1 = doc2.node(body.children[0]).unwrap();
    let mut found_hyperlink = false;
    for &child_id in &para1.children {
        if let Some(child) = doc2.node(child_id) {
            if child
                .attributes
                .get_string(&AttributeKey::HyperlinkUrl)
                .is_some()
            {
                found_hyperlink = true;
                assert_eq!(
                    child
                        .attributes
                        .get_string(&AttributeKey::HyperlinkUrl)
                        .unwrap(),
                    "https://example.com",
                    "Hyperlink URL must survive round-trip"
                );
            }
        }
    }
    assert!(found_hyperlink, "Hyperlink must survive DOCX round-trip");

    // Verify bookmarks
    let bookmarks_start = count_nodes_of_type(&doc2, NodeType::BookmarkStart);
    assert!(
        bookmarks_start >= 1,
        "BookmarkStart must survive DOCX round-trip"
    );

    let bookmarks_end = count_nodes_of_type(&doc2, NodeType::BookmarkEnd);
    assert!(
        bookmarks_end >= 1,
        "BookmarkEnd must survive DOCX round-trip"
    );

    // Verify bookmark name
    let bk_nodes = collect_nodes_of_type(&doc2, NodeType::BookmarkStart);
    let bk = doc2.node(bk_nodes[0]).unwrap();
    assert_eq!(
        bk.attributes.get_string(&AttributeKey::BookmarkName),
        Some("section_one"),
        "Bookmark name must survive round-trip"
    );
}

// ─── Test 12: Mixed Content Stress Test ────────────────────────────────────

#[test]
fn test_mixed_content_stress() {
    // Build a document with ALL content types mixed together
    let engine = Engine::new();
    let doc_builder = DocumentBuilder::new()
        .title("Stress Test Document")
        .author("Test Suite")
        .heading(1, "Introduction")
        .paragraph(|p| {
            p.text("This document tests ")
                .bold("all content types")
                .text(" together.")
        })
        .heading(2, "Formatting Examples")
        .paragraph(|p| {
            p.bold("Bold")
                .text(", ")
                .italic("italic")
                .text(", ")
                .underline("underlined")
                .text(", ")
                .bold_italic("bold italic")
                .text(", ")
                .superscript("super")
                .text(", ")
                .subscript("sub")
        })
        .heading(2, "Tables")
        .table(|t| {
            t.row(|r| r.cell("Name").cell("Value").cell("Description"))
                .row(|r| r.cell("Alpha").cell("1").cell("First item"))
                .row(|r| r.cell("Beta").cell("2").cell("Second item"))
        })
        .heading(2, "Lists")
        .bullet("First bullet point")
        .bullet("Second bullet point")
        .numbered("Step one")
        .numbered("Step two")
        .heading(2, "Links and References")
        .paragraph(|p| {
            p.hyperlink("https://example.com", "Example Link")
                .text(" | ")
                .bookmark_start("ref1")
                .text("Referenced text")
                .bookmark_end()
        })
        .heading(2, "Unicode Content")
        .paragraph(|p| {
            p.text("caf\u{00e9} \u{4e16}\u{754c} \u{0645}\u{0631}\u{062d}\u{0628}\u{0627}")
        });

    let doc = doc_builder.build();

    let original_text = doc.to_plain_text();
    let original_table_count = count_nodes_of_type(&doc, NodeType::Table);

    // Export to DOCX
    let bytes = doc.export(Format::Docx).unwrap();
    assert!(!bytes.is_empty());

    // Reimport
    let doc2 = engine.open(&bytes).unwrap();

    // Verify text is fully preserved
    assert_eq!(
        doc2.to_plain_text(),
        original_text,
        "All text must survive mixed-content DOCX round-trip"
    );

    // Verify structural counts
    let reimported_tables = count_nodes_of_type(&doc2, NodeType::Table);
    assert_eq!(
        reimported_tables, original_table_count,
        "Table count must match"
    );

    // Verify metadata
    assert_eq!(
        doc2.metadata().title.as_deref(),
        Some("Stress Test Document"),
        "Title must survive round-trip"
    );
    assert_eq!(
        doc2.metadata().creator.as_deref(),
        Some("Test Suite"),
        "Author must survive round-trip"
    );

    // Verify formatting survived (check bold on a run)
    let body_id = doc2.body_id().unwrap();
    let body = doc2.node(body_id).unwrap();

    // The "Formatting Examples" content is in the 4th body child (index 3)
    // (heading1, para, heading2, formatting_para)
    // Find a paragraph containing "Bold"
    let mut found_bold = false;
    for &child_id in &body.children {
        if let Some(child) = doc2.node(child_id) {
            if child.node_type == NodeType::Paragraph {
                for &run_id in &child.children {
                    if let Some(run) = doc2.node(run_id) {
                        if run.node_type == NodeType::Run
                            && run.attributes.get_bool(&AttributeKey::Bold) == Some(true)
                        {
                            found_bold = true;
                        }
                    }
                }
            }
        }
    }
    assert!(
        found_bold,
        "Bold formatting must survive mixed-content round-trip"
    );

    // Verify hyperlinks survived
    let mut found_link = false;
    for &child_id in &body.children {
        if let Some(para) = doc2.node(child_id) {
            for &run_id in &para.children {
                if let Some(run) = doc2.node(run_id) {
                    if run
                        .attributes
                        .get_string(&AttributeKey::HyperlinkUrl)
                        .is_some()
                    {
                        found_link = true;
                    }
                }
            }
        }
    }
    assert!(
        found_link,
        "Hyperlinks must survive mixed-content round-trip"
    );

    // Verify bookmarks survived
    assert!(
        count_nodes_of_type(&doc2, NodeType::BookmarkStart) >= 1,
        "Bookmarks must survive mixed-content round-trip"
    );

    // Verify lists survived
    let mut found_list = false;
    for &child_id in &body.children {
        if let Some(para) = doc2.node(child_id) {
            if para.attributes.get(&AttributeKey::ListInfo).is_some() {
                found_list = true;
                break;
            }
        }
    }
    assert!(found_list, "Lists must survive mixed-content round-trip");

    // Cross-format: also verify ODT export works for the whole thing
    #[cfg(feature = "odt")]
    {
        let odt_bytes = doc2.export(Format::Odt).unwrap();
        let from_odt = engine.open_as(&odt_bytes, Format::Odt).unwrap();
        let odt_text = from_odt.to_plain_text();
        assert!(
            odt_text.contains("Introduction"),
            "Heading text must survive DOCX -> ODT"
        );
        assert!(
            odt_text.contains("Alpha"),
            "Table text must survive DOCX -> ODT"
        );
        assert!(
            odt_text.contains("caf\u{00e9}"),
            "Unicode text must survive DOCX -> ODT"
        );
    }
}
