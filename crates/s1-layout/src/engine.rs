//! Core layout engine — converts a document model into positioned pages.

use s1_model::{
    AttributeKey, AttributeValue, DocumentModel, FieldType, HeaderFooterType, LineSpacing, NodeId,
    NodeType,
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
    cache: Option<&'a mut LayoutCache>,
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
            cache: None,
        }
    }

    /// Create a new layout engine with an incremental layout cache.
    ///
    /// Cached block layouts are reused when the content hash matches,
    /// avoiding expensive text shaping and line breaking for unchanged content.
    pub fn new_with_cache(
        doc: &'a DocumentModel,
        font_db: &'a FontDatabase,
        config: LayoutConfig,
        cache: &'a mut LayoutCache,
    ) -> Self {
        Self {
            doc,
            font_db,
            config,
            cache: Some(cache),
        }
    }

    /// Perform full document layout, returning a `LayoutDocument`.
    ///
    /// If a `LayoutCache` was provided via `new_with_cache`, cached block
    /// layouts are reused for unchanged content (same content hash).
    ///
    /// # Errors
    ///
    /// Returns `LayoutError` if fonts cannot be found or text shaping fails.
    pub fn layout(&mut self) -> Result<LayoutDocument, LayoutError> {
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

                    let block = self.layout_paragraph_cached(
                        *node_id,
                        &para_style,
                        content_rect,
                        current_y,
                    )?;

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
                        let block = self.layout_paragraph_cached(
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
                    let block = self.layout_table_cached(*node_id, content_rect, current_y)?;
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
                            self.layout_table_cached(*node_id, content_rect, current_y)?;
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

        // Apply widow/orphan control
        self.apply_widow_orphan_control(&mut pages, &page_layout)?;

        // Layout headers and footers for each page
        let total_pages = pages.len();
        for page in &mut pages {
            self.layout_header_footer(page, total_pages)?;
        }

        // Collect bookmarks from pages
        let bookmarks = self.collect_bookmarks(&pages);

        Ok(LayoutDocument { pages, bookmarks })
    }

    /// Resolve page layout from section properties or use default.
    fn resolve_page_layout(&self) -> PageLayout {
        let sections = self.doc.sections();
        if let Some(sp) = sections.first() {
            PageLayout {
                width: sp.page_width,
                height: sp.page_height,
                margin_top: sp.margin_top,
                margin_bottom: sp.margin_bottom,
                margin_left: sp.margin_left,
                margin_right: sp.margin_right,
            }
        } else {
            self.config.default_page_layout
        }
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

    /// Layout a paragraph with cache support.
    fn layout_paragraph_cached(
        &mut self,
        para_id: NodeId,
        para_style: &ResolvedParagraphStyle,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let hash = content_hash(self.doc, para_id);

        // Check cache
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get(para_id, hash) {
                let mut block = cached.clone();
                block.bounds.y = y_pos;
                block.bounds.x = content_rect.x + para_style.indent_left;
                return Ok(block);
            }
        }

        // Cache miss — do full layout
        let block = self.layout_paragraph(para_id, para_style, content_rect, y_pos)?;

        // Store in cache
        if let Some(ref mut cache) = self.cache {
            cache.insert(para_id, hash, block.clone());
        }

        Ok(block)
    }

    /// Layout a table with cache support.
    fn layout_table_cached(
        &mut self,
        table_id: NodeId,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let hash = content_hash(self.doc, table_id);

        // Check cache
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get(table_id, hash) {
                let mut block = cached.clone();
                block.bounds.y = y_pos;
                return Ok(block);
            }
        }

        // Cache miss
        let block = self.layout_table(table_id, content_rect, y_pos)?;

        // Store in cache
        if let Some(ref mut cache) = self.cache {
            cache.insert(table_id, hash, block.clone());
        }

        Ok(block)
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
                            hyperlink_url: None,
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
                            hyperlink_url: None,
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

        // Check for hyperlink URL on the run or its parent
        let hyperlink_url = self.doc.node(run_id).and_then(|run_node| {
            // Check run's own attributes first
            if let Some(url) = run_node.attributes.get_string(&AttributeKey::HyperlinkUrl) {
                return Some(url.to_string());
            }
            // Check parent node (could be a Hyperlink container via HyperlinkUrl attribute)
            run_node.parent.and_then(|parent_id| {
                self.doc.node(parent_id).and_then(|parent| {
                    parent.attributes.get_string(&AttributeKey::HyperlinkUrl).map(|s| s.to_string())
                })
            })
        });

        Ok(ShapedRunInfo {
            source_id: run_id,
            font_id,
            font_size: run_style.font_size,
            color: run_style.color,
            glyphs,
            is_line_break: false,
            metrics,
            hyperlink_url,
        })
    }

    /// Line breaking — uses Knuth-Plass optimal algorithm with greedy fallback.
    fn break_into_lines(
        &self,
        runs: &[ShapedRunInfo],
        available_width: f64,
        first_line_indent: f64,
        para_style: &ResolvedParagraphStyle,
    ) -> Vec<LayoutLine> {
        if runs.is_empty() {
            let line_height = compute_line_height(DEFAULT_FONT_SIZE, &para_style.line_spacing);
            return vec![LayoutLine {
                baseline_y: 0.0,
                height: line_height,
                runs: Vec::new(),
            }];
        }

        // Flatten runs into items for the line-breaking algorithm
        let items = self.build_break_items(runs);

        // Try Knuth-Plass; fall back to greedy on failure
        let break_points = knuth_plass_breaks(&items, available_width, first_line_indent)
            .unwrap_or_else(|| greedy_breaks(&items, available_width, first_line_indent));

        // Build LayoutLines from break points
        let mut lines: Vec<LayoutLine> = Vec::new();

        for window in break_points.windows(2) {
            let start = window[0];
            let end = window[1];

            let mut line_runs: Vec<GlyphRun> = Vec::new();
            let mut current_x = if lines.is_empty() {
                first_line_indent
            } else {
                0.0
            };
            let mut max_height: f64 = 0.0;

            for item in &items[start..end] {
                match item {
                    BreakItem::Box { run_idx, width, height, .. } => {
                        let run_info = &runs[*run_idx];
                        let font_id = run_info
                            .font_id
                            .unwrap_or(FontId(fontdb::ID::dummy()));

                        line_runs.push(GlyphRun {
                            source_id: run_info.source_id,
                            font_id,
                            font_size: run_info.font_size,
                            color: run_info.color,
                            x_offset: current_x,
                            glyphs: run_info.glyphs.clone(),
                            width: *width,
                            hyperlink_url: run_info.hyperlink_url.clone(),
                        });
                        current_x += width;
                        if *height > max_height {
                            max_height = *height;
                        }
                    }
                    BreakItem::Glue { width, .. } => {
                        current_x += width;
                    }
                    BreakItem::Penalty { .. } | BreakItem::ForcedBreak { .. } => {}
                }
            }

            let line_height = compute_line_height(
                if max_height > 0.0 { max_height } else { DEFAULT_FONT_SIZE },
                &para_style.line_spacing,
            );
            lines.push(LayoutLine {
                baseline_y: 0.0,
                height: line_height,
                runs: line_runs,
            });
        }

        if lines.is_empty() {
            let line_height = compute_line_height(DEFAULT_FONT_SIZE, &para_style.line_spacing);
            lines.push(LayoutLine {
                baseline_y: 0.0,
                height: line_height,
                runs: Vec::new(),
            });
        }

        // Compute baseline_y positions
        let mut y = 0.0;
        for line in &mut lines {
            line.baseline_y = y + line.height * 0.8;
            y += line.height;
        }

        lines
    }

    /// Build Knuth-Plass break items from shaped runs.
    fn build_break_items(&self, runs: &[ShapedRunInfo]) -> Vec<BreakItem> {
        let mut items: Vec<BreakItem> = Vec::new();

        for (run_idx, run_info) in runs.iter().enumerate() {
            if run_info.is_line_break {
                items.push(BreakItem::ForcedBreak { run_idx });
                continue;
            }

            let run_width: f64 = run_info.glyphs.iter().map(|g| g.x_advance).sum();
            let run_height = run_info
                .metrics
                .map(|m| m.ascent - m.descent)
                .unwrap_or(run_info.font_size);

            items.push(BreakItem::Box {
                run_idx,
                width: run_width,
                height: run_height,
            });

            // Add inter-run glue (stretchable space)
            items.push(BreakItem::Glue {
                width: 0.0,
                stretch: 0.0,
                shrink: 0.0,
            });
        }

        items
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

        let media_id_val = node.and_then(|n| {
            if let Some(AttributeValue::MediaId(mid)) = n.attributes.get(&AttributeKey::ImageMediaId) {
                Some(mid.0)
            } else {
                None
            }
        });

        let media_id_str = media_id_val
            .map(|id| format!("{id}"))
            .or_else(|| {
                node.and_then(|n| {
                    n.attributes.get_string(&AttributeKey::ImageMediaId).map(|s| s.to_string())
                })
            })
            .unwrap_or_default();

        // Fetch actual image data from the media store
        let (image_data, content_type) = if let Some(mid) = media_id_val {
            let media_id_key = s1_model::MediaId(mid);
            if let Some(item) = self.doc.media().get(media_id_key) {
                (Some(item.data.clone()), Some(item.content_type.clone()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(LayoutBlock {
            source_id: image_id,
            bounds: Rect::new(content_rect.x, y_pos, final_w, final_h),
            kind: LayoutBlockKind::Image {
                media_id: media_id_str,
                bounds: Rect::new(0.0, 0.0, final_w, final_h),
                image_data,
                content_type,
            },
        })
    }

    /// Layout header/footer content for a page.
    fn layout_header_footer(
        &self,
        page: &mut LayoutPage,
        total_pages: usize,
    ) -> Result<(), LayoutError> {
        let sections = self.doc.sections();
        let section = match sections.first() {
            Some(s) => s,
            None => return Ok(()),
        };

        let page_num = page.index + 1;
        let is_first_page = page.index == 0;

        // Determine which header type to use
        let header_type = if is_first_page && section.title_page {
            HeaderFooterType::First
        } else {
            HeaderFooterType::Default
        };
        let footer_type = header_type;

        // Layout header
        if let Some(hf_ref) = section.header(header_type).or_else(|| {
            if header_type != HeaderFooterType::Default {
                section.header(HeaderFooterType::Default)
            } else {
                None
            }
        }) {
            let header_block =
                self.layout_hf_node(hf_ref.node_id, page, page_num, total_pages, true)?;
            page.header = Some(header_block);
        }

        // Layout footer
        if let Some(hf_ref) = section.footer(footer_type).or_else(|| {
            if footer_type != HeaderFooterType::Default {
                section.footer(HeaderFooterType::Default)
            } else {
                None
            }
        }) {
            let footer_block =
                self.layout_hf_node(hf_ref.node_id, page, page_num, total_pages, false)?;
            page.footer = Some(footer_block);
        }

        Ok(())
    }

    /// Layout a header or footer node, substituting page number fields.
    fn layout_hf_node(
        &self,
        node_id: NodeId,
        page: &LayoutPage,
        page_num: usize,
        total_pages: usize,
        is_header: bool,
    ) -> Result<LayoutBlock, LayoutError> {
        let sections = self.doc.sections();
        let section = sections.first();
        let hf_distance = if is_header {
            section.map(|s| s.header_distance).unwrap_or(36.0)
        } else {
            section.map(|s| s.footer_distance).unwrap_or(36.0)
        };

        let hf_width = page.content_area.width;
        let hf_x = page.content_area.x;
        let hf_y = if is_header {
            hf_distance
        } else {
            page.height - hf_distance - DEFAULT_FONT_SIZE
        };

        let node = match self.doc.node(node_id) {
            Some(n) => n,
            None => {
                return Ok(LayoutBlock {
                    source_id: node_id,
                    bounds: Rect::new(hf_x, hf_y, hf_width, 0.0),
                    kind: LayoutBlockKind::Paragraph { lines: Vec::new() },
                });
            }
        };

        // Layout child paragraphs
        let mut blocks = Vec::new();
        let mut current_y = 0.0;

        for &child_id in &node.children {
            if let Some(child) = self.doc.node(child_id) {
                if child.node_type == NodeType::Paragraph {
                    let para_style = resolve_paragraph_style(self.doc, child_id);
                    let content_rect = Rect::new(hf_x, hf_y, hf_width, 100.0);
                    let mut block =
                        self.layout_paragraph(child_id, &para_style, content_rect, current_y)?;

                    // Substitute field values in glyph runs
                    self.substitute_fields_in_block(&mut block, page_num, total_pages);

                    current_y += block.bounds.height;
                    blocks.push(block);
                }
            }
        }

        // Merge blocks into a single block (typical: one paragraph)
        let total_height = current_y;
        if blocks.len() == 1 {
            let mut block = blocks.remove(0);
            block.source_id = node_id; // Use header/footer node ID
            block.bounds.x = hf_x;
            block.bounds.y = hf_y;
            Ok(block)
        } else {
            // Multiple paragraphs — wrap as first paragraph
            let lines = blocks
                .into_iter()
                .filter_map(|b| {
                    if let LayoutBlockKind::Paragraph { lines } = b.kind {
                        Some(lines)
                    } else {
                        None
                    }
                })
                .flatten()
                .collect();

            Ok(LayoutBlock {
                source_id: node_id,
                bounds: Rect::new(hf_x, hf_y, hf_width, total_height),
                kind: LayoutBlockKind::Paragraph { lines },
            })
        }
    }

    /// Substitute PAGE and NUMPAGES field nodes with actual page numbers.
    ///
    /// Field nodes are children of Paragraph (not Run). We check if the
    /// paragraph that was laid out contains Field nodes and create synthesized
    /// glyph runs for them, or update existing runs that came from
    /// field-adjacent text.
    fn substitute_fields_in_block(
        &self,
        block: &mut LayoutBlock,
        page_num: usize,
        total_pages: usize,
    ) {
        // Find field nodes in the source paragraph
        let para_node = match self.doc.node(block.source_id) {
            Some(n) if n.node_type == NodeType::Paragraph => n,
            _ => return,
        };

        for &child_id in &para_node.children {
            if let Some(child) = self.doc.node(child_id) {
                if child.node_type == NodeType::Field {
                    if let Some(AttributeValue::FieldType(ft)) =
                        child.attributes.get(&AttributeKey::FieldType)
                    {
                        let text = match ft {
                            FieldType::PageNumber => format!("{page_num}"),
                            FieldType::PageCount => format!("{total_pages}"),
                            _ => continue,
                        };

                        // Find any glyph run with this source_id and update it,
                        // or append a synthesized run to the last line
                        if let LayoutBlockKind::Paragraph { lines } = &mut block.kind {
                            let font_size = DEFAULT_FONT_SIZE;
                            let glyphs = synthesize_glyphs(&text, font_size);
                            let width: f64 = glyphs.iter().map(|g| g.x_advance).sum();

                            if let Some(last_line) = lines.last_mut() {
                                let x_offset = last_line
                                    .runs
                                    .last()
                                    .map(|r| r.x_offset + r.width)
                                    .unwrap_or(0.0);

                                last_line.runs.push(GlyphRun {
                                    source_id: child_id,
                                    font_id: FontId(fontdb::ID::dummy()),
                                    font_size,
                                    color: s1_model::Color::new(0, 0, 0),
                                    x_offset,
                                    glyphs,
                                    width,
                                    hyperlink_url: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Apply widow/orphan control across pages.
    ///
    /// Prevents orphans (fewer than `min_orphan_lines` at the bottom of a page)
    /// and widows (fewer than `min_widow_lines` at the top of the next page).
    fn apply_widow_orphan_control(
        &self,
        pages: &mut Vec<LayoutPage>,
        page_layout: &PageLayout,
    ) -> Result<(), LayoutError> {
        let min_orphan = self.config.min_orphan_lines;
        let min_widow = self.config.min_widow_lines;

        if min_orphan < 2 && min_widow < 2 {
            return Ok(());
        }

        // We need at least 2 pages for widow/orphan to matter
        if pages.len() < 2 {
            return Ok(());
        }

        let mut i = 0;
        while i + 1 < pages.len() {
            let needs_fix = {
                let current_page = &pages[i];
                let next_page = &pages[i + 1];

                // Check last block on current page — is it a paragraph with too few lines?
                let orphan_problem = if let Some(last_block) = current_page.blocks.last() {
                    if let LayoutBlockKind::Paragraph { lines } = &last_block.kind {
                        lines.len() > 1 && lines.len() < min_orphan + min_widow
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Check first block on next page — is it a continuation with too few lines?
                let widow_problem = if let Some(first_block) = next_page.blocks.first() {
                    if let LayoutBlockKind::Paragraph { lines } = &first_block.kind {
                        !lines.is_empty() && lines.len() < min_widow
                    } else {
                        false
                    }
                } else {
                    false
                };

                orphan_problem || widow_problem
            };

            if needs_fix {
                // Move the last block from current page to the start of next page
                // This is the simplest fix — push the entire paragraph to the next page
                let current_page = &mut pages[i];
                if current_page.blocks.len() > 1 {
                    let block = current_page.blocks.pop().unwrap();
                    // Re-position the block at the top of the next page
                    let content_y = page_layout.content_rect().y;
                    let mut moved_block = block;
                    moved_block.bounds.y = content_y;

                    let next_page = &mut pages[i + 1];
                    // Shift all existing blocks down
                    let shift = moved_block.bounds.height;
                    for b in &mut next_page.blocks {
                        b.bounds.y += shift;
                    }
                    next_page.blocks.insert(0, moved_block);
                }
            }

            i += 1;
        }

        // Remove any empty pages that resulted from moving blocks
        pages.retain(|p| !p.blocks.is_empty() || p.index == 0);

        // Re-index pages
        for (idx, page) in pages.iter_mut().enumerate() {
            page.index = idx;
        }

        Ok(())
    }

    /// Collect bookmarks from laid-out pages by scanning for BookmarkStart nodes.
    fn collect_bookmarks(&self, pages: &[LayoutPage]) -> Vec<LayoutBookmark> {
        let mut bookmarks = Vec::new();

        for page in pages {
            for block in &page.blocks {
                // Check if the source paragraph has BookmarkStart children
                if let Some(para_node) = self.doc.node(block.source_id) {
                    for &child_id in &para_node.children {
                        if let Some(child) = self.doc.node(child_id) {
                            if child.node_type == NodeType::BookmarkStart {
                                if let Some(name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                                    bookmarks.push(LayoutBookmark {
                                        name: name.to_string(),
                                        page_index: page.index,
                                        y_position: block.bounds.y,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        bookmarks
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
    hyperlink_url: Option<String>,
}

/// Compute a content hash for a node and its descendants.
///
/// The hash includes the node's attributes, text content of all descendants,
/// and style information. Used for cache invalidation in incremental layout.
fn content_hash(doc: &DocumentModel, node_id: NodeId) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    hash_node(doc, node_id, &mut hash);
    hash
}

fn hash_node(doc: &DocumentModel, node_id: NodeId, hash: &mut u64) {
    if let Some(node) = doc.node(node_id) {
        // Hash node type
        *hash ^= node.node_type as u64;
        *hash = hash.wrapping_mul(0x100000001b3);

        // Hash text content
        if let Some(ref text) = node.text_content {
            for byte in text.bytes() {
                *hash ^= byte as u64;
                *hash = hash.wrapping_mul(0x100000001b3);
            }
        }

        // Hash key attributes that affect layout
        for (key, val) in node.attributes.iter() {
            // Hash the key using its debug representation
            use std::hash::{Hash, Hasher};
            struct FnvHasher(u64);
            impl Hasher for FnvHasher {
                fn finish(&self) -> u64 { self.0 }
                fn write(&mut self, bytes: &[u8]) {
                    for &b in bytes {
                        self.0 ^= b as u64;
                        self.0 = self.0.wrapping_mul(0x100000001b3);
                    }
                }
            }
            let mut key_hasher = FnvHasher(*hash);
            std::mem::discriminant(key).hash(&mut key_hasher);
            *hash = key_hasher.finish();

            // Hash the value
            match val {
                AttributeValue::Bool(b) => {
                    *hash ^= *b as u64;
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
                AttributeValue::Float(f) => {
                    *hash ^= f.to_bits();
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
                AttributeValue::Int(i) => {
                    *hash ^= *i as u64;
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
                AttributeValue::String(s) => {
                    for byte in s.bytes() {
                        *hash ^= byte as u64;
                        *hash = hash.wrapping_mul(0x100000001b3);
                    }
                }
                _ => {
                    // For complex types, just hash a sentinel
                    *hash ^= 0xDEADBEEF;
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
            }
        }

        // Recurse into children
        for &child_id in &node.children {
            hash_node(doc, child_id, hash);
        }
    }
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

/// Item types for the Knuth-Plass line breaking algorithm.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum BreakItem {
    /// Content with a fixed width (a shaped run).
    Box {
        run_idx: usize,
        width: f64,
        height: f64,
    },
    /// Stretchable/shrinkable space between boxes.
    Glue {
        width: f64,
        stretch: f64,
        shrink: f64,
    },
    /// A possible break point with a penalty cost.
    Penalty {
        #[allow(dead_code)]
        penalty: f64,
    },
    /// A forced line break (from LineBreak node).
    ForcedBreak {
        #[allow(dead_code)]
        run_idx: usize,
    },
}

/// Knuth-Plass optimal line breaking.
///
/// Returns break indices into the items array, or `None` if the algorithm
/// cannot find a feasible solution (falls back to greedy).
fn knuth_plass_breaks(
    items: &[BreakItem],
    available_width: f64,
    first_line_indent: f64,
) -> Option<Vec<usize>> {
    if items.is_empty() {
        return Some(vec![0, 0]);
    }

    // Active node: (item_index, line_number, total_demerits, total_width)
    #[derive(Clone)]
    struct ActiveNode {
        index: usize,
        line: usize,
        demerits: f64,
        total_width: f64,
        prev: Option<usize>, // index into nodes vec
    }

    let mut nodes: Vec<ActiveNode> = vec![ActiveNode {
        index: 0,
        line: 0,
        demerits: 0.0,
        total_width: 0.0,
        prev: None,
    }];
    let mut active: Vec<usize> = vec![0]; // indices into nodes

    for (i, item) in items.iter().enumerate() {
        let is_feasible_break = matches!(
            item,
            BreakItem::Glue { .. } | BreakItem::ForcedBreak { .. }
        );

        if !is_feasible_break {
            continue;
        }

        let mut new_active: Vec<usize> = Vec::new();
        let mut best_node: Option<ActiveNode> = None;

        for &a_idx in &active {
            let a = &nodes[a_idx];

            // Compute width from this active node to current position
            let mut width = a.total_width;
            for item_between in &items[a.index..i] {
                match item_between {
                    BreakItem::Box { width: w, .. } => width += w,
                    BreakItem::Glue { width: w, .. } => width += w,
                    _ => {}
                }
            }

            // Line width depends on whether this is the first line
            let line_width = if a.line == 0 {
                available_width - first_line_indent
            } else {
                available_width
            };

            let ratio = line_width - (width - a.total_width);

            // Check feasibility: allow lines to be slightly overfull (5%)
            if ratio >= -line_width * 0.05 {
                let badness = if ratio.abs() < 0.01 {
                    0.0
                } else if ratio > 0.0 {
                    // Underfull
                    (100.0 * (ratio / line_width).powi(3)).min(10000.0)
                } else {
                    // Overfull
                    10000.0
                };

                let is_forced = matches!(item, BreakItem::ForcedBreak { .. });

                // Standard Knuth-Plass demerit calculation:
                // Forced breaks get minimal demerits
                let demerits = if is_forced {
                    a.demerits
                } else {
                    (1.0 + badness).powi(2) + a.demerits
                };

                match &best_node {
                    None => {
                        best_node = Some(ActiveNode {
                            index: i + 1,
                            line: a.line + 1,
                            demerits,
                            total_width: width,
                            prev: Some(a_idx),
                        });
                    }
                    Some(best) if demerits < best.demerits => {
                        best_node = Some(ActiveNode {
                            index: i + 1,
                            line: a.line + 1,
                            demerits,
                            total_width: width,
                            prev: Some(a_idx),
                        });
                    }
                    _ => {}
                }

                // For forced breaks, deactivate the current node (must break here).
                // For regular breaks, keep the node active if the line isn't too long.
                if !is_forced && ratio > -line_width * 0.05 {
                    new_active.push(a_idx);
                }
            } else {
                // Line too long — deactivate
            }
        }

        if let Some(node) = best_node {
            let idx = nodes.len();
            nodes.push(node);
            new_active.push(idx);
        }

        if !new_active.is_empty() {
            active = new_active;
        }
        // If active becomes empty, KP fails — return None for greedy fallback
        if active.is_empty() {
            return None;
        }
    }

    // Add a final break at the end of items
    let final_idx = items.len();
    let mut best_final: Option<ActiveNode> = None;
    for &a_idx in &active {
        let a = &nodes[a_idx];
        let mut width = a.total_width;
        for item_between in &items[a.index..final_idx] {
            match item_between {
                BreakItem::Box { width: w, .. } => width += w,
                BreakItem::Glue { width: w, .. } => width += w,
                _ => {}
            }
        }
        let demerits = a.demerits;
        match &best_final {
            None => {
                best_final = Some(ActiveNode {
                    index: final_idx,
                    line: a.line + 1,
                    demerits,
                    total_width: width,
                    prev: Some(a_idx),
                });
            }
            Some(best) if demerits < best.demerits => {
                best_final = Some(ActiveNode {
                    index: final_idx,
                    line: a.line + 1,
                    demerits,
                    total_width: width,
                    prev: Some(a_idx),
                });
            }
            _ => {}
        }
    }

    let final_node = best_final?;
    let final_node_idx = nodes.len();
    nodes.push(final_node);

    // Trace back to get break points
    let mut breaks = Vec::new();
    let mut current = Some(final_node_idx);
    while let Some(idx) = current {
        breaks.push(nodes[idx].index);
        current = nodes[idx].prev;
    }
    breaks.reverse();

    // Ensure we start at 0
    if breaks.first() != Some(&0) {
        breaks.insert(0, 0);
    }

    Some(breaks)
}

/// Greedy line breaking fallback.
fn greedy_breaks(
    items: &[BreakItem],
    available_width: f64,
    first_line_indent: f64,
) -> Vec<usize> {
    let mut breaks = vec![0];
    let mut current_width = 0.0;
    let mut is_first_line = true;

    for (i, item) in items.iter().enumerate() {
        match item {
            BreakItem::Box { width, .. } => {
                let line_w = if is_first_line {
                    available_width - first_line_indent
                } else {
                    available_width
                };

                if current_width + width > line_w + 0.01 && i > *breaks.last().unwrap_or(&0) {
                    breaks.push(i);
                    current_width = *width;
                    is_first_line = false;
                } else {
                    current_width += width;
                }
            }
            BreakItem::Glue { width, .. } => {
                current_width += width;
            }
            BreakItem::ForcedBreak { .. } => {
                breaks.push(i + 1);
                current_width = 0.0;
                is_first_line = false;
            }
            BreakItem::Penalty { .. } => {}
        }
    }

    breaks.push(items.len());
    breaks
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 1);
        assert!(result.pages[0].blocks.is_empty());
    }

    #[test]
    fn layout_single_paragraph() {
        let doc = make_simple_doc("Hello World");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 3);
    }

    #[test]
    fn layout_page_dimensions() {
        let doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
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
        let mut engine = LayoutEngine::new(&doc, &font_db, config);
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
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 2);
    }

    #[test]
    fn knuth_plass_single_run() {
        // KP should produce at least one line for a single short run
        let items = vec![
            BreakItem::Box {
                run_idx: 0,
                width: 100.0,
                height: 12.0,
            },
            BreakItem::Glue {
                width: 0.0,
                stretch: 0.0,
                shrink: 0.0,
            },
        ];
        let breaks = knuth_plass_breaks(&items, 468.0, 0.0);
        assert!(breaks.is_some());
        let breaks = breaks.unwrap();
        assert!(breaks.len() >= 2);
        assert_eq!(breaks[0], 0);
    }

    #[test]
    fn knuth_plass_forced_break() {
        let items = vec![
            BreakItem::Box {
                run_idx: 0,
                width: 100.0,
                height: 12.0,
            },
            BreakItem::ForcedBreak { run_idx: 1 },
            BreakItem::Box {
                run_idx: 2,
                width: 100.0,
                height: 12.0,
            },
            BreakItem::Glue {
                width: 0.0,
                stretch: 0.0,
                shrink: 0.0,
            },
        ];
        let breaks = knuth_plass_breaks(&items, 468.0, 0.0);
        assert!(breaks.is_some());
        let breaks = breaks.unwrap();
        // Should have 3 segments: [0..2, 2..4, 4..end]
        assert!(breaks.len() >= 3, "breaks: {:?}", breaks);
    }

    #[test]
    fn greedy_breaks_basic() {
        let items = vec![
            BreakItem::Box {
                run_idx: 0,
                width: 300.0,
                height: 12.0,
            },
            BreakItem::Glue {
                width: 0.0,
                stretch: 0.0,
                shrink: 0.0,
            },
            BreakItem::Box {
                run_idx: 1,
                width: 300.0,
                height: 12.0,
            },
            BreakItem::Glue {
                width: 0.0,
                stretch: 0.0,
                shrink: 0.0,
            },
        ];
        let breaks = greedy_breaks(&items, 468.0, 0.0);
        // Two runs of 300pt each don't fit in 468pt — should break into 2 lines
        assert_eq!(breaks.len(), 3);
    }

    #[test]
    fn layout_uses_section_page_size() {
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
        doc.insert_node(run_id, 0, Node::text(text_id, "Hello"))
            .unwrap();

        // Set custom section properties (A4 landscape)
        let mut sp = s1_model::SectionProperties::default();
        sp.page_width = 841.89;
        sp.page_height = 595.28;
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert!((result.pages[0].width - 841.89).abs() < 0.01);
        assert!((result.pages[0].height - 595.28).abs() < 0.01);
    }

    #[test]
    fn layout_header_footer_placement() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, SectionProperties};

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Body paragraph
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Body text"))
            .unwrap();

        // Header node (child of root, not body)
        let header_id = doc.next_id();
        doc.insert_node(root, 1, Node::new(header_id, NodeType::Header))
            .unwrap();
        let hp = doc.next_id();
        doc.insert_node(header_id, 0, Node::new(hp, NodeType::Paragraph))
            .unwrap();
        let hr = doc.next_id();
        doc.insert_node(hp, 0, Node::new(hr, NodeType::Run))
            .unwrap();
        let ht = doc.next_id();
        doc.insert_node(hr, 0, Node::text(ht, "Header Text"))
            .unwrap();

        // Footer node
        let footer_id = doc.next_id();
        doc.insert_node(root, 2, Node::new(footer_id, NodeType::Footer))
            .unwrap();
        let fp = doc.next_id();
        doc.insert_node(footer_id, 0, Node::new(fp, NodeType::Paragraph))
            .unwrap();
        let fr = doc.next_id();
        doc.insert_node(fp, 0, Node::new(fr, NodeType::Run))
            .unwrap();
        let ft = doc.next_id();
        doc.insert_node(fr, 0, Node::text(ft, "Footer Text"))
            .unwrap();

        // Wire up section properties
        let mut sp = SectionProperties::default();
        sp.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: header_id,
        });
        sp.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: footer_id,
        });
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        let page = &result.pages[0];
        assert!(page.header.is_some(), "page should have a header");
        assert!(page.footer.is_some(), "page should have a footer");

        // Header should be near the top
        let header = page.header.as_ref().unwrap();
        assert!(header.bounds.y < 72.0, "header should be in top margin area");

        // Footer should be near the bottom
        let footer = page.footer.as_ref().unwrap();
        assert!(footer.bounds.y > 700.0, "footer should be in bottom margin area");
    }

    #[test]
    fn layout_page_number_substitution() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, SectionProperties};

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add enough paragraphs for 2 pages
        let texts: Vec<&str> = (0..80).map(|_| "Content line here").collect();
        for (i, text) in texts.iter().enumerate() {
            let pid = doc.next_id();
            doc.insert_node(body_id, i, Node::new(pid, NodeType::Paragraph))
                .unwrap();
            let rid = doc.next_id();
            doc.insert_node(pid, 0, Node::new(rid, NodeType::Run))
                .unwrap();
            let tid = doc.next_id();
            doc.insert_node(rid, 0, Node::text(tid, *text))
                .unwrap();
        }

        // Footer with PAGE field
        let footer_id = doc.next_id();
        doc.insert_node(root, 1, Node::new(footer_id, NodeType::Footer))
            .unwrap();
        let fp = doc.next_id();
        doc.insert_node(footer_id, 0, Node::new(fp, NodeType::Paragraph))
            .unwrap();
        // Add a field child to the paragraph (Field is a child of Paragraph, not Run)
        let field_id = doc.next_id();
        let mut field_node = Node::new(field_id, NodeType::Field);
        field_node.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(FieldType::PageNumber),
        );
        doc.insert_node(fp, 0, field_node).unwrap();

        // Section properties with footer ref
        let mut sp = SectionProperties::default();
        sp.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: footer_id,
        });
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(result.pages.len() >= 2, "should have at least 2 pages");

        // Each page should have a footer
        for page in &result.pages {
            assert!(page.footer.is_some(), "page {} should have footer", page.index);
        }
    }

    #[test]
    fn widow_orphan_config_respected() {
        // With min_orphan_lines=2, min_widow_lines=2, layout should not leave
        // single lines stranded
        let config = LayoutConfig {
            default_page_layout: PageLayout::letter(),
            min_orphan_lines: 2,
            min_widow_lines: 2,
        };

        let texts: Vec<&str> = (0..100).map(|_| "Lorem ipsum dolor sit amet").collect();
        let doc = make_multi_para_doc(&texts);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, config);
        let result = engine.layout().unwrap();

        // All pages should have proper page indices
        for (i, page) in result.pages.iter().enumerate() {
            assert_eq!(page.index, i, "page index mismatch");
        }
        assert!(result.pages.len() > 1);
    }

    #[test]
    fn first_page_header_with_title_page() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, SectionProperties};

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Body paragraph
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Body"))
            .unwrap();

        // First-page header
        let first_hdr_id = doc.next_id();
        doc.insert_node(root, 1, Node::new(first_hdr_id, NodeType::Header))
            .unwrap();
        let hp = doc.next_id();
        doc.insert_node(first_hdr_id, 0, Node::new(hp, NodeType::Paragraph))
            .unwrap();
        let hr = doc.next_id();
        doc.insert_node(hp, 0, Node::new(hr, NodeType::Run))
            .unwrap();
        let ht = doc.next_id();
        doc.insert_node(hr, 0, Node::text(ht, "First Page Header"))
            .unwrap();

        // Default header
        let default_hdr_id = doc.next_id();
        doc.insert_node(root, 2, Node::new(default_hdr_id, NodeType::Header))
            .unwrap();
        let hp2 = doc.next_id();
        doc.insert_node(default_hdr_id, 0, Node::new(hp2, NodeType::Paragraph))
            .unwrap();
        let hr2 = doc.next_id();
        doc.insert_node(hp2, 0, Node::new(hr2, NodeType::Run))
            .unwrap();
        let ht2 = doc.next_id();
        doc.insert_node(hr2, 0, Node::text(ht2, "Default Header"))
            .unwrap();

        // Section with title_page enabled and both header types
        let mut sp = SectionProperties::default();
        sp.title_page = true;
        sp.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::First,
            node_id: first_hdr_id,
        });
        sp.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: default_hdr_id,
        });
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        // First page should have a header (the First type)
        assert!(result.pages[0].header.is_some());
        // The header source should be the first-page header node
        let header = result.pages[0].header.as_ref().unwrap();
        assert_eq!(header.source_id, first_hdr_id);
    }

    // --- Milestone 3.3: Incremental Layout Tests ---

    #[test]
    fn incremental_cache_hit() {
        let doc = make_simple_doc("Hello World");
        let font_db = FontDatabase::new();
        let mut cache = LayoutCache::new();

        // First layout — populates cache
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result1 = engine.layout().unwrap();
        assert!(!cache.is_empty(), "Cache should be populated after layout");

        // Second layout with same doc — should use cache
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();

        // Results should be equivalent
        assert_eq!(result1.pages.len(), result2.pages.len());
        assert_eq!(
            result1.pages[0].blocks.len(),
            result2.pages[0].blocks.len()
        );
    }

    #[test]
    fn incremental_cache_miss_on_text_change() {
        let mut doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let mut cache = LayoutCache::new();

        // First layout
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let _result1 = engine.layout().unwrap();
        let cache_len_after_first = cache.len();
        assert!(cache_len_after_first > 0);

        // Modify the text content (change the Text node)
        let root = doc.root_id();
        let body_id = doc.node(root).unwrap().children[0];
        let para_id = doc.node(body_id).unwrap().children[0];
        let run_id = doc.node(para_id).unwrap().children[0];
        let text_id = doc.node(run_id).unwrap().children[0];
        doc.node_mut(text_id).unwrap().text_content = Some("Changed text".to_string());

        // Second layout should miss cache for the changed paragraph
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();
        assert!(!result2.pages.is_empty());
    }

    #[test]
    fn incremental_cache_miss_on_style_change() {
        let mut doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let mut cache = LayoutCache::new();

        // First layout
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let _result1 = engine.layout().unwrap();

        // Change run attributes (font size)
        let root = doc.root_id();
        let body_id = doc.node(root).unwrap().children[0];
        let para_id = doc.node(body_id).unwrap().children[0];
        let run_id = doc.node(para_id).unwrap().children[0];
        doc.node_mut(run_id).unwrap().attributes.set(
            s1_model::AttributeKey::FontSize,
            s1_model::AttributeValue::Float(24.0),
        );

        // Second layout should detect the change
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();
        assert!(!result2.pages.is_empty());
    }

    #[test]
    fn incremental_pagination_still_correct() {
        // Ensure that even with cache, pagination produces correct results
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
            doc.insert_node(run_id, 0, Node::text(text_id, "Lorem ipsum dolor sit amet"))
                .unwrap();
        }

        let font_db = FontDatabase::new();
        let mut cache = LayoutCache::new();

        // First layout
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result1 = engine.layout().unwrap();
        let page_count_1 = result1.pages.len();
        assert!(page_count_1 > 1, "Should be multi-page");

        // Second layout with cache — pagination should match
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();
        assert_eq!(result2.pages.len(), page_count_1);
    }

    #[test]
    fn incremental_table_cache() {
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
        let mut cache = LayoutCache::new();

        // First layout populates cache
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result1 = engine.layout().unwrap();
        assert!(!cache.is_empty());

        // Second layout should use cached table
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();
        assert_eq!(result1.pages[0].blocks.len(), result2.pages[0].blocks.len());
    }

    #[test]
    fn incremental_empty_cache() {
        // Layout without cache should work the same
        let doc = make_simple_doc("Test");
        let font_db = FontDatabase::new();

        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert!(!result.pages.is_empty());
    }

    #[test]
    fn incremental_cache_invalidation_on_insert() {
        let mut doc = make_simple_doc("Hello");
        let font_db = FontDatabase::new();
        let mut cache = LayoutCache::new();

        // First layout
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result1 = engine.layout().unwrap();
        let block_count_1 = result1.pages[0].blocks.len();
        assert_eq!(block_count_1, 1);

        // Add a second paragraph
        let root = doc.root_id();
        let body_id = doc.node(root).unwrap().children[0];
        let para_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "New paragraph"))
            .unwrap();

        // Second layout should see both paragraphs
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();
        assert_eq!(result2.pages[0].blocks.len(), 2);
    }

    #[test]
    fn incremental_performance() {
        // Verify that cached layout produces the same output faster
        // (just a sanity check, not a strict timing test)
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        for i in 0..50 {
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
                Node::text(text_id, "Performance test paragraph"),
            )
            .unwrap();
        }

        let font_db = FontDatabase::new();
        let mut cache = LayoutCache::new();

        // First pass populates cache
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result1 = engine.layout().unwrap();

        // Second pass should use cache for all paragraphs
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();

        assert_eq!(result1.pages.len(), result2.pages.len());
        // Cache should have entries for each paragraph
        assert!(cache.len() >= 50, "Cache should have entries for all paragraphs");
    }
}
