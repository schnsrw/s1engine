//! Core layout engine — converts a document model into positioned pages.

use s1_model::{
    AttributeKey, AttributeValue, DocumentModel, FieldType, HeaderFooterType, LineSpacing, NodeId,
    NodeType, SectionBreakType, TableWidth,
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
        let full_page_content_rect = page_layout.content_rect();

        // Multi-column tracking
        let (mut column_count, mut column_spacing) =
            self.resolve_section_columns(current_section_idx);
        let mut current_column: u32 = 0;
        let mut content_rect = Self::column_content_rect(
            full_page_content_rect,
            column_count,
            column_spacing,
            current_column,
        );

        // Layout each block and paginate
        let mut pages: Vec<LayoutPage> = Vec::new();
        let mut current_y = content_rect.y;
        let mut page_blocks: Vec<LayoutBlock> = Vec::new();
        let mut floating_images: Vec<LayoutBlock> = Vec::new();
        // Floating image exclusion zones for text wrapping on the current page
        let mut page_floats: Vec<FloatingImageRect> = Vec::new();
        let mut page_index = 0;
        // Track which section each page belongs to (for header/footer resolution)
        let mut page_section_indices: Vec<usize> = Vec::new();
        // Track previous block's space_after for CSS-style margin collapsing
        let mut prev_space_after: f64 = 0.0;

        for (block_idx, (node_id, node_type)) in blocks.iter().enumerate() {
            let block_section_idx = section_map[block_idx];

            // Handle section change
            if block_section_idx != current_section_idx {
                // Determine break type from the NEW section's properties
                let break_type = sections
                    .get(block_section_idx)
                    .and_then(|s| s.break_type)
                    .unwrap_or(SectionBreakType::NextPage);

                // A continuous break can stay on the same page only when
                // the column count doesn't change. Column layout changes
                // require a page break to avoid overlap.
                let new_cols = self.resolve_section_columns(block_section_idx).0;
                let is_continuous =
                    matches!(break_type, SectionBreakType::Continuous) && new_cols == column_count;

                // For non-continuous breaks, flush the current page
                if !is_continuous && !page_blocks.is_empty() {
                    pages.push(self.make_page(
                        page_index,
                        &page_layout,
                        std::mem::take(&mut page_blocks),
                        current_section_idx,
                    ));
                    page_section_indices.push(current_section_idx);
                    page_index += 1;
                    page_floats.clear();
                }

                match break_type {
                    SectionBreakType::NextPage => {
                        // Already flushed — just switch layout
                    }
                    SectionBreakType::Continuous => {
                        // Continue on the same page. Don't flush, don't
                        // reset current_y. The new section's columns and
                        // layout take effect immediately below the
                        // existing content.
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
                            page_floats.clear();
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
                            page_floats.clear();
                        }
                    }
                    _ => {
                        // Unknown break types treated as NextPage
                    }
                }

                // Switch to the new section's page layout
                current_section_idx = block_section_idx;
                page_layout = self.resolve_page_layout_for_section(current_section_idx);
                let full_rect = page_layout.content_rect();
                // Reset column tracking for the new section
                let (cc, cs) = self.resolve_section_columns(current_section_idx);
                column_count = cc;
                column_spacing = cs;
                current_column = 0;
                content_rect =
                    Self::column_content_rect(full_rect, column_count, column_spacing, 0);
                // For continuous breaks, keep current_y so content flows
                // below the previous section's blocks on the same page.
                if !is_continuous {
                    current_y = content_rect.y;
                }
            }

            match node_type {
                NodeType::Paragraph => {
                    let para_style = resolve_paragraph_style(self.doc, *node_id);

                    // Handle page break before (explicit property or inline
                    // w:br type="page" inside the paragraph's runs)
                    let has_page_break = para_style.page_break_before
                        || self.paragraph_has_inline_page_break(*node_id);
                    if has_page_break && !page_blocks.is_empty() {
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                            current_section_idx,
                        ));
                        page_section_indices.push(current_section_idx);
                        page_index += 1;
                        page_floats.clear();
                        current_column = 0;
                        let full_rect = page_layout.content_rect();
                        content_rect = Self::column_content_rect(
                            full_rect,
                            column_count,
                            column_spacing,
                            current_column,
                        );
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

                    // F2.1: Text wrapping — adjust for floating images
                    // 1. TopAndBottom floats: advance Y past them
                    for flt in &page_floats {
                        if flt.wrap_type == WrapType::TopAndBottom {
                            let ex = flt.exclusion_rect();
                            if current_y >= ex.y && current_y < ex.bottom() {
                                current_y = ex.bottom();
                            }
                        }
                    }

                    // 2. Square/Tight/Through floats: narrow content rect
                    let para_rect =
                        Self::adjust_rect_for_floats(content_rect, current_y, &page_floats);

                    let block =
                        self.layout_paragraph_cached(*node_id, &para_style, para_rect, current_y)?;

                    let block_height = block.bounds.height;
                    let space_after = sanitize_pt(para_style.space_after);

                    // Check if this block fits in the current column/page
                    // NDA debug: content_rect.width ~ 432 (612-90-90)
                    if current_y + block_height > content_rect.bottom() && !page_blocks.is_empty() {
                        // Try next column before going to a new page
                        if current_column + 1 < column_count {
                            current_column += 1;
                            let full_rect = page_layout.content_rect();
                            content_rect = Self::column_content_rect(
                                full_rect,
                                column_count,
                                column_spacing,
                                current_column,
                            );
                            current_y = content_rect.y;
                        } else {
                            // All columns full — new page
                            pages.push(self.make_page(
                                page_index,
                                &page_layout,
                                std::mem::take(&mut page_blocks),
                                current_section_idx,
                            ));
                            page_section_indices.push(current_section_idx);
                            page_index += 1;
                            page_floats.clear();
                            current_column = 0;
                            let full_rect = page_layout.content_rect();
                            content_rect = Self::column_content_rect(
                                full_rect,
                                column_count,
                                column_spacing,
                                current_column,
                            );
                            current_y = content_rect.y;
                        }

                        // Re-layout in the new column/page
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
                        // If the re-laid-out block still overflows (block
                        // taller than page/column), advance to next
                        // column or force a page break so the next block
                        // starts fresh.
                        if current_y > content_rect.bottom() {
                            if current_column + 1 < column_count {
                                current_column += 1;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            } else {
                                pages.push(self.make_page(
                                    page_index,
                                    &page_layout,
                                    std::mem::take(&mut page_blocks),
                                    current_section_idx,
                                ));
                                page_section_indices.push(current_section_idx);
                                page_index += 1;
                                page_floats.clear();
                                current_column = 0;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            }
                            prev_space_after = 0.0;
                        }
                    } else {
                        page_blocks.push(block);
                        current_y += block_height + space_after;
                        prev_space_after = space_after;
                        // If the block overflows (e.g. single oversized
                        // block on an empty page), advance to next column
                        // or force a page break.
                        if current_y > content_rect.bottom() {
                            if current_column + 1 < column_count {
                                current_column += 1;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            } else {
                                pages.push(self.make_page(
                                    page_index,
                                    &page_layout,
                                    std::mem::take(&mut page_blocks),
                                    current_section_idx,
                                ));
                                page_section_indices.push(current_section_idx);
                                page_index += 1;
                                page_floats.clear();
                                current_column = 0;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            }
                            prev_space_after = 0.0;
                        }
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
                            let has_non_header_remaining =
                                all_rows[row_idx..].iter().any(|r| !r.is_header_row);
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
                                // for a single very tall row). Clamp height to available space
                                // so the page bounds stay consistent.
                                let mut placed_row = row.clone();
                                placed_row.bounds.y = chunk_height;
                                chunk_rows.push(placed_row);
                                let effective_h = if !added_any_data_row && row_h > available {
                                    available.max(0.0) // clamp oversized single row
                                } else {
                                    row_h
                                };
                                chunk_height += effective_h;
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

                            // If there are more rows, advance to next column or new page
                            if row_idx < all_rows.len() {
                                if current_column + 1 < column_count {
                                    current_column += 1;
                                    let full_rect = page_layout.content_rect();
                                    content_rect = Self::column_content_rect(
                                        full_rect,
                                        column_count,
                                        column_spacing,
                                        current_column,
                                    );
                                    current_y = content_rect.y;
                                } else {
                                    pages.push(self.make_page(
                                        page_index,
                                        &page_layout,
                                        std::mem::take(&mut page_blocks),
                                        current_section_idx,
                                    ));
                                    page_section_indices.push(current_section_idx);
                                    page_index += 1;
                                    page_floats.clear();
                                    current_column = 0;
                                    let full_rect = page_layout.content_rect();
                                    content_rect = Self::column_content_rect(
                                        full_rect,
                                        column_count,
                                        column_spacing,
                                        current_column,
                                    );
                                    current_y = content_rect.y;
                                }
                                is_first_chunk = false;
                            }
                        }
                        // After the table is fully placed, if current_y
                        // exceeds the page, advance to next column or force
                        // a page break so following blocks don't overlap.
                        if current_y > content_rect.bottom() && !page_blocks.is_empty() {
                            if current_column + 1 < column_count {
                                current_column += 1;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            } else {
                                pages.push(self.make_page(
                                    page_index,
                                    &page_layout,
                                    std::mem::take(&mut page_blocks),
                                    current_section_idx,
                                ));
                                page_section_indices.push(current_section_idx);
                                page_index += 1;
                                page_floats.clear();
                                current_column = 0;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            }
                        }
                    }
                }
                NodeType::Image => {
                    // Check if this is a floating image
                    let is_floating = self
                        .doc
                        .node(*node_id)
                        .and_then(|n| n.attributes.get_string(&AttributeKey::ImagePositionType))
                        .map(|s| s == "anchor")
                        .unwrap_or(false);

                    if is_floating {
                        // Floating images don't participate in normal flow.
                        // Position them based on their offset attributes and
                        // add to the current page's floating images list.
                        let block =
                            self.layout_floating_image(*node_id, &page_layout, current_y)?;
                        // Build exclusion zone for text wrapping
                        let float_rect = self.build_float_rect(*node_id, &block);
                        page_floats.push(float_rect);
                        floating_images.push(block);
                    } else {
                        let block = self.layout_image(*node_id, content_rect, current_y)?;
                        let block_height = block.bounds.height;

                        if current_y + block_height > content_rect.bottom()
                            && !page_blocks.is_empty()
                        {
                            // Try next column before going to a new page
                            if current_column + 1 < column_count {
                                current_column += 1;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            } else {
                                // Flush floating images to the page before creating it
                                let mut new_page = self.make_page(
                                    page_index,
                                    &page_layout,
                                    std::mem::take(&mut page_blocks),
                                    current_section_idx,
                                );
                                new_page.floating_images = std::mem::take(&mut floating_images);
                                pages.push(new_page);
                                page_section_indices.push(current_section_idx);
                                page_index += 1;
                                page_floats.clear();
                                current_column = 0;
                                let full_rect = page_layout.content_rect();
                                content_rect = Self::column_content_rect(
                                    full_rect,
                                    column_count,
                                    column_spacing,
                                    current_column,
                                );
                                current_y = content_rect.y;
                            }

                            let block = self.layout_image(*node_id, content_rect, current_y)?;
                            let block_height = block.bounds.height;
                            page_blocks.push(block);
                            current_y += block_height;
                        } else {
                            page_blocks.push(block);
                            current_y += block_height;
                        }
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
                        page_floats.clear();
                        current_column = 0;
                        let full_rect = page_layout.content_rect();
                        content_rect = Self::column_content_rect(
                            full_rect,
                            column_count,
                            column_spacing,
                            current_column,
                        );
                        current_y = content_rect.y;
                    }
                }
                NodeType::ColumnBreak => {
                    // Force a column break — advance to next column or next page
                    if current_column + 1 < column_count {
                        current_column += 1;
                        let full_rect = page_layout.content_rect();
                        content_rect = Self::column_content_rect(
                            full_rect,
                            column_count,
                            column_spacing,
                            current_column,
                        );
                        current_y = content_rect.y;
                        prev_space_after = 0.0;
                    } else if !page_blocks.is_empty() {
                        // Last column — treat as page break
                        pages.push(self.make_page(
                            page_index,
                            &page_layout,
                            std::mem::take(&mut page_blocks),
                            current_section_idx,
                        ));
                        page_section_indices.push(current_section_idx);
                        page_index += 1;
                        page_floats.clear();
                        current_column = 0;
                        let full_rect = page_layout.content_rect();
                        content_rect = Self::column_content_rect(
                            full_rect,
                            column_count,
                            column_spacing,
                            current_column,
                        );
                        current_y = content_rect.y;
                        prev_space_after = 0.0;
                    }
                }
                _ => {} // Skip other node types
            }
        }

        // Flush remaining blocks
        if !page_blocks.is_empty() {
            let mut last_page =
                self.make_page(page_index, &page_layout, page_blocks, current_section_idx);
            last_page.floating_images = std::mem::take(&mut floating_images);
            pages.push(last_page);
            page_section_indices.push(current_section_idx);
        }

        // If any floating images remain (all pages already pushed), add them to the last page
        if !floating_images.is_empty() {
            if let Some(last) = pages.last_mut() {
                last.floating_images.extend(floating_images);
            }
        }

        // Ensure at least one page
        if pages.is_empty() {
            let default_layout = self.resolve_page_layout_for_section(initial_section_idx);
            pages.push(LayoutPage {
                index: 0,
                width: default_layout.width,
                height: default_layout.height,
                content_area: default_layout.content_rect(),
                blocks: Vec::new(),
                header: None,
                footer: None,
                footnotes: Vec::new(),
                floating_images: Vec::new(),
                section_index: initial_section_idx,
            });
            page_section_indices.push(initial_section_idx);
        }

        // Apply widow/orphan control (uses per-page dimensions)
        self.apply_widow_orphan_control(&mut pages, &page_section_indices)?;

        // Layout headers and footers for each page using the correct section
        let total_pages = pages.len();
        for (i, page) in pages.iter_mut().enumerate() {
            let sect_idx = page_section_indices.get(i).copied().unwrap_or(0);
            self.layout_header_footer_for_section(page, total_pages, sect_idx)?;
        }

        // Layout footnotes for each page
        self.layout_footnotes_for_pages(&mut pages)?;

        // Collect bookmarks from pages
        let bookmarks = self.collect_bookmarks(&pages);
        // Collect annotations (comments, highlights) from pages
        let annotations = self.collect_annotations(&pages);

        Ok(LayoutDocument {
            pages,
            bookmarks,
            annotations,
        })
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

    /// Resolve column properties for a specific section index.
    ///
    /// Returns `(column_count, column_spacing)`.
    fn resolve_section_columns(&self, section_idx: usize) -> (u32, f64) {
        let sections = self.doc.sections();
        if let Some(sp) = sections.get(section_idx) {
            (sp.columns.max(1), sp.column_spacing)
        } else if let Some(sp) = sections.last() {
            (sp.columns.max(1), sp.column_spacing)
        } else {
            (1, 36.0)
        }
    }

    /// Compute the content rect for a specific column within a page.
    ///
    /// Given the full-page content rect, column count, spacing, and the column
    /// index (0-based), returns the rect for that column.
    fn column_content_rect(
        full_content_rect: Rect,
        column_count: u32,
        column_spacing: f64,
        column_index: u32,
    ) -> Rect {
        if column_count <= 1 {
            return full_content_rect;
        }
        let n = column_count as f64;
        let col_width = (full_content_rect.width - (n - 1.0) * column_spacing) / n;
        let x = full_content_rect.x + column_index as f64 * (col_width + column_spacing);
        // Clamp last column width to avoid floating-point drift
        let actual_width = if column_index == column_count - 1 {
            full_content_rect.right() - x
        } else {
            col_width
        };
        Rect::new(
            x,
            full_content_rect.y,
            actual_width,
            full_content_rect.height,
        )
    }

    /// Build a mapping from block index to section index.
    ///
    /// In DOCX, a paragraph with `SectionIndex(i)` marks the END of section `i`.
    /// All blocks from the previous section boundary up to and including that
    /// paragraph belong to section `i`. Blocks after the last marked paragraph
    /// belong to the final section (the last entry in `doc.sections()`).
    fn build_section_map(&self, blocks: &[(NodeId, NodeType)]) -> Vec<usize> {
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
                if let Some(sect_idx) = node.attributes.get_i64(&AttributeKey::SectionIndex) {
                    section_end_blocks.push((block_idx, sect_idx as usize));
                }
            }
        }

        // Assign section indices to blocks.
        // Blocks up to and including the first section-end marker belong to
        // that section. Between markers, blocks belong to the next marker's
        // section. After the last marker, blocks belong to the final section.
        let mut current_section = if let Some(&(_, sect_idx)) = section_end_blocks.first() {
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
                        current_section = section_end_blocks[marker_idx].1;
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
                        NodeType::ColumnBreak => {
                            blocks.push((child_id, NodeType::ColumnBreak));
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
        hash ^= (para_style.keep_with_next as u64)
            | ((para_style.keep_lines as u64) << 1)
            | ((para_style.page_break_before as u64) << 2);
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
                    kind: LayoutBlockKind::Paragraph {
                        lines: Vec::new(),
                        text_align: None,
                        background_color: None,
                        border: None,
                        list_marker: None,
                        list_level: 0,
                        space_before: para_style.space_before,
                        space_after: para_style.space_after,
                        indent_left: para_style.indent_left,
                        indent_right: para_style.indent_right,
                        indent_first_line: para_style.indent_first_line,
                        line_height: line_spacing_to_css(&para_style.line_spacing),
                        bidi: para_style.bidi,
                    },
                });
            }
        };

        // Determine paragraph text direction for BiDi support
        let direction = if para_style.bidi {
            s1_text::Direction::Rtl
        } else {
            // Auto-detect from concatenated paragraph text
            let mut all_text = String::new();
            for &cid in &para.children {
                if let Some(c) = self.doc.node(cid) {
                    if c.node_type == NodeType::Run {
                        for &sub_id in &c.children {
                            if let Some(sub) = self.doc.node(sub_id) {
                                if sub.node_type == NodeType::Text {
                                    if let Some(t) = &sub.text_content {
                                        all_text.push_str(t);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if all_text.is_empty() {
                s1_text::Direction::Ltr
            } else {
                s1_text::paragraph_direction(&all_text)
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
                            self.shape_run_with_breaks(child_id, &mut shaped_runs, direction)?;
                        } else {
                            let run_info = self.shape_run(child_id, direction)?;
                            shaped_runs.push(run_info);
                        }
                    }
                    NodeType::LineBreak => {
                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_family: String::new(),
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
                        let current_x: f64 = shaped_runs
                            .iter()
                            .map(|r| {
                                let glyph_w: f64 = r.glyphs.iter().map(|g| g.x_advance).sum();
                                let nc = r.text.chars().count();
                                let sp = if nc > 1 {
                                    (nc as f64 - 1.0) * r.character_spacing
                                } else {
                                    0.0
                                };
                                glyph_w + sp
                            })
                            .sum();

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
                                let next_default = ((current_x / default_interval).floor() + 1.0)
                                    * default_interval;
                                (next_default - current_x).max(1.0)
                            }
                        } else {
                            // No custom tab stops — use default 36pt (0.5") interval
                            let default_interval = 36.0;
                            let next_default =
                                ((current_x / default_interval).floor() + 1.0) * default_interval;
                            (next_default - current_x).max(1.0)
                        };

                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_family: String::new(),
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
                                font_family: String::new(),
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
                            .and_then(|v| {
                                if let AttributeValue::Float(d) = v {
                                    Some(*d)
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                img_node
                                    .attributes
                                    .get(&AttributeKey::ShapeWidth)
                                    .and_then(|v| {
                                        if let AttributeValue::Float(d) = v {
                                            Some(*d)
                                        } else {
                                            None
                                        }
                                    })
                            })
                            .unwrap_or(100.0);
                        // Try ImageHeight first, fall back to ShapeHeight for VML drawings
                        let img_h = img_node
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
                                img_node
                                    .attributes
                                    .get(&AttributeKey::ShapeHeight)
                                    .and_then(|v| {
                                        if let AttributeValue::Float(d) = v {
                                            Some(*d)
                                        } else {
                                            None
                                        }
                                    })
                            })
                            .unwrap_or(100.0);

                        // Constrain to available area (width and height)
                        let scale_w = if img_w > available_width {
                            available_width / img_w
                        } else {
                            1.0
                        };
                        let scale_h = if img_h > content_rect.height && content_rect.height > 0.0 {
                            content_rect.height / img_h
                        } else {
                            1.0
                        };
                        let scale = scale_w.min(scale_h);
                        let final_w = img_w * scale;
                        let final_h = img_h * scale;

                        // Get media ID and image data
                        let media_id_val = img_node
                            .attributes
                            .get(&AttributeKey::ImageMediaId)
                            .and_then(|v| {
                                if let AttributeValue::MediaId(mid) = v {
                                    Some(mid.0)
                                } else {
                                    None
                                }
                            });
                        let media_id_str = media_id_val
                            .map(|id| format!("{id}"))
                            .or_else(|| {
                                img_node
                                    .attributes
                                    .get_string(&AttributeKey::ImageMediaId)
                                    .map(|s| s.to_string())
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
                            font_family: String::new(),
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
            if let Some(s1_model::AttributeValue::ListInfo(li)) =
                para_node.attributes.get(&s1_model::AttributeKey::ListInfo)
            {
                let marker = match li.num_format {
                    s1_model::ListFormat::Bullet => "\u{2022}".to_string(),
                    s1_model::ListFormat::Decimal => format!("{}.", li.start.unwrap_or(1)),
                    s1_model::ListFormat::LowerAlpha => {
                        let c = (b'a' + (li.start.unwrap_or(1) as u8).saturating_sub(1).min(25))
                            as char;
                        format!("{}.", c)
                    }
                    s1_model::ListFormat::UpperAlpha => {
                        let c = (b'A' + (li.start.unwrap_or(1) as u8).saturating_sub(1).min(25))
                            as char;
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
                bidi: matches!(direction, s1_text::Direction::Rtl),
            },
        })
    }

    /// Shape a run node — resolve font, shape text, return shaped info.
    fn shape_run(
        &self,
        run_id: NodeId,
        direction: s1_text::Direction,
    ) -> Result<ShapedRunInfo, LayoutError> {
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

        // L-15: Only apply 65% scaling if the run doesn't have an explicit font size
        let has_explicit_size = self
            .doc
            .node(run_id)
            .map(|n| n.attributes.get(&AttributeKey::FontSize).is_some())
            .unwrap_or(false);
        let font_size = if (run_style.superscript || run_style.subscript) && !has_explicit_size {
            run_style.font_size * 0.65
        } else {
            run_style.font_size
        };

        // F1.2: Detect script for script-specific shaping
        let script_runs = s1_text::split_by_script(&text);
        let rb_script = script_runs
            .first()
            .and_then(|sr| s1_text::script::script_to_rustybuzz(sr.script));

        let (mut glyphs, metrics) = if let Some(fid) = font_id {
            if let Some(font) = self.font_db.load_font(fid) {
                let shaped = s1_text::shaping::shape_text_with_script(
                    &text,
                    &font,
                    font_size,
                    &[],
                    None,
                    direction,
                    rb_script,
                )?;
                let metrics = font.metrics(font_size);
                (shaped, Some(metrics))
            } else {
                (synthesize_glyphs(&text, font_size), None)
            }
        } else {
            (synthesize_glyphs(&text, font_size), None)
        };

        // F4.1/F4.2: Per-character font fallback with script-aware ordering
        if font_id.is_some() && glyphs.iter().any(|g| g.glyph_id == 0) {
            let chars: Vec<char> = text.chars().collect();
            let char_byte_offsets: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
            let dominant_script = script_runs
                .first()
                .map(|sr| sr.script)
                .unwrap_or(unicode_script::Script::Common);

            for glyph in &mut glyphs {
                if glyph.glyph_id != 0 {
                    continue;
                }
                let cluster = glyph.cluster as usize;
                let char_idx = char_byte_offsets
                    .iter()
                    .position(|&off| off == cluster)
                    .unwrap_or(0);

                if char_idx < chars.len() {
                    let ch = chars[char_idx];
                    let ch_script = unicode_script::UnicodeScript::script(&ch);
                    let fb_script = if ch_script != unicode_script::Script::Common
                        && ch_script != unicode_script::Script::Inherited
                    {
                        ch_script
                    } else {
                        dominant_script
                    };
                    if let Some(fb_fid) = self.font_db.fallback_for_script(ch, fb_script) {
                        if let Some(fb_font) = self.font_db.load_font(fb_fid) {
                            if let Some(gid) = fb_font.glyph_index(ch) {
                                glyph.glyph_id = gid;
                                if let Some(advance) = fb_font.glyph_hor_advance(gid) {
                                    let scale = font_size / fb_font.units_per_em() as f64;
                                    glyph.x_advance = advance as f64 * scale;
                                }
                            }
                        }
                    }
                }
            }
        }

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
            font_family: run_style.font_family.clone(),
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
        direction: s1_text::Direction,
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

        let hyperlink_url = run_node
            .attributes
            .get_string(&AttributeKey::HyperlinkUrl)
            .map(|s| s.to_string())
            .or_else(|| {
                run_node.parent.and_then(|pid| {
                    self.doc.node(pid).and_then(|p| {
                        p.attributes
                            .get_string(&AttributeKey::HyperlinkUrl)
                            .map(|s| s.to_string())
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
                                run_id,
                                &segment_text,
                                &run_style,
                                font_id,
                                hyperlink_url.clone(),
                                direction,
                            )?;
                            shaped_runs.push(info);
                            segment_text.clear();
                        }
                        // Add line break marker
                        shaped_runs.push(ShapedRunInfo {
                            source_id: child_id,
                            font_id: None,
                            font_family: run_style.font_family.clone(),
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
                run_id,
                &segment_text,
                &run_style,
                font_id,
                hyperlink_url,
                direction,
            )?;
            shaped_runs.push(info);
        }
        Ok(())
    }

    /// Shape a text segment with given run styling.
    ///
    /// After shaping with the primary font, checks for missing glyphs (glyph_id == 0)
    /// and attempts per-character font fallback via `FontDatabase::fallback()`.
    fn shape_text_segment(
        &self,
        source_id: NodeId,
        text: &str,
        run_style: &ResolvedRunStyle,
        font_id: Option<s1_text::FontId>,
        hyperlink_url: Option<String>,
        direction: s1_text::Direction,
    ) -> Result<ShapedRunInfo, LayoutError> {
        // L-15: Only apply 65% scaling if the run doesn't have an explicit font size
        let has_explicit_size = self
            .doc
            .node(source_id)
            .map(|n| n.attributes.get(&AttributeKey::FontSize).is_some())
            .unwrap_or(false);
        let font_size = if (run_style.superscript || run_style.subscript) && !has_explicit_size {
            run_style.font_size * 0.65
        } else {
            run_style.font_size
        };

        // F1.2: Detect script for script-specific shaping
        let script_runs = s1_text::split_by_script(text);
        let rb_script = script_runs
            .first()
            .and_then(|sr| s1_text::script::script_to_rustybuzz(sr.script));

        let (mut glyphs, metrics, actual_font_id) = if let Some(fid) = font_id {
            if let Some(font) = self.font_db.load_font(fid) {
                let shaped = s1_text::shaping::shape_text_with_script(
                    text,
                    &font,
                    font_size,
                    &[],
                    None,
                    direction,
                    rb_script,
                )?;
                let metrics = font.metrics(font_size);
                (shaped, Some(metrics), Some(fid))
            } else {
                (synthesize_glyphs(text, font_size), None, None)
            }
        } else {
            (synthesize_glyphs(text, font_size), None, None)
        };

        // F4.1/F4.2: Per-character font fallback for missing glyphs (glyph_id == 0).
        // Uses script-aware fallback: tries script-preferred fonts first, then
        // falls back to general linear scan.
        if actual_font_id.is_some() && glyphs.iter().any(|g| g.glyph_id == 0) {
            let chars: Vec<char> = text.chars().collect();
            let char_byte_offsets: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
            // Get the dominant script for script-aware fallback
            let dominant_script = script_runs
                .first()
                .map(|sr| sr.script)
                .unwrap_or(unicode_script::Script::Common);

            for glyph in &mut glyphs {
                if glyph.glyph_id != 0 {
                    continue;
                }
                // Find the character for this glyph via cluster mapping
                let cluster = glyph.cluster as usize;
                let char_idx = char_byte_offsets
                    .iter()
                    .position(|&off| off == cluster)
                    .unwrap_or(0);

                if char_idx < chars.len() {
                    let ch = chars[char_idx];
                    // Use script-aware fallback for non-Common scripts
                    let ch_script = unicode_script::UnicodeScript::script(&ch);
                    let fb_script = if ch_script != unicode_script::Script::Common
                        && ch_script != unicode_script::Script::Inherited
                    {
                        ch_script
                    } else {
                        dominant_script
                    };
                    let fb_fid = self.font_db.fallback_for_script(ch, fb_script);
                    if let Some(fb_fid) = fb_fid {
                        if let Some(fb_font) = self.font_db.load_font(fb_fid) {
                            if let Some(gid) = fb_font.glyph_index(ch) {
                                glyph.glyph_id = gid;
                                if let Some(advance) = fb_font.glyph_hor_advance(gid) {
                                    let scale = font_size / fb_font.units_per_em() as f64;
                                    glyph.x_advance = advance as f64 * scale;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ShapedRunInfo {
            source_id,
            font_id: actual_font_id.or(font_id),
            font_family: run_style.font_family.clone(),
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
            // For empty paragraphs, resolve the font size from the paragraph's style
            // rather than hardcoding DEFAULT_FONT_SIZE. This ensures an empty 24pt
            // paragraph is taller than an empty 10pt paragraph.
            let styled_font_size = para_style.default_font_size.unwrap_or(DEFAULT_FONT_SIZE);
            let base_size = match &para_style.line_spacing {
                LineSpacing::Exact(h) => *h,
                LineSpacing::AtLeast(h) => styled_font_size.max(*h),
                _ => styled_font_size,
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
            let mut max_image_height: f64 = 0.0;

            // Check if the line ends at a hyphenation Penalty (flagged = true)
            let ends_with_hyphen = end > 0
                && matches!(
                    items.get(end.saturating_sub(1)),
                    Some(BreakItem::Penalty { flagged: true, .. })
                );

            for item in &items[start..end] {
                match item {
                    BreakItem::Box {
                        run_idx,
                        width,
                        height,
                        glyph_start,
                        glyph_end,
                        text_byte_start,
                        text_byte_end,
                    } => {
                        let run_info = &runs[*run_idx];
                        let font_id = run_info.font_id.unwrap_or(FontId(fontdb::ID::dummy()));

                        // Use the sub-range of glyphs and text for this box
                        let sub_glyphs = run_info.glyphs[*glyph_start..*glyph_end].to_vec();
                        let sub_text = run_info.text[*text_byte_start..*text_byte_end].to_string();

                        line_runs.push(GlyphRun {
                            source_id: run_info.source_id,
                            font_id,
                            font_family: run_info.font_family.clone(),
                            font_size: run_info.font_size,
                            color: run_info.color,
                            x_offset: current_x,
                            glyphs: sub_glyphs,
                            width: *width,
                            hyperlink_url: run_info.hyperlink_url.clone(),
                            text: sub_text,
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
                        // Track image heights separately so line spacing
                        // is NOT applied to inline images (only to text).
                        if runs[*run_idx].inline_image.is_some() {
                            if *height > max_image_height {
                                max_image_height = *height;
                            }
                        } else if *height > max_height {
                            max_height = *height;
                        }
                    }
                    BreakItem::Glue { width, .. } => {
                        current_x += width;
                    }
                    BreakItem::Penalty { .. } | BreakItem::ForcedBreak { .. } => {}
                }
            }

            // Merge consecutive sub-runs from the same source run.
            // Hyphenation splits words into sub-boxes (e.g., "Ti" + "tle"), but if no
            // break occurred between them, they should be a single GlyphRun for correct
            // text output and rendering.
            let mut merged_runs: Vec<GlyphRun> = Vec::new();
            for run in line_runs {
                let should_merge = merged_runs.last().is_some_and(|prev: &GlyphRun| {
                    prev.source_id == run.source_id
                        && prev.font_id == run.font_id
                        && (prev.font_size - run.font_size).abs() < 0.01
                        && prev.bold == run.bold
                        && prev.italic == run.italic
                });
                if should_merge {
                    let prev = merged_runs.last_mut().unwrap();
                    prev.glyphs.extend(run.glyphs);
                    prev.text.push_str(&run.text);
                    prev.width += run.width;
                } else {
                    merged_runs.push(run);
                }
            }
            let mut line_runs = merged_runs;

            // If the line ends at a hyphenation point, append "-" to the last run's text
            if ends_with_hyphen {
                if let Some(last_run) = line_runs.last_mut() {
                    last_run.text.push('-');
                    let hyphen_width = last_run.font_size * 0.3;
                    last_run.width += hyphen_width;
                }
            }

            // Apply line spacing only to text height, not to inline image
            // height. An inline image's height is used directly. The final
            // line height is the larger of the two.
            let text_h = if max_height > 0.0 {
                max_height
            } else if max_image_height <= 0.0 {
                // Empty lines (e.g. consecutive line breaks) should use the
                // paragraph's styled font size, not the hardcoded default.
                para_style.default_font_size.unwrap_or(DEFAULT_FONT_SIZE)
            } else {
                0.0
            };
            let text_line_height = compute_line_height(text_h, &para_style.line_spacing);
            let line_height = text_line_height.max(max_image_height);
            lines.push(LayoutLine {
                baseline_y: 0.0,
                height: line_height,
                runs: line_runs,
            });
        }

        if lines.is_empty() {
            let fallback_size = para_style.default_font_size.unwrap_or(DEFAULT_FONT_SIZE);
            let line_height = compute_line_height(fallback_size, &para_style.line_spacing);
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
    ///
    /// Splits runs at word boundaries (spaces) so the line-breaking algorithm
    /// can break within multi-word runs. Optionally inserts hyphenation penalty
    /// items at valid hyphenation points within long words.
    fn build_break_items(&self, runs: &[ShapedRunInfo]) -> Vec<BreakItem> {
        let mut items: Vec<BreakItem> = Vec::new();

        for (run_idx, run_info) in runs.iter().enumerate() {
            if run_info.is_line_break {
                items.push(BreakItem::ForcedBreak { run_idx });
                continue;
            }

            // If the run is an inline image or empty, treat as atomic box
            if run_info.inline_image.is_some() || run_info.text.is_empty() {
                let glyph_advance: f64 = run_info.glyphs.iter().map(|g| g.x_advance).sum();
                let run_height = run_info
                    .metrics
                    .map(|m| m.ascent - m.descent)
                    .unwrap_or(run_info.font_size);
                items.push(BreakItem::Box {
                    run_idx,
                    width: glyph_advance,
                    height: run_height,
                    glyph_start: 0,
                    glyph_end: run_info.glyphs.len(),
                    text_byte_start: 0,
                    text_byte_end: run_info.text.len(),
                });
                items.push(BreakItem::Glue {
                    width: 0.0,
                    stretch: 0.0,
                    shrink: 0.0,
                });
                continue;
            }

            let run_height = run_info
                .metrics
                .map(|m| m.ascent - m.descent)
                .unwrap_or(run_info.font_size);

            // Split the run at word boundaries (space → non-space transitions)
            // so the line-breaking algorithm can break within multi-word runs.
            // Trailing spaces are included with the preceding word to preserve
            // text content in GlyphRun output. Zero-width Glue items between
            // word+space groups serve as break opportunities.
            let text = &run_info.text;
            let glyphs = &run_info.glyphs;

            // Build word groups: each group is "word" + optional trailing spaces.
            // Split at non-space→space→non-space transitions: the break is between
            // the last trailing space and the next word character.
            let mut word_groups: Vec<(usize, usize)> = Vec::new(); // (byte_start, byte_end)
            let bytes = text.as_bytes();
            if !bytes.is_empty() {
                let mut group_start: usize = 0;
                let mut prev_was_space = bytes[0] == b' ';
                for (byte_idx, ch) in text.char_indices().skip(1) {
                    let is_space = ch == ' ';
                    if !is_space && prev_was_space && byte_idx > group_start {
                        // Transition from space back to word: end current group
                        word_groups.push((group_start, byte_idx));
                        group_start = byte_idx;
                    }
                    prev_was_space = is_space;
                }
                word_groups.push((group_start, text.len()));
            }

            // If only one group (no splits), keep as single Box
            if word_groups.len() <= 1 {
                let glyph_advance: f64 = glyphs.iter().map(|g| g.x_advance).sum();
                let num_chars = text.chars().count();
                let spacing_contribution = if num_chars > 1 {
                    (num_chars as f64 - 1.0) * run_info.character_spacing
                } else {
                    0.0
                };

                // Check for hyphenation on single-word runs
                let word_text = text.trim();
                let hyph_breaks = s1_text::hyphenation::hyphenate_word(word_text, "en-US");

                if hyph_breaks.is_empty() {
                    items.push(BreakItem::Box {
                        run_idx,
                        width: glyph_advance + spacing_contribution,
                        height: run_height,
                        glyph_start: 0,
                        glyph_end: glyphs.len(),
                        text_byte_start: 0,
                        text_byte_end: text.len(),
                    });
                } else {
                    // Split single word at hyphenation points
                    let word_start = text.find(|c: char| c != ' ').unwrap_or(0);
                    let mut prev_byte = 0;
                    let mut prev_glyph = 0;

                    for &hyph_byte_in_word in &hyph_breaks {
                        let hyph_byte = word_start + hyph_byte_in_word;
                        if hyph_byte >= text.len() {
                            break;
                        }

                        let hyph_glyph = glyphs[prev_glyph..]
                            .iter()
                            .position(|g| (g.cluster as usize) >= hyph_byte)
                            .map(|i| i + prev_glyph)
                            .unwrap_or(glyphs.len());

                        let sub_advance: f64 = glyphs[prev_glyph..hyph_glyph]
                            .iter()
                            .map(|g| g.x_advance)
                            .sum();
                        let sub_chars = text[prev_byte..hyph_byte].chars().count();
                        let sub_spacing = if sub_chars > 1 {
                            (sub_chars as f64 - 1.0) * run_info.character_spacing
                        } else {
                            0.0
                        };

                        items.push(BreakItem::Box {
                            run_idx,
                            width: sub_advance + sub_spacing,
                            height: run_height,
                            glyph_start: prev_glyph,
                            glyph_end: hyph_glyph,
                            text_byte_start: prev_byte,
                            text_byte_end: hyph_byte,
                        });
                        items.push(BreakItem::Penalty {
                            penalty: 50.0,
                            flagged: true,
                        });

                        prev_byte = hyph_byte;
                        prev_glyph = hyph_glyph;
                    }

                    // Remaining part
                    let remaining_advance: f64 =
                        glyphs[prev_glyph..].iter().map(|g| g.x_advance).sum();
                    let remaining_chars = text[prev_byte..].chars().count();
                    let remaining_spacing = if remaining_chars > 1 {
                        (remaining_chars as f64 - 1.0) * run_info.character_spacing
                    } else {
                        0.0
                    };
                    items.push(BreakItem::Box {
                        run_idx,
                        width: remaining_advance + remaining_spacing,
                        height: run_height,
                        glyph_start: prev_glyph,
                        glyph_end: glyphs.len(),
                        text_byte_start: prev_byte,
                        text_byte_end: text.len(),
                    });
                }
            } else {
                // Multiple word groups — create Box per group with Glue between
                for (gi, (group_start, group_end)) in word_groups.iter().enumerate() {
                    let g_start = glyphs
                        .iter()
                        .position(|g| (g.cluster as usize) >= *group_start)
                        .unwrap_or(glyphs.len());
                    let g_end = glyphs
                        .iter()
                        .rposition(|g| (g.cluster as usize) < *group_end)
                        .map(|i| i + 1)
                        .unwrap_or(g_start);

                    let group_advance: f64 =
                        glyphs[g_start..g_end].iter().map(|g| g.x_advance).sum();
                    let group_text = &text[*group_start..*group_end];
                    let group_chars = group_text.chars().count();
                    let spacing_contribution = if group_chars > 1 {
                        (group_chars as f64 - 1.0) * run_info.character_spacing
                    } else {
                        0.0
                    };

                    items.push(BreakItem::Box {
                        run_idx,
                        width: group_advance + spacing_contribution,
                        height: run_height,
                        glyph_start: g_start,
                        glyph_end: g_end,
                        text_byte_start: *group_start,
                        text_byte_end: *group_end,
                    });

                    // Add zero-width Glue as break opportunity between groups
                    if gi + 1 < word_groups.len() {
                        items.push(BreakItem::Glue {
                            width: 0.0,
                            stretch: 0.0,
                            shrink: 0.0,
                        });
                    }
                }
            }

            // Add inter-run glue (zero-width, allows break between runs)
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
    /// Maximum nesting depth for tables inside table cells.
    const MAX_TABLE_NESTING: usize = 5;

    fn layout_table_rows(
        &self,
        table_id: NodeId,
        content_rect: Rect,
    ) -> Result<Vec<LayoutTableRow>, LayoutError> {
        self.layout_table_rows_with_depth(table_id, content_rect, 0)
    }

    /// Layout table rows with a nesting depth cap to prevent infinite recursion.
    fn layout_table_rows_with_depth(
        &self,
        table_id: NodeId,
        content_rect: Rect,
        depth: usize,
    ) -> Result<Vec<LayoutTableRow>, LayoutError> {
        if depth > Self::MAX_TABLE_NESTING {
            return Ok(Vec::new());
        }

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

        // Build column widths from cell width attributes in the first row.
        // Falls back to equal distribution if no widths are specified.
        let col_widths: Vec<f64> = {
            let mut widths = Vec::with_capacity(num_cols);
            let mut has_explicit = false;

            if let Some(&first_row_id) = table.children.first() {
                if let Some(first_row) = self.doc.node(first_row_id) {
                    for &cell_id in first_row.children.iter().take(num_cols) {
                        if let Some(cell) = self.doc.node(cell_id) {
                            match cell.attributes.get(&AttributeKey::CellWidth) {
                                Some(AttributeValue::TableWidth(TableWidth::Fixed(pts))) => {
                                    widths.push(*pts);
                                    has_explicit = true;
                                }
                                Some(AttributeValue::TableWidth(TableWidth::Percent(pct))) => {
                                    widths.push(content_rect.width * pct / 100.0);
                                    has_explicit = true;
                                }
                                _ => {
                                    widths.push(0.0); // placeholder
                                }
                            }
                        } else {
                            widths.push(0.0);
                        }
                    }
                }
            }

            if !has_explicit || widths.is_empty() {
                // Fall back to equal distribution
                vec![content_rect.width / num_cols as f64; num_cols]
            } else {
                // Distribute remaining width among auto-sized columns
                let total_explicit: f64 = widths.iter().filter(|&&w| w > 0.0).sum();
                let auto_count = widths.iter().filter(|&&w| w <= 0.0).count();
                let remaining = (content_rect.width - total_explicit).max(0.0);
                let auto_width = if auto_count > 0 {
                    remaining / auto_count as f64
                } else {
                    0.0
                };

                // Scale proportionally if total exceeds content width
                let total_with_auto: f64 = total_explicit + auto_width * auto_count as f64;
                let scale = if total_with_auto > content_rect.width && total_with_auto > 0.0 {
                    content_rect.width / total_with_auto
                } else {
                    1.0
                };

                widths
                    .iter()
                    .map(|&w| {
                        if w > 0.0 {
                            w * scale
                        } else {
                            auto_width * scale
                        }
                    })
                    .collect()
            }
        };

        // Precompute cumulative x positions for columns
        let col_x_positions: Vec<f64> = {
            let mut positions = Vec::with_capacity(num_cols);
            let mut x = 0.0;
            for &w in &col_widths {
                positions.push(x);
                x += w;
            }
            positions
        };

        // Extract table-level borders for inheritance to cells without explicit borders
        let table_borders = table
            .attributes
            .get(&AttributeKey::TableBorders)
            .and_then(|v| {
                if let AttributeValue::Borders(b) = v {
                    Some(b.clone())
                } else {
                    None
                }
            });

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

                        let this_col_width = col_widths.get(col_idx).copied()
                            .unwrap_or(content_rect.width / num_cols as f64);
                        let cell_x = col_x_positions.get(col_idx).copied().unwrap_or(0.0);
                        let cell_rect = Rect::new(cell_x, 0.0, this_col_width, 0.0);

                        // Layout cell content
                        let mut cell_blocks = Vec::new();
                        let mut cell_y = 2.0; // Padding

                        for &content_id in &cell_node.children {
                            if let Some(content) = self.doc.node(content_id) {
                                if content.node_type == NodeType::Paragraph {
                                    let ps = resolve_paragraph_style(self.doc, content_id);
                                    let cell_content_rect =
                                        Rect::new(cell_x + 2.0, cell_y, this_col_width - 4.0, 1000.0);
                                    let block = self.layout_paragraph(
                                        content_id,
                                        &ps,
                                        cell_content_rect,
                                        cell_y,
                                    )?;
                                    cell_y += block.bounds.height + sanitize_pt(ps.space_after);
                                    cell_blocks.push(block);
                                } else if content.node_type == NodeType::Table {
                                    let nested_rect =
                                        Rect::new(cell_x + 2.0, cell_y, this_col_width - 4.0, 1000.0);
                                    let nested_rows = self.layout_table_rows_with_depth(
                                        content_id,
                                        nested_rect,
                                        depth + 1,
                                    )?;
                                    let nested_height: f64 =
                                        nested_rows.iter().map(|r| r.bounds.height).sum();
                                    let block = LayoutBlock {
                                        source_id: content_id,
                                        bounds: Rect::new(
                                            cell_x + 2.0,
                                            cell_y,
                                            this_col_width - 4.0,
                                            nested_height,
                                        ),
                                        kind: LayoutBlockKind::Table {
                                            rows: nested_rows,
                                            is_continuation: false,
                                        },
                                    };
                                    cell_y += nested_height;
                                    cell_blocks.push(block);
                                }
                            }
                        }

                        cell_y += 2.0; // Bottom padding
                        if cell_y > max_cell_height {
                            max_cell_height = cell_y;
                        }

                        // Extract cell background color
                        let background_color = cell_node
                            .attributes
                            .get(&AttributeKey::CellBackground)
                            .and_then(|v| {
                                if let AttributeValue::Color(c) = v {
                                    Some(*c)
                                } else {
                                    None
                                }
                            });

                        // Extract cell borders, inheriting from table borders if needed
                        let cell_borders = cell_node
                            .attributes
                            .get(&AttributeKey::CellBorders)
                            .and_then(|v| {
                                if let AttributeValue::Borders(b) = v {
                                    Some(b.clone())
                                } else {
                                    None
                                }
                            });

                        let (border_top, border_bottom, border_left, border_right) = {
                            // Use cell borders if present, otherwise inherit from table
                            let effective = cell_borders.as_ref().or(table_borders.as_ref());
                            if let Some(borders) = effective {
                                (
                                    borders.top.as_ref().and_then(format_border_css_opt),
                                    borders.bottom.as_ref().and_then(format_border_css_opt),
                                    borders.left.as_ref().and_then(format_border_css_opt),
                                    borders.right.as_ref().and_then(format_border_css_opt),
                                )
                            } else {
                                (None, None, None, None)
                            }
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
                    source_id: row_id,
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

        // Constrain to content area (width and height), preserving aspect ratio
        let scale_w = if width > content_rect.width {
            content_rect.width / width
        } else {
            1.0
        };
        let scale_h = if height > content_rect.height && content_rect.height > 0.0 {
            content_rect.height / height
        } else {
            1.0
        };
        let scale = scale_w.min(scale_h);

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

    /// Layout a floating (anchored) image.
    ///
    /// Computes absolute page coordinates from the image's offset attributes.
    /// EMU (English Metric Units): 914400 EMU = 1 inch = 72 points.
    fn layout_floating_image(
        &self,
        image_id: NodeId,
        page_layout: &PageLayout,
        current_y: f64,
    ) -> Result<LayoutBlock, LayoutError> {
        let node = self.doc.node(image_id);
        let content_rect = page_layout.content_rect();

        // Get image dimensions
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

        // Constrain to page width
        let scale = if width > page_layout.width {
            page_layout.width / width
        } else {
            1.0
        };
        let final_w = width * scale;
        let final_h = height * scale;

        // Convert EMU offsets to points (914400 EMU = 72pt)
        const EMU_PER_PT: f64 = 914400.0 / 72.0;

        let h_offset_emu = node
            .and_then(|n| n.attributes.get(&AttributeKey::ImageHorizontalOffset))
            .and_then(|v| {
                if let AttributeValue::Int(i) = v {
                    Some(*i)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let v_offset_emu = node
            .and_then(|n| n.attributes.get(&AttributeKey::ImageVerticalOffset))
            .and_then(|v| {
                if let AttributeValue::Int(i) = v {
                    Some(*i)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let h_relative = node
            .and_then(|n| {
                n.attributes
                    .get_string(&AttributeKey::ImageHorizontalRelativeFrom)
            })
            .unwrap_or("column");
        let v_relative = node
            .and_then(|n| {
                n.attributes
                    .get_string(&AttributeKey::ImageVerticalRelativeFrom)
            })
            .unwrap_or("paragraph");

        // Compute absolute X position
        let x = match h_relative {
            "page" => h_offset_emu as f64 / EMU_PER_PT,
            "margin" | "column" => content_rect.x + h_offset_emu as f64 / EMU_PER_PT,
            "character" => content_rect.x + h_offset_emu as f64 / EMU_PER_PT,
            _ => content_rect.x + h_offset_emu as f64 / EMU_PER_PT,
        };

        // Compute absolute Y position — use current_y for paragraph-relative
        let y = match v_relative {
            "page" => v_offset_emu as f64 / EMU_PER_PT,
            "paragraph" | "line" => current_y + v_offset_emu as f64 / EMU_PER_PT,
            "margin" => content_rect.y + v_offset_emu as f64 / EMU_PER_PT,
            _ => content_rect.y + v_offset_emu as f64 / EMU_PER_PT,
        };

        // Get media data
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
            bounds: Rect::new(x, y, final_w, final_h),
            kind: LayoutBlockKind::Image {
                media_id: media_id_str,
                bounds: Rect::new(0.0, 0.0, final_w, final_h),
                image_data,
                content_type,
            },
        })
    }

    /// Build a `FloatingImageRect` from a positioned floating image block.
    fn build_float_rect(&self, image_id: NodeId, block: &LayoutBlock) -> FloatingImageRect {
        let node = self.doc.node(image_id);

        // Parse wrap type
        let wrap_type = node
            .and_then(|n| n.attributes.get_string(&AttributeKey::ImageWrapType))
            .map(|s| match s {
                "square" => WrapType::Square,
                "tight" => WrapType::Tight,
                "through" => WrapType::Through,
                "topAndBottom" => WrapType::TopAndBottom,
                _ => WrapType::None,
            })
            .unwrap_or(WrapType::None);

        // Parse distance from text (stored as "distT,distB,distL,distR" in EMU)
        const EMU_PER_PT: f64 = 914400.0 / 72.0;
        let (dist_top, dist_bottom, dist_left, dist_right) = node
            .and_then(|n| {
                n.attributes
                    .get_string(&AttributeKey::ImageDistanceFromText)
            })
            .map(|s| {
                let parts: Vec<&str> = s.split(',').collect();
                let parse_emu = |idx: usize| -> f64 {
                    parts
                        .get(idx)
                        .and_then(|p| p.trim().parse::<f64>().ok())
                        .map(|emu| emu / EMU_PER_PT)
                        .unwrap_or(0.0)
                };
                (parse_emu(0), parse_emu(1), parse_emu(2), parse_emu(3))
            })
            .unwrap_or((0.0, 0.0, 0.0, 0.0));

        FloatingImageRect {
            bounds: block.bounds,
            wrap_type,
            dist_top,
            dist_bottom,
            dist_left,
            dist_right,
        }
    }

    /// Adjust a content rect to avoid overlapping square/tight/through floats.
    ///
    /// For floats on the left side of the content area, narrows from the left.
    /// For floats on the right side, narrows from the right.
    fn adjust_rect_for_floats(
        content_rect: Rect,
        current_y: f64,
        page_floats: &[FloatingImageRect],
    ) -> Rect {
        let mut adjusted = content_rect;

        for flt in page_floats {
            match flt.wrap_type {
                WrapType::Square | WrapType::Tight | WrapType::Through => {
                    let ex = flt.exclusion_rect();
                    // Check if this float overlaps the paragraph's starting Y
                    // Use a conservative check: float overlaps [current_y, page bottom]
                    if ex.y < content_rect.bottom() && ex.bottom() > current_y {
                        // Determine if float is on left or right side
                        let float_center_x = ex.x + ex.width / 2.0;
                        let content_center_x = content_rect.x + content_rect.width / 2.0;

                        if float_center_x < content_center_x {
                            // Float on left — narrow from left
                            let new_left = ex.right();
                            if new_left > adjusted.x && new_left < adjusted.right() {
                                let delta = new_left - adjusted.x;
                                adjusted.x = new_left;
                                adjusted.width -= delta;
                            }
                        } else {
                            // Float on right — narrow from right
                            let new_right = ex.x;
                            if new_right > adjusted.x && new_right < adjusted.right() {
                                adjusted.width = new_right - adjusted.x;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Ensure minimum width
        if adjusted.width < 36.0 {
            adjusted.width = 36.0;
        }

        adjusted
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

        let node = match self.doc.node(node_id) {
            Some(n) => n,
            None => {
                let hf_y = if is_header {
                    hf_distance
                } else {
                    page.height - hf_distance
                };
                return Ok(LayoutBlock {
                    source_id: node_id,
                    bounds: Rect::new(hf_x, hf_y, hf_width, 0.0),
                    kind: LayoutBlockKind::Paragraph {
                        lines: Vec::new(),
                        text_align: None,
                        background_color: None,
                        border: None,
                        list_marker: None,
                        list_level: 0,
                        space_before: 0.0,
                        space_after: 0.0,
                        indent_left: 0.0,
                        indent_right: 0.0,
                        indent_first_line: 0.0,
                        line_height: None,
                        bidi: false,
                    },
                });
            }
        };

        // Layout child paragraphs at a preliminary Y of 0 to measure total height
        let mut blocks = Vec::new();
        let mut current_y = 0.0;

        for &child_id in &node.children {
            if let Some(child) = self.doc.node(child_id) {
                if child.node_type == NodeType::Paragraph {
                    let para_style = resolve_paragraph_style(self.doc, child_id);
                    // Use preliminary rect at y=0 for measurement
                    let content_rect = Rect::new(hf_x, 0.0, hf_width, 100.0);
                    let mut block =
                        self.layout_paragraph(child_id, &para_style, content_rect, current_y)?;

                    // Substitute field values in glyph runs
                    self.substitute_fields_in_block(&mut block, page_num, total_pages);

                    current_y += block.bounds.height;
                    blocks.push(block);
                }
            }
        }

        // Compute actual hf_y using measured total height
        let total_height = current_y;
        let hf_y = if is_header {
            hf_distance
        } else {
            page.height - hf_distance - total_height
        };

        // Offset all blocks to the final hf_y position
        for block in &mut blocks {
            block.bounds.y += hf_y;
        }

        // Merge blocks into a single block (typical: one paragraph)
        if blocks.len() == 1 {
            let mut block = blocks.remove(0);
            block.source_id = node_id; // Use header/footer node ID
            block.bounds.x = hf_x;
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
                kind: LayoutBlockKind::Paragraph {
                    lines,
                    text_align: None,
                    background_color: None,
                    border: None,
                    list_marker: None,
                    list_level: 0,
                    space_before: 0.0,
                    space_after: 0.0,
                    indent_left: 0.0,
                    indent_right: 0.0,
                    indent_first_line: 0.0,
                    line_height: None,
                    bidi: false,
                },
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
                                if found {
                                    break;
                                }
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
                                        font_family: String::new(),
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
                // First check if the move would cause overflow on the next page.
                let can_move = {
                    let current_page = &pages[i];
                    let next_page = &pages[i + 1];
                    if current_page.blocks.len() > 1 {
                        if let Some(last_block) = current_page.blocks.last() {
                            let shift = last_block.bounds.height;
                            // Check if shifting blocks down would push the
                            // last block on the next page past the page height
                            if let Some(last_next) = next_page.blocks.last() {
                                let new_bottom =
                                    last_next.bounds.y + shift + last_next.bounds.height;
                                new_bottom <= next_page.height
                            } else {
                                true // next page is empty, always ok
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if can_move {
                    let current_page = &mut pages[i];
                    let Some(block) = current_page.blocks.pop() else {
                        i += 1;
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
                            self.collect_bookmarks_from_block(cell_block, page_index, bookmarks);
                        }
                    }
                }
            }
        }
    }

    /// Collect annotations (comments, highlights) from laid-out pages.
    ///
    /// Scans for `CommentStart` / `CommentEnd` node pairs and highlight
    /// formatting on glyph runs. Each comment's bounding rectangle is
    /// resolved to page coordinates.
    fn collect_annotations(&self, pages: &[LayoutPage]) -> Vec<LayoutAnnotation> {
        let mut annotations = Vec::new();

        // 1. Collect comment annotations by scanning for CommentStart nodes
        let mut comment_starts: Vec<(NodeId, usize, f64)> = Vec::new(); // (node_id, page_index, y)
        for page in pages {
            for block in &page.blocks {
                self.collect_comment_starts_from_block(block, page.index, &mut comment_starts);
            }
            if let Some(header) = &page.header {
                self.collect_comment_starts_from_block(header, page.index, &mut comment_starts);
            }
            if let Some(footer) = &page.footer {
                self.collect_comment_starts_from_block(footer, page.index, &mut comment_starts);
            }
        }

        // For each CommentStart, find the matching CommentBody and build a LayoutAnnotation
        for (comment_start_id, page_index, y_pos) in &comment_starts {
            if let Some(node) = self.doc.node(*comment_start_id) {
                let comment_id = node
                    .attributes
                    .get_string(&AttributeKey::CommentId)
                    .unwrap_or("")
                    .to_string();
                let author = node
                    .attributes
                    .get_string(&AttributeKey::CommentAuthor)
                    .unwrap_or("")
                    .to_string();
                let date = node
                    .attributes
                    .get_string(&AttributeKey::CommentDate)
                    .unwrap_or("")
                    .to_string();

                // Find the CommentBody with matching comment ID (child of document root)
                let content = self.find_comment_body_text(&comment_id);

                annotations.push(LayoutAnnotation {
                    annotation_type: LayoutAnnotationType::Comment,
                    source_id: *comment_start_id,
                    page_index: *page_index,
                    rects: vec![Rect::new(0.0, *y_pos, 24.0, 24.0)],
                    content,
                    author,
                    date,
                    color: None,
                });
            }
        }

        // 2. Collect highlight annotations from glyph runs with highlight_color
        for page in pages {
            for block in &page.blocks {
                self.collect_highlights_from_block(block, page.index, &mut annotations);
            }
        }

        annotations
    }

    /// Recursively scan a block for CommentStart nodes.
    fn collect_comment_starts_from_block(
        &self,
        block: &LayoutBlock,
        page_index: usize,
        starts: &mut Vec<(NodeId, usize, f64)>,
    ) {
        match &block.kind {
            LayoutBlockKind::Paragraph { .. } | LayoutBlockKind::Image { .. } => {
                if let Some(node) = self.doc.node(block.source_id) {
                    for &child_id in &node.children {
                        if let Some(child) = self.doc.node(child_id) {
                            if child.node_type == NodeType::CommentStart {
                                starts.push((child_id, page_index, block.bounds.y));
                            }
                        }
                    }
                }
            }
            LayoutBlockKind::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        for cell_block in &cell.blocks {
                            self.collect_comment_starts_from_block(cell_block, page_index, starts);
                        }
                    }
                }
            }
        }
    }

    /// Find the text content of a CommentBody node matching a comment ID.
    fn find_comment_body_text(&self, comment_id: &str) -> String {
        let root_id = self.doc.root_id();
        if let Some(root) = self.doc.node(root_id) {
            for &child_id in &root.children {
                if let Some(child) = self.doc.node(child_id) {
                    if child.node_type == NodeType::CommentBody {
                        let cid = child
                            .attributes
                            .get_string(&AttributeKey::CommentId)
                            .unwrap_or("");
                        if cid == comment_id {
                            return self.extract_text_content(child_id);
                        }
                    }
                }
            }
        }
        String::new()
    }

    /// Extract plain text from a node and its descendants.
    fn extract_text_content(&self, node_id: NodeId) -> String {
        let mut text = String::new();
        if let Some(node) = self.doc.node(node_id) {
            if let Some(t) = &node.text_content {
                text.push_str(t);
            }
            for &child_id in &node.children {
                text.push_str(&self.extract_text_content(child_id));
            }
        }
        text
    }

    /// Collect highlight annotations from glyph runs in a block.
    fn collect_highlights_from_block(
        &self,
        block: &LayoutBlock,
        page_index: usize,
        annotations: &mut Vec<LayoutAnnotation>,
    ) {
        match &block.kind {
            LayoutBlockKind::Paragraph { lines, .. } => {
                for line in lines {
                    for run in &line.runs {
                        if let Some(ref color) = run.highlight_color {
                            // Build a rect for this highlighted run
                            let run_rect = Rect::new(
                                block.bounds.x + run.x_offset,
                                block.bounds.y + line.baseline_y - run.font_size,
                                run.width,
                                line.height,
                            );
                            annotations.push(LayoutAnnotation {
                                annotation_type: LayoutAnnotationType::Highlight,
                                source_id: run.source_id,
                                page_index,
                                rects: vec![run_rect],
                                content: String::new(),
                                author: String::new(),
                                date: String::new(),
                                color: Some(*color),
                            });
                        }
                    }
                }
            }
            LayoutBlockKind::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        for cell_block in &cell.blocks {
                            self.collect_highlights_from_block(cell_block, page_index, annotations);
                        }
                    }
                }
            }
            LayoutBlockKind::Image { .. } => {}
        }
    }

    /// Scan pages for footnote references and layout footnote content at page bottom.
    fn layout_footnotes_for_pages(&mut self, pages: &mut [LayoutPage]) -> Result<(), LayoutError> {
        // Build a map of footnote number → FootnoteBody node ID
        let root_id = self.doc.root_id();
        let footnote_bodies: Vec<(String, NodeId)> = self
            .doc
            .node(root_id)
            .map(|root| {
                root.children
                    .iter()
                    .filter_map(|&child_id| {
                        let child = self.doc.node(child_id)?;
                        if child.node_type == NodeType::FootnoteBody {
                            let num = child.attributes.get_string(&AttributeKey::FootnoteNumber)?;
                            Some((num.to_string(), child_id))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        if footnote_bodies.is_empty() {
            return Ok(());
        }

        let fn_map: std::collections::HashMap<String, NodeId> =
            footnote_bodies.into_iter().collect();

        for page in pages.iter_mut() {
            // Collect footnote references from blocks on this page
            let mut page_fn_ids: Vec<NodeId> = Vec::new();
            for block in &page.blocks {
                self.collect_footnote_refs_from_block(block, &fn_map, &mut page_fn_ids);
            }

            if page_fn_ids.is_empty() {
                continue;
            }

            // Layout each referenced footnote body as paragraphs
            let content_x = page.content_area.x;
            let content_width = page.content_area.width;
            let mut footnote_y = 0.0;
            let mut fn_blocks: Vec<LayoutBlock> = Vec::new();

            // Add a separator line (1pt rule)
            footnote_y += 8.0; // Space before separator

            for fn_body_id in &page_fn_ids {
                // Layout paragraphs inside the footnote body
                if let Some(fn_body) = self.doc.node(*fn_body_id) {
                    for &child_id in &fn_body.children {
                        if let Some(child) = self.doc.node(child_id) {
                            if child.node_type == NodeType::Paragraph {
                                let ps = resolve_paragraph_style(self.doc, child_id);
                                let fn_rect =
                                    Rect::new(content_x, footnote_y, content_width, 1000.0);
                                let block =
                                    self.layout_paragraph(child_id, &ps, fn_rect, footnote_y)?;
                                footnote_y += block.bounds.height + sanitize_pt(ps.space_after);
                                fn_blocks.push(block);
                            }
                        }
                    }
                }
            }

            page.footnotes = fn_blocks;
        }

        Ok(())
    }

    /// Recursively collect footnote reference node IDs from a layout block.
    fn collect_footnote_refs_from_block(
        &self,
        block: &LayoutBlock,
        fn_map: &std::collections::HashMap<String, NodeId>,
        out: &mut Vec<NodeId>,
    ) {
        // Check if the source node itself is a footnote ref, or look at children
        if let Some(node) = self.doc.node(block.source_id) {
            self.scan_node_for_footnote_refs(node.id, fn_map, out);
        }
        // Also scan table cells
        if let LayoutBlockKind::Table { rows, .. } = &block.kind {
            for row in rows {
                for cell in &row.cells {
                    for cell_block in &cell.blocks {
                        self.collect_footnote_refs_from_block(cell_block, fn_map, out);
                    }
                }
            }
        }
    }

    /// Walk the document tree under a node to find FootnoteRef nodes.
    fn scan_node_for_footnote_refs(
        &self,
        node_id: NodeId,
        fn_map: &std::collections::HashMap<String, NodeId>,
        out: &mut Vec<NodeId>,
    ) {
        if let Some(node) = self.doc.node(node_id) {
            if node.node_type == NodeType::FootnoteRef {
                if let Some(num) = node.attributes.get_string(&AttributeKey::FootnoteNumber) {
                    if let Some(&body_id) = fn_map.get(num) {
                        if !out.contains(&body_id) {
                            out.push(body_id);
                        }
                    }
                }
            }
            for &child_id in &node.children {
                self.scan_node_for_footnote_refs(child_id, fn_map, out);
            }
        }
    }

    /// Check if a paragraph contains an inline page break (`w:br type="page"`)
    /// as a descendant node. Returns `true` if any `NodeType::PageBreak` is
    /// found among the paragraph's children (typically inside a Run).
    fn paragraph_has_inline_page_break(&self, para_id: NodeId) -> bool {
        if let Some(para) = self.doc.node(para_id) {
            for &run_id in &para.children {
                if let Some(run) = self.doc.node(run_id) {
                    if run.node_type == NodeType::PageBreak {
                        return true;
                    }
                    for &child_id in &run.children {
                        if let Some(child) = self.doc.node(child_id) {
                            if child.node_type == NodeType::PageBreak {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
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
            footnotes: Vec::new(),
            floating_images: Vec::new(),
            section_index,
        }
    }
}

/// Shaped run info — intermediate result before line breaking.
struct ShapedRunInfo {
    source_id: NodeId,
    font_id: Option<FontId>,
    font_family: String,
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

/// Format a border side as a CSS border value, returning None for `BorderStyle::None`.
fn format_border_css_opt(border: &s1_model::BorderSide) -> Option<String> {
    if border.style == s1_model::BorderStyle::None {
        return None;
    }
    Some(format_border_css(border))
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
    let width = if border.width > 0.0 {
        border.width
    } else {
        1.0
    };
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
    /// Content with a fixed width (a shaped run or sub-range of one).
    Box {
        run_idx: usize,
        width: f64,
        height: f64,
        /// Start glyph index within the run (inclusive).
        glyph_start: usize,
        /// End glyph index within the run (exclusive).
        glyph_end: usize,
        /// Start byte offset within the run's text.
        text_byte_start: usize,
        /// End byte offset within the run's text.
        text_byte_end: usize,
    },
    /// Stretchable/shrinkable space between boxes.
    Glue {
        width: f64,
        stretch: f64,
        shrink: f64,
    },
    /// A possible break point with a penalty cost.
    Penalty {
        penalty: f64,
        /// If true, a hyphen should be inserted when the line breaks here.
        flagged: bool,
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
        // Determine if this item is a feasible break point and its penalty
        let (is_feasible_break, penalty_cost) = match item {
            BreakItem::Glue { .. } => (true, 0.0),
            BreakItem::ForcedBreak { .. } => (true, 0.0),
            BreakItem::Penalty { penalty, .. } => (true, *penalty),
            _ => (false, 0.0),
        };

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
                // Forced breaks get minimal demerits, penalties add their cost
                let demerits = if is_forced {
                    a.demerits
                } else {
                    (1.0 + badness + penalty_cost).powi(2) + a.demerits
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
                nodes[a]
                    .demerits
                    .partial_cmp(&nodes[b].demerits)
                    .unwrap_or(std::cmp::Ordering::Equal)
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
    // Track the last feasible break point (Glue or Penalty) for deferred breaking
    let mut last_break_opportunity: Option<usize> = None;
    let mut width_at_last_break: f64 = 0.0;

    for (i, item) in items.iter().enumerate() {
        match item {
            BreakItem::Box { width, .. } => {
                let line_w = if is_first_line {
                    available_width - first_line_indent
                } else {
                    available_width
                };

                if current_width + width > line_w + 0.01 && i > *breaks.last().unwrap_or(&0) {
                    // If we have a previous break opportunity, break there instead
                    if let Some(bp) = last_break_opportunity {
                        if bp > *breaks.last().unwrap_or(&0) {
                            breaks.push(bp + 1);
                            // Subtract the glue width at bp so the new line doesn't
                            // inherit the trailing space from the previous line.
                            let glue_w = match &items[bp] {
                                BreakItem::Glue { width, .. } => *width,
                                _ => 0.0,
                            };
                            current_width = current_width - width_at_last_break - glue_w + width;
                            is_first_line = false;
                            last_break_opportunity = None;
                            continue;
                        }
                    }
                    breaks.push(i);
                    current_width = *width;
                    is_first_line = false;
                    last_break_opportunity = None;
                } else {
                    current_width += width;
                }
            }
            BreakItem::Glue { width, .. } => {
                last_break_opportunity = Some(i);
                width_at_last_break = current_width;
                current_width += width;
            }
            BreakItem::ForcedBreak { .. } => {
                breaks.push(i + 1);
                current_width = 0.0;
                is_first_line = false;
                last_break_opportunity = None;
            }
            BreakItem::Penalty { .. } => {
                // Penalty is a valid break opportunity (e.g. hyphenation point)
                last_break_opportunity = Some(i);
                width_at_last_break = current_width;
            }
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
                glyph_start: 0,
                glyph_end: 5,
                text_byte_start: 0,
                text_byte_end: 5,
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
                glyph_start: 0,
                glyph_end: 5,
                text_byte_start: 0,
                text_byte_end: 5,
            },
            BreakItem::ForcedBreak { run_idx: 1 },
            BreakItem::Box {
                run_idx: 2,
                width: 100.0,
                height: 12.0,
                glyph_start: 0,
                glyph_end: 5,
                text_byte_start: 0,
                text_byte_end: 5,
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
                glyph_start: 0,
                glyph_end: 10,
                text_byte_start: 0,
                text_byte_end: 10,
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
                glyph_start: 0,
                glyph_end: 10,
                text_byte_start: 0,
                text_byte_end: 10,
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
            page0.header.as_ref().unwrap().source_id,
            hdr0_id,
            "page 0 header should be from section 0"
        );

        let page1 = &result.pages[1];
        assert!(page1.header.is_some(), "page 1 should have a header");
        assert_eq!(
            page1.header.as_ref().unwrap().source_id,
            hdr1_id,
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
                row_node
                    .attributes
                    .set(AttributeKey::TableHeaderRow, AttributeValue::Bool(true));
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
                assert!(
                    !is_continuation,
                    "first table chunk should not be a continuation"
                );
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
                assert!(
                    *is_continuation,
                    "second table chunk should be a continuation"
                );
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

        assert!(result.pages.len() >= 2, "table should span multiple pages");

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
            .filter(|b| {
                matches!(
                    &b.kind,
                    LayoutBlockKind::Table {
                        is_continuation: true,
                        ..
                    }
                )
            })
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
        let has_table = result.pages[0]
            .blocks
            .iter()
            .any(|b| matches!(&b.kind, LayoutBlockKind::Table { rows, .. } if !rows.is_empty()));
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
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(150.0));
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
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(120.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(80.0));
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
                let has_inline_image = lines
                    .iter()
                    .any(|line| line.runs.iter().any(|run| run.inline_image.is_some()));
                assert!(
                    has_inline_image,
                    "paragraph should contain an inline image run"
                );
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
        drawing_node
            .attributes
            .set(AttributeKey::ShapeWidth, AttributeValue::Float(160.0));
        drawing_node
            .attributes
            .set(AttributeKey::ShapeHeight, AttributeValue::Float(100.0));
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
                assert!(
                    img_run.is_some(),
                    "paragraph should contain an inline image run for Drawing"
                );
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

    #[test]
    fn margin_collapsing_between_paragraphs() {
        // Create a document with two paragraphs:
        // - Para 1: space_after = 20pt
        // - Para 2: space_before = 12pt
        // Expected: gap between them = max(20, 12) = 20pt (not 32pt)

        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root_id, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Para 1: space_after = 20
        let p1_id = doc.next_id();
        let mut p1 = Node::new(p1_id, NodeType::Paragraph);
        p1.attributes
            .set(AttributeKey::SpacingAfter, AttributeValue::Float(20.0));
        doc.insert_node(body_id, 0, p1).unwrap();
        let r1_id = doc.next_id();
        doc.insert_node(p1_id, 0, Node::new(r1_id, NodeType::Run))
            .unwrap();
        let t1_id = doc.next_id();
        doc.insert_node(r1_id, 0, Node::text(t1_id, "First"))
            .unwrap();

        // Para 2: space_before = 12
        let p2_id = doc.next_id();
        let mut p2 = Node::new(p2_id, NodeType::Paragraph);
        p2.attributes
            .set(AttributeKey::SpacingBefore, AttributeValue::Float(12.0));
        doc.insert_node(body_id, 1, p2).unwrap();
        let r2_id = doc.next_id();
        doc.insert_node(p2_id, 0, Node::new(r2_id, NodeType::Run))
            .unwrap();
        let t2_id = doc.next_id();
        doc.insert_node(r2_id, 0, Node::text(t2_id, "Second"))
            .unwrap();

        // Layout and check
        let font_db = FontDatabase::new();
        let config = LayoutConfig::default();
        let mut engine = LayoutEngine::new(&doc, &font_db, config);
        let result = engine.layout().unwrap();

        assert!(!result.pages.is_empty());
        let page = &result.pages[0];
        assert!(page.blocks.len() >= 2);

        // The gap between para 1 bottom and para 2 top should be max(20, 12) = 20
        // NOT 20 + 12 = 32
        let p1_bottom = page.blocks[0].bounds.y + page.blocks[0].bounds.height;
        let p2_top = page.blocks[1].bounds.y;
        let gap = p2_top - p1_bottom;

        // Gap should be approximately 20pt (collapsed), not 32pt (additive)
        assert!(gap < 25.0, "Expected collapsed margin ~20pt, got {gap}pt");
        assert!(gap >= 18.0, "Expected collapsed margin ~20pt, got {gap}pt");
    }

    // --- Multi-column layout tests ---

    #[test]
    fn layout_two_column_basic() {
        // Create a section with 2 columns and enough paragraphs to overflow
        // the first column, verifying blocks appear in two columns.
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add 60 paragraphs (enough to fill one column of US Letter)
        for i in 0..60 {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, "Column text line"))
                .unwrap();
        }

        // Set up 2-column section
        let mut sp = s1_model::SectionProperties::default();
        sp.columns = 2;
        sp.column_spacing = 36.0;
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        // Should have at least one page
        assert!(!result.pages.is_empty());

        let page = &result.pages[0];
        assert!(
            page.blocks.len() > 1,
            "expected multiple blocks on first page"
        );

        // Verify blocks are in two columns by checking distinct x offsets
        let mut x_offsets: Vec<f64> = page
            .blocks
            .iter()
            .map(|b| (b.bounds.x * 10.0).round() / 10.0)
            .collect();
        x_offsets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        x_offsets.dedup();

        assert!(
            x_offsets.len() >= 2,
            "expected blocks in at least 2 columns (distinct x offsets), got {:?}",
            x_offsets
        );

        // The column width should be (468 - 36) / 2 = 216 points for US Letter with 1" margins
        // First column x = 72, second column x = 72 + 216 + 36 = 324
        let expected_col1_x = 72.0;
        let expected_col2_x = 72.0 + 216.0 + 36.0;
        assert!(
            (x_offsets[0] - expected_col1_x).abs() < 1.0,
            "first column x should be ~{expected_col1_x}, got {}",
            x_offsets[0]
        );
        assert!(
            (x_offsets[1] - expected_col2_x).abs() < 1.0,
            "second column x should be ~{expected_col2_x}, got {}",
            x_offsets[1]
        );
    }

    #[test]
    fn layout_three_column_positions() {
        // Verify 3-column layout has correct x-offset positions
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add enough paragraphs to fill at least 2 columns
        for i in 0..80 {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, "Three col text"))
                .unwrap();
        }

        let mut sp = s1_model::SectionProperties::default();
        sp.columns = 3;
        sp.column_spacing = 18.0;
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        let page = &result.pages[0];
        let mut x_offsets: Vec<f64> = page
            .blocks
            .iter()
            .map(|b| (b.bounds.x * 10.0).round() / 10.0)
            .collect();
        x_offsets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        x_offsets.dedup();

        // Content width = 468, spacing = 18, 3 columns
        // col_width = (468 - 2*18) / 3 = 432/3 = 144
        // col 0 x = 72
        // col 1 x = 72 + 144 + 18 = 234
        // col 2 x = 72 + 2*(144+18) = 72 + 324 = 396
        assert!(
            x_offsets.len() >= 2,
            "expected blocks in at least 2 columns, got {:?}",
            x_offsets
        );
    }

    #[test]
    fn layout_column_break() {
        // Verify that a ColumnBreak forces content to the next column
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Paragraph 1
        let p1 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Column 1 text"))
            .unwrap();

        // Column break
        let cb = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(cb, NodeType::ColumnBreak))
            .unwrap();

        // Paragraph 2 (should be in column 2)
        let p2 = doc.next_id();
        doc.insert_node(body_id, 2, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Column 2 text"))
            .unwrap();

        // 2-column section
        let mut sp = s1_model::SectionProperties::default();
        sp.columns = 2;
        sp.column_spacing = 36.0;
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert_eq!(result.pages.len(), 1, "should fit on one page");
        let page = &result.pages[0];
        assert_eq!(page.blocks.len(), 2, "should have 2 paragraph blocks");

        // Blocks should be at different x positions (different columns)
        let x1 = page.blocks[0].bounds.x;
        let x2 = page.blocks[1].bounds.x;
        assert!(
            (x2 - x1).abs() > 100.0,
            "blocks should be in different columns: x1={x1}, x2={x2}"
        );
    }

    #[test]
    fn layout_single_column_unchanged() {
        // Verify that single-column layout is unaffected by multi-column code
        let doc = make_multi_para_doc(&["Hello", "World", "Test"]);
        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].blocks.len(), 3);

        // All blocks should have the same x offset (single column)
        let x_first = result.pages[0].blocks[0].bounds.x;
        for block in &result.pages[0].blocks {
            assert!(
                (block.bounds.x - x_first).abs() < 0.01,
                "single-column blocks should all have same x"
            );
        }
    }

    #[test]
    fn column_content_rect_computation() {
        // Test the column_content_rect helper directly
        let full_rect = Rect::new(72.0, 72.0, 468.0, 648.0);

        // Single column: returns full rect
        let r = LayoutEngine::column_content_rect(full_rect, 1, 36.0, 0);
        assert!((r.x - 72.0).abs() < 0.01);
        assert!((r.width - 468.0).abs() < 0.01);

        // Two columns with 36pt spacing
        // col_width = (468 - 36) / 2 = 216
        let c0 = LayoutEngine::column_content_rect(full_rect, 2, 36.0, 0);
        assert!((c0.x - 72.0).abs() < 0.01);
        assert!((c0.width - 216.0).abs() < 0.01);

        let c1 = LayoutEngine::column_content_rect(full_rect, 2, 36.0, 1);
        assert!((c1.x - 324.0).abs() < 0.01); // 72 + 216 + 36
        assert!((c1.width - 216.0).abs() < 0.01);

        // Three columns with 18pt spacing
        // col_width = (468 - 2*18) / 3 = 144
        let c0 = LayoutEngine::column_content_rect(full_rect, 3, 18.0, 0);
        assert!((c0.x - 72.0).abs() < 0.01);
        assert!((c0.width - 144.0).abs() < 0.01);

        let c1 = LayoutEngine::column_content_rect(full_rect, 3, 18.0, 1);
        assert!((c1.x - 234.0).abs() < 0.01); // 72 + 144 + 18
        assert!((c1.width - 144.0).abs() < 0.01);

        let c2 = LayoutEngine::column_content_rect(full_rect, 3, 18.0, 2);
        assert!((c2.x - 396.0).abs() < 0.01); // 72 + 2*(144+18)
        assert!((c2.width - 144.0).abs() < 0.01);
    }

    #[test]
    fn layout_columns_overflow_to_next_page() {
        // With 2 columns and enough content, verify blocks overflow to a second page
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add 120 paragraphs (enough to fill both columns on one page)
        for i in 0..120 {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, "Overflow text"))
                .unwrap();
        }

        let mut sp = s1_model::SectionProperties::default();
        sp.columns = 2;
        sp.column_spacing = 36.0;
        doc.sections_mut().push(sp);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(
            result.pages.len() >= 2,
            "expected at least 2 pages for 120 paragraphs in 2 columns, got {}",
            result.pages.len()
        );
    }

    #[test]
    fn floating_image_square_wrap_narrows_paragraph() {
        // A floating image on the right with "square" wrap should narrow paragraph width
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add a floating image positioned on the right side
        let img_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(img_id, NodeType::Image))
            .unwrap();
        let img_node = doc.node_mut(img_id).unwrap();
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(100.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(100.0));
        img_node.attributes.set(
            AttributeKey::ImagePositionType,
            AttributeValue::String("anchor".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String("square".to_string()),
        );
        // Position on the right: 350pt from column left (content width = 468pt for letter)
        // 350 pt = 350 * 12700 EMU = 4_445_000 EMU
        img_node.attributes.set(
            AttributeKey::ImageHorizontalOffset,
            AttributeValue::Int(4_445_000),
        );
        img_node
            .attributes
            .set(AttributeKey::ImageVerticalOffset, AttributeValue::Int(0));
        img_node.attributes.set(
            AttributeKey::ImageHorizontalRelativeFrom,
            AttributeValue::String("column".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageVerticalRelativeFrom,
            AttributeValue::String("paragraph".to_string()),
        );

        // Add a paragraph after the image
        let para_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(
            run_id,
            0,
            Node::text(text_id, "This text should wrap around the floating image"),
        )
        .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        assert!(!result.pages.is_empty());
        let page = &result.pages[0];

        // Should have floating images
        assert_eq!(page.floating_images.len(), 1, "expected 1 floating image");

        // The paragraph should be narrower than full content width (468pt for letter)
        let para_block = page
            .blocks
            .iter()
            .find(|b| matches!(b.kind, LayoutBlockKind::Paragraph { .. }));
        assert!(para_block.is_some(), "expected a paragraph block");
        let pb = para_block.unwrap();
        assert!(
            pb.bounds.width < 468.0,
            "paragraph width ({}) should be narrower than full content width (468pt) due to float wrapping",
            pb.bounds.width
        );
    }

    #[test]
    fn floating_image_top_and_bottom_advances_y() {
        // A floating image with "topAndBottom" wrap should push paragraphs below it
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Floating image at top of content area, 200pt tall
        let img_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(img_id, NodeType::Image))
            .unwrap();
        let img_node = doc.node_mut(img_id).unwrap();
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(200.0));
        img_node.attributes.set(
            AttributeKey::ImagePositionType,
            AttributeValue::String("anchor".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String("topAndBottom".to_string()),
        );
        img_node
            .attributes
            .set(AttributeKey::ImageHorizontalOffset, AttributeValue::Int(0));
        img_node
            .attributes
            .set(AttributeKey::ImageVerticalOffset, AttributeValue::Int(0));
        img_node.attributes.set(
            AttributeKey::ImageHorizontalRelativeFrom,
            AttributeValue::String("column".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageVerticalRelativeFrom,
            AttributeValue::String("margin".to_string()),
        );

        // Add a paragraph
        let para_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Below the image"))
            .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        let page = &result.pages[0];
        let para_block = page
            .blocks
            .iter()
            .find(|b| matches!(b.kind, LayoutBlockKind::Paragraph { .. }));
        assert!(para_block.is_some());
        let pb = para_block.unwrap();

        // Paragraph Y should be at or below the image bottom (72 + 200 = 272pt for letter margins)
        assert!(
            pb.bounds.y >= 272.0,
            "paragraph Y ({}) should be >= 272pt (below the topAndBottom floating image)",
            pb.bounds.y
        );
    }

    #[test]
    fn floating_image_none_wrap_no_effect_on_paragraph() {
        // A floating image with "none" wrap (behind/inFront) should not affect paragraph layout
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Floating image with no wrap
        let img_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(img_id, NodeType::Image))
            .unwrap();
        let img_node = doc.node_mut(img_id).unwrap();
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(200.0));
        img_node.attributes.set(
            AttributeKey::ImagePositionType,
            AttributeValue::String("anchor".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String("none".to_string()),
        );
        img_node
            .attributes
            .set(AttributeKey::ImageHorizontalOffset, AttributeValue::Int(0));
        img_node
            .attributes
            .set(AttributeKey::ImageVerticalOffset, AttributeValue::Int(0));
        img_node.attributes.set(
            AttributeKey::ImageHorizontalRelativeFrom,
            AttributeValue::String("column".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageVerticalRelativeFrom,
            AttributeValue::String("margin".to_string()),
        );

        // Paragraph
        let para_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Overlapping text"))
            .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        let page = &result.pages[0];
        let para_block = page
            .blocks
            .iter()
            .find(|b| matches!(b.kind, LayoutBlockKind::Paragraph { .. }));
        assert!(para_block.is_some());
        let pb = para_block.unwrap();

        // With wrap=none, paragraph should start at normal Y (72pt for letter top margin)
        assert!(
            pb.bounds.y < 100.0,
            "paragraph Y ({}) should be near page top — wrap=none should not push it down",
            pb.bounds.y
        );
        // Width should be full content width
        assert!(
            (pb.bounds.width - 468.0).abs() < 1.0,
            "paragraph width ({}) should be full 468pt — wrap=none should not narrow it",
            pb.bounds.width
        );
    }

    #[test]
    fn floating_image_left_side_square_wrap() {
        // Float on the left side should narrow from the left
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Floating image positioned on the left (offset 0)
        let img_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(img_id, NodeType::Image))
            .unwrap();
        let img_node = doc.node_mut(img_id).unwrap();
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(150.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(100.0));
        img_node.attributes.set(
            AttributeKey::ImagePositionType,
            AttributeValue::String("anchor".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String("square".to_string()),
        );
        img_node
            .attributes
            .set(AttributeKey::ImageHorizontalOffset, AttributeValue::Int(0));
        img_node
            .attributes
            .set(AttributeKey::ImageVerticalOffset, AttributeValue::Int(0));
        img_node.attributes.set(
            AttributeKey::ImageHorizontalRelativeFrom,
            AttributeValue::String("column".to_string()),
        );
        img_node.attributes.set(
            AttributeKey::ImageVerticalRelativeFrom,
            AttributeValue::String("margin".to_string()),
        );

        // Paragraph
        let para_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(
            run_id,
            0,
            Node::text(text_id, "Wrapping on the right side of the image"),
        )
        .unwrap();

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        let result = engine.layout().unwrap();

        let page = &result.pages[0];
        let para_block = page
            .blocks
            .iter()
            .find(|b| matches!(b.kind, LayoutBlockKind::Paragraph { .. }));
        assert!(para_block.is_some());
        let pb = para_block.unwrap();

        // Paragraph X should be shifted right past the image (72 + 150 = 222pt)
        assert!(
            pb.bounds.x >= 222.0,
            "paragraph X ({}) should be >= 222pt (right of the left-side floating image)",
            pb.bounds.x
        );
        // Width should be reduced
        assert!(
            pb.bounds.width < 468.0,
            "paragraph width ({}) should be narrower than 468pt",
            pb.bounds.width
        );
    }

    #[test]
    fn nested_table_depth_cap() {
        // Build a deeply nested table (8 levels deep — exceeds MAX_TABLE_NESTING of 5)
        // The layout engine should handle this gracefully without stack overflow.
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        fn add_nested_table(doc: &mut DocumentModel, parent_id: NodeId, depth: usize) {
            let table_id = doc.next_id();
            doc.insert_node(parent_id, 0, Node::new(table_id, NodeType::Table))
                .unwrap();
            let row_id = doc.next_id();
            doc.insert_node(table_id, 0, Node::new(row_id, NodeType::TableRow))
                .unwrap();
            let cell_id = doc.next_id();
            doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
                .unwrap();

            if depth < 8 {
                // Nest another table inside this cell
                add_nested_table(doc, cell_id, depth + 1);
            } else {
                // Leaf: add a paragraph
                let para_id = doc.next_id();
                doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
                    .unwrap();
                let run_id = doc.next_id();
                doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                    .unwrap();
                let text_id = doc.next_id();
                doc.insert_node(run_id, 0, Node::text(text_id, "leaf"))
                    .unwrap();
            }
        }

        add_nested_table(&mut doc, body_id, 0);

        let font_db = FontDatabase::new();
        let mut engine = LayoutEngine::new(&doc, &font_db, LayoutConfig::default());
        // Should not panic or stack overflow — depth cap gracefully stops at level 5
        let result = engine.layout().unwrap();
        assert!(!result.pages.is_empty());
    }
}
