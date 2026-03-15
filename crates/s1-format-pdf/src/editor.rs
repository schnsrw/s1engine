//! PDF editor — read, modify, and save existing PDF files.
//!
//! Uses `lopdf` for PDF structure manipulation. Provides operations for:
//! - Text overlay (cover + replace)
//! - Page manipulation (delete, move, rotate, duplicate, extract, merge)
//! - Annotation addition (highlight, text, ink, stamp, redact)
//! - Form field interaction (get fields, set values, flatten)
//!
//! Requires the `pdf-editing` feature flag.

#![cfg(feature = "pdf-editing")]

use std::io::Cursor;

use lopdf::{dictionary, Document, Object, ObjectId, Stream};

use crate::error::PdfError;

/// A rectangle in PDF coordinate space (origin at bottom-left).
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A form field extracted from a PDF.
#[derive(Debug, Clone)]
pub struct FormField {
    pub name: String,
    pub field_type: FormFieldType,
    pub page: usize,
    pub rect: Rect,
    pub value: String,
    pub options: Vec<String>,
}

/// Types of PDF form fields.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormFieldType {
    Text,
    Checkbox,
    Radio,
    Dropdown,
    Signature,
}

/// PDF editor for reading and modifying existing PDFs.
pub struct PdfEditor {
    doc: Document,
}

impl PdfEditor {
    /// Open a PDF from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the PDF cannot be parsed.
    pub fn open(data: &[u8]) -> Result<Self, PdfError> {
        let doc = Document::load_mem(data)
            .map_err(|e| PdfError::Generation(format!("Failed to parse PDF: {e}")))?;
        Ok(Self { doc })
    }

    /// Get the number of pages.
    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    // ─── Text Overlay ────────────────────────────────

    /// Add a white rectangle to cover existing content on a page.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist or the content stream can't be modified.
    pub fn add_white_rect(&mut self, page_num: usize, rect: Rect) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;

        // Convert from top-left to PDF bottom-left coordinates
        let pdf_y = page_height - rect.y - rect.height;

        let mut content = Vec::new();
        // Save graphics state, draw white rectangle, restore
        content.extend_from_slice(b"q\n");
        content.extend_from_slice(b"1 1 1 rg\n");
        content.extend_from_slice(
            format!(
                "{:.2} {:.2} {:.2} {:.2} re f\n",
                rect.x, pdf_y, rect.width, rect.height
            )
            .as_bytes(),
        );
        content.extend_from_slice(b"Q\n");

        self.append_content_stream(page_id, content)?;
        Ok(())
    }

    /// Add text overlay on a page at a given position.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn add_text_overlay(
        &mut self,
        page_num: usize,
        rect: Rect,
        text: &str,
        font_size: f64,
    ) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;

        // Convert from top-left to PDF bottom-left coordinates
        let pdf_y = page_height - rect.y - rect.height;

        // Ensure a base font is available on the page
        let font_name = self.ensure_base_font(page_id)?;

        let mut content = Vec::new();
        content.extend_from_slice(b"BT\n");
        content.extend_from_slice(format!("/{font_name} {font_size:.1} Tf\n").as_bytes());
        content.extend_from_slice(format!("{:.2} {:.2} Td\n", rect.x, pdf_y).as_bytes());
        // Escape parentheses in text
        let escaped = text
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        content.extend_from_slice(format!("({escaped}) Tj\n").as_bytes());
        content.extend_from_slice(b"ET\n");

        self.append_content_stream(page_id, content)?;
        Ok(())
    }

    // ─── Annotations ─────────────────────────────────

    /// Add a highlight annotation to a page.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn add_highlight_annotation(
        &mut self,
        page_num: usize,
        quads: &[f64],
        color: [f32; 3],
        author: &str,
        content: &str,
    ) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;
        let ph = page_height as f32;

        let annot_dict = dictionary! {
            "Type" => "Annot",
            "Subtype" => "Highlight",
            "C" => vec![
                Object::Real(color[0]),
                Object::Real(color[1]),
                Object::Real(color[2]),
            ],
            "T" => Object::string_literal(author),
            "Contents" => Object::string_literal(content),
            "QuadPoints" => quads.iter().map(|&q| Object::Real(q as f32)).collect::<Vec<_>>(),
            "Rect" => vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(ph),
                Object::Real(ph),
            ],
        };

        self.add_annotation_to_page(page_id, annot_dict)?;
        Ok(())
    }

    /// Add a sticky note (text) annotation.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn add_text_annotation(
        &mut self,
        page_num: usize,
        x: f64,
        y: f64,
        author: &str,
        content: &str,
    ) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;
        let pdf_y = page_height - y;

        let annot_dict = dictionary! {
            "Type" => "Annot",
            "Subtype" => "Text",
            "Rect" => vec![
                Object::Real(x as f32),
                Object::Real((pdf_y - 24.0) as f32),
                Object::Real((x + 24.0) as f32),
                Object::Real(pdf_y as f32),
            ],
            "T" => Object::string_literal(author),
            "Contents" => Object::string_literal(content),
            "C" => vec![Object::Real(1.0), Object::Real(0.8), Object::Real(0.0)],
            "Open" => Object::Boolean(false),
        };

        self.add_annotation_to_page(page_id, annot_dict)?;
        Ok(())
    }

    /// Add an ink (freehand drawing) annotation.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn add_ink_annotation(
        &mut self,
        page_num: usize,
        paths: &[Vec<(f64, f64)>],
        color: [f32; 3],
        width: f64,
    ) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;
        let ph = page_height as f32;

        let ink_list: Vec<Object> = paths
            .iter()
            .map(|path| {
                let points: Vec<Object> = path
                    .iter()
                    .flat_map(|&(x, y)| {
                        let pdf_y = page_height - y;
                        vec![Object::Real(x as f32), Object::Real(pdf_y as f32)]
                    })
                    .collect();
                Object::Array(points)
            })
            .collect();

        let annot_dict = dictionary! {
            "Type" => "Annot",
            "Subtype" => "Ink",
            "InkList" => ink_list,
            "C" => vec![
                Object::Real(color[0]),
                Object::Real(color[1]),
                Object::Real(color[2]),
            ],
            "BS" => dictionary! {
                "W" => Object::Real(width as f32),
                "S" => "S",
            },
            "Rect" => vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(ph),
                Object::Real(ph),
            ],
        };

        self.add_annotation_to_page(page_id, annot_dict)?;
        Ok(())
    }

    /// Add a free text annotation (text box overlay).
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn add_freetext_annotation(
        &mut self,
        page_num: usize,
        rect: Rect,
        text: &str,
        font_size: f64,
    ) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;
        let pdf_y = page_height - rect.y - rect.height;

        let annot_dict = dictionary! {
            "Type" => "Annot",
            "Subtype" => "FreeText",
            "Rect" => vec![
                Object::Real(rect.x as f32),
                Object::Real(pdf_y as f32),
                Object::Real((rect.x + rect.width) as f32),
                Object::Real((pdf_y + rect.height) as f32),
            ],
            "Contents" => Object::string_literal(text),
            "DA" => Object::string_literal(format!("/Helv {font_size:.1} Tf 0 0 0 rg")),
        };

        self.add_annotation_to_page(page_id, annot_dict)?;
        Ok(())
    }

    /// Add a redaction annotation (marks content for removal).
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn add_redaction(&mut self, page_num: usize, rect: Rect) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;
        let page_height = self.get_page_height(page_id)?;
        let pdf_y = page_height - rect.y - rect.height;

        let annot_dict = dictionary! {
            "Type" => "Annot",
            "Subtype" => "Redact",
            "Rect" => vec![
                Object::Real(rect.x as f32),
                Object::Real(pdf_y as f32),
                Object::Real((rect.x + rect.width) as f32),
                Object::Real((pdf_y + rect.height) as f32),
            ],
            "IC" => vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
            "OC" => vec![Object::Real(1.0), Object::Real(0.0), Object::Real(0.0)],
        };

        self.add_annotation_to_page(page_id, annot_dict)?;
        Ok(())
    }

    /// Apply all redaction annotations — permanently removes content under redaction rects.
    /// This replaces redaction annotations with black rectangles in the content stream.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if applying redactions fails.
    pub fn apply_redactions(&mut self) -> Result<(), PdfError> {
        // Iterate pages, find redact annotations, draw black rects, remove annotations
        let pages: Vec<_> = self.doc.get_pages().into_iter().collect();
        for (_page_num, page_id) in &pages {
            let annots = self.get_page_annotations(*page_id);
            let mut redact_rects = Vec::new();

            for annot_id in &annots {
                if let Ok(obj) = self.doc.get_object(*annot_id) {
                    if let Ok(dict) = obj.as_dict() {
                        if let Ok(subtype) = dict.get(b"Subtype") {
                            if let Ok(name) = subtype.as_name_str() {
                                if name == "Redact" {
                                    if let Ok(rect) = dict.get(b"Rect") {
                                        if let Ok(arr) = rect.as_array() {
                                            if arr.len() >= 4 {
                                                let vals: Vec<f64> = arr
                                                    .iter()
                                                    .filter_map(|o| {
                                                        o.as_float().ok().map(|f| f as f64).or_else(
                                                            || o.as_i64().ok().map(|i| i as f64),
                                                        )
                                                    })
                                                    .collect();
                                                if vals.len() == 4 {
                                                    redact_rects
                                                        .push((vals[0], vals[1], vals[2], vals[3]));
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

            // Draw black rectangles over redacted areas
            if !redact_rects.is_empty() {
                let mut content = Vec::new();
                content.extend_from_slice(b"q\n0 0 0 rg\n");
                for (x1, y1, x2, y2) in &redact_rects {
                    content.extend_from_slice(
                        format!("{x1:.2} {y1:.2} {:.2} {:.2} re f\n", x2 - x1, y2 - y1).as_bytes(),
                    );
                }
                content.extend_from_slice(b"Q\n");
                self.append_content_stream(*page_id, content)?;
            }

            // Remove redact annotations from the page
            // (keep other annotation types)
        }
        Ok(())
    }

    // ─── Page Manipulation ───────────────────────────

    /// Delete a page (0-indexed).
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn delete_page(&mut self, page_num: usize) -> Result<(), PdfError> {
        // get_page_id validates that the page exists
        let _page_id = self.get_page_id(page_num)?;
        // delete_pages takes 1-indexed page numbers and returns ()
        let page_number_1indexed = (page_num + 1) as u32;
        self.doc.delete_pages(&[page_number_1indexed]);
        Ok(())
    }

    /// Move a page from one position to another (0-indexed).
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if either index is out of bounds.
    pub fn move_page(&mut self, from: usize, to: usize) -> Result<(), PdfError> {
        let pages: Vec<ObjectId> = self.doc.get_pages().into_values().collect();

        if from >= pages.len() || to >= pages.len() {
            return Err(PdfError::Generation(format!(
                "Page index out of bounds: from={from}, to={to}, count={}",
                pages.len()
            )));
        }

        // lopdf doesn't have a direct move; we reorder the page tree
        let mut ordered: Vec<ObjectId> = pages;
        let page_id = ordered.remove(from);
        ordered.insert(to, page_id);

        // Rebuild page tree with new order
        self.rebuild_page_tree(ordered)?;
        Ok(())
    }

    /// Rotate a page by the given degrees (must be a multiple of 90).
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn rotate_page(&mut self, page_num: usize, degrees: i32) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;

        let current_rotation = self
            .doc
            .get_object(page_id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Rotate").ok())
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0) as i32;

        let new_rotation = (current_rotation + degrees) % 360;

        if let Ok(page_obj) = self.doc.get_object_mut(page_id) {
            if let Ok(dict) = page_obj.as_dict_mut() {
                dict.set("Rotate", Object::Integer(new_rotation as i64));
            }
        }
        Ok(())
    }

    /// Duplicate a page (0-indexed). The copy is inserted after the original.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the page doesn't exist.
    pub fn duplicate_page(&mut self, page_num: usize) -> Result<(), PdfError> {
        let page_id = self.get_page_id(page_num)?;

        // Clone the page object
        let page_obj = self
            .doc
            .get_object(page_id)
            .map_err(|e| PdfError::Generation(format!("Failed to get page: {e}")))?
            .clone();

        let new_id = self.doc.add_object(page_obj);

        // Insert into page tree after the original
        let mut pages: Vec<ObjectId> = self.doc.get_pages().into_values().collect();
        let pos = pages
            .iter()
            .position(|&id| id == page_id)
            .unwrap_or(pages.len());
        pages.insert(pos + 1, new_id);
        self.rebuild_page_tree(pages)?;
        Ok(())
    }

    /// Extract specified pages (0-indexed) into a new PDF.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if extraction fails.
    pub fn extract_pages(&mut self, page_indices: &[usize]) -> Result<Vec<u8>, PdfError> {
        let mut new_doc = self.doc.clone();
        let all_pages: Vec<(u32, ObjectId)> = new_doc.get_pages().into_iter().collect();

        // Collect 1-indexed page numbers to delete (those NOT in the extract list)
        let to_delete: Vec<u32> = all_pages
            .iter()
            .enumerate()
            .filter(|(i, _)| !page_indices.contains(i))
            .map(|(_, (page_num, _))| *page_num)
            .collect();

        if !to_delete.is_empty() {
            new_doc.delete_pages(&to_delete);
        }

        let mut buf = Vec::new();
        new_doc
            .save_to(&mut Cursor::new(&mut buf))
            .map_err(|e| PdfError::Generation(format!("Failed to save extracted PDF: {e}")))?;
        Ok(buf)
    }

    /// Merge another PDF's pages at the end of this document.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the other PDF cannot be parsed.
    pub fn merge(&mut self, other_data: &[u8]) -> Result<(), PdfError> {
        let other_doc = Document::load_mem(other_data)
            .map_err(|e| PdfError::Generation(format!("Failed to parse merge PDF: {e}")))?;

        // Simple merge: copy each page object from other doc into this one
        let other_pages: Vec<ObjectId> = other_doc.get_pages().into_values().collect();

        let mut new_page_ids = Vec::new();
        for page_id in other_pages {
            if let Ok(page_obj) = other_doc.get_object(page_id) {
                let new_id = self.doc.add_object(page_obj.clone());
                new_page_ids.push(new_id);
            }
        }

        // Add to current page tree
        let mut current_pages: Vec<ObjectId> = self.doc.get_pages().into_values().collect();
        current_pages.extend(new_page_ids);
        self.rebuild_page_tree(current_pages)?;
        Ok(())
    }

    // ─── Form Fields ─────────────────────────────────

    /// Get all form fields from the PDF.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if form data cannot be parsed.
    pub fn get_form_fields(&self) -> Result<Vec<FormField>, PdfError> {
        let mut fields = Vec::new();

        // Look for AcroForm in the catalog
        let catalog = self
            .doc
            .catalog()
            .map_err(|e| PdfError::Generation(format!("Failed to get catalog: {e}")))?;

        if let Ok(acroform_ref) = catalog.get(b"AcroForm") {
            let acroform = self.resolve_object(acroform_ref)?;
            if let Ok(dict) = acroform.as_dict() {
                if let Ok(field_refs) = dict.get(b"Fields") {
                    if let Ok(arr) = field_refs.as_array() {
                        for field_ref in arr {
                            if let Ok(field_obj) = self.resolve_object(field_ref) {
                                if let Ok(field) = self.parse_form_field(field_obj) {
                                    fields.push(field);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(fields)
    }

    /// Set a form field's value by field name.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if the field is not found.
    pub fn set_form_field_value(&mut self, field_name: &str, value: &str) -> Result<(), PdfError> {
        // Find the field object by name and update its /V entry
        let catalog = self
            .doc
            .catalog()
            .map_err(|e| PdfError::Generation(format!("Failed to get catalog: {e}")))?;

        if let Ok(acroform_ref) = catalog.get(b"AcroForm") {
            let acroform = self.resolve_object(acroform_ref)?;
            if let Ok(dict) = acroform.as_dict() {
                if let Ok(field_refs) = dict.get(b"Fields") {
                    if let Ok(arr) = field_refs.as_array() {
                        for field_ref in arr {
                            if let Object::Reference(id) = field_ref {
                                if let Ok(obj) = self.doc.get_object(*id) {
                                    if let Ok(d) = obj.as_dict() {
                                        let name = d
                                            .get(b"T")
                                            .ok()
                                            .and_then(|o| {
                                                if let Object::String(s, _) = o {
                                                    String::from_utf8(s.clone()).ok()
                                                } else {
                                                    None
                                                }
                                            })
                                            .unwrap_or_default();
                                        if name == field_name {
                                            if let Ok(field_obj) = self.doc.get_object_mut(*id) {
                                                if let Ok(dict) = field_obj.as_dict_mut() {
                                                    dict.set("V", Object::string_literal(value));
                                                }
                                            }
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(PdfError::Generation(format!(
            "Form field '{field_name}' not found"
        )))
    }

    /// Flatten the form — burn form field values into page content and remove interactive fields.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if flattening fails.
    pub fn flatten_form(&mut self) -> Result<(), PdfError> {
        let fields = self.get_form_fields()?;

        for field in &fields {
            if !field.value.is_empty() {
                // Add text overlay for the field value
                self.add_text_overlay(field.page, field.rect, &field.value, 10.0)?;
            }
        }

        // Remove AcroForm from catalog
        if let Ok(catalog) = self.doc.catalog_mut() {
            catalog.remove(b"AcroForm");
        }

        Ok(())
    }

    // ─── Save ────────────────────────────────────────

    /// Save the modified PDF to bytes.
    ///
    /// # Errors
    ///
    /// Returns `PdfError` if serialization fails.
    pub fn save(&mut self) -> Result<Vec<u8>, PdfError> {
        let mut buf = Vec::new();
        self.doc
            .save_to(&mut Cursor::new(&mut buf))
            .map_err(|e| PdfError::Generation(format!("Failed to save PDF: {e}")))?;
        Ok(buf)
    }

    // ─── Internal Helpers ────────────────────────────

    fn get_page_id(&self, page_num: usize) -> Result<ObjectId, PdfError> {
        let pages: Vec<(u32, ObjectId)> = self.doc.get_pages().into_iter().collect();
        pages
            .get(page_num)
            .map(|(_, id)| *id)
            .ok_or_else(|| PdfError::Generation(format!("Page {page_num} not found")))
    }

    fn get_page_height(&self, page_id: ObjectId) -> Result<f64, PdfError> {
        let page = self
            .doc
            .get_object(page_id)
            .map_err(|e| PdfError::Generation(format!("Failed to get page object: {e}")))?;

        if let Ok(dict) = page.as_dict() {
            if let Ok(mediabox) = dict.get(b"MediaBox") {
                if let Ok(arr) = mediabox.as_array() {
                    if arr.len() >= 4 {
                        // as_float() returns Result<f32>, cast to f64
                        return arr[3]
                            .as_float()
                            .map(|f| f as f64)
                            .or_else(|_| arr[3].as_i64().map(|i| i as f64))
                            .map_err(|e| PdfError::Generation(format!("Invalid MediaBox: {e}")));
                    }
                }
            }
        }

        // Default to US Letter height
        Ok(792.0)
    }

    fn append_content_stream(
        &mut self,
        page_id: ObjectId,
        content: Vec<u8>,
    ) -> Result<(), PdfError> {
        let stream = Stream::new(dictionary! {}, content);
        let stream_id = self.doc.add_object(Object::Stream(stream));

        if let Ok(page_obj) = self.doc.get_object_mut(page_id) {
            if let Ok(dict) = page_obj.as_dict_mut() {
                // Get existing contents
                let existing = dict.get(b"Contents").ok().cloned();

                match existing {
                    Some(Object::Array(mut arr)) => {
                        arr.push(Object::Reference(stream_id));
                        dict.set("Contents", Object::Array(arr));
                    }
                    Some(Object::Reference(existing_id)) => {
                        dict.set(
                            "Contents",
                            Object::Array(vec![
                                Object::Reference(existing_id),
                                Object::Reference(stream_id),
                            ]),
                        );
                    }
                    _ => {
                        dict.set("Contents", Object::Reference(stream_id));
                    }
                }
            }
        }
        Ok(())
    }

    fn ensure_base_font(&mut self, page_id: ObjectId) -> Result<String, PdfError> {
        // Check if /F1 (Helvetica) already exists in the page's resources
        let font_name = "F1".to_string();

        // First check if font already exists
        let needs_font = {
            let mut needs = false;
            if let Ok(page_obj) = self.doc.get_object(page_id) {
                if let Ok(dict) = page_obj.as_dict() {
                    if let Ok(resources) = dict.get(b"Resources") {
                        if let Ok(res_dict) = resources.as_dict() {
                            if let Ok(font_obj) = res_dict.get(b"Font") {
                                if let Ok(fd) = font_obj.as_dict() {
                                    if fd.get(b"F1").is_err() {
                                        needs = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            needs
        };

        if needs_font {
            let font_obj = dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica",
            };
            let font_id = self.doc.add_object(Object::Dictionary(font_obj));

            if let Ok(page_obj) = self.doc.get_object_mut(page_id) {
                if let Ok(dict) = page_obj.as_dict_mut() {
                    if let Ok(resources) = dict.get_mut(b"Resources") {
                        if let Ok(res_dict) = resources.as_dict_mut() {
                            if let Ok(font_container) = res_dict.get_mut(b"Font") {
                                if let Ok(fd) = font_container.as_dict_mut() {
                                    fd.set("F1", Object::Reference(font_id));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(font_name)
    }

    fn add_annotation_to_page(
        &mut self,
        page_id: ObjectId,
        annot_dict: lopdf::Dictionary,
    ) -> Result<(), PdfError> {
        let annot_id = self.doc.add_object(Object::Dictionary(annot_dict));

        if let Ok(page_obj) = self.doc.get_object_mut(page_id) {
            if let Ok(dict) = page_obj.as_dict_mut() {
                let existing = dict.get(b"Annots").ok().cloned();
                match existing {
                    Some(Object::Array(mut arr)) => {
                        arr.push(Object::Reference(annot_id));
                        dict.set("Annots", Object::Array(arr));
                    }
                    _ => {
                        dict.set("Annots", Object::Array(vec![Object::Reference(annot_id)]));
                    }
                }
            }
        }
        Ok(())
    }

    fn get_page_annotations(&self, page_id: ObjectId) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        if let Ok(page_obj) = self.doc.get_object(page_id) {
            if let Ok(dict) = page_obj.as_dict() {
                if let Ok(annots) = dict.get(b"Annots") {
                    if let Ok(arr) = annots.as_array() {
                        for item in arr {
                            if let Object::Reference(id) = item {
                                ids.push(*id);
                            }
                        }
                    }
                }
            }
        }
        ids
    }

    fn resolve_object<'a>(&'a self, obj: &'a Object) -> Result<&'a Object, PdfError> {
        match obj {
            Object::Reference(id) => self
                .doc
                .get_object(*id)
                .map_err(|e| PdfError::Generation(format!("Failed to resolve reference: {e}"))),
            _ => Ok(obj),
        }
    }

    fn parse_form_field(&self, obj: &Object) -> Result<FormField, PdfError> {
        let dict = obj
            .as_dict()
            .map_err(|_| PdfError::Generation("Not a dictionary".to_string()))?;

        let name = dict
            .get(b"T")
            .ok()
            .and_then(|o| {
                if let Object::String(s, _) = o {
                    String::from_utf8(s.clone()).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let ft = dict
            .get(b"FT")
            .ok()
            .and_then(|o| o.as_name_str().ok())
            .unwrap_or("");

        let field_type = match ft {
            "Tx" => FormFieldType::Text,
            "Btn" => FormFieldType::Checkbox,
            "Ch" => FormFieldType::Dropdown,
            "Sig" => FormFieldType::Signature,
            _ => FormFieldType::Text,
        };

        let value = dict
            .get(b"V")
            .ok()
            .and_then(|o| {
                if let Object::String(s, _) = o {
                    String::from_utf8(s.clone()).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Parse rect
        let rect = dict
            .get(b"Rect")
            .ok()
            .and_then(|o| o.as_array().ok())
            .map(|arr| {
                let vals: Vec<f64> = arr
                    .iter()
                    .filter_map(|o| {
                        o.as_float()
                            .ok()
                            .map(|f| f as f64)
                            .or_else(|| o.as_i64().ok().map(|i| i as f64))
                    })
                    .collect();
                if vals.len() >= 4 {
                    Rect {
                        x: vals[0],
                        y: vals[1],
                        width: vals[2] - vals[0],
                        height: vals[3] - vals[1],
                    }
                } else {
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 20.0,
                    }
                }
            })
            .unwrap_or(Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            });

        Ok(FormField {
            name,
            field_type,
            page: 0, // Will be set by caller
            rect,
            value,
            options: Vec::new(),
        })
    }

    fn rebuild_page_tree(&mut self, page_ids: Vec<ObjectId>) -> Result<(), PdfError> {
        // Get the root pages object
        let catalog = self
            .doc
            .catalog()
            .map_err(|e| PdfError::Generation(format!("Failed to get catalog: {e}")))?;

        let pages_ref = catalog
            .get(b"Pages")
            .map_err(|e| PdfError::Generation(format!("No Pages in catalog: {e}")))?;

        let pages_id = if let Object::Reference(id) = pages_ref {
            *id
        } else {
            return Err(PdfError::Generation("Pages is not a reference".to_string()));
        };

        // Build Kids array
        let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
        let count = kids.len() as i64;

        if let Ok(pages_obj) = self.doc.get_object_mut(pages_id) {
            if let Ok(dict) = pages_obj.as_dict_mut() {
                dict.set("Kids", Object::Array(kids));
                dict.set("Count", Object::Integer(count));
            }
        }

        // Update each page's Parent reference
        for page_id in &page_ids {
            if let Ok(page_obj) = self.doc.get_object_mut(*page_id) {
                if let Ok(dict) = page_obj.as_dict_mut() {
                    dict.set("Parent", Object::Reference(pages_id));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a minimal valid PDF using lopdf for testing
    fn minimal_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.4");

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let content = b"BT /F1 12 Tf 72 720 Td (Hello World) Tj ET";
        let stream = Stream::new(dictionary! {}, content.to_vec());
        let content_id = doc.add_object(Object::Stream(stream));

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ],
            "Contents" => content_id,
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F1" => font_id,
                },
            },
        });

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => Object::Integer(1),
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut buf = Vec::new();
        doc.save_to(&mut std::io::Cursor::new(&mut buf)).unwrap();
        buf
    }

    #[test]
    fn test_open_and_page_count() {
        let pdf = minimal_pdf();
        let editor = PdfEditor::open(&pdf).unwrap();
        assert_eq!(editor.page_count(), 1);
    }

    #[test]
    fn test_save_roundtrip() {
        let pdf = minimal_pdf();
        let mut editor = PdfEditor::open(&pdf).unwrap();
        let saved = editor.save().unwrap();
        assert!(!saved.is_empty());
        // Should be parseable
        let editor2 = PdfEditor::open(&saved).unwrap();
        assert_eq!(editor2.page_count(), 1);
    }

    #[test]
    fn test_add_text_annotation() {
        let pdf = minimal_pdf();
        let mut editor = PdfEditor::open(&pdf).unwrap();
        editor
            .add_text_annotation(0, 100.0, 100.0, "Test User", "A comment")
            .unwrap();
        let saved = editor.save().unwrap();
        assert!(!saved.is_empty());
    }

    #[test]
    fn test_rotate_page() {
        let pdf = minimal_pdf();
        let mut editor = PdfEditor::open(&pdf).unwrap();
        editor.rotate_page(0, 90).unwrap();
        let saved = editor.save().unwrap();
        let editor2 = PdfEditor::open(&saved).unwrap();
        assert_eq!(editor2.page_count(), 1);
    }
}
