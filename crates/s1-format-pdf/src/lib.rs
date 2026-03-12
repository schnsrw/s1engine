//! PDF export for s1engine.
//!
//! Generates PDF files from a laid-out document. The pipeline is:
//!
//! ```text
//! DocumentModel → s1-layout (LayoutEngine) → LayoutDocument → s1-format-pdf → PDF bytes
//! ```
//!
//! Uses `pdf-writer` for low-level PDF generation and `subsetter` for font
//! subsetting (embed only used glyphs for reasonable file sizes).

pub mod error;
pub mod writer;

pub use error::PdfError;
pub use writer::write_pdf;
