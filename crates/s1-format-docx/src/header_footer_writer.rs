//! Generate `word/header*.xml` and `word/footer*.xml` from the document model.

use s1_model::{AttributeKey, AttributeValue, DocumentModel, FieldType, NodeId, NodeType};

use crate::content_writer::ImageRelEntry;
use crate::xml_writer::escape_xml;

/// Generate header XML for a Header node.
pub fn write_header_xml(doc: &DocumentModel, header_id: NodeId) -> (String, Vec<ImageRelEntry>) {
    let mut xml = String::new();
    let mut image_rels = Vec::new();

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(r#"<w:hdr xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas" xmlns:mo="http://schemas.microsoft.com/office/mac/office/2008/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:mv="urn:schemas-microsoft-com:mac:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:w10="urn:schemas-microsoft-com:office:word" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">"#);

    write_hf_children(doc, header_id, &mut xml, &mut image_rels);

    xml.push_str("</w:hdr>");
    (xml, image_rels)
}

/// Generate footer XML for a Footer node.
pub fn write_footer_xml(doc: &DocumentModel, footer_id: NodeId) -> (String, Vec<ImageRelEntry>) {
    let mut xml = String::new();
    let mut image_rels = Vec::new();

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(r#"<w:ftr xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas" xmlns:mo="http://schemas.microsoft.com/office/mac/office/2008/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:mv="urn:schemas-microsoft-com:mac:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:w10="urn:schemas-microsoft-com:office:word" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">"#);

    write_hf_children(doc, footer_id, &mut xml, &mut image_rels);

    xml.push_str("</w:ftr>");
    (xml, image_rels)
}

/// Write block-level children of a header/footer node.
fn write_hf_children(
    doc: &DocumentModel,
    hf_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
) {
    let hf_node = match doc.node(hf_id) {
        Some(n) => n,
        None => return,
    };

    let children: Vec<NodeId> = hf_node.children.clone();
    let mut has_content = false;
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        match child.node_type {
            NodeType::Paragraph => {
                write_hf_paragraph(doc, child_id, xml, image_rels);
                has_content = true;
            }
            NodeType::Table => {
                // Reuse the table writer from content_writer
                crate::content_writer::write_table_pub(doc, child_id, xml, image_rels);
                has_content = true;
            }
            _ => {}
        }
    }

    // Headers/footers must have at least one paragraph
    if !has_content {
        xml.push_str("<w:p/>");
    }
}

/// Write a paragraph inside a header/footer, with support for Field nodes.
fn write_hf_paragraph(
    doc: &DocumentModel,
    para_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
) {
    let para = match doc.node(para_id) {
        Some(n) => n,
        None => return,
    };

    xml.push_str("<w:p>");

    // Paragraph properties
    let ppr = crate::content_writer::write_paragraph_properties_from_attrs(&para.attributes);
    if !ppr.is_empty() {
        xml.push_str("<w:pPr>");
        xml.push_str(&ppr);
        xml.push_str("</w:pPr>");
    }

    // Inline children
    let children: Vec<NodeId> = para.children.clone();
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        match child.node_type {
            NodeType::Run => crate::content_writer::write_run_pub(doc, child_id, xml),
            NodeType::Image => {
                crate::content_writer::write_image_pub(doc, child_id, xml, image_rels)
            }
            NodeType::Field => write_field(doc, child_id, xml),
            NodeType::LineBreak => xml.push_str("<w:r><w:br/></w:r>"),
            NodeType::PageBreak => xml.push_str(r#"<w:r><w:br w:type="page"/></w:r>"#),
            NodeType::Tab => xml.push_str("<w:r><w:tab/></w:r>"),
            _ => {}
        }
    }

    xml.push_str("</w:p>");
}

/// Write a Field node as `<w:fldSimple>`.
fn write_field(_doc: &DocumentModel, field_id: NodeId, xml: &mut String) {
    let field_node = match _doc.node(field_id) {
        Some(n) => n,
        None => return,
    };

    let field_type = match field_node.attributes.get(&AttributeKey::FieldType) {
        Some(AttributeValue::FieldType(ft)) => *ft,
        _ => return,
    };

    let instr = field_type_to_instruction(field_type);

    xml.push_str(&format!(
        r#"<w:fldSimple w:instr=" {} "><w:r><w:t>{}</w:t></w:r></w:fldSimple>"#,
        escape_xml(&instr),
        field_type_placeholder(field_type),
    ));
}

/// Map field type to OOXML instruction string.
pub fn field_type_to_instruction(ft: FieldType) -> String {
    match ft {
        FieldType::PageNumber => "PAGE".to_string(),
        FieldType::PageCount => "NUMPAGES".to_string(),
        FieldType::Date => "DATE".to_string(),
        FieldType::Time => "TIME".to_string(),
        FieldType::FileName => "FILENAME".to_string(),
        FieldType::Author => "AUTHOR".to_string(),
        FieldType::TableOfContents => "TOC".to_string(),
        FieldType::Custom => "CUSTOM".to_string(),
        _ => "CUSTOM".to_string(),
    }
}

/// Placeholder display text for a field.
fn field_type_placeholder(ft: FieldType) -> &'static str {
    match ft {
        FieldType::PageNumber => "1",
        FieldType::PageCount => "1",
        FieldType::Date => "1/1/2000",
        FieldType::Time => "12:00",
        FieldType::FileName => "document",
        FieldType::Author => "Author",
        FieldType::TableOfContents => "",
        FieldType::Custom => "",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{Node, NodeId};

    fn make_header_doc(text: &str) -> (DocumentModel, NodeId) {
        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).unwrap().children.len();

        let hdr_id = doc.next_id();
        doc.insert_node(root_id, root_children, Node::new(hdr_id, NodeType::Header))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(hdr_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        (doc, hdr_id)
    }

    #[test]
    fn write_header_with_text() {
        let (doc, hdr_id) = make_header_doc("My Header");
        let (xml, _rels) = write_header_xml(&doc, hdr_id);

        assert!(xml.contains("<w:hdr"));
        assert!(xml.contains("My Header"));
        assert!(xml.contains("</w:hdr>"));
    }

    #[test]
    fn write_footer_with_text() {
        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).unwrap().children.len();

        let ftr_id = doc.next_id();
        doc.insert_node(root_id, root_children, Node::new(ftr_id, NodeType::Footer))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(ftr_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Page Footer"))
            .unwrap();

        let (xml, _rels) = write_footer_xml(&doc, ftr_id);
        assert!(xml.contains("<w:ftr"));
        assert!(xml.contains("Page Footer"));
        assert!(xml.contains("</w:ftr>"));
    }

    #[test]
    fn write_footer_with_page_number_field() {
        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).unwrap().children.len();

        let ftr_id = doc.next_id();
        doc.insert_node(root_id, root_children, Node::new(ftr_id, NodeType::Footer))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(ftr_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let field_id = doc.next_id();
        let mut field = Node::new(field_id, NodeType::Field);
        field.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(FieldType::PageNumber),
        );
        doc.insert_node(para_id, 0, field).unwrap();

        let (xml, _rels) = write_footer_xml(&doc, ftr_id);
        assert!(xml.contains("w:fldSimple"));
        assert!(xml.contains("PAGE"));
    }

    #[test]
    fn write_empty_header_gets_paragraph() {
        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).unwrap().children.len();

        let hdr_id = doc.next_id();
        doc.insert_node(root_id, root_children, Node::new(hdr_id, NodeType::Header))
            .unwrap();

        let (xml, _rels) = write_header_xml(&doc, hdr_id);
        assert!(xml.contains("<w:p/>"));
    }
}
