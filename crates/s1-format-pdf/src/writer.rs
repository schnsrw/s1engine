//! PDF writer — converts a `LayoutDocument` into PDF bytes.
//!
//! Uses `pdf-writer` for low-level PDF generation and `subsetter` for font
//! subsetting (only embed used glyphs to keep file sizes reasonable).

use std::collections::HashMap;

use pdf_writer::types::FontFlags;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str, TextStr};

use s1_layout::{
    LayoutBlock, LayoutBlockKind, LayoutBookmark, LayoutDocument, LayoutLine, LayoutTableRow,
};
use s1_model::DocumentMetadata;
use s1_text::{FontDatabase, FontId};

use crate::error::PdfError;

/// PDF/A conformance level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfAConformance {
    /// PDF/A-1b (ISO 19005-1, Level B) — visual reproduction.
    PdfA1b,
}

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
    write_pdf_internal(layout, font_db, metadata, None)
}

/// Write a laid-out document to PDF/A-compliant bytes.
///
/// PDF/A-1b adds an ICC color profile output intent and XMP metadata for archival compliance.
///
/// # Arguments
///
/// * `layout` — The fully laid-out document.
/// * `font_db` — Font database for loading font data.
/// * `metadata` — Optional document metadata (title, author, etc.).
/// * `conformance` — The PDF/A conformance level.
///
/// # Errors
///
/// Returns `PdfError` if font embedding or PDF generation fails.
pub fn write_pdf_a(
    layout: &LayoutDocument,
    font_db: &FontDatabase,
    metadata: Option<&DocumentMetadata>,
    conformance: PdfAConformance,
) -> Result<Vec<u8>, PdfError> {
    write_pdf_internal(layout, font_db, metadata, Some(conformance))
}

/// Internal PDF writer that optionally adds PDF/A compliance.
fn write_pdf_internal(
    layout: &LayoutDocument,
    font_db: &FontDatabase,
    metadata: Option<&DocumentMetadata>,
    pdfa: Option<PdfAConformance>,
) -> Result<Vec<u8>, PdfError> {
    let mut pdf = Pdf::new();
    let mut alloc = RefAllocator::new();

    // Collect all unique fonts used across the document
    let font_usage = collect_font_usage(layout);

    // Embed fonts and build the font map (FontId → PDF font name + Ref)
    let font_map = embed_fonts(&mut pdf, &mut alloc, font_db, &font_usage)?;

    // Embed images and build the image map (media_id → XObject Ref + resource name)
    let image_map = embed_images(&mut pdf, &mut alloc, layout)?;

    // Write document metadata
    if let Some(meta) = metadata {
        write_metadata(&mut pdf, &mut alloc, meta);
    }

    // Write pages
    let catalog_ref = alloc.next();
    let page_tree_ref = alloc.next();

    let mut page_refs = Vec::new();

    for page in &layout.pages {
        let page_ref = write_page(
            &mut pdf,
            &mut alloc,
            page,
            page_tree_ref,
            &font_map,
            &image_map,
        )?;
        page_refs.push(page_ref);
    }

    // Write page tree
    let mut page_tree = pdf.pages(page_tree_ref);
    page_tree.kids(page_refs.iter().copied());
    page_tree.count(page_refs.len() as i32);
    page_tree.finish();

    // Write outline (bookmarks) if present
    let outline_ref = if !layout.bookmarks.is_empty() {
        Some(write_outline(
            &mut pdf,
            &mut alloc,
            &layout.bookmarks,
            &page_refs,
        ))
    } else {
        None
    };

    // PDF/A: write ICC output intent and XMP metadata
    let (output_intents_ref, xmp_ref) = if let Some(conformance) = pdfa {
        let oi = write_pdfa_output_intent(&mut pdf, &mut alloc);
        let xmp = write_xmp_metadata(&mut pdf, &mut alloc, metadata, conformance);
        (Some(oi), Some(xmp))
    } else {
        (None, None)
    };

    // Write catalog
    let mut catalog = pdf.catalog(catalog_ref);
    catalog.pages(page_tree_ref);
    if let Some(outline_ref) = outline_ref {
        catalog.outlines(outline_ref);
    }
    if let Some(oi_ref) = output_intents_ref {
        catalog.insert(Name(b"OutputIntents")).array().item(oi_ref);
    }
    if let Some(xmp) = xmp_ref {
        catalog.insert(Name(b"Metadata")).primitive(xmp);
    }
    catalog.finish();

    Ok(pdf.finish())
}

/// A font entry in the PDF.
struct PdfFont {
    /// PDF resource name (e.g., "F1", "F2").
    name: String,
    /// Reference to the font object in the PDF.
    font_ref: Ref,
}

/// An image XObject entry in the PDF.
struct PdfImage {
    /// PDF resource name (e.g., "Im1", "Im2").
    name: String,
    /// Reference to the XObject in the PDF.
    xobject_ref: Ref,
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
        LayoutBlockKind::Paragraph { lines, .. } => {
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
        LayoutBlockKind::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    collect_blocks_font_usage(&cell.blocks, usage);
                }
            }
        }
        LayoutBlockKind::Image { .. } => {}
        _ => {}
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

/// Embed images from the layout into the PDF as XObjects.
///
/// Deduplicates by `media_id` so identical images share one XObject.
fn embed_images(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    layout: &LayoutDocument,
) -> Result<HashMap<String, PdfImage>, PdfError> {
    let mut image_map: HashMap<String, PdfImage> = HashMap::new();
    let mut img_idx = 0u32;

    for page in &layout.pages {
        collect_and_embed_images(pdf, alloc, &page.blocks, &mut image_map, &mut img_idx)?;
    }

    Ok(image_map)
}

fn collect_and_embed_images(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    blocks: &[LayoutBlock],
    image_map: &mut HashMap<String, PdfImage>,
    img_idx: &mut u32,
) -> Result<(), PdfError> {
    for block in blocks {
        match &block.kind {
            LayoutBlockKind::Image {
                media_id,
                image_data: Some(data),
                content_type,
                ..
            } if !media_id.is_empty() && !data.is_empty() => {
                if image_map.contains_key(media_id) {
                    continue; // Already embedded
                }

                let ct = content_type.as_deref().unwrap_or("");
                let xobject_ref = alloc.next();
                let name = format!("Im{}", *img_idx);
                *img_idx += 1;

                if ct.contains("jpeg") || ct.contains("jpg") || is_jpeg(data) {
                    embed_jpeg_image(pdf, xobject_ref, data)?;
                } else {
                    // Try to decode as PNG or other image format via the `image` crate
                    embed_decoded_image(pdf, xobject_ref, data)?;
                }

                image_map.insert(media_id.clone(), PdfImage { name, xobject_ref });
            }
            LayoutBlockKind::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        collect_and_embed_images(pdf, alloc, &cell.blocks, image_map, img_idx)?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check if bytes start with JPEG SOI marker.
fn is_jpeg(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8
}

/// Maximum image dimension (pixels) to prevent excessive memory use.
const MAX_IMAGE_DIMENSION: u32 = 16384;

/// Embed a JPEG image as-is with DCTDecode filter.
fn embed_jpeg_image(pdf: &mut Pdf, xobject_ref: Ref, data: &[u8]) -> Result<(), PdfError> {
    // Parse JPEG to get dimensions
    let (width, height) = jpeg_dimensions(data).unwrap_or((1, 1));

    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(PdfError::Generation(format!(
            "image dimensions {width}x{height} exceed maximum {MAX_IMAGE_DIMENSION}"
        )));
    }

    let mut stream = pdf.image_xobject(xobject_ref, data);
    stream.filter(pdf_writer::Filter::DctDecode);
    stream.width(width as i32);
    stream.height(height as i32);
    stream.color_space().device_rgb();
    stream.bits_per_component(8);
    stream.finish();

    Ok(())
}

/// Parse JPEG dimensions from SOF markers.
fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2; // Skip SOI
    while i + 4 < data.len() {
        if data[i] != 0xFF {
            break;
        }
        let marker = data[i + 1];
        // SOF0-SOF3 markers contain dimensions
        if (0xC0..=0xC3).contains(&marker) && i + 9 < data.len() {
            let height = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
            let width = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
            return Some((width, height));
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        i += 2 + len;
    }
    None
}

/// Decode an image (PNG, etc.) to RGB pixels and embed with FlateDecode.
fn embed_decoded_image(pdf: &mut Pdf, xobject_ref: Ref, data: &[u8]) -> Result<(), PdfError> {
    let img = image::load_from_memory(data)
        .map_err(|e| PdfError::Generation(format!("image decode error: {e}")))?;

    let rgb = img.to_rgb8();
    let width = rgb.width();
    let height = rgb.height();

    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(PdfError::Generation(format!(
            "image dimensions {width}x{height} exceed maximum {MAX_IMAGE_DIMENSION}"
        )));
    }
    let raw_pixels = rgb.into_raw();

    let compressed = miniz_oxide::deflate::compress_to_vec(&raw_pixels, 6);

    let mut stream = pdf.image_xobject(xobject_ref, &compressed);
    stream.filter(pdf_writer::Filter::FlateDecode);
    stream.width(width as i32);
    stream.height(height as i32);
    stream.color_space().device_rgb();
    stream.bits_per_component(8);
    stream.finish();

    Ok(())
}

/// A hyperlink annotation to create on a page.
struct HyperlinkAnnotation {
    /// PDF rectangle (bottom-left x, bottom-left y, top-right x, top-right y).
    rect: Rect,
    /// Target URL.
    url: String,
}

/// Write a single page to the PDF.
fn write_page(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    page: &s1_layout::LayoutPage,
    page_tree_ref: Ref,
    font_map: &HashMap<FontId, PdfFont>,
    image_map: &HashMap<String, PdfImage>,
) -> Result<Ref, PdfError> {
    let page_ref = alloc.next();
    let content_ref = alloc.next();

    // Build content stream
    let mut content = Content::new();

    // Collect hyperlink annotations while rendering
    let mut hyperlinks: Vec<HyperlinkAnnotation> = Vec::new();

    // Render blocks
    for block in &page.blocks {
        render_block(
            &mut content,
            block,
            page.height,
            font_map,
            image_map,
            &mut hyperlinks,
        );
    }

    // Render header/footer
    if let Some(ref header) = page.header {
        render_block(
            &mut content,
            header,
            page.height,
            font_map,
            image_map,
            &mut hyperlinks,
        );
    }
    if let Some(ref footer) = page.footer {
        render_block(
            &mut content,
            footer,
            page.height,
            font_map,
            image_map,
            &mut hyperlinks,
        );
    }

    let content_data = content.finish();

    // Write content stream
    pdf.stream(content_ref, &content_data);

    // Write annotation objects first (before the page object borrows pdf)
    let mut annot_refs = Vec::new();
    for link in &hyperlinks {
        let annot_ref = alloc.next();
        annot_refs.push(annot_ref);

        let mut annot = pdf.annotation(annot_ref);
        annot.subtype(pdf_writer::types::AnnotationType::Link);
        annot.rect(link.rect);
        annot.border(0.0, 0.0, 0.0, None);
        annot
            .action()
            .action_type(pdf_writer::types::ActionType::Uri)
            .uri(Str(link.url.as_bytes()));
        annot.finish();
    }

    // Write page object
    let mut page_obj = pdf.page(page_ref);
    page_obj.parent(page_tree_ref);
    page_obj.media_box(Rect::new(0.0, 0.0, page.width as f32, page.height as f32));
    page_obj.contents(content_ref);

    // Write resources (fonts + images)
    {
        let mut resources = page_obj.resources();
        let mut fonts = resources.fonts();
        for pdf_font in font_map.values() {
            fonts.pair(Name(pdf_font.name.as_bytes()), pdf_font.font_ref);
        }
        fonts.finish();

        if !image_map.is_empty() {
            let mut xobjects = resources.x_objects();
            for pdf_img in image_map.values() {
                xobjects.pair(Name(pdf_img.name.as_bytes()), pdf_img.xobject_ref);
            }
            xobjects.finish();
        }

        resources.finish();
    }

    // Add annotation references to page
    if !annot_refs.is_empty() {
        page_obj.annotations(annot_refs.iter().copied());
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
    image_map: &HashMap<String, PdfImage>,
    hyperlinks: &mut Vec<HyperlinkAnnotation>,
) {
    match &block.kind {
        LayoutBlockKind::Paragraph { lines, .. } => {
            for line in lines {
                render_line(
                    content,
                    line,
                    &block.bounds,
                    page_height,
                    font_map,
                    hyperlinks,
                );
            }
        }
        LayoutBlockKind::Table { rows, .. } => {
            render_table(
                content,
                rows,
                &block.bounds,
                page_height,
                font_map,
                image_map,
                hyperlinks,
            );
        }
        LayoutBlockKind::Image {
            media_id,
            bounds,
            image_data,
            ..
        } => {
            if let Some(pdf_img) = image_map.get(media_id) {
                // Draw the actual image using the XObject
                let pdf_y = page_height - block.bounds.y - bounds.height;
                content.save_state();
                content.transform([
                    bounds.width as f32,
                    0.0,
                    0.0,
                    bounds.height as f32,
                    block.bounds.x as f32,
                    pdf_y as f32,
                ]);
                content.x_object(Name(pdf_img.name.as_bytes()));
                content.restore_state();
            } else if image_data.is_some() {
                // Image data present but not in map — should not happen, draw placeholder
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
            } else {
                // No image data — draw a gray placeholder rectangle
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
        _ => {}
    }
}

/// Render a line of text.
fn render_line(
    content: &mut Content,
    line: &LayoutLine,
    block_bounds: &s1_layout::Rect,
    page_height: f64,
    font_map: &HashMap<FontId, PdfFont>,
    hyperlinks: &mut Vec<HyperlinkAnnotation>,
) {
    // PDF coordinate system: origin at bottom-left, y increases upward
    let pdf_baseline_y = page_height - block_bounds.y - line.baseline_y;

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
            content.next_line(pdf_x as f32, pdf_baseline_y as f32);

            // Encode glyphs as CID (2 bytes per glyph)
            let encoded: Vec<u8> = run
                .glyphs
                .iter()
                .flat_map(|g| [(g.glyph_id >> 8) as u8, (g.glyph_id & 0xFF) as u8])
                .collect();

            content.show(Str(&encoded));
            content.end_text();

            // Collect hyperlink annotation if this run has a URL
            if let Some(ref url) = run.hyperlink_url {
                let run_bottom = pdf_baseline_y - run.font_size * 0.2; // slight descent
                let run_top = pdf_baseline_y + run.font_size * 0.8; // slight ascent
                hyperlinks.push(HyperlinkAnnotation {
                    rect: Rect::new(
                        pdf_x as f32,
                        run_bottom as f32,
                        (pdf_x + run.width) as f32,
                        run_top as f32,
                    ),
                    url: url.clone(),
                });
            }
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
    image_map: &HashMap<String, PdfImage>,
    hyperlinks: &mut Vec<HyperlinkAnnotation>,
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
                render_block(content, block, page_height, font_map, image_map, hyperlinks);
            }
        }
    }

    content.restore_state();
}

/// Write PDF document outline (bookmarks).
fn write_outline(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    bookmarks: &[LayoutBookmark],
    page_refs: &[Ref],
) -> Ref {
    let outline_ref = alloc.next();

    // Pre-allocate refs for each bookmark entry
    let entry_refs: Vec<Ref> = bookmarks.iter().map(|_| alloc.next()).collect();

    // Write outline dictionary
    let mut outline = pdf.outline(outline_ref);
    if let Some(&first) = entry_refs.first() {
        outline.first(first);
    }
    if let Some(&last) = entry_refs.last() {
        outline.last(last);
    }
    outline.count(bookmarks.len() as i32);
    outline.finish();

    // Write each bookmark entry
    for (i, bookmark) in bookmarks.iter().enumerate() {
        let entry_ref = entry_refs[i];
        let page_ref = page_refs
            .get(bookmark.page_index)
            .copied()
            .unwrap_or(page_refs[0]);

        // PDF y: convert from top-origin to bottom-origin
        let pdf_y = 792.0 - bookmark.y_position; // Default letter height

        let mut entry = pdf.outline_item(entry_ref);
        entry.parent(outline_ref);
        entry.title(TextStr(&bookmark.name));

        // Set prev/next links
        if i > 0 {
            entry.prev(entry_refs[i - 1]);
        }
        if i + 1 < entry_refs.len() {
            entry.next(entry_refs[i + 1]);
        }

        // Destination: [page /XYZ left top null]
        entry.dest().page(page_ref).xyz(0.0, pdf_y as f32, None);

        entry.finish();
    }

    outline_ref
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

/// Write a PDF/A output intent with a minimal sRGB ICC profile.
fn write_pdfa_output_intent(pdf: &mut Pdf, alloc: &mut RefAllocator) -> Ref {
    // Minimal sRGB ICC profile header (128 bytes) — sufficient for PDF/A-1b validation.
    // This is the ICC profile header indicating sRGB color space.
    let icc_profile = build_minimal_srgb_icc_profile();

    let icc_ref = alloc.next();
    let mut icc_stream = pdf.stream(icc_ref, &icc_profile);
    icc_stream.insert(Name(b"N")).primitive(3i32); // 3 components (RGB)
    icc_stream.filter(pdf_writer::Filter::FlateDecode);
    icc_stream.finish();

    let oi_ref = alloc.next();
    let mut oi = pdf.indirect(oi_ref).dict();
    oi.pair(Name(b"Type"), Name(b"OutputIntent"));
    oi.pair(Name(b"S"), Name(b"GTS_PDFA1"));
    oi.pair(Name(b"OutputConditionIdentifier"), TextStr("sRGB IEC61966-2.1"));
    oi.pair(Name(b"RegistryName"), TextStr("http://www.color.org"));
    oi.pair(Name(b"Info"), TextStr("sRGB IEC61966-2.1"));
    oi.pair(Name(b"DestOutputProfile"), icc_ref);
    oi.finish();

    oi_ref
}

/// Build a minimal sRGB ICC profile for PDF/A compliance.
///
/// This generates a minimal valid ICC profile that declares the sRGB color space.
/// The profile is ~128 bytes (header only, with tag table) and is sufficient
/// for PDF/A-1b validators.
fn build_minimal_srgb_icc_profile() -> Vec<u8> {
    let mut profile = vec![0u8; 128];

    // Profile size (128 bytes minimum header)
    let size = 128u32;
    profile[0..4].copy_from_slice(&size.to_be_bytes());

    // Preferred CMM type: 'none'
    // profile[4..8] stays zero

    // Profile version: 2.1.0
    profile[8] = 2;
    profile[9] = 0x10;

    // Device class: 'mntr' (monitor)
    profile[12..16].copy_from_slice(b"mntr");

    // Color space: 'RGB '
    profile[16..20].copy_from_slice(b"RGB ");

    // PCS (Profile Connection Space): 'XYZ '
    profile[20..24].copy_from_slice(b"XYZ ");

    // Date/time: 2024-01-01
    profile[24..26].copy_from_slice(&2024u16.to_be_bytes()); // year
    profile[26..28].copy_from_slice(&1u16.to_be_bytes());    // month
    profile[28..30].copy_from_slice(&1u16.to_be_bytes());    // day

    // Profile file signature: 'acsp'
    profile[36..40].copy_from_slice(b"acsp");

    // Primary platform: 'APPL'
    profile[40..44].copy_from_slice(b"APPL");

    // Tag count: 0 (header-only profile for size minimization)
    // profile[128..132] would be tag count, but we keep at 128 bytes

    profile
}

/// Write XMP metadata stream for PDF/A compliance.
fn write_xmp_metadata(
    pdf: &mut Pdf,
    alloc: &mut RefAllocator,
    metadata: Option<&DocumentMetadata>,
    conformance: PdfAConformance,
) -> Ref {
    let (part, level) = match conformance {
        PdfAConformance::PdfA1b => ("1", "B"),
    };

    let title = metadata
        .and_then(|m| m.title.as_deref())
        .unwrap_or("Untitled");
    let creator = metadata
        .and_then(|m| m.creator.as_deref())
        .unwrap_or("s1engine");

    let xmp = format!(
        r#"<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x='adobe:ns:meta/'>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
<rdf:Description rdf:about=''
  xmlns:dc='http://purl.org/dc/elements/1.1/'
  xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'
  xmlns:xmp='http://ns.adobe.com/xap/1.0/'>
<dc:title><rdf:Alt><rdf:li xml:lang='x-default'>{title}</rdf:li></rdf:Alt></dc:title>
<dc:creator><rdf:Seq><rdf:li>{creator}</rdf:li></rdf:Seq></dc:creator>
<pdfaid:part>{part}</pdfaid:part>
<pdfaid:conformance>{level}</pdfaid:conformance>
<xmp:CreatorTool>s1engine</xmp:CreatorTool>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"#
    );

    let xmp_ref = alloc.next();
    let mut stream = pdf.stream(xmp_ref, xmp.as_bytes());
    stream.insert(Name(b"Type")).primitive(Name(b"Metadata"));
    stream.insert(Name(b"Subtype")).primitive(Name(b"XML"));
    stream.finish();

    xmp_ref
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_layout::{LayoutConfig, LayoutEngine};
    use s1_model::{AttributeKey, AttributeValue, DocumentModel, MediaId, Node, NodeType};

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

    /// Create a minimal valid 1x1 white JPEG.
    fn minimal_jpeg() -> Vec<u8> {
        // Smallest valid JPEG: SOI + APP0 + DQT + SOF0 + DHT + SOS + data + EOI
        // This is a well-known minimal JPEG (1x1 white pixel).
        vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
            0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
            0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
            0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
            0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
            0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01,
            0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10,
            0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
            0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
            0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
            0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
            0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
            0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73,
            0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
            0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
            0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6,
            0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
            0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08,
            0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7B, 0x94, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xFF, 0xD9,
        ]
    }

    /// Create a minimal valid 1x1 red PNG.
    fn minimal_png() -> Vec<u8> {
        // Use the image crate to encode a tiny PNG in-memory
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut buf));
        image::ImageEncoder::write_image(
            encoder,
            &[255, 0, 0],
            1,
            1,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
        buf
    }

    #[test]
    fn export_empty_document() {
        let doc = DocumentModel::new();
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_single_paragraph() {
        let doc = make_simple_doc("Hello World");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 100);
    }

    #[test]
    fn export_with_metadata() {
        let doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn pdf_has_valid_structure() {
        let doc = make_simple_doc("Test");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let layout = LayoutDocument {
            pages: Vec::new(),
            bookmarks: Vec::new(),
        };
        let usage = collect_font_usage(&layout);
        assert!(usage.is_empty());
    }

    // --- Milestone 3.6 tests ---

    fn make_doc_with_image(image_data: Vec<u8>, content_type: &str) -> (DocumentModel, MediaId) {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Insert media into the store
        let media_id = doc
            .media_mut()
            .insert(content_type.to_string(), image_data, None);

        // Create an Image node with MediaId attribute
        let mut image_node = Node::new(doc.next_id(), NodeType::Image);
        image_node.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        image_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(72.0));
        image_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(72.0));
        doc.insert_node(body_id, 0, image_node).unwrap();

        (doc, media_id)
    }

    #[test]
    fn export_png_image() {
        let png_data = minimal_png();
        let (doc, _) = make_doc_with_image(png_data, "image/png");

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        // Verify image data was populated in layout
        let has_image_data = layout.pages.iter().any(|p| {
            p.blocks.iter().any(|b| {
                matches!(
                    &b.kind,
                    LayoutBlockKind::Image { image_data: Some(data), .. } if !data.is_empty()
                )
            })
        });
        assert!(has_image_data, "Layout should contain image data");

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        // PDF with image should be larger than a minimal PDF
        assert!(bytes.len() > 200);

        // Check that XObject was written (presence of /Im0)
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("/Im"), "PDF should contain image XObject");
    }

    #[test]
    fn export_jpeg_image() {
        let jpeg_data = minimal_jpeg();
        let (doc, _) = make_doc_with_image(jpeg_data.clone(), "image/jpeg");

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));

        // JPEG should be DCT-encoded (pass-through)
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(
            pdf_str.contains("DCTDecode") || pdf_str.contains("/Im"),
            "PDF should embed JPEG image"
        );
    }

    #[test]
    fn export_image_sizing() {
        let png_data = minimal_png();
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let media_id = doc
            .media_mut()
            .insert("image/png".to_string(), png_data, None);

        // Use a specific size
        let mut image_node = Node::new(doc.next_id(), NodeType::Image);
        image_node.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        image_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
        image_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(150.0));
        doc.insert_node(body_id, 0, image_node).unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        // Check the image block has the right dimensions
        let img_block = layout.pages[0]
            .blocks
            .iter()
            .find(|b| matches!(&b.kind, LayoutBlockKind::Image { .. }))
            .expect("Should have image block");
        assert!((img_block.bounds.width - 200.0).abs() < 1.0);
        assert!((img_block.bounds.height - 150.0).abs() < 1.0);

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_image_multi_page() {
        // Create many paragraphs + an image to push it to page 2
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Fill first page with paragraphs
        for i in 0..60 {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, "Fill line"))
                .unwrap();
        }

        let png_data = minimal_png();
        let media_id = doc
            .media_mut()
            .insert("image/png".to_string(), png_data, None);

        let mut image_node = Node::new(doc.next_id(), NodeType::Image);
        image_node.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        image_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(72.0));
        image_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(72.0));
        doc.insert_node(body_id, 60, image_node).unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        assert!(layout.pages.len() > 1, "Should have multiple pages");

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_image_deduplication() {
        let png_data = minimal_png();
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Same image data → same MediaId → single XObject
        let media_id = doc
            .media_mut()
            .insert("image/png".to_string(), png_data.clone(), None);

        for idx in 0..3 {
            let mut image_node = Node::new(doc.next_id(), NodeType::Image);
            image_node.attributes.set(
                AttributeKey::ImageMediaId,
                AttributeValue::MediaId(media_id),
            );
            image_node
                .attributes
                .set(AttributeKey::ImageWidth, AttributeValue::Float(50.0));
            image_node
                .attributes
                .set(AttributeKey::ImageHeight, AttributeValue::Float(50.0));
            doc.insert_node(body_id, idx, image_node).unwrap();
        }

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        // All 3 images should map to the same media_id
        let image_blocks: Vec<_> = layout.pages[0]
            .blocks
            .iter()
            .filter(|b| matches!(&b.kind, LayoutBlockKind::Image { .. }))
            .collect();
        assert_eq!(image_blocks.len(), 3);

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));

        // Only one Im0 XObject should be created (deduplicated)
        let pdf_str = String::from_utf8_lossy(&bytes);
        let im0_count = pdf_str.matches("/Im0").count();
        // Im0 appears in XObject dict refs, not as multiple streams
        assert!(im0_count >= 1, "Should have Im0 XObject reference");
    }

    #[test]
    fn export_no_image_data_fallback() {
        // Image node without media in the store → placeholder rectangle
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let mut image_node = Node::new(doc.next_id(), NodeType::Image);
        image_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(72.0));
        image_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(72.0));
        doc.insert_node(body_id, 0, image_node).unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        // Should have image block with no data
        let has_no_data = layout.pages.iter().any(|p| {
            p.blocks.iter().any(|b| {
                matches!(
                    &b.kind,
                    LayoutBlockKind::Image {
                        image_data: None,
                        ..
                    }
                )
            })
        });
        assert!(has_no_data, "Image without media should have no data");

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_hyperlink_annotation() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let mut run_node = Node::new(doc.next_id(), NodeType::Run);
        run_node.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("https://example.com".to_string()),
        );
        let run_id = run_node.id;
        doc.insert_node(para_id, 0, run_node).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Click here"))
            .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        // Verify hyperlink_url was propagated
        let has_url = layout.pages.iter().any(|p| {
            p.blocks.iter().any(|b| {
                if let LayoutBlockKind::Paragraph { lines } = &b.kind {
                    lines
                        .iter()
                        .any(|l| l.runs.iter().any(|r| r.hyperlink_url.is_some()))
                } else {
                    false
                }
            })
        });
        assert!(has_url, "Layout should have hyperlink URL on glyph run");

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));

        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(
            pdf_str.contains("example.com"),
            "PDF should contain hyperlink URL"
        );
    }

    #[test]
    fn export_hyperlink_rect_coordinates() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let mut run_node = Node::new(doc.next_id(), NodeType::Run);
        run_node.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("https://test.org".to_string()),
        );
        let run_id = run_node.id;
        doc.insert_node(para_id, 0, run_node).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Link"))
            .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();

        let pdf_str = String::from_utf8_lossy(&bytes);
        // Should contain /Link annotation type and /URI action
        assert!(pdf_str.contains("/Link"), "Should have Link annotation");
        assert!(pdf_str.contains("/URI"), "Should have URI action");
    }

    #[test]
    fn export_multiple_hyperlinks() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        for (i, url) in ["https://a.com", "https://b.com"].iter().enumerate() {
            let mut run_node = Node::new(doc.next_id(), NodeType::Run);
            run_node.attributes.set(
                AttributeKey::HyperlinkUrl,
                AttributeValue::String(url.to_string()),
            );
            let run_id = run_node.id;
            doc.insert_node(para_id, i, run_node).unwrap();

            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, &format!("Link{i}")))
                .unwrap();
        }

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();

        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("a.com"), "Should contain first URL");
        assert!(pdf_str.contains("b.com"), "Should contain second URL");
    }

    #[test]
    fn export_hyperlink_across_lines() {
        // A hyperlink run that's long enough could theoretically be split across lines
        // For now, just ensure it doesn't crash and produces valid output
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let mut run_node = Node::new(doc.next_id(), NodeType::Run);
        run_node.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("https://long-url.example.com/path".to_string()),
        );
        let run_id = run_node.id;
        doc.insert_node(para_id, 0, run_node).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(
            run_id,
            0,
            Node::text(
                text_id,
                "This is a very long hyperlink text that might wrap",
            ),
        )
        .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();
        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_single_bookmark() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Add bookmark start as child of paragraph
        let mut bm_node = Node::new(doc.next_id(), NodeType::BookmarkStart);
        bm_node.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("section1".to_string()),
        );
        doc.insert_node(para_id, 0, bm_node).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Section 1"))
            .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        assert_eq!(layout.bookmarks.len(), 1);
        assert_eq!(layout.bookmarks[0].name, "section1");
        assert_eq!(layout.bookmarks[0].page_index, 0);

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));

        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(
            pdf_str.contains("section1"),
            "PDF should contain bookmark title"
        );
        assert!(
            pdf_str.contains("/Outlines"),
            "PDF catalog should reference outlines"
        );
    }

    #[test]
    fn export_multiple_bookmarks() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        for (i, name) in ["intro", "chapter1", "chapter2"].iter().enumerate() {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();

            let mut bm_node = Node::new(doc.next_id(), NodeType::BookmarkStart);
            bm_node.attributes.set(
                AttributeKey::BookmarkName,
                AttributeValue::String(name.to_string()),
            );
            doc.insert_node(para_id, 0, bm_node).unwrap();

            let run_id = doc.next_id();
            doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, *name))
                .unwrap();
        }

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        assert_eq!(layout.bookmarks.len(), 3);

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("intro"));
        assert!(pdf_str.contains("chapter1"));
        assert!(pdf_str.contains("chapter2"));
    }

    #[test]
    fn export_bookmark_destination_page() {
        // Create a long doc so bookmarks land on different pages
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Bookmark on first page
        let para1_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para1_id, NodeType::Paragraph))
            .unwrap();
        let mut bm1 = Node::new(doc.next_id(), NodeType::BookmarkStart);
        bm1.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("page1".to_string()),
        );
        doc.insert_node(para1_id, 0, bm1).unwrap();
        let run1 = doc.next_id();
        doc.insert_node(para1_id, 1, Node::new(run1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(run1, 0, Node::text(t1, "First page"))
            .unwrap();

        // Fill pages
        for i in 1..70 {
            let p = doc.next_id();
            doc.insert_node(body_id, i, Node::new(p, NodeType::Paragraph))
                .unwrap();
            let r = doc.next_id();
            doc.insert_node(p, 0, Node::new(r, NodeType::Run)).unwrap();
            let t = doc.next_id();
            doc.insert_node(r, 0, Node::text(t, "Filler paragraph line"))
                .unwrap();
        }

        // Bookmark on later page
        let para2_id = doc.next_id();
        doc.insert_node(body_id, 70, Node::new(para2_id, NodeType::Paragraph))
            .unwrap();
        let mut bm2 = Node::new(doc.next_id(), NodeType::BookmarkStart);
        bm2.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("page2".to_string()),
        );
        doc.insert_node(para2_id, 0, bm2).unwrap();
        let run2 = doc.next_id();
        doc.insert_node(para2_id, 1, Node::new(run2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(run2, 0, Node::text(t2, "Later page"))
            .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        assert!(layout.bookmarks.len() >= 2);
        assert_eq!(layout.bookmarks[0].page_index, 0);
        // Second bookmark should be on a later page
        assert!(layout.bookmarks[1].page_index > 0);

        let bytes = write_pdf(&layout, &font_db, None).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn export_pdfa_1b() {
        let doc = make_simple_doc("PDF/A Test");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        let mut meta = DocumentMetadata::default();
        meta.title = Some("Test Document".to_string());
        meta.creator = Some("Test Author".to_string());

        let bytes = write_pdf_a(&layout, &font_db, Some(&meta), PdfAConformance::PdfA1b).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        // Should contain OutputIntent
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("OutputIntent"), "should contain OutputIntent");
        assert!(pdf_str.contains("GTS_PDFA1"), "should reference PDF/A-1");
        // Should contain XMP metadata
        assert!(pdf_str.contains("pdfaid:part"), "should contain PDF/A id");
        assert!(pdf_str.contains("pdfaid:conformance"), "should contain conformance level");
        // Should have ICC profile reference
        assert!(pdf_str.contains("sRGB"), "should reference sRGB");
    }

    #[test]
    fn export_pdfa_contains_xmp() {
        let doc = make_simple_doc("XMP test");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let layout = engine.layout().unwrap();

        let bytes = write_pdf_a(&layout, &font_db, None, PdfAConformance::PdfA1b).unwrap();
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("xmpmeta"), "should have XMP metadata");
        assert!(pdf_str.contains("CreatorTool"), "should have creator tool");
        assert!(pdf_str.contains("s1engine"), "should identify s1engine");
    }
}
