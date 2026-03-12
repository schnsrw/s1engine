//! Format conversion pipelines for s1engine.
//!
//! Provides a unified API for converting between document formats.
//! Conversion works through the document model:
//!
//! ```text
//! Source Format → DocumentModel → Target Format
//! ```
//!
//! Supported conversions:
//! - DOC → DOCX/ODT (basic text extraction only)
//! - DOCX ↔ ODT (full model round-trip)

pub mod convert;
pub mod doc_reader;
pub mod error;

pub use convert::{convert, convert_to_model, detect_format, SourceFormat, TargetFormat};
pub use error::ConvertError;
