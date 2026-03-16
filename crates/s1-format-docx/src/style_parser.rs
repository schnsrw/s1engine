//! Parse `word/styles.xml` — style definitions.

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{DocumentModel, Style, StyleType};

use crate::error::DocxError;
use crate::property_parser::{parse_paragraph_properties, parse_run_properties};
use crate::xml_util::{get_attr, get_val};

/// Parse `word/styles.xml` and add styles to the document model.
pub fn parse_styles_xml(xml: &str, doc: &mut DocumentModel) -> Result<(), DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"docDefaults" => {
                parse_doc_defaults(&mut reader, doc)?;
            }
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"style" => {
                let style_id = get_attr(&e, b"styleId").unwrap_or_default();
                let style_type_str = get_attr(&e, b"type").unwrap_or_default();
                let is_default = get_attr(&e, b"default")
                    .map(|v| v == "1" || v == "true")
                    .unwrap_or(false);

                let style_type = match style_type_str.as_str() {
                    "paragraph" => StyleType::Paragraph,
                    "character" => StyleType::Character,
                    "table" => StyleType::Table,
                    "numbering" | "list" => StyleType::List,
                    _ => StyleType::Paragraph,
                };

                parse_style_element(&mut reader, doc, &style_id, style_type, is_default)?;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Parse the contents of a single `<w:style>` element.
fn parse_style_element(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    style_id: &str,
    style_type: StyleType,
    is_default: bool,
) -> Result<(), DocxError> {
    let mut name = String::new();
    let mut parent_id = None;
    let mut next_style_id = None;
    let mut attrs = s1_model::AttributeMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local = e.local_name().as_ref().to_vec();
                match local.as_slice() {
                    b"name" => {
                        if let Some(val) = get_val(&e) {
                            name = val;
                        }
                        skip_to_end(reader)?;
                    }
                    b"basedOn" => {
                        parent_id = get_val(&e);
                        skip_to_end(reader)?;
                    }
                    b"next" => {
                        next_style_id = get_val(&e);
                        skip_to_end(reader)?;
                    }
                    b"pPr" => {
                        let para_attrs = parse_paragraph_properties(reader)?;
                        attrs.merge(&para_attrs);
                    }
                    b"rPr" => {
                        let run_attrs = parse_run_properties(reader)?;
                        attrs.merge(&run_attrs);
                    }
                    _ => {
                        skip_to_end(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let local = e.local_name().as_ref().to_vec();
                match local.as_slice() {
                    b"name" => {
                        if let Some(val) = get_val(&e) {
                            name = val;
                        }
                    }
                    b"basedOn" => {
                        parent_id = get_val(&e);
                    }
                    b"next" => {
                        next_style_id = get_val(&e);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"style" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    // Build and store the style
    let mut style = Style::new(style_id, &name, style_type);
    style.parent_id = parent_id;
    style.next_style_id = next_style_id;
    style.is_default = is_default;
    style.attributes = attrs;

    doc.set_style(style);

    Ok(())
}

/// Parse `<w:docDefaults>` — document-level formatting defaults.
///
/// Extracts default run properties (`rPrDefault`) and paragraph
/// properties (`pPrDefault`) and stores them in the document model.
fn parse_doc_defaults(reader: &mut Reader<&[u8]>, doc: &mut DocumentModel) -> Result<(), DocxError> {
    let defaults = doc.doc_defaults_mut();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local = e.local_name().as_ref().to_vec();
                match local.as_slice() {
                    b"rPr" => {
                        // Parse run properties inside rPrDefault
                        let attrs = parse_run_properties(reader)?;
                        if let Some(s1_model::AttributeValue::Float(fs)) =
                            attrs.get(&s1_model::AttributeKey::FontSize)
                        {
                            defaults.font_size = Some(*fs);
                        }
                        if let Some(s1_model::AttributeValue::String(f)) =
                            attrs.get(&s1_model::AttributeKey::FontFamily)
                        {
                            defaults.font_family = Some(f.clone());
                        }
                    }
                    b"pPr" => {
                        // Parse paragraph properties inside pPrDefault
                        let attrs = parse_paragraph_properties(reader)?;
                        if let Some(s1_model::AttributeValue::Float(v)) =
                            attrs.get(&s1_model::AttributeKey::SpacingAfter)
                        {
                            defaults.space_after = Some(*v);
                        }
                        if let Some(s1_model::AttributeValue::Float(v)) =
                            attrs.get(&s1_model::AttributeKey::SpacingBefore)
                        {
                            defaults.space_before = Some(*v);
                        }
                        if let Some(s1_model::AttributeValue::LineSpacing(
                            s1_model::LineSpacing::Multiple(m),
                        )) = attrs.get(&s1_model::AttributeKey::LineSpacing)
                        {
                            defaults.line_spacing_multiple = Some(*m);
                        }
                    }
                    _ => {
                        // skip unknown children (rPrDefault, pPrDefault wrappers, etc.)
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"docDefaults" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Skip to the matching end tag.
fn skip_to_end(reader: &mut Reader<&[u8]>) -> Result<(), DocxError> {
    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::AttributeKey;

    fn wrap_styles(content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
{content}
</w:styles>"#
        )
    }

    #[test]
    fn parse_basic_style() {
        let xml = wrap_styles(
            r#"<w:style w:type="paragraph" w:styleId="Normal">
            <w:name w:val="Normal"/>
            </w:style>"#,
        );
        let mut doc = DocumentModel::new();
        parse_styles_xml(&xml, &mut doc).unwrap();

        let style = doc.style_by_id("Normal").unwrap();
        assert_eq!(style.name, "Normal");
        assert_eq!(style.style_type, StyleType::Paragraph);
    }

    #[test]
    fn parse_style_with_parent() {
        let xml = wrap_styles(
            r#"<w:style w:type="paragraph" w:styleId="Normal">
            <w:name w:val="Normal"/>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Heading1">
            <w:name w:val="Heading 1"/>
            <w:basedOn w:val="Normal"/>
            <w:next w:val="Normal"/>
            <w:rPr><w:b/><w:sz w:val="48"/></w:rPr>
            </w:style>"#,
        );
        let mut doc = DocumentModel::new();
        parse_styles_xml(&xml, &mut doc).unwrap();

        let style = doc.style_by_id("Heading1").unwrap();
        assert_eq!(style.name, "Heading 1");
        assert_eq!(style.parent_id.as_deref(), Some("Normal"));
        assert_eq!(style.next_style_id.as_deref(), Some("Normal"));
        assert_eq!(style.attributes.get_bool(&AttributeKey::Bold), Some(true));
        // 48 half-points = 24pt
        assert_eq!(
            style.attributes.get_f64(&AttributeKey::FontSize),
            Some(24.0)
        );
    }

    #[test]
    fn parse_default_style() {
        let xml = wrap_styles(
            r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal">
            <w:name w:val="Normal"/>
            </w:style>"#,
        );
        let mut doc = DocumentModel::new();
        parse_styles_xml(&xml, &mut doc).unwrap();

        let style = doc.style_by_id("Normal").unwrap();
        assert!(style.is_default);
    }

    #[test]
    fn parse_character_style() {
        let xml = wrap_styles(
            r#"<w:style w:type="character" w:styleId="BoldChar">
            <w:name w:val="Bold Character"/>
            <w:rPr><w:b/></w:rPr>
            </w:style>"#,
        );
        let mut doc = DocumentModel::new();
        parse_styles_xml(&xml, &mut doc).unwrap();

        let style = doc.style_by_id("BoldChar").unwrap();
        assert_eq!(style.style_type, StyleType::Character);
        assert_eq!(style.attributes.get_bool(&AttributeKey::Bold), Some(true));
    }

    #[test]
    fn parse_style_with_paragraph_properties() {
        let xml = wrap_styles(
            r#"<w:style w:type="paragraph" w:styleId="Center">
            <w:name w:val="Centered"/>
            <w:pPr><w:jc w:val="center"/></w:pPr>
            </w:style>"#,
        );
        let mut doc = DocumentModel::new();
        parse_styles_xml(&xml, &mut doc).unwrap();

        let style = doc.style_by_id("Center").unwrap();
        assert_eq!(
            style.attributes.get_alignment(&AttributeKey::Alignment),
            Some(s1_model::Alignment::Center)
        );
    }

    #[test]
    fn parse_multiple_styles() {
        let xml = wrap_styles(
            r#"<w:style w:type="paragraph" w:styleId="Normal">
            <w:name w:val="Normal"/>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Heading1">
            <w:name w:val="Heading 1"/>
            </w:style>
            <w:style w:type="paragraph" w:styleId="Heading2">
            <w:name w:val="Heading 2"/>
            </w:style>"#,
        );
        let mut doc = DocumentModel::new();
        parse_styles_xml(&xml, &mut doc).unwrap();

        assert_eq!(doc.styles().len(), 3);
    }
}
