//! Error types for format conversion.

use thiserror::Error;

/// Errors that can occur during format conversion.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConvertError {
    /// The input format is not supported for conversion.
    #[error("unsupported conversion: {from} → {to}")]
    UnsupportedConversion {
        /// Source format name.
        from: String,
        /// Target format name.
        to: String,
    },

    /// Failed to read the input file.
    #[error("failed to read input: {0}")]
    ReadError(String),

    /// Failed to parse OLE2/CFB container.
    #[error("invalid DOC file: {0}")]
    InvalidDoc(String),

    /// The DOC file uses features that cannot be converted.
    #[error("DOC conversion incomplete: {0}")]
    PartialConversion(String),

    /// DOCX format error (preserves the original error chain for inspection).
    #[error("DOCX error: {0}")]
    Docx(#[from] s1_format_docx::DocxError),

    /// ODT format error (preserves the original error chain for inspection).
    #[error("ODT error: {0}")]
    Odt(#[from] s1_format_odt::OdtError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
