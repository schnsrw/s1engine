//! Core layout engine — converts a document model into positioned pages.

use s1_model::{
    AttributeKey, AttributeValue, DocumentModel, FieldType, HeaderFooterType, LineSpacing, NodeId,
    NodeType, SectionBreakType,
};
use s1_text::{FontDatabase, FontId, FontMetrics, ShapedGlyph};

use crate::error::LayoutError;
use crate::style_resolver::{
    resolve_paragraph_style, resolve_run_style, ResolvedParagraphStyle, ResolvedRunStyle,
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
    pub fn new(doc: &'a DocumentModel, font_db: &'a FontDatabase, config: LayoutConfig) -> Self {
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
        // Collect all block-level nodes in document order
        let blocks = self.collect_blocks();

        // Build a mapping from block index to section index.
        let section_map = self.build_section_map(&blocks);
        let sections = self.doc.sections();

        // Determine the initial section index and page layout
        let initial_section_idx = if blocks.is_empty() {
            if sections.is_empty() {
                0
            } else {
                sections.len() - 1
            }
        } else {
            section_map[0]
        };
        let mut current_section_idx = initial_section_idx;
        let mut page_layout = self.resolve_page_layout_for_section(current_section_idx);
        let mut content_rect = page_layout.content_rect();

        // Layout each block and paginate
        let mut pages: Vec<LayoutPage> = Vec::new();
        let mut current_y = content_rect.y;
        let mut page_blocks: Vec<LayoutBlock> = Vec::new();
        let mut page_index = 0;
        // Track which section each page belongs to (for header/footer resolution)
        let mut page_section_indices: Vec<usize> = Vec::new();
        // Track previous block's space_after for CSS-style margin collapsing
        let mut prev_space_after: f64 = 0.0;

        for (block_idx, (node_id, node_type)) in blocks.iter().enumerate() {
            let block_section_idx = section_map[block_idx];

            // Handle section change
            if block_section_idx != current_section_idx {
                // Flush current page before switching sections
                if !page_blocks.is_empty() {
                    pages.push(self.make_page(
                        page_index,
                        &page_layout,
                        std::mem::take(&mut page_blocks),
                        current_section_idx,
                    ));
                    page_section_indices.push(current_section_idx);
                    page_index += 1;
                }

                // Determine break type from the NEW section's properties
                let break_type = sections
                    .get(block_section_idx)
                    .and_then(|s| s.break_type)
                    .unwrap_or(SectionBreakType::NextPage);

                match break_type {
                    SectionBreakType::NextPage => {
                        // Already flushed — just switch layout
                    }
                    SectionBreakType::Continuous => {
                        // Don't force a new page; the new layout takes effect
                        // on the NEXT page.
                    }
                    SectionBreakType::EvenPage => {
                        // Ensure the next content starts on an even page
                        // (page_index is 0-based, page number = page_index + 1)
                        let next_page_num = page_index + 1;
                        if next_page_num % 2 != 0 {
                            // Insert a blank page to land on even
                            pages.push(self.make_page(
                                page_index,
                                &page_layout,
                                Vec::new(),
                                current_section_idx,
                            ));
                            page_section_indices.push(current_section_idx);
                            page_index += 1;
                        }
                    }
                    SectionBreakType::OddPage => {
                        let next_page_num = page_index + 1;
                        if next_page_num % 2 != 1 {
                            // Insert a blank page to land on odd
                            pages.push(self.make_page(
                                page_index,
                                &page_layout,
                                Vec::new(),
                                current_section_idx,
                            ));
                            page_section_indices.push(current_section_idx);
                            page_index += 1;
                        }
                    }
                    _ => {
                        // Unknown break types treated as NextPage
                    }
                }

                // Switch to the new section's page layout
                current_section_idx = block_section_idx;
                page_layout = self.resolve_page_layout_for_section(current_section_idx);
                content_rect = page_layout.content_rect();
                current_y = content_rect.y;
            }

            match node_type {
                NodeType::Paragraph => {
                    let para_style = resolve_paragraph_style(self.doc, *node_id);

                    // Handle page break before
                    if para_style.page_break_before && !page_blocks.is_empty() {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                            current_section_idx,
                        ));
                        page_section_indices.push(current_section_idx);
                        page_index += 1;
                        current_y = content_rect.y;
                        prev_space_after = 0.0;
                    }

                    // CSS-style margin collapsing: use the larger of
                    // previous block's space_after and this block's space_before
                    let space_before = sanitize_pt(para_style.space_before);
                    let collapsed_spacing = if prev_space_after > 0.0 || space_before > 0.0 {
                        space_before.max(prev_space_after) - prev_space_after
                    } else {
                        0.0
                    };
                    current_y += collapsed_spacing;

                    let block = self.layout_paragraph_cached(
                        *node_id,
                        &para_style,
                        content_rect,
                        current_y,
                    )?;

                    let block_height = block.bounds.height;
                    let space_after = sanitize_pt(para_style.space_after);

                    // Check if this block fits on the current page
                    if current_y + block_height > content_rect.bottom()
                        && !page_blocks.is_empty()
                    {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                            current_section_idx,
                        ));
                        page_section_indices.push(current_section_idx);
                        page_index += 1;
                        // At top of new page, apply full space_before (no collapse)
                        current_y = content_rect.y + space_before;

                        // Re-layout at top of new page
                        let block = self.layout_paragraph_cached(
                            *node_id,
                            &para_style,
                            content_rect,
                            current_y,
                        )?;
                        let block_height = block.bounds.height;
                        page_blocks.push(block);
                        current_y += block_height + space_after;
                        prev_space_after = space_after;
                    } else {
                        page_blocks.push(block);
                        current_y += block_height + space_after;
                        prev_space_after = space_after;
                    }
                }
                NodeType::Table => {
                    // Layout all rows independently for cross-page splitting
                    let all_rows = self.layout_table_rows(*node_id, content_rect)?;

                    if all_rows.is_empty() {
                        // Empty table — emit an empty table block
                        let block = LayoutBlock {
                            source_id: *node_id,
                            bounds: Rect::new(content_rect.x, current_y, content_rect.width, 0.0),
                            kind: LayoutBlockKind::Table {
                                rows: Vec::new(),
                                is_continuation: false,
                            },
                        };
                        page_blocks.push(block);
                    } else {
                        // Collect header rows (rows marked as header at the start)
                        let header_rows: Vec<LayoutTableRow> = all_rows
                            .iter()
                            .take_while(|r| r.is_header_row)
                            .cloned()
                            .collect();

                        let mut row_idx = 0;
                        let mut is_first_chunk = true;

                        while row_idx < all_rows.len() {
                            let available = content_rect.bottom() - current_y;
                            let mut chunk_rows: Vec<LayoutTableRow> = Vec::new();
                            let mut chunk_height = 0.0;

                            // If this is a continuation, prepend header rows — but
                            // only if there are non-header data rows remaining
                            // (avoids infinite header duplication when all rows are headers).
                            let has_non_header_remaining = all_rows[row_idx..]
                                .iter()
                                .any(|r| !r.is_header_row);
                            if !is_first_chunk
                                && !header_rows.is_empty()
                                && has_non_header_remaining
                            {
                                for hr in &header_rows {
                                    let mut hdr = hr.clone();
                                    hdr.bounds.y = chunk_height;
                                    chunk_rows.push(hdr);
                                    chunk_height += hr.bounds.height;
                                }
                            }

                            // Add data rows that fit
                            let mut added_any_data_row = false;
                            while row_idx < all_rows.len() {
                                let row = &all_rows[row_idx];
                                let row_h = row.bounds.height;

                                if chunk_height + row_h > available && added_any_data_row {
                                    // This row won't fit and we already have content
                                    break;
                                }

                                // Place the row (even if it overflows — prevents infinite loop
                                // for a single very tall row)
                                let mut placed_row = row.clone();
                                placed_row.bounds.y = chunk_height;
                                chunk_rows.push(placed_row);
                                chunk_height += row_h;
                                row_idx += 1;
                                // Skip header rows in data iteration if this is first chunk
                                // (they are already included naturally)
                                added_any_data_row = true;
                            }

                            // Emit a table block for this chunk
                            let is_continuation = !is_first_chunk;
                            let block = LayoutBlock {
                                source_id: *node_id,
                                bounds: Rect::new(
                                    content_rect.x,
                                    current_y,
                                    content_rect.width,
                                    chunk_height,
                                ),
                                kind: LayoutBlockKind::Table {
                                    rows: chunk_rows,
                                    is_continuation,
                                },
                            };
                            page_blocks.push(block);
                            current_y += chunk_height;

                            // If there are more rows, start a new page
                            if row_idx < all_rows.len() {
                                pages.push(self.make_page(
                                    page_index,
                                    &page_layout,
                                    std::mem::take(&mut page_blocks),
                                    current_section_idx,
                                ));
                                page_section_indices.push(current_section_idx);
                                page_index += 1;
                                current_y = content_rect.y;
                                is_first_chunk = false;
                            }
                        }
                    }
                }
                NodeType::Image => {
                    let block =
                        self.layout_image(*node_id, content_rect, current_y)?;
                    let block_height = block.bounds.height;

                    if current_y + block_height > content_rect.bottom()
                        && !page_blocks.is_empty()
                    {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                            current_section_idx,
                        ));
                        page_section_indices.push(current_section_idx);
                        page_index += 1;
                        current_y = content_rect.y;

                        let block = self.layout_image(
                            *node_id,
                            content_rect,
                            current_y,
                        )?;
                        let block_height = block.bounds.height;
                        page_blocks.push(block);
                        current_y += block_height;
                    } else {
                        page_blocks.push(block);
                        current_y += block_height;
                    }
                }
                NodeType::PageBreak => {
                    // Force a page break
                    if !page_blocks.is_empty() {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                            current_section_idx,
                        ));
                        page_section_indices.push(current_section_idx);
                        page_index += 1;
                        current_y = content_rect.y;
                    }
                }
                _ => {} // Skip other node types
            }
        }

        // Flush remaining blocks
        if !page_blocks.is_empty() {
            pages.push(self.make_page(page_index, &page_layout, page_blocks, current_section_idx));
            page_section_indices.push(current_section_idx);
        }

        // Ensure at least one page
        if pages.is_empty() {
            let default_layout =
                self.resolve_page_layout_for_section(initial_section_idx);
            pages.push(LayoutPage {
                index: 0,
                width: default_layout.width,
                height: default_layout.height,
                content_area: default_layout.content_rect(),
                blocks: Vec::new(),
                header: None,
                footer: None,
                section_index: initial_section_idx,
            });
            page_section_indices.push(initial_section_idx);
        }

        // Apply widow/orphan control (uses per-page dimensions)
        self.apply_widow_orphan_control(&mut pages, &page_section_indices)?;

        // Layout headers and footers for each page using the correct section
        let total_pages = pages.len();
        for (i, page) in pages.iter_mut().enumerate() {
            let sect_idx =
                page_section_indices.get(i).copied().unwrap_or(0);
            self.layout_header_footer_for_section(
                page,
                total_pages,
                sect_idx,
            )?;
        }

        // Collect bookmarks from pages
        let bookmarks = self.collect_bookmarks(&pages);

        Ok(LayoutDocument { pages, bookmarks })
    }

    /// Resolve page layout for a specific section index.
    fn resolve_page_layout_for_section(&self, section_idx: usize) -> PageLayout {
        let sections = self.doc.sections();
        if let Some(sp) = sections.get(section_idx) {
            PageLayout {
                width: sp.page_width,
                height: sp.page_height,
                margin_top: sp.margin_top,
                margin_bottom: sp.margin_bottom,
                margin_left: sp.margin_left,
                margin_right: sp.margin_right,
            }
        } else if let Some(sp) = sections.last() {
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

    /// Build a mapping from block index to section index.
    ///
    /// In DOCX, a paragraph with `SectionIndex(i)` marks the END of section `i`.
    /// All blocks from the previous section boundary up to and including that
    /// paragraph belong to section `i`. Blocks after the last marked paragraph
    /// belong to the final section (the last entry in `doc.sections()`).
    fn build_section_map(
        &self,
        blocks: &[(NodeId, NodeType)],
    ) -> Vec<usize> {
        let sections = self.doc.sections();
        let num_sections = sections.len();

        if blocks.is_empty() || num_sections <= 1 {
            // Single section or no sections: all blocks belong to the same section
            let idx = if num_sections > 0 {
                num_sections - 1
            } else {
                0
            };
            return vec![idx; blocks.len()];
        }

        let mut result = vec![0usize; blocks.len()];

        // Find blocks that have SectionIndex attributes.
        // These mark the END of a section.
        let mut section_end_blocks: Vec<(usize, usize)> = Vec::new();

        for (block_idx, (node_id, _)) in blocks.iter().enumerate() {
            if let Some(node) = self.doc.node(*node_id) {
                if let Some(sect_idx) =
                    node.attributes.get_i64(&AttributeKey::SectionIndex)
                {
                    section_end_blocks.push((block_idx, sect_idx as usize));
                }
            }
        }

        // Assign section indices to blocks.
        // Blocks up to and including the first section-end marker belong to
        // that section. Between markers, blocks belong to the next marker's
        // section. After the last marker, blocks belong to the final section.
        let mut current_section = if let Some(&(_, sect_idx)) =
            section_end_blocks.first()
        {
            sect_idx
        } else {
            num_sections - 1
        };

        let mut marker_idx = 0;

        for (block_idx, entry) in result.iter_mut().enumerate() {
            if marker_idx < section_end_blocks.len() {
                let (end_block, sect_idx) = section_end_blocks[marker_idx];
                if block_idx <= end_block {
                    current_section = sect_idx;
                } else {
                    // Past this marker
                    marker_idx += 1;
                    if marker_idx < section_end_blocks.len() {
                        current_section =
                            section_end_blocks[marker_idx].1;
                    } else {
                        current_section = num_sections - 1;
                    }
                }
            }
            *entry = current_section;
        }

        result
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
                        NodeType::TableOfContents => {
                            // Expand TOC child paragraphs inline
                            for &toc_child_id in &child.children {
                                if let Some(toc_child) = self.doc.node(toc_child_id) {
                                    if toc_child.node_type == NodeType::Paragraph {
                                        blocks.push((toc_child_id, NodeType::Paragraph));
                                    }
                                }
                            }
                        }
                        NodeType::Section => {
                            // Recursively collect blocks inside Section containers
                            self.collect_body_blocks(child_id, blocks);
                        }
                        NodeType::PageBreak => {
                            // Treat standalone page break as a paragraph with PageBreakBefore
                            blocks.push((child_id, NodeType::PageBreak));
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
        let mut hash = content_hash(self.doc, para_id);
        // Include available width, indent, and spacing in hash so cache invalidates
        // when layout context changes (H-08 fix)
        hash ^= content_rect.width.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= para_style.indent_left.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= para_style.indent_right.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= para_style.indent_first_line.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        // Include alignment so cache invalidates on alignment change (L-18)
        hash ^= para_style.alignment as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        // Include spacing so cache invalidates on spacing change
        hash ^= para_style.space_before.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= para_style.space_after.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        // Include line spacing
        hash ^= match &para_style.line_spacing {
            LineSpacing::Single => 1u64,
            LineSpacing::OnePointFive => 2,
            LineSpacing::Double => 3,
            LineSpacing::Multiple(m) => 4 ^ m.to_bits(),
            LineSpacing::AtLeast(v) => 5 ^ v.to_bits(),
            LineSpacing::Exact(v) => 6 ^ v.to_bits(),
            _ => 7,
        };
        hash = hash.wrapping_mul(0x100000001b3);
        // Include keep flags and page break
        hash ^= (para_style.keep_with_next as u64) | ((para_style.keep_lines as u64) << 1) | ((para_style.page_break_before as u64) << 2);
        hash = hash.wrapping_mul(0x100000001b3);

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



    /// Layout a paragraph — shape text, break into lines.
    fn layout_paragraph(
        &self,
        para_id: NodeId,
        para_style: &ResolvedParagraphStyle,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let indent_left = sanitize_pt(para_style.indent_left);
        let indent_right = sanitize_pt(para_style.indent_right);
        let available_width = content_rect.width - indent_left - indent_right;
        let x_start = content_rect.x + indent_left;

        let para = match self.doc.node(para_id) {
            Some(n) => n,
            None => {
                return Ok(LayoutBlock {
                    source_id: para_id,
                    bounds: Rect::new(x_start, y_pos, available_width, 0.0),
                    kind: LayoutBlockKind::Paragraph { lines: Vec::new(), text_align: None, background_color: None, border: None, list_marker: None, list_level: 0, space_before: para_style.space_before, space_after: para_style.space_after, indent_left: para_style.indent_left, indent_right: para_style.indent_right, indent_first_line: para_style.indent_first_line, line_height: line_spacing_to_css(&para_style.line_spacing) },
                });
            }
        };

        // Collect shaped runs for this paragraph
        let mut shaped_runs: Vec<ShapedRunInfo> = Vec::new();

        for &child_id in &para.children {
            if let Some(child) = self.doc.node(child_id) {
                match child.node_type {
                    NodeType::Run => {
                        // Check if this run contains any LineBreak children —
                        // if so, split into multiple shaped runs at each break.
                        let mut has_inline_break = false;
                        if let Some(run_node) = self.doc.node(child_id) {
                            for &sub_id in &run_node.children {
                                if let Some(sub) = self.doc.node(sub_id) {
                                    if sub.node_type == NodeType::LineBreak {
                                        has_inline_break = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if has_inline_break {
                            // Split into segments at each LineBreak
                            self.shape_run_with_breaks(child_id, &mut shaped_runs)?;
                        } else {
                            let run_info = self.shape_run(child_id)?;
                            shaped_runs.push(run_info);
                        }
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
                            text: String::new(),
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
                            inline_image: None,
                        });
                    }
                    NodeType::Tab => {
                        // Compute current x position from accumulated shaped runs
                        let current_x: f64 = shaped_runs.iter().map(|r| {
                            let glyph_w: f64 = r.glyphs.iter().map(|g| g.x_advance).sum();
                            let nc = r.text.chars().count();
                            let sp = if nc > 1 { (nc as f64 - 1.0) * r.character_spacing } else { 0.0 };
                            glyph_w + sp
                        }).sum();

                        // Look up paragraph tab stops from attributes
                        let tab_advance = if let Some(AttributeValue::TabStops(ref stops)) =
                            para.attributes.get(&AttributeKey::TabStops)
                        {
                            // Find the next tab stop position after current_x
                            let mut next_stop: Option<f64> = None;
                            for ts in stops {
                                if ts.position > current_x + 0.5 {
                                    match next_stop {
                                        Some(prev) if ts.position < prev => {
                                            next_stop = Some(ts.position);
                                        }
                                        None => {
                                            next_stop = Some(ts.position);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            if let Some(stop_pos) = next_stop {
                                (stop_pos - current_x).max(1.0)
                            } else {
                                // No custom tab stop found — fall back to default 36pt interval
                                let default_interval = 36.0;
                                let next_default = ((current_x / default_interval).floor() + 1.0) * default_interval;
                                (next_default - current_x).max(1.0)
                            }
                        } else {
                            // No custom tab stops — use default 36pt (0.5") interval
                            let default_interval = 36.0;
                            let next_default = ((current_x / default_interval).floor() + 1.0) * default_interval;
                            (next_default - current_x).max(1.0)
                        };

                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_size: DEFAULT_FONT_SIZE,
                            color: s1_model::Color::new(0, 0, 0),
                            glyphs: vec![ShapedGlyph {
                                glyph_id: 0,
                                x_advance: tab_advance,
                                y_advance: 0.0,
                                x_offset: 0.0,
                                y_offset: 0.0,
                                cluster: 0,
                            }],
                            is_line_break: false,
                            metrics: None,
                            hyperlink_url: None,
                            text: "\t".to_string(),
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
                            inline_image: None,
                        });
                    }
                    // Bookmark/Comment markers — no visual output, skip silently
                    NodeType::BookmarkStart
                    | NodeType::BookmarkEnd
                    | NodeType::CommentStart
                    | NodeType::CommentEnd => {}
                    // Field nodes — extract display text from FieldCode attribute
                    NodeType::Field => {
                        let field_text = child
                            .attributes
                            .get_string(&AttributeKey::FieldCode)
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        if !field_text.is_empty() {
                            let glyphs = synthesize_glyphs(&field_text, DEFAULT_FONT_SIZE);
                            shaped_runs.push(ShapedRunInfo {
                                source_id: child_id,
                                font_id: None,
                                font_size: DEFAULT_FONT_SIZE,
                                color: s1_model::Color::new(0, 0, 0),
                                glyphs,
                                is_line_break: false,
                                metrics: None,
                                hyperlink_url: None,
                                text: field_text,
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
                                inline_image: None,
                            });
                        }
                    }
                    // Inline images/drawings within a paragraph
                    NodeType::Drawing | NodeType::Image => {
                        let img_node = child;
                        // Try ImageWidth first, fall back to ShapeWidth for VML drawings
                        let img_w = img_node
                            .attributes
                            .get(&AttributeKey::ImageWidth)
                            .and_then(|v| if let AttributeValue::Float(d) = v { Some(*d) } else { None })
                            .or_else(|| {
                                img_node.attributes.get(&AttributeKey::ShapeWidth)
                                    .and_then(|v| if let AttributeValue::Float(d) = v { Some(*d) } else { None })
                            })
                            .unwrap_or(100.0);
                        // Try ImageHeight first, fall back to ShapeHeight for VML drawings
                        let img_h = img_node
                            .attributes
                            .get(&AttributeKey::ImageHeight)
                            .and_then(|v| if let AttributeValue::Float(d) = v { Some(*d) } else { None })
                            .or_else(|| {
                                img_node.attributes.get(&AttributeKey::ShapeHeight)
                                    .and_then(|v| if let AttributeValue::Float(d) = v { Some(*d) } else { None })
                            })
                            .unwrap_or(100.0);

                        // Constrain to available width
                        let scale = if img_w > available_width { available_width / img_w } else { 1.0 };
                        let final_w = img_w * scale;
                        let final_h = img_h * scale;

                        // Get media ID and image data
                        let media_id_val = img_node
                            .attributes
                            .get(&AttributeKey::ImageMediaId)
                            .and_then(|v| if let AttributeValue::MediaId(mid) = v { Some(mid.0) } else { None });
                        let media_id_str = media_id_val
                            .map(|id| format!("{id}"))
                            .or_else(|| {
                                img_node.attributes.get_string(&AttributeKey::ImageMediaId).map(|s| s.to_string())
                            })
                            .unwrap_or_default();
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

                        // Create a synthetic shaped run with the image's width
                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_size: final_h, // use image height as "font size" for line height
                            color: s1_model::Color::new(0, 0, 0),
                            glyphs: vec![ShapedGlyph {
                                glyph_id: 0,
                                x_advance: final_w,
                                y_advance: 0.0,
                                x_offset: 0.0,
                                y_offset: 0.0,
                                cluster: 0,
                            }],
                            is_line_break: false,
                            metrics: None,
                            hyperlink_url: None,
                            text: String::new(),
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
                            inline_image: Some(InlineImage {
                                media_id: media_id_str,
                                width: final_w,
                                height: final_h,
                                image_data,
                                content_type,
                            }),
                        });
                    }
                    _ => {}
                }
            }
        }

        // Break into lines (greedy algorithm)
        let first_line_indent = sanitize_pt(para_style.indent_first_line);
        let lines =
            self.break_into_lines(&shaped_runs, available_width, first_line_indent, para_style);

        // Compute total paragraph height
        let total_height: f64 = lines.iter().map(|l| l.height).sum();

        // Check if paragraph has list info
        let (list_marker, list_level) = if let Some(para_node) = self.doc.node(para_id) {
            if let Some(s1_model::AttributeValue::ListInfo(li)) = para_node.attributes.get(&s1_model::AttributeKey::ListInfo) {
                let marker = match li.num_format {
                    s1_model::ListFormat::Bullet => "\u{2022}".to_string(),
                    s1_model::ListFormat::Decimal => format!("{}.", li.start.unwrap_or(1)),
                    s1_model::ListFormat::LowerAlpha => {
                        let c = (b'a' + (li.start.unwrap_or(1) as u8).saturating_sub(1).min(25)) as char;
                        format!("{}.", c)
                    }
                    s1_model::ListFormat::UpperAlpha => {
                        let c = (b'A' + (li.start.unwrap_or(1) as u8).saturating_sub(1).min(25)) as char;
                        format!("{}.", c)
                    }
                    s1_model::ListFormat::LowerRoman => format!("{}.", li.start.unwrap_or(1)),
                    s1_model::ListFormat::UpperRoman => format!("{}.", li.start.unwrap_or(1)),
                    _ => "\u{2022}".to_string(),
                };
                (Some(marker), li.level)
            } else {
                (None, 0)
            }
        } else {
            (None, 0)
        };

        Ok(LayoutBlock {
            source_id: para_id,
            bounds: Rect::new(x_start, y_pos, available_width, total_height),
            kind: LayoutBlockKind::Paragraph {
                lines,
                text_align: match para_style.alignment {
                    s1_model::Alignment::Left => None,
                    s1_model::Alignment::Center => Some("center".to_string()),
                    s1_model::Alignment::Right => Some("right".to_string()),
                    s1_model::Alignment::Justify => Some("justify".to_string()),
                    _ => None,
                },
                background_color: None,
                border: None,
                list_marker,
                list_level,
                space_before: para_style.space_before,
                space_after: para_style.space_after,
                indent_left: para_style.indent_left,
                indent_right: para_style.indent_right,
                indent_first_line: para_style.indent_first_line,
                line_height: line_spacing_to_css(&para_style.line_spacing),
            },
        })
    }

    /// Shape a run node — resolve font, shape text, return shaped info.
    fn shape_run(&self, run_id: NodeId) -> Result<ShapedRunInfo, LayoutError> {
        let run_style = resolve_run_style(self.doc, run_id);

        // Find font with extended fallback chain
        let font_id = self
            .font_db
            .find(&run_style.font_family, run_style.bold, run_style.italic)
            // Try the same font without bold/italic
            .or_else(|| self.font_db.find(&run_style.font_family, false, false))
            .or_else(|| self.font_db.find(DEFAULT_FONT_FAMILY, false, false))
            // Common serif fonts
            .or_else(|| self.font_db.find("Times New Roman", false, false))
            .or_else(|| self.font_db.find("Georgia", false, false))
            // Common sans-serif fonts
            .or_else(|| self.font_db.find("Helvetica", false, false))
            .or_else(|| self.font_db.find("Arial", false, false))
            .or_else(|| self.font_db.find("Verdana", false, false))
            .or_else(|| self.font_db.find("Roboto", false, false))
            .or_else(|| self.font_db.find("Noto Sans", false, false))
            .or_else(|| self.font_db.find("DejaVu Sans", false, false))
            .or_else(|| self.font_db.find("Liberation Sans", false, false));

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
                // L-15: Only apply 65% scaling if the run doesn't have an explicit font size
                let has_explicit_size = self.doc.node(run_id)
                    .map(|n| n.attributes.get(&AttributeKey::FontSize).is_some())
                    .unwrap_or(false);
                let font_size = if (run_style.superscript || run_style.subscript) && !has_explicit_size {
                    run_style.font_size * 0.65
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
                    parent
                        .attributes
                        .get_string(&AttributeKey::HyperlinkUrl)
                        .map(|s| s.to_string())
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
            text,
            bold: run_style.bold,
            italic: run_style.italic,
            underline: run_style.underline,
            strikethrough: run_style.strikethrough,
            superscript: run_style.superscript,
            subscript: run_style.subscript,
            highlight_color: run_style.highlight_color,
            character_spacing: run_style.character_spacing,
            revision_type: run_style.revision_type.clone(),
            revision_author: run_style.revision_author.clone(),
            inline_image: None,
        })
    }

    /// Shape a Run that contains LineBreak children — split into multiple shaped runs.
    fn shape_run_with_breaks(
        &self,
        run_id: NodeId,
        shaped_runs: &mut Vec<ShapedRunInfo>,
    ) -> Result<(), LayoutError> {
        let run_node = match self.doc.node(run_id) {
            Some(n) => n,
            None => return Ok(()),
        };
        let run_style = resolve_run_style(self.doc, run_id);
        let font_id = self
            .font_db
            .find(&run_style.font_family, run_style.bold, run_style.italic)
            // Try the same font without bold/italic
            .or_else(|| self.font_db.find(&run_style.font_family, false, false))
            .or_else(|| self.font_db.find(DEFAULT_FONT_FAMILY, false, false))
            // Common serif fonts
            .or_else(|| self.font_db.find("Times New Roman", false, false))
            .or_else(|| self.font_db.find("Georgia", false, false))
            // Common sans-serif fonts
            .or_else(|| self.font_db.find("Helvetica", false, false))
            .or_else(|| self.font_db.find("Arial", false, false))
            .or_else(|| self.font_db.find("Verdana", false, false))
            .or_else(|| self.font_db.find("Roboto", false, false))
            .or_else(|| self.font_db.find("Noto Sans", false, false))
            .or_else(|| self.font_db.find("DejaVu Sans", false, false))
            .or_else(|| self.font_db.find("Liberation Sans", false, false));

        let hyperlink_url = run_node.attributes.get_string(&AttributeKey::HyperlinkUrl)
            .map(|s| s.to_string())
            .or_else(|| {
                run_node.parent.and_then(|pid| {
                    self.doc.node(pid).and_then(|p| {
                        p.attributes.get_string(&AttributeKey::HyperlinkUrl).map(|s| s.to_string())
                    })
                })
            });

        // Iterate children, accumulate text segments between line breaks
        let mut segment_text = String::new();
        for &child_id in &run_node.children {
            if let Some(child) = self.doc.node(child_id) {
                match child.node_type {
                    NodeType::Text => {
                        if let Some(t) = &child.text_content {
                            segment_text.push_str(t);
                        }
                    }
                    NodeType::LineBreak => {
                        // Flush current text segment as a shaped run
                        if !segment_text.is_empty() {
                            let info = self.shape_text_segment(
                                run_id, &segment_text, &run_style, font_id, hyperlink_url.clone(),
                            )?;
                            shaped_runs.push(info);
                            segment_text.clear();
                        }
                        // Add line break marker
                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_size: run_style.font_size,
                            color: run_style.color,
                            glyphs: Vec::new(),
                            is_line_break: true,
                            metrics: None,
                            hyperlink_url: None,
                            text: String::new(),
                            bold: run_style.bold,
                            italic: run_style.italic,
                            underline: run_style.underline,
                            strikethrough: run_style.strikethrough,
                            superscript: run_style.superscript,
                            subscript: run_style.subscript,
                            highlight_color: run_style.highlight_color,
                            character_spacing: run_style.character_spacing,
                            revision_type: run_style.revision_type.clone(),
                            revision_author: run_style.revision_author.clone(),
                            inline_image: None,
                        });
                    }
                    NodeType::Tab => {
                        segment_text.push('\t');
                    }
                    _ => {}
                }
            }
        }
        // Flush remaining text
        if !segment_text.is_empty() {
            let info = self.shape_text_segment(
                run_id, &segment_text, &run_style, font_id, hyperlink_url,
            )?;
            shaped_runs.push(info);
        }
        Ok(())
    }

    /// Shape a text segment with given run styling.
    fn shape_text_segment(
        &self,
        source_id: NodeId,
        text: &str,
        run_style: &ResolvedRunStyle,
        font_id: Option<s1_text::FontId>,
        hyperlink_url: Option<String>,
    ) -> Result<ShapedRunInfo, LayoutError> {
        let (glyphs, metrics) = if let Some(fid) = font_id {
            if let Some(font) = self.font_db.load_font(fid) {
                // L-15: Only apply 65% scaling if the run doesn't have an explicit font size
                let has_explicit_size = self.doc.node(source_id)
                    .map(|n| n.attributes.get(&AttributeKey::FontSize).is_some())
                    .unwrap_or(false);
                let font_size = if (run_style.superscript || run_style.subscript) && !has_explicit_size {
                    run_style.font_size * 0.65
                } else {
                    run_style.font_size
                };
                let glyphs = s1_text::shape_text(
                    text, &font, font_size, &[], None, s1_text::Direction::Ltr,
                )?;
                let metrics = font.metrics(font_size);
                (glyphs, Some(metrics))
            } else {
                (synthesize_glyphs(text, run_style.font_size), None)
            }
        } else {
            (synthesize_glyphs(text, run_style.font_size), None)
        };

        Ok(ShapedRunInfo {
            source_id,
            font_id,
            font_size: run_style.font_size,
            color: run_style.color,
            glyphs,
            is_line_break: false,
            metrics,
            hyperlink_url,
            text: text.to_string(),
            bold: run_style.bold,
            italic: run_style.italic,
            underline: run_style.underline,
            strikethrough: run_style.strikethrough,
            superscript: run_style.superscript,
            subscript: run_style.subscript,
            highlight_color: run_style.highlight_color,
            character_spacing: run_style.character_spacing,
            revision_type: run_style.revision_type.clone(),
            revision_author: run_style.revision_author.clone(),
            inline_image: None,
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
            // For empty paragraphs, use Exact/AtLeast line spacing directly if set,
            // otherwise use DEFAULT_FONT_SIZE. This prevents empty paragraphs from
            // being forced to 13.8pt when a smaller height is styled.
            let base_size = match &para_style.line_spacing {
                LineSpacing::Exact(h) => *h,
                _ => DEFAULT_FONT_SIZE,
            };
            let line_height = compute_line_height(base_size, &para_style.line_spacing);
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
                    BreakItem::Box {
                        run_idx,
                        width,
                        height,
                        ..
                    } => {
                        let run_info = &runs[*run_idx];
                        let font_id = run_info.font_id.unwrap_or(FontId(fontdb::ID::dummy()));

                        line_runs.push(GlyphRun {
                            source_id: run_info.source_id,
                            font_id,
                            font_size: run_info.font_size,
                            color: run_info.color,
                            x_offset: current_x,
                            glyphs: run_info.glyphs.clone(),
                            width: *width,
                            hyperlink_url: run_info.hyperlink_url.clone(),
                            text: run_info.text.clone(),
                            bold: run_info.bold,
                            italic: run_info.italic,
                            underline: run_info.underline,
                            strikethrough: run_info.strikethrough,
                            superscript: run_info.superscript,
                            subscript: run_info.subscript,
                            highlight_color: run_info.highlight_color,
                            character_spacing: run_info.character_spacing,
                            revision_type: run_info.revision_type.clone(),
                            revision_author: run_info.revision_author.clone(),
                            inline_image: run_info.inline_image.clone(),
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
                if max_height > 0.0 {
                    max_height
                } else {
                    DEFAULT_FONT_SIZE
                },
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

            let glyph_advance: f64 = run_info.glyphs.iter().map(|g| g.x_advance).sum();
            // Add character spacing contribution: spacing applies between characters,
            // so (num_chars - 1) * spacing. Guard against empty text.
            let num_chars = run_info.text.chars().count();
            let spacing_contribution = if num_chars > 1 {
                (num_chars as f64 - 1.0) * run_info.character_spacing
            } else {
                0.0
            };
            let run_width = glyph_advance + spacing_contribution;
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

    /// Layout all table rows independently, returning them as a flat list.
    ///
    /// Each row is laid out with position-independent y coordinates (starting from 0).
    /// The caller is responsible for assigning final y positions and splitting
    /// across pages. Column widths are computed from the first row's cell count.
    fn layout_table_rows(
        &self,
        table_id: NodeId,
        content_rect: Rect,
    ) -> Result<Vec<LayoutTableRow>, LayoutError> {
        let table = match self.doc.node(table_id) {
            Some(n) => n,
            None => return Ok(Vec::new()),
        };

        // L-22: Limit table dimensions to prevent OOM on malicious documents
        const MAX_TABLE_ROWS: usize = 10_000;
        const MAX_TABLE_COLS: usize = 1_000;
        let row_count = table.children.len().min(MAX_TABLE_ROWS);

        // Count columns from first row
        let num_cols = table
            .children
            .first()
            .and_then(|&row_id| self.doc.node(row_id))
            .map(|row| row.children.len().min(MAX_TABLE_COLS))
            .unwrap_or(1)
            .max(1);

        let col_width = content_rect.width / num_cols as f64;
        let mut rows: Vec<LayoutTableRow> = Vec::new();
        let mut cumulative_y = 0.0;

        for &row_id in table.children.iter().take(row_count) {
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

                        // Layout cell content
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
                                    cell_y += block.bounds.height + sanitize_pt(ps.space_after);
                                    cell_blocks.push(block);
                                }
                            }
                        }

                        cell_y += 2.0; // Bottom padding
                        if cell_y > max_cell_height {
                            max_cell_height = cell_y;
                        }

                        // Extract cell background color
                        let background_color = cell_node.attributes
                            .get(&AttributeKey::CellBackground)
                            .and_then(|v| if let AttributeValue::Color(c) = v { Some(*c) } else { None });

                        // Extract cell borders
                        let (border_top, border_bottom, border_left, border_right) =
                            if let Some(AttributeValue::Borders(borders)) =
                                cell_node.attributes.get(&AttributeKey::CellBorders)
                            {
                                (
                                    borders.top.as_ref().map(format_border_css),
                                    borders.bottom.as_ref().map(format_border_css),
                                    borders.left.as_ref().map(format_border_css),
                                    borders.right.as_ref().map(format_border_css),
                                )
                            } else {
                                (None, None, None, None)
                            };

                        cells.push(LayoutTableCell {
                            bounds: cell_rect,
                            blocks: cell_blocks,
                            background_color,
                            border_top,
                            border_bottom,
                            border_left,
                            border_right,
                        });
                    }
                }

                // Set actual cell heights
                for cell in &mut cells {
                    cell.bounds.height = max_cell_height;
                }

                let is_header = row_node
                    .attributes
                    .get(&AttributeKey::TableHeaderRow)
                    .map(|v| matches!(v, AttributeValue::Bool(true)))
                    .unwrap_or(false);

                rows.push(LayoutTableRow {
                    bounds: Rect::new(0.0, cumulative_y, content_rect.width, max_cell_height),
                    cells,
                    is_header_row: is_header,
                });
                cumulative_y += max_cell_height;
            }
        }

        Ok(rows)
    }

    /// Layout an image node.
    ///
    /// Handles both `NodeType::Image` (with `ImageWidth`/`ImageHeight`) and
    /// `NodeType::Drawing` (VML shapes with `ShapeWidth`/`ShapeHeight` fallback).
    fn layout_image(
        &self,
        image_id: NodeId,
        content_rect: Rect,
        y_pos: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let node = self.doc.node(image_id);
        let (width, height) = node
            .map(|n| {
                // Try ImageWidth first, fall back to ShapeWidth for VML drawings
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
                    .or_else(|| {
                        n.attributes.get(&AttributeKey::ShapeWidth).and_then(|v| {
                            if let AttributeValue::Float(d) = v {
                                Some(*d)
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(100.0);
                // Try ImageHeight first, fall back to ShapeHeight for VML drawings
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
                    .or_else(|| {
                        n.attributes.get(&AttributeKey::ShapeHeight).and_then(|v| {
                            if let AttributeValue::Float(d) = v {
                                Some(*d)
                            } else {
                                None
                            }
                        })
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
            if let Some(AttributeValue::MediaId(mid)) =
                n.attributes.get(&AttributeKey::ImageMediaId)
            {
                Some(mid.0)
            } else {
                None
            }
        });

        let media_id_str = media_id_val
            .map(|id| format!("{id}"))
            .or_else(|| {
                node.and_then(|n| {
                    n.attributes
                        .get_string(&AttributeKey::ImageMediaId)
                        .map(|s| s.to_string())
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

    /// Layout header/footer content for a page using a specific section's
    /// header/footer references and distances.
    fn layout_header_footer_for_section(
        &self,
        page: &mut LayoutPage,
        total_pages: usize,
        section_idx: usize,
    ) -> Result<(), LayoutError> {
        let sections = self.doc.sections();
        let section = match sections.get(section_idx).or_else(|| sections.last()) {
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
            let header_block = self.layout_hf_node_for_section(
                hf_ref.node_id,
                page,
                page_num,
                total_pages,
                true,
                section_idx,
            )?;
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
            let footer_block = self.layout_hf_node_for_section(
                hf_ref.node_id,
                page,
                page_num,
                total_pages,
                false,
                section_idx,
            )?;
            page.footer = Some(footer_block);
        }

        Ok(())
    }

    /// Layout a header or footer node using a specific section's distances.
    fn layout_hf_node_for_section(
        &self,
        node_id: NodeId,
        page: &LayoutPage,
        page_num: usize,
        total_pages: usize,
        is_header: bool,
        section_idx: usize,
    ) -> Result<LayoutBlock, LayoutError> {
        let sections = self.doc.sections();
        let section = sections.get(section_idx).or_else(|| sections.last());
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
                    kind: LayoutBlockKind::Paragraph { lines: Vec::new(), text_align: None, background_color: None, border: None, list_marker: None, list_level: 0, space_before: 0.0, space_after: 0.0, indent_left: 0.0, indent_right: 0.0, indent_first_line: 0.0, line_height: None },
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
                    if let LayoutBlockKind::Paragraph { lines, .. } = b.kind {
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
                kind: LayoutBlockKind::Paragraph { lines, text_align: None, background_color: None, border: None, list_marker: None, list_level: 0, space_before: 0.0, space_after: 0.0, indent_left: 0.0, indent_right: 0.0, indent_first_line: 0.0, line_height: None },
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

                        // Update existing glyph run with matching source_id,
                        // or append a synthesized run to the last line if not found
                        if let LayoutBlockKind::Paragraph { lines, .. } = &mut block.kind {
                            let font_size = DEFAULT_FONT_SIZE;
                            let glyphs = synthesize_glyphs(&text, font_size);
                            let width: f64 = glyphs.iter().map(|g| g.x_advance).sum();

                            // First try to update an existing run in-place
                            let mut found = false;
                            for line in lines.iter_mut() {
                                for run in line.runs.iter_mut() {
                                    if run.source_id == child_id {
                                        run.text.clone_from(&text);
                                        run.glyphs = glyphs.clone();
                                        run.width = width;
                                        found = true;
                                        break;
                                    }
                                }
                                if found { break; }
                            }

                            // Only append if no existing run was found
                            if !found {
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
                                    text: text.clone(),
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
                                    inline_image: None,
                                });
                              }
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
        page_section_indices: &[usize],
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

        // Track which pages were already empty (intentionally blank for section breaks)
        let initially_empty: Vec<bool> = pages.iter().map(|p| p.blocks.is_empty()).collect();

        let mut i = 0;
        while i + 1 < pages.len() {
            let needs_fix = {
                let current_page = &pages[i];
                let next_page = &pages[i + 1];

                // Check last block on current page — is it a paragraph with too few lines?
                let orphan_problem = if let Some(last_block) = current_page.blocks.last() {
                    if let LayoutBlockKind::Paragraph { lines, .. } = &last_block.kind {
                        lines.len() > 1 && lines.len() < min_orphan + min_widow
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Check first block on next page — is it a continuation with too few lines?
                let widow_problem = if let Some(first_block) = next_page.blocks.first() {
                    if let LayoutBlockKind::Paragraph { lines, .. } = &first_block.kind {
                        !lines.is_empty() && lines.len() < min_widow
                    } else {
                        false
                    }
                } else {
                    false
                };

                orphan_problem || widow_problem
            };

            // L-13: Don't move blocks across section boundaries
            let same_section = i < page_section_indices.len()
                && i + 1 < page_section_indices.len()
                && page_section_indices[i] == page_section_indices[i + 1];

            if needs_fix && same_section {
                // Move the last block from current page to the start of next page.
                // Use the next page's content area for positioning.
                let current_page = &mut pages[i];
                if current_page.blocks.len() > 1 {
                    let Some(block) = current_page.blocks.pop() else {
                        continue;
                    };
                    let next_page = &mut pages[i + 1];
                    let content_y = next_page.content_area.y;
                    let mut moved_block = block;
                    moved_block.bounds.y = content_y;

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

        // Remove only pages that became empty due to widow/orphan block moves.
        // Preserve pages that were intentionally blank (e.g. OddPage/EvenPage section breaks).
        {
            let mut keep = Vec::with_capacity(pages.len());
            for (idx, page) in pages.iter().enumerate() {
                let was_empty = initially_empty.get(idx).copied().unwrap_or(false);
                // Keep page if: it has blocks, OR it was already empty (intentional blank), OR it's the first page
                keep.push(!page.blocks.is_empty() || was_empty || idx == 0);
            }
            let mut k = 0;
            pages.retain(|_| {
                let r = keep[k];
                k += 1;
                r
            });
        }

        // Re-index pages
        for (idx, page) in pages.iter_mut().enumerate() {
            page.index = idx;
        }

        Ok(())
    }

    /// Collect bookmarks from laid-out pages by scanning for BookmarkStart nodes.
    ///
    /// Recursively scans all blocks including table rows/cells and
    /// header/footer blocks so that bookmarks inside those containers are
    /// not missed.
    fn collect_bookmarks(&self, pages: &[LayoutPage]) -> Vec<LayoutBookmark> {
        let mut bookmarks = Vec::new();

        for page in pages {
            // Scan content blocks (paragraphs, tables, images)
            for block in &page.blocks {
                self.collect_bookmarks_from_block(block, page.index, &mut bookmarks);
            }
            // Scan header block
            if let Some(header) = &page.header {
                self.collect_bookmarks_from_block(header, page.index, &mut bookmarks);
            }
            // Scan footer block
            if let Some(footer) = &page.footer {
                self.collect_bookmarks_from_block(footer, page.index, &mut bookmarks);
            }
        }

        bookmarks
    }

    /// Recursively collect bookmarks from a single layout block.
    ///
    /// For paragraph blocks, scans the source node's children for
    /// BookmarkStart nodes. For table blocks, descends into rows, cells,
    /// and their nested content blocks.
    fn collect_bookmarks_from_block(
        &self,
        block: &LayoutBlock,
        page_index: usize,
        bookmarks: &mut Vec<LayoutBookmark>,
    ) {
        match &block.kind {
            LayoutBlockKind::Paragraph { .. } | LayoutBlockKind::Image { .. } => {
                // Check if the source node has BookmarkStart children
                if let Some(node) = self.doc.node(block.source_id) {
                    for &child_id in &node.children {
                        if let Some(child) = self.doc.node(child_id) {
                            if child.node_type == NodeType::BookmarkStart {
                                if let Some(name) =
                                    child.attributes.get_string(&AttributeKey::BookmarkName)
                                {
                                    bookmarks.push(LayoutBookmark {
                                        name: name.to_string(),
                                        page_index,
                                        y_position: block.bounds.y,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            LayoutBlockKind::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        for cell_block in &cell.blocks {
                            self.collect_bookmarks_from_block(
                                cell_block,
                                page_index,
                                bookmarks,
                            );
                        }
                    }
                }
            }
        }
    }

    fn make_page(
        &self,
        index: usize,
        page_layout: &PageLayout,
        blocks: Vec<LayoutBlock>,
        section_index: usize,
    ) -> LayoutPage {
        LayoutPage {
            index,
            width: page_layout.width,
            height: page_layout.height,
            content_area: page_layout.content_rect(),
            blocks,
            header: None,
            footer: None,
            section_index,
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
    /// Original text content.
    text: String,
    /// Bold formatting.
    bold: bool,
    /// Italic formatting.
    italic: bool,
    /// Underline formatting.
    underline: bool,
    /// Strikethrough formatting.
    strikethrough: bool,
    /// Superscript formatting.
    superscript: bool,
    /// Subscript formatting.
    subscript: bool,
    /// Highlight/background color.
    highlight_color: Option<s1_model::Color>,
    /// Character spacing in points.
    character_spacing: f64,
    /// Revision type for track changes.
    revision_type: Option<String>,
    /// Revision author for track changes.
    revision_author: Option<String>,
    /// Inline image data, if this run represents an inline image.
    inline_image: Option<InlineImage>,
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
                fn finish(&self) -> u64 {
                    self.0
                }
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

/// Format a border side as a CSS border value.
fn format_border_css(border: &s1_model::BorderSide) -> String {
    let style_str = match border.style {
        s1_model::BorderStyle::None => "none",
        s1_model::BorderStyle::Single => "solid",
        s1_model::BorderStyle::Double => "double",
        s1_model::BorderStyle::Dotted => "dotted",
        s1_model::BorderStyle::Dashed => "dashed",
        s1_model::BorderStyle::Thick => "solid",
        _ => "solid",
    };
    let width = if border.width > 0.0 { border.width } else { 1.0 };
    format!(
        "{:.1}pt {} #{:02x}{:02x}{:02x}",
        width, style_str, border.color.r, border.color.g, border.color.b
    )
}

/// Synthesize glyphs when no font is available (fallback for headless testing).
///
/// Uses character-class-based width estimation for more accurate line breaking
/// than a single average width for all characters.
fn synthesize_glyphs(text: &str, font_size: f64) -> Vec<ShapedGlyph> {
    text.char_indices()
        .map(|(i, ch)| {
            // Estimate width by character class for more accurate line breaking
            let width_factor = match ch {
                'i' | 'j' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | '"' | '`' => 0.3,
                'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '{' | '}' => 0.4,
                'm' | 'w' | 'M' | 'W' | '@' | '%' => 0.8,
                'A'..='Z' => 0.65,
                ' ' => 0.3,
                _ => 0.5,
            };
            ShapedGlyph {
                glyph_id: 0,
                x_advance: font_size * width_factor,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
                cluster: i as u32,
            }
        })
        .collect()
}

/// Sanitize a floating-point value — replace NaN/infinity with 0.0.
///
/// Prevents garbage layout when style values are malformed or produced
/// by division-by-zero or other floating-point edge cases.
fn sanitize_pt(val: f64) -> f64 {
    if val.is_finite() {
        val
    } else {
        0.0
    }
}

/// Compute line height from the tallest run and line spacing.
fn compute_line_height(max_run_height: f64, line_spacing: &LineSpacing) -> f64 {
    let h = sanitize_pt(max_run_height);
    let result = match line_spacing {
        LineSpacing::Single => h,
        LineSpacing::OnePointFive => h * 1.5,
        LineSpacing::Double => h * 2.0,
        LineSpacing::Multiple(m) => h * sanitize_pt(*m),
        LineSpacing::AtLeast(min) => h.max(sanitize_pt(*min)),
        LineSpacing::Exact(exact) => sanitize_pt(*exact),
        _ => h,
    };
    sanitize_pt(result)
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
        let is_feasible_break =
            matches!(item, BreakItem::Glue { .. } | BreakItem::ForcedBreak { .. });

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
        // L-21: Cap active node count to prevent unbounded growth on very long paragraphs.
        // Keep only the best 100 candidates by demerits.
        if active.len() > 100 {
            active.sort_by(|&a, &b| {
                nodes[a].demerits.partial_cmp(&nodes[b].demerits).unwrap_or(std::cmp::Ordering::Equal)
            });
            active.truncate(100);
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
fn greedy_breaks(items: &[BreakItem], available_width: f64, first_line_indent: f64) -> Vec<usize> {
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

/// Convert a `LineSpacing` value to a CSS-compatible line-height multiplier.
///
/// Returns `None` for the default single spacing (browser default is close enough).
fn line_spacing_to_css(spacing: &LineSpacing) -> Option<f64> {
    match spacing {
        LineSpacing::Single => None, // browser default ~1.2 is close; omit for cleaner output
        LineSpacing::OnePointFive => Some(1.5),
        LineSpacing::Double => Some(2.0),
        LineSpacing::Multiple(m) => {
            if (*m - 1.0).abs() < 0.001 {
                None // essentially single
            } else {
                Some(*m)
            }
        }
        LineSpacing::Exact(pts) => {
            // For exact spacing, emit pt-based value; we store as negative to
            // distinguish from multipliers, but CSS line-height in pt is fine.
            // We'll emit this as a raw points value — html.rs will handle the unit.
            Some(-(*pts)) // negative signals "pt" to the HTML emitter
        }
        LineSpacing::AtLeast(pts) => {
            // AtLeast is a minimum; CSS doesn't have a direct equivalent, so
            // we approximate by using the value as a minimum line-height.
            Some(-(*pts)) // negative signals "pt" to the HTML emitter
        }
        _ => None,
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
            LayoutBlockKind::Paragraph { lines, .. } => {
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
            LayoutBlockKind::Paragraph { lines, .. } => {
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
            LayoutBlockKind::Table { rows, .. } => {
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
        // 'A' is uppercase => 0.65 factor => 12 * 0.65 = 7.8
        assert!((glyphs[0].x_advance - 7.8).abs() < 0.01);
        // 'B' and 'C' are uppercase => 0.65 factor
        assert!((glyphs[1].x_advance - 7.8).abs() < 0.01);

        // Test narrow chars
        let narrow = synthesize_glyphs("i.", 12.0);
        assert!((narrow[0].x_advance - 3.6).abs() < 0.01); // 12 * 0.3

        // Test wide chars
        let wide = synthesize_glyphs("MW", 12.0);
        assert!((wide[0].x_advance - 9.6).abs() < 0.01); // 12 * 0.8
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
        doc.insert_node(r1, 0, Node::text(t1, "Page 1")).unwrap();

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
        doc.insert_node(r2, 0, Node::text(t2, "Page 2")).unwrap();

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
        assert!(
            header.bounds.y < 72.0,
            "header should be in top margin area"
        );

        // Footer should be near the bottom
        let footer = page.footer.as_ref().unwrap();
        assert!(
            footer.bounds.y > 700.0,
            "footer should be in bottom margin area"
        );
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
            doc.insert_node(rid, 0, Node::text(tid, *text)).unwrap();
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
            assert!(
                page.footer.is_some(),
                "page {} should have footer",
                page.index
            );
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
        assert_eq!(result1.pages[0].blocks.len(), result2.pages[0].blocks.len());
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

        // First layout
        let mut engine =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result1 = engine.layout().unwrap();

        // Second layout should produce identical results
        let mut engine2 =
            LayoutEngine::new_with_cache(&doc, &font_db, LayoutConfig::default(), &mut cache);
        let result2 = engine2.layout().unwrap();
        assert_eq!(result1.pages.len(), result2.pages.len());
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
            doc.insert_node(run_id, 0, Node::text(text_id, "Performance test paragraph"))
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
        assert!(
            cache.len() >= 50,
            "Cache should have entries for all paragraphs"
        );
    }

    // --- C.1: Multi-Section Layout Tests ---

    /// Helper: create a document with two sections having different page sizes.
    fn make_two_section_doc(
        s0_width: f64,
        s0_height: f64,
        s1_width: f64,
        s1_height: f64,
    ) -> DocumentModel {
        use s1_model::SectionProperties;

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Paragraph 1: belongs to section 0 (marks end of section 0)
        let p1 = doc.next_id();
        let mut p1_node = Node::new(p1, NodeType::Paragraph);
        p1_node
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, p1_node).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Section 1 content"))
            .unwrap();

        // Paragraph 2: belongs to section 1 (final section)
        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Section 2 content"))
            .unwrap();

        // Section 0 properties
        let mut sp0 = SectionProperties::default();
        sp0.page_width = s0_width;
        sp0.page_height = s0_height;
        sp0.break_type = Some(SectionBreakType::NextPage);
        doc.sections_mut().push(sp0);

        // Section 1 properties (final section)
        let mut sp1 = SectionProperties::default();
        sp1.page_width = s1_width;
        sp1.page_height = s1_height;
        doc.sections_mut().push(sp1);

        doc
    }

    #[test]
    fn layout_multi_section_different_page_sizes() {
        let doc = make_two_section_doc(612.0, 792.0, 841.89, 595.28);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(
            result.pages.len() >= 2,
            "expected at least 2 pages, got {}",
            result.pages.len()
        );

        // First page should use section 0 dimensions (Letter)
        let page0 = &result.pages[0];
        assert!(
            (page0.width - 612.0).abs() < 0.01,
            "page 0 width should be 612.0, got {}",
            page0.width
        );

        // Second page should use section 1 dimensions (A4 landscape)
        let page1 = &result.pages[1];
        assert!(
            (page1.width - 841.89).abs() < 0.01,
            "page 1 width should be 841.89, got {}",
            page1.width
        );
    }

    #[test]
    fn layout_multi_section_different_margins() {
        use s1_model::SectionProperties;

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let p1 = doc.next_id();
        let mut p1_node = Node::new(p1, NodeType::Paragraph);
        p1_node
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, p1_node).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Narrow margins"))
            .unwrap();

        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Wide margins"))
            .unwrap();

        // Section 0: narrow margins
        let mut sp0 = SectionProperties::default();
        sp0.margin_left = 36.0;
        sp0.margin_right = 36.0;
        sp0.break_type = Some(SectionBreakType::NextPage);
        doc.sections_mut().push(sp0);

        // Section 1: wide margins
        let mut sp1 = SectionProperties::default();
        sp1.margin_left = 144.0;
        sp1.margin_right = 144.0;
        doc.sections_mut().push(sp1);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(result.pages.len() >= 2);

        let ca0 = result.pages[0].content_area;
        let ca1 = result.pages[1].content_area;
        assert!(
            ca0.width > ca1.width,
            "section 0 content width ({}) should exceed section 1 ({})",
            ca0.width,
            ca1.width
        );
    }

    #[test]
    fn layout_section_break_next_page() {
        let doc = make_two_section_doc(612.0, 792.0, 612.0, 792.0);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(
            result.pages.len() >= 2,
            "NextPage break should create at least 2 pages, got {}",
            result.pages.len()
        );
    }

    #[test]
    fn layout_section_break_continuous() {
        use s1_model::SectionProperties;

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let p1 = doc.next_id();
        let mut p1_node = Node::new(p1, NodeType::Paragraph);
        p1_node
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, p1_node).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Before break"))
            .unwrap();

        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "After break"))
            .unwrap();

        let mut sp0 = SectionProperties::default();
        sp0.break_type = Some(SectionBreakType::NextPage);
        doc.sections_mut().push(sp0);

        let mut sp1 = SectionProperties::default();
        sp1.break_type = Some(SectionBreakType::Continuous);
        doc.sections_mut().push(sp1);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        // Both paragraphs should be in the layout
        let total_blocks: usize = result.pages.iter().map(|p| p.blocks.len()).sum();
        assert_eq!(total_blocks, 2, "both paragraphs should be laid out");
        // Continuous break should not add extra blank pages
        assert!(
            result.pages.len() <= 2,
            "continuous break should not add extra pages"
        );
    }

    #[test]
    fn layout_section_even_page() {
        use s1_model::SectionProperties;

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let p1 = doc.next_id();
        let mut p1_node = Node::new(p1, NodeType::Paragraph);
        p1_node
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, p1_node).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "First section"))
            .unwrap();

        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Even page section"))
            .unwrap();

        let mut sp0 = SectionProperties::default();
        sp0.break_type = Some(SectionBreakType::NextPage);
        doc.sections_mut().push(sp0);

        let mut sp1 = SectionProperties::default();
        sp1.break_type = Some(SectionBreakType::EvenPage);
        doc.sections_mut().push(sp1);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        // Section 0 on page 1 (odd). EvenPage break needs a blank page.
        assert!(
            result.pages.len() >= 2,
            "even page break should create at least 2 pages"
        );

        // Find the page with section 1's content
        let section1_page = result.pages.iter().find(|p| {
            p.blocks.iter().any(|b| {
                if let LayoutBlockKind::Paragraph { lines, .. } = &b.kind {
                    lines.iter().any(|l| !l.runs.is_empty())
                } else {
                    false
                }
            }) && p.index > 0
        });
        if let Some(page) = section1_page {
            let page_number = page.index + 1;
            assert_eq!(
                page_number % 2,
                0,
                "section 1 should be on even page, got page {}",
                page_number
            );
        }
    }

    #[test]
    fn layout_section_odd_page() {
        use s1_model::SectionProperties;

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let p1 = doc.next_id();
        let mut p1_node = Node::new(p1, NodeType::Paragraph);
        p1_node
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, p1_node).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "First section"))
            .unwrap();

        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Odd page section"))
            .unwrap();

        let mut sp0 = SectionProperties::default();
        sp0.break_type = Some(SectionBreakType::NextPage);
        doc.sections_mut().push(sp0);

        let mut sp1 = SectionProperties::default();
        sp1.break_type = Some(SectionBreakType::OddPage);
        doc.sections_mut().push(sp1);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        // Section 0 on page 1 (odd). OddPage break needs blank page 2 then page 3.
        assert!(
            result.pages.len() >= 3,
            "odd page break should create at least 3 pages, got {}",
            result.pages.len()
        );

        // Last page with content should be on an odd page number
        let last_content_page = result.pages.iter().rev().find(|p| !p.blocks.is_empty());
        if let Some(page) = last_content_page {
            let page_number = page.index + 1;
            assert_eq!(
                page_number % 2,
                1,
                "section 1 should be on odd page, got page {}",
                page_number
            );
        }
    }

    #[test]
    fn layout_section_different_headers() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, SectionProperties};

        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let p1 = doc.next_id();
        let mut p1_node = Node::new(p1, NodeType::Paragraph);
        p1_node
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, p1_node).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Section 0 body"))
            .unwrap();

        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Section 1 body"))
            .unwrap();

        // Header for section 0
        let hdr0_id = doc.next_id();
        doc.insert_node(root, 1, Node::new(hdr0_id, NodeType::Header))
            .unwrap();
        let hp0 = doc.next_id();
        doc.insert_node(hdr0_id, 0, Node::new(hp0, NodeType::Paragraph))
            .unwrap();
        let hr0 = doc.next_id();
        doc.insert_node(hp0, 0, Node::new(hr0, NodeType::Run))
            .unwrap();
        let ht0 = doc.next_id();
        doc.insert_node(hr0, 0, Node::text(ht0, "Header 0"))
            .unwrap();

        // Header for section 1
        let hdr1_id = doc.next_id();
        doc.insert_node(root, 2, Node::new(hdr1_id, NodeType::Header))
            .unwrap();
        let hp1 = doc.next_id();
        doc.insert_node(hdr1_id, 0, Node::new(hp1, NodeType::Paragraph))
            .unwrap();
        let hr1 = doc.next_id();
        doc.insert_node(hp1, 0, Node::new(hr1, NodeType::Run))
            .unwrap();
        let ht1 = doc.next_id();
        doc.insert_node(hr1, 0, Node::text(ht1, "Header 1"))
            .unwrap();

        let mut sp0 = SectionProperties::default();
        sp0.break_type = Some(SectionBreakType::NextPage);
        sp0.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: hdr0_id,
        });
        doc.sections_mut().push(sp0);

        let mut sp1 = SectionProperties::default();
        sp1.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: hdr1_id,
        });
        doc.sections_mut().push(sp1);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(result.pages.len() >= 2);

        let page0 = &result.pages[0];
        assert!(page0.header.is_some(), "page 0 should have a header");
        assert_eq!(
            page0.header.as_ref().unwrap().source_id, hdr0_id,
            "page 0 header should be from section 0"
        );

        let page1 = &result.pages[1];
        assert!(page1.header.is_some(), "page 1 should have a header");
        assert_eq!(
            page1.header.as_ref().unwrap().source_id, hdr1_id,
            "page 1 header should be from section 1"
        );
    }

    #[test]
    fn layout_section_landscape() {
        // Section 0: portrait (612x792), Section 1: landscape (792x612)
        let doc = make_two_section_doc(612.0, 792.0, 792.0, 612.0);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(result.pages.len() >= 2);

        assert!(
            result.pages[0].width < result.pages[0].height,
            "page 0 should be portrait"
        );

        assert!(
            result.pages[1].width > result.pages[1].height,
            "page 1 should be landscape"
        );
    }

    // --- Milestone C.2: Tables Across Page Breaks ---

    /// Helper to build a document with a table of `num_rows` rows and `num_cols` columns.
    /// If `header_row_count` > 0, the first N rows are marked as header rows.
    fn make_table_doc(num_rows: usize, num_cols: usize, header_row_count: usize) -> DocumentModel {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        for row_idx in 0..num_rows {
            let row_id = doc.next_id();
            let mut row_node = Node::new(row_id, NodeType::TableRow);
            if row_idx < header_row_count {
                row_node.attributes.set(
                    AttributeKey::TableHeaderRow,
                    AttributeValue::Bool(true),
                );
            }
            doc.insert_node(table_id, row_idx, row_node).unwrap();

            for col_idx in 0..num_cols {
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
                    Node::text(text_id, &format!("R{}C{}", row_idx, col_idx)),
                )
                .unwrap();
            }
        }

        doc
    }

    #[test]
    fn test_table_split_across_pages() {
        // With letter page (648pt content height) and ~20pt row height,
        // 50 rows will exceed one page (50 * 20 = 1000 > 648).
        let doc = make_table_doc(50, 3, 0);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(
            result.pages.len() >= 2,
            "50-row table should span at least 2 pages, got {}",
            result.pages.len()
        );

        // First page: table with is_continuation = false
        let first_table = &result.pages[0].blocks[0];
        match &first_table.kind {
            LayoutBlockKind::Table {
                rows,
                is_continuation,
            } => {
                assert!(!is_continuation, "first table chunk should not be a continuation");
                assert!(!rows.is_empty(), "first page should have table rows");
            }
            _ => panic!("expected a table block on first page"),
        }

        // Second page: table with is_continuation = true
        let second_table = &result.pages[1].blocks[0];
        match &second_table.kind {
            LayoutBlockKind::Table {
                rows,
                is_continuation,
            } => {
                assert!(*is_continuation, "second table chunk should be a continuation");
                assert!(!rows.is_empty(), "second page should have table rows");
            }
            _ => panic!("expected a table block on second page"),
        }

        // Verify total row count across all pages equals original
        let total_rows: usize = result
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .map(|b| match &b.kind {
                LayoutBlockKind::Table { rows, .. } => rows.len(),
                _ => 0,
            })
            .sum();
        assert_eq!(total_rows, 50, "all 50 rows should be present across pages");
    }

    #[test]
    fn test_table_header_row_repeat() {
        // 50 rows, first row is header — header should appear on continuation pages
        let doc = make_table_doc(50, 2, 1);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(
            result.pages.len() >= 2,
            "table should span multiple pages"
        );

        // Second page should start with a header row
        let second_table = &result.pages[1].blocks[0];
        match &second_table.kind {
            LayoutBlockKind::Table {
                rows,
                is_continuation,
            } => {
                assert!(*is_continuation, "should be continuation");
                assert!(
                    rows[0].is_header_row,
                    "first row on continuation page should be a header row"
                );
            }
            _ => panic!("expected table on second page"),
        }

        // Total rows should be 50 + (number_of_continuation_pages * 1 header row)
        let continuation_pages = result
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .filter(|b| matches!(&b.kind, LayoutBlockKind::Table { is_continuation: true, .. }))
            .count();
        let total_rows: usize = result
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .map(|b| match &b.kind {
                LayoutBlockKind::Table { rows, .. } => rows.len(),
                _ => 0,
            })
            .sum();
        assert_eq!(
            total_rows,
            50 + continuation_pages,
            "total rows = original + repeated headers on continuation pages"
        );
    }

    #[test]
    fn test_table_single_row_fits() {
        // Small table that fits on one page — no splitting should occur
        let doc = make_table_doc(3, 2, 0);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert_eq!(result.pages.len(), 1, "small table should fit on one page");
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Table {
                rows,
                is_continuation,
            } => {
                assert_eq!(rows.len(), 3);
                assert!(!is_continuation, "should not be a continuation");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn test_table_row_too_tall() {
        // Create a table with a single row containing very long text
        // that makes the row taller than the page. Verify it doesn't infinite loop
        // and is placed on the page anyway.
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(table_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
            .unwrap();

        // Add many paragraphs to make the cell very tall
        for i in 0..200 {
            let para_id = doc.next_id();
            doc.insert_node(cell_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(
                run_id,
                0,
                Node::text(text_id, "Very long content that makes this row very tall"),
            )
            .unwrap();
        }

        let font_db = FontDatabase::new();
        let config = LayoutConfig {
            default_page_layout: PageLayout::letter(),
            ..Default::default()
        };
        let mut engine = LayoutEngine::new(&doc, &font_db, config);

        // This should complete without infinite looping
        let result = engine.layout().unwrap();
        assert!(!result.pages.is_empty(), "should produce at least one page");

        // The oversized row should be placed on the page
        let has_table = result.pages[0].blocks.iter().any(|b| {
            matches!(&b.kind, LayoutBlockKind::Table { rows, .. } if !rows.is_empty())
        });
        assert!(has_table, "oversized row should be placed on the page");
    }

    #[test]
    fn test_table_multiple_page_splits() {
        // Create a table with enough rows to span 3+ pages.
        // 648pt content height / 20pt per row ≈ 32 rows per page.
        // 100 rows should span at least 3 pages.
        let doc = make_table_doc(100, 2, 0);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(
            result.pages.len() >= 3,
            "100-row table should span at least 3 pages, got {}",
            result.pages.len()
        );

        // First page: not a continuation
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Table {
                is_continuation, ..
            } => {
                assert!(!is_continuation, "first chunk should not be continuation");
            }
            _ => panic!("expected table on page 0"),
        }

        // All subsequent pages: continuation = true
        for page_idx in 1..result.pages.len() {
            match &result.pages[page_idx].blocks[0].kind {
                LayoutBlockKind::Table {
                    is_continuation, ..
                } => {
                    assert!(
                        *is_continuation,
                        "page {} table should be continuation",
                        page_idx
                    );
                }
                _ => panic!("expected table on page {}", page_idx),
            }
        }

        // Total row count should equal original
        let total_rows: usize = result
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .map(|b| match &b.kind {
                LayoutBlockKind::Table { rows, .. } => rows.len(),
                _ => 0,
            })
            .sum();
        assert_eq!(
            total_rows, 100,
            "all 100 rows should be present across pages"
        );
    }

    #[test]
    fn test_table_split_preserves_columns() {
        // Verify that split tables maintain column widths.
        let doc = make_table_doc(50, 4, 0);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(result.pages.len() >= 2, "should span multiple pages");

        // Collect all table blocks
        let table_blocks: Vec<&LayoutBlock> = result
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .filter(|b| matches!(&b.kind, LayoutBlockKind::Table { .. }))
            .collect();

        assert!(
            table_blocks.len() >= 2,
            "should have at least 2 table chunks"
        );

        // All table chunks should have the same width
        let first_width = table_blocks[0].bounds.width;
        for (i, block) in table_blocks.iter().enumerate() {
            assert!(
                (block.bounds.width - first_width).abs() < 0.01,
                "table chunk {} has different width: {} vs {}",
                i,
                block.bounds.width,
                first_width
            );
        }

        // All rows across all chunks should have consistent cell count (4 columns)
        for (block_idx, block) in table_blocks.iter().enumerate() {
            match &block.kind {
                LayoutBlockKind::Table { rows, .. } => {
                    for (row_idx, row) in rows.iter().enumerate() {
                        assert_eq!(
                            row.cells.len(),
                            4,
                            "block {} row {} should have 4 cells",
                            block_idx,
                            row_idx
                        );
                        // Verify cell widths are equal (total width / 4 columns)
                        let expected_cell_width = first_width / 4.0;
                        for (cell_idx, cell) in row.cells.iter().enumerate() {
                            assert!(
                                (cell.bounds.width - expected_cell_width).abs() < 0.01,
                                "block {} row {} cell {} width mismatch: {} vs {}",
                                block_idx,
                                row_idx,
                                cell_idx,
                                cell.bounds.width,
                                expected_cell_width
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn layout_block_level_image() {
        // Create a document with a block-level Image node as a direct child of Body
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add an Image node as a direct child of Body
        let img_id = doc.next_id();
        let mut img_node = Node::new(img_id, NodeType::Image);
        img_node.attributes.set(
            AttributeKey::ImageWidth,
            AttributeValue::Float(200.0),
        );
        img_node.attributes.set(
            AttributeKey::ImageHeight,
            AttributeValue::Float(150.0),
        );
        doc.insert_node(body_id, 0, img_node).unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Image { bounds, .. } => {
                assert!((bounds.width - 200.0).abs() < 0.01);
                assert!((bounds.height - 150.0).abs() < 0.01);
            }
            other => panic!("Expected Image block, got {:?}", other),
        }
    }

    #[test]
    fn layout_inline_image_in_paragraph() {
        // Create a document with an Image node as a child of a Paragraph
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Add an inline image as a child of the paragraph
        let img_id = doc.next_id();
        let mut img_node = Node::new(img_id, NodeType::Image);
        img_node.attributes.set(
            AttributeKey::ImageWidth,
            AttributeValue::Float(120.0),
        );
        img_node.attributes.set(
            AttributeKey::ImageHeight,
            AttributeValue::Float(80.0),
        );
        doc.insert_node(para_id, 0, img_node).unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 1);
        // The paragraph should contain a line with a glyph run that has an inline_image
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Paragraph { lines, .. } => {
                assert!(!lines.is_empty(), "paragraph should have lines");
                let has_inline_image = lines.iter().any(|line| {
                    line.runs.iter().any(|run| run.inline_image.is_some())
                });
                assert!(has_inline_image, "paragraph should contain an inline image run");
                // Check the inline image dimensions
                let img_run = lines
                    .iter()
                    .flat_map(|l| l.runs.iter())
                    .find(|r| r.inline_image.is_some())
                    .unwrap();
                let inline = img_run.inline_image.as_ref().unwrap();
                assert!((inline.width - 120.0).abs() < 0.01);
                assert!((inline.height - 80.0).abs() < 0.01);
            }
            other => panic!("Expected Paragraph block, got {:?}", other),
        }
    }

    #[test]
    fn layout_inline_drawing_in_paragraph() {
        // Create a document with a Drawing (VML) node as a child of a Paragraph
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Add an inline Drawing with ShapeWidth/ShapeHeight
        let drawing_id = doc.next_id();
        let mut drawing_node = Node::new(drawing_id, NodeType::Drawing);
        drawing_node.attributes.set(
            AttributeKey::ShapeType,
            AttributeValue::String("oval".to_string()),
        );
        drawing_node.attributes.set(
            AttributeKey::ShapeWidth,
            AttributeValue::Float(160.0),
        );
        drawing_node.attributes.set(
            AttributeKey::ShapeHeight,
            AttributeValue::Float(100.0),
        );
        doc.insert_node(para_id, 0, drawing_node).unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 1);
        match &result.pages[0].blocks[0].kind {
            LayoutBlockKind::Paragraph { lines, .. } => {
                assert!(!lines.is_empty(), "paragraph should have lines");
                let img_run = lines
                    .iter()
                    .flat_map(|l| l.runs.iter())
                    .find(|r| r.inline_image.is_some());
                assert!(img_run.is_some(), "paragraph should contain an inline image run for Drawing");
                let inline = img_run.unwrap().inline_image.as_ref().unwrap();
                assert!(
                    (inline.width - 160.0).abs() < 0.01,
                    "expected width 160, got {}",
                    inline.width
                );
                assert!(
                    (inline.height - 100.0).abs() < 0.01,
                    "expected height 100, got {}",
                    inline.height
                );
            }
            other => panic!("Expected Paragraph block, got {:?}", other),
        }
    }
}
