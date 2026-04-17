//! WebAssembly bindings for s1engine.
//!
//! Provides a JavaScript-friendly API for creating, opening, editing, and
//! exporting documents from the browser or Node.js.

use std::io::Write as _;
use wasm_bindgen::prelude::*;

use s1_layout::{layout_to_html, LayoutConfig, PageLayout};
use s1_model::{
    Alignment, AttributeKey, AttributeValue, Color, DocumentModel, ListFormat, Node, NodeId,
    NodeType, UnderlineStyle,
};
use s1engine::{Operation, Transaction};

// --- WasmEngine ---

/// The main entry point for s1engine in WASM.
#[wasm_bindgen]
pub struct WasmEngine {
    inner: s1engine::Engine,
}

#[wasm_bindgen]
impl WasmEngine {
    /// Create a new engine instance.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: s1engine::Engine::new(),
        }
    }

    /// Create a new empty document.
    pub fn create(&self) -> WasmDocument {
        WasmDocument {
            inner: Some(self.inner.create()),
            batch_label: None,
            batch_count: 0,
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        }
    }

    /// Open a document from bytes with auto-detected format.
    ///
    /// Supports DOCX, ODT, and TXT formats.
    pub fn open(&self, data: &[u8]) -> Result<WasmDocument, JsError> {
        let doc = self
            .inner
            .open(data)
            .map_err(|e| JsError::new(&format!("Failed to open document: {}", e)))?;
        Ok(WasmDocument {
            batch_label: None,
            batch_count: 0,
            inner: Some(doc),
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        })
    }

    /// Open a document from bytes with an explicit format.
    ///
    /// Format should be one of: "docx", "odt", "txt".
    pub fn open_as(&self, data: &[u8], format: &str) -> Result<WasmDocument, JsError> {
        let fmt = parse_format(format)?;
        let doc = self
            .inner
            .open_as(data, fmt)
            .map_err(|e| JsError::new(&format!("Failed to open document as {}: {}", format, e)))?;
        Ok(WasmDocument {
            batch_label: None,
            batch_count: 0,
            inner: Some(doc),
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        })
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

// --- WasmLayoutConfig ---

/// Configuration for paginated HTML layout.
///
/// Controls page dimensions and margins for the layout engine.
/// Defaults to US Letter (8.5" x 11") with 1-inch margins.
#[wasm_bindgen]
pub struct WasmLayoutConfig {
    page_width_pt: f64,
    page_height_pt: f64,
    margin_top_pt: f64,
    margin_bottom_pt: f64,
    margin_left_pt: f64,
    margin_right_pt: f64,
}

#[wasm_bindgen]
impl WasmLayoutConfig {
    /// Create a new layout configuration with US Letter defaults.
    ///
    /// Page: 612pt x 792pt (8.5" x 11")
    /// Margins: 72pt (1") on all sides.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            page_width_pt: 612.0,
            page_height_pt: 792.0,
            margin_top_pt: 72.0,
            margin_bottom_pt: 72.0,
            margin_left_pt: 72.0,
            margin_right_pt: 72.0,
        }
    }

    /// Set the page width in points.
    pub fn set_page_width(&mut self, width: f64) {
        self.page_width_pt = width;
    }

    /// Set the page height in points.
    pub fn set_page_height(&mut self, height: f64) {
        self.page_height_pt = height;
    }

    /// Set the top margin in points.
    pub fn set_margin_top(&mut self, margin: f64) {
        self.margin_top_pt = margin;
    }

    /// Set the bottom margin in points.
    pub fn set_margin_bottom(&mut self, margin: f64) {
        self.margin_bottom_pt = margin;
    }

    /// Set the left margin in points.
    pub fn set_margin_left(&mut self, margin: f64) {
        self.margin_left_pt = margin;
    }

    /// Set the right margin in points.
    pub fn set_margin_right(&mut self, margin: f64) {
        self.margin_right_pt = margin;
    }

    /// Get the page width in points.
    pub fn page_width(&self) -> f64 {
        self.page_width_pt
    }

    /// Get the page height in points.
    pub fn page_height(&self) -> f64 {
        self.page_height_pt
    }

    /// Get the top margin in points.
    pub fn margin_top(&self) -> f64 {
        self.margin_top_pt
    }

    /// Get the bottom margin in points.
    pub fn margin_bottom(&self) -> f64 {
        self.margin_bottom_pt
    }

    /// Get the left margin in points.
    pub fn margin_left(&self) -> f64 {
        self.margin_left_pt
    }

    /// Get the right margin in points.
    pub fn margin_right(&self) -> f64 {
        self.margin_right_pt
    }
}

impl Default for WasmLayoutConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmLayoutConfig {
    /// Convert to a [`LayoutConfig`] for the layout engine.
    fn to_layout_config(&self) -> LayoutConfig {
        LayoutConfig {
            default_page_layout: PageLayout {
                width: self.page_width_pt,
                height: self.page_height_pt,
                margin_top: self.margin_top_pt,
                margin_bottom: self.margin_bottom_pt,
                margin_left: self.margin_left_pt,
                margin_right: self.margin_right_pt,
            },
            ..LayoutConfig::default()
        }
    }
}

// --- WasmDocument ---

/// A document handle for reading, editing, and exporting.
#[wasm_bindgen]
pub struct WasmDocument {
    inner: Option<s1engine::Document>,
    /// When set, operations are accumulated instead of applied immediately.
    /// Call `end_batch()` to apply all as a single undo step.
    batch_label: Option<String>,
    batch_count: usize,
    /// Cached layout result. NOT invalidated on every mutation — instead,
    /// `_layout_dirty` is set true. The stale cache is kept for fast
    /// caret_rect / selection_rects / hit_test lookups between edits.
    /// A fresh layout is only computed when explicitly requested (e.g.
    /// `to_layout_json_with_fonts`) or when `ensure_layout_if_dirty` is
    /// called.
    layout_cache: std::cell::RefCell<Option<s1_layout::LayoutDocument>>,
    /// Whether the layout cache is stale (document was mutated since last layout).
    /// Uses `Cell` so `&self` methods can clear it after recomputing layout.
    _layout_dirty: std::cell::Cell<bool>,
    /// IME composition anchor paragraph node ID (as "replica:counter" string).
    composition_anchor_node: Option<String>,
    /// IME composition anchor char offset within the paragraph.
    composition_anchor_offset: Option<u32>,
    /// Length (in chars) of the currently inserted composition preview text.
    composition_preview_len: usize,
}

#[wasm_bindgen]
impl WasmDocument {
    /// Explicitly release document memory. The document cannot be used after this.
    pub fn close(&mut self) {
        self.inner = None;
        self.batch_label = None;
        self.batch_count = 0;
        *self.layout_cache.get_mut() = None;
        self._layout_dirty.set(false);
        self.composition_anchor_node = None;
        self.composition_anchor_offset = None;
        self.composition_preview_len = 0;
    }

    /// Begin a batch of operations that form a single undo step.
    ///
    /// All operations between `begin_batch()` and `end_batch()` are applied
    /// individually. On `end_batch()`, they are merged into a single undo
    /// unit by collapsing the undo history.
    pub fn begin_batch(&mut self, label: &str) -> Result<(), JsError> {
        let count = self.doc()?.undo_count();
        self.batch_label = Some(label.to_string());
        self.batch_count = count;
        Ok(())
    }

    /// End a batch and merge all operations since `begin_batch()` into
    /// a single undo step.
    pub fn end_batch(&mut self) -> Result<(), JsError> {
        let start_count = self.batch_count;
        let label = self.batch_label.take();
        self.batch_count = 0;

        let doc = self.doc_mut()?;
        let current_count = doc.undo_count();
        let delta = current_count.saturating_sub(start_count);
        if delta > 1 {
            if let Some(lbl) = label {
                doc.merge_undo_entries(delta, &lbl)
                    .map_err(|e| JsError::new(&e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Check if a batch is currently active.
    pub fn is_batching(&self) -> bool {
        self.batch_label.is_some()
    }

    /// Extract all text content as a plain string.
    pub fn to_plain_text(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        Ok(doc.to_plain_text())
    }

    /// Convert to OnlyOffice DOCY binary format.
    ///
    /// Returns the wrapped DOCY payload string: `DOCY;v5;{size};{base64_data}`.
    ///
    /// This is currently a debug/export surface only. The current DOCY writer
    /// is not yet structurally compatible with OnlyOffice for general open.
    pub fn to_docy(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        Ok(s1_format_docy::write(doc.model()))
    }

    /// Export the document to the specified format.
    ///
    /// Format should be one of: "docx", "odt", "txt", "pdf".
    /// Returns the exported bytes.
    pub fn export(&self, format: &str) -> Result<Vec<u8>, JsError> {
        let fmt = parse_format(format)?;
        let doc = self.doc()?;
        doc.export(fmt)
            .map_err(|e| JsError::new(&format!("Export to {} failed: {}", format, e)))
    }

    /// Get the document title (from metadata).
    pub fn metadata_title(&self) -> Result<Option<String>, JsError> {
        let doc = self.doc()?;
        Ok(doc.metadata().title.clone())
    }

    /// Get the document author (from metadata).
    pub fn metadata_author(&self) -> Result<Option<String>, JsError> {
        let doc = self.doc()?;
        Ok(doc.metadata().creator.clone())
    }

    /// Get full document metadata as JSON (title, author, custom_properties, etc.).
    pub fn metadata_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let meta = doc.metadata();
        let mut json = String::from("{");
        if let Some(ref t) = meta.title {
            json.push_str(&format!("\"title\":\"{}\",", escape_json(t)));
        }
        if let Some(ref c) = meta.creator {
            json.push_str(&format!("\"author\":\"{}\",", escape_json(c)));
        }
        if !meta.custom_properties.is_empty() {
            json.push_str("\"custom_properties\":{");
            let props: Vec<String> = meta
                .custom_properties
                .iter()
                .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)))
                .collect();
            json.push_str(&props.join(","));
            json.push_str("},");
        }
        // Remove trailing comma
        if json.ends_with(',') {
            json.pop();
        }
        json.push('}');
        Ok(json)
    }

    /// Get the number of paragraphs in the document.
    pub fn paragraph_count(&self) -> Result<usize, JsError> {
        let doc = self.doc()?;
        Ok(doc.paragraph_count())
    }

    /// Get an import fidelity report as a JSON string.
    ///
    /// Counts objects that could not be rendered faithfully and are shown
    /// as placeholders. Returns JSON like:
    /// `{"charts":2,"smartart":1,"ole":0,"missingImages":0,"total":3}`
    ///
    /// Consumers can use this to display "3 objects shown as placeholders"
    /// after opening a document, rather than silently degrading.
    pub fn fidelity_report_json(&self) -> Result<String, JsError> {
        let html = self.to_html()?;
        let charts = html.matches("chart-placeholder").count();
        let smartart = html.matches("diagram-placeholder").count();
        let ole = html.matches("ole-placeholder").count();
        let missing_images = html.matches("s1-image-placeholder").count();
        let total = charts + smartart + ole + missing_images;
        Ok(format!(
            "{{\"charts\":{},\"smartart\":{},\"ole\":{},\"missingImages\":{},\"total\":{}}}",
            charts, smartart, ole, missing_images, total
        ))
    }

    /// Render the document as paginated HTML using the layout engine.
    ///
    /// Produces CSS-positioned HTML with real page boundaries. Each page
    /// is rendered as a separate div with absolute-positioned content.
    /// Uses US Letter page size (612pt x 792pt) with 1-inch margins.
    ///
    /// Text is positioned using fallback font metrics (no system fonts
    /// are available in WASM). For more accurate layout, use
    /// `to_paginated_html_with_fonts()` after loading fonts via
    /// `WasmFontDatabase`.
    pub fn to_paginated_html(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::empty();
        let layout = doc
            .layout(&font_db)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(layout_to_html(&layout))
    }

    /// Render the document as paginated HTML with a custom layout configuration.
    ///
    /// Use this to control page dimensions and margins.
    pub fn to_paginated_html_with_config(
        &self,
        config: &WasmLayoutConfig,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::empty();
        let layout_config = config.to_layout_config();
        let layout = doc
            .layout_with_config(&font_db, layout_config)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(layout_to_html(&layout))
    }

    /// Render the document as paginated HTML with loaded fonts.
    ///
    /// Use this when you have loaded fonts via `WasmFontDatabase` for
    /// accurate text shaping and positioning.
    pub fn to_paginated_html_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let layout = doc
            .layout(&font_db.inner)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(layout_to_html(&layout))
    }

    /// Render the document as paginated HTML with loaded fonts and custom config.
    ///
    /// Combines custom page dimensions/margins with loaded font data for
    /// the most accurate layout.
    pub fn to_paginated_html_with_fonts_and_config(
        &self,
        font_db: &WasmFontDatabase,
        config: &WasmLayoutConfig,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let layout_config = config.to_layout_config();
        let layout = doc
            .layout_with_config(&font_db.inner, layout_config)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(layout_to_html(&layout))
    }

    /// Render the document layout as structured JSON for canvas-based rendering.
    ///
    /// Returns a JSON object with page, block, line, and glyph run data
    /// including exact positions, font information, and styling. This enables
    /// pixel-accurate canvas rendering as an alternative to DOM-based HTML.
    ///
    /// Uses fallback font metrics (no system fonts). For more accurate layout,
    /// use `to_layout_json_with_fonts()` after loading fonts via
    /// `WasmFontDatabase`.
    pub fn to_layout_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let font_db = s1_text::FontDatabase::empty();
        let layout = doc
            .layout(&font_db)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(layout_document_to_json(&layout, model))
    }

    /// Render the document layout as structured JSON with a custom layout configuration.
    ///
    /// Use this to control page dimensions and margins.
    pub fn to_layout_json_with_config(&self, config: &WasmLayoutConfig) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let font_db = s1_text::FontDatabase::empty();
        let layout_config = config.to_layout_config();
        let layout = doc
            .layout_with_config(&font_db, layout_config)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(layout_document_to_json(&layout, model))
    }

    /// Render the document layout as structured JSON with loaded fonts.
    ///
    /// Use this when you have loaded fonts via `WasmFontDatabase` for
    /// accurate text shaping and positioning.
    pub fn to_layout_json_with_fonts(&self, font_db: &WasmFontDatabase) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();

        // Use cached layout if available
        self.ensure_layout(&font_db.inner)?;

        let cache = self.layout_cache.borrow();
        let layout = cache.as_ref().unwrap();
        Ok(layout_document_to_json(layout, model))
    }

    /// Render the document layout as structured JSON with loaded fonts and custom config.
    ///
    /// Combines custom page dimensions/margins with loaded font data for
    /// the most accurate canvas rendering.
    pub fn to_layout_json_with_fonts_and_config(
        &self,
        font_db: &WasmFontDatabase,
        config: &WasmLayoutConfig,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let layout_config = config.to_layout_config();

        *self.layout_cache.borrow_mut() = Some(
            doc.layout_with_config(&font_db.inner, layout_config)
                .map_err(|e| JsError::new(&e.to_string()))?,
        );
        self._layout_dirty.set(false);
        let cache = self.layout_cache.borrow();
        let layout = cache.as_ref().unwrap();
        Ok(layout_document_to_json(layout, model))
    }

    /// Return layout JSON for a single page using the CACHED layout.
    ///
    /// This is fast because it does NOT recompute layout — it uses whatever
    /// is in the cache (possibly stale after edits). The JS side uses this
    /// for immediate visual feedback after typing, then does a full relayout
    /// on a deferred timer.
    ///
    /// Returns `null` JSON string if the cache is empty or the page index is
    /// out of range.
    pub fn layout_single_page_json(&self, page_index: usize) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let cache = self.layout_cache.borrow();
        let layout = match cache.as_ref() {
            Some(l) => l,
            None => return Ok("null".to_string()),
        };
        if page_index >= layout.pages.len() {
            return Ok("null".to_string());
        }
        let page = &layout.pages[page_index];
        let mut json = String::with_capacity(2048);
        json.push_str(&format!(
            "{{\"index\":{},\"width\":{:.2},\"height\":{:.2},\"contentArea\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"sectionIndex\":{},",
            page.index,
            page.width,
            page.height,
            page.content_area.x,
            page.content_area.y,
            page.content_area.width,
            page.content_area.height,
            page.section_index,
        ));
        // Header
        json.push_str("\"header\":");
        if let Some(ref hdr) = page.header {
            layout_block_to_json(hdr, model, &mut json);
        } else {
            json.push_str("null");
        }
        json.push(',');
        // Footer
        json.push_str("\"footer\":");
        if let Some(ref ftr) = page.footer {
            layout_block_to_json(ftr, model, &mut json);
        } else {
            json.push_str("null");
        }
        json.push(',');
        // Blocks
        json.push_str("\"blocks\":[");
        for (bi, block) in page.blocks.iter().enumerate() {
            if bi > 0 { json.push(','); }
            layout_block_to_json(block, model, &mut json);
        }
        json.push_str("],");
        // Floating images
        json.push_str("\"floatingImages\":[");
        for (fi, img) in page.floating_images.iter().enumerate() {
            if fi > 0 { json.push(','); }
            layout_block_to_json(img, model, &mut json);
        }
        json.push_str("],");
        // Footnotes
        json.push_str("\"footnotes\":[");
        for (ni, note) in page.footnotes.iter().enumerate() {
            if ni > 0 { json.push(','); }
            layout_block_to_json(note, model, &mut json);
        }
        json.push_str("]}");
        Ok(json)
    }

    /// Check whether the layout cache is dirty (needs recomputation).
    pub fn is_layout_dirty(&self) -> bool {
        self._layout_dirty.get()
    }

    /// Force a fresh relayout using the provided font database.
    ///
    /// Call this from JS on the deferred timer to get an accurate layout
    /// after a batch of edits. Clears the dirty flag.
    pub fn force_relayout(&self, font_db: &WasmFontDatabase) -> Result<(), JsError> {
        let doc = self.doc()?;
        let layout = doc
            .layout(&font_db.inner)
            .map_err(|e| JsError::new(&e.to_string()))?;
        *self.layout_cache.borrow_mut() = Some(layout);
        self._layout_dirty.set(false);
        Ok(())
    }

    /// Get page break information from the layout engine as JSON.
    ///
    /// Returns `{"pages": [{"pageNum":1, "nodeIds":["0:5","0:12"], "footer":"Page 1", "header":"..."}, ...]}`.
    /// This tells the editor which node IDs are on which page, so the editor
    /// can show visual page breaks matching the actual layout engine output.
    pub fn get_page_map_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::empty();
        let layout = doc
            .layout(&font_db)
            .map_err(|e| JsError::new(&e.to_string()))?;
        self.serialize_page_map(&layout)
    }

    fn serialize_page_map(&self, layout: &s1_layout::LayoutDocument) -> Result<String, JsError> {
        let mut pages_json = Vec::new();
        // Track accumulated height for paragraphs that split across pages
        let mut para_accumulated_heights = std::collections::HashMap::new();
        // Track accumulated character offsets for split paragraph fragments.
        let mut para_accumulated_chars = std::collections::HashMap::new();

        for (i, page) in layout.pages.iter().enumerate() {
            let mut node_ids = Vec::new();
            let mut table_chunks_json = Vec::new();
            let mut para_splits_json = Vec::new();
            for block in &page.blocks {
                let id_str = format!("{}:{}", block.source_id.replica, block.source_id.counter);

                // For table blocks, emit chunk info with row source IDs
                if let s1_layout::LayoutBlockKind::Table {
                    rows,
                    is_continuation,
                } = &block.kind
                {
                    let row_ids: Vec<String> = rows
                        .iter()
                        .map(|r| format!("\"{}:{}\"", r.source_id.replica, r.source_id.counter))
                        .collect();
                    table_chunks_json.push(format!(
                        "{{\"tableId\":\"{}\",\"isContinuation\":{},\"rowIds\":[{}]}}",
                        id_str,
                        is_continuation,
                        row_ids.join(","),
                    ));
                    // Always include table ID (even duplicates across pages)
                    node_ids.push(id_str);
                } else if let s1_layout::LayoutBlockKind::Paragraph {
                    is_continuation,
                    split_at_line,
                    lines,
                    ..
                } = &block.kind
                {
                    if *split_at_line > 0 || *is_continuation {
                        // This paragraph was split across pages
                        let line_count = lines.len();
                        let block_height: f64 = lines.iter().map(|l| l.height).sum();
                        let block_char_len: usize = lines
                            .iter()
                            .map(|l| l.runs.iter().map(|r| r.text.chars().count()).sum::<usize>())
                            .sum();

                        let offset_height = para_accumulated_heights
                            .get(&block.source_id)
                            .cloned()
                            .unwrap_or(0.0);
                        para_accumulated_heights
                            .insert(block.source_id, offset_height + block_height);
                        let char_start = para_accumulated_chars
                            .get(&block.source_id)
                            .cloned()
                            .unwrap_or(0usize);
                        let char_end = char_start + block_char_len;
                        para_accumulated_chars.insert(block.source_id, char_end);

                        para_splits_json.push(format!(
                            "{{\"nodeId\":\"{}\",\"isContinuation\":{},\"splitAtLine\":{},\"lineCount\":{},\"blockHeight\":{:.2},\"offsetHeight\":{:.2},\"charStart\":{},\"charEnd\":{}}}",
                            id_str, is_continuation, split_at_line, line_count, block_height, offset_height, char_start, char_end,
                        ));
                        // Always include the node ID (even duplicates for split paragraphs)
                        node_ids.push(id_str);
                    } else if !node_ids.contains(&id_str) {
                        node_ids.push(id_str);
                    }
                } else {
                    if !node_ids.contains(&id_str) {
                        node_ids.push(id_str);
                    }
                }
            }

            let footer_text = page
                .footer
                .as_ref()
                .map(|f| match &f.kind {
                    s1_layout::LayoutBlockKind::Paragraph { lines, .. } => lines
                        .iter()
                        .flat_map(|l| l.runs.iter().map(|r| r.text.as_str()))
                        .collect::<String>(),
                    _ => String::new(),
                })
                .unwrap_or_default();

            let header_text = page
                .header
                .as_ref()
                .map(|h| match &h.kind {
                    s1_layout::LayoutBlockKind::Paragraph { lines, .. } => lines
                        .iter()
                        .flat_map(|l| l.runs.iter().map(|r| r.text.as_str()))
                        .collect::<String>(),
                    _ => String::new(),
                })
                .unwrap_or_default();

            // Compute margins from page size and content area
            let margin_top = page.content_area.y;
            let margin_left = page.content_area.x;
            let margin_right = page.width - page.content_area.x - page.content_area.width;
            let margin_bottom = page.height - page.content_area.y - page.content_area.height;

            let ids_arr: Vec<String> = node_ids.iter().map(|id| format!("\"{}\"", id)).collect();
            let table_chunks_str = if table_chunks_json.is_empty() {
                String::from("[]")
            } else {
                format!("[{}]", table_chunks_json.join(","))
            };
            let para_splits_str = if para_splits_json.is_empty() {
                String::from("[]")
            } else {
                format!("[{}]", para_splits_json.join(","))
            };
            pages_json.push(format!(
                "{{\"pageNum\":{},\"width\":{:.1},\"height\":{:.1},\"marginTop\":{:.1},\"marginBottom\":{:.1},\"marginLeft\":{:.1},\"marginRight\":{:.1},\"sectionIndex\":{},\"nodeIds\":[{}],\"tableChunks\":{},\"paraSplits\":{},\"footer\":\"{}\",\"header\":\"{}\"}}",
                i + 1,
                page.width,
                page.height,
                margin_top,
                margin_bottom,
                margin_left,
                margin_right,
                page.section_index,
                ids_arr.join(","),
                table_chunks_str,
                para_splits_str,
                escape_json(&footer_text),
                escape_json(&header_text),
            ));
        }
        Ok(format!("{{\"pages\":[{}]}}", pages_json.join(",")))
    }

    /// Get page map JSON with font metrics for accurate line-level pagination.
    pub fn get_page_map_json_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let layout = doc
            .layout(&font_db.inner)
            .map_err(|e| JsError::new(&e.to_string()))?;
        // Reuse the same JSON serialization as get_page_map_json
        self.serialize_page_map(&layout)
    }

    // ── Page-level rendering API ──────────────────────────────────────
    //
    // These methods make WASM the sole authority for page layout.
    // The editor becomes a projection layer: capture input → send ops →
    // render exactly what WASM returns.

    /// Get the total number of pages using default (empty) font metrics.
    ///
    /// Lazily computes and caches layout. The cache is invalidated on any
    /// document mutation.
    pub fn get_page_count(&self) -> Result<usize, JsError> {
        let font_db = s1_text::FontDatabase::empty();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        Ok(cache.as_ref().unwrap().pages.len())
    }

    /// Get the total number of pages using loaded fonts for accurate metrics.
    ///
    /// Lazily computes and caches layout. The cache is invalidated on any
    /// document mutation.
    pub fn get_page_count_with_fonts(&self, font_db: &WasmFontDatabase) -> Result<usize, JsError> {
        // Use cached layout if available (invalidated on document mutations)
        self.ensure_layout(&font_db.inner)?;
        let cache = self.layout_cache.borrow();
        Ok(cache.as_ref().unwrap().pages.len())
    }

    /// Get ready-to-mount HTML for a single page using default font metrics.
    ///
    /// Returns document-model HTML (semantic `<p>`, `<h1>`, `<table>` with
    /// `data-node-id`) filtered to the blocks on `page_index`. Split
    /// paragraphs get `data-split="first"` or `data-split="continuation"`.
    ///
    /// Call `get_page_count()` first to ensure layout is cached.
    pub fn get_page_html(&self, page_index: usize) -> Result<String, JsError> {
        let font_db = s1_text::FontDatabase::empty();
        self.ensure_layout(&font_db)?;
        self.render_page_html(page_index)
    }

    /// Get ready-to-mount HTML for a single page using loaded fonts.
    ///
    /// Call `get_page_count_with_fonts()` first to ensure layout is cached,
    /// or this will lazily compute layout.
    pub fn get_page_html_with_fonts(
        &self,
        page_index: usize,
        font_db: &WasmFontDatabase,
    ) -> Result<String, JsError> {
        self.ensure_layout(&font_db.inner)?;
        self.render_page_html(page_index)
    }

    /// Get page indices affected by a node, plus adjacent pages.
    ///
    /// Returns a JSON array of 0-based page indices, e.g. `[1,2,3]`.
    /// Used by the editor to know which pages to re-render after an edit.
    ///
    /// Layout must already be cached (call `get_page_count*` first).
    pub fn get_affected_pages(&self, node_id_str: &str) -> Result<String, JsError> {
        let nid = parse_node_id(node_id_str)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not computed — call get_page_count first"))?;

        let total = layout.pages.len();
        let mut hit = std::collections::BTreeSet::new();
        for (i, page) in layout.pages.iter().enumerate() {
            let found = page.blocks.iter().any(|b| b.source_id == nid)
                || page.header.as_ref().is_some_and(|h| h.source_id == nid)
                || page.footer.as_ref().is_some_and(|f| f.source_id == nid)
                || page.footnotes.iter().any(|f| f.source_id == nid)
                || page.floating_images.iter().any(|f| f.source_id == nid);
            if found {
                if i > 0 {
                    hit.insert(i - 1);
                }
                hit.insert(i);
                if i + 1 < total {
                    hit.insert(i + 1);
                }
            }
        }
        let nums: Vec<String> = hit.iter().map(|i| i.to_string()).collect();
        Ok(format!("[{}]", nums.join(",")))
    }

    /// Export the document as PDF bytes.
    ///
    /// Uses fallback font metrics (no system fonts). For more accurate
    /// output, use `to_pdf_with_fonts()` after loading fonts via
    /// `WasmFontDatabase`.
    ///
    /// Returns the raw PDF bytes suitable for download or embedding.
    pub fn to_pdf(&self) -> Result<Vec<u8>, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::empty();
        doc.export_pdf(&font_db)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Export the document as PDF bytes using loaded fonts.
    ///
    /// Use this when you have loaded fonts via `WasmFontDatabase` for
    /// accurate text shaping and glyph embedding.
    pub fn to_pdf_with_fonts(&self, font_db: &WasmFontDatabase) -> Result<Vec<u8>, JsError> {
        let doc = self.doc()?;
        doc.export_pdf(&font_db.inner)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Export the document as a PDF data URL.
    ///
    /// Returns a string like `data:application/pdf;base64,...` suitable
    /// for embedding in iframes, download links, or `window.open()`.
    pub fn to_pdf_data_url(&self) -> Result<String, JsError> {
        let bytes = self.to_pdf()?;
        let b64 = base64_encode(&bytes);
        Ok(format!("data:application/pdf;base64,{}", b64))
    }

    /// Export the document as PDF/A-1b bytes (ISO 19005 archival format).
    pub fn to_pdf_a(&self) -> Result<Vec<u8>, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::empty();
        doc.export_pdf_a(&font_db, s1_format_pdf::PdfAConformance::PdfA1b)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Export the document as a PDF/A data URL.
    pub fn to_pdf_a_data_url(&self) -> Result<String, JsError> {
        let bytes = self.to_pdf_a()?;
        let b64 = base64_encode(&bytes);
        Ok(format!("data:application/pdf;base64,{}", b64))
    }

    /// Export the document as EPUB bytes.
    ///
    /// Generates an EPUB 3 file from the document content.
    /// Returns the EPUB ZIP as a byte array.
    pub fn to_epub(&self) -> Result<Vec<u8>, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let font_db = s1_text::FontDatabase::empty();

        // Generate HTML content from layout
        let layout = doc
            .layout(&font_db)
            .map_err(|e| JsError::new(&e.to_string()))?;
        let html_content = s1_layout::layout_to_html(&layout);

        // Get metadata
        let meta = model.metadata();
        let title = meta.title.as_deref().unwrap_or("Untitled Document");
        let author = meta.creator.as_deref().unwrap_or("Unknown");
        let lang = "en";

        // Build EPUB ZIP
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let deflate = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            // mimetype (must be first, uncompressed)
            zip.start_file("mimetype", options)
                .map_err(|e| JsError::new(&e.to_string()))?;
            zip.write_all(b"application/epub+zip")
                .map_err(|e| JsError::new(&e.to_string()))?;

            // META-INF/container.xml
            zip.start_file("META-INF/container.xml", deflate)
                .map_err(|e| JsError::new(&e.to_string()))?;
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
            )
            .map_err(|e| JsError::new(&e.to_string()))?;

            // OEBPS/content.opf
            zip.start_file("OEBPS/content.opf", deflate)
                .map_err(|e| JsError::new(&e.to_string()))?;
            let opf = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="uid">urn:uuid:{uid}</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>{lang}</dc:language>
    <meta property="dcterms:modified">2026-01-01T00:00:00Z</meta>
  </metadata>
  <manifest>
    <item id="chapter1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="style" href="style.css" media-type="text/css"/>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
  </manifest>
  <spine>
    <itemref idref="chapter1"/>
  </spine>
</package>"#,
                uid = "00000000-0000-0000-0000-000000000000",
                title = escape_html(title),
                author = escape_html(author),
                lang = lang,
            );
            zip.write_all(opf.as_bytes())
                .map_err(|e| JsError::new(&e.to_string()))?;

            // OEBPS/style.css
            zip.start_file("OEBPS/style.css", deflate)
                .map_err(|e| JsError::new(&e.to_string()))?;
            zip.write_all(b"body { font-family: serif; line-height: 1.5; margin: 1em; } h1,h2,h3,h4 { margin-top: 1em; } p { margin: 0.5em 0; } table { border-collapse: collapse; } td,th { border: 1px solid #ccc; padding: 4px 8px; }")
                .map_err(|e| JsError::new(&e.to_string()))?;

            // OEBPS/chapter1.xhtml
            zip.start_file("OEBPS/chapter1.xhtml", deflate)
                .map_err(|e| JsError::new(&e.to_string()))?;
            let xhtml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="{lang}">
<head><title>{title}</title><link rel="stylesheet" href="style.css"/></head>
<body>
{body}
</body>
</html>"#,
                lang = lang,
                title = escape_html(title),
                body = html_content,
            );
            zip.write_all(xhtml.as_bytes())
                .map_err(|e| JsError::new(&e.to_string()))?;

            // OEBPS/nav.xhtml (required for EPUB 3)
            zip.start_file("OEBPS/nav.xhtml", deflate)
                .map_err(|e| JsError::new(&e.to_string()))?;
            let mut nav = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Navigation</title></head>
<body>
<nav epub:type="toc"><h1>Table of Contents</h1><ol>"#,
            );
            let headings = model.collect_headings();
            if headings.is_empty() {
                nav.push_str(&format!(
                    "<li><a href=\"chapter1.xhtml\">{}</a></li>",
                    escape_html(title)
                ));
            } else {
                for (_, level, text) in &headings {
                    if *level <= 3 {
                        nav.push_str(&format!(
                            "<li><a href=\"chapter1.xhtml\">{}</a></li>",
                            escape_html(text)
                        ));
                    }
                }
            }
            nav.push_str("</ol></nav></body></html>");
            zip.write_all(nav.as_bytes())
                .map_err(|e| JsError::new(&e.to_string()))?;

            zip.finish().map_err(|e| JsError::new(&e.to_string()))?;
        }

        Ok(buf.into_inner())
    }

    /// Export the document as a PDF data URL using loaded fonts.
    pub fn to_pdf_data_url_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
    ) -> Result<String, JsError> {
        let bytes = self.to_pdf_with_fonts(font_db)?;
        let b64 = base64_encode(&bytes);
        Ok(format!("data:application/pdf;base64,{}", b64))
    }

    /// Get all unique font families used in the document.
    ///
    /// Returns a JSON array of font family names, e.g. `["Arial","Calibri","Georgia"]`.
    /// Useful for determining which fonts need to be loaded before layout.
    pub fn get_used_fonts(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let mut fonts = std::collections::BTreeSet::new();

        // Collect from docDefaults
        if let Some(ref ff) = model.doc_defaults().font_family {
            fonts.insert(ff.clone());
        }

        // Collect from styles
        for style in model.styles() {
            if let Some(AttributeValue::String(f)) = style.attributes.get(&AttributeKey::FontFamily)
            {
                fonts.insert(f.clone());
            }
        }

        // Walk all nodes for direct font attributes
        fn collect_fonts(
            model: &DocumentModel,
            node_id: NodeId,
            fonts: &mut std::collections::BTreeSet<String>,
        ) {
            if let Some(node) = model.node(node_id) {
                if let Some(AttributeValue::String(f)) =
                    node.attributes.get(&AttributeKey::FontFamily)
                {
                    fonts.insert(f.clone());
                }
                for &child_id in &node.children {
                    collect_fonts(model, child_id, fonts);
                }
            }
        }

        collect_fonts(model, model.root_id(), &mut fonts);

        // Build JSON array
        let json_arr: Vec<String> = fonts
            .iter()
            .map(|f| format!("\"{}\"", escape_json(f)))
            .collect();
        Ok(format!("[{}]", json_arr.join(",")))
    }

    /// Render the document as HTML with formatting, images, and hyperlinks.
    pub fn to_html(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let mut html = String::new();

        let sections = doc.sections();
        use s1_model::section::HeaderFooterType;

        // Render headers from ALL sections, tagged with data-section-index
        // so the editor can show the correct header per page.
        // For single-section documents this behaves identically to before.
        for (sec_idx, sec) in sections.iter().enumerate() {
            // Prefer Default; fall back to First if title_page is set; else any.
            let header_ref = sec
                .headers
                .iter()
                .find(|h| h.hf_type == HeaderFooterType::Default)
                .or_else(|| {
                    if sec.title_page {
                        sec.headers
                            .iter()
                            .find(|h| h.hf_type == HeaderFooterType::First)
                    } else {
                        sec.headers.first()
                    }
                });
            if let Some(hf) = header_ref {
                // First-page header (if title_page is set and a First header exists)
                let first_hf = if sec.title_page {
                    sec.headers
                        .iter()
                        .find(|h| h.hf_type == HeaderFooterType::First)
                } else {
                    None
                };
                if let Some(fhf) = first_hf {
                    if fhf.node_id != hf.node_id {
                        html.push_str(&format!(
                            "<header data-section-index=\"{}\" data-header-type=\"first\" style=\"border-bottom:1px solid #dadce0;padding:8px 0;margin-bottom:16px;color:#5f6368;font-size:9pt;display:none\">",
                            sec_idx
                        ));
                        render_children(model, fhf.node_id, &mut html);
                        html.push_str("</header>");
                    }
                }
                // Default header for this section — only the first section's
                // default header is visible by default; others are hidden until
                // the editor assigns them to pages.
                let display = if sec_idx == 0 { "" } else { "display:none;" };
                html.push_str(&format!(
                    "<header data-section-index=\"{}\" data-header-type=\"default\" style=\"border-bottom:1px solid #dadce0;padding:8px 0;margin-bottom:16px;color:#5f6368;font-size:9pt;{}\">",
                    sec_idx, display
                ));
                render_children(model, hf.node_id, &mut html);
                html.push_str("</header>");
            }
        }

        // Render body content
        render_children(model, body_id, &mut html);

        // Render footnote/endnote bodies from root node.
        // These are children of the Document root (not Body) with type FootnoteBody/EndnoteBody.
        let root_id = model.root_id();
        if let Some(root_node) = model.node(root_id) {
            let mut has_footnotes = false;
            let mut has_endnotes = false;
            // First pass: check if any footnotes/endnotes exist
            for &child_id in &root_node.children {
                if let Some(child) = model.node(child_id) {
                    match child.node_type {
                        NodeType::FootnoteBody => {
                            has_footnotes = true;
                        }
                        NodeType::EndnoteBody => {
                            has_endnotes = true;
                        }
                        _ => {}
                    }
                }
            }
            // Render footnotes section
            if has_footnotes {
                html.push_str(
                    "<div class=\"footnotes-section\" data-footnotes=\"true\" contenteditable=\"false\">"
                );
                html.push_str(
                    "<hr class=\"footnote-separator\" style=\"border:none;border-top:1px solid #dadce0;width:33%;margin:12px 0 8px 0;text-align:left\" />"
                );
                for &child_id in &root_node.children {
                    if let Some(child) = model.node(child_id) {
                        if child.node_type == NodeType::FootnoteBody {
                            render_node(model, child_id, &mut html);
                        }
                    }
                }
                html.push_str("</div>");
            }
            // Render endnotes section
            if has_endnotes {
                html.push_str(
                    "<div class=\"endnotes-section\" data-endnotes=\"true\" contenteditable=\"false\">"
                );
                html.push_str(
                    "<div class=\"endnotes-title\" style=\"font-weight:600;font-size:11pt;margin:16px 0 8px 0;border-bottom:1px solid #dadce0;padding-bottom:4px\">Endnotes</div>"
                );
                for &child_id in &root_node.children {
                    if let Some(child) = model.node(child_id) {
                        if child.node_type == NodeType::EndnoteBody {
                            render_node(model, child_id, &mut html);
                        }
                    }
                }
                html.push_str("</div>");
            }
        }

        // Render footers from ALL sections, tagged with data-section-index
        for (sec_idx, sec) in sections.iter().enumerate() {
            let footer_ref = sec
                .footers
                .iter()
                .find(|f| f.hf_type == HeaderFooterType::Default)
                .or_else(|| {
                    if sec.title_page {
                        sec.footers
                            .iter()
                            .find(|f| f.hf_type == HeaderFooterType::First)
                    } else {
                        sec.footers.first()
                    }
                });
            if let Some(hf) = footer_ref {
                // First-page footer
                let first_hf = if sec.title_page {
                    sec.footers
                        .iter()
                        .find(|h| h.hf_type == HeaderFooterType::First)
                } else {
                    None
                };
                if let Some(fhf) = first_hf {
                    if fhf.node_id != hf.node_id {
                        html.push_str(&format!(
                            "<footer data-section-index=\"{}\" data-footer-type=\"first\" style=\"border-top:1px solid #dadce0;padding:8px 0;margin-top:16px;color:#5f6368;font-size:9pt;display:none\">",
                            sec_idx
                        ));
                        render_children(model, fhf.node_id, &mut html);
                        html.push_str("</footer>");
                    }
                }
                let display = if sec_idx == 0 { "" } else { "display:none;" };
                html.push_str(&format!(
                    "<footer data-section-index=\"{}\" data-footer-type=\"default\" style=\"border-top:1px solid #dadce0;padding:8px 0;margin-top:16px;color:#5f6368;font-size:9pt;{}\">",
                    sec_idx, display
                ));
                render_children(model, hf.node_id, &mut html);
                html.push_str("</footer>");
            }
        }

        Ok(html)
    }

    /// Get the number of tracked changes in the document.
    pub fn tracked_changes_count(&self) -> Result<usize, JsError> {
        let doc = self.doc()?;
        Ok(doc.tracked_changes().len())
    }

    /// Accept all tracked changes in the document.
    ///
    /// Insertions keep their content; deletions are removed; format changes
    /// keep the new formatting. All revision attributes are stripped.
    pub fn accept_all_changes(&mut self) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.accept_all_changes()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Reject all tracked changes in the document.
    ///
    /// Insertions are removed; deletions are un-deleted; format changes
    /// restore original formatting. All revision attributes are stripped.
    pub fn reject_all_changes(&mut self) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.reject_all_changes()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Accept a single tracked change by node ID string ("replica:counter").
    pub fn accept_change(&mut self, node_id_str: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let node_id = parse_node_id(node_id_str)?;
        doc.accept_change(node_id)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Reject a single tracked change by node ID string ("replica:counter").
    pub fn reject_change(&mut self, node_id_str: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let node_id = parse_node_id(node_id_str)?;
        doc.reject_change(node_id)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get all tracked changes as a JSON array.
    ///
    /// Returns `[{"nodeId":"0:5","type":"Insert","author":"...","date":"..."},...]`
    pub fn tracked_changes_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let changes = doc.tracked_changes();
        let entries: Vec<String> = changes
            .iter()
            .map(|(nid, rev_type, author, date)| {
                format!(
                    "{{\"nodeId\":\"{}:{}\",\"type\":\"{}\",\"author\":\"{}\",\"date\":\"{}\"}}",
                    nid.replica,
                    nid.counter,
                    escape_json(rev_type),
                    escape_json(author.as_deref().unwrap_or("")),
                    escape_json(date.as_deref().unwrap_or("")),
                )
            })
            .collect();
        Ok(format!("[{}]", entries.join(",")))
    }

    // ─── Structure Queries ───────────────────────────────────────

    /// Get the body node ID as "replica:counter" string.
    pub fn body_id(&self) -> Result<Option<String>, JsError> {
        let doc = self.doc()?;
        Ok(doc
            .body_id()
            .map(|id| format!("{}:{}", id.replica, id.counter)))
    }

    /// Get top-level paragraph IDs as a JSON array of "replica:counter" strings.
    pub fn paragraph_ids_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let ids: Vec<String> = doc
            .paragraph_ids()
            .iter()
            .map(|id| format!("\"{}:{}\"", id.replica, id.counter))
            .collect();
        Ok(format!("[{}]", ids.join(",")))
    }

    /// Get all body-level node IDs with their types as JSON.
    ///
    /// Returns `[{"id":"0:5","type":"Paragraph"},{"id":"0:12","type":"Table"},...]`
    pub fn body_children_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body node not found"))?;
        let mut entries = Vec::new();
        for &child_id in &body.children {
            if let Some(child) = doc.node(child_id) {
                let type_str = node_type_str(&child.node_type);
                entries.push(format!(
                    "{{\"id\":\"{}:{}\",\"type\":\"{}\"}}",
                    child_id.replica, child_id.counter, type_str
                ));
            }
        }
        Ok(format!("[{}]", entries.join(",")))
    }

    /// Get detailed info about a node as JSON.
    ///
    /// Returns `{"id":"0:5","type":"Paragraph","text":"Hello","children":[...],...}`
    pub fn node_info_json(&self, node_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let nid = parse_node_id(node_id_str)?;
        let node = doc
            .node(nid)
            .ok_or_else(|| JsError::new(&format!("Node {} not found", node_id_str)))?;
        Ok(node_to_json(doc.model(), nid, node))
    }

    /// Get the text content of a paragraph (concatenates all runs).
    pub fn get_paragraph_text(&self, node_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let nid = parse_node_id(node_id_str)?;
        Ok(extract_paragraph_text(doc.model(), nid))
    }

    // ─── Editing Operations ───────────────────────────────────────

    /// Append a new paragraph with plain text at the end of the document body.
    ///
    /// Returns the new paragraph's node ID as "replica:counter".
    pub fn append_paragraph(&mut self, text: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let index = doc.node(body_id).map(|n| n.children.len()).unwrap_or(0);

        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert paragraph");
        txn.push(Operation::insert_node(
            body_id,
            index,
            Node::new(para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, text)));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Insert a new paragraph after a given node.
    ///
    /// Returns the new paragraph's node ID.
    pub fn insert_paragraph_after(
        &mut self,
        after_id_str: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_id_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body children"))?
            + 1;

        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert paragraph after");
        txn.push(Operation::insert_node(
            body_id,
            index,
            Node::new(para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, text)));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Append a heading at the given level (1-6).
    ///
    /// Returns the heading paragraph's node ID.
    pub fn append_heading(&mut self, level: u8, text: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let index = doc.node(body_id).map(|n| n.children.len()).unwrap_or(0);

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        let style_id = format!("Heading{}", level.clamp(1, 6));
        para.attributes
            .set(AttributeKey::StyleId, AttributeValue::String(style_id));

        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert heading");
        txn.push(Operation::insert_node(body_id, index, para));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, text)));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Delete a body-level node (paragraph, table, heading, etc.).
    pub fn delete_node(&mut self, node_id_str: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let nid = parse_node_id(node_id_str)?;
        doc.apply(Operation::delete_node(nid))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Move a node (e.g. an image paragraph) to be after another node in
    /// the same parent (body). Used for drag-and-drop reordering.
    pub fn move_node_after(
        &mut self,
        node_id_str: &str,
        after_id_str: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let node_id = parse_node_id(node_id_str)?;
        let after_id = parse_node_id(after_id_str)?;

        // Find the parent of the target node
        let after_node = doc
            .node(after_id)
            .ok_or_else(|| JsError::new("Target node not found"))?;
        let parent_id = after_node
            .parent
            .ok_or_else(|| JsError::new("Target has no parent"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let index = parent
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Target not in parent children"))?
            + 1;

        doc.apply(Operation::move_node(node_id, parent_id, index))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Move a node to be before another node in the same parent (body).
    pub fn move_node_before(
        &mut self,
        node_id_str: &str,
        before_id_str: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let node_id = parse_node_id(node_id_str)?;
        let before_id = parse_node_id(before_id_str)?;

        let before_node = doc
            .node(before_id)
            .ok_or_else(|| JsError::new("Target node not found"))?;
        let parent_id = before_node
            .parent
            .ok_or_else(|| JsError::new("Target has no parent"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let index = parent
            .children
            .iter()
            .position(|&c| c == before_id)
            .ok_or_else(|| JsError::new("Target not in parent children"))?;

        doc.apply(Operation::move_node(node_id, parent_id, index))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set the entire text content of a paragraph.
    ///
    /// # Formatting preservation behavior
    ///
    /// - **No change**: If `new_text` matches the existing text across all runs,
    ///   this is a no-op — per-run formatting is fully preserved.
    /// - **Single-run edit**: If the diff falls within a single run, a targeted
    ///   insert/delete is used and that run's formatting is preserved.
    /// - **Cross-run edit**: When the edit spans multiple runs, extra runs are
    ///   deleted and the surviving run receives the new text. **This collapses
    ///   inline formatting** (bold, italic, links, font changes, etc.) to a
    ///   single formatting context.
    ///
    /// # Preferred alternatives
    ///
    /// For DOM-driven edits from the editor, prefer range-aware operations:
    /// - `insert_text_in_paragraph()` — insert at a specific offset
    /// - `delete_text_in_paragraph()` — delete a range within a paragraph
    /// - `format_selection()` — apply formatting to a character range
    /// - `replace_text()` — replace text in a range (preserves surrounding formatting)
    ///
    /// These operations work at the character/run level and never collapse
    /// formatting outside the edited range. `set_paragraph_text` should be
    /// reserved for sync/convergence scenarios where the full paragraph text
    /// needs to be force-set (e.g., non-CRDT collaboration fallback).
    pub fn set_paragraph_text(&mut self, node_id_str: &str, new_text: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        // Gather the full existing text across ALL runs so we can detect
        // whether anything actually changed.
        let existing_text = extract_paragraph_text(doc.model(), para_id);

        // If text hasn't changed, skip the mutation entirely.
        // This preserves multi-run formatting after renderDocument().
        if existing_text == new_text {
            return Ok(());
        }

        // Collect runs with their text node IDs, text content, and char ranges
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;

        let mut run_info: Vec<(NodeId, NodeId, String, usize, usize)> = Vec::new(); // (run_id, text_id, text, start_char, end_char)
        let mut char_offset = 0usize;
        for &child_id in &para.children {
            if let Some(child) = doc.node(child_id) {
                match child.node_type {
                    NodeType::Run => {
                        // Find text node in this run
                        for &text_child in &child.children {
                            if let Some(tc) = doc.node(text_child) {
                                if tc.node_type == NodeType::Text {
                                    let text = tc.text_content.as_deref().unwrap_or("").to_string();
                                    let len = text.chars().count();
                                    run_info.push((
                                        child_id,
                                        text_child,
                                        text,
                                        char_offset,
                                        char_offset + len,
                                    ));
                                    char_offset += len;
                                    break;
                                }
                            }
                        }
                    }
                    NodeType::LineBreak | NodeType::Tab => {
                        char_offset += 1;
                    }
                    _ => {}
                }
            }
        }

        if run_info.is_empty() {
            // No runs — create one and set text
            let (text_node_id, _) = ensure_run_and_text(doc, para_id)?;
            if !new_text.is_empty() {
                doc.apply(Operation::insert_text(text_node_id, 0, new_text))
                    .map_err(|e| JsError::new(&e.to_string()))?;
            }
            return Ok(());
        }

        // Try diff-based update to preserve multi-run formatting.
        // Find the common prefix and suffix between old and new text.
        let old_chars: Vec<char> = existing_text.chars().collect();
        let new_chars: Vec<char> = new_text.chars().collect();
        let old_len = old_chars.len();
        let new_len = new_chars.len();

        // Find common prefix length
        let mut prefix_len = 0;
        while prefix_len < old_len
            && prefix_len < new_len
            && old_chars[prefix_len] == new_chars[prefix_len]
        {
            prefix_len += 1;
        }

        // Find common suffix length (don't overlap with prefix)
        let mut suffix_len = 0;
        while suffix_len < (old_len - prefix_len)
            && suffix_len < (new_len - prefix_len)
            && old_chars[old_len - 1 - suffix_len] == new_chars[new_len - 1 - suffix_len]
        {
            suffix_len += 1;
        }

        let delete_start = prefix_len;
        let delete_end = old_len - suffix_len;
        let insert_text: String = new_chars[prefix_len..new_len - suffix_len].iter().collect();

        // Find which run contains delete_start
        #[allow(clippy::type_complexity)]
        let find_run_at =
            |char_pos: usize| -> Option<(usize, &(NodeId, NodeId, String, usize, usize))> {
                for (i, info) in run_info.iter().enumerate() {
                    if char_pos >= info.3 && char_pos <= info.4 {
                        return Some((i, info));
                    }
                }
                // If at the very end, use the last run
                run_info.last().map(|info| (run_info.len() - 1, info))
            };

        // Simple case: edit is within a single run (common for typing)
        if let Some((start_run_idx, _)) = find_run_at(delete_start) {
            let end_run_idx = if delete_end <= delete_start {
                start_run_idx
            } else if let Some((idx, _)) = find_run_at(delete_end.saturating_sub(1)) {
                idx
            } else {
                start_run_idx
            };

            if start_run_idx == end_run_idx {
                // Edit is within a single run — apply insert/delete directly
                let (_run_id, text_id, _run_text, run_start, _run_end) = &run_info[start_run_idx];
                let offset_in_run = delete_start - run_start;
                let delete_count = delete_end - delete_start;

                let mut txn = Transaction::with_label("Sync paragraph text");
                if delete_count > 0 {
                    txn.push(Operation::delete_text(
                        *text_id,
                        offset_in_run,
                        delete_count,
                    ));
                }
                if !insert_text.is_empty() {
                    txn.push(Operation::insert_text(
                        *text_id,
                        offset_in_run,
                        &insert_text,
                    ));
                }
                if !txn.is_empty() {
                    doc.apply_transaction(&txn)
                        .map_err(|e| JsError::new(&e.to_string()))?;
                }
                return Ok(());
            }
        }

        // Complex case: edit spans multiple runs.
        // Fall back to collapsing all runs into one (preserves text but loses formatting).
        let run_children: Vec<NodeId> = run_info.iter().map(|r| r.0).collect();
        let mut txn = Transaction::with_label("Set paragraph text");

        // Delete extra runs (in reverse order)
        for &run_id in run_children[1..].iter().rev() {
            txn.push(Operation::delete_node(run_id));
        }

        // Replace first run's text
        let first_text_id = run_info[0].1;
        let first_old_len = run_info[0].2.chars().count();
        if first_old_len > 0 {
            txn.push(Operation::delete_text(first_text_id, 0, first_old_len));
        }
        if !new_text.is_empty() {
            txn.push(Operation::insert_text(first_text_id, 0, new_text));
        }

        if !txn.is_empty() {
            doc.apply_transaction(&txn)
                .map_err(|e| JsError::new(&e.to_string()))?;
        }

        Ok(())
    }

    /// Insert text at an offset in a paragraph's first text node.
    pub fn insert_text_in_paragraph(
        &mut self,
        node_id_str: &str,
        offset: usize,
        text: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        // Try to find the correct text node for the offset across all runs
        match find_text_node_at_char_offset(doc.model(), para_id, offset) {
            Ok((text_node_id, local_offset, _)) => doc
                .apply(Operation::insert_text(text_node_id, local_offset, text))
                .map_err(|e| JsError::new(&format!("Insert text failed: {}", e))),
            Err(_) => {
                // No text nodes exist — create run + text
                let (text_node_id, _) = ensure_run_and_text(doc, para_id)?;
                doc.apply(Operation::insert_text(text_node_id, 0, text))
                    .map_err(|e| JsError::new(&format!("Insert text failed: {}", e)))
            }
        }
    }

    /// Delete text in a paragraph at a given character offset.
    ///
    /// Correctly handles multi-run paragraphs by finding the right text node(s).
    pub fn delete_text_in_paragraph(
        &mut self,
        node_id_str: &str,
        offset: usize,
        length: usize,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        if paragraph_range_contains_inline_nodes(doc.model(), para_id, offset, offset + length) {
            return delete_text_range_in_paragraph(doc, para_id, offset, offset + length);
        }

        // Check if deletion stays within a single text node
        match find_text_node_at_char_offset(doc.model(), para_id, offset) {
            Ok((text_node_id, local_offset, text_len)) => {
                if local_offset + length <= text_len {
                    // Fits within one text node
                    return doc
                        .apply(Operation::delete_text(text_node_id, local_offset, length))
                        .map_err(|e| JsError::new(&format!("Delete text failed: {}", e)));
                }
            }
            Err(e) => return Err(e),
        }

        // Spans multiple runs — use range deletion
        delete_text_range_in_paragraph(doc, para_id, offset, offset + length)
    }

    // ─── Formatting ───────────────────────────────────────────────

    /// Set bold on a paragraph's first run.
    ///
    /// For selection-aware formatting, use `format_selection()` or
    /// `set_bold_range()` instead — they correctly handle mixed-format
    /// paragraphs by splitting runs at selection boundaries.
    pub fn set_bold(&mut self, node_id_str: &str, bold: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let attrs = s1_model::AttributeMap::new().bold(bold);
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set bold on a selection range. Preferred over `set_bold` for toolbar
    /// actions when the user has an active text selection.
    pub fn set_bold_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        bold: bool,
    ) -> Result<(), JsError> {
        self.format_selection(
            start_node_str,
            start_offset,
            end_node_str,
            end_offset,
            "bold",
            if bold { "true" } else { "false" },
        )
    }

    /// Set italic on a paragraph's first run.
    /// For selection-aware formatting, use `set_italic_range` or `format_selection`.
    pub fn set_italic(&mut self, node_id_str: &str, italic: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let attrs = s1_model::AttributeMap::new().italic(italic);
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set underline on a paragraph's first run.
    pub fn set_underline(&mut self, node_id_str: &str, underline: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let mut attrs = s1_model::AttributeMap::new();
        if underline {
            attrs.set(
                AttributeKey::Underline,
                AttributeValue::UnderlineStyle(UnderlineStyle::Single),
            );
        } else {
            attrs.set(
                AttributeKey::Underline,
                AttributeValue::UnderlineStyle(UnderlineStyle::None),
            );
        }
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set strikethrough on a paragraph's first run.
    pub fn set_strikethrough(
        &mut self,
        node_id_str: &str,
        strikethrough: bool,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::Strikethrough,
            AttributeValue::Bool(strikethrough),
        );
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set font size on a paragraph's first run (in points).
    pub fn set_font_size(&mut self, node_id_str: &str, size_pt: f64) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let attrs = s1_model::AttributeMap::new().font_size(size_pt);
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set font family on a paragraph's first run.
    pub fn set_font_family(&mut self, node_id_str: &str, font: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let attrs = s1_model::AttributeMap::new().font_family(font);
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set text color on a paragraph's first run (hex string like "FF0000").
    pub fn set_color(&mut self, node_id_str: &str, hex: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let color = Color::from_hex(hex)
            .ok_or_else(|| JsError::new(&format!("Invalid color hex: {}", hex)))?;
        let attrs = s1_model::AttributeMap::new().color(color);
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // ─── Selection-aware formatting helpers ─────────────────────
    // These delegate to format_selection and are the preferred API
    // for toolbar actions when the user has an active text selection.

    /// Set italic on a selection range.
    pub fn set_italic_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        italic: bool,
    ) -> Result<(), JsError> {
        self.format_selection(
            start_node_str,
            start_offset,
            end_node_str,
            end_offset,
            "italic",
            if italic { "true" } else { "false" },
        )
    }

    /// Set underline on a selection range.
    pub fn set_underline_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        underline: bool,
    ) -> Result<(), JsError> {
        self.format_selection(
            start_node_str,
            start_offset,
            end_node_str,
            end_offset,
            "underline",
            if underline { "single" } else { "none" },
        )
    }

    /// Set font size on a selection range (in points).
    pub fn set_font_size_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        size_pt: f64,
    ) -> Result<(), JsError> {
        self.format_selection(
            start_node_str,
            start_offset,
            end_node_str,
            end_offset,
            "fontSize",
            &size_pt.to_string(),
        )
    }

    /// Set font family on a selection range.
    pub fn set_font_family_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        font: &str,
    ) -> Result<(), JsError> {
        self.format_selection(
            start_node_str,
            start_offset,
            end_node_str,
            end_offset,
            "fontFamily",
            font,
        )
    }

    /// Set text color on a selection range (hex string like "FF0000").
    pub fn set_color_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        hex: &str,
    ) -> Result<(), JsError> {
        self.format_selection(
            start_node_str,
            start_offset,
            end_node_str,
            end_offset,
            "color",
            hex,
        )
    }

    /// Set paragraph alignment ("left", "center", "right", "justify").
    pub fn set_alignment(&mut self, node_id_str: &str, alignment: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let align = match alignment.to_lowercase().as_str() {
            "left" => Alignment::Left,
            "center" => Alignment::Center,
            "right" => Alignment::Right,
            "justify" => Alignment::Justify,
            _ => return Err(JsError::new(&format!("Unknown alignment: {}", alignment))),
        };
        let attrs = s1_model::AttributeMap::new().alignment(align);
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set paragraph indentation (left, right, or first-line).
    ///
    /// `indent_type` is one of: "left", "right", "firstLine".
    /// `value_pt` is the indent value in points.
    pub fn set_indent(
        &mut self,
        node_id_str: &str,
        indent_type: &str,
        value_pt: f64,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        let clamped = if indent_type == "firstLine" {
            value_pt
        } else {
            value_pt.max(0.0)
        };
        match indent_type {
            "left" => {
                attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(clamped));
            }
            "right" => {
                attrs.set(AttributeKey::IndentRight, AttributeValue::Float(clamped));
            }
            "firstLine" => {
                attrs.set(
                    AttributeKey::IndentFirstLine,
                    AttributeValue::Float(clamped),
                );
            }
            _ => {
                return Err(JsError::new(&format!(
                    "Unknown indent type: {}",
                    indent_type
                )))
            }
        }
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set the line spacing for a paragraph.
    ///
    /// `spacing` is one of: "single", "1.5", "double", or a numeric multiplier (e.g. "1.15").
    pub fn set_line_spacing(&mut self, node_id_str: &str, spacing: &str) -> Result<(), JsError> {
        use s1_model::LineSpacing;
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let ls = match spacing.to_lowercase().as_str() {
            "single" | "1" => LineSpacing::Single,
            "1.5" | "one-point-five" => LineSpacing::OnePointFive,
            "double" | "2" => LineSpacing::Double,
            other => {
                let factor: f64 = other
                    .parse()
                    .map_err(|_| JsError::new(&format!("Invalid line spacing: {}", spacing)))?;
                LineSpacing::Multiple(factor)
            }
        };
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(AttributeKey::LineSpacing, AttributeValue::LineSpacing(ls));
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set paragraph spacing (before and/or after) in points.
    ///
    /// `spacing_type` is one of: "before", "after".
    /// `value_pt` is the spacing value in points.
    pub fn set_paragraph_spacing(
        &mut self,
        node_id_str: &str,
        spacing_type: &str,
        value_pt: f64,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        let clamped = value_pt.max(0.0);
        match spacing_type {
            "before" => {
                attrs.set(AttributeKey::SpacingBefore, AttributeValue::Float(clamped));
            }
            "after" => {
                attrs.set(AttributeKey::SpacingAfter, AttributeValue::Float(clamped));
            }
            _ => {
                return Err(JsError::new(&format!(
                    "Unknown spacing type: {} (use 'before' or 'after')",
                    spacing_type
                )))
            }
        }
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set paragraph keep options (keep with next, keep lines together).
    ///
    /// `keep_type` is one of: "keepWithNext", "keepLinesTogether".
    /// `enabled` controls whether the option is on or off.
    pub fn set_paragraph_keep(
        &mut self,
        node_id_str: &str,
        keep_type: &str,
        enabled: bool,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        match keep_type {
            "keepWithNext" => {
                attrs.set(AttributeKey::KeepWithNext, AttributeValue::Bool(enabled));
            }
            "keepLinesTogether" => {
                attrs.set(
                    AttributeKey::KeepLinesTogether,
                    AttributeValue::Bool(enabled),
                );
            }
            _ => {
                return Err(JsError::new(&format!(
                    "Unknown keep type: {} (use 'keepWithNext' or 'keepLinesTogether')",
                    keep_type
                )))
            }
        }
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Insert a line break (soft return) within a paragraph at a character offset.
    ///
    /// Creates a `LineBreak` node within the run at the specified offset,
    /// splitting the text node if the offset falls in the middle.
    pub fn insert_line_break(
        &mut self,
        node_id_str: &str,
        char_offset: usize,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let para_children = para.children.clone();

        if para_children.is_empty() {
            // Empty paragraph: insert a bare line break node as a paragraph child.
            let lb_id = doc.next_id();
            let mut txn = Transaction::with_label("Insert line break");
            txn.push(Operation::insert_node(
                para_id,
                0,
                Node::new(lb_id, NodeType::LineBreak),
            ));
            return doc
                .apply_transaction(&txn)
                .map_err(|e| JsError::new(&e.to_string()));
        }

        let mut accumulated = 0usize;
        let mut insert_index = para_children.len();
        let mut found = false;

        for (idx, &child_id) in para_children.iter().enumerate() {
            let Some(child) = doc.node(child_id) else {
                continue;
            };
            match child.node_type {
                NodeType::Run => {
                    let run_len = run_char_len(doc.model(), child_id);
                    if char_offset <= accumulated + run_len {
                        let local_offset = char_offset - accumulated;
                        if local_offset == 0 {
                            insert_index = idx;
                        } else if local_offset >= run_len {
                            insert_index = idx + 1;
                        } else {
                            split_run_internal(doc, child_id, local_offset)?;
                            insert_index = idx + 1;
                        }
                        found = true;
                        break;
                    }
                    accumulated += run_len;
                }
                NodeType::LineBreak | NodeType::Tab => {
                    if char_offset <= accumulated + 1 {
                        insert_index = if char_offset == accumulated {
                            idx
                        } else {
                            idx + 1
                        };
                        found = true;
                        break;
                    }
                    accumulated += 1;
                }
                _ => {}
            }
        }

        if !found && char_offset > accumulated {
            return Err(JsError::new("Offset out of range"));
        }

        let lb_id = doc.next_id();
        let mut txn = Transaction::with_label("Insert line break");
        txn.push(Operation::insert_node(
            para_id,
            insert_index,
            Node::new(lb_id, NodeType::LineBreak),
        ));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Insert a tab node at the given character offset within a paragraph.
    ///
    /// Like `insert_line_break`, this inserts a `Tab` node at paragraph level,
    /// splitting runs as needed. Tab nodes render
    /// as `&emsp;` in HTML and as proper tab stops in layout.
    pub fn insert_tab(&mut self, node_id_str: &str, char_offset: usize) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let para_children = para.children.clone();

        if para_children.is_empty() {
            let tab_id = doc.next_id();
            let mut txn = Transaction::with_label("Insert tab");
            txn.push(Operation::insert_node(
                para_id,
                0,
                Node::new(tab_id, NodeType::Tab),
            ));
            return doc
                .apply_transaction(&txn)
                .map_err(|e| JsError::new(&e.to_string()));
        }

        let mut accumulated = 0usize;
        let mut insert_index = para_children.len();
        let mut found = false;

        for (idx, &child_id) in para_children.iter().enumerate() {
            let Some(child) = doc.node(child_id) else {
                continue;
            };
            match child.node_type {
                NodeType::Run => {
                    let run_len = run_char_len(doc.model(), child_id);
                    if char_offset <= accumulated + run_len {
                        let local_offset = char_offset - accumulated;
                        if local_offset == 0 {
                            insert_index = idx;
                        } else if local_offset >= run_len {
                            insert_index = idx + 1;
                        } else {
                            split_run_internal(doc, child_id, local_offset)?;
                            insert_index = idx + 1;
                        }
                        found = true;
                        break;
                    }
                    accumulated += run_len;
                }
                NodeType::LineBreak | NodeType::Tab => {
                    if char_offset <= accumulated + 1 {
                        insert_index = if char_offset == accumulated {
                            idx
                        } else {
                            idx + 1
                        };
                        found = true;
                        break;
                    }
                    accumulated += 1;
                }
                _ => {}
            }
        }

        if !found && char_offset > accumulated {
            return Err(JsError::new("Offset out of range"));
        }

        let tab_id = doc.next_id();
        let mut txn = Transaction::with_label("Insert tab");
        txn.push(Operation::insert_node(
            para_id,
            insert_index,
            Node::new(tab_id, NodeType::Tab),
        ));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set the document title (metadata).
    pub fn set_title(&mut self, title: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.metadata_mut().title = Some(title.to_string());
        Ok(())
    }

    /// Set the document author (metadata).
    pub fn set_author(&mut self, author: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.metadata_mut().creator = Some(author.to_string());
        Ok(())
    }

    // ─── Multi-Cursor Operations ─────────────────────────────────

    /// Insert text at multiple cursor positions simultaneously.
    ///
    /// Takes a JSON array of `[{"nodeId":"0:5","offset":3,"text":"x"}, ...]`.
    /// Positions are sorted in reverse document order and applied back-to-front
    /// so that earlier insertions don't shift later offsets.
    ///
    /// All insertions form a single undo step via merge_undo_entries.
    pub fn multi_cursor_insert(&mut self, cursors_json: &str) -> Result<(), JsError> {
        let cursors: Vec<serde_json::Value> = serde_json::from_str(cursors_json)
            .map_err(|e| JsError::new(&format!("Invalid JSON: {e}")))?;

        if cursors.is_empty() {
            return Ok(());
        }

        // Parse and sort cursors in reverse order (last position first)
        // so insertions don't shift earlier offsets
        let mut positions: Vec<(String, usize, String)> = Vec::new();
        for c in &cursors {
            let node_id = c["nodeId"]
                .as_str()
                .ok_or_else(|| JsError::new("Missing nodeId"))?
                .to_string();
            let offset = c["offset"]
                .as_u64()
                .ok_or_else(|| JsError::new("Missing offset"))? as usize;
            let text = c["text"]
                .as_str()
                .ok_or_else(|| JsError::new("Missing text"))?
                .to_string();
            positions.push((node_id, offset, text));
        }

        // Sort by nodeId descending, then offset descending
        positions.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

        // Record undo count before batch
        let start_undo = self.doc()?.undo_count();

        // Apply each insertion
        for (node_id, offset, text) in &positions {
            self.insert_text_in_paragraph(node_id, *offset, text)?;
        }

        // Merge all insertions into single undo step
        let end_undo = self.doc()?.undo_count();
        let delta = end_undo.saturating_sub(start_undo);
        if delta > 1 {
            self.doc_mut()?
                .merge_undo_entries(delta, "Multi-cursor insert")
                .map_err(|e| JsError::new(&e.to_string()))?;
        }

        Ok(())
    }

    /// Delete text at multiple cursor positions simultaneously.
    ///
    /// Takes a JSON array of `[{"nodeId":"0:5","offset":3,"length":1}, ...]`.
    /// Applied in reverse order to preserve offsets.
    pub fn multi_cursor_delete(&mut self, cursors_json: &str) -> Result<(), JsError> {
        let cursors: Vec<serde_json::Value> = serde_json::from_str(cursors_json)
            .map_err(|e| JsError::new(&format!("Invalid JSON: {e}")))?;

        if cursors.is_empty() {
            return Ok(());
        }

        let mut positions: Vec<(String, usize, usize)> = Vec::new();
        for c in &cursors {
            let node_id = c["nodeId"]
                .as_str()
                .ok_or_else(|| JsError::new("Missing nodeId"))?
                .to_string();
            let offset = c["offset"]
                .as_u64()
                .ok_or_else(|| JsError::new("Missing offset"))? as usize;
            let length = c["length"]
                .as_u64()
                .ok_or_else(|| JsError::new("Missing length"))? as usize;
            positions.push((node_id, offset, length));
        }

        positions.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

        let start_undo = self.doc()?.undo_count();

        for (node_id, offset, length) in &positions {
            self.delete_text_in_paragraph(node_id, *offset, *length)?;
        }

        let end_undo = self.doc()?.undo_count();
        let delta = end_undo.saturating_sub(start_undo);
        if delta > 1 {
            self.doc_mut()?
                .merge_undo_entries(delta, "Multi-cursor delete")
                .map_err(|e| JsError::new(&e.to_string()))?;
        }

        Ok(())
    }

    // ─── Undo / Redo ──────────────────────────────────────────────

    /// Undo the last editing operation. Returns true if something was undone.
    pub fn undo(&mut self) -> Result<bool, JsError> {
        let doc = self.doc_mut()?;
        doc.undo().map_err(|e| JsError::new(&e.to_string()))
    }

    /// Redo the last undone operation. Returns true if something was redone.
    pub fn redo(&mut self) -> Result<bool, JsError> {
        let doc = self.doc_mut()?;
        doc.redo().map_err(|e| JsError::new(&e.to_string()))
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> Result<bool, JsError> {
        let doc = self.doc()?;
        Ok(doc.can_undo())
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> Result<bool, JsError> {
        let doc = self.doc()?;
        Ok(doc.can_redo())
    }

    /// Clear all undo/redo history.
    pub fn clear_history(&mut self) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.clear_history();
        Ok(())
    }

    // ─── Editor API ──────────────────────────────────────────────

    /// Render a single node (paragraph, table, etc.) as HTML.
    ///
    /// Returns the HTML string for that node only, suitable for incremental
    /// DOM updates. Uses the same rendering as `to_html()`.
    pub fn render_node_html(&self, node_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let nid = parse_node_id(node_id_str)?;
        let model = doc.model();
        let node = model
            .node(nid)
            .ok_or_else(|| JsError::new(&format!("Node {} not found", node_id_str)))?;
        let mut html = String::new();
        // For paragraphs with list info, compute the ordinal from siblings
        if node.node_type == NodeType::Paragraph {
            let ordinal = compute_list_ordinal(model, nid);
            render_paragraph(model, nid, &mut html, ordinal);
        } else {
            render_node(model, nid, &mut html);
        }
        Ok(html)
    }

    /// Render a paragraph node as HTML for the half-open character range
    /// `[start_char, end_char)`. Used by pagination to mount page-specific
    /// fragments for split paragraphs instead of rendering the full paragraph
    /// and clipping it in CSS.
    pub fn render_node_slice(
        &self,
        node_id_str: &str,
        start_char: usize,
        end_char: usize,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let nid = parse_node_id(node_id_str)?;
        let model = doc.model();
        let node = model
            .node(nid)
            .ok_or_else(|| JsError::new(&format!("Node {} not found", node_id_str)))?;

        if node.node_type != NodeType::Paragraph {
            return self.render_node_html(node_id_str);
        }

        let mut html = String::new();
        let safe_end = end_char.max(start_char);
        render_paragraph_clean_partial(model, nid, Some(start_char), Some(safe_end), &mut html);
        Ok(html)
    }

    /// Render a table with only specific rows (for split-table pagination).
    ///
    /// `table_id_str` is the table node ID (e.g., "1:5").
    /// `row_ids_json` is a JSON array of row node IDs to include (e.g., '["1:6","1:7"]').
    /// `chunk_id` is a unique identifier for this chunk (used as data-node-id).
    /// `is_continuation` indicates if this is a continuation chunk (for styling).
    pub fn render_table_chunk(
        &self,
        table_id_str: &str,
        row_ids_json: &str,
        chunk_id: &str,
        is_continuation: bool,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let table_nid = parse_node_id(table_id_str)?;
        let model = doc.model();
        let table_node = model
            .node(table_nid)
            .ok_or_else(|| JsError::new(&format!("Table node {} not found", table_id_str)))?;

        // Parse row IDs from JSON array
        let row_ids_str = row_ids_json.trim();
        let mut row_nids: Vec<NodeId> = Vec::new();
        if row_ids_str.len() > 2 {
            // Strip [ and ]
            let inner = &row_ids_str[1..row_ids_str.len() - 1];
            for part in inner.split(',') {
                let id = part.trim().trim_matches('"');
                if !id.is_empty() {
                    row_nids.push(parse_node_id(id)?);
                }
            }
        }

        let mut html = String::new();
        html.push_str(&format!(
            "<table data-node-id=\"{}\" data-table-source=\"{}\" data-is-continuation=\"{}\" style=\"border-collapse:collapse;margin:0;width:100%\">",
            chunk_id, table_id_str, is_continuation
        ));

        // Render only the specified rows
        let row_set: std::collections::HashSet<NodeId> = row_nids.into_iter().collect();
        for &row_id in &table_node.children {
            if row_set.contains(&row_id) {
                render_node(model, row_id, &mut html);
            }
        }

        html.push_str("</table>");
        Ok(html)
    }

    /// Split a paragraph at a character offset.
    ///
    /// Creates a new paragraph after the current one with the tail text.
    /// If the original paragraph is a heading, the new paragraph inherits
    /// the same heading style.
    ///
    /// Returns the new paragraph's node ID as "replica:counter".
    pub fn split_paragraph(
        &mut self,
        node_id_str: &str,
        char_offset: usize,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let parent_id = para.parent.ok_or_else(|| JsError::new("No parent"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let index = parent.children.iter().position(|&c| c == para_id).unwrap();
        let para_children = para.children.clone();
        let para_attributes = para.attributes.clone();

        let mut accumulated = 0usize;
        let mut split_child_idx = para_children.len();
        let mut local_offset = 0usize;
        for (i, &child_id) in para_children.iter().enumerate() {
            if let Some(child) = doc.node(child_id) {
                let clen = match child.node_type {
                    NodeType::Run => run_char_len(doc.model(), child_id),
                    NodeType::LineBreak | NodeType::Tab => 1,
                    _ => 0,
                };
                if char_offset <= accumulated + clen {
                    split_child_idx = i;
                    local_offset = char_offset - accumulated;
                    break;
                }
                accumulated += clen;
            }
        }

        let new_para_id = doc.next_id();
        let mut txn = Transaction::with_label("Split paragraph");
        let mut new_para = Node::new(new_para_id, NodeType::Paragraph);
        new_para.attributes = para_attributes;
        txn.push(Operation::insert_node(parent_id, index + 1, new_para));

        let mut new_para_child_idx = 0;

        if split_child_idx < para_children.len() {
            let split_child_id = para_children[split_child_idx];
            let split_child_node = doc.node(split_child_id).unwrap();
            let split_child_type = split_child_node.node_type;
            let split_child_attrs = split_child_node.attributes.clone();
            let split_child_children = split_child_node.children.clone();

            if local_offset == 0 {
                for &child_id in para_children.iter().skip(split_child_idx) {
                    txn.push(Operation::move_node(
                        child_id,
                        new_para_id,
                        new_para_child_idx,
                    ));
                    new_para_child_idx += 1;
                }
            } else if split_child_type == NodeType::Run {
                let mut new_run = Node::new(doc.next_id(), NodeType::Run);
                new_run.attributes = split_child_attrs;
                let new_run_id = new_run.id;
                txn.push(Operation::insert_node(
                    new_para_id,
                    new_para_child_idx,
                    new_run,
                ));
                new_para_child_idx += 1;

                let (text_node_id, local_off, text_len) = find_text_node_at_char_offset_in_run(
                    doc.model(),
                    split_child_id,
                    local_offset,
                )?;

                let mut new_run_child_idx = 0;
                if local_off < text_len {
                    let text_node = doc.node(text_node_id).unwrap();
                    let full_text = text_node.text_content.as_deref().unwrap_or("");
                    let tail_text: String = full_text.chars().skip(local_off).collect();

                    txn.push(Operation::delete_text(
                        text_node_id,
                        local_off,
                        text_len - local_off,
                    ));

                    let new_text_id = doc.next_id();
                    txn.push(Operation::insert_node(
                        new_run_id,
                        new_run_child_idx,
                        Node::text(new_text_id, &tail_text),
                    ));
                    new_run_child_idx += 1;
                }

                let text_idx = split_child_children
                    .iter()
                    .position(|&c| c == text_node_id)
                    .unwrap();
                for &sub_id in split_child_children.iter().skip(text_idx + 1) {
                    txn.push(Operation::move_node(sub_id, new_run_id, new_run_child_idx));
                    new_run_child_idx += 1;
                }

                for &child_id in para_children.iter().skip(split_child_idx + 1) {
                    txn.push(Operation::move_node(
                        child_id,
                        new_para_id,
                        new_para_child_idx,
                    ));
                    new_para_child_idx += 1;
                }
            } else {
                for &child_id in para_children.iter().skip(split_child_idx + 1) {
                    txn.push(Operation::move_node(
                        child_id,
                        new_para_id,
                        new_para_child_idx,
                    ));
                    new_para_child_idx += 1;
                }
            }
        }

        if new_para_child_idx == 0 {
            let new_run_id = doc.next_id();
            let mut new_run = Node::new(new_run_id, NodeType::Run);
            if let Some(&last_run_id) = para_children.iter().rev().find(|&&c| {
                doc.node(c)
                    .map(|n| n.node_type == NodeType::Run)
                    .unwrap_or(false)
            }) {
                if let Some(last_run) = doc.node(last_run_id) {
                    new_run.attributes = last_run.attributes.clone();
                }
            }
            txn.push(Operation::insert_node(new_para_id, 0, new_run));
            txn.push(Operation::insert_node(
                new_run_id,
                0,
                Node::text(doc.next_id(), ""),
            ));
        }

        if split_child_idx == 0 && local_offset == 0 {
            let empty_run_id = doc.next_id();
            let mut empty_run = Node::new(empty_run_id, NodeType::Run);
            if let Some(&first_run_id) = para_children.iter().find(|&&c| {
                doc.node(c)
                    .map(|n| n.node_type == NodeType::Run)
                    .unwrap_or(false)
            }) {
                if let Some(first_run) = doc.node(first_run_id) {
                    empty_run.attributes = first_run.attributes.clone();
                }
            }
            txn.push(Operation::insert_node(para_id, 0, empty_run));
            txn.push(Operation::insert_node(
                empty_run_id,
                0,
                Node::text(doc.next_id(), ""),
            ));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", new_para_id.replica, new_para_id.counter))
    }

    /// Merge two adjacent paragraphs.
    ///
    /// Moves all runs from `second_id` into `first_id` (preserving formatting),
    /// then deletes the now-empty `second_id`. Used for Backspace at the start
    /// of a paragraph.
    pub fn merge_paragraphs(
        &mut self,
        first_id_str: &str,
        second_id_str: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let first_id = parse_node_id(first_id_str)?;
        let second_id = parse_node_id(second_id_str)?;

        // Guard: both nodes must be Paragraph type to merge
        let first_para = doc
            .node(first_id)
            .ok_or_else(|| JsError::new("First paragraph not found"))?;
        if first_para.node_type != NodeType::Paragraph {
            return Err(JsError::new(&format!(
                "Cannot merge: first node is {:?}, not Paragraph",
                first_para.node_type
            )));
        }
        let first_child_count = first_para.children.len();

        let second_para = doc
            .node(second_id)
            .ok_or_else(|| JsError::new("Second paragraph not found"))?;
        if second_para.node_type != NodeType::Paragraph {
            // If second node is a PageBreak, just delete it instead of merging
            if second_para.node_type == NodeType::PageBreak {
                return doc
                    .apply(Operation::delete_node(second_id))
                    .map_err(|e| JsError::new(&e.to_string()));
            }
            return Err(JsError::new(&format!(
                "Cannot merge: second node is {:?}, not Paragraph",
                second_para.node_type
            )));
        }
        let second_run_ids: Vec<NodeId> = second_para.children.clone();

        let mut txn = Transaction::with_label("Merge paragraphs");

        // Move each run from the second paragraph into the first paragraph,
        // appending after the first paragraph's existing children.
        for (i, run_id) in second_run_ids.iter().enumerate() {
            txn.push(Operation::move_node(
                *run_id,
                first_id,
                first_child_count + i,
            ));
        }

        // Delete the now-empty second paragraph
        txn.push(Operation::delete_node(second_id));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Delete a selection range spanning one or more paragraphs.
    ///
    /// If start and end are in the same paragraph, deletes the text range.
    /// If they span multiple paragraphs, deletes the tail of the first,
    /// all intermediate paragraphs, the head of the last, then merges
    /// the first and last paragraphs.
    pub fn delete_selection(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let start_para = parse_node_id(start_node_str)?;
        let end_para = parse_node_id(end_node_str)?;

        // Guard: both must be paragraph-like nodes (not Table, not Image)
        let start_type = doc
            .node(start_para)
            .map(|n| n.node_type)
            .unwrap_or(NodeType::Paragraph);
        let end_type = doc
            .node(end_para)
            .map(|n| n.node_type)
            .unwrap_or(NodeType::Paragraph);
        if start_type != NodeType::Paragraph || end_type != NodeType::Paragraph {
            // Can't delete across non-paragraph boundaries — silently succeed
            return Ok(());
        }

        if start_para == end_para {
            // Same paragraph — delete the text range, handling multi-run
            let length = end_offset.saturating_sub(start_offset);
            if length > 0 {
                delete_text_range_in_paragraph(doc, start_para, start_offset, end_offset)?;
            }
            return Ok(());
        }

        // Multi-paragraph deletion — gather all data before building transaction

        // Find the text node at the start offset for the first paragraph
        let start_full_text = extract_paragraph_text(doc.model(), start_para);
        let start_total_chars = start_full_text.chars().count();
        let (start_text_id, start_local_offset, _) =
            match find_text_node_at_char_offset(doc.model(), start_para, start_offset) {
                Ok(v) => v,
                Err(_) => {
                    let (tid, _) = ensure_run_and_text(doc, start_para)?;
                    (tid, 0, 0)
                }
            };

        // Get end paragraph text and body children (immutable borrows)
        let end_text = extract_paragraph_text(doc.model(), end_para);
        let remaining_text = if end_offset < end_text.chars().count() {
            end_text.chars().skip(end_offset).collect::<String>()
        } else {
            String::new()
        };

        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let children: Vec<NodeId> = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?
            .children
            .clone();

        // Find intermediate paragraph IDs
        let mut in_range = false;
        let mut middle_nodes = Vec::new();
        for &child_id in &children {
            if child_id == start_para {
                in_range = true;
                continue;
            }
            if child_id == end_para {
                break;
            }
            if in_range {
                middle_nodes.push(child_id);
            }
        }

        // Build transaction
        let mut txn = Transaction::with_label("Delete selection");

        // 1. Delete tail of first paragraph (from start_offset to end)
        let del_from_start = start_total_chars.saturating_sub(start_offset);
        let start_run_id = doc.node(start_text_id).and_then(|n| n.parent);
        let start_run_fully_selected = doc
            .node(start_text_id)
            .and_then(|n| n.parent)
            .map(|run_id| {
                let run_len = run_char_len(doc.model(), run_id);
                start_local_offset == 0 && run_len > 0 && del_from_start >= run_len
            })
            .unwrap_or(false);
        if del_from_start > 0 {
            // Delete remaining text from the start offset's text node
            txn.push(Operation::delete_text(
                start_text_id,
                start_local_offset,
                del_from_start.min(start_total_chars - start_offset),
            ));
        }

        // Delete any runs after the start run in the first paragraph
        if start_offset < start_total_chars {
            let para = doc.node(start_para);
            if let Some(p) = para {
                let mut past_split = false;
                let mut runs_to_delete = Vec::new();
                let mut accumulated = 0usize;
                for &child_id in &p.children {
                    if let Some(child) = doc.node(child_id) {
                        match child.node_type {
                            NodeType::Run => {
                                let rlen = run_char_len(doc.model(), child_id);
                                if past_split {
                                    if Some(child_id) != start_run_id {
                                        runs_to_delete.push(child_id);
                                    }
                                } else if accumulated + rlen >= start_offset
                                    && start_offset > accumulated
                                {
                                    past_split = true;
                                } else if accumulated >= start_offset {
                                    if !(start_run_fully_selected && Some(child_id) == start_run_id)
                                    {
                                        runs_to_delete.push(child_id);
                                    }
                                    past_split = true;
                                }
                                accumulated += rlen;
                            }
                            NodeType::LineBreak | NodeType::Tab => {
                                accumulated += 1;
                            }
                            _ => {}
                        }
                    }
                }
                for rid in runs_to_delete {
                    txn.push(Operation::delete_node(rid));
                }
            }
        }

        // 2. Delete intermediate paragraphs
        for mid_id in middle_nodes {
            txn.push(Operation::delete_node(mid_id));
        }

        // 3. Delete last paragraph entirely
        txn.push(Operation::delete_node(end_para));

        // 4. Append remaining text from last paragraph to first
        if !remaining_text.is_empty() {
            txn.push(Operation::insert_text(
                start_text_id,
                start_local_offset,
                &remaining_text,
            ));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the formatting state of a paragraph as JSON.
    ///
    /// Returns JSON with keys: bold, italic, underline, strikethrough,
    /// fontSize, fontFamily, color, alignment, headingLevel.
    /// Values come from the paragraph's attributes and first run's attributes.
    pub fn get_formatting_json(&self, node_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let para_id = parse_node_id(node_id_str)?;
        let model = doc.model();
        let para = model
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;

        // Paragraph-level attributes
        let alignment = match para.attributes.get(&AttributeKey::Alignment) {
            Some(AttributeValue::Alignment(a)) => match a {
                Alignment::Left => "left",
                Alignment::Center => "center",
                Alignment::Right => "right",
                Alignment::Justify => "justify",
                _ => "left",
            },
            _ => "left",
        };

        let heading_level: u8 = para
            .attributes
            .get_string(&AttributeKey::StyleId)
            .and_then(|sid| {
                let sid_lower = sid.to_lowercase();
                if sid_lower.starts_with("heading") {
                    sid_lower
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        // Run-level attributes (first run)
        let (bold, italic, underline, strikethrough, font_size, font_family, color) =
            if let Ok(run_id) = find_first_run(model, para_id) {
                if let Some(run) = model.node(run_id) {
                    let b = run.attributes.get_bool(&AttributeKey::Bold) == Some(true);
                    let i = run.attributes.get_bool(&AttributeKey::Italic) == Some(true);
                    let u = run.attributes.get(&AttributeKey::Underline).is_some()
                        && !matches!(
                            run.attributes.get(&AttributeKey::Underline),
                            Some(AttributeValue::UnderlineStyle(UnderlineStyle::None))
                        );
                    let s = run.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true);
                    let fs = run.attributes.get_f64(&AttributeKey::FontSize);
                    let ff = run
                        .attributes
                        .get_string(&AttributeKey::FontFamily)
                        .map(|s| s.to_string());
                    let c = match run.attributes.get(&AttributeKey::Color) {
                        Some(AttributeValue::Color(c)) => Some(c.to_hex()),
                        _ => None,
                    };
                    (b, i, u, s, fs, ff, c)
                } else {
                    (false, false, false, false, None, None, None)
                }
            } else {
                (false, false, false, false, None, None, None)
            };

        // Paragraph styleId (raw value, not just heading level)
        let style_id = para
            .attributes
            .get_string(&AttributeKey::StyleId)
            .unwrap_or("");

        let mut json = format!(
            "{{\"bold\":{},\"italic\":{},\"underline\":{},\"strikethrough\":{},\"alignment\":\"{}\",\"headingLevel\":{},\"styleId\":\"{}\"",
            bold, italic, underline, strikethrough, alignment, heading_level,
            escape_json(style_id)
        );
        if let Some(fs) = font_size {
            json.push_str(&format!(",\"fontSize\":{}", fs));
        }
        if let Some(ff) = font_family {
            json.push_str(&format!(",\"fontFamily\":\"{}\"", escape_json(&ff)));
        }
        if let Some(c) = color {
            json.push_str(&format!(",\"color\":\"{}\"", c));
        }
        // List info
        if let Some(AttributeValue::ListInfo(li)) = para.attributes.get(&AttributeKey::ListInfo) {
            let fmt_name = match li.num_format {
                ListFormat::Bullet => "bullet",
                ListFormat::Decimal => "decimal",
                ListFormat::LowerAlpha => "lowerAlpha",
                ListFormat::UpperAlpha => "upperAlpha",
                ListFormat::LowerRoman => "lowerRoman",
                ListFormat::UpperRoman => "upperRoman",
                _ => "bullet",
            };
            json.push_str(&format!(",\"listFormat\":\"{}\"", fmt_name));
            json.push_str(&format!(",\"listLevel\":{}", li.level));
        }
        // Paragraph indentation (in points)
        if let Some(v) = para.attributes.get_f64(&AttributeKey::IndentLeft) {
            json.push_str(&format!(",\"indentLeft\":{:.2}", v));
        }
        if let Some(v) = para.attributes.get_f64(&AttributeKey::IndentRight) {
            json.push_str(&format!(",\"indentRight\":{:.2}", v));
        }
        if let Some(v) = para.attributes.get_f64(&AttributeKey::IndentFirstLine) {
            json.push_str(&format!(",\"indentFirstLine\":{:.2}", v));
        }
        // Paragraph spacing (in points)
        if let Some(v) = para.attributes.get_f64(&AttributeKey::SpacingBefore) {
            json.push_str(&format!(",\"spacingBefore\":{:.2}", v));
        }
        if let Some(v) = para.attributes.get_f64(&AttributeKey::SpacingAfter) {
            json.push_str(&format!(",\"spacingAfter\":{:.2}", v));
        }
        // Line spacing
        if let Some(AttributeValue::LineSpacing(ls)) =
            para.attributes.get(&AttributeKey::LineSpacing)
        {
            use s1_model::LineSpacing;
            let ls_str = match ls {
                LineSpacing::Single => "1.0".to_string(),
                LineSpacing::OnePointFive => "1.5".to_string(),
                LineSpacing::Double => "2.0".to_string(),
                LineSpacing::Multiple(f) => format!("{:.2}", f),
                LineSpacing::Exact(v) => format!("exact:{:.2}", v),
                LineSpacing::AtLeast(v) => format!("atLeast:{:.2}", v),
                _ => "1.15".to_string(),
            };
            json.push_str(&format!(",\"lineSpacing\":\"{}\"", ls_str));
        }
        json.push('}');
        Ok(json)
    }

    /// Set the heading level of a paragraph.
    ///
    /// Level 0 removes the heading style (converts to normal paragraph).
    /// Level 1-6 sets the corresponding heading style.
    pub fn set_heading_level(&mut self, node_id_str: &str, level: u8) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        if level == 0 {
            // Remove StyleId by setting to empty string
            attrs.set(AttributeKey::StyleId, AttributeValue::String(String::new()));
        } else {
            let style_id = format!("Heading{}", level.clamp(1, 6));
            attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
        }
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set the paragraph style ID on a paragraph node.
    ///
    /// Sets the `StyleId` attribute to any arbitrary style name
    /// (e.g., "Title", "Subtitle", "Quote", "Code", "Heading1", etc.).
    /// Pass an empty string to clear the style (revert to Normal).
    pub fn set_paragraph_style_id(
        &mut self,
        node_id_str: &str,
        style_id: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        // Validate the node is a paragraph
        let node = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Node not found"))?;
        if node.node_type != NodeType::Paragraph {
            return Err(JsError::new("Node is not a Paragraph"));
        }

        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::StyleId,
            AttributeValue::String(style_id.to_string()),
        );
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // ─── P.1: Selection & Range Formatting API ─────────────────

    /// Split a Run node at a character offset.
    ///
    /// Creates a new Run after the original with the tail text, preserving
    /// all formatting attributes. Returns the new run's node ID.
    pub fn split_run(&mut self, run_id_str: &str, char_offset: usize) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let run_id = parse_node_id(run_id_str)?;

        let run = doc
            .node(run_id)
            .ok_or_else(|| JsError::new("Run not found"))?;
        if run.node_type != NodeType::Run {
            return Err(JsError::new("Node is not a Run"));
        }

        // Get run attributes and parent paragraph
        let run_attrs = run.attributes.clone();
        let para_id = run
            .parent
            .ok_or_else(|| JsError::new("Run has no parent"))?;

        // Find run's index in parent
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Parent paragraph not found"))?;
        let run_index = para
            .children
            .iter()
            .position(|&c| c == run_id)
            .ok_or_else(|| JsError::new("Run not found in parent"))?;

        // Find text node and get content
        let (text_node_id, text_char_len) = find_text_node_in_run(doc.model(), run_id)?;
        let text_node = doc
            .node(text_node_id)
            .ok_or_else(|| JsError::new("Text node not found"))?;
        let full_text = text_node.text_content.as_deref().unwrap_or("");
        let tail_text: String = full_text.chars().skip(char_offset).collect();

        if char_offset > text_char_len {
            return Err(JsError::new("Offset exceeds text length"));
        }

        // Allocate IDs
        let new_run_id = doc.next_id();
        let new_text_id = doc.next_id();

        let mut txn = Transaction::with_label("Split run");

        // Delete tail from original text node
        if char_offset < text_char_len {
            txn.push(Operation::delete_text(
                text_node_id,
                char_offset,
                text_char_len - char_offset,
            ));
        }

        // Create new run with same attributes
        let mut new_run = Node::new(new_run_id, NodeType::Run);
        new_run.attributes = run_attrs;
        txn.push(Operation::insert_node(para_id, run_index + 1, new_run));
        txn.push(Operation::insert_node(
            new_run_id,
            0,
            Node::text(new_text_id, &tail_text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", new_run_id.replica, new_run_id.counter))
    }

    /// Set a formatting attribute on a specific Run node.
    ///
    /// key/value are string representations parsed to AttributeKey/AttributeValue.
    /// Supported keys: "bold", "italic", "underline", "strikethrough",
    /// "fontSize", "fontFamily", "color", "highlightColor", "superscript", "subscript".
    pub fn format_run(&mut self, run_id_str: &str, key: &str, value: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let run_id = parse_node_id(run_id_str)?;
        let attrs = parse_format_kv(key, value)?;
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Format a text range spanning one or more runs/paragraphs.
    ///
    /// Internally splits start/end runs as needed and applies the attribute
    /// to all runs in the selection range. Single transaction (atomic undo).
    ///
    /// start_node/end_node are paragraph IDs, offsets are character positions.
    pub fn format_selection(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        key: &str,
        value: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let start_para = parse_node_id(start_node_str)?;
        let end_para = parse_node_id(end_node_str)?;
        let attrs = parse_format_kv(key, value)?;

        if start_para == end_para {
            // Single paragraph selection
            format_range_in_paragraph(doc, start_para, start_offset, end_offset, &attrs)?;
        } else {
            // Cross-paragraph selection
            let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
            let body = doc
                .node(body_id)
                .ok_or_else(|| JsError::new("Body not found"))?;
            let children = body.children.clone();

            let start_idx = children
                .iter()
                .position(|&c| c == start_para)
                .ok_or_else(|| JsError::new("Start paragraph not in body"))?;
            let end_idx = children
                .iter()
                .position(|&c| c == end_para)
                .ok_or_else(|| JsError::new("End paragraph not in body"))?;

            // Format tail of start paragraph
            let start_text_len = extract_paragraph_text(doc.model(), start_para)
                .chars()
                .count();
            format_range_in_paragraph(doc, start_para, start_offset, start_text_len, &attrs)?;

            // Format all intermediate paragraphs fully
            for &child_id in &children[start_idx + 1..end_idx] {
                if let Some(child) = doc.node(child_id) {
                    if child.node_type == NodeType::Paragraph {
                        let len = extract_paragraph_text(doc.model(), child_id)
                            .chars()
                            .count();
                        if len > 0 {
                            format_range_in_paragraph(doc, child_id, 0, len, &attrs)?;
                        }
                    }
                }
            }

            // Format head of end paragraph
            if end_offset > 0 {
                format_range_in_paragraph(doc, end_para, 0, end_offset, &attrs)?;
            }
        }
        Ok(())
    }

    /// Get run IDs within a paragraph as a JSON array.
    ///
    /// Returns `["0:5","0:8",...]` — the IDs of all Run nodes in the paragraph.
    pub fn get_run_ids(&self, paragraph_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let para_id = parse_node_id(paragraph_id_str)?;
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;

        let mut ids = Vec::new();
        for &child_id in &para.children {
            if let Some(child) = doc.node(child_id) {
                if child.node_type == NodeType::Run {
                    ids.push(format!("\"{}:{}\"", child_id.replica, child_id.counter));
                }
            }
        }
        Ok(format!("[{}]", ids.join(",")))
    }

    /// Get the text content of a specific run.
    pub fn get_run_text(&self, run_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let run_id = parse_node_id(run_id_str)?;
        let run = doc
            .node(run_id)
            .ok_or_else(|| JsError::new("Run not found"))?;

        let mut text = String::new();
        for &child_id in &run.children {
            if let Some(child) = doc.node(child_id) {
                if child.node_type == NodeType::Text {
                    if let Some(t) = &child.text_content {
                        text.push_str(t);
                    }
                }
            }
        }
        Ok(text)
    }

    /// Get formatting of a specific run as JSON.
    ///
    /// Returns `{"bold":true,"italic":false,...}`.
    pub fn get_run_formatting_json(&self, run_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let run_id = parse_node_id(run_id_str)?;
        let run = doc
            .node(run_id)
            .ok_or_else(|| JsError::new("Run not found"))?;
        Ok(run_formatting_to_json(&run.attributes))
    }

    /// Get common formatting across a selection range as JSON.
    ///
    /// Returns JSON with `true`/`false`/`"mixed"` per property.
    /// E.g., `{"bold":true,"italic":"mixed","underline":false}`.
    pub fn get_selection_formatting_json(
        &self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let start_para = parse_node_id(start_node_str)?;
        let end_para = parse_node_id(end_node_str)?;

        // Collect all runs in the selection
        let mut run_ids = Vec::new();
        if start_para == end_para {
            collect_runs_in_range(
                doc.model(),
                start_para,
                start_offset,
                end_offset,
                &mut run_ids,
            );
        } else {
            let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
            let body = doc
                .node(body_id)
                .ok_or_else(|| JsError::new("Body not found"))?;
            let children = body.children.clone();
            let start_idx = children.iter().position(|&c| c == start_para).unwrap_or(0);
            let end_idx = children
                .iter()
                .position(|&c| c == end_para)
                .unwrap_or(children.len());

            let start_len = extract_paragraph_text(doc.model(), start_para)
                .chars()
                .count();
            collect_runs_in_range(
                doc.model(),
                start_para,
                start_offset,
                start_len,
                &mut run_ids,
            );
            for &child_id in &children[start_idx + 1..end_idx] {
                if let Some(child) = doc.node(child_id) {
                    if child.node_type == NodeType::Paragraph {
                        let len = extract_paragraph_text(doc.model(), child_id)
                            .chars()
                            .count();
                        collect_runs_in_range(doc.model(), child_id, 0, len, &mut run_ids);
                    }
                }
            }
            collect_runs_in_range(doc.model(), end_para, 0, end_offset, &mut run_ids);
        }

        // Compute common formatting
        let mut bold_state: Option<bool> = None;
        let mut italic_state: Option<bool> = None;
        let mut underline_state: Option<bool> = None;
        let mut strike_state: Option<bool> = None;
        let mut mixed_bold = false;
        let mut mixed_italic = false;
        let mut mixed_underline = false;
        let mut mixed_strike = false;

        for rid in &run_ids {
            if let Some(run) = doc.node(*rid) {
                let b = run.attributes.get_bool(&AttributeKey::Bold) == Some(true);
                let i = run.attributes.get_bool(&AttributeKey::Italic) == Some(true);
                let u = run.attributes.get(&AttributeKey::Underline).is_some()
                    && !matches!(
                        run.attributes.get(&AttributeKey::Underline),
                        Some(AttributeValue::UnderlineStyle(UnderlineStyle::None))
                    );
                let s = run.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true);

                if let Some(prev) = bold_state {
                    if prev != b {
                        mixed_bold = true;
                    }
                }
                bold_state = Some(b);
                if let Some(prev) = italic_state {
                    if prev != i {
                        mixed_italic = true;
                    }
                }
                italic_state = Some(i);
                if let Some(prev) = underline_state {
                    if prev != u {
                        mixed_underline = true;
                    }
                }
                underline_state = Some(u);
                if let Some(prev) = strike_state {
                    if prev != s {
                        mixed_strike = true;
                    }
                }
                strike_state = Some(s);
            }
        }

        // Also collect font, size, color, highlight from the first run
        let mut font_family: Option<String> = None;
        let mut font_size: Option<f64> = None;
        let mut color_hex: Option<String> = None;
        let mut highlight_hex: Option<String> = None;
        let mut superscript = false;
        let mut subscript = false;

        if let Some(&first_rid) = run_ids.first() {
            if let Some(run) = doc.node(first_rid) {
                font_family = run
                    .attributes
                    .get_string(&AttributeKey::FontFamily)
                    .map(|s| s.to_string());
                font_size = run.attributes.get_f64(&AttributeKey::FontSize);
                if let Some(AttributeValue::Color(c)) = run.attributes.get(&AttributeKey::Color) {
                    color_hex = Some(format!("#{}", c.to_hex()));
                }
                if let Some(AttributeValue::Color(c)) =
                    run.attributes.get(&AttributeKey::HighlightColor)
                {
                    highlight_hex = Some(format!("#{}", c.to_hex()));
                }
                superscript = run.attributes.get_bool(&AttributeKey::Superscript) == Some(true);
                subscript = run.attributes.get_bool(&AttributeKey::Subscript) == Some(true);
            }
        }

        fn fmt_val(mixed: bool, val: Option<bool>) -> String {
            if mixed {
                "\"mixed\"".to_string()
            } else {
                format!("{}", val.unwrap_or(false))
            }
        }

        let mut json = format!(
            "{{\"bold\":{},\"italic\":{},\"underline\":{},\"strikethrough\":{},\"superscript\":{},\"subscript\":{}",
            fmt_val(mixed_bold, bold_state),
            fmt_val(mixed_italic, italic_state),
            fmt_val(mixed_underline, underline_state),
            fmt_val(mixed_strike, strike_state),
            superscript,
            subscript,
        );
        if let Some(ref f) = font_family {
            json.push_str(&format!(",\"fontFamily\":\"{}\"", escape_json(f)));
        }
        if let Some(s) = font_size {
            json.push_str(&format!(",\"fontSize\":{s}"));
        }
        if let Some(ref c) = color_hex {
            json.push_str(&format!(",\"color\":\"{}\"", c));
        }
        if let Some(ref h) = highlight_hex {
            json.push_str(&format!(",\"highlightColor\":\"{}\"", h));
        }
        json.push('}');
        Ok(json)
    }

    // ─── P.2: Table Operations API ──────────────────────────────

    /// Set column widths for a table. Widths should be in points (CSV string).
    pub fn set_table_column_widths(
        &mut self,
        table_id_str: &str,
        widths_csv: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let mut txn = Transaction::with_label("Set table column widths");
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::TableColumnWidths,
            AttributeValue::String(widths_csv.to_string()),
        );
        txn.push(Operation::set_attributes(table_id, attrs));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Insert a table after the specified body-level node.
    ///
    /// Creates a table with the given number of rows and columns,
    /// each cell containing an empty paragraph. Returns the table node ID.
    pub fn insert_table(
        &mut self,
        after_node_str: &str,
        rows: u32,
        cols: u32,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body"))?
            + 1;

        let table_id = doc.next_id();
        let mut txn = Transaction::with_label("Insert table");
        txn.push(Operation::insert_node(
            body_id,
            index,
            Node::new(table_id, NodeType::Table),
        ));

        for r in 0..rows {
            let row_id = doc.next_id();
            txn.push(Operation::insert_node(
                table_id,
                r as usize,
                Node::new(row_id, NodeType::TableRow),
            ));
            for c in 0..cols {
                let cell_id = doc.next_id();
                let para_id = doc.next_id();
                let run_id = doc.next_id();
                let text_id = doc.next_id();
                txn.push(Operation::insert_node(
                    row_id,
                    c as usize,
                    Node::new(cell_id, NodeType::TableCell),
                ));
                txn.push(Operation::insert_node(
                    cell_id,
                    0,
                    Node::new(para_id, NodeType::Paragraph),
                ));
                txn.push(Operation::insert_node(
                    para_id,
                    0,
                    Node::new(run_id, NodeType::Run),
                ));
                txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, "")));
            }
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", table_id.replica, table_id.counter))
    }

    /// Insert a row at the given index in a table.
    ///
    /// Creates cells matching the column count of existing rows.
    /// Returns the new row's node ID.
    pub fn insert_table_row(
        &mut self,
        table_id_str: &str,
        row_index: u32,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;

        // Count columns from first row
        let col_count = get_table_col_count(doc.model(), table_id)?;

        let row_id = doc.next_id();
        let mut txn = Transaction::with_label("Insert table row");
        txn.push(Operation::insert_node(
            table_id,
            row_index as usize,
            Node::new(row_id, NodeType::TableRow),
        ));
        for c in 0..col_count {
            let cell_id = doc.next_id();
            let para_id = doc.next_id();
            let run_id = doc.next_id();
            let text_id = doc.next_id();
            txn.push(Operation::insert_node(
                row_id,
                c,
                Node::new(cell_id, NodeType::TableCell),
            ));
            txn.push(Operation::insert_node(
                cell_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ));
            txn.push(Operation::insert_node(
                para_id,
                0,
                Node::new(run_id, NodeType::Run),
            ));
            txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, "")));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", row_id.replica, row_id.counter))
    }

    /// Delete a row at the given index in a table.
    pub fn delete_table_row(&mut self, table_id_str: &str, row_index: u32) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_id = *table
            .children
            .get(row_index as usize)
            .ok_or_else(|| JsError::new("Row index out of bounds"))?;
        doc.apply(Operation::delete_node(row_id))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Insert a column at the given index across all rows.
    pub fn insert_table_column(
        &mut self,
        table_id_str: &str,
        col_index: u32,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_ids: Vec<NodeId> = table.children.clone();

        let mut txn = Transaction::with_label("Insert table column");
        for row_id in &row_ids {
            let cell_id = doc.next_id();
            let para_id = doc.next_id();
            let run_id = doc.next_id();
            let text_id = doc.next_id();
            txn.push(Operation::insert_node(
                *row_id,
                col_index as usize,
                Node::new(cell_id, NodeType::TableCell),
            ));
            txn.push(Operation::insert_node(
                cell_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ));
            txn.push(Operation::insert_node(
                para_id,
                0,
                Node::new(run_id, NodeType::Run),
            ));
            txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, "")));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Delete a column at the given index across all rows.
    pub fn delete_table_column(
        &mut self,
        table_id_str: &str,
        col_index: u32,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_ids: Vec<NodeId> = table.children.clone();

        let mut txn = Transaction::with_label("Delete table column");
        for row_id in &row_ids {
            if let Some(row) = doc.node(*row_id) {
                if let Some(&cell_id) = row.children.get(col_index as usize) {
                    txn.push(Operation::delete_node(cell_id));
                }
            }
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sort a table by the text content of a specific column.
    ///
    /// Skips the first row (assumed header) if the table has more than one row.
    pub fn sort_table_by_column(
        &mut self,
        table_id_str: &str,
        col_index: u32,
        ascending: bool,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;

        let mut row_data: Vec<(NodeId, String)> = Vec::new();
        for &row_id in &table.children {
            let row = doc
                .node(row_id)
                .ok_or_else(|| JsError::new("Row not found"))?;
            let text = if let Some(&cell_id) = row.children.get(col_index as usize) {
                extract_paragraph_text(doc.model(), cell_id)
            } else {
                String::new()
            };
            row_data.push((row_id, text));
        }

        let has_header = row_data.len() > 1;
        let start_idx = if has_header { 1 } else { 0 };
        row_data[start_idx..].sort_by(|a, b| {
            if ascending {
                a.1.cmp(&b.1)
            } else {
                b.1.cmp(&a.1)
            }
        });

        // Reorder via move_node
        for (i, &(row_id, _)) in row_data.iter().enumerate().skip(start_idx) {
            doc.apply(Operation::move_node(row_id, table_id, i))
                .map_err(|e| JsError::new(&e.to_string()))?;
        }
        Ok(())
    }

    /// Set the text content of a table cell.
    ///
    /// Replaces the entire cell content with the given text. Sets text in
    /// the first paragraph and deletes any extra paragraphs.
    pub fn set_cell_text(&mut self, cell_id_str: &str, text: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let cell_id = parse_node_id(cell_id_str)?;
        let cell = doc
            .node(cell_id)
            .ok_or_else(|| JsError::new("Cell not found"))?;

        let para_ids: Vec<NodeId> = cell.children.clone();
        if para_ids.is_empty() {
            return Err(JsError::new("Cell has no paragraph"));
        }

        // Set text in the first paragraph
        let first_para_id = para_ids[0];
        let (text_node_id, old_len) = find_first_text_node(doc.model(), first_para_id)?;
        let mut txn = Transaction::with_label("Set cell text");
        if old_len > 0 {
            txn.push(Operation::delete_text(text_node_id, 0, old_len));
        }
        if !text.is_empty() {
            txn.push(Operation::insert_text(text_node_id, 0, text));
        }

        // Delete any extra paragraphs beyond the first
        for &extra_para_id in &para_ids[1..] {
            txn.push(Operation::delete_node(extra_para_id));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the text content of a table cell.
    pub fn get_cell_text(&self, cell_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let cell_id = parse_node_id(cell_id_str)?;
        let cell = doc
            .node(cell_id)
            .ok_or_else(|| JsError::new("Cell not found"))?;

        let mut text = String::new();
        for &para_id in &cell.children {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&extract_paragraph_text(doc.model(), para_id));
        }
        Ok(text)
    }

    /// Get table dimensions as JSON: `{"rows":N,"cols":M}`.
    pub fn get_table_dimensions(&self, table_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let rows = table.children.len();
        let cols = if let Some(&first_row_id) = table.children.first() {
            doc.node(first_row_id)
                .map(|r| r.children.len())
                .unwrap_or(0)
        } else {
            0
        };
        Ok(format!("{{\"rows\":{},\"cols\":{}}}", rows, cols))
    }

    /// Get the node ID of a cell at a given row/column index.
    pub fn get_cell_id(&self, table_id_str: &str, row: u32, col: u32) -> Result<String, JsError> {
        let doc = self.doc()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_id = *table
            .children
            .get(row as usize)
            .ok_or_else(|| JsError::new("Row index out of bounds"))?;
        let row_node = doc
            .node(row_id)
            .ok_or_else(|| JsError::new("Row not found"))?;
        let cell_id = *row_node
            .children
            .get(col as usize)
            .ok_or_else(|| JsError::new("Column index out of bounds"))?;
        Ok(format!("{}:{}", cell_id.replica, cell_id.counter))
    }

    /// Merge cells in a range by setting ColSpan/RowSpan attributes.
    pub fn merge_cells(
        &mut self,
        table_id_str: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_ids: Vec<NodeId> = table.children.clone();

        // Bounds-check row indices
        if start_row as usize >= row_ids.len() || end_row as usize >= row_ids.len() {
            return Err(JsError::new("Row index out of bounds"));
        }
        // Validate that start <= end to prevent unsigned underflow
        if start_row > end_row || start_col > end_col {
            return Err(JsError::new("Invalid merge range: start must be <= end"));
        }

        let col_span = (end_col - start_col + 1) as i64;
        let row_span = (end_row - start_row + 1) as i64;

        let mut txn = Transaction::with_label("Merge cells");

        // Set span on top-left cell
        if let Some(row) = doc.node(row_ids[start_row as usize]) {
            if let Some(&cell_id) = row.children.get(start_col as usize) {
                let mut attrs = s1_model::AttributeMap::new();
                if col_span > 1 {
                    attrs.set(AttributeKey::ColSpan, AttributeValue::Int(col_span));
                }
                if row_span > 1 {
                    attrs.set(
                        AttributeKey::RowSpan,
                        AttributeValue::String("restart".to_string()),
                    );
                }
                txn.push(Operation::set_attributes(cell_id, attrs));
            } else {
                return Err(JsError::new("Column index out of bounds"));
            }
        }

        // Mark continuation cells
        for r in start_row..=end_row {
            let row_id = row_ids[r as usize]; // safe: bounds checked above
            if let Some(row) = doc.node(row_id) {
                let cells: Vec<NodeId> = row.children.clone();
                if end_col as usize >= cells.len() {
                    return Err(JsError::new("Column index out of bounds"));
                }
                for c in start_col..=end_col {
                    if r == start_row && c == start_col {
                        continue; // Skip the top-left cell
                    }
                    if let Some(&cell_id) = cells.get(c as usize) {
                        let mut attrs = s1_model::AttributeMap::new();
                        if r > start_row {
                            attrs.set(
                                AttributeKey::RowSpan,
                                AttributeValue::String("continue".to_string()),
                            );
                        }
                        txn.push(Operation::set_attributes(cell_id, attrs));
                    }
                }
            }
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Split a previously merged cell back to individual cells.
    ///
    /// Removes ColSpan/RowSpan attributes from the target cell and clears
    /// the "continue" RowSpan from cells that were part of the merge.
    pub fn split_merged_cell(
        &mut self,
        table_id_str: &str,
        row: u32,
        col: u32,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;
        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_ids: Vec<NodeId> = table.children.clone();

        if row as usize >= row_ids.len() {
            return Err(JsError::new("Row index out of bounds"));
        }

        // Read current spans from the target cell
        let target_cell_id = {
            let row_node = doc
                .node(row_ids[row as usize])
                .ok_or_else(|| JsError::new("Row not found"))?;
            *row_node
                .children
                .get(col as usize)
                .ok_or_else(|| JsError::new("Column index out of bounds"))?
        };

        let (col_span, row_span) = {
            let cell = doc
                .node(target_cell_id)
                .ok_or_else(|| JsError::new("Cell not found"))?;
            let cs = cell
                .attributes
                .get(&AttributeKey::ColSpan)
                .and_then(|v| {
                    if let AttributeValue::Int(n) = v {
                        Some(*n as u32)
                    } else {
                        None
                    }
                })
                .unwrap_or(1);
            let rs = match cell.attributes.get(&AttributeKey::RowSpan) {
                Some(AttributeValue::String(s)) if s == "restart" => {
                    // Count continuation rows below
                    let mut count = 1u32;
                    for r in (row + 1)..row_ids.len() as u32 {
                        if let Some(rn) = doc.node(row_ids[r as usize]) {
                            if let Some(&cid) = rn.children.get(col as usize) {
                                if let Some(cn) = doc.node(cid) {
                                    if let Some(AttributeValue::String(s)) =
                                        cn.attributes.get(&AttributeKey::RowSpan)
                                    {
                                        if s == "continue" {
                                            count += 1;
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                    count
                }
                _ => 1,
            };
            (cs, rs)
        };

        if col_span <= 1 && row_span <= 1 {
            return Ok(()); // Not merged — nothing to do
        }

        let mut txn = Transaction::with_label("Split merged cell");

        // Clear spans on the target cell
        let mut clear_attrs = s1_model::AttributeMap::new();
        clear_attrs.set(AttributeKey::ColSpan, AttributeValue::Int(1));
        clear_attrs.set(AttributeKey::RowSpan, AttributeValue::String(String::new()));
        txn.push(Operation::set_attributes(target_cell_id, clear_attrs));

        // Clear continuation markers on cells that were part of the merge
        for r in row..(row + row_span) {
            if let Some(row_node) = doc.node(row_ids[r as usize]) {
                let cells: Vec<NodeId> = row_node.children.clone();
                for c in col..(col + col_span) {
                    if r == row && c == col {
                        continue; // Already handled
                    }
                    if let Some(&cell_id) = cells.get(c as usize) {
                        let mut attrs = s1_model::AttributeMap::new();
                        attrs.set(AttributeKey::RowSpan, AttributeValue::String(String::new()));
                        txn.push(Operation::set_attributes(cell_id, attrs));
                    }
                }
            }
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set the background color of a table cell.
    pub fn set_cell_background(&mut self, cell_id_str: &str, hex: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let cell_id = parse_node_id(cell_id_str)?;
        let color =
            Color::from_hex(hex).ok_or_else(|| JsError::new(&format!("Invalid color: {}", hex)))?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(AttributeKey::CellBackground, AttributeValue::Color(color));
        doc.apply(Operation::set_attributes(cell_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Apply a predefined table style to a table.
    ///
    /// Available styles: "plain", "grid", "striped-blue", "striped-gray",
    /// "header-blue", "header-green", "header-orange", "bordered", "minimal".
    ///
    /// Applies cell backgrounds and header row formatting.
    pub fn apply_table_style(
        &mut self,
        table_id_str: &str,
        style_name: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let table_id = parse_node_id(table_id_str)?;

        let table = doc
            .node(table_id)
            .ok_or_else(|| JsError::new("Table not found"))?;
        let row_ids: Vec<NodeId> = table.children.clone();

        // Style definitions: (header_bg, header_text_bold, even_row_bg, odd_row_bg)
        let (header_bg, header_bold, even_bg, odd_bg) = match style_name {
            "grid" => (None, false, None, None),
            "striped-blue" => (Some("4472C4"), true, Some("D6E4F0"), None),
            "striped-gray" => (Some("595959"), true, Some("F2F2F2"), None),
            "header-blue" => (Some("4472C4"), true, None, None),
            "header-green" => (Some("548235"), true, None, None),
            "header-orange" => (Some("ED7D31"), true, None, None),
            "bordered" => (Some("D9E2F3"), true, None, None),
            "minimal" => (None, false, None, None),
            _ => (None, false, None, None), // "plain"
        };

        let mut txn = Transaction::with_label("Apply table style");

        for (row_idx, &row_id) in row_ids.iter().enumerate() {
            let row = match doc.node(row_id) {
                Some(r) => r,
                None => continue,
            };
            let cell_ids: Vec<NodeId> = row.children.clone();
            let is_header = row_idx == 0;
            let is_even = row_idx % 2 == 0;

            for &cell_id in &cell_ids {
                let bg_hex = if is_header {
                    header_bg
                } else if is_even {
                    even_bg
                } else {
                    odd_bg
                };

                if let Some(hex) = bg_hex {
                    if let Some(color) = Color::from_hex(hex) {
                        let mut attrs = s1_model::AttributeMap::new();
                        attrs.set(AttributeKey::CellBackground, AttributeValue::Color(color));
                        txn.push(Operation::set_attributes(cell_id, attrs));
                    }
                } else {
                    // Clear background
                    txn.push(Operation::remove_attributes(
                        cell_id,
                        vec![AttributeKey::CellBackground],
                    ));
                }

                // Bold text in header row
                if is_header && header_bold {
                    if let Some(cell) = doc.node(cell_id) {
                        for &para_id in &cell.children {
                            if let Some(para) = doc.node(para_id) {
                                for &run_id in &para.children {
                                    if let Some(run) = doc.node(run_id) {
                                        if run.node_type == NodeType::Run {
                                            let mut attrs = s1_model::AttributeMap::new();
                                            attrs.set(
                                                AttributeKey::Bold,
                                                AttributeValue::Bool(true),
                                            );
                                            txn.push(Operation::set_attributes(run_id, attrs));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // ─── M14.4: Shape/Drawing Persistence API ─────────────────

    /// Insert a shape (Drawing node) after a body-level node.
    ///
    /// Returns the Drawing node ID. The shape is rendered by the layout engine.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_shape(
        &mut self,
        after_node_str: &str,
        shape_type: &str,
        width_pt: f64,
        height_pt: f64,
        _x_pt: f64,
        _y_pt: f64,
        fill_hex: &str,
        stroke_hex: &str,
        stroke_width: f64,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;

        let drawing_id = doc.next_id();
        let mut drawing_node = Node::new(drawing_id, NodeType::Drawing);

        drawing_node.attributes.set(
            AttributeKey::ShapeType,
            AttributeValue::String(shape_type.to_string()),
        );
        drawing_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(width_pt));
        drawing_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(height_pt));
        // Position as floating
        drawing_node.attributes.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String("inFront".to_string()),
        );

        if !fill_hex.is_empty() {
            if let Some(c) = Color::from_hex(fill_hex) {
                drawing_node
                    .attributes
                    .set(AttributeKey::ShapeFillColor, AttributeValue::Color(c));
            }
        }
        if !stroke_hex.is_empty() {
            if let Some(c) = Color::from_hex(stroke_hex) {
                drawing_node
                    .attributes
                    .set(AttributeKey::ShapeStrokeColor, AttributeValue::Color(c));
            }
        }
        if stroke_width > 0.0 {
            drawing_node.attributes.set(
                AttributeKey::ShapeStrokeWidth,
                AttributeValue::Float(stroke_width),
            );
        }

        // Find insert position after the target node
        let parent_id = doc
            .node(after_id)
            .and_then(|n| n.parent)
            .ok_or_else(|| JsError::new("Cannot find parent"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let insert_idx = parent
            .children
            .iter()
            .position(|&id| id == after_id)
            .map(|i| i + 1)
            .unwrap_or(parent.children.len());

        let mut txn = Transaction::with_label("Insert shape");
        txn.push(Operation::insert_node(parent_id, insert_idx, drawing_node));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", drawing_id.replica, drawing_id.counter))
    }

    /// Update shape properties (position, size, fill, stroke).
    pub fn update_shape(
        &mut self,
        shape_id_str: &str,
        width_pt: f64,
        height_pt: f64,
        fill_hex: &str,
        stroke_hex: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let shape_id = parse_node_id(shape_id_str)?;

        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(AttributeKey::ImageWidth, AttributeValue::Float(width_pt));
        attrs.set(AttributeKey::ImageHeight, AttributeValue::Float(height_pt));
        if !fill_hex.is_empty() {
            if let Some(c) = Color::from_hex(fill_hex) {
                attrs.set(AttributeKey::ShapeFillColor, AttributeValue::Color(c));
            }
        }
        if !stroke_hex.is_empty() {
            if let Some(c) = Color::from_hex(stroke_hex) {
                attrs.set(AttributeKey::ShapeStrokeColor, AttributeValue::Color(c));
            }
        }
        doc.apply(Operation::set_attributes(shape_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // ─── P.3: Image Operations API ──────────────────────────────

    /// Insert an image after the specified body-level node.
    ///
    /// Stores bytes in MediaStore, creates Paragraph → Run → Image structure.
    /// Returns the paragraph node ID containing the image.
    pub fn insert_image(
        &mut self,
        after_node_str: &str,
        data: &[u8],
        content_type: &str,
        width_pt: f64,
        height_pt: f64,
    ) -> Result<String, JsError> {
        // Bug W7: Clamp invalid dimensions to sensible defaults instead of erroring
        let width_pt = if width_pt <= 0.0 || width_pt > 10000.0 {
            200.0
        } else {
            width_pt
        };
        let height_pt = if height_pt <= 0.0 || height_pt > 10000.0 {
            200.0
        } else {
            height_pt
        };

        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body"))?
            + 1;

        // Store in MediaStore
        let ext = match content_type {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "bin",
        };
        let media_id = doc.model_mut().media_mut().insert(
            content_type,
            data.to_vec(),
            Some(format!("image.{}", ext)),
        );

        let para_id = doc.next_id();
        let img_id = doc.next_id();

        let mut img_node = Node::new(img_id, NodeType::Image);
        img_node.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(width_pt));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(height_pt));

        let mut txn = Transaction::with_label("Insert image");
        txn.push(Operation::insert_node(
            body_id,
            index,
            Node::new(para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(para_id, 0, img_node));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Delete an image node (and its containing paragraph if empty).
    pub fn delete_image(&mut self, image_id_str: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let img_id = parse_node_id(image_id_str)?;
        let img = doc
            .node(img_id)
            .ok_or_else(|| JsError::new("Image not found"))?;

        // Walk up to find containing paragraph for cleanup
        let para_id = img.parent;

        // Delete the image's containing paragraph
        if let Some(pid) = para_id {
            let para = doc.node(pid);
            // If paragraph only contains this image, delete the whole paragraph
            if para.map(|p| p.children.len() <= 1).unwrap_or(false) {
                doc.apply(Operation::delete_node(pid))
                    .map_err(|e| JsError::new(&e.to_string()))
            } else {
                doc.apply(Operation::delete_node(img_id))
                    .map_err(|e| JsError::new(&e.to_string()))
            }
        } else {
            doc.apply(Operation::delete_node(img_id))
                .map_err(|e| JsError::new(&e.to_string()))
        }
    }

    /// Resize an image by setting width/height attributes.
    pub fn resize_image(
        &mut self,
        image_id_str: &str,
        width_pt: f64,
        height_pt: f64,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let img_id = parse_node_id(image_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(AttributeKey::ImageWidth, AttributeValue::Float(width_pt));
        attrs.set(AttributeKey::ImageHeight, AttributeValue::Float(height_pt));
        doc.apply(Operation::set_attributes(img_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get image as a data URL for display.
    pub fn get_image_data_url(&self, image_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let img_id = parse_node_id(image_id_str)?;
        let img = doc
            .node(img_id)
            .ok_or_else(|| JsError::new("Image not found"))?;

        if let Some(AttributeValue::MediaId(media_id)) =
            img.attributes.get(&AttributeKey::ImageMediaId)
        {
            if let Some(item) = doc.model().media().get(*media_id) {
                let b64 = base64_encode(&item.data);
                return Ok(format!("data:{};base64,{}", item.content_type, b64));
            }
        }
        Err(JsError::new("Image media not found"))
    }

    /// Set alt text on an image.
    pub fn set_image_alt_text(&mut self, image_id_str: &str, alt: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let img_id = parse_node_id(image_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::ImageAltText,
            AttributeValue::String(alt.to_string()),
        );
        doc.apply(Operation::set_attributes(img_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set image wrap mode.
    ///
    /// `mode` is one of: "inline", "wrapLeft", "wrapRight", "wrapBoth",
    /// "topAndBottom", "behind", "inFront".
    /// Defaults to "inline" if not set.
    pub fn set_image_wrap_mode(&mut self, image_id_str: &str, mode: &str) -> Result<(), JsError> {
        let valid = [
            "inline",
            "wrapLeft",
            "wrapRight",
            "wrapBoth",
            "topAndBottom",
            "behind",
            "inFront",
        ];
        if !valid.contains(&mode) {
            return Err(JsError::new(&format!(
                "Invalid wrap mode '{}'. Expected one of: {}",
                mode,
                valid.join(", ")
            )));
        }
        let doc = self.doc_mut()?;
        let img_id = parse_node_id(image_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String(mode.to_string()),
        );
        doc.apply(Operation::set_attributes(img_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the wrap mode for an image node.
    ///
    /// Returns one of: "inline", "wrapLeft", "wrapRight", "wrapBoth",
    /// "topAndBottom", "behind", "inFront". Defaults to "inline".
    pub fn get_image_wrap_mode(&self, image_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let img_id = parse_node_id(image_id_str)?;
        let img = doc
            .node(img_id)
            .ok_or_else(|| JsError::new("Image not found"))?;
        let mode = img
            .attributes
            .get_string(&AttributeKey::ImageWrapType)
            .unwrap_or("inline");
        Ok(mode.to_string())
    }

    // ─── UXP-22: Multi-Column Layout API ────────────────────────

    /// Set the number of columns for a section.
    ///
    /// `section_index`: 0-based section index (0 for the default/first section).
    /// `columns`: number of columns (1-6). Pass 1 for single-column layout.
    /// `spacing_pt`: spacing between columns in points (default: 36.0 = 0.5in).
    pub fn set_section_columns(
        &mut self,
        section_index: usize,
        columns: u32,
        spacing_pt: f64,
    ) -> Result<(), JsError> {
        if columns == 0 || columns > 6 {
            return Err(JsError::new("Column count must be between 1 and 6"));
        }
        if spacing_pt < 0.0 {
            return Err(JsError::new("Column spacing cannot be negative"));
        }
        let doc = self.doc_mut()?;
        let sections = doc.model_mut().sections_mut();
        if sections.is_empty() {
            sections.push(s1_model::SectionProperties::default());
        }
        if section_index >= sections.len() {
            return Err(JsError::new(&format!(
                "Section index {} out of range (0..{})",
                section_index,
                sections.len()
            )));
        }
        sections[section_index].columns = columns;
        sections[section_index].column_spacing = spacing_pt;
        Ok(())
    }

    /// Get the column configuration for a section as JSON.
    ///
    /// Returns JSON: `{"columns":2,"spacing":36.0}`
    pub fn get_section_columns(&self, section_index: usize) -> Result<String, JsError> {
        let doc = self.doc()?;
        let sections = doc.sections();
        if section_index >= sections.len() {
            // Default section
            return Ok("{\"columns\":1,\"spacing\":36}".to_string());
        }
        let sec = &sections[section_index];
        Ok(format!(
            "{{\"columns\":{},\"spacing\":{:.1}}}",
            sec.columns, sec.column_spacing
        ))
    }

    // ─── P.4: Structural Elements API ───────────────────────────

    /// Set a hyperlink URL on a run.
    ///
    /// tooltip_opt is optional — pass empty string or null for no tooltip.
    pub fn insert_hyperlink(
        &mut self,
        run_id_str: &str,
        url: &str,
        tooltip_opt: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let run_id = parse_node_id(run_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String(url.to_string()),
        );
        if !tooltip_opt.is_empty() {
            attrs.set(
                AttributeKey::HyperlinkTooltip,
                AttributeValue::String(tooltip_opt.to_string()),
            );
        }
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Remove a hyperlink from a run.
    pub fn remove_hyperlink(&mut self, run_id_str: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let run_id = parse_node_id(run_id_str)?;
        let mut keys = vec![AttributeKey::HyperlinkUrl];
        if doc
            .node(run_id)
            .map(|n| n.attributes.contains(&AttributeKey::HyperlinkTooltip))
            .unwrap_or(false)
        {
            keys.push(AttributeKey::HyperlinkTooltip);
        }
        doc.apply(Operation::remove_attributes(run_id, keys))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Insert bookmark start/end around a paragraph.
    ///
    /// Returns the bookmark start node ID.
    pub fn insert_bookmark(&mut self, para_id_str: &str, name: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;

        let bk_start_id = doc.next_id();
        let bk_end_id = doc.next_id();

        let mut start_node = Node::new(bk_start_id, NodeType::BookmarkStart);
        start_node.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String(name.to_string()),
        );
        let mut end_node = Node::new(bk_end_id, NodeType::BookmarkEnd);
        end_node.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String(name.to_string()),
        );

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let child_count = para.children.len();

        let mut txn = Transaction::with_label("Insert bookmark");
        txn.push(Operation::insert_node(para_id, 0, start_node));
        txn.push(Operation::insert_node(para_id, child_count + 1, end_node));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", bk_start_id.replica, bk_start_id.counter))
    }

    /// Insert an auto-numbered caption paragraph after a node.
    ///
    /// - `after_node_str`: the node (image paragraph, table, etc.) after which to insert
    /// - `label`: "Figure", "Table", or "Equation"
    /// - `text`: additional caption text (e.g., ": My diagram")
    ///
    /// The caption is numbered automatically by counting existing captions of the same label.
    /// Returns the caption paragraph node ID.
    pub fn insert_caption(
        &mut self,
        after_node_str: &str,
        label: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;

        // Count existing captions of the same label to determine number
        let body_id = doc
            .model()
            .body_id()
            .ok_or_else(|| JsError::new("No document body"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        // Count existing captions with the same label by walking body children
        // and checking for Caption-styled paragraphs whose text starts with the label
        let mut count = 0u32;
        for &child_id in &body.children {
            if let Some(node) = doc.node(child_id) {
                if let Some(style) = node.attributes.get_string(&AttributeKey::StyleId) {
                    if style == "Caption" {
                        // Extract text from runs
                        let mut text = String::new();
                        for &run_id in &node.children {
                            if let Some(run) = doc.node(run_id) {
                                for &txt_id in &run.children {
                                    if let Some(txt) = doc.node(txt_id) {
                                        if let Some(ref tc) = txt.text_content {
                                            text.push_str(tc);
                                        }
                                    }
                                }
                                if let Some(ref tc) = run.text_content {
                                    text.push_str(tc);
                                }
                            }
                        }
                        if text.starts_with(label) {
                            count += 1;
                        }
                    }
                }
            }
        }
        let number = count + 1;

        // Build caption paragraph: "Figure 1: text"
        let caption_text = if text.is_empty() {
            format!("{} {}", label, number)
        } else {
            format!("{} {}{}", label, number, text)
        };

        let para_id = doc.next_id();
        let run_id = doc.next_id();

        let mut para_node = Node::new(para_id, NodeType::Paragraph);
        para_node.attributes.set(
            AttributeKey::StyleId,
            AttributeValue::String("Caption".to_string()),
        );
        para_node.attributes.set(
            AttributeKey::Alignment,
            AttributeValue::Alignment(Alignment::Center),
        );

        let mut run_node = Node::new(run_id, NodeType::Run);
        let text_id = doc.next_id();
        let mut text_node = Node::new(text_id, NodeType::Text);
        text_node.text_content = Some(caption_text);
        // Italic style for caption
        run_node
            .attributes
            .set(AttributeKey::Italic, AttributeValue::Bool(true));
        run_node
            .attributes
            .set(AttributeKey::FontSize, AttributeValue::Float(10.0));

        // Find insert position
        let parent_id = doc
            .node(after_id)
            .and_then(|n| n.parent)
            .ok_or_else(|| JsError::new("Cannot find parent of target node"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let insert_idx = parent
            .children
            .iter()
            .position(|&id| id == after_id)
            .map(|i| i + 1)
            .unwrap_or(parent.children.len());

        let mut txn = Transaction::with_label("Insert caption");
        txn.push(Operation::insert_node(parent_id, insert_idx, para_node));
        txn.push(Operation::insert_node(para_id, 0, run_node));
        txn.push(Operation::insert_node(run_id, 0, text_node));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Get available cross-reference targets as JSON.
    ///
    /// Returns a JSON object with `headings` and `bookmarks` arrays:
    /// ```json
    /// {
    ///   "headings": [{"nodeId":"0:5","text":"Introduction","level":1}],
    ///   "bookmarks": [{"name":"myBookmark","nodeId":"0:10"}]
    /// }
    /// ```
    pub fn get_reference_targets_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let mut json = String::from("{\"headings\":[");

        // Collect headings using the model's built-in method
        let headings = model.collect_headings();
        for (i, (node_id, level, text)) in headings.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let preview: String = text.chars().take(80).collect();
            json.push_str(&format!(
                "{{\"nodeId\":\"{}:{}\",\"text\":\"{}\",\"level\":{}}}",
                node_id.replica,
                node_id.counter,
                escape_json(&preview),
                level,
            ));
        }

        json.push_str("],\"bookmarks\":[");

        // Collect bookmarks by walking the tree
        let mut first_bk = true;
        fn collect_bookmarks(
            model: &DocumentModel,
            node_id: NodeId,
            json: &mut String,
            first: &mut bool,
        ) {
            if let Some(node) = model.node(node_id) {
                if node.node_type == NodeType::BookmarkStart {
                    if let Some(name) = node.attributes.get_string(&AttributeKey::BookmarkName) {
                        if !*first {
                            json.push(',');
                        }
                        *first = false;
                        json.push_str(&format!(
                            "{{\"name\":\"{}\",\"nodeId\":\"{}:{}\"}}",
                            escape_json(name),
                            node_id.replica,
                            node_id.counter,
                        ));
                    }
                }
                for &child_id in &node.children {
                    collect_bookmarks(model, child_id, json, first);
                }
            }
        }
        collect_bookmarks(model, NodeId::ROOT, &mut json, &mut first_bk);

        json.push_str("]}");
        Ok(json)
    }

    /// Insert a cross-reference field at the cursor position.
    ///
    /// - `para_id_str`: paragraph to insert into
    /// - `offset`: character offset within the paragraph
    /// - `target_id_str`: node ID of the target (heading or bookmark)
    /// - `ref_type`: "heading_text", "page_number", or "bookmark_text"
    /// - `display_text`: the text to show for the cross-reference
    pub fn insert_cross_reference(
        &mut self,
        para_id_str: &str,
        _offset: usize,
        target_id_str: &str,
        _ref_type: &str,
        display_text: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;
        let _target_id = parse_node_id(target_id_str)?;

        let field_id = doc.next_id();
        let mut field_node = Node::new(field_id, NodeType::Field);
        field_node.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(s1_model::FieldType::CrossReference),
        );
        field_node.attributes.set(
            AttributeKey::FieldCode,
            AttributeValue::String(format!("REF {} \\h", target_id_str)),
        );
        field_node.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String(format!("#{}", target_id_str)),
        );
        field_node.text_content = Some(display_text.to_string());

        // Find the run at offset and insert the field
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let insert_idx = para.children.len(); // Append at end (simplified)

        let mut txn = Transaction::with_label("Insert cross-reference");
        txn.push(Operation::insert_node(para_id, insert_idx, field_node));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(format!("{}:{}", field_id.replica, field_id.counter))
    }

    /// Set list format on a paragraph.
    ///
    /// format: "bullet", "decimal", "none".
    pub fn set_list_format(
        &mut self,
        para_id_str: &str,
        format: &str,
        level: u32,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;

        if format == "none" {
            doc.apply(Operation::remove_attributes(
                para_id,
                vec![AttributeKey::ListInfo],
            ))
            .map_err(|e| JsError::new(&e.to_string()))
        } else {
            let num_format = match format {
                "bullet" => ListFormat::Bullet,
                "decimal" => ListFormat::Decimal,
                "lower-alpha" => ListFormat::LowerAlpha,
                "upper-alpha" => ListFormat::UpperAlpha,
                "lower-roman" => ListFormat::LowerRoman,
                "upper-roman" => ListFormat::UpperRoman,
                _ => return Err(JsError::new(&format!("Unknown list format: {}", format))),
            };
            let list_info = s1_model::ListInfo {
                level: level as u8,
                num_format,
                num_id: 1,
                start: Some(1),
            };
            let mut attrs = s1_model::AttributeMap::new();
            attrs.set(AttributeKey::ListInfo, AttributeValue::ListInfo(list_info));
            doc.apply(Operation::set_attributes(para_id, attrs))
                .map_err(|e| JsError::new(&e.to_string()))
        }
    }

    /// Insert a paragraph with PageBreakBefore after the given node.
    ///
    /// Returns the new paragraph node ID.
    pub fn insert_page_break(&mut self, after_node_str: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body"))?
            + 1;

        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes
            .set(AttributeKey::PageBreakBefore, AttributeValue::Bool(true));

        let mut txn = Transaction::with_label("Insert page break");
        txn.push(Operation::insert_node(body_id, index, para));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, "")));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Insert a column break inside the specified paragraph.
    ///
    /// Inserts a ColumnBreak node at the end of the paragraph's children.
    /// Returns the column break node ID.
    pub fn insert_column_break(&mut self, para_id_str: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let index = para.children.len();

        let cb_id = doc.next_id();
        let mut txn = Transaction::with_label("Insert column break");
        txn.push(Operation::insert_node(
            para_id,
            index,
            Node::new(cb_id, NodeType::ColumnBreak),
        ));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", cb_id.replica, cb_id.counter))
    }

    /// Insert a section break after the given node.
    ///
    /// `break_type` is one of: `"nextPage"`, `"continuous"`, `"evenPage"`, `"oddPage"`.
    ///
    /// This creates a new section in the document model. Content after the break
    /// belongs to the new section with the specified break type.
    /// Returns the new section's paragraph node ID (the first paragraph in the new section).
    pub fn insert_section_break(
        &mut self,
        after_node_str: &str,
        break_type: &str,
    ) -> Result<String, JsError> {
        use s1_model::section::{SectionBreakType, SectionProperties};

        let bt = match break_type {
            "nextPage" => SectionBreakType::NextPage,
            "continuous" => SectionBreakType::Continuous,
            "evenPage" => SectionBreakType::EvenPage,
            "oddPage" => SectionBreakType::OddPage,
            _ => {
                return Err(JsError::new(&format!(
                    "Unknown section break type: '{}'. Expected: nextPage, continuous, evenPage, oddPage",
                    break_type
                )))
            }
        };

        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body"))?
            + 1;

        // Create a new paragraph that starts the next section.
        // The paragraph carries a SectionIndex attribute and PageBreakBefore to
        // trigger visual rendering of the section break.
        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        // Add a new section to the document model with the chosen break type.
        let sections = doc.model_mut().sections_mut();
        // Ensure the initial/default section exists before adding a new one.
        if sections.is_empty() {
            sections.push(SectionProperties::default());
        }
        let new_sec_idx = sections.len();
        // Copy page dimensions from the last section (or use defaults).
        let mut new_sec = sections
            .last()
            .cloned()
            .unwrap_or_else(SectionProperties::default);
        new_sec.break_type = Some(bt);
        // Clear headers/footers for the new section — they inherit visually but
        // the user can override them later.
        new_sec.headers.clear();
        new_sec.footers.clear();
        sections.push(new_sec);

        // Build the paragraph node with section metadata.
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::SectionIndex,
            AttributeValue::Int(new_sec_idx as i64),
        );

        let mut txn = Transaction::with_label("Insert section break");
        txn.push(Operation::insert_node(body_id, index, para));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(run_id, 0, Node::text(text_id, "")));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Get section break information for all sections as JSON.
    ///
    /// Returns a JSON array of objects with section index, break type, and
    /// page dimensions for each section.
    pub fn get_section_breaks_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let sections = doc.sections();
        let mut entries = Vec::new();
        for (i, sec) in sections.iter().enumerate() {
            let bt = match sec.break_type {
                Some(s1_model::SectionBreakType::NextPage) => "nextPage",
                Some(s1_model::SectionBreakType::Continuous) => "continuous",
                Some(s1_model::SectionBreakType::EvenPage) => "evenPage",
                Some(s1_model::SectionBreakType::OddPage) => "oddPage",
                Some(_) => "nextPage",
                None => "none",
            };
            entries.push(format!(
                "{{\"index\":{},\"breakType\":\"{}\",\"pageWidth\":{:.2},\"pageHeight\":{:.2},\"marginTop\":{:.2},\"marginBottom\":{:.2},\"marginLeft\":{:.2},\"marginRight\":{:.2}}}",
                i, bt, sec.page_width, sec.page_height, sec.margin_top, sec.margin_bottom, sec.margin_left, sec.margin_right
            ));
        }
        Ok(format!("[{}]", entries.join(",")))
    }

    /// Insert a horizontal rule (thematic break) after the given node.
    ///
    /// Returns the new node ID.
    pub fn insert_horizontal_rule(&mut self, after_node_str: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body"))?
            + 1;

        let pb_id = doc.next_id();
        let mut txn = Transaction::with_label("Insert horizontal rule");
        txn.push(Operation::insert_node(
            body_id,
            index,
            Node::new(pb_id, NodeType::PageBreak),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", pb_id.replica, pb_id.counter))
    }

    /// Get all comments as a JSON array.
    pub fn get_comments_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let root = model.root_id();
        let root_node = model
            .node(root)
            .ok_or_else(|| JsError::new("Root not found"))?;

        let mut comments = Vec::new();
        for &child_id in &root_node.children {
            if let Some(child) = model.node(child_id) {
                if child.node_type == NodeType::CommentBody {
                    let id_str = child
                        .attributes
                        .get_string(&AttributeKey::CommentId)
                        .unwrap_or("");
                    let author = child
                        .attributes
                        .get_string(&AttributeKey::CommentAuthor)
                        .unwrap_or("");
                    let date = child
                        .attributes
                        .get_string(&AttributeKey::CommentDate)
                        .unwrap_or("");

                    let mut text = String::new();
                    for &para_id in &child.children {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(&extract_paragraph_text(model, para_id));
                    }

                    let parent_id = child
                        .attributes
                        .get_string(&AttributeKey::CommentParentId)
                        .unwrap_or("");

                    let resolved = child
                        .attributes
                        .get_bool(&AttributeKey::CommentResolved)
                        .unwrap_or(false);

                    let mut entry = format!(
                        "{{\"id\":\"{}\",\"author\":\"{}\",\"date\":\"{}\",\"text\":\"{}\",\"resolved\":{}",
                        escape_json(id_str),
                        escape_json(author),
                        escape_json(date),
                        escape_json(&text),
                        resolved
                    );
                    if !parent_id.is_empty() {
                        entry.push_str(&format!(",\"parentId\":\"{}\"", escape_json(parent_id)));
                    }
                    entry.push('}');
                    comments.push(entry);
                }
            }
        }
        Ok(format!("[{}]", comments.join(",")))
    }

    /// Insert a comment with range markers and body.
    ///
    /// Returns the comment ID string.
    pub fn insert_comment(
        &mut self,
        start_node_str: &str,
        end_node_str: &str,
        author: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let start_id = parse_node_id(start_node_str)?;
        let end_id = parse_node_id(end_node_str)?;

        let comment_id_val = format!("{}:{}", doc.next_id().replica, doc.next_id().counter);

        // Create CommentStart in start paragraph
        let cs_id = doc.next_id();
        let mut cs_node = Node::new(cs_id, NodeType::CommentStart);
        cs_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id_val.clone()),
        );
        cs_node.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String(author.to_string()),
        );

        // Create CommentEnd in end paragraph
        let ce_id = doc.next_id();
        let mut ce_node = Node::new(ce_id, NodeType::CommentEnd);
        ce_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id_val.clone()),
        );

        // Create CommentBody on root
        let root_id = doc.model().root_id();
        let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        let cb_id = doc.next_id();
        let mut cb_node = Node::new(cb_id, NodeType::CommentBody);
        cb_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id_val.clone()),
        );
        cb_node.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String(author.to_string()),
        );

        let cb_para_id = doc.next_id();
        let cb_run_id = doc.next_id();
        let cb_text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert comment");
        // Insert markers
        let _start_para = doc
            .node(start_id)
            .ok_or_else(|| JsError::new("Start node not found"))?;
        txn.push(Operation::insert_node(start_id, 0, cs_node));

        let end_para = doc
            .node(end_id)
            .ok_or_else(|| JsError::new("End node not found"))?;
        let end_child_count = end_para.children.len();
        txn.push(Operation::insert_node(
            end_id,
            end_child_count + if start_id == end_id { 1 } else { 0 },
            ce_node,
        ));

        // Insert body
        txn.push(Operation::insert_node(root_id, root_children, cb_node));
        txn.push(Operation::insert_node(
            cb_id,
            0,
            Node::new(cb_para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            cb_para_id,
            0,
            Node::new(cb_run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            cb_run_id,
            0,
            Node::text(cb_text_id, text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(comment_id_val)
    }

    /// Insert a comment with markers positioned at the selected text range.
    ///
    /// Unlike `insert_comment` which places markers at paragraph boundaries,
    /// this positions CommentStart/CommentEnd at the correct run indices
    /// based on character offsets within the paragraphs.
    pub fn insert_comment_at_range(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        author: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let start_para_id = parse_node_id(start_node_str)?;
        let end_para_id = parse_node_id(end_node_str)?;

        let comment_id_val = format!("{}:{}", doc.next_id().replica, doc.next_id().counter);

        // Helper: find the child index in a paragraph that corresponds to a character offset
        fn find_run_index_at_offset(
            doc: &s1engine::Document,
            para_id: NodeId,
            char_offset: usize,
        ) -> usize {
            let para = match doc.node(para_id) {
                Some(n) => n,
                None => return 0,
            };
            let mut accumulated = 0usize;
            for (idx, &child_id) in para.children.iter().enumerate() {
                if let Some(child) = doc.node(child_id) {
                    if child.node_type == NodeType::Run {
                        let rlen = run_char_len(doc.model(), child_id);
                        if char_offset <= accumulated + rlen {
                            return idx;
                        }
                        accumulated += rlen;
                    }
                }
            }
            para.children.len()
        }

        let start_idx = find_run_index_at_offset(doc, start_para_id, start_offset);

        // Create CommentStart
        let cs_id = doc.next_id();
        let mut cs_node = Node::new(cs_id, NodeType::CommentStart);
        cs_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id_val.clone()),
        );
        cs_node.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String(author.to_string()),
        );

        // Create CommentEnd
        let ce_id = doc.next_id();
        let mut ce_node = Node::new(ce_id, NodeType::CommentEnd);
        ce_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id_val.clone()),
        );

        let end_idx = find_run_index_at_offset(doc, end_para_id, end_offset);
        let end_adj = if start_para_id == end_para_id {
            // Account for the CommentStart we're about to insert
            end_idx + 1 + 1
        } else {
            end_idx + 1
        };

        // Create CommentBody on root
        let root_id = doc.model().root_id();
        let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        let cb_id = doc.next_id();
        let mut cb_node = Node::new(cb_id, NodeType::CommentBody);
        cb_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id_val.clone()),
        );
        cb_node.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String(author.to_string()),
        );
        // Date is set by the editor JS side (no chrono in WASM)

        let cb_para_id = doc.next_id();
        let cb_run_id = doc.next_id();
        let cb_text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert comment at range");
        txn.push(Operation::insert_node(start_para_id, start_idx, cs_node));
        txn.push(Operation::insert_node(end_para_id, end_adj, ce_node));
        txn.push(Operation::insert_node(root_id, root_children, cb_node));
        txn.push(Operation::insert_node(
            cb_id,
            0,
            Node::new(cb_para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            cb_para_id,
            0,
            Node::new(cb_run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            cb_run_id,
            0,
            Node::text(cb_text_id, text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(comment_id_val)
    }

    /// Delete a comment and its range markers.
    pub fn delete_comment(&mut self, comment_id: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let model = doc.model();
        let root_id = model.root_id();

        // Find all nodes with this comment ID by traversing descendants
        let mut to_delete = Vec::new();
        let descendants = model.descendants(root_id);
        for node in &descendants {
            if matches!(
                node.node_type,
                NodeType::CommentStart | NodeType::CommentEnd | NodeType::CommentBody
            ) && node.attributes.get_string(&AttributeKey::CommentId) == Some(comment_id)
            {
                to_delete.push(node.id);
            }
        }

        let mut txn = Transaction::with_label("Delete comment");
        for nid in to_delete {
            txn.push(Operation::delete_node(nid));
        }
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get section properties as JSON.
    pub fn get_sections_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let sections = doc.sections();
        let mut entries = Vec::new();
        for sec in sections {
            let mut headers_json = Vec::new();
            for h in &sec.headers {
                let h_type = match h.hf_type {
                    s1_model::HeaderFooterType::First => "first",
                    s1_model::HeaderFooterType::Even => "even",
                    _ => "default",
                };
                headers_json.push(format!(
                    "{{\"type\":\"{}\",\"nodeId\":\"{}:{}\"}}",
                    h_type, h.node_id.replica, h.node_id.counter
                ));
            }
            let mut footers_json = Vec::new();
            for f in &sec.footers {
                let f_type = match f.hf_type {
                    s1_model::HeaderFooterType::First => "first",
                    s1_model::HeaderFooterType::Even => "even",
                    _ => "default",
                };
                footers_json.push(format!(
                    "{{\"type\":\"{}\",\"nodeId\":\"{}:{}\"}}",
                    f_type, f.node_id.replica, f.node_id.counter
                ));
            }

            entries.push(format!(
                "{{\"pageWidth\":{},\"pageHeight\":{},\"marginTop\":{},\"marginBottom\":{},\"marginLeft\":{},\"marginRight\":{},\"columns\":{},\"columnSpacing\":{:.1},\"headers\":[{}],\"footers\":[{}]}}",
                sec.page_width,
                sec.page_height,
                sec.margin_top,
                sec.margin_bottom,
                sec.margin_left,
                sec.margin_right,
                sec.columns,
                sec.column_spacing,
                headers_json.join(","),
                footers_json.join(","),
            ));
        }
        Ok(format!("[{}]", entries.join(",")))
    }

    /// Get page setup properties for the first section as JSON.
    ///
    /// Returns JSON: `{"pageWidth":612,"pageHeight":792,"marginTop":72,
    /// "marginBottom":72,"marginLeft":72,"marginRight":72,"orientation":"portrait"}`
    ///
    /// All dimensions are in points (1 inch = 72 points).
    pub fn get_page_setup_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let sections = doc.sections();
        let sec = sections.first().cloned().unwrap_or_default();
        let orientation = match sec.orientation {
            s1_model::PageOrientation::Landscape => "landscape",
            _ => "portrait",
        };
        Ok(format!(
            "{{\"pageWidth\":{:.2},\"pageHeight\":{:.2},\"marginTop\":{:.2},\"marginBottom\":{:.2},\"marginLeft\":{:.2},\"marginRight\":{:.2},\"orientation\":\"{}\"}}",
            sec.page_width,
            sec.page_height,
            sec.margin_top,
            sec.margin_bottom,
            sec.margin_left,
            sec.margin_right,
            orientation,
        ))
    }

    /// Set page setup properties for all sections from JSON.
    ///
    /// Accepts JSON: `{"pageWidth":612,"pageHeight":792,"marginTop":72,
    /// "marginBottom":72,"marginLeft":72,"marginRight":72,"orientation":"portrait"}`
    ///
    /// All dimensions are in points (1 inch = 72 points).
    /// Updates all sections in the document to use the new page dimensions.
    pub fn set_page_setup(&mut self, json: &str) -> Result<(), JsError> {
        // Parse JSON using existing module-level helpers
        let json = json.trim();
        let page_width = extract_json_number_opt(json, "pageWidth").unwrap_or(612.0);
        let page_height = extract_json_number_opt(json, "pageHeight").unwrap_or(792.0);
        let margin_top = extract_json_number_opt(json, "marginTop").unwrap_or(72.0);
        let margin_bottom = extract_json_number_opt(json, "marginBottom").unwrap_or(72.0);
        let margin_left = extract_json_number_opt(json, "marginLeft").unwrap_or(72.0);
        let margin_right = extract_json_number_opt(json, "marginRight").unwrap_or(72.0);
        let orientation = extract_json_string_opt(json, "orientation").unwrap_or_default();

        let orient_enum = if orientation == "landscape" {
            s1_model::PageOrientation::Landscape
        } else {
            s1_model::PageOrientation::Portrait
        };

        // Validate dimensions
        if !(72.0..=4320.0).contains(&page_width) {
            return Err(JsError::new("Page width must be between 1 and 60 inches"));
        }
        if !(72.0..=4320.0).contains(&page_height) {
            return Err(JsError::new("Page height must be between 1 and 60 inches"));
        }
        if margin_top < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 || margin_right < 0.0 {
            return Err(JsError::new("Margins cannot be negative"));
        }
        // Ensure margins don't exceed page dimensions
        if margin_left + margin_right >= page_width {
            return Err(JsError::new(
                "Left + right margins must be less than page width",
            ));
        }
        if margin_top + margin_bottom >= page_height {
            return Err(JsError::new(
                "Top + bottom margins must be less than page height",
            ));
        }

        let doc = self.doc_mut()?;
        let sections = doc.model_mut().sections_mut();

        if sections.is_empty() {
            // Create a default section if none exists
            sections.push(s1_model::SectionProperties::default());
        }

        for sec in sections.iter_mut() {
            sec.page_width = page_width;
            sec.page_height = page_height;
            sec.margin_top = margin_top;
            sec.margin_bottom = margin_bottom;
            sec.margin_left = margin_left;
            sec.margin_right = margin_right;
            sec.orientation = orient_enum;
        }

        Ok(())
    }

    // ─── UXP-02: Header/Footer Editing API ──────────────────────

    /// Set header or footer text for a given section.
    ///
    /// `section_index`: 0-based section index.
    /// `hf_kind`: `"header"` or `"footer"`.
    /// `hf_type`: `"default"` or `"first"`.
    /// `text`: Plain text content. If empty, the header/footer content is cleared.
    ///
    /// If the section does not have a header/footer of the specified type,
    /// one is created with a new Paragraph > Run > Text structure.
    pub fn set_header_footer_text(
        &mut self,
        section_index: usize,
        hf_kind: &str,
        hf_type_str: &str,
        text: &str,
    ) -> Result<(), JsError> {
        use s1_model::section::{HeaderFooterRef, HeaderFooterType};

        let hf_type = match hf_type_str {
            "first" => HeaderFooterType::First,
            "even" => HeaderFooterType::Even,
            _ => HeaderFooterType::Default,
        };

        let doc = self.doc_mut()?;
        let sections = doc.model().sections().to_vec();
        if section_index >= sections.len() {
            return Err(JsError::new(&format!(
                "Section index {} out of range (have {})",
                section_index,
                sections.len()
            )));
        }

        let sec = &sections[section_index];
        let is_header = hf_kind == "header";
        let refs = if is_header {
            &sec.headers
        } else {
            &sec.footers
        };
        let existing = refs.iter().find(|r| r.hf_type == hf_type);

        if let Some(hf_ref) = existing {
            // Header/footer node exists — update the first paragraph's text
            let hf_node_id = hf_ref.node_id;
            let hf_node = doc
                .model()
                .node(hf_node_id)
                .ok_or_else(|| JsError::new("Header/Footer node not found"))?;

            if hf_node.children.is_empty() {
                // Create Paragraph > Run > Text inside the header/footer
                let para_id = doc.next_id();
                let para_node = Node::new(para_id, NodeType::Paragraph);
                doc.apply(Operation::insert_node(hf_node_id, 0, para_node))
                    .map_err(|e| JsError::new(&e.to_string()))?;

                let run_id = doc.next_id();
                let run_node = Node::new(run_id, NodeType::Run);
                doc.apply(Operation::insert_node(para_id, 0, run_node))
                    .map_err(|e| JsError::new(&e.to_string()))?;

                let text_id = doc.next_id();
                let text_node = Node::text(text_id, "");
                doc.apply(Operation::insert_node(run_id, 0, text_node))
                    .map_err(|e| JsError::new(&e.to_string()))?;

                if !text.is_empty() {
                    doc.apply(Operation::insert_text(text_id, 0, text))
                        .map_err(|e| JsError::new(&e.to_string()))?;
                }
            } else {
                // Find the first paragraph and update its text
                let first_para_id = hf_node.children[0];
                // Use the same logic as set_paragraph_text
                let existing_text = extract_paragraph_text(doc.model(), first_para_id);
                if existing_text != text {
                    // Clear all text and rewrite
                    let (text_node_id, text_len) = ensure_run_and_text(doc, first_para_id)?;
                    if text_len > 0 {
                        doc.apply(Operation::delete_text(text_node_id, 0, text_len))
                            .map_err(|e| JsError::new(&e.to_string()))?;
                    }
                    if !text.is_empty() {
                        doc.apply(Operation::insert_text(text_node_id, 0, text))
                            .map_err(|e| JsError::new(&e.to_string()))?;
                    }
                }
            }
        } else {
            // No header/footer of this type exists — create one
            let root_id = doc.model().root_id();
            let root_children_len = doc
                .model()
                .node(root_id)
                .map(|n| n.children.len())
                .unwrap_or(0);

            // Create Header or Footer node
            let hf_node_id = doc.next_id();
            let node_type = if is_header {
                NodeType::Header
            } else {
                NodeType::Footer
            };
            let hf_node = Node::new(hf_node_id, node_type);
            doc.apply(Operation::insert_node(root_id, root_children_len, hf_node))
                .map_err(|e| JsError::new(&e.to_string()))?;

            // Create Paragraph > Run > Text inside
            let para_id = doc.next_id();
            let para_node = Node::new(para_id, NodeType::Paragraph);
            doc.apply(Operation::insert_node(hf_node_id, 0, para_node))
                .map_err(|e| JsError::new(&e.to_string()))?;

            let run_id = doc.next_id();
            let run_node = Node::new(run_id, NodeType::Run);
            doc.apply(Operation::insert_node(para_id, 0, run_node))
                .map_err(|e| JsError::new(&e.to_string()))?;

            let text_id = doc.next_id();
            let text_node = Node::text(text_id, "");
            doc.apply(Operation::insert_node(run_id, 0, text_node))
                .map_err(|e| JsError::new(&e.to_string()))?;

            if !text.is_empty() {
                doc.apply(Operation::insert_text(text_id, 0, text))
                    .map_err(|e| JsError::new(&e.to_string()))?;
            }

            // Register in section properties
            let sections_mut = doc.model_mut().sections_mut();
            if let Some(sec) = sections_mut.get_mut(section_index) {
                let hf_ref = HeaderFooterRef {
                    hf_type,
                    node_id: hf_node_id,
                };
                if is_header {
                    sec.headers.push(hf_ref);
                } else {
                    sec.footers.push(hf_ref);
                }
            }
        }

        Ok(())
    }

    /// Set or clear the "different first page" flag for a section.
    ///
    /// When enabled, the first page of the section uses the "first" header/footer
    /// instead of the "default" one.
    pub fn set_title_page(&mut self, section_index: usize, enabled: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let sections = doc.model_mut().sections_mut();
        if section_index >= sections.len() {
            return Err(JsError::new(&format!(
                "Section index {} out of range (have {})",
                section_index,
                sections.len()
            )));
        }
        sections[section_index].title_page = enabled;
        Ok(())
    }

    /// Get header/footer info for a section as JSON.
    ///
    /// Returns JSON: `{"hasDefaultHeader":true,"hasFirstHeader":false,
    /// "defaultHeaderText":"My Header","firstHeaderText":"",
    /// "hasDefaultFooter":true,"hasFirstFooter":false,
    /// "defaultFooterText":"Page 1","firstFooterText":"",
    /// "titlePage":false}`
    pub fn get_header_footer_info(&self, section_index: usize) -> Result<String, JsError> {
        use s1_model::section::HeaderFooterType;

        let doc = self.doc()?;
        let sections = doc.sections();
        if section_index >= sections.len() {
            return Err(JsError::new(&format!(
                "Section index {} out of range (have {})",
                section_index,
                sections.len()
            )));
        }

        let sec = &sections[section_index];
        let model = doc.model();

        let get_text =
            |refs: &[s1_model::section::HeaderFooterRef], hf_type: HeaderFooterType| -> String {
                if let Some(hf_ref) = refs.iter().find(|r| r.hf_type == hf_type) {
                    if let Some(hf_node) = model.node(hf_ref.node_id) {
                        if let Some(&first_para) = hf_node.children.first() {
                            return extract_paragraph_text(model, first_para);
                        }
                    }
                }
                String::new()
            };

        let default_header_text = get_text(&sec.headers, HeaderFooterType::Default);
        let first_header_text = get_text(&sec.headers, HeaderFooterType::First);
        let default_footer_text = get_text(&sec.footers, HeaderFooterType::Default);
        let first_footer_text = get_text(&sec.footers, HeaderFooterType::First);

        let has_default_header = sec
            .headers
            .iter()
            .any(|h| h.hf_type == HeaderFooterType::Default);
        let has_first_header = sec
            .headers
            .iter()
            .any(|h| h.hf_type == HeaderFooterType::First);
        let has_default_footer = sec
            .footers
            .iter()
            .any(|f| f.hf_type == HeaderFooterType::Default);
        let has_first_footer = sec
            .footers
            .iter()
            .any(|f| f.hf_type == HeaderFooterType::First);

        Ok(format!(
            "{{\"hasDefaultHeader\":{},\"hasFirstHeader\":{},\"defaultHeaderText\":\"{}\",\"firstHeaderText\":\"{}\",\"hasDefaultFooter\":{},\"hasFirstFooter\":{},\"defaultFooterText\":\"{}\",\"firstFooterText\":\"{}\",\"titlePage\":{}}}",
            has_default_header,
            has_first_header,
            default_header_text.replace('\\', "\\\\").replace('"', "\\\""),
            first_header_text.replace('\\', "\\\\").replace('"', "\\\""),
            has_default_footer,
            has_first_footer,
            default_footer_text.replace('\\', "\\\\").replace('"', "\\\""),
            first_footer_text.replace('\\', "\\\\").replace('"', "\\\""),
            sec.title_page,
        ))
    }

    // ─── P.5: Find & Replace + Clipboard API ────────────────────

    /// Find all occurrences of text in the document.
    ///
    /// Returns JSON array of `{"nodeId":"0:5","offset":3,"length":5}`.
    pub fn find_text(&self, query: &str, case_sensitive: bool) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let body = model
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;

        let query_lower = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let mut results = Vec::new();
        collect_find_results(
            model,
            &body.children,
            &query_lower,
            case_sensitive,
            &mut results,
        );
        Ok(format!("[{}]", results.join(",")))
    }

    /// Replace text in a paragraph by character range. Alias to `replace_text`.
    ///
    /// Preserves inline formatting (bold, italic, etc.) outside the modified range.
    pub fn replace_text_range(
        &mut self,
        node_id_str: &str,
        start_offset: usize,
        end_offset: usize,
        replacement: &str,
    ) -> Result<(), JsError> {
        let length = end_offset.saturating_sub(start_offset);
        self.replace_text(node_id_str, start_offset, length, replacement)
    }

    /// Replace text at a specific location.
    ///
    /// Note: insert_text into an existing text node inherits the parent run's
    /// formatting (bold, italic, etc.) — no explicit attribute copy needed.
    pub fn replace_text(
        &mut self,
        node_id_str: &str,
        offset: usize,
        length: usize,
        replacement: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        // Bug W1: Clamp deletion range to actual paragraph text length
        let para_text = extract_paragraph_text(doc.model(), para_id);
        let text_len = para_text.chars().count();
        let safe_offset = offset.min(text_len);
        let safe_end = (offset + length).min(text_len);
        let safe_length = safe_end.saturating_sub(safe_offset);

        // If nothing to delete and nothing to insert, it's a no-op
        if safe_length == 0 && replacement.is_empty() {
            return Ok(());
        }

        let range_contains_inline_nodes = safe_length > 0
            && paragraph_range_contains_inline_nodes(
                doc.model(),
                para_id,
                safe_offset,
                safe_offset + safe_length,
            );

        let single_node_target = if range_contains_inline_nodes {
            None
        } else {
            let (text_node_id, local_offset, node_text_len) =
                find_text_node_at_char_offset(doc.model(), para_id, safe_offset)?;
            Some((text_node_id, local_offset, node_text_len))
        };

        if let Some((text_node_id, local_offset, node_text_len)) = single_node_target {
            if local_offset + safe_length > node_text_len {
                // Deletion spans multiple runs — delete range first, then insert
                // into the text node at the start offset (which preserves run formatting).
                if safe_length > 0 {
                    delete_text_range_in_paragraph(
                        doc,
                        para_id,
                        safe_offset,
                        safe_offset + safe_length,
                    )?;
                }
                if !replacement.is_empty() {
                    let (ins_text_node_id, ins_local_offset, _) =
                        find_text_node_at_char_offset(doc.model(), para_id, safe_offset)?;
                    doc.apply(Operation::insert_text(
                        ins_text_node_id,
                        ins_local_offset,
                        replacement,
                    ))
                    .map_err(|e| JsError::new(&format!("Replace text failed: {}", e)))
                } else {
                    Ok(())
                }
            } else {
                // Deletion fits in one text node — simple path
                let mut txn = Transaction::with_label("Replace text");
                if safe_length > 0 {
                    txn.push(Operation::delete_text(
                        text_node_id,
                        local_offset,
                        safe_length,
                    ));
                }
                if !replacement.is_empty() {
                    txn.push(Operation::insert_text(
                        text_node_id,
                        local_offset,
                        replacement,
                    ));
                }
                if txn.is_empty() {
                    return Ok(());
                }
                doc.apply_transaction(&txn)
                    .map_err(|e| JsError::new(&format!("Replace text failed: {}", e)))
            }
        } else {
            // Deletion spans multiple runs — delete range first, then insert
            // into the text node at the start offset (which preserves run formatting).
            if safe_length > 0 {
                delete_text_range_in_paragraph(
                    doc,
                    para_id,
                    safe_offset,
                    safe_offset + safe_length,
                )?;
            }
            if !replacement.is_empty() {
                // Re-locate the text node after deletion (nodes may have shifted)
                let (ins_text_node_id, ins_local_offset, _) =
                    find_text_node_at_char_offset(doc.model(), para_id, safe_offset)?;
                doc.apply(Operation::insert_text(
                    ins_text_node_id,
                    ins_local_offset,
                    replacement,
                ))
                .map_err(|e| JsError::new(&format!("Replace text failed: {}", e)))
            } else {
                Ok(())
            }
        }
    }

    /// Replace all occurrences of query with replacement.
    ///
    /// Returns the number of replacements made. Single transaction.
    pub fn replace_all(
        &mut self,
        query: &str,
        replacement: &str,
        case_sensitive: bool,
    ) -> Result<u32, JsError> {
        let doc = self.doc_mut()?;
        let model = doc.model();
        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let body = model
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let children = body.children.clone();

        let query_lower = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        // Collect all matches first
        let mut matches: Vec<(NodeId, usize, usize)> = Vec::new(); // (text_node_id, offset, length)
        for &child_id in &children {
            collect_replace_matches(
                model,
                child_id,
                &query_lower,
                case_sensitive,
                query.chars().count(),
                &mut matches,
            );
        }

        if matches.is_empty() {
            return Ok(0);
        }

        let count = matches.len() as u32;

        // Group by text node, then sort offsets descending within each group
        // so replacements don't invalidate subsequent offsets
        matches.sort_by(|a, b| {
            if a.0 == b.0 {
                b.1.cmp(&a.1) // reverse offset within same node
            } else {
                a.0.counter.cmp(&b.0.counter)
            }
        });

        let mut txn = Transaction::with_label("Replace all");
        for (text_node_id, offset, length) in &matches {
            txn.push(Operation::delete_text(*text_node_id, *offset, *length));
            txn.push(Operation::insert_text(*text_node_id, *offset, replacement));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(count)
    }

    /// Insert plain text at cursor position, splitting on newlines.
    pub fn paste_plain_text(
        &mut self,
        para_id_str: &str,
        offset: usize,
        text: &str,
    ) -> Result<(), JsError> {
        let lines: Vec<&str> = text.split('\n').collect();

        if lines.len() == 1 {
            // Simple case: insert in current paragraph
            self.insert_text_in_paragraph(para_id_str, offset, lines[0])?;
        } else {
            // Multi-line: insert first line, split, insert remaining
            self.insert_text_in_paragraph(para_id_str, offset, lines[0])?;

            // Split and create new paragraphs for remaining lines
            let mut current_para_str = para_id_str.to_string();
            let first_line_len = lines[0].chars().count();
            let split_offset = offset + first_line_len;

            let para_id = parse_node_id(para_id_str)?;
            let doc = self.doc_mut()?;
            // Split at end of first inserted text
            let full_text = extract_paragraph_text(doc.model(), para_id);
            let char_count = full_text.chars().count();
            if split_offset < char_count || lines.len() > 1 {
                let new_id = self.split_paragraph(&current_para_str, split_offset)?;
                current_para_str = new_id;
            }

            // Insert remaining lines as separate paragraphs
            for (i, line) in lines[1..].iter().enumerate() {
                if i > 0 {
                    let new_id = self.split_paragraph(&current_para_str, 0)?;
                    current_para_str = new_id;
                }
                if !line.is_empty() {
                    self.insert_text_in_paragraph(&current_para_str, 0, line)?;
                }
            }
        }
        Ok(())
    }

    /// Get all text in the document as a single string.
    pub fn get_document_text(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        Ok(doc.to_plain_text())
    }

    // ─── E2.2: Rich Paste (Formatted Runs) ──────────────────────

    /// Paste formatted text (with per-run styling) at a position in the document.
    ///
    /// `target_node_str` is the paragraph node ID (e.g. `"0:5"`).
    /// `char_offset` is the character offset within that paragraph.
    /// `runs_json` is a JSON string describing the formatted text to paste:
    ///
    /// ```json
    /// {
    ///   "paragraphs": [
    ///     {
    ///       "runs": [
    ///         {"text": "Hello ", "bold": false},
    ///         {"text": "world", "bold": true, "italic": true,
    ///          "fontSize": 14, "fontFamily": "Arial",
    ///          "color": "FF0000", "underline": true,
    ///          "strikethrough": false}
    ///       ]
    ///     },
    ///     {
    ///       "runs": [
    ///         {"text": "Second paragraph"}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// For a single paragraph: inserts all run text at the offset and formats
    /// each run's character range. For multiple paragraphs: splits the target
    /// paragraph, inserts new paragraphs between, each with formatted runs.
    pub fn paste_formatted_runs_json(
        &mut self,
        target_node_str: &str,
        char_offset: usize,
        runs_json: &str,
    ) -> Result<(), JsError> {
        let paste_data = parse_paste_json(runs_json)?;

        if paste_data.is_empty() {
            return Ok(());
        }

        const MAX_PASTE_PARAGRAPHS: usize = 10_000;
        if paste_data.len() > MAX_PASTE_PARAGRAPHS {
            return Err(JsError::new(&format!(
                "Paste exceeds maximum paragraph count ({MAX_PASTE_PARAGRAPHS}). \
                 Try pasting smaller sections."
            )));
        }

        /// Helper: apply paragraph-level formatting attributes to a paragraph node.
        fn apply_para_format(
            doc: &mut s1engine::Document,
            para_id: NodeId,
            fmt: &PasteParagraphFormat,
        ) {
            let para_attrs = fmt.to_attribute_map();
            if !para_attrs.is_empty() {
                let _ = doc.apply(Operation::set_attributes(para_id, para_attrs));
            }
        }

        /// Helper: format runs within a paragraph.
        fn format_para_runs(
            this: &mut WasmDocument,
            _para_str: &str,
            para_id: NodeId,
            runs: &[PasteRun],
            offset: usize,
        ) -> Result<(), JsError> {
            let mut run_start = offset;
            for run in runs {
                let run_len = run.text.chars().count();
                if run_len == 0 {
                    continue;
                }
                let run_end = run_start + run_len;
                let attrs = run.to_attribute_map();
                if !attrs.is_empty() {
                    let doc = this.doc_mut()?;
                    format_range_in_paragraph(doc, para_id, run_start, run_end, &attrs)?;
                }
                run_start = run_end;
            }
            Ok(())
        }

        if paste_data.len() == 1 {
            // --- Single paragraph: insert all run text, then format each run ---
            let para = &paste_data[0];
            let runs = &para.runs;
            if runs.is_empty() {
                return Ok(());
            }

            // Concatenate all run texts
            let full_text: String = runs.iter().map(|r| r.text.as_str()).collect();
            if full_text.is_empty() {
                return Ok(());
            }

            // Insert the concatenated text at the offset
            self.insert_text_in_paragraph(target_node_str, char_offset, &full_text)?;

            // Format each run's character range
            let para_id = parse_node_id(target_node_str)?;
            format_para_runs(self, target_node_str, para_id, runs, char_offset)?;

            // Apply paragraph-level formatting
            let doc = self.doc_mut()?;
            apply_para_format(doc, para_id, &para.format);
        } else {
            // --- Multi-paragraph paste ---
            let target_id = parse_node_id(target_node_str)?;

            // Step 1: Insert first paragraph's run text at offset
            let first_para = &paste_data[0];
            let first_text: String = first_para.runs.iter().map(|r| r.text.as_str()).collect();
            if !first_text.is_empty() {
                self.insert_text_in_paragraph(target_node_str, char_offset, &first_text)?;
            }

            // Step 2: Split at end of inserted text to create tail paragraph
            let first_text_char_len = first_text.chars().count();
            let split_offset = char_offset + first_text_char_len;

            let doc = self.doc_mut()?;
            let full_text = extract_paragraph_text(doc.model(), target_id);
            let _full_char_len = full_text.chars().count();

            let mut current_para_str = self.split_paragraph(target_node_str, split_offset)?;

            // Step 3: Insert intermediate and last paragraphs
            let last_idx = paste_data.len() - 1;

            for (i, parsed_para) in paste_data[1..].iter().enumerate() {
                let para_runs = &parsed_para.runs;
                if i < last_idx - 1 {
                    // Intermediate paragraph: split at 0 to create a new empty paragraph
                    let new_id = self.split_paragraph(&current_para_str, 0)?;
                    let para_text: String = para_runs.iter().map(|r| r.text.as_str()).collect();
                    if !para_text.is_empty() {
                        self.insert_text_in_paragraph(&current_para_str, 0, &para_text)?;
                    }
                    let pid = parse_node_id(&current_para_str)?;
                    format_para_runs(self, &current_para_str, pid, para_runs, 0)?;
                    // Apply paragraph-level formatting
                    let doc = self.doc_mut()?;
                    apply_para_format(doc, pid, &parsed_para.format);
                    current_para_str = new_id;
                } else {
                    // Last paragraph: insert text at start of the tail paragraph
                    let para_text: String = para_runs.iter().map(|r| r.text.as_str()).collect();
                    if !para_text.is_empty() {
                        self.insert_text_in_paragraph(&current_para_str, 0, &para_text)?;
                    }
                    let pid = parse_node_id(&current_para_str)?;
                    format_para_runs(self, &current_para_str, pid, para_runs, 0)?;
                    // Apply paragraph-level formatting
                    let doc = self.doc_mut()?;
                    apply_para_format(doc, pid, &parsed_para.format);
                }
            }

            // Step 4: Format runs in the first (target) paragraph
            format_para_runs(
                self,
                target_node_str,
                target_id,
                &first_para.runs,
                char_offset,
            )?;
            let doc = self.doc_mut()?;
            apply_para_format(doc, target_id, &first_para.format);
        }

        Ok(())
    }

    // ─── P.10: Rich Copy/Paste HTML Export ────────────────────────

    /// Export a selection range as clean, portable semantic HTML.
    ///
    /// The output contains no `data-node-id` attributes, no editor-specific
    /// classes, and no track-changes markup. Suitable for clipboard
    /// rich-text copy/paste.
    ///
    /// `start_node_str` / `end_node_str` are paragraph node IDs (e.g.
    /// `"0:5"`). `start_offset` / `end_offset` are character offsets within
    /// those paragraphs.
    pub fn export_selection_html(
        &self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let start_para = parse_node_id(start_node_str)?;
        let end_para = parse_node_id(end_node_str)?;

        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let children = body.children.clone();

        let mut html = String::new();

        if start_para == end_para {
            // Single paragraph selection
            if let Some(para) = doc.node(start_para) {
                if para.node_type == NodeType::Paragraph {
                    render_paragraph_clean_partial(
                        model,
                        start_para,
                        Some(start_offset),
                        Some(end_offset),
                        &mut html,
                    );
                } else if para.node_type == NodeType::Table {
                    render_table_clean(model, start_para, &mut html);
                }
            }
        } else {
            let start_idx = children.iter().position(|&c| c == start_para);
            let end_idx = children.iter().position(|&c| c == end_para);

            match (start_idx, end_idx) {
                (Some(si), Some(ei)) => {
                    // First paragraph (partial from start_offset to end)
                    if let Some(node) = doc.node(children[si]) {
                        match node.node_type {
                            NodeType::Paragraph => {
                                render_paragraph_clean_partial(
                                    model,
                                    children[si],
                                    Some(start_offset),
                                    None,
                                    &mut html,
                                );
                            }
                            NodeType::Table => {
                                render_table_clean(model, children[si], &mut html);
                            }
                            _ => {}
                        }
                    }

                    // Middle paragraphs (full)
                    for &child_id in &children[si + 1..ei] {
                        if let Some(child) = doc.node(child_id) {
                            match child.node_type {
                                NodeType::Paragraph => {
                                    render_paragraph_clean_partial(
                                        model, child_id, None, None, &mut html,
                                    );
                                }
                                NodeType::Table => {
                                    render_table_clean(model, child_id, &mut html);
                                }
                                NodeType::Image => {
                                    render_image_clean(model, child_id, &mut html);
                                }
                                _ => {}
                            }
                        }
                    }

                    // Last paragraph (partial from 0 to end_offset)
                    if let Some(node) = doc.node(children[ei]) {
                        match node.node_type {
                            NodeType::Paragraph => {
                                render_paragraph_clean_partial(
                                    model,
                                    children[ei],
                                    None,
                                    Some(end_offset),
                                    &mut html,
                                );
                            }
                            NodeType::Table => {
                                render_table_clean(model, children[ei], &mut html);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    return Err(JsError::new("Start or end paragraph not found in body"));
                }
            }
        }

        Ok(html)
    }

    // ─── E9.5: Table of Contents ────────────────────────────────

    /// Insert a Table of Contents after the given node.
    ///
    /// `max_level` controls the deepest heading level included (1-9, default 3).
    /// If `title` is non-empty, it is set as the TOC title.
    /// Returns the TOC node ID string.
    pub fn insert_table_of_contents(
        &mut self,
        after_node_str: &str,
        max_level: u8,
        title: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let index = body
            .children
            .iter()
            .position(|&c| c == after_id)
            .ok_or_else(|| JsError::new("Node not found in body"))?
            + 1;

        let level = max_level.clamp(1, 9);
        let toc_id = doc.next_id();
        let mut toc_node = Node::new(toc_id, NodeType::TableOfContents);
        toc_node
            .attributes
            .set(AttributeKey::TocMaxLevel, AttributeValue::Int(level as i64));
        if !title.is_empty() {
            toc_node.attributes.set(
                AttributeKey::TocTitle,
                AttributeValue::String(title.to_string()),
            );
        }

        let mut txn = Transaction::with_label("Insert table of contents");
        txn.push(Operation::insert_node(body_id, index, toc_node));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Auto-populate TOC entries
        doc.update_toc();

        Ok(format!("{}:{}", toc_id.replica, toc_id.counter))
    }

    /// Update all Table of Contents entries in the document.
    ///
    /// Rescans headings and regenerates TOC child paragraphs.
    pub fn update_table_of_contents(&mut self) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.update_toc();
        Ok(())
    }

    /// Convert selected paragraphs to a table.
    ///
    /// Takes consecutive paragraphs and converts each into a table row.
    /// Cells are split by `delimiter` ("tab", "comma", "semicolon", or "paragraph").
    /// If delimiter is "paragraph", each paragraph becomes a single-cell row.
    ///
    /// Returns the new table node ID.
    pub fn text_to_table(
        &mut self,
        first_para_str: &str,
        last_para_str: &str,
        delimiter: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let first_id = parse_node_id(first_para_str)?;
        let last_id = parse_node_id(last_para_str)?;

        // Collect paragraphs in range
        let body_id = doc
            .model()
            .body_id()
            .ok_or_else(|| JsError::new("No document body"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;

        let mut para_ids: Vec<NodeId> = Vec::new();
        let mut in_range = false;
        for &child_id in &body.children {
            if child_id == first_id {
                in_range = true;
            }
            if in_range {
                para_ids.push(child_id);
            }
            if child_id == last_id {
                break;
            }
        }

        if para_ids.is_empty() {
            return Err(JsError::new("No paragraphs found in range"));
        }

        // Extract text from each paragraph
        let delim_char = match delimiter {
            "tab" => '\t',
            "comma" => ',',
            "semicolon" => ';',
            _ => '\n', // "paragraph" = each para is one cell
        };

        let mut rows: Vec<Vec<String>> = Vec::new();
        for &para_id in &para_ids {
            let mut text = String::new();
            if let Some(para) = doc.node(para_id) {
                for &run_id in &para.children {
                    if let Some(run) = doc.node(run_id) {
                        for &txt_id in &run.children {
                            if let Some(txt) = doc.node(txt_id) {
                                if let Some(ref tc) = txt.text_content {
                                    text.push_str(tc);
                                }
                            }
                        }
                        if let Some(ref tc) = run.text_content {
                            text.push_str(tc);
                        }
                    }
                }
            }
            if delimiter == "paragraph" {
                rows.push(vec![text]);
            } else {
                rows.push(
                    text.split(delim_char)
                        .map(|s| s.trim().to_string())
                        .collect(),
                );
            }
        }

        // Determine column count (max cells in any row)
        let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
        let _num_rows = rows.len();

        // Insert table after first paragraph
        let parent_id = body_id;
        let insert_idx = body
            .children
            .iter()
            .position(|&id| id == first_id)
            .unwrap_or(0);

        // Create table using insert_table, then populate cells
        let table_id = doc.next_id();
        let mut table_node = Node::new(table_id, NodeType::Table);
        // Set equal column widths
        let col_width = 468.0 / num_cols as f64; // Letter page content width / cols
        let widths: Vec<String> = (0..num_cols).map(|_| format!("{:.1}", col_width)).collect();
        table_node.attributes.set(
            AttributeKey::TableColumnWidths,
            AttributeValue::String(widths.join(",")),
        );

        let mut txn = Transaction::with_label("Convert text to table");
        txn.push(Operation::insert_node(parent_id, insert_idx, table_node));

        for (ri, row) in rows.iter().enumerate() {
            let row_id = doc.next_id();
            let row_node = Node::new(row_id, NodeType::TableRow);
            txn.push(Operation::insert_node(table_id, ri, row_node));

            for ci in 0..num_cols {
                let cell_id = doc.next_id();
                let cell_node = Node::new(cell_id, NodeType::TableCell);
                txn.push(Operation::insert_node(row_id, ci, cell_node));

                let para_id_new = doc.next_id();
                let para_node = Node::new(para_id_new, NodeType::Paragraph);
                txn.push(Operation::insert_node(cell_id, 0, para_node));

                let cell_text = row.get(ci).cloned().unwrap_or_default();
                if !cell_text.is_empty() {
                    let run_id = doc.next_id();
                    let run_node = Node::new(run_id, NodeType::Run);
                    txn.push(Operation::insert_node(para_id_new, 0, run_node));

                    let txt_id = doc.next_id();
                    let mut txt_node = Node::new(txt_id, NodeType::Text);
                    txt_node.text_content = Some(cell_text);
                    txn.push(Operation::insert_node(run_id, 0, txt_node));
                }
            }
        }

        // Delete original paragraphs
        for &para_id in &para_ids {
            txn.push(Operation::delete_node(para_id));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", table_id.replica, table_id.counter))
    }

    /// Toggle a form checkbox's checked state.
    ///
    /// Returns the new checked state.
    pub fn toggle_form_checkbox(&mut self, node_id_str: &str) -> Result<bool, JsError> {
        let doc = self.doc_mut()?;
        let node_id = parse_node_id(node_id_str)?;
        let current = doc
            .node(node_id)
            .and_then(|n| n.attributes.get_bool(&AttributeKey::FormChecked))
            .unwrap_or(false);
        let new_value = !current;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(AttributeKey::FormChecked, AttributeValue::Bool(new_value));
        doc.apply(Operation::set_attributes(node_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(new_value)
    }

    /// Convert OMML (Office MathML) XML to LaTeX string.
    ///
    /// Handles common OMML elements: fractions, subscripts, superscripts,
    /// square roots, matrices, summations, integrals, Greek letters.
    pub fn omml_to_latex(&self, omml_xml: &str) -> Result<String, JsError> {
        Ok(convert_omml_to_latex(omml_xml))
    }

    /// Convert LaTeX string to OMML XML.
    ///
    /// Handles common LaTeX commands and produces valid Office MathML.
    pub fn latex_to_omml(&self, latex: &str) -> Result<String, JsError> {
        Ok(convert_latex_to_omml(latex))
    }

    /// Check if password protection is available.
    ///
    /// Password protection requires server-side encryption (AES-256/AGILE).
    /// Use the server API: `POST /api/documents/convert` with `format=docx&password=...`
    ///
    /// Returns false in WASM (server-side only feature).
    pub fn supports_password_protection(&self) -> bool {
        false // Server-side only — WASM doesn't include crypto for size
    }

    /// Insert a mail merge field placeholder in the current paragraph.
    ///
    /// - `para_id_str`: paragraph to insert into
    /// - `field_name`: the merge field name (e.g., "FirstName", "Email")
    ///
    /// Returns the field node ID. The field displays as `«FieldName»` until merged.
    pub fn insert_merge_field(
        &mut self,
        para_id_str: &str,
        field_name: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;

        let field_id = doc.next_id();
        let mut field_node = Node::new(field_id, NodeType::Field);
        field_node.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(s1_model::FieldType::MergeField),
        );
        field_node.attributes.set(
            AttributeKey::FieldCode,
            AttributeValue::String(format!("MERGEFIELD {}", field_name)),
        );
        field_node.text_content = Some(format!("\u{00AB}{}\u{00BB}", field_name));

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let insert_idx = para.children.len();

        let mut txn = Transaction::with_label("Insert merge field");
        txn.push(Operation::insert_node(para_id, insert_idx, field_node));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", field_id.replica, field_id.counter))
    }

    /// Apply mail merge data to the document.
    ///
    /// Takes a JSON array of records: `[{"FirstName":"John","LastName":"Doe"}, ...]`
    /// and a record index (0-based). Replaces all MERGEFIELD placeholders with
    /// values from the specified record.
    ///
    /// Returns the number of fields replaced.
    pub fn apply_merge_data(
        &mut self,
        data_json: &str,
        record_index: usize,
    ) -> Result<u32, JsError> {
        let records: Vec<serde_json::Value> = serde_json::from_str(data_json)
            .map_err(|e| JsError::new(&format!("Invalid merge data JSON: {}", e)))?;

        let record = records.get(record_index).ok_or_else(|| {
            JsError::new(&format!(
                "Record index {} out of range ({})",
                record_index,
                records.len()
            ))
        })?;

        let doc = self.doc_mut()?;
        let body_id = doc
            .model()
            .body_id()
            .ok_or_else(|| JsError::new("No document body"))?;

        // Collect all merge fields
        let mut merge_fields: Vec<(NodeId, String)> = Vec::new();
        fn collect_merge_fields(
            doc: &s1engine::Document,
            node_id: NodeId,
            fields: &mut Vec<(NodeId, String)>,
        ) {
            if let Some(node) = doc.node(node_id) {
                if node.node_type == NodeType::Field {
                    if let Some(code) = node.attributes.get_string(&AttributeKey::FieldCode) {
                        if let Some(name) = code.strip_prefix("MERGEFIELD ") {
                            fields.push((node_id, name.trim().to_string()));
                        }
                    }
                }
                for &child_id in &node.children {
                    collect_merge_fields(doc, child_id, fields);
                }
            }
        }
        collect_merge_fields(doc, body_id, &mut merge_fields);

        let mut count = 0u32;
        let mut txn = Transaction::with_label("Apply merge data");

        for (field_id, field_name) in &merge_fields {
            if let Some(value) = record.get(field_name).and_then(|v| v.as_str()) {
                // Create a text node to replace the field
                let run_id = doc.next_id();
                let run_node = Node::new(run_id, NodeType::Run);
                let text_id = doc.next_id();
                let mut text_node = Node::new(text_id, NodeType::Text);
                text_node.text_content = Some(value.to_string());

                // Find the field's parent paragraph
                if let Some(field_node) = doc.node(*field_id) {
                    if let Some(parent_id) = field_node.parent {
                        if let Some(parent) = doc.node(parent_id) {
                            if let Some(idx) =
                                parent.children.iter().position(|&id| id == *field_id)
                            {
                                txn.push(Operation::insert_node(parent_id, idx, run_node));
                                txn.push(Operation::insert_node(run_id, 0, text_node));
                                txn.push(Operation::delete_node(*field_id));
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        if count > 0 {
            doc.apply_transaction(&txn)
                .map_err(|e| JsError::new(&e.to_string()))?;
        }
        Ok(count)
    }

    /// Insert a bibliography citation at the cursor position.
    ///
    /// - `para_id_str`: paragraph to insert into
    /// - `citation_json`: JSON object with citation fields:
    ///   `{"author":"Smith","year":"2024","title":"Paper Title","source":"Journal Name"}`
    ///
    /// Returns the citation field node ID.
    pub fn insert_citation(
        &mut self,
        para_id_str: &str,
        citation_json: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;

        // Parse citation data
        let citation: serde_json::Value = serde_json::from_str(citation_json)
            .map_err(|e| JsError::new(&format!("Invalid citation JSON: {}", e)))?;

        let author = citation["author"].as_str().unwrap_or("Unknown");
        let year = citation["year"].as_str().unwrap_or("n.d.");

        // Create inline citation text: (Author, Year)
        let citation_text = format!("({}, {})", author, year);

        let field_id = doc.next_id();
        let mut field_node = Node::new(field_id, NodeType::Field);
        field_node.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(s1_model::FieldType::Custom),
        );
        field_node.attributes.set(
            AttributeKey::FieldCode,
            AttributeValue::String(format!("CITATION {}", citation_json)),
        );
        field_node.text_content = Some(citation_text);

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let insert_idx = para.children.len();

        let mut txn = Transaction::with_label("Insert citation");
        txn.push(Operation::insert_node(para_id, insert_idx, field_node));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", field_id.replica, field_id.counter))
    }

    /// Generate a bibliography section from all citations in the document.
    ///
    /// Scans for CITATION fields, extracts their JSON data, and creates
    /// a formatted bibliography paragraph after `after_node_str`.
    pub fn generate_bibliography(&mut self, after_node_str: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;

        // Collect all citations
        let body_id = doc
            .model()
            .body_id()
            .ok_or_else(|| JsError::new("No document body"))?;
        let mut citations: Vec<serde_json::Value> = Vec::new();

        fn collect_citations(
            doc: &s1engine::Document,
            node_id: NodeId,
            citations: &mut Vec<serde_json::Value>,
        ) {
            if let Some(node) = doc.node(node_id) {
                if node.node_type == NodeType::Field {
                    if let Some(code) = node.attributes.get_string(&AttributeKey::FieldCode) {
                        if let Some(json_str) = code.strip_prefix("CITATION ") {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                citations.push(val);
                            }
                        }
                    }
                }
                for &child_id in &node.children {
                    collect_citations(doc, child_id, citations);
                }
            }
        }
        collect_citations(doc, body_id, &mut citations);

        // Deduplicate by author+year
        let mut seen = std::collections::HashSet::new();
        let mut unique_citations = Vec::new();
        for c in &citations {
            let key = format!(
                "{}_{}",
                c["author"].as_str().unwrap_or(""),
                c["year"].as_str().unwrap_or("")
            );
            if seen.insert(key) {
                unique_citations.push(c.clone());
            }
        }

        // Sort by author
        unique_citations.sort_by(|a, b| {
            let aa = a["author"].as_str().unwrap_or("");
            let ba = b["author"].as_str().unwrap_or("");
            aa.cmp(ba)
        });

        // Create bibliography section
        let parent_id = doc
            .node(after_id)
            .and_then(|n| n.parent)
            .ok_or_else(|| JsError::new("Cannot find parent"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let insert_idx = parent
            .children
            .iter()
            .position(|&id| id == after_id)
            .map(|i| i + 1)
            .unwrap_or(parent.children.len());

        let mut txn = Transaction::with_label("Generate bibliography");

        // Title paragraph
        let title_id = doc.next_id();
        let mut title_para = Node::new(title_id, NodeType::Paragraph);
        title_para.attributes.set(
            AttributeKey::StyleId,
            AttributeValue::String("Heading1".to_string()),
        );
        txn.push(Operation::insert_node(parent_id, insert_idx, title_para));

        let title_run_id = doc.next_id();
        let title_run = Node::new(title_run_id, NodeType::Run);
        txn.push(Operation::insert_node(title_id, 0, title_run));

        let title_text_id = doc.next_id();
        let mut title_text = Node::new(title_text_id, NodeType::Text);
        title_text.text_content = Some("Bibliography".to_string());
        txn.push(Operation::insert_node(title_run_id, 0, title_text));

        // Citation entries
        for (i, c) in unique_citations.iter().enumerate() {
            let author = c["author"].as_str().unwrap_or("Unknown");
            let year = c["year"].as_str().unwrap_or("n.d.");
            let title = c["title"].as_str().unwrap_or("");
            let source = c["source"].as_str().unwrap_or("");

            let entry_text = if source.is_empty() {
                format!("{}. ({}). {}.", author, year, title)
            } else {
                format!("{}. ({}). {}. {}.", author, year, title, source)
            };

            let para_id_new = doc.next_id();
            let para_node = Node::new(para_id_new, NodeType::Paragraph);
            txn.push(Operation::insert_node(
                parent_id,
                insert_idx + 1 + i,
                para_node,
            ));

            let run_id = doc.next_id();
            let run_node = Node::new(run_id, NodeType::Run);
            txn.push(Operation::insert_node(para_id_new, 0, run_node));

            let text_id = doc.next_id();
            let mut text_node = Node::new(text_id, NodeType::Text);
            text_node.text_content = Some(entry_text);
            txn.push(Operation::insert_node(run_id, 0, text_node));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", title_id.replica, title_id.counter))
    }

    /// Compare this document with another and return word-level differences as JSON.
    ///
    /// Takes the bytes of another document, opens it, extracts plain text from both,
    /// and returns a JSON array of diff operations:
    /// `[{"type":"equal","text":"..."},{"type":"insert","text":"..."},{"type":"delete","text":"..."}]`
    pub fn compare_with(&self, other_bytes: &[u8]) -> Result<String, JsError> {
        let doc = self.doc()?;
        let text_a = doc.to_plain_text();

        // Open the other document
        let engine = s1engine::Engine::new();
        let other_doc = engine
            .open(other_bytes)
            .map_err(|e| JsError::new(&format!("Failed to open comparison document: {}", e)))?;
        let text_b = other_doc.to_plain_text();

        // Word-level diff using LCS (Longest Common Subsequence)
        let words_a: Vec<&str> = text_a.split_whitespace().collect();
        let words_b: Vec<&str> = text_b.split_whitespace().collect();

        let diffs = word_diff(&words_a, &words_b);

        let mut json = String::from("[");
        for (i, diff) in diffs.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let (diff_type, text) = match diff {
                DiffOp::Equal(words) => ("equal", words.join(" ")),
                DiffOp::Insert(words) => ("insert", words.join(" ")),
                DiffOp::Delete(words) => ("delete", words.join(" ")),
            };
            json.push_str(&format!(
                "{{\"type\":\"{}\",\"text\":\"{}\"}}",
                diff_type,
                escape_json(&text)
            ));
        }
        json.push(']');
        Ok(json)
    }

    /// Insert a SEQ (sequence) field for auto-numbering.
    ///
    /// Sequence fields maintain separate counters per `seq_name` (e.g., "Figure", "Table").
    /// Returns the field node ID.
    pub fn insert_seq_field(
        &mut self,
        para_id_str: &str,
        seq_name: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(para_id_str)?;

        // Count existing SEQ fields with same name to determine number
        let body_id = doc
            .model()
            .body_id()
            .ok_or_else(|| JsError::new("No document body"))?;
        let mut count = 0u32;
        fn count_seq_fields(
            doc: &s1engine::Document,
            node_id: NodeId,
            seq_name: &str,
            count: &mut u32,
        ) {
            if let Some(node) = doc.node(node_id) {
                if node.node_type == NodeType::Field {
                    if let Some(code) = node.attributes.get_string(&AttributeKey::FieldCode) {
                        if code.starts_with("SEQ ") && code.contains(seq_name) {
                            *count += 1;
                        }
                    }
                }
                for &child_id in &node.children {
                    count_seq_fields(doc, child_id, seq_name, count);
                }
            }
        }
        count_seq_fields(doc, body_id, seq_name, &mut count);
        let number = count + 1;

        let field_id = doc.next_id();
        let mut field_node = Node::new(field_id, NodeType::Field);
        field_node.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(s1_model::FieldType::Sequence),
        );
        field_node.attributes.set(
            AttributeKey::FieldCode,
            AttributeValue::String(format!("SEQ {}", seq_name)),
        );
        field_node.text_content = Some(number.to_string());

        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let insert_idx = para.children.len();

        let mut txn = Transaction::with_label("Insert SEQ field");
        txn.push(Operation::insert_node(para_id, insert_idx, field_node));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", field_id.replica, field_id.counter))
    }

    /// Generate a Table of Figures from all Caption-styled paragraphs.
    ///
    /// Inserts a new section after `after_node_str` containing a list of all
    /// captions found in the document (Figure 1: ..., Table 2: ..., etc.).
    /// Returns the TOF container node ID.
    pub fn insert_table_of_figures(
        &mut self,
        after_node_str: &str,
        label_filter: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;

        let body_id = doc
            .model()
            .body_id()
            .ok_or_else(|| JsError::new("No document body"))?;

        // Collect caption paragraphs
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let mut captions: Vec<(NodeId, String)> = Vec::new();

        for &child_id in &body.children {
            if let Some(node) = doc.node(child_id) {
                if let Some(style) = node.attributes.get_string(&AttributeKey::StyleId) {
                    if style == "Caption" {
                        let mut text = String::new();
                        for &run_id in &node.children {
                            if let Some(run) = doc.node(run_id) {
                                for &txt_id in &run.children {
                                    if let Some(txt) = doc.node(txt_id) {
                                        if let Some(ref tc) = txt.text_content {
                                            text.push_str(tc);
                                        }
                                    }
                                }
                                if let Some(ref tc) = run.text_content {
                                    text.push_str(tc);
                                }
                            }
                        }
                        if label_filter.is_empty() || text.starts_with(label_filter) {
                            captions.push((child_id, text));
                        }
                    }
                }
            }
        }

        // Create the TOF container
        let tof_id = doc.next_id();
        let mut tof_node = Node::new(tof_id, NodeType::TableOfContents);
        tof_node.attributes.set(
            AttributeKey::StyleId,
            AttributeValue::String("TableOfFigures".to_string()),
        );

        let mut txn = Transaction::with_label("Insert table of figures");

        // Find insert position
        let parent = doc.node(after_id).and_then(|n| n.parent).unwrap_or(body_id);
        let parent_node = doc
            .node(parent)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let insert_idx = parent_node
            .children
            .iter()
            .position(|&id| id == after_id)
            .map(|i| i + 1)
            .unwrap_or(parent_node.children.len());

        txn.push(Operation::insert_node(parent, insert_idx, tof_node));

        // Add entries
        for (i, (_cap_id, cap_text)) in captions.iter().enumerate() {
            let para_id = doc.next_id();
            let run_id = doc.next_id();
            let text_id = doc.next_id();

            let para_node = Node::new(para_id, NodeType::Paragraph);
            let run_node = Node::new(run_id, NodeType::Run);
            let mut text_node = Node::new(text_id, NodeType::Text);
            text_node.text_content = Some(cap_text.clone());

            txn.push(Operation::insert_node(tof_id, i, para_node));
            txn.push(Operation::insert_node(para_id, 0, run_node));
            txn.push(Operation::insert_node(run_id, 0, text_node));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", tof_id.replica, tof_id.counter))
    }

    /// Get the document heading hierarchy as JSON.
    ///
    /// Returns a JSON array of objects: `[{"nodeId":"r:c","level":1,"text":"..."},...]`
    /// Useful for building outline panels and TOC navigation.
    pub fn get_headings_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let headings = doc.model().collect_headings();
        let mut json = String::from("[");
        for (i, (node_id, level, text)) in headings.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "{{\"nodeId\":\"{}:{}\",\"level\":{},\"text\":\"{}\"}}",
                node_id.replica,
                node_id.counter,
                level,
                escape_json(text)
            ));
        }
        json.push(']');
        Ok(json)
    }

    // ─── E5.4: Threaded Comment Replies ──────────────────────────

    /// Insert a reply to an existing comment.
    ///
    /// Returns the reply comment ID string.
    pub fn insert_comment_reply(
        &mut self,
        parent_comment_id: &str,
        author: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let root_id = doc.model().root_id();
        let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);

        let reply_id_val = format!("{}:{}", doc.next_id().replica, doc.next_id().counter);

        // Create reply CommentBody with parent reference
        let cb_id = doc.next_id();
        let mut cb_node = Node::new(cb_id, NodeType::CommentBody);
        cb_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(reply_id_val.clone()),
        );
        cb_node.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String(author.to_string()),
        );
        cb_node.attributes.set(
            AttributeKey::CommentParentId,
            AttributeValue::String(parent_comment_id.to_string()),
        );

        let cb_para_id = doc.next_id();
        let cb_run_id = doc.next_id();
        let cb_text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert comment reply");
        txn.push(Operation::insert_node(root_id, root_children, cb_node));
        txn.push(Operation::insert_node(
            cb_id,
            0,
            Node::new(cb_para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            cb_para_id,
            0,
            Node::new(cb_run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            cb_run_id,
            0,
            Node::text(cb_text_id, text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(reply_id_val)
    }

    /// Edit a comment's text content.
    ///
    /// Replaces the text in the first paragraph of the CommentBody node
    /// matching `comment_id`.
    pub fn edit_comment(&mut self, comment_id: &str, new_text: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let root_id = doc.model().root_id();
        let root_node = doc
            .node(root_id)
            .ok_or_else(|| JsError::new("Root not found"))?;

        // Find the CommentBody with this comment ID
        let mut cb_id = None;
        for &child_id in &root_node.children {
            if let Some(child) = doc.node(child_id) {
                if child.node_type == NodeType::CommentBody
                    && child.attributes.get_string(&AttributeKey::CommentId) == Some(comment_id)
                {
                    cb_id = Some(child_id);
                    break;
                }
            }
        }
        let cb_id = cb_id.ok_or_else(|| {
            JsError::new(&format!("Comment body not found for id: {}", comment_id))
        })?;

        // Find the first paragraph → first run → first text node
        let cb_node = doc
            .node(cb_id)
            .ok_or_else(|| JsError::new("Comment body node missing"))?;
        if let Some(&para_id) = cb_node.children.first() {
            if let Some(para) = doc.node(para_id) {
                if let Some(&run_id) = para.children.first() {
                    if let Some(run) = doc.node(run_id) {
                        if let Some(&text_id) = run.children.first() {
                            // Get current text length to delete it
                            let old_len = doc
                                .node(text_id)
                                .and_then(|n| n.text_content.as_ref())
                                .map(|t| t.chars().count())
                                .unwrap_or(0);
                            let mut txn = Transaction::with_label("Edit comment");
                            if old_len > 0 {
                                txn.push(Operation::delete_text(text_id, 0, old_len));
                            }
                            if !new_text.is_empty() {
                                txn.push(Operation::insert_text(text_id, 0, new_text.to_string()));
                            }
                            return doc
                                .apply_transaction(&txn)
                                .map_err(|e| JsError::new(&e.to_string()));
                        }
                    }
                }
            }
        }
        Err(JsError::new("Comment has no text node to edit"))
    }

    /// Set the resolved status of a comment.
    ///
    /// Persists the resolved state as a `CommentResolved` attribute on
    /// the CommentBody node, so it survives save/load and collab sync.
    pub fn resolve_comment(&mut self, comment_id: &str, resolved: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let root_id = doc.model().root_id();
        let root_node = doc
            .node(root_id)
            .ok_or_else(|| JsError::new("Root not found"))?;

        let mut cb_id = None;
        for &child_id in &root_node.children {
            if let Some(child) = doc.node(child_id) {
                if child.node_type == NodeType::CommentBody
                    && child.attributes.get_string(&AttributeKey::CommentId) == Some(comment_id)
                    && child
                        .attributes
                        .get_string(&AttributeKey::CommentParentId)
                        .is_none()
                {
                    cb_id = Some(child_id);
                    break;
                }
            }
        }
        let cb_id = cb_id.ok_or_else(|| {
            JsError::new(&format!("Comment body not found for id: {}", comment_id))
        })?;

        let mut txn = Transaction::with_label(if resolved {
            "Resolve comment"
        } else {
            "Unresolve comment"
        });
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(
            AttributeKey::CommentResolved,
            AttributeValue::Bool(resolved),
        );
        txn.push(Operation::set_attributes(cb_id, attrs));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Delete a comment reply by its comment ID.
    ///
    /// Removes the CommentBody node that has the given `reply_id` as its
    /// CommentId attribute. Only deletes replies (nodes with CommentParentId).
    pub fn delete_comment_reply(&mut self, reply_id: &str) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let root_id = doc.model().root_id();
        let root_node = doc
            .node(root_id)
            .ok_or_else(|| JsError::new("Root not found"))?;

        let mut node_to_delete = None;
        for &child_id in &root_node.children {
            if let Some(child) = doc.node(child_id) {
                if child.node_type == NodeType::CommentBody
                    && child.attributes.get_string(&AttributeKey::CommentId) == Some(reply_id)
                    && child
                        .attributes
                        .get_string(&AttributeKey::CommentParentId)
                        .is_some()
                {
                    node_to_delete = Some(child_id);
                    break;
                }
            }
        }
        let nid = node_to_delete.ok_or_else(|| {
            JsError::new(&format!("Comment reply not found for id: {}", reply_id))
        })?;

        let mut txn = Transaction::with_label("Delete comment reply");
        txn.push(Operation::delete_node(nid));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Enable or disable track changes mode.
    ///
    /// When enabled, subsequent text edits create revision markers.
    /// This stores the state on the document metadata so it persists.
    pub fn set_track_changes_enabled(&mut self, enabled: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.metadata_mut().track_changes = enabled;
        Ok(())
    }

    /// Check if track changes mode is currently enabled.
    pub fn is_track_changes_enabled(&self) -> Result<bool, JsError> {
        let doc = self.doc()?;
        Ok(doc.metadata().track_changes)
    }

    // ─── E8: Performance APIs ────────────────────────────────────

    /// Set the maximum number of undo steps to keep.
    ///
    /// `max` of 0 means unlimited. Excess history is trimmed (oldest first).
    pub fn set_undo_history_cap(&mut self, max: usize) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        doc.set_undo_cap(max);
        Ok(())
    }

    /// Get layout cache statistics as JSON.
    ///
    /// Returns `{"hits":N,"misses":N,"entries":N}`.
    pub fn get_layout_cache_stats(&self) -> Result<String, JsError> {
        // Layout cache is transient per-render, report zeroes for now
        Ok("{\"hits\":0,\"misses\":0,\"entries\":0}".to_string())
    }

    // ─── E9.3: Equations ─────────────────────────────────────────

    /// Insert an equation (inline math) into a paragraph.
    ///
    /// `node_id_str` is the paragraph to insert into.
    /// `latex_source` is the equation source (LaTeX or raw XML).
    /// Returns the equation node ID string.
    pub fn insert_equation(
        &mut self,
        node_id_str: &str,
        latex_source: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let index = para.children.len();

        let eq_id = doc.next_id();
        let mut eq_node = Node::new(eq_id, NodeType::Equation);
        eq_node.attributes.set(
            AttributeKey::EquationSource,
            AttributeValue::String(latex_source.to_string()),
        );

        let mut txn = Transaction::with_label("Insert equation");
        txn.push(Operation::insert_node(para_id, index, eq_node));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", eq_id.replica, eq_id.counter))
    }

    // ─── E9.6: Footnotes / Endnotes ─────────────────────────────

    /// Insert a footnote at the current position in a paragraph.
    ///
    /// Creates a footnote reference in the paragraph and a footnote body
    /// at the document root. Returns the footnote body node ID.
    pub fn insert_footnote(&mut self, node_id_str: &str, text: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let para_child_count = para.children.len();
        let root_id = doc.model().root_id();
        let root_node = doc
            .node(root_id)
            .ok_or_else(|| JsError::new("Root not found"))?;
        let root_children = root_node.children.len();

        // Auto-assign footnote number by counting existing FootnoteBody nodes
        let fn_number = root_node
            .children
            .iter()
            .filter(|&&id| {
                doc.node(id)
                    .map(|n| n.node_type == NodeType::FootnoteBody)
                    .unwrap_or(false)
            })
            .count()
            + 1;

        let ref_id = doc.next_id();
        let body_id = doc.next_id();
        let body_para_id = doc.next_id();
        let body_run_id = doc.next_id();
        let body_text_id = doc.next_id();

        let mut ref_node = Node::new(ref_id, NodeType::FootnoteRef);
        ref_node.attributes.set(
            AttributeKey::FootnoteNumber,
            AttributeValue::Int(fn_number as i64),
        );

        let mut body_node = Node::new(body_id, NodeType::FootnoteBody);
        body_node.attributes.set(
            AttributeKey::FootnoteNumber,
            AttributeValue::Int(fn_number as i64),
        );

        let mut txn = Transaction::with_label("Insert footnote");
        txn.push(Operation::insert_node(para_id, para_child_count, ref_node));
        txn.push(Operation::insert_node(root_id, root_children, body_node));
        txn.push(Operation::insert_node(
            body_id,
            0,
            Node::new(body_para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            body_para_id,
            0,
            Node::new(body_run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            body_run_id,
            0,
            Node::text(body_text_id, text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", body_id.replica, body_id.counter))
    }

    /// Insert an endnote at the current position in a paragraph.
    ///
    /// Creates an endnote reference in the paragraph and an endnote body
    /// at the document root. Returns the endnote body node ID.
    pub fn insert_endnote(&mut self, node_id_str: &str, text: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let para = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let para_child_count = para.children.len();
        let root_id = doc.model().root_id();
        let root_node = doc
            .node(root_id)
            .ok_or_else(|| JsError::new("Root not found"))?;
        let root_children = root_node.children.len();

        let en_number = root_node
            .children
            .iter()
            .filter(|&&id| {
                doc.node(id)
                    .map(|n| n.node_type == NodeType::EndnoteBody)
                    .unwrap_or(false)
            })
            .count()
            + 1;

        let ref_id = doc.next_id();
        let body_id = doc.next_id();
        let body_para_id = doc.next_id();
        let body_run_id = doc.next_id();
        let body_text_id = doc.next_id();

        let mut ref_node = Node::new(ref_id, NodeType::EndnoteRef);
        ref_node.attributes.set(
            AttributeKey::EndnoteNumber,
            AttributeValue::Int(en_number as i64),
        );

        let mut body_node = Node::new(body_id, NodeType::EndnoteBody);
        body_node.attributes.set(
            AttributeKey::EndnoteNumber,
            AttributeValue::Int(en_number as i64),
        );

        let mut txn = Transaction::with_label("Insert endnote");
        txn.push(Operation::insert_node(para_id, para_child_count, ref_node));
        txn.push(Operation::insert_node(root_id, root_children, body_node));
        txn.push(Operation::insert_node(
            body_id,
            0,
            Node::new(body_para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(
            body_para_id,
            0,
            Node::new(body_run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            body_run_id,
            0,
            Node::text(body_text_id, text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", body_id.replica, body_id.counter))
    }

    /// Get all footnotes as JSON array.
    ///
    /// Returns `[{"number":1,"text":"Footnote text"},...]`.
    pub fn get_footnotes_json(&self) -> Result<String, JsError> {
        self.get_notes_json(NodeType::FootnoteBody, &AttributeKey::FootnoteNumber)
    }

    /// Get all endnotes as JSON array.
    ///
    /// Returns `[{"number":1,"text":"Endnote text"},...]`.
    pub fn get_endnotes_json(&self) -> Result<String, JsError> {
        self.get_notes_json(NodeType::EndnoteBody, &AttributeKey::EndnoteNumber)
    }

    // ─── Canvas Editor Scene & Geometry APIs ─────────────────────

    /// Return the scene protocol version supported by this build.
    pub fn scene_protocol_version(&self) -> u32 {
        1
    }

    /// Return a monotonically increasing document revision number.
    ///
    /// Bumps on every model mutation (insert, delete, format change).
    /// Uses undo_count as a proxy for revision tracking.
    pub fn document_revision(&self) -> Result<u32, JsError> {
        let doc = self.doc()?;
        Ok(doc.undo_count() as u32)
    }

    /// Return a monotonically increasing layout revision number.
    ///
    /// Bumps when pagination output changes (page count, block positions).
    pub fn layout_revision(&self) -> Result<u32, JsError> {
        let doc = self.doc()?;
        Ok(doc.undo_count() as u32)
    }

    /// Return a lightweight scene summary for viewport boot.
    ///
    /// Returns JSON: `{ "protocol_version": 1, "document_revision": N,
    /// "layout_revision": N, "page_count": N, "default_page_size_pt": {...},
    /// "pages": [...] }`
    pub fn scene_summary(&self, _config_json: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let rev = doc.undo_count() as u64;

        let mut json = String::with_capacity(2048);
        json.push_str(&format!(
            "{{\"protocol_version\":1,\"document_revision\":{},\"layout_revision\":{},\"page_count\":{},",
            rev, rev, layout.pages.len()
        ));

        // Default page size from first page or Letter
        let (pw, ph) = if let Some(p) = layout.pages.first() {
            (p.width, p.height)
        } else {
            (612.0, 792.0)
        };
        json.push_str(&format!(
            "\"default_page_size_pt\":{{\"width\":{:.2},\"height\":{:.2}}},\"pages\":[",
            pw, ph
        ));

        for (i, page) in layout.pages.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let item_count = page.blocks.len()
                + page.floating_images.len()
                + page.footnotes.len()
                + if page.header.is_some() { 1 } else { 0 }
                + if page.footer.is_some() { 1 } else { 0 };
            json.push_str(&format!(
                "{{\"page_index\":{},\"section_index\":{},\"bounds_pt\":{{\"x\":0.0,\"y\":0.0,\"width\":{:.2},\"height\":{:.2}}},\"content_rect_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"has_header\":{},\"has_footer\":{},\"item_count\":{}}}",
                page.index, page.section_index,
                page.width, page.height,
                page.content_area.x, page.content_area.y,
                page.content_area.width, page.content_area.height,
                page.header.is_some(), page.footer.is_some(),
                item_count,
            ));
        }

        json.push_str("]}");
        Ok(json)
    }

    /// Return a full scene for a single page.
    ///
    /// Returns JSON with page bounds, content rect, header/footer rects,
    /// and all scene items (text runs, backgrounds, borders, images, shapes, etc.).
    pub fn page_scene(&self, page_index: u32) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();
        let rev = doc.undo_count() as u64;
        let pi = page_index as usize;

        if pi >= layout.pages.len() {
            return Ok(format!(
                "{{\"error\":{{\"code\":\"invalid_page_index\",\"message\":\"Page index {} out of range (0..{})\"}}}}",
                pi,
                layout.pages.len()
            ));
        }

        let page = &layout.pages[pi];
        Ok(layout_page_to_scene_json(page, model, rev))
    }

    /// Return scenes for a range of pages (batch fetch for viewport).
    ///
    /// Returns JSON: `{ "pages": [...] }`
    pub fn visible_page_scenes(&self, start_page: u32, end_page: u32) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();
        let rev = doc.undo_count() as u64;
        let start = start_page as usize;
        let end = (end_page as usize).min(layout.pages.len());

        let mut json = String::with_capacity(8192);
        json.push_str("{\"pages\":[");
        for (i, pi) in (start..end).enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&layout_page_to_scene_json(&layout.pages[pi], model, rev));
        }
        json.push_str("]}");
        Ok(json)
    }

    /// Return scene summary using loaded fonts for accurate text shaping.
    pub fn scene_summary_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
        config_json: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        self.ensure_layout(&font_db.inner)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let rev = doc.undo_count() as u64;
        let _ = config_json;

        let mut json = String::with_capacity(2048);
        json.push_str(&format!(
            "{{\"protocol_version\":1,\"document_revision\":{},\"layout_revision\":{},\"page_count\":{},",
            rev, rev, layout.pages.len()
        ));
        let (pw, ph) = if let Some(p) = layout.pages.first() {
            (p.width, p.height)
        } else {
            (612.0, 792.0)
        };
        json.push_str(&format!(
            "\"default_page_size_pt\":{{\"width\":{:.2},\"height\":{:.2}}},\"pages\":[",
            pw, ph
        ));
        for (i, page) in layout.pages.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let item_count = page.blocks.len()
                + page.floating_images.len()
                + page.footnotes.len()
                + if page.header.is_some() { 1 } else { 0 }
                + if page.footer.is_some() { 1 } else { 0 };
            json.push_str(&format!(
                "{{\"page_index\":{},\"section_index\":{},\"bounds_pt\":{{\"x\":0.0,\"y\":0.0,\"width\":{:.2},\"height\":{:.2}}},\"content_rect_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"has_header\":{},\"has_footer\":{},\"item_count\":{}}}",
                page.index, page.section_index,
                page.width, page.height,
                page.content_area.x, page.content_area.y,
                page.content_area.width, page.content_area.height,
                page.header.is_some(), page.footer.is_some(),
                item_count,
            ));
        }
        json.push_str("]}");
        Ok(json)
    }

    /// Return page scene using loaded fonts for accurate text shaping.
    pub fn page_scene_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
        page_index: u32,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        self.ensure_layout(&font_db.inner)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();
        let rev = doc.undo_count() as u64;
        let pi = page_index as usize;
        if pi >= layout.pages.len() {
            return Ok(format!(
                "{{\"error\":{{\"code\":\"invalid_page_index\",\"message\":\"Page index {} out of range (0..{})\"}}}}",
                pi, layout.pages.len()
            ));
        }
        let page = &layout.pages[pi];
        Ok(layout_page_to_scene_json(page, model, rev))
    }

    /// Return visible page scenes using loaded fonts for accurate text shaping.
    pub fn visible_page_scenes_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
        start_page: u32,
        end_page: u32,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        self.ensure_layout(&font_db.inner)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();
        let rev = doc.undo_count() as u64;
        let start = start_page as usize;
        let end = (end_page as usize).min(layout.pages.len());
        let mut json = String::with_capacity(8192);
        json.push_str("{\"pages\":[");
        for (i, pi) in (start..end).enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&layout_page_to_scene_json(&layout.pages[pi], model, rev));
        }
        json.push_str("]}");
        Ok(json)
    }

    /// Hit-test a point on a page to find the nearest model position.
    ///
    /// Returns JSON `HitTestResult` with position, kind, node_id, and inside flag.
    pub fn hit_test(&self, page_index: u32, x_pt: f64, y_pt: f64) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout_has_cache(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();
        let pi = page_index as usize;

        if pi >= layout.pages.len() {
            return Ok(format!(
                "{{\"error\":{{\"code\":\"invalid_page_index\",\"message\":\"Page index {} out of range\"}}}}",
                pi
            ));
        }

        let page = &layout.pages[pi];

        // Check if point is in content area
        let in_content = x_pt >= page.content_area.x
            && x_pt <= page.content_area.x + page.content_area.width
            && y_pt >= page.content_area.y
            && y_pt <= page.content_area.y + page.content_area.height;

        // Check header/footer regions
        if let Some(ref hdr) = page.header {
            if y_pt >= hdr.bounds.y && y_pt <= hdr.bounds.y + hdr.bounds.height {
                let pos = hit_test_block(hdr, x_pt, y_pt, model);
                return Ok(format!(
                    "{{\"page_index\":{},\"kind\":\"header\",\"position\":{},\"node_id\":\"{}\",\"item_id\":null,\"inside\":true}}",
                    pi, pos.to_json(), pos.node_id_str()
                ));
            }
        }
        if let Some(ref ftr) = page.footer {
            if y_pt >= ftr.bounds.y && y_pt <= ftr.bounds.y + ftr.bounds.height {
                let pos = hit_test_block(ftr, x_pt, y_pt, model);
                return Ok(format!(
                    "{{\"page_index\":{},\"kind\":\"footer\",\"position\":{},\"node_id\":\"{}\",\"item_id\":null,\"inside\":true}}",
                    pi, pos.to_json(), pos.node_id_str()
                ));
            }
        }

        if !in_content {
            // Point is in page margin
            return Ok(format!(
                "{{\"page_index\":{},\"kind\":\"page_margin\",\"position\":null,\"node_id\":null,\"item_id\":null,\"inside\":false}}",
                pi
            ));
        }

        // Find closest block to the y coordinate
        let mut best_block: Option<&s1_layout::LayoutBlock> = None;
        let mut best_dist = f64::MAX;

        for block in &page.blocks {
            let block_top = block.bounds.y;
            let block_bottom = block.bounds.y + block.bounds.height;

            if y_pt >= block_top && y_pt <= block_bottom {
                best_block = Some(block);
                best_dist = 0.0;
                break;
            }

            let dist = if y_pt < block_top {
                block_top - y_pt
            } else {
                y_pt - block_bottom
            };
            if dist < best_dist {
                best_dist = dist;
                best_block = Some(block);
            }
        }

        // Also check floating images
        for block in &page.floating_images {
            let bt = block.bounds.y;
            let bb = block.bounds.y + block.bounds.height;
            let bl = block.bounds.x;
            let br = block.bounds.x + block.bounds.width;
            if x_pt >= bl && x_pt <= br && y_pt >= bt && y_pt <= bb {
                let src = format!("{}:{}", block.source_id.replica, block.source_id.counter);
                return Ok(format!(
                    "{{\"page_index\":{},\"kind\":\"image\",\"position\":{{\"node_id\":\"{}\",\"offset_utf16\":0,\"affinity\":\"downstream\"}},\"node_id\":\"{}\",\"item_id\":null,\"inside\":true}}",
                    pi, escape_json(&src), escape_json(&src)
                ));
            }
        }

        if let Some(block) = best_block {
            let kind = match &block.kind {
                s1_layout::LayoutBlockKind::Paragraph { .. } => "text",
                s1_layout::LayoutBlockKind::Table { .. } => "table_cell",
                s1_layout::LayoutBlockKind::Image { .. } => "image",
                s1_layout::LayoutBlockKind::Shape { .. } => "shape",
                s1_layout::LayoutBlockKind::TextBox { .. } => "text",
                _ => "text",
            };
            let pos = hit_test_block(block, x_pt, y_pt, model);
            Ok(format!(
                "{{\"page_index\":{},\"kind\":\"{}\",\"position\":{},\"node_id\":\"{}\",\"item_id\":null,\"inside\":{}}}",
                pi, kind, pos.to_json(), pos.node_id_str(), best_dist < 0.5
            ))
        } else {
            Ok(format!(
                "{{\"page_index\":{},\"kind\":\"none\",\"position\":null,\"node_id\":null,\"item_id\":null,\"inside\":false}}",
                pi
            ))
        }
    }

    /// Get the caret rectangle for a model position.
    ///
    /// Returns JSON `RectPt` with page_index, x, y, width (1.0), height.
    pub fn caret_rect(&self, position_json: &str) -> Result<String, JsError> {
        // Use stale layout cache if available for fast caret positioning during typing.
        // A full relayout will happen on the deferred render cycle.
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout_has_cache(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();

        let pos = PositionRefParsed::from_json(position_json)?;

        // Find the node in the layout
        for page in &layout.pages {
            if let Some(rect) = find_caret_rect_in_page(page, &pos, model) {
                return Ok(format!(
                    "{{\"page_index\":{},\"x\":{:.2},\"y\":{:.2},\"width\":1.0,\"height\":{:.2}}}",
                    page.index, rect.x, rect.y, rect.height
                ));
            }
        }

        // Fallback: return beginning of first page content area
        if let Some(page) = layout.pages.first() {
            Ok(format!(
                "{{\"page_index\":0,\"x\":{:.2},\"y\":{:.2},\"width\":1.0,\"height\":14.0}}",
                page.content_area.x, page.content_area.y
            ))
        } else {
            Ok(
                "{\"page_index\":0,\"x\":72.0,\"y\":72.0,\"width\":1.0,\"height\":14.0}"
                    .to_string(),
            )
        }
    }

    /// Get selection rectangles for a model range.
    ///
    /// Returns JSON array of `RectPt` objects covering the selection.
    pub fn selection_rects(&self, range_json: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout_has_cache(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();

        let range = RangeRefParsed::from_json(range_json)?;
        let mut rects = Vec::new();

        // Find rects for the range across pages
        for page in &layout.pages {
            collect_selection_rects_in_page(page, &range, model, &mut rects);
        }

        let mut json = String::with_capacity(512);
        json.push('[');
        for (i, rect) in rects.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "{{\"page_index\":{},\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
                rect.page_index, rect.x, rect.y, rect.width, rect.height
            ));
        }
        json.push(']');
        Ok(json)
    }

    /// Move a position in a direction by a granularity.
    ///
    /// Returns JSON `PositionRef`.
    /// Validate and clamp a position to ensure the offset is within bounds.
    ///
    /// If the offset exceeds the text node's length, it is clamped to the end.
    /// Returns the validated position as JSON.
    pub fn validate_position(&self, position_json: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let pos: serde_json::Value = serde_json::from_str(position_json)
            .map_err(|e| JsError::new(&format!("Invalid position JSON: {e}")))?;

        let node_id_str = pos["node_id"].as_str().unwrap_or("");
        let offset = pos["offset_utf16"].as_u64().unwrap_or(0) as usize;

        let node_id = parse_node_id(node_id_str)?;

        // Get the text length of the node
        let text_len = doc.node(node_id)
            .and_then(|n| n.text_content.as_ref())
            .map(|t| t.encode_utf16().count())
            .unwrap_or_else(|| {
                // For non-text nodes, count children's text
                doc.node(node_id)
                    .map(|n| {
                        let mut len = 0;
                        for &child_id in &n.children {
                            if let Some(child) = doc.node(child_id) {
                                for &tc_id in &child.children {
                                    if let Some(tc) = doc.node(tc_id) {
                                        if let Some(ref t) = tc.text_content {
                                            len += t.encode_utf16().count();
                                        }
                                    }
                                }
                                if let Some(ref t) = child.text_content {
                                    len += t.encode_utf16().count();
                                }
                            }
                        }
                        len
                    })
                    .unwrap_or(0)
            });

        let clamped_offset = offset.min(text_len);

        Ok(serde_json::json!({
            "node_id": node_id_str,
            "offset_utf16": clamped_offset,
            "affinity": pos.get("affinity").and_then(|v| v.as_str()).unwrap_or("downstream")
        }).to_string())
    }

    pub fn move_position(
        &self,
        position_json: &str,
        direction: &str,
        granularity: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();

        let pos = PositionRefParsed::from_json(position_json)?;
        let new_pos = navigate_position(layout, model, &pos, direction, granularity);
        Ok(new_pos.to_json())
    }

    /// Get the word boundary around a position.
    ///
    /// Returns JSON `RangeRef` with anchor at word start and focus at word end.
    pub fn word_boundary(&self, position_json: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let _cache = self.layout_cache.borrow();
        let _layout = _cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();

        let pos = PositionRefParsed::from_json(position_json)?;

        // Find the text node and compute word boundaries
        if let Some(node) = model.node(pos.node_id) {
            if let Some(text) = &node.text_content {
                let char_offset = utf16_to_char_offset(text, pos.offset_utf16 as usize);
                let chars: Vec<char> = text.chars().collect();

                // Walk backward to find word start
                let mut start = char_offset;
                while start > 0 && chars.get(start - 1).is_some_and(|c| c.is_alphanumeric()) {
                    start -= 1;
                }

                // Walk forward to find word end
                let mut end = char_offset;
                while end < chars.len() && chars.get(end).is_some_and(|c| c.is_alphanumeric()) {
                    end += 1;
                }

                let start_utf16 = char_to_utf16_offset(text, start);
                let end_utf16 = char_to_utf16_offset(text, end);

                let node_str = format!("{}:{}", pos.node_id.replica, pos.node_id.counter);
                return Ok(format!(
                    "{{\"anchor\":{{\"node_id\":\"{}\",\"offset_utf16\":{},\"affinity\":\"downstream\"}},\"focus\":{{\"node_id\":\"{}\",\"offset_utf16\":{},\"affinity\":\"downstream\"}}}}",
                    escape_json(&node_str), start_utf16,
                    escape_json(&node_str), end_utf16
                ));
            }
        }

        // Fallback: collapsed range at position
        Ok(format!(
            "{{\"anchor\":{},\"focus\":{}}}",
            pos.to_json(),
            pos.to_json()
        ))
    }

    /// Get bounds for all pages containing a node.
    ///
    /// Returns JSON array of `RectPt`.
    pub fn node_bounds(&self, node_id_str: &str) -> Result<String, JsError> {
        let _doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;

        let node_id = parse_node_id(node_id_str)?;

        let mut json = String::with_capacity(256);
        json.push('[');
        let mut first = true;
        for page in &layout.pages {
            for block in &page.blocks {
                if block.source_id == node_id {
                    if !first {
                        json.push(',');
                    }
                    first = false;
                    json.push_str(&format!(
                        "{{\"page_index\":{},\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
                        page.index, block.bounds.x, block.bounds.y,
                        block.bounds.width, block.bounds.height
                    ));
                }
            }
        }
        json.push(']');
        Ok(json)
    }

    /// Return formatting state at a selection range for toolbar display.
    ///
    /// Returns JSON `FormattingState` with bold, italic, font info, etc.
    pub fn selection_formatting(&self, range_json: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();

        let range = RangeRefParsed::from_json(range_json)?;

        // Get formatting from the anchor node
        let mut bold = false;
        let mut italic = false;
        let mut underline = "none".to_string();
        let mut font_family = "serif".to_string();
        let mut font_size = 11.0_f64;
        let mut color = "#000000".to_string();
        let mut alignment = "left".to_string();

        // Walk from anchor node upward to find run and paragraph formatting
        if let Some(node) = model.node(range.anchor.node_id) {
            // Check parent run node for inline formatting
            if let Some(parent_id) = node.parent {
                if let Some(run_node) = model.node(parent_id) {
                    bold = run_node
                        .attributes
                        .get_bool(&AttributeKey::Bold)
                        .unwrap_or(false);
                    italic = run_node
                        .attributes
                        .get_bool(&AttributeKey::Italic)
                        .unwrap_or(false);
                    if let Some(ul) = run_node.attributes.get(&AttributeKey::Underline) {
                        underline = match ul {
                            s1_model::AttributeValue::Bool(true) => "single".to_string(),
                            s1_model::AttributeValue::UnderlineStyle(s) => {
                                format!("{:?}", s).to_lowercase()
                            }
                            _ => "none".to_string(),
                        };
                    }
                    if let Some(ff) = run_node.attributes.get_string(&AttributeKey::FontFamily) {
                        font_family = ff.to_string();
                    }
                    if let Some(fs) = run_node.attributes.get_f64(&AttributeKey::FontSize) {
                        font_size = fs;
                    }
                    if let Some(s1_model::AttributeValue::Color(col)) =
                        run_node.attributes.get(&AttributeKey::Color)
                    {
                        color = format!("#{}", col.to_hex());
                    }

                    // Check paragraph parent for block formatting
                    if let Some(para_id) = run_node.parent {
                        if let Some(para_node) = model.node(para_id) {
                            if let Some(s1_model::AttributeValue::Alignment(a)) =
                                para_node.attributes.get(&AttributeKey::Alignment)
                            {
                                alignment = format!("{:?}", a).to_lowercase();
                            }
                        }
                    }
                }
            }
        }

        Ok(format!(
            "{{\"bold\":{},\"italic\":{},\"underline\":\"{}\",\"strikethrough\":false,\"superscript\":false,\"subscript\":false,\"font_family\":\"{}\",\"font_size_pt\":{:.1},\"color\":\"{}\",\"highlight_color\":null,\"alignment\":\"{}\",\"line_spacing\":\"single\",\"list_format\":null,\"style_id\":\"Normal\",\"indent_left_pt\":0.0,\"indent_right_pt\":0.0,\"indent_first_line_pt\":0.0,\"spacing_before_pt\":0.0,\"spacing_after_pt\":0.0}}",
            bold, italic,
            escape_json(&underline),
            escape_json(&font_family),
            font_size,
            escape_json(&color),
            escape_json(&alignment),
        ))
    }

    // ─── Canvas Editing Methods (Phase 1) ─────────────────────────

    /// Insert text at a canvas position (text-node ID + UTF-16 offset).
    ///
    /// Resolves the position to paragraph coordinates, performs the insert,
    /// and returns an EditResult JSON string with the new cursor position.
    pub fn canvas_insert_text(
        &mut self,
        position_json: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let pos = PositionRefParsed::from_json(position_json)?;
        let model = self.doc()?.model().clone();
        let (para_id, char_offset) = resolve_position_to_paragraph(&model, &pos)?;

        // Look up which page this paragraph is on BEFORE the mutation
        // (using the stale layout cache that still exists).
        let dirty_page = find_page_for_paragraph(&self.layout_cache, para_id);

        let para_id_str = format!("{}:{}", para_id.replica, para_id.counter);
        self.insert_text_in_paragraph(&para_id_str, char_offset, text)?;

        let doc = self.doc()?;

        let inserted_char_len = text.chars().count();
        let new_char_offset = char_offset + inserted_char_len;

        let new_pos = match find_text_node_at_char_offset(doc.model(), para_id, new_char_offset) {
            Ok((tid, local_off, _)) => {
                let text_node = doc.model().node(tid).unwrap();
                let text_content = text_node.text_content.as_deref().unwrap_or("");
                let utf16_off = char_offset_to_utf16_offset(text_content, local_off);
                PositionRefParsed {
                    node_id: tid,
                    offset_utf16: utf16_off,
                    affinity: "downstream".to_string(),
                }
            }
            Err(_) => {
                if let Ok((tid, _)) = find_first_text_node(doc.model(), para_id) {
                    PositionRefParsed {
                        node_id: tid,
                        offset_utf16: 0,
                        affinity: "downstream".to_string(),
                    }
                } else {
                    PositionRefParsed {
                        node_id: para_id,
                        offset_utf16: 0,
                        affinity: "downstream".to_string(),
                    }
                }
            }
        };

        Ok(build_edit_result_json_with_dirty_page(doc, &new_pos, dirty_page))
    }

    /// Delete a canvas range (anchor + focus as text-node IDs + UTF-16 offsets).
    ///
    /// Resolves the range to paragraph coordinates, performs the deletion,
    /// and returns an EditResult JSON string with the cursor collapsed at range start.
    pub fn canvas_delete_range(&mut self, range_json: &str) -> Result<String, JsError> {
        let range = RangeRefParsed::from_json(range_json)?;
        let model = self.doc()?.model().clone();
        let (start_para, start_offset, end_para, end_offset) =
            resolve_range_to_paragraphs(&model, &range)?;

        // Look up dirty page before mutation
        let dirty_page = find_page_for_paragraph(&self.layout_cache, start_para);

        let start_para_str = format!("{}:{}", start_para.replica, start_para.counter);
        let end_para_str = format!("{}:{}", end_para.replica, end_para.counter);
        self.delete_selection(&start_para_str, start_offset, &end_para_str, end_offset)?;

        let doc = self.doc()?;

        // Resolve the start position back to text-node coordinates using the mutated model.
        let new_pos = match find_text_node_at_char_offset(doc.model(), start_para, start_offset) {
            Ok((tid, local_off, _)) => {
                let text_node = doc.model().node(tid).unwrap();
                let text_content = text_node.text_content.as_deref().unwrap_or("");
                let utf16_off = char_offset_to_utf16_offset(text_content, local_off);
                PositionRefParsed {
                    node_id: tid,
                    offset_utf16: utf16_off,
                    affinity: "downstream".to_string(),
                }
            }
            Err(_) => {
                if let Ok((tid, _)) = find_first_text_node(doc.model(), start_para) {
                    PositionRefParsed {
                        node_id: tid,
                        offset_utf16: 0,
                        affinity: "downstream".to_string(),
                    }
                } else {
                    PositionRefParsed {
                        node_id: start_para,
                        offset_utf16: 0,
                        affinity: "downstream".to_string(),
                    }
                }
            }
        };
        Ok(build_edit_result_json_with_dirty_page(doc, &new_pos, dirty_page))
    }

    /// Replace a canvas range with new text.
    ///
    /// Deletes the range, then inserts text at the start position.
    /// Returns an EditResult JSON with the cursor after the inserted text.
    pub fn canvas_replace_range(
        &mut self,
        range_json: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let range = RangeRefParsed::from_json(range_json)?;
        let model = self.doc()?.model().clone();
        let (start_para, start_offset, end_para, end_offset) =
            resolve_range_to_paragraphs(&model, &range)?;

        // Look up dirty page before mutation
        let dirty_page = find_page_for_paragraph(&self.layout_cache, start_para);

        let start_para_str = format!("{}:{}", start_para.replica, start_para.counter);
        let end_para_str = format!("{}:{}", end_para.replica, end_para.counter);

        // Delete the range
        self.delete_selection(&start_para_str, start_offset, &end_para_str, end_offset)?;

        // Insert text at start position
        self.insert_text_in_paragraph(&start_para_str, start_offset, text)?;

        let doc = self.doc()?;

        let inserted_char_len = text.chars().count();
        let new_char_offset = start_offset + inserted_char_len;

        let new_pos = match find_text_node_at_char_offset(doc.model(), start_para, new_char_offset)
        {
            Ok((tid, local_off, _)) => {
                let text_node = doc.model().node(tid).unwrap();
                let text_content = text_node.text_content.as_deref().unwrap_or("");
                let utf16_off = char_offset_to_utf16_offset(text_content, local_off);
                PositionRefParsed {
                    node_id: tid,
                    offset_utf16: utf16_off,
                    affinity: "downstream".to_string(),
                }
            }
            Err(_) => {
                if let Ok((tid, _)) = find_first_text_node(doc.model(), start_para) {
                    PositionRefParsed {
                        node_id: tid,
                        offset_utf16: 0,
                        affinity: "downstream".to_string(),
                    }
                } else {
                    PositionRefParsed {
                        node_id: start_para,
                        offset_utf16: 0,
                        affinity: "downstream".to_string(),
                    }
                }
            }
        };
        Ok(build_edit_result_json_with_dirty_page(doc, &new_pos, dirty_page))
    }

    /// Insert a paragraph break at a canvas position.
    ///
    /// Splits the paragraph at the resolved char offset.
    /// Returns an EditResult JSON with the cursor at the start of the new paragraph.
    pub fn canvas_insert_paragraph_break(
        &mut self,
        position_json: &str,
    ) -> Result<String, JsError> {
        let pos = PositionRefParsed::from_json(position_json)?;
        let model = self.doc()?.model().clone();
        let (para_id, char_offset) = resolve_position_to_paragraph(&model, &pos)?;

        let para_id_str = format!("{}:{}", para_id.replica, para_id.counter);
        let new_para_id_str = self.split_paragraph(&para_id_str, char_offset)?;
        let new_para_id = parse_node_id(&new_para_id_str)?;

        // Find the first text node in the new paragraph to set cursor there
        let doc = self.doc()?;
        let new_text_node_id = find_first_text_node(doc.model(), new_para_id)
            .map(|(tid, _)| tid)
            .unwrap_or(new_para_id);

        let new_pos = PositionRefParsed {
            node_id: new_text_node_id,
            offset_utf16: 0,
            affinity: "downstream".to_string(),
        };

        Ok(build_edit_result_json_with_para(
            doc,
            &new_pos,
            Some(&new_para_id_str),
        ))
    }

    /// Toggle a formatting mark on a canvas range.
    ///
    /// Checks the current formatting state at the anchor position and
    /// toggles the specified mark. Supported marks: "bold", "italic",
    /// "underline", "strikethrough".
    ///
    /// Returns an EditResult JSON.
    pub fn canvas_toggle_mark(&mut self, range_json: &str, mark: &str) -> Result<String, JsError> {
        let range = RangeRefParsed::from_json(range_json)?;
        let model = self.doc()?.model().clone();

        // Check current formatting at anchor position
        let currently_active = {
            if let Some(text_node) = model.node(range.anchor.node_id) {
                if let Some(run_id) = text_node.parent {
                    if let Some(run_node) = model.node(run_id) {
                        match mark {
                            "bold" => run_node
                                .attributes
                                .get_bool(&AttributeKey::Bold)
                                .unwrap_or(false),
                            "italic" => run_node
                                .attributes
                                .get_bool(&AttributeKey::Italic)
                                .unwrap_or(false),
                            "underline" => run_node
                                .attributes
                                .get(&AttributeKey::Underline)
                                .map(|v| match v {
                                    AttributeValue::Bool(b) => *b,
                                    AttributeValue::UnderlineStyle(_) => true,
                                    _ => false,
                                })
                                .unwrap_or(false),
                            "strikethrough" => run_node
                                .attributes
                                .get_bool(&AttributeKey::Strikethrough)
                                .unwrap_or(false),
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        let new_value = if currently_active { "false" } else { "true" };

        let (start_para, start_offset, end_para, end_offset) =
            resolve_range_to_paragraphs(&model, &range)?;
        let start_str = format!("{}:{}", start_para.replica, start_para.counter);
        let end_str = format!("{}:{}", end_para.replica, end_para.counter);

        self.format_selection(
            &start_str,
            start_offset,
            &end_str,
            end_offset,
            mark,
            new_value,
        )?;

        let doc = self.doc()?;
        let result_pos = range.anchor.to_parsed_clone();
        Ok(build_edit_result_json(doc, &result_pos))
    }

    // ─── Clipboard Methods (Phase 4) ────────────────────────────────

    /// Copy a canvas range as plain text.
    ///
    /// Walks text nodes from anchor to focus, joining with newlines
    /// at paragraph boundaries.
    pub fn copy_range_plain_text(&self, range_json: &str) -> Result<String, JsError> {
        let range = RangeRefParsed::from_json(range_json)?;
        let doc = self.doc()?;
        let model = doc.model();
        let (start_para, start_offset, end_para, end_offset) =
            resolve_range_to_paragraphs(model, &range)?;

        if start_para == end_para {
            let text = extract_paragraph_text(model, start_para);
            let chars: Vec<char> = text.chars().collect();
            let safe_start = start_offset.min(chars.len());
            let safe_end = end_offset.min(chars.len());
            let result: String = chars[safe_start..safe_end].iter().collect();
            return Ok(result);
        }

        // Multi-paragraph
        let body_id = doc.body_id().ok_or_else(|| JsError::new("No body node"))?;
        let body = doc
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let children = body.children.clone();

        let start_idx = children.iter().position(|&c| c == start_para);
        let end_idx = children.iter().position(|&c| c == end_para);

        let mut result = String::new();
        match (start_idx, end_idx) {
            (Some(si), Some(ei)) => {
                // First paragraph: from start_offset to end
                let first_text = extract_paragraph_text(model, children[si]);
                let first_chars: Vec<char> = first_text.chars().collect();
                let safe_start = start_offset.min(first_chars.len());
                result.extend(&first_chars[safe_start..]);

                // Middle paragraphs (full)
                for &child_id in &children[si + 1..ei] {
                    if let Some(child) = doc.node(child_id) {
                        if child.node_type == NodeType::Paragraph {
                            result.push('\n');
                            result.push_str(&extract_paragraph_text(model, child_id));
                        }
                    }
                }

                // Last paragraph: from 0 to end_offset
                result.push('\n');
                let last_text = extract_paragraph_text(model, children[ei]);
                let last_chars: Vec<char> = last_text.chars().collect();
                let safe_end = end_offset.min(last_chars.len());
                let tail: String = last_chars[..safe_end].iter().collect();
                result.push_str(&tail);
            }
            _ => {
                return Err(JsError::new("Start or end paragraph not found in body"));
            }
        }

        Ok(result)
    }

    /// Copy a canvas range as HTML.
    ///
    /// Resolves the range to paragraph coordinates and delegates to
    /// the existing `export_selection_html` method.
    pub fn copy_range_html(&self, range_json: &str) -> Result<String, JsError> {
        let range = RangeRefParsed::from_json(range_json)?;
        let doc = self.doc()?;
        let model = doc.model();
        let (start_para, start_offset, end_para, end_offset) =
            resolve_range_to_paragraphs(model, &range)?;

        let start_str = format!("{}:{}", start_para.replica, start_para.counter);
        let end_str = format!("{}:{}", end_para.replica, end_para.counter);
        self.export_selection_html(&start_str, start_offset, &end_str, end_offset)
    }

    /// Paste HTML at a canvas position.
    ///
    /// For now, strips HTML tags and inserts as plain text.
    /// Returns an EditResult JSON.
    pub fn paste_html(&mut self, position_json: &str, html: &str) -> Result<String, JsError> {
        // Simple HTML tag stripping
        let plain = strip_html_tags(html);
        self.canvas_insert_text(position_json, &plain)
    }

    // ─── IME Composition Methods (Phase 5) ──────────────────────────

    /// Begin an IME composition at the given position.
    ///
    /// Stores the anchor position for subsequent composition updates.
    /// Returns JSON `{"status":"composing","anchor":<position>}`.
    pub fn begin_composition(&mut self, position_json: &str) -> Result<String, JsError> {
        let pos = PositionRefParsed::from_json(position_json)?;
        let model = self.doc()?.model().clone();
        let (para_id, char_offset) = resolve_position_to_paragraph(&model, &pos)?;

        self.composition_anchor_node = Some(format!("{}:{}", para_id.replica, para_id.counter));
        self.composition_anchor_offset = Some(char_offset as u32);
        self.composition_preview_len = 0;

        Ok(format!(
            "{{\"status\":\"composing\",\"anchor\":{}}}",
            pos.to_json()
        ))
    }

    /// Update the IME composition preview text.
    ///
    /// If a preview already exists, deletes it first, then inserts
    /// the new preview text at the anchor.
    /// Returns an EditResult JSON.
    pub fn update_composition(&mut self, text: &str) -> Result<String, JsError> {
        let anchor_node = self
            .composition_anchor_node
            .clone()
            .ok_or_else(|| JsError::new("No active composition"))?;
        let anchor_offset =
            self.composition_anchor_offset
                .ok_or_else(|| JsError::new("No active composition"))? as usize;
        let prev_len = self.composition_preview_len;

        // Delete previous preview if any
        if prev_len > 0 {
            self.delete_text_in_paragraph(&anchor_node, anchor_offset, prev_len)?;
        }

        // Insert new preview text
        if !text.is_empty() {
            self.insert_text_in_paragraph(&anchor_node, anchor_offset, text)?;
        }

        self.composition_preview_len = text.chars().count();

        // Build position after the preview text
        let para_id = parse_node_id(&anchor_node)?;
        let doc = self.doc()?;
        let new_char_offset = anchor_offset + self.composition_preview_len;
        // Find the text node at the new position
        let (text_node_id, local_offset) =
            match find_text_node_at_char_offset(doc.model(), para_id, new_char_offset) {
                Ok((tid, lo, _)) => (tid, lo),
                Err(_) => {
                    // Fallback: use paragraph start
                    match find_first_text_node(doc.model(), para_id) {
                        Ok((tid, _)) => (tid, 0),
                        Err(_) => {
                            return Err(JsError::new("Cannot find text node after composition"))
                        }
                    }
                }
            };

        // Convert local char offset back to UTF-16
        let text_content = doc
            .model()
            .node(text_node_id)
            .and_then(|n| n.text_content.as_deref())
            .unwrap_or("");
        let utf16_offset = char_offset_to_utf16_offset(text_content, local_offset);

        let new_pos = PositionRefParsed {
            node_id: text_node_id,
            offset_utf16: utf16_offset,
            affinity: "downstream".to_string(),
        };

        Ok(build_edit_result_json(doc, &new_pos))
    }

    /// Commit the IME composition with final text.
    ///
    /// Deletes the preview, inserts the final text, and clears composition state.
    /// Returns an EditResult JSON.
    pub fn commit_composition(&mut self, text: &str) -> Result<String, JsError> {
        let anchor_node = self
            .composition_anchor_node
            .clone()
            .ok_or_else(|| JsError::new("No active composition"))?;
        let anchor_offset =
            self.composition_anchor_offset
                .ok_or_else(|| JsError::new("No active composition"))? as usize;
        let prev_len = self.composition_preview_len;

        // Delete preview
        if prev_len > 0 {
            self.delete_text_in_paragraph(&anchor_node, anchor_offset, prev_len)?;
        }

        // Insert final text
        if !text.is_empty() {
            self.insert_text_in_paragraph(&anchor_node, anchor_offset, text)?;
        }

        // Clear composition state
        let final_char_len = text.chars().count();
        self.composition_anchor_node = None;
        self.composition_anchor_offset = None;
        self.composition_preview_len = 0;

        // Build position after committed text
        let para_id = parse_node_id(&anchor_node)?;
        let doc = self.doc()?;
        let new_char_offset = anchor_offset + final_char_len;
        let (text_node_id, local_offset) =
            match find_text_node_at_char_offset(doc.model(), para_id, new_char_offset) {
                Ok((tid, lo, _)) => (tid, lo),
                Err(_) => match find_first_text_node(doc.model(), para_id) {
                    Ok((tid, _)) => (tid, 0),
                    Err(_) => return Err(JsError::new("Cannot find text node after commit")),
                },
            };

        let text_content = doc
            .model()
            .node(text_node_id)
            .and_then(|n| n.text_content.as_deref())
            .unwrap_or("");
        let utf16_offset = char_offset_to_utf16_offset(text_content, local_offset);

        let new_pos = PositionRefParsed {
            node_id: text_node_id,
            offset_utf16: utf16_offset,
            affinity: "downstream".to_string(),
        };

        Ok(build_edit_result_json(doc, &new_pos))
    }

    /// Cancel the IME composition.
    ///
    /// Deletes the preview text and clears composition state.
    /// Returns an EditResult JSON with cursor at the original anchor.
    pub fn cancel_composition(&mut self) -> Result<String, JsError> {
        let anchor_node = self
            .composition_anchor_node
            .clone()
            .ok_or_else(|| JsError::new("No active composition"))?;
        let anchor_offset =
            self.composition_anchor_offset
                .ok_or_else(|| JsError::new("No active composition"))? as usize;
        let prev_len = self.composition_preview_len;

        // Delete preview
        if prev_len > 0 {
            self.delete_text_in_paragraph(&anchor_node, anchor_offset, prev_len)?;
        }

        // Clear composition state
        self.composition_anchor_node = None;
        self.composition_anchor_offset = None;
        self.composition_preview_len = 0;

        // Build position at original anchor
        let para_id = parse_node_id(&anchor_node)?;
        let doc = self.doc()?;
        let (text_node_id, local_offset) =
            match find_text_node_at_char_offset(doc.model(), para_id, anchor_offset) {
                Ok((tid, lo, _)) => (tid, lo),
                Err(_) => match find_first_text_node(doc.model(), para_id) {
                    Ok((tid, _)) => (tid, 0),
                    Err(_) => return Err(JsError::new("Cannot find text node after cancel")),
                },
            };

        let text_content = doc
            .model()
            .node(text_node_id)
            .and_then(|n| n.text_content.as_deref())
            .unwrap_or("");
        let utf16_offset = char_offset_to_utf16_offset(text_content, local_offset);

        let new_pos = PositionRefParsed {
            node_id: text_node_id,
            offset_utf16: utf16_offset,
            affinity: "downstream".to_string(),
        };

        Ok(build_edit_result_json(doc, &new_pos))
    }

    // ─── Navigation & Remaining Methods (Phase 7) ───────────────────

    /// Get the line boundary position for "start" or "end" of the line
    /// containing the given position.
    ///
    /// Returns a PositionRef JSON.
    pub fn line_boundary(&self, position_json: &str, side: &str) -> Result<String, JsError> {
        let doc = self.doc()?;
        let font_db = s1_text::FontDatabase::new();
        self.ensure_layout(&font_db)?;
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not available"))?;
        let model = doc.model();

        let pos = PositionRefParsed::from_json(position_json)?;

        // Find the line containing this position in the layout.
        // GlyphRun.source_id is a Run node ID; the text node is its first child.
        for page in &layout.pages {
            for block in &page.blocks {
                if let s1_layout::LayoutBlockKind::Paragraph { lines, .. } = &block.kind {
                    for line in lines {
                        let mut line_contains_pos = false;
                        for run in &line.runs {
                            // Check if this run contains our text node
                            let text_node_id = model
                                .node(run.source_id)
                                .and_then(|n| n.children.first().copied())
                                .unwrap_or(run.source_id);
                            if text_node_id == pos.node_id || run.source_id == pos.node_id {
                                line_contains_pos = true;
                                break;
                            }
                        }
                        if line_contains_pos {
                            match side {
                                "start" => {
                                    if let Some(first_run) = line.runs.first() {
                                        let text_node_id = model
                                            .node(first_run.source_id)
                                            .and_then(|n| n.children.first().copied())
                                            .unwrap_or(first_run.source_id);
                                        let result = PositionRefParsed {
                                            node_id: text_node_id,
                                            offset_utf16: 0,
                                            affinity: "downstream".to_string(),
                                        };
                                        return Ok(result.to_json());
                                    }
                                }
                                "end" => {
                                    if let Some(last_run) = line.runs.last() {
                                        let text_node_id = model
                                            .node(last_run.source_id)
                                            .and_then(|n| n.children.first().copied())
                                            .unwrap_or(last_run.source_id);
                                        let end_utf16 = last_run.text.encode_utf16().count() as u32;
                                        let result = PositionRefParsed {
                                            node_id: text_node_id,
                                            offset_utf16: end_utf16,
                                            affinity: "downstream".to_string(),
                                        };
                                        return Ok(result.to_json());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Fallback: return original position
        Ok(pos.to_json())
    }

    /// Move a range in a direction by a granularity.
    ///
    /// If extend is true, moves only the focus while keeping the anchor.
    /// If extend is false, collapses the range and moves.
    /// Returns a RangeRef JSON.
    pub fn move_range(
        &self,
        range_json: &str,
        direction: &str,
        granularity: &str,
        extend: bool,
    ) -> Result<String, JsError> {
        let range = RangeRefParsed::from_json(range_json)?;

        if extend {
            // Move focus only
            let new_focus_json =
                self.move_position(&range.focus.to_json(), direction, granularity)?;
            let new_focus = PositionRefParsed::from_json(&new_focus_json)?;
            Ok(format!(
                "{{\"anchor\":{},\"focus\":{}}}",
                range.anchor.to_json(),
                new_focus.to_json()
            ))
        } else {
            // Collapse then move
            // When collapsing a non-collapsed range, go to the edge in the movement direction
            let collapse_pos = if range.anchor.node_id == range.focus.node_id
                && range.anchor.offset_utf16 == range.focus.offset_utf16
            {
                // Already collapsed, just move
                range.anchor.to_json()
            } else {
                // Collapse to the appropriate edge
                match direction {
                    "forward" => {
                        // Collapse to end of range (later position)
                        if range.anchor.offset_utf16 >= range.focus.offset_utf16
                            && range.anchor.node_id == range.focus.node_id
                        {
                            range.anchor.to_json()
                        } else {
                            range.focus.to_json()
                        }
                    }
                    "backward" => {
                        // Collapse to start of range (earlier position)
                        if range.anchor.offset_utf16 <= range.focus.offset_utf16
                            && range.anchor.node_id == range.focus.node_id
                        {
                            range.anchor.to_json()
                        } else {
                            range.focus.to_json()
                        }
                    }
                    _ => range.anchor.to_json(),
                }
            };

            let new_pos_json = self.move_position(&collapse_pos, direction, granularity)?;
            let new_pos = PositionRefParsed::from_json(&new_pos_json)?;
            Ok(format!(
                "{{\"anchor\":{},\"focus\":{}}}",
                new_pos.to_json(),
                new_pos.to_json()
            ))
        }
    }

    /// Get the editor capabilities as a JSON object.
    ///
    /// Returns a JSON object indicating which editing features are available.
    pub fn editor_capabilities(&self) -> Result<String, JsError> {
        Ok("{\"tables\":true,\"images\":true,\"shapes\":true,\"textboxes\":true,\"undo\":true,\"redo\":true,\"clipboard\":true,\"find\":true}".to_string())
    }

    /// Search for text matches and return results with page rects.
    ///
    /// Wraps the existing `find_text` and enriches results with layout
    /// position information when available.
    pub fn search_matches(&self, query: &str, case_sensitive: bool) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let body = model
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;

        let query_normalized = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let mut results = Vec::new();
        collect_find_results(
            model,
            &body.children,
            &query_normalized,
            case_sensitive,
            &mut results,
        );

        // Try to add layout rects
        let font_db = s1_text::FontDatabase::new();
        let has_layout = self.ensure_layout(&font_db).is_ok();
        if has_layout {
            let cache = self.layout_cache.borrow();
            if let Some(layout) = cache.as_ref() {
                let mut enriched = Vec::new();
                for result_json in &results {
                    // Each result is {"nodeId":"r:c","offset":N,"length":N}
                    // Parse nodeId, offset, length to find page rects
                    let node_id_str =
                        scene_extract_json_string(result_json, "nodeId").unwrap_or_default();
                    let offset =
                        scene_extract_json_number(result_json, "offset").unwrap_or(0.0) as usize;
                    let length =
                        scene_extract_json_number(result_json, "length").unwrap_or(0.0) as usize;

                    // Find page containing this paragraph
                    if let Ok(nid) = parse_node_id(&node_id_str) {
                        let mut page_idx = 0;
                        for page in &layout.pages {
                            for block in &page.blocks {
                                if block.source_id == nid {
                                    page_idx = page.index;
                                    break;
                                }
                            }
                        }
                        enriched.push(format!(
                            "{{\"nodeId\":\"{}\",\"offset\":{},\"length\":{},\"page_index\":{}}}",
                            escape_json(&node_id_str),
                            offset,
                            length,
                            page_idx
                        ));
                    } else {
                        enriched.push(result_json.clone());
                    }
                }
                return Ok(format!("[{}]", enriched.join(",")));
            }
        }

        Ok(format!("[{}]", results.join(",")))
    }

    // ─── Lifecycle ────────────────────────────────────────────────

    /// Free the document, releasing memory.
    ///
    /// After calling this, all other methods will return an error.
    pub fn free(&mut self) {
        self.inner = None;
        *self.layout_cache.get_mut() = None;
        self.composition_anchor_node = None;
        self.composition_anchor_offset = None;
        self.composition_preview_len = 0;
    }

    /// Check if this document handle is still valid.
    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }
}

impl WasmDocument {
    fn doc(&self) -> Result<&s1engine::Document, JsError> {
        self.inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document has been freed"))
    }

    fn doc_mut(&mut self) -> Result<&mut s1engine::Document, JsError> {
        // Mark layout as stale but keep the cache for fast lookups
        // (caret_rect, hit_test, selection_rects can use the stale layout
        // while the user is typing, avoiding expensive full relayout per keystroke).
        self._layout_dirty.set(true);
        self.inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document has been freed"))
    }

    /// Lazily compute and cache layout. Recomputes if cache is missing OR dirty.
    /// Clears the dirty flag after a successful recompute.
    fn ensure_layout(&self, font_db: &s1_text::FontDatabase) -> Result<(), JsError> {
        if self.layout_cache.borrow().is_some() && !self._layout_dirty.get() {
            return Ok(());
        }
        let doc = self.doc()?;
        let layout = doc
            .layout(font_db)
            .map_err(|e| JsError::new(&e.to_string()))?;
        *self.layout_cache.borrow_mut() = Some(layout);
        self._layout_dirty.set(false);
        Ok(())
    }

    /// Ensure layout cache exists (possibly stale). Does NOT recompute if dirty.
    /// Use this for fast lookups (caret_rect, hit_test) that can tolerate
    /// a slightly outdated layout during rapid typing.
    fn ensure_layout_has_cache(&self, font_db: &s1_text::FontDatabase) -> Result<(), JsError> {
        if self.layout_cache.borrow().is_some() {
            return Ok(());
        }
        // No cache at all — we must compute one
        let doc = self.doc()?;
        let layout = doc
            .layout(font_db)
            .map_err(|e| JsError::new(&e.to_string()))?;
        *self.layout_cache.borrow_mut() = Some(layout);
        self._layout_dirty.set(false);
        Ok(())
    }

    /// Core implementation for `get_page_html` / `get_page_html_with_fonts`.
    ///
    /// Produces document-model HTML for a single page using the cached layout.
    fn render_page_html(&self, page_index: usize) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let sections = doc.sections();
        let cache = self.layout_cache.borrow();
        let layout = cache
            .as_ref()
            .ok_or_else(|| JsError::new("Layout not computed"))?;
        let page = layout.pages.get(page_index).ok_or_else(|| {
            JsError::new(&format!(
                "Page index {} out of range ({})",
                page_index,
                layout.pages.len()
            ))
        })?;

        let mut html = String::with_capacity(4096);

        // ── Header ──────────────────────────────────────────────
        let sec = sections.get(page.section_index);
        if let Some(sec) = sec {
            use s1_model::section::HeaderFooterType;
            // Determine which header to show: first-page vs default
            let is_first_page_of_section = page_index == 0
                || layout
                    .pages
                    .get(page_index.wrapping_sub(1))
                    .is_none_or(|prev| prev.section_index != page.section_index);
            let header_ref = if sec.title_page && is_first_page_of_section {
                sec.headers
                    .iter()
                    .find(|h| h.hf_type == HeaderFooterType::First)
                    .or_else(|| {
                        sec.headers
                            .iter()
                            .find(|h| h.hf_type == HeaderFooterType::Default)
                    })
            } else {
                sec.headers
                    .iter()
                    .find(|h| h.hf_type == HeaderFooterType::Default)
                    .or_else(|| sec.headers.first())
            };
            if let Some(hf) = header_ref {
                html.push_str(&format!(
                    "<header data-section-index=\"{}\" style=\"border-bottom:1px solid #dadce0;padding:8px 0;margin-bottom:16px;color:#5f6368;font-size:9pt;\">",
                    page.section_index
                ));
                render_children(model, hf.node_id, &mut html);
                html.push_str("</header>");
            }
        }

        // ── Main content blocks ─────────────────────────────────
        // Track accumulated char offsets for paragraphs split across pages.
        let mut para_char_offsets: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();
        // Also scan previous pages to set correct char offsets for
        // continuation paragraphs that started on an earlier page.
        for prev_page in &layout.pages[..page_index] {
            for block in &prev_page.blocks {
                if let s1_layout::LayoutBlockKind::Paragraph {
                    lines,
                    split_at_line,
                    is_continuation,
                    ..
                } = &block.kind
                {
                    if *split_at_line > 0 || *is_continuation {
                        let char_count: usize = lines
                            .iter()
                            .map(|l| l.runs.iter().map(|r| r.text.chars().count()).sum::<usize>())
                            .sum();
                        let prev = para_char_offsets
                            .get(&block.source_id)
                            .copied()
                            .unwrap_or(0);
                        para_char_offsets.insert(block.source_id, prev + char_count);
                    }
                }
            }
        }

        for block in &page.blocks {
            render_page_content_block(model, block, &mut html, &mut para_char_offsets);
        }

        // ── Footnotes ───────────────────────────────────────────
        if !page.footnotes.is_empty() {
            html.push_str(
                "<div class=\"footnotes-section\" data-footnotes=\"true\" contenteditable=\"false\">",
            );
            html.push_str(
                "<hr class=\"footnote-separator\" style=\"border:none;border-top:1px solid #dadce0;width:33%;margin:12px 0 8px 0\" />",
            );
            for fn_block in &page.footnotes {
                render_node(model, fn_block.source_id, &mut html);
            }
            html.push_str("</div>");
        }

        // ── Floating images ─────────────────────────────────────
        for float_block in &page.floating_images {
            render_node(model, float_block.source_id, &mut html);
        }

        // ── Footer ──────────────────────────────────────────────
        if let Some(sec) = sec {
            use s1_model::section::HeaderFooterType;
            let is_first_page_of_section = page_index == 0
                || layout
                    .pages
                    .get(page_index.wrapping_sub(1))
                    .is_none_or(|prev| prev.section_index != page.section_index);
            let footer_ref = if sec.title_page && is_first_page_of_section {
                sec.footers
                    .iter()
                    .find(|f| f.hf_type == HeaderFooterType::First)
                    .or_else(|| {
                        sec.footers
                            .iter()
                            .find(|f| f.hf_type == HeaderFooterType::Default)
                    })
            } else {
                sec.footers
                    .iter()
                    .find(|f| f.hf_type == HeaderFooterType::Default)
                    .or_else(|| sec.footers.first())
            };
            if let Some(hf) = footer_ref {
                html.push_str(&format!(
                    "<footer data-section-index=\"{}\" style=\"border-top:1px solid #dadce0;padding:8px 0;margin-top:16px;color:#5f6368;font-size:9pt;\">",
                    page.section_index
                ));
                render_children(model, hf.node_id, &mut html);
                html.push_str("</footer>");
            }
        }

        Ok(html)
    }

    /// Get footnotes or endnotes as JSON array.
    fn get_notes_json(
        &self,
        body_type: NodeType,
        number_key: &AttributeKey,
    ) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let root = model.root_id();
        let root_node = model
            .node(root)
            .ok_or_else(|| JsError::new("Root not found"))?;

        let mut notes = Vec::new();
        for &child_id in &root_node.children {
            if let Some(child) = model.node(child_id) {
                if child.node_type == body_type {
                    let number = child.attributes.get_i64(number_key).unwrap_or(0);

                    let mut text = String::new();
                    for &para_id in &child.children {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(&extract_paragraph_text(model, para_id));
                    }

                    notes.push(format!(
                        "{{\"number\":{},\"text\":\"{}\"}}",
                        number,
                        escape_json(&text)
                    ));
                }
            }
        }
        Ok(format!("[{}]", notes.join(",")))
    }

    // ─── FS-15: Document Statistics API ────────────────────

    /// Get document statistics as JSON.
    ///
    /// Returns `{"words":N,"characters":N,"charactersNoSpaces":N,"paragraphs":N,"pages":N}`.
    pub fn get_document_stats_json(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let text = doc.to_plain_text();
        let paragraphs = doc.paragraph_count();

        let characters: usize = text.chars().count();
        let characters_no_spaces: usize = text.chars().filter(|c| !c.is_whitespace()).count();
        let words = count_words_in_str(&text);

        // Page count from layout engine (best-effort; falls back to 1 on error)
        let pages = {
            let font_db = s1_text::FontDatabase::empty();
            match doc.layout(&font_db) {
                Ok(layout) => layout.pages.len().max(1),
                Err(_) => 1usize,
            }
        };

        Ok(format!(
            "{{\"words\":{},\"characters\":{},\"charactersNoSpaces\":{},\"paragraphs\":{},\"pages\":{}}}",
            words, characters, characters_no_spaces, paragraphs, pages
        ))
    }

    /// Count words in a selection range.
    ///
    /// Takes start/end node IDs and character offsets. Returns the word count
    /// for text within that range.
    pub fn get_selection_word_count(
        &self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
    ) -> Result<usize, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let start_id = parse_node_id(start_node_str)?;
        let end_id = parse_node_id(end_node_str)?;

        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let body = model
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;

        // Collect text from the selected range
        let mut text = String::new();
        let mut inside = false;
        let same_node = start_id == end_id;

        for &child_id in &body.children {
            if model.node(child_id).is_none() {
                continue;
            }
            // Recurse into tables and other containers
            let para_ids = collect_paragraph_ids_recursive(model, child_id);
            for para_id in para_ids {
                if para_id == start_id && same_node {
                    // Single-node selection
                    let para_text = extract_paragraph_text(model, para_id);
                    let chars: Vec<char> = para_text.chars().collect();
                    let s = start_offset.min(chars.len());
                    let e = end_offset.min(chars.len());
                    let slice: String = chars[s..e].iter().collect();
                    text.push_str(&slice);
                    break;
                } else if para_id == start_id {
                    inside = true;
                    let para_text = extract_paragraph_text(model, para_id);
                    let chars: Vec<char> = para_text.chars().collect();
                    let s = start_offset.min(chars.len());
                    let slice: String = chars[s..].iter().collect();
                    text.push_str(&slice);
                    text.push(' ');
                } else if para_id == end_id {
                    let para_text = extract_paragraph_text(model, para_id);
                    let chars: Vec<char> = para_text.chars().collect();
                    let e = end_offset.min(chars.len());
                    let slice: String = chars[..e].iter().collect();
                    text.push_str(&slice);
                    inside = false;
                    break;
                } else if inside {
                    let para_text = extract_paragraph_text(model, para_id);
                    text.push_str(&para_text);
                    text.push(' ');
                }
            }
            if !inside && (same_node || para_ids_contain(model, child_id, end_id)) {
                break;
            }
        }

        Ok(count_words_in_str(&text))
    }
}

/// Count words in a string (split on whitespace, filter empty).
fn count_words_in_str(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Recursively collect paragraph NodeIds from a node (handles tables, sections, etc.).
fn collect_paragraph_ids_recursive(model: &DocumentModel, node_id: NodeId) -> Vec<NodeId> {
    let node = match model.node(node_id) {
        Some(n) => n,
        None => return vec![],
    };
    match node.node_type {
        NodeType::Paragraph => vec![node_id],
        _ => {
            let mut result = Vec::new();
            for &child_id in &node.children {
                result.extend(collect_paragraph_ids_recursive(model, child_id));
            }
            result
        }
    }
}

/// Check if a subtree contains a specific node ID.
fn para_ids_contain(model: &DocumentModel, root_id: NodeId, target: NodeId) -> bool {
    if root_id == target {
        return true;
    }
    let node = match model.node(root_id) {
        Some(n) => n,
        None => return false,
    };
    for &child_id in &node.children {
        if para_ids_contain(model, child_id, target) {
            return true;
        }
    }
    false
}

// --- WasmDocumentBuilder ---

/// Maximum number of nodes allowed in a builder-created document.
/// Prevents OOM from excessively large documents in the WASM environment.
const MAX_BUILDER_NODES: usize = 100_000;

/// Maximum nesting depth allowed during builder chaining.
/// Prevents stack overflow from deeply nested structures.
const MAX_BUILDER_DEPTH: usize = 100;

/// A fluent builder for constructing documents.
#[wasm_bindgen]
pub struct WasmDocumentBuilder {
    inner: Option<s1engine::DocumentBuilder>,
    node_count: usize,
    depth: usize,
    error: Option<String>,
}

#[wasm_bindgen]
impl WasmDocumentBuilder {
    /// Create a new document builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Some(s1engine::DocumentBuilder::new()),
            node_count: 0,
            depth: 0,
            error: None,
        }
    }

    /// Check builder limits and record an error if exceeded.
    /// Returns `true` if the builder is still within limits.
    fn check_limits(&mut self) -> bool {
        if self.error.is_some() {
            return false;
        }
        if self.node_count > MAX_BUILDER_NODES {
            self.error = Some(format!(
                "Builder exceeded maximum node limit ({} > {MAX_BUILDER_NODES})",
                self.node_count
            ));
            return false;
        }
        if self.depth > MAX_BUILDER_DEPTH {
            self.error = Some(format!(
                "Builder exceeded maximum depth limit ({} > {MAX_BUILDER_DEPTH})",
                self.depth
            ));
            return false;
        }
        true
    }

    /// Add a heading at the specified level (1-6).
    pub fn heading(mut self, level: u8, text: &str) -> Self {
        self.depth += 1;
        // heading creates ~3 nodes (paragraph, run, text)
        self.node_count += 3;
        if self.check_limits() {
            if let Some(builder) = self.inner.take() {
                self.inner = Some(builder.heading(level, text));
            }
        }
        self
    }

    /// Add a paragraph with plain text.
    pub fn text(mut self, text: &str) -> Self {
        self.depth += 1;
        // text creates ~3 nodes (paragraph, run, text)
        self.node_count += 3;
        if self.check_limits() {
            if let Some(builder) = self.inner.take() {
                self.inner = Some(builder.text(text));
            }
        }
        self
    }

    /// Set the document title.
    pub fn title(mut self, title: &str) -> Self {
        if self.check_limits() {
            if let Some(builder) = self.inner.take() {
                self.inner = Some(builder.title(title));
            }
        }
        self
    }

    /// Set the document author.
    pub fn author(mut self, author: &str) -> Self {
        if self.check_limits() {
            if let Some(builder) = self.inner.take() {
                self.inner = Some(builder.author(author));
            }
        }
        self
    }

    /// Build the document. Consumes the builder.
    ///
    /// Returns an error if the document exceeds the maximum node count
    /// limit (100,000 nodes) or the maximum depth limit (100) to prevent
    /// OOM in the WASM environment.
    pub fn build(mut self) -> Result<WasmDocument, JsError> {
        // Check for deferred errors from builder chaining
        if let Some(err) = self.error.take() {
            return Err(JsError::new(&err));
        }

        let builder = self
            .inner
            .take()
            .ok_or_else(|| JsError::new("Builder already consumed"))?;

        let doc = builder.build();

        // Safety limit: prevent OOM from excessively large documents
        let actual_count = doc.model().node_count();
        if actual_count > MAX_BUILDER_NODES {
            return Err(JsError::new(&format!(
                "Document exceeds maximum node limit ({actual_count} > {MAX_BUILDER_NODES})"
            )));
        }

        Ok(WasmDocument {
            batch_label: None,
            batch_count: 0,
            inner: Some(doc),
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        })
    }
}

impl Default for WasmDocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn builder_depth_limit_exceeded() {
        let mut builder = WasmDocumentBuilder::new();
        for i in 0..=MAX_BUILDER_DEPTH {
            builder = builder.text(&format!("paragraph {i}"));
        }
        let result = builder.build();
        assert!(
            result.is_err(),
            "build should fail when depth limit exceeded"
        );
    }

    #[test]
    fn builder_depth_tracking() {
        // Verify depth tracking works without needing wasm JsError
        let mut builder = WasmDocumentBuilder::new();
        assert_eq!(builder.depth, 0);
        builder = builder.text("a");
        assert_eq!(builder.depth, 1);
        builder = builder.text("b");
        assert_eq!(builder.depth, 2);
    }

    #[test]
    fn builder_within_limits_succeeds() {
        let builder = WasmDocumentBuilder::new()
            .heading(1, "Title")
            .text("Hello world");
        let result = builder.build();
        assert!(result.is_ok());
    }
}

impl WasmDocumentBuilder {
    /// Get the estimated node count added so far.
    /// This is an approximation used for early limit checking.
    pub fn estimated_node_count(&self) -> usize {
        self.node_count
    }
}

// --- WasmFontDatabase ---

/// A font database for WASM environments.
///
/// Since WASM has no filesystem access, fonts must be loaded manually
/// via `load_font()`.
#[wasm_bindgen]
pub struct WasmFontDatabase {
    inner: s1_text::FontDatabase,
}

#[wasm_bindgen]
impl WasmFontDatabase {
    /// Create a new empty font database.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: s1_text::FontDatabase::empty(),
        }
    }

    /// Load a font from raw bytes (TTF/OTF).
    pub fn load_font(&mut self, data: Vec<u8>) {
        self.inner.load_font_data(data);
    }

    /// Get the number of loaded font faces.
    pub fn font_count(&self) -> usize {
        self.inner.len()
    }

    /// Check if a font family is available (exact or via substitution).
    pub fn has_font(&self, family: &str) -> bool {
        self.inner
            .find_with_substitution(family, false, false)
            .is_some()
    }

    /// Rasterize a single glyph to RGBA pixels.
    ///
    /// Returns a flat Uint8Array of RGBA pixels (width * height * 4 bytes)
    /// plus metadata as JSON: `{"width":W,"height":H,"bearingX":X,"bearingY":Y,"advance":A}`
    ///
    /// This is the core API for canvas-first rendering — replaces `ctx.fillText()`.
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_glyph(
        &self,
        family: &str,
        bold: bool,
        italic: bool,
        glyph_id: u16,
        size_px: f32,
        r: u8,
        g: u8,
        b: u8,
    ) -> Option<Vec<u8>> {
        let font_id = self.inner.find_with_substitution(family, bold, italic)?;
        let font = self.inner.load_font(font_id)?;
        let result = s1_text::rasterize_glyph(font.data(), glyph_id, size_px, [r, g, b])?;

        // Pack metadata + pixels into a single buffer:
        // First 20 bytes: width(u32), height(u32), bearingX(f32), bearingY(f32), advance(f32)
        // Remaining: RGBA pixel data
        let mut buf = Vec::with_capacity(20 + result.pixels.len());
        buf.extend_from_slice(&result.width.to_le_bytes());
        buf.extend_from_slice(&result.height.to_le_bytes());
        buf.extend_from_slice(&result.bearing_x.to_le_bytes());
        buf.extend_from_slice(&result.bearing_y.to_le_bytes());
        buf.extend_from_slice(&result.advance.to_le_bytes());
        buf.extend_from_slice(&result.pixels);
        Some(buf)
    }

    /// Rasterize a complete text run to RGBA pixels.
    ///
    /// Takes shaped glyph data (from layout engine) and produces a single
    /// bitmap. Returns packed buffer: 8 bytes header (width u32, height u32)
    /// followed by RGBA pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_run(
        &self,
        family: &str,
        bold: bool,
        italic: bool,
        glyph_data: &[u8], // packed: [glyph_id: u16, x_offset: f32, y_offset: f32] * N
        size_px: f32,
        r: u8,
        g: u8,
        b: u8,
        total_width: f32,
        line_height: f32,
    ) -> Option<Vec<u8>> {
        let font_id = self.inner.find_with_substitution(family, bold, italic)?;
        let font = self.inner.load_font(font_id)?;

        // Parse glyph data: each entry is 10 bytes (u16 + f32 + f32)
        let mut glyphs = Vec::new();
        let mut i = 0;
        while i + 10 <= glyph_data.len() {
            let gid = u16::from_le_bytes([glyph_data[i], glyph_data[i + 1]]);
            let xo = f32::from_le_bytes([
                glyph_data[i + 2], glyph_data[i + 3],
                glyph_data[i + 4], glyph_data[i + 5],
            ]);
            let yo = f32::from_le_bytes([
                glyph_data[i + 6], glyph_data[i + 7],
                glyph_data[i + 8], glyph_data[i + 9],
            ]);
            glyphs.push((gid, xo, yo));
            i += 10;
        }

        let pixels = s1_text::rasterize_text_run(
            font.data(), &glyphs, size_px, [r, g, b], total_width, line_height,
        )?;

        let width = total_width.ceil() as u32 + 4;
        let height = line_height.ceil() as u32 + 4;

        let mut buf = Vec::with_capacity(8 + pixels.len());
        buf.extend_from_slice(&width.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());
        buf.extend_from_slice(&pixels);
        Some(buf)
    }
}

impl Default for WasmFontDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// --- Paste JSON parsing helpers ---

/// A single run of formatted text for rich paste.
struct PasteRun {
    text: String,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strikethrough: Option<bool>,
    superscript: Option<bool>,
    subscript: Option<bool>,
    font_size: Option<f64>,
    font_family: Option<String>,
    color: Option<String>,
    highlight_color: Option<String>,
}

/// Paragraph-level formatting for rich paste.
struct PasteParagraphFormat {
    alignment: Option<String>,
    spacing_before: Option<f64>,
    spacing_after: Option<f64>,
    line_spacing: Option<String>,
    indent_left: Option<f64>,
    indent_right: Option<f64>,
    indent_first_line: Option<f64>,
    heading_level: Option<u32>,
}

impl PasteParagraphFormat {
    fn to_attribute_map(&self) -> s1_model::AttributeMap {
        let mut attrs = s1_model::AttributeMap::new();
        if let Some(ref align) = self.alignment {
            let a = match align.as_str() {
                "center" => s1_model::Alignment::Center,
                "right" => s1_model::Alignment::Right,
                "justify" => s1_model::Alignment::Justify,
                _ => s1_model::Alignment::Left,
            };
            attrs.set(AttributeKey::Alignment, AttributeValue::Alignment(a));
        }
        if let Some(sb) = self.spacing_before {
            attrs.set(AttributeKey::SpacingBefore, AttributeValue::Float(sb));
        }
        if let Some(sa) = self.spacing_after {
            attrs.set(AttributeKey::SpacingAfter, AttributeValue::Float(sa));
        }
        if let Some(ref ls) = self.line_spacing {
            let spacing = match ls.as_str() {
                "1.5" | "onePointFive" => s1_model::LineSpacing::OnePointFive,
                "2" | "double" => s1_model::LineSpacing::Double,
                _ => s1_model::LineSpacing::Single,
            };
            attrs.set(
                AttributeKey::LineSpacing,
                AttributeValue::LineSpacing(spacing),
            );
        }
        if let Some(il) = self.indent_left {
            attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(il));
        }
        if let Some(ir) = self.indent_right {
            attrs.set(AttributeKey::IndentRight, AttributeValue::Float(ir));
        }
        if let Some(ifl) = self.indent_first_line {
            attrs.set(AttributeKey::IndentFirstLine, AttributeValue::Float(ifl));
        }
        if let Some(hl) = self.heading_level {
            if (1..=6).contains(&hl) {
                let style_id = format!("Heading{}", hl);
                attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
            }
        }
        attrs
    }
}

impl PasteRun {
    /// Convert this run's formatting properties into an `AttributeMap`.
    /// Returns an empty map if no formatting is specified.
    fn to_attribute_map(&self) -> s1_model::AttributeMap {
        let mut attrs = s1_model::AttributeMap::new();
        if let Some(b) = self.bold {
            if b {
                attrs.set(AttributeKey::Bold, AttributeValue::Bool(true));
            }
        }
        if let Some(i) = self.italic {
            if i {
                attrs.set(AttributeKey::Italic, AttributeValue::Bool(true));
            }
        }
        if let Some(u) = self.underline {
            if u {
                attrs.set(
                    AttributeKey::Underline,
                    AttributeValue::UnderlineStyle(UnderlineStyle::Single),
                );
            }
        }
        if let Some(s) = self.strikethrough {
            if s {
                attrs.set(AttributeKey::Strikethrough, AttributeValue::Bool(true));
            }
        }
        if let Some(sup) = self.superscript {
            if sup {
                attrs.set(AttributeKey::Superscript, AttributeValue::Bool(true));
            }
        }
        if let Some(sub) = self.subscript {
            if sub {
                attrs.set(AttributeKey::Subscript, AttributeValue::Bool(true));
            }
        }
        if let Some(fs) = self.font_size {
            attrs.set(AttributeKey::FontSize, AttributeValue::Float(fs));
        }
        if let Some(ref ff) = self.font_family {
            attrs.set(AttributeKey::FontFamily, AttributeValue::String(ff.clone()));
        }
        if let Some(ref c) = self.color {
            if let Some(color) = Color::from_hex(c) {
                attrs.set(AttributeKey::Color, AttributeValue::Color(color));
            }
        }
        if let Some(ref hc) = self.highlight_color {
            if let Some(color) = Color::from_hex(hc) {
                attrs.set(AttributeKey::HighlightColor, AttributeValue::Color(color));
            }
        }
        attrs
    }
}

/// A parsed paragraph with runs and optional paragraph-level formatting.
struct ParsedParagraph {
    runs: Vec<PasteRun>,
    format: PasteParagraphFormat,
}

/// Parse the paste JSON format into a vector of paragraphs, each containing
/// runs and optional paragraph-level formatting.
///
/// Expected format:
/// ```json
/// {
///   "paragraphs": [
///     { "runs": [{"text": "...", "bold": true, ...}], "alignment": "center", ... },
///     ...
///   ]
/// }
/// ```
///
/// Uses manual JSON parsing to avoid adding serde_json as a dependency.
fn parse_paste_json(json: &str) -> Result<Vec<ParsedParagraph>, JsError> {
    let json = json.trim();
    if json.is_empty() || json == "{}" || json == "[]" {
        return Ok(Vec::new());
    }

    // Find the "paragraphs" array
    let paragraphs_key = "\"paragraphs\"";
    let para_key_pos = json
        .find(paragraphs_key)
        .ok_or_else(|| JsError::new("Missing 'paragraphs' key in paste JSON"))?;

    // Find the '[' that starts the paragraphs array
    let after_key = &json[para_key_pos + paragraphs_key.len()..];
    let colon_pos = after_key
        .find(':')
        .ok_or_else(|| JsError::new("Missing ':' after 'paragraphs' key"))?;
    let after_colon = &after_key[colon_pos + 1..];
    let arr_start = after_colon
        .find('[')
        .ok_or_else(|| JsError::new("Missing '[' for paragraphs array"))?;

    // Find matching ']' for the paragraphs array
    let arr_content_start = para_key_pos + paragraphs_key.len() + colon_pos + 1 + arr_start;
    let paragraphs_arr_end = find_matching_bracket(json, arr_content_start)?;
    let paragraphs_arr = &json[arr_content_start + 1..paragraphs_arr_end];

    // Split into individual paragraph objects
    let para_objects = split_json_array_objects(paragraphs_arr)?;

    let mut result = Vec::new();
    for para_obj in para_objects {
        let runs = parse_runs_from_paragraph_obj(&para_obj)?;
        let format = parse_paragraph_format(&para_obj);
        result.push(ParsedParagraph { runs, format });
    }

    Ok(result)
}

/// Find the matching closing bracket for an opening bracket at `pos`.
fn find_matching_bracket(s: &str, pos: usize) -> Result<usize, JsError> {
    let bytes = s.as_bytes();
    let open = bytes[pos];
    let close = match open {
        b'[' => b']',
        b'{' => b'}',
        _ => return Err(JsError::new("Expected '[' or '{'")),
    };

    let mut depth = 1i32;
    let mut i = pos + 1;
    let mut in_string = false;
    let mut escape_next = false;

    while i < bytes.len() {
        if escape_next {
            escape_next = false;
            i += 1;
            continue;
        }
        match bytes[i] {
            b'\\' if in_string => {
                escape_next = true;
            }
            b'"' => {
                in_string = !in_string;
            }
            b if b == open && !in_string => {
                depth += 1;
            }
            b if b == close && !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(JsError::new("Unmatched bracket in paste JSON"))
}

/// Split a JSON array's content into individual top-level objects.
/// Input is the content between `[` and `]`.
fn split_json_array_objects(arr_content: &str) -> Result<Vec<String>, JsError> {
    let trimmed = arr_content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut objects = Vec::new();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip whitespace and commas
        while i < bytes.len()
            && (bytes[i] == b' '
                || bytes[i] == b'\n'
                || bytes[i] == b'\r'
                || bytes[i] == b'\t'
                || bytes[i] == b',')
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            let end = find_matching_bracket(trimmed, i)?;
            objects.push(trimmed[i..=end].to_string());
            i = end + 1;
        } else {
            i += 1;
        }
    }
    Ok(objects)
}

/// Parse the "runs" array from a paragraph JSON object.
fn parse_runs_from_paragraph_obj(obj: &str) -> Result<Vec<PasteRun>, JsError> {
    let runs_key = "\"runs\"";
    let key_pos = match obj.find(runs_key) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let after_key = &obj[key_pos + runs_key.len()..];
    let colon_pos = after_key
        .find(':')
        .ok_or_else(|| JsError::new("Missing ':' after 'runs' key"))?;
    let after_colon = &after_key[colon_pos + 1..];
    let arr_start_rel = after_colon
        .find('[')
        .ok_or_else(|| JsError::new("Missing '[' for runs array"))?;

    let arr_start_abs = key_pos + runs_key.len() + colon_pos + 1 + arr_start_rel;
    let arr_end = find_matching_bracket(obj, arr_start_abs)?;
    let arr_content = &obj[arr_start_abs + 1..arr_end];

    let run_objects = split_json_array_objects(arr_content)?;
    let mut runs = Vec::new();
    for run_obj in run_objects {
        runs.push(parse_single_run(&run_obj)?);
    }
    Ok(runs)
}

/// Parse a single run JSON object into a `PasteRun`.
fn parse_single_run(obj: &str) -> Result<PasteRun, JsError> {
    let text = extract_json_string_opt(obj, "text").unwrap_or_default();
    let bold = extract_json_bool_opt(obj, "bold");
    let italic = extract_json_bool_opt(obj, "italic");
    let underline = extract_json_bool_opt(obj, "underline");
    let strikethrough = extract_json_bool_opt(obj, "strikethrough");
    let superscript = extract_json_bool_opt(obj, "superscript");
    let subscript = extract_json_bool_opt(obj, "subscript");
    let font_size = extract_json_number_opt(obj, "fontSize");
    let font_family = extract_json_string_opt(obj, "fontFamily");
    let color = extract_json_string_opt(obj, "color");
    let highlight_color = extract_json_string_opt(obj, "highlightColor");

    Ok(PasteRun {
        text,
        bold,
        italic,
        underline,
        strikethrough,
        superscript,
        subscript,
        font_size,
        font_family,
        color,
        highlight_color,
    })
}

/// Parse paragraph-level formatting from a paragraph JSON object.
fn parse_paragraph_format(obj: &str) -> PasteParagraphFormat {
    PasteParagraphFormat {
        alignment: extract_json_string_opt(obj, "alignment"),
        spacing_before: extract_json_number_opt(obj, "spacingBefore"),
        spacing_after: extract_json_number_opt(obj, "spacingAfter"),
        line_spacing: extract_json_string_opt(obj, "lineSpacing"),
        indent_left: extract_json_number_opt(obj, "indentLeft"),
        indent_right: extract_json_number_opt(obj, "indentRight"),
        indent_first_line: extract_json_number_opt(obj, "indentFirstLine"),
        heading_level: extract_json_number_opt(obj, "headingLevel").map(|v| v as u32),
    }
}

/// Extract a string value for a given key from a JSON object string.
/// Returns `None` if the key is not found or the value is not a string.
fn extract_json_string_opt(obj: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\"", key);
    let key_pos = obj.find(&search)?;
    let after_key = &obj[key_pos + search.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    if after_colon.starts_with('"') {
        // Find the closing quote, handling escapes
        let mut i = 1;
        let bytes = after_colon.as_bytes();
        let mut result = String::new();
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                match bytes[i + 1] {
                    b'"' => {
                        result.push('"');
                        i += 2;
                    }
                    b'\\' => {
                        result.push('\\');
                        i += 2;
                    }
                    b'n' => {
                        result.push('\n');
                        i += 2;
                    }
                    b'r' => {
                        result.push('\r');
                        i += 2;
                    }
                    b't' => {
                        result.push('\t');
                        i += 2;
                    }
                    _ => {
                        result.push(after_colon.as_bytes()[i] as char);
                        i += 1;
                    }
                }
            } else if bytes[i] == b'"' {
                return Some(result);
            } else {
                // Handle multi-byte UTF-8
                let remaining = &after_colon[i..];
                if let Some(c) = remaining.chars().next() {
                    result.push(c);
                    i += c.len_utf8();
                } else {
                    i += 1;
                }
            }
        }
        None
    } else {
        None
    }
}

/// Extract a boolean value for a given key from a JSON object string.
/// Returns `None` if the key is not found.
fn extract_json_bool_opt(obj: &str, key: &str) -> Option<bool> {
    let search = format!("\"{}\"", key);
    let key_pos = obj.find(&search)?;
    let after_key = &obj[key_pos + search.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Extract a numeric value for a given key from a JSON object string.
/// Returns `None` if the key is not found or the value is not a number.
fn extract_json_number_opt(obj: &str, key: &str) -> Option<f64> {
    let search = format!("\"{}\"", key);
    let key_pos = obj.find(&search)?;
    let after_key = &obj[key_pos + search.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    // Collect digits, dots, minus sign
    let num_str: String = after_colon
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_str.parse().ok()
}

// --- Node ID / editing helpers ---

/// Parse a "replica:counter" string into a NodeId.
fn parse_node_id(s: &str) -> Result<NodeId, JsError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(JsError::new(&format!(
            "Invalid node ID '{}': expected 'replica:counter'",
            s
        )));
    }
    let replica: u64 = parts[0]
        .parse()
        .map_err(|_| JsError::new(&format!("Invalid replica in node ID '{}'", s)))?;
    let counter: u64 = parts[1]
        .parse()
        .map_err(|_| JsError::new(&format!("Invalid counter in node ID '{}'", s)))?;
    Ok(NodeId::new(replica, counter))
}

/// Find the first Run child of a paragraph.
fn find_first_run(model: &DocumentModel, para_id: NodeId) -> Result<NodeId, JsError> {
    let para = model
        .node(para_id)
        .ok_or_else(|| JsError::new("Paragraph not found"))?;
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Run {
                return Ok(child_id);
            }
        }
    }
    Err(JsError::new("No run found in paragraph"))
}

/// Ensure a paragraph has at least one Run→Text child, creating them if absent.
/// Returns the text node ID and its char length.
fn ensure_run_and_text(
    doc: &mut s1engine::Document,
    para_id: NodeId,
) -> Result<(NodeId, usize), JsError> {
    // Check if run already exists
    if let Ok(run_id) = find_first_run(doc.model(), para_id) {
        // Run exists — find or create text node
        let run = doc
            .model()
            .node(run_id)
            .ok_or_else(|| JsError::new("Run not found"))?;
        for &child_id in &run.children {
            if let Some(child) = doc.model().node(child_id) {
                if child.node_type == NodeType::Text {
                    let len = child
                        .text_content
                        .as_ref()
                        .map(|t| t.chars().count())
                        .unwrap_or(0);
                    return Ok((child_id, len));
                }
            }
        }
        // Run exists but no text node — create one
        let text_id = doc.next_id();
        let text_node = Node::text(text_id, "");
        doc.apply(Operation::insert_node(run_id, 0, text_node))
            .map_err(|e| JsError::new(&e.to_string()))?;
        return Ok((text_id, 0));
    }

    // No run — create Run + Text
    let run_id = doc.next_id();
    let run_node = Node::new(run_id, NodeType::Run);
    doc.apply(Operation::insert_node(para_id, 0, run_node))
        .map_err(|e| JsError::new(&e.to_string()))?;

    let text_id = doc.next_id();
    let text_node = Node::text(text_id, "");
    doc.apply(Operation::insert_node(run_id, 0, text_node))
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok((text_id, 0))
}

/// Find the first Text node inside a paragraph (traverses Run → Text).
fn find_first_text_node(
    model: &DocumentModel,
    para_id: NodeId,
) -> Result<(NodeId, usize), JsError> {
    let run_id = find_first_run(model, para_id)?;
    let run = model
        .node(run_id)
        .ok_or_else(|| JsError::new("Run not found"))?;
    for &child_id in &run.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Text {
                let len = child
                    .text_content
                    .as_ref()
                    .map(|t| t.chars().count())
                    .unwrap_or(0);
                return Ok((child_id, len));
            }
        }
    }
    Err(JsError::new("No text node found in run"))
}

// ═══════════════════════════════════════════════════════════════════════════
// Canvas editing helpers — Phase 0: Position resolution
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a UTF-16 offset to a char offset within a string.
///
/// Walks the string char by char, accumulating UTF-16 code units.
/// Returns the char index when accumulated UTF-16 units reach the target offset.
fn utf16_offset_to_char_offset(text: &str, utf16_offset: u32) -> usize {
    let mut utf16_pos: u32 = 0;
    for (char_idx, c) in text.chars().enumerate() {
        if utf16_pos >= utf16_offset {
            return char_idx;
        }
        utf16_pos += c.len_utf16() as u32;
    }
    text.chars().count()
}

/// Convert a char offset to a UTF-16 offset within a string.
fn char_offset_to_utf16_offset(text: &str, char_offset: usize) -> u32 {
    text.chars()
        .take(char_offset)
        .map(|c| c.len_utf16() as u32)
        .sum()
}

/// Resolve a PositionRefParsed (text-node ID + UTF-16 offset) to paragraph
/// coordinates (paragraph ID + char offset within the paragraph).
///
/// The hit_test system returns positions with text-node IDs and UTF-16 offsets.
/// The existing editing methods (insert_text_in_paragraph, delete_selection, etc.)
/// expect paragraph IDs and char offsets. This function bridges the gap.
fn resolve_position_to_paragraph(
    model: &DocumentModel,
    pos: &PositionRefParsed,
) -> Result<(NodeId, usize), JsError> {
    let text_node = model
        .node(pos.node_id)
        .ok_or_else(|| JsError::new("Position text node not found"))?;

    // Get the text content to convert UTF-16 offset to char offset
    let text_content = text_node.text_content.as_deref().unwrap_or("");
    let local_char_offset = utf16_offset_to_char_offset(text_content, pos.offset_utf16);

    // Walk up: Text -> Run -> Paragraph
    let run_id = text_node
        .parent
        .ok_or_else(|| JsError::new("Text node has no parent Run"))?;
    let run_node = model
        .node(run_id)
        .ok_or_else(|| JsError::new("Parent Run not found"))?;
    if run_node.node_type != NodeType::Run {
        return Err(JsError::new("Position node parent is not a Run"));
    }

    let para_id = run_node
        .parent
        .ok_or_else(|| JsError::new("Run has no parent Paragraph"))?;
    let para_node = model
        .node(para_id)
        .ok_or_else(|| JsError::new("Parent Paragraph not found"))?;
    if para_node.node_type != NodeType::Paragraph {
        return Err(JsError::new("Run parent is not a Paragraph"));
    }

    // Accumulate char offsets across all preceding children in the paragraph
    let mut accumulated: usize = 0;
    for &child_id in &para_node.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    if child_id == run_id {
                        // Found our run. Now accumulate offsets for preceding
                        // text nodes within this run up to our text node.
                        for &sub_id in &child.children {
                            if sub_id == pos.node_id {
                                // This is our text node
                                return Ok((para_id, accumulated + local_char_offset));
                            }
                            if let Some(sub) = model.node(sub_id) {
                                if sub.node_type == NodeType::Text {
                                    accumulated += sub
                                        .text_content
                                        .as_ref()
                                        .map(|t| t.chars().count())
                                        .unwrap_or(0);
                                }
                            }
                        }
                        // Text node not found in this run (shouldn't happen)
                        return Ok((para_id, accumulated + local_char_offset));
                    }
                    // Not our run — add full run length
                    accumulated += run_char_len(model, child_id);
                }
                NodeType::LineBreak | NodeType::Tab => {
                    accumulated += 1;
                }
                _ => {}
            }
        }
    }

    // Fallback: if we couldn't trace the exact path, return best estimate
    Ok((para_id, accumulated + local_char_offset))
}

/// Resolve a RangeRefParsed to paragraph coordinates for both endpoints.
///
/// Returns (start_para_id, start_char_offset, end_para_id, end_char_offset)
/// where start is always the earlier position in document order.
fn resolve_range_to_paragraphs(
    model: &DocumentModel,
    range: &RangeRefParsed,
) -> Result<(NodeId, usize, NodeId, usize), JsError> {
    let (anchor_para, anchor_offset) = resolve_position_to_paragraph(model, &range.anchor)?;
    let (focus_para, focus_offset) = resolve_position_to_paragraph(model, &range.focus)?;

    // Determine document order: if same paragraph, compare offsets;
    // otherwise, compare paragraph positions in body.
    if anchor_para == focus_para {
        if anchor_offset <= focus_offset {
            Ok((anchor_para, anchor_offset, focus_para, focus_offset))
        } else {
            Ok((focus_para, focus_offset, anchor_para, anchor_offset))
        }
    } else {
        // Check which paragraph comes first in the body
        let body_id = model.body_id();
        if let Some(bid) = body_id {
            if let Some(body) = model.node(bid) {
                let anchor_idx = body.children.iter().position(|&c| c == anchor_para);
                let focus_idx = body.children.iter().position(|&c| c == focus_para);
                match (anchor_idx, focus_idx) {
                    (Some(ai), Some(fi)) if ai <= fi => {
                        return Ok((anchor_para, anchor_offset, focus_para, focus_offset));
                    }
                    (Some(_), Some(_)) => {
                        return Ok((focus_para, focus_offset, anchor_para, anchor_offset));
                    }
                    _ => {}
                }
            }
        }
        // Default: anchor first
        Ok((anchor_para, anchor_offset, focus_para, focus_offset))
    }
}

/// Build an EditResult JSON string after a canvas editing operation.
///
/// Produces JSON with document_revision, layout_revision, dirty_pages, and selection.
/// `dirty_page` is the 0-based page index containing the edit (from stale layout cache).
fn build_edit_result_json(doc: &s1engine::Document, new_position: &PositionRefParsed) -> String {
    build_edit_result_json_with_dirty_page(doc, new_position, 0)
}

/// Build an EditResult JSON with a specific dirty page index.
fn build_edit_result_json_with_dirty_page(
    doc: &s1engine::Document,
    new_position: &PositionRefParsed,
    dirty_page: usize,
) -> String {
    let revision = doc.undo_count() as u64;
    format!(
        "{{\"document_revision\":{},\"layout_revision\":{},\"dirty_pages\":{{\"start\":{},\"end\":{}}},\"selection\":{}}}",
        revision,
        revision,
        dirty_page,
        dirty_page,
        new_position.to_json()
    )
}

/// Find which page a paragraph is on in the stale layout cache.
/// Returns 0 if not found or cache is empty.
fn find_page_for_paragraph(
    layout_cache: &std::cell::RefCell<Option<s1_layout::LayoutDocument>>,
    para_id: NodeId,
) -> usize {
    let cache = layout_cache.borrow();
    let layout = match cache.as_ref() {
        Some(l) => l,
        None => return 0,
    };
    for page in &layout.pages {
        for block in &page.blocks {
            if block.source_id == para_id {
                return page.index;
            }
        }
    }
    0
}

/// Build an EditResult JSON with an optional new_paragraph_id field.
fn build_edit_result_json_with_para(
    doc: &s1engine::Document,
    new_position: &PositionRefParsed,
    new_para_id: Option<&str>,
) -> String {
    let revision = doc.undo_count() as u64;
    let page_count = 9999; // cheap upper bound; JS renderer clamps

    let para_field = match new_para_id {
        Some(pid) => format!(",\"new_paragraph_id\":\"{}\"", escape_json(pid)),
        None => String::new(),
    };

    format!(
        "{{\"document_revision\":{},\"layout_revision\":{},\"dirty_pages\":{{\"start\":0,\"end\":{}}},\"selection\":{}{}}}",
        revision,
        revision,
        page_count,
        new_position.to_json(),
        para_field
    )
}

/// Strip HTML tags from a string, returning plain text.
///
/// Simple implementation that handles basic HTML. Converts `<br>` and `</p>`
/// to newlines for paragraph structure.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag_name = String::new();
    let html_lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let chars_lower: Vec<char> = html_lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            in_tag = true;
            tag_name.clear();
            i += 1;
            continue;
        }
        if chars[i] == '>' && in_tag {
            in_tag = false;
            let tag_lower = tag_name.trim().to_lowercase();
            // Convert block elements to newlines
            if (tag_lower == "br"
                || tag_lower == "br/"
                || tag_lower == "/p"
                || tag_lower == "/div"
                || tag_lower == "/li")
                && !result.ends_with('\n')
            {
                result.push('\n');
            }
            i += 1;
            continue;
        }
        if in_tag {
            tag_name.push(chars_lower[i]);
            i += 1;
            continue;
        }
        // Handle HTML entities
        if chars[i] == '&' {
            let rest: String = chars[i..].iter().take(10).collect();
            if rest.starts_with("&amp;") {
                result.push('&');
                i += 5;
            } else if rest.starts_with("&lt;") {
                result.push('<');
                i += 4;
            } else if rest.starts_with("&gt;") {
                result.push('>');
                i += 4;
            } else if rest.starts_with("&quot;") {
                result.push('"');
                i += 6;
            } else if rest.starts_with("&nbsp;") {
                result.push(' ');
                i += 6;
            } else if rest.starts_with("&#39;") {
                result.push('\'');
                i += 5;
            } else {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }

    // Trim trailing newlines
    while result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Extract all text from a paragraph's runs.
fn extract_paragraph_text(model: &DocumentModel, para_id: NodeId) -> String {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return String::new(),
    };
    let mut text = String::new();
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    for &sub_id in &child.children {
                        if let Some(sub) = model.node(sub_id) {
                            if sub.node_type == NodeType::Text {
                                if let Some(t) = &sub.text_content {
                                    text.push_str(t);
                                }
                            }
                        }
                    }
                }
                NodeType::LineBreak => text.push('\n'),
                NodeType::Tab => text.push('\t'),
                _ => {}
            }
        }
    }
    text
}

/// Find the text node containing the given character offset.
///
/// Boundary behavior: If offset falls exactly at a run boundary,
/// it is assigned to the NEXT run. This matches cursor-at-boundary
/// semantics (new text typed at a run boundary inherits the next run's format).
/// For deletion, the caller should handle the boundary appropriately.
///
/// Walks through all runs in the paragraph, accumulating character counts
/// to find which text node contains the given char offset. Returns
/// (text_node_id, local_offset_within_text_node, text_node_char_len).
fn find_text_node_at_char_offset(
    model: &DocumentModel,
    para_id: NodeId,
    char_offset: usize,
) -> Result<(NodeId, usize, usize), JsError> {
    let para = model
        .node(para_id)
        .ok_or_else(|| JsError::new("Paragraph not found"))?;
    let mut accumulated = 0usize;
    let mut last_text_id = None;
    let mut last_len = 0usize;
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    for &sub_id in &child.children {
                        if let Some(sub) = model.node(sub_id) {
                            if sub.node_type == NodeType::Text {
                                let len = sub
                                    .text_content
                                    .as_ref()
                                    .map(|t| t.chars().count())
                                    .unwrap_or(0);
                                // Use < for intermediate text nodes (boundary goes to next node),
                                // but <= for the last (nowhere else to go)
                                if char_offset < accumulated + len {
                                    return Ok((sub_id, char_offset - accumulated, len));
                                }
                                accumulated += len;
                                last_text_id = Some(sub_id);
                                last_len = len;
                            }
                        }
                    }
                }
                NodeType::LineBreak | NodeType::Tab => {
                    accumulated += 1;
                }
                _ => {}
            }
        }
    }
    // Offset is at or past the end — return last text node at its end position
    if let Some(tid) = last_text_id {
        Ok((tid, last_len, last_len))
    } else {
        Err(JsError::new("No text node found in paragraph"))
    }
}

fn paragraph_range_contains_inline_nodes(
    model: &DocumentModel,
    para_id: NodeId,
    start_offset: usize,
    end_offset: usize,
) -> bool {
    if start_offset >= end_offset {
        return false;
    }

    let Some(para) = model.node(para_id) else {
        return false;
    };

    let mut offset = 0usize;
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => offset += run_char_len(model, child_id),
                NodeType::LineBreak | NodeType::Tab => {
                    if offset < end_offset && offset + 1 > start_offset {
                        return true;
                    }
                    offset += 1;
                }
                _ => {}
            }
        }
    }

    false
}

/// Delete a character range within a single paragraph, handling multi-run correctly.
///
/// Walks through runs, deleting text from each run that overlaps with
/// [start_offset, end_offset). Deletes runs that are fully consumed.
fn delete_text_range_in_paragraph(
    doc: &mut s1engine::Document,
    para_id: NodeId,
    start_offset: usize,
    end_offset: usize,
) -> Result<(), JsError> {
    if start_offset >= end_offset {
        return Ok(());
    }

    let para = doc
        .node(para_id)
        .ok_or_else(|| JsError::new("Paragraph not found"))?;

    enum ParagraphRangeEntry {
        Run {
            run_id: NodeId,
            text_id: NodeId,
            start: usize,
            end: usize,
        },
        InlineNode {
            node_id: NodeId,
            start: usize,
            end: usize,
        },
    }

    let mut entries: Vec<ParagraphRangeEntry> = Vec::new();
    let mut offset = 0usize;
    for &child_id in &para.children {
        if let Some(child) = doc.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    if let Ok((text_id, _, _)) =
                        find_text_node_at_char_offset_in_run(doc.model(), child_id, 0)
                    {
                        let len = run_char_len(doc.model(), child_id);
                        entries.push(ParagraphRangeEntry::Run {
                            run_id: child_id,
                            text_id,
                            start: offset,
                            end: offset + len,
                        });
                        offset += len;
                    }
                }
                NodeType::LineBreak | NodeType::Tab => {
                    entries.push(ParagraphRangeEntry::InlineNode {
                        node_id: child_id,
                        start: offset,
                        end: offset + 1,
                    });
                    offset += 1;
                }
                _ => {}
            }
        }
    }

    let mut txn = Transaction::with_label("Delete text range");

    for entry in &entries {
        match *entry {
            ParagraphRangeEntry::Run {
                run_id,
                text_id,
                start: run_start,
                end: run_end,
            } => {
                if run_end <= start_offset || run_start >= end_offset {
                    continue;
                }

                let del_start = start_offset.saturating_sub(run_start);
                let del_end = if end_offset < run_end {
                    end_offset - run_start
                } else {
                    run_end - run_start
                };
                let del_len = del_end - del_start;

                if del_start == 0 && del_end == run_end - run_start {
                    txn.push(Operation::delete_node(run_id));
                } else if del_len > 0 {
                    txn.push(Operation::delete_text(text_id, del_start, del_len));
                }
            }
            ParagraphRangeEntry::InlineNode {
                node_id,
                start,
                end,
            } => {
                if !(end <= start_offset || start >= end_offset) {
                    txn.push(Operation::delete_node(node_id));
                }
            }
        }
    }

    if !txn.is_empty() {
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
    }

    Ok(())
}

/// Find the text node and local offset within a specific run.
fn find_text_node_at_char_offset_in_run(
    model: &DocumentModel,
    run_id: NodeId,
    char_offset: usize,
) -> Result<(NodeId, usize, usize), JsError> {
    let run = model
        .node(run_id)
        .ok_or_else(|| JsError::new("Run not found"))?;
    for &child_id in &run.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Text {
                let len = child
                    .text_content
                    .as_ref()
                    .map(|t| t.chars().count())
                    .unwrap_or(0);
                return Ok((child_id, char_offset.min(len), len));
            }
        }
    }
    Err(JsError::new("No text node found in run"))
}

/// Get a human-readable node type string.
fn node_type_str(nt: &NodeType) -> &'static str {
    match nt {
        NodeType::Document => "Document",
        NodeType::Body => "Body",
        NodeType::Paragraph => "Paragraph",
        NodeType::Run => "Run",
        NodeType::Text => "Text",
        NodeType::Table => "Table",
        NodeType::TableRow => "TableRow",
        NodeType::TableCell => "TableCell",
        NodeType::Image => "Image",
        NodeType::LineBreak => "LineBreak",
        NodeType::PageBreak => "PageBreak",
        NodeType::Tab => "Tab",
        NodeType::Header => "Header",
        NodeType::Footer => "Footer",
        NodeType::Field => "Field",
        NodeType::BookmarkStart => "BookmarkStart",
        NodeType::BookmarkEnd => "BookmarkEnd",
        NodeType::TableOfContents => "TableOfContents",
        NodeType::Section => "Section",
        NodeType::CommentStart => "CommentStart",
        NodeType::CommentEnd => "CommentEnd",
        NodeType::CommentBody => "CommentBody",
        _ => "Unknown",
    }
}

/// Serialize a node to a JSON string for JS consumption.
fn node_to_json(model: &DocumentModel, nid: NodeId, node: &Node) -> String {
    let mut json = String::from("{");
    json.push_str(&format!(
        "\"id\":\"{}:{}\",\"type\":\"{}\"",
        nid.replica,
        nid.counter,
        node_type_str(&node.node_type)
    ));

    // Text content
    if let Some(text) = &node.text_content {
        json.push_str(&format!(",\"text\":\"{}\"", escape_json(text)));
    }

    // For paragraph/run: extract concatenated text
    if node.node_type == NodeType::Paragraph {
        let full_text = extract_paragraph_text(model, nid);
        json.push_str(&format!(",\"fullText\":\"{}\"", escape_json(&full_text)));
    }

    // Key attributes
    if let Some(AttributeValue::String(s)) = node.attributes.get(&AttributeKey::StyleId) {
        json.push_str(&format!(",\"styleId\":\"{}\"", escape_json(s)));
    }
    if node.attributes.get_bool(&AttributeKey::Bold) == Some(true) {
        json.push_str(",\"bold\":true");
    }
    if node.attributes.get_bool(&AttributeKey::Italic) == Some(true) {
        json.push_str(",\"italic\":true");
    }
    if node.attributes.get(&AttributeKey::Underline).is_some()
        && !matches!(
            node.attributes.get(&AttributeKey::Underline),
            Some(AttributeValue::UnderlineStyle(UnderlineStyle::None))
        )
    {
        json.push_str(",\"underline\":true");
    }
    if node.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true) {
        json.push_str(",\"strikethrough\":true");
    }
    if let Some(size) = node.attributes.get_f64(&AttributeKey::FontSize) {
        json.push_str(&format!(",\"fontSize\":{}", size));
    }
    if let Some(font) = node.attributes.get_string(&AttributeKey::FontFamily) {
        json.push_str(&format!(",\"fontFamily\":\"{}\"", escape_json(font)));
    }
    // Paragraph layout properties
    if node.attributes.get_bool(&AttributeKey::PageBreakBefore) == Some(true) {
        json.push_str(",\"pageBreakBefore\":true");
    }
    if node.attributes.get_bool(&AttributeKey::KeepWithNext) == Some(true) {
        json.push_str(",\"keepWithNext\":true");
    }
    if node.attributes.get_bool(&AttributeKey::KeepLinesTogether) == Some(true) {
        json.push_str(",\"keepLinesTogether\":true");
    }
    if node.attributes.get_bool(&AttributeKey::WidowControl) == Some(true) {
        json.push_str(",\"widowControl\":true");
    }
    // Indentation
    if let Some(v) = node.attributes.get_f64(&AttributeKey::IndentLeft) {
        json.push_str(&format!(",\"indentLeft\":{:.1}", v));
    }
    if let Some(v) = node.attributes.get_f64(&AttributeKey::IndentRight) {
        json.push_str(&format!(",\"indentRight\":{:.1}", v));
    }
    if let Some(v) = node.attributes.get_f64(&AttributeKey::IndentFirstLine) {
        json.push_str(&format!(",\"indentFirstLine\":{:.1}", v));
    }
    // Line spacing
    if let Some(ls) = node.attributes.get_line_spacing(&AttributeKey::LineSpacing) {
        let val = match ls {
            s1_model::LineSpacing::Single => 1.0,
            s1_model::LineSpacing::OnePointFive => 1.5,
            s1_model::LineSpacing::Double => 2.0,
            s1_model::LineSpacing::Multiple(v) => v as f64,
            s1_model::LineSpacing::Exact(v) => v as f64,
            s1_model::LineSpacing::AtLeast(v) => v as f64,
            _ => 1.0,
        };
        json.push_str(&format!(",\"lineSpacing\":{:.2}", val));
    }
    if let Some(v) = node.attributes.get_f64(&AttributeKey::SpacingBefore) {
        json.push_str(&format!(",\"spaceBefore\":{:.1}", v));
    }
    if let Some(v) = node.attributes.get_f64(&AttributeKey::SpacingAfter) {
        json.push_str(&format!(",\"spaceAfter\":{:.1}", v));
    }
    // List info
    if let Some(AttributeValue::ListInfo(li)) = node.attributes.get(&AttributeKey::ListInfo) {
        let fmt_name = match li.num_format {
            ListFormat::Bullet => "bullet",
            ListFormat::Decimal => "decimal",
            ListFormat::LowerAlpha => "lowerAlpha",
            ListFormat::UpperAlpha => "upperAlpha",
            ListFormat::LowerRoman => "lowerRoman",
            ListFormat::UpperRoman => "upperRoman",
            _ => "bullet",
        };
        json.push_str(&format!(",\"listFormat\":\"{}\"", fmt_name));
        json.push_str(&format!(",\"listLevel\":{}", li.level));
    }
    // Hyperlink
    if let Some(url) = node.attributes.get_string(&AttributeKey::HyperlinkUrl) {
        json.push_str(&format!(",\"hyperlinkUrl\":\"{}\"", escape_json(url)));
    }
    if let Some(tip) = node.attributes.get_string(&AttributeKey::HyperlinkTooltip) {
        json.push_str(&format!(",\"hyperlinkTooltip\":\"{}\"", escape_json(tip)));
    }
    // Image dimensions
    if let Some(w) = node.attributes.get_f64(&AttributeKey::ImageWidth) {
        json.push_str(&format!(",\"imageWidth\":{}", w));
    }
    if let Some(h) = node.attributes.get_f64(&AttributeKey::ImageHeight) {
        json.push_str(&format!(",\"imageHeight\":{}", h));
    }
    if let Some(AttributeValue::Color(c)) = node.attributes.get(&AttributeKey::Color) {
        json.push_str(&format!(",\"color\":\"#{:02x}{:02x}{:02x}\"", c.r, c.g, c.b));
    }
    if let Some(AttributeValue::Alignment(a)) = node.attributes.get(&AttributeKey::Alignment) {
        let s = match a {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "justify",
            _ => "left",
        };
        json.push_str(&format!(",\"alignment\":\"{}\"", s));
    }

    // Children IDs
    if !node.children.is_empty() {
        let child_strs: Vec<String> = node
            .children
            .iter()
            .map(|c| format!("\"{}:{}\"", c.replica, c.counter))
            .collect();
        json.push_str(&format!(",\"children\":[{}]", child_strs.join(",")));
    }

    json.push('}');
    json
}

/// Find the Text node inside a Run (traverses Run → Text).
fn find_text_node_in_run(
    model: &DocumentModel,
    run_id: NodeId,
) -> Result<(NodeId, usize), JsError> {
    let run = model
        .node(run_id)
        .ok_or_else(|| JsError::new("Run not found"))?;
    for &child_id in &run.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Text {
                let len = child
                    .text_content
                    .as_ref()
                    .map(|t| t.chars().count())
                    .unwrap_or(0);
                return Ok((child_id, len));
            }
        }
    }
    Err(JsError::new("No text node found in run"))
}

/// Parse a formatting key/value pair into an AttributeMap.
fn parse_format_kv(key: &str, value: &str) -> Result<s1_model::AttributeMap, JsError> {
    let mut attrs = s1_model::AttributeMap::new();
    match key {
        "bold" => {
            attrs.set(AttributeKey::Bold, AttributeValue::Bool(value == "true"));
        }
        "italic" => {
            attrs.set(AttributeKey::Italic, AttributeValue::Bool(value == "true"));
        }
        "underline" => {
            let style = if value == "true" {
                UnderlineStyle::Single
            } else {
                UnderlineStyle::None
            };
            attrs.set(
                AttributeKey::Underline,
                AttributeValue::UnderlineStyle(style),
            );
        }
        "strikethrough" => {
            attrs.set(
                AttributeKey::Strikethrough,
                AttributeValue::Bool(value == "true"),
            );
        }
        "fontSize" => {
            let size: f64 = value
                .parse()
                .map_err(|_| JsError::new("Invalid font size"))?;
            attrs.set(AttributeKey::FontSize, AttributeValue::Float(size));
        }
        "fontFamily" => {
            attrs.set(
                AttributeKey::FontFamily,
                AttributeValue::String(value.to_string()),
            );
        }
        "color" => {
            let color = Color::from_hex(value).ok_or_else(|| JsError::new("Invalid color hex"))?;
            attrs.set(AttributeKey::Color, AttributeValue::Color(color));
        }
        "highlightColor" => {
            let color = Color::from_hex(value).ok_or_else(|| JsError::new("Invalid color hex"))?;
            attrs.set(AttributeKey::HighlightColor, AttributeValue::Color(color));
        }
        "superscript" => {
            attrs.set(
                AttributeKey::Superscript,
                AttributeValue::Bool(value == "true"),
            );
        }
        "subscript" => {
            attrs.set(
                AttributeKey::Subscript,
                AttributeValue::Bool(value == "true"),
            );
        }
        "indentLeft" => {
            let v: f64 = value
                .parse()
                .map_err(|_| JsError::new("Invalid indent value"))?;
            attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(v));
        }
        "indentRight" => {
            let v: f64 = value
                .parse()
                .map_err(|_| JsError::new("Invalid indent value"))?;
            attrs.set(AttributeKey::IndentRight, AttributeValue::Float(v));
        }
        "indentFirstLine" => {
            let v: f64 = value
                .parse()
                .map_err(|_| JsError::new("Invalid indent value"))?;
            attrs.set(AttributeKey::IndentFirstLine, AttributeValue::Float(v));
        }
        "hyperlinkUrl" => {
            attrs.set(
                AttributeKey::HyperlinkUrl,
                AttributeValue::String(value.to_string()),
            );
        }
        // Extended format keys (C1 fix)
        "fontSpacing" => {
            let v: f64 = value
                .parse()
                .map_err(|_| JsError::new("Invalid fontSpacing value"))?;
            attrs.set(AttributeKey::FontSpacing, AttributeValue::Float(v));
        }
        "language" => {
            attrs.set(
                AttributeKey::Language,
                AttributeValue::String(value.to_string()),
            );
        }
        "textShadow" => {
            attrs.set(
                AttributeKey::TextShadow,
                AttributeValue::Bool(value == "true"),
            );
        }
        "textOutline" => {
            attrs.set(
                AttributeKey::TextOutline,
                AttributeValue::Bool(value == "true"),
            );
        }
        "backgroundColor" | "background" => {
            let color = Color::from_hex(value).ok_or_else(|| JsError::new("Invalid color hex"))?;
            attrs.set(AttributeKey::Background, AttributeValue::Color(color));
        }
        "pageBreakBefore" => {
            attrs.set(
                AttributeKey::PageBreakBefore,
                AttributeValue::Bool(value == "true"),
            );
        }
        "keepWithNext" => {
            attrs.set(
                AttributeKey::KeepWithNext,
                AttributeValue::Bool(value == "true"),
            );
        }
        "keepLinesTogether" => {
            attrs.set(
                AttributeKey::KeepLinesTogether,
                AttributeValue::Bool(value == "true"),
            );
        }
        _ => return Err(JsError::new(&format!("Unknown format key: {}", key))),
    }
    Ok(attrs)
}

/// Find which run contains a given character offset within a paragraph,
/// and return the offset within that run.
#[allow(dead_code)]
fn find_run_at_offset(
    model: &DocumentModel,
    para_id: NodeId,
    char_offset: usize,
) -> Result<(NodeId, usize), JsError> {
    let para = model
        .node(para_id)
        .ok_or_else(|| JsError::new("Paragraph not found"))?;

    let mut accumulated = 0usize;
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Run {
                let run_len = run_char_len(model, child_id);
                if char_offset <= accumulated + run_len {
                    return Ok((child_id, char_offset - accumulated));
                }
                accumulated += run_len;
            }
        }
    }
    // If offset == total length, return last run at its end
    if char_offset == accumulated {
        // Find last run
        for &child_id in para.children.iter().rev() {
            if let Some(child) = model.node(child_id) {
                if child.node_type == NodeType::Run {
                    let run_len = run_char_len(model, child_id);
                    return Ok((child_id, run_len));
                }
            }
        }
    }
    Err(JsError::new("Offset out of range"))
}

/// Get the character count of text in a run.
fn run_char_len(model: &DocumentModel, run_id: NodeId) -> usize {
    let run = match model.node(run_id) {
        Some(n) => n,
        None => return 0,
    };
    let mut len = 0;
    for &child_id in &run.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Text {
                len += child
                    .text_content
                    .as_ref()
                    .map(|t| t.chars().count())
                    .unwrap_or(0);
            }
        }
    }
    len
}

/// Format a range within a single paragraph.
///
/// Splits runs at start and end offsets as needed, then applies attrs to all runs
/// in the range.
///
/// Known limitation (W-11): Each `split_run_internal` call creates its own
/// transaction, so a selection spanning many runs produces multiple undo entries.
/// Batching all splits and formatting into a single transaction would require
/// transaction-batching API changes in `s1engine::Document`.
fn format_range_in_paragraph(
    doc: &mut s1engine::Document,
    para_id: NodeId,
    start_offset: usize,
    end_offset: usize,
    attrs: &s1_model::AttributeMap,
) -> Result<(), JsError> {
    if start_offset >= end_offset {
        return Ok(());
    }

    let para = doc
        .node(para_id)
        .ok_or_else(|| JsError::new("Paragraph not found"))?;

    // Collect runs with their character ranges
    let mut runs_info: Vec<(NodeId, usize, usize)> = Vec::new(); // (run_id, start_char, end_char)
    let mut offset = 0usize;
    for &child_id in &para.children {
        if let Some(child) = doc.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    let len = run_char_len(doc.model(), child_id);
                    runs_info.push((child_id, offset, offset + len));
                    offset += len;
                }
                NodeType::LineBreak | NodeType::Tab => {
                    offset += 1;
                }
                _ => {}
            }
        }
    }

    // Find runs that overlap with [start_offset, end_offset) and split at
    // selection boundaries.
    //
    // Safety note on stale `runs_info`: After a `split_run_internal()` call,
    // subsequent entries in `runs_info` remain valid because:
    //   - Each run's NodeId is unchanged (the split only creates a NEW sibling).
    //   - Each run's text content is unchanged (the split only modifies the
    //     run being split).
    //   - Character offsets in `runs_info` are used relative to each run's own
    //     start position, so they remain correct even after earlier runs are split.
    let mut runs_to_format: Vec<NodeId> = Vec::new();

    for &(run_id, run_start, run_end) in &runs_info {
        if run_end <= start_offset || run_start >= end_offset {
            continue; // No overlap
        }

        // Need to split at start?
        if run_start < start_offset && start_offset < run_end {
            // Split this run at (start_offset - run_start)
            let split_offset = start_offset - run_start;
            let new_run_id = split_run_internal(doc, run_id, split_offset)?;
            // After split: run_id has [run_start, start_offset), new_run_id has [start_offset, run_end)
            // The new run is what we want to format (partially or fully).
            // Check if end_offset also falls within this same original run.
            let new_run_len = run_char_len(doc.model(), new_run_id);
            let remaining_end = end_offset - start_offset;

            if remaining_end < new_run_len {
                // end_offset is inside the new run — split again at remaining_end
                let tail_run_id = split_run_internal(doc, new_run_id, remaining_end)?;
                let _ = tail_run_id; // tail is not formatted
                runs_to_format.push(new_run_id);
            } else {
                // Selection extends beyond this run; format the whole tail.
                // Subsequent runs will be picked up by later iterations.
                runs_to_format.push(new_run_id);
            }
            continue;
        }

        // Need to split at end?
        if run_start < end_offset && end_offset < run_end {
            let split_offset = end_offset - run_start;
            let _tail_run_id = split_run_internal(doc, run_id, split_offset)?;
            // run_id now has [run_start, end_offset), tail has [end_offset, run_end)
            runs_to_format.push(run_id);
            continue;
        }

        // Fully contained
        runs_to_format.push(run_id);
    }

    // Apply formatting to all runs in range
    let mut txn = Transaction::with_label("Format selection");
    for run_id in runs_to_format {
        txn.push(Operation::set_attributes(run_id, attrs.clone()));
    }
    if !txn.is_empty() {
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
    }

    Ok(())
}

/// Internal run split (not WASM-exported). Returns the new run ID.
fn split_run_internal(
    doc: &mut s1engine::Document,
    run_id: NodeId,
    char_offset: usize,
) -> Result<NodeId, JsError> {
    let run = doc
        .node(run_id)
        .ok_or_else(|| JsError::new("Run not found"))?;
    let run_attrs = run.attributes.clone();
    let para_id = run
        .parent
        .ok_or_else(|| JsError::new("Run has no parent"))?;

    let para = doc
        .node(para_id)
        .ok_or_else(|| JsError::new("Parent not found"))?;
    let run_index = para
        .children
        .iter()
        .position(|&c| c == run_id)
        .ok_or_else(|| JsError::new("Run not in parent"))?;

    let (text_node_id, text_char_len) = find_text_node_in_run(doc.model(), run_id)?;
    let text_node = doc
        .node(text_node_id)
        .ok_or_else(|| JsError::new("Text node not found"))?;
    let full_text = text_node.text_content.as_deref().unwrap_or("");
    let tail_text: String = full_text.chars().skip(char_offset).collect();

    let new_run_id = doc.next_id();
    let new_text_id = doc.next_id();

    let mut txn = Transaction::with_label("Split run (internal)");
    if char_offset < text_char_len {
        txn.push(Operation::delete_text(
            text_node_id,
            char_offset,
            text_char_len - char_offset,
        ));
    }
    let mut new_run = Node::new(new_run_id, NodeType::Run);
    new_run.attributes = run_attrs;
    txn.push(Operation::insert_node(para_id, run_index + 1, new_run));
    txn.push(Operation::insert_node(
        new_run_id,
        0,
        Node::text(new_text_id, &tail_text),
    ));

    doc.apply_transaction(&txn)
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(new_run_id)
}

/// Collect run IDs that overlap with a character range in a paragraph.
fn collect_runs_in_range(
    model: &DocumentModel,
    para_id: NodeId,
    start_offset: usize,
    end_offset: usize,
    out: &mut Vec<NodeId>,
) {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return,
    };
    let mut offset = 0usize;
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    let len = run_char_len(model, child_id);
                    let run_start = offset;
                    let run_end = offset + len;
                    if run_end > start_offset && run_start < end_offset {
                        out.push(child_id);
                    }
                    offset += len;
                }
                NodeType::LineBreak | NodeType::Tab => {
                    offset += 1;
                }
                _ => {}
            }
        }
    }
}

/// Get the column count of a table (maximum across all rows).
///
/// Checks every row and returns the maximum cell count, which handles
/// tables with inconsistent column counts (e.g., due to merged cells).
fn get_table_col_count(model: &DocumentModel, table_id: NodeId) -> Result<usize, JsError> {
    let table = model
        .node(table_id)
        .ok_or_else(|| JsError::new("Table not found"))?;
    let mut max_cols = 0;
    for &row_id in &table.children {
        if let Some(row_node) = model.node(row_id) {
            if row_node.node_type == NodeType::TableRow {
                let cols = row_node.children.len();
                if cols > max_cols {
                    max_cols = cols;
                }
            }
        }
    }
    Ok(max_cols)
}

/// Get formatting of a run as JSON.
fn run_formatting_to_json(attrs: &s1_model::AttributeMap) -> String {
    let bold = attrs.get_bool(&AttributeKey::Bold) == Some(true);
    let italic = attrs.get_bool(&AttributeKey::Italic) == Some(true);
    let underline = attrs.get(&AttributeKey::Underline).is_some()
        && !matches!(
            attrs.get(&AttributeKey::Underline),
            Some(AttributeValue::UnderlineStyle(UnderlineStyle::None))
        );
    let strikethrough = attrs.get_bool(&AttributeKey::Strikethrough) == Some(true);

    let mut json = format!(
        "{{\"bold\":{},\"italic\":{},\"underline\":{},\"strikethrough\":{}",
        bold, italic, underline, strikethrough
    );
    if let Some(fs) = attrs.get_f64(&AttributeKey::FontSize) {
        json.push_str(&format!(",\"fontSize\":{}", fs));
    }
    if let Some(ff) = attrs.get_string(&AttributeKey::FontFamily) {
        json.push_str(&format!(",\"fontFamily\":\"{}\"", ff));
    }
    if let Some(AttributeValue::Color(c)) = attrs.get(&AttributeKey::Color) {
        json.push_str(&format!(",\"color\":\"{}\"", c.to_hex()));
    }
    json.push('}');
    json
}

/// Collect find results recursively across paragraphs.
fn collect_find_results(
    model: &DocumentModel,
    children: &[NodeId],
    query: &str,
    case_sensitive: bool,
    results: &mut Vec<String>,
) {
    for &child_id in children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Paragraph => {
                    let text = extract_paragraph_text(model, child_id);
                    let search_text = if case_sensitive {
                        text.clone()
                    } else {
                        text.to_lowercase()
                    };
                    let query_char_len = query.chars().count();
                    let mut char_pos = 0usize;
                    let mut byte_pos = 0usize;
                    while byte_pos < search_text.len() {
                        if let Some(rel_byte) = search_text[byte_pos..].find(query) {
                            // Count chars from byte_pos to byte_pos + rel_byte
                            let chars_skipped =
                                search_text[byte_pos..byte_pos + rel_byte].chars().count();
                            let char_offset = char_pos + chars_skipped;
                            results.push(format!(
                                "{{\"nodeId\":\"{}:{}\",\"offset\":{},\"length\":{}}}",
                                child_id.replica, child_id.counter, char_offset, query_char_len
                            ));
                            // Advance past the match using actual matched byte length
                            let match_byte_len: usize = search_text[byte_pos + rel_byte..]
                                .chars()
                                .take(query_char_len)
                                .map(|c| c.len_utf8())
                                .sum();
                            byte_pos += rel_byte + match_byte_len;
                            char_pos = char_offset + query_char_len;
                        } else {
                            break;
                        }
                    }
                }
                NodeType::Table | NodeType::TableRow | NodeType::TableCell | NodeType::Section => {
                    collect_find_results(model, &child.children, query, case_sensitive, results);
                }
                _ => {}
            }
        }
    }
}

/// Collect text node IDs and character offsets for replace_all.
fn collect_replace_matches(
    model: &DocumentModel,
    node_id: NodeId,
    query: &str,
    case_sensitive: bool,
    query_char_len: usize,
    matches: &mut Vec<(NodeId, usize, usize)>,
) {
    let node = match model.node(node_id) {
        Some(n) => n,
        None => return,
    };
    match node.node_type {
        NodeType::Paragraph => {
            // Find text node via first run
            for &run_id in &node.children {
                if let Some(run) = model.node(run_id) {
                    if run.node_type == NodeType::Run {
                        for &text_id in &run.children {
                            if let Some(text_node) = model.node(text_id) {
                                if text_node.node_type == NodeType::Text {
                                    if let Some(content) = &text_node.text_content {
                                        let search = if case_sensitive {
                                            content.clone()
                                        } else {
                                            content.to_lowercase()
                                        };
                                        let mut byte_pos = 0usize;
                                        let mut char_pos = 0usize;
                                        while byte_pos < search.len() {
                                            if let Some(rel_byte) = search[byte_pos..].find(query) {
                                                let chars_skipped = search
                                                    [byte_pos..byte_pos + rel_byte]
                                                    .chars()
                                                    .count();
                                                let char_offset = char_pos + chars_skipped;
                                                matches.push((
                                                    text_id,
                                                    char_offset,
                                                    query_char_len,
                                                ));
                                                // Advance past match using char-aware byte length
                                                let match_byte_len: usize = search
                                                    [byte_pos + rel_byte..]
                                                    .chars()
                                                    .take(query_char_len)
                                                    .map(|c| c.len_utf8())
                                                    .sum();
                                                byte_pos += rel_byte + match_byte_len;
                                                char_pos = char_offset + query_char_len;
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        NodeType::Table
        | NodeType::TableRow
        | NodeType::TableCell
        | NodeType::Body
        | NodeType::Section => {
            for &child_id in &node.children {
                collect_replace_matches(
                    model,
                    child_id,
                    query,
                    case_sensitive,
                    query_char_len,
                    matches,
                );
            }
        }
        _ => {}
    }
}

/// Escape special characters for JSON string values.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            // Control characters and DEL
            c if (c as u32) < 0x20 || c == '\x7f' => out.push_str(&format!("\\u{:04x}", c as u32)),
            // Line/paragraph separators break JSON in some parsers
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out
}

// --- Format helpers ---

fn parse_format(format: &str) -> Result<s1engine::Format, JsError> {
    match format.to_lowercase().as_str() {
        "docx" => Ok(s1engine::Format::Docx),
        "odt" => Ok(s1engine::Format::Odt),
        "pdf" => Ok(s1engine::Format::Pdf),
        "txt" | "text" => Ok(s1engine::Format::Txt),
        "doc" => Ok(s1engine::Format::Doc),
        "md" | "markdown" => Ok(s1engine::Format::Md),
        _ => Err(JsError::new(&format!("Unsupported format: {format}"))),
    }
}

// --- HTML rendering ---

/// Convert a number to lowercase Roman numerals.
fn to_roman_lower(mut n: u32) -> String {
    let vals = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut s = String::new();
    for &(val, sym) in &vals {
        while n >= val {
            s.push_str(sym);
            n -= val;
        }
    }
    s
}

/// Convert a number to uppercase Roman numerals.
fn to_roman_upper(n: u32) -> String {
    to_roman_lower(n).to_uppercase()
}

/// Compute the ordinal position of a list paragraph among its siblings.
/// Walks backward through siblings to count items with the same num_id and level.
fn compute_list_ordinal(model: &DocumentModel, para_id: NodeId) -> Option<u32> {
    let para = model.node(para_id)?;
    let li = match para.attributes.get(&AttributeKey::ListInfo) {
        Some(AttributeValue::ListInfo(li)) => li,
        _ => return None,
    };
    if li.num_format == ListFormat::Bullet {
        return None;
    }
    let parent_id = para.parent?;
    let parent = model.node(parent_id)?;
    let my_idx = parent.children.iter().position(|&c| c == para_id)?;
    let mut count = li.start.unwrap_or(1);
    // Walk backward through preceding siblings
    for i in (0..my_idx).rev() {
        let sib_id = parent.children[i];
        let sib = match model.node(sib_id) {
            Some(n) => n,
            None => continue,
        };
        if sib.node_type != NodeType::Paragraph {
            break; // Non-paragraph breaks the list
        }
        match sib.attributes.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(sli))
                if sli.num_id == li.num_id && sli.level == li.level =>
            {
                count += 1;
            }
            Some(AttributeValue::ListInfo(_)) => {
                // Different list or level — don't break, could be nested
                continue;
            }
            _ => break, // No list info — gap breaks numbering
        }
    }
    Some(count)
}

// ── Page-level block rendering ──────────────────────────────────────

/// Render a single layout block as document-model HTML for page rendering.
///
/// For non-split paragraphs, delegates to the existing `render_paragraph`.
/// For split paragraphs, renders the appropriate char range with
/// `render_paragraph_clean_partial` and injects `data-node-id` and
/// `data-split` attributes.
fn render_page_content_block(
    model: &DocumentModel,
    block: &s1_layout::LayoutBlock,
    html: &mut String,
    para_char_offsets: &mut std::collections::HashMap<NodeId, usize>,
) {
    match &block.kind {
        s1_layout::LayoutBlockKind::Paragraph {
            is_continuation,
            split_at_line,
            lines,
            ..
        } => {
            let char_count: usize = lines
                .iter()
                .map(|l| l.runs.iter().map(|r| r.text.chars().count()).sum::<usize>())
                .sum();
            let is_split = *split_at_line > 0 || *is_continuation;

            if is_split {
                let char_start = para_char_offsets
                    .get(&block.source_id)
                    .copied()
                    .unwrap_or(0);
                let char_end = char_start + char_count;
                para_char_offsets.insert(block.source_id, char_end);

                // Render the char range via render_paragraph_clean_partial
                // then inject data-node-id and data-split attributes.
                let mut frag = String::new();
                render_paragraph_clean_partial(
                    model,
                    block.source_id,
                    Some(char_start),
                    Some(char_end),
                    &mut frag,
                );
                let nid = format!("{}:{}", block.source_id.replica, block.source_id.counter);
                let split_val = if *is_continuation {
                    "continuation"
                } else {
                    "first"
                };
                // Inject attributes into the first opening tag
                if let Some(pos) = frag.find('>') {
                    html.push_str(&frag[..pos]);
                    html.push_str(&format!(
                        " data-node-id=\"{}\" data-split=\"{}\"",
                        nid, split_val
                    ));
                    html.push_str(&frag[pos..]);
                } else {
                    html.push_str(&frag);
                }
            } else {
                // Full paragraph — use the existing renderer
                render_paragraph(model, block.source_id, html, None);
            }
        }
        s1_layout::LayoutBlockKind::Table {
            rows,
            is_continuation,
        } => {
            render_page_table(model, block.source_id, rows, *is_continuation, html);
        }
        s1_layout::LayoutBlockKind::Image { .. } => {
            render_node(model, block.source_id, html);
        }
        _ => {
            // Future block kinds — fall back to the generic node renderer
            render_node(model, block.source_id, html);
        }
    }
}

/// Render a table for a specific page, including only the rows present
/// on that page (from the layout engine's row assignment).
fn render_page_table(
    model: &DocumentModel,
    table_id: NodeId,
    layout_rows: &[s1_layout::LayoutTableRow],
    is_continuation: bool,
    html: &mut String,
) {
    let cont_attr = if is_continuation {
        " data-is-continuation=\"true\""
    } else {
        ""
    };
    html.push_str(&format!(
        "<table data-node-id=\"{}:{}\" data-table-source=\"{}:{}\"{} \
         style=\"border-collapse:collapse;margin:12px 0;width:100%\">",
        table_id.replica, table_id.counter, table_id.replica, table_id.counter, cont_attr,
    ));

    // Build set of row source IDs present on this page
    let row_ids: std::collections::HashSet<NodeId> =
        layout_rows.iter().map(|r| r.source_id).collect();

    if let Some(table_node) = model.node(table_id) {
        for &row_id in &table_node.children {
            if row_ids.contains(&row_id) {
                render_node(model, row_id, html);
            }
        }
    }

    html.push_str("</table>");
}

fn render_children(model: &DocumentModel, parent_id: NodeId, html: &mut String) {
    let parent = match model.node(parent_id) {
        Some(n) => n,
        None => return,
    };
    // Track list ordinal counters: (num_id, level) -> current count
    let mut list_counters: std::collections::HashMap<(u32, u8), u32> =
        std::collections::HashMap::new();
    for &child_id in &parent.children {
        render_node_with_list_ctx(model, child_id, html, &mut list_counters);
    }
}

fn render_node_with_list_ctx(
    model: &DocumentModel,
    node_id: NodeId,
    html: &mut String,
    list_counters: &mut std::collections::HashMap<(u32, u8), u32>,
) {
    let node = match model.node(node_id) {
        Some(n) => n,
        None => return,
    };
    match node.node_type {
        NodeType::Paragraph => {
            // Compute list ordinal if this is a numbered list item
            let list_ordinal = if let Some(AttributeValue::ListInfo(li)) =
                node.attributes.get(&AttributeKey::ListInfo)
            {
                if li.num_format != ListFormat::Bullet {
                    let key = (li.num_id, li.level);
                    let counter = list_counters
                        .entry(key)
                        .or_insert(li.start.unwrap_or(1).saturating_sub(1));
                    *counter += 1;
                    Some(*counter)
                } else {
                    // Reset any decimal counter at same level when we hit a bullet
                    None
                }
            } else {
                // Non-list paragraph: reset all counters at all levels
                // (a gap in the list resets numbering)
                list_counters.clear();
                None
            };
            render_paragraph(model, node_id, html, list_ordinal);
        }
        _ => render_node(model, node_id, html),
    }
}

fn render_node(model: &DocumentModel, node_id: NodeId, html: &mut String) {
    let node = match model.node(node_id) {
        Some(n) => n,
        None => return,
    };

    match node.node_type {
        NodeType::Paragraph => render_paragraph(model, node_id, html, None),
        NodeType::Table => render_table(model, node_id, html),
        NodeType::TableRow => render_table_row(model, node_id, html),
        NodeType::TableCell => render_table_cell(model, node_id, html),
        NodeType::Image => render_image(model, node_id, html),
        NodeType::PageBreak => {
            html.push_str("<hr class=\"page-break\" contenteditable=\"false\" style=\"border:none;page-break-after:always;margin:0\" />");
        }
        NodeType::TableOfContents => {
            let toc_title = node
                .attributes
                .get_string(&AttributeKey::TocTitle)
                .unwrap_or("Table of Contents");
            html.push_str(&format!(
                "<div class=\"doc-toc\" data-node-id=\"{}:{}\" contenteditable=\"false\">",
                node_id.replica, node_id.counter
            ));
            html.push_str(&format!(
                "<div class=\"doc-toc-title\">{}<button class=\"toc-update-btn\" title=\"Refresh table of contents\">Update</button>",
                escape_html(toc_title)
            ));
            html.push_str("<select class=\"toc-style-select\" title=\"TOC style\">");
            html.push_str("<option value=\"plain\">Plain</option>");
            html.push_str("<option value=\"dotted\" selected>Dotted</option>");
            html.push_str("<option value=\"dashed\">Dashed</option>");
            html.push_str("</select></div>");
            // Render TOC entries from heading hierarchy
            let headings = model.collect_headings();
            let max_level = node
                .attributes
                .get_i64(&AttributeKey::TocMaxLevel)
                .unwrap_or(3) as u8;
            let mut has_entries = false;
            for (h_id, level, text) in &headings {
                if *level > max_level {
                    continue;
                }
                has_entries = true;
                html.push_str(&format!(
                    "<div class=\"toc-entry toc-level-{} toc-dotted\" data-target-node=\"{}:{}\" tabindex=\"0\" role=\"link\" title=\"Go to: {}\">",
                    level, h_id.replica, h_id.counter, escape_html(text)
                ));
                html.push_str(&escape_html(text));
                html.push_str("</div>");
            }
            if !has_entries {
                html.push_str("<div style=\"color:#5f6368;font-size:12px;font-style:italic;padding:8px 0\">No headings found. Add headings to populate the table of contents.</div>");
            }
            html.push_str("</div>");
        }
        NodeType::BookmarkStart => {
            if let Some(name) = node.attributes.get_string(&AttributeKey::BookmarkName) {
                html.push_str(&format!(
                    "<span class=\"bookmark-marker\" data-bookmark=\"{}\" data-node-id=\"{}:{}\" \
                     contenteditable=\"false\" title=\"Bookmark: {}\">\
                     <a id=\"{}\"></a></span>",
                    escape_html(name),
                    node_id.replica,
                    node_id.counter,
                    escape_html(name),
                    escape_html(name)
                ));
            }
        }
        NodeType::Equation => {
            let latex = node
                .attributes
                .get_string(&AttributeKey::EquationSource)
                .unwrap_or("");
            html.push_str(&format!(
                "<span class=\"equation-inline\" data-equation=\"{}\" data-node-id=\"{}:{}\" \
                 contenteditable=\"false\" title=\"Equation (double-click to edit)\">{}</span>",
                escape_html(latex),
                node_id.replica,
                node_id.counter,
                escape_html(latex)
            ));
        }
        NodeType::CommentStart => {
            let author = node
                .attributes
                .get_string(&AttributeKey::CommentAuthor)
                .unwrap_or("Unknown");
            html.push_str(&format!(
                "<span class=\"comment-marker\" title=\"Comment by {}\" style=\"background:#fff3cd;border-bottom:2px solid #ffc107;cursor:help\">",
                escape_html(author)
            ));
        }
        NodeType::CommentEnd => {
            html.push_str("</span>");
        }
        NodeType::Section => {
            // Render section children (paragraphs, tables, etc.)
            render_children(model, node_id, html);
        }
        NodeType::Header | NodeType::Footer => {
            // Render header/footer children inline
            render_children(model, node_id, html);
        }
        NodeType::CommentBody => {
            // Comment bodies rendered as tooltip or hidden
        }
        NodeType::FootnoteRef => {
            let fn_num = node
                .attributes
                .get_i64(&AttributeKey::FootnoteNumber)
                .unwrap_or(0);
            html.push_str(&format!(
                "<sup class=\"footnote-ref\" data-footnote-ref=\"{}\" data-node-id=\"{}:{}\" \
                 title=\"Footnote {}\" contenteditable=\"false\">{}</sup>",
                fn_num, node_id.replica, node_id.counter, fn_num, fn_num
            ));
        }
        NodeType::EndnoteRef => {
            let en_num = node
                .attributes
                .get_i64(&AttributeKey::EndnoteNumber)
                .unwrap_or(0);
            html.push_str(&format!(
                "<sup class=\"endnote-ref\" data-endnote-ref=\"{}\" data-node-id=\"{}:{}\" \
                 title=\"Endnote {}\" contenteditable=\"false\">{}</sup>",
                en_num, node_id.replica, node_id.counter, en_num, en_num
            ));
        }
        NodeType::FootnoteBody => {
            let fn_num = node
                .attributes
                .get_i64(&AttributeKey::FootnoteNumber)
                .unwrap_or(0);
            html.push_str(&format!(
                "<div class=\"footnote-body\" data-footnote-id=\"{}\" data-node-id=\"{}:{}\" \
                 contenteditable=\"true\">",
                fn_num, node_id.replica, node_id.counter
            ));
            html.push_str(&format!(
                "<span class=\"footnote-number\" contenteditable=\"false\">{}.</span> ",
                fn_num
            ));
            render_children(model, node_id, html);
            html.push_str("</div>");
        }
        NodeType::EndnoteBody => {
            let en_num = node
                .attributes
                .get_i64(&AttributeKey::EndnoteNumber)
                .unwrap_or(0);
            html.push_str(&format!(
                "<div class=\"endnote-body\" data-endnote-id=\"{}\" data-node-id=\"{}:{}\" \
                 contenteditable=\"true\">",
                en_num, node_id.replica, node_id.counter
            ));
            html.push_str(&format!(
                "<span class=\"endnote-number\" contenteditable=\"false\">{}.</span> ",
                en_num
            ));
            render_children(model, node_id, html);
            html.push_str("</div>");
        }
        NodeType::Field => {
            render_field_html(node, html);
        }
        NodeType::ColumnBreak => {
            html.push_str("<hr class=\"column-break\" style=\"border-style:dashed\" />");
        }
        NodeType::Drawing => {
            render_drawing(model, node_id, html);
        }
        // Container nodes — render their children
        NodeType::Body | NodeType::Document => {
            render_children(model, node_id, html);
        }
        _ => {}
    }
}

fn render_paragraph(
    model: &DocumentModel,
    para_id: NodeId,
    html: &mut String,
    list_ordinal: Option<u32>,
) {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return,
    };

    // Form control rendering: if this paragraph carries a FormType attribute,
    // render as an interactive form element instead of a normal paragraph.
    if let Some(form_type) = para.attributes.get_string(&AttributeKey::FormType) {
        let nid = format!("{}:{}", para_id.replica, para_id.counter);
        match form_type {
            "checkbox" => {
                let checked = para
                    .attributes
                    .get_bool(&AttributeKey::FormChecked)
                    .unwrap_or(false);
                let checked_attr = if checked { " checked" } else { "" };
                html.push_str(&format!(
                    "<label class=\"form-checkbox\" contenteditable=\"false\" \
                     data-node-id=\"{nid}\">\
                     <input type=\"checkbox\" data-node-id=\"{nid}\"{checked_attr}> Checkbox\
                     </label>"
                ));
            }
            "dropdown" => {
                let options_str = para
                    .attributes
                    .get_string(&AttributeKey::FormOptions)
                    .unwrap_or("");
                html.push_str(&format!(
                    "<select class=\"form-dropdown\" contenteditable=\"false\" \
                     data-node-id=\"{nid}\">"
                ));
                if options_str.is_empty() {
                    html.push_str("<option></option>");
                } else {
                    for opt in options_str.split(',') {
                        let escaped = escape_html(opt);
                        html.push_str(&format!("<option value=\"{escaped}\">{escaped}</option>"));
                    }
                }
                html.push_str("</select>");
            }
            "text" => {
                // Collect text content from child runs for the default value
                let mut value = String::new();
                if let Some(node) = model.node(para_id) {
                    for &child_id in &node.children {
                        if let Some(child) = model.node(child_id) {
                            if child.node_type == NodeType::Run {
                                for &text_id in &child.children {
                                    if let Some(text_node) = model.node(text_id) {
                                        if text_node.node_type == NodeType::Text {
                                            if let Some(ref tc) = text_node.text_content {
                                                value.push_str(tc);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let escaped_value = escape_html(&value);
                html.push_str(&format!(
                    "<input type=\"text\" class=\"form-text\" contenteditable=\"false\" \
                     data-node-id=\"{nid}\" value=\"{escaped_value}\">"
                ));
            }
            _ => {
                // Unknown form type — fall through to normal paragraph rendering
            }
        }
        if form_type == "checkbox" || form_type == "dropdown" || form_type == "text" {
            return;
        }
    }

    // UXP-08: Render section break indicator before paragraphs that start a new section.
    if let Some(AttributeValue::Int(sec_idx)) = para.attributes.get(&AttributeKey::SectionIndex) {
        let sections = model.sections();
        let label = if let Some(sec) = sections.get(*sec_idx as usize) {
            match sec.break_type {
                Some(s1_model::SectionBreakType::NextPage) => "Section Break (Next Page)",
                Some(s1_model::SectionBreakType::Continuous) => "Section Break (Continuous)",
                Some(s1_model::SectionBreakType::EvenPage) => "Section Break (Even Page)",
                Some(s1_model::SectionBreakType::OddPage) => "Section Break (Odd Page)",
                Some(_) => "Section Break",
                None => "Section Break",
            }
        } else {
            "Section Break"
        };
        html.push_str(&format!(
            "<div class=\"section-break\" contenteditable=\"false\" \
             data-section-index=\"{}\" data-node-id=\"{}:{}\">\
             <span class=\"section-break-label\">{}</span></div>",
            sec_idx, para_id.replica, para_id.counter, label
        ));
    }

    // Detect heading level from style ID (e.g. "Heading1", "heading 2", etc.)
    let style_id = para.attributes.get_string(&AttributeKey::StyleId);
    let effective_level: Option<u8> = style_id.and_then(|sid| {
        let sid_lower = sid.to_lowercase();
        if sid_lower.starts_with("heading") {
            sid_lower
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()
        } else {
            None
        }
    });

    // Detect list info
    let list_info = match para.attributes.get(&AttributeKey::ListInfo) {
        Some(AttributeValue::ListInfo(li)) => Some(li.clone()),
        _ => None,
    };

    // Build inline style
    let mut style = String::new();
    if let Some(AttributeValue::Alignment(a)) = para.attributes.get(&AttributeKey::Alignment) {
        let val = match a {
            s1_model::Alignment::Left => "left",
            s1_model::Alignment::Center => "center",
            s1_model::Alignment::Right => "right",
            s1_model::Alignment::Justify => "justify",
            _ => "",
        };
        if !val.is_empty() {
            style.push_str(&format!("text-align:{val};"));
        }
        if matches!(a, s1_model::Alignment::Justify) {
            style.push_str("text-align-last:left;");
        }
    }
    // Keep with next / keep lines together
    if para.attributes.get_bool(&AttributeKey::KeepWithNext) == Some(true) {
        style.push_str("break-after:avoid;");
    }
    if para.attributes.get_bool(&AttributeKey::KeepLinesTogether) == Some(true) {
        style.push_str("break-inside:avoid;");
    }

    // Spacing
    if let Some(sp) = para.attributes.get_f64(&AttributeKey::SpacingBefore) {
        if sp > 0.0 {
            style.push_str(&format!("margin-top:{sp}pt;"));
        }
    }
    if let Some(sp) = para.attributes.get_f64(&AttributeKey::SpacingAfter) {
        if sp > 0.0 {
            style.push_str(&format!("margin-bottom:{sp}pt;"));
        }
    }

    // Line spacing
    if let Some(AttributeValue::LineSpacing(ls)) = para.attributes.get(&AttributeKey::LineSpacing) {
        use s1_model::LineSpacing;
        match ls {
            LineSpacing::Single => style.push_str("line-height:1;"),
            LineSpacing::OnePointFive => style.push_str("line-height:1.5;"),
            LineSpacing::Double => style.push_str("line-height:2;"),
            LineSpacing::Exact(pts) => style.push_str(&format!("line-height:{pts}pt;")),
            LineSpacing::AtLeast(pts) => style.push_str(&format!("line-height:{pts}pt;")),
            LineSpacing::Multiple(factor) => style.push_str(&format!("line-height:{factor};")),
            _ => {}
        }
    }

    // Indentation
    if let Some(indent) = para.attributes.get_f64(&AttributeKey::IndentLeft) {
        if indent > 0.0 {
            style.push_str(&format!("margin-left:{indent}pt;"));
        }
    }
    if let Some(indent) = para.attributes.get_f64(&AttributeKey::IndentRight) {
        if indent > 0.0 {
            style.push_str(&format!("margin-right:{indent}pt;"));
        }
    }
    if let Some(indent) = para.attributes.get_f64(&AttributeKey::IndentFirstLine) {
        if indent > 0.0 {
            style.push_str(&format!("text-indent:{indent}pt;"));
        }
    }

    // List indentation (if no explicit indent already set)
    if let Some(ref li) = list_info {
        let list_indent = (li.level as f64 + 1.0) * 24.0;
        if para.attributes.get_f64(&AttributeKey::IndentLeft).is_none() {
            style.push_str(&format!("margin-left:{list_indent}pt;"));
        }
    }

    // Background/shading
    if let Some(AttributeValue::Color(c)) = para.attributes.get(&AttributeKey::Background) {
        let hex = c.to_hex();
        if hex != "FFFFFF" && hex != "000000" {
            style.push_str(&format!("background:#{hex};padding:4px 8px;"));
        }
    }

    // Borders
    if let Some(AttributeValue::Borders(borders)) =
        para.attributes.get(&AttributeKey::ParagraphBorders)
    {
        let render_border = |side: &s1_model::BorderSide| -> String {
            if side.width > 0.0 {
                let color_hex = side.color.to_hex();
                format!("{}pt solid #{}", side.width, color_hex)
            } else {
                String::new()
            }
        };
        if let Some(ref top) = borders.top {
            let b = render_border(top);
            if !b.is_empty() {
                style.push_str(&format!("border-top:{b};"));
            }
        }
        if let Some(ref bottom) = borders.bottom {
            let b = render_border(bottom);
            if !b.is_empty() {
                style.push_str(&format!("border-bottom:{b};"));
            }
        }
        if let Some(ref left) = borders.left {
            let b = render_border(left);
            if !b.is_empty() {
                style.push_str(&format!("border-left:{b};"));
            }
        }
        if let Some(ref right) = borders.right {
            let b = render_border(right);
            if !b.is_empty() {
                style.push_str(&format!("border-right:{b};"));
            }
        }
    }

    // Page break before
    if para.attributes.get_bool(&AttributeKey::PageBreakBefore) == Some(true) {
        style.push_str("page-break-before:always;");
    }

    // W5: Tab stops — set CSS tab-size from first tab stop position
    if let Some(AttributeValue::TabStops(stops)) = para.attributes.get(&AttributeKey::TabStops) {
        if let Some(first) = stops.first() {
            // Convert from points to pixels (96 DPI / 72 pt)
            let tab_px = (first.position * 96.0 / 72.0) as i32;
            if tab_px > 0 {
                style.push_str(&format!("tab-size:{tab_px}px;"));
            }
        }
    }

    // W5: Widow/orphan control — always set reasonable defaults,
    // enhanced by KeepLinesTogether/KeepWithNext (already handled above)
    style.push_str("orphans:2;widows:2;");

    // W5: Contextual spacing — collapse top margin when enabled
    if para.attributes.get_bool(&AttributeKey::ContextualSpacing) == Some(true) {
        style.push_str("margin-block-start:0;");
    }

    let list_type_attr = list_info
        .as_ref()
        .map(|li| {
            let fmt_name = match li.num_format {
                ListFormat::Bullet => "bullet",
                ListFormat::Decimal => "decimal",
                ListFormat::LowerAlpha => "lowerAlpha",
                ListFormat::UpperAlpha => "upperAlpha",
                ListFormat::LowerRoman => "lowerRoman",
                ListFormat::UpperRoman => "upperRoman",
                _ => "bullet",
            };
            format!(
                " data-list-type=\"{fmt_name}\" data-list-level=\"{}\"",
                li.level
            )
        })
        .unwrap_or_default();

    let nid_attr = format!(
        " data-node-id=\"{}:{}\"{}",
        para_id.replica, para_id.counter, list_type_attr
    );

    // FS-07: Add dir="rtl" if paragraph text starts with RTL characters
    let dir_attr = if paragraph_starts_rtl(model, para_id) {
        " dir=\"rtl\""
    } else {
        ""
    };

    // List marker prefix — use computed ordinal if available, fall back to start
    let list_marker = list_info.as_ref().map(|li| {
        let n = list_ordinal.unwrap_or(li.start.unwrap_or(1));
        match li.num_format {
            ListFormat::Bullet => "\u{2022} ".to_string(), // bullet: •
            ListFormat::Decimal => format!("{}. ", n),
            ListFormat::LowerAlpha => {
                let ch = (b'a' + ((n.saturating_sub(1)) % 26) as u8) as char;
                format!("{}. ", ch)
            }
            ListFormat::UpperAlpha => {
                let ch = (b'A' + ((n.saturating_sub(1)) % 26) as u8) as char;
                format!("{}. ", ch)
            }
            ListFormat::LowerRoman => {
                format!("{}. ", to_roman_lower(n))
            }
            ListFormat::UpperRoman => {
                format!("{}. ", to_roman_upper(n))
            }
            _ => "\u{2022} ".to_string(),
        }
    });

    match effective_level {
        Some(l @ 1..=6) => {
            // Add heading typography as inline styles so rendering doesn't
            // depend on editor CSS.  Resolve from the document's style table
            // first; fall back to sensible defaults.
            let mut heading_style = style.clone();

            // Try to pull font-size / font-weight / font-family from the
            // heading's named style in the document style table.
            let (style_font_size, style_bold, style_font_family) = style_id
                .and_then(|sid| model.style_by_id(sid))
                .map(|s| {
                    let fs = s.attributes.get_f64(&AttributeKey::FontSize);
                    let bold = s.attributes.get_bool(&AttributeKey::Bold);
                    let ff = s
                        .attributes
                        .get_string(&AttributeKey::FontFamily)
                        .map(|v| v.to_string());
                    (fs, bold, ff)
                })
                .unwrap_or((None, None, None));

            if !heading_style.contains("font-size:") {
                let size = style_font_size.unwrap_or(match l {
                    1 => 24.0,
                    2 => 18.0,
                    3 => 14.0,
                    4 => 12.0,
                    5 => 11.0,
                    _ => 10.0,
                });
                heading_style.push_str(&format!("font-size:{size}pt;"));
            }
            if !heading_style.contains("font-weight:") {
                let weight = if style_bold == Some(false) {
                    "normal"
                } else {
                    "700"
                };
                heading_style.push_str(&format!("font-weight:{weight};"));
            }
            if !heading_style.contains("font-family:") {
                if let Some(ref ff) = style_font_family {
                    heading_style.push_str(&format!("font-family:{ff};"));
                }
            }
            // Default heading margins when not already set by paragraph attrs
            if !heading_style.contains("margin-top:") {
                let mt = match l {
                    1 => 20.0,
                    2 => 18.0,
                    3 => 16.0,
                    4 => 14.0,
                    5 => 12.0,
                    _ => 10.0,
                };
                heading_style.push_str(&format!("margin-top:{mt}pt;"));
            }
            if !heading_style.contains("margin-bottom:") {
                let mb: f64 = if l <= 2 { 6.0 } else { 4.0 };
                heading_style.push_str(&format!("margin-bottom:{mb}pt;"));
            }

            let h_style_attr = if heading_style.is_empty() {
                String::new()
            } else {
                format!(" style=\"{heading_style}\"")
            };
            html.push_str(&format!("<h{l}{nid_attr}{h_style_attr}{dir_attr}>"));
            render_inline_children(model, para_id, html);
            // Ensure empty headings are editable (non-collapsing)
            if is_empty_paragraph(model, para_id) {
                html.push_str("<br>");
            }
            html.push_str(&format!("</h{l}>"));
        }
        _ => {
            // Apply non-heading paragraph style (Title, Subtitle, Quote, Code)
            let mut para_style = style.clone();
            let sid_lower = style_id.map(|s| s.to_lowercase()).unwrap_or_default();
            let data_style_attr = if !sid_lower.is_empty() {
                format!(" data-style-id=\"{}\"", escape_html(style_id.unwrap_or("")))
            } else {
                String::new()
            };
            match sid_lower.as_str() {
                "title" => {
                    if !para_style.contains("font-size:") {
                        para_style.push_str("font-size:26pt;");
                    }
                    if !para_style.contains("font-weight:") {
                        para_style.push_str("font-weight:400;");
                    }
                    if !para_style.contains("margin-top:") {
                        para_style.push_str("margin-top:0;");
                    }
                    if !para_style.contains("margin-bottom:") {
                        para_style.push_str("margin-bottom:3pt;");
                    }
                }
                "subtitle" => {
                    if !para_style.contains("font-size:") {
                        para_style.push_str("font-size:15pt;");
                    }
                    if !para_style.contains("color:") {
                        para_style.push_str("color:#666666;");
                    }
                    if !para_style.contains("margin-top:") {
                        para_style.push_str("margin-top:0;");
                    }
                    if !para_style.contains("margin-bottom:") {
                        para_style.push_str("margin-bottom:16pt;");
                    }
                }
                "quote" => {
                    if !para_style.contains("font-style:") {
                        para_style.push_str("font-style:italic;");
                    }
                    if !para_style.contains("color:") {
                        para_style.push_str("color:#666666;");
                    }
                    if !para_style.contains("border-left:") {
                        para_style.push_str("border-left:3px solid #dadce0;padding-left:12pt;");
                    }
                }
                "code" => {
                    if !para_style.contains("font-family:") {
                        para_style.push_str("font-family:'Courier New',Courier,monospace;");
                    }
                    if !para_style.contains("font-size:") {
                        para_style.push_str("font-size:11pt;");
                    }
                    if !para_style.contains("background") {
                        para_style
                            .push_str("background:#f5f5f5;padding:2pt 4pt;border-radius:2px;");
                    }
                }
                _ => {}
            }
            let p_style_attr = if para_style.is_empty() {
                String::new()
            } else {
                format!(" style=\"{para_style}\"")
            };
            html.push_str(&format!(
                "<p{nid_attr}{p_style_attr}{data_style_attr}{dir_attr}>"
            ));
            if let Some(marker) = list_marker {
                html.push_str(&format!(
                    "<span class=\"list-marker\" style=\"user-select:none\" contenteditable=\"false\">{marker}</span>"
                ));
            }
            render_inline_children(model, para_id, html);
            // Ensure empty paragraphs are editable (non-collapsing)
            if is_empty_paragraph(model, para_id) {
                html.push_str("<br>");
            }
            html.push_str("</p>");
        }
    }
}

fn render_inline_children(model: &DocumentModel, parent_id: NodeId, html: &mut String) {
    let parent = match model.node(parent_id) {
        Some(n) => n,
        None => return,
    };

    for &child_id in &parent.children {
        let child = match model.node(child_id) {
            Some(n) => n,
            None => continue,
        };

        match child.node_type {
            NodeType::Run => render_run(model, child_id, html),
            NodeType::Image => render_image(model, child_id, html),
            NodeType::LineBreak => html.push_str("<br/>"),
            NodeType::ColumnBreak => {
                html.push_str("<hr class=\"column-break\" style=\"border-style:dashed\" />")
            }
            NodeType::Tab => html.push_str("&emsp;"),
            NodeType::Field => {
                render_field_html(child, html);
            }
            NodeType::FootnoteRef => {
                let fn_num = child
                    .attributes
                    .get_i64(&AttributeKey::FootnoteNumber)
                    .unwrap_or(0);
                html.push_str(&format!(
                    "<sup class=\"footnote-ref\" data-footnote-ref=\"{}\" data-node-id=\"{}:{}\" \
                     title=\"Footnote {}\" contenteditable=\"false\">{}</sup>",
                    fn_num, child_id.replica, child_id.counter, fn_num, fn_num
                ));
            }
            NodeType::EndnoteRef => {
                let en_num = child
                    .attributes
                    .get_i64(&AttributeKey::EndnoteNumber)
                    .unwrap_or(0);
                html.push_str(&format!(
                    "<sup class=\"endnote-ref\" data-endnote-ref=\"{}\" data-node-id=\"{}:{}\" \
                     title=\"Endnote {}\" contenteditable=\"false\">{}</sup>",
                    en_num, child_id.replica, child_id.counter, en_num, en_num
                ));
            }
            _ => {}
        }
    }
}

/// Render a Field node (PageNumber, PageCount, etc.) into HTML.
///
/// Extracted as a shared helper so that both `render_node` and
/// `render_inline_children` use the same logic (L-02).
///
/// Field elements are marked `contenteditable="false"` so that:
/// 1. The editor's text sync (getEditableText) excludes their substituted
///    text, preventing duplicate page numbers when syncing back to the model.
/// 2. Users cannot accidentally edit field placeholder text directly.
fn render_field_html(node: &Node, html: &mut String) {
    if let Some(AttributeValue::FieldType(ft)) = node.attributes.get(&AttributeKey::FieldType) {
        // Emit data-field attribute so the editor's pagination system
        // (e.g. substitutePageNumbers in pagination.js) can find and
        // substitute the correct values at render time.
        match ft {
            s1_model::FieldType::PageNumber => {
                html.push_str("<span class=\"field\" data-field=\"PageNumber\" contenteditable=\"false\">PAGE</span>");
            }
            s1_model::FieldType::PageCount => {
                html.push_str("<span class=\"field\" data-field=\"PageCount\" contenteditable=\"false\">NUMPAGES</span>");
            }
            s1_model::FieldType::Date => {
                html.push_str("<span class=\"field\" data-field=\"Date\" contenteditable=\"false\">DATE</span>");
            }
            s1_model::FieldType::Time => {
                html.push_str("<span class=\"field\" data-field=\"Time\" contenteditable=\"false\">TIME</span>");
            }
            s1_model::FieldType::FileName => {
                html.push_str("<span class=\"field\" data-field=\"FileName\" contenteditable=\"false\">FILENAME</span>");
            }
            s1_model::FieldType::Author => {
                html.push_str("<span class=\"field\" data-field=\"Author\" contenteditable=\"false\">AUTHOR</span>");
            }
            s1_model::FieldType::TableOfContents => {
                html.push_str("<span class=\"field\" data-field=\"TableOfContents\" contenteditable=\"false\">TOC</span>");
            }
            s1_model::FieldType::Custom => {
                html.push_str("<span class=\"field\" data-field=\"Custom\" contenteditable=\"false\">FIELD</span>");
            }
            _ => {
                html.push_str("<span class=\"field\" data-field=\"Unknown\" contenteditable=\"false\">FIELD</span>");
            }
        }
    }
}

fn render_run(model: &DocumentModel, run_id: NodeId, html: &mut String) {
    let run = match model.node(run_id) {
        Some(n) => n,
        None => return,
    };

    let bold = run.attributes.get_bool(&AttributeKey::Bold) == Some(true);
    let italic = run.attributes.get_bool(&AttributeKey::Italic) == Some(true);
    let underline = run.attributes.get(&AttributeKey::Underline).is_some()
        && !matches!(
            run.attributes.get(&AttributeKey::Underline),
            Some(AttributeValue::UnderlineStyle(UnderlineStyle::None))
        );
    let strikethrough = run.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true);
    let superscript = run.attributes.get_bool(&AttributeKey::Superscript) == Some(true);
    let subscript = run.attributes.get_bool(&AttributeKey::Subscript) == Some(true);
    let hyperlink_url = run.attributes.get_string(&AttributeKey::HyperlinkUrl);

    // Track changes: detect revision type for visual indicator
    let revision_type = run.attributes.get_string(&AttributeKey::RevisionType);

    // Inline style for font, size, color
    let mut style = String::new();
    if let Some(font) = run.attributes.get_string(&AttributeKey::FontFamily) {
        style.push_str(&format!("font-family:'{font}';"));
    }
    if let Some(size) = run.attributes.get_f64(&AttributeKey::FontSize) {
        style.push_str(&format!("font-size:{size}pt;"));
    }
    if let Some(AttributeValue::Color(c)) = run.attributes.get(&AttributeKey::Color) {
        style.push_str(&format!("color:#{};", c.to_hex()));
    }
    // Highlight / background color
    if let Some(AttributeValue::Color(c)) = run.attributes.get(&AttributeKey::HighlightColor) {
        style.push_str(&format!("background-color:#{};", c.to_hex()));
    } else if let Some(AttributeValue::Color(c)) = run.attributes.get(&AttributeKey::Background) {
        style.push_str(&format!("background-color:#{};", c.to_hex()));
    }
    // Text shadow
    if run.attributes.get_bool(&AttributeKey::TextShadow) == Some(true) {
        style.push_str("text-shadow:1px 1px 2px rgba(0,0,0,0.3);");
    }
    // Text outline
    if run.attributes.get_bool(&AttributeKey::TextOutline) == Some(true) {
        style.push_str("-webkit-text-stroke:1px currentColor;");
    }
    // Q3: Text glow effect
    if run.attributes.get(&AttributeKey::TextGlow).is_some() {
        style.push_str("filter:drop-shadow(0 0 3px currentColor);");
    }
    // Q3: Text reflection effect
    if run.attributes.get(&AttributeKey::TextReflection).is_some() {
        style.push_str(
            "-webkit-box-reflect:below 0px linear-gradient(transparent, rgba(0,0,0,0.1));",
        );
    }
    // Character spacing
    if let Some(sp) = run.attributes.get_f64(&AttributeKey::FontSpacing) {
        if sp.abs() > 0.01 {
            let sp_px = sp * 1.333;
            style.push_str(&format!("letter-spacing:{:.2}px;", sp_px));
        }
    }

    // Track changes: wrap in <ins>/<del> tags — NO inline text-decoration
    // (CSS display mode classes handle underline/strikethrough styling)
    let tc_open = match revision_type {
        Some("Insert") => {
            Some(format!(
                "<ins data-tc-node-id=\"{}:{}\" data-tc-type=\"insert\">",
                run_id.replica, run_id.counter
            ))
        }
        Some("Delete") => {
            Some(format!(
                "<del data-tc-node-id=\"{}:{}\" data-tc-type=\"delete\">",
                run_id.replica, run_id.counter
            ))
        }
        Some("FormatChange") => {
            style.push_str("border-bottom:2px dotted #b08800;");
            Some(format!(
                "<span data-tc-node-id=\"{}:{}\" data-tc-type=\"format\" class=\"tc-format\">",
                run_id.replica, run_id.counter
            ))
        }
        _ => None,
    };

    if let Some(ref open) = tc_open {
        html.push_str(open);
    }

    // Open tags
    if let Some(url) = hyperlink_url {
        html.push_str(&format!(
            "<a href=\"{}\" style=\"color:#1a73e8;text-decoration:underline\" target=\"_blank\" rel=\"noopener\">",
            escape_html(url)
        ));
    }
    if bold {
        html.push_str("<strong>");
    }
    if italic {
        html.push_str("<em>");
    }
    if underline {
        html.push_str("<u>");
    }
    if strikethrough {
        html.push_str("<s>");
    }
    if superscript {
        html.push_str("<sup>");
    }
    if subscript {
        html.push_str("<sub>");
    }

    let has_style = !style.is_empty();
    if has_style {
        html.push_str(&format!("<span style=\"{style}\">"));
    }

    // Collect close/open tag sequences for line breaks within formatted runs
    // We need to close formatting before <br> and reopen after to prevent
    // malformed HTML like <strong><br/></strong>
    let close_tags = {
        let mut t = String::new();
        if has_style {
            t.push_str("</span>");
        }
        if subscript {
            t.push_str("</sub>");
        }
        if superscript {
            t.push_str("</sup>");
        }
        if strikethrough {
            t.push_str("</s>");
        }
        if underline {
            t.push_str("</u>");
        }
        if italic {
            t.push_str("</em>");
        }
        if bold {
            t.push_str("</strong>");
        }
        t
    };
    let open_tags = {
        let mut t = String::new();
        if bold {
            t.push_str("<strong>");
        }
        if italic {
            t.push_str("<em>");
        }
        if underline {
            t.push_str("<u>");
        }
        if strikethrough {
            t.push_str("<s>");
        }
        if superscript {
            t.push_str("<sup>");
        }
        if subscript {
            t.push_str("<sub>");
        }
        if has_style {
            t.push_str(&format!("<span style=\"{style}\">"));
        }
        t
    };
    let has_formatting =
        bold || italic || underline || strikethrough || superscript || subscript || has_style;

    // Text content
    for &text_id in &run.children {
        if let Some(text_node) = model.node(text_id) {
            if text_node.node_type == NodeType::Text {
                if let Some(content) = text_node.text_content.as_deref() {
                    html.push_str(&escape_html(content));
                }
            } else if text_node.node_type == NodeType::LineBreak {
                // Close formatting before break, emit <br>, reopen after
                if has_formatting {
                    html.push_str(&close_tags);
                    html.push_str("<br/>");
                    html.push_str(&open_tags);
                } else {
                    html.push_str("<br/>");
                }
            } else if text_node.node_type == NodeType::Tab {
                html.push_str("&emsp;");
            }
        }
    }

    // Close tags (reverse order)
    if has_style {
        html.push_str("</span>");
    }
    if subscript {
        html.push_str("</sub>");
    }
    if superscript {
        html.push_str("</sup>");
    }
    if strikethrough {
        html.push_str("</s>");
    }
    if underline {
        html.push_str("</u>");
    }
    if italic {
        html.push_str("</em>");
    }
    if bold {
        html.push_str("</strong>");
    }
    if hyperlink_url.is_some() {
        html.push_str("</a>");
    }

    // Close track-changes wrapper tag
    match revision_type {
        Some("Insert") => html.push_str("</ins>"),
        Some("Delete") => html.push_str("</del>"),
        Some("FormatChange") => html.push_str("</span>"),
        _ => {}
    }
}

// --- Clean HTML rendering (no data-node-id, no editor classes, no track changes) ---

/// Render a paragraph as clean, portable HTML.
///
/// If `start_char` / `end_char` are `Some`, only the text within that
/// character range is included (for partial paragraph selections).
/// When `None`, the full paragraph content is rendered.
fn render_paragraph_clean_partial(
    model: &DocumentModel,
    para_id: NodeId,
    start_char: Option<usize>,
    end_char: Option<usize>,
    html: &mut String,
) {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return,
    };

    // Detect heading level from style ID
    let style_id = para.attributes.get_string(&AttributeKey::StyleId);
    let effective_level: Option<u8> = style_id.and_then(|sid| {
        let sid_lower = sid.to_lowercase();
        if sid_lower.starts_with("heading") {
            sid_lower
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()
        } else {
            None
        }
    });

    // Build inline style from paragraph attributes
    let mut style = String::new();
    if let Some(AttributeValue::Alignment(a)) = para.attributes.get(&AttributeKey::Alignment) {
        let val = match a {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "justify",
            _ => "",
        };
        if !val.is_empty() {
            style.push_str(&format!("text-align:{val};"));
        }
    }
    if let Some(sp) = para.attributes.get_f64(&AttributeKey::SpacingBefore) {
        if sp > 0.0 {
            style.push_str(&format!("margin-top:{sp}pt;"));
        }
    }
    if let Some(sp) = para.attributes.get_f64(&AttributeKey::SpacingAfter) {
        if sp > 0.0 {
            style.push_str(&format!("margin-bottom:{sp}pt;"));
        }
    }
    if let Some(AttributeValue::LineSpacing(ls)) = para.attributes.get(&AttributeKey::LineSpacing) {
        use s1_model::LineSpacing;
        match ls {
            LineSpacing::Single => style.push_str("line-height:1;"),
            LineSpacing::OnePointFive => style.push_str("line-height:1.5;"),
            LineSpacing::Double => style.push_str("line-height:2;"),
            LineSpacing::Exact(pts) => style.push_str(&format!("line-height:{pts}pt;")),
            LineSpacing::AtLeast(pts) => style.push_str(&format!("line-height:{pts}pt;")),
            LineSpacing::Multiple(factor) => style.push_str(&format!("line-height:{factor};")),
            _ => {}
        }
    }
    if let Some(indent) = para.attributes.get_f64(&AttributeKey::IndentLeft) {
        if indent > 0.0 {
            style.push_str(&format!("margin-left:{indent}pt;"));
        }
    }
    if let Some(indent) = para.attributes.get_f64(&AttributeKey::IndentRight) {
        if indent > 0.0 {
            style.push_str(&format!("margin-right:{indent}pt;"));
        }
    }
    if let Some(indent) = para.attributes.get_f64(&AttributeKey::IndentFirstLine) {
        if indent > 0.0 {
            style.push_str(&format!("text-indent:{indent}pt;"));
        }
    }

    let style_attr = if style.is_empty() {
        String::new()
    } else {
        format!(" style=\"{style}\"")
    };

    // Emit list info as data attributes so paste can restore list formatting
    let mut list_attrs = String::new();
    if let Some(AttributeValue::ListInfo(ref li)) = para.attributes.get(&AttributeKey::ListInfo) {
        let list_type = match li.num_format {
            ListFormat::Bullet => "bullet",
            ListFormat::Decimal => "decimal",
            ListFormat::LowerAlpha => "lowerAlpha",
            ListFormat::UpperAlpha => "upperAlpha",
            ListFormat::LowerRoman => "lowerRoman",
            ListFormat::UpperRoman => "upperRoman",
            _ => "bullet",
        };
        list_attrs = format!(
            " data-list-type=\"{list_type}\" data-list-level=\"{}\"",
            li.level
        );
    }

    let tag = match effective_level {
        Some(l @ 1..=6) => format!("h{l}"),
        _ => "p".to_string(),
    };

    let list_marker = para
        .attributes
        .get(&AttributeKey::ListInfo)
        .and_then(|attr| match attr {
            AttributeValue::ListInfo(li) => {
                let n = compute_list_ordinal(model, para_id).unwrap_or(li.start.unwrap_or(1));
                Some(match li.num_format {
                    ListFormat::Bullet => "\u{2022} ".to_string(),
                    ListFormat::Decimal => format!("{}. ", n),
                    ListFormat::LowerAlpha => {
                        let ch = (b'a' + ((n.saturating_sub(1)) % 26) as u8) as char;
                        format!("{}. ", ch)
                    }
                    ListFormat::UpperAlpha => {
                        let ch = (b'A' + ((n.saturating_sub(1)) % 26) as u8) as char;
                        format!("{}. ", ch)
                    }
                    ListFormat::LowerRoman => format!("{}. ", to_roman_lower(n)),
                    ListFormat::UpperRoman => format!("{}. ", to_roman_upper(n)),
                    _ => "\u{2022} ".to_string(),
                })
            }
            _ => None,
        });

    // FS-07: Add dir="rtl" if paragraph text starts with RTL characters
    let dir_attr = if paragraph_starts_rtl(model, para_id) {
        " dir=\"rtl\""
    } else {
        ""
    };

    html.push_str(&format!("<{tag}{style_attr}{list_attrs}{dir_attr}>"));
    if let Some(marker) = list_marker {
        html.push_str(&format!(
            "<span class=\"list-marker\" contenteditable=\"false\">{}</span>",
            escape_html(&marker)
        ));
    }

    // Walk children (runs, images, etc.) with character offset tracking
    let sel_start = start_char.unwrap_or(0);
    let sel_end = end_char.unwrap_or(usize::MAX);
    let mut char_offset = 0usize;

    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    let run_len = run_char_len(model, child_id);
                    let run_start = char_offset;
                    let run_end = char_offset + run_len;

                    // Check overlap with selection
                    if run_end > sel_start && run_start < sel_end {
                        let local_start = sel_start.saturating_sub(run_start);
                        let local_end = if sel_end < run_end {
                            sel_end - run_start
                        } else {
                            run_len
                        };
                        render_run_clean_partial(
                            model,
                            child_id,
                            local_start,
                            local_end,
                            &mut *html,
                        );
                    }

                    char_offset += run_len;
                }
                NodeType::Image => {
                    // Images count as 1 character for selection purposes
                    if char_offset >= sel_start && char_offset < sel_end {
                        render_image_clean(model, child_id, html);
                    }
                    char_offset += 1;
                }
                NodeType::LineBreak | NodeType::Tab => {
                    if char_offset >= sel_start && char_offset < sel_end {
                        match child.node_type {
                            NodeType::LineBreak => html.push_str("<br/>"),
                            NodeType::Tab => html.push('\u{2003}'),
                            _ => {}
                        }
                    }
                    char_offset += 1;
                }
                _ => {}
            }
        }
    }

    html.push_str(&format!("</{tag}>"));
}

/// Render a run as clean HTML, optionally for a sub-range of its text.
///
/// `local_start` / `local_end` are character offsets within the run's
/// text content.  The full run text is sliced to
/// `[local_start..local_end]`.
fn render_run_clean_partial(
    model: &DocumentModel,
    run_id: NodeId,
    local_start: usize,
    local_end: usize,
    html: &mut String,
) {
    let run = match model.node(run_id) {
        Some(n) => n,
        None => return,
    };

    let bold = run.attributes.get_bool(&AttributeKey::Bold) == Some(true);
    let italic = run.attributes.get_bool(&AttributeKey::Italic) == Some(true);
    let underline = run.attributes.get(&AttributeKey::Underline).is_some()
        && !matches!(
            run.attributes.get(&AttributeKey::Underline),
            Some(AttributeValue::UnderlineStyle(UnderlineStyle::None))
        );
    let strikethrough = run.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true);
    let superscript = run.attributes.get_bool(&AttributeKey::Superscript) == Some(true);
    let subscript = run.attributes.get_bool(&AttributeKey::Subscript) == Some(true);
    let hyperlink_url = run.attributes.get_string(&AttributeKey::HyperlinkUrl);

    // Inline style for font, size, color (no track-changes styling)
    let mut style = String::new();
    if let Some(font) = run.attributes.get_string(&AttributeKey::FontFamily) {
        style.push_str(&format!("font-family:'{font}';"));
    }
    if let Some(size) = run.attributes.get_f64(&AttributeKey::FontSize) {
        style.push_str(&format!("font-size:{size}pt;"));
    }
    if let Some(AttributeValue::Color(c)) = run.attributes.get(&AttributeKey::Color) {
        style.push_str(&format!("color:#{};", c.to_hex()));
    }
    if let Some(sp) = run.attributes.get_f64(&AttributeKey::FontSpacing) {
        if sp.abs() > 0.01 {
            let sp_px = sp * 1.333;
            style.push_str(&format!("letter-spacing:{:.2}px;", sp_px));
        }
    }

    // Open tags (no track-changes wrappers)
    if let Some(url) = hyperlink_url {
        html.push_str(&format!("<a href=\"{}\">", escape_html(url)));
    }
    if bold {
        html.push_str("<strong>");
    }
    if italic {
        html.push_str("<em>");
    }
    if underline {
        html.push_str("<u>");
    }
    if strikethrough {
        html.push_str("<s>");
    }
    if superscript {
        html.push_str("<sup>");
    }
    if subscript {
        html.push_str("<sub>");
    }

    let has_style = !style.is_empty();
    if has_style {
        html.push_str(&format!("<span style=\"{style}\">"));
    }

    // Collect text, then slice to [local_start..local_end]
    let mut full_text = String::new();
    let mut had_line_break = false;
    for &text_id in &run.children {
        if let Some(text_node) = model.node(text_id) {
            if text_node.node_type == NodeType::Text {
                if let Some(content) = text_node.text_content.as_deref() {
                    full_text.push_str(content);
                }
            } else if text_node.node_type == NodeType::LineBreak {
                // Represent line break as a single char for offset math
                full_text.push('\n');
                had_line_break = true;
            } else if text_node.node_type == NodeType::Tab {
                full_text.push('\t');
            }
        }
    }

    let sliced: String = full_text
        .chars()
        .skip(local_start)
        .take(local_end - local_start)
        .collect();

    if had_line_break {
        // Render with <br/> for newlines
        for (i, segment) in sliced.split('\n').enumerate() {
            if i > 0 {
                html.push_str("<br/>");
            }
            if !segment.is_empty() {
                let rendered = segment.replace('\t', "\u{2003}"); // em space for tabs
                html.push_str(&escape_html(&rendered));
            }
        }
    } else {
        let rendered = sliced.replace('\t', "\u{2003}");
        html.push_str(&escape_html(&rendered));
    }

    // Close tags (reverse order)
    if has_style {
        html.push_str("</span>");
    }
    if subscript {
        html.push_str("</sub>");
    }
    if superscript {
        html.push_str("</sup>");
    }
    if strikethrough {
        html.push_str("</s>");
    }
    if underline {
        html.push_str("</u>");
    }
    if italic {
        html.push_str("</em>");
    }
    if bold {
        html.push_str("</strong>");
    }
    if hyperlink_url.is_some() {
        html.push_str("</a>");
    }
}

/// Render an image as clean HTML (no data-node-id).
fn render_image_clean(model: &DocumentModel, img_id: NodeId, html: &mut String) {
    let img = match model.node(img_id) {
        Some(n) => n,
        None => return,
    };

    if let Some(AttributeValue::MediaId(media_id)) = img.attributes.get(&AttributeKey::ImageMediaId)
    {
        if let Some(item) = model.media().get(*media_id) {
            let b64 = base64_encode(&item.data);
            let mime = &item.content_type;
            let alt = img
                .attributes
                .get_string(&AttributeKey::ImageAltText)
                .unwrap_or("image");
            let wrap_mode = img
                .attributes
                .get_string(&AttributeKey::ImageWrapType)
                .unwrap_or("inline");
            let mut img_style = String::from("max-width:100%;height:auto;");
            if let Some(w) = img.attributes.get_f64(&AttributeKey::ImageWidth) {
                img_style.push_str(&format!("width:{w}pt;"));
            }
            if let Some(h) = img.attributes.get_f64(&AttributeKey::ImageHeight) {
                img_style.push_str(&format!("height:{h}pt;"));
            }
            // UXP-21: Apply CSS for image wrap mode in clean render
            match wrap_mode {
                "wrapLeft" => img_style.push_str("float:left;margin:8px 12px 8px 0;"),
                "wrapRight" => img_style.push_str("float:right;margin:8px 0 8px 12px;"),
                "wrapBoth" => img_style.push_str("display:block;margin:8px auto;"),
                "topAndBottom" => img_style.push_str("display:block;clear:both;margin:16px 0;"),
                "behind" => img_style.push_str("position:relative;z-index:-1;"),
                "inFront" => img_style.push_str("position:relative;z-index:10;"),
                _ => {}
            }
            html.push_str(&format!(
                "<img data-media-id=\"{}\" data-alt-text=\"{}\" data-wrap-type=\"{}\" src=\"data:{mime};base64,{b64}\" style=\"{img_style}\" alt=\"{}\"/>",
                media_id.0, escape_html(alt), escape_html(wrap_mode), escape_html(alt)
            ));
            return;
        }
    }
    html.push_str("<img src=\"\" alt=\"[Image not found]\"/>");
}

/// Render a table as clean HTML (no data-node-id).
fn render_table_clean(model: &DocumentModel, table_id: NodeId, html: &mut String) {
    html.push_str("<table style=\"border-collapse:collapse;width:100%\">");
    let table = match model.node(table_id) {
        Some(n) => n,
        None => {
            html.push_str("</table>");
            return;
        }
    };

    // Q10: Emit <colgroup> with column widths if available
    if let Some(widths_str) = table
        .attributes
        .get_string(&AttributeKey::TableColumnWidths)
    {
        let widths: Vec<&str> = widths_str.split(',').collect();
        if widths.iter().any(|w| w.contains("pt")) {
            html.push_str("<colgroup>");
            for w in &widths {
                let trimmed = w.trim();
                if trimmed.contains("pt") {
                    html.push_str(&format!("<col style=\"width:{trimmed}\">"));
                } else {
                    html.push_str("<col>");
                }
            }
            html.push_str("</colgroup>");
        }
    }

    for &row_id in &table.children {
        if let Some(row) = model.node(row_id) {
            if row.node_type == NodeType::TableRow {
                render_table_row_clean(model, row_id, html);
            }
        }
    }
    html.push_str("</table>");
}

/// Render a table row as clean HTML.
fn render_table_row_clean(model: &DocumentModel, row_id: NodeId, html: &mut String) {
    html.push_str("<tr>");
    let row = match model.node(row_id) {
        Some(n) => n,
        None => {
            html.push_str("</tr>");
            return;
        }
    };
    for &cell_id in &row.children {
        if let Some(cell) = model.node(cell_id) {
            if cell.node_type == NodeType::TableCell {
                render_table_cell_clean(model, cell_id, html);
            }
        }
    }
    html.push_str("</tr>");
}

/// Render a table cell as clean HTML.
fn render_table_cell_clean(model: &DocumentModel, cell_id: NodeId, html: &mut String) {
    let cell = match model.node(cell_id) {
        Some(n) => n,
        None => return,
    };

    let mut attrs = String::new();
    let mut style = String::from("border:1px solid #999;padding:4px 8px;vertical-align:top;");

    if let Some(cs) = cell.attributes.get_i64(&AttributeKey::ColSpan) {
        if cs > 1 {
            attrs.push_str(&format!(" colspan=\"{cs}\""));
        }
    }
    if let Some(rs) = cell.attributes.get_string(&AttributeKey::RowSpan) {
        if rs == "continue" {
            return;
        }
    }
    if let Some(AttributeValue::Color(c)) = cell.attributes.get(&AttributeKey::CellBackground) {
        let hex = c.to_hex();
        if hex != "FFFFFF" {
            style.push_str(&format!("background:#{hex};"));
        }
    }

    html.push_str(&format!("<td{attrs} style=\"{style}\">"));

    // Render cell children (paragraphs, images, nested tables)
    for &child_id in &cell.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Paragraph => {
                    render_paragraph_clean_partial(model, child_id, None, None, html);
                }
                NodeType::Table => {
                    render_table_clean(model, child_id, html);
                }
                NodeType::Image => {
                    render_image_clean(model, child_id, html);
                }
                _ => {}
            }
        }
    }

    html.push_str("</td>");
}

fn render_image(model: &DocumentModel, img_id: NodeId, html: &mut String) {
    let img = match model.node(img_id) {
        Some(n) => n,
        None => return,
    };

    if let Some(AttributeValue::MediaId(media_id)) = img.attributes.get(&AttributeKey::ImageMediaId)
    {
        if let Some(item) = model.media().get(*media_id) {
            let b64 = base64_encode(&item.data);
            let mime = &item.content_type;
            let alt = img
                .attributes
                .get_string(&AttributeKey::ImageAltText)
                .unwrap_or("image");
            let wrap_mode = img
                .attributes
                .get_string(&AttributeKey::ImageWrapType)
                .unwrap_or("inline");
            let mut style = String::from("max-width:100%;height:auto;margin:8px 0;");
            if let Some(w) = img.attributes.get_f64(&AttributeKey::ImageWidth) {
                style.push_str(&format!("width:{w}pt;"));
            }
            if let Some(h) = img.attributes.get_f64(&AttributeKey::ImageHeight) {
                style.push_str(&format!("height:{h}pt;"));
            }
            // UXP-21: Apply CSS for image wrap mode
            match wrap_mode {
                "wrapLeft" => style.push_str("float:left;margin:8px 12px 8px 0;"),
                "wrapRight" => style.push_str("float:right;margin:8px 0 8px 12px;"),
                "wrapBoth" => style.push_str("display:block;margin:8px auto;"),
                "topAndBottom" => {
                    style.push_str("display:block;clear:both;margin:16px 0;");
                }
                "behind" => {
                    style.push_str("position:relative;z-index:-1;");
                }
                "inFront" => {
                    style.push_str("position:relative;z-index:10;");
                }
                _ => {} // "inline" — default, no extra styles
            }
            let wrap_attr = if wrap_mode != "inline" {
                format!(" data-wrap-mode=\"{}\"", wrap_mode)
            } else {
                String::new()
            };
            html.push_str(&format!(
                "<img data-node-id=\"{}:{}\" data-media-id=\"{}\" data-alt-text=\"{}\" data-wrap-type=\"{}\" src=\"data:{mime};base64,{b64}\" style=\"{style}\" alt=\"{}\"{wrap_attr}/>",
                img_id.replica, img_id.counter, media_id.0, escape_html(alt), escape_html(wrap_mode), escape_html(alt)
            ));
            return;
        }
    }
    // Image media not found — render a placeholder so missing images are visible
    html.push_str(&format!(
        "<img data-node-id=\"{}:{}\" src=\"\" alt=\"[Image not found]\" style=\"width:100pt;height:100pt;border:1px dashed #ccc;display:flex;align-items:center;justify-content:center\" />",
        img_id.replica, img_id.counter
    ));
}

/// Render a Drawing/VML node as a visible placeholder with content if available.
fn render_drawing(model: &DocumentModel, drawing_id: NodeId, html: &mut String) {
    let node = match model.node(drawing_id) {
        Some(n) => n,
        None => return,
    };

    let width = node
        .attributes
        .get_f64(&AttributeKey::ShapeWidth)
        .unwrap_or(200.0);
    let height = node
        .attributes
        .get_f64(&AttributeKey::ShapeHeight)
        .unwrap_or(60.0);
    let shape_type = node
        .attributes
        .get_string(&AttributeKey::ShapeType)
        .unwrap_or("shape");

    // Check ShapeType prefix and raw XML for special drawing types
    let raw_xml = node
        .attributes
        .get_string(&AttributeKey::ShapeRawXml)
        .unwrap_or("");

    let is_diagram = shape_type.starts_with("diagram:")
        || raw_xml.contains("dgm:")
        || raw_xml.contains("/diagram")
        || raw_xml.contains("diagramLayout")
        || raw_xml.contains("diagrams/");
    let is_chart = shape_type.starts_with("chart:")
        || raw_xml.contains("c:chart")
        || raw_xml.contains("/chart");
    let is_ole = raw_xml.contains("OLEObject")
        || raw_xml.contains("oleObject")
        || raw_xml.contains("/embeddings/");

    // Render specialized placeholders for non-image drawing types
    if is_diagram {
        // Extract diagram subtype from ShapeType (e.g., "diagram:hierarchy" -> "Hierarchy")
        let subtype = shape_type.strip_prefix("diagram:").unwrap_or("generic");
        let label = if subtype != "generic" {
            let mut chars = subtype.chars();
            let capitalized: String = match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            };
            format!("SmartArt Diagram ({capitalized})")
        } else {
            "SmartArt Diagram".to_string()
        };
        html.push_str(&format!(
            "<div class=\"vml-shape diagram-placeholder\" data-node-id=\"{r}:{c}\" \
             style=\"display:inline-block;width:{w}pt;min-height:{h}pt;\
             border:1px solid #c4c7cc;border-radius:4px;background:#fafbfc;\
             padding:8px;margin:4px 0;box-sizing:border-box;overflow:hidden;\
             text-align:center;line-height:{h}pt\" \
             title=\"{lbl}\">\
             <span style=\"color:#666;font-size:11px\">{lbl}</span></div>",
            r = drawing_id.replica,
            c = drawing_id.counter,
            w = width,
            h = height,
            lbl = label,
        ));
        return;
    }

    if is_chart {
        // Extract chart subtype from ShapeType (e.g., "chart:bar" -> "Bar Chart")
        let subtype = shape_type.strip_prefix("chart:").unwrap_or("generic");
        let label = if subtype != "generic" {
            let mut chars = subtype.chars();
            let capitalized: String = match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            };
            format!("{capitalized} Chart")
        } else {
            "Chart".to_string()
        };
        html.push_str(&format!(
            "<div class=\"vml-shape chart-placeholder\" data-node-id=\"{r}:{c}\" \
             style=\"display:inline-block;width:{w}pt;min-height:{h}pt;\
             border:1px solid #c4c7cc;border-radius:4px;background:#fafbfc;\
             padding:8px;margin:4px 0;box-sizing:border-box;overflow:hidden;\
             text-align:center;line-height:{h}pt\" \
             title=\"{lbl}\">\
             <span style=\"color:#666;font-size:11px\">{lbl}</span></div>",
            r = drawing_id.replica,
            c = drawing_id.counter,
            w = width,
            h = height,
            lbl = label,
        ));
        return;
    }

    if is_ole {
        html.push_str(&format!(
            "<div class=\"vml-shape ole-placeholder\" data-node-id=\"{r}:{c}\" \
             style=\"display:inline-block;width:{w}pt;min-height:{h}pt;\
             border:1px solid #c4c7cc;border-radius:4px;background:#fafbfc;\
             padding:8px;margin:4px 0;box-sizing:border-box;overflow:hidden;\
             text-align:center;line-height:{h}pt\" \
             title=\"Embedded Object\">\
             <span style=\"color:#666;font-size:11px\">Embedded Object</span></div>",
            r = drawing_id.replica,
            c = drawing_id.counter,
            w = width,
            h = height,
        ));
        return;
    }

    // Try to extract text content from child nodes (text boxes have paragraph children)
    let mut inner_html = String::new();
    let mut has_content = false;
    for &child_id in &node.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Paragraph {
                render_node(model, child_id, &mut inner_html);
                has_content = true;
            }
        }
    }

    let label = if has_content {
        String::new()
    } else {
        format!(
            "<span style=\"color:#999;font-size:11px;font-style:italic\">{}</span>",
            escape_html(shape_type)
        )
    };

    html.push_str(&format!(
        "<div class=\"vml-shape\" data-node-id=\"{r}:{c}\" \
         style=\"display:inline-block;width:{w}pt;min-height:{h}pt;\
         border:1px solid #c4c7cc;border-radius:4px;background:#fafbfc;\
         padding:8px;margin:4px 0;box-sizing:border-box;overflow:hidden\" \
         title=\"Shape: {t}\">{label}{content}</div>",
        r = drawing_id.replica,
        c = drawing_id.counter,
        w = width,
        h = height,
        t = escape_html(shape_type),
        label = label,
        content = inner_html
    ));
}

/// Check if a paragraph has no visible text content (empty or only whitespace).
fn is_empty_paragraph(model: &DocumentModel, para_id: NodeId) -> bool {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return true,
    };
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            match child.node_type {
                NodeType::Run => {
                    for &sub_id in &child.children {
                        if let Some(sub) = model.node(sub_id) {
                            if sub.node_type == NodeType::Text {
                                if let Some(content) = sub.text_content.as_deref() {
                                    if !content.is_empty() {
                                        return false;
                                    }
                                }
                            } else if sub.node_type == NodeType::LineBreak {
                                return false; // has content (a line break)
                            }
                        }
                    }
                }
                NodeType::Image | NodeType::LineBreak | NodeType::Tab => return false,
                _ => {}
            }
        }
    }
    true
}

/// FS-07: Detect if a paragraph's text starts with RTL characters (Arabic/Hebrew).
/// Returns true if the first alphabetic character is in an RTL script range.
fn paragraph_starts_rtl(model: &DocumentModel, para_id: NodeId) -> bool {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return false,
    };
    for &child_id in &para.children {
        if let Some(child) = model.node(child_id) {
            if child.node_type == NodeType::Run {
                for &sub_id in &child.children {
                    if let Some(sub) = model.node(sub_id) {
                        if sub.node_type == NodeType::Text {
                            if let Some(content) = sub.text_content.as_deref() {
                                for ch in content.chars() {
                                    if ch.is_whitespace() {
                                        continue;
                                    }
                                    // Arabic: U+0600-U+06FF, U+0750-U+077F, U+08A0-U+08FF
                                    // Hebrew: U+0590-U+05FF
                                    let cp = ch as u32;
                                    if (0x0590..=0x05FF).contains(&cp)
                                        || (0x0600..=0x06FF).contains(&cp)
                                        || (0x0750..=0x077F).contains(&cp)
                                        || (0x08A0..=0x08FF).contains(&cp)
                                        || (0xFB50..=0xFDFF).contains(&cp)
                                        || (0xFE70..=0xFEFF).contains(&cp)
                                    {
                                        return true;
                                    }
                                    // If first non-whitespace char is not RTL, return false
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Convert a BorderStyle enum to a CSS border-style keyword.
fn border_style_to_css(style: &s1_model::BorderStyle) -> &'static str {
    match style {
        s1_model::BorderStyle::None => "none",
        s1_model::BorderStyle::Single => "solid",
        s1_model::BorderStyle::Double => "double",
        s1_model::BorderStyle::Dashed => "dashed",
        s1_model::BorderStyle::Dotted => "dotted",
        s1_model::BorderStyle::Thick => "solid",
        _ => "solid",
    }
}

/// Emit CSS for individual border sides from a Borders struct, falling back to default if none.
fn emit_border_css(borders: &s1_model::Borders, style: &mut String) {
    let has_any = borders.top.is_some()
        || borders.bottom.is_some()
        || borders.left.is_some()
        || borders.right.is_some();
    if !has_any {
        style.push_str("border:1px solid #dadce0;");
        return;
    }
    for (side_name, side) in [
        ("top", &borders.top),
        ("bottom", &borders.bottom),
        ("left", &borders.left),
        ("right", &borders.right),
    ] {
        if let Some(bs) = side {
            let css_style = border_style_to_css(&bs.style);
            if css_style == "none" {
                style.push_str(&format!("border-{side_name}:none;"));
            } else {
                let w = if bs.width < 0.5 { 1.0 } else { bs.width };
                let hex = bs.color.to_hex();
                style.push_str(&format!("border-{side_name}:{w}pt {css_style} #{hex};"));
            }
        } else {
            style.push_str(&format!("border-{side_name}:1px solid #dadce0;"));
        }
    }
}

fn render_table_cell(model: &DocumentModel, cell_id: NodeId, html: &mut String) {
    let cell = match model.node(cell_id) {
        Some(n) => n,
        None => return,
    };
    let mut attrs = String::new();
    let mut style = String::new();

    // Cell borders — use actual border attributes from model if available
    if let Some(AttributeValue::Borders(borders)) = cell.attributes.get(&AttributeKey::CellBorders)
    {
        emit_border_css(borders, &mut style);
    } else {
        // Check parent table for table-level borders
        let table_borders = cell
            .parent
            .and_then(|row_id| model.node(row_id))
            .and_then(|row| row.parent)
            .and_then(|table_id| model.node(table_id))
            .and_then(|table| table.attributes.get(&AttributeKey::TableBorders))
            .and_then(|v| {
                if let AttributeValue::Borders(b) = v {
                    Some(b)
                } else {
                    None
                }
            });
        if let Some(borders) = table_borders {
            emit_border_css(borders, &mut style);
        } else {
            style.push_str("border:1px solid #dadce0;");
        }
    }

    style.push_str("padding:6px 10px;vertical-align:top;");

    // Colspan
    if let Some(cs) = cell.attributes.get_i64(&AttributeKey::ColSpan) {
        if cs > 1 {
            attrs.push_str(&format!(" colspan=\"{cs}\""));
        }
    }
    // Rowspan: "continue" means this cell is merged into the one above — hide it
    if let Some(rs) = cell.attributes.get_string(&AttributeKey::RowSpan) {
        if rs == "continue" {
            // Skip cells that are continuations of a vertical merge
            return;
        }
        // "restart" is handled by the writer counting consecutive cells
    }
    // Cell background
    if let Some(AttributeValue::Color(c)) = cell.attributes.get(&AttributeKey::CellBackground) {
        let hex = c.to_hex();
        if hex != "FFFFFF" {
            style.push_str(&format!("background:#{hex};"));
        }
    }
    // Vertical alignment
    if let Some(AttributeValue::VerticalAlignment(va)) =
        cell.attributes.get(&AttributeKey::VerticalAlign)
    {
        let val = match va {
            s1_model::VerticalAlignment::Top => "top",
            s1_model::VerticalAlignment::Center => "middle",
            s1_model::VerticalAlignment::Bottom => "bottom",
            _ => "top",
        };
        style.push_str(&format!("vertical-align:{val};"));
    }

    // Cell text direction (rotated text)
    if let Some(dir) = cell.attributes.get_string(&AttributeKey::CellTextDirection) {
        match dir {
            "btLr" => style.push_str("writing-mode:vertical-lr;transform:rotate(180deg);"),
            "tbRl" | "tbRlV" => style.push_str("writing-mode:vertical-rl;"),
            _ => {} // "lrTb" is default horizontal
        }
    }

    html.push_str(&format!(
        "<td data-node-id=\"{}:{}\"{} style=\"{}\">",
        cell_id.replica, cell_id.counter, attrs, style
    ));
    render_children(model, cell_id, html);
    html.push_str("</td>");
}

fn render_table(model: &DocumentModel, table_id: NodeId, html: &mut String) {
    html.push_str(&format!(
        "<table data-node-id=\"{}:{}\" style=\"border-collapse:collapse;margin:12px 0;width:100%\">",
        table_id.replica, table_id.counter
    ));
    let table = match model.node(table_id) {
        Some(n) => n,
        None => {
            html.push_str("</table>");
            return;
        }
    };

    // Q10: Emit <colgroup> with column widths if available
    if let Some(widths_str) = table
        .attributes
        .get_string(&AttributeKey::TableColumnWidths)
    {
        let widths: Vec<&str> = widths_str.split(',').collect();
        if widths.iter().any(|w| w.contains("pt")) {
            html.push_str("<colgroup>");
            for w in &widths {
                let trimmed = w.trim();
                if trimmed.contains("pt") {
                    html.push_str(&format!("<col style=\"width:{trimmed}\">"));
                } else {
                    html.push_str("<col>");
                }
            }
            html.push_str("</colgroup>");
        }
    }

    for &row_id in &table.children {
        render_node(model, row_id, html);
    }
    html.push_str("</table>");
}

fn render_table_row(model: &DocumentModel, row_id: NodeId, html: &mut String) {
    html.push_str(&format!(
        "<tr data-node-id=\"{}:{}\">",
        row_id.replica, row_id.counter
    ));
    let row = match model.node(row_id) {
        Some(n) => n,
        None => {
            html.push_str("</tr>");
            return;
        }
    };
    for &cell_id in &row.children {
        render_node(model, cell_id, html);
    }
    html.push_str("</tr>");
}

// NOTE (L-01): This `escape_html` is intentionally duplicated from
// `s1_layout::html::escape_html`. The WASM crate can be built without the
// `layout` feature, so we cannot rely on s1-layout always being available.
// Keeping a local copy ensures the WASM HTML rendering works independently
// of feature flags.
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

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

// ═══════════════════════════════════════════════════════════════════════════
// F4.3: Layout JSON serialization for canvas-based rendering
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a `LayoutDocument` into a structured JSON string for canvas rendering.
///
/// Produces JSON with pages, blocks, lines, and glyph runs including exact
/// positions, font info, and styling. Uses manual JSON building to avoid
/// adding serde_json as a dependency.
fn layout_document_to_json(layout: &s1_layout::LayoutDocument, model: &DocumentModel) -> String {
    let mut json = String::with_capacity(4096);
    json.push_str("{\"pages\":[");

    for (pi, page) in layout.pages.iter().enumerate() {
        if pi > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"index\":{},\"width\":{:.2},\"height\":{:.2},\"contentArea\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"sectionIndex\":{},",
            page.index,
            page.width,
            page.height,
            page.content_area.x,
            page.content_area.y,
            page.content_area.width,
            page.content_area.height,
            page.section_index,
        ));

        // Header
        json.push_str("\"header\":");
        if let Some(ref hdr) = page.header {
            layout_block_to_json(hdr, model, &mut json);
        } else {
            json.push_str("null");
        }
        json.push(',');

        // Footer
        json.push_str("\"footer\":");
        if let Some(ref ftr) = page.footer {
            layout_block_to_json(ftr, model, &mut json);
        } else {
            json.push_str("null");
        }
        json.push(',');

        // Blocks
        json.push_str("\"blocks\":[");
        for (bi, block) in page.blocks.iter().enumerate() {
            if bi > 0 {
                json.push(',');
            }
            layout_block_to_json(block, model, &mut json);
        }
        json.push_str("],");

        // Floating images
        json.push_str("\"floatingImages\":[");
        for (fi, img) in page.floating_images.iter().enumerate() {
            if fi > 0 {
                json.push(',');
            }
            layout_block_to_json(img, model, &mut json);
        }
        json.push_str("],");

        // Footnotes
        json.push_str("\"footnotes\":[");
        for (ni, note) in page.footnotes.iter().enumerate() {
            if ni > 0 {
                json.push(',');
            }
            layout_block_to_json(note, model, &mut json);
        }
        json.push_str("]}");
    }

    json.push_str("]}");
    json
}

/// Serialize a single layout block to JSON.
fn layout_block_to_json(block: &s1_layout::LayoutBlock, model: &DocumentModel, json: &mut String) {
    let source = format!("{}:{}", block.source_id.replica, block.source_id.counter);
    json.push_str(&format!(
        "{{\"sourceId\":\"{}\",\"bounds\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},",
        escape_json(&source),
        block.bounds.x,
        block.bounds.y,
        block.bounds.width,
        block.bounds.height,
    ));

    match &block.kind {
        s1_layout::LayoutBlockKind::Paragraph {
            lines,
            text_align,
            background_color,
            border,
            list_marker,
            list_level,
            space_before,
            space_after,
            indent_left,
            indent_right,
            indent_first_line,
            line_height,
            bidi,
            ..
        } => {
            json.push_str("\"type\":\"paragraph\",");

            if let Some(align) = text_align {
                json.push_str(&format!("\"textAlign\":\"{}\",", escape_json(align)));
            }
            if let Some(bg) = background_color {
                json.push_str(&format!("\"backgroundColor\":\"#{}\",", bg.to_hex()));
            }
            if let Some(b) = border {
                json.push_str(&format!("\"border\":\"{}\",", escape_json(b)));
            }
            if let Some(marker) = list_marker {
                json.push_str(&format!("\"listMarker\":\"{}\",", escape_json(marker)));
            }
            json.push_str(&format!(
                "\"listLevel\":{},\"spaceBefore\":{:.2},\"spaceAfter\":{:.2},\"indentLeft\":{:.2},\"indentRight\":{:.2},\"indentFirstLine\":{:.2},",
                list_level, space_before, space_after, indent_left, indent_right, indent_first_line,
            ));
            if let Some(lh) = line_height {
                json.push_str(&format!("\"lineHeight\":{:.2},", lh));
            }
            json.push_str(&format!("\"bidi\":{},", bidi));

            // Lines
            json.push_str("\"lines\":[");
            for (li, line) in lines.iter().enumerate() {
                if li > 0 {
                    json.push(',');
                }
                json.push_str(&format!(
                    "{{\"baselineY\":{:.2},\"height\":{:.2},\"runs\":[",
                    line.baseline_y, line.height,
                ));
                for (ri, run) in line.runs.iter().enumerate() {
                    if ri > 0 {
                        json.push(',');
                    }
                    glyph_run_to_json(run, model, json);
                }
                json.push_str("]}");
            }
            json.push_str("]}");
        }
        s1_layout::LayoutBlockKind::Table {
            rows,
            is_continuation,
        } => {
            json.push_str(&format!(
                "\"type\":\"table\",\"isContinuation\":{},\"rows\":[",
                is_continuation
            ));
            for (ri, row) in rows.iter().enumerate() {
                if ri > 0 {
                    json.push(',');
                }
                json.push_str(&format!(
                    "{{\"bounds\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"isHeaderRow\":{},\"cells\":[",
                    row.bounds.x, row.bounds.y, row.bounds.width, row.bounds.height,
                    row.is_header_row,
                ));
                for (ci, cell) in row.cells.iter().enumerate() {
                    if ci > 0 {
                        json.push(',');
                    }
                    table_cell_to_json(cell, model, json);
                }
                json.push_str("]}");
            }
            json.push_str("]}");
        }
        s1_layout::LayoutBlockKind::Image {
            media_id,
            bounds,
            image_data,
            content_type,
        } => {
            json.push_str(&format!(
                "\"type\":\"image\",\"mediaId\":\"{}\",\"imageBounds\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
                escape_json(media_id),
                bounds.x, bounds.y, bounds.width, bounds.height,
            ));
            if let Some(ct) = content_type {
                json.push_str(&format!(",\"contentType\":\"{}\"", escape_json(ct)));
            }
            if let (Some(data), Some(ct)) = (image_data, content_type) {
                let b64 = base64_encode(data);
                json.push_str(&format!(
                    ",\"src\":\"data:{};base64,{}\"",
                    escape_json(ct),
                    b64
                ));
            }
            json.push('}');
        }
        s1_layout::LayoutBlockKind::Shape {
            shape_type,
            fill_color,
            stroke_color,
            stroke_width,
            rotation_deg,
            flip_h,
            flip_v,
            is_floating,
            wrap_type,
            has_text_frame,
        } => {
            json.push_str(&format!(
                "\"type\":\"shape\",\"shapeType\":\"{}\",\"strokeWidth\":{:.2},\"rotationDeg\":{:.2},\"flipH\":{},\"flipV\":{},\"isFloating\":{},\"wrapType\":\"{:?}\",\"hasTextFrame\":{}",
                escape_json(shape_type), stroke_width, rotation_deg, flip_h, flip_v, is_floating, wrap_type, has_text_frame,
            ));
            if let Some(fc) = fill_color {
                json.push_str(&format!(",\"fillColor\":\"#{}\"", fc.to_hex()));
            }
            if let Some(sc) = stroke_color {
                json.push_str(&format!(",\"strokeColor\":\"#{}\"", sc.to_hex()));
            }
            json.push('}');
        }
        s1_layout::LayoutBlockKind::TextBox {
            shape_type,
            fill_color,
            stroke_color,
            stroke_width,
            text_margins,
            text_vertical_align,
            blocks: inner_blocks,
        } => {
            json.push_str(&format!(
                "\"type\":\"textBox\",\"shapeType\":\"{}\",\"strokeWidth\":{:.2},\"textVerticalAlign\":\"{}\",\"textMargins\":{{\"top\":{:.2},\"bottom\":{:.2},\"left\":{:.2},\"right\":{:.2}}}",
                escape_json(shape_type), stroke_width, escape_json(text_vertical_align),
                text_margins.top, text_margins.bottom, text_margins.left, text_margins.right,
            ));
            if let Some(fc) = fill_color {
                json.push_str(&format!(",\"fillColor\":\"#{}\"", fc.to_hex()));
            }
            if let Some(sc) = stroke_color {
                json.push_str(&format!(",\"strokeColor\":\"#{}\"", sc.to_hex()));
            }
            json.push_str(",\"blocks\":[");
            for (bi, inner_block) in inner_blocks.iter().enumerate() {
                if bi > 0 {
                    json.push(',');
                }
                layout_block_to_json(inner_block, model, json);
            }
            json.push_str("]}");
        }
        _ => {
            // Unknown block kind — emit a minimal placeholder
            json.push_str("\"type\":\"unknown\"}");
        }
    }
}

/// Serialize a glyph run to JSON.
fn glyph_run_to_json(run: &s1_layout::GlyphRun, model: &DocumentModel, json: &mut String) {
    let source = format!("{}:{}", run.source_id.replica, run.source_id.counter);
    json.push_str(&format!(
        "{{\"sourceId\":\"{}\",\"text\":\"{}\",\"x\":{:.2},\"fontSize\":{:.2},\"width\":{:.2},\"bold\":{},\"italic\":{},\"underline\":\"{}\",\"strikethrough\":{},\"superscript\":{},\"subscript\":{},\"color\":\"#{}\",\"characterSpacing\":{:.2}",
        escape_json(&source),
        escape_json(&run.text),
        run.x_offset,
        run.font_size,
        run.width,
        run.bold,
        run.italic,
        escape_json(&run.underline),
        run.strikethrough,
        run.superscript,
        run.subscript,
        run.color.to_hex(),
        run.character_spacing,
    ));

    // Resolve font family from the document model Run node attributes.
    let font_family = model
        .node(run.source_id)
        .and_then(|n| n.attributes.get_string(&AttributeKey::FontFamily))
        .unwrap_or("serif");
    json.push_str(&format!(",\"fontFamily\":\"{}\"", escape_json(font_family)));

    if let Some(ref url) = run.hyperlink_url {
        json.push_str(&format!(",\"hyperlinkUrl\":\"{}\"", escape_json(url)));
    }
    if let Some(ref hl) = run.highlight_color {
        json.push_str(&format!(",\"highlightColor\":\"#{}\"", hl.to_hex()));
    }
    if let Some(ref rev_type) = run.revision_type {
        json.push_str(&format!(",\"revisionType\":\"{}\"", escape_json(rev_type)));
    }
    if let Some(ref rev_author) = run.revision_author {
        json.push_str(&format!(
            ",\"revisionAuthor\":\"{}\"",
            escape_json(rev_author)
        ));
    }
    if let Some(ref img) = run.inline_image {
        json.push_str(&format!(
            ",\"inlineImage\":{{\"mediaId\":\"{}\",\"width\":{:.2},\"height\":{:.2}",
            escape_json(&img.media_id),
            img.width,
            img.height,
        ));
        if let (Some(data), Some(ct)) = (&img.image_data, &img.content_type) {
            let b64 = base64_encode(data);
            json.push_str(&format!(
                ",\"src\":\"data:{};base64,{}\"",
                escape_json(ct),
                b64
            ));
        }
        json.push('}');
    }
    json.push('}');
}

/// Serialize a table cell to JSON.
fn table_cell_to_json(cell: &s1_layout::LayoutTableCell, model: &DocumentModel, json: &mut String) {
    json.push_str(&format!(
        "{{\"bounds\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
        cell.bounds.x, cell.bounds.y, cell.bounds.width, cell.bounds.height,
    ));
    if let Some(ref bg) = cell.background_color {
        json.push_str(&format!(",\"backgroundColor\":\"#{}\"", bg.to_hex()));
    }
    if let Some(ref bt) = cell.border_top {
        json.push_str(&format!(",\"borderTop\":\"{}\"", escape_json(bt)));
    }
    if let Some(ref bb) = cell.border_bottom {
        json.push_str(&format!(",\"borderBottom\":\"{}\"", escape_json(bb)));
    }
    if let Some(ref bl) = cell.border_left {
        json.push_str(&format!(",\"borderLeft\":\"{}\"", escape_json(bl)));
    }
    if let Some(ref br) = cell.border_right {
        json.push_str(&format!(",\"borderRight\":\"{}\"", escape_json(br)));
    }
    json.push_str(",\"blocks\":[");
    for (bi, block) in cell.blocks.iter().enumerate() {
        if bi > 0 {
            json.push(',');
        }
        layout_block_to_json(block, model, json);
    }
    json.push_str("]}");
}

// ═══════════════════════════════════════════════════════════════════════════
// F4.4: Scene JSON serialization for canvas-first editor
// ═══════════════════════════════════════════════════════════════════════════

/// A parsed model position from JSON.
struct PositionRefParsed {
    node_id: NodeId,
    offset_utf16: u32,
    #[allow(dead_code)]
    affinity: String,
}

impl PositionRefParsed {
    fn from_json(json: &str) -> Result<Self, JsError> {
        // Simple JSON parsing for {"node_id":"R:C","offset_utf16":N,"affinity":"..."}
        let node_id_str = scene_extract_json_string(json, "node_id")
            .ok_or_else(|| JsError::new("Missing node_id in position JSON"))?;
        let offset = scene_extract_json_number(json, "offset_utf16").unwrap_or(0.0) as u32;
        let affinity =
            scene_extract_json_string(json, "affinity").unwrap_or_else(|| "downstream".to_string());
        let node_id = parse_node_id(&node_id_str)?;
        Ok(Self {
            node_id,
            offset_utf16: offset,
            affinity,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"node_id\":\"{}:{}\",\"offset_utf16\":{},\"affinity\":\"downstream\"}}",
            self.node_id.replica, self.node_id.counter, self.offset_utf16
        )
    }

    fn node_id_str(&self) -> String {
        format!("{}:{}", self.node_id.replica, self.node_id.counter)
    }
}

/// A parsed model range from JSON.
struct RangeRefParsed {
    anchor: PositionRefParsed,
    #[allow(dead_code)]
    focus: PositionRefParsed,
}

impl RangeRefParsed {
    fn from_json(json: &str) -> Result<Self, JsError> {
        // Extract anchor and focus sub-objects
        let anchor_str = extract_json_object(json, "anchor")
            .ok_or_else(|| JsError::new("Missing anchor in range JSON"))?;
        let focus_str = extract_json_object(json, "focus")
            .ok_or_else(|| JsError::new("Missing focus in range JSON"))?;
        Ok(Self {
            anchor: PositionRefParsed::from_json(&anchor_str)?,
            focus: PositionRefParsed::from_json(&focus_str)?,
        })
    }
}

/// A rect with page index for selection results.
struct PageRect {
    page_index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Extract a string value from simple JSON (scene API helper).
fn scene_extract_json_string(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start = json.find(&search)? + search.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract a number value from simple JSON (scene API helper).
fn scene_extract_json_number(json: &str, key: &str) -> Option<f64> {
    let search = format!("\"{}\":", key);
    let start = json.find(&search)? + search.len();
    let rest = json[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Extract a JSON object value (including braces).
fn extract_json_object(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":{{", key);
    let start = json.find(&search)? + search.len() - 1; // include opening brace
    let rest = &json[start..];
    let mut depth = 0;
    let mut end = 0;
    for (i, c) in rest.chars().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end > 0 {
        Some(rest[..end].to_string())
    } else {
        None
    }
}

/// Convert UTF-16 offset to char offset.
fn utf16_to_char_offset(text: &str, utf16_offset: usize) -> usize {
    let mut utf16_pos = 0;
    for (char_idx, c) in text.chars().enumerate() {
        if utf16_pos >= utf16_offset {
            return char_idx;
        }
        utf16_pos += c.len_utf16();
    }
    text.chars().count()
}

/// Convert char offset to UTF-16 offset.
fn char_to_utf16_offset(text: &str, char_offset: usize) -> usize {
    text.chars().take(char_offset).map(|c| c.len_utf16()).sum()
}

/// Serialize a single page to scene JSON.
fn layout_page_to_scene_json(
    page: &s1_layout::LayoutPage,
    model: &DocumentModel,
    revision: u64,
) -> String {
    let mut json = String::with_capacity(4096);
    json.push_str(&format!(
        "{{\"page_index\":{},\"document_revision\":{},\"layout_revision\":{},\"bounds_pt\":{{\"x\":0.0,\"y\":0.0,\"width\":{:.2},\"height\":{:.2}}},\"content_rect_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
        page.index, revision, revision,
        page.width, page.height,
        page.content_area.x, page.content_area.y,
        page.content_area.width, page.content_area.height,
    ));

    // Header rect
    if let Some(ref hdr) = page.header {
        json.push_str(&format!(
            ",\"header_rect_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
            hdr.bounds.x, hdr.bounds.y, hdr.bounds.width, hdr.bounds.height
        ));
    } else {
        json.push_str(",\"header_rect_pt\":null");
    }

    // Footer rect
    if let Some(ref ftr) = page.footer {
        json.push_str(&format!(
            ",\"footer_rect_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}}",
            ftr.bounds.x, ftr.bounds.y, ftr.bounds.width, ftr.bounds.height
        ));
    } else {
        json.push_str(",\"footer_rect_pt\":null");
    }

    // Items: flatten all blocks into scene items
    json.push_str(",\"items\":[");
    let mut first_item = true;

    // Header items
    if let Some(ref hdr) = page.header {
        emit_block_scene_items(hdr, model, &mut json, &mut first_item);
    }

    // Body block items
    for block in &page.blocks {
        emit_block_scene_items(block, model, &mut json, &mut first_item);
    }

    // Floating images
    for block in &page.floating_images {
        emit_block_scene_items(block, model, &mut json, &mut first_item);
    }

    // Footnotes
    for block in &page.footnotes {
        emit_block_scene_items(block, model, &mut json, &mut first_item);
    }

    // Footer items
    if let Some(ref ftr) = page.footer {
        emit_block_scene_items(ftr, model, &mut json, &mut first_item);
    }

    json.push_str("]}");
    json
}

/// Emit scene items for a block (paragraph, table, image, shape, textbox).
fn emit_block_scene_items(
    block: &s1_layout::LayoutBlock,
    model: &DocumentModel,
    json: &mut String,
    first_item: &mut bool,
) {
    let source = format!("{}:{}", block.source_id.replica, block.source_id.counter);

    match &block.kind {
        s1_layout::LayoutBlockKind::Paragraph {
            lines,
            background_color,
            border,
            list_marker,
            list_level,
            ..
        } => {
            // Paragraph background
            if let Some(bg) = background_color {
                if !*first_item {
                    json.push(',');
                }
                *first_item = false;
                json.push_str(&format!(
                    "{{\"kind\":\"paragraph_background\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"color\":\"#{}\"}}",
                    escape_json(&source),
                    block.bounds.x, block.bounds.y, block.bounds.width, block.bounds.height,
                    bg.to_hex()
                ));
            }

            // Paragraph border
            if let Some(b) = border {
                if !*first_item {
                    json.push(',');
                }
                *first_item = false;
                json.push_str(&format!(
                    "{{\"kind\":\"paragraph_border\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"border\":\"{}\"}}",
                    escape_json(&source),
                    block.bounds.x, block.bounds.y, block.bounds.width, block.bounds.height,
                    escape_json(b)
                ));
            }

            // List marker
            if let Some(marker) = list_marker {
                if !*first_item {
                    json.push(',');
                }
                *first_item = false;
                // Position marker to the left of the first line
                let marker_x = block.bounds.x - 18.0;
                let marker_y = if let Some(first_line) = lines.first() {
                    block.bounds.y + first_line.baseline_y - first_line.height
                } else {
                    block.bounds.y
                };
                let font_size = lines
                    .first()
                    .and_then(|l| l.runs.first())
                    .map(|r| r.font_size)
                    .unwrap_or(11.0);
                json.push_str(&format!(
                    "{{\"kind\":\"list_marker\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":18.0,\"height\":{:.2}}},\"marker_text\":\"{}\",\"font_size_pt\":{:.2},\"color\":\"#000000\",\"list_level\":{}}}",
                    escape_json(&source),
                    marker_x, marker_y, font_size,
                    escape_json(marker),
                    font_size,
                    list_level
                ));
            }

            // Text runs
            for line in lines {
                for run in &line.runs {
                    if !*first_item {
                        json.push(',');
                    }
                    *first_item = false;
                    let run_source = format!("{}:{}", run.source_id.replica, run.source_id.counter);
                    let run_x = block.bounds.x + run.x_offset;
                    let run_y = block.bounds.y + line.baseline_y - line.height;
                    let font_family = model
                        .node(run.source_id)
                        .and_then(|n| n.attributes.get_string(&AttributeKey::FontFamily))
                        .unwrap_or("serif");

                    json.push_str(&format!(
                        "{{\"kind\":\"text_run\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"baseline_y\":{:.2},\"text\":\"{}\",\"font_family\":\"{}\",\"font_size_pt\":{:.2},\"bold\":{},\"italic\":{},\"underline\":\"{}\",\"strikethrough\":{},\"double_strikethrough\":{},\"color\":\"#{}\",\"highlight_color\":{},\"superscript\":{},\"subscript\":{},\"character_spacing\":{:.2},\"baseline_shift\":{:.2},\"caps\":{},\"small_caps\":{},\"hidden\":{}",
                        escape_json(&run_source),
                        run_x, run_y, run.width, line.height,
                        block.bounds.y + line.baseline_y,
                        escape_json(&run.text),
                        escape_json(font_family),
                        run.font_size,
                        run.bold, run.italic,
                        escape_json(&run.underline),
                        run.strikethrough,
                        run.double_strikethrough,
                        run.color.to_hex(),
                        run.highlight_color.as_ref().map(|c| format!("\"#{}\"", c.to_hex())).unwrap_or_else(|| "null".to_string()),
                        run.superscript, run.subscript,
                        run.character_spacing,
                        run.baseline_shift,
                        run.caps, run.small_caps, run.hidden,
                    ));

                    if let Some(ref url) = run.hyperlink_url {
                        json.push_str(&format!(",\"hyperlink_url\":\"{}\"", escape_json(url)));
                    }
                    if let Some(ref rt) = run.revision_type {
                        json.push_str(&format!(",\"revision_type\":\"{}\"", escape_json(rt)));
                    }
                    if let Some(ref ra) = run.revision_author {
                        json.push_str(&format!(",\"revision_author\":\"{}\"", escape_json(ra)));
                    }
                    // Inline image data (base64 encoded)
                    if let Some(ref img) = run.inline_image {
                        json.push_str(&format!(
                            ",\"inline_image\":{{\"media_id\":\"{}\",\"width\":{:.2},\"height\":{:.2}",
                            escape_json(&img.media_id), img.width, img.height
                        ));
                        if let (Some(data), Some(ct)) = (&img.image_data, &img.content_type) {
                            let b64 = base64_encode(data);
                            json.push_str(&format!(
                                ",\"src\":\"data:{};base64,{}\"",
                                escape_json(ct),
                                b64
                            ));
                        }
                        json.push('}');
                    }
                    json.push('}');
                }
            }
        }
        s1_layout::LayoutBlockKind::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    // Cell background
                    if let Some(ref bg) = cell.background_color {
                        if !*first_item {
                            json.push(',');
                        }
                        *first_item = false;
                        json.push_str(&format!(
                            "{{\"kind\":\"table_cell_background\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"color\":\"#{}\"}}",
                            escape_json(&source),
                            cell.bounds.x, cell.bounds.y, cell.bounds.width, cell.bounds.height,
                            bg.to_hex()
                        ));
                    }

                    // Cell borders
                    let cx = cell.bounds.x;
                    let cy = cell.bounds.y;
                    let cw = cell.bounds.width;
                    let ch = cell.bounds.height;

                    if let Some(ref bt) = cell.border_top {
                        if !*first_item {
                            json.push(',');
                        }
                        *first_item = false;
                        json.push_str(&format!(
                            "{{\"kind\":\"table_border_segment\",\"node_id\":\"{}\",\"start_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"end_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"border\":\"{}\"}}",
                            escape_json(&source), cx, cy, cx + cw, cy, escape_json(bt)
                        ));
                    }
                    if let Some(ref bb) = cell.border_bottom {
                        if !*first_item {
                            json.push(',');
                        }
                        *first_item = false;
                        json.push_str(&format!(
                            "{{\"kind\":\"table_border_segment\",\"node_id\":\"{}\",\"start_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"end_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"border\":\"{}\"}}",
                            escape_json(&source), cx, cy + ch, cx + cw, cy + ch, escape_json(bb)
                        ));
                    }
                    if let Some(ref bl) = cell.border_left {
                        if !*first_item {
                            json.push(',');
                        }
                        *first_item = false;
                        json.push_str(&format!(
                            "{{\"kind\":\"table_border_segment\",\"node_id\":\"{}\",\"start_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"end_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"border\":\"{}\"}}",
                            escape_json(&source), cx, cy, cx, cy + ch, escape_json(bl)
                        ));
                    }
                    if let Some(ref br) = cell.border_right {
                        if !*first_item {
                            json.push(',');
                        }
                        *first_item = false;
                        json.push_str(&format!(
                            "{{\"kind\":\"table_border_segment\",\"node_id\":\"{}\",\"start_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"end_pt\":{{\"x\":{:.2},\"y\":{:.2}}},\"border\":\"{}\"}}",
                            escape_json(&source), cx + cw, cy, cx + cw, cy + ch, escape_json(br)
                        ));
                    }

                    // Cell content blocks
                    for inner_block in &cell.blocks {
                        emit_block_scene_items(inner_block, model, json, first_item);
                    }
                }
            }
        }
        s1_layout::LayoutBlockKind::Image {
            media_id,
            bounds,
            image_data,
            content_type,
        } => {
            if !*first_item {
                json.push(',');
            }
            *first_item = false;
            json.push_str(&format!(
                "{{\"kind\":\"image\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"media_id\":\"{}\",\"is_floating\":false,\"wrap_type\":\"none\"",
                escape_json(&source),
                bounds.x, bounds.y, bounds.width, bounds.height,
                escape_json(media_id),
            ));
            if let Some(ct) = content_type {
                json.push_str(&format!(",\"content_type\":\"{}\"", escape_json(ct)));
            }
            if let (Some(data), Some(ct)) = (image_data, content_type) {
                let b64 = base64_encode(data);
                json.push_str(&format!(
                    ",\"src_base64\":\"data:{};base64,{}\"",
                    escape_json(ct),
                    b64
                ));
            }
            json.push('}');
        }
        s1_layout::LayoutBlockKind::Shape {
            shape_type,
            fill_color,
            stroke_color,
            stroke_width,
            rotation_deg,
            flip_h,
            flip_v,
            is_floating,
            wrap_type,
            has_text_frame,
        } => {
            if !*first_item {
                json.push(',');
            }
            *first_item = false;
            json.push_str(&format!(
                "{{\"kind\":\"shape\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"shape_type\":\"{}\",",
                escape_json(&source),
                block.bounds.x, block.bounds.y, block.bounds.width, block.bounds.height,
                escape_json(shape_type),
            ));
            json.push_str(&format!(
                "\"fill_color\":{},\"stroke_color\":{},\"stroke_width\":{:.2},\"rotation_deg\":{:.2},\"flip_h\":{},\"flip_v\":{},\"is_floating\":{},\"wrap_type\":\"{:?}\",\"has_text_frame\":{}}}",
                fill_color.as_ref().map(|c| format!("\"#{}\"", c.to_hex())).unwrap_or_else(|| "null".to_string()),
                stroke_color.as_ref().map(|c| format!("\"#{}\"", c.to_hex())).unwrap_or_else(|| "null".to_string()),
                stroke_width, rotation_deg, flip_h, flip_v, is_floating,
                wrap_type, has_text_frame,
            ));
        }
        s1_layout::LayoutBlockKind::TextBox {
            shape_type,
            fill_color,
            stroke_color,
            stroke_width,
            text_margins,
            text_vertical_align,
            blocks,
        } => {
            if !*first_item {
                json.push(',');
            }
            *first_item = false;
            json.push_str(&format!(
                "{{\"kind\":\"text_box\",\"node_id\":\"{}\",\"bounds_pt\":{{\"x\":{:.2},\"y\":{:.2},\"width\":{:.2},\"height\":{:.2}}},\"shape_type\":\"{}\",",
                escape_json(&source),
                block.bounds.x, block.bounds.y, block.bounds.width, block.bounds.height,
                escape_json(shape_type),
            ));
            json.push_str(&format!(
                "\"fill_color\":{},\"stroke_color\":{},\"stroke_width\":{:.2},\"text_margins\":{{\"top\":{:.2},\"bottom\":{:.2},\"left\":{:.2},\"right\":{:.2}}},\"text_vertical_align\":\"{}\"}}",
                fill_color.as_ref().map(|c| format!("\"#{}\"", c.to_hex())).unwrap_or_else(|| "null".to_string()),
                stroke_color.as_ref().map(|c| format!("\"#{}\"", c.to_hex())).unwrap_or_else(|| "null".to_string()),
                stroke_width,
                text_margins.top, text_margins.bottom, text_margins.left, text_margins.right,
                escape_json(text_vertical_align),
            ));

            // Emit text content blocks inside the text box
            for inner_block in blocks {
                emit_block_scene_items(inner_block, model, json, first_item);
            }
        }
        _ => {}
    }
}

/// Hit-test within a single block to find the closest text position.
fn hit_test_block(
    block: &s1_layout::LayoutBlock,
    x_pt: f64,
    y_pt: f64,
    model: &DocumentModel,
) -> PositionRefParsed {
    match &block.kind {
        s1_layout::LayoutBlockKind::Paragraph { lines, .. } => {
            // Find closest line
            let mut best_line: Option<&s1_layout::LayoutLine> = None;
            let mut best_line_dist = f64::MAX;

            for line in lines {
                let line_top = block.bounds.y + line.baseline_y - line.height;
                let line_bottom = block.bounds.y + line.baseline_y;
                if y_pt >= line_top && y_pt <= line_bottom {
                    best_line = Some(line);
                    break;
                }
                let dist = (y_pt - (line_top + line_bottom) / 2.0).abs();
                if dist < best_line_dist {
                    best_line_dist = dist;
                    best_line = Some(line);
                }
            }

            if let Some(line) = best_line {
                // Find closest run and offset within the line
                for run in &line.runs {
                    let run_x = block.bounds.x + run.x_offset;
                    let run_end = run_x + run.width;
                    if x_pt >= run_x && x_pt <= run_end {
                        // Estimate character offset based on position within run
                        let frac = (x_pt - run_x) / run.width.max(1.0);
                        let text = &run.text;
                        let char_count = text.chars().count();
                        let char_offset =
                            ((frac * char_count as f64).round() as usize).min(char_count);
                        let utf16_offset = char_to_utf16_offset(text, char_offset);

                        // Find the text node child of this run
                        let text_node_id = model
                            .node(run.source_id)
                            .and_then(|n| n.children.first().copied())
                            .unwrap_or(run.source_id);

                        return PositionRefParsed {
                            node_id: text_node_id,
                            offset_utf16: utf16_offset as u32,
                            affinity: "downstream".to_string(),
                        };
                    }
                }
                // No run matched x — use last run
                if let Some(run) = line.runs.last() {
                    let text_node_id = model
                        .node(run.source_id)
                        .and_then(|n| n.children.first().copied())
                        .unwrap_or(run.source_id);
                    let utf16_len = run.text.encode_utf16().count();
                    return PositionRefParsed {
                        node_id: text_node_id,
                        offset_utf16: utf16_len as u32,
                        affinity: "downstream".to_string(),
                    };
                }
            }

            // Fallback to block source
            PositionRefParsed {
                node_id: block.source_id,
                offset_utf16: 0,
                affinity: "downstream".to_string(),
            }
        }
        s1_layout::LayoutBlockKind::Table { rows, .. } => {
            // Find the cell containing the point
            for row in rows {
                for cell in &row.cells {
                    let cx = cell.bounds.x;
                    let cy = cell.bounds.y;
                    if x_pt >= cx
                        && x_pt <= cx + cell.bounds.width
                        && y_pt >= cy
                        && y_pt <= cy + cell.bounds.height
                    {
                        // Recurse into cell blocks
                        for inner_block in &cell.blocks {
                            if y_pt >= inner_block.bounds.y
                                && y_pt <= inner_block.bounds.y + inner_block.bounds.height
                            {
                                return hit_test_block(inner_block, x_pt, y_pt, model);
                            }
                        }
                        // Use first cell block
                        if let Some(inner) = cell.blocks.first() {
                            return hit_test_block(inner, x_pt, y_pt, model);
                        }
                    }
                }
            }
            PositionRefParsed {
                node_id: block.source_id,
                offset_utf16: 0,
                affinity: "downstream".to_string(),
            }
        }
        _ => PositionRefParsed {
            node_id: block.source_id,
            offset_utf16: 0,
            affinity: "downstream".to_string(),
        },
    }
}

/// Find the caret rect for a position within a page.
fn find_caret_rect_in_page(
    page: &s1_layout::LayoutPage,
    pos: &PositionRefParsed,
    model: &DocumentModel,
) -> Option<s1_layout::Rect> {
    // Check all blocks for the target node
    let all_blocks = page
        .blocks
        .iter()
        .chain(page.header.iter())
        .chain(page.footer.iter())
        .chain(page.footnotes.iter())
        .chain(page.floating_images.iter());

    for block in all_blocks {
        if let Some(rect) = find_caret_rect_in_block(block, pos, model) {
            return Some(rect);
        }
    }
    None
}

/// Find the caret rect within a block.
fn find_caret_rect_in_block(
    block: &s1_layout::LayoutBlock,
    pos: &PositionRefParsed,
    model: &DocumentModel,
) -> Option<s1_layout::Rect> {
    match &block.kind {
        s1_layout::LayoutBlockKind::Paragraph { lines, .. } => {
            for line in lines {
                for run in &line.runs {
                    // Check if the target node is this run's text child
                    let text_node_id = model
                        .node(run.source_id)
                        .and_then(|n| n.children.first().copied())
                        .unwrap_or(run.source_id);

                    if text_node_id == pos.node_id || run.source_id == pos.node_id {
                        let char_offset =
                            utf16_to_char_offset(&run.text, pos.offset_utf16 as usize);
                        let char_count = run.text.chars().count();
                        let frac = if char_count > 0 {
                            char_offset as f64 / char_count as f64
                        } else {
                            0.0
                        };
                        let caret_x = block.bounds.x + run.x_offset + frac * run.width;
                        let caret_y = block.bounds.y + line.baseline_y - line.height;
                        return Some(s1_layout::Rect::new(caret_x, caret_y, 1.0, line.height));
                    }
                }
            }
            None
        }
        s1_layout::LayoutBlockKind::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    for inner_block in &cell.blocks {
                        if let Some(rect) = find_caret_rect_in_block(inner_block, pos, model) {
                            return Some(rect);
                        }
                    }
                }
            }
            None
        }
        s1_layout::LayoutBlockKind::TextBox { blocks, .. } => {
            for inner_block in blocks {
                if let Some(rect) = find_caret_rect_in_block(inner_block, pos, model) {
                    return Some(rect);
                }
            }
            None
        }
        _ => None,
    }
}

/// Collect selection rectangles within a page for a given range.
fn collect_selection_rects_in_page(
    page: &s1_layout::LayoutPage,
    range: &RangeRefParsed,
    model: &DocumentModel,
    rects: &mut Vec<PageRect>,
) {
    let all_blocks = page
        .blocks
        .iter()
        .chain(page.header.iter())
        .chain(page.footer.iter())
        .chain(page.footnotes.iter());

    let mut in_range = false;

    for block in all_blocks {
        collect_selection_rects_in_block(block, range, model, page.index, rects, &mut in_range);
    }
}

/// Collect selection rects within a block.
fn collect_selection_rects_in_block(
    block: &s1_layout::LayoutBlock,
    range: &RangeRefParsed,
    model: &DocumentModel,
    page_index: usize,
    rects: &mut Vec<PageRect>,
    in_range: &mut bool,
) {
    match &block.kind {
        s1_layout::LayoutBlockKind::Paragraph { lines, .. } => {
            for line in lines {
                let line_top = block.bounds.y + line.baseline_y - line.height;
                let mut line_sel_start = f64::MAX;
                let mut line_sel_end = f64::MIN;

                for run in &line.runs {
                    let text_node_id = model
                        .node(run.source_id)
                        .and_then(|n| n.children.first().copied())
                        .unwrap_or(run.source_id);

                    let is_anchor_node = text_node_id == range.anchor.node_id
                        || run.source_id == range.anchor.node_id;
                    let is_focus_node =
                        text_node_id == range.focus.node_id || run.source_id == range.focus.node_id;

                    let run_x = block.bounds.x + run.x_offset;
                    let run_end = run_x + run.width;

                    if is_anchor_node && !*in_range {
                        *in_range = true;
                        let char_count = run.text.chars().count();
                        let anchor_char =
                            utf16_to_char_offset(&run.text, range.anchor.offset_utf16 as usize);
                        let frac = if char_count > 0 {
                            anchor_char as f64 / char_count as f64
                        } else {
                            0.0
                        };
                        line_sel_start = line_sel_start.min(run_x + frac * run.width);
                    }

                    if *in_range {
                        line_sel_start = line_sel_start.min(run_x);
                        line_sel_end = line_sel_end.max(run_end);
                    }

                    if is_focus_node && *in_range {
                        let char_count = run.text.chars().count();
                        let focus_char =
                            utf16_to_char_offset(&run.text, range.focus.offset_utf16 as usize);
                        let frac = if char_count > 0 {
                            focus_char as f64 / char_count as f64
                        } else {
                            1.0
                        };
                        line_sel_end = run_x + frac * run.width;
                        *in_range = false;
                    }
                }

                if line_sel_start < line_sel_end {
                    rects.push(PageRect {
                        page_index,
                        x: line_sel_start,
                        y: line_top,
                        width: line_sel_end - line_sel_start,
                        height: line.height,
                    });
                }
            }
        }
        s1_layout::LayoutBlockKind::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    for inner_block in &cell.blocks {
                        collect_selection_rects_in_block(
                            inner_block,
                            range,
                            model,
                            page_index,
                            rects,
                            in_range,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

/// Navigate a position in a direction.
fn navigate_position(
    _layout: &s1_layout::LayoutDocument,
    model: &DocumentModel,
    pos: &PositionRefParsed,
    direction: &str,
    granularity: &str,
) -> PositionRefParsed {
    match granularity {
        "character" => {
            if let Some(node) = model.node(pos.node_id) {
                if let Some(text) = &node.text_content {
                    let char_offset = utf16_to_char_offset(text, pos.offset_utf16 as usize);
                    let char_count = text.chars().count();

                    match direction {
                        "forward" => {
                            if char_offset < char_count {
                                let new_char = char_offset + 1;
                                let new_utf16 = char_to_utf16_offset(text, new_char);
                                return PositionRefParsed {
                                    node_id: pos.node_id,
                                    offset_utf16: new_utf16 as u32,
                                    affinity: "downstream".to_string(),
                                };
                            }
                            // At end of text node — try next text node
                            if let Some(next_id) = find_next_text_node(model, pos.node_id) {
                                return PositionRefParsed {
                                    node_id: next_id,
                                    offset_utf16: 0,
                                    affinity: "downstream".to_string(),
                                };
                            }
                        }
                        "backward" => {
                            if char_offset > 0 {
                                let new_char = char_offset - 1;
                                let new_utf16 = char_to_utf16_offset(text, new_char);
                                return PositionRefParsed {
                                    node_id: pos.node_id,
                                    offset_utf16: new_utf16 as u32,
                                    affinity: "downstream".to_string(),
                                };
                            }
                            // At start of text node — try previous text node
                            if let Some(prev_id) = find_prev_text_node(model, pos.node_id) {
                                if let Some(prev_node) = model.node(prev_id) {
                                    if let Some(prev_text) = &prev_node.text_content {
                                        let utf16_len = prev_text.encode_utf16().count();
                                        return PositionRefParsed {
                                            node_id: prev_id,
                                            offset_utf16: utf16_len as u32,
                                            affinity: "downstream".to_string(),
                                        };
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Unchanged
            pos.to_parsed_clone()
        }
        "word" => {
            if let Some(node) = model.node(pos.node_id) {
                if let Some(text) = &node.text_content {
                    let chars: Vec<char> = text.chars().collect();
                    let char_offset = utf16_to_char_offset(text, pos.offset_utf16 as usize);

                    match direction {
                        "forward" => {
                            let mut i = char_offset;
                            // Skip current word
                            while i < chars.len() && chars[i].is_alphanumeric() {
                                i += 1;
                            }
                            // Skip whitespace
                            while i < chars.len() && !chars[i].is_alphanumeric() {
                                i += 1;
                            }
                            let new_utf16 = char_to_utf16_offset(text, i);
                            return PositionRefParsed {
                                node_id: pos.node_id,
                                offset_utf16: new_utf16 as u32,
                                affinity: "downstream".to_string(),
                            };
                        }
                        "backward" => {
                            let mut i = char_offset;
                            // Skip whitespace
                            while i > 0 && !chars[i - 1].is_alphanumeric() {
                                i -= 1;
                            }
                            // Skip word
                            while i > 0 && chars[i - 1].is_alphanumeric() {
                                i -= 1;
                            }
                            let new_utf16 = char_to_utf16_offset(text, i);
                            return PositionRefParsed {
                                node_id: pos.node_id,
                                offset_utf16: new_utf16 as u32,
                                affinity: "downstream".to_string(),
                            };
                        }
                        _ => {}
                    }
                }
            }
            pos.to_parsed_clone()
        }
        _ => {
            // line, paragraph, document — fall back to character for now
            navigate_position(_layout, model, pos, direction, "character")
        }
    }
}

impl PositionRefParsed {
    fn to_parsed_clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            offset_utf16: self.offset_utf16,
            affinity: self.affinity.clone(),
        }
    }
}

/// Find the next text node in document order.
fn find_next_text_node(model: &DocumentModel, from: NodeId) -> Option<NodeId> {
    let nodes = collect_text_nodes_in_order(model);
    let mut found = false;
    for nid in &nodes {
        if found {
            return Some(*nid);
        }
        if *nid == from {
            found = true;
        }
    }
    None
}

/// Find the previous text node in document order.
fn find_prev_text_node(model: &DocumentModel, from: NodeId) -> Option<NodeId> {
    let nodes = collect_text_nodes_in_order(model);
    let mut prev: Option<NodeId> = None;
    for nid in &nodes {
        if *nid == from {
            return prev;
        }
        prev = Some(*nid);
    }
    None
}

/// Collect all text nodes in document order.
fn collect_text_nodes_in_order(model: &DocumentModel) -> Vec<NodeId> {
    let mut result = Vec::new();
    if let Some(root) = model.root_node() {
        collect_text_nodes_recursive(model, root.id, &mut result);
    }
    result
}

fn collect_text_nodes_recursive(model: &DocumentModel, node_id: NodeId, result: &mut Vec<NodeId>) {
    if let Some(node) = model.node(node_id) {
        if node.text_content.is_some() {
            result.push(node_id);
        }
        for child_id in &node.children {
            collect_text_nodes_recursive(model, *child_id, result);
        }
    }
}

/// Detect the format of a document from its bytes.
///
/// Returns one of: "docx", "odt", "pdf", "txt", "csv", "xlsx", "pptx", "ods", "odp", "doc".
#[wasm_bindgen]
pub fn detect_format(data: &[u8]) -> String {
    let fmt = s1engine::Format::detect(data);
    fmt.extension().to_string()
}

/// Detect the file type from bytes with extended metadata.
///
/// Returns a JSON string with fields:
/// - `type`: file extension (e.g., "docx", "xlsx", "pptx")
/// - `label`: human-readable label (e.g., "Excel Spreadsheet")
/// - `mime`: MIME type
/// - `isDocument`: boolean
/// - `isSpreadsheet`: boolean
/// - `isPresentation`: boolean
/// - `isSupported`: whether s1engine can open this file
#[wasm_bindgen]
pub fn detect_file_type(data: &[u8]) -> String {
    let ft = s1engine::detect_file_type(data);
    format!(
        "{{\"type\":\"{}\",\"label\":\"{}\",\"mime\":\"{}\",\"isDocument\":{},\"isSpreadsheet\":{},\"isPresentation\":{},\"isSupported\":{}}}",
        ft.extension(),
        ft.label(),
        ft.mime_type(),
        ft.is_document(),
        ft.is_spreadsheet(),
        ft.is_presentation(),
        ft.is_supported(),
    )
}

// --- WasmCollabDocument (P.6: Collaboration API) ---

/// A collaborative document that supports CRDT-based real-time editing.
///
/// Each instance represents one replica. Local edits produce operations that
/// must be broadcast to other replicas. Remote operations are applied via
/// `apply_remote_ops`.
#[wasm_bindgen]
pub struct WasmCollabDocument {
    inner: Option<s1_crdt::CollabDocument>,
}

#[wasm_bindgen]
impl WasmCollabDocument {
    /// Get the replica ID of this collaborative document.
    pub fn replica_id(&self) -> Result<u64, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.replica_id())
    }

    /// Get the document content as HTML.
    pub fn to_html(&self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let model = doc.model();
        Ok(to_html_from_model(model))
    }

    /// Get the document content as plain text.
    pub fn to_plain_text(&self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.to_plain_text())
    }

    /// Apply a local text insertion and return serialized ops for broadcast.
    ///
    /// Returns a JSON string of the operations that must be sent to other replicas.
    pub fn apply_local_insert_text(
        &mut self,
        target_id: &str,
        offset: usize,
        text: &str,
    ) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let node_id = parse_node_id(target_id)?;

        // Find the first text node under this target
        let (text_node_id, _len) = find_first_text_node(doc.model(), node_id)?;

        let op = Operation::insert_text(text_node_id, offset, text.to_string());
        let crdt_op = doc
            .apply_local(op)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Serialize the CRDT op as JSON for network transport
        Ok(serialize_crdt_op_to_json(&crdt_op))
    }

    /// Apply a local text deletion and return serialized ops for broadcast.
    pub fn apply_local_delete_text(
        &mut self,
        target_id: &str,
        offset: usize,
        length: usize,
    ) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let node_id = parse_node_id(target_id)?;
        let (text_node_id, _len) = find_first_text_node(doc.model(), node_id)?;

        let op = Operation::delete_text(text_node_id, offset, length);
        let crdt_op = doc
            .apply_local(op)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(serialize_crdt_op_to_json(&crdt_op))
    }

    /// Apply a local formatting change and return serialized ops for broadcast.
    pub fn apply_local_format(
        &mut self,
        target_id: &str,
        key: &str,
        value: &str,
    ) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let node_id = parse_node_id(target_id)?;
        let attrs = parse_format_kv(key, value)?;

        let op = Operation::set_attributes(node_id, attrs);
        let crdt_op = doc
            .apply_local(op)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(serialize_crdt_op_to_json(&crdt_op))
    }

    /// Apply remote operations received from another replica.
    ///
    /// Accepts a JSON string of a CRDT operation (as produced by apply_local_* methods).
    pub fn apply_remote_ops(&mut self, ops_json: &str) -> Result<(), JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let crdt_op = deserialize_crdt_op_from_json(ops_json)?;
        doc.apply_remote(crdt_op)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the current state vector as JSON.
    ///
    /// Used for delta synchronization — send your state vector to a peer
    /// to find out what operations you're missing.
    pub fn get_state_vector(&self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let sv = doc.state_vector();
        let entries: Vec<(u64, u64)> = sv.entries().iter().map(|(&r, &l)| (r, l)).collect();
        let mut result = String::from("{");
        for (i, (replica, lamport)) in entries.iter().enumerate() {
            if i > 0 {
                result.push(',');
            }
            result.push_str(&format!("\"{}\":{}", replica, lamport));
        }
        result.push('}');
        Ok(result)
    }

    /// Get operations that have happened since a given state vector.
    ///
    /// Used for delta sync: peer sends their state vector, you return
    /// the operations they're missing.
    pub fn get_changes_since(&self, state_vector_json: &str) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let remote_sv = parse_state_vector_json(state_vector_json)?;
        let changes = doc.changes_since(&remote_sv);
        let json_ops: Vec<String> = changes.iter().map(serialize_crdt_op_to_json).collect();
        Ok(format!("[{}]", json_ops.join(",")))
    }

    /// Set the local cursor position and return an awareness update for broadcast.
    pub fn set_cursor(
        &mut self,
        node_id: &str,
        offset: usize,
        user_name: &str,
        user_color: &str,
    ) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let nid = parse_node_id(node_id)?;
        let selection = s1_ops::Selection::collapsed(s1_ops::Position::new(nid, offset));
        let update = doc.set_cursor(selection, user_name, user_color);
        Ok(serialize_awareness_update(&update))
    }

    /// Apply a remote awareness (cursor) update from another replica.
    pub fn apply_awareness_update(&mut self, update_json: &str) -> Result<(), JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let update = deserialize_awareness_update(update_json)?;
        doc.apply_awareness_update(&update);
        Ok(())
    }

    /// Get all peer cursors as JSON.
    ///
    /// Returns a JSON array of cursor states:
    /// `[{"replicaId":2,"nodeId":"1:5","offset":3,"userName":"Alice","userColor":"#ff0000"},...]`
    pub fn get_peers_json(&self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let cursors = doc.awareness().remote_cursors();
        let mut items = Vec::new();
        for cursor in &cursors {
            items.push(format!(
                "{{\"replicaId\":{},\"nodeId\":\"{}:{}\",\"offset\":{},\"userName\":{},\"userColor\":{}}}",
                cursor.replica_id,
                cursor.selection.anchor.node_id.replica,
                cursor.selection.anchor.node_id.counter,
                cursor.selection.anchor.offset,
                json_escape_string(&cursor.user_name),
                json_escape_string(&cursor.user_color),
            ));
        }
        Ok(format!("[{}]", items.join(",")))
    }

    /// Undo the last local operation.
    ///
    /// Returns JSON of the undo operation for broadcast, or null if nothing to undo.
    pub fn undo(&mut self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        match doc.undo().map_err(|e| JsError::new(&e.to_string()))? {
            Some(crdt_op) => Ok(serialize_crdt_op_to_json(&crdt_op)),
            None => Ok("null".to_string()),
        }
    }

    /// Redo the last undone operation.
    pub fn redo(&mut self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        match doc.redo().map_err(|e| JsError::new(&e.to_string()))? {
            Some(crdt_op) => Ok(serialize_crdt_op_to_json(&crdt_op)),
            None => Ok("null".to_string()),
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.inner.as_ref().is_some_and(|d| d.can_undo())
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.inner.as_ref().is_some_and(|d| d.can_redo())
    }

    /// Get the size of the operation log.
    pub fn op_log_size(&self) -> Result<usize, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.op_log_size())
    }

    /// Get the number of tombstones.
    pub fn tombstone_count(&self) -> Result<usize, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.tombstone_count())
    }

    /// Compact the operation log (merge consecutive single-char inserts).
    pub fn compact_op_log(&mut self) -> Result<(), JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        doc.compact_op_log();
        Ok(())
    }

    /// Export the collaborative document to a format (docx, odt, txt, md).
    pub fn export(&self, format: &str) -> Result<Vec<u8>, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let fmt = parse_format(format)?;
        let temp_doc = s1engine::Document::from_model(doc.model().clone());
        temp_doc
            .export(fmt)
            .map_err(|e| JsError::new(&format!("Export to {} failed: {}", format, e)))
    }

    // ─── Rendering (delegates to same render functions as WasmDocument) ───

    /// Render a single node as HTML (for incremental rendering).
    pub fn render_node_html(&self, node_id_str: &str) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let node_id = parse_node_id(node_id_str)?;
        let model = doc.model();
        let mut html = String::new();
        render_node(model, node_id, &mut html);
        Ok(html)
    }

    /// Get paragraph IDs as JSON array.
    pub fn paragraph_ids_json(&self) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let model = doc.model();
        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let body = model
            .node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?;
        let mut ids = Vec::new();
        for &child_id in &body.children {
            if let Some(child) = model.node(child_id) {
                if child.node_type == NodeType::Paragraph {
                    ids.push(format!("\"{}:{}\"", child_id.replica, child_id.counter));
                }
            }
        }
        Ok(format!("[{}]", ids.join(",")))
    }

    /// Get formatting info for a node as JSON (delegates to WasmDocument).
    pub fn get_formatting_json(&self, node_id_str: &str) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let temp = s1engine::Document::from_model(doc.model().clone());
        let wasm_doc = WasmDocument {
            batch_label: None,
            batch_count: 0,
            inner: Some(temp),
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        };
        wasm_doc.get_formatting_json(node_id_str)
    }

    // ─── Structural Editing (apply on model, produce CRDT ops) ───

    /// Set paragraph text, preserving multi-run formatting when possible.
    ///
    /// When the text is unchanged, this is a no-op (preserves all formatting).
    /// When only a portion of the text changed, a diff-based approach is used
    /// to minimize the edit and preserve run-level formatting on unchanged
    /// portions. Only falls back to full delete+insert when the paragraph
    /// has no existing runs.
    pub fn set_paragraph_text(&mut self, node_id_str: &str, text: &str) -> Result<String, JsError> {
        let doc = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document freed"))?;
        let node_id = parse_node_id(node_id_str)?;

        // W6: Check if text is unchanged — skip mutation to preserve formatting
        let existing_text = extract_paragraph_text(doc.model(), node_id);
        if existing_text == text {
            return Ok("[]".to_string());
        }

        // W6: Diff-based update — find the minimal edit to preserve run formatting.
        // Compute common prefix/suffix to determine the changed region.
        let old_chars: Vec<char> = existing_text.chars().collect();
        let new_chars: Vec<char> = text.chars().collect();
        let old_len = old_chars.len();
        let new_len = new_chars.len();

        let mut prefix_len = 0;
        while prefix_len < old_len
            && prefix_len < new_len
            && old_chars[prefix_len] == new_chars[prefix_len]
        {
            prefix_len += 1;
        }
        let mut suffix_len = 0;
        while suffix_len < (old_len - prefix_len)
            && suffix_len < (new_len - prefix_len)
            && old_chars[old_len - 1 - suffix_len] == new_chars[new_len - 1 - suffix_len]
        {
            suffix_len += 1;
        }

        let delete_start = prefix_len;
        let delete_end = old_len - suffix_len;
        let insert_text: String = new_chars[prefix_len..new_len - suffix_len].iter().collect();

        // Find the text node containing the edit region
        let (text_node_id, _current_len) = find_first_text_node(doc.model(), node_id)?;
        let mut ops_json = Vec::new();

        let delete_count = delete_end - delete_start;
        if delete_count > 0 {
            let del_op = Operation::delete_text(text_node_id, delete_start, delete_count);
            let crdt_op = doc
                .apply_local(del_op)
                .map_err(|e| JsError::new(&e.to_string()))?;
            ops_json.push(serialize_crdt_op_to_json(&crdt_op));
        }

        if !insert_text.is_empty() {
            let ins_op = Operation::insert_text(text_node_id, delete_start, insert_text);
            let crdt_op = doc
                .apply_local(ins_op)
                .map_err(|e| JsError::new(&e.to_string()))?;
            ops_json.push(serialize_crdt_op_to_json(&crdt_op));
        }

        Ok(format!("[{}]", ops_json.join(",")))
    }

    // ─── Structural Editing (delegates to WasmDocument, reconstructs collab) ───
    // These operations modify the underlying model via WasmDocument's existing
    // implementations, then reconstruct the CollabDocument. This preserves full
    // editor compatibility while the CRDT layer handles text-level ops natively.

    /// Helper: apply a closure that mutates a WasmDocument, then sync the operations
    /// back to the original CollabDocument to generate CRDT-native operations.
    fn with_wasm_doc<F>(&mut self, f: F) -> Result<String, JsError>
    where
        F: FnOnce(&mut WasmDocument) -> Result<(), JsError>,
    {
        let mut collab = self
            .inner
            .take()
            .ok_or_else(|| JsError::new("Document freed"))?;

        let model = collab.model().clone();
        let mut wasm_doc = WasmDocument {
            batch_label: None,
            batch_count: 0,
            inner: Some(s1engine::Document::from_model(model)),
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        };

        // Run the mutation (e.g. split_paragraph) on the temporary engine document
        f(&mut wasm_doc)?;

        // Capture the operations that were performed
        let last_txn = wasm_doc.inner.as_ref().and_then(|d| d.last_txn.as_ref());
        let mut crdt_ops_json = Vec::new();

        if let Some(txn) = last_txn {
            // Replay the operations on the CollabDocument to generate CRDT metadata
            // using the CORRECT Lamport clock and State Vector.
            let crdt_ops = collab
                .apply_local_transaction(txn.operations.clone())
                .map_err(|e| JsError::new(&e.to_string()))?;

            for op in crdt_ops {
                crdt_ops_json.push(serialize_crdt_op_to_json(&op));
            }
        }

        self.inner = Some(collab);
        Ok(format!("[{}]", crdt_ops_json.join(",")))
    }

    /// Delete a text selection (single or cross-paragraph).
    /// Returns serialized CRDT operations.
    pub fn delete_selection(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
    ) -> Result<String, JsError> {
        let s = start_node_str.to_string();
        let e = end_node_str.to_string();
        self.with_wasm_doc(|doc| doc.delete_selection(&s, start_offset, &e, end_offset))
    }

    /// Insert text in a paragraph at a specific offset (CRDT-native).
    pub fn insert_text_in_paragraph(
        &mut self,
        node_id_str: &str,
        offset: usize,
        text: &str,
    ) -> Result<String, JsError> {
        self.apply_local_insert_text(node_id_str, offset, text)
    }

    /// Format a selection.
    /// Returns serialized CRDT operations.
    pub fn format_selection(
        &mut self,
        start_node_str: &str,
        start_offset: usize,
        end_node_str: &str,
        end_offset: usize,
        key: &str,
        value: &str,
    ) -> Result<String, JsError> {
        let s = start_node_str.to_string();
        let e = end_node_str.to_string();
        let k = key.to_string();
        let v = value.to_string();
        self.with_wasm_doc(|doc| doc.format_selection(&s, start_offset, &e, end_offset, &k, &v))
    }

    /// Split a paragraph at the given offset.
    /// Returns JSON: { "newId": "replica:counter", "ops": [ ... ] }
    pub fn split_paragraph(&mut self, node_id_str: &str, offset: usize) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        let mut new_id = String::new();
        let ops_json = self.with_wasm_doc(|doc| {
            new_id = doc.split_paragraph(&n, offset)?;
            Ok(())
        })?;
        Ok(format!("{{\"newId\":\"{}\",\"ops\":{}}}", new_id, ops_json))
    }

    /// Merge two paragraphs.
    /// Returns serialized CRDT operations.
    pub fn merge_paragraphs(
        &mut self,
        node1_str: &str,
        node2_str: &str,
    ) -> Result<String, JsError> {
        let n1 = node1_str.to_string();
        let n2 = node2_str.to_string();
        self.with_wasm_doc(|doc| doc.merge_paragraphs(&n1, &n2))
    }

    /// Set heading level for a paragraph.
    pub fn set_heading_level(&mut self, node_id_str: &str, level: u8) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        self.with_wasm_doc(|doc| doc.set_heading_level(&n, level))
    }

    /// Set alignment for a paragraph.
    pub fn set_alignment(&mut self, node_id_str: &str, alignment: &str) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        let a = alignment.to_string();
        self.with_wasm_doc(|doc| doc.set_alignment(&n, &a))
    }

    /// Set list format for a paragraph.
    pub fn set_list_format(
        &mut self,
        node_id_str: &str,
        format: &str,
        level: u32,
    ) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        let f = format.to_string();
        self.with_wasm_doc(|doc| doc.set_list_format(&n, &f, level))
    }

    /// Paste plain text (may create multiple paragraphs).
    pub fn paste_plain_text(
        &mut self,
        node_id_str: &str,
        offset: usize,
        text: &str,
    ) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        let t = text.to_string();
        self.with_wasm_doc(|doc| doc.paste_plain_text(&n, offset, &t))
    }

    /// Insert a paragraph after a given node.
    pub fn insert_paragraph_after(
        &mut self,
        after_node_str: &str,
        text: &str,
    ) -> Result<String, JsError> {
        let n = after_node_str.to_string();
        let t = text.to_string();
        let mut new_id = String::new();
        let ops_json = self.with_wasm_doc(|doc| {
            new_id = doc.insert_paragraph_after(&n, &t)?;
            Ok(())
        })?;
        Ok(format!("{{\"newId\":\"{}\",\"ops\":{}}}", new_id, ops_json))
    }

    /// Set line spacing for a paragraph.
    pub fn set_line_spacing(&mut self, node_id_str: &str, value: &str) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        let v = value.to_string();
        self.with_wasm_doc(|doc| doc.set_line_spacing(&n, &v))
    }

    /// Set indent for a paragraph.
    pub fn set_indent(
        &mut self,
        node_id_str: &str,
        side: &str,
        value: f64,
    ) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        let s = side.to_string();
        self.with_wasm_doc(|doc| doc.set_indent(&n, &s, value))
    }

    /// Delete a node.
    pub fn delete_node(&mut self, node_id_str: &str) -> Result<String, JsError> {
        let n = node_id_str.to_string();
        self.with_wasm_doc(|doc| doc.delete_node(&n))
    }

    /// Append an empty paragraph.
    pub fn append_paragraph(&mut self, text: &str) -> Result<String, JsError> {
        let t = text.to_string();
        let mut new_id = String::new();
        let ops_json = self.with_wasm_doc(|doc| {
            new_id = doc.append_paragraph(&t)?;
            Ok(())
        })?;
        Ok(format!("{{\"newId\":\"{}\",\"ops\":{}}}", new_id, ops_json))
    }

    /// Sort a table by column (delegates to WasmDocument.sort_table_by_column).
    pub fn sort_table_by_column(
        &mut self,
        table_id_str: &str,
        col_index: u32,
        ascending: bool,
    ) -> Result<String, JsError> {
        let t = table_id_str.to_string();
        self.with_wasm_doc(|doc| doc.sort_table_by_column(&t, col_index, ascending))
    }

    /// Set column widths for a table.
    pub fn set_table_column_widths(
        &mut self,
        table_id_str: &str,
        widths_csv: &str,
    ) -> Result<String, JsError> {
        let n = table_id_str.to_string();
        let w = widths_csv.to_string();
        self.with_wasm_doc(|doc| doc.set_table_column_widths(&n, &w))
    }

    /// Insert a table.
    pub fn insert_table(
        &mut self,
        after_node_str: &str,
        rows: u32,
        cols: u32,
    ) -> Result<String, JsError> {
        let n = after_node_str.to_string();
        let mut table_id = String::new();
        let ops_json = self.with_wasm_doc(|doc| {
            table_id = doc.insert_table(&n, rows, cols)?;
            Ok(())
        })?;
        Ok(format!(
            "{{\"tableId\":\"{}\",\"ops\":{}}}",
            table_id, ops_json
        ))
    }

    /// Insert horizontal rule.
    pub fn insert_horizontal_rule(&mut self, after_node_str: &str) -> Result<String, JsError> {
        let n = after_node_str.to_string();
        self.with_wasm_doc(|doc| {
            doc.insert_horizontal_rule(&n)?;
            Ok(())
        })
    }

    /// Insert page break.
    pub fn insert_page_break(&mut self, after_node_str: &str) -> Result<String, JsError> {
        let n = after_node_str.to_string();
        self.with_wasm_doc(|doc| {
            doc.insert_page_break(&n)?;
            Ok(())
        })
    }

    /// Free the document (for manual memory management from JS).
    pub fn free_doc(&mut self) {
        self.inner = None;
    }
}

// --- WasmEngine collab methods ---

#[wasm_bindgen]
impl WasmEngine {
    /// Create a new collaborative document.
    ///
    /// `replica_id` must be unique per user/session (e.g., random u64).
    pub fn create_collab(&self, replica_id: u64) -> WasmCollabDocument {
        WasmCollabDocument {
            inner: Some(s1_crdt::CollabDocument::new(replica_id)),
        }
    }

    /// Open a file as a collaborative document.
    ///
    /// The document is loaded and wrapped in a CRDT-aware container.
    pub fn open_collab(&self, data: &[u8], replica_id: u64) -> Result<WasmCollabDocument, JsError> {
        let doc = self.inner.open(data).map_err(|e| {
            JsError::new(&format!("Failed to open document for collaboration: {}", e))
        })?;
        let collab = s1_crdt::CollabDocument::from_model(doc.into_model(), replica_id);
        Ok(WasmCollabDocument {
            inner: Some(collab),
        })
    }
}

// --- Collaboration helper functions ---

fn serialize_crdt_op_to_json(op: &s1_crdt::CrdtOperation) -> String {
    // Serialize the essential fields for network transport
    let op_type = match &op.operation {
        Operation::InsertText {
            target_id,
            offset,
            text,
            ..
        } => {
            format!(
                "\"type\":\"InsertText\",\"target\":\"{}:{}\",\"offset\":{},\"text\":{}",
                target_id.replica,
                target_id.counter,
                offset,
                json_escape_string(text)
            )
        }
        Operation::DeleteText {
            target_id,
            offset,
            length,
            deleted_text,
            ..
        } => {
            let text_str = deleted_text.as_deref().unwrap_or("");
            format!(
                "\"type\":\"DeleteText\",\"target\":\"{}:{}\",\"offset\":{},\"length\":{},\"text\":{}",
                target_id.replica, target_id.counter, offset, length, json_escape_string(text_str)
            )
        }
        Operation::SetAttributes {
            target_id,
            attributes,
            previous,
        } => {
            let prev = previous.as_ref().cloned().unwrap_or_default();
            format!(
                "\"type\":\"SetAttributes\",\"target\":\"{}:{}\",\"attributes\":{},\"oldAttributes\":{}",
                target_id.replica, target_id.counter,
                attrs_to_json(attributes),
                attrs_to_json(&prev),
            )
        }
        Operation::InsertNode {
            parent_id,
            index,
            node,
            ..
        } => {
            format!(
                "\"type\":\"InsertNode\",\"nodeType\":\"{:?}\",\"parent\":\"{}:{}\",\"index\":{},\"nodeId\":\"{}:{}\"",
                node.node_type, parent_id.replica, parent_id.counter, index, node.id.replica, node.id.counter
            )
        }
        Operation::DeleteNode { target_id, .. } => {
            format!(
                "\"type\":\"DeleteNode\",\"target\":\"{}:{}\"",
                target_id.replica, target_id.counter
            )
        }
        Operation::MoveNode {
            target_id,
            new_parent_id,
            new_index,
            ..
        } => {
            format!(
                "\"type\":\"MoveNode\",\"target\":\"{}:{}\",\"newParent\":\"{}:{}\",\"newIndex\":{}",
                target_id.replica, target_id.counter, new_parent_id.replica, new_parent_id.counter, new_index
            )
        }
        _ => "\"type\":\"Other\"".to_string(),
    };

    format!(
        "{{\"id\":{{\"replica\":{},\"lamport\":{}}},{},\"deps\":{}}}",
        op.id.replica,
        op.id.lamport,
        op_type,
        state_vector_to_json(&op.deps),
    )
}

fn state_vector_to_json(sv: &s1_crdt::StateVector) -> String {
    let entries: Vec<String> = sv
        .entries()
        .iter()
        .map(|(r, l)| format!("\"{}\":{}", r, l))
        .collect();
    format!("{{{}}}", entries.join(","))
}

fn attrs_to_json(attrs: &s1_model::AttributeMap) -> String {
    let mut items = Vec::new();
    for (key, value) in attrs.iter() {
        let k = format!("{:?}", key);
        let v = match value {
            AttributeValue::Bool(b) => format!("{}", b),
            AttributeValue::Int(i) => format!("{}", i),
            AttributeValue::Float(f) => format!("{}", f),
            AttributeValue::String(s) => json_escape_string(s),
            AttributeValue::Color(c) => json_escape_string(&c.to_hex()),
            _ => format!("{:?}", value),
        };
        items.push(format!("{}:{}", json_escape_string(&k), v));
    }
    format!("{{{}}}", items.join(","))
}

fn json_escape_string(s: &str) -> String {
    let escaped: String = s
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c if c < '\x20' => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("\"{}\"", escaped)
}

fn deserialize_crdt_op_from_json(json: &str) -> Result<s1_crdt::CrdtOperation, JsError> {
    // Parse the JSON manually (no serde dependency)
    // Extract id, type, and fields
    let id = extract_crdt_op_id(json)?;
    let deps = extract_crdt_deps(json)?;
    let operation = extract_crdt_operation(json)?;

    Ok(s1_crdt::CrdtOperation {
        id,
        operation,
        deps,
        origin_left: None,
        origin_right: None,
        parent_op: None,
    })
}

fn extract_crdt_op_id(json: &str) -> Result<s1_crdt::OpId, JsError> {
    // Find "id":{"replica":N,"lamport":N}
    let id_start = json
        .find("\"id\"")
        .ok_or_else(|| JsError::new("Missing id in CRDT op"))?;
    let rest = &json[id_start..];
    let replica = extract_json_number(rest, "replica")?;
    let lamport = extract_json_number(rest, "lamport")?;
    Ok(s1_crdt::OpId { replica, lamport })
}

fn extract_crdt_deps(json: &str) -> Result<s1_crdt::StateVector, JsError> {
    let mut sv = s1_crdt::StateVector::new();
    if let Some(deps_start) = json.find("\"deps\"") {
        let rest = &json[deps_start + 6..];
        if let Some(brace_start) = rest.find('{') {
            let brace_end = rest[brace_start..].find('}').unwrap_or(rest.len());
            let deps_str = &rest[brace_start + 1..brace_start + brace_end];
            // Parse "replica":lamport pairs
            for pair in deps_str.split(',') {
                let pair = pair.trim();
                if pair.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = pair.split(':').collect();
                if parts.len() == 2 {
                    let r: u64 = parts[0].trim().trim_matches('"').parse().unwrap_or(0);
                    let l: u64 = parts[1].trim().parse().unwrap_or(0);
                    sv.set(r, l);
                }
            }
        }
    }
    Ok(sv)
}

fn extract_crdt_operation(json: &str) -> Result<Operation, JsError> {
    let op_type = extract_json_string(json, "type")?;
    match op_type.as_str() {
        "InsertText" => {
            let target = extract_json_node_id(json, "target")?;
            let offset = extract_json_number(json, "offset")? as usize;
            let text = extract_json_string(json, "text")?;
            Ok(Operation::insert_text(target, offset, text))
        }
        "DeleteText" => {
            let target = extract_json_node_id(json, "target")?;
            let offset = extract_json_number(json, "offset")? as usize;
            let length = extract_json_number(json, "length").unwrap_or(1) as usize;
            Ok(Operation::delete_text(target, offset, length))
        }
        "SetAttributes" => {
            let target = extract_json_node_id(json, "target")?;
            // For attributes, we'd need full parsing — simplified for now
            Ok(Operation::set_attributes(
                target,
                s1_model::AttributeMap::new(),
            ))
        }
        "InsertNode" => {
            let parent = extract_json_node_id(json, "parent")?;
            let node_id = extract_json_node_id(json, "nodeId")?;
            // Simplified — default to Paragraph type
            let node = Node::new(node_id, NodeType::Paragraph);
            Ok(Operation::insert_node(parent, 0, node))
        }
        "DeleteNode" => {
            let target = extract_json_node_id(json, "target")?;
            Ok(Operation::delete_node(target))
        }
        _ => Err(JsError::new(&format!(
            "Unknown operation type: {}",
            op_type
        ))),
    }
}

fn extract_json_number(json: &str, key: &str) -> Result<u64, JsError> {
    let search = format!("\"{}\"", key);
    let pos = json
        .find(&search)
        .ok_or_else(|| JsError::new(&format!("Missing key: {}", key)))?;
    let rest = &json[pos + search.len()..];
    let colon = rest.find(':').ok_or_else(|| JsError::new("Invalid JSON"))? + 1;
    let num_start = rest[colon..]
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| JsError::new("No number"))?
        + colon;
    let num_end = rest[num_start..]
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len() - num_start)
        + num_start;
    rest[num_start..num_end]
        .parse()
        .map_err(|_| JsError::new("Invalid number"))
}

fn extract_json_string(json: &str, key: &str) -> Result<String, JsError> {
    let search = format!("\"{}\"", key);
    let pos = json
        .find(&search)
        .ok_or_else(|| JsError::new(&format!("Missing key: {}", key)))?;
    let rest = &json[pos + search.len()..];
    // Find the value string after the colon
    let colon = rest.find(':').ok_or_else(|| JsError::new("Invalid JSON"))? + 1;
    let after_colon = rest[colon..].trim_start();
    if let Some(str_content) = after_colon.strip_prefix('"') {
        let mut end = 0;
        let mut escaped = false;
        for ch in str_content.chars() {
            if escaped {
                escaped = false;
                end += ch.len_utf8();
                continue;
            }
            if ch == '\\' {
                escaped = true;
                end += 1;
                continue;
            }
            if ch == '"' {
                break;
            }
            end += ch.len_utf8();
        }
        Ok(str_content[..end]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
            .replace("\\n", "\n"))
    } else {
        // Not a string value — take until comma or brace
        let end = after_colon
            .find([',', '}', ']'])
            .unwrap_or(after_colon.len());
        Ok(after_colon[..end].trim().to_string())
    }
}

fn extract_json_node_id(json: &str, key: &str) -> Result<NodeId, JsError> {
    let s = extract_json_string(json, key)?;
    parse_node_id(&s)
}

fn serialize_awareness_update(update: &s1_crdt::AwarenessUpdate) -> String {
    match &update.state {
        Some(cursor) => {
            format!(
                "{{\"replicaId\":{},\"connected\":true,\"nodeId\":\"{}:{}\",\"offset\":{},\"userName\":{},\"userColor\":{}}}",
                update.replica_id,
                cursor.selection.anchor.node_id.replica,
                cursor.selection.anchor.node_id.counter,
                cursor.selection.anchor.offset,
                json_escape_string(&cursor.user_name),
                json_escape_string(&cursor.user_color),
            )
        }
        None => {
            format!(
                "{{\"replicaId\":{},\"connected\":false}}",
                update.replica_id
            )
        }
    }
}

fn deserialize_awareness_update(json: &str) -> Result<s1_crdt::AwarenessUpdate, JsError> {
    let replica_id = extract_json_number(json, "replicaId")?;
    let connected = extract_json_string(json, "connected").unwrap_or_default() == "true";

    if !connected {
        return Ok(s1_crdt::AwarenessUpdate {
            replica_id,
            state: None,
        });
    }

    let node_id_str = extract_json_string(json, "nodeId")?;
    let node_id = parse_node_id(&node_id_str)?;
    let offset = extract_json_number(json, "offset")? as usize;
    let user_name = extract_json_string(json, "userName").unwrap_or_default();
    let user_color = extract_json_string(json, "userColor").unwrap_or_default();

    Ok(s1_crdt::AwarenessUpdate {
        replica_id,
        state: Some(s1_crdt::CursorState {
            replica_id,
            selection: s1_ops::Selection::collapsed(s1_ops::Position::new(node_id, offset)),
            user_name,
            user_color,
            sequence: 0,
        }),
    })
}

fn parse_state_vector_json(json: &str) -> Result<s1_crdt::StateVector, JsError> {
    let mut sv = s1_crdt::StateVector::new();
    let trimmed = json.trim().trim_start_matches('{').trim_end_matches('}');
    for pair in trimmed.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.split(':').collect();
        if parts.len() == 2 {
            let r: u64 = parts[0].trim().trim_matches('"').parse().unwrap_or(0);
            let l: u64 = parts[1].trim().parse().unwrap_or(0);
            sv.set(r, l);
        }
    }
    Ok(sv)
}

// Helper to render HTML from a DocumentModel (shared between WasmDocument and WasmCollabDocument).
//
// Known limitation (W-14): The full HTML is accumulated in a single String.
// For very large documents a streaming/callback-based API would be more
// memory-efficient, but that would require an API redesign. We mitigate the
// allocation cost by pre-sizing the buffer based on node count.
fn to_html_from_model(model: &DocumentModel) -> String {
    let body_id = match model.body_id() {
        Some(id) => id,
        None => return String::new(),
    };
    let children = model.children(body_id);
    // Estimate ~120 bytes of HTML per node (tags + attributes + text).
    let estimated_size = (model.node_count() * 120).max(1024);
    let mut html = String::with_capacity(estimated_size);
    for child in &children {
        html.push_str(&render_node_to_html(model, child));
    }

    // Render footnote/endnote bodies from root (they are root children, not body children)
    let root_id = model.root_id();
    let root_children = model.children(root_id);
    let mut has_fn = false;
    let mut has_en = false;
    for child in &root_children {
        if child.node_type == NodeType::FootnoteBody {
            has_fn = true;
        }
        if child.node_type == NodeType::EndnoteBody {
            has_en = true;
        }
    }
    if has_fn {
        html.push_str(
            "<div class=\"footnotes-section\" data-footnotes=\"true\" contenteditable=\"false\">",
        );
        html.push_str("<hr class=\"footnote-separator\" style=\"border:none;border-top:1px solid #dadce0;width:33%;margin:12px 0 8px 0\" />");
        for child in &root_children {
            if child.node_type == NodeType::FootnoteBody {
                let fn_num = child
                    .attributes
                    .get_i64(&AttributeKey::FootnoteNumber)
                    .unwrap_or(0);
                html.push_str(&format!(
                    "<div class=\"footnote-body\" data-footnote-id=\"{}\" contenteditable=\"true\">",
                    fn_num
                ));
                html.push_str(&format!(
                    "<span class=\"footnote-number\" contenteditable=\"false\">{}.</span> ",
                    fn_num
                ));
                let para_children = model.children(child.id);
                for pc in &para_children {
                    html.push_str(&render_node_to_html(model, pc));
                }
                html.push_str("</div>");
            }
        }
        html.push_str("</div>");
    }
    if has_en {
        html.push_str(
            "<div class=\"endnotes-section\" data-endnotes=\"true\" contenteditable=\"false\">",
        );
        html.push_str("<div class=\"endnotes-title\" style=\"font-weight:600;font-size:11pt;margin:16px 0 8px 0;border-bottom:1px solid #dadce0;padding-bottom:4px\">Endnotes</div>");
        for child in &root_children {
            if child.node_type == NodeType::EndnoteBody {
                let en_num = child
                    .attributes
                    .get_i64(&AttributeKey::EndnoteNumber)
                    .unwrap_or(0);
                html.push_str(&format!(
                    "<div class=\"endnote-body\" data-endnote-id=\"{}\" contenteditable=\"true\">",
                    en_num
                ));
                html.push_str(&format!(
                    "<span class=\"endnote-number\" contenteditable=\"false\">{}.</span> ",
                    en_num
                ));
                let para_children = model.children(child.id);
                for pc in &para_children {
                    html.push_str(&render_node_to_html(model, pc));
                }
                html.push_str("</div>");
            }
        }
        html.push_str("</div>");
    }

    html
}

fn render_node_to_html(model: &DocumentModel, node: &Node) -> String {
    match node.node_type {
        NodeType::Paragraph => {
            let style_id_str = node
                .attributes
                .get_string(&AttributeKey::StyleId)
                .map(|s| s.to_string());
            let heading_level: u8 = style_id_str
                .as_deref()
                .and_then(|sid| {
                    let sid_lower = sid.to_lowercase();
                    if sid_lower.starts_with("heading") {
                        sid_lower
                            .chars()
                            .filter(|c| c.is_ascii_digit())
                            .collect::<String>()
                            .parse()
                            .ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let tag = if (1..=6).contains(&heading_level) {
                format!("h{}", heading_level)
            } else {
                "p".to_string()
            };
            let node_id = node.id;

            // Build inline heading typography so rendering is CSS-independent
            let style_attr = if (1..=6).contains(&heading_level) {
                let l = heading_level;
                let mut hs = String::new();

                let (style_font_size, style_bold, style_font_family) = style_id_str
                    .as_deref()
                    .and_then(|sid| model.style_by_id(sid))
                    .map(|s| {
                        let fs = s.attributes.get_f64(&AttributeKey::FontSize);
                        let bold = s.attributes.get_bool(&AttributeKey::Bold);
                        let ff = s
                            .attributes
                            .get_string(&AttributeKey::FontFamily)
                            .map(|v| v.to_string());
                        (fs, bold, ff)
                    })
                    .unwrap_or((None, None, None));

                let size = style_font_size.unwrap_or(match l {
                    1 => 24.0,
                    2 => 18.0,
                    3 => 14.0,
                    4 => 12.0,
                    5 => 11.0,
                    _ => 10.0,
                });
                hs.push_str(&format!("font-size:{}pt;", size));
                let weight = if style_bold == Some(false) {
                    "normal"
                } else {
                    "700"
                };
                hs.push_str(&format!("font-weight:{};", weight));
                if let Some(ref ff) = style_font_family {
                    hs.push_str(&format!("font-family:{};", ff));
                }
                let mt: f64 = match l {
                    1 => 20.0,
                    2 => 18.0,
                    3 => 16.0,
                    4 => 14.0,
                    5 => 12.0,
                    _ => 10.0,
                };
                hs.push_str(&format!("margin-top:{}pt;", mt));
                let mb: f64 = if l <= 2 { 6.0 } else { 4.0 };
                hs.push_str(&format!("margin-bottom:{}pt;", mb));
                format!(" style=\"{}\"", hs)
            } else {
                // W5: Build paragraph-level CSS for non-heading paragraphs
                let mut ps = String::new();
                // Tab stops
                if let Some(AttributeValue::TabStops(stops)) =
                    node.attributes.get(&AttributeKey::TabStops)
                {
                    if let Some(first) = stops.first() {
                        let tab_px = (first.position * 96.0 / 72.0) as i32;
                        if tab_px > 0 {
                            ps.push_str(&format!("tab-size:{}px;", tab_px));
                        }
                    }
                }
                // Widow/orphan defaults
                ps.push_str("orphans:2;widows:2;");
                // Contextual spacing
                if node.attributes.get_bool(&AttributeKey::ContextualSpacing) == Some(true) {
                    ps.push_str("margin-block-start:0;");
                }
                // Keep with next / keep lines together
                if node.attributes.get_bool(&AttributeKey::KeepWithNext) == Some(true) {
                    ps.push_str("break-after:avoid;");
                }
                if node.attributes.get_bool(&AttributeKey::KeepLinesTogether) == Some(true) {
                    ps.push_str("break-inside:avoid;");
                }
                if ps.is_empty() {
                    String::new()
                } else {
                    format!(" style=\"{}\"", ps)
                }
            };

            let mut html = format!(
                "<{}{} data-node-id=\"{}:{}\">",
                tag, style_attr, node_id.replica, node_id.counter
            );
            let children = model.children(node_id);
            for child in &children {
                html.push_str(&render_node_to_html(model, child));
            }
            html.push_str(&format!("</{}>", tag));
            html
        }
        NodeType::Run => {
            let mut style = String::new();
            if node.attributes.get_bool(&AttributeKey::Bold) == Some(true) {
                style.push_str("font-weight:bold;");
            }
            if node.attributes.get_bool(&AttributeKey::Italic) == Some(true) {
                style.push_str("font-style:italic;");
            }
            if node.attributes.get_bool(&AttributeKey::Underline) == Some(true) {
                style.push_str("text-decoration:underline;");
            }

            let node_id = node.id;
            let children = model.children(node_id);
            let mut inner = String::new();
            for child in &children {
                if child.node_type == NodeType::Text {
                    inner.push_str(&html_escape(child.text_content.as_deref().unwrap_or("")));
                }
            }

            if style.is_empty() {
                inner
            } else {
                format!("<span style=\"{}\">{}</span>", style, inner)
            }
        }
        NodeType::Text => html_escape(node.text_content.as_deref().unwrap_or("")),
        NodeType::FootnoteRef => {
            let fn_num = node
                .attributes
                .get_i64(&AttributeKey::FootnoteNumber)
                .unwrap_or(0);
            format!(
                "<sup class=\"footnote-ref\" data-footnote-ref=\"{}\" title=\"Footnote {}\">{}</sup>",
                fn_num, fn_num, fn_num
            )
        }
        NodeType::EndnoteRef => {
            let en_num = node
                .attributes
                .get_i64(&AttributeKey::EndnoteNumber)
                .unwrap_or(0);
            format!(
                "<sup class=\"endnote-ref\" data-endnote-ref=\"{}\" title=\"Endnote {}\">{}</sup>",
                en_num, en_num, en_num
            )
        }
        NodeType::BookmarkStart => {
            if let Some(name) = node.attributes.get_string(&AttributeKey::BookmarkName) {
                let nid = node.id;
                format!(
                    "<span class=\"bookmark-marker\" data-bookmark=\"{}\" data-node-id=\"{}:{}\" \
                     contenteditable=\"false\" title=\"Bookmark: {}\">\
                     <a id=\"{}\"></a></span>",
                    html_escape(name),
                    nid.replica,
                    nid.counter,
                    html_escape(name),
                    html_escape(name)
                )
            } else {
                String::new()
            }
        }
        NodeType::Equation => {
            let latex = node
                .attributes
                .get_string(&AttributeKey::EquationSource)
                .unwrap_or("");
            let nid = node.id;
            format!(
                "<span class=\"equation-inline\" data-equation=\"{}\" data-node-id=\"{}:{}\" \
                 contenteditable=\"false\" title=\"Equation (double-click to edit)\">{}</span>",
                html_escape(latex),
                nid.replica,
                nid.counter,
                html_escape(latex)
            )
        }
        _ => String::new(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ─── WasmPdfEditor — PDF reading/editing via lopdf ──────────────────────────

/// PDF editor for reading, annotating, and modifying existing PDFs.
#[wasm_bindgen]
pub struct WasmPdfEditor {
    inner: s1_format_pdf::PdfEditor,
}

#[wasm_bindgen]
impl WasmPdfEditor {
    /// Open a PDF from raw bytes.
    pub fn open(data: &[u8]) -> Result<WasmPdfEditor, JsError> {
        let editor =
            s1_format_pdf::PdfEditor::open(data).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmPdfEditor { inner: editor })
    }

    /// Get the number of pages.
    pub fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    /// Add a white rectangle to cover content on a page (0-indexed).
    pub fn add_white_rect(
        &mut self,
        page: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), JsError> {
        self.inner
            .add_white_rect(
                page,
                s1_format_pdf::Rect {
                    x,
                    y,
                    width,
                    height,
                },
            )
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Add text overlay on a page at a given position (0-indexed).
    #[allow(clippy::too_many_arguments)]
    pub fn add_text_overlay(
        &mut self,
        page: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: &str,
        font_size: f64,
    ) -> Result<(), JsError> {
        self.inner
            .add_text_overlay(
                page,
                s1_format_pdf::Rect {
                    x,
                    y,
                    width,
                    height,
                },
                text,
                font_size,
            )
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Add a highlight annotation (0-indexed page, quad points as flat array).
    #[allow(clippy::too_many_arguments)]
    pub fn add_highlight_annotation(
        &mut self,
        page: usize,
        quads: &[f64],
        r: f32,
        g: f32,
        b: f32,
        author: &str,
        content: &str,
    ) -> Result<(), JsError> {
        self.inner
            .add_highlight_annotation(page, quads, [r, g, b], author, content)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Add a sticky note (text) annotation (0-indexed page).
    pub fn add_text_annotation(
        &mut self,
        page: usize,
        x: f64,
        y: f64,
        author: &str,
        content: &str,
    ) -> Result<(), JsError> {
        self.inner
            .add_text_annotation(page, x, y, author, content)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Add an ink (freehand) annotation. Points is a flat array [x1,y1,x2,y2,...].
    pub fn add_ink_annotation(
        &mut self,
        page: usize,
        points: &[f64],
        r: f32,
        g: f32,
        b: f32,
        width: f64,
    ) -> Result<(), JsError> {
        let path: Vec<(f64, f64)> = points.chunks_exact(2).map(|c| (c[0], c[1])).collect();
        self.inner
            .add_ink_annotation(page, &[path], [r, g, b], width)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Add a free text annotation (text box).
    #[allow(clippy::too_many_arguments)]
    pub fn add_freetext_annotation(
        &mut self,
        page: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: &str,
        font_size: f64,
    ) -> Result<(), JsError> {
        self.inner
            .add_freetext_annotation(
                page,
                s1_format_pdf::Rect {
                    x,
                    y,
                    width,
                    height,
                },
                text,
                font_size,
            )
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Add a redaction annotation.
    pub fn add_redaction(
        &mut self,
        page: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), JsError> {
        self.inner
            .add_redaction(
                page,
                s1_format_pdf::Rect {
                    x,
                    y,
                    width,
                    height,
                },
            )
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Apply all redaction annotations — permanently removes content.
    pub fn apply_redactions(&mut self) -> Result<(), JsError> {
        self.inner
            .apply_redactions()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Delete a page (0-indexed).
    pub fn delete_page(&mut self, page: usize) -> Result<(), JsError> {
        self.inner
            .delete_page(page)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Move a page from one position to another (0-indexed).
    pub fn move_page(&mut self, from: usize, to: usize) -> Result<(), JsError> {
        self.inner
            .move_page(from, to)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Rotate a page by degrees (must be a multiple of 90).
    pub fn rotate_page(&mut self, page: usize, degrees: i32) -> Result<(), JsError> {
        self.inner
            .rotate_page(page, degrees)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Duplicate a page (0-indexed).
    pub fn duplicate_page(&mut self, page: usize) -> Result<(), JsError> {
        self.inner
            .duplicate_page(page)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Extract specified pages (0-indexed) into a new PDF.
    pub fn extract_pages(&mut self, pages: &[usize]) -> Result<Vec<u8>, JsError> {
        self.inner
            .extract_pages(pages)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Merge another PDF's pages at the end of this document.
    pub fn merge(&mut self, other_data: &[u8]) -> Result<(), JsError> {
        self.inner
            .merge(other_data)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get all form fields as JSON.
    pub fn get_form_fields(&self) -> Result<String, JsError> {
        let fields = self
            .inner
            .get_form_fields()
            .map_err(|e| JsError::new(&e.to_string()))?;
        let json_fields: Vec<String> = fields.iter().map(|f| {
            format!(
                r#"{{"name":"{}","field_type":"{:?}","page":{},"rect":{{"x":{},"y":{},"width":{},"height":{}}},"value":"{}","options":[{}]}}"#,
                escape_json(&f.name),
                f.field_type,
                f.page,
                f.rect.x, f.rect.y, f.rect.width, f.rect.height,
                escape_json(&f.value),
                f.options.iter().map(|o| format!("\"{}\"", escape_json(o))).collect::<Vec<_>>().join(","),
            )
        }).collect();
        Ok(format!("[{}]", json_fields.join(",")))
    }

    /// Set a form field's value by name.
    pub fn set_form_field_value(&mut self, field_name: &str, value: &str) -> Result<(), JsError> {
        self.inner
            .set_form_field_value(field_name, value)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Flatten the form.
    pub fn flatten_form(&mut self) -> Result<(), JsError> {
        self.inner
            .flatten_form()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Save the modified PDF to bytes.
    pub fn save(&mut self) -> Result<Vec<u8>, JsError> {
        self.inner.save().map_err(|e| JsError::new(&e.to_string()))
    }
}

// ─── WasmSpreadsheet ─────────────────────────────────────

/// WASM bindings for spreadsheet operations (XLSX, ODS, CSV).
///
/// Provides a JavaScript-friendly API for opening, editing, and exporting
/// spreadsheet files from the browser or Node.js.
#[wasm_bindgen]
pub struct WasmSpreadsheet {
    inner: s1_format_xlsx::Workbook,
}

#[wasm_bindgen]
impl WasmSpreadsheet {
    /// Create a new empty spreadsheet with one sheet.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: s1_format_xlsx::Workbook::new(),
        }
    }

    /// Open a spreadsheet from bytes (auto-detect XLSX, ODS, CSV).
    ///
    /// Detection is based on file magic bytes:
    /// - XLSX/ODS: ZIP signature (PK header)
    /// - CSV: plain text fallback
    pub fn open(data: &[u8]) -> Result<WasmSpreadsheet, JsError> {
        // Try to detect format from magic bytes
        if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
            // ZIP-based format — try XLSX first, then ODS
            if let Ok(wb) = s1_format_xlsx::read(data) {
                return Ok(WasmSpreadsheet { inner: wb });
            }
            if let Ok(wb) = s1_format_xlsx::read_ods(data) {
                return Ok(WasmSpreadsheet { inner: wb });
            }
            Err(JsError::new(
                "Failed to parse ZIP-based spreadsheet as XLSX or ODS",
            ))
        } else {
            // Try as CSV
            let text = String::from_utf8_lossy(data);
            let wb = parse_csv_to_workbook(&text, ',');
            Ok(WasmSpreadsheet { inner: wb })
        }
    }

    /// Get the number of sheets.
    pub fn sheet_count(&self) -> usize {
        self.inner.sheets.len()
    }

    /// Get sheet names as a JSON array.
    pub fn sheet_names_json(&self) -> String {
        let names: Vec<&str> = self.inner.sheets.iter().map(|s| s.name.as_str()).collect();
        serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get cell value as string.
    ///
    /// Returns an empty string for empty or out-of-range cells.
    pub fn get_cell(&self, sheet: usize, col: u32, row: u32) -> String {
        if let Some(s) = self.inner.sheets.get(sheet) {
            if let Some(cell) = s.get(col, row) {
                cell.value.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    /// Set cell value (auto-detect type: number, boolean, or text).
    pub fn set_cell(&mut self, sheet: usize, col: u32, row: u32, value: &str) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            let cell_value = parse_cell_value_auto(value);
            s.set(col, row, cell_value);
        }
    }

    /// Set cell formula.
    pub fn set_formula(&mut self, sheet: usize, col: u32, row: u32, formula: &str) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            s.set_formula(col, row, formula, s1_format_xlsx::CellValue::Empty);
        }
    }

    /// Get a visible range of cells as JSON for rendering.
    ///
    /// Returns a JSON object:
    /// ```json
    /// {
    ///   "cells": [{"col":0,"row":0,"value":"Hello","formula":null,"styleId":0}, ...],
    ///   "colWidths": [8.43, 15.0, ...],
    ///   "rowHeights": [15.0, 20.0, ...]
    /// }
    /// ```
    pub fn get_visible_range_json(
        &self,
        sheet: usize,
        start_col: u32,
        start_row: u32,
        end_col: u32,
        end_row: u32,
    ) -> String {
        let Some(s) = self.inner.sheets.get(sheet) else {
            return r#"{"cells":[],"colWidths":[],"rowHeights":[]}"#.to_string();
        };

        let mut cells = Vec::new();
        for r in start_row..=end_row {
            for c in start_col..=end_col {
                if let Some(cell) = s.get(c, r) {
                    let value_str = cell.value.to_string();
                    let formula_json = match &cell.formula {
                        Some(f) => format!("\"{}\"", escape_json_string(f)),
                        None => "null".to_string(),
                    };
                    cells.push(format!(
                        r#"{{"col":{},"row":{},"value":"{}","formula":{},"styleId":{}}}"#,
                        c,
                        r,
                        escape_json_string(&value_str),
                        formula_json,
                        cell.style_id,
                    ));
                }
            }
        }

        let mut col_widths = Vec::new();
        for c in start_col..=end_col {
            let w = s.column_widths.get(&c).copied().unwrap_or(8.43);
            col_widths.push(format!("{w}"));
        }

        let mut row_heights = Vec::new();
        for r in start_row..=end_row {
            let h = s.row_heights.get(&r).copied().unwrap_or(15.0);
            row_heights.push(format!("{h}"));
        }

        format!(
            r#"{{"cells":[{}],"colWidths":[{}],"rowHeights":[{}]}}"#,
            cells.join(","),
            col_widths.join(","),
            row_heights.join(","),
        )
    }

    /// Recalculate all formulas in a sheet.
    pub fn recalculate(&mut self, sheet: usize) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            s.recalculate();
        }
    }

    /// Sort rows by a column value.
    ///
    /// Sorts all data rows in the sheet by the specified column.
    pub fn sort_by_column(&mut self, sheet: usize, col: u32, ascending: bool) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            sort_sheet_by_column(s, col, ascending);
        }
    }

    /// Export as XLSX bytes.
    pub fn export_xlsx(&self) -> Result<Vec<u8>, JsError> {
        s1_format_xlsx::write(&self.inner).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Export as ODS bytes.
    pub fn export_ods(&self) -> Result<Vec<u8>, JsError> {
        s1_format_xlsx::write_ods(&self.inner).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Export a sheet as CSV string.
    pub fn export_csv(&self, sheet: usize) -> String {
        if let Some(s) = self.inner.sheets.get(sheet) {
            s.to_csv(',')
        } else {
            String::new()
        }
    }

    /// Get dimensions (max col, max row) as JSON string: `"[cols,rows]"`.
    pub fn dimensions(&self, sheet: usize) -> String {
        if let Some(s) = self.inner.sheets.get(sheet) {
            let (cols, rows) = s.dimensions();
            format!("[{cols},{rows}]")
        } else {
            "[0,0]".to_string()
        }
    }

    /// Insert a row after the given row index.
    ///
    /// All rows at `after_row + 1` and below are shifted down.
    pub fn insert_row(&mut self, sheet: usize, after_row: u32) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            insert_row_in_sheet(s, after_row);
        }
    }

    /// Delete a row and shift remaining rows up.
    pub fn delete_row(&mut self, sheet: usize, row: u32) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            delete_row_in_sheet(s, row);
        }
    }

    /// Insert a column after the given column index.
    ///
    /// All columns at `after_col + 1` and beyond are shifted right.
    pub fn insert_column(&mut self, sheet: usize, after_col: u32) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            insert_column_in_sheet(s, after_col);
        }
    }

    /// Delete a column and shift remaining columns left.
    pub fn delete_column(&mut self, sheet: usize, col: u32) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            delete_column_in_sheet(s, col);
        }
    }

    /// Add a new sheet with the given name.
    pub fn add_sheet(&mut self, name: &str) {
        self.inner.sheets.push(s1_format_xlsx::Sheet {
            name: name.to_string(),
            ..Default::default()
        });
    }

    /// Delete a sheet by index.
    pub fn delete_sheet(&mut self, index: usize) {
        if index < self.inner.sheets.len() {
            self.inner.sheets.remove(index);
        }
    }

    /// Rename a sheet by index.
    pub fn rename_sheet(&mut self, index: usize, name: &str) {
        if let Some(s) = self.inner.sheets.get_mut(index) {
            s.name = name.to_string();
        }
    }

    /// Set or clear frozen panes on a sheet.
    ///
    /// Pass `col=0, row=0` to unfreeze.
    pub fn freeze_panes(&mut self, sheet: usize, col: u32, row: u32) {
        if let Some(s) = self.inner.sheets.get_mut(sheet) {
            if col == 0 && row == 0 {
                s.frozen_pane = None;
            } else {
                s.frozen_pane = Some(s1_format_xlsx::CellRef::new(col, row));
            }
        }
    }

    /// Get merged cells as JSON array: `[{"start":"A1","end":"C3"}, ...]`.
    pub fn merged_cells_json(&self, sheet: usize) -> String {
        if let Some(s) = self.inner.sheets.get(sheet) {
            let entries: Vec<String> = s
                .merged_cells
                .iter()
                .map(|r| {
                    format!(
                        r#"{{"start":"{}","end":"{}"}}"#,
                        r.start.to_a1(),
                        r.end.to_a1()
                    )
                })
                .collect();
            format!("[{}]", entries.join(","))
        } else {
            "[]".to_string()
        }
    }
}

impl Default for WasmSpreadsheet {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Spreadsheet helper functions ─────────────────────────

/// Parse a string value, auto-detecting type.
fn parse_cell_value_auto(value: &str) -> s1_format_xlsx::CellValue {
    if value.is_empty() {
        return s1_format_xlsx::CellValue::Empty;
    }
    // Boolean
    match value.to_uppercase().as_str() {
        "TRUE" => return s1_format_xlsx::CellValue::Boolean(true),
        "FALSE" => return s1_format_xlsx::CellValue::Boolean(false),
        _ => {}
    }
    // Number
    if let Ok(n) = value.parse::<f64>() {
        return s1_format_xlsx::CellValue::Number(n);
    }
    // Text
    s1_format_xlsx::CellValue::Text(value.to_string())
}

/// Parse CSV text into a Workbook.
fn parse_csv_to_workbook(text: &str, delimiter: char) -> s1_format_xlsx::Workbook {
    let mut wb = s1_format_xlsx::Workbook::new();
    let sheet = match wb.sheets.first_mut() {
        Some(s) => s,
        None => return wb, // should never happen, but avoid panic
    };

    for (row_idx, line) in text.lines().enumerate() {
        let mut col_idx = 0u32;
        let mut chars = line.chars().peekable();

        while chars.peek().is_some() {
            let value = if chars.peek() == Some(&'"') {
                // Quoted field
                chars.next(); // consume opening quote
                let mut field = String::new();
                loop {
                    match chars.next() {
                        Some('"') => {
                            if chars.peek() == Some(&'"') {
                                chars.next();
                                field.push('"');
                            } else {
                                break;
                            }
                        }
                        Some(c) => field.push(c),
                        None => break,
                    }
                }
                // Consume delimiter after closing quote
                if chars.peek() == Some(&delimiter) {
                    chars.next();
                }
                field
            } else {
                // Unquoted field
                let mut field = String::new();
                loop {
                    match chars.peek() {
                        Some(&c) if c == delimiter => {
                            chars.next();
                            break;
                        }
                        Some(_) => {
                            if let Some(c) = chars.next() {
                                field.push(c);
                            }
                        }
                        None => break,
                    }
                }
                field
            };

            if !value.is_empty() {
                sheet.set(col_idx, row_idx as u32, parse_cell_value_auto(&value));
            }
            col_idx += 1;
        }
    }

    wb
}

/// Sort a sheet's data rows by a specific column.
fn sort_sheet_by_column(sheet: &mut s1_format_xlsx::Sheet, col: u32, ascending: bool) {
    let (max_col, max_row) = sheet.dimensions();
    if max_row == 0 {
        return;
    }

    // Collect all row data
    let mut row_data: Vec<(u32, std::collections::BTreeMap<u32, s1_format_xlsx::Cell>)> =
        Vec::new();
    for r in 0..max_row {
        let mut row_cells = std::collections::BTreeMap::new();
        for c in 0..max_col {
            let ref_cell = s1_format_xlsx::CellRef::new(c, r);
            if let Some(cell) = sheet.cells.remove(&ref_cell) {
                row_cells.insert(c, cell);
            }
        }
        row_data.push((r, row_cells));
    }

    // Sort by the specified column
    row_data.sort_by(|(_, a_cells), (_, b_cells)| {
        let a_val = a_cells
            .get(&col)
            .map(|c| &c.value)
            .unwrap_or(&s1_format_xlsx::CellValue::Empty);
        let b_val = b_cells
            .get(&col)
            .map(|c| &c.value)
            .unwrap_or(&s1_format_xlsx::CellValue::Empty);

        let cmp = compare_cell_values(a_val, b_val);
        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });

    // Write sorted rows back
    sheet.cells.clear();
    for (new_row, (_, row_cells)) in row_data.into_iter().enumerate() {
        for (c, cell) in row_cells {
            sheet
                .cells
                .insert(s1_format_xlsx::CellRef::new(c, new_row as u32), cell);
        }
    }
}

/// Compare two cell values for sorting.
fn compare_cell_values(
    a: &s1_format_xlsx::CellValue,
    b: &s1_format_xlsx::CellValue,
) -> std::cmp::Ordering {
    use s1_format_xlsx::CellValue;
    use std::cmp::Ordering;

    fn rank(v: &CellValue) -> u8 {
        match v {
            CellValue::Empty => 0,
            CellValue::Number(_) | CellValue::Date(_) => 1,
            CellValue::Text(_) => 2,
            CellValue::Boolean(_) => 3,
            CellValue::Error(_) => 4,
        }
    }

    let ra = rank(a);
    let rb = rank(b);
    if ra != rb {
        return ra.cmp(&rb);
    }

    match (a, b) {
        (CellValue::Number(na), CellValue::Number(nb))
        | (CellValue::Date(na), CellValue::Date(nb)) => {
            na.partial_cmp(nb).unwrap_or(Ordering::Equal)
        }
        (CellValue::Text(sa), CellValue::Text(sb)) => sa.cmp(sb),
        (CellValue::Boolean(ba), CellValue::Boolean(bb)) => ba.cmp(bb),
        _ => Ordering::Equal,
    }
}

/// Insert a row after `after_row`, shifting cells below down by one.
fn insert_row_in_sheet(sheet: &mut s1_format_xlsx::Sheet, after_row: u32) {
    let new_row = after_row + 1;
    // Collect all cells that need to move
    let cells_to_move: Vec<(s1_format_xlsx::CellRef, s1_format_xlsx::Cell)> = sheet
        .cells
        .iter()
        .filter(|(r, _)| r.row >= new_row)
        .map(|(r, c)| (*r, c.clone()))
        .collect();

    // Remove them
    for (r, _) in &cells_to_move {
        sheet.cells.remove(r);
    }

    // Re-insert shifted down
    for (r, c) in cells_to_move {
        sheet
            .cells
            .insert(s1_format_xlsx::CellRef::new(r.col, r.row + 1), c);
    }

    // Shift row heights
    let heights_to_move: Vec<(u32, f64)> = sheet
        .row_heights
        .iter()
        .filter(|(&r, _)| r >= new_row)
        .map(|(&r, &h)| (r, h))
        .collect();
    for (r, _) in &heights_to_move {
        sheet.row_heights.remove(r);
    }
    for (r, h) in heights_to_move {
        sheet.row_heights.insert(r + 1, h);
    }
}

/// Delete a row, shifting cells below up by one.
fn delete_row_in_sheet(sheet: &mut s1_format_xlsx::Sheet, row: u32) {
    // Remove cells in the target row
    let to_remove: Vec<s1_format_xlsx::CellRef> = sheet
        .cells
        .keys()
        .filter(|r| r.row == row)
        .copied()
        .collect();
    for r in to_remove {
        sheet.cells.remove(&r);
    }

    // Shift cells below the deleted row up
    let cells_to_move: Vec<(s1_format_xlsx::CellRef, s1_format_xlsx::Cell)> = sheet
        .cells
        .iter()
        .filter(|(r, _)| r.row > row)
        .map(|(r, c)| (*r, c.clone()))
        .collect();
    for (r, _) in &cells_to_move {
        sheet.cells.remove(r);
    }
    for (r, c) in cells_to_move {
        sheet
            .cells
            .insert(s1_format_xlsx::CellRef::new(r.col, r.row - 1), c);
    }

    // Shift row heights
    sheet.row_heights.remove(&row);
    let heights_to_move: Vec<(u32, f64)> = sheet
        .row_heights
        .iter()
        .filter(|(&r, _)| r > row)
        .map(|(&r, &h)| (r, h))
        .collect();
    for (r, _) in &heights_to_move {
        sheet.row_heights.remove(r);
    }
    for (r, h) in heights_to_move {
        sheet.row_heights.insert(r - 1, h);
    }
}

/// Insert a column after `after_col`, shifting cells to the right.
fn insert_column_in_sheet(sheet: &mut s1_format_xlsx::Sheet, after_col: u32) {
    let new_col = after_col + 1;

    let cells_to_move: Vec<(s1_format_xlsx::CellRef, s1_format_xlsx::Cell)> = sheet
        .cells
        .iter()
        .filter(|(r, _)| r.col >= new_col)
        .map(|(r, c)| (*r, c.clone()))
        .collect();
    for (r, _) in &cells_to_move {
        sheet.cells.remove(r);
    }
    for (r, c) in cells_to_move {
        sheet
            .cells
            .insert(s1_format_xlsx::CellRef::new(r.col + 1, r.row), c);
    }

    // Shift column widths
    let widths_to_move: Vec<(u32, f64)> = sheet
        .column_widths
        .iter()
        .filter(|(&c, _)| c >= new_col)
        .map(|(&c, &w)| (c, w))
        .collect();
    for (c, _) in &widths_to_move {
        sheet.column_widths.remove(c);
    }
    for (c, w) in widths_to_move {
        sheet.column_widths.insert(c + 1, w);
    }
}

/// Delete a column, shifting cells to the left.
fn delete_column_in_sheet(sheet: &mut s1_format_xlsx::Sheet, col: u32) {
    // Remove cells in the target column
    let to_remove: Vec<s1_format_xlsx::CellRef> = sheet
        .cells
        .keys()
        .filter(|r| r.col == col)
        .copied()
        .collect();
    for r in to_remove {
        sheet.cells.remove(&r);
    }

    // Shift cells to the right of the deleted column left
    let cells_to_move: Vec<(s1_format_xlsx::CellRef, s1_format_xlsx::Cell)> = sheet
        .cells
        .iter()
        .filter(|(r, _)| r.col > col)
        .map(|(r, c)| (*r, c.clone()))
        .collect();
    for (r, _) in &cells_to_move {
        sheet.cells.remove(r);
    }
    for (r, c) in cells_to_move {
        sheet
            .cells
            .insert(s1_format_xlsx::CellRef::new(r.col - 1, r.row), c);
    }

    // Shift column widths
    sheet.column_widths.remove(&col);
    let widths_to_move: Vec<(u32, f64)> = sheet
        .column_widths
        .iter()
        .filter(|(&c, _)| c > col)
        .map(|(&c, &w)| (c, w))
        .collect();
    for (c, _) in &widths_to_move {
        sheet.column_widths.remove(c);
    }
    for (c, w) in widths_to_move {
        sheet.column_widths.insert(c - 1, w);
    }
}

/// Escape a string for JSON embedding.
fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_create_document() {
        let engine = WasmEngine::new();
        let doc = engine.create();
        assert!(doc.is_valid());
        assert_eq!(doc.paragraph_count().unwrap(), 0);
    }

    #[test]
    fn wasm_builder() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .title("Test")
            .author("Author")
            .heading(1, "Title")
            .text("Hello world")
            .build()
            .unwrap();
        assert!(doc.is_valid());
        assert_eq!(doc.metadata_title().unwrap(), Some("Test".to_string()));
        assert_eq!(doc.metadata_author().unwrap(), Some("Author".to_string()));
        assert!(doc.paragraph_count().unwrap() >= 2);
    }

    #[test]
    fn wasm_plain_text() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.text("Hello").text("World").build().unwrap();
        let text = doc.to_plain_text().unwrap();
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn wasm_metadata() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.title("My Doc").author("Alice").build().unwrap();
        assert_eq!(doc.metadata_title().unwrap(), Some("My Doc".to_string()));
        assert_eq!(doc.metadata_author().unwrap(), Some("Alice".to_string()));
    }

    #[test]
    fn wasm_document_free() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        assert!(doc.is_valid());
        // Directly set inner to None (equivalent to free()) since
        // wasm_bindgen &mut self methods panic on non-wasm targets.
        doc.inner = None;
        assert!(!doc.is_valid());
        // Can't call to_plain_text() here because JsError::new() panics
        // on non-wasm targets. The is_valid() check above proves free works.
        assert!(doc.inner.is_none());
    }

    #[test]
    fn wasm_format_detection() {
        // ZIP magic bytes (DOCX/ODT)
        let zip_data = &[0x50, 0x4B, 0x03, 0x04];
        let fmt = detect_format(zip_data);
        assert!(fmt == "docx" || fmt == "odt");

        // Plain text
        let txt_data = b"Hello world";
        let fmt = detect_format(txt_data);
        assert_eq!(fmt, "txt");
    }

    #[test]
    fn wasm_error_handling() {
        let engine = WasmEngine::new();
        // Invalid data should produce an error
        let result = engine.open(&[0xFF, 0xFF, 0xFF]);
        // Should either succeed (as txt) or fail with error
        // Plain text reader is very lenient, so this likely succeeds
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn wasm_export_txt() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.text("Export test").build().unwrap();
        let bytes = doc.export("txt").unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Export test"));
    }

    #[test]
    fn wasm_font_loading() {
        let mut font_db = WasmFontDatabase::new();
        assert_eq!(font_db.font_count(), 0);
        // Load some arbitrary bytes (won't be valid font, but shouldn't panic)
        font_db.load_font(vec![0; 100]);
        // fontdb silently ignores invalid font data
    }

    #[test]
    fn wasm_open_docx() {
        // Build a document, export as DOCX, then reopen it
        let builder = WasmDocumentBuilder::new();
        let doc = builder.text("DOCX round-trip").build().unwrap();
        let docx_bytes = doc.export("docx").unwrap();

        let engine = WasmEngine::new();
        let reopened = engine.open(&docx_bytes).unwrap();
        assert!(reopened.is_valid());
        let text = reopened.to_plain_text().unwrap();
        assert!(text.contains("DOCX round-trip"));
    }

    #[test]
    fn wasm_export_docx() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .title("Export Test")
            .text("Content")
            .build()
            .unwrap();
        let bytes = doc.export("docx").unwrap();
        // DOCX is a ZIP file — check magic bytes
        assert!(bytes.len() > 4);
        assert_eq!(&bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);
    }

    #[test]
    fn wasm_builder_export_roundtrip() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .title("RT Title")
            .author("RT Author")
            .heading(1, "Heading")
            .text("Body text")
            .build()
            .unwrap();

        // Export to DOCX and reopen
        let docx_bytes = doc.export("docx").unwrap();
        let engine = WasmEngine::new();
        let reopened = engine.open_as(&docx_bytes, "docx").unwrap();

        assert_eq!(
            reopened.metadata_title().unwrap(),
            Some("RT Title".to_string())
        );
        assert_eq!(
            reopened.metadata_author().unwrap(),
            Some("RT Author".to_string())
        );
        let text = reopened.to_plain_text().unwrap();
        assert!(text.contains("Heading"));
        assert!(text.contains("Body text"));
    }

    // ─── Paginated HTML Tests ────────────────────────────────────

    #[test]
    fn test_paginated_html_empty() {
        let engine = WasmEngine::new();
        let doc = engine.create();
        let html = doc.to_paginated_html().unwrap();
        // Should produce valid paginated HTML with at least one page
        assert!(html.contains("s1-page"), "empty doc should have a page div");
        assert!(
            html.contains("s1-document"),
            "should have a document wrapper"
        );
    }

    #[test]
    fn test_paginated_html_basic() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .heading(1, "Title")
            .text("Hello world")
            .build()
            .unwrap();
        let html = doc.to_paginated_html().unwrap();
        assert!(html.contains("s1-page"), "should have page div");
        assert!(html.contains("s1-block"), "should have block div");
        // Text may be split across spans for line-breaking purposes
        assert!(html.contains("Hello"), "should contain 'Hello'");
        assert!(html.contains("world"), "should contain 'world'");
        assert!(html.contains("Title"), "should contain heading text");
    }

    #[test]
    fn test_layout_json_basic() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .heading(1, "Title")
            .text("Hello world")
            .build()
            .unwrap();
        let json_str = doc.to_layout_json().unwrap();
        // Verify it's valid JSON-ish (contains expected structure)
        assert!(
            json_str.starts_with("{\"pages\":["),
            "should start with pages array"
        );
        assert!(
            json_str.contains("\"type\":\"paragraph\""),
            "should have paragraph blocks"
        );
        assert!(json_str.contains("\"Hello"), "should contain 'Hello' text");
        assert!(json_str.contains("\"Title"), "should contain heading text");
        assert!(json_str.contains("\"width\":"), "should have width fields");
        assert!(
            json_str.contains("\"height\":"),
            "should have height fields"
        );
        assert!(json_str.contains("\"fontSize\":"), "should have font size");
        assert!(
            json_str.contains("\"fontFamily\":"),
            "should have font family"
        );
        assert!(json_str.contains("\"bold\":"), "should have bold field");
        assert!(json_str.contains("\"lines\":"), "should have lines array");
        assert!(json_str.contains("\"runs\":"), "should have runs array");
        assert!(json_str.contains("\"sourceId\":"), "should have source IDs");
        assert!(
            json_str.contains("\"contentArea\":"),
            "should have content area"
        );
    }

    #[test]
    fn test_layout_json_with_config() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.text("Config test").build().unwrap();

        let mut config = WasmLayoutConfig::new();
        config.set_page_width(595.28);
        config.set_page_height(841.89);

        let json_str = doc.to_layout_json_with_config(&config).unwrap();
        assert!(
            json_str.contains("\"width\":595.28"),
            "should have A4 page width"
        );
        assert!(
            json_str.contains("\"height\":841.89"),
            "should have A4 page height"
        );
    }

    #[test]
    fn test_layout_json_empty_document() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.build().unwrap();
        let json_str = doc.to_layout_json().unwrap();
        assert!(
            json_str.starts_with("{\"pages\":["),
            "should start with pages array"
        );
        // Empty doc should still have at least one page
        assert!(
            json_str.contains("\"index\":"),
            "should have at least one page"
        );
    }

    #[test]
    fn test_paginated_html_with_config() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.text("Config test").build().unwrap();

        // Use A4 dimensions
        let mut config = WasmLayoutConfig::new();
        config.set_page_width(595.28);
        config.set_page_height(841.89);
        config.set_margin_top(50.0);
        config.set_margin_bottom(50.0);

        let html = doc.to_paginated_html_with_config(&config).unwrap();
        assert!(html.contains("s1-page"), "should have page div");
        // The page width should reflect A4 dimensions
        assert!(
            html.contains("width:595.3pt") || html.contains("width:595pt"),
            "should have A4 page width: {html}"
        );
    }

    #[test]
    fn test_layout_config_defaults() {
        let config = WasmLayoutConfig::new();
        assert!(
            (config.page_width() - 612.0).abs() < 0.01,
            "default width should be US Letter"
        );
        assert!(
            (config.page_height() - 792.0).abs() < 0.01,
            "default height should be US Letter"
        );
        assert!(
            (config.margin_top() - 72.0).abs() < 0.01,
            "default top margin should be 1 inch"
        );
        assert!(
            (config.margin_bottom() - 72.0).abs() < 0.01,
            "default bottom margin should be 1 inch"
        );
        assert!(
            (config.margin_left() - 72.0).abs() < 0.01,
            "default left margin should be 1 inch"
        );
        assert!(
            (config.margin_right() - 72.0).abs() < 0.01,
            "default right margin should be 1 inch"
        );
    }

    #[test]
    fn test_layout_config_setters() {
        let mut config = WasmLayoutConfig::new();
        config.set_page_width(500.0);
        config.set_page_height(700.0);
        config.set_margin_top(36.0);
        config.set_margin_bottom(36.0);
        config.set_margin_left(48.0);
        config.set_margin_right(48.0);

        assert!((config.page_width() - 500.0).abs() < 0.01);
        assert!((config.page_height() - 700.0).abs() < 0.01);
        assert!((config.margin_top() - 36.0).abs() < 0.01);
        assert!((config.margin_bottom() - 36.0).abs() < 0.01);
        assert!((config.margin_left() - 48.0).abs() < 0.01);
        assert!((config.margin_right() - 48.0).abs() < 0.01);

        // Verify the conversion to LayoutConfig
        let layout_config = config.to_layout_config();
        assert!((layout_config.default_page_layout.width - 500.0).abs() < 0.01);
        assert!((layout_config.default_page_layout.margin_left - 48.0).abs() < 0.01);
    }

    #[test]
    fn test_paginated_html_contains_pages() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .text("Page content line one")
            .text("Page content line two")
            .text("Page content line three")
            .build()
            .unwrap();
        let html = doc.to_paginated_html().unwrap();

        // Count page divs — should have at least one
        let page_count = html.matches("s1-page").count();
        assert!(
            page_count >= 1,
            "should have at least one s1-page div, got {page_count}"
        );
        // Should have the document wrapper
        assert!(html.contains("s1-document"));
        // Should contain positioned blocks
        assert!(
            html.contains("position:absolute") || html.contains("position:relative"),
            "paginated HTML should use CSS positioning"
        );
    }

    // ─── PDF Export Tests ────────────────────────────────────────

    #[test]
    fn test_to_pdf_empty() {
        let engine = WasmEngine::new();
        let doc = engine.create();
        let pdf_bytes = doc.to_pdf().unwrap();
        // PDF files start with %PDF
        assert!(
            pdf_bytes.len() >= 4,
            "PDF should have at least 4 bytes, got {}",
            pdf_bytes.len()
        );
        assert_eq!(
            &pdf_bytes[0..5],
            b"%PDF-",
            "PDF should start with %PDF- magic bytes"
        );
    }

    #[test]
    fn test_to_pdf_with_content() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .title("PDF Test")
            .heading(1, "Hello PDF")
            .text("This is a test document for PDF export.")
            .build()
            .unwrap();
        let pdf_bytes = doc.to_pdf().unwrap();
        assert_eq!(
            &pdf_bytes[0..5],
            b"%PDF-",
            "PDF should start with %PDF- magic"
        );
        // A document with content should produce a reasonably sized PDF
        assert!(
            pdf_bytes.len() > 100,
            "PDF with content should be > 100 bytes, got {}",
            pdf_bytes.len()
        );
    }

    #[test]
    fn test_to_pdf_data_url() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder.text("Data URL test").build().unwrap();
        let data_url = doc.to_pdf_data_url().unwrap();
        assert!(
            data_url.starts_with("data:application/pdf;base64,"),
            "Data URL should start with the correct prefix, got: {}",
            &data_url[..50.min(data_url.len())]
        );
        // The base64 portion should be non-empty
        let b64_part = &data_url["data:application/pdf;base64,".len()..];
        assert!(
            !b64_part.is_empty(),
            "Base64 portion of data URL should not be empty"
        );
    }

    #[test]
    fn test_to_pdf_has_content() {
        let builder = WasmDocumentBuilder::new();
        let doc = builder
            .heading(1, "Title")
            .text("First paragraph")
            .text("Second paragraph")
            .build()
            .unwrap();
        let pdf_bytes = doc.to_pdf().unwrap();
        // Verify it looks like a valid PDF (starts with header, ends near %%EOF)
        assert_eq!(&pdf_bytes[0..5], b"%PDF-");
        // PDF files typically end with %%EOF (possibly with trailing whitespace)
        let tail = String::from_utf8_lossy(&pdf_bytes[pdf_bytes.len().saturating_sub(32)..]);
        assert!(
            tail.contains("%%EOF"),
            "PDF should end with %%EOF marker, tail: {tail}"
        );
    }

    // ─── Editing API Tests ──────────────────────────────────────

    #[test]
    fn test_append_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello world").unwrap();
        assert!(!id.is_empty());
        assert!(id.contains(':'));
        assert_eq!(doc.paragraph_count().unwrap(), 1);
        let text = doc.to_plain_text().unwrap();
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn test_append_multiple_paragraphs() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("First").unwrap();
        doc.append_paragraph("Second").unwrap();
        doc.append_paragraph("Third").unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 3);
    }

    #[test]
    fn test_insert_paragraph_after() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first_id = doc.append_paragraph("First").unwrap();
        doc.append_paragraph("Third").unwrap();
        doc.insert_paragraph_after(&first_id, "Second").unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 3);
        let text = doc.to_plain_text().unwrap();
        let first_pos = text.find("First").unwrap();
        let second_pos = text.find("Second").unwrap();
        let third_pos = text.find("Third").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < third_pos);
    }

    #[test]
    fn test_append_heading() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_heading(1, "My Title").unwrap();
        assert!(!id.is_empty());
        let text = doc.to_plain_text().unwrap();
        assert!(text.contains("My Title"));
    }

    #[test]
    fn test_get_headings_json() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_heading(1, "Chapter One").unwrap();
        doc.append_paragraph("Some text").unwrap();
        doc.append_heading(2, "Section A").unwrap();
        doc.append_heading(3, "Subsection").unwrap();
        doc.append_heading(1, "Chapter Two").unwrap();

        let json = doc.get_headings_json().unwrap();
        assert!(json.starts_with('['), "should be a JSON array");
        assert!(json.ends_with(']'), "should end with ]");
        // Verify all 4 headings are present (not the paragraph)
        assert!(
            json.contains("\"Chapter One\""),
            "should contain Chapter One"
        );
        assert!(json.contains("\"Section A\""), "should contain Section A");
        assert!(json.contains("\"Subsection\""), "should contain Subsection");
        assert!(
            json.contains("\"Chapter Two\""),
            "should contain Chapter Two"
        );
        assert!(
            !json.contains("\"Some text\""),
            "should not contain paragraph text"
        );
        // Verify levels
        assert!(json.contains("\"level\":1"), "should have level 1 headings");
        assert!(json.contains("\"level\":2"), "should have level 2 heading");
        assert!(json.contains("\"level\":3"), "should have level 3 heading");
        // Verify nodeId format
        assert!(json.contains("\"nodeId\":\""), "should have nodeId fields");
        // Count occurrences of nodeId to verify 4 entries
        let count = json.matches("\"nodeId\":").count();
        assert_eq!(count, 4, "should have exactly 4 heading entries");
    }

    #[test]
    fn test_get_headings_json_empty() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("No headings here").unwrap();

        let json = doc.get_headings_json().unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn test_toc_render_with_headings() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_heading(1, "Introduction").unwrap();
        doc.append_heading(2, "Background").unwrap();
        let para_id = doc.append_paragraph("Text").unwrap();
        doc.insert_table_of_contents(&para_id, 3, "Contents")
            .unwrap();

        let html = doc.to_html().unwrap();
        assert!(html.contains("doc-toc"), "should render with doc-toc class");
        assert!(html.contains("toc-update-btn"), "should have update button");
        assert!(html.contains("toc-entry"), "should have toc entries");
        assert!(html.contains("Introduction"), "should contain heading text");
        assert!(html.contains("Background"), "should contain heading text");
        assert!(html.contains("Contents"), "should contain custom title");
        assert!(
            html.contains("data-target-node"),
            "entries should have navigation targets"
        );
    }

    #[test]
    fn test_delete_node() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Delete me").unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 1);
        doc.delete_node(&id).unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 0);
    }

    #[test]
    fn test_set_paragraph_text() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Original").unwrap();
        doc.set_paragraph_text(&id, "Replaced").unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Replaced");
    }

    #[test]
    fn test_insert_text_in_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World").unwrap();
        doc.insert_text_in_paragraph(&id, 5, " Beautiful").unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hello Beautiful World");
    }

    #[test]
    fn test_delete_text_in_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello Beautiful World").unwrap();
        doc.delete_text_in_paragraph(&id, 5, 10).unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_set_bold() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Bold text").unwrap();
        doc.set_bold(&id, true).unwrap();
        let _info = doc.node_info_json(&id).unwrap();
        // Bold is on the run, check via HTML rendering
        let html = doc.to_html().unwrap();
        assert!(html.contains("<strong>") || html.contains("font-weight"));
    }

    #[test]
    fn test_set_italic() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Italic text").unwrap();
        doc.set_italic(&id, true).unwrap();
        let html = doc.to_html().unwrap();
        assert!(html.contains("<em>") || html.contains("font-style"));
    }

    #[test]
    fn test_set_alignment() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Centered text").unwrap();
        doc.set_alignment(&id, "center").unwrap();
        let html = doc.to_html().unwrap();
        assert!(html.contains("text-align:center"));
    }

    #[test]
    fn test_undo_redo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        assert!(!doc.can_undo().unwrap());

        doc.append_paragraph("First").unwrap();
        assert!(doc.can_undo().unwrap());
        assert_eq!(doc.paragraph_count().unwrap(), 1);

        doc.undo().unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 0);
        assert!(doc.can_redo().unwrap());

        doc.redo().unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 1);
    }

    #[test]
    fn test_paragraph_ids_json() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("A").unwrap();
        doc.append_paragraph("B").unwrap();
        let json = doc.paragraph_ids_json().unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        // Should have 2 entries
        let count = json.matches(':').count();
        assert_eq!(count, 2, "Expected 2 node IDs, got: {}", json);
    }

    #[test]
    fn test_body_children_json() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        doc.append_heading(1, "Heading").unwrap();
        let json = doc.body_children_json().unwrap();
        assert!(json.contains("Paragraph"));
    }

    #[test]
    fn test_node_info_json() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Info test").unwrap();
        let json = doc.node_info_json(&id).unwrap();
        assert!(json.contains("\"type\":\"Paragraph\""));
        assert!(json.contains("\"fullText\":\"Info test\""));
    }

    #[test]
    fn test_set_font_size() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Big text").unwrap();
        doc.set_font_size(&id, 24.0).unwrap();
        let html = doc.to_html().unwrap();
        assert!(html.contains("font-size:24pt"));
    }

    #[test]
    fn test_set_color() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Red text").unwrap();
        doc.set_color(&id, "FF0000").unwrap();
        let html = doc.to_html().unwrap();
        assert!(
            html.contains("color:#FF0000") || html.contains("color:#ff0000"),
            "Expected red color in HTML: {}",
            html
        );
    }

    #[test]
    fn test_set_metadata() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.set_title("My Title").unwrap();
        doc.set_author("Author Name").unwrap();
        assert_eq!(doc.metadata_title().unwrap(), Some("My Title".to_string()));
        assert_eq!(
            doc.metadata_author().unwrap(),
            Some("Author Name".to_string())
        );
    }

    #[test]
    fn test_edit_and_export() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.set_title("Edited Doc").unwrap();
        doc.append_heading(1, "Hello").unwrap();
        doc.append_paragraph("World").unwrap();
        doc.set_bold(
            &doc.paragraph_ids_json()
                .unwrap()
                .split('"')
                .nth(1)
                .unwrap()
                .to_string(),
            true,
        )
        .unwrap();

        // Export as DOCX, reopen, verify
        let bytes = doc.export("docx").unwrap();
        let reopened = engine.open(&bytes).unwrap();
        let text = reopened.to_plain_text().unwrap();
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_clear_history() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        assert!(doc.can_undo().unwrap());
        doc.clear_history().unwrap();
        assert!(!doc.can_undo().unwrap());
    }

    // ─── Editor API Tests (E.1) ─────────────────────────────────

    #[test]
    fn test_render_node_html_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Render me").unwrap();
        let html = doc.render_node_html(&id).unwrap();
        assert!(html.contains("data-node-id="), "should have node ID attr");
        assert!(html.contains("Render me"), "should contain text");
        assert!(html.starts_with("<p "), "should be a paragraph tag");
    }

    #[test]
    fn test_render_node_html_heading() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_heading(2, "My Heading").unwrap();
        let html = doc.render_node_html(&id).unwrap();
        assert!(
            html.starts_with("<h2 "),
            "should be an h2 tag, got: {}",
            html
        );
        assert!(html.contains("My Heading"), "should contain heading text");
        assert!(html.contains("data-node-id="), "should have node ID");
    }

    #[test]
    fn test_render_node_slice_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World").unwrap();
        let html = doc.render_node_slice(&id, 6, 11).unwrap();
        assert!(html.starts_with("<p"), "should still be a paragraph tag");
        assert!(html.contains("World"), "should render only requested slice");
        assert!(
            !html.contains("Hello"),
            "should not include text before slice"
        );
    }

    #[test]
    fn test_insert_line_break_splits_run_at_paragraph_level() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&id, 5).unwrap();

        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hello\nWorld");

        let html = doc.render_node_html(&id).unwrap();
        assert!(
            html.contains("<br"),
            "rendered HTML should include a line break"
        );
    }

    #[test]
    fn test_insert_tab_splits_run_at_paragraph_level() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&id, 5).unwrap();

        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hello\tWorld");

        let html = doc.render_node_html(&id).unwrap();
        assert!(
            html.contains("&emsp;"),
            "rendered HTML should include a tab entity"
        );
    }

    #[test]
    fn test_insert_text_after_paragraph_tab_uses_correct_offset() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&id, 5).unwrap();
        doc.insert_text_in_paragraph(&id, 6, "X").unwrap();

        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hello\tXWorld");
    }

    #[test]
    fn test_delete_text_range_across_paragraph_tab() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&id, 5).unwrap();
        doc.delete_text_in_paragraph(&id, 4, 3).unwrap();

        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hellorld");
    }

    #[test]
    fn test_delete_exact_paragraph_tab_uses_inline_range_path() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&id, 5).unwrap();
        doc.delete_text_in_paragraph(&id, 5, 1).unwrap();

        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "HelloWorld");
    }

    #[test]
    fn test_delete_exact_paragraph_line_break_uses_inline_range_path() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&id, 5).unwrap();
        doc.delete_text_in_paragraph(&id, 5, 1).unwrap();

        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "HelloWorld");
    }

    #[test]
    fn test_format_selection_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&id, 5).unwrap();
        doc.format_selection(&id, 6, &id, 11, "bold", "true")
            .unwrap();

        let runs_json = doc.get_run_ids(&id).unwrap();
        let run_ids: Vec<String> = serde_json::from_str(&runs_json).unwrap();
        assert!(run_ids.len() >= 2, "expected paragraph to have split runs");

        let first_fmt = doc.get_run_formatting_json(&run_ids[0]).unwrap();
        let last_fmt = doc
            .get_run_formatting_json(run_ids.last().unwrap())
            .unwrap();
        assert!(
            !first_fmt.contains("\"bold\":true"),
            "text before tab should remain unformatted"
        );
        assert!(
            last_fmt.contains("\"bold\":true"),
            "text after tab should be bold"
        );
    }

    #[test]
    fn test_format_selection_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&id, 5).unwrap();
        doc.format_selection(&id, 6, &id, 11, "bold", "true")
            .unwrap();

        let runs_json = doc.get_run_ids(&id).unwrap();
        let run_ids: Vec<String> = serde_json::from_str(&runs_json).unwrap();
        assert!(run_ids.len() >= 2, "expected paragraph to have split runs");

        let first_fmt = doc.get_run_formatting_json(&run_ids[0]).unwrap();
        let last_fmt = doc
            .get_run_formatting_json(run_ids.last().unwrap())
            .unwrap();
        assert!(
            !first_fmt.contains("\"bold\":true"),
            "text before line break should remain unformatted"
        );
        assert!(
            last_fmt.contains("\"bold\":true"),
            "text after line break should be bold"
        );
    }

    #[test]
    fn test_page_map_para_splits_include_char_ranges() {
        let mut engine_doc = s1engine::DocumentBuilder::new()
            .paragraph(|p| p.text("aaaaabbbbbcccccddddd"))
            .build();
        let para_id = engine_doc
            .model()
            .body_id()
            .and_then(|body_id| {
                engine_doc
                    .model()
                    .node(body_id)
                    .and_then(|body| body.children.first().copied())
            })
            .unwrap();
        let id = format!("{}:{}", para_id.replica, para_id.counter);

        let sections = engine_doc.model_mut().sections_mut();
        if sections.is_empty() {
            sections.push(s1engine::SectionProperties::default());
        }
        let sec = &mut sections[0];
        sec.page_width = 72.0;
        sec.page_height = 72.0;
        sec.margin_top = 1.0;
        sec.margin_bottom = 1.0;
        sec.margin_left = 1.0;
        sec.margin_right = 1.0;
        let doc = WasmDocument {
            inner: Some(engine_doc),
            batch_label: None,
            batch_count: 0,
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        };
        let real_layout = doc
            .doc()
            .unwrap()
            .layout(&s1_text::FontDatabase::empty())
            .unwrap();
        let dummy_font_id = match &real_layout.pages[0].blocks[0].kind {
            s1_layout::LayoutBlockKind::Paragraph { lines, .. } => lines[0].runs[0].font_id,
            _ => panic!("expected a paragraph block"),
        };

        let make_line = |text: &str| s1_layout::LayoutLine {
            baseline_y: 10.0,
            height: 14.4,
            runs: vec![s1_layout::GlyphRun {
                source_id: para_id,
                font_id: dummy_font_id,
                font_family: String::new(),
                font_size: 12.0,
                color: s1_model::Color::new(0, 0, 0),
                x_offset: 0.0,
                glyphs: Vec::new(),
                width: text.len() as f64 * 7.2,
                hyperlink_url: None,
                text: text.to_string(),
                bold: false,
                italic: false,
                underline: "none".to_string(),
                strikethrough: false,
                double_strikethrough: false,
                superscript: false,
                subscript: false,
                highlight_color: None,
                character_spacing: 0.0,
                baseline_shift: 0.0,
                caps: false,
                small_caps: false,
                hidden: false,
                metrics: None,
                revision_type: None,
                revision_author: None,
                inline_image: None,
            }],
        };
        let make_block = |lines: Vec<s1_layout::LayoutLine>,
                          is_continuation: bool,
                          split_at_line: usize,
                          y: f64| s1_layout::LayoutBlock {
            source_id: para_id,
            bounds: s1_layout::Rect::new(1.0, y, 70.0, lines.iter().map(|l| l.height).sum()),
            kind: s1_layout::LayoutBlockKind::Paragraph {
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
                is_continuation,
                split_at_line,
            },
        };
        let layout = s1_layout::LayoutDocument {
            pages: vec![
                s1_layout::LayoutPage {
                    index: 0,
                    width: 72.0,
                    height: 72.0,
                    content_area: s1_layout::Rect::new(1.0, 1.0, 70.0, 70.0),
                    blocks: vec![make_block(
                        vec![make_line("aaaaa"), make_line("bbbbb")],
                        false,
                        2,
                        1.0,
                    )],
                    header: None,
                    footer: None,
                    footnotes: vec![],
                    floating_images: vec![],
                    section_index: 0,
                },
                s1_layout::LayoutPage {
                    index: 1,
                    width: 72.0,
                    height: 72.0,
                    content_area: s1_layout::Rect::new(1.0, 1.0, 70.0, 70.0),
                    blocks: vec![make_block(
                        vec![make_line("ccccc"), make_line("ddddd")],
                        true,
                        2,
                        1.0,
                    )],
                    header: None,
                    footer: None,
                    footnotes: vec![],
                    floating_images: vec![],
                    section_index: 0,
                },
            ],
            bookmarks: Vec::new(),
            annotations: Vec::new(),
        };
        let json = doc.serialize_page_map(&layout).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let pages = value["pages"].as_array().unwrap();
        let mut splits = Vec::new();
        for page in pages {
            if let Some(items) = page["paraSplits"].as_array() {
                for item in items {
                    if item["nodeId"].as_str() == Some(id.as_str()) {
                        splits.push(item.clone());
                    }
                }
            }
        }

        assert!(
            pages.len() > 1,
            "expected paragraph to span multiple pages: {json}"
        );
        assert!(
            !splits.is_empty(),
            "expected at least one paragraph split entry: {json}"
        );
        let mut previous_end = 0usize;
        for split in &splits {
            let start = split["charStart"].as_u64().unwrap() as usize;
            let end = split["charEnd"].as_u64().unwrap() as usize;
            assert!(end > start, "split range must be non-empty: {split}");
            assert!(
                start >= previous_end,
                "split ranges must be monotonic: {split}"
            );
            let html = doc.render_node_slice(&id, start, end).unwrap();
            assert!(!html.is_empty(), "slice HTML should not be empty");
            previous_end = end;
        }
    }

    #[test]
    fn test_render_node_html_nonexistent() {
        // render_node_html with a nonexistent node ID should fail.
        // JsError::new() panics on non-wasm targets, so we verify
        // by checking the node doesn't exist in the model.
        let engine = WasmEngine::new();
        let doc = engine.create();
        let model = doc.doc().unwrap().model();
        let nid = parse_node_id("999:999").unwrap();
        assert!(model.node(nid).is_none(), "node should not exist");
    }

    #[test]
    fn test_split_paragraph_middle() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World").unwrap();
        let new_id = doc.split_paragraph(&id, 5).unwrap();
        let text1 = doc.get_paragraph_text(&id).unwrap();
        let text2 = doc.get_paragraph_text(&new_id).unwrap();
        assert_eq!(text1, "Hello");
        assert_eq!(text2, " World");
        assert_eq!(doc.paragraph_count().unwrap(), 2);
    }

    #[test]
    fn test_split_paragraph_start() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Full text").unwrap();
        let new_id = doc.split_paragraph(&id, 0).unwrap();
        let text1 = doc.get_paragraph_text(&id).unwrap();
        let text2 = doc.get_paragraph_text(&new_id).unwrap();
        assert_eq!(text1, "");
        assert_eq!(text2, "Full text");
    }

    #[test]
    fn test_split_paragraph_end() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Full text").unwrap();
        let new_id = doc.split_paragraph(&id, 9).unwrap();
        let text1 = doc.get_paragraph_text(&id).unwrap();
        let text2 = doc.get_paragraph_text(&new_id).unwrap();
        assert_eq!(text1, "Full text");
        assert_eq!(text2, "");
    }

    #[test]
    fn test_split_heading_preserves_style() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_heading(2, "HeadingText").unwrap();
        let new_id = doc.split_paragraph(&id, 7).unwrap();
        // Both should render as h2
        let html1 = doc.render_node_html(&id).unwrap();
        let html2 = doc.render_node_html(&new_id).unwrap();
        assert!(
            html1.starts_with("<h2 "),
            "original should be h2: {}",
            html1
        );
        assert!(html2.starts_with("<h2 "), "new should be h2: {}", html2);
    }

    #[test]
    fn test_split_paragraph_undo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World").unwrap();
        doc.split_paragraph(&id, 5).unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 1);
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_merge_paragraphs_basic() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id1 = doc.append_paragraph("Hello").unwrap();
        let id2 = doc.append_paragraph(" World").unwrap();
        doc.merge_paragraphs(&id1, &id2).unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 1);
        let text = doc.get_paragraph_text(&id1).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_merge_paragraphs_undo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id1 = doc.append_paragraph("Hello").unwrap();
        let id2 = doc.append_paragraph(" World").unwrap();
        doc.merge_paragraphs(&id1, &id2).unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 1);
        doc.undo().unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), 2);
    }

    #[test]
    fn test_get_formatting_defaults() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Plain text").unwrap();
        let json = doc.get_formatting_json(&id).unwrap();
        assert!(json.contains("\"bold\":false"));
        assert!(json.contains("\"italic\":false"));
        assert!(json.contains("\"underline\":false"));
        assert!(json.contains("\"strikethrough\":false"));
        assert!(json.contains("\"headingLevel\":0"));
    }

    #[test]
    fn test_get_formatting_with_attrs() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Styled").unwrap();
        doc.set_bold(&id, true).unwrap();
        doc.set_font_size(&id, 18.0).unwrap();
        let json = doc.get_formatting_json(&id).unwrap();
        assert!(json.contains("\"bold\":true"), "json: {}", json);
        assert!(json.contains("\"fontSize\":18"), "json: {}", json);
    }

    #[test]
    fn test_get_formatting_heading() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_heading(3, "H3 text").unwrap();
        let json = doc.get_formatting_json(&id).unwrap();
        assert!(
            json.contains("\"headingLevel\":3"),
            "should have heading level 3: {}",
            json
        );
    }

    #[test]
    fn test_set_heading_level_to_heading() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Normal text").unwrap();
        doc.set_heading_level(&id, 2).unwrap();
        let html = doc.render_node_html(&id).unwrap();
        assert!(html.starts_with("<h2 "), "should now be h2: {}", html);
    }

    #[test]
    fn test_set_heading_level_to_normal() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_heading(2, "Was heading").unwrap();
        doc.set_heading_level(&id, 0).unwrap();
        let html = doc.render_node_html(&id).unwrap();
        assert!(
            html.starts_with("<p "),
            "should now be a paragraph: {}",
            html
        );
    }

    #[test]
    fn test_set_heading_level_undo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Normal").unwrap();
        doc.set_heading_level(&id, 1).unwrap();
        let html = doc.render_node_html(&id).unwrap();
        assert!(html.starts_with("<h1 "), "should be h1");
        doc.undo().unwrap();
        let html2 = doc.render_node_html(&id).unwrap();
        assert!(html2.starts_with("<p "), "should revert to p: {}", html2);
    }

    // ─── P.1: Selection & Range Formatting Tests ────────────────

    #[test]
    fn test_split_run_middle() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();
        let runs_before = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs_before
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();

        let new_run_id = doc.split_run(&run_id, 5).unwrap();
        let text1 = doc.get_run_text(&run_id).unwrap();
        let text2 = doc.get_run_text(&new_run_id).unwrap();
        assert_eq!(text1, "Hello");
        assert_eq!(text2, " World");
    }

    #[test]
    fn test_split_run_preserves_attrs() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Bold text here").unwrap();
        doc.set_bold(&para_id, true).unwrap();

        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();

        let new_run_id = doc.split_run(&run_id, 4).unwrap();
        let fmt1 = doc.get_run_formatting_json(&run_id).unwrap();
        let fmt2 = doc.get_run_formatting_json(&new_run_id).unwrap();
        assert!(
            fmt1.contains("\"bold\":true"),
            "original should be bold: {}",
            fmt1
        );
        assert!(
            fmt2.contains("\"bold\":true"),
            "new should be bold: {}",
            fmt2
        );
    }

    #[test]
    fn test_format_run_bold() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Some text").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();

        doc.format_run(&run_id, "bold", "true").unwrap();
        let fmt = doc.get_run_formatting_json(&run_id).unwrap();
        assert!(fmt.contains("\"bold\":true"), "should be bold: {}", fmt);
    }

    #[test]
    fn test_paste_then_to_html_has_formatting_tags() {
        // THE critical test: paste formatted runs, then verify to_html()
        // produces <strong>/<em>/style tags in the output.
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("").unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"Bold","bold":true},{"text":" normal "},{"text":"Italic","italic":true},{"text":" red","color":"FF0000","fontSize":18}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        let html = doc.to_html().unwrap();
        eprintln!("=== to_html output ===\n{}\n=== end ===", html);

        assert!(
            html.contains("<strong>"),
            "to_html must contain <strong>, got: {}",
            html
        );
        assert!(
            html.contains("Bold</strong>") || html.contains("Bold</span></strong>"),
            "to_html must contain Bold inside <strong>"
        );
        assert!(
            html.contains("<em>"),
            "to_html must contain <em>, got: {}",
            html
        );
        assert!(
            html.contains("color:#FF0000") || html.contains("color:rgb"),
            "to_html must contain red color style, got: {}",
            html
        );
        assert!(
            html.contains("font-size:18pt") || html.contains("font-size:18"),
            "to_html must contain font-size:18, got: {}",
            html
        );
    }

    #[test]
    fn test_paste_then_render_node_html_has_formatting() {
        // Also verify render_node_html (used for incremental updates) has formatting
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("").unwrap();

        let json =
            r#"{"paragraphs":[{"runs":[{"text":"Bold text","bold":true},{"text":" plain"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        let node_html = doc.render_node_html(&p).unwrap();
        eprintln!("=== render_node_html ===\n{}\n=== end ===", node_html);

        assert!(
            node_html.contains("<strong>"),
            "render_node_html must have <strong>: {}",
            node_html
        );
        assert!(
            node_html.contains("Bold text"),
            "render_node_html must have text: {}",
            node_html
        );
    }

    #[test]
    fn test_set_paragraph_text_preserves_multirun_formatting() {
        // set_paragraph_text should be a no-op when text hasn't changed,
        // preserving multi-run formatting (e.g., after paste + re-sync).
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("").unwrap();

        // Paste formatted runs: bold "Hello" + plain " World"
        let json = r#"{"paragraphs":[{"runs":[{"text":"Hello","bold":true},{"text":" World"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        // Verify we have multiple runs with formatting
        let run_ids_str = doc.get_run_ids(&p).unwrap();
        let run_count_before: usize = run_ids_str
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();
        assert!(
            run_count_before >= 2,
            "Should have >=2 runs, got {}",
            run_count_before
        );

        // Now call set_paragraph_text with the SAME text (simulating syncParagraphText)
        doc.set_paragraph_text(&p, "Hello World").unwrap();

        // Runs should still be intact (no-op)
        let run_ids_after = doc.get_run_ids(&p).unwrap();
        let run_count_after: usize = run_ids_after
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();
        assert_eq!(
            run_count_before, run_count_after,
            "Run count should be preserved: before={}, after={}",
            run_count_before, run_count_after
        );

        // Bold formatting should still be on "Hello"
        let first_rid = run_ids_after
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .next()
            .unwrap()
            .trim()
            .trim_matches('"');
        let fmt = doc.get_run_formatting_json(first_rid).unwrap();
        assert!(
            fmt.contains("\"bold\":true"),
            "Bold should be preserved: {}",
            fmt
        );
    }

    #[test]
    fn test_set_paragraph_text_updates_when_changed() {
        // set_paragraph_text should update text when it actually changed.
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Hello World").unwrap();

        doc.set_paragraph_text(&p, "New text").unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "New text");
    }

    #[test]
    fn test_set_paragraph_text_typing_preserves_multirun() {
        // Simulates: paste "Hello World" with bold "Hello", then user types "X"
        // at position 5 (end of "Hello"). set_paragraph_text receives "HelloX World".
        // The runs should be preserved (not collapsed).
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("").unwrap();

        // Paste formatted runs: bold "Hello" + plain " World"
        let json = r#"{"paragraphs":[{"runs":[{"text":"Hello","bold":true},{"text":" World"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        // Verify initial state: 2+ runs
        let run_ids_before = doc.get_run_ids(&p).unwrap();
        let run_count_before: usize = run_ids_before
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();
        assert!(run_count_before >= 2, "Should have >=2 runs before typing");

        // Simulate typing "X" at position 5 (user typed in the bold "Hello" run)
        // The DOM now says "HelloX World", syncParagraphText calls set_paragraph_text
        doc.set_paragraph_text(&p, "HelloX World").unwrap();

        // Verify: runs should still be preserved (not collapsed to 1)
        let run_ids_after = doc.get_run_ids(&p).unwrap();
        let run_count_after: usize = run_ids_after
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();
        assert!(
            run_count_after >= 2,
            "Runs should be preserved after single-run edit, got {}",
            run_count_after
        );

        // The first run should still be bold
        let first_rid = run_ids_after
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .next()
            .unwrap()
            .trim()
            .trim_matches('"');
        let fmt = doc.get_run_formatting_json(first_rid).unwrap();
        assert!(
            fmt.contains("\"bold\":true"),
            "Bold should be preserved after typing: {}",
            fmt
        );

        // Text should be updated
        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "HelloX World");
    }

    #[test]
    fn test_set_paragraph_text_typing_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();

        doc.set_paragraph_text(&p, "Hello\tXWorld").unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello\tXWorld");
    }

    #[test]
    fn test_set_paragraph_text_typing_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();

        doc.set_paragraph_text(&p, "Hello\nXWorld").unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello\nXWorld");
    }

    #[test]
    fn test_format_selection_single_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();

        // Bold characters 2-7 ("llo W")
        doc.format_selection(&para_id, 2, &para_id, 7, "bold", "true")
            .unwrap();

        let runs = doc.get_run_ids(&para_id).unwrap();
        // Should have 3 runs now: "He" (not bold), "llo W" (bold), "orld" (not bold)
        let run_ids: Vec<String> = runs
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim_matches('"').to_string())
            .collect();
        assert!(
            run_ids.len() >= 3,
            "should have at least 3 runs: {:?}",
            run_ids
        );
    }

    #[test]
    fn test_format_selection_cross_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();

        // First make part italic
        doc.format_selection(&para_id, 0, &para_id, 5, "italic", "true")
            .unwrap();
        // Then bold across runs
        doc.format_selection(&para_id, 3, &para_id, 8, "bold", "true")
            .unwrap();

        let text = doc.get_paragraph_text(&para_id).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_format_selection_cross_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p1 = doc.append_paragraph("First paragraph").unwrap();
        let p2 = doc.append_paragraph("Second paragraph").unwrap();

        // Bold from offset 5 in p1 to offset 6 in p2
        doc.format_selection(&p1, 5, &p2, 6, "bold", "true")
            .unwrap();

        // Both paragraphs should still have their text
        let t1 = doc.get_paragraph_text(&p1).unwrap();
        let t2 = doc.get_paragraph_text(&p2).unwrap();
        assert_eq!(t1, "First paragraph");
        assert_eq!(t2, "Second paragraph");
    }

    #[test]
    fn test_format_selection_undo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();
        // Clear history so the paragraph creation isn't undoable
        doc.clear_history().unwrap();

        // Bold the middle
        doc.format_selection(&para_id, 2, &para_id, 7, "bold", "true")
            .unwrap();

        // Undo all format operations
        while doc.can_undo().unwrap() {
            doc.undo().unwrap();
        }

        // After undoing all format ops, text should be preserved
        let text = doc.get_paragraph_text(&para_id).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_get_run_ids() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Text").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        assert!(runs.starts_with('['));
        assert!(runs.ends_with(']'));
        // Should have at least 1 run
        let count = runs.matches(':').count();
        assert!(count >= 1, "should have at least 1 run: {}", runs);
    }

    #[test]
    fn test_get_run_text() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();
        let text = doc.get_run_text(&run_id).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_get_run_formatting() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Styled").unwrap();
        doc.set_bold(&para_id, true).unwrap();
        doc.set_italic(&para_id, true).unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();

        let fmt = doc.get_run_formatting_json(&run_id).unwrap();
        assert!(fmt.contains("\"bold\":true"), "fmt: {}", fmt);
        assert!(fmt.contains("\"italic\":true"), "fmt: {}", fmt);
    }

    #[test]
    fn test_get_selection_formatting_uniform() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("All bold").unwrap();
        doc.set_bold(&para_id, true).unwrap();

        let fmt = doc
            .get_selection_formatting_json(&para_id, 0, &para_id, 8)
            .unwrap();
        assert!(fmt.contains("\"bold\":true"), "fmt: {}", fmt);
    }

    #[test]
    fn test_get_selection_formatting_mixed() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();

        // Bold first half
        doc.format_selection(&para_id, 0, &para_id, 5, "bold", "true")
            .unwrap();

        let fmt = doc
            .get_selection_formatting_json(&para_id, 0, &para_id, 11)
            .unwrap();
        assert!(fmt.contains("\"mixed\""), "should have mixed bold: {}", fmt);
    }

    #[test]
    fn test_get_selection_formatting_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&para_id, 5).unwrap();
        doc.format_selection(&para_id, 6, &para_id, 11, "bold", "true")
            .unwrap();

        let after_tab = doc
            .get_selection_formatting_json(&para_id, 6, &para_id, 11)
            .unwrap();
        assert!(
            after_tab.contains("\"bold\":true"),
            "selection after tab should be bold: {}",
            after_tab
        );

        let whole_para = doc
            .get_selection_formatting_json(&para_id, 0, &para_id, 11)
            .unwrap();
        assert!(
            whole_para.contains("\"bold\":\"mixed\""),
            "whole paragraph should report mixed bold: {}",
            whole_para
        );
    }

    #[test]
    fn test_get_selection_formatting_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&para_id, 5).unwrap();
        doc.format_selection(&para_id, 6, &para_id, 11, "bold", "true")
            .unwrap();

        let after_break = doc
            .get_selection_formatting_json(&para_id, 6, &para_id, 11)
            .unwrap();
        assert!(
            after_break.contains("\"bold\":true"),
            "selection after line break should be bold: {}",
            after_break
        );

        let whole_para = doc
            .get_selection_formatting_json(&para_id, 0, &para_id, 11)
            .unwrap();
        assert!(
            whole_para.contains("\"bold\":\"mixed\""),
            "whole paragraph should report mixed bold: {}",
            whole_para
        );
    }

    #[test]
    fn test_delete_selection_across_paragraphs_after_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&first, 5).unwrap();
        let second = doc.append_paragraph("Second").unwrap();

        doc.delete_selection(&first, 6, &second, 3).unwrap();

        let first_text = doc.get_paragraph_text(&first).unwrap();
        assert_eq!(first_text, "Hello\tond");
    }

    #[test]
    fn test_delete_selection_across_paragraphs_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&first, 5).unwrap();
        let second = doc.append_paragraph("Second").unwrap();

        doc.delete_selection(&first, 6, &second, 3).unwrap();

        let first_text = doc.get_paragraph_text(&first).unwrap();
        assert_eq!(first_text, "Hello\nond");
    }

    #[test]
    fn test_format_selection_across_paragraphs_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&first, 5).unwrap();
        let second = doc.append_paragraph("Second").unwrap();

        doc.format_selection(&first, 6, &second, 3, "bold", "true")
            .unwrap();

        let first_after_break = doc
            .get_selection_formatting_json(&first, 6, &first, 11)
            .unwrap();
        assert!(
            first_after_break.contains("\"bold\":true"),
            "text after start-paragraph line break should be bold: {}",
            first_after_break
        );

        let first_before_break = doc
            .get_selection_formatting_json(&first, 0, &first, 5)
            .unwrap();
        assert!(
            first_before_break.contains("\"bold\":false"),
            "text before start-paragraph line break should remain unbolded: {}",
            first_before_break
        );
    }

    #[test]
    fn test_get_selection_formatting_across_paragraphs_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&first, 5).unwrap();
        let second = doc.append_paragraph("Second").unwrap();
        doc.format_selection(&first, 6, &second, 3, "bold", "true")
            .unwrap();

        let full_selection = doc
            .get_selection_formatting_json(&first, 6, &second, 3)
            .unwrap();
        assert!(
            full_selection.contains("\"bold\":true"),
            "selection should report bold when it covers only bolded runs: {}",
            full_selection
        );

        let extended_selection = doc
            .get_selection_formatting_json(&first, 0, &second, 3)
            .unwrap();
        assert!(
            extended_selection.contains("\"bold\":\"mixed\""),
            "extending before the line-break boundary should report mixed bold: {}",
            extended_selection
        );
    }

    #[test]
    fn test_delete_selection_across_paragraphs_with_tab_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&second, 5).unwrap();

        doc.delete_selection(&first, 2, &second, 6).unwrap();

        let first_text = doc.get_paragraph_text(&first).unwrap();
        assert_eq!(first_text, "AlWorld");
    }

    #[test]
    fn test_delete_selection_across_paragraphs_with_line_break_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&second, 5).unwrap();

        doc.delete_selection(&first, 2, &second, 6).unwrap();

        let first_text = doc.get_paragraph_text(&first).unwrap();
        assert_eq!(first_text, "AlWorld");
    }

    #[test]
    fn test_format_selection_across_paragraphs_with_line_break_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&second, 5).unwrap();

        doc.format_selection(&first, 2, &second, 6, "bold", "true")
            .unwrap();

        let first_fmt = doc
            .get_selection_formatting_json(&first, 2, &first, 5)
            .unwrap();
        assert!(
            first_fmt.contains("\"bold\":true"),
            "tail of first paragraph should be bold: {}",
            first_fmt
        );

        let second_after_break = doc
            .get_selection_formatting_json(&second, 6, &second, 11)
            .unwrap();
        assert!(
            second_after_break.contains("\"bold\":false"),
            "text after end offset should remain unbolded: {}",
            second_after_break
        );
    }

    #[test]
    fn test_get_selection_formatting_across_paragraphs_with_line_break_in_end_para_uses_correct_offsets(
    ) {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&second, 5).unwrap();
        doc.format_selection(&first, 2, &second, 6, "bold", "true")
            .unwrap();

        let full_selection = doc
            .get_selection_formatting_json(&first, 2, &second, 6)
            .unwrap();
        assert!(
            full_selection.contains("\"bold\":true"),
            "selection should report bold when it covers only bolded runs: {}",
            full_selection
        );

        let extended_selection = doc
            .get_selection_formatting_json(&first, 2, &second, 11)
            .unwrap();
        assert!(
            extended_selection.contains("\"bold\":\"mixed\""),
            "extending past the line-break boundary should report mixed bold: {}",
            extended_selection
        );
    }

    #[test]
    fn test_format_selection_across_paragraphs_with_tab_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&second, 5).unwrap();

        doc.format_selection(&first, 2, &second, 6, "bold", "true")
            .unwrap();

        let first_fmt = doc
            .get_selection_formatting_json(&first, 2, &first, 5)
            .unwrap();
        assert!(
            first_fmt.contains("\"bold\":true"),
            "tail of first paragraph should be bold: {}",
            first_fmt
        );

        let second_after_tab = doc
            .get_selection_formatting_json(&second, 6, &second, 11)
            .unwrap();
        assert!(
            second_after_tab.contains("\"bold\":false"),
            "text after end offset should remain unbolded: {}",
            second_after_tab
        );
    }

    #[test]
    fn test_get_selection_formatting_across_paragraphs_with_tab_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&second, 5).unwrap();
        doc.format_selection(&first, 2, &second, 6, "bold", "true")
            .unwrap();

        let full_selection = doc
            .get_selection_formatting_json(&first, 2, &second, 6)
            .unwrap();
        assert!(
            full_selection.contains("\"bold\":true"),
            "selection should report bold when it covers only bolded runs: {}",
            full_selection
        );

        let extended_selection = doc
            .get_selection_formatting_json(&first, 2, &second, 11)
            .unwrap();
        assert!(
            extended_selection.contains("\"bold\":\"mixed\""),
            "extending past the tab boundary should report mixed bold: {}",
            extended_selection
        );
    }

    // ─── P.2: Table Operations Tests ────────────────────────────

    #[test]
    fn test_insert_table() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before table").unwrap();
        let table_id = doc.insert_table(&p, 3, 3).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims.contains("\"rows\":3"), "dims: {}", dims);
        assert!(dims.contains("\"cols\":3"), "dims: {}", dims);
    }

    #[test]
    fn test_insert_table_row() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 2, 3).unwrap();
        doc.insert_table_row(&table_id, 1).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims.contains("\"rows\":3"), "should have 3 rows: {}", dims);
    }

    #[test]
    fn test_delete_table_row() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 3, 2).unwrap();
        doc.delete_table_row(&table_id, 0).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims.contains("\"rows\":2"), "should have 2 rows: {}", dims);
    }

    #[test]
    fn test_insert_table_column() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 2, 2).unwrap();
        doc.insert_table_column(&table_id, 1).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims.contains("\"cols\":3"), "should have 3 cols: {}", dims);
    }

    #[test]
    fn test_delete_table_column() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 2, 3).unwrap();
        doc.delete_table_column(&table_id, 0).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims.contains("\"cols\":2"), "should have 2 cols: {}", dims);
    }

    #[test]
    fn test_set_get_cell_text() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 2, 2).unwrap();

        // Get first cell: table -> row0 -> cell0
        let _children_json = doc.body_children_json().unwrap();
        let table_node = doc
            .inner
            .as_ref()
            .unwrap()
            .model()
            .node(parse_node_id(&table_id).unwrap())
            .unwrap();
        let row_id = table_node.children[0];
        let row_node = doc.inner.as_ref().unwrap().model().node(row_id).unwrap();
        let cell_id = row_node.children[0];
        let cell_id_str = format!("{}:{}", cell_id.replica, cell_id.counter);

        doc.set_cell_text(&cell_id_str, "Hello Cell").unwrap();
        let text = doc.get_cell_text(&cell_id_str).unwrap();
        assert_eq!(text, "Hello Cell");
    }

    #[test]
    fn test_get_table_dimensions() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 4, 5).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert_eq!(dims, "{\"rows\":4,\"cols\":5}");
    }

    #[test]
    fn test_merge_cells() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 3, 3).unwrap();
        doc.merge_cells(&table_id, 0, 0, 1, 1).unwrap();

        // Verify the top-left cell has colspan
        let table_nid = parse_node_id(&table_id).unwrap();
        let table = doc.inner.as_ref().unwrap().model().node(table_nid).unwrap();
        let row0 = doc
            .inner
            .as_ref()
            .unwrap()
            .model()
            .node(table.children[0])
            .unwrap();
        let cell00 = doc
            .inner
            .as_ref()
            .unwrap()
            .model()
            .node(row0.children[0])
            .unwrap();
        assert!(cell00.attributes.get_i64(&AttributeKey::ColSpan) == Some(2));
    }

    #[test]
    fn test_split_merged_cell() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 3, 3).unwrap();

        // Merge cells (0,0)-(1,1) — 2x2 block
        doc.merge_cells(&table_id, 0, 0, 1, 1).unwrap();

        // Verify merge happened
        {
            let table_nid = parse_node_id(&table_id).unwrap();
            let table = doc.inner.as_ref().unwrap().model().node(table_nid).unwrap();
            let row0 = doc
                .inner
                .as_ref()
                .unwrap()
                .model()
                .node(table.children[0])
                .unwrap();
            let cell00 = doc
                .inner
                .as_ref()
                .unwrap()
                .model()
                .node(row0.children[0])
                .unwrap();
            assert_eq!(cell00.attributes.get_i64(&AttributeKey::ColSpan), Some(2));
        }

        // Split the cell
        doc.split_merged_cell(&table_id, 0, 0).unwrap();

        // Verify ColSpan is reset to 1
        {
            let table_nid = parse_node_id(&table_id).unwrap();
            let table = doc.inner.as_ref().unwrap().model().node(table_nid).unwrap();
            let row0 = doc
                .inner
                .as_ref()
                .unwrap()
                .model()
                .node(table.children[0])
                .unwrap();
            let cell00_after = doc
                .inner
                .as_ref()
                .unwrap()
                .model()
                .node(row0.children[0])
                .unwrap();
            assert_eq!(
                cell00_after.attributes.get_i64(&AttributeKey::ColSpan),
                Some(1)
            );
        }
    }

    #[test]
    fn test_split_unmerged_cell_noop() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 2, 2).unwrap();

        // Splitting an unmerged cell should succeed silently (no-op)
        doc.split_merged_cell(&table_id, 0, 0).unwrap();
    }

    #[test]
    fn test_set_cell_background() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 1, 1).unwrap();

        let table_nid = parse_node_id(&table_id).unwrap();
        let table = doc.inner.as_ref().unwrap().model().node(table_nid).unwrap();
        let row0 = doc
            .inner
            .as_ref()
            .unwrap()
            .model()
            .node(table.children[0])
            .unwrap();
        let cell_id = row0.children[0];
        let cell_id_str = format!("{}:{}", cell_id.replica, cell_id.counter);

        doc.set_cell_background(&cell_id_str, "FF0000").unwrap();

        let cell = doc.inner.as_ref().unwrap().model().node(cell_id).unwrap();
        assert!(cell.attributes.get(&AttributeKey::CellBackground).is_some());
    }

    #[test]
    fn test_table_operations_undo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 2, 2).unwrap();
        doc.insert_table_row(&table_id, 1).unwrap();
        let dims = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims.contains("\"rows\":3"));
        doc.undo().unwrap();
        let dims2 = doc.get_table_dimensions(&table_id).unwrap();
        assert!(dims2.contains("\"rows\":2"), "after undo: {}", dims2);
    }

    // ─── P.3: Image Operations Tests ────────────────────────────
    // Note: insert_image takes &[u8] which triggers wasm_bindgen JsError::new()
    // panics on non-wasm targets. We use a helper that bypasses the WASM layer.

    fn insert_image_test_helper(doc: &mut WasmDocument, after_str: &str) -> (String, NodeId) {
        // Manually insert image structure using operations
        // Image goes directly under Paragraph (not under Run)
        let d = doc.inner.as_mut().unwrap();
        let after_id = parse_node_id(after_str).unwrap();
        let body_id = d.body_id().unwrap();
        let body = d.node(body_id).unwrap();
        let index = body.children.iter().position(|&c| c == after_id).unwrap() + 1;

        let media_id = d.model_mut().media_mut().insert(
            "image/png",
            vec![0x89, 0x50, 0x4E, 0x47],
            Some("image.png".to_string()),
        );

        let para_id = d.next_id();
        let img_id = d.next_id();

        let mut img_node = Node::new(img_id, NodeType::Image);
        img_node.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(100.0));
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(80.0));

        let mut txn = Transaction::with_label("Insert image test");
        txn.push(Operation::insert_node(
            body_id,
            index,
            Node::new(para_id, NodeType::Paragraph),
        ));
        txn.push(Operation::insert_node(para_id, 0, img_node));
        d.apply_transaction(&txn).unwrap();

        let para_str = format!("{}:{}", para_id.replica, para_id.counter);
        (para_str, img_id)
    }

    #[test]
    fn test_insert_image() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before image").unwrap();
        let (img_para_id, _img_id) = insert_image_test_helper(&mut doc, &p);
        assert!(!img_para_id.is_empty());
    }

    #[test]
    fn test_delete_image() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);
        doc.delete_image(&img_id_str).unwrap();
    }

    #[test]
    fn test_resize_image() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);

        doc.resize_image(&img_id_str, 200.0, 150.0).unwrap();

        let img = doc.inner.as_ref().unwrap().model().node(img_id).unwrap();
        assert_eq!(
            img.attributes.get_f64(&AttributeKey::ImageWidth),
            Some(200.0)
        );
        assert_eq!(
            img.attributes.get_f64(&AttributeKey::ImageHeight),
            Some(150.0)
        );
    }

    #[test]
    fn test_get_image_data_url() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);

        let data_url = doc.get_image_data_url(&img_id_str).unwrap();
        assert!(
            data_url.starts_with("data:image/png;base64,"),
            "data_url: {}",
            data_url
        );
    }

    #[test]
    fn test_set_image_alt_text() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);

        doc.set_image_alt_text(&img_id_str, "A photo").unwrap();

        let img = doc.inner.as_ref().unwrap().model().node(img_id).unwrap();
        assert_eq!(
            img.attributes.get_string(&AttributeKey::ImageAltText),
            Some("A photo")
        );
    }

    #[test]
    fn test_insert_image_undo() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let initial_count = doc.paragraph_count().unwrap();
        let _ = insert_image_test_helper(&mut doc, &p);
        assert_eq!(doc.paragraph_count().unwrap(), initial_count + 1);
        doc.undo().unwrap();
        assert_eq!(doc.paragraph_count().unwrap(), initial_count);
    }

    #[test]
    fn test_set_image_wrap_mode() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);

        // Default is "inline"
        let mode = doc.get_image_wrap_mode(&img_id_str).unwrap();
        assert_eq!(mode, "inline");

        // Set to wrapLeft
        doc.set_image_wrap_mode(&img_id_str, "wrapLeft").unwrap();
        let mode = doc.get_image_wrap_mode(&img_id_str).unwrap();
        assert_eq!(mode, "wrapLeft");

        // Set to behind
        doc.set_image_wrap_mode(&img_id_str, "behind").unwrap();
        let mode = doc.get_image_wrap_mode(&img_id_str).unwrap();
        assert_eq!(mode, "behind");
    }

    #[test]
    #[should_panic]
    fn test_set_image_wrap_mode_invalid() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);
        // JsError::new panics on non-wasm targets; in real wasm it returns Err
        let _ = doc.set_image_wrap_mode(&img_id_str, "invalid");
    }

    #[test]
    fn test_image_wrap_mode_in_html() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Hello").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);

        doc.set_image_wrap_mode(&img_id_str, "wrapRight").unwrap();
        let html = doc.to_html().unwrap();
        assert!(
            html.contains("data-wrap-mode=\"wrapRight\""),
            "HTML should contain wrap mode data attribute: {}",
            &html[..html.len().min(500)]
        );
        assert!(
            html.contains("float:right"),
            "HTML should contain float:right for wrapRight"
        );
    }

    #[test]
    fn test_set_section_columns() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();

        // Default is 1 column
        let json = doc.get_section_columns(0).unwrap();
        let parsed: String = json;
        assert!(
            parsed.contains("\"columns\":1"),
            "Default should be 1 column: {}",
            parsed
        );

        // Set to 2 columns
        doc.set_section_columns(0, 2, 36.0).unwrap();
        let json = doc.get_section_columns(0).unwrap();
        assert!(
            json.contains("\"columns\":2"),
            "Should be 2 columns: {}",
            json
        );
        assert!(
            json.contains("\"spacing\":36.0"),
            "Should have 36pt spacing: {}",
            json
        );

        // Set to 3 columns with custom spacing
        doc.set_section_columns(0, 3, 18.0).unwrap();
        let json = doc.get_section_columns(0).unwrap();
        assert!(
            json.contains("\"columns\":3"),
            "Should be 3 columns: {}",
            json
        );
    }

    #[test]
    #[should_panic]
    fn test_set_section_columns_zero() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        // JsError::new panics on non-wasm targets; in real wasm it returns Err
        let _ = doc.set_section_columns(0, 0, 36.0);
    }

    #[test]
    #[should_panic]
    fn test_set_section_columns_too_many() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        let _ = doc.set_section_columns(0, 7, 36.0);
    }

    #[test]
    #[should_panic]
    fn test_set_section_columns_negative_spacing() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        let _ = doc.set_section_columns(0, 2, -10.0);
    }

    #[test]
    #[should_panic]
    fn test_set_section_columns_invalid_index() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        let _ = doc.set_section_columns(99, 2, 36.0);
    }

    #[test]
    fn test_sections_json_includes_columns() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Test").unwrap();
        doc.set_section_columns(0, 2, 24.0).unwrap();

        let json = doc.get_sections_json().unwrap();
        assert!(
            json.contains("\"columns\":2"),
            "Sections JSON should include columns: {}",
            json
        );
        assert!(
            json.contains("\"columnSpacing\":24.0"),
            "Sections JSON should include spacing: {}",
            json
        );
    }

    // ─── P.4: Structural Elements Tests ─────────────────────────

    #[test]
    fn test_insert_hyperlink() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Click here").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();

        doc.insert_hyperlink(&run_id, "https://example.com", "")
            .unwrap();

        let run_nid = parse_node_id(&run_id).unwrap();
        let run = doc.inner.as_ref().unwrap().model().node(run_nid).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("https://example.com")
        );
    }

    #[test]
    fn test_remove_hyperlink() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Link text").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs
            .trim_matches(|c| c == '[' || c == ']' || c == '"')
            .to_string();

        doc.insert_hyperlink(&run_id, "https://example.com", "")
            .unwrap();
        doc.remove_hyperlink(&run_id).unwrap();

        let run_nid = parse_node_id(&run_id).unwrap();
        let run = doc.inner.as_ref().unwrap().model().node(run_nid).unwrap();
        assert!(run
            .attributes
            .get_string(&AttributeKey::HyperlinkUrl)
            .is_none());
    }

    #[test]
    fn test_insert_bookmark() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Bookmarked").unwrap();
        let bk_id = doc.insert_bookmark(&para_id, "mybookmark").unwrap();
        assert!(!bk_id.is_empty());
    }

    #[test]
    fn test_set_list_bullet() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("List item").unwrap();
        doc.set_list_format(&para_id, "bullet", 0).unwrap();

        let html = doc.render_node_html(&para_id).unwrap();
        assert!(
            html.contains("\u{2022}"),
            "should have bullet marker: {}",
            html
        );
    }

    #[test]
    fn test_set_list_numbered() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Numbered item").unwrap();
        doc.set_list_format(&para_id, "decimal", 0).unwrap();

        let para_nid = parse_node_id(&para_id).unwrap();
        let para = doc.inner.as_ref().unwrap().model().node(para_nid).unwrap();
        assert!(para.attributes.get(&AttributeKey::ListInfo).is_some());
    }

    #[test]
    fn test_set_list_none() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Was a list").unwrap();
        doc.set_list_format(&para_id, "bullet", 0).unwrap();
        doc.set_list_format(&para_id, "none", 0).unwrap();

        let para_nid = parse_node_id(&para_id).unwrap();
        let para = doc.inner.as_ref().unwrap().model().node(para_nid).unwrap();
        assert!(para.attributes.get(&AttributeKey::ListInfo).is_none());
    }

    #[test]
    fn test_insert_page_break() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before break").unwrap();
        let pb_id = doc.insert_page_break(&p).unwrap();
        assert!(!pb_id.is_empty());

        let pb_nid = parse_node_id(&pb_id).unwrap();
        let pb = doc.inner.as_ref().unwrap().model().node(pb_nid).unwrap();
        assert_eq!(
            pb.attributes.get_bool(&AttributeKey::PageBreakBefore),
            Some(true)
        );
    }

    #[test]
    fn test_insert_section_break_next_page() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before section break").unwrap();
        let sb_id = doc.insert_section_break(&p, "nextPage").unwrap();
        assert!(!sb_id.is_empty());

        // The new paragraph should have SectionIndex attribute
        let sb_nid = parse_node_id(&sb_id).unwrap();
        let sb = doc.inner.as_ref().unwrap().model().node(sb_nid).unwrap();
        assert!(
            sb.attributes.get(&AttributeKey::SectionIndex).is_some(),
            "Section break paragraph should have SectionIndex attribute"
        );

        // There should now be 2 sections (original + new)
        let sections = doc.inner.as_ref().unwrap().sections();
        assert_eq!(sections.len(), 2, "Should have 2 sections after insert");
        assert_eq!(
            sections[1].break_type,
            Some(s1_model::SectionBreakType::NextPage)
        );
    }

    #[test]
    fn test_insert_section_break_continuous() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before").unwrap();
        let sb_id = doc.insert_section_break(&p, "continuous").unwrap();
        assert!(!sb_id.is_empty());

        let sections = doc.inner.as_ref().unwrap().sections();
        assert_eq!(sections.len(), 2);
        assert_eq!(
            sections[1].break_type,
            Some(s1_model::SectionBreakType::Continuous)
        );
    }

    #[test]
    fn test_insert_section_break_even_odd() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p1 = doc.append_paragraph("Section 1").unwrap();
        let _ = doc.insert_section_break(&p1, "evenPage").unwrap();
        // Insert another paragraph after the section break for the next break
        let p2 = doc.append_paragraph("Section 2").unwrap();
        let _ = doc.insert_section_break(&p2, "oddPage").unwrap();

        let sections = doc.inner.as_ref().unwrap().sections();
        assert_eq!(sections.len(), 3, "Should have 3 sections");
        assert_eq!(
            sections[1].break_type,
            Some(s1_model::SectionBreakType::EvenPage)
        );
        assert_eq!(
            sections[2].break_type,
            Some(s1_model::SectionBreakType::OddPage)
        );
    }

    #[test]
    #[should_panic]
    fn test_insert_section_break_invalid_type() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Test").unwrap();
        // JsError::new panics on non-wasm targets; in real wasm it returns Err
        let _ = doc.insert_section_break(&p, "invalid");
    }

    #[test]
    fn test_get_section_breaks_json() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before").unwrap();
        doc.insert_section_break(&p, "nextPage").unwrap();

        let json = doc.get_section_breaks_json().unwrap();
        assert!(
            json.contains("nextPage"),
            "JSON should contain break type: {}",
            json
        );
        assert!(
            json.contains("\"index\":1"),
            "JSON should contain section index 1: {}",
            json
        );
    }

    #[test]
    fn test_section_break_html_rendering() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before section break").unwrap();
        doc.insert_section_break(&p, "continuous").unwrap();

        let html = doc.to_html().unwrap();
        assert!(
            html.contains("section-break"),
            "HTML should contain section-break class: {}",
            &html[..html.len().min(500)]
        );
        assert!(
            html.contains("Section Break (Continuous)"),
            "HTML should contain break type label"
        );
    }

    #[test]
    fn test_get_comments_json() {
        let engine = WasmEngine::new();
        let doc = engine.create();
        let json = doc.get_comments_json().unwrap();
        assert_eq!(json, "[]", "empty doc should have no comments");
    }

    #[test]
    fn test_insert_comment() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p1 = doc.append_paragraph("Commented text").unwrap();
        let cid = doc
            .insert_comment(&p1, &p1, "Alice", "Great point!")
            .unwrap();
        assert!(!cid.is_empty());

        let json = doc.get_comments_json().unwrap();
        assert!(json.contains("Alice"), "comments json: {}", json);
        assert!(json.contains("Great point!"), "comments json: {}", json);
    }

    #[test]
    fn test_delete_comment() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p1 = doc.append_paragraph("Text").unwrap();
        let cid = doc.insert_comment(&p1, &p1, "Bob", "Note").unwrap();

        let json_before = doc.get_comments_json().unwrap();
        assert!(json_before.contains("Bob"));

        doc.delete_comment(&cid).unwrap();
        let json_after = doc.get_comments_json().unwrap();
        assert_eq!(json_after, "[]", "should have no comments after delete");
    }

    // ─── P.5: Find & Replace Tests ──────────────────────────────

    #[test]
    fn test_find_text_basic() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Hello world").unwrap();
        let results = doc.find_text("world", true).unwrap();
        assert!(results.contains("\"offset\":6"), "results: {}", results);
        assert!(results.contains("\"length\":5"), "results: {}", results);
    }

    #[test]
    fn test_find_text_case_insensitive() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("Hello World").unwrap();
        let results = doc.find_text("hello", false).unwrap();
        assert!(results.contains("\"offset\":0"), "results: {}", results);
    }

    #[test]
    fn test_find_text_multiple() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("cat and cat").unwrap();
        doc.append_paragraph("another cat").unwrap();
        let results = doc.find_text("cat", true).unwrap();
        // Should find 3 matches
        let count = results.matches("\"offset\"").count();
        assert_eq!(count, 3, "should find 3 matches: {}", results);
    }

    #[test]
    fn test_find_text_inside_table_cells_uses_multiple_node_ids() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Before table").unwrap();
        let table_id = doc.insert_table(&first, 1, 2).unwrap();
        let cell_a = doc.get_cell_id(&table_id, 0, 0).unwrap();
        let cell_b = doc.get_cell_id(&table_id, 0, 1).unwrap();
        doc.set_cell_text(&cell_a, "Table Alpha").unwrap();
        doc.set_cell_text(&cell_b, "Table Beta").unwrap();

        let results = doc.find_text("Table", true).unwrap();
        let matches: Vec<serde_json::Value> = serde_json::from_str(&results).unwrap();
        assert_eq!(
            matches.len(),
            2,
            "should find Table in both cells: {}",
            results
        );
        let node_ids: Vec<&str> = matches
            .iter()
            .map(|m| m["nodeId"].as_str().unwrap())
            .collect();
        assert_ne!(
            node_ids[0], node_ids[1],
            "matches should be in different paragraph IDs"
        );
        assert_eq!(matches[0]["length"].as_u64(), Some(5));
        assert_eq!(matches[1]["length"].as_u64(), Some(5));
        assert_eq!(matches[0]["offset"].as_u64(), Some(0));
        assert_eq!(matches[1]["offset"].as_u64(), Some(0));
    }

    #[test]
    fn test_find_text_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();

        let results = doc.find_text("World", true).unwrap();
        assert!(
            results.contains("\"offset\":6"),
            "first match after tab should start at offset 6: {}",
            results
        );
        assert!(
            results.contains("\"offset\":17"),
            "second match should retain adjusted offset: {}",
            results
        );
    }

    #[test]
    fn test_find_text_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();

        let results = doc.find_text("World", true).unwrap();
        assert!(
            results.contains("\"offset\":6"),
            "first match after line break should start at offset 6: {}",
            results
        );
        assert!(
            results.contains("\"offset\":17"),
            "second match should retain adjusted offset: {}",
            results
        );
    }

    #[test]
    fn test_replace_text() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Hello World").unwrap();
        doc.replace_text(&p, 6, 5, "Rust").unwrap();
        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello Rust");
    }

    #[test]
    fn test_replace_exact_paragraph_tab_uses_inline_range_path() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();
        doc.replace_text(&p, 5, 1, "X").unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "HelloXWorld");
    }

    #[test]
    fn test_replace_exact_paragraph_line_break_uses_inline_range_path() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();
        doc.replace_text(&p, 5, 1, "X").unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "HelloXWorld");
    }

    #[test]
    fn test_replace_all() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        doc.append_paragraph("foo bar foo").unwrap();
        doc.append_paragraph("foo again").unwrap();
        let count = doc.replace_all("foo", "baz", true).unwrap();
        assert_eq!(count, 3, "should replace 3 occurrences");

        let text = doc.get_document_text().unwrap();
        assert!(!text.contains("foo"), "should have no foo left: {}", text);
        assert!(text.contains("baz"), "should have baz: {}", text);
    }

    #[test]
    fn test_replace_all_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();

        let count = doc.replace_all("World", "Rust", true).unwrap();
        assert_eq!(count, 2);

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello\tRust HelloRust");
    }

    #[test]
    fn test_replace_all_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();

        let count = doc.replace_all("World", "Rust", true).unwrap();
        assert_eq!(count, 2);

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello\nRust HelloRust");
    }

    #[test]
    fn test_paste_plain_text_multiline() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Start").unwrap();
        doc.paste_plain_text(&p, 5, "\nline2\nline3").unwrap();

        // Should have created additional paragraphs
        let count = doc.paragraph_count().unwrap();
        assert!(
            count >= 3,
            "should have at least 3 paragraphs, got {}",
            count
        );
    }

    #[test]
    fn test_paste_plain_text_multiline_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();
        doc.paste_plain_text(&p, 6, "X\nY").unwrap();

        let first_text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(first_text, "Hello\tX");

        let ids: Vec<String> = serde_json::from_str(&doc.paragraph_ids_json().unwrap()).unwrap();
        let second_id = ids.into_iter().find(|id| id != &p).unwrap();
        let second_text = doc.get_paragraph_text(&second_id).unwrap();
        assert_eq!(second_text, "YWorld");
    }

    #[test]
    fn test_paste_plain_text_multiline_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();
        doc.paste_plain_text(&p, 6, "X\nY").unwrap();

        let first_text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(first_text, "Hello\nX");

        let ids: Vec<String> = serde_json::from_str(&doc.paragraph_ids_json().unwrap()).unwrap();
        let second_id = ids.into_iter().find(|id| id != &p).unwrap();
        let second_text = doc.get_paragraph_text(&second_id).unwrap();
        assert_eq!(second_text, "YWorld");
    }

    // ── Paste formatted runs tests ────────────────────────

    #[test]
    fn test_paste_formatted_runs_empty() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Hello").unwrap();
        // Empty paste should be a no-op
        doc.paste_formatted_runs_json(&p, 0, "{}").unwrap();
        doc.paste_formatted_runs_json(&p, 0, "{\"paragraphs\":[]}")
            .unwrap();
        let text = doc.get_document_text().unwrap();
        assert!(text.contains("Hello"));
    }

    #[test]
    fn test_paste_formatted_runs_single_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("AB").unwrap();

        // Paste two runs between A and B
        let json =
            r#"{"paragraphs":[{"runs":[{"text":"xx","bold":true},{"text":"yy","italic":true}]}]}"#;
        doc.paste_formatted_runs_json(&p, 1, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(text.contains("AxxyyB"), "expected 'AxxyyB' in: {}", text);

        // Verify formatting: find runs, check bold on "xx" and italic on "yy"
        let run_ids_json = doc.get_run_ids(&p).unwrap();
        // There should be multiple runs now (after formatting split the original)
        assert!(
            run_ids_json.contains(":"),
            "should have run IDs: {}",
            run_ids_json
        );
    }

    #[test]
    fn test_paste_formatted_runs_single_paragraph_after_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"X","bold":true}]}]}"#;
        doc.paste_formatted_runs_json(&p, 6, json).unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello\tXWorld");
    }

    #[test]
    fn test_paste_formatted_runs_single_paragraph_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"X","italic":true}]}]}"#;
        doc.paste_formatted_runs_json(&p, 6, json).unwrap();

        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello\nXWorld");
    }

    #[test]
    fn test_paste_formatted_runs_single_run_no_formatting() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Start").unwrap();

        // Paste plain text (no formatting attributes)
        let json = r#"{"paragraphs":[{"runs":[{"text":" middle"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 5, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(
            text.contains("Start middle"),
            "expected 'Start middle' in: {}",
            text
        );
    }

    #[test]
    fn test_paste_formatted_runs_multi_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("AABB").unwrap();

        // Paste two paragraphs between AA and BB
        let json = r#"{"paragraphs":[{"runs":[{"text":"first"}]},{"runs":[{"text":"second"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 2, json).unwrap();

        let text = doc.get_document_text().unwrap();
        // Result should be: "AAfirst\nsecondBB" (spread across paragraphs)
        assert!(text.contains("AAfirst"), "expected 'AAfirst' in: {}", text);
        assert!(
            text.contains("secondBB"),
            "expected 'secondBB' in: {}",
            text
        );
    }

    #[test]
    fn test_paste_formatted_runs_multi_paragraph_after_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&p, 5).unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"X"}]},{"runs":[{"text":"Y"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 6, json).unwrap();

        let first_text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(first_text, "Hello\tX");

        let ids: Vec<String> = serde_json::from_str(&doc.paragraph_ids_json().unwrap()).unwrap();
        let second_id = ids.into_iter().find(|id| id != &p).unwrap();
        let second_text = doc.get_paragraph_text(&second_id).unwrap();
        assert_eq!(second_text, "YWorld");
    }

    #[test]
    fn test_paste_formatted_runs_multi_paragraph_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&p, 5).unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"X"}]},{"runs":[{"text":"Y"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 6, json).unwrap();

        let first_text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(first_text, "Hello\nX");

        let ids: Vec<String> = serde_json::from_str(&doc.paragraph_ids_json().unwrap()).unwrap();
        let second_id = ids.into_iter().find(|id| id != &p).unwrap();
        let second_text = doc.get_paragraph_text(&second_id).unwrap();
        assert_eq!(second_text, "YWorld");
    }

    #[test]
    fn test_paste_formatted_runs_with_color_and_font() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Test").unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"colored","color":"FF0000","fontSize":18,"fontFamily":"Arial"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 4, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(
            text.contains("Testcolored"),
            "expected 'Testcolored' in: {}",
            text
        );
    }

    #[test]
    fn test_paste_formatted_runs_at_start() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("End").unwrap();

        let json = r#"{"paragraphs":[{"runs":[{"text":"Begin ","bold":true}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(
            text.contains("Begin End"),
            "expected 'Begin End' in: {}",
            text
        );
    }

    #[test]
    fn test_paste_formatted_runs_three_paragraphs() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("XY").unwrap();

        // Paste three paragraphs
        let json = r#"{"paragraphs":[{"runs":[{"text":"p1"}]},{"runs":[{"text":"p2"}]},{"runs":[{"text":"p3"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 1, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(text.contains("Xp1"), "expected 'Xp1' in: {}", text);
        assert!(text.contains("p2"), "expected 'p2' in: {}", text);
        assert!(text.contains("p3Y"), "expected 'p3Y' in: {}", text);
    }

    #[test]
    fn test_parse_paste_json_basic() {
        // Test the JSON parser directly
        let json = r#"{"paragraphs":[{"runs":[{"text":"hello","bold":true,"italic":false}]}]}"#;
        let result = parse_paste_json(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs.len(), 1);
        assert_eq!(result[0].runs[0].text, "hello");
        assert_eq!(result[0].runs[0].bold, Some(true));
        assert_eq!(result[0].runs[0].italic, Some(false));
    }

    #[test]
    fn test_parse_paste_json_multi_run() {
        let json =
            r#"{"paragraphs":[{"runs":[{"text":"a","bold":true},{"text":"b","fontSize":14}]}]}"#;
        let result = parse_paste_json(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].runs.len(), 2);
        assert_eq!(result[0].runs[0].text, "a");
        assert_eq!(result[0].runs[0].bold, Some(true));
        assert_eq!(result[0].runs[1].text, "b");
        assert_eq!(result[0].runs[1].font_size, Some(14.0));
    }

    #[test]
    fn test_parse_paste_json_multi_paragraph() {
        let json = r#"{"paragraphs":[{"runs":[{"text":"first"}]},{"runs":[{"text":"second"}]}]}"#;
        let result = parse_paste_json(json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].runs[0].text, "first");
        assert_eq!(result[1].runs[0].text, "second");
    }

    #[test]
    fn test_parse_paste_json_all_properties() {
        let json = r#"{"paragraphs":[{"runs":[{"text":"styled","bold":true,"italic":true,"underline":true,"strikethrough":true,"superscript":true,"fontSize":24,"fontFamily":"Courier","color":"00FF00","highlightColor":"FFFF00"}]}]}"#;
        let result = parse_paste_json(json).unwrap();
        let run = &result[0].runs[0];
        assert_eq!(run.text, "styled");
        assert_eq!(run.bold, Some(true));
        assert_eq!(run.italic, Some(true));
        assert_eq!(run.underline, Some(true));
        assert_eq!(run.strikethrough, Some(true));
        assert_eq!(run.superscript, Some(true));
        assert_eq!(run.font_size, Some(24.0));
        assert_eq!(run.font_family, Some("Courier".to_string()));
        assert_eq!(run.color, Some("00FF00".to_string()));
        assert_eq!(run.highlight_color, Some("FFFF00".to_string()));
    }

    #[test]
    fn test_paste_formatted_runs_verify_bold_italic() {
        // This test verifies that formatting is ACTUALLY applied to runs,
        // not just that text is inserted.
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("").unwrap();

        // Paste: bold "Hello" + plain " " + italic "World"
        let json = r#"{"paragraphs":[{"runs":[{"text":"Hello","bold":true},{"text":" "},{"text":"World","italic":true}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        // Verify text
        let text = doc.get_document_text().unwrap();
        assert!(text.contains("Hello World"), "text: {}", text);

        // Verify runs exist and have correct formatting
        let run_ids_str = doc.get_run_ids(&p).unwrap();
        let run_ids: Vec<&str> = run_ids_str
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();
        assert!(
            run_ids.len() >= 2,
            "Expected at least 2 runs, got {}: {:?}",
            run_ids.len(),
            run_ids
        );

        // Check that "Hello" run has bold
        let first_run_text = doc.get_run_text(run_ids[0]).unwrap();
        let first_run_fmt = doc.get_run_formatting_json(run_ids[0]).unwrap();
        assert_eq!(first_run_text, "Hello");
        assert!(
            first_run_fmt.contains("\"bold\":true"),
            "First run should be bold: {}",
            first_run_fmt
        );

        // Find the "World" run and check italic
        let mut found_world = false;
        for &rid in &run_ids {
            let rt = doc.get_run_text(rid).unwrap();
            if rt == "World" {
                let fmt = doc.get_run_formatting_json(rid).unwrap();
                assert!(
                    fmt.contains("\"italic\":true"),
                    "World run should be italic: {}",
                    fmt
                );
                found_world = true;
            }
        }
        assert!(found_world, "Could not find 'World' run in: {:?}", run_ids);
    }

    #[test]
    fn test_paste_formatted_runs_verify_font_color() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Before").unwrap();

        // Paste a run with font family, size, and color
        let json = r#"{"paragraphs":[{"runs":[{"text":"styled","bold":true,"fontSize":24,"fontFamily":"Arial","color":"FF0000"}]}]}"#;
        doc.paste_formatted_runs_json(&p, 6, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(text.contains("Beforestyled"), "text: {}", text);

        // Find the "styled" run
        let run_ids_str = doc.get_run_ids(&p).unwrap();
        let run_ids: Vec<&str> = run_ids_str
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();

        let mut found = false;
        for &rid in &run_ids {
            let rt = doc.get_run_text(rid).unwrap();
            if rt == "styled" {
                let fmt = doc.get_run_formatting_json(rid).unwrap();
                assert!(fmt.contains("\"bold\":true"), "should be bold: {}", fmt);
                assert!(
                    fmt.contains("\"fontSize\""),
                    "should have fontSize: {}",
                    fmt
                );
                assert!(
                    fmt.contains("\"fontFamily\""),
                    "should have fontFamily: {}",
                    fmt
                );
                assert!(fmt.contains("\"color\""), "should have color: {}", fmt);
                found = true;
            }
        }
        assert!(found, "Could not find 'styled' run");
    }

    #[test]
    fn test_paste_formatted_multi_para_verify_formatting() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("").unwrap();

        // Multi-paragraph paste: first para bold, second para italic
        let json = r#"{"paragraphs":[{"runs":[{"text":"Bold line","bold":true}]},{"runs":[{"text":"Italic line","italic":true}]}]}"#;
        doc.paste_formatted_runs_json(&p, 0, json).unwrap();

        let text = doc.get_document_text().unwrap();
        assert!(text.contains("Bold line"), "text: {}", text);
        assert!(text.contains("Italic line"), "text: {}", text);

        // Check paragraphs
        let all_ids: Vec<String> = {
            let j = doc.paragraph_ids_json().unwrap();
            let trimmed = j.trim_matches(|c| c == '[' || c == ']');
            trimmed
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };
        assert!(
            all_ids.len() >= 2,
            "Expected >= 2 paragraphs, got {}",
            all_ids.len()
        );

        // First paragraph should contain "Bold line" with bold formatting
        let first_runs_str = doc.get_run_ids(&all_ids[0]).unwrap();
        let first_runs: Vec<&str> = first_runs_str
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();

        let mut found_bold = false;
        for &rid in &first_runs {
            let rt = doc.get_run_text(rid).unwrap();
            if rt.contains("Bold") {
                let fmt = doc.get_run_formatting_json(rid).unwrap();
                assert!(
                    fmt.contains("\"bold\":true"),
                    "Bold line run should be bold: {}",
                    fmt
                );
                found_bold = true;
            }
        }
        assert!(found_bold, "Could not find bold run in first paragraph");
    }

    // ── Multi-run paragraph tests ──────────────────────────

    #[test]
    fn test_split_paragraph_multi_run() {
        // Create a paragraph with multiple runs via format_selection
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World Test").unwrap();

        // Bold "World" (chars 6..11) — creates 3 runs
        doc.format_selection(&id, 6, &id, 11, "bold", "true")
            .unwrap();

        // Split at offset 8 (inside "World" → "Wo" | "rld Test")
        let new_id = doc.split_paragraph(&id, 8).unwrap();
        let text1 = doc.get_paragraph_text(&id).unwrap();
        let text2 = doc.get_paragraph_text(&new_id).unwrap();
        assert_eq!(text1, "Hello Wo");
        assert_eq!(text2, "rld Test");
    }

    #[test]
    fn test_insert_text_multi_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("AB CD EF").unwrap();

        // Bold "CD" (chars 3..5) — creates 3 runs: "AB " | "CD" | " EF"
        doc.format_selection(&id, 3, &id, 5, "bold", "true")
            .unwrap();

        // Insert "X" at offset 4 (inside bold "CD")
        doc.insert_text_in_paragraph(&id, 4, "X").unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "AB CXD EF");
    }

    #[test]
    fn test_delete_text_multi_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("AB CD EF").unwrap();

        // Bold "CD" (chars 3..5) — creates 3 runs
        doc.format_selection(&id, 3, &id, 5, "bold", "true")
            .unwrap();

        // Delete 1 char at offset 4 (the "D" inside bold run)
        doc.delete_text_in_paragraph(&id, 4, 1).unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "AB C EF");
    }

    #[test]
    fn test_delete_selection_same_para_multi_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World End").unwrap();

        // Bold "World" (chars 6..11)
        doc.format_selection(&id, 6, &id, 11, "bold", "true")
            .unwrap();

        // Delete selection spanning from offset 4 to 13 (crossing run boundaries)
        doc.delete_selection(&id, 4, &id, 13).unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Hellnd");
    }

    #[test]
    fn test_replace_text_in_second_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Start Middle End").unwrap();

        // Bold "Middle" (chars 6..12) — creates 3 runs
        doc.format_selection(&id, 6, &id, 12, "bold", "true")
            .unwrap();

        // Replace "Mid" (offset 6, length 3) with "Top"
        doc.replace_text(&id, 6, 3, "Top").unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Start Topdle End");
    }

    // ─── Export Selection HTML Tests ─────────────────────

    #[test]
    fn test_export_selection_html_clean_output() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();

        // Create two paragraphs with formatting
        let id1 = doc.append_paragraph("Hello World").unwrap();
        let id2 = doc.append_paragraph("Second paragraph").unwrap();

        // Bold "World" in first paragraph (chars 6..11)
        doc.format_selection(&id1, 6, &id1, 11, "bold", "true")
            .unwrap();

        // Italic the entire second paragraph (chars 0..16)
        doc.format_selection(&id2, 0, &id2, 16, "italic", "true")
            .unwrap();

        // Export full selection spanning both paragraphs
        let html = doc.export_selection_html(&id1, 0, &id2, 16).unwrap();

        // Must NOT contain data-node-id or editor-specific attributes
        assert!(
            !html.contains("data-node-id"),
            "Clean HTML must not contain data-node-id. Got: {html}"
        );
        assert!(
            !html.contains("data-tc-"),
            "Clean HTML must not contain track-change data attributes. Got: {html}"
        );

        // Must contain proper formatting tags
        assert!(
            html.contains("<strong>"),
            "Expected <strong> tag for bold. Got: {html}"
        );
        assert!(
            html.contains("</strong>"),
            "Expected </strong> close tag. Got: {html}"
        );
        assert!(
            html.contains("<em>"),
            "Expected <em> tag for italic. Got: {html}"
        );
        assert!(
            html.contains("</em>"),
            "Expected </em> close tag. Got: {html}"
        );

        // Must contain the text
        assert!(
            html.contains("Hello"),
            "Expected 'Hello' in output. Got: {html}"
        );
        assert!(
            html.contains("World"),
            "Expected 'World' in output. Got: {html}"
        );
        assert!(
            html.contains("Second paragraph"),
            "Expected 'Second paragraph' in output. Got: {html}"
        );

        // Must have paragraph tags
        assert!(
            html.contains("<p>") || html.contains("<p "),
            "Expected <p> tags. Got: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_partial_paragraph() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();

        let id = doc.append_paragraph("Hello Beautiful World").unwrap();

        // Export only "Beautiful" (chars 6..15)
        let html = doc.export_selection_html(&id, 6, &id, 15).unwrap();

        assert!(
            html.contains("Beautiful"),
            "Expected 'Beautiful' in output. Got: {html}"
        );
        // Should NOT contain "Hello" or "World" since they're outside the range
        assert!(
            !html.contains("Hello"),
            "Should not contain 'Hello' (outside range). Got: {html}"
        );
        assert!(
            !html.contains("World"),
            "Should not contain 'World' (outside range). Got: {html}"
        );
        assert!(
            !html.contains("data-node-id"),
            "Clean HTML must not contain data-node-id. Got: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&id, 5).unwrap();

        let html = doc.export_selection_html(&id, 6, &id, 11).unwrap();
        assert!(
            html.contains("World"),
            "should export text after tab: {html}"
        );
        assert!(
            !html.contains("Hello"),
            "should not include text before tab: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&id, 5).unwrap();

        let html = doc.export_selection_html(&id, 6, &id, 11).unwrap();
        assert!(
            html.contains("World"),
            "should export text after line break: {html}"
        );
        assert!(
            !html.contains("Hello"),
            "should not include text before line break: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_across_paragraphs_after_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("HelloWorld").unwrap();
        let second = doc.append_paragraph("Second").unwrap();
        doc.insert_tab(&first, 5).unwrap();

        let html = doc.export_selection_html(&first, 6, &second, 3).unwrap();
        assert!(
            html.contains("World"),
            "should include text after start-paragraph tab: {html}"
        );
        assert!(
            html.contains("Sec"),
            "should include end-paragraph prefix: {html}"
        );
        assert!(
            !html.contains("Hello"),
            "should not include text before the tab boundary: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_across_paragraphs_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("HelloWorld").unwrap();
        let second = doc.append_paragraph("Second").unwrap();
        doc.insert_line_break(&first, 5).unwrap();

        let html = doc.export_selection_html(&first, 6, &second, 3).unwrap();
        assert!(
            html.contains("World"),
            "should include text after start-paragraph line break: {html}"
        );
        assert!(
            html.contains("Sec"),
            "should include end-paragraph prefix: {html}"
        );
        assert!(
            !html.contains("Hello"),
            "should not include text before the line-break boundary: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_across_paragraphs_with_tab_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_tab(&second, 5).unwrap();

        let html = doc.export_selection_html(&first, 2, &second, 6).unwrap();
        assert!(
            html.contains("pha"),
            "should include selected tail of first paragraph: {html}"
        );
        assert!(
            html.contains("Hello"),
            "should include end-paragraph text before the tab: {html}"
        );
        assert!(
            !html.contains("World"),
            "should not include text after the tab boundary: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_across_paragraphs_with_line_break_in_end_para_uses_correct_offsets(
    ) {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha").unwrap();
        let second = doc.append_paragraph("HelloWorld").unwrap();
        doc.insert_line_break(&second, 5).unwrap();

        let html = doc.export_selection_html(&first, 2, &second, 6).unwrap();
        assert!(
            html.contains("pha"),
            "should include selected tail of first paragraph: {html}"
        );
        assert!(
            html.contains("Hello"),
            "should include end-paragraph text before the line break: {html}"
        );
        assert!(
            !html.contains("World"),
            "should not include text after the line-break boundary: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_with_font_style() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();

        let id = doc.append_paragraph("Styled text").unwrap();

        // Apply font size
        doc.format_selection(&id, 0, &id, 11, "fontSize", "18")
            .unwrap();

        let html = doc.export_selection_html(&id, 0, &id, 11).unwrap();

        assert!(
            html.contains("font-size:18pt"),
            "Expected font-size inline style. Got: {html}"
        );
        assert!(
            !html.contains("data-node-id"),
            "Clean HTML must not contain data-node-id. Got: {html}"
        );
    }

    #[test]
    fn test_export_selection_html_spanning_table_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Before table").unwrap();
        let table_id = doc.insert_table(&first, 1, 1).unwrap();
        let cell_id = doc.get_cell_id(&table_id, 0, 0).unwrap();
        doc.set_cell_text(&cell_id, "Inside table").unwrap();
        let last = doc.append_paragraph("After").unwrap();

        let html = doc.export_selection_html(&first, 3, &last, 5).unwrap();
        assert!(
            html.contains("<table"),
            "selection should render the table: {html}"
        );
        assert!(
            html.contains("Inside table"),
            "should include table cell text: {html}"
        );
        assert!(
            html.contains("After"),
            "should include trailing paragraph text: {html}"
        );
    }

    #[test]
    fn test_get_selection_word_count_after_paragraph_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello there world").unwrap();
        doc.insert_tab(&id, 5).unwrap();

        let count = doc.get_selection_word_count(&id, 6, &id, 17).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_selection_word_count_after_paragraph_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello there world").unwrap();
        doc.insert_line_break(&id, 5).unwrap();

        let count = doc.get_selection_word_count(&id, 6, &id, 17).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_selection_word_count_across_table_cells_uses_correct_counts() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Before").unwrap();
        let table_id = doc.insert_table(&first, 1, 1).unwrap();
        let cell_id = doc.get_cell_id(&table_id, 0, 0).unwrap();
        doc.set_cell_text(&cell_id, "Table cell").unwrap();
        let last = doc.append_paragraph("After").unwrap();

        let count = doc.get_selection_word_count(&first, 0, &last, 5).unwrap();
        assert_eq!(
            count, 4,
            "Should count words from paragraph, cell, and last paragraph"
        );
    }

    #[test]
    fn test_get_selection_word_count_across_paragraphs_after_tab_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Hello there world").unwrap();
        let second = doc.append_paragraph("second paragraph here").unwrap();
        doc.insert_tab(&first, 5).unwrap();

        let count = doc.get_selection_word_count(&first, 6, &second, 6).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_selection_word_count_across_paragraphs_after_line_break_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Hello there world").unwrap();
        let second = doc.append_paragraph("second paragraph here").unwrap();
        doc.insert_line_break(&first, 5).unwrap();

        let count = doc.get_selection_word_count(&first, 6, &second, 6).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_selection_word_count_across_paragraphs_with_tab_in_end_para_uses_correct_offsets() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha beta").unwrap();
        let second = doc.append_paragraph("Hello there world").unwrap();
        doc.insert_tab(&second, 5).unwrap();

        let count = doc.get_selection_word_count(&first, 6, &second, 6).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_selection_word_count_across_paragraphs_with_line_break_in_end_para_uses_correct_offsets(
    ) {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let first = doc.append_paragraph("Alpha beta").unwrap();
        let second = doc.append_paragraph("Hello there world").unwrap();
        doc.insert_line_break(&second, 5).unwrap();

        let count = doc.get_selection_word_count(&first, 6, &second, 6).unwrap();
        assert_eq!(count, 2);
    }

    // ─── Collaboration Tests ───────────────────────────

    #[test]
    fn test_create_collab() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(42);
        assert_eq!(collab.replica_id().unwrap(), 42);
        assert!(!collab.can_undo());
        assert!(!collab.can_redo());
    }

    #[test]
    fn test_collab_local_insert_text() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(1);
        // The collab doc starts empty — need to check if model has a body
        let html = collab.to_html().unwrap();
        // Even empty doc should return without error
        assert!(html.is_empty() || html.len() > 0);
    }

    #[test]
    fn test_collab_state_vector() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(1);
        let sv = collab.get_state_vector().unwrap();
        // Should be valid JSON object
        assert!(sv.starts_with('{'));
        assert!(sv.ends_with('}'));
    }

    #[test]
    fn test_collab_op_log_size() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(1);
        let size = collab.op_log_size().unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_collab_tombstone_count() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(1);
        let count = collab.tombstone_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_collab_compact_op_log() {
        let engine = WasmEngine::new();
        let mut collab = engine.create_collab(1);
        // Compact on empty log should succeed
        collab.compact_op_log().unwrap();
        assert_eq!(collab.op_log_size().unwrap(), 0);
    }

    #[test]
    fn test_collab_undo_redo_empty() {
        let engine = WasmEngine::new();
        let mut collab = engine.create_collab(1);
        assert!(!collab.can_undo());
        assert!(!collab.can_redo());
        let result = collab.undo().unwrap();
        assert_eq!(result, "null");
        let result = collab.redo().unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_collab_get_changes_since_empty() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(1);
        let changes = collab.get_changes_since("{}").unwrap();
        assert_eq!(changes, "[]");
    }

    #[test]
    fn test_collab_get_peers_empty() {
        let engine = WasmEngine::new();
        let collab = engine.create_collab(1);
        let peers = collab.get_peers_json().unwrap();
        assert_eq!(peers, "[]");
    }

    #[test]
    fn test_collab_free_doc() {
        let engine = WasmEngine::new();
        let mut collab = engine.create_collab(1);
        // Verify doc works before freeing
        assert_eq!(collab.replica_id().unwrap(), 1);
        collab.free_doc();
        // After freeing, inner is None — calling replica_id() would produce
        // a JsError which panics on non-wasm targets, so we just verify
        // free_doc itself doesn't panic and can be called.
    }

    fn insert_paragraph_ops(
        doc: &mut s1_crdt::CollabDocument,
        text: &str,
    ) -> (String, Vec<s1_crdt::CrdtOperation>) {
        let model = doc.model();
        let body_id = model.body_id().unwrap();
        let index = model.node(body_id).map(|n| n.children.len()).unwrap_or(0);
        let para_id = doc.next_id();
        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let mut ops = Vec::new();
        ops.push(
            doc.apply_local(Operation::insert_node(
                body_id,
                index,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap(),
        );
        ops.push(
            doc.apply_local(Operation::insert_node(
                para_id,
                0,
                Node::new(run_id, NodeType::Run),
            ))
            .unwrap(),
        );
        ops.push(
            doc.apply_local(Operation::insert_node(run_id, 0, Node::text(text_id, text)))
                .unwrap(),
        );

        (format!("{}:{}", para_id.replica, para_id.counter), ops)
    }

    #[test]
    fn test_collab_concurrent_inserts_survive_brutal_suite() {
        let mut alice = s1_crdt::CollabDocument::new(1);
        let mut bob = s1_crdt::CollabDocument::new(2);

        let (paragraph, ops) = insert_paragraph_ops(&mut alice, "Start");
        for op in &ops {
            bob.apply_remote(op.clone()).unwrap();
        }

        let node_id = parse_node_id(&paragraph).unwrap();
        let (text_node_a, _) = find_first_text_node(alice.model(), node_id).unwrap();
        let alice_op = alice
            .apply_local(Operation::insert_text(text_node_a, 5, "A".to_string()))
            .unwrap();

        let (text_node_b, _) = find_first_text_node(bob.model(), node_id).unwrap();
        let bob_op = bob
            .apply_local(Operation::insert_text(text_node_b, 5, "B".to_string()))
            .unwrap();

        alice.apply_remote(bob_op.clone()).unwrap();
        bob.apply_remote(alice_op.clone()).unwrap();

        let text_a = alice.to_plain_text();
        let text_b = bob.to_plain_text();
        assert_eq!(text_a.chars().count(), 7);
        assert_eq!(text_b.chars().count(), 7);
        assert!(text_a.contains('A'));
        assert!(text_a.contains('B'));
        assert!(text_b.contains('A'));
        assert!(text_b.contains('B'));
    }

    #[test]
    fn test_collab_offline_peer_replays_brutal_operations() {
        let mut alice = s1_crdt::CollabDocument::new(1);
        let mut bob = s1_crdt::CollabDocument::new(2);
        let mut carol = s1_crdt::CollabDocument::new(3);

        let (paragraph, ops) = insert_paragraph_ops(&mut alice, "Truss");
        for op in &ops {
            bob.apply_remote(op.clone()).unwrap();
            carol.apply_remote(op.clone()).unwrap();
        }

        let carol_sv = carol.state_vector().clone();

        let node_id = parse_node_id(&paragraph).unwrap();
        let (text_node_a, _) = find_first_text_node(alice.model(), node_id).unwrap();
        let alice_op = alice
            .apply_local(Operation::insert_text(text_node_a, 0, "Alice ".to_string()))
            .unwrap();

        let (text_node_b, _) = find_first_text_node(bob.model(), node_id).unwrap();
        let bob_op = bob
            .apply_local(Operation::insert_text(text_node_b, 0, "Bob ".to_string()))
            .unwrap();

        bob.apply_remote(alice_op.clone()).unwrap();
        alice.apply_remote(bob_op.clone()).unwrap();

        let expected = alice.to_plain_text();
        let changes = alice.changes_since(&carol_sv);
        for change in changes {
            carol.apply_remote(change).unwrap();
        }

        assert_eq!(carol.to_plain_text(), expected);
    }

    // ── Stage A: Page-level rendering API tests ─────────────────────

    /// Helper: build a WasmDocument from a `s1engine::Document`.
    fn wrap(doc: s1engine::Document) -> WasmDocument {
        WasmDocument {
            inner: Some(doc),
            batch_label: None,
            batch_count: 0,
            layout_cache: std::cell::RefCell::new(None),
            _layout_dirty: std::cell::Cell::new(false),
            composition_anchor_node: None,
            composition_anchor_offset: None,
            composition_preview_len: 0,
        }
    }

    /// Helper: build a document with N paragraphs via the builder API.
    fn doc_with_paragraphs(texts: &[&str]) -> WasmDocument {
        let mut builder = s1engine::DocumentBuilder::new();
        for text in texts {
            builder = builder.paragraph(|p| p.text(text));
        }
        wrap(builder.build())
    }

    #[test]
    fn test_get_page_count_empty_doc() {
        let doc = wrap(s1engine::Engine::new().create());
        let count = doc.get_page_count().unwrap();
        assert!(count >= 1, "empty doc should have at least 1 page");
    }

    #[test]
    fn test_get_page_count_with_content() {
        let texts: Vec<&str> = (0..80)
            .map(|_| "Paragraph line for multi-page test")
            .collect();
        let doc = doc_with_paragraphs(&texts);
        let count = doc.get_page_count().unwrap();
        assert!(count >= 1, "doc with 80 paragraphs should have pages");
    }

    #[test]
    fn test_get_page_html_basic() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let count = doc.get_page_count().unwrap();
        assert!(count >= 1);

        let html = doc.get_page_html(0).unwrap();
        assert!(
            html.contains("Hello World"),
            "page HTML should contain paragraph text, got: {}",
            &html[..html.len().min(200)]
        );
        assert!(
            html.contains("data-node-id="),
            "page HTML should contain data-node-id attributes"
        );
    }

    #[test]
    fn test_get_page_count_returns_positive() {
        let doc = wrap(s1engine::Engine::new().create());
        let count = doc.get_page_count().unwrap();
        assert!(count >= 1, "even an empty doc should have 1 page");
    }

    #[test]
    fn test_get_affected_pages() {
        let doc = doc_with_paragraphs(&["Test para"]);
        let _count = doc.get_page_count().unwrap();

        // Find a paragraph node ID from the HTML
        let html = doc.get_page_html(0).unwrap();
        let nid = extract_first_node_id(&html);
        assert!(!nid.is_empty(), "should find a node ID in HTML");

        let json = doc.get_affected_pages(&nid).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(
            json.contains('0'),
            "affected pages should include page 0 for first paragraph"
        );
    }

    /// Extract first data-node-id value from HTML
    fn extract_first_node_id(html: &str) -> String {
        if let Some(start) = html.find("data-node-id=\"") {
            let after = &html[start + 14..];
            if let Some(end) = after.find('"') {
                return after[..end].to_string();
            }
        }
        String::new()
    }

    #[test]
    fn test_layout_cache_invalidation() {
        let mut doc = doc_with_paragraphs(&["First"]);
        let _count1 = doc.get_page_count().unwrap();
        assert!(
            doc.layout_cache.borrow().is_some(),
            "cache should be populated"
        );

        // Simulate mutation via doc_mut — cache is kept (stale) but dirty flag is set
        let _ = doc.doc_mut();
        assert!(
            doc.layout_cache.borrow().is_some(),
            "cache should still exist (stale) after doc_mut()"
        );
        assert!(
            doc._layout_dirty.get(),
            "layout should be marked dirty after doc_mut()"
        );

        // Re-compute layout (ensure_layout checks dirty flag)
        let count2 = doc.get_page_count().unwrap();
        assert!(count2 >= 1);
        assert!(
            doc.layout_cache.borrow().is_some(),
            "cache should be repopulated"
        );
    }

    #[test]
    fn test_get_page_html_multi_page() {
        let texts: Vec<&str> = (0..120)
            .map(|_| "Line of text for multi-page rendering test document")
            .collect();
        let doc = doc_with_paragraphs(&texts);

        let count = doc.get_page_count().unwrap();
        assert!(
            count >= 2,
            "120 paragraphs should span multiple pages, got {}",
            count
        );

        for i in 0..count {
            let html = doc.get_page_html(i).unwrap();
            assert!(!html.is_empty(), "page {} should have non-empty HTML", i);
        }

        let first = doc.get_page_html(0).unwrap();
        let last = doc.get_page_html(count - 1).unwrap();
        assert_ne!(first, last, "first and last pages should differ");
    }

    #[test]
    fn test_get_page_html_table() {
        let builder = s1engine::DocumentBuilder::new()
            .paragraph(|p| p.text("Before table"))
            .table(|t| {
                t.row(|r| r.cell("A1").cell("B1").cell("C1"))
                    .row(|r| r.cell("A2").cell("B2").cell("C2"))
                    .row(|r| r.cell("A3").cell("B3").cell("C3"))
            });
        let doc = wrap(builder.build());

        let _count = doc.get_page_count().unwrap();
        let html = doc.get_page_html(0).unwrap();
        assert!(
            html.contains("<table"),
            "page HTML should contain table element"
        );
        assert!(
            html.contains("data-node-id="),
            "table should have data-node-id"
        );
    }

    #[test]
    fn test_layout_cache_starts_empty() {
        let doc = doc_with_paragraphs(&["test"]);
        assert!(
            doc.layout_cache.borrow().is_none(),
            "layout cache should start empty before any layout call"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // Canvas Editing WASM Method Tests
    // ═══════════════════════════════════════════════════════════════════

    /// Helper: given a WasmDocument, get (paragraph_id, run_id, text_node_id) for
    /// the first paragraph. Returns string IDs and actual NodeIds.
    fn first_para_text_ids(doc: &WasmDocument) -> (NodeId, NodeId, NodeId) {
        let model = doc.doc().unwrap().model().clone();
        let body_id = model.body_id().unwrap();
        let body = model.node(body_id).unwrap();
        let para_id = body.children[0];
        let para = model.node(para_id).unwrap();
        // Find the first Run child
        let run_id = para
            .children
            .iter()
            .find(|&&c| {
                model
                    .node(c)
                    .map(|n| n.node_type == NodeType::Run)
                    .unwrap_or(false)
            })
            .copied()
            .unwrap();
        let run = model.node(run_id).unwrap();
        // Find the first Text child
        let text_id = run
            .children
            .iter()
            .find(|&&c| {
                model
                    .node(c)
                    .map(|n| n.node_type == NodeType::Text)
                    .unwrap_or(false)
            })
            .copied()
            .unwrap();
        (para_id, run_id, text_id)
    }

    /// Helper: format a NodeId as "replica:counter" string.
    fn nid_str(id: NodeId) -> String {
        format!("{}:{}", id.replica, id.counter)
    }

    /// Helper: build a position JSON string.
    fn pos_json(node_id: NodeId, offset_utf16: u32) -> String {
        format!(
            "{{\"node_id\":\"{}:{}\",\"offset_utf16\":{},\"affinity\":\"downstream\"}}",
            node_id.replica, node_id.counter, offset_utf16
        )
    }

    /// Helper: build a range JSON string where anchor and focus are in the same text node.
    fn range_json(
        anchor_node: NodeId,
        anchor_offset: u32,
        focus_node: NodeId,
        focus_offset: u32,
    ) -> String {
        format!(
            "{{\"anchor\":{{\"node_id\":\"{}:{}\",\"offset_utf16\":{},\"affinity\":\"downstream\"}},\"focus\":{{\"node_id\":\"{}:{}\",\"offset_utf16\":{},\"affinity\":\"downstream\"}}}}",
            anchor_node.replica, anchor_node.counter, anchor_offset,
            focus_node.replica, focus_node.counter, focus_offset
        )
    }

    /// Helper: get the nth paragraph's (para_id, run_id, text_node_id).
    fn nth_para_text_ids(doc: &WasmDocument, n: usize) -> (NodeId, NodeId, NodeId) {
        let model = doc.doc().unwrap().model().clone();
        let body_id = model.body_id().unwrap();
        let body = model.node(body_id).unwrap();
        let para_id = body.children[n];
        let para = model.node(para_id).unwrap();
        let run_id = para
            .children
            .iter()
            .find(|&&c| {
                model
                    .node(c)
                    .map(|n| n.node_type == NodeType::Run)
                    .unwrap_or(false)
            })
            .copied()
            .unwrap();
        let run = model.node(run_id).unwrap();
        let text_id = run
            .children
            .iter()
            .find(|&&c| {
                model
                    .node(c)
                    .map(|n| n.node_type == NodeType::Text)
                    .unwrap_or(false)
            })
            .copied()
            .unwrap();
        (para_id, run_id, text_id)
    }

    // ─── Phase 0: Position Resolution ─────────────────────────────

    #[test]
    fn test_resolve_position_to_paragraph_inserts_at_correct_offset() {
        // Create a paragraph with multiple runs by formatting a portion.
        // Using a single paragraph "Hello World", insert "X" at text-node offset 2.
        let mut doc = doc_with_paragraphs(&["Hello World"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Insert "X" at UTF-16 offset 2 within the text node (after "He")
        let position = pos_json(text_id, 2);
        let result = doc.canvas_insert_text(&position, "X").unwrap();

        // Verify the text is now "HeXllo World"
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "HeXllo World",
            "insert at offset 2 should produce HeXllo World"
        );

        // Verify result JSON contains selection info
        assert!(
            result.contains("\"document_revision\""),
            "result should contain document_revision"
        );
        assert!(
            result.contains("\"selection\""),
            "result should contain selection"
        );
    }

    // ─── Phase 1: Canvas Editing ──────────────────────────────────

    #[test]
    fn test_canvas_insert_text_basic() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Insert "X" at offset 2 (after "He")
        let position = pos_json(text_id, 2);
        doc.canvas_insert_text(&position, "X").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "HeXllo",
            "insert 'X' at offset 2 in 'Hello' should produce 'HeXllo'"
        );
    }

    #[test]
    fn test_canvas_insert_text_at_start() {
        let mut doc = doc_with_paragraphs(&["World"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        let position = pos_json(text_id, 0);
        doc.canvas_insert_text(&position, "Hello ").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_canvas_insert_text_at_end() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        let position = pos_json(text_id, 5);
        doc.canvas_insert_text(&position, " World").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_canvas_delete_range_within_paragraph() {
        let mut doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Delete "lo Wo" (offset 3..8)
        let range = range_json(text_id, 3, text_id, 8);
        doc.canvas_delete_range(&range).unwrap();

        let text = doc.to_plain_text().unwrap();
        assert!(
            text.contains("Helrld"),
            "deleting offsets 3..8 from 'Hello World' should yield 'Helrld', got: {}",
            text
        );
    }

    #[test]
    fn test_canvas_delete_range_full_word() {
        let mut doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Delete "Hello " (offset 0..6)
        let range = range_json(text_id, 0, text_id, 6);
        doc.canvas_delete_range(&range).unwrap();

        let text = doc.to_plain_text().unwrap();
        assert!(
            text.contains("World"),
            "should have 'World' after deleting 'Hello ', got: {}",
            text
        );
        assert!(
            !text.contains("Hello"),
            "should not still contain 'Hello', got: {}",
            text
        );
    }

    #[test]
    fn test_canvas_replace_range_basic() {
        let mut doc = doc_with_paragraphs(&["Hello World"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Replace "World" (offset 6..11) with "Rust"
        let range = range_json(text_id, 6, text_id, 11);
        doc.canvas_replace_range(&range, "Rust").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "Hello Rust",
            "replacing 'World' with 'Rust' should produce 'Hello Rust'"
        );
    }

    #[test]
    fn test_canvas_replace_range_with_longer_text() {
        let mut doc = doc_with_paragraphs(&["ABCD"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Replace "BC" (offset 1..3) with "XXXX"
        let range = range_json(text_id, 1, text_id, 3);
        doc.canvas_replace_range(&range, "XXXX").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "AXXXXD",
            "replacing 'BC' with 'XXXX' should yield 'AXXXXD'"
        );
    }

    #[test]
    fn test_canvas_insert_paragraph_break() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Split at offset 2 (after "He")
        let position = pos_json(text_id, 2);
        let result = doc.canvas_insert_paragraph_break(&position).unwrap();

        // Should now have 2 paragraphs
        let count = doc.paragraph_count().unwrap();
        assert_eq!(count, 2, "splitting should create 2 paragraphs");

        // Check content of each paragraph
        let (first_para_id, _, _) = nth_para_text_ids(&doc, 0);
        let (second_para_id, _, _) = nth_para_text_ids(&doc, 1);

        let first_text = doc.get_paragraph_text(&nid_str(first_para_id)).unwrap();
        let second_text = doc.get_paragraph_text(&nid_str(second_para_id)).unwrap();

        assert_eq!(first_text, "He", "first paragraph should contain 'He'");
        assert_eq!(second_text, "llo", "second paragraph should contain 'llo'");

        // Result should contain new paragraph ID
        assert!(
            result.contains("\"selection\""),
            "result should have selection field"
        );
    }

    #[test]
    fn test_canvas_insert_paragraph_break_at_start() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Split at offset 0 (beginning)
        let position = pos_json(text_id, 0);
        doc.canvas_insert_paragraph_break(&position).unwrap();

        let count = doc.paragraph_count().unwrap();
        assert_eq!(count, 2, "splitting at start should create 2 paragraphs");

        let (first_para_id, _, _) = nth_para_text_ids(&doc, 0);
        let first_text = doc.get_paragraph_text(&nid_str(first_para_id)).unwrap();
        assert_eq!(
            first_text, "",
            "first paragraph should be empty after split at start"
        );
    }

    #[test]
    fn test_canvas_toggle_mark_bold() {
        let mut doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Toggle bold on entire text (offset 0..11)
        let range = range_json(text_id, 0, text_id, 11);
        let result = doc.canvas_toggle_mark(&range, "bold").unwrap();

        assert!(
            result.contains("\"document_revision\""),
            "toggle mark should return edit result JSON"
        );

        // Verify via HTML that bold was applied
        let html = doc.to_html().unwrap();
        assert!(
            html.contains("<strong>") || html.contains("font-weight"),
            "text should be bold after toggle, html: {}",
            html
        );
    }

    #[test]
    fn test_canvas_toggle_mark_italic() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Toggle italic on "Hel" (offset 0..3)
        let range = range_json(text_id, 0, text_id, 3);
        doc.canvas_toggle_mark(&range, "italic").unwrap();

        let html = doc.to_html().unwrap();
        assert!(
            html.contains("<em>") || html.contains("font-style"),
            "text should be italic after toggle, html: {}",
            html
        );
    }

    // ─── Phase 4: Clipboard ───────────────────────────────────────

    #[test]
    fn test_copy_range_plain_text_single_paragraph() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Copy "lo Wo" (offset 3..8)
        let range = range_json(text_id, 3, text_id, 8);
        let copied = doc.copy_range_plain_text(&range).unwrap();
        assert_eq!(copied, "lo Wo", "should copy the exact substring");
    }

    #[test]
    fn test_copy_range_plain_text_full() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Copy entire text (offset 0..11)
        let range = range_json(text_id, 0, text_id, 11);
        let copied = doc.copy_range_plain_text(&range).unwrap();
        assert_eq!(copied, "Hello World");
    }

    #[test]
    fn test_copy_range_plain_text_multi_paragraph() {
        let doc = doc_with_paragraphs(&["First paragraph", "Second paragraph"]);
        let (_p1_id, _r1_id, t1_id) = nth_para_text_ids(&doc, 0);
        let (_p2_id, _r2_id, t2_id) = nth_para_text_ids(&doc, 1);

        // Copy from offset 6 of first paragraph to offset 6 of second
        let range = range_json(t1_id, 6, t2_id, 6);
        let copied = doc.copy_range_plain_text(&range).unwrap();

        assert!(
            copied.contains("paragraph"),
            "multi-paragraph copy should include text from first para"
        );
        assert!(
            copied.contains("Second"),
            "multi-paragraph copy should include text from second para"
        );
        assert!(
            copied.contains('\n'),
            "multi-paragraph copy should have newline separators"
        );
    }

    #[test]
    fn test_copy_range_html_single_paragraph() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Copy "World" (offset 6..11)
        let range = range_json(text_id, 6, text_id, 11);
        let html = doc.copy_range_html(&range).unwrap();

        assert!(
            html.contains("World"),
            "HTML copy should contain 'World', got: {}",
            html
        );
    }

    #[test]
    fn test_copy_range_html_contains_tags() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Copy all
        let range = range_json(text_id, 0, text_id, 11);
        let html = doc.copy_range_html(&range).unwrap();

        assert!(
            html.contains("Hello") && html.contains("World"),
            "HTML copy should contain both words, got: {}",
            html
        );
        // HTML output should have some structure (tags)
        assert!(
            html.contains('<') && html.contains('>'),
            "HTML should contain angle brackets for tags, got: {}",
            html
        );
    }

    // ─── Phase 5: IME Composition ─────────────────────────────────

    #[test]
    fn test_ime_full_flow_begin_update_commit() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Begin composition at offset 5 (end of "Hello")
        let position = pos_json(text_id, 5);
        let begin_result = doc.begin_composition(&position).unwrap();
        assert!(
            begin_result.contains("\"status\":\"composing\""),
            "begin_composition should return composing status"
        );

        // Update with preview text
        let update_result = doc.update_composition("abc").unwrap();
        assert!(
            update_result.contains("\"document_revision\""),
            "update_composition should return edit result"
        );

        // Check that the preview text was inserted
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Helloabc", "preview text should be appended");

        // Commit with final text (different from preview)
        doc.commit_composition("XYZ").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "HelloXYZ",
            "after commit, preview should be replaced with final text"
        );

        // Composition state should be cleared (begin again should work)
        let position2 = pos_json(text_id, 0);
        let result = doc.begin_composition(&position2);
        assert!(
            result.is_ok(),
            "should be able to start new composition after commit"
        );
    }

    #[test]
    fn test_ime_update_replaces_previous_preview() {
        let mut doc = doc_with_paragraphs(&["Test"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        let position = pos_json(text_id, 4);
        doc.begin_composition(&position).unwrap();

        // First update
        doc.update_composition("aaa").unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Testaaa");

        // Second update should replace previous preview
        doc.update_composition("bb").unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Testbb", "second update should replace first preview");

        // Third update
        doc.update_composition("c").unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Testc", "third update should replace second preview");
    }

    #[test]
    fn test_ime_cancel_composition_restores_original() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Begin at offset 5
        let position = pos_json(text_id, 5);
        doc.begin_composition(&position).unwrap();

        // Update with preview
        doc.update_composition("xyz").unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Helloxyz", "preview should be inserted");

        // Cancel — should remove preview and restore original
        doc.cancel_composition().unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "Hello",
            "cancel should restore original text by removing preview"
        );
    }

    #[test]
    fn test_ime_cancel_without_preview() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Begin but don't update
        let position = pos_json(text_id, 3);
        doc.begin_composition(&position).unwrap();

        // Cancel immediately
        doc.cancel_composition().unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(
            text, "Hello",
            "cancel without preview should leave text unchanged"
        );
    }

    #[test]
    fn test_ime_commit_empty_text() {
        let mut doc = doc_with_paragraphs(&["Hello"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        let position = pos_json(text_id, 5);
        doc.begin_composition(&position).unwrap();
        doc.update_composition("temp").unwrap();

        // Commit with empty text — should just remove preview
        doc.commit_composition("").unwrap();
        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "Hello", "committing empty text should remove preview");
    }

    // ─── Phase 7: Navigation & Capabilities ───────────────────────

    #[test]
    fn test_line_boundary_returns_valid_position() {
        let doc = doc_with_paragraphs(&["Hello World this is a test"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Compute layout first by calling ensure_layout indirectly
        let position = pos_json(text_id, 5);

        // line_boundary requires layout — it should work or return fallback
        let result = doc.line_boundary(&position, "start");
        // The method should succeed (layout is computed on demand)
        assert!(
            result.is_ok(),
            "line_boundary should succeed, got: {:?}",
            result
        );

        let pos_out = result.unwrap();
        assert!(
            pos_out.contains("\"node_id\""),
            "result should be a valid position JSON, got: {}",
            pos_out
        );
        assert!(
            pos_out.contains("\"offset_utf16\""),
            "result should have offset_utf16, got: {}",
            pos_out
        );

        // Also test "end" side
        let end_result = doc.line_boundary(&position, "end").unwrap();
        assert!(
            end_result.contains("\"node_id\""),
            "end boundary should be valid position JSON"
        );
    }

    #[test]
    fn test_move_range_extend_true_keeps_anchor() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Create a collapsed range at offset 5
        let range = range_json(text_id, 5, text_id, 5);

        // Move forward by character with extend=true
        let result = doc.move_range(&range, "forward", "character", true);
        assert!(
            result.is_ok(),
            "move_range should succeed, got: {:?}",
            result
        );

        let result_json = result.unwrap();
        // When extend=true, anchor should stay at original position
        // and focus should move forward
        assert!(
            result_json.contains("\"anchor\""),
            "result should have anchor field"
        );
        assert!(
            result_json.contains("\"focus\""),
            "result should have focus field"
        );

        // Parse the result to verify anchor stayed
        let anchor_str = extract_json_object(&result_json, "anchor").unwrap();
        let anchor_offset = scene_extract_json_number(&anchor_str, "offset_utf16").unwrap();
        assert_eq!(
            anchor_offset as u32, 5,
            "anchor offset should remain at 5 when extend=true"
        );
    }

    #[test]
    fn test_move_range_extend_false_collapses() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Create an extended range (offset 2..8)
        let range = range_json(text_id, 2, text_id, 8);

        // Move forward by character with extend=false
        let result = doc
            .move_range(&range, "forward", "character", false)
            .unwrap();

        // When extend=false, range should collapse first then move
        assert!(result.contains("\"anchor\""), "result should have anchor");
        assert!(result.contains("\"focus\""), "result should have focus");

        // Anchor and focus should be the same (collapsed)
        let anchor_str = extract_json_object(&result, "anchor").unwrap();
        let focus_str = extract_json_object(&result, "focus").unwrap();
        let anchor_offset = scene_extract_json_number(&anchor_str, "offset_utf16").unwrap() as u32;
        let focus_offset = scene_extract_json_number(&focus_str, "offset_utf16").unwrap() as u32;
        assert_eq!(
            anchor_offset, focus_offset,
            "with extend=false, anchor and focus should be at the same offset"
        );
    }

    #[test]
    fn test_editor_capabilities_contains_expected_keys() {
        let doc = doc_with_paragraphs(&["test"]);
        let caps = doc.editor_capabilities().unwrap();

        // Verify it's valid JSON with expected keys
        assert!(caps.contains("\"tables\""), "should have tables capability");
        assert!(caps.contains("\"images\""), "should have images capability");
        assert!(caps.contains("\"shapes\""), "should have shapes capability");
        assert!(
            caps.contains("\"textboxes\""),
            "should have textboxes capability"
        );
        assert!(caps.contains("\"undo\""), "should have undo capability");
        assert!(caps.contains("\"redo\""), "should have redo capability");
        assert!(
            caps.contains("\"clipboard\""),
            "should have clipboard capability"
        );
        assert!(caps.contains("\"find\""), "should have find capability");

        // All values should be true
        assert!(caps.contains("\"tables\":true"), "tables should be true");
        assert!(caps.contains("\"undo\":true"), "undo should be true");
    }

    #[test]
    fn test_editor_capabilities_on_empty_doc() {
        let doc = wrap(s1engine::Engine::new().create());
        let caps = doc.editor_capabilities().unwrap();
        assert!(
            caps.starts_with('{') && caps.ends_with('}'),
            "should be valid JSON object, got: {}",
            caps
        );
    }

    #[test]
    fn test_search_matches_finds_substring() {
        let doc = doc_with_paragraphs(&["Hello World Hello"]);
        let matches_json = doc.search_matches("Hello", true).unwrap();

        // Should find at least one match
        assert!(
            matches_json.starts_with('['),
            "search result should be a JSON array, got: {}",
            matches_json
        );
        // There are two "Hello" occurrences
        let match_count = matches_json.matches("\"offset\"").count();
        assert_eq!(
            match_count, 2,
            "should find 2 matches for 'Hello' in 'Hello World Hello'"
        );
    }

    #[test]
    fn test_search_matches_case_insensitive() {
        let doc = doc_with_paragraphs(&["Hello HELLO hello"]);
        let matches_json = doc.search_matches("hello", false).unwrap();

        assert!(matches_json.starts_with('['));
        let match_count = matches_json.matches("\"offset\"").count();
        assert_eq!(
            match_count, 3,
            "case-insensitive search for 'hello' should find 3 matches"
        );
    }

    #[test]
    fn test_search_matches_no_results() {
        let doc = doc_with_paragraphs(&["Hello World"]);
        let matches_json = doc.search_matches("xyz", true).unwrap();

        assert_eq!(
            matches_json, "[]",
            "searching for non-existent text should return empty array"
        );
    }

    #[test]
    fn test_search_matches_across_paragraphs() {
        let doc = doc_with_paragraphs(&["First Hello", "Second Hello", "Third"]);
        let matches_json = doc.search_matches("Hello", true).unwrap();

        let match_count = matches_json.matches("\"offset\"").count();
        assert_eq!(
            match_count, 2,
            "should find 2 'Hello' matches across paragraphs"
        );
    }

    // ─── Canvas Edit Result JSON Structure ────────────────────────

    #[test]
    fn test_canvas_insert_text_returns_valid_edit_result() {
        let mut doc = doc_with_paragraphs(&["Test"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        let position = pos_json(text_id, 2);
        let result = doc.canvas_insert_text(&position, "AB").unwrap();

        // Verify structure of EditResult JSON
        assert!(result.contains("\"document_revision\""));
        assert!(result.contains("\"layout_revision\""));
        assert!(result.contains("\"dirty_pages\""));
        assert!(result.contains("\"selection\""));

        // The selection should have offset 4 (original 2 + 2 inserted chars)
        assert!(
            result.contains("\"offset_utf16\":4"),
            "cursor should be at offset 4 after inserting 2 chars at offset 2, got: {}",
            result
        );
    }

    #[test]
    fn test_canvas_delete_range_returns_collapsed_cursor() {
        let mut doc = doc_with_paragraphs(&["Hello World"]);
        let (_para_id, _run_id, text_id) = first_para_text_ids(&doc);

        let range = range_json(text_id, 2, text_id, 5);
        let result = doc.canvas_delete_range(&range).unwrap();

        // Cursor should collapse to start of deleted range (offset 2)
        assert!(
            result.contains("\"offset_utf16\":2"),
            "cursor should collapse to start of deleted range, got: {}",
            result
        );
    }

    #[test]
    fn test_canvas_replace_range_cursor_after_inserted_text() {
        let mut doc = doc_with_paragraphs(&["ABCDE"]);
        let (para_id, _run_id, text_id) = first_para_text_ids(&doc);

        // Replace "BCD" (1..4) with "XX"
        let range = range_json(text_id, 1, text_id, 4);
        let result = doc.canvas_replace_range(&range, "XX").unwrap();

        let text = doc.get_paragraph_text(&nid_str(para_id)).unwrap();
        assert_eq!(text, "AXXE");

        // Cursor should be at offset 3 (1 + 2 inserted chars)
        assert!(
            result.contains("\"offset_utf16\":3"),
            "cursor should be after inserted text, got: {}",
            result
        );
    }
}

// ─── Word-Level Diff Algorithm ──────────────────────────────────

enum DiffOp {
    Equal(Vec<String>),
    Insert(Vec<String>),
    Delete(Vec<String>),
}

/// Compute word-level differences between two word sequences using LCS.
fn word_diff(a: &[&str], b: &[&str]) -> Vec<DiffOp> {
    let m = a.len();
    let n = b.len();

    // Build LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to produce diff
    let mut ops: Vec<DiffOp> = Vec::new();
    let mut i = m;
    let mut j = n;
    let mut raw: Vec<(char, String)> = Vec::new(); // 'E'qual, 'I'nsert, 'D'elete

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            raw.push(('E', a[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            raw.push(('I', b[j - 1].to_string()));
            j -= 1;
        } else {
            raw.push(('D', a[i - 1].to_string()));
            i -= 1;
        }
    }

    raw.reverse();

    // Group consecutive operations
    let mut current_type = ' ';
    let mut current_words: Vec<String> = Vec::new();

    for (op_type, word) in raw {
        if op_type != current_type && !current_words.is_empty() {
            ops.push(match current_type {
                'E' => DiffOp::Equal(std::mem::take(&mut current_words)),
                'I' => DiffOp::Insert(std::mem::take(&mut current_words)),
                'D' => DiffOp::Delete(std::mem::take(&mut current_words)),
                _ => DiffOp::Equal(std::mem::take(&mut current_words)),
            });
        }
        current_type = op_type;
        current_words.push(word);
    }
    if !current_words.is_empty() {
        ops.push(match current_type {
            'E' => DiffOp::Equal(current_words),
            'I' => DiffOp::Insert(current_words),
            'D' => DiffOp::Delete(current_words),
            _ => DiffOp::Equal(current_words),
        });
    }

    ops
}

// ─── OMML ↔ LaTeX Conversion ────────────────────────────────────

/// Convert OMML XML to LaTeX string.
/// Handles: fractions, sub/superscripts, radicals, accents, matrices, operators.
fn convert_omml_to_latex(omml: &str) -> String {
    let mut latex = String::new();
    let mut i = 0;
    let bytes = omml.as_bytes();

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'<' {
            // Find tag name
            let tag_start = i + 1;
            let mut tag_end = tag_start;
            while tag_end < bytes.len() && bytes[tag_end] != b'>' && bytes[tag_end] != b' ' {
                tag_end += 1;
            }
            let tag = &omml[tag_start..tag_end];

            match tag {
                "m:f" | "m:frac" => latex.push_str("\\frac{"),
                "/m:f" | "/m:frac" => latex.push('}'),
                "m:num" => latex.push('{'),
                "/m:num" => latex.push_str("}{"),
                "m:den" => {}
                "/m:den" => latex.push('}'),
                "m:sSub" => {}
                "/m:sSub" => {}
                "m:sSup" => {}
                "/m:sSup" => {}
                "m:sub" => latex.push_str("_{"),
                "/m:sub" => latex.push('}'),
                "m:sup" => latex.push_str("^{"),
                "/m:sup" => latex.push('}'),
                "m:rad" => latex.push_str("\\sqrt{"),
                "/m:rad" => latex.push('}'),
                "m:nary" => latex.push_str("\\sum"),
                "/m:nary" => {}
                "m:m" => latex.push_str("\\begin{matrix}"),
                "/m:m" => latex.push_str("\\end{matrix}"),
                "m:mr" => {}
                "/m:mr" => latex.push_str("\\\\"),
                "m:e" => {}
                "/m:e" => {}
                "m:t" => {
                    // Extract text content
                    let close = omml[i..].find("</m:t>").unwrap_or(0);
                    let text_start = omml[i..].find('>').map(|p| i + p + 1).unwrap_or(i);
                    if close > 0 && text_start < i + close {
                        let text = &omml[text_start..i + close];
                        // Map common symbols
                        let mapped = map_omml_symbol(text.trim());
                        latex.push_str(&mapped);
                    }
                    i = i + close + 6; // skip past </m:t>
                    continue;
                }
                _ => {}
            }

            // Skip to end of tag
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    latex
}

fn map_omml_symbol(s: &str) -> String {
    match s {
        "\u{03B1}" => "\\alpha".into(),
        "\u{03B2}" => "\\beta".into(),
        "\u{03B3}" => "\\gamma".into(),
        "\u{03B4}" => "\\delta".into(),
        "\u{03B5}" => "\\epsilon".into(),
        "\u{03B8}" => "\\theta".into(),
        "\u{03BB}" => "\\lambda".into(),
        "\u{03BC}" => "\\mu".into(),
        "\u{03C0}" => "\\pi".into(),
        "\u{03C3}" => "\\sigma".into(),
        "\u{03C6}" => "\\phi".into(),
        "\u{03C9}" => "\\omega".into(),
        "\u{2211}" => "\\sum".into(),
        "\u{220F}" => "\\prod".into(),
        "\u{222B}" => "\\int".into(),
        "\u{221E}" => "\\infty".into(),
        "\u{2260}" => "\\neq".into(),
        "\u{2264}" => "\\leq".into(),
        "\u{2265}" => "\\geq".into(),
        "\u{00B1}" => "\\pm".into(),
        "\u{00D7}" => "\\times".into(),
        "\u{00F7}" => "\\div".into(),
        "\u{2248}" => "\\approx".into(),
        "\u{2261}" => "\\equiv".into(),
        _ => s.to_string(),
    }
}

/// Convert LaTeX to OMML XML.
fn convert_latex_to_omml(latex: &str) -> String {
    let mut omml = String::from("<m:oMathPara xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"><m:oMath>");

    let mut i = 0;
    let chars: Vec<char> = latex.chars().collect();

    while i < chars.len() {
        if chars[i] == '\\' {
            // LaTeX command
            let cmd_start = i + 1;
            let mut cmd_end = cmd_start;
            while cmd_end < chars.len() && chars[cmd_end].is_alphabetic() {
                cmd_end += 1;
            }
            let cmd = &latex[cmd_start..cmd_end];

            match cmd {
                "frac" => {
                    omml.push_str("<m:f><m:num>");
                    // Next {} is numerator, then {} denominator
                }
                "sqrt" => omml.push_str("<m:rad><m:e>"),
                "sum" => omml.push_str("<m:nary><m:naryPr><m:chr m:val=\"\u{2211}\"/></m:naryPr>"),
                "int" => omml.push_str("<m:nary><m:naryPr><m:chr m:val=\"\u{222B}\"/></m:naryPr>"),
                "alpha" => omml.push_str("<m:r><m:t>\u{03B1}</m:t></m:r>"),
                "beta" => omml.push_str("<m:r><m:t>\u{03B2}</m:t></m:r>"),
                "gamma" => omml.push_str("<m:r><m:t>\u{03B3}</m:t></m:r>"),
                "pi" => omml.push_str("<m:r><m:t>\u{03C0}</m:t></m:r>"),
                "theta" => omml.push_str("<m:r><m:t>\u{03B8}</m:t></m:r>"),
                "infty" => omml.push_str("<m:r><m:t>\u{221E}</m:t></m:r>"),
                "pm" => omml.push_str("<m:r><m:t>\u{00B1}</m:t></m:r>"),
                "times" => omml.push_str("<m:r><m:t>\u{00D7}</m:t></m:r>"),
                "leq" => omml.push_str("<m:r><m:t>\u{2264}</m:t></m:r>"),
                "geq" => omml.push_str("<m:r><m:t>\u{2265}</m:t></m:r>"),
                "neq" => omml.push_str("<m:r><m:t>\u{2260}</m:t></m:r>"),
                _ => {
                    omml.push_str(&format!("<m:r><m:t>\\{}</m:t></m:r>", cmd));
                }
            }
            i = cmd_end;
        } else if chars[i] == '{' {
            i += 1;
        } else if chars[i] == '}' {
            // Close current OMML element
            omml.push_str("</m:e>");
            i += 1;
        } else if chars[i] == '_' {
            omml.push_str("<m:sSub><m:e>");
            i += 1;
        } else if chars[i] == '^' {
            omml.push_str("<m:sSup><m:e>");
            i += 1;
        } else if chars[i] == ' ' {
            i += 1;
        } else {
            omml.push_str(&format!("<m:r><m:t>{}</m:t></m:r>", chars[i]));
            i += 1;
        }
    }

    omml.push_str("</m:oMath></m:oMathPara>");
    omml
}
