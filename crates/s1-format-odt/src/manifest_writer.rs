//! Generate `META-INF/manifest.xml` for ODF packages.

use crate::xml_util::escape_xml;

/// Generate `manifest.xml` content.
///
/// `image_paths` should be the list of `Pictures/...` paths for images.
/// `has_metadata` controls whether the `meta.xml` entry is included.
pub fn write_manifest_xml(image_paths: &[&str], has_metadata: bool) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
<manifest:file-entry manifest:full-path="/" manifest:version="1.2" manifest:media-type="application/vnd.oasis.opendocument.text"/>
<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
<manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
<manifest:file-entry manifest:full-path="settings.xml" manifest:media-type="text/xml"/>"#,
    );

    if has_metadata {
        xml.push_str(
            r#"<manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>"#,
        );
    }

    for path in image_paths {
        let mime = crate::xml_util::mime_for_extension(path.rsplit('.').next().unwrap_or(""))
            .unwrap_or("application/octet-stream");

        xml.push_str(&format!(
            r#"<manifest:file-entry manifest:full-path="{}" manifest:media-type="{}"/>"#,
            escape_xml(path),
            mime,
        ));
    }

    xml.push_str("</manifest:manifest>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_no_images() {
        let xml = write_manifest_xml(&[], true);
        assert!(xml.contains("manifest:full-path=\"/\""));
        assert!(xml.contains("content.xml"));
        assert!(xml.contains("styles.xml"));
        assert!(xml.contains("settings.xml"));
        assert!(xml.contains("meta.xml"));
    }

    #[test]
    fn manifest_no_metadata() {
        let xml = write_manifest_xml(&[], false);
        assert!(xml.contains("content.xml"));
        assert!(xml.contains("styles.xml"));
        assert!(xml.contains("settings.xml"));
        assert!(!xml.contains("meta.xml"));
    }

    #[test]
    fn manifest_with_images() {
        let xml = write_manifest_xml(&["Pictures/1.png", "Pictures/2.jpg"], true);
        assert!(xml.contains("Pictures/1.png"));
        assert!(xml.contains("image/png"));
        assert!(xml.contains("Pictures/2.jpg"));
        assert!(xml.contains("image/jpeg"));
    }
}
