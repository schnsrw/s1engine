//! DOCX writer — main entry point.
//!
//! Writes a [`DocumentModel`] as a DOCX file (ZIP archive).

use std::io::{Cursor, Write};

use s1_model::DocumentModel;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::comments_writer::write_comments_xml;
use crate::content_writer::{write_document_xml, HyperlinkRelEntry, ImageRelEntry};
use crate::error::DocxError;
use crate::header_footer_writer::{write_footer_xml, write_header_xml};
use crate::metadata_writer::write_core_xml;
use crate::numbering_writer::write_numbering_xml;
use crate::section_writer::HfRelEntry;
use crate::style_writer::write_styles_xml;

/// Write a [`DocumentModel`] as DOCX bytes.
///
/// The output is a valid ZIP archive containing the OOXML structure:
/// - `[Content_Types].xml`
/// - `_rels/.rels`
/// - `word/document.xml`
/// - `word/_rels/document.xml.rels`
/// - `word/styles.xml` (if styles exist)
/// - `docProps/core.xml` (if metadata exists)
pub fn write(doc: &DocumentModel) -> Result<Vec<u8>, DocxError> {
    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));
    let options = SimpleFileOptions::default();

    let has_styles = !doc.styles().is_empty();
    let core_xml = write_core_xml(doc);
    let has_core = core_xml.is_some();
    let numbering_xml = write_numbering_xml(doc);
    let has_numbering = numbering_xml.is_some();
    let comments_xml = write_comments_xml(doc);
    let has_comments = comments_xml.is_some();

    // Generate header/footer XML files and collect relationship info
    let mut hf_parts: Vec<HfPartEntry> = Vec::new();
    let mut hf_image_rels: Vec<ImageRelEntry> = Vec::new();
    let mut hf_counter = 0u32;

    for section in doc.sections() {
        for hf_ref in section.headers.iter().chain(section.footers.iter()) {
            // Check if we already wrote this node
            if hf_parts.iter().any(|p| p.node_id == hf_ref.node_id) {
                continue;
            }
            hf_counter += 1;
            let is_header = doc
                .node(hf_ref.node_id)
                .map(|n| n.node_type == s1_model::NodeType::Header)
                .unwrap_or(true);

            let (filename, xml, img_rels) = if is_header {
                let fname = format!("header{hf_counter}.xml");
                let (xml, rels) = write_header_xml(doc, hf_ref.node_id);
                (fname, xml, rels)
            } else {
                let fname = format!("footer{hf_counter}.xml");
                let (xml, rels) = write_footer_xml(doc, hf_ref.node_id);
                (fname, xml, rels)
            };

            let rid = format!("rHf{hf_counter}");
            hf_parts.push(HfPartEntry {
                node_id: hf_ref.node_id,
                rid: rid.clone(),
                filename: filename.clone(),
                xml,
                is_header,
                hf_type: hf_ref.hf_type,
            });
            hf_image_rels.extend(img_rels);
        }
    }

    // Generate document XML with section properties
    let (doc_xml, image_rels, hyperlink_rels) = write_document_xml_with_sections(doc, &hf_parts);

    // Merge image rels from headers/footers (for content types)
    let all_image_rels: Vec<&ImageRelEntry> =
        image_rels.iter().chain(hf_image_rels.iter()).collect();

    // Collect unique image extensions for [Content_Types].xml
    let image_extensions: std::collections::HashSet<&str> = all_image_rels
        .iter()
        .map(|r| r.extension.as_str())
        .collect();

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(
        content_types_xml(
            has_styles,
            has_core,
            has_numbering,
            has_comments,
            &image_extensions,
            &hf_parts,
        )
        .as_bytes(),
    )?;

    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(rels_xml(has_core).as_bytes())?;

    // word/_rels/document.xml.rels
    zip.start_file("word/_rels/document.xml.rels", options)?;
    zip.write_all(
        document_rels_xml(
            has_styles,
            has_numbering,
            has_comments,
            &image_rels,
            &hf_parts,
            &hyperlink_rels,
        )
        .as_bytes(),
    )?;

    // word/document.xml
    zip.start_file("word/document.xml", options)?;
    zip.write_all(doc_xml.as_bytes())?;

    // word/header*.xml and word/footer*.xml
    for part in &hf_parts {
        let path = format!("word/{}", part.filename);
        zip.start_file(&path, options)?;
        zip.write_all(part.xml.as_bytes())?;
    }

    // word/media/* (image files from body + headers/footers, deduplicated)
    {
        let mut written_media_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
        for rel in image_rels.iter().chain(hf_image_rels.iter()) {
            let path = format!("word/{}", rel.target);
            if written_media_paths.contains(&path) {
                continue; // same image already written (e.g., shared between body and header)
            }
            if let Some(item) = doc.media().get(rel.media_id) {
                zip.start_file(&path, options)?;
                zip.write_all(&item.data)?;
                written_media_paths.insert(path);
            }
        }
    }

    // word/styles.xml (optional)
    if has_styles {
        zip.start_file("word/styles.xml", options)?;
        let styles_xml = write_styles_xml(doc);
        zip.write_all(styles_xml.as_bytes())?;
    }

    // word/numbering.xml (optional)
    if let Some(ref nxml) = numbering_xml {
        zip.start_file("word/numbering.xml", options)?;
        zip.write_all(nxml.as_bytes())?;
    }

    // word/comments.xml (optional)
    if let Some(ref cxml) = comments_xml {
        zip.start_file("word/comments.xml", options)?;
        zip.write_all(cxml.as_bytes())?;
    }

    // docProps/core.xml (optional)
    if let Some(ref core) = core_xml {
        zip.start_file("docProps/core.xml", options)?;
        zip.write_all(core.as_bytes())?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

/// Entry for a header/footer part to be written into the ZIP.
struct HfPartEntry {
    node_id: s1_model::NodeId,
    rid: String,
    filename: String,
    xml: String,
    is_header: bool,
    hf_type: s1_model::HeaderFooterType,
}

/// Write document XML with section properties embedded.
fn write_document_xml_with_sections(
    doc: &DocumentModel,
    hf_parts: &[HfPartEntry],
) -> (String, Vec<ImageRelEntry>, Vec<HyperlinkRelEntry>) {
    use s1_model::{AttributeKey, NodeId, NodeType};

    let (xml, image_rels, hyperlink_rels) = write_document_xml(doc);

    // If there are sections, we need to inject sectPr elements.
    // The approach: rewrite the body with section properties.
    if doc.sections().is_empty() {
        return (xml, image_rels, hyperlink_rels);
    }

    // Rebuild: find </w:body> and inject final sectPr before it
    let sections = doc.sections();

    // Build section XML for inline sections (attached to paragraphs via SectionIndex)
    // and the final section (last element before </w:body>)

    // For now, we regenerate the full document XML with sections injected.
    let mut new_xml = String::new();
    let mut body_image_rels: Vec<ImageRelEntry> = Vec::new();
    let mut body_hyperlink_rels: Vec<HyperlinkRelEntry> = Vec::new();

    new_xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    new_xml.push('\n');
    new_xml.push_str(
        r#"<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas" xmlns:mo="http://schemas.microsoft.com/office/mac/office/2008/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:mv="urn:schemas-microsoft-com:mac:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:w10="urn:schemas-microsoft-com:office:word" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">"#,
    );
    new_xml.push('\n');

    if let Some(body_id) = doc.body_id() {
        new_xml.push_str("<w:body>");

        if let Some(body) = doc.node(body_id) {
            let children: Vec<NodeId> = body.children.clone();
            for child_id in children {
                let child = match doc.node(child_id) {
                    Some(n) => n,
                    None => continue,
                };
                match child.node_type {
                    NodeType::Paragraph => {
                        // Check if this paragraph ends a section (has SectionIndex attr)
                        let sect_idx = child.attributes.get_i64(&AttributeKey::SectionIndex);
                        write_paragraph_with_section(
                            doc,
                            child_id,
                            &mut new_xml,
                            &mut body_image_rels,
                            &mut body_hyperlink_rels,
                            sect_idx,
                            hf_parts,
                        );
                    }
                    NodeType::Table => {
                        crate::content_writer::write_table_with_hyperlinks_pub(
                            doc,
                            child_id,
                            &mut new_xml,
                            &mut body_image_rels,
                            &mut body_hyperlink_rels,
                        );
                    }
                    _ => {}
                }
            }
        }

        // Write the final section (the last one without a SectionIndex on a paragraph)
        if let Some(final_sect) = sections.last() {
            // Determine which section index is the final one
            let final_idx = sections.len() - 1;
            // Check if it was already written inline
            let already_written = doc
                .body_id()
                .and_then(|bid| doc.node(bid))
                .map(|body| {
                    body.children.iter().any(|&cid| {
                        doc.node(cid)
                            .and_then(|n| n.attributes.get_i64(&AttributeKey::SectionIndex))
                            .map(|idx| idx as usize == final_idx)
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);

            if !already_written {
                let hf_rel_entries = build_hf_rel_entries(final_sect, hf_parts);
                new_xml.push_str("<w:sectPr>");
                new_xml.push_str(&crate::section_writer::write_section_properties(
                    final_sect,
                    &hf_rel_entries,
                ));
                new_xml.push_str("</w:sectPr>");
            }
        }

        new_xml.push_str("</w:body>");
    }

    new_xml.push_str("</w:document>");
    (new_xml, body_image_rels, body_hyperlink_rels)
}

/// Write a paragraph, optionally followed by an inline sectPr in its pPr.
///
/// This mirrors the logic of `content_writer::write_paragraph()` for hyperlink
/// grouping, bookmarks, and comments so that section-mode documents preserve
/// all inline features.
fn write_paragraph_with_section(
    doc: &DocumentModel,
    para_id: s1_model::NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
    sect_idx: Option<i64>,
    hf_parts: &[HfPartEntry],
) {
    use s1_model::{AttributeKey, AttributeValue, NodeId, NodeType};

    let para = match doc.node(para_id) {
        Some(n) => n,
        None => return,
    };

    xml.push_str("<w:p>");

    // Paragraph properties (possibly with inline sectPr)
    let ppr = crate::content_writer::write_paragraph_properties_from_attrs(&para.attributes);
    let sect_xml = if let Some(idx) = sect_idx {
        let idx = idx as usize;
        doc.sections().get(idx).map(|props| {
            let hf_entries = build_hf_rel_entries(props, hf_parts);
            let mut s = String::from("<w:sectPr>");
            s.push_str(&crate::section_writer::write_section_properties(
                props,
                &hf_entries,
            ));
            s.push_str("</w:sectPr>");
            s
        })
    } else {
        None
    };

    if !ppr.is_empty() || sect_xml.is_some() {
        xml.push_str("<w:pPr>");
        xml.push_str(&ppr);
        if let Some(ref sect) = sect_xml {
            xml.push_str(sect);
        }
        xml.push_str("</w:pPr>");
    }

    // Inline children — group consecutive runs with the same HyperlinkUrl into
    // `<w:hyperlink>` elements, and handle bookmarks + comments.
    let children: Vec<NodeId> = para.children.clone();
    let mut i = 0;
    while i < children.len() {
        let child_id = children[i];
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => {
                i += 1;
                continue;
            }
        };

        match child.node_type {
            NodeType::Run => {
                // Check if this run is a hyperlink
                if let Some(url) = child.attributes.get_string(&AttributeKey::HyperlinkUrl) {
                    let url = url.to_string();
                    // Find all consecutive runs with the same URL
                    let hyp_start = i;
                    while i < children.len() {
                        if let Some(n) = doc.node(children[i]) {
                            if n.node_type == NodeType::Run
                                && n.attributes.get_string(&AttributeKey::HyperlinkUrl)
                                    == Some(&url)
                            {
                                i += 1;
                                continue;
                            }
                        }
                        break;
                    }

                    // Write hyperlink wrapper
                    if let Some(anchor) = url.strip_prefix('#') {
                        // Internal anchor
                        xml.push_str(&format!(
                            r#"<w:hyperlink w:anchor="{}">"#,
                            crate::xml_writer::escape_xml(anchor)
                        ));
                    } else {
                        // External link — create relationship
                        let rid = format!("rHyp{}", hyperlink_rels.len() + 1);
                        hyperlink_rels.push(HyperlinkRelEntry {
                            rid: rid.clone(),
                            target: url.clone(),
                        });
                        xml.push_str(&format!(r#"<w:hyperlink r:id="{rid}">"#));
                    }
                    for &run_id in &children[hyp_start..i] {
                        crate::content_writer::write_run_pub(doc, run_id, xml);
                    }
                    xml.push_str("</w:hyperlink>");
                } else {
                    crate::content_writer::write_run_pub(doc, child_id, xml);
                    i += 1;
                }
            }
            NodeType::Image => {
                crate::content_writer::write_image_pub(doc, child_id, xml, image_rels);
                i += 1;
            }
            NodeType::Field => {
                // Write field
                if let Some(AttributeValue::FieldType(ft)) =
                    child.attributes.get(&AttributeKey::FieldType)
                {
                    let instr = crate::header_footer_writer::field_type_to_instruction(*ft);
                    let placeholder = match ft {
                        s1_model::FieldType::PageNumber => "1",
                        s1_model::FieldType::PageCount => "1",
                        _ => "",
                    };
                    xml.push_str(&format!(
                        r#"<w:fldSimple w:instr=" {} "><w:r><w:t>{}</w:t></w:r></w:fldSimple>"#,
                        crate::xml_writer::escape_xml(&instr),
                        placeholder,
                    ));
                }
                i += 1;
            }
            NodeType::LineBreak => {
                xml.push_str("<w:r><w:br/></w:r>");
                i += 1;
            }
            NodeType::PageBreak => {
                xml.push_str(r#"<w:r><w:br w:type="page"/></w:r>"#);
                i += 1;
            }
            NodeType::ColumnBreak => {
                xml.push_str(r#"<w:r><w:br w:type="column"/></w:r>"#);
                i += 1;
            }
            NodeType::Tab => {
                xml.push_str("<w:r><w:tab/></w:r>");
                i += 1;
            }
            NodeType::BookmarkStart => {
                if let Some(bk_name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                    xml.push_str(&format!(
                        r#"<w:bookmarkStart w:id="{i}" w:name="{}"/>"#,
                        crate::xml_writer::escape_xml(bk_name)
                    ));
                }
                i += 1;
            }
            NodeType::BookmarkEnd => {
                xml.push_str(&format!(r#"<w:bookmarkEnd w:id="{i}"/>"#));
                i += 1;
            }
            NodeType::CommentStart => {
                if let Some(cid) = child.attributes.get_string(&AttributeKey::CommentId) {
                    xml.push_str(&format!(
                        r#"<w:commentRangeStart w:id="{}"/>"#,
                        crate::xml_writer::escape_xml(cid)
                    ));
                }
                i += 1;
            }
            NodeType::CommentEnd => {
                if let Some(cid) = child.attributes.get_string(&AttributeKey::CommentId) {
                    xml.push_str(&format!(
                        r#"<w:commentRangeEnd w:id="{}"/>"#,
                        crate::xml_writer::escape_xml(cid)
                    ));
                    // Add commentReference run
                    xml.push_str(&format!(
                        r#"<w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="{}"/></w:r>"#,
                        crate::xml_writer::escape_xml(cid)
                    ));
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    xml.push_str("</w:p>");
}

/// Build HfRelEntry list for a section from the master hf_parts list.
fn build_hf_rel_entries(
    props: &s1_model::SectionProperties,
    hf_parts: &[HfPartEntry],
) -> Vec<HfRelEntry> {
    let mut entries = Vec::new();
    for hf_ref in props.headers.iter().chain(props.footers.iter()) {
        if let Some(part) = hf_parts.iter().find(|p| p.node_id == hf_ref.node_id) {
            entries.push(HfRelEntry {
                rid: part.rid.clone(),
                hf_type: part.hf_type,
                is_header: part.is_header,
            });
        }
    }
    entries
}

/// Generate `[Content_Types].xml`.
fn content_types_xml(
    has_styles: bool,
    has_core: bool,
    has_numbering: bool,
    has_comments: bool,
    image_extensions: &std::collections::HashSet<&str>,
    hf_parts: &[HfPartEntry],
) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>"#,
    );

    // Image extension defaults
    for ext in image_extensions {
        if let Some(mime) = crate::xml_util::mime_for_extension(ext) {
            xml.push_str(&format!(
                r#"
  <Default Extension="{ext}" ContentType="{mime}"/>"#
            ));
        }
    }

    xml.push_str(
        r#"
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>"#,
    );

    if has_styles {
        xml.push_str(
            r#"
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>"#,
        );
    }

    if has_numbering {
        xml.push_str(
            r#"
  <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>"#,
        );
    }

    if has_comments {
        xml.push_str(
            r#"
  <Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/>"#,
        );
    }

    for part in hf_parts {
        let ct = if part.is_header {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"
        } else {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"
        };
        xml.push_str(&format!(
            r#"
  <Override PartName="/word/{}" ContentType="{ct}"/>"#,
            part.filename
        ));
    }

    if has_core {
        xml.push_str(
            r#"
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>"#,
        );
    }

    xml.push_str("\n</Types>");
    xml
}

/// Generate `_rels/.rels`.
fn rels_xml(has_core: bool) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>"#,
    );

    if has_core {
        xml.push_str(
            r#"
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>"#,
        );
    }

    xml.push_str("\n</Relationships>");
    xml
}

/// Generate `word/_rels/document.xml.rels`.
fn document_rels_xml(
    has_styles: bool,
    has_numbering: bool,
    has_comments: bool,
    image_rels: &[ImageRelEntry],
    hf_parts: &[HfPartEntry],
    hyperlink_rels: &[HyperlinkRelEntry],
) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );

    if has_styles {
        xml.push_str(
            r#"
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
        );
    }

    if has_numbering {
        xml.push_str(
            r#"
  <Relationship Id="rIdNumbering" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>"#,
        );
    }

    if has_comments {
        xml.push_str(
            r#"
  <Relationship Id="rIdComments" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments.xml"/>"#,
        );
    }

    for rel in image_rels {
        xml.push_str(&format!(
            r#"
  <Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{}"/>"#,
            rel.rid, rel.target
        ));
    }

    for part in hf_parts {
        let rel_type = if part.is_header {
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header"
        } else {
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer"
        };
        xml.push_str(&format!(
            r#"
  <Relationship Id="{}" Type="{rel_type}" Target="{}"/>"#,
            part.rid, part.filename
        ));
    }

    for hyp in hyperlink_rels {
        xml.push_str(&format!(
            r#"
  <Relationship Id="{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#,
            hyp.rid,
            crate::xml_writer::escape_xml(&hyp.target)
        ));
    }

    xml.push_str("\n</Relationships>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AttributeKey, AttributeMap, AttributeValue, Node, NodeType, Style, StyleType};

    fn make_simple_doc(text: &str) -> DocumentModel {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        doc
    }

    #[test]
    fn write_and_read_roundtrip_text() {
        let doc = make_simple_doc("Hello World");
        let bytes = write(&doc).unwrap();

        let doc2 = crate::read(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Hello World");
    }

    #[test]
    fn write_and_read_roundtrip_formatting() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes = AttributeMap::new().bold(true).font_size(24.0);
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Bold Title"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        assert_eq!(doc2.to_plain_text(), "Bold Title");

        let body_id2 = doc2.body_id().unwrap();
        let body2 = doc2.node(body_id2).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        let run2 = doc2.node(para2.children[0]).unwrap();

        use s1_model::AttributeKey;
        assert_eq!(run2.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(run2.attributes.get_f64(&AttributeKey::FontSize), Some(24.0));
    }

    #[test]
    fn write_and_read_roundtrip_styles() {
        let mut doc = make_simple_doc("Styled");

        let mut style = Style::new("Heading1", "Heading 1", StyleType::Paragraph);
        style.attributes = AttributeMap::new().bold(true).font_size(24.0);
        doc.set_style(style);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        assert_eq!(doc2.to_plain_text(), "Styled");
        let s = doc2.style_by_id("Heading1").unwrap();
        assert_eq!(s.name, "Heading 1");
        use s1_model::AttributeKey;
        assert_eq!(s.attributes.get_bool(&AttributeKey::Bold), Some(true));
    }

    #[test]
    fn write_and_read_roundtrip_metadata() {
        let mut doc = make_simple_doc("Content");
        doc.metadata_mut().title = Some("My Document".to_string());
        doc.metadata_mut().creator = Some("Test Author".to_string());

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        assert_eq!(doc2.metadata().title.as_deref(), Some("My Document"));
        assert_eq!(doc2.metadata().creator.as_deref(), Some("Test Author"));
    }

    #[test]
    fn write_produces_valid_zip() {
        let doc = make_simple_doc("Test");
        let bytes = write(&doc).unwrap();

        // Should be readable as a ZIP
        let cursor = Cursor::new(&bytes);
        let archive = zip::ZipArchive::new(cursor).unwrap();

        let names: Vec<&str> = archive.file_names().collect();
        assert!(names.contains(&"[Content_Types].xml"));
        assert!(names.contains(&"_rels/.rels"));
        assert!(names.contains(&"word/document.xml"));
        assert!(names.contains(&"word/_rels/document.xml.rels"));
    }

    #[test]
    fn write_and_read_roundtrip_table() {
        use s1_model::{AttributeKey, AttributeValue};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Table with properties
        let tbl_id = doc.next_id();
        let mut tbl = Node::new(tbl_id, NodeType::Table);
        tbl.attributes.set(
            AttributeKey::TableWidth,
            AttributeValue::TableWidth(s1_model::TableWidth::Fixed(468.0)),
        );
        doc.insert_node(body_id, 0, tbl).unwrap();

        // Row
        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        // Cell 1
        let c1_id = doc.next_id();
        let mut c1 = Node::new(c1_id, NodeType::TableCell);
        c1.attributes.set(
            AttributeKey::CellWidth,
            AttributeValue::TableWidth(s1_model::TableWidth::Fixed(234.0)),
        );
        doc.insert_node(row_id, 0, c1).unwrap();

        let p1 = doc.next_id();
        doc.insert_node(c1_id, 0, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Hello")).unwrap();

        // Cell 2
        let c2_id = doc.next_id();
        doc.insert_node(row_id, 1, Node::new(c2_id, NodeType::TableCell))
            .unwrap();
        let p2 = doc.next_id();
        doc.insert_node(c2_id, 0, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "World")).unwrap();

        // Round-trip
        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify text
        let text = doc2.to_plain_text();
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));

        // Verify structure
        let body_id2 = doc2.body_id().unwrap();
        let body2 = doc2.node(body_id2).unwrap();
        let table2 = doc2.node(body2.children[0]).unwrap();
        assert_eq!(table2.node_type, NodeType::Table);

        // Verify table width round-tripped
        match table2.attributes.get(&AttributeKey::TableWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Fixed(pts))) => {
                assert!((*pts - 468.0).abs() < 0.01);
            }
            other => panic!("Expected TableWidth::Fixed(468), got {:?}", other),
        }

        // Verify cell width round-tripped
        let row2 = doc2.node(table2.children[0]).unwrap();
        let cell2 = doc2.node(row2.children[0]).unwrap();
        match cell2.attributes.get(&AttributeKey::CellWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Fixed(pts))) => {
                assert!((*pts - 234.0).abs() < 0.01);
            }
            other => panic!("Expected CellWidth::Fixed(234), got {:?}", other),
        }
    }

    #[test]
    fn write_and_read_roundtrip_image() {
        use s1_model::AttributeKey;

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Add a fake PNG image
        let png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let media_id = doc.media_mut().insert(
            "image/png",
            png_bytes.clone(),
            Some("photo.png".to_string()),
        );

        let img_id = doc.next_id();
        let mut img = Node::new(img_id, NodeType::Image);
        img.attributes.set(
            AttributeKey::ImageMediaId,
            s1_model::AttributeValue::MediaId(media_id),
        );
        img.attributes.set(
            AttributeKey::ImageWidth,
            s1_model::AttributeValue::Float(300.0),
        );
        img.attributes.set(
            AttributeKey::ImageHeight,
            s1_model::AttributeValue::Float(200.0),
        );
        img.attributes.set(
            AttributeKey::ImageAltText,
            s1_model::AttributeValue::String("A photo".into()),
        );
        doc.insert_node(para_id, 0, img).unwrap();

        // Round-trip
        let bytes = write(&doc).unwrap();

        // Verify ZIP contains media file
        let cursor = Cursor::new(&bytes);
        let archive = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<&str> = archive.file_names().collect();
        assert!(names.iter().any(|n| n.starts_with("word/media/image")));

        // Read back and verify
        let doc2 = crate::read(&bytes).unwrap();
        let body_id2 = doc2.body_id().unwrap();
        let body2 = doc2.node(body_id2).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        let img2 = doc2.node(para2.children[0]).unwrap();

        assert_eq!(img2.node_type, NodeType::Image);
        assert!((img2.attributes.get_f64(&AttributeKey::ImageWidth).unwrap() - 300.0).abs() < 0.1);
        assert!((img2.attributes.get_f64(&AttributeKey::ImageHeight).unwrap() - 200.0).abs() < 0.1);

        // Verify media bytes survived round-trip
        if let Some(s1_model::AttributeValue::MediaId(mid)) =
            img2.attributes.get(&AttributeKey::ImageMediaId)
        {
            let item = doc2.media().get(mid.clone()).unwrap();
            assert_eq!(item.data, png_bytes);
            assert_eq!(item.content_type, "image/png");
        } else {
            panic!("Expected ImageMediaId attribute");
        }
    }

    #[test]
    fn write_and_read_roundtrip_bullet_list() {
        use s1_model::{
            AbstractNumbering, AttributeKey, AttributeValue, ListFormat, ListInfo,
            NumberingInstance, NumberingLevel,
        };

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Set up numbering definitions
        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
            abstract_num_id: 0,
            name: None,
            levels: vec![NumberingLevel {
                level: 0,
                num_format: ListFormat::Bullet,
                level_text: "\u{2022}".into(),
                start: 1,
                indent_left: Some(36.0),
                indent_hanging: Some(18.0),
                alignment: None,
                bullet_font: Some("Symbol".into()),
            }],
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 1,
            abstract_num_id: 0,
            level_overrides: vec![],
        });

        // Add 2 bullet list paragraphs
        for (i, text) in ["Item A", "Item B"].iter().enumerate() {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::ListInfo,
                AttributeValue::ListInfo(ListInfo {
                    level: 0,
                    num_format: ListFormat::Bullet,
                    num_id: 1,
                    start: None,
                }),
            );
            doc.insert_node(body_id, i, para).unwrap();

            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, *text))
                .unwrap();
        }

        // Round-trip
        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify text
        let text = doc2.to_plain_text();
        assert!(text.contains("Item A"));
        assert!(text.contains("Item B"));

        // Verify numbering definitions survived
        assert_eq!(doc2.numbering().abstract_nums.len(), 1);
        assert_eq!(doc2.numbering().instances.len(), 1);
        assert_eq!(
            doc2.numbering().resolve_format(1, 0),
            Some(ListFormat::Bullet)
        );

        // Verify paragraph has ListInfo
        let body_id2 = doc2.body_id().unwrap();
        let body2 = doc2.node(body_id2).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        if let Some(AttributeValue::ListInfo(info)) = para2.attributes.get(&AttributeKey::ListInfo)
        {
            assert_eq!(info.level, 0);
            assert_eq!(info.num_id, 1);
            assert_eq!(info.num_format, ListFormat::Bullet);
        } else {
            panic!("Expected ListInfo attribute on paragraph");
        }
    }

    #[test]
    fn write_and_read_roundtrip_numbered_list() {
        use s1_model::{
            AbstractNumbering, AttributeKey, AttributeValue, ListFormat, ListInfo,
            NumberingInstance, NumberingLevel,
        };

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
            abstract_num_id: 0,
            name: None,
            levels: vec![
                NumberingLevel {
                    level: 0,
                    num_format: ListFormat::Decimal,
                    level_text: "%1.".into(),
                    start: 1,
                    indent_left: None,
                    indent_hanging: None,
                    alignment: None,
                    bullet_font: None,
                },
                NumberingLevel {
                    level: 1,
                    num_format: ListFormat::LowerAlpha,
                    level_text: "%2)".into(),
                    start: 1,
                    indent_left: None,
                    indent_hanging: None,
                    alignment: None,
                    bullet_font: None,
                },
            ],
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 1,
            abstract_num_id: 0,
            level_overrides: vec![],
        });

        // Level 0 item
        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 0,
                num_format: ListFormat::Decimal,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 0, para).unwrap();
        let r = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(r, NodeType::Run))
            .unwrap();
        let t = doc.next_id();
        doc.insert_node(r, 0, Node::text(t, "First")).unwrap();

        // Level 1 sub-item
        let para2_id = doc.next_id();
        let mut para2 = Node::new(para2_id, NodeType::Paragraph);
        para2.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 1,
                num_format: ListFormat::LowerAlpha,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 1, para2).unwrap();
        let r2 = doc.next_id();
        doc.insert_node(para2_id, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Sub-item")).unwrap();

        // Round-trip
        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify numbering definitions
        assert_eq!(doc2.numbering().abstract_nums[0].levels.len(), 2);

        // Verify level 0
        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let p1 = doc2.node(body2.children[0]).unwrap();
        if let Some(AttributeValue::ListInfo(info)) = p1.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(info.level, 0);
            assert_eq!(info.num_format, ListFormat::Decimal);
        } else {
            panic!("Expected ListInfo on first paragraph");
        }

        // Verify level 1
        let p2 = doc2.node(body2.children[1]).unwrap();
        if let Some(AttributeValue::ListInfo(info)) = p2.attributes.get(&AttributeKey::ListInfo) {
            assert_eq!(info.level, 1);
            assert_eq!(info.num_format, ListFormat::LowerAlpha);
        } else {
            panic!("Expected ListInfo on second paragraph");
        }
    }

    #[test]
    fn write_and_read_roundtrip_section_properties() {
        use s1_model::SectionProperties;

        let mut doc = make_simple_doc("Hello");
        let mut sect = SectionProperties::default();
        sect.page_width = 792.0; // Landscape dimensions
        sect.page_height = 612.0;
        sect.orientation = s1_model::PageOrientation::Landscape;
        sect.margin_top = 54.0; // 0.75 inch
        sect.margin_bottom = 54.0;
        sect.columns = 2;
        sect.column_spacing = 36.0;
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        assert_eq!(doc2.to_plain_text(), "Hello");
        assert_eq!(doc2.sections().len(), 1);

        let s = &doc2.sections()[0];
        assert!((s.page_width - 792.0).abs() < 0.1);
        assert!((s.page_height - 612.0).abs() < 0.1);
        assert_eq!(s.orientation, s1_model::PageOrientation::Landscape);
        assert!((s.margin_top - 54.0).abs() < 0.1);
        assert_eq!(s.columns, 2);
    }

    #[test]
    fn write_and_read_roundtrip_header_footer() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, SectionProperties};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let doc_root = doc.root_id();

        // Create a paragraph in the body
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Body text"))
            .unwrap();

        // Create a Header node as child of document root
        let header_id = doc.next_id();
        doc.insert_node(doc_root, 1, Node::new(header_id, NodeType::Header))
            .unwrap();
        let hp_id = doc.next_id();
        doc.insert_node(header_id, 0, Node::new(hp_id, NodeType::Paragraph))
            .unwrap();
        let hr_id = doc.next_id();
        doc.insert_node(hp_id, 0, Node::new(hr_id, NodeType::Run))
            .unwrap();
        let ht_id = doc.next_id();
        doc.insert_node(hr_id, 0, Node::text(ht_id, "My Header"))
            .unwrap();

        // Create a Footer node as child of document root
        let footer_id = doc.next_id();
        doc.insert_node(doc_root, 2, Node::new(footer_id, NodeType::Footer))
            .unwrap();
        let fp_id = doc.next_id();
        doc.insert_node(footer_id, 0, Node::new(fp_id, NodeType::Paragraph))
            .unwrap();
        let fr_id = doc.next_id();
        doc.insert_node(fp_id, 0, Node::new(fr_id, NodeType::Run))
            .unwrap();
        let ft_id = doc.next_id();
        doc.insert_node(fr_id, 0, Node::text(ft_id, "My Footer"))
            .unwrap();

        // Set up section with header/footer references
        let mut sect = SectionProperties::default();
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: header_id,
        });
        sect.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: footer_id,
        });
        doc.sections_mut().push(sect);

        // Round-trip
        let bytes = write(&doc).unwrap();

        // Verify ZIP contains header and footer files
        let cursor = Cursor::new(&bytes);
        let archive = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<&str> = archive.file_names().collect();
        assert!(names.iter().any(|n| n.starts_with("word/header")));
        assert!(names.iter().any(|n| n.starts_with("word/footer")));

        // Read back and verify
        let doc2 = crate::read(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Body text");

        // Verify sections
        assert_eq!(doc2.sections().len(), 1);
        let s = &doc2.sections()[0];
        assert_eq!(s.headers.len(), 1);
        assert_eq!(s.footers.len(), 1);
        assert_eq!(s.headers[0].hf_type, HeaderFooterType::Default);
        assert_eq!(s.footers[0].hf_type, HeaderFooterType::Default);

        // Verify header content
        let hdr_node = doc2.node(s.headers[0].node_id).unwrap();
        assert_eq!(hdr_node.node_type, NodeType::Header);
        let hdr_para = doc2.node(hdr_node.children[0]).unwrap();
        let hdr_run = doc2.node(hdr_para.children[0]).unwrap();
        let hdr_text = doc2.node(hdr_run.children[0]).unwrap();
        assert_eq!(hdr_text.text_content.as_deref(), Some("My Header"));

        // Verify footer content
        let ftr_node = doc2.node(s.footers[0].node_id).unwrap();
        assert_eq!(ftr_node.node_type, NodeType::Footer);
        let ftr_para = doc2.node(ftr_node.children[0]).unwrap();
        let ftr_run = doc2.node(ftr_para.children[0]).unwrap();
        let ftr_text = doc2.node(ftr_run.children[0]).unwrap();
        assert_eq!(ftr_text.text_content.as_deref(), Some("My Footer"));
    }

    #[test]
    fn write_and_read_roundtrip_first_page_header() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, SectionProperties};

        let mut doc = make_simple_doc("Content");
        let doc_root = doc.root_id();

        // Default header
        let def_hdr_id = doc.next_id();
        doc.insert_node(doc_root, 1, Node::new(def_hdr_id, NodeType::Header))
            .unwrap();
        let hp = doc.next_id();
        doc.insert_node(def_hdr_id, 0, Node::new(hp, NodeType::Paragraph))
            .unwrap();
        let hr = doc.next_id();
        doc.insert_node(hp, 0, Node::new(hr, NodeType::Run))
            .unwrap();
        let ht = doc.next_id();
        doc.insert_node(hr, 0, Node::text(ht, "Default Header"))
            .unwrap();

        // First-page header
        let first_hdr_id = doc.next_id();
        doc.insert_node(doc_root, 2, Node::new(first_hdr_id, NodeType::Header))
            .unwrap();
        let fhp = doc.next_id();
        doc.insert_node(first_hdr_id, 0, Node::new(fhp, NodeType::Paragraph))
            .unwrap();
        let fhr = doc.next_id();
        doc.insert_node(fhp, 0, Node::new(fhr, NodeType::Run))
            .unwrap();
        let fht = doc.next_id();
        doc.insert_node(fhr, 0, Node::text(fht, "First Page Header"))
            .unwrap();

        let mut sect = SectionProperties::default();
        sect.title_page = true;
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: def_hdr_id,
        });
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::First,
            node_id: first_hdr_id,
        });
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let s = &doc2.sections()[0];
        assert!(s.title_page);
        assert_eq!(s.headers.len(), 2);

        let def_hdr = s.header(HeaderFooterType::Default).unwrap();
        let first_hdr = s.header(HeaderFooterType::First).unwrap();

        // Verify default header content
        let def_node = doc2.node(def_hdr.node_id).unwrap();
        let def_text_node = doc2
            .node(
                doc2.node(doc2.node(def_node.children[0]).unwrap().children[0])
                    .unwrap()
                    .children[0],
            )
            .unwrap();
        assert_eq!(
            def_text_node.text_content.as_deref(),
            Some("Default Header")
        );

        // Verify first-page header content
        let first_node = doc2.node(first_hdr.node_id).unwrap();
        let first_text_node = doc2
            .node(
                doc2.node(doc2.node(first_node.children[0]).unwrap().children[0])
                    .unwrap()
                    .children[0],
            )
            .unwrap();
        assert_eq!(
            first_text_node.text_content.as_deref(),
            Some("First Page Header")
        );
    }

    #[test]
    fn write_and_read_roundtrip_section_break() {
        use s1_model::{AttributeKey, AttributeValue, SectionBreakType, SectionProperties};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Paragraph 1 (ends section 0 with continuous break)
        let p1 = doc.next_id();
        let mut para1 = Node::new(p1, NodeType::Paragraph);
        para1
            .attributes
            .set(AttributeKey::SectionIndex, AttributeValue::Int(0));
        doc.insert_node(body_id, 0, para1).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Section 1")).unwrap();

        // Paragraph 2 (in section 1)
        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Section 2")).unwrap();

        // Section 0: continuous break
        let mut sect0 = SectionProperties::default();
        sect0.break_type = Some(SectionBreakType::Continuous);
        doc.sections_mut().push(sect0);

        // Section 1: final section (no break type)
        let sect1 = SectionProperties::default();
        doc.sections_mut().push(sect1);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        assert_eq!(doc2.to_plain_text(), "Section 1\nSection 2");
        assert_eq!(doc2.sections().len(), 2);
        assert_eq!(
            doc2.sections()[0].break_type,
            Some(SectionBreakType::Continuous)
        );
        assert!(doc2.sections()[1].break_type.is_none());
    }

    #[test]
    fn write_multiple_paragraphs_roundtrip() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        for (i, text) in ["First", "Second", "Third"].into_iter().enumerate() {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();

            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();

            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, text))
                .unwrap();
        }

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "First\nSecond\nThird");
    }

    #[test]
    fn roundtrip_hyperlink_external() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("https://example.com".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Click"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        assert!(!para2.children.is_empty());

        let run2 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("https://example.com")
        );
    }

    #[test]
    fn roundtrip_hyperlink_internal() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("#SomeBookmark".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Go"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        let run2 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("#SomeBookmark")
        );
    }

    #[test]
    fn roundtrip_bookmarks() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // BookmarkStart
        let bk_start_id = doc.next_id();
        let mut bk_start = Node::new(bk_start_id, NodeType::BookmarkStart);
        bk_start.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("BM1".into()),
        );
        doc.insert_node(para_id, 0, bk_start).unwrap();

        // Run
        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Content"))
            .unwrap();

        // BookmarkEnd
        let bk_end_id = doc.next_id();
        doc.insert_node(para_id, 2, Node::new(bk_end_id, NodeType::BookmarkEnd))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        // BookmarkStart, Run, BookmarkEnd
        assert_eq!(para2.children.len(), 3);

        let bk2 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(bk2.node_type, NodeType::BookmarkStart);
        assert_eq!(
            bk2.attributes.get_string(&AttributeKey::BookmarkName),
            Some("BM1")
        );

        assert_eq!(
            doc2.node(para2.children[2]).unwrap().node_type,
            NodeType::BookmarkEnd
        );
    }

    #[test]
    fn roundtrip_tab_stops() {
        use s1_model::{TabAlignment, TabLeader, TabStop};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::TabStops,
            AttributeValue::TabStops(vec![
                TabStop {
                    position: 36.0,
                    alignment: TabAlignment::Left,
                    leader: TabLeader::None,
                },
                TabStop {
                    position: 72.0,
                    alignment: TabAlignment::Right,
                    leader: TabLeader::Dot,
                },
            ]),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Test"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();

        if let Some(AttributeValue::TabStops(tabs)) = para2.attributes.get(&AttributeKey::TabStops)
        {
            assert_eq!(tabs.len(), 2);
            assert_eq!(tabs[0].position, 36.0);
            assert_eq!(tabs[0].alignment, TabAlignment::Left);
            assert_eq!(tabs[0].leader, TabLeader::None);
            assert_eq!(tabs[1].position, 72.0);
            assert_eq!(tabs[1].alignment, TabAlignment::Right);
            assert_eq!(tabs[1].leader, TabLeader::Dot);
        } else {
            panic!("Expected TabStops attribute after round-trip");
        }
    }

    #[test]
    fn roundtrip_paragraph_borders() {
        use s1_model::{BorderSide, BorderStyle, Borders, Color};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::ParagraphBorders,
            AttributeValue::Borders(Borders {
                top: Some(BorderSide {
                    style: BorderStyle::Single,
                    width: 1.0,
                    color: Color::new(0, 0, 0),
                    spacing: 0.0,
                }),
                bottom: None,
                left: None,
                right: None,
            }),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Bordered"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();

        if let Some(AttributeValue::Borders(borders)) =
            para2.attributes.get(&AttributeKey::ParagraphBorders)
        {
            assert!(borders.top.is_some());
            assert_eq!(borders.top.as_ref().unwrap().style, BorderStyle::Single);
        } else {
            panic!("Expected ParagraphBorders after round-trip");
        }
    }

    #[test]
    fn roundtrip_paragraph_shading() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::Background,
            AttributeValue::Color(s1_model::Color::new(0, 128, 255)),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Shaded"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();

        let bg = para2.attributes.get_color(&AttributeKey::Background);
        assert_eq!(bg, Some(s1_model::Color::new(0, 128, 255)));
    }

    #[test]
    fn roundtrip_character_spacing() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::FontSpacing, AttributeValue::Float(3.0));
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Spaced"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        let run2 = doc2.node(para2.children[0]).unwrap();

        assert_eq!(
            run2.attributes.get_f64(&AttributeKey::FontSpacing),
            Some(3.0)
        );
    }

    #[test]
    fn roundtrip_superscript_subscript() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Superscript run
        let run1_id = doc.next_id();
        let mut run1 = Node::new(run1_id, NodeType::Run);
        run1.attributes
            .set(AttributeKey::Superscript, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run1).unwrap();
        let t1 = doc.next_id();
        doc.insert_node(run1_id, 0, Node::text(t1, "sup")).unwrap();

        // Subscript run
        let run2_id = doc.next_id();
        let mut run2 = Node::new(run2_id, NodeType::Run);
        run2.attributes
            .set(AttributeKey::Subscript, AttributeValue::Bool(true));
        doc.insert_node(para_id, 1, run2).unwrap();
        let t2 = doc.next_id();
        doc.insert_node(run2_id, 0, Node::text(t2, "sub")).unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        assert_eq!(para2.children.len(), 2);

        let r1 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(
            r1.attributes.get_bool(&AttributeKey::Superscript),
            Some(true)
        );

        let r2 = doc2.node(para2.children[1]).unwrap();
        assert_eq!(r2.attributes.get_bool(&AttributeKey::Subscript), Some(true));
    }

    #[test]
    fn roundtrip_comments() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create paragraph with comment range
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // CommentStart
        let cs_id = doc.next_id();
        let mut cs = Node::new(cs_id, NodeType::CommentStart);
        cs.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("0".into()));
        doc.insert_node(para_id, 0, cs).unwrap();

        // Run
        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Commented text"))
            .unwrap();

        // CommentEnd
        let ce_id = doc.next_id();
        let mut ce = Node::new(ce_id, NodeType::CommentEnd);
        ce.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("0".into()));
        doc.insert_node(para_id, 2, ce).unwrap();

        // Create CommentBody node
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).unwrap().children.len();
        let cb_id = doc.next_id();
        let mut cb = Node::new(cb_id, NodeType::CommentBody);
        cb.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("0".into()));
        cb.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String("Tester".into()),
        );
        doc.insert_node(root_id, root_children, cb).unwrap();

        let cp_id = doc.next_id();
        doc.insert_node(cb_id, 0, Node::new(cp_id, NodeType::Paragraph))
            .unwrap();
        let cr_id = doc.next_id();
        doc.insert_node(cp_id, 0, Node::new(cr_id, NodeType::Run))
            .unwrap();
        let ct_id = doc.next_id();
        doc.insert_node(cr_id, 0, Node::text(ct_id, "My comment"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify comment ranges in body
        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();

        // Should have CommentStart, Run, CommentEnd (+ possibly commentReference run)
        assert!(para2.children.len() >= 3);

        let cs2 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(cs2.node_type, NodeType::CommentStart);
        assert_eq!(
            cs2.attributes.get_string(&AttributeKey::CommentId),
            Some("0")
        );

        // Verify CommentBody was round-tripped
        let root2 = doc2.node(doc2.root_id()).unwrap();
        let comment_bodies: Vec<_> = root2
            .children
            .iter()
            .filter(|&&id| {
                doc2.node(id)
                    .is_some_and(|n| n.node_type == NodeType::CommentBody)
            })
            .collect();
        assert_eq!(comment_bodies.len(), 1);

        let cb2 = doc2.node(*comment_bodies[0]).unwrap();
        assert_eq!(
            cb2.attributes.get_string(&AttributeKey::CommentAuthor),
            Some("Tester")
        );
    }
}
