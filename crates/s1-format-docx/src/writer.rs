//! DOCX writer — main entry point.
//!
//! Writes a [`DocumentModel`] as a DOCX file (ZIP archive).

use std::io::{Cursor, Write};

use s1_model::DocumentModel;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::comments_writer::write_comments_xml;
use crate::content_writer::{write_document_xml, HyperlinkRelEntry, ImageRelEntry};
use crate::endnotes_writer::write_endnotes_xml;
use crate::error::DocxError;
use crate::footnotes_writer::write_footnotes_xml;
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
// OOXML constraint validation (ECMA-376):
// - Empty table cells: handled in content_writer::write_table_cell (inserts <w:p/>)
// - Empty runs: handled in content_writer::write_run (skips empty non-revision runs)
// - Required pgSz/pgMar: handled in write_document_xml_with_sections (injects default sectPr)
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
    let footnotes_xml = write_footnotes_xml(doc);
    let has_footnotes = footnotes_xml.is_some();
    let endnotes_xml = write_endnotes_xml(doc);
    let has_endnotes = endnotes_xml.is_some();

    // Generate header/footer XML files and collect relationship info
    let mut hf_parts: Vec<HfPartEntry> = Vec::new();
    let mut hf_image_rels: Vec<ImageRelEntry> = Vec::new();
    let mut hf_counter = 0u32;

    for section in doc.sections() {
        for hf_ref in section.headers.iter().chain(section.footers.iter()) {
            // Validate that the referenced node exists in the document
            if doc.node(hf_ref.node_id).is_none() {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[s1-format-docx] Warning: header/footer node {:?} referenced in section but not found in document",
                    hf_ref.node_id
                );
                continue; // skip invalid reference
            }
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
            has_footnotes,
            has_endnotes,
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
            has_footnotes,
            has_endnotes,
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
        let mut written_media_paths: std::collections::HashSet<String> =
            std::collections::HashSet::new();
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

    // word/footnotes.xml (optional)
    if let Some(ref fxml) = footnotes_xml {
        zip.start_file("word/footnotes.xml", options)?;
        zip.write_all(fxml.as_bytes())?;
    }

    // word/endnotes.xml (optional)
    if let Some(ref exml) = endnotes_xml {
        zip.start_file("word/endnotes.xml", options)?;
        zip.write_all(exml.as_bytes())?;
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

    let (mut xml, image_rels, hyperlink_rels) = write_document_xml(doc);

    // OOXML constraint (ECMA-376 §17.6): every document body should have a
    // sectPr with pgSz and pgMar.  When the model has no explicit sections we
    // inject a default sectPr (US Letter, 1-inch margins) before </w:body>.
    if doc.sections().is_empty() {
        let default_sect = s1_model::SectionProperties::default();
        let sect_inner = crate::section_writer::write_section_properties(&default_sect, &[]);
        // Inject just before closing </w:body>
        if let Some(pos) = xml.rfind("</w:body>") {
            let sect_xml = format!("<w:sectPr>{sect_inner}</w:sectPr>");
            xml.insert_str(pos, &sect_xml);
        }
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
            NodeType::FootnoteRef => {
                if let Some(fid) = child.attributes.get_string(&AttributeKey::FootnoteNumber) {
                    xml.push_str(&format!(
                        r#"<w:r><w:rPr><w:rStyle w:val="FootnoteReference"/><w:vertAlign w:val="superscript"/></w:rPr><w:footnoteReference w:id="{}"/></w:r>"#,
                        crate::xml_writer::escape_xml(fid)
                    ));
                }
                i += 1;
            }
            NodeType::EndnoteRef => {
                if let Some(eid) = child.attributes.get_string(&AttributeKey::EndnoteNumber) {
                    xml.push_str(&format!(
                        r#"<w:r><w:rPr><w:rStyle w:val="EndnoteReference"/><w:vertAlign w:val="superscript"/></w:rPr><w:endnoteReference w:id="{}"/></w:r>"#,
                        crate::xml_writer::escape_xml(eid)
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
#[allow(clippy::too_many_arguments)]
fn content_types_xml(
    has_styles: bool,
    has_core: bool,
    has_numbering: bool,
    has_comments: bool,
    has_footnotes: bool,
    has_endnotes: bool,
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

    if has_footnotes {
        xml.push_str(
            r#"
  <Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>"#,
        );
    }

    if has_endnotes {
        xml.push_str(
            r#"
  <Override PartName="/word/endnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml"/>"#,
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
#[allow(clippy::too_many_arguments)]
fn document_rels_xml(
    has_styles: bool,
    has_numbering: bool,
    has_comments: bool,
    has_footnotes: bool,
    has_endnotes: bool,
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

    if has_footnotes {
        xml.push_str(
            r#"
  <Relationship Id="rIdFootnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>"#,
        );
    }

    if has_endnotes {
        xml.push_str(
            r#"
  <Relationship Id="rIdEndnotes" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes" Target="endnotes.xml"/>"#,
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
    fn roundtrip_paragraph_style_id() {
        // Verify that a paragraph with a StyleId attribute survives write → read
        use s1_model::{AttributeKey, AttributeValue, Node, NodeType};
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add a Title style definition
        let title_style = Style::new("Title", "Title", StyleType::Paragraph);
        doc.set_style(title_style);

        // Create paragraph with StyleId = "Title"
        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::StyleId,
            AttributeValue::String("Title".into()),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "My Title"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify the paragraph still has StyleId = "Title"
        let body2 = doc2.node(doc2.root_id()).unwrap().children[0];
        let para2 = doc2.node(body2).unwrap().children[0];
        let para_node = doc2.node(para2).unwrap();
        assert_eq!(
            para_node.attributes.get_string(&AttributeKey::StyleId),
            Some("Title"),
            "paragraph StyleId should survive round-trip"
        );
        // Verify the style definition survived
        assert!(
            doc2.style_by_id("Title").is_some(),
            "Title style definition should survive"
        );
    }

    #[test]
    fn roundtrip_multiple_paragraph_styles() {
        // Multiple paragraphs with different non-heading styles
        use s1_model::{AttributeKey, AttributeValue, Node, NodeType};
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Add style definitions
        for (id, name) in [
            ("Title", "Title"),
            ("Subtitle", "Subtitle"),
            ("Quote", "Quote"),
        ] {
            doc.set_style(Style::new(id, name, StyleType::Paragraph));
        }

        // Create paragraphs with different styles
        for (i, (style_id, text)) in [
            ("Title", "Doc Title"),
            ("Subtitle", "A subtitle"),
            ("Quote", "A famous quote"),
        ]
        .iter()
        .enumerate()
        {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::StyleId,
                AttributeValue::String(style_id.to_string()),
            );
            doc.insert_node(body_id, i, para).unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, *text))
                .unwrap();
        }

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.root_id()).unwrap().children[0];
        let body_node = doc2.node(body2).unwrap();
        assert!(body_node.children.len() >= 3, "should have 3 paragraphs");

        let expected_styles = ["Title", "Subtitle", "Quote"];
        for (i, expected) in expected_styles.iter().enumerate() {
            let para_node = doc2.node(body_node.children[i]).unwrap();
            assert_eq!(
                para_node.attributes.get_string(&AttributeKey::StyleId),
                Some(*expected),
                "paragraph {i} should have style '{expected}'"
            );
        }
    }

    #[test]
    fn roundtrip_style_with_inheritance() {
        // Style with basedOn chain should preserve the parent reference
        use s1_model::{AttributeKey, Node, NodeType};
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();

        // Normal → Heading1 (basedOn Normal)
        let normal = Style::new("Normal", "Normal", StyleType::Paragraph);
        doc.set_style(normal);

        let mut h1 = Style::new("Heading1", "Heading 1", StyleType::Paragraph);
        h1.parent_id = Some("Normal".to_string());
        h1.attributes = AttributeMap::new().bold(true).font_size(24.0);
        doc.set_style(h1);

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Test"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let s = doc2.style_by_id("Heading1").unwrap();
        assert_eq!(
            s.parent_id.as_deref(),
            Some("Normal"),
            "style basedOn should survive round-trip"
        );
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

    #[test]
    fn roundtrip_ins() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Insert".into()),
        );
        run.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(1));
        run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Alice".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2024-06-01T00:00:00Z".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "inserted text"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        assert!(!para2.children.is_empty());

        let run2 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionType),
            Some("Insert")
        );
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Alice")
        );
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionDate),
            Some("2024-06-01T00:00:00Z")
        );
        assert_eq!(run2.attributes.get_i64(&AttributeKey::RevisionId), Some(1));
        assert_eq!(doc2.to_plain_text(), "inserted text");
    }

    #[test]
    fn roundtrip_del() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Delete".into()),
        );
        run.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(2));
        run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Bob".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "deleted text"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        assert!(!para2.children.is_empty());

        let run2 = doc2.node(para2.children[0]).unwrap();
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionType),
            Some("Delete")
        );
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Bob")
        );
        assert_eq!(doc2.to_plain_text(), "deleted text");
    }

    #[test]
    fn roundtrip_mixed_tracked() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Normal run
        let r1_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(r1_id, NodeType::Run))
            .unwrap();
        let t1_id = doc.next_id();
        doc.insert_node(r1_id, 0, Node::text(t1_id, "normal "))
            .unwrap();

        // Insert run
        let r2_id = doc.next_id();
        let mut r2 = Node::new(r2_id, NodeType::Run);
        r2.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Insert".into()),
        );
        r2.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(10));
        r2.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Carl".into()),
        );
        doc.insert_node(para_id, 1, r2).unwrap();
        let t2_id = doc.next_id();
        doc.insert_node(r2_id, 0, Node::text(t2_id, "added"))
            .unwrap();

        // Delete run
        let r3_id = doc.next_id();
        let mut r3 = Node::new(r3_id, NodeType::Run);
        r3.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("Delete".into()),
        );
        r3.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(11));
        r3.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Dana".into()),
        );
        doc.insert_node(para_id, 2, r3).unwrap();
        let t3_id = doc.next_id();
        doc.insert_node(r3_id, 0, Node::text(t3_id, " removed"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let para2 = doc2.node(body2.children[0]).unwrap();
        assert_eq!(para2.children.len(), 3);

        // Normal run
        let r1_2 = doc2.node(para2.children[0]).unwrap();
        assert!(r1_2
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_none());

        // Insert run
        let r2_2 = doc2.node(para2.children[1]).unwrap();
        assert_eq!(
            r2_2.attributes.get_string(&AttributeKey::RevisionType),
            Some("Insert")
        );

        // Delete run
        let r3_2 = doc2.node(para2.children[2]).unwrap();
        assert_eq!(
            r3_2.attributes.get_string(&AttributeKey::RevisionType),
            Some("Delete")
        );

        assert_eq!(doc2.to_plain_text(), "normal added removed");
    }

    /// DOCX-06: Round-trip test for nested tables (table inside a table cell).
    #[test]
    fn roundtrip_nested_table() {
        use s1_model::{AttributeKey, AttributeValue, Node, NodeType, TableWidth};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create outer table: 1 row, 1 cell
        let outer_table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(outer_table_id, NodeType::Table))
            .unwrap();
        let outer_row_id = doc.next_id();
        doc.insert_node(
            outer_table_id,
            0,
            Node::new(outer_row_id, NodeType::TableRow),
        )
        .unwrap();
        let outer_cell_id = doc.next_id();
        let mut outer_cell = Node::new(outer_cell_id, NodeType::TableCell);
        outer_cell.attributes.set(
            AttributeKey::CellWidth,
            AttributeValue::TableWidth(TableWidth::Fixed(300.0)),
        );
        doc.insert_node(outer_row_id, 0, outer_cell).unwrap();

        // Add a paragraph before the inner table (OOXML requires at least one <w:p>)
        let pre_para_id = doc.next_id();
        doc.insert_node(
            outer_cell_id,
            0,
            Node::new(pre_para_id, NodeType::Paragraph),
        )
        .unwrap();
        let pre_run_id = doc.next_id();
        doc.insert_node(pre_para_id, 0, Node::new(pre_run_id, NodeType::Run))
            .unwrap();
        let pre_text_id = doc.next_id();
        doc.insert_node(pre_run_id, 0, Node::text(pre_text_id, "Outer cell"))
            .unwrap();

        // Create inner (nested) table: 1 row, 1 cell with text
        let inner_table_id = doc.next_id();
        doc.insert_node(outer_cell_id, 1, Node::new(inner_table_id, NodeType::Table))
            .unwrap();
        let inner_row_id = doc.next_id();
        doc.insert_node(
            inner_table_id,
            0,
            Node::new(inner_row_id, NodeType::TableRow),
        )
        .unwrap();
        let inner_cell_id = doc.next_id();
        doc.insert_node(
            inner_row_id,
            0,
            Node::new(inner_cell_id, NodeType::TableCell),
        )
        .unwrap();

        let inner_para_id = doc.next_id();
        doc.insert_node(
            inner_cell_id,
            0,
            Node::new(inner_para_id, NodeType::Paragraph),
        )
        .unwrap();
        let inner_run_id = doc.next_id();
        doc.insert_node(inner_para_id, 0, Node::new(inner_run_id, NodeType::Run))
            .unwrap();
        let inner_text_id = doc.next_id();
        doc.insert_node(inner_run_id, 0, Node::text(inner_text_id, "Inner cell"))
            .unwrap();

        // Write and read back
        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify structure: body > table > row > cell > [paragraph, table]
        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        assert!(!body2.children.is_empty(), "body should have children");

        let outer_tbl = doc2.node(body2.children[0]).unwrap();
        assert_eq!(outer_tbl.node_type, NodeType::Table);

        let outer_row = doc2.node(outer_tbl.children[0]).unwrap();
        assert_eq!(outer_row.node_type, NodeType::TableRow);

        let outer_cell2 = doc2.node(outer_row.children[0]).unwrap();
        assert_eq!(outer_cell2.node_type, NodeType::TableCell);
        assert!(
            outer_cell2.children.len() >= 2,
            "outer cell should have at least 2 children (paragraph + nested table), got {}",
            outer_cell2.children.len()
        );

        // Find the nested table
        let nested_tbl = outer_cell2
            .children
            .iter()
            .find(|&&id| {
                doc2.node(id)
                    .map(|n| n.node_type == NodeType::Table)
                    .unwrap_or(false)
            })
            .expect("should find nested table in outer cell");
        let nested_tbl_node = doc2.node(*nested_tbl).unwrap();
        assert_eq!(nested_tbl_node.node_type, NodeType::Table);

        // Verify nested table has content
        let nested_row = doc2.node(nested_tbl_node.children[0]).unwrap();
        assert_eq!(nested_row.node_type, NodeType::TableRow);
        let nested_cell = doc2.node(nested_row.children[0]).unwrap();
        assert_eq!(nested_cell.node_type, NodeType::TableCell);

        // Verify text survived
        assert!(doc2.to_plain_text().contains("Outer cell"));
        assert!(doc2.to_plain_text().contains("Inner cell"));
    }

    /// DOCX-06: Round-trip test for mixed bullet/numbered list paragraphs.
    #[test]
    fn roundtrip_mixed_list() {
        use s1_model::{
            AbstractNumbering, AttributeKey, AttributeValue, ListFormat, ListInfo, Node, NodeType,
            NumberingInstance, NumberingLevel,
        };

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Set up numbering definitions: one bullet, one decimal
        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
            abstract_num_id: 0,
            name: None,
            levels: vec![NumberingLevel {
                level: 0,
                num_format: ListFormat::Bullet,
                level_text: "\u{F0B7}".into(),
                start: 1,
                indent_left: Some(36.0),
                indent_hanging: Some(18.0),
                alignment: Some(s1_model::Alignment::Left),
                bullet_font: Some("Symbol".into()),
            }],
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 1,
            abstract_num_id: 0,
            level_overrides: vec![],
        });

        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
            abstract_num_id: 1,
            name: None,
            levels: vec![NumberingLevel {
                level: 0,
                num_format: ListFormat::Decimal,
                level_text: "%1.".into(),
                start: 1,
                indent_left: Some(36.0),
                indent_hanging: Some(18.0),
                alignment: Some(s1_model::Alignment::Left),
                bullet_font: None,
            }],
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 2,
            abstract_num_id: 1,
            level_overrides: vec![],
        });

        // Paragraph 0: bullet list item
        let p0_id = doc.next_id();
        let mut p0 = Node::new(p0_id, NodeType::Paragraph);
        p0.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 0,
                num_format: ListFormat::Bullet,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 0, p0).unwrap();
        let r0_id = doc.next_id();
        doc.insert_node(p0_id, 0, Node::new(r0_id, NodeType::Run))
            .unwrap();
        let t0_id = doc.next_id();
        doc.insert_node(r0_id, 0, Node::text(t0_id, "Bullet item"))
            .unwrap();

        // Paragraph 1: numbered list item
        let p1_id = doc.next_id();
        let mut p1 = Node::new(p1_id, NodeType::Paragraph);
        p1.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 0,
                num_format: ListFormat::Decimal,
                num_id: 2,
                start: None,
            }),
        );
        doc.insert_node(body_id, 1, p1).unwrap();
        let r1_id = doc.next_id();
        doc.insert_node(p1_id, 0, Node::new(r1_id, NodeType::Run))
            .unwrap();
        let t1_id = doc.next_id();
        doc.insert_node(r1_id, 0, Node::text(t1_id, "Numbered item"))
            .unwrap();

        // Paragraph 2: another bullet
        let p2_id = doc.next_id();
        let mut p2 = Node::new(p2_id, NodeType::Paragraph);
        p2.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 0,
                num_format: ListFormat::Bullet,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 2, p2).unwrap();
        let r2_id = doc.next_id();
        doc.insert_node(p2_id, 0, Node::new(r2_id, NodeType::Run))
            .unwrap();
        let t2_id = doc.next_id();
        doc.insert_node(r2_id, 0, Node::text(t2_id, "Second bullet"))
            .unwrap();

        // Write and read back
        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        assert!(body2.children.len() >= 3, "should have 3 paragraphs");

        // Verify list info on each paragraph
        let para0 = doc2.node(body2.children[0]).unwrap();
        let li0 = para0
            .attributes
            .get_list_info(&AttributeKey::ListInfo)
            .expect("paragraph 0 should have ListInfo");
        assert_eq!(li0.num_format, ListFormat::Bullet);
        assert_eq!(li0.num_id, 1);

        let para1 = doc2.node(body2.children[1]).unwrap();
        let li1 = para1
            .attributes
            .get_list_info(&AttributeKey::ListInfo)
            .expect("paragraph 1 should have ListInfo");
        assert_eq!(li1.num_format, ListFormat::Decimal);
        assert_eq!(li1.num_id, 2);

        let para2 = doc2.node(body2.children[2]).unwrap();
        let li2 = para2
            .attributes
            .get_list_info(&AttributeKey::ListInfo)
            .expect("paragraph 2 should have ListInfo");
        assert_eq!(li2.num_format, ListFormat::Bullet);
        assert_eq!(li2.num_id, 1);

        // Verify text content
        let text = doc2.to_plain_text();
        assert!(text.contains("Bullet item"));
        assert!(text.contains("Numbered item"));
        assert!(text.contains("Second bullet"));
    }

    /// DOCX-06: Round-trip test for multiple sections with different page sizes.
    #[test]
    fn roundtrip_multiple_sections() {
        use s1_model::{Node, NodeType, PageOrientation, SectionProperties};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Section 1: US Letter portrait (default)
        let mut sect1 = SectionProperties::default();
        sect1.page_width = 612.0; // 8.5" in points
        sect1.page_height = 792.0; // 11" in points

        // Section 2: A4 landscape
        let mut sect2 = SectionProperties::default();
        sect2.page_width = 841.89; // A4 landscape width (297mm)
        sect2.page_height = 595.28; // A4 landscape height (210mm)
        sect2.orientation = PageOrientation::Landscape;
        sect2.margin_top = 54.0; // 0.75 inch margins
        sect2.margin_bottom = 54.0;

        doc.sections_mut().push(sect1);
        doc.sections_mut().push(sect2);

        // Paragraph in first section (marked with SectionIndex 0)
        let p0_id = doc.next_id();
        let mut p0 = Node::new(p0_id, NodeType::Paragraph);
        p0.attributes.set(
            s1_model::AttributeKey::SectionIndex,
            s1_model::AttributeValue::Int(0),
        );
        doc.insert_node(body_id, 0, p0).unwrap();
        let r0_id = doc.next_id();
        doc.insert_node(p0_id, 0, Node::new(r0_id, NodeType::Run))
            .unwrap();
        let t0_id = doc.next_id();
        doc.insert_node(r0_id, 0, Node::text(t0_id, "Section 1 content"))
            .unwrap();

        // Paragraph in second section
        let p1_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p1_id, NodeType::Paragraph))
            .unwrap();
        let r1_id = doc.next_id();
        doc.insert_node(p1_id, 0, Node::new(r1_id, NodeType::Run))
            .unwrap();
        let t1_id = doc.next_id();
        doc.insert_node(r1_id, 0, Node::text(t1_id, "Section 2 content"))
            .unwrap();

        // Write and read back
        let bytes = write(&doc).unwrap();
        let doc2 = crate::read(&bytes).unwrap();

        // Verify sections
        let sections = doc2.sections();
        assert!(
            sections.len() >= 2,
            "should have at least 2 sections, got {}",
            sections.len()
        );

        // Section 1: US Letter portrait
        let s1 = &sections[0];
        assert!(
            (s1.page_width - 612.0).abs() < 1.0,
            "section 1 width: {}",
            s1.page_width
        );
        assert!(
            (s1.page_height - 792.0).abs() < 1.0,
            "section 1 height: {}",
            s1.page_height
        );

        // Section 2: A4 landscape
        let s2 = &sections[sections.len() - 1];
        assert!(
            (s2.page_width - 841.89).abs() < 2.0,
            "section 2 width: {}",
            s2.page_width
        );
        assert!(
            (s2.page_height - 595.28).abs() < 2.0,
            "section 2 height: {}",
            s2.page_height
        );
        assert_eq!(s2.orientation, PageOrientation::Landscape);

        // Verify text content
        let text = doc2.to_plain_text();
        assert!(text.contains("Section 1 content"));
        assert!(text.contains("Section 2 content"));
    }

    /// DOCX-11: Verify that header/footer references to non-existent nodes
    /// are silently skipped (not written as invalid references).
    #[test]
    fn write_skips_invalid_header_footer_node_refs() {
        use s1_model::{
            HeaderFooterRef, HeaderFooterType, Node, NodeId, NodeType, SectionProperties,
        };

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create a section that references a header NodeId that does NOT exist
        let fake_header_id = NodeId::new(99, 999);
        let mut sect = SectionProperties::default();
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: fake_header_id,
        });
        doc.sections_mut().push(sect);

        // Add a paragraph so the doc isn't empty
        let p_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p_id, NodeType::Paragraph))
            .unwrap();
        let r_id = doc.next_id();
        doc.insert_node(p_id, 0, Node::new(r_id, NodeType::Run))
            .unwrap();
        let t_id = doc.next_id();
        doc.insert_node(r_id, 0, Node::text(t_id, "Content"))
            .unwrap();

        // Writing should succeed (not panic or error) — the invalid ref is skipped
        let bytes = write(&doc).unwrap();

        // Reading back should also succeed
        let doc2 = crate::read(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Content");

        // The section should not have header references (the invalid one was skipped)
        // Note: section may or may not be preserved depending on whether it was the
        // only section, but we should not crash or produce invalid XML.
    }

    // -----------------------------------------------------------------------
    // OOXML constraint validation tests (DOCX-10)
    // -----------------------------------------------------------------------

    #[test]
    fn ooxml_empty_table_cell_gets_default_paragraph() {
        // ECMA-376 §17.4.17: every <w:tc> must contain at least one <w:p>.
        // Build a table with a cell that has zero children and verify the
        // written XML contains a <w:p/> inside that cell.
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let tbl_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        // Cell with NO children at all
        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
            .unwrap();

        let bytes = write(&doc).unwrap();

        // Read the document.xml from the ZIP and check for <w:p/> inside <w:tc>
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc_xml = String::new();
        {
            use std::io::Read as _;
            let mut f = archive.by_name("word/document.xml").unwrap();
            f.read_to_string(&mut doc_xml).unwrap();
        }

        // The cell must contain at least one paragraph element
        assert!(
            doc_xml.contains("<w:tc><w:p/></w:tc>")
                || doc_xml.contains("<w:tc><w:tcPr>") && doc_xml.contains("<w:p/></w:tc>"),
            "Empty table cell should get a default <w:p/> — got: {}",
            &doc_xml[doc_xml.find("<w:tc>").unwrap_or(0)
                ..doc_xml
                    .find("</w:tc>")
                    .map(|p| p + 7)
                    .unwrap_or(doc_xml.len())]
        );

        // Round-trip should succeed
        let doc2 = crate::read(&bytes).unwrap();
        let body2 = doc2.node(doc2.body_id().unwrap()).unwrap();
        let tbl2 = doc2.node(body2.children[0]).unwrap();
        assert_eq!(tbl2.node_type, NodeType::Table);
    }

    #[test]
    fn ooxml_empty_runs_are_omitted() {
        // Empty <w:r></w:r> elements (no text, no special content) are wasteful
        // and should be omitted from the output.
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Run 1: empty (no children) — should be omitted
        let empty_run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(empty_run_id, NodeType::Run))
            .unwrap();

        // Run 2: has text content — should be kept
        let good_run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(good_run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(good_run_id, 0, Node::text(text_id, "Hello"))
            .unwrap();

        let bytes = write(&doc).unwrap();

        // Read the document.xml from the ZIP
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc_xml = String::new();
        {
            use std::io::Read as _;
            let mut f = archive.by_name("word/document.xml").unwrap();
            f.read_to_string(&mut doc_xml).unwrap();
        }

        // Count <w:r> occurrences — should be exactly 1 (the non-empty run)
        let run_count = doc_xml.matches("<w:r>").count();
        assert_eq!(
            run_count, 1,
            "Expected exactly 1 <w:r> (empty run should be omitted), got {run_count} in: {doc_xml}"
        );

        // Verify the text survived
        assert!(doc_xml.contains("Hello"));
    }

    #[test]
    fn ooxml_empty_run_with_revision_is_preserved() {
        // Runs with revision tracking attributes should NOT be omitted even
        // if they have no text children.
        use s1_model::{AttributeKey, AttributeValue};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Empty run with FormatChange revision — should be preserved
        let rev_run_id = doc.next_id();
        let mut rev_run = Node::new(rev_run_id, NodeType::Run);
        rev_run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("FormatChange".into()),
        );
        rev_run
            .attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(42));
        rev_run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("test".into()),
        );
        doc.insert_node(para_id, 0, rev_run).unwrap();

        let bytes = write(&doc).unwrap();

        // Read the document.xml
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc_xml = String::new();
        {
            use std::io::Read as _;
            let mut f = archive.by_name("word/document.xml").unwrap();
            f.read_to_string(&mut doc_xml).unwrap();
        }

        // The revision run should still be present even though it has no text
        assert!(
            doc_xml.contains("<w:r>"),
            "Revision run should be preserved even without text: {doc_xml}"
        );
        assert!(
            doc_xml.contains("rPrChange"),
            "FormatChange should emit rPrChange: {doc_xml}"
        );
    }

    #[test]
    fn ooxml_default_section_when_no_sections_defined() {
        // ECMA-376 §17.6: the document body should have sectPr with pgSz
        // and pgMar.  When no sections are defined in the model, the writer
        // should inject a default sectPr (US Letter, 1-inch margins).
        let doc = make_simple_doc("Minimal doc");
        assert!(
            doc.sections().is_empty(),
            "precondition: no sections in model"
        );

        let bytes = write(&doc).unwrap();

        // Read the raw document.xml
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc_xml = String::new();
        {
            use std::io::Read as _;
            let mut f = archive.by_name("word/document.xml").unwrap();
            f.read_to_string(&mut doc_xml).unwrap();
        }

        // Must contain sectPr with pgSz and pgMar
        assert!(
            doc_xml.contains("<w:sectPr>"),
            "document.xml should have <w:sectPr>: {doc_xml}"
        );
        assert!(
            doc_xml.contains("<w:pgSz"),
            "sectPr should contain <w:pgSz>: {doc_xml}"
        );
        assert!(
            doc_xml.contains("<w:pgMar"),
            "sectPr should contain <w:pgMar>: {doc_xml}"
        );

        // Verify default US Letter dimensions in twips (612pt = 12240, 792pt = 15840)
        assert!(
            doc_xml.contains(r#"w:w="12240""#),
            "pgSz width should be 12240 twips (US Letter): {doc_xml}"
        );
        assert!(
            doc_xml.contains(r#"w:h="15840""#),
            "pgSz height should be 15840 twips (US Letter): {doc_xml}"
        );

        // Verify 1-inch margins (72pt = 1440 twips)
        assert!(
            doc_xml.contains(r#"w:top="1440""#),
            "pgMar top should be 1440 twips (1 inch): {doc_xml}"
        );

        // Round-trip should succeed and now produce a section
        let doc2 = crate::read(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Minimal doc");
        // The reader should pick up the section from the injected sectPr
        assert!(
            !doc2.sections().is_empty(),
            "round-tripped doc should have a section from injected sectPr"
        );
    }

    #[test]
    fn ooxml_explicit_section_still_works() {
        // When explicit sections exist, no extra default sectPr should be injected.
        use s1_model::SectionProperties;

        let mut doc = make_simple_doc("With section");
        let mut sect = SectionProperties::default();
        sect.page_width = 595.0; // A4-ish
        sect.page_height = 842.0;
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();

        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc_xml = String::new();
        {
            use std::io::Read as _;
            let mut f = archive.by_name("word/document.xml").unwrap();
            f.read_to_string(&mut doc_xml).unwrap();
        }

        // Should have exactly one sectPr with the custom dimensions
        let sect_count = doc_xml.matches("<w:sectPr>").count();
        assert_eq!(
            sect_count, 1,
            "should have exactly 1 sectPr, got {sect_count}"
        );
        // A4 width: 595pt * 20 = 11900 twips
        assert!(
            doc_xml.contains(r#"w:w="11900""#),
            "should use custom page width: {doc_xml}"
        );
    }
}
