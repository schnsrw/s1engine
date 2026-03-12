//! Error types for the ODT format crate.

/// Error type for ODT format operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OdtError {
    /// Error reading the ZIP archive.
    #[error("ODT ZIP error: {0}")]
    Zip(String),
    /// Error parsing XML content.
    #[error("ODT XML error: {0}")]
    Xml(String),
    /// A required file is missing from the ODT archive.
    #[error("Missing file in ODT: {0}")]
    MissingFile(String),
    /// The document structure is invalid.
    #[error("Invalid ODT structure: {0}")]
    InvalidStructure(String),
}

impl From<zip::result::ZipError> for OdtError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e.to_string())
    }
}

impl From<quick_xml::Error> for OdtError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e.to_string())
    }
}

impl From<std::io::Error> for OdtError {
    fn from(e: std::io::Error) -> Self {
        Self::Zip(e.to_string())
    }
}
