//! DOCX (OOXML) reader/writer for s1engine.
//!
//! Reads `.docx` files (Office Open XML) into the s1engine document model
//! and writes them back. DOCX files are ZIP archives containing XML.
//!
//! # Phase 1 (current)
//! - Paragraphs, runs, text content
//! - Run formatting: bold, italic, underline, strikethrough, font, size, color
//! - Paragraph formatting: alignment, spacing, indentation
//! - Style definitions and references
//! - Document metadata (title, author, etc.)
//!
//! # Phase 2 (planned)
//! - Tables, images, lists, headers/footers, hyperlinks, bookmarks, comments

pub mod comments_parser;
pub mod comments_writer;
pub mod content_parser;
pub mod content_writer;
pub mod endnotes_parser;
pub mod endnotes_writer;
pub mod error;
pub mod footnotes_parser;
pub mod footnotes_writer;
pub mod header_footer_parser;
pub mod header_footer_writer;
pub mod metadata_parser;
pub mod metadata_writer;
pub mod numbering_parser;
pub mod numbering_writer;
pub mod property_parser;
pub mod reader;
pub mod section_parser;
pub mod section_writer;
pub mod streaming;
pub mod style_parser;
pub mod style_writer;
pub mod writer;
pub mod xml_util;
pub mod xml_writer;

// Re-export primary types at crate root.
pub use error::DocxError;
pub use reader::read;
pub use writer::write;
