//! Supported document formats.

use std::ffi::OsStr;
use std::path::Path;

use crate::Error;

/// Supported document formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Office Open XML (`.docx`)
    Docx,
    /// Open Document Format (`.odt`)
    Odt,
    /// Portable Document Format (`.pdf`) — export only
    Pdf,
    /// Plain text (`.txt`)
    Txt,
    /// Legacy Microsoft Word binary (`.doc`) -- read only, requires `doc-legacy` feature
    Doc,
    /// Markdown (`.md`, `.markdown`)
    Md,
}

impl Format {
    /// Detect format from a file extension.
    ///
    /// Returns an error if the extension is not recognized.
    pub fn from_extension(ext: &OsStr) -> Result<Self, Error> {
        let ext = ext.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "docx" => Ok(Self::Docx),
            "odt" => Ok(Self::Odt),
            "pdf" => Ok(Self::Pdf),
            "txt" | "text" => Ok(Self::Txt),
            "doc" => Ok(Self::Doc),
            "md" | "markdown" => Ok(Self::Md),
            _ => Err(Error::UnsupportedFormat(format!(
                "Unknown file extension: .{ext}"
            ))),
        }
    }

    /// Detect format from a file path.
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        path.extension()
            .ok_or_else(|| Error::UnsupportedFormat("No file extension".to_string()))
            .and_then(Self::from_extension)
    }

    /// Try to detect format from the first bytes of content.
    ///
    /// This checks magic bytes:
    /// - ZIP signature (`PK\x03\x04`) → DOCX (could also be ODT, detected later)
    /// - `%PDF` → PDF
    /// - Otherwise → TXT
    pub fn detect(data: &[u8]) -> Self {
        if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
            // ZIP-based — could be DOCX or ODT.
            // Check for OOXML content type marker.
            let s = String::from_utf8_lossy(data);
            if s.contains("word/") || s.contains("openxmlformats") {
                Self::Docx
            } else if s.contains("content.xml") || s.contains("opendocument") {
                Self::Odt
            } else {
                // Default ZIP to DOCX
                Self::Docx
            }
        } else if data.len() >= 4 && &data[0..4] == b"%PDF" {
            Self::Pdf
        } else if data.len() >= 8 && data[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
        {
            Self::Doc
        } else {
            Self::Txt
        }
    }

    /// File extension for this format (without dot).
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Odt => "odt",
            Self::Pdf => "pdf",
            Self::Txt => "txt",
            Self::Doc => "doc",
            Self::Md => "md",
        }
    }

    /// MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Odt => "application/vnd.oasis.opendocument.text",
            Self::Pdf => "application/pdf",
            Self::Txt => "text/plain",
            Self::Doc => "application/msword",
            Self::Md => "text/markdown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_from_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("docx")).unwrap(),
            Format::Docx
        );
        assert_eq!(
            Format::from_extension(OsStr::new("txt")).unwrap(),
            Format::Txt
        );
        assert_eq!(
            Format::from_extension(OsStr::new("odt")).unwrap(),
            Format::Odt
        );
        assert_eq!(
            Format::from_extension(OsStr::new("pdf")).unwrap(),
            Format::Pdf
        );
    }

    #[test]
    fn detect_from_extension_case_insensitive() {
        assert_eq!(
            Format::from_extension(OsStr::new("DOCX")).unwrap(),
            Format::Docx
        );
        assert_eq!(
            Format::from_extension(OsStr::new("TXT")).unwrap(),
            Format::Txt
        );
    }

    #[test]
    fn detect_unknown_extension() {
        assert!(Format::from_extension(OsStr::new("xyz")).is_err());
    }

    #[test]
    fn detect_from_path() {
        let path = Path::new("report.docx");
        assert_eq!(Format::from_path(path).unwrap(), Format::Docx);
    }

    #[test]
    fn detect_from_bytes_zip() {
        // ZIP magic bytes
        let mut data = b"PK\x03\x04".to_vec();
        data.extend_from_slice(b"word/document.xml");
        assert_eq!(Format::detect(&data), Format::Docx);
    }

    #[test]
    fn detect_from_bytes_pdf() {
        assert_eq!(Format::detect(b"%PDF-1.4"), Format::Pdf);
    }

    #[test]
    fn detect_from_bytes_txt() {
        assert_eq!(Format::detect(b"Hello World"), Format::Txt);
    }

    #[test]
    fn format_extension() {
        assert_eq!(Format::Docx.extension(), "docx");
        assert_eq!(Format::Txt.extension(), "txt");
    }

    #[test]
    fn format_mime_type() {
        assert!(Format::Docx.mime_type().contains("wordprocessingml"));
        assert_eq!(Format::Txt.mime_type(), "text/plain");
        assert_eq!(Format::Doc.mime_type(), "application/msword");
    }

    #[test]
    fn detect_from_bytes_doc() {
        let ole2 = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert_eq!(Format::detect(&ole2), Format::Doc);
    }

    #[test]
    fn detect_doc_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("doc")).unwrap(),
            Format::Doc
        );
        assert_eq!(Format::Doc.extension(), "doc");
    }
}
