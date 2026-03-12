//! Format conversion pipeline.
//!
//! Provides a unified API for converting between document formats.
//! Conversion works through the document model:
//!
//! ```text
//! Source Format → DocumentModel → Target Format
//! ```

use s1_model::DocumentModel;

use crate::doc_reader;
use crate::error::ConvertError;

/// Supported source formats for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceFormat {
    /// Legacy Microsoft Word binary format (.doc).
    Doc,
    /// Office Open XML (.docx).
    Docx,
    /// Open Document Format (.odt).
    Odt,
}

/// Supported target formats for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TargetFormat {
    /// Office Open XML (.docx).
    Docx,
    /// Open Document Format (.odt).
    Odt,
}

/// Convert document data from one format to another.
///
/// Returns the converted document as bytes in the target format.
///
/// # Supported conversions
///
/// | From | To | Notes |
/// |------|-----|-------|
/// | DOC  | DOCX | Basic text extraction only (no formatting) |
/// | DOC  | ODT  | Basic text extraction only |
/// | DOCX | ODT  | Full model round-trip |
/// | ODT  | DOCX | Full model round-trip |
///
/// # Errors
///
/// Returns `ConvertError` if the conversion is not supported or fails.
pub fn convert(data: &[u8], from: SourceFormat, to: TargetFormat) -> Result<Vec<u8>, ConvertError> {
    // Step 1: Read source into DocumentModel
    let doc = read_source(data, from)?;

    // Step 2: Write to target format
    write_target(&doc, to)
}

/// Convert document data from one format to a DocumentModel.
///
/// Useful when consumers want the model rather than re-encoded bytes.
pub fn convert_to_model(data: &[u8], from: SourceFormat) -> Result<DocumentModel, ConvertError> {
    read_source(data, from)
}

/// Detect the source format from file bytes.
///
/// Returns `None` if the format cannot be detected.
pub fn detect_format(data: &[u8]) -> Option<SourceFormat> {
    if doc_reader::is_doc_file(data) {
        Some(SourceFormat::Doc)
    } else if data.len() >= 4 && &data[..4] == b"PK\x03\x04" {
        // ZIP-based — could be DOCX or ODT
        // Check for DOCX content types
        if let Ok(text) = std::str::from_utf8(data) {
            if text.contains("word/") {
                return Some(SourceFormat::Docx);
            }
        }
        // Default to DOCX for ZIP files (more common)
        // In practice, consumers should specify the format explicitly
        Some(SourceFormat::Docx)
    } else {
        None
    }
}

fn read_source(data: &[u8], from: SourceFormat) -> Result<DocumentModel, ConvertError> {
    match from {
        SourceFormat::Doc => doc_reader::read_doc(data),
        SourceFormat::Docx => {
            s1_format_docx::read(data).map_err(|e| ConvertError::Docx(format!("{e}")))
        }
        SourceFormat::Odt => {
            s1_format_odt::read(data).map_err(|e| ConvertError::Odt(format!("{e}")))
        }
    }
}

fn write_target(doc: &DocumentModel, to: TargetFormat) -> Result<Vec<u8>, ConvertError> {
    match to {
        TargetFormat::Docx => {
            s1_format_docx::write(doc).map_err(|e| ConvertError::Docx(format!("{e}")))
        }
        TargetFormat::Odt => {
            s1_format_odt::write(doc).map_err(|e| ConvertError::Odt(format!("{e}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_doc_format() {
        let magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert_eq!(detect_format(&magic), Some(SourceFormat::Doc));
    }

    #[test]
    fn detect_zip_format() {
        let zip_magic = [0x50, 0x4B, 0x03, 0x04, 0, 0, 0, 0];
        let detected = detect_format(&zip_magic);
        assert!(detected.is_some());
    }

    #[test]
    fn detect_unknown_format() {
        assert_eq!(detect_format(b"random data"), None);
    }

    #[test]
    fn convert_docx_to_odt() {
        // Build a minimal DOCX, convert to ODT
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, s1_model::Node::new(body_id, s1_model::NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(
            body_id,
            0,
            s1_model::Node::new(para_id, s1_model::NodeType::Paragraph),
        )
        .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(
            para_id,
            0,
            s1_model::Node::new(run_id, s1_model::NodeType::Run),
        )
        .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, s1_model::Node::text(text_id, "Convert me"))
            .unwrap();

        let docx_bytes =
            s1_format_docx::write(&doc).unwrap();

        // Convert DOCX → ODT
        let odt_bytes = convert(&docx_bytes, SourceFormat::Docx, TargetFormat::Odt).unwrap();
        assert!(!odt_bytes.is_empty());

        // Verify ODT can be read back
        let model = s1_format_odt::read(&odt_bytes).unwrap();
        // Should have content
        let root = model.root_id();
        let root_node = model.node(root).unwrap();
        assert!(!root_node.children.is_empty());
    }

    #[test]
    fn convert_odt_to_docx() {
        // Build a minimal ODT, convert to DOCX
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, s1_model::Node::new(body_id, s1_model::NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(
            body_id,
            0,
            s1_model::Node::new(para_id, s1_model::NodeType::Paragraph),
        )
        .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(
            para_id,
            0,
            s1_model::Node::new(run_id, s1_model::NodeType::Run),
        )
        .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, s1_model::Node::text(text_id, "Cross format"))
            .unwrap();

        let odt_bytes = s1_format_odt::write(&doc).unwrap();

        // Convert ODT → DOCX
        let docx_bytes = convert(&odt_bytes, SourceFormat::Odt, TargetFormat::Docx).unwrap();
        assert!(!docx_bytes.is_empty());

        // Verify DOCX can be read back
        let model = s1_format_docx::read(&docx_bytes).unwrap();
        let root = model.root_id();
        let root_node = model.node(root).unwrap();
        assert!(!root_node.children.is_empty());
    }

    #[test]
    fn convert_invalid_doc() {
        let result = convert(b"not a doc", SourceFormat::Doc, TargetFormat::Docx);
        assert!(result.is_err());
    }

    #[test]
    fn convert_to_model_docx() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, s1_model::Node::new(body_id, s1_model::NodeType::Body))
            .unwrap();
        let docx_bytes = s1_format_docx::write(&doc).unwrap();

        let model = convert_to_model(&docx_bytes, SourceFormat::Docx).unwrap();
        assert!(model.node(model.root_id()).is_some());
    }
}
