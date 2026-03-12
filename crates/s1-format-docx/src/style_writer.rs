//! Generate `word/styles.xml` from a DocumentModel.

use s1_model::{DocumentModel, StyleType};

use crate::content_writer::write_paragraph_properties_from_attrs;
use crate::content_writer::write_run_properties_from_attrs;
use crate::xml_writer::escape_xml;

/// Generate `word/styles.xml` content.
pub fn write_styles_xml(doc: &DocumentModel) -> String {
    let styles = doc.styles();
    if styles.is_empty() {
        return String::new();
    }

    let mut xml = String::new();

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(
        r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    );

    for style in styles {
        let type_str = match style.style_type {
            StyleType::Paragraph => "paragraph",
            StyleType::Character => "character",
            StyleType::Table => "table",
            StyleType::List => "numbering",
            _ => "paragraph",
        };

        xml.push_str(&format!(
            r#"<w:style w:type="{}" w:styleId="{}""#,
            type_str,
            escape_xml(&style.id)
        ));

        if style.is_default {
            xml.push_str(r#" w:default="1""#);
        }

        xml.push('>');

        // Name
        xml.push_str(&format!(r#"<w:name w:val="{}"/>"#, escape_xml(&style.name)));

        // basedOn
        if let Some(ref parent) = style.parent_id {
            xml.push_str(&format!(r#"<w:basedOn w:val="{}"/>"#, escape_xml(parent)));
        }

        // next
        if let Some(ref next) = style.next_style_id {
            xml.push_str(&format!(r#"<w:next w:val="{}"/>"#, escape_xml(next)));
        }

        // Paragraph properties
        let ppr = write_paragraph_properties_from_attrs(&style.attributes);
        if !ppr.is_empty() {
            xml.push_str("<w:pPr>");
            xml.push_str(&ppr);
            xml.push_str("</w:pPr>");
        }

        // Run properties
        let rpr = write_run_properties_from_attrs(&style.attributes);
        if !rpr.is_empty() {
            xml.push_str("<w:rPr>");
            xml.push_str(&rpr);
            xml.push_str("</w:rPr>");
        }

        xml.push_str("</w:style>");
    }

    xml.push_str("</w:styles>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AttributeMap, Style};

    #[test]
    fn write_empty_styles() {
        let doc = DocumentModel::new();
        let xml = write_styles_xml(&doc);
        assert!(xml.is_empty());
    }

    #[test]
    fn write_basic_style() {
        let mut doc = DocumentModel::new();
        let style = Style::new("Normal", "Normal", StyleType::Paragraph);
        doc.set_style(style);

        let xml = write_styles_xml(&doc);
        assert!(xml.contains(r#"w:type="paragraph""#));
        assert!(xml.contains(r#"w:styleId="Normal""#));
        assert!(xml.contains(r#"<w:name w:val="Normal"/>"#));
    }

    #[test]
    fn write_style_with_parent_and_next() {
        let mut doc = DocumentModel::new();

        let mut style = Style::new("Heading1", "Heading 1", StyleType::Paragraph);
        style.parent_id = Some("Normal".to_string());
        style.next_style_id = Some("Normal".to_string());
        style.attributes = AttributeMap::new().bold(true).font_size(24.0);
        doc.set_style(style);

        let xml = write_styles_xml(&doc);
        assert!(xml.contains(r#"<w:basedOn w:val="Normal"/>"#));
        assert!(xml.contains(r#"<w:next w:val="Normal"/>"#));
        assert!(xml.contains("<w:b/>"));
        assert!(xml.contains(r#"<w:sz w:val="48"/>"#));
    }

    #[test]
    fn write_default_style() {
        let mut doc = DocumentModel::new();
        let mut style = Style::new("Normal", "Normal", StyleType::Paragraph);
        style.is_default = true;
        doc.set_style(style);

        let xml = write_styles_xml(&doc);
        assert!(xml.contains(r#"w:default="1""#));
    }

    #[test]
    fn write_character_style() {
        let mut doc = DocumentModel::new();
        let style = Style::new("BoldChar", "Bold Character", StyleType::Character);
        doc.set_style(style);

        let xml = write_styles_xml(&doc);
        assert!(xml.contains(r#"w:type="character""#));
        assert!(xml.contains(r#"w:styleId="BoldChar""#));
    }
}
