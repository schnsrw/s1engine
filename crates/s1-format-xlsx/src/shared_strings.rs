//! Parse and write `xl/sharedStrings.xml`.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::XlsxError;

/// Parse shared strings from XML.
pub fn parse_shared_strings(xml: &str) -> Result<Vec<String>, XlsxError> {
    let mut strings = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_si = false;
    let mut in_t = false;
    let mut current = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"si" => {
                        in_si = true;
                        current.clear();
                    }
                    b"t" if in_si => {
                        in_t = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"si" => {
                        in_si = false;
                        strings.push(std::mem::take(&mut current));
                    }
                    b"t" => {
                        in_t = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_t => {
                if let Ok(text) = e.unescape() {
                    current.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(strings)
}

/// Generate shared strings XML.
pub fn write_shared_strings(strings: &[String]) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(&format!(
        r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{}" uniqueCount="{}">"#,
        strings.len(),
        strings.len()
    ));
    for s in strings {
        xml.push_str("<si><t>");
        xml.push_str(&quick_xml::escape::escape(s));
        xml.push_str("</t></si>");
    }
    xml.push_str("</sst>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let xml = r#"<?xml version="1.0"?>
        <sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="3">
            <si><t>Hello</t></si>
            <si><t>World</t></si>
            <si><t>Foo &amp; Bar</t></si>
        </sst>"#;
        let strings = parse_shared_strings(xml).unwrap();
        assert_eq!(strings, vec!["Hello", "World", "Foo & Bar"]);
    }

    #[test]
    fn roundtrip() {
        let original = vec!["A".to_string(), "B & C".to_string(), "D\"E".to_string()];
        let xml = write_shared_strings(&original);
        let parsed = parse_shared_strings(&xml).unwrap();
        assert_eq!(original, parsed);
    }
}
