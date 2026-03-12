//! Error types for s1-layout.

use thiserror::Error;

/// Errors that can occur during layout.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LayoutError {
    /// Text shaping failed.
    #[error("text shaping error: {0}")]
    Shaping(#[from] s1_text::TextError),

    /// Invalid document structure.
    #[error("invalid document structure: {0}")]
    InvalidStructure(String),

    /// No fonts available for text.
    #[error("no font available for text: {0}")]
    NoFont(String),
}
