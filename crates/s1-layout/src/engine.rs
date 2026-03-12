//! Core layout engine — converts a document model into positioned pages.

use s1_model::{
    AttributeKey, AttributeValue, DocumentModel, LineSpacing, NodeId, NodeType,
};
use s1_text::{FontDatabase, FontId, FontMetrics, ShapedGlyph};

use crate::error::LayoutError;
use crate::style_resolver::{
    resolve_paragraph_style, resolve_run_style, ResolvedParagraphStyle,
    DEFAULT_FONT_FAMILY, DEFAULT_FONT_SIZE,
};
use crate::types::*;

/// Layout engine configuration.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Default page layout for sections without explicit properties.
    pub default_page_layout: PageLayout,
    /// Minimum number of lines at the top of a page (orphan control).
    pub min_orphan_lines: usize,
    /// Minimum number of lines at the bottom before a break (widow control).
    pub min_widow_lines: usize,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            default_page_layout: PageLayout::letter(),
            min_orphan_lines: 2,
            min_widow_lines: 2,
        }
    }
}

/// The layout engine. Converts a `DocumentModel` into a `LayoutDocument`.
pub struct LayoutEngine<'a> {
    doc: &'a DocumentModel,
    font_db: &'a FontDatabase,
    config: LayoutConfig,
}

impl<'a> LayoutEngine<'a> {
    /// Create a new layout engine.
    pub fn new(
        doc: &'a DocumentModel,
        font_db: &'a FontDatabase,
        config: LayoutConfig,
    ) -> Self {
        Self {
            doc,
            font_db,
            config,
        }
    }

    /// Perform full document layout, returning a `LayoutDocument`.
    ///
    /// # Errors
    ///
    /// Returns `LayoutError` if fonts cannot be found or text shaping fails.
    pub fn layout(&self) -> Result<LayoutDocument, LayoutError> {
        let page_layout = self.resolve_page_layout();
        let content_rect = page_layout.content_rect();

        // Collect all block-level nodes in document order
        let blocks = self.collect_blocks();

        // Layout each block and paginate
        let mut pages: Vec<LayoutPage> = Vec::new();
        let mut current_y = content_rect.y;
        let mut page_blocks: Vec<LayoutBlock> = Vec::new();
        let mut page_index = 0;

        for (node_id, node_type) in &blocks {
            match node_type {
                NodeType::Paragraph => {
                    let para_style = resolve_paragraph_style(self.doc, *node_id);

                    // Handle page break before
                    if para_style.page_break_before && !page_blocks.is_empty() {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                        ));
                        page_index += 1;
                        current_y = content_rect.y;
                    }

                    // Add spacing before
                    current_y += para_style.space_before;

                    let block =
                        self.layout_paragraph(*node_id, &para_style, content_rect, current_y)?;

                    let block_height = block.bounds.height;

                    // Check if this block fits on the current page
                    if current_y + block_height > content_rect.bottom()
                        && !page_blocks.is_empty()
                    {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                        ));
                        page_index += 1;
                        current_y = content_rect.y + para_style.space_before;

                        // Re-layout at top of new page
                        let block = self.layout_paragraph(
                            *node_id,
                            &para_style,
                            content_rect,
                            current_y,
                        )?;
                        let block_height = block.bounds.height;
                        page_blocks.push(block);
                        current_y += block_height + para_style.space_after;
                    } else {
                        page_blocks.push(block);
                        current_y += block_height + para_style.space_after;
                    }
                }
                NodeType::Table => {
                    let block = self.layout_table(*node_id, content_rect, current_y)?;
                    let block_height = block.bounds.height;

                    if current_y + block_height > content_rect.bottom()
                        && !page_blocks.is_empty()
                    {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                        ));
                        page_index += 1;
                        current_y = content_rect.y;

                        let block =
                            self.layout_table(*node_id, content_rect, current_y)?;
                        let block_height = block.bounds.height;
                        page_blocks.push(block);
                        current_y += block_height;
                    } else {
                        page_blocks.push(block);
                        current_y += block_height;
                    }
                }
                NodeType::Image => {
                    let block = self.layout_image(*node_id, content_rect, current_y)?;
                    let block_height = block.bounds.height;

                    if current_y + block_height > content_rect.bottom()
                        && !page_blocks.is_empty()
                    {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                        ));
                        page_index += 1;
                        current_y = content_rect.y;

                        let block = self.layout_image(*node_id, content_rect, current_y)?;
                        let block_height = block.bounds.height;
                        page_blocks.push(block);
                        current_y += block_height;
                    } else {
                        page_blocks.push(block);
                        current_y += block_height;
                    }
                }
                _ => {} // Skip other node types
            }
        }

        // Flush remaining blocks
        if !page_blocks.is_empty() {
            pages.push(self.make_page(page_index, &page_layout, page_blocks));
        }

        // Ensure at least one page
        if pages.is_empty() {
            pages.push(LayoutPage {
                index: 0,
                width: page_layout.width,
                height: page_layout.height,
                content_area: content_rect,
                blocks: Vec::new(),
                header: None,
                footer: None,
            });
        }

        Ok(LayoutDocument { pages })
    }

    /// Resolve page layout from section properties or use default.
    fn resolve_page_layout(&self) -> PageLayout {
        // Look for section properties on the document
        // For now, use default. Section-specific page sizes will be added in 3.3.
        self.config.default_page_layout
    }

    /// Collect all block-level node IDs in document order.
    fn collect_blocks(&self) -> Vec<(NodeId, NodeType)> {
        let mut blocks = Vec::new();
        let root_id = self.doc.root_id();

        // Find Body node
        if let Some(root) = self.doc.node(root_id) {
            for &child_id in &root.children {
                if let Some(child) = self.doc.node(child_id) {
                    if child.node_type == NodeType::Body {
                        self.collect_body_blocks(child_id, &mut blocks);
                    }
                }
            }
        }

        blocks
    }

    fn collect_body_blocks(&self, body_id: NodeId, blocks: &mut Vec<(NodeId, NodeType)>) {
        if let Some(body) = self.doc.node(body_id) {
            for &child_id in &body.children {
                if let Some(child) = self.doc.node(child_id) {
                    match child.node_type {
                        NodeType::Paragraph | NodeType::Table | NodeType::Image => {
                            blocks.push((child_id, child.node_type));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Layout a paragraph — shape text, break into lines.
    fn layout_paragraph(
        &self,
        para_id: NodeId,
        para_style: &ResolvedParagraphStyle,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let available_width =
            content_rect.width - para_style.indent_left - para_style.indent_right;
        let x_start = content_rect.x + para_style.indent_left;

        let para = match self.doc.node(para_id) {
            Some(n) => n,
            None => {
                return Ok(LayoutBlock {
                    source_id: para_id,
                    bounds: Rect::new(x_start, y_pos, available_width, 0.0),
                    kind: LayoutBlockKind::Paragraph { lines: Vec::new() },
                });
            }
        };

        // Collect shaped runs for this paragraph
        let mut shaped_runs: Vec<ShapedRunInfo> = Vec::new();

        for &child_id in &para.children {
            if let Some(child) = self.doc.node(child_id) {
                match child.node_type {
                    NodeType::Run => {
                        let run_info = self.shape_run(child_id)?;
                        shaped_runs.push(run_info);
                    }
                    NodeType::LineBreak => {
                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_size: DEFAULT_FONT_SIZE,
                            color: s1_model::Color::new(0, 0, 0),
                            glyphs: Vec::new(),
                            is_line_break: true,
                            metrics: None,
                        });
                    }
                    NodeType::Tab => {
                        // Represent tab as a space-like run with fixed advance
                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_size: DEFAULT_FONT_SIZE,
                            color: s1_model::Color::new(0, 0, 0),
                            glyphs: vec![ShapedGlyph {
                                glyph_id: 0,
                                x_advance: 36.0, // ~0.5" tab stop
                                y_advance: 0.0,
                                x_offset: 0.0,
                                y_offset: 0.0,
                                cluster: 0,
                            }],
                            is_line_break: false,
                            metrics: None,
                        });
                    }
                    _ => {}
                }
            }
        }

        // Break into lines (greedy algorithm)
        let first_line_indent = para_style.indent_first_line;
        let lines = self.break_into_lines(
            &shaped_runs,
            available_width,
            first_line_indent,
            para_style,
        );

        // Compute total paragraph height
        let total_height: f64 = lines.iter().map(|l| l.height).sum();

        Ok(LayoutBlock {
            source_id: para_id,
            bounds: Rect::new(x_start, y_pos, available_width, total_height),
            kind: LayoutBlockKind::Paragraph { lines },
        })
    }

    /// Shape a run node — resolve font, shape text, return shaped info.
    fn shape_run(&self, run_id: NodeId) -> Result<ShapedRunInfo, LayoutError> {
        let run_style = resolve_run_style(self.doc, run_id);

        // Find font
        let font_id = self
            .font_db
            .find(&run_style.font_family, run_style.bold, run_style.italic)
            .or_else(|| self.font_db.find(DEFAULT_FONT_FAMILY, false, false))
            .or_else(|| self.font_db.find("Helvetica", false, false))
            .or_else(|| self.font_db.find("Arial", false, false))
            .or_else(|| self.font_db.find("DejaVu Sans", false, false));

        // Collect text from children
        let mut text = String::new();
        if let Some(run) = self.doc.node(run_id) {
            for &child_id in &run.children {
                if let Some(child) = self.doc.node(child_id) {
                    if child.node_type == NodeType::Text {
                        if let Some(t) = &child.text_content {
                            text.push_str(t);
                        }
                    }
                }
            }
        }

        let (glyphs, metrics) = if let Some(fid) = font_id {
            if let Some(font) = self.font_db.load_font(fid) {
                let font_size = if run_style.superscript || run_style.subscript {
                    run_style.font_size * 0.65 // sub/super at ~65% size
                } else {
                    run_style.font_size
                };

                let glyphs = s1_text::shape_text(
                    &text,
                    &font,
                    font_size,
                    &[],
                    None,
                    s1_text::Direction::Ltr,
                )?;
                let metrics = font.metrics(font_size);
                (glyphs, Some(metrics))
            } else {
                (synthesize_glyphs(&text, run_style.font_size), None)
            }
        } else {
            (synthesize_glyphs(&text, run_style.font_size), None)
        };

        Ok(ShapedRunInfo {
            source_id: run_id,
            font_id,
            font_size: run_style.font_size,
            color: run_style.color,
            glyphs,
            is_line_break: false,
            metrics,
        })
    }

    /// Greedy line breaking algorithm.
    fn break_into_lines(
        &self,
        runs: &[ShapedRunInfo],
        available_width: f64,
        first_line_indent: f64,
        para_style: &ResolvedParagraphStyle,
    ) -> Vec<LayoutLine> {
        if runs.is_empty() {
            // Empty paragraph — one empty line with default height
            let line_height = compute_line_height(DEFAULT_FONT_SIZE, &para_style.line_spacing);
            return vec![LayoutLine {
                baseline_y: 0.0,
                height: line_height,
                runs: Vec::new(),
            }];
        }

        let mut lines: Vec<LayoutLine> = Vec::new();
        let mut current_runs: Vec<GlyphRun> = Vec::new();
        let mut current_x = first_line_indent;
        let mut line_width = if lines.is_empty() {
            available_width - first_line_indent
        } else {
            available_width
        };
        let mut max_line_height: f64 = 0.0;

        for run_info in runs {
            if run_info.is_line_break {
                // Force a new line
                let line_height = if max_line_height > 0.0 {
                    compute_line_height(max_line_height, &para_style.line_spacing)
                } else {
                    compute_line_height(DEFAULT_FONT_SIZE, &para_style.line_spacing)
                };
                lines.push(LayoutLine {
                    baseline_y: 0.0, // Will be computed later
                    height: line_height,
                    runs: std::mem::take(&mut current_runs),
                });
                current_x = 0.0;
                line_width = available_width;
                max_line_height = 0.0;
                continue;
            }

            let run_width: f64 = run_info.glyphs.iter().map(|g| g.x_advance).sum();
            let run_height = run_info
                .metrics
                .map(|m| m.ascent - m.descent)
                .unwrap_or(run_info.font_size);

            if current_x + run_width > line_width + 0.01 && !current_runs.is_empty() {
                // This run doesn't fit — break line
                let line_height = compute_line_height(
                    if max_line_height > 0.0 {
                        max_line_height
                    } else {
                        run_info.font_size
                    },
                    &para_style.line_spacing,
                );
                lines.push(LayoutLine {
                    baseline_y: 0.0,
                    height: line_height,
                    runs: std::mem::take(&mut current_runs),
                });
                current_x = 0.0;
                line_width = available_width;
                max_line_height = 0.0;
            }

            if run_height > max_line_height {
                max_line_height = run_height;
            }

            let font_id = run_info
                .font_id
                .unwrap_or(FontId(fontdb::ID::dummy()));

            current_runs.push(GlyphRun {
                source_id: run_info.source_id,
                font_id,
                font_size: run_info.font_size,
                color: run_info.color,
                x_offset: current_x,
                glyphs: run_info.glyphs.clone(),
                width: run_width,
            });

            current_x += run_width;
        }

        // Flush remaining runs
        if !current_runs.is_empty() {
            let line_height = compute_line_height(
                if max_line_height > 0.0 {
                    max_line_height
                } else {
                    DEFAULT_FONT_SIZE
                },
                &para_style.line_spacing,
            );
            lines.push(LayoutLine {
                baseline_y: 0.0,
                height: line_height,
                runs: current_runs,
            });
        }

        // Compute baseline_y positions
        let mut y = 0.0;
        for line in &mut lines {
            line.baseline_y = y + line.height * 0.8; // Approximate baseline
            y += line.height;
        }

        lines
    }

    /// Layout a table — compute column widths, row heights.
    fn layout_table(
        &self,
        table_id: NodeId,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let table = match self.doc.node(table_id) {
            Some(n) => n,
            None => {
                return Ok(LayoutBlock {
                    source_id: table_id,
                    bounds: Rect::new(content_rect.x, y_pos, content_rect.width, 0.0),
                    kind: LayoutBlockKind::Table { rows: Vec::new() },
                });
            }
        };

        // Count columns from first row
        let num_cols = table
            .children
            .first()
            .and_then(|&row_id| self.doc.node(row_id))
            .map(|row| row.children.len())
            .unwrap_or(1)
            .max(1);

        let col_width = content_rect.width / num_cols as f64;
        let mut rows: Vec<LayoutTableRow> = Vec::new();
        let mut table_y = 0.0;

        for &row_id in &table.children {
            if let Some(row_node) = self.doc.node(row_id) {
                if row_node.node_type != NodeType::TableRow {
                    continue;
                }

                let mut cells: Vec<LayoutTableCell> = Vec::new();
                let mut max_cell_height: f64 = 20.0; // Minimum row height

                for (col_idx, &cell_id) in row_node.children.iter().enumerate() {
                    if let Some(cell_node) = self.doc.node(cell_id) {
                        if cell_node.node_type != NodeType::TableCell {
                            continue;
                        }

                        let cell_x = col_idx as f64 * col_width;
                        let cell_rect = Rect::new(cell_x, 0.0, col_width, 0.0);

                        // Layout cell content (paragraphs inside the cell)
                        let mut cell_blocks = Vec::new();
                        let mut cell_y = 2.0; // Padding

                        for &content_id in &cell_node.children {
                            if let Some(content) = self.doc.node(content_id) {
                                if content.node_type == NodeType::Paragraph {
                                    let ps = resolve_paragraph_style(self.doc, content_id);
                                    let cell_content_rect =
                                        Rect::new(cell_x + 2.0, cell_y, col_width - 4.0, 1000.0);
                                    let block = self.layout_paragraph(
                                        content_id,
                                        &ps,
                                        cell_content_rect,
                                        cell_y,
                                    )?;
                                    cell_y += block.bounds.height + ps.space_after;
                                    cell_blocks.push(block);
                                }
                            }
                        }

                        cell_y += 2.0; // Bottom padding
                        if cell_y > max_cell_height {
                            max_cell_height = cell_y;
                        }

                        cells.push(LayoutTableCell {
                            bounds: cell_rect,
                            blocks: cell_blocks,
                        });
                    }
                }

                // Set actual cell heights
                for cell in &mut cells {
                    cell.bounds.height = max_cell_height;
                }

                rows.push(LayoutTableRow {
                    bounds: Rect::new(0.0, table_y, content_rect.width, max_cell_height),
                    cells,
                });
                table_y += max_cell_height;
            }
        }

        Ok(LayoutBlock {
            source_id: table_id,
            bounds: Rect::new(content_rect.x, y_pos, content_rect.width, table_y),
            kind: LayoutBlockKind::Table { rows },
        })
    }

    /// Layout an image node.
    fn layout_image(
        &self,
        image_id: NodeId,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let node = self.doc.node(image_id);
        let (width, height) = node
            .map(|n| {
                let w = n
                    .attributes
                    .get(&AttributeKey::ImageWidth)
                    .and_then(|v| {
                        if let AttributeValue::Float(d) = v {
                            Some(*d)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(100.0);
                let h = n
                    .attributes
                    .get(&AttributeKey::ImageHeight)
                    .and_then(|v| {
                        if let AttributeValue::Float(d) = v {
                            Some(*d)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(100.0);
                (w, h)
            })
            .unwrap_or((100.0, 100.0));

        // Constrain to content width
        let scale = if width > content_rect.width {
            content_rect.width / width
        } else {
            1.0
        };

        let final_w = width * scale;
        let final_h = height * scale;

        let media_id = node
            .and_then(|n| {
                if let Some(AttributeValue::MediaId(mid)) = n.attributes.get(&AttributeKey::ImageMediaId) {
                    Some(format!("{}", mid.0))
                } else {
                    n.attributes.get_string(&AttributeKey::ImageMediaId).map(|s| s.to_string())
                }
            })
            .unwrap_or_default();

        Ok(LayoutBlock {
            source_id: image_id,
            bounds: Rect::new(content_rect.x, y_pos, final_w, final_h),
            kind: LayoutBlockKind::Image {
                media_id,
                bounds: Rect::new(0.0, 0.0, final_w, final_h),
            },
        })
    }

    fn make_page(
        &self,
        index: usize,
        page_layout: &PageLayout,
        blocks: Vec<LayoutBlock>,
    ) -> LayoutPage {
        LayoutPage {
            index,
            width: page_layout.width,
            height: page_layout.height,
            content_area: page_layout.content_rect(),
            blocks,
            header: None,
            footer: None,
        }
    }
}

/// Shaped run info — intermediate result before line breaking.
struct ShapedRunInfo {
    source_id: NodeId,
    font_id: Option<FontId>,
    font_size: f64,
    color: s1_model::Color,
    glyphs: Vec<ShapedGlyph>,
    is_line_break: bool,
    metrics: Option<FontMetrics>,
}

/// Synthesize glyphs when no font is available (fallback for headless testing).
fn synthesize_glyphs(text: &str, font_size: f64) -> Vec<ShapedGlyph> {
    let avg_advance = font_size * 0.6; // Rough average character width
    text.char_indices()
        .map(|(i, _)| ShapedGlyph {
            glyph_id: 0,
            x_advance: avg_advance,
            y_advance: 0.0,
            x_offset: 0.0,
            y_offset: 0.0,
            cluster: i as u32,
        })
        .collect()
}

/// Compute line height from the tallest run and line spacing.
fn compute_line_height(max_run_height: f64, line_spacing: &LineSpacing) -> f64 {
    match line_spacing {
        LineSpacing::Single => max_run_height,
        LineSpacing::OnePointFive => max_run_height * 1.5,
        LineSpacing::Double => max_run_height * 2.0,
        LineSpacing::Multiple(m) => max_run_height * m,
        LineSpacing::AtLeast(min) => max_run_height.max(*min),
        LineSpacing::Exact(exact) => *exact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{Node, NodeType};

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

    fn make_multi_para_doc(texts: &[&str]) -> DocumentModel {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        for (i, text) in texts.iter().enumerate() {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, *text))
                .unwrap();
        }
        doc
    }

    #[test]
    fn layout_empty_document() {
        let doc = DocumentModel::new();
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 1);
        assert!(result.pages[0].blocks.is_empty());
    }

    #[test]
    fn layout_single_paragraph() {
        let doc = make_simple_doc("Hello World");
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Paragraph { lines } => {
                assert!(!lines.is_empty());
                assert!(!lines[0].runs.is_empty());
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn layout_multiple_paragraphs() {
        let doc = make_multi_para_doc(&["First", "Second", "Third"]);
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 3);
    }

    #[test]
    fn layout_page_dimensions() {
        let doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        let page = &result.pages[0];
        assert_eq!(page.width, 612.0);
        assert_eq!(page.height, 792.0);
        assert_eq!(page.content_area.x, 72.0);
    }

    #[test]
    fn layout_paragraph_has_correct_position() {
        let doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        let block = &result.pages[0].blocks[0];
        // Should start at content area origin
        assert!(block.bounds.x >= 72.0);
        assert!(block.bounds.y >= 72.0);
    }

    #[test]
    fn layout_pagination() {
        // Create enough paragraphs to fill multiple pages
        let texts: Vec<&str> = (0..100).map(|_| "Lorem ipsum dolor sit amet").collect();
        let doc = make_multi_para_doc(&texts);
        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert!(
            result.pages.len() > 1,
            "expected multiple pages, got {}",
            result.pages.len()
        );
    }

    #[test]
    fn layout_empty_paragraph() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        // No runs — empty paragraph

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Paragraph { lines } => {
                assert_eq!(lines.len(), 1);
                assert!(lines[0].height > 0.0);
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn layout_with_bold_text() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Bold, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run).unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Bold text"))
            .unwrap();

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages[0].blocks.len(), 1);
    }

    #[test]
    fn layout_table() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Create a 2x2 table
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
                doc.insert_node(
                    run_id,
                    0,
                    Node::text(text_id, &format!("R{row_idx}C{col_idx}")),
                )
                .unwrap();
            }
        }

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Table { rows } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].cells.len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn layout_with_custom_page_size() {
        let doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let config = LayoutConfig {
            default_page_layout: PageLayout::a4(),
            ..Default::default()
        };
        let engine = LayoutEngine::new(&doc, &font_db, config);
        let result = engine.layout().unwrap();
        assert!((result.pages[0].width - 595.28).abs() < 0.01);
    }

    #[test]
    fn synthesize_glyphs_for_text() {
        let glyphs = synthesize_glyphs("ABC", 12.0);
        assert_eq!(glyphs.len(), 3);
        assert!((glyphs[0].x_advance - 7.2).abs() < 0.01); // 12 * 0.6
    }

    #[test]
    fn compute_line_height_multiple() {
        let h = compute_line_height(12.0, &LineSpacing::Multiple(1.5));
        assert_eq!(h, 18.0);
    }

    #[test]
    fn compute_line_height_at_least() {
        let h = compute_line_height(12.0, &LineSpacing::AtLeast(20.0));
        assert_eq!(h, 20.0);
        let h2 = compute_line_height(25.0, &LineSpacing::AtLeast(20.0));
        assert_eq!(h2, 25.0);
    }

    #[test]
    fn compute_line_height_exact() {
        let h = compute_line_height(12.0, &LineSpacing::Exact(15.0));
        assert_eq!(h, 15.0);
    }

    #[test]
    fn page_break_before() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // First paragraph
        let p1 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Page 1"))
            .unwrap();

        // Second paragraph with page break before
        let p2 = doc.next_id();
        let mut p2_node = Node::new(p2, NodeType::Paragraph);
        p2_node
            .attributes
            .set(AttributeKey::PageBreakBefore, AttributeValue::Bool(true));
        doc.insert_node(body_id, 1, p2_node).unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Page 2"))
            .unwrap();

        let font_db = FontDatabase::new();
        let engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 2);
    }
}
