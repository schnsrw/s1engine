//! ODT reader — parse an ODT ZIP archive into a `DocumentModel`.

use std::collections::HashMap;
use std::io::{Cursor, Read as IoRead};

use s1_model::{DocumentModel, MediaId};
use zip::ZipArchive;

use crate::content_parser::{parse_content_body, ParseContext};
use crate::error::OdtError;
use crate::metadata_parser::parse_meta_xml;
use crate::style_parser::{parse_automatic_styles, parse_styles_xml};
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

    // 1. Parse styles.xml (optional — named styles)
    if let Ok(styles_xml) = read_zip_entry(&mut archive, "styles.xml") {
        parse_styles_xml(&styles_xml, &mut doc)?;
    }

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
}
