//! Error types for the DOCX format crate.

/// Error type for DOCX format operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocxError {
    /// Error reading the ZIP archive.
    #[error("DOCX ZIP error: {0}")]
    Zip(String),
    /// Error parsing XML content.
    #[error("DOCX XML error: {0}")]
    Xml(String),
    /// A required file is missing from the DOCX archive.
    #[error("Missing file in DOCX: {0}")]
    MissingFile(String),
    /// The document structure is invalid.
    #[error("Invalid DOCX structure: {0}")]
    InvalidStructure(String),
}

impl From<zip::result::ZipError> for DocxError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e.to_string())
    }
}

impl From<quick_xml::Error> for DocxError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e.to_string())
    }
}

impl From<std::io::Error> for DocxError {
    fn from(e: std::io::Error) -> Self {
        Self::Zip(e.to_string())
    }
}
