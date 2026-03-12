//! Error types for PDF export.

use thiserror::Error;

/// Errors that can occur during PDF export.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PdfError {
    /// Layout failed.
    #[error("layout error: {0}")]
    Layout(#[from] s1_layout::LayoutError),

    /// Font embedding failed.
    #[error("font error: {0}")]
    Font(String),

    /// PDF generation failed.
    #[error("PDF generation error: {0}")]
    Generation(String),
}
