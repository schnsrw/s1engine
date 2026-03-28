//! Write `<office:text>` body content for ODF `content.xml`.

use std::collections::HashMap;

use s1_model::{
    AttributeKey, AttributeMap, AttributeValue, DocumentModel, FieldType, ListFormat, NodeType,
};

use crate::property_writer::{
    write_paragraph_properties, write_table_cell_properties, write_text_properties,
};
use crate::xml_util::{escape_xml, points_to_cm};

/// An auto-style definition collected during writing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AutoStyleKey {
    text_props: String,
    para_props: String,
    cell_props: String,
    family: String,
}

/// Image entry discovered during writing.
pub struct ImageEntry {
    pub href: String,
    pub media_id: s1_model::MediaId,
}

/// Write the full `content.xml` from a `DocumentModel`.
///
/// Returns `(xml_string, image_entries)`.
pub fn write_content_xml(doc: &DocumentModel) -> (String, Vec<ImageEntry>) {
    let mut auto_styles: HashMap<AutoStyleKey, String> = HashMap::new();
    let mut auto_counter = 0u32;
    let mut images: Vec<ImageEntry> = Vec::new();
    let mut needs_bullet_list_style = false;
    let mut needs_number_list_style = false;

    // First pass: collect body XML and auto-styles
    let body_xml = write_body(
        doc,
        &mut auto_styles,
        &mut auto_counter,
        &mut images,
        &mut needs_bullet_list_style,
        &mut needs_number_list_style,
    );

    // Build the full content.xml
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/" office:version="1.2">"#,
    );

    // Write automatic styles (including list styles)
    let has_auto_styles =
        !auto_styles.is_empty() || needs_bullet_list_style || needs_number_list_style;
    if has_auto_styles {
        xml.push_str("<office:automatic-styles>");
        // Sort for deterministic output
        let mut sorted: Vec<_> = auto_styles.iter().collect();
        sorted.sort_by(|(_, a), (_, b)| a.cmp(b));
        for (key, name) in sorted {
            xml.push_str(&format!(
                r#"<style:style style:name="{}" style:family="{}""#,
                name, key.family
            ));
            // Check if there's a parent style reference
            xml.push('>');
            if !key.para_props.is_empty() {
                xml.push_str(&key.para_props);
            }
            if !key.text_props.is_empty() {
                xml.push_str(&key.text_props);
            }
            if !key.cell_props.is_empty() {
                xml.push_str(&key.cell_props);
            }
            xml.push_str("</style:style>");
        }
        // Write bullet list style definition
        if needs_bullet_list_style {
            xml.push_str(r#"<text:list-style style:name="S1BulletList">"#);
            for level in 1..=10 {
                xml.push_str(&format!(
                    r#"<text:list-level-style-bullet text:level="{}" text:bullet-char="&#x2022;"><style:list-level-properties text:list-level-position-and-space-mode="label-alignment"><style:list-level-label-alignment text:label-followed-by="listtab" fo:margin-left="{}cm" fo:text-indent="-0.635cm"/></style:list-level-properties></text:list-level-style-bullet>"#,
                    level,
                    level as f64 * 0.635
                ));
            }
            xml.push_str("</text:list-style>");
        }
        // Write ordered list style definition
        if needs_number_list_style {
            xml.push_str(r#"<text:list-style style:name="S1NumberList">"#);
            for level in 1..=10 {
                xml.push_str(&format!(
                    r#"<text:list-level-style-number text:level="{}" style:num-format="1" text:start-value="1"><style:list-level-properties text:list-level-position-and-space-mode="label-alignment"><style:list-level-label-alignment text:label-followed-by="listtab" fo:margin-left="{}cm" fo:text-indent="-0.635cm"/></style:list-level-properties></text:list-level-style-number>"#,
                    level,
                    level as f64 * 0.635
                ));
            }
            xml.push_str("</text:list-style>");
        }
        xml.push_str("</office:automatic-styles>");
    }

    xml.push_str("<office:body><office:text>");
    xml.push_str(&body_xml);
    xml.push_str("</office:text></office:body></office:document-content>");

    (xml, images)
}

/// Write the body children (paragraphs, tables, etc.).
fn write_body(
    doc: &DocumentModel,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
    needs_bullet_list_style: &mut bool,
    needs_number_list_style: &mut bool,
) -> String {
    let mut xml = String::new();

    let body_id = match doc.body_id() {
        Some(id) => id,
        None => return xml,
    };

    let body = match doc.node(body_id) {
        Some(n) => n,
        None => return xml,
    };

    // Q12: Write back preserved tracked-changes raw XML if present
    if let Some(raw) = body.attributes.get_string(&AttributeKey::RawXml) {
        if raw.contains("tracked-changes") {
            xml.push_str(raw);
        }
    }

    // Track list nesting depth for reconstructing ODF nested lists.
    // The stack depth represents the current nesting level (0 = top-level list).
    // We track whether the current list-item at each level is still open (needs closing
    // before opening a sibling or before closing the enclosing list).
    let mut list_depth: usize = 0; // number of currently open <text:list> levels
    let mut list_item_open: Vec<bool> = Vec::new(); // per-level: is a <text:list-item> open?

    let mut table_counter = 0u32;

    for &child_id in &body.children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };

        match child.node_type {
            NodeType::Paragraph => {
                // Check if this paragraph is a list item
                let list_info = child.attributes.get(&AttributeKey::ListInfo);
                if let Some(AttributeValue::ListInfo(info)) = list_info {
                    let target_level = info.level as usize;
                    let target_depth = target_level + 1; // depth 1 = level 0

                    // If we need to go shallower, close levels
                    while list_depth > target_depth {
                        // Close list-item if open at this depth
                        if list_item_open.last().copied().unwrap_or(false) {
                            xml.push_str("</text:list-item>");
                        }
                        xml.push_str("</text:list>");
                        list_item_open.pop();
                        list_depth -= 1;
                    }

                    // At the same depth, close previous list-item (sibling)
                    if list_depth == target_depth && list_item_open.last().copied().unwrap_or(false)
                    {
                        xml.push_str("</text:list-item>");
                        if let Some(last) = list_item_open.last_mut() {
                            *last = false;
                        }
                    }

                    // Determine if this list item is ordered
                    let is_ordered = is_ordered_format(info.num_format);

                    // If we need to go deeper, open nested lists
                    while list_depth < target_depth {
                        // For deeper levels, we need the parent list-item to remain open.
                        // If we are at depth 0, open the top-level list. Otherwise open
                        // a nested <text:list> inside the current (still-open) list-item.
                        if list_depth == 0 {
                            // Top-level list: set style name and track format
                            if is_ordered {
                                *needs_number_list_style = true;
                                xml.push_str(
                                    r#"<text:list text:style-name="S1NumberList">"#,
                                );
                            } else {
                                *needs_bullet_list_style = true;
                                xml.push_str(
                                    r#"<text:list text:style-name="S1BulletList">"#,
                                );
                            }
                        } else {
                            xml.push_str("<text:list>");
                        }
                        list_depth += 1;
                        list_item_open.push(false);
                    }

                    // Open a new list-item at the target depth
                    xml.push_str("<text:list-item>");
                    if let Some(last) = list_item_open.last_mut() {
                        *last = true;
                    }

                    write_paragraph(doc, child_id, &mut xml, auto_styles, counter, images);
                    // Don't close list-item yet — next item might nest inside it
                } else {
                    // Close any open lists
                    close_list_stack(&mut list_depth, &mut list_item_open, &mut xml);

                    // Check if heading
                    let is_heading = child
                        .attributes
                        .get_string(&AttributeKey::StyleId)
                        .is_some_and(|s| s.starts_with("Heading"));

                    if is_heading {
                        let level = child
                            .attributes
                            .get_string(&AttributeKey::StyleId)
                            .and_then(|s| s.strip_prefix("Heading"))
                            .and_then(|l| l.parse::<u8>().ok())
                            .unwrap_or(1);
                        write_heading(doc, child_id, level, &mut xml, auto_styles, counter, images);
                    } else {
                        write_paragraph(doc, child_id, &mut xml, auto_styles, counter, images);
                    }
                }
            }
            NodeType::Table => {
                close_list_stack(&mut list_depth, &mut list_item_open, &mut xml);
                write_table(
                    doc,
                    child_id,
                    &mut xml,
                    auto_styles,
                    counter,
                    images,
                    &mut table_counter,
                );
            }
            NodeType::TableOfContents => {
                close_list_stack(&mut list_depth, &mut list_item_open, &mut xml);
                write_toc_odt(doc, child_id, &mut xml, auto_styles, counter, images);
            }
            NodeType::PageBreak => {
                close_list_stack(&mut list_depth, &mut list_item_open, &mut xml);
                // Emit a paragraph with break-before page style
                let style_name = get_or_create_auto_style(
                    auto_styles,
                    counter,
                    AutoStyleKey {
                        text_props: String::new(),
                        para_props: r#"<style:paragraph-properties fo:break-before="page"/>"#
                            .to_string(),
                        cell_props: String::new(),
                        family: "paragraph".to_string(),
                    },
                );
                xml.push_str(&format!(r#"<text:p text:style-name="{}"/>"#, style_name));
            }
            NodeType::ColumnBreak => {
                close_list_stack(&mut list_depth, &mut list_item_open, &mut xml);
                let style_name = get_or_create_auto_style(
                    auto_styles,
                    counter,
                    AutoStyleKey {
                        text_props: String::new(),
                        para_props: r#"<style:paragraph-properties fo:break-before="column"/>"#
                            .to_string(),
                        cell_props: String::new(),
                        family: "paragraph".to_string(),
                    },
                );
                xml.push_str(&format!(r#"<text:p text:style-name="{}"/>"#, style_name));
            }
            _ => {}
        }
    }

    close_list_stack(&mut list_depth, &mut list_item_open, &mut xml);
    xml
}

/// Close all open list levels.
fn close_list_stack(depth: &mut usize, item_open: &mut Vec<bool>, xml: &mut String) {
    while *depth > 0 {
        if item_open.last().copied().unwrap_or(false) {
            xml.push_str("</text:list-item>");
        }
        xml.push_str("</text:list>");
        item_open.pop();
        *depth -= 1;
    }
}

/// Return `true` if the list format represents an ordered (numbered) list.
fn is_ordered_format(fmt: ListFormat) -> bool {
    matches!(
        fmt,
        ListFormat::Decimal
            | ListFormat::LowerAlpha
            | ListFormat::UpperAlpha
            | ListFormat::LowerRoman
            | ListFormat::UpperRoman
    )
}

/// Write a paragraph as `<text:p>`.
fn write_paragraph(
    doc: &DocumentModel,
    para_id: s1_model::NodeId,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    let para = match doc.node(para_id) {
        Some(n) => n,
        None => return,
    };

    // Check if we need an auto-style for paragraph formatting
    let style_name = get_or_create_paragraph_auto_style(&para.attributes, auto_styles, counter);

    if let Some(ref name) = style_name {
        xml.push_str(&format!(r#"<text:p text:style-name="{name}">"#));
    } else if let Some(sid) = para.attributes.get_string(&AttributeKey::StyleId) {
        xml.push_str(&format!(
            r#"<text:p text:style-name="{}">"#,
            escape_xml(sid)
        ));
    } else {
        xml.push_str("<text:p>");
    }

    write_paragraph_children(doc, para_id, xml, auto_styles, counter, images);

    xml.push_str("</text:p>");
}

/// Write a heading as `<text:h>`.
fn write_heading(
    doc: &DocumentModel,
    para_id: s1_model::NodeId,
    level: u8,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    let para = match doc.node(para_id) {
        Some(n) => n,
        None => return,
    };

    let style_ref = para
        .attributes
        .get_string(&AttributeKey::StyleId)
        .unwrap_or("Heading1");

    xml.push_str(&format!(
        r#"<text:h text:style-name="{}" text:outline-level="{}">"#,
        escape_xml(style_ref),
        level,
    ));

    write_paragraph_children(doc, para_id, xml, auto_styles, counter, images);
    xml.push_str("</text:h>");
}

/// Write children of a paragraph (runs, breaks, fields, images).
fn write_paragraph_children(
    doc: &DocumentModel,
    para_id: s1_model::NodeId,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    let para = match doc.node(para_id) {
        Some(n) => n,
        None => return,
    };

    let mut current_hyperlink_url: Option<String> = None;

    for &child_id in &para.children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };

        match child.node_type {
            NodeType::Run => {
                let child_url = child
                    .attributes
                    .get_string(&AttributeKey::HyperlinkUrl)
                    .map(|s| s.to_string());

                // Close hyperlink if URL changed
                if current_hyperlink_url.is_some() && current_hyperlink_url != child_url {
                    xml.push_str("</text:a>");
                    current_hyperlink_url = None;
                }

                // Open hyperlink if needed
                if let Some(ref url) = child_url {
                    if current_hyperlink_url.is_none() {
                        xml.push_str(&format!(
                            r#"<text:a xlink:href="{}" xlink:type="simple">"#,
                            escape_xml(url)
                        ));
                        current_hyperlink_url = Some(url.clone());
                    }
                }

                write_run(doc, child_id, xml, auto_styles, counter);
            }
            NodeType::LineBreak => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                xml.push_str("<text:line-break/>");
            }
            NodeType::Tab => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                xml.push_str("<text:tab/>");
            }
            NodeType::Field => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                write_field(child, xml);
            }
            NodeType::Image => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                write_image(doc, child, xml, images);
            }
            NodeType::BookmarkStart => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                if let Some(name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                    xml.push_str(&format!(
                        r#"<text:bookmark-start text:name="{}"/>"#,
                        escape_xml(name)
                    ));
                }
            }
            NodeType::BookmarkEnd => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                if let Some(name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                    xml.push_str(&format!(
                        r#"<text:bookmark-end text:name="{}"/>"#,
                        escape_xml(name)
                    ));
                }
            }
            NodeType::CommentStart => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                if let Some(comment_id) = child.attributes.get_string(&AttributeKey::CommentId) {
                    write_annotation(doc, comment_id, xml);
                }
            }
            NodeType::CommentEnd => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                if let Some(comment_id) = child.attributes.get_string(&AttributeKey::CommentId) {
                    xml.push_str(&format!(
                        r#"<office:annotation-end office:name="{}"/>"#,
                        escape_xml(comment_id)
                    ));
                }
            }
            NodeType::FootnoteRef => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                write_footnote(doc, child, xml, auto_styles, counter, images);
            }
            NodeType::EndnoteRef => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                write_endnote(doc, child, xml, auto_styles, counter, images);
            }
            NodeType::PageBreak => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                // ODF uses a paragraph with page-break style
                // Inline page breaks inside paragraphs are not directly supported in ODF,
                // but we can insert a soft page break marker
            }
            NodeType::ColumnBreak => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                // Column breaks are not directly expressible inline in ODF
            }
            NodeType::Drawing => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                // Emit raw XML if stored, otherwise skip
                if let Some(raw) = child.attributes.get_string(&AttributeKey::ShapeRawXml) {
                    xml.push_str(raw);
                }
            }
            NodeType::Equation => {
                close_hyperlink(&mut current_hyperlink_url, xml);
                // Equations can be stored as raw source
                if let Some(eq_src) = child.attributes.get_string(&AttributeKey::EquationSource) {
                    xml.push_str("<text:span>[");
                    xml.push_str(&escape_xml(eq_src));
                    xml.push_str("]</text:span>");
                }
            }
            _ => {}
        }
    }

    // Close any remaining hyperlink
    close_hyperlink(&mut current_hyperlink_url, xml);
}

/// Close a `<text:a>` hyperlink element if one is open.
fn close_hyperlink(current_url: &mut Option<String>, xml: &mut String) {
    if current_url.is_some() {
        xml.push_str("</text:a>");
        *current_url = None;
    }
}

/// Write an `<office:annotation>` element for a comment.
///
/// Finds the CommentBody node by matching CommentId and writes its content.
fn write_annotation(doc: &DocumentModel, comment_id: &str, xml: &mut String) {
    // Find the CommentBody node with matching CommentId
    let root_id = doc.root_id();
    let root = match doc.node(root_id) {
        Some(n) => n,
        None => return,
    };

    let mut body_node = None;
    for &child_id in &root.children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        if child.node_type == NodeType::CommentBody
            && child.attributes.get_string(&AttributeKey::CommentId) == Some(comment_id)
        {
            body_node = Some(child);
            break;
        }
    }

    xml.push_str(&format!(
        r#"<office:annotation office:name="{}">"#,
        escape_xml(comment_id)
    ));

    if let Some(body) = body_node {
        if let Some(author) = body.attributes.get_string(&AttributeKey::CommentAuthor) {
            xml.push_str(&format!("<dc:creator>{}</dc:creator>", escape_xml(author)));
        }
        if let Some(date) = body.attributes.get_string(&AttributeKey::CommentDate) {
            xml.push_str(&format!("<dc:date>{}</dc:date>", escape_xml(date)));
        }

        // Write annotation body paragraphs
        for &para_id in &body.children {
            let para = match doc.node(para_id) {
                Some(n) if n.node_type == NodeType::Paragraph => n,
                _ => continue,
            };
            xml.push_str("<text:p>");
            for &run_id in &para.children {
                write_annotation_inline(doc, run_id, xml);
            }
            xml.push_str("</text:p>");
        }
    }

    xml.push_str("</office:annotation>");
}

/// Write inline content inside an annotation paragraph.
fn write_annotation_inline(doc: &DocumentModel, node_id: s1_model::NodeId, xml: &mut String) {
    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return,
    };
    match node.node_type {
        NodeType::Run => {
            for &child_id in &node.children {
                write_annotation_inline(doc, child_id, xml);
            }
        }
        NodeType::Text => {
            if let Some(text) = &node.text_content {
                xml.push_str(&escape_xml(text));
            }
        }
        _ => {}
    }
}

/// Write a footnote reference and its body as `<text:note>`.
fn write_footnote(
    doc: &DocumentModel,
    ref_node: &s1_model::Node,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    write_note(
        doc,
        ref_node,
        "footnote",
        NodeType::FootnoteBody,
        xml,
        auto_styles,
        counter,
        images,
    );
}

/// Write an endnote reference and its body as `<text:note>`.
fn write_endnote(
    doc: &DocumentModel,
    ref_node: &s1_model::Node,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    write_note(
        doc,
        ref_node,
        "endnote",
        NodeType::EndnoteBody,
        xml,
        auto_styles,
        counter,
        images,
    );
}

/// Write a footnote or endnote as `<text:note>`.
#[allow(clippy::too_many_arguments)]
fn write_note(
    doc: &DocumentModel,
    ref_node: &s1_model::Node,
    note_class: &str,
    body_type: NodeType,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    let note_num_key = if note_class == "footnote" {
        AttributeKey::FootnoteNumber
    } else {
        AttributeKey::EndnoteNumber
    };

    let note_number = ref_node
        .attributes
        .get(&note_num_key)
        .and_then(|v| {
            if let AttributeValue::Int(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .unwrap_or(1);

    // Find the matching body node under the document root
    let root_id = doc.root_id();
    let root = match doc.node(root_id) {
        Some(n) => n,
        None => return,
    };

    let mut body_node = None;
    for &child_id in &root.children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        if child.node_type == body_type {
            let child_num = child
                .attributes
                .get(&note_num_key)
                .and_then(|v| {
                    if let AttributeValue::Int(n) = v {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            if child_num == note_number {
                body_node = Some(child);
                break;
            }
        }
    }

    xml.push_str(&format!(r#"<text:note text:note-class="{}">"#, note_class));
    xml.push_str(&format!(
        "<text:note-citation>{}</text:note-citation>",
        note_number
    ));
    xml.push_str("<text:note-body>");

    if let Some(body) = body_node {
        for &para_id in &body.children {
            if let Some(para) = doc.node(para_id) {
                if para.node_type == NodeType::Paragraph {
                    write_paragraph(doc, para_id, xml, auto_styles, counter, images);
                }
            }
        }
    } else {
        // No body found — emit empty paragraph to be valid ODF
        xml.push_str("<text:p/>");
    }

    xml.push_str("</text:note-body>");
    xml.push_str("</text:note>");
}

/// Write a run as `<text:span>` (or bare text if no formatting).
fn write_run(
    doc: &DocumentModel,
    run_id: s1_model::NodeId,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
) {
    let run = match doc.node(run_id) {
        Some(n) => n,
        None => return,
    };

    let text_props = write_text_properties(&run.attributes);
    let has_formatting = !text_props.is_empty();

    if has_formatting {
        let key = AutoStyleKey {
            text_props: text_props.clone(),
            para_props: String::new(),
            cell_props: String::new(),
            family: "text".to_string(),
        };
        let name = auto_styles
            .entry(key)
            .or_insert_with(|| {
                *counter += 1;
                format!("T{}", *counter)
            })
            .clone();
        xml.push_str(&format!(r#"<text:span text:style-name="{name}">"#));
    }

    // Write run children (text nodes)
    for &child_id in &run.children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        match child.node_type {
            NodeType::Text => {
                if let Some(ref text) = child.text_content {
                    write_text_with_breaks(text, xml);
                }
            }
            NodeType::LineBreak => xml.push_str("<text:line-break/>"),
            NodeType::Tab => xml.push_str("<text:tab/>"),
            _ => {}
        }
    }

    if has_formatting {
        xml.push_str("</text:span>");
    }
}

/// Write a field node.
fn write_field(node: &s1_model::Node, xml: &mut String) {
    if let Some(AttributeValue::FieldType(ft)) = node.attributes.get(&AttributeKey::FieldType) {
        // Get optional display value from FieldCode
        let display_val = node
            .attributes
            .get_string(&AttributeKey::FieldCode)
            .unwrap_or_default();

        match ft {
            FieldType::PageNumber => {
                xml.push_str(r#"<text:page-number text:select-page="current"/>"#);
            }
            FieldType::PageCount => {
                xml.push_str(r#"<text:page-count text:select-page="current"/>"#);
            }
            FieldType::Date => {
                if display_val.is_empty() {
                    xml.push_str("<text:date/>");
                } else {
                    xml.push_str("<text:date>");
                    xml.push_str(&escape_xml(display_val));
                    xml.push_str("</text:date>");
                }
            }
            FieldType::Time => {
                if display_val.is_empty() {
                    xml.push_str("<text:time/>");
                } else {
                    xml.push_str("<text:time>");
                    xml.push_str(&escape_xml(display_val));
                    xml.push_str("</text:time>");
                }
            }
            FieldType::Author => {
                if display_val.is_empty() {
                    xml.push_str("<text:author-name/>");
                } else {
                    xml.push_str("<text:author-name>");
                    xml.push_str(&escape_xml(display_val));
                    xml.push_str("</text:author-name>");
                }
            }
            FieldType::FileName => {
                if display_val.is_empty() {
                    xml.push_str("<text:file-name/>");
                } else {
                    xml.push_str("<text:file-name>");
                    xml.push_str(&escape_xml(display_val));
                    xml.push_str("</text:file-name>");
                }
            }
            _ => {}
        }
    }
}

/// Write text content, converting embedded `\n` to `<text:line-break/>`
/// and `\t` to `<text:tab/>`.
fn write_text_with_breaks(text: &str, xml: &mut String) {
    let mut first_line = true;
    for line in text.split('\n') {
        if !first_line {
            xml.push_str("<text:line-break/>");
        }
        first_line = false;
        for (i, part) in line.split('\t').enumerate() {
            if i > 0 {
                xml.push_str("<text:tab/>");
            }
            xml.push_str(&escape_xml(part));
        }
    }
}

/// Write an image node as `<draw:frame><draw:image>`.
fn write_image(
    doc: &DocumentModel,
    node: &s1_model::Node,
    xml: &mut String,
    images: &mut Vec<ImageEntry>,
) {
    let media_id = match node.attributes.get(&AttributeKey::ImageMediaId) {
        Some(AttributeValue::MediaId(id)) => *id,
        _ => return,
    };

    let width = node
        .attributes
        .get_f64(&AttributeKey::ImageWidth)
        .unwrap_or(72.0);
    let height = node
        .attributes
        .get_f64(&AttributeKey::ImageHeight)
        .unwrap_or(72.0);
    let alt_text = node
        .attributes
        .get_string(&AttributeKey::ImageAltText)
        .unwrap_or("");

    // Determine image path in ODT
    let ext = doc
        .media()
        .get(media_id)
        .map(|m| crate::xml_util::extension_for_mime(&m.content_type).to_string())
        .unwrap_or_else(|| "png".to_string());

    let href = format!("Pictures/{}.{}", media_id.0, ext);

    images.push(ImageEntry {
        href: href.clone(),
        media_id,
    });

    xml.push_str(&format!(
        r#"<draw:frame draw:name="{}" svg:width="{}" svg:height="{}" text:anchor-type="as-char">"#,
        escape_xml(alt_text),
        points_to_cm(width),
        points_to_cm(height),
    ));
    xml.push_str(&format!(
        r#"<draw:image xlink:href="{}" xlink:type="simple" xlink:show="embed" xlink:actuate="onLoad"/>"#,
        escape_xml(&href),
    ));
    xml.push_str("</draw:frame>");
}

/// Write a Table of Contents as `<text:table-of-content>`.
fn write_toc_odt(
    doc: &DocumentModel,
    toc_id: s1_model::NodeId,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
) {
    let toc = match doc.node(toc_id) {
        Some(n) => n,
        None => return,
    };

    let max_level = toc
        .attributes
        .get(&AttributeKey::TocMaxLevel)
        .and_then(|v| match v {
            AttributeValue::Int(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(3);

    let title = toc
        .attributes
        .get_string(&AttributeKey::TocTitle)
        .unwrap_or("Table of Contents");

    xml.push_str(r#"<text:table-of-content text:name="TOC" text:protected="false">"#);

    // Build TOC source element with preserved attributes
    let use_index_marks = toc.attributes.get_bool(&AttributeKey::TocUseIndexMarks);
    let use_index_source_styles = toc
        .attributes
        .get_bool(&AttributeKey::TocUseIndexSourceStyles);
    let index_scope = toc.attributes.get_string(&AttributeKey::TocIndexScope);

    let mut source_attrs = format!(r#" text:outline-level="{}""#, max_level);
    if let Some(flag) = use_index_marks {
        source_attrs.push_str(&format!(
            r#" text:use-index-marks="{}""#,
            if flag { "true" } else { "false" }
        ));
    }
    if let Some(flag) = use_index_source_styles {
        source_attrs.push_str(&format!(
            r#" text:use-index-source-styles="{}""#,
            if flag { "true" } else { "false" }
        ));
    }
    if let Some(scope) = index_scope {
        source_attrs.push_str(&format!(r#" text:index-scope="{}""#, escape_xml(scope)));
    }

    xml.push_str(&format!("<text:table-of-content-source{}>", source_attrs));
    // NOTE: Child elements like <text:index-entry-tab-stop> inside the source
    // element are not yet preserved on round-trip.
    xml.push_str(&format!(
        "<text:index-title-template>{}</text:index-title-template>",
        escape_xml(title)
    ));
    xml.push_str("</text:table-of-content-source>");

    xml.push_str("<text:index-body>");
    // Title
    xml.push_str("<text:index-title>");
    xml.push_str(&format!(
        "<text:p text:style-name=\"Contents_20_Heading\">{}</text:p>",
        escape_xml(title)
    ));
    xml.push_str("</text:index-title>");

    // Cached entry paragraphs
    for &child_id in &toc.children {
        if let Some(child) = doc.node(child_id) {
            if child.node_type == s1_model::NodeType::Paragraph {
                write_paragraph(doc, child_id, xml, auto_styles, counter, images);
            }
        }
    }

    xml.push_str("</text:index-body>");
    xml.push_str("</text:table-of-content>");
}

/// Write a table as `<table:table>`.
fn write_table(
    doc: &DocumentModel,
    table_id: s1_model::NodeId,
    xml: &mut String,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    images: &mut Vec<ImageEntry>,
    table_counter: &mut u32,
) {
    let table = match doc.node(table_id) {
        Some(n) => n,
        None => return,
    };

    *table_counter += 1;
    xml.push_str(&format!(
        r#"<table:table table:name="Table{}">"#,
        *table_counter
    ));

    // Emit <table:table-column> elements — count columns from first row
    let col_count = table
        .children
        .iter()
        .filter_map(|&row_id| doc.node(row_id))
        .find(|n| n.node_type == NodeType::TableRow)
        .map(|row| {
            row.children
                .iter()
                .filter(|&&cid| {
                    doc.node(cid)
                        .is_some_and(|n| n.node_type == NodeType::TableCell)
                })
                .count()
        })
        .unwrap_or(0);

    if col_count > 0 {
        xml.push_str(&format!(
            r#"<table:table-column table:number-columns-repeated="{}"/>"#,
            col_count
        ));
    }

    for &row_id in &table.children {
        let row = match doc.node(row_id) {
            Some(n) if n.node_type == NodeType::TableRow => n,
            _ => continue,
        };

        xml.push_str("<table:table-row>");

        for &cell_id in &row.children {
            let cell = match doc.node(cell_id) {
                Some(n) if n.node_type == NodeType::TableCell => n,
                _ => continue,
            };

            // Cell style
            let cell_style = get_or_create_cell_auto_style(&cell.attributes, auto_styles, counter);

            let mut cell_tag = String::from("<table:table-cell");
            if let Some(ref name) = cell_style {
                cell_tag.push_str(&format!(r#" table:style-name="{name}""#));
            }

            // Col span
            if let Some(n) = cell.attributes.get_i64(&AttributeKey::ColSpan) {
                if n > 1 {
                    cell_tag.push_str(&format!(r#" table:number-columns-spanned="{n}""#));
                }
            }
            // Row span
            if let Some(n) = cell.attributes.get_i64(&AttributeKey::RowSpan) {
                if n > 1 {
                    cell_tag.push_str(&format!(r#" table:number-rows-spanned="{n}""#));
                }
            }

            cell_tag.push('>');
            xml.push_str(&cell_tag);

            // Cell contents — handle paragraphs, nested tables, and images.
            // ODF requires at least one <text:p/> in every cell.
            let mut cell_has_content = false;
            for &cc_id in &cell.children {
                let cc = match doc.node(cc_id) {
                    Some(n) => n,
                    None => continue,
                };
                match cc.node_type {
                    NodeType::Paragraph => {
                        write_paragraph(doc, cc_id, xml, auto_styles, counter, images);
                        cell_has_content = true;
                    }
                    NodeType::Table => {
                        write_table(doc, cc_id, xml, auto_styles, counter, images, table_counter);
                        cell_has_content = true;
                    }
                    NodeType::Image => {
                        // Wrap standalone image in a <text:p> as required by ODF
                        xml.push_str("<text:p>");
                        write_image(doc, cc, xml, images);
                        xml.push_str("</text:p>");
                        cell_has_content = true;
                    }
                    _ => {}
                }
            }
            if !cell_has_content {
                xml.push_str("<text:p/>");
            }

            xml.push_str("</table:table-cell>");
        }

        xml.push_str("</table:table-row>");
    }

    xml.push_str("</table:table>");
}

/// Get or create a paragraph-level auto-style. Returns `None` if no formatting needed.
/// Get or create an auto-style from a pre-built `AutoStyleKey`.
fn get_or_create_auto_style(
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
    key: AutoStyleKey,
) -> String {
    auto_styles
        .entry(key)
        .or_insert_with(|| {
            *counter += 1;
            format!("P{}", *counter)
        })
        .clone()
}

fn get_or_create_paragraph_auto_style(
    attrs: &AttributeMap,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
) -> Option<String> {
    let para_props = write_paragraph_properties(attrs);

    if para_props.is_empty() {
        return None;
    }

    let key = AutoStyleKey {
        text_props: String::new(),
        para_props,
        cell_props: String::new(),
        family: "paragraph".to_string(),
    };

    let name = auto_styles
        .entry(key)
        .or_insert_with(|| {
            *counter += 1;
            format!("P{}", *counter)
        })
        .clone();

    Some(name)
}

/// Get or create a table-cell auto-style. Returns `None` if no formatting needed.
fn get_or_create_cell_auto_style(
    attrs: &AttributeMap,
    auto_styles: &mut HashMap<AutoStyleKey, String>,
    counter: &mut u32,
) -> Option<String> {
    let cell_props = write_table_cell_properties(attrs);

    if cell_props.is_empty() {
        return None;
    }

    let key = AutoStyleKey {
        text_props: String::new(),
        para_props: String::new(),
        cell_props,
        family: "table-cell".to_string(),
    };

    let name = auto_styles
        .entry(key)
        .or_insert_with(|| {
            *counter += 1;
            format!("C{}", *counter)
        })
        .clone();

    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{Node, NodeType};

    fn build_simple_doc(text: &str) -> DocumentModel {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let run = Node::new(run_id, NodeType::Run);
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let text_node = Node::text(text_id, text);
        doc.insert_node(run_id, 0, text_node).unwrap();

        doc
    }

    #[test]
    fn write_single_paragraph() {
        let doc = build_simple_doc("Hello world");
        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains("<text:p>Hello world</text:p>"));
    }

    #[test]
    fn write_bold_text() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes = AttributeMap::new().bold(true);
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let text_node = Node::text(text_id, "bold text");
        doc.insert_node(run_id, 0, text_node).unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains("text:span"));
        assert!(xml.contains("bold text"));
        assert!(xml.contains(r#"fo:font-weight="bold""#));
    }

    #[test]
    fn write_escapes_special_chars() {
        let doc = build_simple_doc("A < B & C > D");
        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains("A &lt; B &amp; C &gt; D"));
    }

    #[test]
    fn write_empty_paragraph() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains("<text:p></text:p>") || xml.contains("<text:p/>"));
    }

    #[test]
    fn write_table() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(table_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
            .unwrap();

        let para_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Cell"))
            .unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains(r#"<table:table table:name="Table1">"#));
        assert!(xml.contains(r#"<table:table-column table:number-columns-repeated="1"/>"#));
        assert!(xml.contains("<table:table-row>"));
        assert!(xml.contains("<table:table-cell>"));
        assert!(xml.contains("Cell"));
    }

    #[test]
    fn write_toc_odt() {
        use s1_model::{AttributeKey, AttributeValue};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let toc_id = doc.next_id();
        let mut toc = Node::new(toc_id, NodeType::TableOfContents);
        toc.attributes
            .set(AttributeKey::TocMaxLevel, AttributeValue::Int(2));
        toc.attributes.set(
            AttributeKey::TocTitle,
            AttributeValue::String("Contents".into()),
        );
        doc.insert_node(body_id, 0, toc).unwrap();

        // Add a cached entry paragraph
        let p_id = doc.next_id();
        doc.insert_node(toc_id, 0, Node::new(p_id, NodeType::Paragraph))
            .unwrap();
        let r_id = doc.next_id();
        doc.insert_node(p_id, 0, Node::new(r_id, NodeType::Run))
            .unwrap();
        let t_id = doc.next_id();
        doc.insert_node(r_id, 0, Node::text(t_id, "Chapter One"))
            .unwrap();

        let (xml, _) = write_content_xml(&doc);

        assert!(xml.contains("text:table-of-content"));
        assert!(xml.contains(r#"text:outline-level="2""#));
        assert!(xml.contains("text:index-body"));
        assert!(xml.contains("Contents")); // title
        assert!(xml.contains("Chapter One")); // cached entry
    }

    #[test]
    fn write_toc_source_attributes() {
        use s1_model::{AttributeKey, AttributeValue};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let toc_id = doc.next_id();
        let mut toc = Node::new(toc_id, NodeType::TableOfContents);
        toc.attributes
            .set(AttributeKey::TocMaxLevel, AttributeValue::Int(4));
        toc.attributes.set(
            AttributeKey::TocTitle,
            AttributeValue::String("TOC Title".into()),
        );
        toc.attributes
            .set(AttributeKey::TocUseIndexMarks, AttributeValue::Bool(true));
        toc.attributes.set(
            AttributeKey::TocUseIndexSourceStyles,
            AttributeValue::Bool(false),
        );
        toc.attributes.set(
            AttributeKey::TocIndexScope,
            AttributeValue::String("document".into()),
        );
        doc.insert_node(body_id, 0, toc).unwrap();

        let p_id = doc.next_id();
        doc.insert_node(toc_id, 0, Node::new(p_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _) = write_content_xml(&doc);

        assert!(
            xml.contains(r#"text:outline-level="4""#),
            "outline-level should be 4. XML: {xml}"
        );
        assert!(
            xml.contains(r#"text:use-index-marks="true""#),
            "use-index-marks should be written. XML: {xml}"
        );
        assert!(
            xml.contains(r#"text:use-index-source-styles="false""#),
            "use-index-source-styles should be written. XML: {xml}"
        );
        assert!(
            xml.contains(r#"text:index-scope="document""#),
            "index-scope should be written. XML: {xml}"
        );
    }

    #[test]
    fn write_hyperlink_external() {
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
        doc.insert_node(run_id, 0, Node::text(text_id, "Click me"))
            .unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains(r#"<text:a xlink:href="https://example.com" xlink:type="simple">"#));
        assert!(xml.contains("Click me"));
        assert!(xml.contains("</text:a>"));
    }

    #[test]
    fn write_bookmark_start_end() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let bs_id = doc.next_id();
        let mut bs = Node::new(bs_id, NodeType::BookmarkStart);
        bs.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("bm1".into()),
        );
        doc.insert_node(para_id, 0, bs).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Bookmarked"))
            .unwrap();

        let be_id = doc.next_id();
        let mut be_node = Node::new(be_id, NodeType::BookmarkEnd);
        be_node.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("bm1".into()),
        );
        doc.insert_node(para_id, 2, be_node).unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains(r#"<text:bookmark-start text:name="bm1"/>"#));
        assert!(xml.contains("Bookmarked"));
        assert!(xml.contains(r#"<text:bookmark-end text:name="bm1"/>"#));
    }

    #[test]
    fn roundtrip_hyperlink() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("https://rust-lang.org".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Rust"))
            .unwrap();

        // Write
        let odt_bytes = crate::write(&doc).unwrap();

        // Read back
        let doc2 = crate::read(&odt_bytes).unwrap();
        let body2 = doc2.body_id().unwrap();
        let body_node = doc2.node(body2).unwrap();
        let para = doc2.node(body_node.children[0]).unwrap();

        // Find hyperlink run
        let mut found = false;
        for &cid in &para.children {
            let child = doc2.node(cid).unwrap();
            if let Some(url) = child.attributes.get_string(&AttributeKey::HyperlinkUrl) {
                assert_eq!(url, "https://rust-lang.org");
                found = true;
            }
        }
        assert!(found, "Hyperlink URL not preserved in round-trip");
    }

    #[test]
    fn roundtrip_bookmarks() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let bs_id = doc.next_id();
        let mut bs = Node::new(bs_id, NodeType::BookmarkStart);
        bs.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("mark1".into()),
        );
        doc.insert_node(para_id, 0, bs).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "content"))
            .unwrap();

        let be_id = doc.next_id();
        let mut be_node = Node::new(be_id, NodeType::BookmarkEnd);
        be_node.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("mark1".into()),
        );
        doc.insert_node(para_id, 2, be_node).unwrap();

        // Write
        let odt_bytes = crate::write(&doc).unwrap();

        // Read back
        let doc2 = crate::read(&odt_bytes).unwrap();
        let body2 = doc2.body_id().unwrap();
        let body_node = doc2.node(body2).unwrap();
        let para = doc2.node(body_node.children[0]).unwrap();

        let mut found_start = false;
        let mut found_end = false;
        for &cid in &para.children {
            let child = doc2.node(cid).unwrap();
            if child.node_type == NodeType::BookmarkStart {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::BookmarkName),
                    Some("mark1")
                );
                found_start = true;
            }
            if child.node_type == NodeType::BookmarkEnd {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::BookmarkName),
                    Some("mark1")
                );
                found_end = true;
            }
        }
        assert!(found_start, "BookmarkStart not preserved");
        assert!(found_end, "BookmarkEnd not preserved");
    }

    #[test]
    fn write_annotation() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create CommentBody on root
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        let cb_id = doc.next_id();
        let mut cb = Node::new(cb_id, NodeType::CommentBody);
        cb.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("c1".into()));
        cb.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String("Alice".into()),
        );
        cb.attributes.set(
            AttributeKey::CommentDate,
            AttributeValue::String("2024-01-15".into()),
        );
        doc.insert_node(root_id, root_children, cb).unwrap();

        let cp_id = doc.next_id();
        doc.insert_node(cb_id, 0, Node::new(cp_id, NodeType::Paragraph))
            .unwrap();
        let cr_id = doc.next_id();
        doc.insert_node(cp_id, 0, Node::new(cr_id, NodeType::Run))
            .unwrap();
        let ct_id = doc.next_id();
        doc.insert_node(cr_id, 0, Node::text(ct_id, "Nice!"))
            .unwrap();

        // Create paragraph with CommentStart/End
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let cs_id = doc.next_id();
        let mut cs = Node::new(cs_id, NodeType::CommentStart);
        cs.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("c1".into()));
        doc.insert_node(para_id, 0, cs).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "annotated"))
            .unwrap();

        let ce_id = doc.next_id();
        let mut ce = Node::new(ce_id, NodeType::CommentEnd);
        ce.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("c1".into()));
        doc.insert_node(para_id, 2, ce).unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(xml.contains(r#"office:annotation office:name="c1""#));
        assert!(xml.contains("<dc:creator>Alice</dc:creator>"));
        assert!(xml.contains("<dc:date>2024-01-15</dc:date>"));
        assert!(xml.contains("Nice!"));
        assert!(xml.contains(r#"office:annotation-end office:name="c1""#));
    }

    #[test]
    fn write_no_comments() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "plain"))
            .unwrap();

        let (xml, _) = write_content_xml(&doc);
        assert!(!xml.contains("office:annotation"));
    }

    #[test]
    fn roundtrip_annotation() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create CommentBody on root
        let root_id = doc.root_id();
        let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        let cb_id = doc.next_id();
        let mut cb = Node::new(cb_id, NodeType::CommentBody);
        cb.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("c1".into()));
        cb.attributes.set(
            AttributeKey::CommentAuthor,
            AttributeValue::String("Bob".into()),
        );
        doc.insert_node(root_id, root_children, cb).unwrap();

        let cp_id = doc.next_id();
        doc.insert_node(cb_id, 0, Node::new(cp_id, NodeType::Paragraph))
            .unwrap();
        let cr_id = doc.next_id();
        doc.insert_node(cp_id, 0, Node::new(cr_id, NodeType::Run))
            .unwrap();
        let ct_id = doc.next_id();
        doc.insert_node(cr_id, 0, Node::text(ct_id, "Feedback"))
            .unwrap();

        // Create paragraph
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let cs_id = doc.next_id();
        let mut cs = Node::new(cs_id, NodeType::CommentStart);
        cs.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("c1".into()));
        doc.insert_node(para_id, 0, cs).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "text"))
            .unwrap();

        let ce_id = doc.next_id();
        let mut ce = Node::new(ce_id, NodeType::CommentEnd);
        ce.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("c1".into()));
        doc.insert_node(para_id, 2, ce).unwrap();

        // Write
        let odt_bytes = crate::write(&doc).unwrap();

        // Read back
        let doc2 = crate::read(&odt_bytes).unwrap();
        let body2 = doc2.body_id().unwrap();
        let body_node = doc2.node(body2).unwrap();
        let para = doc2.node(body_node.children[0]).unwrap();

        // Verify CommentStart and CommentEnd exist
        let mut found_cs = false;
        let mut found_ce = false;
        for &cid in &para.children {
            let child = doc2.node(cid).unwrap();
            if child.node_type == NodeType::CommentStart {
                found_cs = true;
            }
            if child.node_type == NodeType::CommentEnd {
                found_ce = true;
            }
        }
        assert!(found_cs, "CommentStart not preserved in round-trip");
        assert!(found_ce, "CommentEnd not preserved in round-trip");

        // Verify CommentBody exists
        let root2 = doc2.root_id();
        let root_node = doc2.node(root2).unwrap();
        let mut found_body = false;
        for &cid in &root_node.children {
            let child = doc2.node(cid).unwrap();
            if child.node_type == NodeType::CommentBody {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentAuthor),
                    Some("Bob")
                );
                found_body = true;
            }
        }
        assert!(found_body, "CommentBody not preserved in round-trip");
    }

    #[test]
    fn test_nested_list_xml_wellformed() {
        use s1_model::{AttributeKey, AttributeValue, ListFormat, ListInfo};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Level 0 item
        let p0 = doc.next_id();
        let mut para0 = Node::new(p0, NodeType::Paragraph);
        para0.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 0,
                num_format: ListFormat::Bullet,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 0, para0).unwrap();
        let r0 = doc.next_id();
        doc.insert_node(p0, 0, Node::new(r0, NodeType::Run))
            .unwrap();
        let t0 = doc.next_id();
        doc.insert_node(r0, 0, Node::text(t0, "Level 0")).unwrap();

        // Level 1 item
        let p1 = doc.next_id();
        let mut para1 = Node::new(p1, NodeType::Paragraph);
        para1.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 1,
                num_format: ListFormat::Bullet,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 1, para1).unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Level 1")).unwrap();

        // Level 2 item
        let p2 = doc.next_id();
        let mut para2 = Node::new(p2, NodeType::Paragraph);
        para2.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 2,
                num_format: ListFormat::Bullet,
                num_id: 1,
                start: None,
            }),
        );
        doc.insert_node(body_id, 2, para2).unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "Level 2")).unwrap();

        // Write to ODT and extract content.xml
        let odt_bytes = crate::write(&doc).unwrap();
        let cursor = std::io::Cursor::new(&odt_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut content_xml = String::new();
        {
            use std::io::Read as _;
            archive
                .by_name("content.xml")
                .unwrap()
                .read_to_string(&mut content_xml)
                .unwrap();
        }

        // Verify well-formed XML by parsing with quick_xml
        let mut reader = quick_xml::Reader::from_str(&content_xml);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("content.xml is not well-formed XML: {e}"),
                _ => {}
            }
        }

        // Verify nested <text:list> inside <text:list-item>
        assert!(
            content_xml.contains("<text:list-item><text:p>Level 0</text:p><text:list>"),
            "Level 0 item should contain a nested <text:list>. XML: {content_xml}"
        );
        assert!(
            content_xml.contains("<text:list-item><text:p>Level 1</text:p><text:list>"),
            "Level 1 item should contain a nested <text:list>. XML: {content_xml}"
        );
        assert!(
            content_xml.contains("<text:list-item><text:p>Level 2</text:p>"),
            "Level 2 item should exist. XML: {content_xml}"
        );
    }

    #[test]
    fn test_table_has_columns() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(table_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        // Add 3 cells
        for i in 0..3 {
            let cell_id = doc.next_id();
            doc.insert_node(row_id, i, Node::new(cell_id, NodeType::TableCell))
                .unwrap();
            let para_id = doc.next_id();
            doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
                .unwrap();
            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, &format!("C{i}")))
                .unwrap();
        }

        // Write to ODT and extract content.xml
        let odt_bytes = crate::write(&doc).unwrap();
        let cursor = std::io::Cursor::new(&odt_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut content_xml = String::new();
        {
            use std::io::Read as _;
            archive
                .by_name("content.xml")
                .unwrap()
                .read_to_string(&mut content_xml)
                .unwrap();
        }

        assert!(
            content_xml.contains(r#"<table:table-column table:number-columns-repeated="3"/>"#),
            "Table should have table-column element with 3 columns. XML: {content_xml}"
        );
        assert!(
            content_xml.contains(r#"table:name="Table1""#),
            "Table should have a table:name attribute. XML: {content_xml}"
        );
    }
}
