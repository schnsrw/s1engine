//! Plain text reader with encoding detection.
//!
//! Supports UTF-8 (with or without BOM), UTF-16 LE/BE (via BOM), and
//! falls back to Latin-1 (ISO 8859-1) if UTF-8 decoding fails.
//!
//! Line endings: `\r\n`, `\r`, and `\n` are all handled.
//! Each line becomes a Paragraph → Run → Text node.
//! Empty lines become empty paragraphs (no Run/Text children).

use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};
use s1_model::{AttributeKey, AttributeValue, DocumentModel, ListFormat, ListInfo, Node, NodeType};

use crate::error::TxtError;

/// The encoding that was detected during reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DetectedEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Latin1,
}

/// Result of reading a text file.
pub struct ReadResult {
    /// The parsed document model.
    pub document: DocumentModel,
    /// The encoding that was detected.
    pub encoding: DetectedEncoding,
}

/// Read plain text bytes into a [`DocumentModel`].
///
/// Performs encoding detection in this order:
/// 1. UTF-8 BOM (`EF BB BF`)
/// 2. UTF-16 LE BOM (`FF FE`)
/// 3. UTF-16 BE BOM (`FE FF`)
/// 4. Valid UTF-8 (no BOM)
/// 5. Latin-1 fallback (ISO 8859-1 — never fails, every byte is valid)
pub fn read(input: &[u8]) -> Result<ReadResult, TxtError> {
    let (text, encoding) = decode(input)?;
    let doc = text_to_document(&text);
    Ok(ReadResult {
        document: doc,
        encoding,
    })
}

/// Detect encoding and decode bytes to a string.
fn decode(input: &[u8]) -> Result<(String, DetectedEncoding), TxtError> {
    // Empty input
    if input.is_empty() {
        return Ok((String::new(), DetectedEncoding::Utf8));
    }

    // Check for BOM
    if input.len() >= 3 && input[0] == 0xEF && input[1] == 0xBB && input[2] == 0xBF {
        // UTF-8 BOM
        let (text, _, had_errors) = UTF_8.decode(&input[3..]);
        if had_errors {
            return Err(TxtError::DecodingError {
                encoding: "UTF-8 (BOM)".into(),
                message: "Invalid UTF-8 sequence after BOM".into(),
            });
        }
        return Ok((text.into_owned(), DetectedEncoding::Utf8Bom));
    }

    if input.len() >= 2 && input[0] == 0xFF && input[1] == 0xFE {
        // UTF-16 LE BOM
        let (text, _, had_errors) = UTF_16LE.decode(input);
        if had_errors {
            return Err(TxtError::DecodingError {
                encoding: "UTF-16 LE".into(),
                message: "Invalid UTF-16 LE sequence".into(),
            });
        }
        return Ok((text.into_owned(), DetectedEncoding::Utf16Le));
    }

    if input.len() >= 2 && input[0] == 0xFE && input[1] == 0xFF {
        // UTF-16 BE BOM
        let (text, _, had_errors) = UTF_16BE.decode(input);
        if had_errors {
            return Err(TxtError::DecodingError {
                encoding: "UTF-16 BE".into(),
                message: "Invalid UTF-16 BE sequence".into(),
            });
        }
        return Ok((text.into_owned(), DetectedEncoding::Utf16Be));
    }

    // No BOM — try UTF-8
    match std::str::from_utf8(input) {
        Ok(text) => Ok((text.to_string(), DetectedEncoding::Utf8)),
        Err(_) => {
            // Fall back to Latin-1 (ISO 8859-1) — every byte maps to a valid Unicode code point
            let text: String = input.iter().map(|&b| b as char).collect();
            Ok((text, DetectedEncoding::Latin1))
        }
    }
}

/// Convert decoded text into a document model.
///
/// Detects structural markers:
/// - `# ` through `###### ` → headings (StyleId "Heading1"–"Heading6")
/// - `- ` → bullet list item (with indentation for nesting)
/// - `N. ` → numbered list item (with indentation for nesting)
/// - `---` (exactly) → page break node
fn text_to_document(text: &str) -> DocumentModel {
    let mut doc = DocumentModel::new();
    let body_id = doc.body_id().unwrap();

    // Normalize line endings: \r\n → \n, then \r → \n
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

    let lines: Vec<&str> = normalized.split('\n').collect();

    let mut child_index = 0;
    for line in &lines {
        // Thematic break → empty paragraph with PageBreakBefore
        if *line == "---" {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes
                .set(AttributeKey::PageBreakBefore, AttributeValue::Bool(true));
            doc.insert_node(body_id, child_index, para).unwrap();
            child_index += 1;
            continue;
        }

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);

        // Check for heading marker: # Title
        let content = if let Some((level, rest)) = parse_heading_marker(line) {
            para.attributes.set(
                AttributeKey::StyleId,
                AttributeValue::String(format!("Heading{}", level)),
            );
            rest
        }
        // Check for list markers (with possible indentation)
        else if let Some((level, list_format, start, rest)) = parse_list_marker(line) {
            para.attributes.set(
                AttributeKey::ListInfo,
                AttributeValue::ListInfo(ListInfo {
                    level,
                    num_format: list_format,
                    num_id: 1,
                    start: Some(start),
                }),
            );
            rest
        } else {
            line
        };

        doc.insert_node(body_id, child_index, para).unwrap();

        // Only add Run + Text for non-empty content
        if !content.is_empty() {
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();

            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, content))
                .unwrap();
        }

        child_index += 1;
    }

    doc
}

/// Parse a heading marker at the start of a line.
/// Returns `(level, remaining_text)` if found.
fn parse_heading_marker(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hash_count = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hash_count > 6 {
        return None;
    }
    // Must have a space after the hashes
    let rest = &trimmed[hash_count..];
    rest.strip_prefix(' ').map(|text| (hash_count as u8, text))
}

/// Parse a list marker at the start of a line.
/// Returns `(nesting_level, format, start_number, remaining_text)` if found.
/// Nesting level is determined by leading indent (2 spaces per level).
fn parse_list_marker(line: &str) -> Option<(u8, ListFormat, u32, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let level = (indent / 2) as u8;
    let stripped = &line[indent..];

    // Bullet: "- text"
    if let Some(rest) = stripped.strip_prefix("- ") {
        return Some((level, ListFormat::Bullet, 1, rest));
    }

    // Numbered: "N. text"
    let digit_end = stripped.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_end > 0 {
        let after_digits = &stripped[digit_end..];
        if let Some(rest) = after_digits.strip_prefix(". ") {
            if let Ok(num) = stripped[..digit_end].parse::<u32>() {
                return Some((level, ListFormat::Decimal, num, rest));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_empty() {
        let result = read(b"").unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Utf8);
        assert_eq!(result.document.to_plain_text(), "");
    }

    #[test]
    fn read_single_line() {
        let result = read(b"Hello World").unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Utf8);
        assert_eq!(result.document.to_plain_text(), "Hello World");
    }

    #[test]
    fn read_multiple_lines() {
        let result = read(b"Line 1\nLine 2\nLine 3").unwrap();
        assert_eq!(result.document.to_plain_text(), "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn read_blank_lines() {
        let result = read(b"Line 1\n\nLine 3").unwrap();
        let text = crate::write_string(&result.document);
        assert_eq!(text, "Line 1\n\nLine 3");
    }

    #[test]
    fn read_crlf() {
        let result = read(b"Line 1\r\nLine 2\r\nLine 3").unwrap();
        assert_eq!(result.document.to_plain_text(), "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn read_cr_only() {
        let result = read(b"Line 1\rLine 2").unwrap();
        assert_eq!(result.document.to_plain_text(), "Line 1\nLine 2");
    }

    #[test]
    fn read_utf8_bom() {
        let mut input = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        input.extend_from_slice(b"Hello BOM");
        let result = read(&input).unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Utf8Bom);
        assert_eq!(result.document.to_plain_text(), "Hello BOM");
    }

    #[test]
    fn read_utf16_le_bom() {
        // "Hi" in UTF-16 LE with BOM
        let input: Vec<u8> = vec![
            0xFF, 0xFE, // BOM
            b'H', 0x00, b'i', 0x00,
        ];
        let result = read(&input).unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Utf16Le);
        assert_eq!(result.document.to_plain_text(), "Hi");
    }

    #[test]
    fn read_utf16_be_bom() {
        // "Hi" in UTF-16 BE with BOM
        let input: Vec<u8> = vec![
            0xFE, 0xFF, // BOM
            0x00, b'H', 0x00, b'i',
        ];
        let result = read(&input).unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Utf16Be);
        assert_eq!(result.document.to_plain_text(), "Hi");
    }

    #[test]
    fn read_latin1_fallback() {
        // 0xE9 = 'é' in Latin-1, but invalid as standalone UTF-8
        let input: Vec<u8> = vec![b'c', b'a', b'f', 0xE9];
        let result = read(&input).unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Latin1);
        assert_eq!(result.document.to_plain_text(), "café");
    }

    #[test]
    fn read_utf8_multibyte() {
        let input = "こんにちは".as_bytes(); // Japanese
        let result = read(input).unwrap();
        assert_eq!(result.encoding, DetectedEncoding::Utf8);
        assert_eq!(result.document.to_plain_text(), "こんにちは");
    }

    #[test]
    fn read_preserves_structure() {
        let result = read(b"Hello\nWorld").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        // Two paragraphs under body
        assert_eq!(body.children.len(), 2);

        // First paragraph has Run > Text "Hello"
        let para1 = doc.node(body.children[0]).unwrap();
        assert_eq!(para1.node_type, NodeType::Paragraph);
        assert_eq!(para1.children.len(), 1);

        let run1 = doc.node(para1.children[0]).unwrap();
        assert_eq!(run1.node_type, NodeType::Run);

        let text1 = doc.node(run1.children[0]).unwrap();
        assert_eq!(text1.text_content.as_deref(), Some("Hello"));
    }

    #[test]
    fn read_empty_lines_are_empty_paragraphs() {
        let result = read(b"A\n\nB").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        assert_eq!(body.children.len(), 3);

        // Middle paragraph should have no children (empty)
        let empty_para = doc.node(body.children[1]).unwrap();
        assert_eq!(empty_para.node_type, NodeType::Paragraph);
        assert!(empty_para.children.is_empty());
    }

    #[test]
    fn read_trailing_newline() {
        let result = read(b"Hello\n").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        // "Hello\n" splits into ["Hello", ""] → 2 paragraphs
        assert_eq!(body.children.len(), 2);
    }

    #[test]
    fn roundtrip_simple() {
        let input = "Hello World\nSecond line\n\nFourth line";
        let result = read(input.as_bytes()).unwrap();
        let output = crate::write_string(&result.document);
        assert_eq!(output, input);
    }

    #[test]
    fn roundtrip_unicode() {
        let input = "こんにちは\ncafé\nüñíçödé";
        let result = read(input.as_bytes()).unwrap();
        let output = crate::write_string(&result.document);
        assert_eq!(output, input);
    }

    #[test]
    fn roundtrip_empty() {
        let input = "";
        let result = read(input.as_bytes()).unwrap();
        let output = crate::write_string(&result.document);
        assert_eq!(output, input);
    }

    #[test]
    fn read_heading_markers() {
        let result = read(b"# Title\n## Subtitle\n### Section").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        assert_eq!(body.children.len(), 3);

        let p1 = doc.node(body.children[0]).unwrap();
        assert_eq!(
            p1.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading1")
        );

        let p2 = doc.node(body.children[1]).unwrap();
        assert_eq!(
            p2.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading2")
        );

        let p3 = doc.node(body.children[2]).unwrap();
        assert_eq!(
            p3.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading3")
        );

        // Verify text content is stripped of marker
        // Verify text content stripped of marker
        let run = doc.node(p1.children[0]).unwrap();
        let text_node = doc.node(run.children[0]).unwrap();
        assert_eq!(text_node.text_content.as_deref(), Some("Title"));
    }

    #[test]
    fn read_bullet_markers() {
        let result = read(b"- Apple\n- Banana").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let p1 = doc.node(body.children[0]).unwrap();
        if let Some(AttributeValue::ListInfo(li)) = p1.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(li.level, 0);
            assert_eq!(li.num_format, ListFormat::Bullet);
        } else {
            panic!("Expected ListInfo on bullet item");
        }
        let run = doc.node(p1.children[0]).unwrap();
        let text_node = doc.node(run.children[0]).unwrap();
        assert_eq!(text_node.text_content.as_deref(), Some("Apple"));
    }

    #[test]
    fn read_numbered_markers() {
        let result = read(b"1. First\n2. Second").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let p1 = doc.node(body.children[0]).unwrap();
        if let Some(AttributeValue::ListInfo(li)) = p1.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(li.level, 0);
            assert_eq!(li.num_format, ListFormat::Decimal);
            assert_eq!(li.start, Some(1));
        } else {
            panic!("Expected ListInfo on numbered item");
        }

        let p2 = doc.node(body.children[1]).unwrap();
        if let Some(AttributeValue::ListInfo(li)) = p2.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(li.start, Some(2));
        } else {
            panic!("Expected ListInfo on second numbered item");
        }
    }

    #[test]
    fn read_nested_list_markers() {
        let result = read(b"- Top\n  - Nested\n    - Deep").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        assert_eq!(body.children.len(), 3);

        let p0 = doc.node(body.children[0]).unwrap();
        if let Some(AttributeValue::ListInfo(li)) = p0.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(li.level, 0);
        } else {
            panic!("Expected level 0");
        }

        let p1 = doc.node(body.children[1]).unwrap();
        if let Some(AttributeValue::ListInfo(li)) = p1.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(li.level, 1);
        } else {
            panic!("Expected level 1");
        }

        let p2 = doc.node(body.children[2]).unwrap();
        if let Some(AttributeValue::ListInfo(li)) = p2.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(li.level, 2);
        } else {
            panic!("Expected level 2");
        }
    }

    #[test]
    fn read_horizontal_rule() {
        let result = read(b"Before\n---\nAfter").unwrap();
        let doc = &result.document;
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        assert_eq!(body.children.len(), 3);
        let middle = doc.node(body.children[1]).unwrap();
        assert_eq!(middle.node_type, NodeType::Paragraph);
        assert_eq!(
            middle.attributes.get_bool(&AttributeKey::PageBreakBefore),
            Some(true)
        );
    }

    #[test]
    fn roundtrip_structured_txt() {
        let input = "# Title\nSome text.\n- Item 1\n- Item 2";
        let result = read(input.as_bytes()).unwrap();
        let output = crate::write_string(&result.document);
        assert_eq!(output, input);
    }

    #[test]
    fn roundtrip_list_txt() {
        let input = "- Apple\n  - Red\n  - Green\n- Banana";
        let result = read(input.as_bytes()).unwrap();
        let output = crate::write_string(&result.document);
        assert_eq!(output, input);
    }

    #[test]
    fn roundtrip_numbered_list_txt() {
        let input = "1. First\n2. Second\n3. Third";
        let result = read(input.as_bytes()).unwrap();
        let output = crate::write_string(&result.document);
        assert_eq!(output, input);
    }
}
