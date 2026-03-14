//! Convert a `LayoutDocument` to paginated HTML.
//!
//! Produces CSS-positioned HTML with real page boundaries. Each page is a
//! positioned div with absolute-positioned content blocks, lines, and glyph runs.
//! Text is searchable, accessible, and selectable since we use real DOM elements
//! rather than Canvas rendering.

use crate::types::*;
use s1_model::Color;

/// Options for HTML rendering.
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Include page shadows and margins between pages.
    pub page_shadows: bool,
    /// Background color for pages (CSS color string).
    pub page_background: String,
    /// Gap between pages in pixels.
    pub page_gap: f64,
    /// Include a wrapper div around all pages.
    pub wrap_in_container: bool,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            page_shadows: true,
            page_background: "white".to_string(),
            page_gap: 20.0,
            wrap_in_container: true,
        }
    }
}

/// Convert a laid-out document to paginated HTML.
///
/// The output uses CSS positioning (`position: absolute`) with `pt` units
/// to place content exactly where the layout engine computed it. Pages are
/// rendered as separate divs with configurable styling.
///
/// # Examples
///
/// ```ignore
/// let html = layout_to_html(&layout_doc);
/// ```
pub fn layout_to_html(doc: &LayoutDocument) -> String {
    layout_to_html_with_options(doc, &HtmlOptions::default())
}

/// Convert a laid-out document to paginated HTML with custom options.
///
/// See [`HtmlOptions`] for available configuration.
///
/// # Examples
///
/// ```ignore
/// let options = HtmlOptions {
///     page_shadows: false,
///     ..Default::default()
/// };
/// let html = layout_to_html_with_options(&layout_doc, &options);
/// ```
pub fn layout_to_html_with_options(doc: &LayoutDocument, options: &HtmlOptions) -> String {
    let mut html = String::new();

    if options.wrap_in_container {
        html.push_str("<div class=\"s1-document\" style=\"display:flex;flex-direction:column;align-items:center;\">");
    }

    for page in &doc.pages {
        render_page(&mut html, page, options, &doc.bookmarks);
    }

    if options.wrap_in_container {
        html.push_str("</div>");
    }

    html
}

/// Render a single page.
fn render_page(html: &mut String, page: &LayoutPage, options: &HtmlOptions, bookmarks: &[LayoutBookmark]) {
    let shadow = if options.page_shadows {
        "box-shadow:0 2px 8px rgba(0,0,0,0.3);"
    } else {
        ""
    };
    let gap = options.page_gap;

    html.push_str(&format!(
        "<div class=\"s1-page\" style=\"width:{w}pt;height:{h}pt;position:relative;background:{bg};margin:{gap}px auto;{shadow}overflow:hidden\">",
        w = fmt_pt(page.width),
        h = fmt_pt(page.height),
        bg = escape_attr(&options.page_background),
    ));

    // Render bookmark anchors that belong to this page at their correct Y position
    for bookmark in bookmarks {
        if bookmark.page_index == page.index {
            html.push_str(&format!(
                "<a id=\"{}\" style=\"position:absolute;top:{}pt\"></a>",
                escape_attr(&bookmark.name),
                fmt_pt(bookmark.y_position),
            ));
        }
    }

    // Render header if present
    if let Some(header) = &page.header {
        render_block(html, header);
    }

    // Render content blocks
    for block in &page.blocks {
        render_block(html, block);
    }

    // Render footer if present
    if let Some(footer) = &page.footer {
        render_block(html, footer);
    }

    html.push_str("</div>");
}

/// Render a layout block (paragraph, table, or image).
fn render_block(html: &mut String, block: &LayoutBlock) {
    match &block.kind {
        LayoutBlockKind::Paragraph { lines, text_align, background_color, border } => {
            render_paragraph(html, block, lines, text_align.as_deref(), background_color.as_ref(), border.as_deref());
        }
        LayoutBlockKind::Table { rows, .. } => {
            render_table(html, block, rows);
        }
        LayoutBlockKind::Image {
            image_data,
            content_type,
            ..
        } => {
            render_image(html, block, image_data, content_type);
        }
        // Note: all current LayoutBlockKind variants are handled above.
        // If new variants are added, they will produce a compile error here.
    }
}

/// Render a paragraph block with lines and glyph runs.
fn render_paragraph(html: &mut String, block: &LayoutBlock, lines: &[LayoutLine], text_align: Option<&str>, background_color: Option<&Color>, border: Option<&str>) {
    let b = &block.bounds;
    let mut extra_style = String::new();
    if let Some(align) = text_align {
        extra_style.push_str(&format!(";text-align:{align}"));
    }
    if let Some(bg) = background_color {
        extra_style.push_str(&format!(";background-color:#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b));
    }
    if let Some(bdr) = border {
        extra_style.push_str(&format!(";border:{bdr}"));
    }
    html.push_str(&format!(
        "<div class=\"s1-block\" style=\"position:absolute;left:{x}pt;top:{y}pt;width:{w}pt{extra}\">",
        x = fmt_pt(b.x),
        y = fmt_pt(b.y),
        w = fmt_pt(b.width),
        extra = extra_style,
    ));

    for line in lines {
        render_line(html, line);
    }

    html.push_str("</div>");
}

/// Render a single line of text.
fn render_line(html: &mut String, line: &LayoutLine) {
    html.push_str(&format!(
        "<div class=\"s1-line\" style=\"height:{h}pt;position:relative\">",
        h = fmt_pt(line.height),
    ));

    for run in &line.runs {
        render_glyph_run(html, run);
    }

    html.push_str("</div>");
}

/// Render a single glyph run as a styled span, optionally wrapped in a hyperlink.
fn render_glyph_run(html: &mut String, run: &GlyphRun) {
    // Build inline style
    let mut style = String::new();
    style.push_str(&format!(
        "font-size:{sz}pt;position:absolute;left:{x}pt",
        sz = fmt_pt(run.font_size),
        x = fmt_pt(run.x_offset),
    ));

    // Color (skip if black to keep output smaller)
    if run.color.r != 0 || run.color.g != 0 || run.color.b != 0 {
        style.push_str(&format!(
            ";color:#{:02x}{:02x}{:02x}",
            run.color.r, run.color.g, run.color.b
        ));
    }

    // Bold
    if run.bold {
        style.push_str(";font-weight:bold");
    }

    // Italic
    if run.italic {
        style.push_str(";font-style:italic");
    }

    // Underline and strikethrough
    match (run.underline, run.strikethrough) {
        (true, true) => style.push_str(";text-decoration:underline line-through"),
        (true, false) => style.push_str(";text-decoration:underline"),
        (false, true) => style.push_str(";text-decoration:line-through"),
        (false, false) => {}
    }

    // Superscript / subscript
    if run.superscript {
        style.push_str(";vertical-align:super;font-size:0.65em");
    } else if run.subscript {
        style.push_str(";vertical-align:sub;font-size:0.65em");
    }

    // Highlight / background color
    if let Some(hl) = &run.highlight_color {
        style.push_str(&format!(
            ";background-color:#{:02x}{:02x}{:02x}",
            hl.r, hl.g, hl.b
        ));
    }

    // Character spacing
    if run.character_spacing.abs() > 0.01 {
        style.push_str(&format!(";letter-spacing:{:.1}pt", run.character_spacing));
    }

    let escaped_text = escape_html(&run.text);

    // Determine track change wrapper tag
    let (tc_open, tc_close) = match run.revision_type.as_deref() {
        Some("insertion") => {
            let title = run.revision_author.as_deref().unwrap_or("");
            (
                format!(
                    "<ins style=\"text-decoration:underline;color:green\" title=\"{}\">",
                    escape_attr(title)
                ),
                "</ins>".to_string(),
            )
        }
        Some("deletion") => {
            let title = run.revision_author.as_deref().unwrap_or("");
            (
                format!(
                    "<del style=\"text-decoration:line-through;color:red\" title=\"{}\">",
                    escape_attr(title)
                ),
                "</del>".to_string(),
            )
        }
        _ => (String::new(), String::new()),
    };

    // Wrap in hyperlink if URL is present
    if let Some(url) = &run.hyperlink_url {
        html.push_str(&tc_open);
        html.push_str(&format!(
            "<a href=\"{}\" style=\"{}\"><span style=\"{}\">{}</span></a>",
            escape_attr(url),
            "color:inherit;text-decoration:inherit",
            style,
            escaped_text,
        ));
        html.push_str(&tc_close);
    } else {
        html.push_str(&tc_open);
        html.push_str(&format!(
            "<span style=\"{}\">{}</span>",
            style, escaped_text,
        ));
        html.push_str(&tc_close);
    }
}

/// Render a table block.
fn render_table(html: &mut String, block: &LayoutBlock, rows: &[LayoutTableRow]) {
    let b = &block.bounds;
    html.push_str(&format!(
        "<div class=\"s1-table\" style=\"position:absolute;left:{x}pt;top:{y}pt;width:{w}pt\">",
        x = fmt_pt(b.x),
        y = fmt_pt(b.y),
        w = fmt_pt(b.width),
    ));

    for row in rows {
        html.push_str(&format!(
            "<div class=\"s1-table-row\" style=\"position:relative;height:{h}pt\">",
            h = fmt_pt(row.bounds.height),
        ));

        for cell in &row.cells {
            let mut cell_style = format!(
                "position:absolute;left:{x}pt;top:0pt;width:{w}pt;height:{h}pt;overflow:hidden",
                x = fmt_pt(cell.bounds.x),
                w = fmt_pt(cell.bounds.width),
                h = fmt_pt(cell.bounds.height),
            );

            // Cell borders (use originals or fallback to 1px solid #ccc)
            let bt = cell.border_top.as_deref().unwrap_or("1px solid #ccc");
            let bb = cell.border_bottom.as_deref().unwrap_or("1px solid #ccc");
            let bl = cell.border_left.as_deref().unwrap_or("1px solid #ccc");
            let br = cell.border_right.as_deref().unwrap_or("1px solid #ccc");
            cell_style.push_str(&format!(
                ";border-top:{bt};border-bottom:{bb};border-left:{bl};border-right:{br}"
            ));

            // Cell background color
            if let Some(bg) = &cell.background_color {
                cell_style.push_str(&format!(
                    ";background-color:#{:02x}{:02x}{:02x}",
                    bg.r, bg.g, bg.b
                ));
            }

            html.push_str(&format!(
                "<div class=\"s1-table-cell\" style=\"{cell_style}\">",
            ));

            // Render cell content blocks (recursively)
            for cell_block in &cell.blocks {
                // Adjust child block positioning relative to cell
                render_block(html, cell_block);
            }

            html.push_str("</div>");
        }

        html.push_str("</div>");
    }

    html.push_str("</div>");
}

/// Render an image block.
fn render_image(
    html: &mut String,
    block: &LayoutBlock,
    image_data: &Option<Vec<u8>>,
    content_type: &Option<String>,
) {
    let b = &block.bounds;

    if let Some(data) = image_data {
        let mime = content_type.as_deref().unwrap_or("image/png");
        let b64 = base64_encode(data);
        html.push_str(&format!(
            "<img class=\"s1-image\" src=\"data:{mime};base64,{b64}\" style=\"position:absolute;left:{x}pt;top:{y}pt;width:{w}pt;height:{h}pt\" alt=\"\"/>",
            x = fmt_pt(b.x),
            y = fmt_pt(b.y),
            w = fmt_pt(b.width),
            h = fmt_pt(b.height),
        ));
    } else {
        // No image data available — render a placeholder
        html.push_str(&format!(
            "<div class=\"s1-image-placeholder\" style=\"position:absolute;left:{x}pt;top:{y}pt;width:{w}pt;height:{h}pt;background:#eee;border:1px dashed #aaa\"></div>",
            x = fmt_pt(b.x),
            y = fmt_pt(b.y),
            w = fmt_pt(b.width),
            h = fmt_pt(b.height),
        ));
    }
}

/// Escape HTML special characters in text content.
fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape a string for use in an HTML attribute value.
fn escape_attr(text: &str) -> String {
    // Same as escape_html — covers all attribute-unsafe characters
    escape_html(text)
}

/// Format a floating-point value for CSS `pt` output.
///
/// Avoids unnecessary decimal places for integer values.
fn fmt_pt(value: f64) -> String {
    if (value - value.round()).abs() < 0.001 {
        format!("{}", value as i64)
    } else {
        format!("{:.1}", value)
    }
}

/// Base64-encode binary data.
///
/// Pure-Rust implementation with no external dependencies.
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{Color, NodeId};
    use s1_text::FontId;

    fn dummy_font_id() -> FontId {
        FontId(fontdb::ID::dummy())
    }

    fn dummy_node_id() -> NodeId {
        NodeId::new(0, 1)
    }

    /// Create a minimal LayoutDocument with one page, one paragraph, one line, one run.
    fn make_simple_doc(text: &str) -> LayoutDocument {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: text.len() as f64 * 7.2,
            hyperlink_url: None,
            text: text.to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine {
            baseline_y: 10.0,
            height: 14.4,
            runs: vec![run],
        };

        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };

        let page = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block],
            header: None,
            footer: None,
        };

        LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        }
    }

    #[test]
    fn html_empty_document() {
        let doc = LayoutDocument {
            pages: vec![LayoutPage {
                index: 0,
                width: 612.0,
                height: 792.0,
                content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
                blocks: Vec::new(),
                header: None,
                footer: None,
            }],
            bookmarks: Vec::new(),
        };
        let html = layout_to_html(&doc);
        assert!(html.contains("s1-document"));
        assert!(html.contains("s1-page"));
        assert!(html.contains("width:612pt"));
        assert!(html.contains("height:792pt"));
    }

    #[test]
    fn html_single_paragraph() {
        let doc = make_simple_doc("Hello World");
        let html = layout_to_html(&doc);
        assert!(html.contains("Hello World"));
        assert!(html.contains("s1-block"));
        assert!(html.contains("s1-line"));
        assert!(html.contains("font-size:12pt"));
    }

    #[test]
    fn html_multi_page() {
        let page1 = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: Vec::new(),
            header: None,
            footer: None,
        };
        let page2 = LayoutPage {
            index: 1,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: Vec::new(),
            header: None,
            footer: None,
        };
        let doc = LayoutDocument {
            pages: vec![page1, page2],
            bookmarks: Vec::new(),
        };
        let html = layout_to_html(&doc);
        let page_count = html.matches("s1-page").count();
        assert_eq!(page_count, 2);
    }

    #[test]
    fn html_formatting() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 14.0,
            color: Color::new(255, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "Bold Red".to_string(),
            bold: true,
            italic: true,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine {
            baseline_y: 10.0,
            height: 16.8,
            runs: vec![run],
        };

        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 16.8),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };

        let page = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block],
            header: None,
            footer: None,
        };

        let doc = LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        };

        let html = layout_to_html(&doc);
        assert!(html.contains("font-weight:bold"), "missing bold: {html}");
        assert!(html.contains("font-style:italic"), "missing italic: {html}");
        assert!(html.contains("color:#ff0000"), "missing color: {html}");
        assert!(html.contains("Bold Red"));
    }

    #[test]
    fn html_table() {
        let cell_block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(0.0, 0.0, 200.0, 14.4),
            kind: LayoutBlockKind::Paragraph {
                text_align: None,
                background_color: None,
                border: None,
                lines: vec![LayoutLine {
                    baseline_y: 10.0,
                    height: 14.4,
                    runs: vec![GlyphRun {
                        source_id: dummy_node_id(),
                        font_id: dummy_font_id(),
                        font_size: 12.0,
                        color: Color::new(0, 0, 0),
                        x_offset: 0.0,
                        glyphs: Vec::new(),
                        width: 40.0,
                        hyperlink_url: None,
                        text: "Cell 1".to_string(),
                        bold: false,
                        italic: false,
                        underline: false,
                        strikethrough: false,
                        superscript: false,
                        subscript: false,
                        highlight_color: None,
                        character_spacing: 0.0,
                        revision_type: None,
                        revision_author: None,
                    }],
                }],
            },
        };

        let cell = LayoutTableCell {
            bounds: Rect::new(0.0, 0.0, 200.0, 20.0),
            blocks: vec![cell_block],
            background_color: None,
            border_top: None,
            border_bottom: None,
            border_left: None,
            border_right: None,
        };

        let row = LayoutTableRow {
            bounds: Rect::new(0.0, 0.0, 400.0, 20.0),
            cells: vec![cell],
            is_header_row: false,
        };

        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 400.0, 20.0),
            kind: LayoutBlockKind::Table { rows: vec![row], is_continuation: false },
        };

        let page = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block],
            header: None,
            footer: None,
        };

        let doc = LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        };

        let html = layout_to_html(&doc);
        assert!(html.contains("s1-table"), "missing table class: {html}");
        assert!(html.contains("s1-table-row"), "missing row class: {html}");
        assert!(html.contains("s1-table-cell"), "missing cell class: {html}");
        assert!(html.contains("Cell 1"), "missing cell text: {html}");
    }

    #[test]
    fn html_image() {
        let image_bytes = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes (stub)
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 200.0, 150.0),
            kind: LayoutBlockKind::Image {
                media_id: "img1".to_string(),
                bounds: Rect::new(0.0, 0.0, 200.0, 150.0),
                image_data: Some(image_bytes),
                content_type: Some("image/png".to_string()),
            },
        };

        let page = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block],
            header: None,
            footer: None,
        };

        let doc = LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        };

        let html = layout_to_html(&doc);
        assert!(html.contains("data:image/png;base64,"), "missing base64 image: {html}");
        assert!(html.contains("s1-image"), "missing image class: {html}");
        assert!(html.contains("width:200pt"), "missing width: {html}");
        assert!(html.contains("height:150pt"), "missing height: {html}");
    }

    #[test]
    fn html_header_footer() {
        let header_run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 10.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "Header Text".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let footer_run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 10.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "Footer Text".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let header_block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 20.0, 468.0, 12.0),
            kind: LayoutBlockKind::Paragraph {
                lines: vec![LayoutLine {
                    baseline_y: 10.0,
                    height: 12.0,
                    runs: vec![header_run],
                }],
                text_align: None,
                background_color: None,
                border: None,
            },
        };

        let footer_block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 760.0, 468.0, 12.0),
            kind: LayoutBlockKind::Paragraph {
                lines: vec![LayoutLine {
                    baseline_y: 10.0,
                    height: 12.0,
                    runs: vec![footer_run],
                }],
                text_align: None,
                background_color: None,
                border: None,
            },
        };

        let page = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: Vec::new(),
            header: Some(header_block),
            footer: Some(footer_block),
        };

        let doc = LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        };

        let html = layout_to_html(&doc);
        assert!(html.contains("Header Text"), "missing header text: {html}");
        assert!(html.contains("Footer Text"), "missing footer text: {html}");
        // Header should appear before footer in the output
        let header_pos = html.find("Header Text").unwrap();
        let footer_pos = html.find("Footer Text").unwrap();
        assert!(header_pos < footer_pos, "header should come before footer");
    }

    #[test]
    fn html_hyperlinks() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 255),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 60.0,
            hyperlink_url: Some("https://example.com".to_string()),
            text: "Click here".to_string(),
            bold: false,
            italic: false,
            underline: true,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine {
            baseline_y: 10.0,
            height: 14.4,
            runs: vec![run],
        };

        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };

        let page = LayoutPage {
            index: 0,
            width: 612.0,
            height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block],
            header: None,
            footer: None,
        };

        let doc = LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        };

        let html = layout_to_html(&doc);
        assert!(
            html.contains("href=\"https://example.com\""),
            "missing hyperlink URL: {html}"
        );
        assert!(html.contains("Click here"), "missing link text: {html}");
        assert!(html.contains("<a "), "missing <a> tag: {html}");
    }

    #[test]
    fn html_page_dimensions() {
        let page = LayoutPage {
            index: 0,
            width: 595.28,
            height: 841.89,
            content_area: Rect::new(72.0, 72.0, 451.28, 697.89),
            blocks: Vec::new(),
            header: None,
            footer: None,
        };

        let doc = LayoutDocument {
            pages: vec![page],
            bookmarks: Vec::new(),
        };

        let html = layout_to_html(&doc);
        // A4 dimensions should appear in the style
        assert!(
            html.contains("width:595.3pt") || html.contains("width:595pt"),
            "missing A4 width: {html}"
        );
        assert!(
            html.contains("height:841.9pt") || html.contains("height:842pt"),
            "missing A4 height: {html}"
        );
    }

    #[test]
    fn html_escapes_special_chars() {
        let doc = make_simple_doc("Hello <World> & \"Friends\"");
        let html = layout_to_html(&doc);
        assert!(
            html.contains("Hello &lt;World&gt; &amp; &quot;Friends&quot;"),
            "special chars not escaped: {html}"
        );
        // Must NOT contain raw angle brackets in text
        assert!(!html.contains("<World>"), "raw angle brackets in output: {html}");
    }

    #[test]
    fn html_superscript() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 20.0,
            hyperlink_url: None,
            text: "2".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: true,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine { baseline_y: 10.0, height: 14.4, runs: vec![run] };
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };
        let page = LayoutPage {
            index: 0, width: 612.0, height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block], header: None, footer: None,
        };
        let doc = LayoutDocument { pages: vec![page], bookmarks: Vec::new() };
        let html = layout_to_html(&doc);
        assert!(html.contains("vertical-align:super"), "missing superscript: {html}");
    }

    #[test]
    fn html_subscript() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 20.0,
            hyperlink_url: None,
            text: "2".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: true,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine { baseline_y: 10.0, height: 14.4, runs: vec![run] };
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };
        let page = LayoutPage {
            index: 0, width: 612.0, height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block], header: None, footer: None,
        };
        let doc = LayoutDocument { pages: vec![page], bookmarks: Vec::new() };
        let html = layout_to_html(&doc);
        assert!(html.contains("vertical-align:sub"), "missing subscript: {html}");
    }

    #[test]
    fn html_highlight_color() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "Highlighted".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: Some(Color::new(255, 255, 0)),
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine { baseline_y: 10.0, height: 14.4, runs: vec![run] };
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };
        let page = LayoutPage {
            index: 0, width: 612.0, height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block], header: None, footer: None,
        };
        let doc = LayoutDocument { pages: vec![page], bookmarks: Vec::new() };
        let html = layout_to_html(&doc);
        assert!(html.contains("background-color:#ffff00"), "missing highlight: {html}");
    }

    #[test]
    fn html_track_changes_insertion() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "inserted text".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: Some("insertion".to_string()),
            revision_author: Some("Author A".to_string()),
        };

        let line = LayoutLine { baseline_y: 10.0, height: 14.4, runs: vec![run] };
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };
        let page = LayoutPage {
            index: 0, width: 612.0, height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block], header: None, footer: None,
        };
        let doc = LayoutDocument { pages: vec![page], bookmarks: Vec::new() };
        let html = layout_to_html(&doc);
        assert!(html.contains("<ins"), "missing <ins> tag: {html}");
        assert!(html.contains("color:green"), "missing green color: {html}");
        assert!(html.contains("Author A"), "missing author: {html}");
        assert!(html.contains("inserted text"), "missing text: {html}");
    }

    #[test]
    fn html_track_changes_deletion() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "deleted text".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: Some("deletion".to_string()),
            revision_author: Some("Author B".to_string()),
        };

        let line = LayoutLine { baseline_y: 10.0, height: 14.4, runs: vec![run] };
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };
        let page = LayoutPage {
            index: 0, width: 612.0, height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block], header: None, footer: None,
        };
        let doc = LayoutDocument { pages: vec![page], bookmarks: Vec::new() };
        let html = layout_to_html(&doc);
        assert!(html.contains("<del"), "missing <del> tag: {html}");
        assert!(html.contains("color:red"), "missing red color: {html}");
        assert!(html.contains("Author B"), "missing author: {html}");
        assert!(html.contains("deleted text"), "missing text: {html}");
    }

    #[test]
    fn html_character_spacing() {
        let run = GlyphRun {
            source_id: dummy_node_id(),
            font_id: dummy_font_id(),
            font_size: 12.0,
            color: Color::new(0, 0, 0),
            x_offset: 0.0,
            glyphs: Vec::new(),
            width: 50.0,
            hyperlink_url: None,
            text: "Spaced".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 2.5,
            revision_type: None,
            revision_author: None,
        };

        let line = LayoutLine { baseline_y: 10.0, height: 14.4, runs: vec![run] };
        let block = LayoutBlock {
            source_id: dummy_node_id(),
            bounds: Rect::new(72.0, 72.0, 468.0, 14.4),
            kind: LayoutBlockKind::Paragraph { lines: vec![line], text_align: None, background_color: None, border: None },
        };
        let page = LayoutPage {
            index: 0, width: 612.0, height: 792.0,
            content_area: Rect::new(72.0, 72.0, 468.0, 648.0),
            blocks: vec![block], header: None, footer: None,
        };
        let doc = LayoutDocument { pages: vec![page], bookmarks: Vec::new() };
        let html = layout_to_html(&doc);
        assert!(html.contains("letter-spacing:2.5pt"), "missing letter-spacing: {html}");
    }
}
