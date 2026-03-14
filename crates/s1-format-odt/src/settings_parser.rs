//! Parse `settings.xml` from ODF packages.
//!
//! Extracts application-level configuration items such as zoom level.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::OdtError;

/// Parsed ODF settings from settings.xml.
#[derive(Debug, Clone, Default)]
pub struct OdtSettings {
    /// View zoom percentage (e.g., 100, 150, 200).
    pub zoom_percent: Option<u32>,
}

/// Parse `settings.xml` and return extracted settings.
///
/// # Errors
///
/// Returns `OdtError` if the XML cannot be parsed.
pub fn parse_settings_xml(xml: &str) -> Result<OdtSettings, OdtError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut settings = OdtSettings::default();
    let mut current_name: Option<String> = None;
    let mut in_config_item = false;
    let mut item_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name();
                if name.as_ref() == b"config-item" {
                    // Extract config:name attribute
                    current_name = None;
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"name" {
                            current_name =
                                Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                    }
                    in_config_item = true;
                    item_text.clear();
                }
            }
            Ok(Event::Text(t)) => {
                if in_config_item {
                    if let Ok(s) = t.unescape() {
                        item_text.push_str(&s);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = e.local_name();
                if name.as_ref() == b"config-item" && in_config_item {
                    if let Some(ref config_name) = current_name {
                        if config_name == "ZoomFactor" {
                            if let Ok(val) = item_text.trim().parse::<u32>() {
                                settings.zoom_percent = Some(val);
                            }
                        }
                    }
                    in_config_item = false;
                    current_name = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(format!("settings.xml: {e}"))),
            _ => {}
        }
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zoom_factor() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0" office:version="1.2">
<office:settings>
<config:config-item-set config:name="ooo:view-settings">
<config:config-item-map-indexed config:name="Views">
<config:config-item-map-entry>
<config:config-item config:name="ZoomFactor" config:type="short">150</config:config-item>
</config:config-item-map-entry>
</config:config-item-map-indexed>
</config:config-item-set>
</office:settings>
</office:document-settings>"#;

        let settings = parse_settings_xml(xml).unwrap();
        assert_eq!(settings.zoom_percent, Some(150));
    }

    #[test]
    fn parse_empty_settings() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0" office:version="1.2">
<office:settings/>
</office:document-settings>"#;

        let settings = parse_settings_xml(xml).unwrap();
        assert_eq!(settings.zoom_percent, None);
    }

    #[test]
    fn parse_settings_no_zoom() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0" office:version="1.2">
<office:settings>
<config:config-item-set config:name="ooo:configuration-settings">
<config:config-item config:name="PrinterName" config:type="string">MyPrinter</config:config-item>
</config:config-item-set>
</office:settings>
</office:document-settings>"#;

        let settings = parse_settings_xml(xml).unwrap();
        assert_eq!(settings.zoom_percent, None);
    }
}
