//! Error types for s1-text.

use thiserror::Error;

/// Errors that can occur during text processing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TextError {
    /// Failed to parse a font file.
    #[error("failed to parse font: {0}")]
    FontParse(String),

    /// Font not found in the database.
    #[error("font not found: {0}")]
    FontNotFound(String),

    /// Text shaping failed.
    #[error("shaping failed: {0}")]
    ShapingFailed(String),

    /// Invalid font data.
    #[error("invalid font data: {0}")]
    InvalidFontData(String),
}
