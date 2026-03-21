//! Error types for XLSX operations.

#[derive(Debug, thiserror::Error)]
pub enum XlsxError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML error: {0}")]
    Xml(String),

    #[error("Invalid cell reference: {0}")]
    InvalidCellRef(String),

    #[error("Sheet not found: {0}")]
    SheetNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Write error: {0}")]
    Write(String),
}
