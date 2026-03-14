//! ODT (ODF) reader/writer for s1engine.
//!
//! This crate provides functions to read ODT files into a `DocumentModel`
//! and write a `DocumentModel` back to ODT format.

pub mod error;

mod content_parser;
mod content_writer;
mod manifest_writer;
mod metadata_parser;
mod metadata_writer;
mod property_parser;
mod property_writer;
mod settings_parser;
mod settings_writer;
mod style_parser;
mod style_writer;
mod xml_util;

pub mod reader;
pub mod writer;

pub use error::OdtError;
pub use reader::read;
pub use writer::write;
