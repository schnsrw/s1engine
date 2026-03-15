//! PDF export and editing for s1engine.
//!
//! ## Export Pipeline
//!
//! ```text
//! DocumentModel → s1-layout (LayoutEngine) → LayoutDocument → s1-format-pdf → PDF bytes
//! ```
//!
//! Uses `pdf-writer` for low-level PDF generation and `subsetter` for font
//! subsetting (embed only used glyphs for reasonable file sizes).
//!
//! ## PDF Editing (requires `pdf-editing` feature)
//!
//! With the `pdf-editing` feature enabled, provides `PdfEditor` for reading
//! and modifying existing PDFs: text overlay, annotations, page manipulation,
//! form filling, and more. Uses `lopdf` for PDF structure manipulation.

pub mod error;
pub mod writer;

#[cfg(feature = "pdf-editing")]
pub mod editor;

pub use error::PdfError;
pub use writer::{write_pdf, write_pdf_a, PdfAConformance};

#[cfg(feature = "pdf-editing")]
pub use editor::{FormField, FormFieldType, PdfEditor, Rect};
