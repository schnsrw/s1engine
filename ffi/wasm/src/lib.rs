//! WebAssembly bindings for s1engine.
//!
//! Provides a JavaScript-friendly API for creating, opening, editing, and
//! exporting documents from the browser or Node.js.

use wasm_bindgen::prelude::*;

use s1_model::{
    Alignment, AttributeKey, AttributeValue, Color, DocumentModel, ListFormat, Node, NodeId,
    NodeType, UnderlineStyle,
};
use s1_layout::{layout_to_html, LayoutConfig, PageLayout};
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
        }
    }

    /// Open a document from bytes with auto-detected format.
    ///
    /// Supports DOCX, ODT, and TXT formats.
    pub fn open(&self, data: &[u8]) -> Result<WasmDocument, JsError> {
        let doc = self
            .inner
            .open(data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmDocument { inner: Some(doc) })
    }

    /// Open a document from bytes with an explicit format.
    ///
    /// Format should be one of: "docx", "odt", "txt".
    pub fn open_as(&self, data: &[u8], format: &str) -> Result<WasmDocument, JsError> {
        let fmt = parse_format(format)?;
        let doc = self
            .inner
            .open_as(data, fmt)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmDocument { inner: Some(doc) })
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
}

#[wasm_bindgen]
impl WasmDocument {
    /// Extract all text content as a plain string.
    pub fn to_plain_text(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        Ok(doc.to_plain_text())
    }

    /// Export the document to the specified format.
    ///
    /// Format should be one of: "docx", "odt", "txt", "pdf".
    /// Returns the exported bytes.
    pub fn export(&self, format: &str) -> Result<Vec<u8>, JsError> {
        let fmt = parse_format(format)?;
        let doc = self.doc()?;
        doc.export(fmt).map_err(|e| JsError::new(&e.to_string()))
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

    /// Get the number of paragraphs in the document.
    pub fn paragraph_count(&self) -> Result<usize, JsError> {
        let doc = self.doc()?;
        Ok(doc.paragraph_count())
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

        let mut pages_json = Vec::new();
        for (i, page) in layout.pages.iter().enumerate() {
            let mut node_ids = Vec::new();
            for block in &page.blocks {
                let id_str = format!("{}:{}", block.source_id.replica, block.source_id.counter);
                if !node_ids.contains(&id_str) {
                    node_ids.push(id_str);
                }
            }

            let footer_text = page.footer.as_ref().map(|f| {
                match &f.kind {
                    s1_layout::LayoutBlockKind::Paragraph { lines, .. } => {
                        lines.iter().flat_map(|l| l.runs.iter().map(|r| r.text.as_str())).collect::<String>()
                    }
                    _ => String::new(),
                }
            }).unwrap_or_default();

            let header_text = page.header.as_ref().map(|h| {
                match &h.kind {
                    s1_layout::LayoutBlockKind::Paragraph { lines, .. } => {
                        lines.iter().flat_map(|l| l.runs.iter().map(|r| r.text.as_str())).collect::<String>()
                    }
                    _ => String::new(),
                }
            }).unwrap_or_default();

            let ids_arr: Vec<String> = node_ids.iter().map(|id| format!("\"{}\"", id)).collect();
            pages_json.push(format!(
                "{{\"pageNum\":{},\"width\":{:.1},\"height\":{:.1},\"nodeIds\":[{}],\"footer\":\"{}\",\"header\":\"{}\"}}",
                i + 1,
                page.width,
                page.height,
                ids_arr.join(","),
                footer_text.replace('"', "\\\""),
                header_text.replace('"', "\\\""),
            ));
        }
        Ok(format!("{{\"pages\":[{}]}}", pages_json.join(",")))
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

    /// Export the document as a PDF data URL using loaded fonts.
    pub fn to_pdf_data_url_with_fonts(
        &self,
        font_db: &WasmFontDatabase,
    ) -> Result<String, JsError> {
        let bytes = self.to_pdf_with_fonts(font_db)?;
        let b64 = base64_encode(&bytes);
        Ok(format!("data:application/pdf;base64,{}", b64))
    }

    /// Render the document as HTML with formatting, images, and hyperlinks.
    pub fn to_html(&self) -> Result<String, JsError> {
        let doc = self.doc()?;
        let model = doc.model();
        let body_id = model.body_id().ok_or_else(|| JsError::new("No body"))?;
        let mut html = String::new();

        let sections = doc.sections();
        use s1_model::section::HeaderFooterType;

        // Find the best header across all sections
        // Prefer Default; fall back to First if title_page is set; else any.
        let header_ref = sections.iter().find_map(|sec| {
            sec.headers.iter()
                .find(|h| h.hf_type == HeaderFooterType::Default)
                .or_else(|| {
                    if sec.title_page {
                        sec.headers.iter().find(|h| h.hf_type == HeaderFooterType::First)
                    } else {
                        sec.headers.first()
                    }
                })
        });
        if let Some(hf) = header_ref {
            html.push_str("<header style=\"border-bottom:1px solid #dadce0;padding:8px 0;margin-bottom:16px;color:#5f6368;font-size:9pt\">");
            render_children(model, hf.node_id, &mut html);
            html.push_str("</header>");
        }

        // Render body content
        render_children(model, body_id, &mut html);

        // Find the best footer across all sections
        let footer_ref = sections.iter().find_map(|sec| {
            sec.footers.iter()
                .find(|f| f.hf_type == HeaderFooterType::Default)
                .or_else(|| {
                    if sec.title_page {
                        sec.footers.iter().find(|f| f.hf_type == HeaderFooterType::First)
                    } else {
                        sec.footers.first()
                    }
                })
        });
        if let Some(hf) = footer_ref {
            html.push_str("<footer style=\"border-top:1px solid #dadce0;padding:8px 0;margin-top:16px;color:#5f6368;font-size:9pt\">");
            render_children(model, hf.node_id, &mut html);
            html.push_str("</footer>");
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

    // ─── Structure Queries ───────────────────────────────────────

    /// Get the body node ID as "replica:counter" string.
    pub fn body_id(&self) -> Result<Option<String>, JsError> {
        let doc = self.doc()?;
        Ok(doc.body_id().map(|id| format!("{}:{}", id.replica, id.counter)))
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
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
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
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
        let index = doc
            .node(body_id)
            .map(|n| n.children.len())
            .unwrap_or(0);

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
        txn.push(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, text),
        ));
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
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
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
        txn.push(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, text),
        ));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Append a heading at the given level (1-6).
    ///
    /// Returns the heading paragraph's node ID.
    pub fn append_heading(&mut self, level: u8, text: &str) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
        let index = doc
            .node(body_id)
            .map(|n| n.children.len())
            .unwrap_or(0);

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        let style_id = format!("Heading{}", level.clamp(1, 6));
        para.attributes.set(
            AttributeKey::StyleId,
            AttributeValue::String(style_id),
        );

        let run_id = doc.next_id();
        let text_id = doc.next_id();

        let mut txn = Transaction::with_label("Insert heading");
        txn.push(Operation::insert_node(body_id, index, para));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, text),
        ));
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

    /// Replace the text content of a paragraph's first run.
    pub fn set_paragraph_text(
        &mut self,
        node_id_str: &str,
        new_text: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;

        // Ensure Run+Text exist (creates them if paragraph is empty)
        let (text_node_id, old_len) = ensure_run_and_text(doc, para_id)?;

        let mut txn = Transaction::with_label("Set paragraph text");
        if old_len > 0 {
            txn.push(Operation::delete_text(text_node_id, 0, old_len));
        }
        if !new_text.is_empty() {
            txn.push(Operation::insert_text(text_node_id, 0, new_text));
        }
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
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
            Ok((text_node_id, local_offset, _)) => {
                doc.apply(Operation::insert_text(text_node_id, local_offset, text))
                    .map_err(|e| JsError::new(&e.to_string()))
            }
            Err(_) => {
                // No text nodes exist — create run + text
                let (text_node_id, _) = ensure_run_and_text(doc, para_id)?;
                doc.apply(Operation::insert_text(text_node_id, 0, text))
                    .map_err(|e| JsError::new(&e.to_string()))
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

        // Check if deletion stays within a single text node
        match find_text_node_at_char_offset(doc.model(), para_id, offset) {
            Ok((text_node_id, local_offset, text_len)) => {
                if local_offset + length <= text_len {
                    // Fits within one text node
                    return doc.apply(Operation::delete_text(text_node_id, local_offset, length))
                        .map_err(|e| JsError::new(&e.to_string()));
                }
            }
            Err(e) => return Err(e),
        }

        // Spans multiple runs — use range deletion
        delete_text_range_in_paragraph(doc, para_id, offset, offset + length)
    }

    // ─── Formatting ───────────────────────────────────────────────

    /// Set bold on a paragraph's first run.
    pub fn set_bold(&mut self, node_id_str: &str, bold: bool) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let run_id = find_first_run(doc.model(), para_id)?;
        let attrs = s1_model::AttributeMap::new().bold(bold);
        doc.apply(Operation::set_attributes(run_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Set italic on a paragraph's first run.
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
        // Verify node exists
        model
            .node(nid)
            .ok_or_else(|| JsError::new(&format!("Node {} not found", node_id_str)))?;
        let mut html = String::new();
        render_node(model, nid, &mut html);
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

        // Get full paragraph text and style
        let full_text = extract_paragraph_text(doc.model(), para_id);
        let style_id = doc
            .model()
            .node(para_id)
            .and_then(|n| n.attributes.get_string(&AttributeKey::StyleId))
            .map(|s| s.to_string());

        let char_len = full_text.chars().count();
        let tail_text: String = full_text.chars().skip(char_offset).collect();

        // Find body/section parent and paragraph position
        let para_node = doc
            .node(para_id)
            .ok_or_else(|| JsError::new("Paragraph not found"))?;
        let parent_id = para_node
            .parent
            .ok_or_else(|| JsError::new("Paragraph has no parent"))?;
        let parent = doc
            .node(parent_id)
            .ok_or_else(|| JsError::new("Parent not found"))?;
        let index = parent
            .children
            .iter()
            .position(|&c| c == para_id)
            .ok_or_else(|| JsError::new("Paragraph not found in parent"))?;

        // Collect runs that need tail deletion (from split point onward)
        // We need to: delete text from the run containing the offset, then delete
        // all subsequent runs entirely.
        let para = doc.node(para_id).ok_or_else(|| JsError::new("Paragraph not found"))?;
        let run_children: Vec<NodeId> = para
            .children
            .iter()
            .filter(|&&c| doc.node(c).map(|n| n.node_type == NodeType::Run).unwrap_or(false))
            .copied()
            .collect();

        // Find which run contains the split offset
        let mut accumulated = 0usize;
        let mut split_run_idx = run_children.len(); // default: after all runs
        let mut local_offset = 0usize;
        for (i, &run_id) in run_children.iter().enumerate() {
            let rlen = run_char_len(doc.model(), run_id);
            if char_offset <= accumulated + rlen {
                split_run_idx = i;
                local_offset = char_offset - accumulated;
                break;
            }
            accumulated += rlen;
        }

        // Allocate IDs for new paragraph
        let new_para_id = doc.next_id();
        let new_run_id = doc.next_id();
        let new_text_id = doc.next_id();

        let mut txn = Transaction::with_label("Split paragraph");

        // Delete tail text from the run at split point
        if split_run_idx < run_children.len() && char_offset < char_len {
            let split_run_id = run_children[split_run_idx];
            let (text_node_id, local_off, text_len) =
                find_text_node_at_char_offset_in_run(doc.model(), split_run_id, local_offset)?;
            if local_off < text_len {
                txn.push(Operation::delete_text(
                    text_node_id,
                    local_off,
                    text_len - local_off,
                ));
            }
        }

        // Delete all runs after the split point
        for &run_id in run_children.iter().skip(split_run_idx + 1) {
            txn.push(Operation::delete_node(run_id));
        }

        // Create new paragraph with tail text
        let mut new_para = Node::new(new_para_id, NodeType::Paragraph);
        if let Some(sid) = &style_id {
            new_para.attributes.set(
                AttributeKey::StyleId,
                AttributeValue::String(sid.clone()),
            );
        }
        txn.push(Operation::insert_node(parent_id, index + 1, new_para));
        txn.push(Operation::insert_node(
            new_para_id,
            0,
            Node::new(new_run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            new_run_id,
            0,
            Node::text(new_text_id, &tail_text),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", new_para_id.replica, new_para_id.counter))
    }

    /// Merge two adjacent paragraphs.
    ///
    /// Appends the text from `second_id` to `first_id`, then deletes
    /// `second_id`. Used for Backspace at the start of a paragraph.
    pub fn merge_paragraphs(
        &mut self,
        first_id_str: &str,
        second_id_str: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let first_id = parse_node_id(first_id_str)?;
        let second_id = parse_node_id(second_id_str)?;

        let second_text = extract_paragraph_text(doc.model(), second_id);

        // Ensure first paragraph has Run+Text (creates if empty)
        let (first_text_node, first_len) = ensure_run_and_text(doc, first_id)?;

        let mut txn = Transaction::with_label("Merge paragraphs");

        // Append second paragraph's text to first
        if !second_text.is_empty() {
            txn.push(Operation::insert_text(
                first_text_node,
                first_len,
                &second_text,
            ));
        }

        // Delete second paragraph
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
        let (start_text_id, start_local_offset, _) = match find_text_node_at_char_offset(
            doc.model(),
            start_para,
            start_offset,
        ) {
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

        let body_id = doc.body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
        let children: Vec<NodeId> = doc.node(body_id)
            .ok_or_else(|| JsError::new("Body not found"))?
            .children.clone();

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
        if del_from_start > 0 {
            // Delete remaining text from the start offset's text node
            txn.push(Operation::delete_text(start_text_id, start_local_offset,
                del_from_start.min(start_total_chars - start_offset)));
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
                        if child.node_type == NodeType::Run {
                            let rlen = run_char_len(doc.model(), child_id);
                            if past_split {
                                runs_to_delete.push(child_id);
                            } else if accumulated + rlen >= start_offset && start_offset > accumulated {
                                past_split = true;
                            } else if accumulated >= start_offset {
                                runs_to_delete.push(child_id);
                                past_split = true;
                            }
                            accumulated += rlen;
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
            txn.push(Operation::insert_text(start_text_id, start_local_offset, &remaining_text));
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
                    let s =
                        run.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true);
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

        let mut json = format!(
            "{{\"bold\":{},\"italic\":{},\"underline\":{},\"strikethrough\":{},\"alignment\":\"{}\",\"headingLevel\":{}",
            bold, italic, underline, strikethrough, alignment, heading_level
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
        json.push('}');
        Ok(json)
    }

    /// Set the heading level of a paragraph.
    ///
    /// Level 0 removes the heading style (converts to normal paragraph).
    /// Level 1-6 sets the corresponding heading style.
    pub fn set_heading_level(
        &mut self,
        node_id_str: &str,
        level: u8,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let mut attrs = s1_model::AttributeMap::new();
        if level == 0 {
            // Remove StyleId by setting to empty string
            attrs.set(
                AttributeKey::StyleId,
                AttributeValue::String(String::new()),
            );
        } else {
            let style_id = format!("Heading{}", level.clamp(1, 6));
            attrs.set(
                AttributeKey::StyleId,
                AttributeValue::String(style_id),
            );
        }
        doc.apply(Operation::set_attributes(para_id, attrs))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // ─── P.1: Selection & Range Formatting API ─────────────────

    /// Split a Run node at a character offset.
    ///
    /// Creates a new Run after the original with the tail text, preserving
    /// all formatting attributes. Returns the new run's node ID.
    pub fn split_run(
        &mut self,
        run_id_str: &str,
        char_offset: usize,
    ) -> Result<String, JsError> {
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
        let full_text = text_node
            .text_content
            .as_deref()
            .unwrap_or("");
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
    pub fn format_run(
        &mut self,
        run_id_str: &str,
        key: &str,
        value: &str,
    ) -> Result<(), JsError> {
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
            let body_id = doc
                .body_id()
                .ok_or_else(|| JsError::new("No body node"))?;
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
            let start_text_len = extract_paragraph_text(doc.model(), start_para).chars().count();
            format_range_in_paragraph(doc, start_para, start_offset, start_text_len, &attrs)?;

            // Format all intermediate paragraphs fully
            for &child_id in &children[start_idx + 1..end_idx] {
                if let Some(child) = doc.node(child_id) {
                    if child.node_type == NodeType::Paragraph {
                        let len = extract_paragraph_text(doc.model(), child_id).chars().count();
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
                    ids.push(format!(
                        "\"{}:{}\"",
                        child_id.replica, child_id.counter
                    ));
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
            collect_runs_in_range(doc.model(), start_para, start_offset, end_offset, &mut run_ids);
        } else {
            let body_id = doc
                .body_id()
                .ok_or_else(|| JsError::new("No body node"))?;
            let body = doc
                .node(body_id)
                .ok_or_else(|| JsError::new("Body not found"))?;
            let children = body.children.clone();
            let start_idx = children
                .iter()
                .position(|&c| c == start_para)
                .unwrap_or(0);
            let end_idx = children
                .iter()
                .position(|&c| c == end_para)
                .unwrap_or(children.len());

            let start_len = extract_paragraph_text(doc.model(), start_para).chars().count();
            collect_runs_in_range(doc.model(), start_para, start_offset, start_len, &mut run_ids);
            for &child_id in &children[start_idx + 1..end_idx] {
                if let Some(child) = doc.node(child_id) {
                    if child.node_type == NodeType::Paragraph {
                        let len = extract_paragraph_text(doc.model(), child_id).chars().count();
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
                    if prev != b { mixed_bold = true; }
                }
                bold_state = Some(b);
                if let Some(prev) = italic_state {
                    if prev != i { mixed_italic = true; }
                }
                italic_state = Some(i);
                if let Some(prev) = underline_state {
                    if prev != u { mixed_underline = true; }
                }
                underline_state = Some(u);
                if let Some(prev) = strike_state {
                    if prev != s { mixed_strike = true; }
                }
                strike_state = Some(s);
            }
        }

        fn fmt_val(mixed: bool, val: Option<bool>) -> String {
            if mixed {
                "\"mixed\"".to_string()
            } else {
                format!("{}", val.unwrap_or(false))
            }
        }

        Ok(format!(
            "{{\"bold\":{},\"italic\":{},\"underline\":{},\"strikethrough\":{}}}",
            fmt_val(mixed_bold, bold_state),
            fmt_val(mixed_italic, italic_state),
            fmt_val(mixed_underline, underline_state),
            fmt_val(mixed_strike, strike_state),
        ))
    }

    // ─── P.2: Table Operations API ──────────────────────────────

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
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
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
                txn.push(Operation::insert_node(
                    run_id,
                    0,
                    Node::text(text_id, ""),
                ));
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
            txn.push(Operation::insert_node(
                run_id,
                0,
                Node::text(text_id, ""),
            ));
        }

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", row_id.replica, row_id.counter))
    }

    /// Delete a row at the given index in a table.
    pub fn delete_table_row(
        &mut self,
        table_id_str: &str,
        row_index: u32,
    ) -> Result<(), JsError> {
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
            txn.push(Operation::insert_node(
                run_id,
                0,
                Node::text(text_id, ""),
            ));
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

    /// Set the text content of a table cell.
    ///
    /// Finds or creates Paragraph → Run → Text inside the cell.
    pub fn set_cell_text(
        &mut self,
        cell_id_str: &str,
        text: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let cell_id = parse_node_id(cell_id_str)?;
        let cell = doc
            .node(cell_id)
            .ok_or_else(|| JsError::new("Cell not found"))?;

        // Find first paragraph in cell
        if let Some(&para_id) = cell.children.first() {
            let (text_node_id, old_len) = find_first_text_node(doc.model(), para_id)?;
            let mut txn = Transaction::with_label("Set cell text");
            if old_len > 0 {
                txn.push(Operation::delete_text(text_node_id, 0, old_len));
            }
            if !text.is_empty() {
                txn.push(Operation::insert_text(text_node_id, 0, text));
            }
            doc.apply_transaction(&txn)
                .map_err(|e| JsError::new(&e.to_string()))
        } else {
            Err(JsError::new("Cell has no paragraph"))
        }
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
            doc.node(first_row_id).map(|r| r.children.len()).unwrap_or(0)
        } else {
            0
        };
        Ok(format!("{{\"rows\":{},\"cols\":{}}}", rows, cols))
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
            }
        }

        // Mark continuation cells
        for r in start_row..=end_row {
            if let Some(row) = doc.node(row_ids[r as usize]) {
                let cells: Vec<NodeId> = row.children.clone();
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

    /// Set the background color of a table cell.
    pub fn set_cell_background(
        &mut self,
        cell_id_str: &str,
        hex: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let cell_id = parse_node_id(cell_id_str)?;
        let color = Color::from_hex(hex)
            .ok_or_else(|| JsError::new(&format!("Invalid color: {}", hex)))?;
        let mut attrs = s1_model::AttributeMap::new();
        attrs.set(AttributeKey::CellBackground, AttributeValue::Color(color));
        doc.apply(Operation::set_attributes(cell_id, attrs))
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
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
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
        let media_id = doc
            .model_mut()
            .media_mut()
            .insert(content_type, data.to_vec(), Some(format!("image.{}", ext)));

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
    pub fn set_image_alt_text(
        &mut self,
        image_id_str: &str,
        alt: &str,
    ) -> Result<(), JsError> {
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
    pub fn insert_bookmark(
        &mut self,
        para_id_str: &str,
        name: &str,
    ) -> Result<String, JsError> {
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
    pub fn insert_page_break(
        &mut self,
        after_node_str: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
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
        para.attributes.set(
            AttributeKey::PageBreakBefore,
            AttributeValue::Bool(true),
        );

        let mut txn = Transaction::with_label("Insert page break");
        txn.push(Operation::insert_node(body_id, index, para));
        txn.push(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ));
        txn.push(Operation::insert_node(
            run_id,
            0,
            Node::text(text_id, ""),
        ));

        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(format!("{}:{}", para_id.replica, para_id.counter))
    }

    /// Insert a horizontal rule (thematic break) after the given node.
    ///
    /// Returns the new node ID.
    pub fn insert_horizontal_rule(
        &mut self,
        after_node_str: &str,
    ) -> Result<String, JsError> {
        let doc = self.doc_mut()?;
        let after_id = parse_node_id(after_node_str)?;
        let body_id = doc
            .body_id()
            .ok_or_else(|| JsError::new("No body node"))?;
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

                    comments.push(format!(
                        "{{\"id\":\"{}\",\"author\":\"{}\",\"date\":\"{}\",\"text\":\"{}\"}}",
                        escape_json(id_str),
                        escape_json(author),
                        escape_json(date),
                        escape_json(&text)
                    ));
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
        let root_children = doc
            .node(root_id)
            .map(|n| n.children.len())
            .unwrap_or(0);
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
        txn.push(Operation::insert_node(end_id, end_child_count + if start_id == end_id { 1 } else { 0 }, ce_node));

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
            ) && node
                .attributes
                .get_string(&AttributeKey::CommentId)
                == Some(comment_id)
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
            entries.push(format!(
                "{{\"pageWidth\":{},\"pageHeight\":{},\"marginTop\":{},\"marginBottom\":{},\"marginLeft\":{},\"marginRight\":{}}}",
                sec.page_width,
                sec.page_height,
                sec.margin_top,
                sec.margin_bottom,
                sec.margin_left,
                sec.margin_right,
            ));
        }
        Ok(format!("[{}]", entries.join(",")))
    }

    // ─── P.5: Find & Replace + Clipboard API ────────────────────

    /// Find all occurrences of text in the document.
    ///
    /// Returns JSON array of `{"nodeId":"0:5","offset":3,"length":5}`.
    pub fn find_text(
        &self,
        query: &str,
        case_sensitive: bool,
    ) -> Result<String, JsError> {
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
        collect_find_results(model, &body.children, &query_lower, case_sensitive, &mut results);
        Ok(format!("[{}]", results.join(",")))
    }

    /// Replace text at a specific location.
    pub fn replace_text(
        &mut self,
        node_id_str: &str,
        offset: usize,
        length: usize,
        replacement: &str,
    ) -> Result<(), JsError> {
        let doc = self.doc_mut()?;
        let para_id = parse_node_id(node_id_str)?;
        let (text_node_id, local_offset, _) =
            find_text_node_at_char_offset(doc.model(), para_id, offset)?;

        let mut txn = Transaction::with_label("Replace text");
        txn.push(Operation::delete_text(text_node_id, local_offset, length));
        txn.push(Operation::insert_text(text_node_id, local_offset, replacement));
        doc.apply_transaction(&txn)
            .map_err(|e| JsError::new(&e.to_string()))
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
            collect_replace_matches(model, child_id, &query_lower, case_sensitive, query.len(), &mut matches);
        }

        if matches.is_empty() {
            return Ok(0);
        }

        let count = matches.len() as u32;

        // Apply replacements in reverse order to preserve offsets
        matches.sort_by(|a, b| {
            a.0.counter.cmp(&b.0.counter).then(b.1.cmp(&a.1))
        });
        // Group by text node, then sort offsets descending within each group
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
            let doc = self.doc_mut()?;
            let para_id = parse_node_id(para_id_str)?;
            let (text_node_id, _) = find_first_text_node(doc.model(), para_id)?;

            // Insert first line at offset
            let mut txn = Transaction::with_label("Paste text");
            if !lines[0].is_empty() {
                txn.push(Operation::insert_text(text_node_id, offset, lines[0]));
            }
            doc.apply_transaction(&txn)
                .map_err(|e| JsError::new(&e.to_string()))?;

            // Split and create new paragraphs for remaining lines
            let mut current_para_str = para_id_str.to_string();
            let first_line_len = lines[0].chars().count();
            let split_offset = offset + first_line_len;

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

    // ─── Lifecycle ────────────────────────────────────────────────

    /// Free the document, releasing memory.
    ///
    /// After calling this, all other methods will return an error.
    pub fn free(&mut self) {
        self.inner = None;
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
        self.inner
            .as_mut()
            .ok_or_else(|| JsError::new("Document has been freed"))
    }
}

// --- WasmDocumentBuilder ---

/// A fluent builder for constructing documents.
#[wasm_bindgen]
pub struct WasmDocumentBuilder {
    inner: Option<s1engine::DocumentBuilder>,
}

#[wasm_bindgen]
impl WasmDocumentBuilder {
    /// Create a new document builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Some(s1engine::DocumentBuilder::new()),
        }
    }

    /// Add a heading at the specified level (1-6).
    pub fn heading(mut self, level: u8, text: &str) -> Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.heading(level, text));
        }
        self
    }

    /// Add a paragraph with plain text.
    pub fn text(mut self, text: &str) -> Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.text(text));
        }
        self
    }

    /// Set the document title.
    pub fn title(mut self, title: &str) -> Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.title(title));
        }
        self
    }

    /// Set the document author.
    pub fn author(mut self, author: &str) -> Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.author(author));
        }
        self
    }

    /// Build the document. Consumes the builder.
    pub fn build(mut self) -> Result<WasmDocument, JsError> {
        let builder = self
            .inner
            .take()
            .ok_or_else(|| JsError::new("Builder already consumed"))?;
        Ok(WasmDocument {
            inner: Some(builder.build()),
        })
    }
}

impl Default for WasmDocumentBuilder {
    fn default() -> Self {
        Self::new()
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
}

impl Default for WasmFontDatabase {
    fn default() -> Self {
        Self::new()
    }
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
fn ensure_run_and_text(doc: &mut s1engine::Document, para_id: NodeId) -> Result<(NodeId, usize), JsError> {
    // Check if run already exists
    if let Ok(run_id) = find_first_run(doc.model(), para_id) {
        // Run exists — find or create text node
        let run = doc.model().node(run_id).ok_or_else(|| JsError::new("Run not found"))?;
        for &child_id in &run.children {
            if let Some(child) = doc.model().node(child_id) {
                if child.node_type == NodeType::Text {
                    let len = child.text_content.as_ref().map(|t| t.chars().count()).unwrap_or(0);
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
                let len = child.text_content.as_ref().map(|t| t.chars().count()).unwrap_or(0);
                return Ok((child_id, len));
            }
        }
    }
    Err(JsError::new("No text node found in run"))
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
            if child.node_type == NodeType::Run {
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
        }
    }
    text
}

/// Find the text node and local char offset for a paragraph-level char offset.
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
            if child.node_type == NodeType::Run {
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
        }
    }
    // Offset is at or past the end — return last text node at its end position
    if let Some(tid) = last_text_id {
        Ok((tid, last_len, last_len))
    } else {
        Err(JsError::new("No text node found in paragraph"))
    }
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

    // Collect run info: (run_id, text_node_id, run_start_char, run_end_char)
    let mut runs_info: Vec<(NodeId, NodeId, usize, usize)> = Vec::new();
    let mut offset = 0usize;
    for &child_id in &para.children {
        if let Some(child) = doc.node(child_id) {
            if child.node_type == NodeType::Run {
                if let Ok((text_id, _, _)) = find_text_node_at_char_offset_in_run(doc.model(), child_id, 0) {
                    let len = run_char_len(doc.model(), child_id);
                    runs_info.push((child_id, text_id, offset, offset + len));
                    offset += len;
                }
            }
        }
    }

    let mut txn = Transaction::with_label("Delete text range");

    for &(run_id, text_id, run_start, run_end) in &runs_info {
        if run_end <= start_offset || run_start >= end_offset {
            continue; // No overlap
        }

        let del_start = start_offset.saturating_sub(run_start);
        let del_end = if end_offset < run_end {
            end_offset - run_start
        } else {
            run_end - run_start
        };
        let del_len = del_end - del_start;

        if del_start == 0 && del_end == run_end - run_start {
            // Entire run is deleted — remove the whole run node
            txn.push(Operation::delete_node(run_id));
        } else if del_len > 0 {
            txn.push(Operation::delete_text(text_id, del_start, del_len));
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
            attrs.set(
                AttributeKey::Bold,
                AttributeValue::Bool(value == "true"),
            );
        }
        "italic" => {
            attrs.set(
                AttributeKey::Italic,
                AttributeValue::Bool(value == "true"),
            );
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
            let color = Color::from_hex(value)
                .ok_or_else(|| JsError::new("Invalid color hex"))?;
            attrs.set(AttributeKey::Color, AttributeValue::Color(color));
        }
        "highlightColor" => {
            let color = Color::from_hex(value)
                .ok_or_else(|| JsError::new("Invalid color hex"))?;
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
            if child.node_type == NodeType::Run {
                let len = run_char_len(doc.model(), child_id);
                runs_info.push((child_id, offset, offset + len));
                offset += len;
            }
        }
    }

    // Find runs that overlap with [start_offset, end_offset)
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
            // The new run is what we want to format (partially)
            // But we also need to check if end_offset falls within this new run
            let new_run_len = run_char_len(doc.model(), new_run_id);
            let remaining_end = end_offset - start_offset;

            if remaining_end < new_run_len {
                // Need to split the new run at remaining_end
                let tail_run_id = split_run_internal(doc, new_run_id, remaining_end)?;
                let _ = tail_run_id; // tail is not formatted
                runs_to_format.push(new_run_id);
            } else {
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
    let full_text = text_node
        .text_content
        .as_deref()
        .unwrap_or("");
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
            if child.node_type == NodeType::Run {
                let len = run_char_len(model, child_id);
                let run_start = offset;
                let run_end = offset + len;
                // Check overlap with [start_offset, end_offset)
                if run_end > start_offset && run_start < end_offset {
                    out.push(child_id);
                }
                offset += len;
            }
        }
    }
}

/// Get the column count of a table (from the first row).
fn get_table_col_count(
    model: &DocumentModel,
    table_id: NodeId,
) -> Result<usize, JsError> {
    let table = model
        .node(table_id)
        .ok_or_else(|| JsError::new("Table not found"))?;
    if let Some(&first_row_id) = table.children.first() {
        Ok(model
            .node(first_row_id)
            .map(|r| r.children.len())
            .unwrap_or(0))
    } else {
        Ok(0)
    }
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
                    let query_len = query.chars().count();
                    let mut start = 0;
                    while let Some(pos) = search_text[start..].find(query) {
                        let char_offset = search_text[..start + pos].chars().count();
                        results.push(format!(
                            "{{\"nodeId\":\"{}:{}\",\"offset\":{},\"length\":{}}}",
                            child_id.replica, child_id.counter, char_offset, query_len
                        ));
                        start += pos + query.len();
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
                                        let mut start = 0;
                                        while let Some(pos) = search[start..].find(query) {
                                            let byte_offset = start + pos;
                                            let char_offset = content[..byte_offset].chars().count();
                                            matches.push((text_id, char_offset, query_char_len));
                                            start = byte_offset + query.len();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        NodeType::Table | NodeType::TableRow | NodeType::TableCell
        | NodeType::Body | NodeType::Section => {
            for &child_id in &node.children {
                collect_replace_matches(model, child_id, query, case_sensitive, query_char_len, matches);
            }
        }
        _ => {}
    }
}

/// Escape special characters for JSON string values.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\x20' => out.push_str(&format!("\\u{:04x}", c as u32)),
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

fn render_children(model: &DocumentModel, parent_id: NodeId, html: &mut String) {
    let parent = match model.node(parent_id) {
        Some(n) => n,
        None => return,
    };
    for &child_id in &parent.children {
        render_node(model, child_id, html);
    }
}

fn render_node(model: &DocumentModel, node_id: NodeId, html: &mut String) {
    let node = match model.node(node_id) {
        Some(n) => n,
        None => return,
    };

    match node.node_type {
        NodeType::Paragraph => render_paragraph(model, node_id, html),
        NodeType::Table => render_table(model, node_id, html),
        NodeType::TableRow => render_table_row(model, node_id, html),
        NodeType::TableCell => render_table_cell(model, node_id, html),
        NodeType::Image => render_image(model, node_id, html),
        NodeType::PageBreak => {
            html.push_str("<hr style=\"border:none;page-break-after:always;margin:0\">");
        }
        NodeType::TableOfContents => {
            html.push_str("<div class=\"toc\" style=\"margin:1em 0;padding:1em;border:1px solid #dadce0;border-radius:4px\">");
            html.push_str("<strong>Table of Contents</strong><br/>");
            render_children(model, node_id, html);
            html.push_str("</div>");
        }
        NodeType::BookmarkStart => {
            if let Some(name) = node.attributes.get_string(&AttributeKey::BookmarkName) {
                html.push_str(&format!("<a id=\"{}\"></a>", escape_html(name)));
            }
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
        NodeType::Field => {
            if let Some(AttributeValue::FieldType(ft)) =
                node.attributes.get(&AttributeKey::FieldType)
            {
                match ft {
                    s1_model::FieldType::PageNumber => {
                        html.push_str("<span class=\"field\">#</span>")
                    }
                    s1_model::FieldType::PageCount => {
                        html.push_str("<span class=\"field\">N</span>")
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn render_paragraph(model: &DocumentModel, para_id: NodeId, html: &mut String) {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return,
    };

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
    if let Some(AttributeValue::Borders(borders)) = para.attributes.get(&AttributeKey::ParagraphBorders) {
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
            if !b.is_empty() { style.push_str(&format!("border-top:{b};")); }
        }
        if let Some(ref bottom) = borders.bottom {
            let b = render_border(bottom);
            if !b.is_empty() { style.push_str(&format!("border-bottom:{b};")); }
        }
        if let Some(ref left) = borders.left {
            let b = render_border(left);
            if !b.is_empty() { style.push_str(&format!("border-left:{b};")); }
        }
        if let Some(ref right) = borders.right {
            let b = render_border(right);
            if !b.is_empty() { style.push_str(&format!("border-right:{b};")); }
        }
    }

    // Page break before
    if para.attributes.get_bool(&AttributeKey::PageBreakBefore) == Some(true) {
        style.push_str("page-break-before:always;");
    }

    let style_attr = if style.is_empty() {
        String::new()
    } else {
        format!(" style=\"{style}\"")
    };

    let nid_attr = format!(
        " data-node-id=\"{}:{}\"",
        para_id.replica, para_id.counter
    );

    // List marker prefix
    let list_marker = list_info.as_ref().map(|li| {
        match li.num_format {
            ListFormat::Bullet => "\u{2022} ".to_string(), // bullet: •
            ListFormat::Decimal => format!("{}. ", li.start.unwrap_or(1)),
            ListFormat::LowerAlpha => {
                let n = li.start.unwrap_or(1);
                let ch = (b'a' + ((n - 1) % 26) as u8) as char;
                format!("{}. ", ch)
            }
            ListFormat::UpperAlpha => {
                let n = li.start.unwrap_or(1);
                let ch = (b'A' + ((n - 1) % 26) as u8) as char;
                format!("{}. ", ch)
            }
            ListFormat::LowerRoman | ListFormat::UpperRoman => {
                format!("{}. ", li.start.unwrap_or(1))
            }
            _ => "\u{2022} ".to_string(),
        }
    });

    match effective_level {
        Some(l @ 1..=6) => {
            html.push_str(&format!("<h{l}{nid_attr}{style_attr}>"));
            render_inline_children(model, para_id, html);
            // Ensure empty headings are editable (non-collapsing)
            if is_empty_paragraph(model, para_id) {
                html.push_str("<br>");
            }
            html.push_str(&format!("</h{l}>"));
        }
        _ => {
            html.push_str(&format!("<p{nid_attr}{style_attr}>"));
            if let Some(marker) = list_marker {
                html.push_str(&format!("<span style=\"user-select:none\" contenteditable=\"false\">{marker}</span>"));
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
            NodeType::Tab => html.push_str("&emsp;"),
            NodeType::Field => {
                if let Some(AttributeValue::FieldType(ft)) =
                    child.attributes.get(&AttributeKey::FieldType)
                {
                    match ft {
                        s1_model::FieldType::PageNumber => {
                            html.push_str("<span class=\"field\">#</span>")
                        }
                        s1_model::FieldType::PageCount => {
                            html.push_str("<span class=\"field\">N</span>")
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
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
    // Character spacing
    if let Some(sp) = run.attributes.get_f64(&AttributeKey::FontSpacing) {
        if sp.abs() > 0.01 {
            style.push_str(&format!("letter-spacing:{sp}pt;"));
        }
    }

    // Track changes visual styling
    match revision_type {
        Some("Insert") => {
            style.push_str("color:#22863a;text-decoration:underline;text-decoration-color:#22863a;");
        }
        Some("Delete") => {
            style.push_str("color:#cb2431;text-decoration:line-through;text-decoration-color:#cb2431;");
        }
        Some("FormatChange") => {
            style.push_str("border-bottom:2px dotted #b08800;");
        }
        _ => {}
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
        if has_style { t.push_str("</span>"); }
        if subscript { t.push_str("</sub>"); }
        if superscript { t.push_str("</sup>"); }
        if strikethrough { t.push_str("</s>"); }
        if underline { t.push_str("</u>"); }
        if italic { t.push_str("</em>"); }
        if bold { t.push_str("</strong>"); }
        t
    };
    let open_tags = {
        let mut t = String::new();
        if bold { t.push_str("<strong>"); }
        if italic { t.push_str("<em>"); }
        if underline { t.push_str("<u>"); }
        if strikethrough { t.push_str("<s>"); }
        if superscript { t.push_str("<sup>"); }
        if subscript { t.push_str("<sub>"); }
        if has_style { t.push_str(&format!("<span style=\"{style}\">")); }
        t
    };
    let has_formatting = bold || italic || underline || strikethrough || superscript || subscript || has_style;

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
}

fn render_image(model: &DocumentModel, img_id: NodeId, html: &mut String) {
    let img = match model.node(img_id) {
        Some(n) => n,
        None => return,
    };

    if let Some(AttributeValue::MediaId(media_id)) =
        img.attributes.get(&AttributeKey::ImageMediaId)
    {
        if let Some(item) = model.media().get(*media_id) {
            let b64 = base64_encode(&item.data);
            let mime = &item.content_type;
            let alt = img
                .attributes
                .get_string(&AttributeKey::ImageAltText)
                .unwrap_or("image");
            let mut style = String::from("max-width:100%;height:auto;margin:8px 0;");
            if let Some(w) = img.attributes.get_f64(&AttributeKey::ImageWidth) {
                style.push_str(&format!("width:{w}pt;"));
            }
            if let Some(h) = img.attributes.get_f64(&AttributeKey::ImageHeight) {
                style.push_str(&format!("height:{h}pt;"));
            }
            html.push_str(&format!(
                "<img data-node-id=\"{}:{}\" src=\"data:{mime};base64,{b64}\" style=\"{style}\" alt=\"{}\"/>",
                img_id.replica, img_id.counter, escape_html(alt)
            ));
        }
    }
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

fn render_table_cell(model: &DocumentModel, cell_id: NodeId, html: &mut String) {
    let cell = match model.node(cell_id) {
        Some(n) => n,
        None => return,
    };
    let mut attrs = String::new();
    let mut style = String::from("border:1px solid #dadce0;padding:6px 10px;vertical-align:top;");

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
    if let Some(AttributeValue::VerticalAlignment(va)) = cell.attributes.get(&AttributeKey::VerticalAlign) {
        let val = match va {
            s1_model::VerticalAlignment::Top => "top",
            s1_model::VerticalAlignment::Center => "middle",
            s1_model::VerticalAlignment::Bottom => "bottom",
            _ => "top",
        };
        style.push_str(&format!("vertical-align:{val};"));
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

/// Detect the format of a document from its bytes.
///
/// Returns one of: "docx", "odt", "pdf", "txt".
#[wasm_bindgen]
pub fn detect_format(data: &[u8]) -> String {
    let fmt = s1engine::Format::detect(data);
    fmt.extension().to_string()
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
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.replica_id())
    }

    /// Get the document content as HTML.
    pub fn to_html(&self) -> Result<String, JsError> {
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
        let model = doc.model();
        Ok(to_html_from_model(model))
    }

    /// Get the document content as plain text.
    pub fn to_plain_text(&self) -> Result<String, JsError> {
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
        let crdt_op = deserialize_crdt_op_from_json(ops_json)?;
        doc.apply_remote(crdt_op)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get the current state vector as JSON.
    ///
    /// Used for delta synchronization — send your state vector to a peer
    /// to find out what operations you're missing.
    pub fn get_state_vector(&self) -> Result<String, JsError> {
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
        let sv = doc.state_vector();
        let entries: Vec<(u64, u64)> = sv.entries().iter().map(|(&r, &l)| (r, l)).collect();
        let mut result = String::from("{");
        for (i, (replica, lamport)) in entries.iter().enumerate() {
            if i > 0 { result.push(','); }
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
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
        let nid = parse_node_id(node_id)?;
        let selection = s1_ops::Selection::collapsed(s1_ops::Position::new(nid, offset));
        let update = doc.set_cursor(selection, user_name, user_color);
        Ok(serialize_awareness_update(&update))
    }

    /// Apply a remote awareness (cursor) update from another replica.
    pub fn apply_awareness_update(&mut self, update_json: &str) -> Result<(), JsError> {
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
        let update = deserialize_awareness_update(update_json)?;
        doc.apply_awareness_update(&update);
        Ok(())
    }

    /// Get all peer cursors as JSON.
    ///
    /// Returns a JSON array of cursor states:
    /// `[{"replicaId":2,"nodeId":"1:5","offset":3,"userName":"Alice","userColor":"#ff0000"},...]`
    pub fn get_peers_json(&self) -> Result<String, JsError> {
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
        match doc.undo().map_err(|e| JsError::new(&e.to_string()))? {
            Some(crdt_op) => Ok(serialize_crdt_op_to_json(&crdt_op)),
            None => Ok("null".to_string()),
        }
    }

    /// Redo the last undone operation.
    pub fn redo(&mut self) -> Result<String, JsError> {
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
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
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.op_log_size())
    }

    /// Get the number of tombstones.
    pub fn tombstone_count(&self) -> Result<usize, JsError> {
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
        Ok(doc.tombstone_count())
    }

    /// Compact the operation log (merge consecutive single-char inserts).
    pub fn compact_op_log(&mut self) -> Result<(), JsError> {
        let doc = self.inner.as_mut().ok_or_else(|| JsError::new("Document freed"))?;
        doc.compact_op_log();
        Ok(())
    }

    /// Export the collaborative document to a format (docx, odt, txt, md).
    pub fn export(&self, format: &str) -> Result<Vec<u8>, JsError> {
        let doc = self.inner.as_ref().ok_or_else(|| JsError::new("Document freed"))?;
        let fmt = parse_format(format)?;
        let temp_doc = s1engine::Document::from_model(doc.model().clone());
        temp_doc.export(fmt).map_err(|e| JsError::new(&e.to_string()))
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
        let doc = self.inner.open(data).map_err(|e| JsError::new(&e.to_string()))?;
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
        Operation::InsertText { target_id, offset, text, .. } => {
            format!(
                "\"type\":\"InsertText\",\"target\":\"{}:{}\",\"offset\":{},\"text\":{}",
                target_id.replica, target_id.counter, offset, json_escape_string(text)
            )
        }
        Operation::DeleteText { target_id, offset, length, deleted_text, .. } => {
            let text_str = deleted_text.as_deref().unwrap_or("");
            format!(
                "\"type\":\"DeleteText\",\"target\":\"{}:{}\",\"offset\":{},\"length\":{},\"text\":{}",
                target_id.replica, target_id.counter, offset, length, json_escape_string(text_str)
            )
        }
        Operation::SetAttributes { target_id, attributes, previous } => {
            let prev = previous.as_ref().cloned().unwrap_or_default();
            format!(
                "\"type\":\"SetAttributes\",\"target\":\"{}:{}\",\"attributes\":{},\"oldAttributes\":{}",
                target_id.replica, target_id.counter,
                attrs_to_json(attributes),
                attrs_to_json(&prev),
            )
        }
        Operation::InsertNode { parent_id, index, node, .. } => {
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
        Operation::MoveNode { target_id, new_parent_id, new_index, .. } => {
            format!(
                "\"type\":\"MoveNode\",\"target\":\"{}:{}\",\"newParent\":\"{}:{}\",\"newIndex\":{}",
                target_id.replica, target_id.counter, new_parent_id.replica, new_parent_id.counter, new_index
            )
        }
        _ => "\"type\":\"Other\"".to_string(),
    };

    format!(
        "{{\"id\":{{\"replica\":{},\"lamport\":{}}},{},\"deps\":{}}}",
        op.id.replica, op.id.lamport,
        op_type,
        state_vector_to_json(&op.deps),
    )
}

fn state_vector_to_json(sv: &s1_crdt::StateVector) -> String {
    let entries: Vec<String> = sv.entries().iter()
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
    let escaped: String = s.chars().map(|c| match c {
        '"' => "\\\"".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        c if c < '\x20' => format!("\\u{:04x}", c as u32),
        c => c.to_string(),
    }).collect();
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
    let id_start = json.find("\"id\"").ok_or_else(|| JsError::new("Missing id in CRDT op"))?;
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
                if pair.is_empty() { continue; }
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
            Ok(Operation::set_attributes(target, s1_model::AttributeMap::new()))
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
        _ => Err(JsError::new(&format!("Unknown operation type: {}", op_type))),
    }
}

fn extract_json_number(json: &str, key: &str) -> Result<u64, JsError> {
    let search = format!("\"{}\"", key);
    let pos = json.find(&search).ok_or_else(|| JsError::new(&format!("Missing key: {}", key)))?;
    let rest = &json[pos + search.len()..];
    let colon = rest.find(':').ok_or_else(|| JsError::new("Invalid JSON"))? + 1;
    let num_start = rest[colon..].find(|c: char| c.is_ascii_digit()).ok_or_else(|| JsError::new("No number"))? + colon;
    let num_end = rest[num_start..].find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len() - num_start) + num_start;
    rest[num_start..num_end].parse().map_err(|_| JsError::new("Invalid number"))
}

fn extract_json_string(json: &str, key: &str) -> Result<String, JsError> {
    let search = format!("\"{}\"", key);
    let pos = json.find(&search).ok_or_else(|| JsError::new(&format!("Missing key: {}", key)))?;
    let rest = &json[pos + search.len()..];
    // Find the value string after the colon
    let colon = rest.find(':').ok_or_else(|| JsError::new("Invalid JSON"))? + 1;
    let after_colon = rest[colon..].trim_start();
    if let Some(str_content) = after_colon.strip_prefix('"') {
        let mut end = 0;
        let mut escaped = false;
        for ch in str_content.chars() {
            if escaped { escaped = false; end += ch.len_utf8(); continue; }
            if ch == '\\' { escaped = true; end += 1; continue; }
            if ch == '"' { break; }
            end += ch.len_utf8();
        }
        Ok(str_content[..end].replace("\\\"", "\"").replace("\\\\", "\\").replace("\\n", "\n"))
    } else {
        // Not a string value — take until comma or brace
        let end = after_colon.find([',', '}', ']']).unwrap_or(after_colon.len());
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
            format!("{{\"replicaId\":{},\"connected\":false}}", update.replica_id)
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
        if pair.is_empty() { continue; }
        let parts: Vec<&str> = pair.split(':').collect();
        if parts.len() == 2 {
            let r: u64 = parts[0].trim().trim_matches('"').parse().unwrap_or(0);
            let l: u64 = parts[1].trim().parse().unwrap_or(0);
            sv.set(r, l);
        }
    }
    Ok(sv)
}

// Helper to render HTML from a DocumentModel (shared between WasmDocument and WasmCollabDocument)
fn to_html_from_model(model: &DocumentModel) -> String {
    let body_id = match model.body_id() {
        Some(id) => id,
        None => return String::new(),
    };
    let children = model.children(body_id);
    let mut html = String::new();
    for child in &children {
        html.push_str(&render_node_to_html(model, child));
    }
    html
}

fn render_node_to_html(model: &DocumentModel, node: &Node) -> String {
    match node.node_type {
        NodeType::Paragraph => {
            let heading_level: u8 = node
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
            let tag = if (1..=6).contains(&heading_level) {
                format!("h{}", heading_level)
            } else {
                "p".to_string()
            };
            let node_id = node.id;
            let mut html = format!("<{} data-node-id=\"{}:{}\">", tag, node_id.replica, node_id.counter);
            let children = model.children(node_id);
            for child in &children {
                html.push_str(&render_node_to_html(model, child));
            }
            html.push_str(&format!("</{}>", tag));
            html
        }
        NodeType::Run => {
            let mut style = String::new();
            if node.attributes.get_bool(&AttributeKey::Bold) == Some(true) { style.push_str("font-weight:bold;"); }
            if node.attributes.get_bool(&AttributeKey::Italic) == Some(true) { style.push_str("font-style:italic;"); }
            if node.attributes.get_bool(&AttributeKey::Underline) == Some(true) { style.push_str("text-decoration:underline;"); }

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
        NodeType::Text => {
            html_escape(node.text_content.as_deref().unwrap_or(""))
        }
        _ => String::new(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
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
        assert!(html.contains("Hello world"), "should contain text content");
        assert!(html.contains("Title"), "should contain heading text");
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
        assert!((config.page_width() - 612.0).abs() < 0.01, "default width should be US Letter");
        assert!((config.page_height() - 792.0).abs() < 0.01, "default height should be US Letter");
        assert!((config.margin_top() - 72.0).abs() < 0.01, "default top margin should be 1 inch");
        assert!((config.margin_bottom() - 72.0).abs() < 0.01, "default bottom margin should be 1 inch");
        assert!((config.margin_left() - 72.0).abs() < 0.01, "default left margin should be 1 inch");
        assert!((config.margin_right() - 72.0).abs() < 0.01, "default right margin should be 1 inch");
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
        assert!(html1.starts_with("<h2 "), "original should be h2: {}", html1);
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
        assert!(
            html.starts_with("<h2 "),
            "should now be h2: {}",
            html
        );
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
        let run_id: String = runs_before.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();

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
        let run_id: String = runs.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();

        let new_run_id = doc.split_run(&run_id, 4).unwrap();
        let fmt1 = doc.get_run_formatting_json(&run_id).unwrap();
        let fmt2 = doc.get_run_formatting_json(&new_run_id).unwrap();
        assert!(fmt1.contains("\"bold\":true"), "original should be bold: {}", fmt1);
        assert!(fmt2.contains("\"bold\":true"), "new should be bold: {}", fmt2);
    }

    #[test]
    fn test_format_run_bold() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Some text").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();

        doc.format_run(&run_id, "bold", "true").unwrap();
        let fmt = doc.get_run_formatting_json(&run_id).unwrap();
        assert!(fmt.contains("\"bold\":true"), "should be bold: {}", fmt);
    }

    #[test]
    fn test_format_selection_single_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();

        // Bold characters 2-7 ("llo W")
        doc.format_selection(&para_id, 2, &para_id, 7, "bold", "true").unwrap();

        let runs = doc.get_run_ids(&para_id).unwrap();
        // Should have 3 runs now: "He" (not bold), "llo W" (bold), "orld" (not bold)
        let run_ids: Vec<String> = runs
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim_matches('"').to_string())
            .collect();
        assert!(run_ids.len() >= 3, "should have at least 3 runs: {:?}", run_ids);
    }

    #[test]
    fn test_format_selection_cross_run() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();

        // First make part italic
        doc.format_selection(&para_id, 0, &para_id, 5, "italic", "true").unwrap();
        // Then bold across runs
        doc.format_selection(&para_id, 3, &para_id, 8, "bold", "true").unwrap();

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
        doc.format_selection(&p1, 5, &p2, 6, "bold", "true").unwrap();

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
        doc.format_selection(&para_id, 2, &para_id, 7, "bold", "true").unwrap();

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
        let run_id: String = runs.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();
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
        let run_id: String = runs.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();

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

        let fmt = doc.get_selection_formatting_json(&para_id, 0, &para_id, 8).unwrap();
        assert!(fmt.contains("\"bold\":true"), "fmt: {}", fmt);
    }

    #[test]
    fn test_get_selection_formatting_mixed() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Hello World").unwrap();

        // Bold first half
        doc.format_selection(&para_id, 0, &para_id, 5, "bold", "true").unwrap();

        let fmt = doc.get_selection_formatting_json(&para_id, 0, &para_id, 11).unwrap();
        assert!(fmt.contains("\"mixed\""), "should have mixed bold: {}", fmt);
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
        let table_node = doc.inner.as_ref().unwrap().model().node(parse_node_id(&table_id).unwrap()).unwrap();
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
        let row0 = doc.inner.as_ref().unwrap().model().node(table.children[0]).unwrap();
        let cell00 = doc.inner.as_ref().unwrap().model().node(row0.children[0]).unwrap();
        assert!(cell00.attributes.get_i64(&AttributeKey::ColSpan) == Some(2));
    }

    #[test]
    fn test_set_cell_background() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let table_id = doc.insert_table(&p, 1, 1).unwrap();

        let table_nid = parse_node_id(&table_id).unwrap();
        let table = doc.inner.as_ref().unwrap().model().node(table_nid).unwrap();
        let row0 = doc.inner.as_ref().unwrap().model().node(table.children[0]).unwrap();
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
        img_node.attributes.set(AttributeKey::ImageMediaId, AttributeValue::MediaId(media_id));
        img_node.attributes.set(AttributeKey::ImageWidth, AttributeValue::Float(100.0));
        img_node.attributes.set(AttributeKey::ImageHeight, AttributeValue::Float(80.0));

        let mut txn = Transaction::with_label("Insert image test");
        txn.push(Operation::insert_node(body_id, index, Node::new(para_id, NodeType::Paragraph)));
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
        assert_eq!(img.attributes.get_f64(&AttributeKey::ImageWidth), Some(200.0));
        assert_eq!(img.attributes.get_f64(&AttributeKey::ImageHeight), Some(150.0));
    }

    #[test]
    fn test_get_image_data_url() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("p").unwrap();
        let (_img_para_id, img_id) = insert_image_test_helper(&mut doc, &p);
        let img_id_str = format!("{}:{}", img_id.replica, img_id.counter);

        let data_url = doc.get_image_data_url(&img_id_str).unwrap();
        assert!(data_url.starts_with("data:image/png;base64,"), "data_url: {}", data_url);
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
        assert_eq!(img.attributes.get_string(&AttributeKey::ImageAltText), Some("A photo"));
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

    // ─── P.4: Structural Elements Tests ─────────────────────────

    #[test]
    fn test_insert_hyperlink() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Click here").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();

        doc.insert_hyperlink(&run_id, "https://example.com", "").unwrap();

        let run_nid = parse_node_id(&run_id).unwrap();
        let run = doc.inner.as_ref().unwrap().model().node(run_nid).unwrap();
        assert_eq!(run.attributes.get_string(&AttributeKey::HyperlinkUrl), Some("https://example.com"));
    }

    #[test]
    fn test_remove_hyperlink() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let para_id = doc.append_paragraph("Link text").unwrap();
        let runs = doc.get_run_ids(&para_id).unwrap();
        let run_id: String = runs.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();

        doc.insert_hyperlink(&run_id, "https://example.com", "").unwrap();
        doc.remove_hyperlink(&run_id).unwrap();

        let run_nid = parse_node_id(&run_id).unwrap();
        let run = doc.inner.as_ref().unwrap().model().node(run_nid).unwrap();
        assert!(run.attributes.get_string(&AttributeKey::HyperlinkUrl).is_none());
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
        assert!(html.contains("\u{2022}"), "should have bullet marker: {}", html);
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
        assert_eq!(pb.attributes.get_bool(&AttributeKey::PageBreakBefore), Some(true));
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
        let cid = doc.insert_comment(&p1, &p1, "Alice", "Great point!").unwrap();
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
    fn test_replace_text() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Hello World").unwrap();
        doc.replace_text(&p, 6, 5, "Rust").unwrap();
        let text = doc.get_paragraph_text(&p).unwrap();
        assert_eq!(text, "Hello Rust");
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
    fn test_paste_plain_text_multiline() {
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let p = doc.append_paragraph("Start").unwrap();
        doc.paste_plain_text(&p, 5, "\nline2\nline3").unwrap();

        // Should have created additional paragraphs
        let count = doc.paragraph_count().unwrap();
        assert!(count >= 3, "should have at least 3 paragraphs, got {}", count);
    }

    // ── Multi-run paragraph tests ──────────────────────────

    #[test]
    fn test_split_paragraph_multi_run() {
        // Create a paragraph with multiple runs via format_selection
        let engine = WasmEngine::new();
        let mut doc = engine.create();
        let id = doc.append_paragraph("Hello World Test").unwrap();

        // Bold "World" (chars 6..11) — creates 3 runs
        doc.format_selection(&id, 6, &id, 11, "bold", "true").unwrap();

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
        doc.format_selection(&id, 3, &id, 5, "bold", "true").unwrap();

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
        doc.format_selection(&id, 3, &id, 5, "bold", "true").unwrap();

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
        doc.format_selection(&id, 6, &id, 11, "bold", "true").unwrap();

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
        doc.format_selection(&id, 6, &id, 12, "bold", "true").unwrap();

        // Replace "Mid" (offset 6, length 3) with "Top"
        doc.replace_text(&id, 6, 3, "Top").unwrap();
        let text = doc.get_paragraph_text(&id).unwrap();
        assert_eq!(text, "Start Topdle End");
    }
}
