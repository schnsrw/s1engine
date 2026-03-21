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
    /// Comma-separated values (`.csv`) — import/export via s1-convert
    Csv,
    /// Microsoft Excel XLSX — recognized but not yet editable
    Xlsx,
    /// Microsoft PowerPoint PPTX — recognized but not yet editable
    Pptx,
    /// OpenDocument Spreadsheet (`.ods`) — recognized but not yet editable
    Ods,
    /// OpenDocument Presentation (`.odp`) — recognized but not yet editable
    Odp,
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
            "csv" => Ok(Self::Csv),
            "xlsx" => Ok(Self::Xlsx),
            "pptx" => Ok(Self::Pptx),
            "ods" => Ok(Self::Ods),
            "odp" => Ok(Self::Odp),
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
    /// - ZIP signature (`PK\x03\x04`) → inspects contents for DOCX/XLSX/PPTX/ODT/ODS/ODP
    /// - `%PDF` → PDF
    /// - OLE2 compound → DOC
    /// - Otherwise → TXT
    pub fn detect(data: &[u8]) -> Self {
        if data.len() >= 8 && data[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
            Self::Doc
        } else if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
            // ZIP-based — inspect for specific Office format
            detect_zip_format(data)
        } else if data.len() >= 4 && &data[0..4] == b"%PDF" {
            Self::Pdf
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
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
            Self::Pptx => "pptx",
            Self::Ods => "ods",
            Self::Odp => "odp",
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
            Self::Csv => "text/csv",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Ods => "application/vnd.oasis.opendocument.spreadsheet",
            Self::Odp => "application/vnd.oasis.opendocument.presentation",
        }
    }

    /// Whether this format is a document (word-processing) format.
    pub fn is_document(&self) -> bool {
        matches!(
            self,
            Self::Docx | Self::Odt | Self::Doc | Self::Txt | Self::Md
        )
    }

    /// Whether this format is a spreadsheet format.
    pub fn is_spreadsheet(&self) -> bool {
        matches!(self, Self::Xlsx | Self::Ods | Self::Csv)
    }

    /// Whether this format is a presentation format.
    pub fn is_presentation(&self) -> bool {
        matches!(self, Self::Pptx | Self::Odp)
    }

    /// Whether this format is currently supported for reading/writing by s1engine.
    pub fn is_editable(&self) -> bool {
        matches!(
            self,
            Self::Docx | Self::Odt | Self::Doc | Self::Txt | Self::Md | Self::Csv
        )
    }
}

/// Inspect ZIP archive bytes to determine the specific Office format.
fn detect_zip_format(data: &[u8]) -> Format {
    let s = String::from_utf8_lossy(data);

    // OOXML markers
    if s.contains("word/") || s.contains("wordprocessingml") {
        return Format::Docx;
    }
    if s.contains("xl/") || s.contains("spreadsheetml") {
        return Format::Xlsx;
    }
    if s.contains("ppt/") || s.contains("presentationml") {
        return Format::Pptx;
    }

    // ODF markers
    if s.contains("opendocument.text") {
        return Format::Odt;
    }
    if s.contains("opendocument.spreadsheet") {
        return Format::Ods;
    }
    if s.contains("opendocument.presentation") {
        return Format::Odp;
    }
    if s.contains("content.xml") || s.contains("opendocument") {
        return Format::Odt;
    }

    // Default ZIP to DOCX (most common)
    Format::Docx
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

    // ─── New format detection tests ─────────────────────────────

    #[test]
    fn detect_xlsx_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("xlsx")).unwrap(),
            Format::Xlsx
        );
        assert_eq!(Format::Xlsx.extension(), "xlsx");
        assert!(Format::Xlsx.mime_type().contains("spreadsheetml"));
    }

    #[test]
    fn detect_pptx_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("pptx")).unwrap(),
            Format::Pptx
        );
        assert_eq!(Format::Pptx.extension(), "pptx");
        assert!(Format::Pptx.mime_type().contains("presentationml"));
    }

    #[test]
    fn detect_ods_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("ods")).unwrap(),
            Format::Ods
        );
        assert_eq!(Format::Ods.extension(), "ods");
    }

    #[test]
    fn detect_odp_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("odp")).unwrap(),
            Format::Odp
        );
        assert_eq!(Format::Odp.extension(), "odp");
    }

    #[test]
    fn detect_csv_extension() {
        assert_eq!(
            Format::from_extension(OsStr::new("csv")).unwrap(),
            Format::Csv
        );
        assert_eq!(Format::Csv.extension(), "csv");
        assert_eq!(Format::Csv.mime_type(), "text/csv");
    }

    #[test]
    fn detect_xlsx_from_bytes() {
        let mut data = b"PK\x03\x04".to_vec();
        data.extend_from_slice(b"xl/worksheets/sheet1.xml");
        assert_eq!(Format::detect(&data), Format::Xlsx);
    }

    #[test]
    fn detect_pptx_from_bytes() {
        let mut data = b"PK\x03\x04".to_vec();
        data.extend_from_slice(b"ppt/slides/slide1.xml");
        assert_eq!(Format::detect(&data), Format::Pptx);
    }

    #[test]
    fn format_category_helpers() {
        assert!(Format::Docx.is_document());
        assert!(!Format::Docx.is_spreadsheet());
        assert!(!Format::Docx.is_presentation());
        assert!(Format::Docx.is_editable());

        assert!(Format::Xlsx.is_spreadsheet());
        assert!(!Format::Xlsx.is_document());
        assert!(!Format::Xlsx.is_editable());

        assert!(Format::Pptx.is_presentation());
        assert!(!Format::Pptx.is_document());
        assert!(!Format::Pptx.is_editable());

        assert!(Format::Csv.is_spreadsheet());
        assert!(Format::Csv.is_editable());
    }
}
