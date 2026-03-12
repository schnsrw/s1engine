//! Write s1-model styles as ODF `styles.xml`.

use s1_model::{
    AttributeKey, AttributeValue, DocumentModel, FieldType, HeaderFooterType, NodeType,
    PageOrientation, StyleType,
};

use crate::property_writer::{write_paragraph_properties, write_text_properties};
use crate::xml_util::{escape_xml, points_to_cm};

/// Generate `styles.xml` content.
///
/// Returns `None` if the document has no named styles and no section properties.
pub fn write_styles_xml(doc: &DocumentModel) -> Option<String> {
    let styles = doc.styles();
    let sections = doc.sections();

    if styles.is_empty() && sections.is_empty() {
        return None;
    }

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">"#,
    );

    // Write named styles
    if !styles.is_empty() {
        xml.push_str("<office:styles>");
        for style in styles {
            let family = match style.style_type {
                StyleType::Paragraph => "paragraph",
                StyleType::Character => "text",
                StyleType::Table => "table",
                StyleType::List => "list",
                _ => "paragraph",
            };

            xml.push_str(&format!(
                r#"<style:style style:name="{}" style:display-name="{}" style:family="{}""#,
                escape_xml(&style.id),
                escape_xml(&style.name),
                family,
            ));

            if let Some(ref parent) = style.parent_id {
                xml.push_str(&format!(
                    r#" style:parent-style-name="{}""#,
                    escape_xml(parent)
                ));
            }

            let text_props = write_text_properties(&style.attributes);
            let para_props = write_paragraph_properties(&style.attributes);

            if text_props.is_empty() && para_props.is_empty() {
                xml.push_str("/>");
            } else {
                xml.push('>');
                if !para_props.is_empty() {
                    xml.push_str(&para_props);
                }
                if !text_props.is_empty() {
                    xml.push_str(&text_props);
                }
                xml.push_str("</style:style>");
            }
        }
        xml.push_str("</office:styles>");
    }

    // Write page layout and master page from section properties
    if let Some(sect) = sections.first() {
        // <office:automatic-styles> with <style:page-layout>
        xml.push_str("<office:automatic-styles>");
        xml.push_str(r#"<style:page-layout style:name="pm1">"#);
        write_page_layout_properties(sect, &mut xml);
        xml.push_str("</style:page-layout>");
        xml.push_str("</office:automatic-styles>");

        // <office:master-styles> with <style:master-page>
        xml.push_str("<office:master-styles>");
        xml.push_str(r#"<style:master-page style:name="Standard" style:page-layout-name="pm1">"#);

        // Default header
        if let Some(hf_ref) = sect.header(HeaderFooterType::Default) {
            xml.push_str("<style:header>");
            write_hf_content(doc, hf_ref.node_id, &mut xml);
            xml.push_str("</style:header>");
        }

        // Default footer
        if let Some(hf_ref) = sect.footer(HeaderFooterType::Default) {
            xml.push_str("<style:footer>");
            write_hf_content(doc, hf_ref.node_id, &mut xml);
            xml.push_str("</style:footer>");
        }

        // First-page header
        if let Some(hf_ref) = sect.header(HeaderFooterType::First) {
            xml.push_str("<style:header-first>");
            write_hf_content(doc, hf_ref.node_id, &mut xml);
            xml.push_str("</style:header-first>");
        }

        // First-page footer
        if let Some(hf_ref) = sect.footer(HeaderFooterType::First) {
            xml.push_str("<style:footer-first>");
            write_hf_content(doc, hf_ref.node_id, &mut xml);
            xml.push_str("</style:footer-first>");
        }

        xml.push_str("</style:master-page>");
        xml.push_str("</office:master-styles>");
    }

    xml.push_str("</office:document-styles>");
    Some(xml)
}

/// Write `<style:page-layout-properties>` from SectionProperties.
fn write_page_layout_properties(sect: &s1_model::SectionProperties, xml: &mut String) {
    xml.push_str("<style:page-layout-properties");
    xml.push_str(&format!(
        r#" fo:page-width="{}""#,
        points_to_cm(sect.page_width)
    ));
    xml.push_str(&format!(
        r#" fo:page-height="{}""#,
        points_to_cm(sect.page_height)
    ));
    let orient = match sect.orientation {
        PageOrientation::Landscape => "landscape",
        PageOrientation::Portrait => "portrait",
        _ => "portrait",
    };
    xml.push_str(&format!(r#" style:print-orientation="{orient}""#));
    xml.push_str(&format!(
        r#" fo:margin-top="{}""#,
        points_to_cm(sect.margin_top)
    ));
    xml.push_str(&format!(
        r#" fo:margin-bottom="{}""#,
        points_to_cm(sect.margin_bottom)
    ));
    xml.push_str(&format!(
        r#" fo:margin-left="{}""#,
        points_to_cm(sect.margin_left)
    ));
    xml.push_str(&format!(
        r#" fo:margin-right="{}""#,
        points_to_cm(sect.margin_right)
    ));
    xml.push_str("/>");
}

/// Write header/footer content paragraphs.
fn write_hf_content(doc: &DocumentModel, hf_id: s1_model::NodeId, xml: &mut String) {
    let hf_node = match doc.node(hf_id) {
        Some(n) => n,
        None => return,
    };

    for &para_id in &hf_node.children {
        let para = match doc.node(para_id) {
            Some(n) if n.node_type == NodeType::Paragraph => n,
            _ => continue,
        };

        xml.push_str("<text:p>");
        for &child_id in &para.children {
            let child = match doc.node(child_id) {
                Some(n) => n,
                None => continue,
            };
            match child.node_type {
                NodeType::Run => {
                    for &text_id in &child.children {
                        let text_node = match doc.node(text_id) {
                            Some(n) => n,
                            None => continue,
                        };
                        if let Some(ref text) = text_node.text_content {
                            xml.push_str(&escape_xml(text));
                        }
                    }
                }
                NodeType::Field => {
                    if let Some(AttributeValue::FieldType(ft)) =
                        child.attributes.get(&AttributeKey::FieldType)
                    {
                        match ft {
                            FieldType::PageNumber => xml.push_str("<text:page-number/>"),
                            FieldType::PageCount => xml.push_str("<text:page-count/>"),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        xml.push_str("</text:p>");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AttributeMap, Style, StyleType};

    #[test]
    fn write_no_styles() {
        let doc = DocumentModel::new();
        assert!(write_styles_xml(&doc).is_none());
    }

    #[test]
    fn write_paragraph_style() {
        let mut doc = DocumentModel::new();
        let attrs = AttributeMap::new().bold(true).font_size(24.0);
        let style =
            Style::new("Heading1", "Heading 1", StyleType::Paragraph).with_attributes(attrs);
        doc.set_style(style);

        let xml = write_styles_xml(&doc).unwrap();
        assert!(xml.contains(r#"style:name="Heading1""#));
        assert!(xml.contains(r#"style:display-name="Heading 1""#));
        assert!(xml.contains(r#"style:family="paragraph""#));
        assert!(xml.contains(r#"fo:font-weight="bold""#));
        assert!(xml.contains(r#"fo:font-size="24pt""#));
    }

    #[test]
    fn write_style_with_parent() {
        let mut doc = DocumentModel::new();
        let style = Style::new("Child", "Child", StyleType::Paragraph).with_parent("Parent");
        doc.set_style(style);

        let xml = write_styles_xml(&doc).unwrap();
        assert!(xml.contains(r#"style:parent-style-name="Parent""#));
    }

    #[test]
    fn write_character_style() {
        let mut doc = DocumentModel::new();
        let attrs = AttributeMap::new().italic(true);
        let style = Style::new("Emphasis", "Emphasis", StyleType::Character).with_attributes(attrs);
        doc.set_style(style);

        let xml = write_styles_xml(&doc).unwrap();
        assert!(xml.contains(r#"style:family="text""#));
        assert!(xml.contains(r#"fo:font-style="italic""#));
    }

    #[test]
    fn write_page_layout() {
        use s1_model::SectionProperties;

        let mut doc = DocumentModel::new();
        let mut sect = SectionProperties::default();
        sect.page_width = 595.276; // A4
        sect.page_height = 841.89;
        sect.margin_top = 72.0;
        sect.margin_bottom = 72.0;
        doc.sections_mut().push(sect);

        let xml = write_styles_xml(&doc).unwrap();
        assert!(xml.contains("style:page-layout"));
        assert!(xml.contains("fo:page-width"));
        assert!(xml.contains("fo:page-height"));
        assert!(xml.contains("style:master-page"));
    }

    #[test]
    fn write_header_footer() {
        use s1_model::{
            AttributeKey, AttributeValue, FieldType, HeaderFooterRef, HeaderFooterType, Node,
            NodeType, SectionProperties,
        };

        let mut doc = DocumentModel::new();

        // Create header node
        let hdr_id = doc.next_id();
        let root_id = doc.root_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(hdr_id, NodeType::Header))
            .unwrap();

        let hp_id = doc.next_id();
        doc.insert_node(hdr_id, 0, Node::new(hp_id, NodeType::Paragraph))
            .unwrap();

        let hr_id = doc.next_id();
        doc.insert_node(hp_id, 0, Node::new(hr_id, NodeType::Run))
            .unwrap();

        let ht_id = doc.next_id();
        doc.insert_node(hr_id, 0, Node::text(ht_id, "My Header"))
            .unwrap();

        // Create footer node with page number field
        let ftr_id = doc.next_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(ftr_id, NodeType::Footer))
            .unwrap();

        let fp_id = doc.next_id();
        doc.insert_node(ftr_id, 0, Node::new(fp_id, NodeType::Paragraph))
            .unwrap();

        let fr_id = doc.next_id();
        doc.insert_node(fp_id, 0, Node::new(fr_id, NodeType::Run))
            .unwrap();

        let ft_id = doc.next_id();
        doc.insert_node(fr_id, 0, Node::text(ft_id, "Page "))
            .unwrap();

        let ff_id = doc.next_id();
        let mut field = Node::new(ff_id, NodeType::Field);
        field.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(FieldType::PageNumber),
        );
        doc.insert_node(fp_id, 1, field).unwrap();

        // Set up section properties
        let mut sect = SectionProperties::default();
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: hdr_id,
        });
        sect.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: ftr_id,
        });
        doc.sections_mut().push(sect);

        let xml = write_styles_xml(&doc).unwrap();
        assert!(xml.contains("<style:header>"));
        assert!(xml.contains("My Header"));
        assert!(xml.contains("</style:header>"));
        assert!(xml.contains("<style:footer>"));
        assert!(xml.contains("Page "));
        assert!(xml.contains("<text:page-number/>"));
        assert!(xml.contains("</style:footer>"));
    }
}
