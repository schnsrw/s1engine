//! Generate `settings.xml` for ODF packages.
//!
//! The ODF specification defines `settings.xml` for application-level config items
//! such as view zoom, cursor position, printer settings, etc. We generate a minimal
//! but spec-compliant settings.xml with commonly-used view settings.

/// Generate `settings.xml` content with view settings.
///
/// `zoom_percent` is the zoom level (default: 100).
pub fn write_settings_xml(zoom_percent: u32) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0" office:version="1.2">"#,
    );

    xml.push_str(r#"<office:settings>"#);

    // View settings (ooo:view-settings)
    xml.push_str(r#"<config:config-item-set config:name="ooo:view-settings">"#);
    xml.push_str(r#"<config:config-item-map-indexed config:name="Views">"#);
    xml.push_str(r#"<config:config-item-map-entry>"#);

    // Zoom factor
    xml.push_str(&format!(
        r#"<config:config-item config:name="ZoomFactor" config:type="short">{zoom_percent}</config:config-item>"#
    ));

    // View layout (0=auto, 1=single, 2=columns, 3=book)
    xml.push_str(
        r#"<config:config-item config:name="ViewLayoutColumns" config:type="short">0</config:config-item>"#,
    );

    // Zoom type (0=optimal, 1=page width, 2=entire page, 3=by value)
    xml.push_str(
        r#"<config:config-item config:name="ZoomType" config:type="short">3</config:config-item>"#,
    );

    xml.push_str(r#"</config:config-item-map-entry>"#);
    xml.push_str(r#"</config:config-item-map-indexed>"#);
    xml.push_str(r#"</config:config-item-set>"#);

    // Configuration settings (ooo:configuration-settings)
    xml.push_str(r#"<config:config-item-set config:name="ooo:configuration-settings">"#);

    // Save version on close
    xml.push_str(
        r#"<config:config-item config:name="SaveVersionOnClose" config:type="boolean">false</config:config-item>"#,
    );

    // Print settings
    xml.push_str(
        r#"<config:config-item config:name="PrinterName" config:type="string"></config:config-item>"#,
    );

    xml.push_str(r#"</config:config-item-set>"#);

    xml.push_str(r#"</office:settings>"#);
    xml.push_str(r#"</office:document-settings>"#);

    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_zoom() {
        let xml = write_settings_xml(100);
        assert!(xml.contains("office:document-settings"));
        assert!(xml.contains("office:version=\"1.2\""));
        assert!(xml.contains("ZoomFactor"));
        assert!(xml.contains(">100</config:config-item>"));
        assert!(xml.contains("ooo:view-settings"));
        assert!(xml.contains("ooo:configuration-settings"));
    }

    #[test]
    fn settings_custom_zoom() {
        let xml = write_settings_xml(150);
        assert!(xml.contains(">150</config:config-item>"));
    }

    #[test]
    fn settings_contains_view_layout() {
        let xml = write_settings_xml(100);
        assert!(xml.contains("ViewLayoutColumns"));
        assert!(xml.contains("ZoomType"));
    }
}
