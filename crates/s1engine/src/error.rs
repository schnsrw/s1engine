//! Unified error type for s1engine.

use std::fmt;

/// Top-level error type for s1engine operations.
#[derive(Debug)]
pub enum Error {
    /// Error from a format reader/writer (DOCX, TXT, etc.).
    Format(String),
    /// Error from an operation (insert, delete, etc.).
    Operation(s1_ops::OperationError),
    /// I/O error (file read/write).
    Io(std::io::Error),
    /// The requested format is not supported or not enabled via feature flags.
    UnsupportedFormat(String),
    /// Error from the CRDT collaboration subsystem.
    #[cfg(feature = "crdt")]
    Crdt(s1_crdt::CrdtError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(msg) => write!(f, "Format error: {msg}"),
            Self::Operation(e) => write!(f, "Operation error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::UnsupportedFormat(fmt_name) => {
                write!(f, "Unsupported format: {fmt_name}")
            }
            #[cfg(feature = "crdt")]
            Self::Crdt(e) => write!(f, "CRDT error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(e) => Some(e),
            Self::Io(e) => Some(e),
            #[cfg(feature = "crdt")]
            Self::Crdt(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<s1_ops::OperationError> for Error {
    fn from(e: s1_ops::OperationError) -> Self {
        Self::Operation(e)
    }
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

#[cfg(feature = "crdt")]
impl From<s1_crdt::CrdtError> for Error {
    fn from(e: s1_crdt::CrdtError) -> Self {
        Self::Crdt(e)
    }
}
