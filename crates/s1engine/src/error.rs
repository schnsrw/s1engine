//! Unified error type for s1engine.

/// Top-level error type for s1engine operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error from a format reader/writer (DOCX, TXT, etc.).
    #[error("Format error: {0}")]
    Format(String),
    /// Error from an operation (insert, delete, etc.).
    #[error("Operation error: {0}")]
    Operation(#[from] s1_ops::OperationError),
    /// I/O error (file read/write).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The requested format is not supported or not enabled via feature flags.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    /// Error from the CRDT collaboration subsystem.
    #[cfg(feature = "crdt")]
    #[error("CRDT error: {0}")]
    Crdt(#[from] s1_crdt::CrdtError),
}

#[cfg(feature = "docx")]
impl From<s1_format_docx::DocxError> for Error {
    fn from(e: s1_format_docx::DocxError) -> Self {
        Self::Format(e.to_string())
    }
}

#[cfg(feature = "odt")]
impl From<s1_format_odt::OdtError> for Error {
    fn from(e: s1_format_odt::OdtError) -> Self {
        Self::Format(e.to_string())
    }
}

#[cfg(feature = "txt")]
impl From<s1_format_txt::TxtError> for Error {
    fn from(e: s1_format_txt::TxtError) -> Self {
        Self::Format(e.to_string())
    }
}
