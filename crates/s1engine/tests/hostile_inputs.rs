//! Hostile input tests -- ensure parsers never panic on malformed data.
//!
//! These serve the same purpose as cargo-fuzz targets but run in stable Rust
//! and are included in CI. They test that all reader entry points return
//! `Err(...)` rather than panicking on garbage, truncated, or adversarial input.

use s1engine::{Engine, Format};

// ─── Random / garbage bytes ─────────────────────────────────────────

#[test]
fn empty_bytes_does_not_panic() {
    let engine = Engine::new();
    // Empty input falls through to TXT (which accepts anything) -- must not panic
    let _ = engine.open(b"");
}

#[test]
fn random_bytes_does_not_panic() {
    let engine = Engine::new();
    let garbage: Vec<u8> = (0..256).map(|i| (i * 37 + 13) as u8).collect();
    // Random bytes fall through to TXT -- must not panic
    let _ = engine.open(&garbage);
}

#[test]
fn single_byte_inputs() {
    let engine = Engine::new();
    for b in 0..=255u8 {
        // Must not panic, may return Ok (for TXT) or Err
        let _ = engine.open(&[b]);
    }
}

#[test]
fn null_bytes_returns_error_or_empty() {
    let engine = Engine::new();
    let nulls = vec![0u8; 1024];
    // Null bytes detected as TXT (binary content), should not panic
    let _ = engine.open(&nulls);
}

// ─── Truncated ZIP (DOCX/ODT) ──────────────────────────────────────

#[test]
fn truncated_zip_header() {
    let engine = Engine::new();
    // ZIP magic followed by truncation
    let data = b"PK\x03\x04";
    assert!(engine.open_as(data, Format::Docx).is_err());
    assert!(engine.open_as(data, Format::Odt).is_err());
}

#[test]
fn truncated_zip_with_partial_header() {
    let engine = Engine::new();
    let data = b"PK\x03\x04\x14\x00\x00\x00\x08\x00";
    assert!(engine.open_as(data, Format::Docx).is_err());
}

#[test]
fn valid_zip_but_missing_document_xml() {
    // Create a valid ZIP with no word/document.xml
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("dummy.txt", options).unwrap();
    zip.write_all(b"not a docx").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    assert!(engine.open_as(&bytes, Format::Docx).is_err());
}

#[test]
fn valid_zip_with_malformed_xml() {
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(b"<<<not valid xml>>>").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    // Should return Err, not panic
    let _ = engine.open_as(&bytes, Format::Docx);
}

#[test]
fn valid_zip_with_empty_document_xml() {
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(b"").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    let _ = engine.open_as(&bytes, Format::Docx);
}

// ─── Malformed XML content ──────────────────────────────────────────

#[test]
fn docx_with_deeply_nested_xml() {
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("word/document.xml", options).unwrap();

    let mut xml = String::from(r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#);
    // 100 levels of nested paragraphs (invalid but shouldn't panic)
    for _ in 0..100 {
        xml.push_str("<w:p><w:r><w:t>x</w:t></w:r>");
    }
    for _ in 0..100 {
        xml.push_str("</w:p>");
    }
    xml.push_str("</w:body></w:document>");

    zip.write_all(xml.as_bytes()).unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    let _ = engine.open_as(&bytes, Format::Docx);
}

#[test]
fn docx_with_huge_attribute_values() {
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("word/document.xml", options).unwrap();

    let huge_val = "x".repeat(10_000);
    let xml = format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="{huge_val}"/></w:pPr><w:r><w:t>text</w:t></w:r></w:p></w:body></w:document>"#
    );

    zip.write_all(xml.as_bytes()).unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    let _ = engine.open_as(&bytes, Format::Docx);
}

// ─── ODT hostile inputs ─────────────────────────────────────────────

#[test]
fn odt_with_missing_content_xml() {
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("mimetype", options).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    assert!(engine.open_as(&bytes, Format::Odt).is_err());
}

#[test]
fn odt_with_malformed_content_xml() {
    use std::io::{Cursor, Write};
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(buf));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("content.xml", options).unwrap();
    zip.write_all(b"}{}{not xml at all}{").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let engine = Engine::new();
    let _ = engine.open_as(&bytes, Format::Odt);
}

// ─── TXT hostile inputs ─────────────────────────────────────────────

#[test]
fn txt_with_mixed_encodings() {
    let engine = Engine::new();
    // UTF-16 BOM followed by invalid UTF-16
    let mut data = vec![0xFF, 0xFE]; // UTF-16 LE BOM
    data.extend_from_slice(&[0x00; 100]); // null bytes
    let _ = engine.open_as(&data, Format::Txt);
}

#[test]
fn txt_with_very_long_lines() {
    let engine = Engine::new();
    let long_line = "a".repeat(100_000);
    let result = engine.open_as(long_line.as_bytes(), Format::Txt);
    assert!(result.is_ok());
    let doc = result.unwrap();
    assert!(!doc.to_plain_text().is_empty());
}

// ─── Format detection edge cases ────────────────────────────────────

#[test]
fn format_detection_pdf_magic() {
    let engine = Engine::new();
    // PDF magic bytes — should be detected but reading is unsupported
    let data = b"%PDF-1.4 garbage";
    let result = engine.open(data);
    // Either detected as PDF (unsupported) or as TXT
    let _ = result;
}

#[test]
fn format_detection_ole2_magic() {
    let engine = Engine::new();
    // OLE2 magic bytes
    let data = b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1more garbage";
    // Should not panic
    let _ = engine.open(data);
}

// ─── Export from empty/minimal documents ────────────────────────────

#[test]
fn export_empty_document_all_formats() {
    let engine = Engine::new();
    let doc = engine.create();

    assert!(doc.export(Format::Docx).is_ok());
    assert!(doc.export(Format::Odt).is_ok());
    assert!(doc.export(Format::Txt).is_ok());
}

#[test]
fn roundtrip_empty_document_docx() {
    let engine = Engine::new();
    let doc = engine.create();
    let bytes = doc.export(Format::Docx).unwrap();
    let reopened = engine.open(&bytes).unwrap();
    assert_eq!(reopened.to_plain_text(), "");
}
