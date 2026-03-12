//! Engine — factory for creating and opening documents.

use std::path::Path;

use crate::document::Document;
use crate::error::Error;
use crate::format::Format;

/// The main entry point for s1engine.
///
/// `Engine` is a lightweight factory for creating and opening documents.
/// It holds no state and can be shared across threads.
///
/// # Example
///
/// ```no_run
/// use s1engine::Engine;
///
/// let engine = Engine::new();
/// let doc = engine.create();
/// ```
pub struct Engine;

impl Engine {
    /// Create a new engine instance.
    pub fn new() -> Self {
        Self
    }

    /// Create a new empty document.
    pub fn create(&self) -> Document {
        Document::new()
    }

    /// Open a document from raw bytes.
    ///
    /// The format is auto-detected from the content.
    pub fn open(&self, data: &[u8]) -> Result<Document, Error> {
        let format = Format::detect(data);
        self.open_as(data, format)
    }

    /// Open a document from raw bytes with an explicit format.
    pub fn open_as(&self, data: &[u8], format: Format) -> Result<Document, Error> {
        let model = match format {
            #[cfg(feature = "docx")]
            Format::Docx => s1_format_docx::read(data)?,
            #[cfg(feature = "odt")]
            Format::Odt => s1_format_odt::read(data)?,
            #[cfg(feature = "txt")]
            Format::Txt => {
                let result = s1_format_txt::read(data)?;
                result.document
            }
            #[cfg(feature = "convert")]
            Format::Doc => {
                s1_convert::doc_reader::read_doc(data)
                    .map_err(|e| Error::Format(e.to_string()))?
            }
            #[allow(unreachable_patterns)]
            _ => {
                return Err(Error::UnsupportedFormat(format!(
                    "{:?} reading not available (check feature flags)",
                    format
                )));
            }
        };
        Ok(Document::from_model(model))
    }

    /// Open a document from a file path.
    ///
    /// Format is detected from the file extension.
    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<Document, Error> {
        let path = path.as_ref();
        let format = Format::from_path(path)?;
        let data = std::fs::read(path)?;
        self.open_as(&data, format)
    }

    /// Create a new empty collaborative document.
    ///
    /// Each collaborating user should have a unique `replica_id`.
    #[cfg(feature = "crdt")]
    pub fn create_collab(&self, replica_id: u64) -> s1_crdt::CollabDocument {
        s1_crdt::CollabDocument::new(replica_id)
    }

    /// Open a document as a collaborative document from raw bytes.
    #[cfg(feature = "crdt")]
    pub fn open_collab(
        &self,
        data: &[u8],
        replica_id: u64,
    ) -> Result<s1_crdt::CollabDocument, Error> {
        let doc = self.open(data)?;
        Ok(s1_crdt::CollabDocument::from_model(
            doc.into_model(),
            replica_id,
        ))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
