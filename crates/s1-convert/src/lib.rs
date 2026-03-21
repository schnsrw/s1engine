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

pub mod chpx;
pub mod convert;
pub mod csv_parser;
pub mod doc_reader;
pub mod error;
pub mod fib;
pub mod font_table;
pub mod papx;
pub mod piece_table;
pub mod sprm;
pub mod stylesheet;
pub mod summary_info;

pub use convert::{
    convert, convert_to_model, convert_with_warnings, csv_to_docx, csv_to_model, detect_file_type,
    detect_format, docx_to_csv, is_supported, model_to_csv, validate_conversion, ConvertWarning,
    FileType, SourceFormat, TargetFormat,
};
pub use csv_parser::{
    detect_delimiter, parse_csv as parse_csv_raw, parse_csv_streaming,
    parse_csv_streaming_with_delimiter, parse_csv_with_delimiter, parse_csv_with_headers,
    parse_tsv, write_csv, write_csv_with_delimiter, CsvData, CsvError,
};
pub use error::ConvertError;
