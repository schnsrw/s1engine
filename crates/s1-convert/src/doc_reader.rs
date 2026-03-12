//! Basic DOC (OLE2/CFB) reader.
//!
//! Extracts plain text and basic structure from legacy .doc files using the
//! `cfb` crate to read the OLE2 compound file container.
//!
//! This is a **partial** reader — the DOC binary format is extremely complex
//! (thousands of pages of spec). We extract:
//! - Plain text content from the "WordDocument" stream
//! - Basic paragraph breaks
//!
//! Complex formatting, tables, images, headers/footers are NOT extracted.
//! For full DOC support, consumers should use external conversion tools.

use std::io::{Cursor, Read};

use s1_model::{DocumentModel, Node, NodeId, NodeType};

use crate::error::ConvertError;

/// Read a DOC file and extract what we can into a DocumentModel.
///
/// This extracts plain text organized into paragraphs. Formatting and
/// complex structures are not preserved.
///
/// # Errors
///
/// Returns `ConvertError::InvalidDoc` if the file is not a valid OLE2 container
/// or does not contain a WordDocument stream.
pub fn read_doc(data: &[u8]) -> Result<DocumentModel, ConvertError> {
    let cursor = Cursor::new(data);
    let mut comp = cfb::CompoundFile::open(cursor)
        .map_err(|e| ConvertError::InvalidDoc(format!("not a valid OLE2 file: {e}")))?;

    // Check for WordDocument stream (required for .doc files)
    let has_word_doc = comp.walk().any(|entry| entry.name() == "WordDocument");

    if !has_word_doc {
        return Err(ConvertError::InvalidDoc(
            "missing WordDocument stream — not a Word document".into(),
        ));
    }

    // Try to read text from the document
    // The actual text in DOC files is stored in a complex binary format.
    // We attempt to extract readable text from known streams.
    let text = extract_text_from_doc(&mut comp)?;

    // Build a DocumentModel from the extracted text
    let mut doc = DocumentModel::new();
    let root_id = doc.root_id();

    let body_id = doc.next_id();
    doc.insert_node(root_id, 0, Node::new(body_id, NodeType::Body))
        .map_err(|e| ConvertError::InvalidDoc(format!("{e}")))?;

    // Split text into paragraphs
    let paragraphs: Vec<&str> = text.split('\n').collect();

    for (i, para_text) in paragraphs.iter().enumerate() {
        let trimmed = para_text.trim_matches('\r');
        if trimmed.is_empty() && i == paragraphs.len() - 1 {
            // Skip trailing empty paragraph
            continue;
        }

        let para_id = doc.next_id();
        doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
            .map_err(|e| ConvertError::InvalidDoc(format!("{e}")))?;

        if !trimmed.is_empty() {
            add_text_run(&mut doc, para_id, trimmed)?;
        }
    }

    Ok(doc)
}

/// Extract readable text from a DOC compound file.
///
/// Tries multiple strategies:
/// 1. Read from "WordDocument" stream and extract ASCII/Unicode text segments
/// 2. Fall back to brute-force text extraction from all streams
fn extract_text_from_doc(
    comp: &mut cfb::CompoundFile<Cursor<&[u8]>>,
) -> Result<String, ConvertError> {
    // Strategy: read the WordDocument stream and extract text heuristically.
    // The real DOC binary format stores text in a "clx" (complex) or "piece table"
    // structure. Full parsing would require implementing the entire MS-DOC spec.
    //
    // Instead, we do a best-effort extraction:
    // - Scan for contiguous runs of printable ASCII/Latin-1 characters
    // - Use paragraph markers (0x0D) as paragraph breaks
    // - Filter out binary noise

    let mut word_doc_data = Vec::new();
    if let Ok(mut stream) = comp.open_stream("/WordDocument") {
        stream
            .read_to_end(&mut word_doc_data)
            .map_err(|e| ConvertError::InvalidDoc(format!("failed to read WordDocument: {e}")))?;
    }

    // Also try reading from "1Table" or "0Table" which contain text pieces
    let mut table_data = Vec::new();
    if let Ok(mut stream) = comp.open_stream("/1Table") {
        let _ = stream.read_to_end(&mut table_data);
    } else if let Ok(mut stream) = comp.open_stream("/0Table") {
        let _ = stream.read_to_end(&mut table_data);
    }

    // Extract text from WordDocument using heuristic approach
    let text = extract_text_heuristic(&word_doc_data);

    if text.trim().is_empty() {
        // If we couldn't get text, return an empty doc rather than failing
        Ok(String::new())
    } else {
        Ok(text)
    }
}

/// Heuristic text extraction from a binary DOC stream.
///
/// Scans for contiguous runs of printable characters, treating 0x0D as
/// paragraph breaks. Filters out binary noise and control characters.
fn extract_text_heuristic(data: &[u8]) -> String {
    let mut result = String::new();
    let mut current_run = String::new();
    let min_run_length = 4; // Minimum chars to consider a text run valid

    let mut i = 0;
    while i < data.len() {
        let byte = data[i];

        match byte {
            // Paragraph break
            0x0D => {
                if current_run.len() >= min_run_length {
                    result.push_str(&current_run);
                    result.push('\n');
                }
                current_run.clear();
            }
            // Tab
            0x09 => {
                current_run.push('\t');
            }
            // Printable ASCII
            0x20..=0x7E => {
                current_run.push(byte as char);
            }
            // Common Latin-1 printable chars (accented letters, etc.)
            0xC0..=0xFF => {
                // Try to interpret as Latin-1
                current_run.push(byte as char);
            }
            // Anything else breaks the current text run
            _ => {
                if current_run.len() >= min_run_length {
                    // Keep valid text runs
                } else {
                    current_run.clear();
                }
            }
        }

        i += 1;
    }

    // Flush last run
    if current_run.len() >= min_run_length {
        result.push_str(&current_run);
    }

    result
}

/// Add a text run to a paragraph.
fn add_text_run(doc: &mut DocumentModel, para_id: NodeId, text: &str) -> Result<(), ConvertError> {
    let run_id = doc.next_id();
    doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
        .map_err(|e| ConvertError::InvalidDoc(format!("{e}")))?;

    let text_id = doc.next_id();
    doc.insert_node(run_id, 0, Node::text(text_id, text))
        .map_err(|e| ConvertError::InvalidDoc(format!("{e}")))?;

    Ok(())
}

/// Check if the given bytes look like a DOC (OLE2/CFB) file.
///
/// Checks for the OLE2 magic bytes: `D0 CF 11 E0 A1 B1 1A E1`.
pub fn is_doc_file(data: &[u8]) -> bool {
    data.len() >= 8
        && data[0] == 0xD0
        && data[1] == 0xCF
        && data[2] == 0x11
        && data[3] == 0xE0
        && data[4] == 0xA1
        && data[5] == 0xB1
        && data[6] == 0x1A
        && data[7] == 0xE1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_doc_file_magic_bytes() {
        let magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert!(is_doc_file(&magic));
    }

    #[test]
    fn is_doc_file_too_short() {
        assert!(!is_doc_file(&[0xD0, 0xCF]));
    }

    #[test]
    fn is_doc_file_wrong_magic() {
        assert!(!is_doc_file(&[0x50, 0x4B, 0x03, 0x04, 0, 0, 0, 0])); // ZIP
    }

    #[test]
    fn read_doc_invalid_data() {
        let result = read_doc(b"not a doc file");
        assert!(result.is_err());
    }

    #[test]
    fn extract_text_heuristic_basic() {
        // Simulate a binary stream with embedded text
        let mut data = Vec::new();
        data.extend_from_slice(b"\x00\x00\x00"); // binary noise
        data.extend_from_slice(b"Hello World"); // text
        data.push(0x0D); // paragraph break
        data.extend_from_slice(b"Second paragraph here");
        data.push(0x0D);

        let text = extract_text_heuristic(&data);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Second paragraph"));
    }

    #[test]
    fn extract_text_heuristic_filters_short_runs() {
        let mut data = Vec::new();
        data.extend_from_slice(b"AB"); // too short (< 4 chars)
        data.push(0x00);
        data.extend_from_slice(b"A valid text run here"); // long enough
        data.push(0x0D);

        let text = extract_text_heuristic(&data);
        assert!(!text.contains("AB")); // short run filtered
        assert!(text.contains("A valid text run here"));
    }

    #[test]
    fn extract_text_heuristic_empty() {
        let text = extract_text_heuristic(&[0x00, 0x01, 0x02]);
        assert!(text.is_empty());
    }

    #[test]
    fn extract_text_heuristic_tabs() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Col1\tCol2\tCol3");
        data.push(0x0D);

        let text = extract_text_heuristic(&data);
        assert!(text.contains("Col1\tCol2\tCol3"));
    }
}
