//! Generate `word/document.xml` from a DocumentModel.

use s1_model::{
    Alignment, AttributeKey, AttributeValue, BorderSide, BorderStyle, Borders, Color,
    DocumentModel, FieldType, LineSpacing, MediaId, NodeId, NodeType, TabAlignment, TabLeader,
    TableWidth, UnderlineStyle, VerticalAlignment,
};

use crate::xml_util::{extension_for_mime, points_to_emu};
use crate::xml_writer::escape_xml;

/// An image relationship entry collected during writing.
pub struct ImageRelEntry {
    /// Relationship ID (e.g., "rId10")
    pub rid: String,
    /// Target path within the ZIP (e.g., "media/image1.png")
    pub target: String,
    /// MediaId for looking up bytes in the media store
    pub media_id: MediaId,
    /// File extension (e.g., "png")
    pub extension: String,
}

/// A hyperlink relationship entry collected during writing.
pub struct HyperlinkRelEntry {
    /// Relationship ID (e.g., "rHyp1")
    pub rid: String,
    /// External URL target
    pub target: String,
}

/// Generate `word/document.xml` content and collect relationship entries.
pub fn write_document_xml(
    doc: &DocumentModel,
) -> (String, Vec<ImageRelEntry>, Vec<HyperlinkRelEntry>) {
    let mut xml = String::new();
    let mut image_rels: Vec<ImageRelEntry> = Vec::new();
    let mut hyperlink_rels: Vec<HyperlinkRelEntry> = Vec::new();

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(
        r#"<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas" xmlns:mo="http://schemas.microsoft.com/office/mac/office/2008/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:mv="urn:schemas-microsoft-com:mac:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:w10="urn:schemas-microsoft-com:office:word" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">"#,
    );
    xml.push('\n');

    // Write body
    if let Some(body_id) = doc.body_id() {
        xml.push_str("<w:body>");
        write_body_children(doc, body_id, &mut xml, &mut image_rels, &mut hyperlink_rels);
        xml.push_str("</w:body>");
    }

    xml.push_str("</w:document>");
    (xml, image_rels, hyperlink_rels)
}

/// Write the children of the body node.
fn write_body_children(
    doc: &DocumentModel,
    body_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
) {
    let body = match doc.node(body_id) {
        Some(n) => n,
        None => return,
    };

    let children: Vec<NodeId> = body.children.clone();
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };

        match child.node_type {
            NodeType::Paragraph => {
                write_paragraph(doc, child_id, xml, image_rels, hyperlink_rels);
            }
            NodeType::Table => write_table(doc, child_id, xml, image_rels, hyperlink_rels),
            NodeType::TableOfContents => {
                write_toc(doc, child_id, xml, image_rels, hyperlink_rels);
            }
            _ => {}
        }
    }
}

/// Write a `<w:sdt>` Table of Contents element.
fn write_toc(
    doc: &DocumentModel,
    toc_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
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

    let field_code = format!(r#"TOC \o "1-{}" \h \z \u"#, max_level);

    // SDT wrapper
    xml.push_str("<w:sdt>");
    xml.push_str("<w:sdtPr>");
    xml.push_str(r#"<w:docPartObj><w:docPartGallery w:val="Table of Contents"/><w:docPartUnique/></w:docPartObj>"#);
    xml.push_str("</w:sdtPr>");
    xml.push_str("<w:sdtContent>");

    // Field begin paragraph
    xml.push_str(r#"<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> "#);
    xml.push_str(&escape_xml(&field_code));
    xml.push_str(r#" </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r></w:p>"#);

    // Cached entry paragraphs (children of the TOC node)
    for &child_id in &toc.children {
        if let Some(child) = doc.node(child_id) {
            if child.node_type == NodeType::Paragraph {
                write_paragraph(doc, child_id, xml, image_rels, hyperlink_rels);
            }
        }
    }

    // Field end paragraph
    xml.push_str(r#"<w:p><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#);

    xml.push_str("</w:sdtContent>");
    xml.push_str("</w:sdt>");
}

/// Write a `<w:p>` element.
fn write_paragraph(
    doc: &DocumentModel,
    para_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
) {
    let para = match doc.node(para_id) {
        Some(n) => n,
        None => return,
    };

    xml.push_str("<w:p>");

    // Paragraph properties
    let ppr = write_paragraph_properties(para);
    if !ppr.is_empty() {
        xml.push_str("<w:pPr>");
        xml.push_str(&ppr);
        xml.push_str("</w:pPr>");
    }

    // Inline children — group consecutive runs with the same HyperlinkUrl into
    // `<w:hyperlink>` elements.
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
                            escape_xml(anchor)
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
                        write_run(doc, run_id, xml);
                    }
                    xml.push_str("</w:hyperlink>");
                } else if let Some(rev_type) =
                    child.attributes.get_string(&AttributeKey::RevisionType)
                {
                    // Tracked change — group consecutive runs with same revision
                    let rev_type = rev_type.to_string();
                    if rev_type == "Insert"
                        || rev_type == "Delete"
                        || rev_type == "MoveTo"
                        || rev_type == "MoveFrom"
                    {
                        let rev_id = child.attributes.get_i64(&AttributeKey::RevisionId);
                        let rev_author = child
                            .attributes
                            .get_string(&AttributeKey::RevisionAuthor)
                            .map(|s| s.to_string());
                        let rev_date = child
                            .attributes
                            .get_string(&AttributeKey::RevisionDate)
                            .map(|s| s.to_string());

                        // Find all consecutive runs with same revision info
                        let rev_start = i;
                        while i < children.len() {
                            if let Some(n) = doc.node(children[i]) {
                                if n.node_type == NodeType::Run
                                    && n.attributes.get_string(&AttributeKey::RevisionType)
                                        == Some(&rev_type)
                                    && n.attributes.get_i64(&AttributeKey::RevisionId) == rev_id
                                {
                                    i += 1;
                                    continue;
                                }
                            }
                            break;
                        }

                        // Write tracked change wrapper
                        let tag = match rev_type.as_str() {
                            "Insert" => "ins",
                            "Delete" => "del",
                            "MoveTo" => "moveTo",
                            "MoveFrom" => "moveFrom",
                            _ => "ins", // fallback
                        };
                        let mut wrapper = format!("<w:{tag}");
                        if let Some(id) = rev_id {
                            wrapper.push_str(&format!(r#" w:id="{id}""#));
                        }
                        if let Some(ref author) = rev_author {
                            wrapper.push_str(&format!(r#" w:author="{}""#, escape_xml(author)));
                        }
                        if let Some(ref date) = rev_date {
                            wrapper.push_str(&format!(r#" w:date="{}""#, escape_xml(date)));
                        }
                        wrapper.push('>');
                        xml.push_str(&wrapper);

                        for &run_id in &children[rev_start..i] {
                            write_run(doc, run_id, xml);
                        }

                        xml.push_str(&format!("</w:{tag}>"));
                    } else {
                        // FormatChange — write_run handles rPrChange
                        write_run(doc, child_id, xml);
                        i += 1;
                    }
                } else {
                    write_run(doc, child_id, xml);
                    i += 1;
                }
            }
            NodeType::Image => {
                write_image(doc, child_id, xml, image_rels);
                i += 1;
            }
            NodeType::Field => {
                write_field_node(doc, child_id, xml);
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
                        escape_xml(bk_name)
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
                        escape_xml(cid)
                    ));
                }
                i += 1;
            }
            NodeType::CommentEnd => {
                if let Some(cid) = child.attributes.get_string(&AttributeKey::CommentId) {
                    xml.push_str(&format!(
                        r#"<w:commentRangeEnd w:id="{}"/>"#,
                        escape_xml(cid)
                    ));
                    // Add commentReference run
                    xml.push_str(&format!(
                        r#"<w:r><w:rPr><w:rStyle w:val="CommentReference"/></w:rPr><w:commentReference w:id="{}"/></w:r>"#,
                        escape_xml(cid)
                    ));
                }
                i += 1;
            }
            NodeType::Drawing => {
                // Write shape using stored raw XML for round-trip fidelity
                if let Some(raw) = child.attributes.get_string(&AttributeKey::ShapeRawXml) {
                    xml.push_str("<w:r>");
                    xml.push_str(raw);
                    xml.push_str("</w:r>");
                }
                i += 1;
            }
            NodeType::FootnoteRef => {
                if let Some(fid) = child.attributes.get_string(&AttributeKey::FootnoteNumber) {
                    xml.push_str(&format!(
                        r#"<w:r><w:rPr><w:rStyle w:val="FootnoteReference"/><w:vertAlign w:val="superscript"/></w:rPr><w:footnoteReference w:id="{}"/></w:r>"#,
                        escape_xml(fid)
                    ));
                }
                i += 1;
            }
            NodeType::EndnoteRef => {
                if let Some(eid) = child.attributes.get_string(&AttributeKey::EndnoteNumber) {
                    xml.push_str(&format!(
                        r#"<w:r><w:rPr><w:rStyle w:val="EndnoteReference"/><w:vertAlign w:val="superscript"/></w:rPr><w:endnoteReference w:id="{}"/></w:r>"#,
                        escape_xml(eid)
                    ));
                }
                i += 1;
            }
            NodeType::Equation => {
                // Write the raw equation XML stored in EquationSource attribute
                if let Some(source) = child.attributes.get_string(&AttributeKey::EquationSource) {
                    xml.push_str(source);
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

/// Write a `<w:tbl>` element.
fn write_table(
    doc: &DocumentModel,
    table_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
) {
    let table = match doc.node(table_id) {
        Some(n) => n,
        None => return,
    };

    xml.push_str("<w:tbl>");

    // Table properties
    let tbl_pr = write_table_properties(&table.attributes);
    if !tbl_pr.is_empty() {
        xml.push_str("<w:tblPr>");
        xml.push_str(&tbl_pr);
        xml.push_str("</w:tblPr>");
    }

    // Table grid — derive from first row's cell widths
    write_table_grid(doc, table, xml);

    // Rows
    let children: Vec<NodeId> = table.children.clone();
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        if child.node_type == NodeType::TableRow {
            write_table_row(doc, child_id, xml, image_rels, hyperlink_rels);
        }
    }

    xml.push_str("</w:tbl>");
}

/// Write `<w:tblGrid>` derived from the first row's cell widths.
fn write_table_grid(doc: &DocumentModel, table: &s1_model::Node, xml: &mut String) {
    if table.children.is_empty() {
        return;
    }

    let first_row = match doc.node(table.children[0]) {
        Some(n) if n.node_type == NodeType::TableRow => n,
        _ => return,
    };

    let mut has_widths = false;
    let mut grid_cols = Vec::new();

    for &cell_id in &first_row.children {
        if let Some(cell) = doc.node(cell_id) {
            if let Some(AttributeValue::TableWidth(tw)) =
                cell.attributes.get(&AttributeKey::CellWidth)
            {
                match tw {
                    TableWidth::Fixed(pts) => {
                        grid_cols.push(points_to_twips(*pts));
                        has_widths = true;
                    }
                    _ => grid_cols.push(0),
                }
            } else {
                grid_cols.push(0);
            }
        }
    }

    if has_widths {
        xml.push_str("<w:tblGrid>");
        for w in grid_cols {
            xml.push_str(&format!(r#"<w:gridCol w:w="{w}"/>"#));
        }
        xml.push_str("</w:tblGrid>");
    }
}

/// Generate table properties XML.
fn write_table_properties(attrs: &s1_model::AttributeMap) -> String {
    let mut tpr = String::new();

    // Table style reference
    if let Some(style_id) = attrs.get_string(&AttributeKey::StyleId) {
        tpr.push_str(&format!(
            r#"<w:tblStyle w:val="{}"/>"#,
            escape_xml(style_id)
        ));
    }

    // Table width
    if let Some(AttributeValue::TableWidth(tw)) = attrs.get(&AttributeKey::TableWidth) {
        match tw {
            TableWidth::Auto => {
                tpr.push_str(r#"<w:tblW w:w="0" w:type="auto"/>"#);
            }
            TableWidth::Fixed(pts) => {
                let twips = points_to_twips(*pts);
                tpr.push_str(&format!(r#"<w:tblW w:w="{twips}" w:type="dxa"/>"#));
            }
            TableWidth::Percent(pct) => {
                let val = (*pct * 50.0) as i64;
                tpr.push_str(&format!(r#"<w:tblW w:w="{val}" w:type="pct"/>"#));
            }
            _ => {}
        }
    }

    // Table alignment
    if let Some(alignment) = attrs.get_alignment(&AttributeKey::TableAlignment) {
        let val = match alignment {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "left",
            _ => "left",
        };
        tpr.push_str(&format!(r#"<w:jc w:val="{val}"/>"#));
    }

    // Table borders
    if let Some(AttributeValue::Borders(borders)) = attrs.get(&AttributeKey::TableBorders) {
        tpr.push_str("<w:tblBorders>");
        write_borders(borders, &mut tpr);
        tpr.push_str("</w:tblBorders>");
    }

    // Table property change tracking (tblPrChange)
    if attrs.get_string(&AttributeKey::RevisionType) == Some("PropertyChange") {
        write_property_change_element("tblPrChange", "tblPr", attrs, &mut tpr);
    }

    tpr
}

/// Write a `<w:tr>` element.
fn write_table_row(
    doc: &DocumentModel,
    row_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
) {
    let row = match doc.node(row_id) {
        Some(n) => n,
        None => return,
    };

    xml.push_str("<w:tr>");

    // Row properties (header row, property change tracking)
    let trpr = write_row_properties(&row.attributes);
    if !trpr.is_empty() {
        xml.push_str("<w:trPr>");
        xml.push_str(&trpr);
        xml.push_str("</w:trPr>");
    }

    let children: Vec<NodeId> = row.children.clone();
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        if child.node_type == NodeType::TableCell {
            write_table_cell(doc, child_id, xml, image_rels, hyperlink_rels);
        }
    }

    xml.push_str("</w:tr>");
}

/// Write a `<w:tc>` element.
fn write_table_cell(
    doc: &DocumentModel,
    cell_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
) {
    let cell = match doc.node(cell_id) {
        Some(n) => n,
        None => return,
    };

    xml.push_str("<w:tc>");

    // Cell properties
    let tcp = write_cell_properties(&cell.attributes);
    if !tcp.is_empty() {
        xml.push_str("<w:tcPr>");
        xml.push_str(&tcp);
        xml.push_str("</w:tcPr>");
    }

    // Cell content (paragraphs, nested tables)
    let children: Vec<NodeId> = cell.children.clone();
    let mut has_content = false;
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };
        match child.node_type {
            NodeType::Paragraph => {
                write_paragraph(doc, child_id, xml, image_rels, hyperlink_rels);
                has_content = true;
            }
            NodeType::Table => {
                write_table(doc, child_id, xml, image_rels, hyperlink_rels);
                has_content = true;
            }
            _ => {}
        }
    }

    // OOXML requires at least one <w:p> inside a <w:tc>
    if !has_content {
        xml.push_str("<w:p/>");
    }

    xml.push_str("</w:tc>");
}

/// Generate cell properties XML.
fn write_cell_properties(attrs: &s1_model::AttributeMap) -> String {
    let mut tcp = String::new();

    // Cell width
    if let Some(AttributeValue::TableWidth(tw)) = attrs.get(&AttributeKey::CellWidth) {
        match tw {
            TableWidth::Auto => {
                tcp.push_str(r#"<w:tcW w:w="0" w:type="auto"/>"#);
            }
            TableWidth::Fixed(pts) => {
                let twips = points_to_twips(*pts);
                tcp.push_str(&format!(r#"<w:tcW w:w="{twips}" w:type="dxa"/>"#));
            }
            TableWidth::Percent(pct) => {
                let val = (*pct * 50.0) as i64;
                tcp.push_str(&format!(r#"<w:tcW w:w="{val}" w:type="pct"/>"#));
            }
            _ => {}
        }
    }

    // Column span
    if let Some(span) = attrs.get_i64(&AttributeKey::ColSpan) {
        if span > 1 {
            tcp.push_str(&format!(r#"<w:gridSpan w:val="{span}"/>"#));
        }
    }

    // Vertical merge
    if let Some(merge) = attrs.get_string(&AttributeKey::RowSpan) {
        match merge {
            "restart" => tcp.push_str(r#"<w:vMerge w:val="restart"/>"#),
            "continue" => tcp.push_str("<w:vMerge/>"),
            _ => {}
        }
    }

    // Vertical alignment
    if let Some(AttributeValue::VerticalAlignment(va)) = attrs.get(&AttributeKey::VerticalAlign) {
        let val = match va {
            VerticalAlignment::Top => "top",
            VerticalAlignment::Center => "center",
            VerticalAlignment::Bottom => "bottom",
            _ => "top",
        };
        tcp.push_str(&format!(r#"<w:vAlign w:val="{val}"/>"#));
    }

    // Cell shading/background
    if let Some(color) = attrs.get_color(&AttributeKey::CellBackground) {
        tcp.push_str(&format!(
            r#"<w:shd w:val="clear" w:fill="{}"/>"#,
            color.to_hex()
        ));
    }

    // Cell borders
    if let Some(AttributeValue::Borders(borders)) = attrs.get(&AttributeKey::CellBorders) {
        tcp.push_str("<w:tcBorders>");
        write_borders(borders, &mut tcp);
        tcp.push_str("</w:tcBorders>");
    }

    // Cell property change tracking (tcPrChange)
    if attrs.get_string(&AttributeKey::RevisionType) == Some("PropertyChange") {
        write_property_change_element("tcPrChange", "tcPr", attrs, &mut tcp);
    }

    tcp
}

/// Generate row properties XML.
fn write_row_properties(attrs: &s1_model::AttributeMap) -> String {
    let mut trp = String::new();

    // Header row
    if attrs.get_bool(&AttributeKey::TableHeaderRow) == Some(true) {
        trp.push_str("<w:tblHeader/>");
    }

    // Row property change tracking (trPrChange)
    if attrs.get_string(&AttributeKey::RevisionType) == Some("PropertyChange") {
        write_property_change_element("trPrChange", "trPr", attrs, &mut trp);
    }

    trp
}

/// Write border sides (top, bottom, left, right) shared between table and cell borders.
fn write_borders(borders: &Borders, xml: &mut String) {
    if let Some(ref side) = borders.top {
        write_border_side("top", side, xml);
    }
    if let Some(ref side) = borders.left {
        write_border_side("left", side, xml);
    }
    if let Some(ref side) = borders.bottom {
        write_border_side("bottom", side, xml);
    }
    if let Some(ref side) = borders.right {
        write_border_side("right", side, xml);
    }
}

/// Write a single border side element.
fn write_border_side(name: &str, side: &BorderSide, xml: &mut String) {
    let style = match side.style {
        BorderStyle::None => "none",
        BorderStyle::Single => "single",
        BorderStyle::Double => "double",
        BorderStyle::Dashed => "dashed",
        BorderStyle::Dotted => "dotted",
        BorderStyle::Thick => "thick",
        _ => "none",
    };
    let sz = (side.width * 8.0) as i64; // points to eighths of a point
    let color = side.color.to_hex();
    let space = side.spacing as i64;
    xml.push_str(&format!(
        r#"<w:{name} w:val="{style}" w:sz="{sz}" w:space="{space}" w:color="{color}"/>"#
    ));
}

/// Write an image as `<w:r><w:drawing>…</w:drawing></w:r>`.
/// Supports both inline (`wp:inline`) and floating (`wp:anchor`) images.
fn write_image(
    doc: &DocumentModel,
    image_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
) {
    let image = match doc.node(image_id) {
        Some(n) => n,
        None => return,
    };

    // Get the media ID from the image node
    let media_id = match image.attributes.get(&AttributeKey::ImageMediaId) {
        Some(AttributeValue::MediaId(mid)) => *mid,
        _ => return,
    };

    // Look up the media item to get content type and extension
    let media_item = match doc.media().get(media_id) {
        Some(item) => item,
        None => return,
    };

    let ext = extension_for_mime(&media_item.content_type);
    let idx = image_rels.len() + 1;
    let rid = format!("rImg{idx}");
    let target = format!("media/image{idx}.{ext}");

    image_rels.push(ImageRelEntry {
        rid: rid.clone(),
        target,
        media_id,
        extension: ext.to_string(),
    });

    // Dimensions in EMU (default 100×100 pts if not specified)
    let width_pts = image
        .attributes
        .get_f64(&AttributeKey::ImageWidth)
        .unwrap_or(100.0);
    let height_pts = image
        .attributes
        .get_f64(&AttributeKey::ImageHeight)
        .unwrap_or(100.0);
    let cx = points_to_emu(width_pts);
    let cy = points_to_emu(height_pts);

    let alt = image
        .attributes
        .get_string(&AttributeKey::ImageAltText)
        .unwrap_or("");

    // Check if this is a floating (anchor) image
    let is_floating = image
        .attributes
        .get_string(&AttributeKey::ImagePositionType)
        .map(|s| s == "anchor")
        .unwrap_or(false);

    xml.push_str("<w:r><w:drawing>");

    if is_floating {
        // Floating image — output wp:anchor
        let dist_str = image
            .attributes
            .get_string(&AttributeKey::ImageDistanceFromText)
            .unwrap_or("0,0,0,0");
        let dists: Vec<&str> = dist_str.split(',').collect();
        let dist_t = dists.first().unwrap_or(&"0");
        let dist_b = dists.get(1).unwrap_or(&"0");
        let dist_l = dists.get(2).unwrap_or(&"0");
        let dist_r = dists.get(3).unwrap_or(&"0");

        xml.push_str(&format!(
            r#"<wp:anchor distT="{dist_t}" distB="{dist_b}" distL="{dist_l}" distR="{dist_r}" simplePos="0" relativeHeight="0" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1">"#
        ));
        xml.push_str(r#"<wp:simplePos x="0" y="0"/>"#);

        // Horizontal position
        let h_rel = image
            .attributes
            .get_string(&AttributeKey::ImageHorizontalRelativeFrom)
            .unwrap_or("column");
        let h_off = image
            .attributes
            .get_i64(&AttributeKey::ImageHorizontalOffset)
            .unwrap_or(0);
        xml.push_str(&format!(
            r#"<wp:positionH relativeFrom="{h_rel}"><wp:posOffset>{h_off}</wp:posOffset></wp:positionH>"#
        ));

        // Vertical position
        let v_rel = image
            .attributes
            .get_string(&AttributeKey::ImageVerticalRelativeFrom)
            .unwrap_or("paragraph");
        let v_off = image
            .attributes
            .get_i64(&AttributeKey::ImageVerticalOffset)
            .unwrap_or(0);
        xml.push_str(&format!(
            r#"<wp:positionV relativeFrom="{v_rel}"><wp:posOffset>{v_off}</wp:posOffset></wp:positionV>"#
        ));

        xml.push_str(&format!(r#"<wp:extent cx="{cx}" cy="{cy}"/>"#));
        xml.push_str("<wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/>");
        xml.push_str(&format!(
            r#"<wp:docPr id="{idx}" name="Image{idx}" descr="{}"/>"#,
            escape_xml(alt)
        ));

        // Wrap type
        let wrap = image
            .attributes
            .get_string(&AttributeKey::ImageWrapType)
            .unwrap_or("square");
        match wrap {
            "none" => xml.push_str("<wp:wrapNone/>"),
            "tight" => xml.push_str(r#"<wp:wrapTight wrapText="bothSides"><wp:wrapPolygon edited="0"><wp:start x="0" y="0"/><wp:lineTo x="0" y="21600"/><wp:lineTo x="21600" y="21600"/><wp:lineTo x="21600" y="0"/><wp:lineTo x="0" y="0"/></wp:wrapPolygon></wp:wrapTight>"#),
            "through" => xml.push_str(r#"<wp:wrapThrough wrapText="bothSides"><wp:wrapPolygon edited="0"><wp:start x="0" y="0"/><wp:lineTo x="0" y="21600"/><wp:lineTo x="21600" y="21600"/><wp:lineTo x="21600" y="0"/><wp:lineTo x="0" y="0"/></wp:wrapPolygon></wp:wrapThrough>"#),
            "topAndBottom" => xml.push_str("<wp:wrapTopAndBottom/>"),
            _ => xml.push_str(r#"<wp:wrapSquare wrapText="bothSides"/>"#), // default: square
        }
    } else {
        // Inline image — output wp:inline
        xml.push_str("<wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">");
        xml.push_str(&format!(r#"<wp:extent cx="{cx}" cy="{cy}"/>"#));
        xml.push_str(&format!(
            r#"<wp:docPr id="{idx}" name="Image{idx}" descr="{}"/>"#,
            escape_xml(alt)
        ));
    }

    // Common graphic element (same for both inline and anchor)
    xml.push_str("<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">");
    xml.push_str(&format!(
        r#"<pic:pic><pic:nvPicPr><pic:cNvPr id="{idx}" name="Image{idx}"/><pic:cNvPicPr/></pic:nvPicPr>"#
    ));
    xml.push_str(&format!(
        r#"<pic:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>"#
    ));
    xml.push_str(&format!(
        r#"<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>"#
    ));

    if is_floating {
        xml.push_str("</pic:pic></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r>");
    } else {
        xml.push_str("</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>");
    }
}

/// Generate paragraph properties XML from a Node.
fn write_paragraph_properties(para: &s1_model::Node) -> String {
    write_paragraph_properties_from_attrs(&para.attributes)
}

/// Generate paragraph properties XML from an AttributeMap.
///
/// Public so the style writer can reuse it.
pub fn write_paragraph_properties_from_attrs(attrs: &s1_model::AttributeMap) -> String {
    let mut ppr = String::new();

    // Style reference
    if let Some(style_id) = attrs.get_string(&AttributeKey::StyleId) {
        ppr.push_str(&format!(r#"<w:pStyle w:val="{}"/>"#, escape_xml(style_id)));
    }

    // List numbering reference
    if let Some(AttributeValue::ListInfo(info)) = attrs.get(&AttributeKey::ListInfo) {
        ppr.push_str("<w:numPr>");
        ppr.push_str(&format!(r#"<w:ilvl w:val="{}"/>"#, info.level));
        ppr.push_str(&format!(r#"<w:numId w:val="{}"/>"#, info.num_id));
        ppr.push_str("</w:numPr>");
    }

    // Alignment
    if let Some(alignment) = attrs.get_alignment(&AttributeKey::Alignment) {
        let val = match alignment {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "both",
            _ => "left",
        };
        ppr.push_str(&format!(r#"<w:jc w:val="{val}"/>"#));
    }

    // Spacing
    let before = attrs.get_f64(&AttributeKey::SpacingBefore);
    let after = attrs.get_f64(&AttributeKey::SpacingAfter);
    let line_spacing = attrs.get(&AttributeKey::LineSpacing);

    if before.is_some() || after.is_some() || line_spacing.is_some() {
        let mut spacing_attrs = String::new();
        if let Some(pts) = before {
            spacing_attrs.push_str(&format!(r#" w:before="{}""#, points_to_twips(pts)));
        }
        if let Some(pts) = after {
            spacing_attrs.push_str(&format!(r#" w:after="{}""#, points_to_twips(pts)));
        }
        if let Some(AttributeValue::LineSpacing(ls)) = line_spacing {
            match ls {
                LineSpacing::Single => {
                    spacing_attrs.push_str(r#" w:line="240" w:lineRule="auto""#);
                }
                LineSpacing::OnePointFive => {
                    spacing_attrs.push_str(r#" w:line="360" w:lineRule="auto""#);
                }
                LineSpacing::Double => {
                    spacing_attrs.push_str(r#" w:line="480" w:lineRule="auto""#);
                }
                LineSpacing::Multiple(m) => {
                    let val = (m * 240.0) as i64;
                    spacing_attrs.push_str(&format!(r#" w:line="{val}" w:lineRule="auto""#));
                }
                LineSpacing::Exact(pts) => {
                    let val = points_to_twips(*pts);
                    spacing_attrs.push_str(&format!(r#" w:line="{val}" w:lineRule="exact""#));
                }
                LineSpacing::AtLeast(pts) => {
                    let val = points_to_twips(*pts);
                    spacing_attrs.push_str(&format!(r#" w:line="{val}" w:lineRule="atLeast""#));
                }
                _ => {}
            }
        }
        ppr.push_str(&format!("<w:spacing{spacing_attrs}/>"));
    }

    // Indentation
    let left = attrs.get_f64(&AttributeKey::IndentLeft);
    let right = attrs.get_f64(&AttributeKey::IndentRight);
    let first_line = attrs.get_f64(&AttributeKey::IndentFirstLine);

    if left.is_some() || right.is_some() || first_line.is_some() {
        let mut ind_attrs = String::new();
        if let Some(pts) = left {
            ind_attrs.push_str(&format!(r#" w:left="{}""#, points_to_twips(pts)));
        }
        if let Some(pts) = right {
            ind_attrs.push_str(&format!(r#" w:right="{}""#, points_to_twips(pts)));
        }
        if let Some(pts) = first_line {
            ind_attrs.push_str(&format!(r#" w:firstLine="{}""#, points_to_twips(pts)));
        }
        ppr.push_str(&format!("<w:ind{ind_attrs}/>"));
    }

    // Toggle properties
    if attrs.get_bool(&AttributeKey::KeepWithNext) == Some(true) {
        ppr.push_str("<w:keepNext/>");
    }
    if attrs.get_bool(&AttributeKey::KeepLinesTogether) == Some(true) {
        ppr.push_str("<w:keepLines/>");
    }
    if attrs.get_bool(&AttributeKey::PageBreakBefore) == Some(true) {
        ppr.push_str("<w:pageBreakBefore/>");
    }
    if attrs.get_bool(&AttributeKey::Bidi) == Some(true) {
        ppr.push_str("<w:bidi/>");
    }
    if attrs.get_bool(&AttributeKey::SuppressAutoHyphens) == Some(true) {
        ppr.push_str("<w:suppressAutoHyphens/>");
    }
    if attrs.get_bool(&AttributeKey::ContextualSpacing) == Some(true) {
        ppr.push_str("<w:contextualSpacing/>");
    }
    if attrs.get_bool(&AttributeKey::WordWrap) == Some(false) {
        ppr.push_str(r#"<w:wordWrap w:val="false"/>"#);
    }

    // Tab stops
    if let Some(AttributeValue::TabStops(tab_stops)) = attrs.get(&AttributeKey::TabStops) {
        if !tab_stops.is_empty() {
            ppr.push_str("<w:tabs>");
            for ts in tab_stops {
                let pos = points_to_twips(ts.position);
                let val = match ts.alignment {
                    TabAlignment::Left => "left",
                    TabAlignment::Center => "center",
                    TabAlignment::Right => "right",
                    TabAlignment::Decimal => "decimal",
                    _ => "left",
                };
                let leader = match ts.leader {
                    TabLeader::None => None,
                    TabLeader::Dot => Some("dot"),
                    TabLeader::Dash => Some("hyphen"),
                    TabLeader::Underscore => Some("underscore"),
                    _ => None,
                };
                if let Some(ldr) = leader {
                    ppr.push_str(&format!(
                        r#"<w:tab w:val="{val}" w:pos="{pos}" w:leader="{ldr}"/>"#
                    ));
                } else {
                    ppr.push_str(&format!(r#"<w:tab w:val="{val}" w:pos="{pos}"/>"#));
                }
            }
            ppr.push_str("</w:tabs>");
        }
    }

    // Paragraph borders
    if let Some(AttributeValue::Borders(borders)) = attrs.get(&AttributeKey::ParagraphBorders) {
        ppr.push_str("<w:pBdr>");
        write_borders(borders, &mut ppr);
        ppr.push_str("</w:pBdr>");
    }

    // Paragraph shading/background
    if let Some(color) = attrs.get_color(&AttributeKey::Background) {
        ppr.push_str(&format!(
            r#"<w:shd w:val="clear" w:fill="{}"/>"#,
            color.to_hex()
        ));
    }

    // Paragraph property change tracking (pPrChange)
    if attrs.get_string(&AttributeKey::RevisionType) == Some("PropertyChange") {
        write_property_change_element("pPrChange", "pPr", attrs, &mut ppr);
    }

    ppr
}

/// Write a property change element (`pPrChange`, `tcPrChange`, `trPrChange`, `tblPrChange`).
///
/// Emits `<w:{tag} w:id="..." w:author="..." w:date="..."><w:{inner_tag}/></w:{tag}>`.
fn write_property_change_element(
    tag: &str,
    inner_tag: &str,
    attrs: &s1_model::AttributeMap,
    xml: &mut String,
) {
    let id = attrs.get_i64(&AttributeKey::RevisionId).unwrap_or(0);
    let author = attrs
        .get_string(&AttributeKey::RevisionAuthor)
        .unwrap_or_default();
    let date = attrs
        .get_string(&AttributeKey::RevisionDate)
        .unwrap_or_default();

    xml.push_str(&format!(
        r#"<w:{tag} w:id="{id}" w:author="{}" w:date="{}">"#,
        escape_xml(author),
        escape_xml(date),
    ));
    // Write empty old properties element (original formatting not stored)
    xml.push_str(&format!("<w:{inner_tag}/>"));
    xml.push_str(&format!("</w:{tag}>"));
}

/// Public wrapper for write_run (for use by header/footer writer).
pub fn write_run_pub(doc: &DocumentModel, run_id: NodeId, xml: &mut String) {
    write_run(doc, run_id, xml);
}

/// Public wrapper for write_image (for use by header/footer writer).
pub fn write_image_pub(
    doc: &DocumentModel,
    image_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
) {
    write_image(doc, image_id, xml, image_rels);
}

/// Public wrapper for write_table (for use by header/footer writer).
pub fn write_table_pub(
    doc: &DocumentModel,
    table_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
) {
    let mut dummy_hyp_rels = Vec::new();
    write_table(doc, table_id, xml, image_rels, &mut dummy_hyp_rels);
}

/// Public wrapper for write_table that also collects hyperlink relationships.
pub fn write_table_with_hyperlinks_pub(
    doc: &DocumentModel,
    table_id: NodeId,
    xml: &mut String,
    image_rels: &mut Vec<ImageRelEntry>,
    hyperlink_rels: &mut Vec<HyperlinkRelEntry>,
) {
    write_table(doc, table_id, xml, image_rels, hyperlink_rels);
}

/// Write a `<w:r>` element.
///
/// OOXML constraint: empty `<w:r></w:r>` is technically valid but wasteful.
/// We skip runs that have no text content unless they carry revision-tracking
/// attributes (Insert/Delete/MoveFrom/MoveTo/FormatChange) which are
/// semantically meaningful even without visible text.
fn write_run(doc: &DocumentModel, run_id: NodeId, xml: &mut String) {
    let run = match doc.node(run_id) {
        Some(n) => n,
        None => return,
    };

    // Check if the run has any text content
    let has_text = run.children.iter().any(|&cid| {
        doc.node(cid)
            .map(|c| {
                c.node_type == NodeType::Text
                    && c.text_content
                        .as_ref()
                        .map(|t| !t.is_empty())
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    });

    // Skip empty runs that have no revision-tracking purpose
    if !has_text {
        let has_revision = run
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_some();
        if !has_revision {
            return;
        }
    }

    let is_delete = matches!(
        run.attributes.get_string(&AttributeKey::RevisionType),
        Some("Delete") | Some("MoveFrom")
    );

    xml.push_str("<w:r>");

    // Run properties (includes rPrChange for FormatChange revisions)
    let rpr = write_run_properties(run);
    if !rpr.is_empty() {
        xml.push_str("<w:rPr>");
        xml.push_str(&rpr);
        xml.push_str("</w:rPr>");
    }

    // Text children — use <w:delText> for delete revisions
    let text_tag = if is_delete { "delText" } else { "t" };
    let children: Vec<NodeId> = run.children.clone();
    for child_id in children {
        let child = match doc.node(child_id) {
            Some(n) => n,
            None => continue,
        };

        if child.node_type == NodeType::Text {
            if let Some(text) = &child.text_content {
                xml.push_str(&format!(r#"<w:{text_tag} xml:space="preserve">"#));
                xml.push_str(&escape_xml(text));
                xml.push_str(&format!("</w:{text_tag}>"));
            }
        }
    }

    xml.push_str("</w:r>");
}

/// Generate run properties XML from a Node.
fn write_run_properties(run: &s1_model::Node) -> String {
    write_run_properties_from_attrs(&run.attributes)
}

/// Generate run properties XML from an AttributeMap.
///
/// Public so the style writer can reuse it.
pub fn write_run_properties_from_attrs(attrs: &s1_model::AttributeMap) -> String {
    let mut rpr = String::new();

    // Style reference
    if let Some(style_id) = attrs.get_string(&AttributeKey::StyleId) {
        rpr.push_str(&format!(r#"<w:rStyle w:val="{}"/>"#, escape_xml(style_id)));
    }

    // Font family
    if let Some(font) = attrs.get_string(&AttributeKey::FontFamily) {
        let escaped = escape_xml(font);
        rpr.push_str(&format!(
            r#"<w:rFonts w:ascii="{escaped}" w:hAnsi="{escaped}"/>"#
        ));
    }

    // Bold
    if let Some(bold) = attrs.get_bool(&AttributeKey::Bold) {
        if bold {
            rpr.push_str("<w:b/>");
        } else {
            rpr.push_str(r#"<w:b w:val="false"/>"#);
        }
    }

    // Italic
    if let Some(italic) = attrs.get_bool(&AttributeKey::Italic) {
        if italic {
            rpr.push_str("<w:i/>");
        } else {
            rpr.push_str(r#"<w:i w:val="false"/>"#);
        }
    }

    // Strikethrough
    if let Some(true) = attrs.get_bool(&AttributeKey::Strikethrough) {
        rpr.push_str("<w:strike/>");
    }

    // Underline
    if let Some(AttributeValue::UnderlineStyle(style)) = attrs.get(&AttributeKey::Underline) {
        let val = match style {
            UnderlineStyle::None => "none",
            UnderlineStyle::Single => "single",
            UnderlineStyle::Double => "double",
            UnderlineStyle::Thick => "thick",
            UnderlineStyle::Dotted => "dotted",
            UnderlineStyle::Dashed => "dash",
            UnderlineStyle::Wave => "wave",
            _ => "none",
        };
        rpr.push_str(&format!(r#"<w:u w:val="{val}"/>"#));
    }

    // Font size (points → half-points)
    if let Some(pts) = attrs.get_f64(&AttributeKey::FontSize) {
        let half_pts = (pts * 2.0) as i64;
        rpr.push_str(&format!(r#"<w:sz w:val="{half_pts}"/>"#));
    }

    // Text color
    if let Some(color) = attrs.get_color(&AttributeKey::Color) {
        rpr.push_str(&format!(r#"<w:color w:val="{}"/>"#, color.to_hex()));
    }

    // Highlight color — use w:highlight for standard colors, w:shd for arbitrary
    if let Some(color) = attrs.get_color(&AttributeKey::HighlightColor) {
        let name = color_to_highlight_name(color);
        if name != "yellow" || (color.r == 255 && color.g == 255 && color.b == 0) {
            // Known named color
            rpr.push_str(&format!(r#"<w:highlight w:val="{name}"/>"#));
        } else {
            // Arbitrary color — use shading
            rpr.push_str(&format!(
                r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
                color.to_hex()
            ));
        }
    }

    // Superscript / Subscript
    if attrs.get_bool(&AttributeKey::Superscript) == Some(true) {
        rpr.push_str(r#"<w:vertAlign w:val="superscript"/>"#);
    } else if attrs.get_bool(&AttributeKey::Subscript) == Some(true) {
        rpr.push_str(r#"<w:vertAlign w:val="subscript"/>"#);
    }

    // Character spacing (points → twips)
    if let Some(pts) = attrs.get_f64(&AttributeKey::FontSpacing) {
        let twips = points_to_twips(pts);
        rpr.push_str(&format!(r#"<w:spacing w:val="{twips}"/>"#));
    }

    // Text shadow
    if attrs.get_bool(&AttributeKey::TextShadow) == Some(true) {
        rpr.push_str("<w:shadow/>");
    }

    // Text outline
    if attrs.get_bool(&AttributeKey::TextOutline) == Some(true) {
        rpr.push_str("<w:outline/>");
    }

    // Language
    if let Some(lang) = attrs.get_string(&AttributeKey::Language) {
        rpr.push_str(&format!(r#"<w:lang w:val="{}"/>"#, escape_xml(lang)));
    }

    // Track changes — format change revision (rPrChange)
    if attrs.get_string(&AttributeKey::RevisionType) == Some("FormatChange") {
        let mut rpr_change = String::from("<w:rPrChange");
        if let Some(id) = attrs.get_i64(&AttributeKey::RevisionId) {
            rpr_change.push_str(&format!(r#" w:id="{id}""#));
        }
        if let Some(author) = attrs.get_string(&AttributeKey::RevisionAuthor) {
            rpr_change.push_str(&format!(r#" w:author="{}""#, escape_xml(author)));
        }
        if let Some(date) = attrs.get_string(&AttributeKey::RevisionDate) {
            rpr_change.push_str(&format!(r#" w:date="{}""#, escape_xml(date)));
        }
        rpr_change.push_str("><w:rPr/></w:rPrChange>");
        rpr.push_str(&rpr_change);
    }

    rpr
}

/// Convert points to twips (twentieths of a point).
fn points_to_twips(pts: f64) -> i64 {
    (pts * 20.0) as i64
}

/// Write a Field node as `<w:fldSimple>`.
fn write_field_node(doc: &DocumentModel, field_id: NodeId, xml: &mut String) {
    let field = match doc.node(field_id) {
        Some(n) => n,
        None => return,
    };

    let field_type = match field.attributes.get(&AttributeKey::FieldType) {
        Some(AttributeValue::FieldType(ft)) => *ft,
        _ => return,
    };

    let instr = crate::header_footer_writer::field_type_to_instruction(field_type);
    let placeholder = match field_type {
        FieldType::PageNumber => "1",
        FieldType::PageCount => "1",
        _ => "",
    };

    xml.push_str(&format!(
        r#"<w:fldSimple w:instr=" {} "><w:r><w:t>{}</w:t></w:r></w:fldSimple>"#,
        escape_xml(&instr),
        placeholder,
    ));
}

/// Write a `w:sectPr` element for the body (final section or inline).
pub fn write_section_xml(
    doc: &DocumentModel,
    section_idx: usize,
    xml: &mut String,
    hf_rel_entries: &[crate::section_writer::HfRelEntry],
) {
    if let Some(props) = doc.sections().get(section_idx) {
        xml.push_str("<w:sectPr>");
        xml.push_str(&crate::section_writer::write_section_properties(
            props,
            hf_rel_entries,
        ));
        xml.push_str("</w:sectPr>");
    }
}

/// Best-effort mapping of Color to OOXML highlight color name.
fn color_to_highlight_name(color: Color) -> &'static str {
    match (color.r, color.g, color.b) {
        (255, 255, 0) => "yellow",
        (0, 255, 0) => "green",
        (0, 255, 255) => "cyan",
        (255, 0, 255) => "magenta",
        (0, 0, 255) => "blue",
        (255, 0, 0) => "red",
        (0, 0, 0) => "black",
        (255, 255, 255) => "white",
        _ => "yellow", // fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AttributeMap, Node};

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
    fn write_simple_document() {
        let doc = make_simple_doc("Hello World");
        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);

        assert!(xml.contains("<w:t xml:space=\"preserve\">Hello World</w:t>"));
        assert!(xml.contains("<w:body>"));
        assert!(xml.contains("<w:p>"));
        assert!(xml.contains("<w:r>"));
    }

    #[test]
    fn write_bold_run() {
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
        doc.insert_node(run_id, 0, Node::text(text_id, "Bold"))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);

        assert!(xml.contains("<w:b/>"));
        assert!(xml.contains(r#"<w:sz w:val="48"/>"#)); // 24pt = 48 half-points
    }

    #[test]
    fn write_paragraph_alignment() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes = AttributeMap::new().alignment(Alignment::Center);
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:jc w:val="center"/>"#));
    }

    #[test]
    fn write_paragraph_spacing() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes
            .set(AttributeKey::SpacingBefore, AttributeValue::Float(12.0));
        para.attributes
            .set(AttributeKey::SpacingAfter, AttributeValue::Float(6.0));
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"w:before="240""#)); // 12pt = 240 twips
        assert!(xml.contains(r#"w:after="120""#)); // 6pt = 120 twips
    }

    #[test]
    fn write_escapes_special_chars() {
        let doc = make_simple_doc("A & B < C");
        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("A &amp; B &lt; C"));
    }

    #[test]
    fn write_empty_paragraph() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:p></w:p>"));
    }

    #[test]
    fn write_line_break() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let br_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(br_id, NodeType::LineBreak))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:r><w:br/></w:r>"));
    }

    // ─── Table writing tests ──────────────────────────────────────────

    #[test]
    fn write_simple_table() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Table > Row > 2 Cells > 1 Paragraph each
        let tbl_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        // Cell 1
        let cell1_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell1_id, NodeType::TableCell))
            .unwrap();
        let p1 = doc.next_id();
        doc.insert_node(cell1_id, 0, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "A1")).unwrap();

        // Cell 2
        let cell2_id = doc.next_id();
        doc.insert_node(row_id, 1, Node::new(cell2_id, NodeType::TableCell))
            .unwrap();
        let p2 = doc.next_id();
        doc.insert_node(cell2_id, 0, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "B1")).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:tbl>"));
        assert!(xml.contains("<w:tr>"));
        assert!(xml.contains("<w:tc>"));
        assert!(xml.contains("A1"));
        assert!(xml.contains("B1"));
        assert!(xml.contains("</w:tc>"));
        assert!(xml.contains("</w:tr>"));
        assert!(xml.contains("</w:tbl>"));
    }

    #[test]
    fn write_table_with_properties() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let tbl_id = doc.next_id();
        let mut tbl = Node::new(tbl_id, NodeType::Table);
        tbl.attributes.set(
            AttributeKey::TableWidth,
            AttributeValue::TableWidth(s1_model::TableWidth::Fixed(468.0)),
        );
        tbl.attributes.set(
            AttributeKey::TableAlignment,
            AttributeValue::Alignment(Alignment::Center),
        );
        doc.insert_node(body_id, 0, tbl).unwrap();

        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        let cell_id = doc.next_id();
        let mut cell = Node::new(cell_id, NodeType::TableCell);
        cell.attributes.set(
            AttributeKey::CellWidth,
            AttributeValue::TableWidth(s1_model::TableWidth::Fixed(234.0)),
        );
        doc.insert_node(row_id, 0, cell).unwrap();

        // Must have a paragraph inside
        let p_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(p_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"w:w="9360" w:type="dxa""#)); // 468pt = 9360 twips
        assert!(xml.contains(r#"<w:jc w:val="center"/>"#));
        assert!(xml.contains(r#"w:w="4680" w:type="dxa""#)); // 234pt = 4680 twips
                                                             // Grid col should also appear
        assert!(xml.contains(r#"<w:gridCol w:w="4680"/>"#));
    }

    #[test]
    fn write_table_cell_merge() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let tbl_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        // Cell with colspan=2
        let cell_id = doc.next_id();
        let mut cell = Node::new(cell_id, NodeType::TableCell);
        cell.attributes
            .set(AttributeKey::ColSpan, AttributeValue::Int(2));
        doc.insert_node(row_id, 0, cell).unwrap();
        let p_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(p_id, NodeType::Paragraph))
            .unwrap();

        // Cell with vMerge restart
        let cell2_id = doc.next_id();
        let mut cell2 = Node::new(cell2_id, NodeType::TableCell);
        cell2.attributes.set(
            AttributeKey::RowSpan,
            AttributeValue::String("restart".into()),
        );
        doc.insert_node(row_id, 1, cell2).unwrap();
        let p2_id = doc.next_id();
        doc.insert_node(cell2_id, 0, Node::new(p2_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:gridSpan w:val="2"/>"#));
        assert!(xml.contains(r#"<w:vMerge w:val="restart"/>"#));
    }

    #[test]
    fn write_empty_cell_gets_paragraph() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let tbl_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        // Empty cell — no children
        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        // Should have a <w:p/> to satisfy OOXML requirements
        assert!(xml.contains("<w:p/>"));
    }

    #[test]
    fn write_font_and_color() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes = AttributeMap::new().font_family("Arial").color(Color::RED);
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Red"))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"w:ascii="Arial""#));
        assert!(xml.contains(r#"<w:color w:val="FF0000"/>"#));
    }

    // ─── Image writing tests ──────────────────────────────────────────

    #[test]
    fn write_inline_image() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Insert an image into the media store
        let media_id = doc.media_mut().insert(
            "image/png",
            vec![0x89, 0x50, 0x4E, 0x47],
            Some("test.png".to_string()),
        );

        let img_id = doc.next_id();
        let mut img = Node::new(img_id, NodeType::Image);
        img.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        img.attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
        img.attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(150.0));
        img.attributes.set(
            AttributeKey::ImageAltText,
            AttributeValue::String("A test image".into()),
        );
        doc.insert_node(para_id, 0, img).unwrap();

        let (xml, rels, _hyp_rels) = write_document_xml(&doc);

        // Check drawing XML structure
        assert!(xml.contains("<w:drawing>"));
        assert!(xml.contains("<wp:inline"));
        assert!(xml.contains("wp:extent"));
        assert!(xml.contains("a:blip"));
        assert!(xml.contains("pic:pic"));
        assert!(xml.contains("r:embed=\"rImg1\""));
        assert!(xml.contains("descr=\"A test image\""));

        // Check EMU values (200pt * 12700 = 2540000, 150pt * 12700 = 1905000)
        assert!(xml.contains(r#"cx="2540000""#));
        assert!(xml.contains(r#"cy="1905000""#));

        // Check relationship entry
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].rid, "rImg1");
        assert_eq!(rels[0].target, "media/image1.png");
        assert_eq!(rels[0].extension, "png");
    }

    #[test]
    fn write_multiple_images() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Two images
        for i in 0..2 {
            let media_id =
                doc.media_mut()
                    .insert("image/jpeg", vec![0xFF, 0xD8, i], None::<String>);

            let img_id = doc.next_id();
            let mut img = Node::new(img_id, NodeType::Image);
            img.attributes.set(
                AttributeKey::ImageMediaId,
                AttributeValue::MediaId(media_id),
            );
            doc.insert_node(para_id, i as usize, img).unwrap();
        }

        let (xml, rels, _hyp_rels) = write_document_xml(&doc);

        assert_eq!(rels.len(), 2);
        assert_eq!(rels[0].rid, "rImg1");
        assert_eq!(rels[1].rid, "rImg2");
        assert!(xml.contains("r:embed=\"rImg1\""));
        assert!(xml.contains("r:embed=\"rImg2\""));
    }

    // ─── List writing tests ───────────────────────────────────────────

    #[test]
    fn write_paragraph_with_numpr() {
        use s1_model::{ListFormat, ListInfo};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

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
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Bullet item"))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:numPr>"));
        assert!(xml.contains(r#"<w:ilvl w:val="0"/>"#));
        assert!(xml.contains(r#"<w:numId w:val="1"/>"#));
        assert!(xml.contains("</w:numPr>"));
    }

    #[test]
    fn write_paragraph_with_numpr_level_2() {
        use s1_model::{ListFormat, ListInfo};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level: 2,
                num_format: ListFormat::LowerRoman,
                num_id: 5,
                start: None,
            }),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:ilvl w:val="2"/>"#));
        assert!(xml.contains(r#"<w:numId w:val="5"/>"#));
    }

    #[test]
    fn write_hyperlink_external() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("https://example.com".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let mut text = Node::new(text_id, NodeType::Text);
        text.text_content = Some("Click here".into());
        doc.insert_node(run_id, 0, text).unwrap();

        let (xml, _rels, hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:hyperlink r:id="rHyp1">"#));
        assert!(xml.contains("Click here"));
        assert!(xml.contains("</w:hyperlink>"));
        assert_eq!(hyp_rels.len(), 1);
        assert_eq!(hyp_rels[0].rid, "rHyp1");
        assert_eq!(hyp_rels[0].target, "https://example.com");
    }

    #[test]
    fn write_hyperlink_internal_anchor() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String("#_MyBookmark".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let mut text = Node::new(text_id, NodeType::Text);
        text.text_content = Some("Go to bookmark".into());
        doc.insert_node(run_id, 0, text).unwrap();

        let (xml, _rels, hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:hyperlink w:anchor="_MyBookmark">"#));
        assert!(xml.contains("Go to bookmark"));
        // Internal anchor should NOT create a relationship entry
        assert!(hyp_rels.is_empty());
    }

    #[test]
    fn write_hyperlink_groups_consecutive_runs() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        // Two runs with the same URL should be grouped
        for (idx, text_str) in ["Click ", "here"].iter().enumerate() {
            let run_id = doc.next_id();
            let mut run = Node::new(run_id, NodeType::Run);
            run.attributes.set(
                AttributeKey::HyperlinkUrl,
                AttributeValue::String("https://example.com".into()),
            );
            doc.insert_node(para_id, idx, run).unwrap();

            let text_id = doc.next_id();
            let mut text = Node::new(text_id, NodeType::Text);
            text.text_content = Some(text_str.to_string());
            doc.insert_node(run_id, 0, text).unwrap();
        }

        let (xml, _rels, hyp_rels) = write_document_xml(&doc);
        // Should produce ONE hyperlink wrapping TWO runs
        assert_eq!(xml.matches("<w:hyperlink").count(), 1);
        assert_eq!(xml.matches("</w:hyperlink>").count(), 1);
        assert!(xml.contains("Click "));
        assert!(xml.contains("here"));
        assert_eq!(hyp_rels.len(), 1);
    }

    #[test]
    fn write_bookmark_start_end() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let bk_start_id = doc.next_id();
        let mut bk_start = Node::new(bk_start_id, NodeType::BookmarkStart);
        bk_start.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String("MyBookmark".into()),
        );
        doc.insert_node(para_id, 0, bk_start).unwrap();

        // A run between bookmarks
        let run_id = doc.next_id();
        let run = Node::new(run_id, NodeType::Run);
        doc.insert_node(para_id, 1, run).unwrap();
        let text_id = doc.next_id();
        let mut text = Node::new(text_id, NodeType::Text);
        text.text_content = Some("Bookmarked text".into());
        doc.insert_node(run_id, 0, text).unwrap();

        let bk_end_id = doc.next_id();
        let bk_end = Node::new(bk_end_id, NodeType::BookmarkEnd);
        doc.insert_node(para_id, 2, bk_end).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:bookmarkStart w:id="#));
        assert!(xml.contains(r#"w:name="MyBookmark"/>"#));
        assert!(xml.contains(r#"<w:bookmarkEnd w:id="#));
        assert!(xml.contains("Bookmarked text"));
    }

    #[test]
    fn write_tab_stops() {
        use s1_model::{TabAlignment, TabLeader, TabStop};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::TabStops,
            AttributeValue::TabStops(vec![
                TabStop {
                    position: 36.0, // 36 points = 720 twips
                    alignment: TabAlignment::Left,
                    leader: TabLeader::None,
                },
                TabStop {
                    position: 72.0, // 72 points = 1440 twips
                    alignment: TabAlignment::Right,
                    leader: TabLeader::Dot,
                },
                TabStop {
                    position: 108.0,
                    alignment: TabAlignment::Center,
                    leader: TabLeader::Dash,
                },
                TabStop {
                    position: 144.0,
                    alignment: TabAlignment::Decimal,
                    leader: TabLeader::Underscore,
                },
            ]),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:tabs>"));
        assert!(xml.contains(r#"<w:tab w:val="left" w:pos="720"/>"#));
        assert!(xml.contains(r#"<w:tab w:val="right" w:pos="1440" w:leader="dot"/>"#));
        assert!(xml.contains(r#"<w:tab w:val="center" w:pos="2160" w:leader="hyphen"/>"#));
        assert!(xml.contains(r#"<w:tab w:val="decimal" w:pos="2880" w:leader="underscore"/>"#));
        assert!(xml.contains("</w:tabs>"));
    }

    #[test]
    fn write_paragraph_borders() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);

        let borders = Borders {
            top: Some(BorderSide {
                style: BorderStyle::Single,
                width: 1.0,
                color: Color::new(0, 0, 0),
                spacing: 0.0,
            }),
            bottom: Some(BorderSide {
                style: BorderStyle::Double,
                width: 2.0,
                color: Color::new(255, 0, 0),
                spacing: 0.0,
            }),
            left: None,
            right: None,
        };
        para.attributes.set(
            AttributeKey::ParagraphBorders,
            AttributeValue::Borders(borders),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:pBdr>"));
        assert!(xml.contains(r#"<w:top w:val="single""#));
        assert!(xml.contains(r#"w:color="000000""#));
        assert!(xml.contains(r#"<w:bottom w:val="double""#));
        assert!(xml.contains(r#"w:color="FF0000""#));
        assert!(xml.contains("</w:pBdr>"));
    }

    #[test]
    fn write_paragraph_shading() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::Background,
            AttributeValue::Color(Color::new(255, 255, 0)),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:shd w:val="clear" w:fill="FFFF00"/>"#));
    }

    #[test]
    fn write_suppress_auto_hyphens() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::SuppressAutoHyphens,
            AttributeValue::Bool(true),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:suppressAutoHyphens/>"));
    }

    #[test]
    fn write_character_spacing() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes.set(
            AttributeKey::FontSpacing,
            AttributeValue::Float(2.0), // 2 points = 40 twips
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let mut text = Node::new(text_id, NodeType::Text);
        text.text_content = Some("Spaced".into());
        doc.insert_node(run_id, 0, text).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:spacing w:val="40"/>"#));
    }

    #[test]
    fn write_comment_range() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        // CommentStart
        let cs_id = doc.next_id();
        let mut cs = Node::new(cs_id, NodeType::CommentStart);
        cs.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("42".into()));
        doc.insert_node(para_id, 0, cs).unwrap();

        // Run
        let run_id = doc.next_id();
        let run = Node::new(run_id, NodeType::Run);
        doc.insert_node(para_id, 1, run).unwrap();
        let text_id = doc.next_id();
        let mut text = Node::new(text_id, NodeType::Text);
        text.text_content = Some("Text".into());
        doc.insert_node(run_id, 0, text).unwrap();

        // CommentEnd
        let ce_id = doc.next_id();
        let mut ce = Node::new(ce_id, NodeType::CommentEnd);
        ce.attributes
            .set(AttributeKey::CommentId, AttributeValue::String("42".into()));
        doc.insert_node(para_id, 2, ce).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:commentRangeStart w:id="42"/>"#));
        assert!(xml.contains(r#"<w:commentRangeEnd w:id="42"/>"#));
        assert!(xml.contains(r#"<w:commentReference w:id="42"/>"#));
    }

    #[test]
    fn write_toc_sdt() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create TOC node with max level 2
        let toc_id = doc.next_id();
        let mut toc = Node::new(toc_id, NodeType::TableOfContents);
        toc.attributes
            .set(AttributeKey::TocMaxLevel, AttributeValue::Int(2));
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

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);

        // Should produce SDT wrapper
        assert!(xml.contains("<w:sdt>"));
        assert!(xml.contains(r#"<w:docPartGallery w:val="Table of Contents"/>"#));
        assert!(xml.contains("</w:sdtContent>"));
        assert!(xml.contains("</w:sdt>"));

        // Field code should reflect max level (quotes are XML-escaped)
        assert!(xml.contains(r#"TOC \o &quot;1-2&quot; \h \z \u"#));

        // Cached entry text should appear
        assert!(xml.contains("Chapter One"));
    }

    #[test]
    fn write_toc_empty_no_entries() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let toc_id = doc.next_id();
        let toc = Node::new(toc_id, NodeType::TableOfContents);
        doc.insert_node(body_id, 0, toc).unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);

        // SDT should still be written even with no entries
        assert!(xml.contains("<w:sdt>"));
        assert!(xml.contains(r#"fldCharType="begin"#));
        assert!(xml.contains(r#"fldCharType="end"#));
    }

    // ─── Track changes writing tests ────────────────────────────────

    #[test]
    fn write_ins_basic() {
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
            AttributeValue::String("John".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2024-01-01T12:00:00Z".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "inserted"))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:ins w:id="1""#));
        assert!(xml.contains(r#"w:author="John""#));
        assert!(xml.contains(r#"w:date="2024-01-01T12:00:00Z""#));
        assert!(xml.contains("inserted"));
        assert!(xml.contains("</w:ins>"));
        // Should use <w:t>, not <w:delText>
        assert!(xml.contains("<w:t xml:space=\"preserve\">inserted</w:t>"));
    }

    #[test]
    fn write_del_basic() {
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
            AttributeValue::String("Jane".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "deleted"))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains(r#"<w:del w:id="2""#));
        assert!(xml.contains(r#"w:author="Jane""#));
        assert!(xml.contains("</w:del>"));
        // Should use <w:delText>, not <w:t>
        assert!(xml.contains("<w:delText xml:space=\"preserve\">deleted</w:delText>"));
        assert!(!xml.contains("<w:t xml:space=\"preserve\">deleted</w:t>"));
    }

    #[test]
    fn write_rpr_change() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Bold, AttributeValue::Bool(true));
        run.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("FormatChange".into()),
        );
        run.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(3));
        run.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Bob".into()),
        );
        run.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2024-01-03T09:00:00Z".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "reformatted"))
            .unwrap();

        let (xml, _rels, _hyp_rels) = write_document_xml(&doc);
        assert!(xml.contains("<w:b/>"));
        assert!(xml.contains(r#"<w:rPrChange w:id="3""#));
        assert!(xml.contains(r#"w:author="Bob""#));
        assert!(xml.contains(r#"w:date="2024-01-03T09:00:00Z""#));
        assert!(xml.contains("</w:rPrChange>"));
        // FormatChange should NOT wrap in <w:ins> or <w:del>
        assert!(!xml.contains("<w:ins"));
        assert!(!xml.contains("<w:del"));
    }

    #[test]
    fn write_floating_image_anchor() {
        let mut doc = DocumentModel::new();
        let para_id = doc.next_id();
        doc.insert_node(
            doc.body_id().unwrap(),
            0,
            Node::new(para_id, NodeType::Paragraph),
        )
        .unwrap();

        // Create a Run to hold the image
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        // Create floating image node
        let img_id = doc.next_id();
        let mut img = Node::new(img_id, NodeType::Image);
        let media_id = doc.media_mut().insert(
            "image/png",
            vec![0x89, 0x50, 0x4E, 0x47],
            Some("float.png".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        img.attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(200.0));
        img.attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(150.0));
        img.attributes.set(
            AttributeKey::ImagePositionType,
            AttributeValue::String("anchor".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String("square".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageHorizontalOffset,
            AttributeValue::Int(914400),
        );
        img.attributes.set(
            AttributeKey::ImageVerticalOffset,
            AttributeValue::Int(457200),
        );
        img.attributes.set(
            AttributeKey::ImageHorizontalRelativeFrom,
            AttributeValue::String("column".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageVerticalRelativeFrom,
            AttributeValue::String("paragraph".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageDistanceFromText,
            AttributeValue::String("45720,45720,114300,114300".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageAltText,
            AttributeValue::String("Float test".to_string()),
        );
        doc.insert_node(para_id, 0, img).unwrap();

        let (xml, image_rels, _) = write_document_xml(&doc);

        // Should use wp:anchor, not wp:inline
        assert!(xml.contains("<wp:anchor"), "should contain wp:anchor");
        assert!(!xml.contains("<wp:inline"), "should not contain wp:inline");

        // Check positioning
        assert!(xml.contains(r#"relativeFrom="column">"#));
        assert!(xml.contains("<wp:posOffset>914400</wp:posOffset>"));
        assert!(xml.contains(r#"relativeFrom="paragraph">"#));
        assert!(xml.contains("<wp:posOffset>457200</wp:posOffset>"));

        // Check wrap type
        assert!(xml.contains("<wp:wrapSquare"));

        // Check distances
        assert!(xml.contains(r#"distT="45720""#));
        assert!(xml.contains(r#"distL="114300""#));

        // Check image relationship
        assert_eq!(image_rels.len(), 1);
    }

    #[test]
    fn write_shape_roundtrip_raw_xml() {
        let mut doc = DocumentModel::new();
        let para_id = doc.next_id();
        doc.insert_node(
            doc.body_id().unwrap(),
            0,
            Node::new(para_id, NodeType::Paragraph),
        )
        .unwrap();

        let shape_id = doc.next_id();
        let mut shape = Node::new(shape_id, NodeType::Drawing);
        shape.attributes.set(
            AttributeKey::ShapeType,
            AttributeValue::String("rect".to_string()),
        );
        shape
            .attributes
            .set(AttributeKey::ShapeWidth, AttributeValue::Float(200.0));
        shape
            .attributes
            .set(AttributeKey::ShapeHeight, AttributeValue::Float(100.0));
        shape.attributes.set(
            AttributeKey::ShapeFillColor,
            AttributeValue::String("FF0000".to_string()),
        );
        let raw =
            r##"<w:pict><v:rect style="width:200pt;height:100pt" fillcolor="#FF0000"/></w:pict>"##;
        shape.attributes.set(
            AttributeKey::ShapeRawXml,
            AttributeValue::String(raw.to_string()),
        );
        doc.insert_node(para_id, 0, shape).unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        // Should contain the raw VML wrapped in a run
        assert!(xml.contains("<w:r><w:pict>"), "should wrap shape in run");
        assert!(xml.contains("v:rect"), "should preserve shape element");
        assert!(xml.contains("fillcolor"), "should preserve fill color");
    }

    #[test]
    fn write_inline_image_default() {
        let mut doc = DocumentModel::new();
        let para_id = doc.next_id();
        doc.insert_node(
            doc.body_id().unwrap(),
            0,
            Node::new(para_id, NodeType::Paragraph),
        )
        .unwrap();

        let img_id = doc.next_id();
        let mut img = Node::new(img_id, NodeType::Image);
        let media_id = doc.media_mut().insert(
            "image/png",
            vec![0x89, 0x50],
            Some("inline.png".to_string()),
        );
        img.attributes.set(
            AttributeKey::ImageMediaId,
            AttributeValue::MediaId(media_id),
        );
        img.attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(100.0));
        img.attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(100.0));
        // No ImagePositionType set — should default to inline
        doc.insert_node(para_id, 0, img).unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        // Should use wp:inline, not wp:anchor
        assert!(xml.contains("<wp:inline"), "should contain wp:inline");
        assert!(!xml.contains("<wp:anchor"), "should not contain wp:anchor");
    }

    #[test]
    fn write_drawing_xml_roundtrip() {
        // Test that a Drawing node with DrawingML raw XML is written correctly
        let mut doc = DocumentModel::new();
        let para_id = doc.next_id();
        doc.insert_node(
            doc.body_id().unwrap(),
            0,
            Node::new(para_id, NodeType::Paragraph),
        )
        .unwrap();

        let drawing_id = doc.next_id();
        let mut drawing = Node::new(drawing_id, NodeType::Drawing);
        drawing.attributes.set(
            AttributeKey::ShapeType,
            AttributeValue::String("drawing".to_string()),
        );
        let raw = r#"<w:drawing><wp:inline><wp:extent cx="5486400" cy="3200400"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="rId8"/></a:graphicData></a:graphic></wp:inline></w:drawing>"#;
        drawing.attributes.set(
            AttributeKey::ShapeRawXml,
            AttributeValue::String(raw.to_string()),
        );
        doc.insert_node(para_id, 0, drawing).unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        // Should wrap the drawing XML in a run
        assert!(
            xml.contains("<w:r><w:drawing>"),
            "should wrap drawing in run"
        );
        assert!(xml.contains("c:chart"), "should preserve chart reference");
        assert!(
            xml.contains("</w:drawing></w:r>"),
            "should close run after drawing"
        );
    }

    #[test]
    fn write_move_to_revision() {
        let mut doc = DocumentModel::new();
        let para_id = doc.next_id();
        doc.insert_node(
            doc.body_id().unwrap(),
            0,
            Node::new(para_id, NodeType::Paragraph),
        )
        .unwrap();

        let run_id = doc.next_id();
        let mut run_node = Node::new(run_id, NodeType::Run);
        run_node.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("MoveTo".to_string()),
        );
        run_node
            .attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(5));
        run_node.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Alice".to_string()),
        );
        doc.insert_node(para_id, 0, run_node).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "moved"))
            .unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        assert!(
            xml.contains(r#"<w:moveTo w:id="5""#),
            "should output moveTo wrapper: {xml}"
        );
        assert!(xml.contains(r#"w:author="Alice""#), "should include author");
        assert!(xml.contains("</w:moveTo>"), "should close moveTo");
        // MoveTo should use <w:t>, not <w:delText>
        assert!(
            xml.contains("<w:t xml:space=\"preserve\">moved</w:t>"),
            "moveTo should use w:t"
        );
    }

    #[test]
    fn write_move_from_revision() {
        let mut doc = DocumentModel::new();
        let para_id = doc.next_id();
        doc.insert_node(
            doc.body_id().unwrap(),
            0,
            Node::new(para_id, NodeType::Paragraph),
        )
        .unwrap();

        let run_id = doc.next_id();
        let mut run_node = Node::new(run_id, NodeType::Run);
        run_node.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("MoveFrom".to_string()),
        );
        run_node
            .attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(6));
        run_node.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Bob".to_string()),
        );
        doc.insert_node(para_id, 0, run_node).unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "moved away"))
            .unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        assert!(
            xml.contains(r#"<w:moveFrom w:id="6""#),
            "should output moveFrom wrapper: {xml}"
        );
        assert!(xml.contains("</w:moveFrom>"), "should close moveFrom");
        // MoveFrom should use <w:delText> like Delete
        assert!(
            xml.contains("<w:delText xml:space=\"preserve\">moved away</w:delText>"),
            "moveFrom should use w:delText: {xml}"
        );
    }

    #[test]
    fn write_ppr_change() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::Alignment,
            AttributeValue::Alignment(Alignment::Center),
        );
        para.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("PropertyChange".into()),
        );
        para.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(50));
        para.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Alice".into()),
        );
        para.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-01-01T12:00:00Z".into()),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        assert!(
            xml.contains(
                r#"<w:pPrChange w:id="50" w:author="Alice" w:date="2026-01-01T12:00:00Z">"#
            ),
            "should output pPrChange: {xml}"
        );
        assert!(xml.contains("</w:pPrChange>"), "should close pPrChange");
    }

    #[test]
    fn write_tcpr_change() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        doc.insert_node(table_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();

        let cell_id = doc.next_id();
        let mut cell = Node::new(cell_id, NodeType::TableCell);
        cell.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("PropertyChange".into()),
        );
        cell.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(60));
        cell.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Bob".into()),
        );
        cell.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-02-15T08:00:00Z".into()),
        );
        doc.insert_node(row_id, 0, cell).unwrap();

        // Add a paragraph inside the cell (required for valid DOCX)
        let para_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        assert!(
            xml.contains(
                r#"<w:tcPrChange w:id="60" w:author="Bob" w:date="2026-02-15T08:00:00Z">"#
            ),
            "should output tcPrChange: {xml}"
        );
    }

    #[test]
    fn write_trpr_change() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        let row_id = doc.next_id();
        let mut row = Node::new(row_id, NodeType::TableRow);
        row.attributes
            .set(AttributeKey::TableHeaderRow, AttributeValue::Bool(true));
        row.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("PropertyChange".into()),
        );
        row.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(70));
        row.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Carol".into()),
        );
        row.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-03-10T10:00:00Z".into()),
        );
        doc.insert_node(table_id, 0, row).unwrap();

        // Add a cell + paragraph inside
        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        assert!(
            xml.contains("<w:tblHeader/>"),
            "should output tblHeader: {xml}"
        );
        assert!(
            xml.contains(
                r#"<w:trPrChange w:id="70" w:author="Carol" w:date="2026-03-10T10:00:00Z">"#
            ),
            "should output trPrChange: {xml}"
        );
    }

    #[test]
    fn write_tblpr_change() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let table_id = doc.next_id();
        let mut table = Node::new(table_id, NodeType::Table);
        table.attributes.set(
            AttributeKey::TableAlignment,
            AttributeValue::Alignment(Alignment::Center),
        );
        table.attributes.set(
            AttributeKey::RevisionType,
            AttributeValue::String("PropertyChange".into()),
        );
        table
            .attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(80));
        table.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String("Dave".into()),
        );
        table.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String("2026-03-12T14:00:00Z".into()),
        );
        doc.insert_node(body_id, 0, table).unwrap();

        // Add row/cell/paragraph
        let row_id = doc.next_id();
        doc.insert_node(table_id, 0, Node::new(row_id, NodeType::TableRow))
            .unwrap();
        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let (xml, _, _) = write_document_xml(&doc);

        assert!(
            xml.contains(
                r#"<w:tblPrChange w:id="80" w:author="Dave" w:date="2026-03-12T14:00:00Z">"#
            ),
            "should output tblPrChange: {xml}"
        );
    }

    #[test]
    fn empty_run_is_omitted_from_xml() {
        // A run with no text children and no revision attributes should produce
        // no <w:r> element in the output.
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Empty run (no children at all)
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let (xml, _, _) = write_document_xml(&doc);
        assert!(
            !xml.contains("<w:r>"),
            "empty run should be omitted from XML, got: {xml}"
        );
    }

    #[test]
    fn run_with_empty_text_node_is_omitted() {
        // A run whose only child is a Text node with empty string should be omitted.
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "")).unwrap();

        let (xml, _, _) = write_document_xml(&doc);
        assert!(
            !xml.contains("<w:r>"),
            "run with empty text should be omitted, got: {xml}"
        );
    }

    #[test]
    fn run_with_text_is_preserved() {
        let doc = make_simple_doc("Keep me");
        let (xml, _, _) = write_document_xml(&doc);
        assert!(
            xml.contains("<w:r>"),
            "run with text should be preserved: {xml}"
        );
        assert!(xml.contains("Keep me"));
    }

    #[test]
    fn empty_run_with_revision_is_preserved() {
        // A run with revision tracking but no text should still be emitted.
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
        doc.insert_node(para_id, 0, run).unwrap();

        let (xml, _, _) = write_document_xml(&doc);
        assert!(
            xml.contains("<w:r>"),
            "revision run should be preserved even without text: {xml}"
        );
    }
}
