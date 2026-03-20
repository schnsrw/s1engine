//! DOCX reader — main entry point.
//!
//! Reads a DOCX file (ZIP archive) and produces a [`DocumentModel`].

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::DocumentModel;
use zip::ZipArchive;

use crate::comments_parser::parse_comments_xml;
use crate::content_parser::parse_document_xml;
use crate::endnotes_parser::parse_endnotes_xml;
use crate::error::DocxError;
use crate::footnotes_parser::parse_footnotes_xml;
use crate::header_footer_parser::{parse_footer_xml, parse_header_xml};
use crate::metadata_parser::parse_core_xml;
use crate::numbering_parser::parse_numbering_xml;
use crate::section_parser::parse_hf_type;
use crate::style_parser::parse_styles_xml;
use crate::xml_util::get_attr;

/// Read a DOCX file from bytes and produce a [`DocumentModel`].
///
/// The DOCX format is a ZIP archive containing XML files.
/// This reader handles:
/// - `word/document.xml` — main document content (paragraphs, runs, text, formatting, images)
/// - `word/styles.xml` — style definitions
/// - `docProps/core.xml` — document metadata
/// - `word/_rels/document.xml.rels` — relationships (for images, etc.)
/// - `word/media/*` — embedded media files
pub fn read(input: &[u8]) -> Result<DocumentModel, DocxError> {
    let cursor = Cursor::new(input);
    let mut archive = ZipArchive::new(cursor)?;

    let mut doc = DocumentModel::new();

    // Parse styles first (needed for style references in document.xml)
    if let Ok(styles_xml) = read_zip_entry(&mut archive, "word/styles.xml") {
        parse_styles_xml(&styles_xml, &mut doc)?;
    }

    // Parse numbering definitions (for list support)
    if let Ok(numbering_xml) = read_zip_entry(&mut archive, "word/numbering.xml") {
        parse_numbering_xml(&numbering_xml, &mut doc)?;
    }

    // Parse relationships (rId → target path for images, etc.)
    let rels = if let Ok(rels_xml) = read_zip_entry(&mut archive, "word/_rels/document.xml.rels") {
        parse_relationships(&rels_xml)
    } else {
        HashMap::new()
    };

    // Extract media files (word/media/*)
    let media = extract_media_files(&mut archive);

    // Clone numbering defs before passing to content parser (which takes &mut doc)
    let numbering = doc.numbering().clone();

    // Parse main document content (with relationships, media, and numbering)
    let doc_xml = read_zip_entry(&mut archive, "word/document.xml")?;
    let raw_sections = parse_document_xml(&doc_xml, &mut doc, &rels, &media, &numbering)?;

    // Resolve header/footer references in sections
    // First, parse all referenced header/footer XML files
    let mut rid_to_node_id: HashMap<String, s1_model::NodeId> = HashMap::new();

    for raw_sect in &raw_sections {
        for hf_ref in &raw_sect.hf_refs {
            if rid_to_node_id.contains_key(&hf_ref.rid) {
                continue; // Already parsed
            }
            // Resolve rId to file path
            if let Some(target) = rels.get(&hf_ref.rid) {
                let path = format!("word/{target}");
                if let Ok(hf_xml) = read_zip_entry(&mut archive, &path) {
                    // Parse header/footer-specific relationships if any
                    let hf_rels_path = format!("word/_rels/{}.rels", target);
                    let hf_rels =
                        if let Ok(hf_rels_xml) = read_zip_entry(&mut archive, &hf_rels_path) {
                            parse_relationships(&hf_rels_xml)
                        } else {
                            HashMap::new()
                        };

                    let node_id = if hf_ref.is_header {
                        parse_header_xml(&hf_xml, &mut doc, &hf_rels, &media, &numbering)?
                    } else {
                        parse_footer_xml(&hf_xml, &mut doc, &hf_rels, &media, &numbering)?
                    };
                    rid_to_node_id.insert(hf_ref.rid.clone(), node_id);
                }
            }
        }
    }

    // Now build final SectionProperties with resolved NodeIds
    for raw_sect in raw_sections {
        let mut props = raw_sect.props;
        for hf_ref in &raw_sect.hf_refs {
            if let Some(&node_id) = rid_to_node_id.get(&hf_ref.rid) {
                let hf_type = parse_hf_type(&hf_ref.hf_type);
                let model_ref = s1_model::HeaderFooterRef { hf_type, node_id };
                if hf_ref.is_header {
                    props.headers.push(model_ref);
                } else {
                    props.footers.push(model_ref);
                }
            }
        }
        doc.sections_mut().push(props);
    }

    // Parse metadata
    if let Ok(core_xml) = read_zip_entry(&mut archive, "docProps/core.xml") {
        parse_core_xml(&core_xml, &mut doc)?;
    }

    // Parse comments (word/comments.xml)
    if let Ok(comments_xml) = read_zip_entry(&mut archive, "word/comments.xml") {
        parse_comments_xml(&comments_xml, &mut doc)?;
    }

    // Parse footnotes (word/footnotes.xml)
    if let Ok(footnotes_xml) = read_zip_entry(&mut archive, "word/footnotes.xml") {
        parse_footnotes_xml(&footnotes_xml, &mut doc)?;
    }

    // Parse endnotes (word/endnotes.xml)
    if let Ok(endnotes_xml) = read_zip_entry(&mut archive, "word/endnotes.xml") {
        parse_endnotes_xml(&endnotes_xml, &mut doc)?;
    }

    // P5: Detect and preserve digital signatures (_xmlsignatures/ entries)
    let signature_entries = extract_signature_entries(&mut archive);
    if !signature_entries.is_empty() {
        doc.metadata_mut()
            .custom_properties
            .insert("hasDigitalSignature".to_string(), "true".to_string());

        // Parse each signature XML to extract signer info and store the first
        // signature's subject/date as top-level metadata.
        let mut sig_index = 0u32;
        for (path, content) in &signature_entries {
            // Only parse .xml files (skip .rels and other support files)
            if path.ends_with(".xml") && !path.ends_with(".rels") {
                let mut info = crate::signature_parser::parse_signature_xml(content);
                crate::signature_parser::validate_signature(&mut info);
                if sig_index == 0 {
                    doc.metadata_mut()
                        .custom_properties
                        .insert("signatureValid".to_string(), info.validation_status.clone());
                    if let Some(ref subject) = info.subject {
                        doc.metadata_mut()
                            .custom_properties
                            .insert("signatureSubject".to_string(), subject.clone());
                    }
                    if let Some(ref time) = info.signing_time {
                        doc.metadata_mut()
                            .custom_properties
                            .insert("signatureDate".to_string(), time.clone());
                    }
                }
                sig_index += 1;
            }
        }

        // Store signature count
        if sig_index > 0 {
            doc.metadata_mut()
                .custom_properties
                .insert("signatureCount".to_string(), sig_index.to_string());
        }

        // Preserve raw signature entries for round-trip fidelity using the
        // preserved_parts mechanism (binary-safe storage in the document model).
        for (path, content) in &signature_entries {
            doc.add_preserved_part(path.clone(), content.as_bytes().to_vec());
        }
    }

    // Preserve ZIP entries for round-trip fidelity (custom XML, charts, diagrams, embeddings, VBA)
    let preserve_prefixes = [
        "customXml/",
        "word/diagrams/",
        "word/charts/",
        "word/embeddings/",
    ];
    let preserve_files = ["word/vbaProject.bin", "word/vbaData.xml"];
    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            let should_preserve = preserve_prefixes.iter().any(|p| name.starts_with(p))
                || preserve_files.iter().any(|f| name == *f);
            if should_preserve {
                let mut data = Vec::new();
                if entry.read_to_end(&mut data).is_ok() && !data.is_empty() {
                    doc.add_preserved_part(name, data);
                }
            }
        }
    }

    // P5: Detect VBA macros
    let has_macros = doc
        .preserved_parts()
        .keys()
        .any(|k| k.contains("vbaProject"))
        || (0..archive.len()).any(|i| {
            archive
                .by_index(i)
                .map(|f| f.name().contains("vbaProject"))
                .unwrap_or(false)
        });
    if has_macros {
        doc.metadata_mut()
            .custom_properties
            .insert("hasMacros".to_string(), "true".to_string());
    }

    Ok(doc)
}

/// Read a file from the ZIP archive as a UTF-8 string.
/// Maximum decompressed size for a single ZIP entry (256 MB).
const MAX_ZIP_ENTRY_SIZE: u64 = 256 * 1024 * 1024;

/// Maximum decompressed size for media files (64 MB).
const MAX_MEDIA_ENTRY_SIZE: u64 = 64 * 1024 * 1024;

fn read_zip_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
) -> Result<String, DocxError> {
    let mut file = archive
        .by_name(path)
        .map_err(|_| DocxError::MissingFile(path.to_string()))?;

    if file.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(DocxError::InvalidStructure(format!(
            "ZIP entry '{path}' too large: {} bytes (max {MAX_ZIP_ENTRY_SIZE})",
            file.size()
        )));
    }

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Read a file from the ZIP archive as raw bytes.
fn read_zip_entry_bytes(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
) -> Result<Vec<u8>, DocxError> {
    let mut file = archive
        .by_name(path)
        .map_err(|_| DocxError::MissingFile(path.to_string()))?;

    if file.size() > MAX_MEDIA_ENTRY_SIZE {
        return Err(DocxError::InvalidStructure(format!(
            "ZIP entry '{path}' too large: {} bytes (max {MAX_MEDIA_ENTRY_SIZE})",
            file.size()
        )));
    }

    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

/// Parse `word/_rels/document.xml.rels` — maps rId to Target path.
fn parse_relationships(xml: &str) -> HashMap<String, String> {
    let mut rels = HashMap::new();
    let mut reader = Reader::from_str(xml);

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if e.local_name().as_ref() == b"Relationship" =>
            {
                if let (Some(id), Some(target)) = (get_attr(&e, b"Id"), get_attr(&e, b"Target")) {
                    rels.insert(id, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("[s1-format-docx] Warning: XML error in relationships: {e}");
                break;
            }
            _ => {}
        }
    }

    rels
}

/// Extract all `word/media/*` files from the ZIP as binary data.
fn extract_media_files(archive: &mut ZipArchive<Cursor<&[u8]>>) -> HashMap<String, Vec<u8>> {
    let mut media = HashMap::new();

    // Collect media file paths first (can't borrow archive twice)
    let paths: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.starts_with("word/media/") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    for path in paths {
        if let Ok(data) = read_zip_entry_bytes(archive, &path) {
            // Store with path relative to word/ (e.g., "media/image1.png")
            let relative = path.strip_prefix("word/").unwrap_or(&path).to_string();
            media.insert(relative, data);
        }
    }

    media
}

/// Extract all `_xmlsignatures/*` entries from the ZIP archive as UTF-8 strings.
///
/// Returns a vec of `(path, content)` tuples preserving the original ZIP paths.
/// Non-UTF-8 entries (e.g., binary certificate files) are skipped since the
/// round-trip preservation only needs the XML and .rels files.
fn extract_signature_entries(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Vec<(String, String)> {
    let paths: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.starts_with("_xmlsignatures/") && name.len() > "_xmlsignatures/".len() {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    let mut entries = Vec::new();
    for path in paths {
        if let Ok(content) = read_zip_entry(archive, &path) {
            entries.push((path, content));
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::AttributeKey;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Build a minimal DOCX file as bytes.
    fn make_docx(doc_xml: &str, styles_xml: Option<&str>, core_xml: Option<&str>) -> Vec<u8> {
        let buf = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(buf));
        let options = SimpleFileOptions::default();

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )
        .unwrap();

        // _rels/.rels
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
        )
        .unwrap();

        // word/document.xml
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(doc_xml.as_bytes()).unwrap();

        // Optional: word/styles.xml
        if let Some(styles) = styles_xml {
            zip.start_file("word/styles.xml", options).unwrap();
            zip.write_all(styles.as_bytes()).unwrap();
        }

        // Optional: docProps/core.xml
        if let Some(core) = core_xml {
            zip.start_file("docProps/core.xml", options).unwrap();
            zip.write_all(core.as_bytes()).unwrap();
        }

        zip.finish().unwrap().into_inner()
    }

    fn simple_doc_xml(body_content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{body_content}</w:body>
</w:document>"#
        )
    }

    #[test]
    fn read_minimal_docx() {
        let doc_xml = simple_doc_xml(r#"<w:p><w:r><w:t>Hello World</w:t></w:r></w:p>"#);
        let docx = make_docx(&doc_xml, None, None);

        let doc = read(&docx).unwrap();
        assert_eq!(doc.to_plain_text(), "Hello World");
    }

    #[test]
    fn read_multiple_paragraphs() {
        let doc_xml = simple_doc_xml(
            r#"<w:p><w:r><w:t>First paragraph</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p>
            <w:p><w:r><w:t>Third paragraph</w:t></w:r></w:p>"#,
        );
        let docx = make_docx(&doc_xml, None, None);

        let doc = read(&docx).unwrap();
        assert_eq!(
            doc.to_plain_text(),
            "First paragraph\nSecond paragraph\nThird paragraph"
        );
    }

    #[test]
    fn read_with_formatting() {
        let doc_xml = simple_doc_xml(
            r#"<w:p>
            <w:r><w:rPr><w:b/><w:sz w:val="48"/></w:rPr><w:t>Bold Title</w:t></w:r>
            </w:p>"#,
        );
        let docx = make_docx(&doc_xml, None, None);

        let doc = read(&docx).unwrap();
        assert_eq!(doc.to_plain_text(), "Bold Title");

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(run.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(run.attributes.get_f64(&AttributeKey::FontSize), Some(24.0));
        // 48 half-pts
    }

    #[test]
    fn read_with_styles() {
        let doc_xml = simple_doc_xml(
            r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>"#,
        );
        let styles_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="Heading 1"/>
    <w:rPr><w:b/><w:sz w:val="48"/></w:rPr>
  </w:style>
</w:styles>"#;
        let docx = make_docx(&doc_xml, Some(styles_xml), None);

        let doc = read(&docx).unwrap();
        assert_eq!(doc.to_plain_text(), "Title");

        // Style should be loaded
        let style = doc.style_by_id("Heading1").unwrap();
        assert_eq!(style.name, "Heading 1");
        assert_eq!(style.attributes.get_bool(&AttributeKey::Bold), Some(true));
    }

    #[test]
    fn read_with_metadata() {
        let doc_xml = simple_doc_xml(r#"<w:p><w:r><w:t>Content</w:t></w:r></w:p>"#);
        let core_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>My Document</dc:title>
  <dc:creator>Test Author</dc:creator>
</cp:coreProperties>"#;
        let docx = make_docx(&doc_xml, None, Some(core_xml));

        let doc = read(&docx).unwrap();
        assert_eq!(doc.metadata().title.as_deref(), Some("My Document"));
        assert_eq!(doc.metadata().creator.as_deref(), Some("Test Author"));
    }

    #[test]
    fn read_invalid_zip() {
        let result = read(b"not a zip file");
        assert!(result.is_err());
    }

    #[test]
    fn read_missing_document_xml() {
        // ZIP without word/document.xml
        let buf = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(buf));
        let options = SimpleFileOptions::default();
        zip.start_file("dummy.txt", options).unwrap();
        zip.write_all(b"dummy").unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let result = read(&bytes);
        assert!(result.is_err());
    }

    /// Helper: build a DOCX with digital signature entries in _xmlsignatures/.
    fn make_signed_docx(doc_xml: &str, sig_xml: &str) -> Vec<u8> {
        let buf = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(buf));
        let options = SimpleFileOptions::default();

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )
        .unwrap();

        // _rels/.rels
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
        )
        .unwrap();

        // word/document.xml
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(doc_xml.as_bytes()).unwrap();

        // _xmlsignatures/sig1.xml — digital signature entry
        zip.start_file("_xmlsignatures/sig1.xml", options).unwrap();
        zip.write_all(sig_xml.as_bytes()).unwrap();

        // _xmlsignatures/_rels/origin.sigs.rels — relationship file
        zip.start_file("_xmlsignatures/_rels/origin.sigs.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/signature" Target="sig1.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn read_detects_digital_signature() {
        let doc_xml = simple_doc_xml(r#"<w:p><w:r><w:t>Signed</w:t></w:r></w:p>"#);
        let sig_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <KeyInfo>
    <X509Data>
      <X509SubjectName>CN=Test Signer, O=Test Corp</X509SubjectName>
      <X509Certificate>MIICdTCCAd4CCQDZh...</X509Certificate>
    </X509Data>
  </KeyInfo>
  <Object>
    <SignatureProperties>
      <SignatureProperty>
        <SignedSignatureProperties>
          <SigningTime>2025-06-15T10:30:00Z</SigningTime>
        </SignedSignatureProperties>
      </SignatureProperty>
    </SignatureProperties>
  </Object>
</Signature>"#;

        let docx = make_signed_docx(&doc_xml, sig_xml);
        let doc = read(&docx).unwrap();

        assert_eq!(doc.to_plain_text(), "Signed");

        let props = &doc.metadata().custom_properties;
        assert_eq!(
            props.get("hasDigitalSignature").map(|s| s.as_str()),
            Some("true")
        );
        // When crypto feature is enabled, validate_signature runs and may return
        // "no_signature_value" for test data. Without crypto, it returns "unverified".
        assert!(
            props.get("signatureValid").is_some(),
            "signatureValid metadata should be present"
        );
        assert_eq!(
            props.get("signatureSubject").map(|s| s.as_str()),
            Some("CN=Test Signer, O=Test Corp")
        );
        assert_eq!(
            props.get("signatureDate").map(|s| s.as_str()),
            Some("2025-06-15T10:30:00Z")
        );
        assert_eq!(props.get("signatureCount").map(|s| s.as_str()), Some("1"));
    }

    #[test]
    fn read_preserves_signature_entries() {
        let doc_xml = simple_doc_xml(r#"<w:p><w:r><w:t>Signed</w:t></w:r></w:p>"#);
        let sig_xml =
            r#"<?xml version="1.0"?><Signature xmlns="http://www.w3.org/2000/09/xmldsig#"/>"#;

        let docx = make_signed_docx(&doc_xml, sig_xml);
        let doc = read(&docx).unwrap();

        let parts = doc.preserved_parts();

        // Signature XML should be preserved in preserved_parts
        let stored_sig = parts.get("_xmlsignatures/sig1.xml");
        assert!(stored_sig.is_some(), "sig1.xml should be preserved");
        let sig_str = std::str::from_utf8(stored_sig.unwrap()).unwrap();
        assert!(
            sig_str.contains("xmldsig"),
            "stored content should contain the original XML"
        );

        // Relationship file should also be preserved
        let stored_rels = parts.get("_xmlsignatures/_rels/origin.sigs.rels");
        assert!(stored_rels.is_some(), "rels file should be preserved");
    }

    #[test]
    fn read_no_signature_no_metadata() {
        let doc_xml = simple_doc_xml(r#"<w:p><w:r><w:t>Unsigned</w:t></w:r></w:p>"#);
        let docx = make_docx(&doc_xml, None, None);

        let doc = read(&docx).unwrap();

        let props = &doc.metadata().custom_properties;
        assert!(props.get("hasDigitalSignature").is_none());
        assert!(props.get("signatureSubject").is_none());
        assert!(props.get("signatureDate").is_none());
        assert!(props.get("signatureValid").is_none());
    }
}
