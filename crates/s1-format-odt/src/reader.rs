//! ODT reader — parse an ODT ZIP archive into a `DocumentModel`.

use std::collections::HashMap;
use std::io::{Cursor, Read as IoRead};

use s1_model::{DocumentModel, MediaId, NodeId};
use zip::ZipArchive;

use crate::content_parser::{parse_content_body, ParseContext};
use crate::error::OdtError;
use crate::metadata_parser::parse_meta_xml;
use crate::style_parser::{parse_automatic_styles, parse_styles_xml, HfContent, HfSegment};
use crate::xml_util::mime_for_extension;

/// Read an ODT file from bytes into a `DocumentModel`.
///
/// # Errors
///
/// Returns `OdtError` if the archive is invalid, required files are missing,
/// or XML content cannot be parsed.
pub fn read(data: &[u8]) -> Result<DocumentModel, OdtError> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;
    let mut doc = DocumentModel::new();

    // 1. Parse styles.xml (optional — named styles + page layout + master page)
    let master_page = if let Ok(styles_xml) = read_zip_entry(&mut archive, "styles.xml") {
        parse_styles_xml(&styles_xml, &mut doc)?
    } else {
        None
    };

    // 2. Read content.xml (required)
    let content_xml = read_zip_entry(&mut archive, "content.xml")
        .map_err(|_| OdtError::MissingFile("content.xml".to_string()))?;

    // 3. Extract images from Pictures/
    let image_map = extract_images(&mut archive, &mut doc)?;

    // 4. Parse content.xml: first extract automatic styles, then parse body
    parse_content(&content_xml, &mut doc, image_map)?;

    // 5. Parse meta.xml (optional)
    if let Ok(meta_xml) = read_zip_entry(&mut archive, "meta.xml") {
        parse_meta_xml(&meta_xml, &mut doc)?;
    }

    // 6. Build section properties from master page info
    if let Some(mp) = master_page {
        build_section_from_master_page(&mut doc, mp)?;
    }

    Ok(doc)
}

/// Parse content.xml: extract automatic styles and body content.
fn parse_content(
    xml: &str,
    doc: &mut DocumentModel,
    image_map: HashMap<String, MediaId>,
) -> Result<(), OdtError> {
    let mut reader = quick_xml::Reader::from_reader(xml.as_bytes());
    let mut auto_styles = HashMap::new();

    // Scan for automatic-styles and office:text
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"automatic-styles" => {
                        auto_styles = parse_automatic_styles(&mut reader)?;
                    }
                    b"text" => {
                        let ctx = ParseContext {
                            auto_styles: auto_styles.clone(),
                            image_map,
                        };
                        parse_content_body(&mut reader, doc, &ctx)?;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(())
}

/// Maximum decompressed size for a single ZIP entry (256 MB).
const MAX_ZIP_ENTRY_SIZE: u64 = 256 * 1024 * 1024;

/// Maximum decompressed size for media files (64 MB).
const MAX_MEDIA_ENTRY_SIZE: u64 = 64 * 1024 * 1024;

/// Read a ZIP entry as a UTF-8 string.
fn read_zip_entry(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<String, OdtError> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| OdtError::MissingFile(name.to_string()))?;

    if file.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(OdtError::Xml(format!(
            "ZIP entry '{name}' too large: {} bytes",
            file.size()
        )));
    }

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Extract all images from `Pictures/` into the document's MediaStore.
///
/// Returns a map of href path → MediaId.
fn extract_images(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    doc: &mut DocumentModel,
) -> Result<HashMap<String, MediaId>, OdtError> {
    let mut map = HashMap::new();

    let image_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.starts_with("Pictures/") && name.len() > "Pictures/".len() {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    for name in image_names {
        let mut file = archive.by_name(&name)?;
        if file.size() > MAX_MEDIA_ENTRY_SIZE {
            continue; // Skip oversized media files
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let ext = name.rsplit('.').next().unwrap_or("");
        let mime = mime_for_extension(ext)
            .unwrap_or("application/octet-stream")
            .to_string();

        let media_id = doc.media_mut().insert(mime, data, Some(name.clone()));
        map.insert(name, media_id);
    }

    Ok(map)
}

/// Build SectionProperties and Header/Footer nodes from the parsed ODF master page.
fn build_section_from_master_page(
    doc: &mut DocumentModel,
    mp: crate::style_parser::OdtMasterPage,
) -> Result<(), OdtError> {
    use s1_model::{HeaderFooterRef, HeaderFooterType, PageOrientation, SectionProperties};

    let layout = &mp.layout;
    let mut sect = SectionProperties::default();

    if let Some(w) = layout.page_width {
        sect.page_width = w;
    }
    if let Some(h) = layout.page_height {
        sect.page_height = h;
    }
    if let Some(ref o) = layout.orientation {
        sect.orientation = match o.as_str() {
            "landscape" => PageOrientation::Landscape,
            _ => PageOrientation::Portrait,
        };
    }
    if let Some(v) = layout.margin_top {
        sect.margin_top = v;
    }
    if let Some(v) = layout.margin_bottom {
        sect.margin_bottom = v;
    }
    if let Some(v) = layout.margin_left {
        sect.margin_left = v;
    }
    if let Some(v) = layout.margin_right {
        sect.margin_right = v;
    }

    // Create header/footer nodes
    if let Some(header) = mp.header {
        let node_id = create_hf_node(doc, s1_model::NodeType::Header, &header)?;
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id,
        });
    }
    if let Some(footer) = mp.footer {
        let node_id = create_hf_node(doc, s1_model::NodeType::Footer, &footer)?;
        sect.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id,
        });
    }
    if let Some(first_header) = mp.first_header {
        sect.title_page = true;
        let node_id = create_hf_node(doc, s1_model::NodeType::Header, &first_header)?;
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::First,
            node_id,
        });
    }
    if let Some(first_footer) = mp.first_footer {
        sect.title_page = true;
        let node_id = create_hf_node(doc, s1_model::NodeType::Footer, &first_footer)?;
        sect.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::First,
            node_id,
        });
    }

    // Only add section if there's meaningful layout or header/footer info
    let has_layout = layout.page_width.is_some()
        || layout.page_height.is_some()
        || layout.margin_top.is_some()
        || layout.orientation.is_some();
    let has_hf = !sect.headers.is_empty() || !sect.footers.is_empty();
    if has_layout || has_hf {
        doc.sections_mut().push(sect);
    }

    Ok(())
}

/// Create a Header or Footer node with content from HfContent.
fn create_hf_node(
    doc: &mut DocumentModel,
    node_type: s1_model::NodeType,
    content: &HfContent,
) -> Result<NodeId, OdtError> {
    use s1_model::{AttributeKey, AttributeValue, FieldType, Node, NodeType};

    let hf_id = doc.next_id();
    let root_id = doc.root_id();
    let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
    doc.insert_node(root_id, root_children, Node::new(hf_id, node_type))
        .map_err(|e| OdtError::InvalidStructure(format!("{e}")))?;

    for para_segments in &content.paragraphs {
        let para_id = doc.next_id();
        let para_idx = doc.node(hf_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(hf_id, para_idx, Node::new(para_id, NodeType::Paragraph))
            .map_err(|e| OdtError::InvalidStructure(format!("{e}")))?;

        for segment in para_segments {
            match segment {
                HfSegment::Text(text) => {
                    let run_id = doc.next_id();
                    let run_idx =
                        doc.node(para_id).map(|n| n.children.len()).unwrap_or(0);
                    doc.insert_node(para_id, run_idx, Node::new(run_id, NodeType::Run))
                        .map_err(|e| OdtError::InvalidStructure(format!("{e}")))?;

                    let text_id = doc.next_id();
                    doc.insert_node(run_id, 0, Node::text(text_id, text))
                        .map_err(|e| OdtError::InvalidStructure(format!("{e}")))?;
                }
                HfSegment::PageNumber => {
                    let field_id = doc.next_id();
                    let mut field = Node::new(field_id, NodeType::Field);
                    field.attributes.set(
                        AttributeKey::FieldType,
                        AttributeValue::FieldType(FieldType::PageNumber),
                    );
                    let idx =
                        doc.node(para_id).map(|n| n.children.len()).unwrap_or(0);
                    doc.insert_node(para_id, idx, field)
                        .map_err(|e| OdtError::InvalidStructure(format!("{e}")))?;
                }
                HfSegment::PageCount => {
                    let field_id = doc.next_id();
                    let mut field = Node::new(field_id, NodeType::Field);
                    field.attributes.set(
                        AttributeKey::FieldType,
                        AttributeValue::FieldType(FieldType::PageCount),
                    );
                    let idx =
                        doc.node(para_id).map(|n| n.children.len()).unwrap_or(0);
                    doc.insert_node(para_id, idx, field)
                        .map_err(|e| OdtError::InvalidStructure(format!("{e}")))?;
                }
            }
        }
    }

    Ok(hf_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid ODT archive in memory.
    fn build_minimal_odt(content_xml: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // mimetype must be first, uncompressed
        zip.start_file("mimetype", options).unwrap();
        std::io::Write::write_all(&mut zip, b"application/vnd.oasis.opendocument.text").unwrap();

        let options_deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("content.xml", options_deflated).unwrap();
        std::io::Write::write_all(&mut zip, content_xml.as_bytes()).unwrap();

        let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/>
<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#;
        zip.start_file("META-INF/manifest.xml", options_deflated)
            .unwrap();
        std::io::Write::write_all(&mut zip, manifest.as_bytes()).unwrap();

        zip.finish().unwrap();
        buf
    }

    #[test]
    fn read_minimal_odt() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body><office:text>
<text:p>Hello world</text:p>
</office:text></office:body>
</office:document-content>"#;

        let odt = build_minimal_odt(content);
        let doc = read(&odt).unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);
    }

    #[test]
    fn read_multiple_paragraphs() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body><office:text>
<text:p>First</text:p>
<text:p>Second</text:p>
<text:p>Third</text:p>
</office:text></office:body>
</office:document-content>"#;

        let odt = build_minimal_odt(content);
        let doc = read(&odt).unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 3);
    }

    #[test]
    fn read_invalid_zip() {
        let result = read(b"not a zip file");
        assert!(result.is_err());
    }

    #[test]
    fn read_missing_content_xml() {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);

        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("mimetype", options).unwrap();
        std::io::Write::write_all(&mut zip, b"application/vnd.oasis.opendocument.text").unwrap();
        zip.finish().unwrap();

        let result = read(&buf);
        assert!(result.is_err());
    }

    /// Build an ODT archive with both content.xml and styles.xml.
    fn build_odt_with_styles(content_xml: &str, styles_xml: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);

        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("mimetype", stored).unwrap();
        std::io::Write::write_all(&mut zip, b"application/vnd.oasis.opendocument.text")
            .unwrap();

        zip.start_file("content.xml", deflated).unwrap();
        std::io::Write::write_all(&mut zip, content_xml.as_bytes()).unwrap();

        zip.start_file("styles.xml", deflated).unwrap();
        std::io::Write::write_all(&mut zip, styles_xml.as_bytes()).unwrap();

        let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/>
<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
<manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#;
        zip.start_file("META-INF/manifest.xml", deflated).unwrap();
        std::io::Write::write_all(&mut zip, manifest.as_bytes()).unwrap();

        zip.finish().unwrap();
        buf
    }

    #[test]
    fn read_with_page_layout() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body><office:text>
<text:p>Hello</text:p>
</office:text></office:body>
</office:document-content>"#;

        let styles = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
<office:automatic-styles>
  <style:page-layout style:name="pm1">
    <style:page-layout-properties fo:page-width="21cm" fo:page-height="29.7cm"
      style:print-orientation="portrait"
      fo:margin-top="2cm" fo:margin-bottom="2cm"/>
  </style:page-layout>
</office:automatic-styles>
<office:master-styles>
  <style:master-page style:name="Standard" style:page-layout-name="pm1"/>
</office:master-styles>
</office:document-styles>"#;

        let odt = build_odt_with_styles(content, styles);
        let doc = read(&odt).unwrap();

        assert_eq!(doc.sections().len(), 1);
        let sect = &doc.sections()[0];
        // A4: 21cm ≈ 595.28pt
        assert!((sect.page_width - 595.276).abs() < 1.0);
        // 29.7cm ≈ 841.89pt
        assert!((sect.page_height - 841.89).abs() < 1.0);
    }

    #[test]
    fn read_with_header_footer() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:body><office:text>
<text:p>Body text</text:p>
</office:text></office:body>
</office:document-content>"#;

        let styles = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:automatic-styles>
  <style:page-layout style:name="pm1">
    <style:page-layout-properties fo:page-width="8.5in" fo:page-height="11in"/>
  </style:page-layout>
</office:automatic-styles>
<office:master-styles>
  <style:master-page style:name="Standard" style:page-layout-name="pm1">
    <style:header>
      <text:p>Report Header</text:p>
    </style:header>
    <style:footer>
      <text:p>Page <text:page-number/></text:p>
    </style:footer>
  </style:master-page>
</office:master-styles>
</office:document-styles>"#;

        let odt = build_odt_with_styles(content, styles);
        let doc = read(&odt).unwrap();

        assert_eq!(doc.sections().len(), 1);
        let sect = &doc.sections()[0];
        assert!(sect.has_headers());
        assert!(sect.has_footers());

        // Verify header node exists and contains text
        let hdr_ref = sect
            .header(s1_model::HeaderFooterType::Default)
            .unwrap();
        let hdr_node = doc.node(hdr_ref.node_id).unwrap();
        assert_eq!(hdr_node.node_type, s1_model::NodeType::Header);
        assert!(!hdr_node.children.is_empty());

        // Verify footer has paragraph with field
        let ftr_ref = sect
            .footer(s1_model::HeaderFooterType::Default)
            .unwrap();
        let ftr_node = doc.node(ftr_ref.node_id).unwrap();
        assert_eq!(ftr_node.node_type, s1_model::NodeType::Footer);
    }
}
