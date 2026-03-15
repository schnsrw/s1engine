//! Parse `word/document.xml` — the main document content.
//!
//! Handles paragraphs, runs, text, breaks, tabs, tables, and images.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{
    AttributeKey, AttributeMap, AttributeValue, DocumentModel, FieldType, Node, NodeId, NodeType,
    NumberingDefinitions,
};

use crate::error::DocxError;
use crate::property_parser::{
    parse_cell_properties, parse_row_properties, parse_run_properties, parse_table_properties,
};
use crate::section_parser::{parse_section_properties, RawSectionProperties};
use crate::xml_util::{emu_to_points, get_attr, mime_for_extension};

/// Context passed through the parser for resolving images.
struct ParseContext<'a> {
    /// rId → target path (from word/_rels/document.xml.rels)
    rels: &'a HashMap<String, String>,
    /// target path → raw bytes (from word/media/*)
    media: &'a HashMap<String, Vec<u8>>,
    /// Numbering definitions for resolving list info.
    numbering: &'a NumberingDefinitions,
}

/// Parse `word/document.xml` into the document model.
///
/// Returns any raw section properties found (both inline in paragraph properties
/// and the final body-level sectPr). The reader uses these to resolve header/footer
/// rIds to NodeIds.
pub fn parse_document_xml(
    xml: &str,
    doc: &mut DocumentModel,
    rels: &HashMap<String, String>,
    media: &HashMap<String, Vec<u8>>,
    numbering: &NumberingDefinitions,
) -> Result<Vec<RawSectionProperties>, DocxError> {
    let ctx = ParseContext {
        rels,
        media,
        numbering,
    };
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut raw_sections = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"body" => {
                raw_sections = parse_body(&mut reader, doc, &ctx)?;
                break;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(raw_sections)
}

/// Parse `<w:body>` contents. Returns raw section properties for rId resolution.
fn parse_body(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
) -> Result<Vec<RawSectionProperties>, DocxError> {
    let body_id = doc
        .body_id()
        .ok_or_else(|| DocxError::InvalidStructure("No body node in model".into()))?;

    let mut child_index = 0;
    let mut raw_sections: Vec<RawSectionProperties> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"p" => {
                        let inline_sect = parse_paragraph(reader, doc, body_id, child_index, ctx)?;
                        if let Some(raw) = inline_sect {
                            // Mark the paragraph with its section index
                            let section_idx = raw_sections.len() as i64;
                            if let Some(para_node) = doc
                                .node(body_id)
                                .and_then(|b| b.children.get(child_index).copied())
                            {
                                if let Some(node) = doc.node_mut(para_node) {
                                    node.attributes.set(
                                        AttributeKey::SectionIndex,
                                        AttributeValue::Int(section_idx),
                                    );
                                }
                            }
                            raw_sections.push(raw);
                        }
                        child_index += 1;
                    }
                    b"tbl" => {
                        parse_table(reader, doc, body_id, child_index, ctx)?;
                        child_index += 1;
                    }
                    b"sdt" => {
                        if parse_sdt_toc(reader, doc, body_id, child_index, ctx)? {
                            child_index += 1;
                        }
                    }
                    b"sectPr" => {
                        // Final section (direct child of body)
                        let raw = parse_section_properties(reader)?;
                        raw_sections.push(raw);
                    }
                    b"ins" => {
                        let rev = extract_revision_attrs(&e, "Insert");
                        parse_tracked_block(
                            reader,
                            doc,
                            body_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"ins",
                        )?;
                    }
                    b"del" => {
                        let rev = extract_revision_attrs(&e, "Delete");
                        parse_tracked_block(
                            reader,
                            doc,
                            body_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"del",
                        )?;
                    }
                    b"moveTo" => {
                        let rev = extract_revision_attrs(&e, "MoveTo");
                        parse_tracked_block(
                            reader,
                            doc,
                            body_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"moveTo",
                        )?;
                    }
                    b"moveFrom" => {
                        let rev = extract_revision_attrs(&e, "MoveFrom");
                        parse_tracked_block(
                            reader,
                            doc,
                            body_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"moveFrom",
                        )?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                // Skip move-related range markers at body level
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"moveToRangeStart"
                    | b"moveToRangeEnd"
                    | b"moveFromRangeStart"
                    | b"moveFromRangeEnd" => {
                        // Silently skip — these are range markers for moved content
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"body" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(raw_sections)
}

/// Parse block-level content (paragraphs, tables) into a parent container.
///
/// This is used by header/footer parsing which shares the same block-level
/// content model as the body.
pub(crate) fn parse_block_content(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    parent_id: NodeId,
    rels: &HashMap<String, String>,
    media: &HashMap<String, Vec<u8>>,
    numbering: &NumberingDefinitions,
    end_tag: &[u8],
) -> Result<(), DocxError> {
    let ctx = ParseContext {
        rels,
        media,
        numbering,
    };
    let mut child_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"p" => {
                        let _inline_sect =
                            parse_paragraph(reader, doc, parent_id, child_index, &ctx)?;
                        child_index += 1;
                    }
                    b"tbl" => {
                        parse_table(reader, doc, parent_id, child_index, &ctx)?;
                        child_index += 1;
                    }
                    b"ins" => {
                        let rev = extract_revision_attrs(&e, "Insert");
                        parse_tracked_block(
                            reader,
                            doc,
                            parent_id,
                            &mut child_index,
                            &ctx,
                            &rev,
                            b"ins",
                        )?;
                    }
                    b"del" => {
                        let rev = extract_revision_attrs(&e, "Delete");
                        parse_tracked_block(
                            reader,
                            doc,
                            parent_id,
                            &mut child_index,
                            &ctx,
                            &rev,
                            b"del",
                        )?;
                    }
                    b"moveTo" => {
                        let rev = extract_revision_attrs(&e, "MoveTo");
                        parse_tracked_block(
                            reader,
                            doc,
                            parent_id,
                            &mut child_index,
                            &ctx,
                            &rev,
                            b"moveTo",
                        )?;
                    }
                    b"moveFrom" => {
                        let rev = extract_revision_attrs(&e, "MoveFrom");
                        parse_tracked_block(
                            reader,
                            doc,
                            parent_id,
                            &mut child_index,
                            &ctx,
                            &rev,
                            b"moveFrom",
                        )?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Parse `<w:p>` — a paragraph. Returns any inline sectPr found in pPr.
fn parse_paragraph(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    parent_id: NodeId,
    index: usize,
    ctx: &ParseContext,
) -> Result<Option<RawSectionProperties>, DocxError> {
    let para_id = doc.next_id();
    doc.insert_node(parent_id, index, Node::new(para_id, NodeType::Paragraph))
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;

    let mut child_index = 0;
    let mut inline_section: Option<RawSectionProperties> = None;
    let mut field_state = FieldState::None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"pPr" => {
                        let (mut attrs, sect) = parse_paragraph_properties_with_section(reader)?;
                        // Resolve list format from numbering definitions
                        resolve_list_info(&mut attrs, ctx.numbering);
                        if let Some(node) = doc.node_mut(para_id) {
                            node.attributes = attrs;
                        }
                        inline_section = sect;
                    }
                    b"r" => {
                        parse_run(
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                            &mut field_state,
                        )?;
                    }
                    // Simple fields (e.g., page number)
                    b"fldSimple" => {
                        parse_fld_simple(&e, reader, doc, para_id, &mut child_index, ctx)?;
                    }
                    // Hyperlinks contain runs with a URL target
                    b"hyperlink" => {
                        parse_hyperlink_runs(
                            &e,
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                            &mut field_state,
                        )?;
                    }
                    // Bookmark start/end
                    b"bookmarkStart" => {
                        let bk_id = doc.next_id();
                        let mut bk_node = Node::new(bk_id, NodeType::BookmarkStart);
                        if let Some(name) = get_attr(&e, b"name") {
                            bk_node
                                .attributes
                                .set(AttributeKey::BookmarkName, AttributeValue::String(name));
                        }
                        doc.insert_node(para_id, child_index, bk_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                        skip_element(reader)?;
                    }
                    b"bookmarkEnd" => {
                        let bk_id = doc.next_id();
                        let bk_node = Node::new(bk_id, NodeType::BookmarkEnd);
                        doc.insert_node(para_id, child_index, bk_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                        skip_element(reader)?;
                    }
                    b"commentRangeStart" => {
                        let crs_id = doc.next_id();
                        let mut crs_node = Node::new(crs_id, NodeType::CommentStart);
                        if let Some(id) = get_attr(&e, b"id") {
                            crs_node
                                .attributes
                                .set(AttributeKey::CommentId, AttributeValue::String(id));
                        }
                        doc.insert_node(para_id, child_index, crs_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                        skip_element(reader)?;
                    }
                    b"commentRangeEnd" => {
                        let cre_id = doc.next_id();
                        let mut cre_node = Node::new(cre_id, NodeType::CommentEnd);
                        if let Some(id) = get_attr(&e, b"id") {
                            cre_node
                                .attributes
                                .set(AttributeKey::CommentId, AttributeValue::String(id));
                        }
                        doc.insert_node(para_id, child_index, cre_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                        skip_element(reader)?;
                    }
                    // mc:AlternateContent can appear as direct child of paragraph
                    b"AlternateContent" => {
                        parse_alternate_content_into_paragraph(
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                        )?;
                    }
                    // Inline equation: <m:oMath>
                    b"oMath" => {
                        let raw_xml = capture_element_xml(reader, &e)?;
                        let eq_id = doc.next_id();
                        let mut eq_node = Node::new(eq_id, NodeType::Equation);
                        eq_node.attributes.set(
                            AttributeKey::EquationSource,
                            AttributeValue::String(raw_xml),
                        );
                        doc.insert_node(para_id, child_index, eq_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                    }
                    // Display (block) equation: <m:oMathPara>
                    b"oMathPara" => {
                        let raw_xml = capture_element_xml(reader, &e)?;
                        let eq_id = doc.next_id();
                        let mut eq_node = Node::new(eq_id, NodeType::Equation);
                        eq_node.attributes.set(
                            AttributeKey::EquationSource,
                            AttributeValue::String(raw_xml),
                        );
                        doc.insert_node(para_id, child_index, eq_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                    }
                    b"ins" => {
                        let rev = extract_revision_attrs(&e, "Insert");
                        parse_tracked_inline_runs(
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            &mut field_state,
                            b"ins",
                        )?;
                    }
                    b"del" => {
                        let rev = extract_revision_attrs(&e, "Delete");
                        parse_tracked_inline_runs(
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            &mut field_state,
                            b"del",
                        )?;
                    }
                    b"moveTo" => {
                        let rev = extract_revision_attrs(&e, "MoveTo");
                        parse_tracked_inline_runs(
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            &mut field_state,
                            b"moveTo",
                        )?;
                    }
                    b"moveFrom" => {
                        let rev = extract_revision_attrs(&e, "MoveFrom");
                        parse_tracked_inline_runs(
                            reader,
                            doc,
                            para_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            &mut field_state,
                            b"moveFrom",
                        )?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"bookmarkStart" => {
                        let bk_id = doc.next_id();
                        let mut bk_node = Node::new(bk_id, NodeType::BookmarkStart);
                        if let Some(bk_name) = get_attr(&e, b"name") {
                            bk_node
                                .attributes
                                .set(AttributeKey::BookmarkName, AttributeValue::String(bk_name));
                        }
                        doc.insert_node(para_id, child_index, bk_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                    }
                    b"bookmarkEnd" => {
                        let bk_id = doc.next_id();
                        let bk_node = Node::new(bk_id, NodeType::BookmarkEnd);
                        doc.insert_node(para_id, child_index, bk_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                    }
                    b"commentRangeStart" => {
                        let crs_id = doc.next_id();
                        let mut crs_node = Node::new(crs_id, NodeType::CommentStart);
                        if let Some(id) = get_attr(&e, b"id") {
                            crs_node
                                .attributes
                                .set(AttributeKey::CommentId, AttributeValue::String(id));
                        }
                        doc.insert_node(para_id, child_index, crs_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                    }
                    b"commentRangeEnd" => {
                        let cre_id = doc.next_id();
                        let mut cre_node = Node::new(cre_id, NodeType::CommentEnd);
                        if let Some(id) = get_attr(&e, b"id") {
                            cre_node
                                .attributes
                                .set(AttributeKey::CommentId, AttributeValue::String(id));
                        }
                        doc.insert_node(para_id, child_index, cre_node)
                            .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                        child_index += 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"p" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(inline_section)
}

/// Parse paragraph properties, also handling an inline `w:sectPr` if present.
fn parse_paragraph_properties_with_section(
    reader: &mut Reader<&[u8]>,
) -> Result<(AttributeMap, Option<RawSectionProperties>), DocxError> {
    // We need to parse pPr ourselves to catch sectPr within it
    let mut attrs = AttributeMap::new();
    let mut sect: Option<RawSectionProperties> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"sectPr" => {
                        sect = Some(parse_section_properties(reader)?);
                    }
                    _ => {
                        // Re-wrap into a mini XML string and use property_parser.
                        // Instead, handle known pPr children inline.
                        // For simplicity, delegate to a sub-parse for pPr content.
                        parse_ppr_child(&e, reader, &mut attrs)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"sectPr" => {
                        // Empty sectPr — use defaults
                        sect = Some(RawSectionProperties {
                            props: s1_model::SectionProperties::default(),
                            hf_refs: Vec::new(),
                        });
                    }
                    _ => {
                        parse_ppr_child_empty(&e, &mut attrs);
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"pPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok((attrs, sect))
}

/// Handle a Start event child of `<w:pPr>` (not sectPr).
fn parse_ppr_child(
    e: &quick_xml::events::BytesStart<'_>,
    reader: &mut Reader<&[u8]>,
    attrs: &mut AttributeMap,
) -> Result<(), DocxError> {
    use crate::property_parser;
    use crate::xml_util::{get_val, is_toggle_on};

    let name = e.local_name().as_ref().to_vec();
    match name.as_slice() {
        b"pStyle" => {
            if let Some(val) = get_val(e) {
                attrs.set(AttributeKey::StyleId, AttributeValue::String(val));
            }
            skip_element(reader)?;
        }
        b"jc" => {
            if let Some(val) = get_val(e) {
                let alignment = match val.as_str() {
                    "center" => Some(s1_model::Alignment::Center),
                    "right" | "end" => Some(s1_model::Alignment::Right),
                    "both" | "distribute" => Some(s1_model::Alignment::Justify),
                    _ => Some(s1_model::Alignment::Left),
                };
                if let Some(a) = alignment {
                    attrs.set(AttributeKey::Alignment, AttributeValue::Alignment(a));
                }
            }
            skip_element(reader)?;
        }
        b"numPr" => {
            if let Some(list_info) = property_parser::parse_num_pr(reader)? {
                attrs.set(AttributeKey::ListInfo, AttributeValue::ListInfo(list_info));
            }
        }
        b"spacing" => {
            property_parser::parse_spacing_attrs(e, attrs);
            skip_element(reader)?;
        }
        b"ind" => {
            property_parser::parse_indent_attrs(e, attrs);
            skip_element(reader)?;
        }
        b"keepNext" => {
            if is_toggle_on(e) {
                attrs.set(AttributeKey::KeepWithNext, AttributeValue::Bool(true));
            }
            skip_element(reader)?;
        }
        b"keepLines" => {
            if is_toggle_on(e) {
                attrs.set(AttributeKey::KeepLinesTogether, AttributeValue::Bool(true));
            }
            skip_element(reader)?;
        }
        b"pageBreakBefore" => {
            if is_toggle_on(e) {
                attrs.set(AttributeKey::PageBreakBefore, AttributeValue::Bool(true));
            }
            skip_element(reader)?;
        }
        b"tabs" => {
            let tab_stops = property_parser::parse_tabs_pub(reader)?;
            if !tab_stops.is_empty() {
                attrs.set(AttributeKey::TabStops, AttributeValue::TabStops(tab_stops));
            }
        }
        b"pBdr" => {
            let borders = property_parser::parse_borders(reader, b"pBdr")?;
            attrs.set(
                AttributeKey::ParagraphBorders,
                AttributeValue::Borders(borders),
            );
        }
        _ => {
            skip_element(reader)?;
        }
    }
    Ok(())
}

/// Handle an Empty event child of `<w:pPr>` (not sectPr).
fn parse_ppr_child_empty(e: &quick_xml::events::BytesStart<'_>, attrs: &mut AttributeMap) {
    use crate::xml_util::{get_val, is_toggle_on};

    let name = e.local_name().as_ref().to_vec();
    match name.as_slice() {
        b"pStyle" => {
            if let Some(val) = get_val(e) {
                attrs.set(AttributeKey::StyleId, AttributeValue::String(val));
            }
        }
        b"jc" => {
            if let Some(val) = get_val(e) {
                let alignment = match val.as_str() {
                    "center" => Some(s1_model::Alignment::Center),
                    "right" | "end" => Some(s1_model::Alignment::Right),
                    "both" | "distribute" => Some(s1_model::Alignment::Justify),
                    _ => Some(s1_model::Alignment::Left),
                };
                if let Some(a) = alignment {
                    attrs.set(AttributeKey::Alignment, AttributeValue::Alignment(a));
                }
            }
        }
        b"spacing" => {
            crate::property_parser::parse_spacing_attrs(e, attrs);
        }
        b"ind" => {
            crate::property_parser::parse_indent_attrs(e, attrs);
        }
        b"keepNext" => {
            if is_toggle_on(e) {
                attrs.set(AttributeKey::KeepWithNext, AttributeValue::Bool(true));
            }
        }
        b"keepLines" => {
            if is_toggle_on(e) {
                attrs.set(AttributeKey::KeepLinesTogether, AttributeValue::Bool(true));
            }
        }
        b"pageBreakBefore" => {
            if is_toggle_on(e) {
                attrs.set(AttributeKey::PageBreakBefore, AttributeValue::Bool(true));
            }
        }
        b"shd" => {
            use crate::xml_util::get_attr;
            if let Some(fill) = get_attr(e, b"fill") {
                if fill != "auto" {
                    if let Some(color) = s1_model::Color::from_hex(&fill) {
                        attrs.set(AttributeKey::Background, AttributeValue::Color(color));
                    }
                }
            }
        }
        _ => {}
    }
}

/// Parse `<w:tbl>` — a table.
fn parse_table(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    parent_id: NodeId,
    index: usize,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    let table_id = doc.next_id();
    doc.insert_node(parent_id, index, Node::new(table_id, NodeType::Table))
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;

    let mut row_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tblPr" => {
                        let attrs = parse_table_properties(reader)?;
                        if let Some(node) = doc.node_mut(table_id) {
                            node.attributes = attrs;
                        }
                    }
                    b"tblGrid" => {
                        skip_element(reader)?;
                    }
                    b"tr" => {
                        parse_table_row(reader, doc, table_id, row_index, ctx)?;
                        row_index += 1;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"tbl" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Parse `<w:tr>` — a table row.
fn parse_table_row(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    table_id: NodeId,
    index: usize,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    let row_id = doc.next_id();
    doc.insert_node(table_id, index, Node::new(row_id, NodeType::TableRow))
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;

    let mut cell_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"trPr" => {
                        let attrs = parse_row_properties(reader)?;
                        if !attrs.is_empty() {
                            if let Some(node) = doc.node_mut(row_id) {
                                for (key, value) in attrs.iter() {
                                    node.attributes.set(key.clone(), value.clone());
                                }
                            }
                        }
                    }
                    b"tc" => {
                        parse_table_cell(reader, doc, row_id, cell_index, ctx)?;
                        cell_index += 1;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"tr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Parse `<w:tc>` — a table cell.
fn parse_table_cell(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    row_id: NodeId,
    index: usize,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    let cell_id = doc.next_id();
    doc.insert_node(row_id, index, Node::new(cell_id, NodeType::TableCell))
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;

    let mut child_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tcPr" => {
                        let attrs = parse_cell_properties(reader)?;
                        if let Some(node) = doc.node_mut(cell_id) {
                            node.attributes = attrs;
                        }
                    }
                    b"p" => {
                        parse_paragraph(reader, doc, cell_id, child_index, ctx)?;
                        child_index += 1;
                    }
                    b"tbl" => {
                        parse_table(reader, doc, cell_id, child_index, ctx)?;
                        child_index += 1;
                    }
                    b"ins" => {
                        let rev = extract_revision_attrs(&e, "Insert");
                        parse_tracked_block(
                            reader,
                            doc,
                            cell_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"ins",
                        )?;
                    }
                    b"del" => {
                        let rev = extract_revision_attrs(&e, "Delete");
                        parse_tracked_block(
                            reader,
                            doc,
                            cell_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"del",
                        )?;
                    }
                    b"moveTo" => {
                        let rev = extract_revision_attrs(&e, "MoveTo");
                        parse_tracked_block(
                            reader,
                            doc,
                            cell_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"moveTo",
                        )?;
                    }
                    b"moveFrom" => {
                        let rev = extract_revision_attrs(&e, "MoveFrom");
                        parse_tracked_block(
                            reader,
                            doc,
                            cell_id,
                            &mut child_index,
                            ctx,
                            &rev,
                            b"moveFrom",
                        )?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"tc" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Intermediate representation for run content before building nodes.
enum RunContent {
    Text(String),
    Break(NodeType),
    Tab,
    Image(ImageInfo),
    Field(FieldType, String),
    /// A shape/drawing element with its raw VML/XML for round-trip preservation.
    Shape(ShapeInfo),
    /// A non-image drawing element (chart, DrawingML shape, etc.) captured as raw XML.
    DrawingXml(String),
    /// An inline footnote reference (w:id value).
    FootnoteRef(String),
    /// An inline endnote reference (w:id value).
    EndnoteRef(String),
}

/// Information about a parsed shape.
struct ShapeInfo {
    /// Shape type (e.g., "rect", "roundrect", "oval", "line", "shape")
    shape_type: String,
    /// Width in points (if available)
    width_pts: Option<f64>,
    /// Height in points (if available)
    height_pts: Option<f64>,
    /// Fill color hex (if available)
    fill_color: Option<String>,
    /// Stroke color hex (if available)
    stroke_color: Option<String>,
    /// The raw XML of the entire <w:pict> element for round-trip fidelity
    raw_xml: String,
}

/// State machine for tracking complex field parsing (fldChar begin/separate/end)
/// across one or more runs within a paragraph.
enum FieldState {
    /// Not inside a complex field.
    None,
    /// Saw fldChar begin, waiting for instrText.
    Begin,
    /// Saw instrText with the field instruction code.
    HasInstr(String),
    /// Saw fldChar separate — display value follows (skip it).
    Separate(String),
}

/// Extracted image info from a `<w:drawing>` element.
struct ImageInfo {
    /// Relationship target path (e.g., "media/image1.png")
    rel_target: String,
    width_pts: Option<f64>,
    height_pts: Option<f64>,
    alt_text: Option<String>,
    /// Whether this is an inline or anchor (floating) image
    is_floating: bool,
    /// Text wrap type for floating images
    wrap_type: Option<String>,
    /// Horizontal offset in EMUs (for floating images)
    h_offset: Option<i64>,
    /// Vertical offset in EMUs (for floating images)
    v_offset: Option<i64>,
    /// Horizontal relative-from (for floating images)
    h_relative_from: Option<String>,
    /// Vertical relative-from (for floating images)
    v_relative_from: Option<String>,
    /// Distance from text: (distT, distB, distL, distR) in EMUs
    dist_from_text: Option<(i64, i64, i64, i64)>,
}

/// Handle a `<w:fldChar>` element (whether self-closing or not).
///
/// Processes the `fldCharType` attribute to drive the field state machine
/// and creates a `RunContent::Field` when the field ends.
fn handle_fld_char(
    e: &quick_xml::events::BytesStart<'_>,
    field_state: &mut FieldState,
    content: &mut Vec<RunContent>,
) {
    let fld_type = get_attr(e, b"fldCharType");
    match fld_type.as_deref() {
        Some("begin") => {
            *field_state = FieldState::Begin;
        }
        Some("separate") => {
            // Move instruction text into Separate state
            let instr = match std::mem::replace(field_state, FieldState::None) {
                FieldState::HasInstr(s) => s,
                _ => String::new(),
            };
            *field_state = FieldState::Separate(instr);
        }
        Some("end") => {
            // Extract instruction and create Field content
            let instr = match std::mem::replace(field_state, FieldState::None) {
                FieldState::Separate(s) => s,
                FieldState::HasInstr(s) => s,
                _ => String::new(),
            };
            if !instr.is_empty() {
                let ft = parse_field_instruction(&instr);
                content.push(RunContent::Field(ft, instr.trim().to_string()));
            }
        }
        _ => {}
    }
}

/// Parse `<w:r>` — a run of text with formatting.
fn parse_run(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    ctx: &ParseContext,
    field_state: &mut FieldState,
) -> Result<(), DocxError> {
    let mut run_attrs = AttributeMap::new();
    let mut content: Vec<RunContent> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"rPr" => {
                        run_attrs = parse_run_properties(reader)?;
                    }
                    b"t" | b"delText" => {
                        let text = read_text_content_tag(reader, &name)?;
                        if !text.is_empty() {
                            // When in the "separate" phase of a complex field,
                            // the <w:t> holds the display value — skip it.
                            if !matches!(field_state, FieldState::Separate(_)) {
                                content.push(RunContent::Text(text));
                            }
                        }
                    }
                    b"drawing" => match parse_drawing(reader, ctx)? {
                        Some(DrawingResult::Image(info)) => {
                            content.push(RunContent::Image(*info));
                        }
                        Some(DrawingResult::RawXml(xml)) => {
                            content.push(RunContent::DrawingXml(xml));
                        }
                        None => {}
                    },
                    // instrText holds the field instruction (e.g., " PAGE ")
                    b"instrText" => {
                        let instr = read_instr_text_content(reader)?;
                        if !instr.is_empty() {
                            *field_state = FieldState::HasInstr(instr);
                        }
                    }
                    // mc:AlternateContent wraps drawing in some DOCX files
                    b"AlternateContent" => {
                        parse_alternate_content(reader, &mut content, ctx)?;
                    }
                    // fldChar can appear as non-self-closing element in some DOCX producers
                    b"fldChar" => {
                        handle_fld_char(&e, field_state, &mut content);
                        // Consume the closing </w:fldChar> tag
                        skip_element(reader)?;
                    }
                    // VML shapes: <w:pict><v:shape>...</v:shape></w:pict>
                    b"pict" => {
                        if let Some(shape) = parse_pict(reader)? {
                            content.push(RunContent::Shape(shape));
                        }
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"br" => {
                        let break_type = get_attr(&e, b"type");
                        let node_type = match break_type.as_deref() {
                            Some("page") => NodeType::PageBreak,
                            Some("column") => NodeType::ColumnBreak,
                            _ => NodeType::LineBreak,
                        };
                        content.push(RunContent::Break(node_type));
                    }
                    b"tab" => {
                        content.push(RunContent::Tab);
                    }
                    b"t" => {
                        // Self-closing <w:t/> — empty text, skip
                    }
                    b"fldChar" => {
                        handle_fld_char(&e, field_state, &mut content);
                    }
                    b"footnoteReference" => {
                        if let Some(id) = get_attr(&e, b"id") {
                            content.push(RunContent::FootnoteRef(id));
                        }
                    }
                    b"endnoteReference" => {
                        if let Some(id) = get_attr(&e, b"id") {
                            content.push(RunContent::EndnoteRef(id));
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"r" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    // Build nodes from collected content.
    // Text items go into runs; breaks/tabs/images go directly into the paragraph.
    let mut texts: Vec<String> = Vec::new();

    for item in content {
        match item {
            RunContent::Text(text) => {
                texts.push(text);
            }
            RunContent::Break(node_type) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                let break_id = doc.next_id();
                doc.insert_node(para_id, *child_index, Node::new(break_id, node_type))
                    .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                *child_index += 1;
            }
            RunContent::Tab => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                let tab_id = doc.next_id();
                doc.insert_node(para_id, *child_index, Node::new(tab_id, NodeType::Tab))
                    .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                *child_index += 1;
            }
            RunContent::Image(info) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                insert_image_node(doc, para_id, child_index, &info, ctx)?;
            }
            RunContent::Field(field_type, instr) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                let field_id = doc.next_id();
                let mut field_node = Node::new(field_id, NodeType::Field);
                field_node.attributes.set(
                    AttributeKey::FieldType,
                    AttributeValue::FieldType(field_type),
                );
                field_node
                    .attributes
                    .set(AttributeKey::FieldCode, AttributeValue::String(instr));
                doc.insert_node(para_id, *child_index, field_node)
                    .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                *child_index += 1;
            }
            RunContent::Shape(info) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                insert_shape_node(doc, para_id, child_index, &info)?;
            }
            RunContent::DrawingXml(raw_xml) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                insert_drawing_xml_node(doc, para_id, child_index, &raw_xml)?;
            }
            RunContent::FootnoteRef(id) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                let ref_id = doc.next_id();
                let mut ref_node = Node::new(ref_id, NodeType::FootnoteRef);
                ref_node
                    .attributes
                    .set(AttributeKey::FootnoteNumber, AttributeValue::String(id));
                doc.insert_node(para_id, *child_index, ref_node)
                    .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                *child_index += 1;
            }
            RunContent::EndnoteRef(id) => {
                flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;
                let ref_id = doc.next_id();
                let mut ref_node = Node::new(ref_id, NodeType::EndnoteRef);
                ref_node
                    .attributes
                    .set(AttributeKey::EndnoteNumber, AttributeValue::String(id));
                doc.insert_node(para_id, *child_index, ref_node)
                    .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
                *child_index += 1;
            }
        }
    }

    // Flush remaining text
    flush_texts_to_run(&mut texts, &run_attrs, doc, para_id, child_index)?;

    Ok(())
}

/// Read text inside `<w:instrText>...</w:instrText>`.
fn read_instr_text_content(reader: &mut Reader<&[u8]>) -> Result<String, DocxError> {
    let mut text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Text(e)) => {
                text.push_str(&e.unescape().map_err(|e| DocxError::Xml(format!("{e}")))?);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"instrText" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(text)
}

/// Parse `<mc:AlternateContent>` inside a run — descend into `<mc:Choice>` to find
/// `<w:drawing>` elements. Skips `<mc:Fallback>` content to avoid duplicates.
/// This is how some DOCX producers (Google Docs, etc.) wrap inline images.
fn parse_alternate_content(
    reader: &mut Reader<&[u8]>,
    content: &mut Vec<RunContent>,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    let mut depth = 1u32;
    let mut in_fallback = false;
    let mut fallback_depth = 0u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"Fallback" => {
                        in_fallback = true;
                        fallback_depth = depth;
                    }
                    b"drawing" if !in_fallback => {
                        match parse_drawing(reader, ctx)? {
                            Some(DrawingResult::Image(info)) => {
                                content.push(RunContent::Image(*info));
                            }
                            Some(DrawingResult::RawXml(xml)) => {
                                content.push(RunContent::DrawingXml(xml));
                            }
                            None => {}
                        }
                        depth -= 1; // parse_drawing consumed the </drawing> end tag
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(_)) => {}
            Ok(Event::End(_)) => {
                if in_fallback && depth == fallback_depth {
                    in_fallback = false;
                }
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

/// Parse `<mc:AlternateContent>` as a direct child of a paragraph.
///
/// Descends into `<mc:Choice>` to find `<w:drawing>` elements and inserts
/// any found images directly as children of the paragraph. Skips `<mc:Fallback>`.
fn parse_alternate_content_into_paragraph(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    let mut depth = 1u32;
    let mut in_fallback = false;
    let mut fallback_depth = 0u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"Fallback" => {
                        in_fallback = true;
                        fallback_depth = depth;
                    }
                    b"drawing" if !in_fallback => {
                        match parse_drawing(reader, ctx)? {
                            Some(DrawingResult::Image(info)) => {
                                insert_image_node(doc, para_id, child_index, &info, ctx)?;
                            }
                            Some(DrawingResult::RawXml(xml)) => {
                                insert_drawing_xml_node(doc, para_id, child_index, &xml)?;
                            }
                            None => {}
                        }
                        depth -= 1; // parse_drawing consumed the </drawing> end tag
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(_)) => {}
            Ok(Event::End(_)) => {
                if in_fallback && depth == fallback_depth {
                    in_fallback = false;
                }
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

/// Result of parsing a `<w:drawing>` element.
enum DrawingResult {
    /// An image was found (has `<a:blip r:embed="..."/>`).
    Image(Box<ImageInfo>),
    /// A non-image drawing (chart, DrawingML shape, etc.) captured as raw XML.
    RawXml(String),
}

/// Parse `<w:drawing>` — extract image info from inline or anchor drawings.
///
/// If the drawing contains an image blip, returns `DrawingResult::Image`.
/// Otherwise (charts, DrawingML shapes, etc.), captures the raw XML for
/// round-trip preservation and returns `DrawingResult::RawXml`.
fn parse_drawing(
    reader: &mut Reader<&[u8]>,
    ctx: &ParseContext,
) -> Result<Option<DrawingResult>, DocxError> {
    let mut embed_rid: Option<String> = None;
    let mut width_pts: Option<f64> = None;
    let mut height_pts: Option<f64> = None;
    let mut alt_text: Option<String> = None;
    let mut is_floating = false;
    let mut wrap_type: Option<String> = None;
    let mut h_offset: Option<i64> = None;
    let mut v_offset: Option<i64> = None;
    let mut h_relative_from: Option<String> = None;
    let mut v_relative_from: Option<String> = None;
    let mut dist_from_text: Option<(i64, i64, i64, i64)> = None;

    // Capture raw XML alongside normal parsing for non-image drawings
    let mut raw_xml = String::from("<w:drawing>");

    let mut depth = 1u32;
    // Track whether we're inside positionH or positionV to capture posOffset text
    let mut in_position_h = false;
    let mut in_position_v = false;
    let mut in_pos_offset = false;
    let mut pos_offset_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                // Capture raw XML
                raw_xml.push('<');
                raw_xml.push_str(std::str::from_utf8(&e).unwrap_or("?"));
                raw_xml.push('>');

                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"anchor" => {
                        is_floating = true;
                        // Extract distance attributes
                        let dist_t = get_attr(&e, b"distT")
                            .and_then(|s| s.parse::<i64>().ok())
                            .unwrap_or(0);
                        let dist_b = get_attr(&e, b"distB")
                            .and_then(|s| s.parse::<i64>().ok())
                            .unwrap_or(0);
                        let dist_l = get_attr(&e, b"distL")
                            .and_then(|s| s.parse::<i64>().ok())
                            .unwrap_or(0);
                        let dist_r = get_attr(&e, b"distR")
                            .and_then(|s| s.parse::<i64>().ok())
                            .unwrap_or(0);
                        if dist_t != 0 || dist_b != 0 || dist_l != 0 || dist_r != 0 {
                            dist_from_text = Some((dist_t, dist_b, dist_l, dist_r));
                        }
                    }
                    b"positionH" => {
                        in_position_h = true;
                        h_relative_from = get_attr(&e, b"relativeFrom");
                    }
                    b"positionV" => {
                        in_position_v = true;
                        v_relative_from = get_attr(&e, b"relativeFrom");
                    }
                    b"posOffset" => {
                        in_pos_offset = true;
                        pos_offset_text.clear();
                    }
                    // Wrap type detection
                    b"wrapSquare" => {
                        wrap_type = Some("square".to_string());
                    }
                    b"wrapTight" => {
                        wrap_type = Some("tight".to_string());
                    }
                    b"wrapThrough" => {
                        wrap_type = Some("through".to_string());
                    }
                    b"wrapTopAndBottom" => {
                        wrap_type = Some("topAndBottom".to_string());
                    }
                    b"wrapNone" => {
                        wrap_type = Some("none".to_string());
                    }
                    b"extent" => {
                        if let Some(cx) = get_attr(&e, b"cx") {
                            width_pts = emu_to_points(&cx);
                        }
                        if let Some(cy) = get_attr(&e, b"cy") {
                            height_pts = emu_to_points(&cy);
                        }
                    }
                    b"docPr" => {
                        if let Some(descr) = get_attr(&e, b"descr") {
                            if !descr.is_empty() {
                                alt_text = Some(descr);
                            }
                        }
                    }
                    b"blip" => {
                        if let Some(rid) = get_attr(&e, b"embed") {
                            embed_rid = Some(rid);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                // Capture raw XML text
                raw_xml.push_str(std::str::from_utf8(t.as_ref()).unwrap_or(""));

                if in_pos_offset {
                    if let Ok(s) = t.unescape() {
                        pos_offset_text.push_str(&s);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                // Capture raw XML
                raw_xml.push('<');
                raw_xml.push_str(std::str::from_utf8(&e).unwrap_or("?"));
                raw_xml.push_str("/>");

                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"wrapSquare" => {
                        wrap_type = Some("square".to_string());
                    }
                    b"wrapTight" => {
                        wrap_type = Some("tight".to_string());
                    }
                    b"wrapThrough" => {
                        wrap_type = Some("through".to_string());
                    }
                    b"wrapTopAndBottom" => {
                        wrap_type = Some("topAndBottom".to_string());
                    }
                    b"wrapNone" => {
                        wrap_type = Some("none".to_string());
                    }
                    b"extent" => {
                        if let Some(cx) = get_attr(&e, b"cx") {
                            width_pts = emu_to_points(&cx);
                        }
                        if let Some(cy) = get_attr(&e, b"cy") {
                            height_pts = emu_to_points(&cy);
                        }
                    }
                    b"docPr" => {
                        if let Some(descr) = get_attr(&e, b"descr") {
                            if !descr.is_empty() {
                                alt_text = Some(descr);
                            }
                        }
                    }
                    b"blip" => {
                        if let Some(rid) = get_attr(&e, b"embed") {
                            embed_rid = Some(rid);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"posOffset" => {
                        if in_pos_offset {
                            if let Ok(val) = pos_offset_text.trim().parse::<i64>() {
                                if in_position_h {
                                    h_offset = Some(val);
                                } else if in_position_v {
                                    v_offset = Some(val);
                                }
                            }
                            in_pos_offset = false;
                        }
                    }
                    b"positionH" => {
                        in_position_h = false;
                    }
                    b"positionV" => {
                        in_position_v = false;
                    }
                    _ => {}
                }
                // Capture raw XML
                raw_xml.push_str("</");
                raw_xml.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or("?"));
                raw_xml.push('>');

                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::CData(ref cd)) => {
                raw_xml.push_str("<![CDATA[");
                raw_xml.push_str(std::str::from_utf8(cd.as_ref()).unwrap_or(""));
                raw_xml.push_str("]]>");
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    // If we found an image blip, resolve and return as image
    if let Some(rid) = embed_rid {
        if let Some(target) = ctx.rels.get(&rid) {
            return Ok(Some(DrawingResult::Image(Box::new(ImageInfo {
                rel_target: target.clone(),
                width_pts,
                height_pts,
                alt_text,
                is_floating,
                wrap_type,
                h_offset,
                v_offset,
                h_relative_from,
                v_relative_from,
                dist_from_text,
            }))));
        }
    }

    // No image blip — this is a chart, DrawingML shape, or other non-image drawing.
    // Preserve the raw XML for round-trip fidelity.
    Ok(Some(DrawingResult::RawXml(raw_xml)))
}

/// Create an Image node from parsed drawing info and store media in the model.
fn insert_image_node(
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    info: &ImageInfo,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    // Look up media bytes
    let data = match ctx.media.get(&info.rel_target) {
        Some(d) => d.clone(),
        None => return Ok(()), // Media not found — skip silently
    };

    // Determine content type from extension
    let ext = info.rel_target.rsplit('.').next().unwrap_or("bin");
    let content_type = mime_for_extension(ext).unwrap_or("application/octet-stream");

    // Store in media store (dedup by content hash)
    let media_id = doc
        .media_mut()
        .insert(content_type, data, Some(info.rel_target.clone()));

    // Create Image node
    let image_id = doc.next_id();
    let mut image_node = Node::new(image_id, NodeType::Image);
    image_node.attributes.set(
        AttributeKey::ImageMediaId,
        AttributeValue::MediaId(media_id),
    );
    if let Some(w) = info.width_pts {
        image_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(w));
    }
    if let Some(h) = info.height_pts {
        image_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(h));
    }
    if let Some(ref alt) = info.alt_text {
        image_node.attributes.set(
            AttributeKey::ImageAltText,
            AttributeValue::String(alt.clone()),
        );
    }

    // Store floating image positioning data
    if info.is_floating {
        image_node.attributes.set(
            AttributeKey::ImagePositionType,
            AttributeValue::String("anchor".to_string()),
        );
        if let Some(ref wt) = info.wrap_type {
            image_node.attributes.set(
                AttributeKey::ImageWrapType,
                AttributeValue::String(wt.clone()),
            );
        }
        if let Some(ho) = info.h_offset {
            image_node
                .attributes
                .set(AttributeKey::ImageHorizontalOffset, AttributeValue::Int(ho));
        }
        if let Some(vo) = info.v_offset {
            image_node
                .attributes
                .set(AttributeKey::ImageVerticalOffset, AttributeValue::Int(vo));
        }
        if let Some(ref hrf) = info.h_relative_from {
            image_node.attributes.set(
                AttributeKey::ImageHorizontalRelativeFrom,
                AttributeValue::String(hrf.clone()),
            );
        }
        if let Some(ref vrf) = info.v_relative_from {
            image_node.attributes.set(
                AttributeKey::ImageVerticalRelativeFrom,
                AttributeValue::String(vrf.clone()),
            );
        }
        if let Some((dt, db, dl, dr)) = info.dist_from_text {
            image_node.attributes.set(
                AttributeKey::ImageDistanceFromText,
                AttributeValue::String(format!("{},{},{},{}", dt, db, dl, dr)),
            );
        }
    }

    doc.insert_node(para_id, *child_index, image_node)
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
    *child_index += 1;

    Ok(())
}

/// Parse `<w:pict>` containing VML shape elements.
///
/// Extracts shape type, dimensions, fill/stroke colors, and stores the raw XML
/// for round-trip preservation of shapes.
fn parse_pict(reader: &mut Reader<&[u8]>) -> Result<Option<ShapeInfo>, DocxError> {
    let mut shape_type: Option<String> = None;
    let mut width_pts: Option<f64> = None;
    let mut height_pts: Option<f64> = None;
    let mut fill_color: Option<String> = None;
    let mut stroke_color: Option<String> = None;
    let mut raw_parts: Vec<String> = Vec::new();
    let mut depth = 1u32;

    raw_parts.push("<w:pict>".to_string());

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = e.local_name().as_ref().to_vec();
                // Capture raw XML for round-trip
                raw_parts.push(format!(
                    "<{}>",
                    std::str::from_utf8(e.name().as_ref()).unwrap_or("?")
                ));

                match name.as_slice() {
                    b"shape" | b"rect" | b"roundrect" | b"oval" | b"line" | b"polyline"
                    | b"group" => {
                        let type_name = String::from_utf8_lossy(&name).to_string();
                        shape_type = Some(type_name);

                        // Try to extract style attribute for dimensions
                        if let Some(style) = get_attr(e, b"style") {
                            parse_vml_style(&style, &mut width_pts, &mut height_pts);
                        }
                        // Fill color from fillcolor attribute
                        if let Some(fc) = get_attr(e, b"fillcolor") {
                            fill_color = Some(fc.trim_start_matches('#').to_string());
                        }
                        // Stroke color from strokecolor attribute
                        if let Some(sc) = get_attr(e, b"strokecolor") {
                            stroke_color = Some(sc.trim_start_matches('#').to_string());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                raw_parts.push(format!(
                    "<{}/>",
                    std::str::from_utf8(e.name().as_ref()).unwrap_or("?")
                ));
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"shape" | b"rect" | b"roundrect" | b"oval" | b"line" => {
                        let type_name = String::from_utf8_lossy(&name).to_string();
                        shape_type = Some(type_name);
                        if let Some(style) = get_attr(e, b"style") {
                            parse_vml_style(&style, &mut width_pts, &mut height_pts);
                        }
                        if let Some(fc) = get_attr(e, b"fillcolor") {
                            fill_color = Some(fc.trim_start_matches('#').to_string());
                        }
                        if let Some(sc) = get_attr(e, b"strokecolor") {
                            stroke_color = Some(sc.trim_start_matches('#').to_string());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Ok(s) = t.unescape() {
                    raw_parts.push(s.to_string());
                }
            }
            Ok(Event::End(ref e)) => {
                raw_parts.push(format!(
                    "</{}>",
                    std::str::from_utf8(e.name().as_ref()).unwrap_or("?")
                ));
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

    // Only create a shape node if we actually found a shape element
    match shape_type {
        Some(st) => Ok(Some(ShapeInfo {
            shape_type: st,
            width_pts,
            height_pts,
            fill_color,
            stroke_color,
            raw_xml: raw_parts.join(""),
        })),
        None => Ok(None),
    }
}

/// Parse VML style string for width/height.
/// e.g., "width:100pt;height:50pt" or "width:2in;height:1in"
fn parse_vml_style(style: &str, width: &mut Option<f64>, height: &mut Option<f64>) {
    for part in style.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("width:") {
            *width = parse_vml_length(val.trim());
        } else if let Some(val) = part.strip_prefix("height:") {
            *height = parse_vml_length(val.trim());
        }
    }
}

/// Parse a VML length value to points.
fn parse_vml_length(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("pt") {
        v.trim().parse::<f64>().ok()
    } else if let Some(v) = s.strip_suffix("in") {
        v.trim().parse::<f64>().ok().map(|x| x * 72.0)
    } else if let Some(v) = s.strip_suffix("cm") {
        v.trim().parse::<f64>().ok().map(|x| x * 28.3465)
    } else if let Some(v) = s.strip_suffix("mm") {
        v.trim().parse::<f64>().ok().map(|x| x * 2.83465)
    } else if let Some(v) = s.strip_suffix("px") {
        v.trim().parse::<f64>().ok().map(|x| x * 0.75) // 96 DPI → 72 DPI
    } else {
        // Try bare number as points
        s.parse::<f64>().ok()
    }
}

/// Create a Drawing node from parsed shape info.
fn insert_shape_node(
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    info: &ShapeInfo,
) -> Result<(), DocxError> {
    let shape_id = doc.next_id();
    let mut shape_node = Node::new(shape_id, NodeType::Drawing);

    shape_node.attributes.set(
        AttributeKey::ShapeType,
        AttributeValue::String(info.shape_type.clone()),
    );
    if let Some(w) = info.width_pts {
        shape_node
            .attributes
            .set(AttributeKey::ShapeWidth, AttributeValue::Float(w));
    }
    if let Some(h) = info.height_pts {
        shape_node
            .attributes
            .set(AttributeKey::ShapeHeight, AttributeValue::Float(h));
    }
    if let Some(ref fc) = info.fill_color {
        shape_node.attributes.set(
            AttributeKey::ShapeFillColor,
            AttributeValue::String(fc.clone()),
        );
    }
    if let Some(ref sc) = info.stroke_color {
        shape_node.attributes.set(
            AttributeKey::ShapeStrokeColor,
            AttributeValue::String(sc.clone()),
        );
    }
    // Store raw XML for round-trip preservation
    shape_node.attributes.set(
        AttributeKey::ShapeRawXml,
        AttributeValue::String(info.raw_xml.clone()),
    );

    doc.insert_node(para_id, *child_index, shape_node)
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
    *child_index += 1;

    Ok(())
}

/// Create a Drawing node from raw DrawingML XML (charts, shapes without image blip, etc.).
fn insert_drawing_xml_node(
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    raw_xml: &str,
) -> Result<(), DocxError> {
    let drawing_id = doc.next_id();
    let mut drawing_node = Node::new(drawing_id, NodeType::Drawing);

    drawing_node.attributes.set(
        AttributeKey::ShapeType,
        AttributeValue::String("drawing".to_string()),
    );
    drawing_node.attributes.set(
        AttributeKey::ShapeRawXml,
        AttributeValue::String(raw_xml.to_string()),
    );

    doc.insert_node(para_id, *child_index, drawing_node)
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
    *child_index += 1;

    Ok(())
}

/// Create a Run node with accumulated text content.
fn flush_texts_to_run(
    texts: &mut Vec<String>,
    run_attrs: &AttributeMap,
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
) -> Result<(), DocxError> {
    if texts.is_empty() {
        return Ok(());
    }

    let combined: String = texts.drain(..).collect();
    if combined.is_empty() {
        return Ok(());
    }

    let run_id = doc.next_id();
    let mut run = Node::new(run_id, NodeType::Run);
    run.attributes = run_attrs.clone();
    doc.insert_node(para_id, *child_index, run)
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
    *child_index += 1;

    let text_id = doc.next_id();
    doc.insert_node(run_id, 0, Node::text(text_id, combined))
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;

    Ok(())
}

/// Read text content inside `<w:t>...</w:t>`.
#[allow(dead_code)]
fn read_text_content(reader: &mut Reader<&[u8]>) -> Result<String, DocxError> {
    let mut text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Text(e)) => {
                text.push_str(&e.unescape().map_err(|e| DocxError::Xml(format!("{e}")))?);
            }
            Ok(Event::CData(e)) => {
                text.push_str(std::str::from_utf8(&e).map_err(|e| DocxError::Xml(format!("{e}")))?);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"t" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(text)
}

/// Parse `<w:fldSimple>` — a simple field (e.g., page number).
fn parse_fld_simple(
    e: &quick_xml::events::BytesStart<'_>,
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    _ctx: &ParseContext,
) -> Result<(), DocxError> {
    // Get instruction text from w:instr attribute
    let instr = get_attr(e, b"instr").unwrap_or_default();
    let field_type = parse_field_instruction(&instr);

    // Create a Field node
    let field_id = doc.next_id();
    let mut field_node = Node::new(field_id, NodeType::Field);
    field_node.attributes.set(
        AttributeKey::FieldType,
        AttributeValue::FieldType(field_type),
    );
    field_node.attributes.set(
        AttributeKey::FieldCode,
        AttributeValue::String(instr.trim().to_string()),
    );
    doc.insert_node(para_id, *child_index, field_node)
        .map_err(|e| DocxError::InvalidStructure(format!("{e}")))?;
    *child_index += 1;

    // Skip the content (the displayed value which we don't need to store)
    skip_element(reader)?;

    Ok(())
}

/// Parse a field instruction string to determine the field type.
fn parse_field_instruction(instr: &str) -> FieldType {
    let trimmed = instr.trim().to_uppercase();
    if trimmed.starts_with("PAGE") {
        FieldType::PageNumber
    } else if trimmed.starts_with("NUMPAGES") || trimmed.starts_with("SECTIONPAGES") {
        FieldType::PageCount
    } else if trimmed.starts_with("DATE")
        || trimmed.starts_with("CREATEDATE")
        || trimmed.starts_with("SAVEDATE")
    {
        FieldType::Date
    } else if trimmed.starts_with("TIME") {
        FieldType::Time
    } else if trimmed.starts_with("FILENAME") {
        FieldType::FileName
    } else if trimmed.starts_with("AUTHOR") {
        FieldType::Author
    } else if trimmed.starts_with("TOC") {
        FieldType::TableOfContents
    } else {
        FieldType::Custom
    }
}

/// Parse `<w:hyperlink>` — resolve the relationship ID to a URL and
/// tag inner runs with the hyperlink URL attribute.
fn parse_hyperlink_runs(
    hyperlink_elem: &quick_xml::events::BytesStart<'_>,
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    ctx: &ParseContext,
    field_state: &mut FieldState,
) -> Result<(), DocxError> {
    // Resolve the r:id to a URL via relationships
    let url = get_attr(hyperlink_elem, b"id").and_then(|rid| ctx.rels.get(&rid).cloned());
    let tooltip = get_attr(hyperlink_elem, b"tooltip");
    let anchor = get_attr(hyperlink_elem, b"anchor");

    // Build effective URL: external link (from rels) or internal anchor
    let effective_url = match (&url, &anchor) {
        (Some(u), _) => Some(u.clone()),
        (None, Some(a)) => Some(format!("#{a}")),
        _ => None,
    };

    // Track where runs start so we can tag them with the hyperlink
    let start_index = *child_index;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"r" => {
                        parse_run(reader, doc, para_id, child_index, ctx, field_state)?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"hyperlink" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    // Tag all runs created inside this hyperlink with the URL
    if let Some(ref href) = effective_url {
        if let Some(para) = doc.node(para_id) {
            let children: Vec<NodeId> = para.children.clone();
            for &child_id in children.get(start_index..*child_index).unwrap_or(&[]) {
                if let Some(node) = doc.node_mut(child_id) {
                    node.attributes.set(
                        AttributeKey::HyperlinkUrl,
                        AttributeValue::String(href.clone()),
                    );
                    if let Some(ref tt) = tooltip {
                        node.attributes.set(
                            AttributeKey::HyperlinkTooltip,
                            AttributeValue::String(tt.clone()),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Resolve `ListInfo` placeholder format from numbering definitions.
fn resolve_list_info(attrs: &mut AttributeMap, numbering: &NumberingDefinitions) {
    if let Some(AttributeValue::ListInfo(info)) = attrs.get(&AttributeKey::ListInfo).cloned() {
        let num_format = numbering
            .resolve_format(info.num_id, info.level)
            .unwrap_or(info.num_format);
        let start = numbering.resolve_start(info.num_id, info.level);
        attrs.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(s1_model::ListInfo {
                level: info.level,
                num_format,
                num_id: info.num_id,
                start,
            }),
        );
    }
}

/// Try to parse an `<w:sdt>` as a Table of Contents.
///
/// If the SDT contains a `docPartGallery` with value "Table of Contents",
/// creates a `NodeType::TableOfContents` node with cached entry paragraphs.
/// Returns `true` if a TOC node was created, `false` if the SDT was skipped.
fn parse_sdt_toc(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    parent_id: NodeId,
    child_index: usize,
    ctx: &ParseContext,
) -> Result<bool, DocxError> {
    let mut is_toc = false;
    let mut in_sdt_pr = false;
    let mut in_sdt_content = false;
    let mut toc_id: Option<NodeId> = None;
    let mut toc_child_index = 0;
    let mut in_field = false; // track fldChar begin/separate/end

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"sdtPr" => {
                        in_sdt_pr = true;
                    }
                    b"docPartGallery" if in_sdt_pr => {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"val" {
                                let val = String::from_utf8_lossy(&attr.value);
                                if val.contains("Table of Contents") {
                                    is_toc = true;
                                }
                            }
                        }
                        skip_element(reader)?;
                    }
                    b"docPartObj" if in_sdt_pr => {
                        // Don't skip — descend into it to find docPartGallery
                    }
                    b"sdtContent" => {
                        in_sdt_content = true;
                        if is_toc {
                            // Create the TOC node
                            let id = doc.next_id();
                            let mut toc = Node::new(id, NodeType::TableOfContents);
                            toc.attributes.set(
                                AttributeKey::TocMaxLevel,
                                AttributeValue::Int(3), // default; may be updated from field code
                            );
                            let _ = doc.insert_node(parent_id, child_index, toc);
                            toc_id = Some(id);
                        }
                    }
                    b"p" if in_sdt_content && is_toc => {
                        if let Some(tid) = toc_id {
                            // Parse paragraph; check for fldChar to detect field code
                            // vs cached entries — skip field-code paragraphs
                            parse_sdt_toc_paragraph(
                                reader,
                                doc,
                                tid,
                                &mut toc_child_index,
                                &mut in_field,
                                ctx,
                            )?;
                        } else {
                            skip_element(reader)?;
                        }
                    }
                    b"p" if in_sdt_content && !is_toc => {
                        // Non-TOC SDT: parse content paragraphs as regular body content
                        let _ = parse_paragraph(reader, doc, parent_id, child_index, ctx)?;
                    }
                    _ if in_sdt_pr => {
                        skip_element(reader)?;
                    }
                    _ if in_sdt_content => {
                        skip_element(reader)?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) => {
                let local = e.local_name();
                let name = local.as_ref();
                match name {
                    b"sdtPr" => {
                        in_sdt_pr = false;
                    }
                    b"sdtContent" => {
                        in_sdt_content = false;
                    }
                    b"sdt" => break,
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                if name == b"docPartGallery" && in_sdt_pr {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"val" {
                            let val = String::from_utf8_lossy(&attr.value);
                            if val.contains("Table of Contents") {
                                is_toc = true;
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(toc_id.is_some())
}

/// Parse a paragraph inside an SDT TOC.
///
/// Field-code paragraphs (containing fldChar begin/separate/end) are consumed
/// but not added to the TOC node. Only cached entry paragraphs are kept.
fn parse_sdt_toc_paragraph(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    toc_id: NodeId,
    toc_child_index: &mut usize,
    _in_field: &mut bool,
    ctx: &ParseContext,
) -> Result<(), DocxError> {
    // We'll collect the paragraph XML events and check for field chars
    // Simple approach: parse as normal paragraph, then check if it contains field chars
    // Parse paragraph normally into the TOC, then remove field-code-only paragraphs
    let _inline_sect = parse_paragraph(reader, doc, toc_id, *toc_child_index, ctx)?;

    // Check if this paragraph is a field-code-only paragraph (fldChar begin/separate/end)
    // by inspecting the paragraph's children for Field nodes
    let para_id = doc
        .node(toc_id)
        .and_then(|n| n.children.get(*toc_child_index).copied());

    let _is_field_para = if let Some(pid) = para_id {
        // Check for field markers in the raw paragraph
        let para_node = doc.node(pid);
        if let Some(pn) = para_node {
            // If this paragraph has only runs with no meaningful text, and we're in a field,
            // skip it. Simple heuristic: check if paragraph text is empty
            let mut text = String::new();
            collect_para_text(doc, pid, &mut text);

            // Detect field begin/end via instrText content
            let has_instr = pn.children.iter().any(|&cid| {
                doc.node(cid)
                    .map(|n| n.node_type == NodeType::Field)
                    .unwrap_or(false)
            });

            if text.trim().is_empty() && pn.children.is_empty() {
                true // empty paragraph (likely field-only)
            } else {
                has_instr && text.trim().is_empty()
            }
        } else {
            false
        }
    } else {
        false
    };

    // Field begin/separate/end paragraphs get parsed but we check if they're empty
    // For simplicity: if the paragraph was parsed and has text, keep it
    if let Some(pid) = para_id {
        let mut text = String::new();
        collect_para_text(doc, pid, &mut text);

        // Keep paragraphs that have actual text content (TOC entries)
        // Remove field-only or empty paragraphs
        if text.trim().is_empty() {
            let _ = doc.remove_node(pid);
        } else {
            *toc_child_index += 1;
        }
    }

    Ok(())
}

/// Collect plain text from a paragraph (helper).
fn collect_para_text(doc: &DocumentModel, node_id: NodeId, out: &mut String) {
    if let Some(node) = doc.node(node_id) {
        if let Some(text) = &node.text_content {
            out.push_str(text);
        }
        let children: Vec<NodeId> = node.children.clone();
        for child_id in children {
            collect_para_text(doc, child_id, out);
        }
    }
}

/// Read text content inside a tag that ends with the given tag name.
///
/// Like `read_text_content` but matches on any end tag name (e.g., `t` or `delText`).
fn read_text_content_tag(reader: &mut Reader<&[u8]>, tag_name: &[u8]) -> Result<String, DocxError> {
    let mut text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Text(e)) => {
                text.push_str(&e.unescape().map_err(|e| DocxError::Xml(format!("{e}")))?);
            }
            Ok(Event::CData(e)) => {
                text.push_str(std::str::from_utf8(&e).map_err(|e| DocxError::Xml(format!("{e}")))?);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == tag_name => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(text)
}

/// Revision info extracted from tracked change elements (w:ins, w:del).
#[derive(Debug, Clone)]
struct RevisionInfo {
    /// Type of revision: "Insert" or "Delete".
    revision_type: String,
    /// Author of the revision, if present.
    author: Option<String>,
    /// Date of the revision, if present.
    date: Option<String>,
    /// Revision ID from `w:id` attribute.
    id: Option<i64>,
}

/// Extract revision attributes from a tracked change start element.
fn extract_revision_attrs(
    e: &quick_xml::events::BytesStart<'_>,
    revision_type: &str,
) -> RevisionInfo {
    let author = get_attr(e, b"author");
    let date = get_attr(e, b"date");
    let id = get_attr(e, b"id").and_then(|s| s.parse::<i64>().ok());
    RevisionInfo {
        revision_type: revision_type.to_string(),
        author,
        date,
        id,
    }
}

/// Apply revision attributes to a node.
fn apply_revision_attrs(node: &mut Node, rev: &RevisionInfo) {
    node.attributes.set(
        AttributeKey::RevisionType,
        AttributeValue::String(rev.revision_type.clone()),
    );
    if let Some(ref author) = rev.author {
        node.attributes.set(
            AttributeKey::RevisionAuthor,
            AttributeValue::String(author.clone()),
        );
    }
    if let Some(ref date) = rev.date {
        node.attributes.set(
            AttributeKey::RevisionDate,
            AttributeValue::String(date.clone()),
        );
    }
    if let Some(id) = rev.id {
        node.attributes
            .set(AttributeKey::RevisionId, AttributeValue::Int(id));
    }
}

/// Tag all Run children of a node with revision attributes.
fn tag_children_with_revision(doc: &mut DocumentModel, parent_id: NodeId, rev: &RevisionInfo) {
    let children: Vec<NodeId> = match doc.node(parent_id) {
        Some(n) => n.children.clone(),
        None => return,
    };
    for child_id in children {
        let is_run = doc
            .node(child_id)
            .map(|n| n.node_type == NodeType::Run)
            .unwrap_or(false);
        if is_run {
            if let Some(node) = doc.node_mut(child_id) {
                apply_revision_attrs(node, rev);
            }
        }
    }
}

/// Parse block-level tracked changes (w:ins or w:del containing paragraphs/tables).
/// Tags each paragraph's run children with revision attributes.
fn parse_tracked_block(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    parent_id: NodeId,
    child_index: &mut usize,
    ctx: &ParseContext,
    rev: &RevisionInfo,
    end_tag: &[u8],
) -> Result<(), DocxError> {
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"p" => {
                        let start_idx = *child_index;
                        parse_paragraph(reader, doc, parent_id, *child_index, ctx)?;
                        // Apply revision attributes to runs within this paragraph
                        if let Some(para_node_id) = doc
                            .node(parent_id)
                            .and_then(|p| p.children.get(start_idx).copied())
                        {
                            tag_children_with_revision(doc, para_node_id, rev);
                        }
                        *child_index += 1;
                    }
                    b"tbl" => {
                        parse_table(reader, doc, parent_id, *child_index, ctx)?;
                        *child_index += 1;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }
    Ok(())
}

/// Parse inline-level tracked changes (w:ins or w:del containing runs).
/// Tags each parsed run with revision attributes.
#[allow(clippy::too_many_arguments)]
fn parse_tracked_inline_runs(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    para_id: NodeId,
    child_index: &mut usize,
    ctx: &ParseContext,
    rev: &RevisionInfo,
    field_state: &mut FieldState,
    end_tag: &[u8],
) -> Result<(), DocxError> {
    let start_index = *child_index;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"r" => {
                        parse_run(reader, doc, para_id, child_index, ctx, field_state)?;
                    }
                    _ => {
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    // Tag all runs created inside this tracked change with revision attributes
    if let Some(para) = doc.node(para_id) {
        let children: Vec<NodeId> = para.children.clone();
        for &child_id in children.get(start_index..*child_index).unwrap_or(&[]) {
            if let Some(node) = doc.node_mut(child_id) {
                apply_revision_attrs(node, rev);
            }
        }
    }

    Ok(())
}

/// Capture an element and all its children as raw XML text.
///
/// The opening `<tag ...>` has already been consumed by the reader.
/// `start` is the `BytesStart` from that event. This function reads events
/// until the matching closing tag and returns the complete XML string
/// (including opening and closing tags).
fn capture_element_xml(
    reader: &mut Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
) -> Result<String, DocxError> {
    let mut xml = String::new();

    // Reconstruct the opening tag: <name attrs...>
    xml.push('<');
    xml.push_str(std::str::from_utf8(start).unwrap_or("?"));
    xml.push('>');

    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                xml.push('<');
                xml.push_str(std::str::from_utf8(e).unwrap_or("?"));
                xml.push('>');
            }
            Ok(Event::Empty(ref e)) => {
                xml.push('<');
                xml.push_str(std::str::from_utf8(e).unwrap_or("?"));
                xml.push_str("/>");
            }
            Ok(Event::Text(ref t)) => {
                // Use the raw escaped text to preserve XML entities
                xml.push_str(std::str::from_utf8(t.as_ref()).unwrap_or(""));
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                xml.push_str("</");
                xml.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or("?"));
                xml.push('>');
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::CData(ref cd)) => {
                xml.push_str("<![CDATA[");
                xml.push_str(std::str::from_utf8(cd.as_ref()).unwrap_or(""));
                xml.push_str("]]>");
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(xml)
}

/// Skip an element and all its children.
fn skip_element(reader: &mut Reader<&[u8]>) -> Result<(), DocxError> {
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
    use s1_model::{AttributeKey, AttributeValue};

    fn wrap_doc(body_content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
<w:body>{body_content}</w:body>
</w:document>"#
        )
    }

    /// Parse helper that passes empty rels/media (for tests without images).
    fn parse_doc(xml: &str, doc: &mut DocumentModel) {
        let rels = HashMap::new();
        let media = HashMap::new();
        let numbering = s1_model::NumberingDefinitions::default();
        parse_document_xml(xml, doc, &rels, &media, &numbering).unwrap();
    }

    #[test]
    fn parse_single_paragraph() {
        let xml = wrap_doc(r#"<w:p><w:r><w:t>Hello World</w:t></w:r></w:p>"#);
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);
        assert_eq!(doc.to_plain_text(), "Hello World");
    }

    #[test]
    fn parse_multiple_paragraphs() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:t>First</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second</w:t></w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);
        assert_eq!(doc.to_plain_text(), "First\nSecond");
    }

    #[test]
    fn parse_empty_paragraph() {
        let xml = wrap_doc(r#"<w:p></w:p>"#);
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1); // one empty paragraph
    }

    #[test]
    fn parse_bold_run() {
        let xml = wrap_doc(r#"<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r></w:p>"#);
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        // Find the run node
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(run.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(doc.to_plain_text(), "Bold");
    }

    #[test]
    fn parse_multiple_runs() {
        let xml = wrap_doc(
            r#"<w:p>
            <w:r><w:rPr><w:b/></w:rPr><w:t>Hello </w:t></w:r>
            <w:r><w:rPr><w:i/></w:rPr><w:t>World</w:t></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);
        assert_eq!(doc.to_plain_text(), "Hello World");

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 2); // two runs

        let run1 = doc.node(para.children[0]).unwrap();
        assert_eq!(run1.attributes.get_bool(&AttributeKey::Bold), Some(true));

        let run2 = doc.node(para.children[1]).unwrap();
        assert_eq!(run2.attributes.get_bool(&AttributeKey::Italic), Some(true));
    }

    #[test]
    fn parse_paragraph_alignment() {
        let xml = wrap_doc(
            r#"<w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:t>Centered</w:t></w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(
            para.attributes.get_alignment(&AttributeKey::Alignment),
            Some(s1_model::Alignment::Center)
        );
    }

    #[test]
    fn parse_line_break() {
        let xml = wrap_doc(r#"<w:p><w:r><w:t>Before</w:t><w:br/><w:t>After</w:t></w:r></w:p>"#);
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        // Should produce: Paragraph > Run("Before"), LineBreak, Run("After")
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(para.children.len(), 3);
        let child0 = doc.node(para.children[0]).unwrap();
        assert_eq!(child0.node_type, NodeType::Run);

        let child1 = doc.node(para.children[1]).unwrap();
        assert_eq!(child1.node_type, NodeType::LineBreak);

        let child2 = doc.node(para.children[2]).unwrap();
        assert_eq!(child2.node_type, NodeType::Run);
    }

    #[test]
    fn parse_page_break() {
        let xml = wrap_doc(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#);
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(para.children.len(), 1);
        let child = doc.node(para.children[0]).unwrap();
        assert_eq!(child.node_type, NodeType::PageBreak);
    }

    #[test]
    fn parse_tab() {
        let xml = wrap_doc(r#"<w:p><w:r><w:t>Col1</w:t><w:tab/><w:t>Col2</w:t></w:r></w:p>"#);
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(para.children.len(), 3); // Run, Tab, Run
        let child1 = doc.node(para.children[1]).unwrap();
        assert_eq!(child1.node_type, NodeType::Tab);
    }

    #[test]
    fn parse_paragraph_with_style_ref() {
        let xml = wrap_doc(
            r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(
            para.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading1")
        );
    }

    #[test]
    fn parse_unknown_elements_ignored() {
        // Unknown elements should be silently skipped, not cause errors
        let xml = wrap_doc(
            r#"<w:p>
            <w:bookmarkStart w:id="0" w:name="_GoBack"/>
            <w:r><w:t>Text</w:t></w:r>
            <w:bookmarkEnd w:id="0"/>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);
        assert_eq!(doc.to_plain_text(), "Text");
    }

    // ─── Table parsing tests ──────────────────────────────────────────

    #[test]
    fn parse_simple_table() {
        let xml = wrap_doc(
            r#"<w:tbl>
            <w:tr>
                <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
                <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
            </w:tr>
            <w:tr>
                <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
                <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
            </w:tr>
            </w:tbl>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        // Verify structure: Body > Table > 2 rows > 2 cells each > 1 paragraph each
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let table = doc.node(body.children[0]).unwrap();
        assert_eq!(table.node_type, NodeType::Table);
        assert_eq!(table.children.len(), 2);

        let row0 = doc.node(table.children[0]).unwrap();
        assert_eq!(row0.node_type, NodeType::TableRow);
        assert_eq!(row0.children.len(), 2);

        let cell00 = doc.node(row0.children[0]).unwrap();
        assert_eq!(cell00.node_type, NodeType::TableCell);

        // Text extraction should include all cell text
        let text = doc.to_plain_text();
        assert!(text.contains("A1"));
        assert!(text.contains("B2"));
    }

    #[test]
    fn parse_table_with_properties() {
        let xml = wrap_doc(
            r#"<w:tbl>
            <w:tblPr>
                <w:tblW w:w="9360" w:type="dxa"/>
                <w:jc w:val="center"/>
            </w:tblPr>
            <w:tblGrid><w:gridCol w:w="4680"/><w:gridCol w:w="4680"/></w:tblGrid>
            <w:tr>
                <w:tc>
                    <w:tcPr><w:tcW w:w="4680" w:type="dxa"/></w:tcPr>
                    <w:p><w:r><w:t>Cell</w:t></w:r></w:p>
                </w:tc>
            </w:tr>
            </w:tbl>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let table = doc.node(body.children[0]).unwrap();

        // Table width should be 468pt (9360 twips)
        match table.attributes.get(&AttributeKey::TableWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Fixed(pts))) => {
                assert!((*pts - 468.0).abs() < 0.01);
            }
            other => panic!("Expected TableWidth::Fixed, got {:?}", other),
        }

        // Table alignment
        assert_eq!(
            table
                .attributes
                .get_alignment(&AttributeKey::TableAlignment),
            Some(s1_model::Alignment::Center)
        );

        // Cell width
        let row = doc.node(table.children[0]).unwrap();
        let cell = doc.node(row.children[0]).unwrap();
        match cell.attributes.get(&AttributeKey::CellWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Fixed(pts))) => {
                assert!((*pts - 234.0).abs() < 0.01); // 4680 twips = 234pt
            }
            other => panic!("Expected CellWidth Fixed, got {:?}", other),
        }
    }

    #[test]
    fn parse_table_cell_merge() {
        let xml = wrap_doc(
            r#"<w:tbl>
            <w:tr>
                <w:tc>
                    <w:tcPr><w:gridSpan w:val="2"/></w:tcPr>
                    <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
                </w:tc>
            </w:tr>
            <w:tr>
                <w:tc>
                    <w:tcPr><w:vMerge w:val="restart"/></w:tcPr>
                    <w:p><w:r><w:t>Top</w:t></w:r></w:p>
                </w:tc>
                <w:tc>
                    <w:tcPr><w:vMerge/></w:tcPr>
                    <w:p/>
                </w:tc>
            </w:tr>
            </w:tbl>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let table = doc.node(body.children[0]).unwrap();

        // Row 0, Cell 0: gridSpan=2
        let row0 = doc.node(table.children[0]).unwrap();
        let cell = doc.node(row0.children[0]).unwrap();
        assert_eq!(cell.attributes.get_i64(&AttributeKey::ColSpan), Some(2));

        // Row 1, Cell 0: vMerge restart
        let row1 = doc.node(table.children[1]).unwrap();
        let cell_top = doc.node(row1.children[0]).unwrap();
        assert_eq!(
            cell_top.attributes.get_string(&AttributeKey::RowSpan),
            Some("restart")
        );

        // Row 1, Cell 1: vMerge continue
        let cell_cont = doc.node(row1.children[1]).unwrap();
        assert_eq!(
            cell_cont.attributes.get_string(&AttributeKey::RowSpan),
            Some("continue")
        );
    }

    #[test]
    fn parse_table_mixed_with_paragraphs() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:t>Before</w:t></w:r></w:p>
            <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
            <w:p><w:r><w:t>After</w:t></w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 3); // Paragraph, Table, Paragraph

        let text = doc.to_plain_text();
        assert!(text.contains("Before"));
        assert!(text.contains("Cell"));
        assert!(text.contains("After"));
    }

    #[test]
    fn parse_nested_table() {
        let xml = wrap_doc(
            r#"<w:tbl><w:tr><w:tc>
                <w:tbl><w:tr><w:tc>
                    <w:p><w:r><w:t>Nested</w:t></w:r></w:p>
                </w:tc></w:tr></w:tbl>
            </w:tc></w:tr></w:tbl>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let outer_table = doc.node(body.children[0]).unwrap();
        assert_eq!(outer_table.node_type, NodeType::Table);

        let outer_row = doc.node(outer_table.children[0]).unwrap();
        let outer_cell = doc.node(outer_row.children[0]).unwrap();
        assert_eq!(outer_cell.node_type, NodeType::TableCell);

        // Nested table inside the cell
        let inner_table = doc.node(outer_cell.children[0]).unwrap();
        assert_eq!(inner_table.node_type, NodeType::Table);

        let text = doc.to_plain_text();
        assert!(text.contains("Nested"));
    }

    // ─── Image parsing tests ──────────────────────────────────────────

    #[test]
    fn parse_inline_image() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:drawing><wp:inline>
                <wp:extent cx="914400" cy="457200"/>
                <wp:docPr id="1" name="Picture 1" descr="A test image"/>
                <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                    <a:blip r:embed="rId4"/>
                </pic:blipFill></pic:pic></a:graphicData></a:graphic>
            </wp:inline></w:drawing></w:r></w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId4".to_string(), "media/image1.png".to_string());

        let mut media = HashMap::new();
        media.insert("media/image1.png".to_string(), vec![0x89, 0x50, 0x4E, 0x47]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Paragraph should have an Image child
        assert_eq!(para.children.len(), 1);
        let img = doc.node(para.children[0]).unwrap();
        assert_eq!(img.node_type, NodeType::Image);

        // Check dimensions: 914400 EMU / 12700 = 72pt, 457200 / 12700 = 36pt
        assert!((img.attributes.get_f64(&AttributeKey::ImageWidth).unwrap() - 72.0).abs() < 0.01);
        assert!((img.attributes.get_f64(&AttributeKey::ImageHeight).unwrap() - 36.0).abs() < 0.01);

        // Check alt text
        assert_eq!(
            img.attributes.get_string(&AttributeKey::ImageAltText),
            Some("A test image")
        );

        // Check media was stored
        assert_eq!(doc.media().len(), 1);
    }

    #[test]
    fn parse_floating_image() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:drawing><wp:anchor>
                <wp:extent cx="1270000" cy="635000"/>
                <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                    <a:blip r:embed="rId5"/>
                </pic:blipFill></pic:pic></a:graphicData></a:graphic>
            </wp:anchor></w:drawing></w:r></w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId5".to_string(), "media/image2.jpg".to_string());

        let mut media = HashMap::new();
        media.insert("media/image2.jpg".to_string(), vec![0xFF, 0xD8, 0xFF]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let img = doc.node(para.children[0]).unwrap();
        assert_eq!(img.node_type, NodeType::Image);

        // 1270000 / 12700 = 100pt
        assert!((img.attributes.get_f64(&AttributeKey::ImageWidth).unwrap() - 100.0).abs() < 0.01);
    }

    #[test]
    fn parse_image_missing_media_skipped() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:drawing><wp:inline>
                <wp:extent cx="914400" cy="457200"/>
                <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                    <a:blip r:embed="rId4"/>
                </pic:blipFill></pic:pic></a:graphicData></a:graphic>
            </wp:inline></w:drawing></w:r></w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId4".to_string(), "media/missing.png".to_string());
        let media = HashMap::new(); // No media files

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        // Image should be skipped, paragraph should be empty
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 0);
    }

    #[test]
    fn parse_text_and_image_in_same_paragraph() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:r><w:t>Before </w:t></w:r>
                <w:r><w:drawing><wp:inline>
                    <wp:extent cx="914400" cy="914400"/>
                    <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                        <a:blip r:embed="rId4"/>
                    </pic:blipFill></pic:pic></a:graphicData></a:graphic>
                </wp:inline></w:drawing></w:r>
                <w:r><w:t> After</w:t></w:r>
            </w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId4".to_string(), "media/img.png".to_string());
        let mut media = HashMap::new();
        media.insert("media/img.png".to_string(), vec![1, 2, 3]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Run("Before "), Image, Run(" After")
        assert_eq!(para.children.len(), 3);
        assert_eq!(doc.node(para.children[0]).unwrap().node_type, NodeType::Run);
        assert_eq!(
            doc.node(para.children[1]).unwrap().node_type,
            NodeType::Image
        );
        assert_eq!(doc.node(para.children[2]).unwrap().node_type, NodeType::Run);
    }

    /// Parse helper with custom relationships (for hyperlink tests).
    fn parse_doc_with_rels(xml: &str, doc: &mut DocumentModel, rels: &HashMap<String, String>) {
        let media = HashMap::new();
        let numbering = s1_model::NumberingDefinitions::default();
        parse_document_xml(xml, doc, rels, &media, &numbering).unwrap();
    }

    #[test]
    fn parse_hyperlink_external() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:hyperlink r:id="rId5">
                    <w:r><w:t>Click here</w:t></w:r>
                </w:hyperlink>
            </w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId5".to_string(), "https://example.com".to_string());

        let mut doc = DocumentModel::new();
        parse_doc_with_rels(&xml, &mut doc, &rels);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 1);

        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.node_type, NodeType::Run);
        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("https://example.com")
        );
    }

    #[test]
    fn parse_hyperlink_internal_anchor() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:hyperlink w:anchor="MyBookmark">
                    <w:r><w:t>Go to bookmark</w:t></w:r>
                </w:hyperlink>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("#MyBookmark")
        );
    }

    #[test]
    fn parse_hyperlink_with_tooltip() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:hyperlink r:id="rId6" w:tooltip="My Tooltip">
                    <w:r><w:t>Link text</w:t></w:r>
                </w:hyperlink>
            </w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId6".to_string(), "https://example.org".to_string());

        let mut doc = DocumentModel::new();
        parse_doc_with_rels(&xml, &mut doc, &rels);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("https://example.org")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkTooltip),
            Some("My Tooltip")
        );
    }

    #[test]
    fn parse_hyperlink_multiple_runs() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:hyperlink r:id="rId7">
                    <w:r><w:rPr><w:b/></w:rPr><w:t>Bold </w:t></w:r>
                    <w:r><w:t>normal</w:t></w:r>
                </w:hyperlink>
            </w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId7".to_string(), "https://test.com".to_string());

        let mut doc = DocumentModel::new();
        parse_doc_with_rels(&xml, &mut doc, &rels);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        // Both runs should have the same HyperlinkUrl
        assert_eq!(para.children.len(), 2);
        for &child_id in &para.children {
            let run = doc.node(child_id).unwrap();
            assert_eq!(
                run.attributes.get_string(&AttributeKey::HyperlinkUrl),
                Some("https://test.com")
            );
        }
    }

    #[test]
    fn parse_bookmark_start_end() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:bookmarkStart w:id="0" w:name="TestBookmark"/>
                <w:r><w:t>Content</w:t></w:r>
                <w:bookmarkEnd w:id="0"/>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        // BookmarkStart, Run, BookmarkEnd
        assert_eq!(para.children.len(), 3);

        let bk_start = doc.node(para.children[0]).unwrap();
        assert_eq!(bk_start.node_type, NodeType::BookmarkStart);
        assert_eq!(
            bk_start.attributes.get_string(&AttributeKey::BookmarkName),
            Some("TestBookmark")
        );

        let bk_end = doc.node(para.children[2]).unwrap();
        assert_eq!(bk_end.node_type, NodeType::BookmarkEnd);
    }

    #[test]
    fn parse_tab_stops() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:pPr>
                    <w:tabs>
                        <w:tab w:val="left" w:pos="720"/>
                        <w:tab w:val="right" w:pos="1440" w:leader="dot"/>
                        <w:tab w:val="center" w:pos="2160" w:leader="hyphen"/>
                        <w:tab w:val="decimal" w:pos="2880" w:leader="underscore"/>
                    </w:tabs>
                </w:pPr>
                <w:r><w:t>Text</w:t></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        if let Some(AttributeValue::TabStops(tabs)) = para.attributes.get(&AttributeKey::TabStops) {
            assert_eq!(tabs.len(), 4);
            assert_eq!(tabs[0].position, 36.0); // 720 twips / 20 = 36 points
            assert_eq!(tabs[0].alignment, s1_model::TabAlignment::Left);
            assert_eq!(tabs[0].leader, s1_model::TabLeader::None);

            assert_eq!(tabs[1].position, 72.0);
            assert_eq!(tabs[1].alignment, s1_model::TabAlignment::Right);
            assert_eq!(tabs[1].leader, s1_model::TabLeader::Dot);

            assert_eq!(tabs[2].position, 108.0);
            assert_eq!(tabs[2].alignment, s1_model::TabAlignment::Center);
            assert_eq!(tabs[2].leader, s1_model::TabLeader::Dash);

            assert_eq!(tabs[3].position, 144.0);
            assert_eq!(tabs[3].alignment, s1_model::TabAlignment::Decimal);
            assert_eq!(tabs[3].leader, s1_model::TabLeader::Underscore);
        } else {
            panic!("Expected TabStops attribute");
        }
    }

    #[test]
    fn parse_paragraph_borders() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:pPr>
                    <w:pBdr>
                        <w:top w:val="single" w:sz="4" w:color="000000"/>
                        <w:bottom w:val="double" w:sz="8" w:color="FF0000"/>
                    </w:pBdr>
                </w:pPr>
                <w:r><w:t>Bordered</w:t></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        if let Some(AttributeValue::Borders(borders)) =
            para.attributes.get(&AttributeKey::ParagraphBorders)
        {
            assert!(borders.top.is_some());
            let top = borders.top.as_ref().unwrap();
            assert_eq!(top.style, s1_model::BorderStyle::Single);
            assert_eq!(top.color, s1_model::Color::new(0, 0, 0));

            assert!(borders.bottom.is_some());
            let bottom = borders.bottom.as_ref().unwrap();
            assert_eq!(bottom.style, s1_model::BorderStyle::Double);
            assert_eq!(bottom.color, s1_model::Color::new(255, 0, 0));

            assert!(borders.left.is_none());
            assert!(borders.right.is_none());
        } else {
            panic!("Expected ParagraphBorders attribute");
        }
    }

    #[test]
    fn parse_paragraph_shading() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:pPr>
                    <w:shd w:val="clear" w:fill="FFFF00"/>
                </w:pPr>
                <w:r><w:t>Shaded</w:t></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        let bg = para.attributes.get_color(&AttributeKey::Background);
        assert_eq!(bg, Some(s1_model::Color::new(255, 255, 0)));
    }

    #[test]
    fn parse_character_spacing() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:r>
                    <w:rPr><w:spacing w:val="40"/></w:rPr>
                    <w:t>Spaced</w:t>
                </w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        let spacing = run.attributes.get_f64(&AttributeKey::FontSpacing);
        assert_eq!(spacing, Some(2.0)); // 40 twips / 20 = 2.0 points
    }

    #[test]
    fn parse_superscript() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:r>
                    <w:rPr><w:vertAlign w:val="superscript"/></w:rPr>
                    <w:t>2</w:t>
                </w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(
            run.attributes.get_bool(&AttributeKey::Superscript),
            Some(true)
        );
    }

    #[test]
    fn parse_subscript() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:r>
                    <w:rPr><w:vertAlign w:val="subscript"/></w:rPr>
                    <w:t>2</w:t>
                </w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(
            run.attributes.get_bool(&AttributeKey::Subscript),
            Some(true)
        );
    }

    #[test]
    fn parse_comment_range() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:commentRangeStart w:id="0"/>
                <w:r><w:t>Commented</w:t></w:r>
                <w:commentRangeEnd w:id="0"/>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        // CommentStart, Run, CommentEnd
        assert_eq!(para.children.len(), 3);

        let cs = doc.node(para.children[0]).unwrap();
        assert_eq!(cs.node_type, NodeType::CommentStart);
        assert_eq!(
            cs.attributes.get_string(&AttributeKey::CommentId),
            Some("0")
        );

        let ce = doc.node(para.children[2]).unwrap();
        assert_eq!(ce.node_type, NodeType::CommentEnd);
        assert_eq!(
            ce.attributes.get_string(&AttributeKey::CommentId),
            Some("0")
        );
    }

    #[test]
    fn parse_toc_sdt() {
        let xml = wrap_doc(
            r#"<w:sdt>
                <w:sdtPr>
                    <w:docPartObj>
                        <w:docPartGallery w:val="Table of Contents"/>
                        <w:docPartUnique/>
                    </w:docPartObj>
                </w:sdtPr>
                <w:sdtContent>
                    <w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> TOC \o "1-3" \h \z \u </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r></w:p>
                    <w:p><w:r><w:t>Chapter One</w:t></w:r></w:p>
                    <w:p><w:r><w:t>Section A</w:t></w:r></w:p>
                    <w:p><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>
                </w:sdtContent>
            </w:sdt>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert!(!body.children.is_empty());

        // First child should be a TableOfContents node
        let toc = doc.node(body.children[0]).unwrap();
        assert_eq!(toc.node_type, NodeType::TableOfContents);

        // Should have the cached entry paragraphs (field-only paragraphs removed)
        assert!(
            toc.children.len() >= 2,
            "Expected at least 2 cached entries, got {}",
            toc.children.len()
        );

        // Verify entry text
        let entry1 = doc.node(toc.children[0]).unwrap();
        assert_eq!(entry1.node_type, NodeType::Paragraph);
    }

    #[test]
    fn parse_toc_with_empty_gallery() {
        // docPartGallery as an empty/self-closing element (the writer produces this form)
        let xml = wrap_doc(
            r#"<w:sdt>
                <w:sdtPr>
                    <w:docPartObj>
                        <w:docPartGallery w:val="Table of Contents"/>
                    </w:docPartObj>
                </w:sdtPr>
                <w:sdtContent>
                    <w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r></w:p>
                    <w:p><w:r><w:t>Entry Text</w:t></w:r></w:p>
                    <w:p><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>
                </w:sdtContent>
            </w:sdt>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body = doc.node(doc.body_id().unwrap()).unwrap();
        let toc = doc.node(body.children[0]).unwrap();
        assert_eq!(toc.node_type, NodeType::TableOfContents);
        assert!(!toc.children.is_empty(), "Should have cached entry");
    }

    #[test]
    fn parse_non_toc_sdt_passes_through() {
        // An SDT that is NOT a Table of Contents should parse its paragraphs normally
        let xml = wrap_doc(
            r#"<w:sdt>
                <w:sdtPr>
                    <w:alias w:val="Some Control"/>
                </w:sdtPr>
                <w:sdtContent>
                    <w:p><w:r><w:t>Normal text</w:t></w:r></w:p>
                </w:sdtContent>
            </w:sdt>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        // Should have a paragraph (not a TOC node)
        let first = doc.node(body.children[0]).unwrap();
        assert_eq!(first.node_type, NodeType::Paragraph);
        assert_eq!(doc.to_plain_text(), "Normal text");
    }

    // ─── Complex field (fldChar) parsing tests ──────────────────────────

    #[test]
    fn parse_complex_field_single_run() {
        // All fldChar elements in a single run (as in the Chat Reaction DOCX footer)
        let xml = wrap_doc(
            r#"<w:p><w:r>
                <w:fldChar w:fldCharType="begin" />
                <w:instrText xml:space="preserve">PAGE</w:instrText>
                <w:fldChar w:fldCharType="separate" />
                <w:fldChar w:fldCharType="end" />
            </w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have a Field node as child of the paragraph
        let field = doc.node(para.children[0]).unwrap();
        assert_eq!(field.node_type, NodeType::Field);
        assert_eq!(
            field.attributes.get(&AttributeKey::FieldType),
            Some(&AttributeValue::FieldType(FieldType::PageNumber))
        );
    }

    #[test]
    fn parse_complex_field_across_runs() {
        // fldChar elements spanning multiple runs
        let xml = wrap_doc(
            r#"<w:p>
                <w:r><w:fldChar w:fldCharType="begin" /></w:r>
                <w:r><w:instrText xml:space="preserve"> NUMPAGES </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate" /></w:r>
                <w:r><w:t>5</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end" /></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have a Field node; the display value "5" should be skipped
        let field = doc.node(para.children[0]).unwrap();
        assert_eq!(field.node_type, NodeType::Field);
        assert_eq!(
            field.attributes.get(&AttributeKey::FieldType),
            Some(&AttributeValue::FieldType(FieldType::PageCount))
        );
        // No text "5" should leak through
        assert_eq!(doc.to_plain_text(), "");
    }

    // ─── mc:AlternateContent image parsing test ─────────────────────────

    #[test]
    fn parse_image_in_alternate_content() {
        // Drawing wrapped in mc:AlternateContent (common in Google Docs DOCX exports)
        let xml = wrap_doc(
            r#"<w:p><w:r>
                <mc:AlternateContent>
                    <mc:Choice Requires="wpg">
                        <w:drawing><wp:inline>
                            <wp:extent cx="914400" cy="457200"/>
                            <wp:docPr id="1" name="Picture 1"/>
                            <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                                <a:blip r:embed="rId5"/>
                            </pic:blipFill></pic:pic></a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Choice>
                    <mc:Fallback/>
                </mc:AlternateContent>
            </w:r></w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId5".to_string(), "media/image1.png".to_string());

        let mut media = HashMap::new();
        media.insert("media/image1.png".to_string(), vec![0x89, 0x50, 0x4E, 0x47]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have an Image node inside the paragraph
        let image = doc.node(para.children[0]).unwrap();
        assert_eq!(image.node_type, NodeType::Image);
        assert!(image.attributes.get(&AttributeKey::ImageMediaId).is_some());
        assert_eq!(doc.media().len(), 1);
    }

    // ─── Track changes (ins/del/rPrChange) parsing tests ─────────────

    #[test]
    fn parse_ins_basic() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:ins w:id="1" w:author="John" w:date="2024-01-01T12:00:00Z">
                    <w:r><w:rPr><w:b/></w:rPr><w:t>inserted text</w:t></w:r>
                </w:ins>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 1);

        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.node_type, NodeType::Run);
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("Insert")
        );
        assert_eq!(run.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(doc.to_plain_text(), "inserted text");
    }

    #[test]
    fn parse_ins_with_author_date() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:ins w:id="5" w:author="Alice" w:date="2024-06-15T09:30:00Z">
                    <w:r><w:t>new content</w:t></w:r>
                </w:ins>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("Insert")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Alice")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionDate),
            Some("2024-06-15T09:30:00Z")
        );
        assert_eq!(run.attributes.get_i64(&AttributeKey::RevisionId), Some(5));
    }

    #[test]
    fn parse_del_basic() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:del w:id="2" w:author="Jane" w:date="2024-01-02T10:00:00Z">
                    <w:r><w:rPr><w:b/></w:rPr><w:delText>deleted text</w:delText></w:r>
                </w:del>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 1);

        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.node_type, NodeType::Run);
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("Delete")
        );
    }

    #[test]
    fn parse_del_text_content() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:del w:id="3" w:author="Bob" w:date="2024-02-01T08:00:00Z">
                    <w:r><w:delText>removed words</w:delText></w:r>
                </w:del>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        assert_eq!(doc.to_plain_text(), "removed words");

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Bob")
        );
        assert_eq!(run.attributes.get_i64(&AttributeKey::RevisionId), Some(3));
    }

    #[test]
    fn parse_rpr_change() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:r>
                    <w:rPr>
                        <w:b/>
                        <w:rPrChange w:id="3" w:author="Bob" w:date="2024-01-03T09:00:00Z">
                            <w:rPr><w:i/></w:rPr>
                        </w:rPrChange>
                    </w:rPr>
                    <w:t>reformatted</w:t>
                </w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();

        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("FormatChange")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Bob")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionDate),
            Some("2024-01-03T09:00:00Z")
        );
        assert_eq!(run.attributes.get_i64(&AttributeKey::RevisionId), Some(3));
        // Current formatting should still be parsed
        assert_eq!(run.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(doc.to_plain_text(), "reformatted");
    }

    #[test]
    fn parse_mixed_tracked_untracked() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:r><w:t>normal </w:t></w:r>
                <w:ins w:id="10" w:author="Ed" w:date="2024-03-01T00:00:00Z">
                    <w:r><w:t>inserted</w:t></w:r>
                </w:ins>
                <w:r><w:t> more</w:t></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 3);

        // First run — no revision
        let run1 = doc.node(para.children[0]).unwrap();
        assert!(run1
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_none());

        // Second run — Insert revision
        let run2 = doc.node(para.children[1]).unwrap();
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionType),
            Some("Insert")
        );

        // Third run — no revision
        let run3 = doc.node(para.children[2]).unwrap();
        assert!(run3
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_none());

        assert_eq!(doc.to_plain_text(), "normal inserted more");
    }

    #[test]
    fn parse_ins_multiple_runs() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:ins w:id="7" w:author="Sam" w:date="2024-04-01T00:00:00Z">
                    <w:r><w:rPr><w:b/></w:rPr><w:t>bold </w:t></w:r>
                    <w:r><w:rPr><w:i/></w:rPr><w:t>italic</w:t></w:r>
                </w:ins>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 2);

        // Both runs should have Insert revision
        for &child_id in &para.children {
            let run = doc.node(child_id).unwrap();
            assert_eq!(
                run.attributes.get_string(&AttributeKey::RevisionType),
                Some("Insert")
            );
            assert_eq!(
                run.attributes.get_string(&AttributeKey::RevisionAuthor),
                Some("Sam")
            );
        }

        // First run bold, second italic
        let run1 = doc.node(para.children[0]).unwrap();
        assert_eq!(run1.attributes.get_bool(&AttributeKey::Bold), Some(true));
        let run2 = doc.node(para.children[1]).unwrap();
        assert_eq!(run2.attributes.get_bool(&AttributeKey::Italic), Some(true));

        assert_eq!(doc.to_plain_text(), "bold italic");
    }

    #[test]
    fn parse_nested_ins_del() {
        // ins and del in same paragraph
        let xml = wrap_doc(
            r#"<w:p>
                <w:ins w:id="1" w:author="A" w:date="2024-01-01T00:00:00Z">
                    <w:r><w:t>added</w:t></w:r>
                </w:ins>
                <w:r><w:t> middle </w:t></w:r>
                <w:del w:id="2" w:author="B" w:date="2024-01-02T00:00:00Z">
                    <w:r><w:delText>removed</w:delText></w:r>
                </w:del>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 3);

        let run1 = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run1.attributes.get_string(&AttributeKey::RevisionType),
            Some("Insert")
        );

        let run2 = doc.node(para.children[1]).unwrap();
        assert!(run2
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_none());

        let run3 = doc.node(para.children[2]).unwrap();
        assert_eq!(
            run3.attributes.get_string(&AttributeKey::RevisionType),
            Some("Delete")
        );

        assert_eq!(doc.to_plain_text(), "added middle removed");
    }

    // ─── Bug fix: non-self-closing fldChar parsing ──────────────────────

    #[test]
    fn parse_complex_field_non_self_closing_fld_char() {
        // Some DOCX producers emit <w:fldChar></w:fldChar> instead of <w:fldChar/>
        let xml = wrap_doc(
            r#"<w:p>
                <w:r><w:fldChar w:fldCharType="begin"></w:fldChar></w:r>
                <w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"></w:fldChar></w:r>
                <w:r><w:t>3</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"></w:fldChar></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have a Field node; the display value "3" should be skipped
        let field = doc.node(para.children[0]).unwrap();
        assert_eq!(field.node_type, NodeType::Field);
        assert_eq!(
            field.attributes.get(&AttributeKey::FieldType),
            Some(&AttributeValue::FieldType(FieldType::PageNumber))
        );
        // No text "3" should leak through
        assert_eq!(doc.to_plain_text(), "");
    }

    #[test]
    fn parse_complex_field_mixed_self_closing_and_non() {
        // Mix of self-closing and non-self-closing fldChar within a single run
        let xml = wrap_doc(
            r#"<w:p><w:r>
                <w:fldChar w:fldCharType="begin"></w:fldChar>
                <w:instrText xml:space="preserve">NUMPAGES</w:instrText>
                <w:fldChar w:fldCharType="separate" />
                <w:fldChar w:fldCharType="end"></w:fldChar>
            </w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        let field = doc.node(para.children[0]).unwrap();
        assert_eq!(field.node_type, NodeType::Field);
        assert_eq!(
            field.attributes.get(&AttributeKey::FieldType),
            Some(&AttributeValue::FieldType(FieldType::PageCount))
        );
    }

    // ─── Bug fix: mc:AlternateContent at paragraph level ────────────────

    #[test]
    fn parse_alternate_content_at_paragraph_level() {
        // mc:AlternateContent as a direct child of w:p (not inside w:r)
        let xml = wrap_doc(
            r#"<w:p>
                <mc:AlternateContent>
                    <mc:Choice Requires="wps">
                        <w:drawing><wp:inline>
                            <wp:extent cx="914400" cy="457200"/>
                            <wp:docPr id="1" name="Picture 1"/>
                            <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                                <a:blip r:embed="rId5"/>
                            </pic:blipFill></pic:pic></a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Choice>
                    <mc:Fallback>
                        <w:drawing><wp:inline>
                            <wp:extent cx="914400" cy="457200"/>
                            <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                                <a:blip r:embed="rId5"/>
                            </pic:blipFill></pic:pic></a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Fallback>
                </mc:AlternateContent>
            </w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId5".to_string(), "media/image1.png".to_string());

        let mut media = HashMap::new();
        media.insert("media/image1.png".to_string(), vec![0x89, 0x50, 0x4E, 0x47]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have exactly one Image node (not duplicated from Fallback)
        assert_eq!(para.children.len(), 1);
        let image = doc.node(para.children[0]).unwrap();
        assert_eq!(image.node_type, NodeType::Image);
        assert!(image.attributes.get(&AttributeKey::ImageMediaId).is_some());
    }

    #[test]
    fn parse_alternate_content_skips_fallback_in_run() {
        // Verify that mc:Fallback drawing inside a run is not duplicated
        let xml = wrap_doc(
            r#"<w:p><w:r>
                <mc:AlternateContent>
                    <mc:Choice Requires="wpg">
                        <w:drawing><wp:inline>
                            <wp:extent cx="914400" cy="457200"/>
                            <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                                <a:blip r:embed="rId5"/>
                            </pic:blipFill></pic:pic></a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Choice>
                    <mc:Fallback>
                        <w:drawing><wp:inline>
                            <wp:extent cx="914400" cy="457200"/>
                            <a:graphic><a:graphicData><pic:pic><pic:blipFill>
                                <a:blip r:embed="rId5"/>
                            </pic:blipFill></pic:pic></a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Fallback>
                </mc:AlternateContent>
            </w:r></w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId5".to_string(), "media/image1.png".to_string());

        let mut media = HashMap::new();
        media.insert("media/image1.png".to_string(), vec![0x89, 0x50, 0x4E, 0x47]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have exactly one Image node (Fallback's drawing should be skipped)
        assert_eq!(para.children.len(), 1);
        let image = doc.node(para.children[0]).unwrap();
        assert_eq!(image.node_type, NodeType::Image);
    }

    #[test]
    fn parse_floating_image_with_positioning() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:drawing><wp:anchor distT="45720" distB="45720" distL="114300" distR="114300" simplePos="0" relativeHeight="251658240" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1">
                <wp:simplePos x="0" y="0"/>
                <wp:positionH relativeFrom="column"><wp:posOffset>914400</wp:posOffset></wp:positionH>
                <wp:positionV relativeFrom="paragraph"><wp:posOffset>457200</wp:posOffset></wp:positionV>
                <wp:extent cx="1828800" cy="1371600"/>
                <wp:effectExtent l="0" t="0" r="0" b="0"/>
                <wp:wrapSquare wrapText="bothSides"/>
                <wp:docPr id="1" name="Picture 1" descr="Floating test"/>
                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                    <pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="img1"/><pic:cNvPicPr/></pic:nvPicPr>
                    <pic:blipFill><a:blip r:embed="rId10"/></pic:blipFill>
                    <pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1828800" cy="1371600"/></a:xfrm></pic:spPr>
                    </pic:pic>
                </a:graphicData></a:graphic>
            </wp:anchor></w:drawing></w:r></w:p>"#,
        );
        let mut rels = HashMap::new();
        rels.insert("rId10".to_string(), "media/float.jpg".to_string());
        let mut media = HashMap::new();
        media.insert("media/float.jpg".to_string(), vec![0xFF, 0xD8, 0xFF, 0xE0]);

        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let img = doc.node(para.children[0]).unwrap();
        assert_eq!(img.node_type, NodeType::Image);

        // Verify floating position type
        assert_eq!(
            img.attributes.get_string(&AttributeKey::ImagePositionType),
            Some("anchor")
        );

        // Verify wrap type
        assert_eq!(
            img.attributes.get_string(&AttributeKey::ImageWrapType),
            Some("square")
        );

        // Verify horizontal positioning: column-relative, 914400 EMU offset
        assert_eq!(
            img.attributes
                .get_string(&AttributeKey::ImageHorizontalRelativeFrom),
            Some("column")
        );
        assert_eq!(
            img.attributes.get_i64(&AttributeKey::ImageHorizontalOffset),
            Some(914400)
        );

        // Verify vertical positioning: paragraph-relative, 457200 EMU offset
        assert_eq!(
            img.attributes
                .get_string(&AttributeKey::ImageVerticalRelativeFrom),
            Some("paragraph")
        );
        assert_eq!(
            img.attributes.get_i64(&AttributeKey::ImageVerticalOffset),
            Some(457200)
        );

        // Verify dimensions: 1828800 / 12700 = 144pt, 1371600 / 12700 = 108pt
        assert!((img.attributes.get_f64(&AttributeKey::ImageWidth).unwrap() - 144.0).abs() < 0.01);
        assert!((img.attributes.get_f64(&AttributeKey::ImageHeight).unwrap() - 108.0).abs() < 0.01);

        // Verify distance from text
        assert_eq!(
            img.attributes
                .get_string(&AttributeKey::ImageDistanceFromText),
            Some("45720,45720,114300,114300")
        );

        // Verify alt text
        assert_eq!(
            img.attributes.get_string(&AttributeKey::ImageAltText),
            Some("Floating test")
        );
    }

    #[test]
    fn parse_vml_shape_rect() {
        let xml = wrap_doc(
            r##"<w:p><w:r><w:pict>
                <v:rect style="width:200pt;height:100pt" fillcolor="#FF0000" strokecolor="#0000FF"/>
            </w:pict></w:r></w:p>"##,
        );
        let rels = HashMap::new();
        let media = HashMap::new();
        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert!(
            !para.children.is_empty(),
            "paragraph should have a drawing child"
        );

        let shape = doc.node(para.children[0]).unwrap();
        assert_eq!(shape.node_type, NodeType::Drawing);
        assert_eq!(
            shape.attributes.get_string(&AttributeKey::ShapeType),
            Some("rect")
        );
        assert!(
            (shape.attributes.get_f64(&AttributeKey::ShapeWidth).unwrap() - 200.0).abs() < 0.01
        );
        assert!(
            (shape
                .attributes
                .get_f64(&AttributeKey::ShapeHeight)
                .unwrap()
                - 100.0)
                .abs()
                < 0.01
        );
        assert_eq!(
            shape.attributes.get_string(&AttributeKey::ShapeFillColor),
            Some("FF0000")
        );
        assert_eq!(
            shape.attributes.get_string(&AttributeKey::ShapeStrokeColor),
            Some("0000FF")
        );
        // Raw XML should be preserved
        assert!(shape
            .attributes
            .get_string(&AttributeKey::ShapeRawXml)
            .unwrap()
            .contains("w:pict"));
    }

    #[test]
    fn parse_vml_shape_with_style_dimensions() {
        let xml = wrap_doc(
            r#"<w:p><w:r><w:pict>
                <v:shape style="width:2in;height:1in">
                    <v:textbox>Some text</v:textbox>
                </v:shape>
            </w:pict></w:r></w:p>"#,
        );
        let rels = HashMap::new();
        let media = HashMap::new();
        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let shape = doc.node(para.children[0]).unwrap();
        assert_eq!(shape.node_type, NodeType::Drawing);
        assert_eq!(
            shape.attributes.get_string(&AttributeKey::ShapeType),
            Some("shape")
        );
        // 2in = 144pt, 1in = 72pt
        assert!(
            (shape.attributes.get_f64(&AttributeKey::ShapeWidth).unwrap() - 144.0).abs() < 0.01
        );
        assert!(
            (shape
                .attributes
                .get_f64(&AttributeKey::ShapeHeight)
                .unwrap()
                - 72.0)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn parse_empty_pict_no_shape() {
        // <w:pict> with no recognized shape element should produce no Drawing node
        let xml = wrap_doc(r#"<w:p><w:r><w:pict><o:OLEObject/></w:pict></w:r></w:p>"#);
        let rels = HashMap::new();
        let media = HashMap::new();
        let mut doc = DocumentModel::new();
        parse_document_xml(
            &xml,
            &mut doc,
            &rels,
            &media,
            &s1_model::NumberingDefinitions::default(),
        )
        .unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(
            para.children.len(),
            0,
            "no shape should be created for OLE-only pict"
        );
    }

    // ─── DrawingML chart/shape raw XML round-trip ───────────────────────

    #[test]
    fn parse_drawing_chart_preserved_as_raw_xml() {
        // A <w:drawing> containing a chart reference (no <a:blip>)
        let xml = wrap_doc(
            r#"<w:p><w:r><w:drawing><wp:inline>
                <wp:extent cx="5486400" cy="3200400"/>
                <wp:docPr id="1" name="Chart 1"/>
                <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
                    <c:chart r:id="rId8"/>
                </a:graphicData></a:graphic>
            </wp:inline></w:drawing></w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have a Drawing node (not an Image)
        assert_eq!(para.children.len(), 1, "paragraph should have one child");
        let drawing = doc.node(para.children[0]).unwrap();
        assert_eq!(drawing.node_type, NodeType::Drawing);

        // Should have shape type "drawing"
        assert_eq!(
            drawing.attributes.get_string(&AttributeKey::ShapeType),
            Some("drawing")
        );

        // Should have raw XML containing the chart reference
        let raw = drawing
            .attributes
            .get_string(&AttributeKey::ShapeRawXml)
            .expect("should have raw XML");
        assert!(
            raw.contains("<w:drawing>"),
            "raw XML should start with <w:drawing>"
        );
        assert!(
            raw.contains("c:chart"),
            "raw XML should contain chart reference"
        );
        assert!(
            raw.contains("</w:drawing>"),
            "raw XML should end with </w:drawing>"
        );
    }

    #[test]
    fn parse_drawingml_shape_preserved_as_raw_xml() {
        // A <w:drawing> containing a DrawingML shape (wps:wsp), no image blip
        let xml = wrap_doc(
            r#"<w:p><w:r><w:drawing><wp:anchor distT="0" distB="0" distL="114300" distR="114300">
                <wp:extent cx="1828800" cy="914400"/>
                <wp:docPr id="2" name="Rectangle 1"/>
                <a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">
                    <wps:wsp><wps:spPr><a:prstGeom prst="rect"/></wps:spPr></wps:wsp>
                </a:graphicData></a:graphic>
            </wp:anchor></w:drawing></w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(para.children.len(), 1);
        let drawing = doc.node(para.children[0]).unwrap();
        assert_eq!(drawing.node_type, NodeType::Drawing);

        let raw = drawing
            .attributes
            .get_string(&AttributeKey::ShapeRawXml)
            .expect("should have raw XML");
        assert!(
            raw.contains("wps:wsp"),
            "raw XML should contain the shape element"
        );
        assert!(raw.contains("prstGeom"), "raw XML should contain geometry");
    }

    #[test]
    fn parse_drawing_chart_in_alternate_content() {
        // Chart wrapped in mc:AlternateContent
        let xml = wrap_doc(
            r#"<w:p><w:r>
                <mc:AlternateContent>
                    <mc:Choice Requires="cx">
                        <w:drawing><wp:inline>
                            <wp:extent cx="5486400" cy="3200400"/>
                            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
                                <c:chart r:id="rId10"/>
                            </a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Choice>
                    <mc:Fallback/>
                </mc:AlternateContent>
            </w:r></w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have exactly one Drawing node from the Choice branch
        assert_eq!(para.children.len(), 1);
        let drawing = doc.node(para.children[0]).unwrap();
        assert_eq!(drawing.node_type, NodeType::Drawing);
        let raw = drawing
            .attributes
            .get_string(&AttributeKey::ShapeRawXml)
            .expect("should have raw XML");
        assert!(raw.contains("c:chart"), "raw XML should contain chart");
    }

    #[test]
    fn parse_drawing_chart_at_paragraph_level_alternate_content() {
        // mc:AlternateContent as direct child of paragraph (not inside w:r)
        let xml = wrap_doc(
            r#"<w:p>
                <mc:AlternateContent>
                    <mc:Choice Requires="cx">
                        <w:drawing><wp:inline>
                            <wp:extent cx="5486400" cy="3200400"/>
                            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
                                <c:chart r:id="rId11"/>
                            </a:graphicData></a:graphic>
                        </wp:inline></w:drawing>
                    </mc:Choice>
                    <mc:Fallback/>
                </mc:AlternateContent>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert_eq!(para.children.len(), 1);
        let drawing = doc.node(para.children[0]).unwrap();
        assert_eq!(drawing.node_type, NodeType::Drawing);
    }

    // ─── Move tracking (w:moveTo / w:moveFrom) ─────────────────────────

    #[test]
    fn parse_move_to_block_level() {
        let xml = wrap_doc(
            r#"<w:moveTo w:id="1" w:author="Alice" w:date="2024-05-01T10:00:00Z">
                <w:p><w:r><w:t>moved here</w:t></w:r></w:p>
            </w:moveTo>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let para = doc.node(body.children[0]).unwrap();
        // The paragraph should have a run child
        assert!(!para.children.is_empty());
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.node_type, NodeType::Run);

        // The run should be tagged with MoveTo revision
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("MoveTo")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Alice")
        );
    }

    #[test]
    fn parse_move_from_block_level() {
        let xml = wrap_doc(
            r#"<w:moveFrom w:id="2" w:author="Bob" w:date="2024-05-01T11:00:00Z">
                <w:p><w:r><w:t>moved from here</w:t></w:r></w:p>
            </w:moveFrom>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("MoveFrom")
        );
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Bob")
        );
    }

    #[test]
    fn parse_move_to_inline_level() {
        // Inline moveTo inside a paragraph
        let xml = wrap_doc(
            r#"<w:p>
                <w:r><w:t>before </w:t></w:r>
                <w:moveTo w:id="3" w:author="Carol">
                    <w:r><w:t>moved text</w:t></w:r>
                </w:moveTo>
                <w:r><w:t> after</w:t></w:r>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have 3 runs: "before ", "moved text", " after"
        assert_eq!(para.children.len(), 3);

        // First run — no revision
        let run1 = doc.node(para.children[0]).unwrap();
        assert!(run1
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_none());

        // Second run — MoveTo
        let run2 = doc.node(para.children[1]).unwrap();
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionType),
            Some("MoveTo")
        );
        assert_eq!(
            run2.attributes.get_string(&AttributeKey::RevisionAuthor),
            Some("Carol")
        );

        // Third run — no revision
        let run3 = doc.node(para.children[2]).unwrap();
        assert!(run3
            .attributes
            .get_string(&AttributeKey::RevisionType)
            .is_none());
    }

    #[test]
    fn parse_move_from_inline_level() {
        let xml = wrap_doc(
            r#"<w:p>
                <w:moveFrom w:id="4" w:author="Dave">
                    <w:r><w:t>originally here</w:t></w:r>
                </w:moveFrom>
            </w:p>"#,
        );
        let mut doc = DocumentModel::new();
        parse_doc(&xml, &mut doc);

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::RevisionType),
            Some("MoveFrom")
        );
    }
}
