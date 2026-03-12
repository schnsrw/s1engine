//! Markdown reader/writer for s1engine.
//!
//! Reads Markdown (CommonMark + GFM extensions) into a [`DocumentModel`] and
//! writes a [`DocumentModel`] back to Markdown text.

mod reader;
mod writer;

pub use reader::read;
pub use writer::write;

use s1_model::DocumentModel;

/// Errors produced by the Markdown format crate.
#[derive(Debug, thiserror::Error)]
pub enum MdError {
    /// A model insertion error.
    #[error("model error: {0}")]
    Model(String),
}

/// Read Markdown bytes into a [`DocumentModel`].
///
/// The input is interpreted as UTF-8.
///
/// # Errors
///
/// Returns `MdError` if the document model cannot be constructed.
pub fn read_bytes(input: &[u8]) -> Result<DocumentModel, MdError> {
    let text = String::from_utf8_lossy(input);
    read(&text)
}

/// Write a [`DocumentModel`] to Markdown bytes (UTF-8).
pub fn write_bytes(doc: &DocumentModel) -> Vec<u8> {
    write(doc).into_bytes()
}

/// Write a [`DocumentModel`] to a Markdown string.
pub fn write_string(doc: &DocumentModel) -> String {
    write(doc)
}
