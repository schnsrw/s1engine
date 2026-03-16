//! Generate `word/styles.xml` from a DocumentModel.

use s1_model::{DocumentModel, StyleType};

use crate::content_writer::write_paragraph_properties_from_attrs;
use crate::content_writer::write_run_properties_from_attrs;
use crate::xml_writer::escape_xml;

/// Generate `word/styles.xml` content.
pub fn write_styles_xml(doc: &DocumentModel) -> String {
    let styles = doc.styles();
    let defaults = doc.doc_defaults();
    let has_defaults = defaults.font_family.is_some()
        || defaults.font_size.is_some()
        || defaults.line_spacing_multiple.is_some()
        || defaults.space_after.is_some()
        || defaults.space_before.is_some();

    if styles.is_empty() && !has_defaults {
        return String::new();
    }

    let mut xml = String::new();

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(
        r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    );

    // Emit docDefaults before styles
    if has_defaults {
        xml.push_str("<w:docDefaults>");

        // rPrDefault — default run properties
        let has_rpr = defaults.font_family.is_some() || defaults.font_size.is_some();
        if has_rpr {
            xml.push_str("<w:rPrDefault><w:rPr>");
            if let Some(ref ff) = defaults.font_family {
                let escaped = escape_xml(ff);
                xml.push_str(&format!(
                    r#"<w:rFonts w:ascii="{escaped}" w:hAnsi="{escaped}"/>"#
                ));
            }
            if let Some(fs) = defaults.font_size {
                let half_pts = (fs * 2.0) as i64;
                xml.push_str(&format!(r#"<w:sz w:val="{half_pts}"/>"#));
            }
            xml.push_str("</w:rPr></w:rPrDefault>");
        }

        // pPrDefault — default paragraph properties
        let has_ppr = defaults.space_after.is_some()
            || defaults.space_before.is_some()
            || defaults.line_spacing_multiple.is_some();
        if has_ppr {
            xml.push_str("<w:pPrDefault><w:pPr>");
            let mut spacing_attrs = String::new();
            if let Some(pts) = defaults.space_before {
                spacing_attrs.push_str(&format!(r#" w:before="{}""#, (pts * 20.0) as i64));
            }
            if let Some(pts) = defaults.space_after {
                spacing_attrs.push_str(&format!(r#" w:after="{}""#, (pts * 20.0) as i64));
            }
            if let Some(m) = defaults.line_spacing_multiple {
                let val = (m * 240.0) as i64;
                spacing_attrs.push_str(&format!(r#" w:line="{val}" w:lineRule="auto""#));
            }
            if !spacing_attrs.is_empty() {
                xml.push_str(&format!(r#"<w:spacing{spacing_attrs}/>"#));
            }
            xml.push_str("</w:pPr></w:pPrDefault>");
        }

        xml.push_str("</w:docDefaults>");
    }

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

    #[test]
    fn write_doc_defaults() {
        let mut doc = DocumentModel::new();
        {
            let defaults = doc.doc_defaults_mut();
            defaults.font_family = Some("Arial".to_string());
            defaults.font_size = Some(11.0); // 11pt = 22 half-points
            defaults.space_after = Some(6.0); // 6pt = 120 twips
            defaults.line_spacing_multiple = Some(1.15); // 276/240
        }
        // Need at least one style or defaults to emit
        let xml = write_styles_xml(&doc);
        assert!(xml.contains("<w:docDefaults>"));
        assert!(xml.contains(r#"<w:rFonts w:ascii="Arial" w:hAnsi="Arial"/>"#));
        assert!(xml.contains(r#"<w:sz w:val="22"/>"#));
        assert!(xml.contains(r#"w:after="120""#));
        assert!(xml.contains(r#"w:line="276""#));
        assert!(xml.contains("</w:docDefaults>"));
    }

    #[test]
    fn write_doc_defaults_only_no_styles() {
        let mut doc = DocumentModel::new();
        {
            let defaults = doc.doc_defaults_mut();
            defaults.font_size = Some(10.0);
        }
        let xml = write_styles_xml(&doc);
        assert!(!xml.is_empty());
        assert!(xml.contains("<w:docDefaults>"));
        assert!(xml.contains(r#"<w:sz w:val="20"/>"#));
    }
}
