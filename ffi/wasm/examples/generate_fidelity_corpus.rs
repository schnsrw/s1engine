use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use s1engine::{
    AttributeKey, AttributeValue, Color, Document, DocumentBuilder, Format, ListFormat, Node,
    NodeType, PageOrientation, SectionBreakType, SectionProperties, TableWidth,
};
use s1engine_wasm::WasmEngine;
use wasm_bindgen::JsError;

struct CaseSpec {
    id: &'static str,
    source_doc: &'static str,
    layout_json: &'static str,
    page_map_json: &'static str,
    pdf_path: &'static str,
    build: fn() -> Document,
}

fn repeated_paragraph(seed: &str, index: usize) -> String {
    format!(
        "Paragraph {}. {} This content is intentionally long enough to exercise line wrapping, paragraph spacing, and page flow across multiple pages.",
        index + 1,
        seed
    )
}

// ─── Tier 1 Builders ────────────────────────────────────────────────────────

fn build_basic_paragraphs() -> Document {
    let mut builder = DocumentBuilder::new()
        .title("Tier 1 Basic Paragraphs")
        .author("s1engine fidelity generator")
        .heading(1, "Tier 1 Basic Paragraphs")
        .paragraph(|p| {
            p.text("This document exercises heading, paragraph, and inline formatting behavior. ")
                .bold("Bold text")
                .italic(" italic text")
                .underline(" underlined text")
        })
        .heading(2, "List Content")
        .bullet("First bullet item")
        .bullet("Second bullet item")
        .numbered("First numbered step")
        .numbered("Second numbered step");

    for i in 0..28 {
        let text = repeated_paragraph(
            "Tier 1 baseline paragraph for pagination and line-break validation.",
            i,
        );
        builder = builder.paragraph(|p| p.text(&text));
    }

    builder.build()
}

fn build_headers_footers() -> Document {
    let mut builder = DocumentBuilder::new()
        .title("Tier 1 Headers and Footers")
        .author("s1engine fidelity generator")
        .section_with_header_footer("Tier 1 Header", "Tier 1 Footer")
        .heading(1, "Headers and Footers")
        .paragraph(|p| {
            p.text("This document validates repeated page regions. ")
                .bold("Header and footer placement")
                .text(" should remain stable across multiple pages.")
        });

    for i in 0..34 {
        let text = repeated_paragraph(
            "Header/footer validation content intended to span enough pages to make repeated regions observable.",
            i,
        );
        builder = builder.paragraph(|p| p.text(&text));
    }

    builder.build()
}

// ─── Tier 2 Builders ────────────────────────────────────────────────────────

/// Build a 4x4 table with merged cells, header row, and cell backgrounds.
fn build_tables_merged() -> Document {
    let mut doc = DocumentBuilder::new()
        .title("Tier 2 Tables with Merged Cells")
        .author("s1engine fidelity generator")
        .heading(1, "Table with Merged Cells")
        .paragraph(|p| {
            p.text("This document exercises table cell merging (horizontal and vertical spans), ")
                .text("header row repetition, and cell background colors.")
        })
        .table(|t| {
            // Row 0: header row — 4 cells
            t.row(|r| {
                r.cell("Column A")
                    .cell("Column B")
                    .cell("Column C")
                    .cell("Column D")
            })
            // Row 1: first cell spans 2 columns, remaining 2 normal
            .row(|r| r.cell("Merged A1-B1").cell("").cell("C1").cell("D1"))
            // Row 2: normal cells except col D starts a vertical merge
            .row(|r| r.cell("A2").cell("B2").cell("C2").cell("D2-D3 merged"))
            // Row 3: col D continues the vertical merge from row 2
            .row(|r| r.cell("A3").cell("B3").cell("C3").cell(""))
        })
        .build();

    // Post-process: set cell attributes via model_mut
    let model = doc.model_mut();
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap();
    // The table is the second child (first is heading, second is paragraph, third is table)
    let table_id = body.children[2];
    let table = model.node(table_id).unwrap().clone();

    // Row 0: mark as header row
    let row0_id = table.children[0];
    if let Some(row0) = model.node_mut(row0_id) {
        row0.attributes
            .set(AttributeKey::TableHeaderRow, AttributeValue::Bool(true));
    }
    // Set header cell backgrounds to a light blue
    let row0_node = model.node(row0_id).unwrap().clone();
    for &cell_id in &row0_node.children {
        if let Some(cell) = model.node_mut(cell_id) {
            cell.attributes.set(
                AttributeKey::CellBackground,
                AttributeValue::Color(Color::new(200, 220, 240)),
            );
        }
    }

    // Row 1: first cell spans 2 columns (ColSpan=2), second cell is empty filler
    let row1_id = table.children[1];
    let row1_node = model.node(row1_id).unwrap().clone();
    if let Some(cell_a) = model.node_mut(row1_node.children[0]) {
        cell_a
            .attributes
            .set(AttributeKey::ColSpan, AttributeValue::Int(2));
        cell_a.attributes.set(
            AttributeKey::CellBackground,
            AttributeValue::Color(Color::new(255, 240, 200)),
        );
    }

    // Row 2: cell D starts vertical merge (RowSpan=2)
    let row2_id = table.children[2];
    let row2_node = model.node(row2_id).unwrap().clone();
    if let Some(cell_d) = model.node_mut(row2_node.children[3]) {
        cell_d
            .attributes
            .set(AttributeKey::RowSpan, AttributeValue::Int(2));
        cell_d.attributes.set(
            AttributeKey::CellBackground,
            AttributeValue::Color(Color::new(220, 240, 220)),
        );
    }

    // Set table width to 100%
    if let Some(tbl) = model.node_mut(table_id) {
        tbl.attributes.set(
            AttributeKey::TableWidth,
            AttributeValue::TableWidth(TableWidth::Percent(100.0)),
        );
    }

    // Add filler paragraphs to push content across pages
    let body_children_count = model.node(body_id).unwrap().children.len();
    for i in 0..20 {
        let para_id = model.next_id();
        let _ = model.insert_node(
            body_id,
            body_children_count + i,
            Node::new(para_id, NodeType::Paragraph),
        );

        let run_id = model.next_id();
        let _ = model.insert_node(para_id, 0, Node::new(run_id, NodeType::Run));

        let text = repeated_paragraph("Table fidelity filler paragraph for pagination.", i);
        let text_id = model.next_id();
        let _ = model.insert_node(run_id, 0, Node::text(text_id, &text));
    }

    doc
}

/// Build a document with 2 sections: section 1 is single-column Letter portrait,
/// section 2 is two-column A4 landscape.
fn build_multi_section() -> Document {
    // Section 1: Letter portrait, single column (default)
    let section1 = SectionProperties {
        break_type: Some(SectionBreakType::NextPage),
        ..SectionProperties::default()
    };

    // Section 2: A4 landscape, two columns
    let section2 = SectionProperties {
        page_width: 841.89,  // A4 landscape width (297mm)
        page_height: 595.28, // A4 landscape height (210mm)
        orientation: PageOrientation::Landscape,
        columns: 2,
        column_spacing: 36.0,
        margin_top: 72.0,
        margin_bottom: 72.0,
        margin_left: 72.0,
        margin_right: 72.0,
        break_type: None, // final section
        ..SectionProperties::default()
    };

    let mut builder = DocumentBuilder::new()
        .title("Tier 2 Multi-Section Layout")
        .author("s1engine fidelity generator")
        .section(section1)
        .heading(1, "Section 1: Letter Portrait")
        .paragraph(|p| {
            p.text("This is the first section using US Letter portrait orientation ")
                .text("with a single column layout. Content flows normally.")
        });

    // Add paragraphs to fill section 1
    for i in 0..15 {
        let text = repeated_paragraph("Section 1 filler content for Letter portrait.", i);
        builder = builder.paragraph(|p| p.text(&text));
    }

    builder = builder
        .section(section2)
        .heading(1, "Section 2: A4 Landscape Two-Column")
        .paragraph(|p| {
            p.text("This second section uses A4 landscape orientation with two columns. ")
                .text("Content should flow into two columns across the wider page.")
        });

    // Add paragraphs to fill section 2
    for i in 0..20 {
        let text = repeated_paragraph("Section 2 two-column A4 landscape filler content.", i);
        builder = builder.paragraph(|p| p.text(&text));
    }

    builder.build()
}

/// Build a document with nested numbered and bulleted lists (3 levels) plus a bookmark.
fn build_lists_bookmarks() -> Document {
    let mut doc = DocumentBuilder::new()
        .title("Tier 2 Lists and Bookmarks")
        .author("s1engine fidelity generator")
        .heading(1, "Nested Lists and Bookmarks")
        .paragraph(|p| {
            p.text("This document exercises nested numbered and bulleted lists at three ")
                .text("indent levels, plus bookmark annotations.")
        })
        // Numbered list — level 0
        .heading(2, "Numbered List (3 levels)")
        .numbered("First top-level numbered item")
        .list_item("Sub-item 1.1", 1, ListFormat::Decimal, 2)
        .list_item("Sub-item 1.1.1", 2, ListFormat::LowerRoman, 2)
        .list_item("Sub-item 1.1.2", 2, ListFormat::LowerRoman, 2)
        .list_item("Sub-item 1.2", 1, ListFormat::Decimal, 2)
        .numbered("Second top-level numbered item")
        .list_item("Sub-item 2.1", 1, ListFormat::Decimal, 2)
        .list_item("Sub-item 2.1.1", 2, ListFormat::LowerRoman, 2)
        .numbered("Third top-level numbered item")
        // Bulleted list — level 0
        .heading(2, "Bulleted List (3 levels)")
        .bullet("Top-level bullet A")
        .list_item("Sub-bullet A.1", 1, ListFormat::Bullet, 1)
        .list_item("Sub-bullet A.1.a", 2, ListFormat::Bullet, 1)
        .list_item("Sub-bullet A.1.b", 2, ListFormat::Bullet, 1)
        .list_item("Sub-bullet A.2", 1, ListFormat::Bullet, 1)
        .bullet("Top-level bullet B")
        .list_item("Sub-bullet B.1", 1, ListFormat::Bullet, 1)
        .list_item("Sub-bullet B.1.a", 2, ListFormat::Bullet, 1)
        .bullet("Top-level bullet C")
        // Paragraph with bookmark
        .heading(2, "Bookmark Section")
        .paragraph(|p| {
            p.bookmark_start("important_section")
                .text("This paragraph is wrapped in a bookmark named 'important_section'. ")
                .bold("Bookmark anchors")
                .text(" allow cross-references and navigation within the document.")
                .bookmark_end()
        })
        .build();

    // Add filler paragraphs for pagination
    let model = doc.model_mut();
    let body_id = model.body_id().unwrap();
    let body_children_count = model.node(body_id).unwrap().children.len();
    for i in 0..15 {
        let para_id = model.next_id();
        let _ = model.insert_node(
            body_id,
            body_children_count + i,
            Node::new(para_id, NodeType::Paragraph),
        );

        let run_id = model.next_id();
        let _ = model.insert_node(para_id, 0, Node::new(run_id, NodeType::Run));

        let text = repeated_paragraph("List and bookmark filler paragraph.", i);
        let text_id = model.next_id();
        let _ = model.insert_node(run_id, 0, Node::text(text_id, &text));
    }

    doc
}

// ─── Tier 3 Builders ────────────────────────────────────────────────────────

/// Create a minimal 1x1 white PNG image in memory for testing.
fn make_test_png(width: u32, height: u32) -> Vec<u8> {
    // Minimal valid PNG: 1-pixel white image scaled via IHDR dimensions.
    // We generate a proper minimal PNG with raw IDAT.
    let mut buf = Vec::new();

    // PNG signature
    buf.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR chunk
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&width.to_be_bytes());
    ihdr_data.extend_from_slice(&height.to_be_bytes());
    ihdr_data.push(8); // bit depth
    ihdr_data.push(2); // color type RGB
    ihdr_data.push(0); // compression
    ihdr_data.push(0); // filter
    ihdr_data.push(0); // interlace
    write_png_chunk(&mut buf, b"IHDR", &ihdr_data);

    // IDAT chunk: uncompressed deflate of raw scanlines
    // Each scanline: filter byte (0) + width * 3 bytes RGB (white = 0xFF)
    let scanline_len = 1 + (width as usize) * 3;
    let raw_data_len = scanline_len * (height as usize);
    // Build one scanline: filter=0 then (width) white RGB pixels
    let mut scanline = vec![0u8; scanline_len];
    for px in 0..(width as usize) {
        scanline[1 + px * 3] = 0xFF;
        scanline[1 + px * 3 + 1] = 0xFF;
        scanline[1 + px * 3 + 2] = 0xFF;
    }
    let mut raw_data = Vec::with_capacity(raw_data_len);
    for _ in 0..height {
        raw_data.extend_from_slice(&scanline);
    }

    // Wrap in a minimal zlib/deflate stream:
    // zlib header (0x78, 0x01) + stored blocks + adler32
    let mut deflate = Vec::new();
    deflate.push(0x78); // CMF
    deflate.push(0x01); // FLG

    // Split raw_data into stored deflate blocks (max 65535 bytes each)
    let mut offset = 0;
    while offset < raw_data.len() {
        let remaining = raw_data.len() - offset;
        let block_size = remaining.min(65535);
        let is_final = offset + block_size >= raw_data.len();
        deflate.push(if is_final { 0x01 } else { 0x00 }); // BFINAL + BTYPE=00 (stored)
        deflate.extend_from_slice(&(block_size as u16).to_le_bytes());
        deflate.extend_from_slice(&(!(block_size as u16)).to_le_bytes());
        deflate.extend_from_slice(&raw_data[offset..offset + block_size]);
        offset += block_size;
    }

    // Adler-32 checksum of the raw data
    let adler = adler32(&raw_data);
    deflate.extend_from_slice(&adler.to_be_bytes());

    write_png_chunk(&mut buf, b"IDAT", &deflate);

    // IEND chunk
    write_png_chunk(&mut buf, b"IEND", &[]);

    buf
}

fn write_png_chunk(buf: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buf.extend_from_slice(chunk_type);
    buf.extend_from_slice(data);
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    let crc = crc32(&crc_data);
    buf.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Build a document with an inline image and a floating image with square wrap.
fn build_inline_floating_images() -> Document {
    let mut doc = DocumentBuilder::new()
        .title("Tier 3 Inline and Floating Images")
        .author("s1engine fidelity generator")
        .heading(1, "Inline and Floating Images")
        .paragraph(|p| {
            p.text("This document exercises both inline and floating image placement. ")
                .text("An inline image appears within the text flow, while a floating image ")
                .text("uses anchor positioning with square text wrap.")
        })
        .heading(2, "Inline Image")
        .paragraph(|p| p.text("The inline image appears below in its own paragraph."))
        .paragraph(|p| p.text("")) // placeholder — we will insert the inline image here
        .paragraph(|p| {
            p.text("Text continues after the inline image. The image should be embedded ")
                .text("in the normal document flow.")
        })
        .heading(2, "Floating Image")
        .paragraph(|p| {
            p.text("The floating image in this section uses square text wrapping. ")
                .text("Surrounding text should flow around the image boundaries. ")
                .text("This tests the layout engine's ability to handle anchor-positioned ")
                .text("objects with wrap constraints.")
        })
        .build();

    let model = doc.model_mut();

    // Insert test images into media store
    let inline_png = make_test_png(200, 150);
    let float_png = make_test_png(180, 120);
    let inline_media_id =
        model
            .media_mut()
            .insert("image/png", inline_png, Some("inline-test.png".into()));
    let float_media_id =
        model
            .media_mut()
            .insert("image/png", float_png, Some("float-test.png".into()));

    // Find the empty placeholder paragraph (6th child of body: heading, para, heading, para, placeholder, para)
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap().clone();

    // Insert inline image into the placeholder paragraph (index 4 — 0-based)
    let placeholder_para_id = body.children[4];
    let img_id = model.next_id();
    let mut img_node = Node::new(img_id, NodeType::Image);
    img_node.attributes.set(
        AttributeKey::ImageMediaId,
        AttributeValue::MediaId(inline_media_id),
    );
    img_node
        .attributes
        .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
    img_node
        .attributes
        .set(AttributeKey::ImageHeight, AttributeValue::Float(150.0));
    img_node.attributes.set(
        AttributeKey::ImageAltText,
        AttributeValue::String("Inline test image".into()),
    );
    img_node.attributes.set(
        AttributeKey::ImagePositionType,
        AttributeValue::String("inline".into()),
    );
    // Insert at position 0 in the placeholder paragraph (before any existing runs)
    let placeholder_child_count = model
        .node(placeholder_para_id)
        .map(|n| n.children.len())
        .unwrap_or(0);
    let _ = model.insert_node(placeholder_para_id, placeholder_child_count, img_node);

    // Insert floating image into the last content paragraph
    // Find the paragraph after "Floating Image" heading (the long description paragraph)
    let float_para_id = body.children[body.children.len() - 1];
    let float_img_id = model.next_id();
    let mut float_node = Node::new(float_img_id, NodeType::Image);
    float_node.attributes.set(
        AttributeKey::ImageMediaId,
        AttributeValue::MediaId(float_media_id),
    );
    float_node
        .attributes
        .set(AttributeKey::ImageWidth, AttributeValue::Float(180.0));
    float_node
        .attributes
        .set(AttributeKey::ImageHeight, AttributeValue::Float(120.0));
    float_node.attributes.set(
        AttributeKey::ImageAltText,
        AttributeValue::String("Floating test image with square wrap".into()),
    );
    float_node.attributes.set(
        AttributeKey::ImagePositionType,
        AttributeValue::String("anchor".into()),
    );
    float_node.attributes.set(
        AttributeKey::ImageWrapType,
        AttributeValue::String("square".into()),
    );
    float_node.attributes.set(
        AttributeKey::ImageHorizontalOffset,
        AttributeValue::Int(2000000), // ~2 inches in EMUs from left
    );
    float_node.attributes.set(
        AttributeKey::ImageVerticalOffset,
        AttributeValue::Int(500000), // offset from paragraph
    );
    float_node.attributes.set(
        AttributeKey::ImageHorizontalRelativeFrom,
        AttributeValue::String("column".into()),
    );
    float_node.attributes.set(
        AttributeKey::ImageVerticalRelativeFrom,
        AttributeValue::String("paragraph".into()),
    );
    float_node.attributes.set(
        AttributeKey::ImageDistanceFromText,
        AttributeValue::String("91440,91440,91440,91440".into()), // ~0.1" wrap distance
    );
    let float_para_child_count = model
        .node(float_para_id)
        .map(|n| n.children.len())
        .unwrap_or(0);
    let _ = model.insert_node(float_para_id, float_para_child_count, float_node);

    // Add filler paragraphs
    let body_children_count = model.node(body_id).unwrap().children.len();
    for i in 0..20 {
        let para_id = model.next_id();
        let _ = model.insert_node(
            body_id,
            body_children_count + i,
            Node::new(para_id, NodeType::Paragraph),
        );
        let run_id = model.next_id();
        let _ = model.insert_node(para_id, 0, Node::new(run_id, NodeType::Run));
        let text = repeated_paragraph("Image fidelity filler paragraph for pagination.", i);
        let text_id = model.next_id();
        let _ = model.insert_node(run_id, 0, Node::text(text_id, &text));
    }

    doc
}

/// Build a document with comment anchors and tracked changes (insertion + deletion).
fn build_comments_review() -> Document {
    let mut doc = DocumentBuilder::new()
        .title("Tier 3 Comments and Review")
        .author("s1engine fidelity generator")
        .heading(1, "Comments and Tracked Changes")
        .paragraph(|p| {
            p.text("This document exercises comment annotations and tracked change ")
                .text("markup (insertions and deletions).")
        })
        .heading(2, "Commented Text")
        .paragraph(|p| {
            p.text("This sentence has no comment. ")
                .text("This part has a comment attached.")
                .text(" And this part is after the comment.")
        })
        .heading(2, "Tracked Insertion")
        .paragraph(|p| {
            p.text("Original text before the insertion. ")
                .text("INSERTED TEXT HERE. ")
                .text("Original text after the insertion.")
        })
        .heading(2, "Tracked Deletion")
        .paragraph(|p| {
            p.text("Text before deletion. ")
                .text("THIS TEXT WAS DELETED. ")
                .text("Text after deletion.")
        })
        .build();

    let model = doc.model_mut();
    let root_id = model.root_id();
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap().clone();

    // --- Add comment annotation ---
    // The paragraph with "This part has a comment attached." is body.children[3]
    // (0: heading, 1: intro para, 2: heading, 3: commented para, 4: heading, 5: insert para, 6: heading, 7: delete para)
    let comment_para_id = body.children[3];

    // Create CommentBody node as child of document root
    let root_child_count = model.node(root_id).unwrap().children.len();
    let comment_body_id = model.next_id();
    let _ = model.insert_node(
        root_id,
        root_child_count,
        Node::new(comment_body_id, NodeType::CommentBody),
    );
    // Add a paragraph inside the comment body
    let comment_inner_para_id = model.next_id();
    let _ = model.insert_node(
        comment_body_id,
        0,
        Node::new(comment_inner_para_id, NodeType::Paragraph),
    );
    let comment_run_id = model.next_id();
    let _ = model.insert_node(
        comment_inner_para_id,
        0,
        Node::new(comment_run_id, NodeType::Run),
    );
    let comment_text_id = model.next_id();
    let _ = model.insert_node(
        comment_run_id,
        0,
        Node::text(comment_text_id, "Review: Please verify this claim."),
    );

    // Insert CommentStart before the second run in the comment paragraph
    let comment_start_id = model.next_id();
    let mut cs_node = Node::new(comment_start_id, NodeType::CommentStart);
    cs_node.attributes.set(
        AttributeKey::CommentId,
        AttributeValue::String("comment-1".into()),
    );
    cs_node.attributes.set(
        AttributeKey::CommentAuthor,
        AttributeValue::String("Reviewer".into()),
    );
    cs_node.attributes.set(
        AttributeKey::CommentDate,
        AttributeValue::String("2026-03-15T10:30:00Z".into()),
    );
    // Insert before the second run (index 1)
    let _ = model.insert_node(comment_para_id, 1, cs_node);

    // Insert CommentEnd after the second run (now at index 3 because CommentStart shifted it)
    let comment_end_id = model.next_id();
    let mut ce_node = Node::new(comment_end_id, NodeType::CommentEnd);
    ce_node.attributes.set(
        AttributeKey::CommentId,
        AttributeValue::String("comment-1".into()),
    );
    let _ = model.insert_node(comment_para_id, 3, ce_node);

    // --- Mark tracked insertion ---
    // The insertion paragraph is body.children[5]
    let insert_para_id = body.children[5];
    let insert_para = model.node(insert_para_id).unwrap().clone();
    // The second run ("INSERTED TEXT HERE.") is the one to mark as inserted
    let inserted_run_id = insert_para.children[1];
    if let Some(run) = model.node_mut(inserted_run_id) {
        run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Insert".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Editor".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-03-15T11:00:00Z".into()),
        );
        run.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(1001));
    }

    // --- Mark tracked deletion ---
    // The deletion paragraph is body.children[7]
    let delete_para_id = body.children[7];
    let delete_para = model.node(delete_para_id).unwrap().clone();
    // The second run ("THIS TEXT WAS DELETED.") is the one to mark as deleted
    let deleted_run_id = delete_para.children[1];
    if let Some(run) = model.node_mut(deleted_run_id) {
        run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Delete".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Editor".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-03-15T11:05:00Z".into()),
        );
        run.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(1002));
    }

    // Add filler paragraphs
    let body_children_count = model.node(body_id).unwrap().children.len();
    for i in 0..15 {
        let para_id = model.next_id();
        let _ = model.insert_node(
            body_id,
            body_children_count + i,
            Node::new(para_id, NodeType::Paragraph),
        );
        let run_id = model.next_id();
        let _ = model.insert_node(para_id, 0, Node::new(run_id, NodeType::Run));
        let text = repeated_paragraph("Comments and review filler paragraph.", i);
        let text_id = model.next_id();
        let _ = model.insert_node(run_id, 0, Node::text(text_id, &text));
    }

    doc
}

// ─── Tier 4 Builders ────────────────────────────────────────────────────────

/// Build a document with 200+ paragraphs mixing Latin, CJK, Arabic, and Hebrew text.
fn build_large_multilingual() -> Document {
    let mut builder = DocumentBuilder::new()
        .title("Tier 4 Large Multilingual Document")
        .author("s1engine fidelity generator")
        .heading(1, "Multilingual Document (200+ Paragraphs)");

    let latin_samples = [
        "The quick brown fox jumps over the lazy dog. Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam.",
        "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.",
        "Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
    ];

    let cjk_samples = [
        "\u{6587}\u{5B57}\u{5316}\u{3051}\u{3057}\u{306A}\u{3044}\u{30E2}\u{30C0}\u{30F3}\u{306A}\u{30C9}\u{30AD}\u{30E5}\u{30E1}\u{30F3}\u{30C8}\u{30A8}\u{30F3}\u{30B8}\u{30F3}\u{3002}\u{8AAD}\u{307F}\u{8FBC}\u{307F}\u{3001}\u{66F8}\u{304D}\u{8FBC}\u{307F}\u{3001}\u{7DE8}\u{96C6}\u{3001}\u{5909}\u{63DB}\u{304C}\u{53EF}\u{80FD}\u{3067}\u{3059}\u{3002}", // Japanese
        "\u{73B0}\u{4EE3}\u{6587}\u{6863}\u{5F15}\u{64CE}\u{652F}\u{6301}\u{591A}\u{79CD}\u{6587}\u{6863}\u{683C}\u{5F0F}\u{3002}\u{5305}\u{62EC}DOCX\u{3001}ODT\u{3001}PDF\u{548C}\u{7EAF}\u{6587}\u{672C}\u{683C}\u{5F0F}\u{3002}", // Chinese
        "\u{D604}\u{B300}\u{C801}\u{C778} \u{BB38}\u{C11C} \u{C5D4}\u{C9C4}\u{C740} \u{B2E4}\u{C591}\u{D55C} \u{BB38}\u{C11C} \u{D615}\u{C2DD}\u{C744} \u{C9C0}\u{C6D0}\u{D569}\u{B2C8}\u{B2E4}. DOCX, ODT, PDF \u{BC0F} \u{D14D}\u{C2A4}\u{D2B8} \u{D615}\u{C2DD}\u{C744} \u{D3EC}\u{D568}\u{D569}\u{B2C8}\u{B2E4}.", // Korean
    ];

    let arabic_samples = [
        "\u{0645}\u{062D}\u{0631}\u{0643} \u{0627}\u{0644}\u{0645}\u{0633}\u{062A}\u{0646}\u{062F}\u{0627}\u{062A} \u{0627}\u{0644}\u{062D}\u{062F}\u{064A}\u{062B} \u{064A}\u{062F}\u{0639}\u{0645} \u{062A}\u{0646}\u{0633}\u{064A}\u{0642}\u{0627}\u{062A} \u{0645}\u{062A}\u{0639}\u{062F}\u{062F}\u{0629} \u{0644}\u{0644}\u{0645}\u{0633}\u{062A}\u{0646}\u{062F}\u{0627}\u{062A}.",
        "\u{064A}\u{062A}\u{0636}\u{0645}\u{0646} \u{0630}\u{0644}\u{0643} DOCX \u{0648}ODT \u{0648}PDF \u{0648}\u{0627}\u{0644}\u{0646}\u{0635} \u{0627}\u{0644}\u{0639}\u{0627}\u{062F}\u{064A}.",
    ];

    let hebrew_samples = [
        "\u{05DE}\u{05E0}\u{05D5}\u{05E2} \u{05DE}\u{05E1}\u{05DE}\u{05DB}\u{05D9}\u{05DD} \u{05DE}\u{05D5}\u{05D3}\u{05E8}\u{05E0}\u{05D9} \u{05EA}\u{05D5}\u{05DE}\u{05DA} \u{05D1}\u{05EA}\u{05D1}\u{05E0}\u{05D9}\u{05D5}\u{05EA} \u{05DE}\u{05E1}\u{05DE}\u{05DB}\u{05D9}\u{05DD} \u{05DE}\u{05E8}\u{05D5}\u{05D1}\u{05D5}\u{05EA}.",
        "\u{05D6}\u{05D4} \u{05DB}\u{05D5}\u{05DC}\u{05DC} DOCX, ODT, PDF \u{05D5}\u{05EA}\u{05D1}\u{05E0}\u{05D9}\u{05D5}\u{05EA} \u{05D8}\u{05E7}\u{05E1}\u{05D8} \u{05E8}\u{05D2}\u{05D9}\u{05DC}.",
    ];

    // Generate 200+ paragraphs cycling through scripts
    for i in 0..210 {
        let cycle = i % 11; // cycle through different script categories
        match cycle {
            0..=3 => {
                let sample = latin_samples[i % latin_samples.len()];
                let text = format!("[Latin #{}] {}", i + 1, sample);
                builder = builder.paragraph(|p| p.text(&text));
            }
            4..=6 => {
                let sample = cjk_samples[i % cjk_samples.len()];
                let text = format!("[CJK #{}] {}", i + 1, sample);
                builder = builder.paragraph(|p| p.text(&text));
            }
            7..=8 => {
                let sample = arabic_samples[i % arabic_samples.len()];
                let text = format!("[Arabic #{}] {}", i + 1, sample);
                builder = builder.paragraph(|p| p.text(&text));
            }
            9..=10 => {
                let sample = hebrew_samples[i % hebrew_samples.len()];
                let text = format!("[Hebrew #{}] {}", i + 1, sample);
                builder = builder.paragraph(|p| p.text(&text));
            }
            _ => unreachable!(),
        }

        // Add section headings every 50 paragraphs
        if i > 0 && i % 50 == 0 {
            builder = builder.heading(2, &format!("Section Break at Paragraph {}", i));
        }
    }

    builder.build()
}

/// Build a stress-test document with 100 paragraphs, tables, and images.
fn build_stress_recovery() -> Document {
    let mut builder = DocumentBuilder::new()
        .title("Tier 4 Stress and Recovery")
        .author("s1engine fidelity generator")
        .heading(1, "Stress Test Document");

    // Interleave paragraphs, tables, and images
    for i in 0..100 {
        // Every 10th element is a table
        if i % 10 == 0 && i > 0 {
            let table_label = format!("Table at position {}", i);
            builder = builder.heading(2, &table_label).table(|t| {
                t.row(|r| {
                    r.cell(&format!("T{} R0C0", i))
                        .cell(&format!("T{} R0C1", i))
                        .cell(&format!("T{} R0C2", i))
                })
                .row(|r| {
                    r.cell(&format!("T{} R1C0", i))
                        .cell(&format!("T{} R1C1", i))
                        .cell(&format!("T{} R1C2", i))
                })
                .row(|r| {
                    r.cell(&format!("T{} R2C0", i))
                        .cell(&format!("T{} R2C1", i))
                        .cell(&format!("T{} R2C2", i))
                })
            });
        }

        // Regular paragraph with varying formatting
        let text = format!(
            "Stress paragraph {}. This paragraph exercises layout under load. \
             Content varies to prevent layout caching from masking real performance. \
             Random seed value: {}. End of paragraph {}.",
            i + 1,
            (i * 7 + 13) % 997,
            i + 1,
        );

        if i % 3 == 0 {
            builder = builder.paragraph(|p| p.bold(&text));
        } else if i % 3 == 1 {
            builder = builder.paragraph(|p| p.italic(&text));
        } else {
            builder = builder.paragraph(|p| p.text(&text));
        }
    }

    let mut doc = builder.build();

    // Add images at specific positions using model_mut
    let model = doc.model_mut();

    // Create test images
    let img_data_small = make_test_png(100, 80);
    let img_data_large = make_test_png(300, 200);
    let media_small =
        model
            .media_mut()
            .insert("image/png", img_data_small, Some("stress-small.png".into()));
    let media_large =
        model
            .media_mut()
            .insert("image/png", img_data_large, Some("stress-large.png".into()));

    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap().clone();

    // Insert images at positions 20, 40, 60, 80 in the body children
    let image_positions = [20, 40, 60, 80];
    for (idx, &pos) in image_positions.iter().enumerate() {
        if pos < body.children.len() {
            let target_para_id = body.children[pos];
            let child_count = model
                .node(target_para_id)
                .map(|n| n.children.len())
                .unwrap_or(0);

            let img_id = model.next_id();
            let mut img_node = Node::new(img_id, NodeType::Image);
            let (media_id, w, h) = if idx % 2 == 0 {
                (media_small, 100.0, 80.0)
            } else {
                (media_large, 300.0, 200.0)
            };
            img_node.attributes.set(
                AttributeKey::ImageMediaId,
                AttributeValue::MediaId(media_id),
            );
            img_node
                .attributes
                .set(AttributeKey::ImageWidth, AttributeValue::Float(w));
            img_node
                .attributes
                .set(AttributeKey::ImageHeight, AttributeValue::Float(h));
            img_node.attributes.set(
                AttributeKey::ImageAltText,
                AttributeValue::String(format!("Stress test image {}", idx + 1)),
            );
            img_node.attributes.set(
                AttributeKey::ImagePositionType,
                AttributeValue::String("inline".into()),
            );
            let _ = model.insert_node(target_para_id, child_count, img_node);
        }
    }

    doc
}

// ─── Infrastructure ─────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn ensure_parent(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_case(root: &Path, case: &CaseSpec) -> Result<(), Box<dyn Error>> {
    let source_path = root.join(case.source_doc);
    let layout_path = root.join(case.layout_json);
    let page_map_path = root.join(case.page_map_json);
    let pdf_path = root.join(case.pdf_path);

    ensure_parent(&source_path)?;
    ensure_parent(&layout_path)?;
    ensure_parent(&page_map_path)?;
    ensure_parent(&pdf_path)?;

    let doc = (case.build)();
    let docx_bytes = doc.export(Format::Docx)?;
    fs::write(&source_path, &docx_bytes)?;

    let engine = WasmEngine::new();
    let wasm_doc = engine
        .open_as(&docx_bytes, "docx")
        .map_err(|e: JsError| format!("{e:?}"))?;
    let layout_json = wasm_doc
        .to_layout_json()
        .map_err(|e: JsError| format!("{e:?}"))?;
    let page_map_json = wasm_doc
        .get_page_map_json()
        .map_err(|e: JsError| format!("{e:?}"))?;
    let pdf_bytes = wasm_doc.to_pdf().map_err(|e: JsError| format!("{e:?}"))?;

    fs::write(&layout_path, layout_json)?;
    fs::write(&page_map_path, page_map_json)?;
    fs::write(&pdf_path, pdf_bytes)?;

    println!("generated {}", case.id);
    println!("  source: {}", source_path.display());
    println!("  layout: {}", layout_path.display());
    println!("  page_map: {}", page_map_path.display());
    println!("  pdf: {}", pdf_path.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = workspace_root();
    let cases = [
        // ─── Tier 1 ─────────────────────────────────────────────
        CaseSpec {
            id: "tier1_basic_paragraphs",
            source_doc: "tests/fidelity/corpus/tier1/basic-paragraphs.docx",
            layout_json: "tests/fidelity/artifacts/tier1_basic_paragraphs.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier1_basic_paragraphs.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier1_basic_paragraphs.engine.pdf",
            build: build_basic_paragraphs,
        },
        CaseSpec {
            id: "tier1_headers_footers",
            source_doc: "tests/fidelity/corpus/tier1/headers-footers.docx",
            layout_json: "tests/fidelity/artifacts/tier1_headers_footers.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier1_headers_footers.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier1_headers_footers.engine.pdf",
            build: build_headers_footers,
        },
        // ─── Tier 2 ─────────────────────────────────────────────
        CaseSpec {
            id: "tier2_tables_merged",
            source_doc: "tests/fidelity/corpus/tier2/tables-merged.docx",
            layout_json: "tests/fidelity/artifacts/tier2_tables_merged.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier2_tables_merged.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier2_tables_merged.engine.pdf",
            build: build_tables_merged,
        },
        CaseSpec {
            id: "tier2_multi_section",
            source_doc: "tests/fidelity/corpus/tier2/multi-section.docx",
            layout_json: "tests/fidelity/artifacts/tier2_multi_section.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier2_multi_section.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier2_multi_section.engine.pdf",
            build: build_multi_section,
        },
        CaseSpec {
            id: "tier2_lists_bookmarks",
            source_doc: "tests/fidelity/corpus/tier2/lists-bookmarks.docx",
            layout_json: "tests/fidelity/artifacts/tier2_lists_bookmarks.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier2_lists_bookmarks.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier2_lists_bookmarks.engine.pdf",
            build: build_lists_bookmarks,
        },
        // ─── Tier 3 ─────────────────────────────────────────────
        CaseSpec {
            id: "tier3_inline_floating_images",
            source_doc: "tests/fidelity/corpus/tier3/inline-floating-images.docx",
            layout_json: "tests/fidelity/artifacts/tier3_inline_floating_images.engine.layout.json",
            page_map_json:
                "tests/fidelity/artifacts/tier3_inline_floating_images.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier3_inline_floating_images.engine.pdf",
            build: build_inline_floating_images,
        },
        CaseSpec {
            id: "tier3_comments_review",
            source_doc: "tests/fidelity/corpus/tier3/comments-review.docx",
            layout_json: "tests/fidelity/artifacts/tier3_comments_review.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier3_comments_review.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier3_comments_review.engine.pdf",
            build: build_comments_review,
        },
        // ─── Tier 4 ─────────────────────────────────────────────
        CaseSpec {
            id: "tier4_large_multilingual",
            source_doc: "tests/fidelity/corpus/tier4/large-multilingual.docx",
            layout_json: "tests/fidelity/artifacts/tier4_large_multilingual.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier4_large_multilingual.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier4_large_multilingual.engine.pdf",
            build: build_large_multilingual,
        },
        CaseSpec {
            id: "tier4_stress_recovery",
            source_doc: "tests/fidelity/corpus/tier4/stress-recovery.docx",
            layout_json: "tests/fidelity/artifacts/tier4_stress_recovery.engine.layout.json",
            page_map_json: "tests/fidelity/artifacts/tier4_stress_recovery.engine.page_map.json",
            pdf_path: "tests/fidelity/artifacts/tier4_stress_recovery.engine.pdf",
            build: build_stress_recovery,
        },
    ];

    for case in &cases {
        write_case(&root, case)?;
    }

    Ok(())
}
