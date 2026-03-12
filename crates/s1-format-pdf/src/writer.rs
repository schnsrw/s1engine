//! PDF writer — converts a `LayoutDocument` into PDF bytes.
//!
//! Uses `pdf-writer` for low-level PDF generation and `subsetter` for font
//! subsetting (only embed used glyphs to keep file sizes reasonable).

use std::collections::HashMap;

use pdf_writer::types::FontFlags;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str, TextStr};

use s1_layout::{LayoutBlock, LayoutBlockKind, LayoutDocument, LayoutLine, LayoutTableRow};
use s1_model::DocumentMetadata;
use s1_text::{FontDatabase, FontId};

use crate::error::PdfError;

/// Write a laid-out document to PDF bytes.
///
/// # Arguments
///
/// * `layout` — The fully laid-out document.
/// * `font_db` — Font database for loading font data.
/// * `metadata` — Optional document metadata (title, author, etc.).
///
/// # Errors
///
/// Returns `PdfError` if font embedding or PDF generation fails.
pub fn write_pdf(
    layout: &LayoutDocument,
    font_db: &FontDatabase,
    metadata: Option<&DocumentMetadata>,
) -> Result<Vec<u8>, PdfError> {
    let mut pdf = Pdf::new();
    let mut alloc = RefAllocator::new();

    // Collect all unique fonts used across the document
    let font_usage = collect_font_usage(layout);

    // Embed fonts and build the font map (FontId → PDF font name + Ref)
    let font_map = embed_fonts(&mut pdf, &mut alloc, font_db, &font_usage)?;

    // Write document metadata
    if let Some(meta) = metadata {
        write_metadata(&mut pdf, &mut alloc, meta);
    }

    // Write pages
    let catalog_ref = alloc.next();
    let page_tree_ref = alloc.next();

    let mut page_refs = Vec::new();

    for page in &layout.pages {
        let page_ref = write_page(&mut pdf, &mut alloc, page, page_tree_ref, &font_map)?;
        page_refs.push(page_ref);
    }

    // Write page tree
    let mut page_tree = pdf.pages(page_tree_ref);
    page_tree.kids(page_refs.iter().copied());
    page_tree.count(page_refs.len() as i32);
    page_tree.finish();

    // Write catalog
    pdf.catalog(catalog_ref).pages(page_tree_ref);

    Ok(pdf.finish())
}

/// A font entry in the PDF.
struct PdfFont {
    /// PDF resource name (e.g., "F1", "F2").
    name: String,
    /// Reference to the font object in the PDF.
    font_ref: Ref,
}

/// Reference allocator.
struct RefAllocator {
    next: i32,
}

impl RefAllocator {
    fn new() -> Self {
        Self { next: 1 }
    }

    fn next(&mut self) -> Ref {
        let r = Ref::new(self.next);
        self.next += 1;
        r
    }
}

/// Collect all unique FontIds used in the layout.
fn collect_font_usage(layout: &LayoutDocument) -> HashMap<FontId, Vec<u16>> {
    let mut usage: HashMap<FontId, Vec<u16>> = HashMap::new();

    for page in &layout.pages {
        collect_blocks_font_usage(&page.blocks, &mut usage);
        if let Some(ref header) = page.header {
            collect_block_font_usage(header, &mut usage);
        }
        if let Some(ref footer) = page.footer {
            collect_block_font_usage(footer, &mut usage);
        }
    }

    usage
}

fn collect_blocks_font_usage(blocks: &[LayoutBlock], usage: &mut HashMap<FontId, Vec<u16>>) {
    for block in blocks {
        collect_block_font_usage(block, usage);
    }
}

fn collect_block_font_usage(block: &LayoutBlock, usage: &mut HashMap<FontId, Vec<u16>>) {
    match &block.kind {
        LayoutBlockKind::Paragraph { lines } => {
            for line in lines {
                for run in &line.runs {
                    let glyphs = usage.entry(run.font_id).or_default();
                    for g in &run.glyphs {
                        if !glyphs.contains(&g.glyph_id) {
                            glyphs.push(g.glyph_id);
                        }
                    }
                }
            }
        }
        LayoutBlockKind::Table { rows } => {
            for row in rows {
                for cell in &row.cells {
                    collect_blocks_font_usage(&cell.blocks, usage);
                }
            }
        }
        LayoutBlockKind::Image { .. } => {}
    }
}

/// Embed fonts into the PDF and return a mapping from FontId to PDF font info.
fn embed_fonts(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    font_db: &FontDatabase,
    font_usage: &HashMap<FontId, Vec<u16>>,
) -> Result<HashMap<FontId, PdfFont>, PdfError> {
    let mut font_map = HashMap::new();

    for (font_idx, (font_id, glyph_ids)) in font_usage.iter().enumerate() {
        let font_name = format!("F{}", font_idx);

        let font_ref = alloc.next();
        let cid_ref = alloc.next();
        let descriptor_ref = alloc.next();
        let cmap_ref = alloc.next();
        let data_ref = alloc.next();

        if let Some(font) = font_db.load_font(*font_id) {
            // Build a GlyphRemapper for subsetting
            let remapper = subsetter::GlyphRemapper::new_from_glyphs(glyph_ids);

            // Try to subset the font
            let font_data = font.data();
            let subset_data = match subsetter::subset(font_data, 0, &remapper) {
                Ok(data) => data,
                Err(_) => font_data.to_vec(), // Fall back to full font
            };

            let metrics = font.metrics(1000.0);
            let upem = font.units_per_em();

            // Write the font stream (compressed)
            let compressed = miniz_oxide::deflate::compress_to_vec(&subset_data, 6);
            let mut stream = pdf.stream(data_ref, &compressed);
            stream.filter(pdf_writer::Filter::FlateDecode);
            stream.pair(Name(b"Length1"), subset_data.len() as i32);
            stream.finish();

            // Font descriptor
            let mut descriptor = pdf.font_descriptor(descriptor_ref);
            descriptor.name(Name(font.family_name().as_bytes()));
            descriptor.flags(FontFlags::NON_SYMBOLIC);
            descriptor.bbox(Rect::new(
                0.0,
                metrics.descent as f32 * upem as f32 / 1000.0,
                1000.0,
                metrics.ascent as f32 * upem as f32 / 1000.0,
            ));
            descriptor.italic_angle(if font.is_italic() { -12.0 } else { 0.0 });
            descriptor.ascent(metrics.ascent as f32 * upem as f32 / 1000.0);
            descriptor.descent(metrics.descent as f32 * upem as f32 / 1000.0);
            descriptor.cap_height(metrics.ascent as f32 * upem as f32 / 1000.0 * 0.7);
            descriptor.stem_v(80.0);
            descriptor.font_file2(data_ref);
            descriptor.finish();

            // CIDFont
            let mut cid_font = pdf.cid_font(cid_ref);
            cid_font.subtype(pdf_writer::types::CidFontType::Type2);
            cid_font.base_font(Name(font.family_name().as_bytes()));
            cid_font.system_info(pdf_writer::types::SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            });
            cid_font.font_descriptor(descriptor_ref);
            cid_font.default_width(1000.0);

            // Write glyph widths
            if !glyph_ids.is_empty() {
                let mut sorted_gids: Vec<u16> = glyph_ids.clone();
                sorted_gids.sort();
                let widths: Vec<f32> = sorted_gids
                    .iter()
                    .map(|&gid| {
                        font.glyph_hor_advance(gid)
                            .map(|a| a as f32 * 1000.0 / upem as f32)
                            .unwrap_or(500.0)
                    })
                    .collect();
                if let Some(&first_gid) = sorted_gids.first() {
                    cid_font
                        .widths()
                        .consecutive(first_gid, widths.iter().copied());
                }
            }
            cid_font.finish();

            // ToUnicode CMap (minimal)
            let cmap_data = build_tounicode_cmap();
            pdf.stream(cmap_ref, &cmap_data);

            // Type0 font
            let mut type0 = pdf.type0_font(font_ref);
            type0.base_font(Name(font.family_name().as_bytes()));
            type0.encoding_predefined(Name(b"Identity-H"));
            type0.descendant_font(cid_ref);
            type0.to_unicode(cmap_ref);
            type0.finish();
        } else {
            // No font data — write a placeholder standard font
            let mut font_obj = pdf.type1_font(font_ref);
            font_obj.base_font(Name(b"Helvetica"));
            font_obj.finish();
        }

        font_map.insert(
            *font_id,
            PdfFont {
                name: font_name,
                font_ref,
            },
        );
    }

    Ok(font_map)
}

/// Write a single page to the PDF.
fn write_page(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    page: &s1_layout::LayoutPage,
    page_tree_ref: Ref,
    font_map: &HashMap<FontId, PdfFont>,
) -> Result<Ref, PdfError> {
    let page_ref = alloc.next();
    let content_ref = alloc.next();

    // Build content stream
    let mut content = Content::new();

    // Render blocks
    for block in &page.blocks {
        render_block(&mut content, block, page.height, font_map);
    }

    let content_data = content.finish();

    // Write content stream
    pdf.stream(content_ref, &content_data);

    // Write page object
    let mut page_obj = pdf.page(page_ref);
    page_obj.parent(page_tree_ref);
    page_obj.media_box(Rect::new(0.0, 0.0, page.width as f32, page.height as f32));
    page_obj.contents(content_ref);

    // Write font resources inline on the page
    {
        let mut resources = page_obj.resources();
        let mut fonts = resources.fonts();
        for pdf_font in font_map.values() {
            fonts.pair(Name(pdf_font.name.as_bytes()), pdf_font.font_ref);
        }
        fonts.finish();
        resources.finish();
    }

    page_obj.finish();

    Ok(page_ref)
}

/// Render a layout block into a PDF content stream.
fn render_block(
    content: &mut Content,
    block: &LayoutBlock,
    page_height: f64,
    font_map: &HashMap<FontId, PdfFont>,
) {
    match &block.kind {
        LayoutBlockKind::Paragraph { lines } => {
            for line in lines {
                render_line(content, line, &block.bounds, page_height, font_map);
            }
        }
        LayoutBlockKind::Table { rows } => {
            render_table(content, rows, &block.bounds, page_height, font_map);
        }
        LayoutBlockKind::Image { bounds, .. } => {
            // Draw a placeholder rectangle for images
            let pdf_y = page_height - block.bounds.y - bounds.height;
            content.save_state();
            content.set_stroke_rgb(0.5, 0.5, 0.5);
            content.rect(
                block.bounds.x as f32,
                pdf_y as f32,
                bounds.width as f32,
                bounds.height as f32,
            );
            content.stroke();
            content.restore_state();
        }
    }
}

/// Render a line of text.
fn render_line(
    content: &mut Content,
    line: &LayoutLine,
    block_bounds: &s1_layout::Rect,
    page_height: f64,
    font_map: &HashMap<FontId, PdfFont>,
) {
    // PDF coordinate system: origin at bottom-left, y increases upward
    let pdf_y = page_height - block_bounds.y - line.baseline_y;

    for run in &line.runs {
        if run.glyphs.is_empty() {
            continue;
        }

        if let Some(pdf_font) = font_map.get(&run.font_id) {
            let pdf_x = block_bounds.x + run.x_offset;

            // Set color
            content.set_fill_rgb(
                run.color.r as f32 / 255.0,
                run.color.g as f32 / 255.0,
                run.color.b as f32 / 255.0,
            );

            content.begin_text();
            content.set_font(Name(pdf_font.name.as_bytes()), run.font_size as f32);
            content.next_line(pdf_x as f32, pdf_y as f32);

            // Encode glyphs as CID (2 bytes per glyph)
            let encoded: Vec<u8> = run
                .glyphs
                .iter()
                .flat_map(|g| [(g.glyph_id >> 8) as u8, (g.glyph_id & 0xFF) as u8])
                .collect();

            content.show(Str(&encoded));
            content.end_text();
        }
    }
}

/// Render table rows.
fn render_table(
    content: &mut Content,
    rows: &[LayoutTableRow],
    table_bounds: &s1_layout::Rect,
    page_height: f64,
    font_map: &HashMap<FontId, PdfFont>,
) {
    // Draw cell borders
    content.save_state();
    content.set_stroke_rgb(0.0, 0.0, 0.0);
    content.set_line_width(0.5);

    for row in rows {
        let row_pdf_y = page_height - table_bounds.y - row.bounds.y - row.bounds.height;

        // Draw row border
        content.rect(
            (table_bounds.x + row.bounds.x) as f32,
            row_pdf_y as f32,
            row.bounds.width as f32,
            row.bounds.height as f32,
        );
        content.stroke();

        // Draw cell borders and content
        for cell in &row.cells {
            let cell_x = table_bounds.x + cell.bounds.x;
            let cell_pdf_y = row_pdf_y;

            content.rect(
                cell_x as f32,
                cell_pdf_y as f32,
                cell.bounds.width as f32,
                cell.bounds.height as f32,
            );
            content.stroke();

            // Render cell content
            for block in &cell.blocks {
                render_block(content, block, page_height, font_map);
            }
        }
    }

    content.restore_state();
}

/// Write PDF metadata.
fn write_metadata(pdf: &mut Pdf, alloc: &mut RefAllocator, meta: &DocumentMetadata) {
    let info_ref = alloc.next();
    let mut info = pdf.document_info(info_ref);
    if let Some(ref title) = meta.title {
        info.title(TextStr(title));
    }
    if let Some(ref creator) = meta.creator {
        info.author(TextStr(creator));
    }
    if let Some(ref description) = meta.description {
        info.subject(TextStr(description));
    }
    info.creator(TextStr("s1engine"));
    info.finish();
}

/// Build a minimal ToUnicode CMap.
fn build_tounicode_cmap() -> Vec<u8> {
    // Minimal CMap that maps glyph IDs to Unicode code points.
    let cmap = b"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (UCS)
/Supplement 0
>> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
endcmap
CMapName currentdict /CMap defineresource pop
end
end";
    cmap.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_layout::{LayoutConfig, LayoutEngine};
    use s1_model::{DocumentModel, Node, NodeType};

    fn make_simple_doc(text: &str) -> DocumentModel {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();
        doc
    }

    #[test]
    fn export_empty_document() {
        let doc = DocumentModel::new();
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_single_paragraph() {
        let doc = make_simple_doc("Hello World");
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 100);
    }

    #[test]
    fn export_with_metadata() {
        let doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        let mut meta = DocumentMetadata::default();
        meta.title = Some("Test Document".to_string());
        meta.creator = Some("Test Author".to_string());

        let bytes = write_pdf(&layout, &font_db, Some(&meta)).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("Test Document"));
        assert!(pdf_str.contains("Test Author"));
    }

    #[test]
    fn export_multi_paragraph() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        for i in 0..5 {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, &format!("Paragraph {i}")))
                .unwrap();
        }

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_multi_page() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        for i in 0..80 {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(
                run_id,
                0,
                Node::text(text_id, "Lorem ipsum dolor sit amet, consectetur"),
            )
            .unwrap();
        }

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        assert!(layout.pages.len() > 1);

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_with_table() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        for row_idx in 0..2 {
            let row_id = doc.next_id();
            doc.insert_node(table_id, row_idx, Node::new(row_id, NodeType::TableRow))
                .unwrap();
            for col_idx in 0..2 {
                let cell_id = doc.next_id();
                doc.insert_node(row_id, col_idx, Node::new(cell_id, NodeType::TableCell))
                    .unwrap();
                let para_id = doc.next_id();
                doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
                    .unwrap();
                let run_id = doc.next_id();
                doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                    .unwrap();
                let text_id = doc.next_id();
                doc.insert_node(run_id, 0, Node::text(text_id, "Cell"))
                    .unwrap();
            }
        }

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn pdf_has_valid_structure() {
        let doc = make_simple_doc("Test");
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();

        // Check PDF header
        assert!(bytes.starts_with(b"%PDF"));
        // Check PDF trailer marker
        let trailer = String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(100)..]);
        assert!(trailer.contains("%%EOF"));
    }

    #[test]
    fn collect_font_usage_empty() {
        let layout = LayoutDocument { pages: Vec::new() };
        let usage = collect_font_usage(&layout);
        assert!(usage.is_empty());
    }
}
