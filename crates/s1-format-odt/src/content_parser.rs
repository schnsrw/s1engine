//! Parse `<office:body><office:text>` content from ODF `content.xml`.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{
    AttributeKey, AttributeMap, AttributeValue, DocumentModel, FieldType, ListFormat, ListInfo,
    MediaId, Node, NodeType,
};

use crate::error::OdtError;
use crate::xml_util::get_attr;

/// Context passed to the content parser.
pub struct ParseContext {
    /// Automatic styles resolved from `<office:automatic-styles>`.
    pub auto_styles: HashMap<String, AttributeMap>,
    /// Map of image href paths → MediaId (populated by reader after extracting images).
    pub image_map: HashMap<String, MediaId>,
}

/// Parse the body of `content.xml` from a reader positioned at `<office:text>`.
///
/// The reader should be positioned just after consuming the `<office:text>` start tag.
pub fn parse_content_body(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
) -> Result<(), OdtError> {
    let body_id = doc
        .body_id()
        .ok_or_else(|| OdtError::InvalidStructure("Document has no body node".to_string()))?;

    let mut body_child_index = doc.node(body_id).map_or(0, |n| n.children.len());

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" => {
                        parse_paragraph_into(
                            reader,
                            doc,
                            e,
                            ctx,
                            body_id,
                            body_child_index,
                            false,
                            None,
                        )?;
                        body_child_index += 1;
                    }
                    b"h" => {
                        let level = get_attr(e, b"outline-level")
                            .and_then(|v| v.parse::<u8>().ok())
                            .unwrap_or(1);
                        parse_paragraph_into(
                            reader,
                            doc,
                            e,
                            ctx,
                            body_id,
                            body_child_index,
                            true,
                            Some(level),
                        )?;
                        body_child_index += 1;
                    }
                    b"list" => {
                        let count = parse_list(reader, doc, ctx, body_id, body_child_index, 0)?;
                        body_child_index += count;
                    }
                    b"table" => {
                        parse_table_into(reader, doc, ctx, body_id, body_child_index)?;
                        body_child_index += 1;
                    }
                    b"table-of-content" => {
                        parse_toc_into(reader, doc, ctx, body_id, body_child_index, e)?;
                        body_child_index += 1;
                    }
                    // ODT-11 (WONTFIX): Non-image draw elements (SVG shapes, text boxes,
                    // connectors, etc.) are silently dropped during parsing. The s1-model
                    // supports Image and Drawing node types, but full shape/SVG support
                    // (geometry, transforms, grouped objects) is a separate feature that
                    // would require significant model extensions. These elements are skipped
                    // with a debug-mode warning.
                    b"custom-shape" | b"rect" | b"circle" | b"ellipse" | b"line" | b"polygon"
                    | b"polyline" | b"path" | b"text-box" | b"g" | b"connector" => {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "[s1-format-odt] Note: non-image draw element <draw:{}> skipped \
                             (ODT-11 / WONTFIX — shape/SVG support is a separate feature)",
                            String::from_utf8_lossy(local.as_ref())
                        );
                        skip_element_odt(reader)?;
                    }

                    // Q12 — ODF Change Tracking (text:tracked-changes, text:changed-region).
                    //
                    // ODF documents may contain change-tracking markup that records
                    // insertions, deletions, and format changes made by reviewers.
                    // The top-level container is <text:tracked-changes> which holds
                    // one or more <text:changed-region> children. Inline, the
                    // elements <text:change-start>, <text:change-end>, and
                    // <text:change> reference those regions.
                    //
                    // Full semantic modelling of tracked changes is out-of-scope for
                    // now. To ensure round-trip preservation we capture the raw XML
                    // of the <text:tracked-changes> block as an opaque attribute on
                    // the body node. This mirrors the approach used by the DOCX reader
                    // for unknown namespace elements.
                    //
                    // When a full tracked-changes model is added, this block should
                    // be replaced with structured parsing.
                    b"tracked-changes" => {
                        let raw = capture_raw_xml(reader, "tracked-changes")?;
                        if let Some(body) = doc.node_mut(body_id) {
                            body.attributes
                                .set(AttributeKey::RawXml, AttributeValue::String(raw));
                        }
                    }

                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"text" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(())
}

/// Parse a `<text:p>` or `<text:h>` element and insert it into `parent_id` at `index`.
#[allow(clippy::too_many_arguments)]
fn parse_paragraph_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    start: &quick_xml::events::BytesStart<'_>,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    index: usize,
    is_heading: bool,
    heading_level: Option<u8>,
) -> Result<s1_model::NodeId, OdtError> {
    let para_id = doc.next_id();
    let mut para_node = Node::new(para_id, NodeType::Paragraph);

    // Apply auto-style or named style reference
    if let Some(style_name) = get_attr(start, b"style-name") {
        if let Some(auto_attrs) = ctx.auto_styles.get(&style_name) {
            para_node.attributes.merge(auto_attrs);
        } else {
            para_node
                .attributes
                .set(AttributeKey::StyleId, AttributeValue::String(style_name));
        }
    }

    // Set heading style
    if is_heading {
        if let Some(level) = heading_level {
            let style_name = format!("Heading{level}");
            para_node
                .attributes
                .set(AttributeKey::StyleId, AttributeValue::String(style_name));
        }
    }

    // Insert paragraph into parent
    doc.insert_node(parent_id, index, para_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    // Now parse children and add them to the paragraph
    let end_tag: &[u8] = if is_heading { b"h" } else { b"p" };
    let mut child_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"span" => {
                        let added = parse_span_into(reader, doc, e, ctx, para_id, child_index)?;
                        child_index += added;
                    }
                    b"a" => {
                        // Hyperlink — extract URL and set on child runs.
                        //
                        // ODT-12 (known limitation): Internal hyperlinks targeting bookmarks
                        // are parsed as regular hyperlinks with href="#bookmark-name". The
                        // cross-reference resolution (linking the href to the actual
                        // BookmarkStart node position in the document tree) is not performed.
                        // Both bookmark start/end elements and the hyperlink hrefs are
                        // preserved, so the structural data survives a round-trip — but a
                        // consumer that needs to navigate to a bookmark target must resolve
                        // the "#name" href against BookmarkStart nodes themselves. This is
                        // consistent with how DOCX bookmarks + internal hyperlinks work.
                        let url = get_attr(e, b"href").unwrap_or_default();
                        let added =
                            parse_hyperlink_into(reader, doc, e, ctx, para_id, child_index, &url)?;
                        child_index += added;
                    }
                    b"frame" => {
                        if parse_frame_into(reader, doc, e, ctx, para_id, child_index)? {
                            child_index += 1;
                        }
                    }
                    b"annotation" => {
                        let added = parse_annotation_into(reader, doc, e, para_id, child_index)?;
                        child_index += added;
                    }
                    b"note" => {
                        // Footnote or endnote (<text:note>)
                        let added = parse_note_into(reader, doc, ctx, e, para_id, child_index)?;
                        child_index += added;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"line-break" => {
                        let lb_id = doc.next_id();
                        doc.insert_node(
                            para_id,
                            child_index,
                            Node::new(lb_id, NodeType::LineBreak),
                        )
                        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        child_index += 1;
                    }
                    b"tab" => {
                        let tab_id = doc.next_id();
                        doc.insert_node(para_id, child_index, Node::new(tab_id, NodeType::Tab))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        child_index += 1;
                    }
                    b"s" => {
                        let count = get_attr(e, b"c")
                            .and_then(|v| v.parse::<usize>().ok())
                            .unwrap_or(1);
                        let run_id = doc.next_id();
                        doc.insert_node(para_id, child_index, Node::new(run_id, NodeType::Run))
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, " ".repeat(count)))
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        child_index += 1;
                    }
                    b"page-number" => {
                        let field_id = doc.next_id();
                        let mut field_node = Node::new(field_id, NodeType::Field);
                        field_node.attributes.set(
                            AttributeKey::FieldType,
                            AttributeValue::FieldType(FieldType::PageNumber),
                        );
                        doc.insert_node(para_id, child_index, field_node)
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        child_index += 1;
                    }
                    b"page-count" => {
                        let field_id = doc.next_id();
                        let mut field_node = Node::new(field_id, NodeType::Field);
                        field_node.attributes.set(
                            AttributeKey::FieldType,
                            AttributeValue::FieldType(FieldType::PageCount),
                        );
                        doc.insert_node(para_id, child_index, field_node)
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        child_index += 1;
                    }
                    b"bookmark-start" => {
                        if let Some(name) = get_attr(e, b"name") {
                            let bm_id = doc.next_id();
                            let mut bm = Node::new(bm_id, NodeType::BookmarkStart);
                            bm.attributes
                                .set(AttributeKey::BookmarkName, AttributeValue::String(name));
                            doc.insert_node(para_id, child_index, bm)
                                .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                            child_index += 1;
                        }
                    }
                    b"bookmark-end" => {
                        if let Some(name) = get_attr(e, b"name") {
                            let bm_id = doc.next_id();
                            let mut bm = Node::new(bm_id, NodeType::BookmarkEnd);
                            bm.attributes
                                .set(AttributeKey::BookmarkName, AttributeValue::String(name));
                            doc.insert_node(para_id, child_index, bm)
                                .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                            child_index += 1;
                        }
                    }
                    b"bookmark" => {
                        // Collapsed bookmark — create both start and end
                        if let Some(name) = get_attr(e, b"name") {
                            let bs_id = doc.next_id();
                            let mut bs = Node::new(bs_id, NodeType::BookmarkStart);
                            bs.attributes.set(
                                AttributeKey::BookmarkName,
                                AttributeValue::String(name.clone()),
                            );
                            doc.insert_node(para_id, child_index, bs)
                                .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                            child_index += 1;

                            let be_id = doc.next_id();
                            let mut be_node = Node::new(be_id, NodeType::BookmarkEnd);
                            be_node
                                .attributes
                                .set(AttributeKey::BookmarkName, AttributeValue::String(name));
                            doc.insert_node(para_id, child_index, be_node)
                                .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                            child_index += 1;
                        }
                    }
                    b"annotation-end" => {
                        if let Some(name) = get_attr(e, b"name") {
                            let ce_id = doc.next_id();
                            let mut ce = Node::new(ce_id, NodeType::CommentEnd);
                            ce.attributes
                                .set(AttributeKey::CommentId, AttributeValue::String(name));
                            doc.insert_node(para_id, child_index, ce)
                                .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                            child_index += 1;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Ok(text) = t.unescape() {
                    let text = text.to_string();
                    if !text.is_empty() {
                        let run_id = doc.next_id();
                        doc.insert_node(para_id, child_index, Node::new(run_id, NodeType::Run))
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, text))
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        child_index += 1;
                    }
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(para_id)
}

/// Parse a `<text:span>` and insert Run+Text nodes into `parent_id`.
///
/// Returns the number of nodes inserted at the parent level.
fn parse_span_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    start: &quick_xml::events::BytesStart<'_>,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    start_index: usize,
) -> Result<usize, OdtError> {
    let mut count = 0;

    // Get style attributes for this span
    let mut run_attrs = AttributeMap::new();
    if let Some(style_name) = get_attr(start, b"style-name") {
        if let Some(auto_attrs) = ctx.auto_styles.get(&style_name) {
            run_attrs.merge(auto_attrs);
        }
    }

    let end_tag = start.local_name();
    let end_tag_bytes = end_tag.as_ref().to_vec();

    loop {
        match reader.read_event() {
            Ok(Event::Text(ref t)) => {
                if let Ok(text) = t.unescape() {
                    let text = text.to_string();
                    if !text.is_empty() {
                        let run_id = doc.next_id();
                        let mut run_node = Node::new(run_id, NodeType::Run);
                        run_node.attributes.merge(&run_attrs);
                        doc.insert_node(parent_id, start_index + count, run_node)
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, text))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        count += 1;
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"line-break" => {
                        let lb_id = doc.next_id();
                        doc.insert_node(
                            parent_id,
                            start_index + count,
                            Node::new(lb_id, NodeType::LineBreak),
                        )
                        .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        count += 1;
                    }
                    b"tab" => {
                        let tab_id = doc.next_id();
                        doc.insert_node(
                            parent_id,
                            start_index + count,
                            Node::new(tab_id, NodeType::Tab),
                        )
                        .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        count += 1;
                    }
                    b"s" => {
                        let sc = get_attr(e, b"c")
                            .and_then(|v| v.parse::<usize>().ok())
                            .unwrap_or(1);
                        let run_id = doc.next_id();
                        let mut run_node = Node::new(run_id, NodeType::Run);
                        run_node.attributes.merge(&run_attrs);
                        doc.insert_node(parent_id, start_index + count, run_node)
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, " ".repeat(sc)))
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        count += 1;
                    }
                    b"page-number" => {
                        let field_id = doc.next_id();
                        let mut field_node = Node::new(field_id, NodeType::Field);
                        field_node.attributes.set(
                            AttributeKey::FieldType,
                            AttributeValue::FieldType(FieldType::PageNumber),
                        );
                        doc.insert_node(parent_id, start_index + count, field_node)
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        count += 1;
                    }
                    b"page-count" => {
                        let field_id = doc.next_id();
                        let mut field_node = Node::new(field_id, NodeType::Field);
                        field_node.attributes.set(
                            AttributeKey::FieldType,
                            AttributeValue::FieldType(FieldType::PageCount),
                        );
                        doc.insert_node(parent_id, start_index + count, field_node)
                            .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                        count += 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == end_tag_bytes.as_slice() => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(count)
}

/// Parse a `<text:a>` hyperlink element and insert Run+Text nodes with HyperlinkUrl.
///
/// Returns the number of nodes inserted at the parent level.
fn parse_hyperlink_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    start: &quick_xml::events::BytesStart<'_>,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    start_index: usize,
    url: &str,
) -> Result<usize, OdtError> {
    let mut count = 0;

    // Get style attributes for this anchor element
    let mut run_attrs = AttributeMap::new();
    if let Some(style_name) = get_attr(start, b"style-name") {
        if let Some(auto_attrs) = ctx.auto_styles.get(&style_name) {
            run_attrs.merge(auto_attrs);
        }
    }
    // Set hyperlink URL on all runs created within this link
    if !url.is_empty() {
        run_attrs.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String(url.to_string()),
        );
    }

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"span" => {
                // Nested span inside hyperlink — merge span style + hyperlink attrs
                let mut span_attrs = run_attrs.clone();
                if let Some(style_name) = get_attr(e, b"style-name") {
                    if let Some(auto_attrs) = ctx.auto_styles.get(&style_name) {
                        span_attrs.merge(auto_attrs);
                    }
                }
                // Parse span content with merged attributes
                loop {
                    match reader.read_event() {
                        Ok(Event::Text(ref t)) => {
                            if let Ok(text) = t.unescape() {
                                let text = text.to_string();
                                if !text.is_empty() {
                                    let run_id = doc.next_id();
                                    let mut run_node = Node::new(run_id, NodeType::Run);
                                    run_node.attributes.merge(&span_attrs);
                                    doc.insert_node(parent_id, start_index + count, run_node)
                                        .map_err(|e| {
                                            OdtError::InvalidStructure(format!("{e:?}"))
                                        })?;
                                    let text_id = doc.next_id();
                                    doc.insert_node(run_id, 0, Node::text(text_id, text))
                                        .map_err(|e| {
                                            OdtError::InvalidStructure(format!("{e:?}"))
                                        })?;
                                    count += 1;
                                }
                            }
                        }
                        Ok(Event::End(ref e)) if e.local_name().as_ref() == b"span" => break,
                        Ok(Event::Eof) => break,
                        Err(e) => return Err(OdtError::Xml(e.to_string())),
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Ok(text) = t.unescape() {
                    let text = text.to_string();
                    if !text.is_empty() {
                        let run_id = doc.next_id();
                        let mut run_node = Node::new(run_id, NodeType::Run);
                        run_node.attributes.merge(&run_attrs);
                        doc.insert_node(parent_id, start_index + count, run_node)
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, text))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        count += 1;
                    }
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"a" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(count)
}

/// Parse an `<office:annotation>` element.
///
/// Creates a CommentStart node (inline in the paragraph) and a CommentBody node
/// (as child of the Document root) to hold the annotation content.
/// Returns the number of nodes inserted at the parent level (always 1 for CommentStart).
fn parse_annotation_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    start: &quick_xml::events::BytesStart<'_>,
    parent_id: s1_model::NodeId,
    index: usize,
) -> Result<usize, OdtError> {
    let comment_name = get_attr(start, b"name").unwrap_or_default();
    let mut author: Option<String> = None;
    let mut date: Option<String> = None;

    // Create CommentBody node as child of Document root
    let body_id = doc.next_id();
    let body = Node::new(body_id, NodeType::CommentBody);
    let root_id = doc.root_id();
    let root_children = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
    doc.insert_node(root_id, root_children, body)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    let mut para_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                match e.local_name().as_ref() {
                    b"creator" => {
                        if let Ok(text) = reader.read_text(e.to_end().name()) {
                            author = Some(text.to_string());
                        }
                    }
                    b"date" => {
                        if let Ok(text) = reader.read_text(e.to_end().name()) {
                            date = Some(text.to_string());
                        }
                    }
                    b"p" => {
                        // Annotation body paragraph
                        let cp_id = doc.next_id();
                        doc.insert_node(body_id, para_index, Node::new(cp_id, NodeType::Paragraph))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        // Parse paragraph text content
                        parse_annotation_paragraph(reader, doc, cp_id)?;
                        para_index += 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"annotation" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    // Set metadata on the CommentBody
    let comment_id = if comment_name.is_empty() {
        format!("odt-comment-{}", body_id.counter)
    } else {
        comment_name
    };

    if let Some(body_node) = doc.node_mut(body_id) {
        body_node.attributes.set(
            AttributeKey::CommentId,
            AttributeValue::String(comment_id.clone()),
        );
        if let Some(a) = author {
            body_node
                .attributes
                .set(AttributeKey::CommentAuthor, AttributeValue::String(a));
        }
        if let Some(d) = date {
            body_node
                .attributes
                .set(AttributeKey::CommentDate, AttributeValue::String(d));
        }
    }

    // Create CommentStart node inline in the paragraph
    let cs_id = doc.next_id();
    let mut cs = Node::new(cs_id, NodeType::CommentStart);
    cs.attributes
        .set(AttributeKey::CommentId, AttributeValue::String(comment_id));
    doc.insert_node(parent_id, index, cs)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    Ok(1)
}

/// Parse text content inside an annotation paragraph.
fn parse_annotation_paragraph(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    para_id: s1_model::NodeId,
) -> Result<(), OdtError> {
    let mut child_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Text(ref t)) => {
                if let Ok(text) = t.unescape() {
                    let text = text.to_string();
                    if !text.is_empty() {
                        let run_id = doc.next_id();
                        doc.insert_node(para_id, child_index, Node::new(run_id, NodeType::Run))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, text))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        child_index += 1;
                    }
                }
            }
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"span" => {
                // Formatted text inside annotation — extract text
                if let Ok(text) = reader.read_text(e.to_end().name()) {
                    let text = text.to_string();
                    if !text.is_empty() {
                        let run_id = doc.next_id();
                        doc.insert_node(para_id, child_index, Node::new(run_id, NodeType::Run))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        let text_id = doc.next_id();
                        doc.insert_node(run_id, 0, Node::text(text_id, text))
                            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
                        child_index += 1;
                    }
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"p" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(())
}

/// Parse a `<draw:frame>` element and insert an Image node if possible.
///
/// Returns true if an image node was inserted.
fn parse_frame_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    start: &quick_xml::events::BytesStart<'_>,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    index: usize,
) -> Result<bool, OdtError> {
    let alt_text = get_attr(start, b"name").unwrap_or_default();

    let width = get_attr(start, b"width").and_then(|v| crate::xml_util::parse_length(&v));
    let height = get_attr(start, b"height").and_then(|v| crate::xml_util::parse_length(&v));

    let mut href: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if e.local_name().as_ref() == b"image" =>
            {
                href = get_attr(e, b"href");
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"frame" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    if href.is_none() {
        // ODT-11 (WONTFIX): Non-image content within <draw:frame> (e.g. text boxes,
        // embedded objects) is skipped. See ODT-11 for rationale.
        #[cfg(debug_assertions)]
        eprintln!(
            "[s1-format-odt] Note: non-image draw element within <draw:frame> skipped \
             (ODT-11 / WONTFIX)"
        );
        return Ok(false);
    }
    let href = href.unwrap();

    let media_id = match ctx.image_map.get(&href) {
        Some(id) => *id,
        None => {
            #[cfg(debug_assertions)]
            eprintln!(
                "[s1-format-odt] Warning: image reference not found in archive: {}",
                href
            );
            return Ok(false);
        }
    };

    let img_id = doc.next_id();
    let mut img_node = Node::new(img_id, NodeType::Image);
    img_node.attributes.set(
        AttributeKey::ImageMediaId,
        AttributeValue::MediaId(media_id),
    );
    if let Some(w) = width {
        img_node
            .attributes
            .set(AttributeKey::ImageWidth, AttributeValue::Float(w));
    }
    if let Some(h) = height {
        img_node
            .attributes
            .set(AttributeKey::ImageHeight, AttributeValue::Float(h));
    }
    if !alt_text.is_empty() {
        img_node
            .attributes
            .set(AttributeKey::ImageAltText, AttributeValue::String(alt_text));
    }

    doc.insert_node(parent_id, index, img_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    Ok(true)
}

/// Parse a `<text:note>` element (footnote or endnote).
///
/// Creates a FootnoteRef/EndnoteRef inline marker and a FootnoteBody/EndnoteBody
/// node attached to the document root. Returns the number of inline nodes inserted (0 or 1).
fn parse_note_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    _ctx: &ParseContext,
    start: &quick_xml::events::BytesStart<'_>,
    parent_id: s1_model::NodeId,
    index: usize,
) -> Result<usize, OdtError> {
    // Determine note class: "footnote" or "endnote"
    let note_class = get_attr(start, b"note-class").unwrap_or_else(|| "footnote".to_string());
    let is_endnote = note_class == "endnote";

    let mut citation_text = String::new();
    let mut body_paragraphs: Vec<String> = Vec::new();
    let mut in_body = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"note-citation" => {
                        // Read citation text (the footnote number/mark)
                    }
                    b"note-body" => {
                        in_body = true;
                    }
                    b"p" if in_body => {
                        // Collect paragraph text from note body
                        let mut para_text = String::new();
                        loop {
                            match reader.read_event() {
                                Ok(Event::Text(ref t)) => {
                                    if let Ok(text) = t.unescape() {
                                        para_text.push_str(&text);
                                    }
                                }
                                Ok(Event::Start(ref inner))
                                    if inner.local_name().as_ref() == b"span" => {}
                                Ok(Event::End(ref inner))
                                    if inner.local_name().as_ref() == b"span" => {}
                                Ok(Event::End(ref inner))
                                    if inner.local_name().as_ref() == b"p" =>
                                {
                                    break;
                                }
                                Ok(Event::Eof) => break,
                                Err(e) => return Err(OdtError::Xml(e.to_string())),
                                _ => {}
                            }
                        }
                        if !para_text.is_empty() {
                            body_paragraphs.push(para_text);
                        }
                    }
                    _ if in_body => {
                        skip_element_odt(reader)?;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if !in_body {
                    if let Ok(text) = t.unescape() {
                        citation_text.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"note-body" => in_body = false,
                    b"note-citation" => {}
                    b"note" => break,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    // Create the inline reference marker
    let ref_type = if is_endnote {
        NodeType::EndnoteRef
    } else {
        NodeType::FootnoteRef
    };
    let ref_id = doc.next_id();
    let mut ref_node = Node::new(ref_id, ref_type);

    // Parse citation into a typed attribute value and store on both the ref
    // node and the body node so the writer can match them during output.
    let attr_key = if is_endnote {
        AttributeKey::EndnoteNumber
    } else {
        AttributeKey::FootnoteNumber
    };
    let citation_value: Option<AttributeValue> = if !citation_text.is_empty() {
        if let Ok(num) = citation_text.parse::<i64>() {
            Some(AttributeValue::Int(num))
        } else {
            Some(AttributeValue::String(citation_text))
        }
    } else {
        None
    };

    if let Some(ref val) = citation_value {
        ref_node.attributes.set(attr_key.clone(), val.clone());
    }

    doc.insert_node(parent_id, index, ref_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    // Create the body node at the document root
    let body_type = if is_endnote {
        NodeType::EndnoteBody
    } else {
        NodeType::FootnoteBody
    };
    let body_node_id = doc.next_id();
    let mut body_node = Node::new(body_node_id, body_type);

    // Store the same note number on the body node so the writer can match
    // the inline reference to its body content.
    if let Some(val) = citation_value {
        body_node.attributes.set(attr_key, val);
    }

    let doc_root = doc.root_id();
    let root_child_count = doc.node(doc_root).map_or(0, |n| n.children.len());
    doc.insert_node(doc_root, root_child_count, body_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    // Add paragraph content to the body node
    for (i, text) in body_paragraphs.into_iter().enumerate() {
        let para_id = doc.next_id();
        let para_node = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_node_id, i, para_node)
            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, text))
            .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;
    }

    Ok(1)
}

/// Parse a `<text:list>` element, flattening items as Paragraph children of `parent_id`.
///
/// Returns the number of paragraphs added to the parent.
fn parse_list(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    start_index: usize,
    level: u8,
) -> Result<usize, OdtError> {
    let mut count = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"list-item" => {
                let item_count =
                    parse_list_item(reader, doc, ctx, parent_id, start_index + count, level)?;
                count += item_count;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"list" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(count)
}

/// Parse a `<text:list-item>` element.
///
/// **Design note (ODT-05 / WONTFIX):** ODF allows multi-paragraph list items
/// (several `<text:p>` children inside one `<text:list-item>`), but the s1-model
/// uses a flat paragraph model with `ListInfo` attributes. Each paragraph inside
/// a list item becomes a separate body-level paragraph tagged with its list level.
/// This means multi-paragraph list items lose their grouping — the first paragraph
/// gets the bullet/number and subsequent paragraphs appear as continuation
/// paragraphs at the same level. Nested `<text:list>` elements are handled by
/// incrementing the level counter. This is by design and would require a
/// fundamental model change to support true multi-paragraph list items.
fn parse_list_item(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    start_index: usize,
    level: u8,
) -> Result<usize, OdtError> {
    let mut count = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" | b"h" => {
                        let is_heading = local.as_ref() == b"h";
                        let heading_level = if is_heading {
                            get_attr(e, b"outline-level").and_then(|v| v.parse::<u8>().ok())
                        } else {
                            None
                        };
                        let node_id = parse_paragraph_into(
                            reader,
                            doc,
                            e,
                            ctx,
                            parent_id,
                            start_index + count,
                            is_heading,
                            heading_level,
                        )?;
                        // Set list info on the paragraph
                        if let Some(node) = doc.node_mut(node_id) {
                            node.attributes.set(
                                AttributeKey::ListInfo,
                                AttributeValue::ListInfo(ListInfo {
                                    level,
                                    num_format: ListFormat::Bullet,
                                    num_id: 0,
                                    start: None,
                                }),
                            );
                        }
                        if count > 0 {
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "[s1-format-odt] Note: multi-paragraph list item detected at level {}; \
                                 additional paragraphs are flattened (ODT-05 / WONTFIX)",
                                level
                            );
                        }
                        count += 1;
                    }
                    b"list" => {
                        // Nested list → increment level
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "[s1-format-odt] Note: nested <text:list> inside list-item at level {} \
                             flattened to level {} (ODT-05 / by design)",
                            level,
                            level + 1
                        );
                        let nested = parse_list(
                            reader,
                            doc,
                            ctx,
                            parent_id,
                            start_index + count,
                            level + 1,
                        )?;
                        count += nested;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"list-item" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(count)
}

/// Parse a `<table:table>` element and insert it into `parent_id`.
/// Parse `<text:table-of-content>` into a `NodeType::TableOfContents` node.
fn parse_toc_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    child_index: usize,
    _start_event: &quick_xml::events::BytesStart,
) -> Result<(), OdtError> {
    let toc_id = doc.next_id();
    let mut toc_node = Node::new(toc_id, NodeType::TableOfContents);
    toc_node.attributes.set(
        AttributeKey::TocMaxLevel,
        AttributeValue::Int(3), // default; updated from source element
    );
    doc.insert_node(parent_id, child_index, toc_node)
        .map_err(|e| OdtError::Xml(format!("TOC insert: {e}")))?;

    let mut toc_child_index = 0;
    let mut in_index_body = false;
    let mut in_index_title = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"table-of-content-source" => {
                        // Read outline-level attribute
                        if let Some(level_str) = get_attr(e, b"outline-level") {
                            if let Ok(level) = level_str.parse::<i64>() {
                                if let Some(toc) = doc.node_mut(toc_id) {
                                    toc.attributes
                                        .set(AttributeKey::TocMaxLevel, AttributeValue::Int(level));
                                }
                            }
                        }
                        // Preserve use-index-marks attribute
                        if let Some(val) = get_attr(e, b"use-index-marks") {
                            let flag = val == "true";
                            if let Some(toc) = doc.node_mut(toc_id) {
                                toc.attributes.set(
                                    AttributeKey::TocUseIndexMarks,
                                    AttributeValue::Bool(flag),
                                );
                            }
                        }
                        // Preserve use-index-source-styles attribute
                        if let Some(val) = get_attr(e, b"use-index-source-styles") {
                            let flag = val == "true";
                            if let Some(toc) = doc.node_mut(toc_id) {
                                toc.attributes.set(
                                    AttributeKey::TocUseIndexSourceStyles,
                                    AttributeValue::Bool(flag),
                                );
                            }
                        }
                        // Preserve index-scope attribute
                        if let Some(val) = get_attr(e, b"index-scope") {
                            if let Some(toc) = doc.node_mut(toc_id) {
                                toc.attributes
                                    .set(AttributeKey::TocIndexScope, AttributeValue::String(val));
                            }
                        }
                        // NOTE: Child elements like <text:index-entry-tab-stop> inside the
                        // source element are not yet preserved on round-trip.
                        skip_element_odt(reader)?;
                    }
                    b"index-body" => {
                        in_index_body = true;
                    }
                    b"index-title" => {
                        in_index_title = true;
                        // Read title text attribute
                        if let Some(title) = get_attr(e, b"name") {
                            if let Some(toc) = doc.node_mut(toc_id) {
                                toc.attributes
                                    .set(AttributeKey::TocTitle, AttributeValue::String(title));
                            }
                        }
                    }
                    b"p" if in_index_body && !in_index_title => {
                        // Cached entry paragraph
                        parse_paragraph_into(
                            reader,
                            doc,
                            e,
                            ctx,
                            toc_id,
                            toc_child_index,
                            false,
                            None,
                        )?;
                        toc_child_index += 1;
                    }
                    b"p" if in_index_title => {
                        // Title paragraph — skip it (we store title as attribute)
                        skip_element_odt(reader)?;
                    }
                    _ => {
                        skip_element_odt(reader)?;
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                let name = local.as_ref();
                match name {
                    b"index-body" => in_index_body = false,
                    b"index-title" => in_index_title = false,
                    b"table-of-content" => break,
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.local_name().as_ref().to_vec();
                if name.as_slice() == b"table-of-content-source" {
                    if let Some(level_str) = get_attr(e, b"outline-level") {
                        if let Ok(level) = level_str.parse::<i64>() {
                            if let Some(toc) = doc.node_mut(toc_id) {
                                toc.attributes
                                    .set(AttributeKey::TocMaxLevel, AttributeValue::Int(level));
                            }
                        }
                    }
                    if let Some(val) = get_attr(e, b"use-index-marks") {
                        let flag = val == "true";
                        if let Some(toc) = doc.node_mut(toc_id) {
                            toc.attributes
                                .set(AttributeKey::TocUseIndexMarks, AttributeValue::Bool(flag));
                        }
                    }
                    if let Some(val) = get_attr(e, b"use-index-source-styles") {
                        let flag = val == "true";
                        if let Some(toc) = doc.node_mut(toc_id) {
                            toc.attributes.set(
                                AttributeKey::TocUseIndexSourceStyles,
                                AttributeValue::Bool(flag),
                            );
                        }
                    }
                    if let Some(val) = get_attr(e, b"index-scope") {
                        if let Some(toc) = doc.node_mut(toc_id) {
                            toc.attributes
                                .set(AttributeKey::TocIndexScope, AttributeValue::String(val));
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(())
}

/// Capture the raw XML content of an element (including children) as a string.
///
/// The reader must be positioned just after reading the `Start` event for the
/// element identified by `tag_name`. The function reads until the matching `End`
/// event and returns the inner XML (including nested elements) as a String.
/// This is used for round-trip preservation of elements we don't semantically model.
fn capture_raw_xml(reader: &mut Reader<&[u8]>, tag_name: &str) -> Result<String, OdtError> {
    let mut depth = 1u32;
    let mut buf = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                buf.push('<');
                buf.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                for attr in e.attributes().flatten() {
                    buf.push(' ');
                    buf.push_str(&String::from_utf8_lossy(attr.key.as_ref()));
                    buf.push_str("=\"");
                    buf.push_str(&String::from_utf8_lossy(&attr.value));
                    buf.push('"');
                }
                buf.push('>');
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                buf.push_str("</");
                buf.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                buf.push('>');
            }
            Ok(Event::Empty(ref e)) => {
                buf.push('<');
                buf.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                for attr in e.attributes().flatten() {
                    buf.push(' ');
                    buf.push_str(&String::from_utf8_lossy(attr.key.as_ref()));
                    buf.push_str("=\"");
                    buf.push_str(&String::from_utf8_lossy(&attr.value));
                    buf.push('"');
                }
                buf.push_str("/>");
            }
            Ok(Event::Text(ref e)) => {
                buf.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }
    let _ = tag_name; // used for documentation clarity at call-site
    Ok(buf)
}

/// Skip an element and all its children (ODT version).
fn skip_element_odt(reader: &mut Reader<&[u8]>) -> Result<(), OdtError> {
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
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }
    Ok(())
}

fn parse_table_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
    parent_id: s1_model::NodeId,
    index: usize,
) -> Result<s1_model::NodeId, OdtError> {
    let table_id = doc.next_id();
    let table_node = Node::new(table_id, NodeType::Table);
    doc.insert_node(parent_id, index, table_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    let mut row_index = 0;
    let mut _column_count: usize = 0;
    let mut column_widths: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"table-row" => {
                        parse_table_row_into(reader, doc, ctx, table_id, row_index)?;
                        row_index += 1;
                    }
                    b"table-column" => {
                        let repeated = get_attr(e, b"number-columns-repeated")
                            .and_then(|v| v.parse::<usize>().ok())
                            .unwrap_or(1);
                        _column_count += repeated;
                        // Track column style names for width extraction
                        let style_name = get_attr(e, b"style-name").unwrap_or_default();
                        for _ in 0..repeated {
                            column_widths.push(style_name.clone());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"table-column" {
                    let repeated = get_attr(e, b"number-columns-repeated")
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(1);
                    _column_count += repeated;
                    let style_name = get_attr(e, b"style-name").unwrap_or_default();
                    for _ in 0..repeated {
                        column_widths.push(style_name.clone());
                    }
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"table" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    // Store column style names on the table node for round-trip fidelity.
    // Column widths are resolved from these style names during layout.
    if !column_widths.is_empty() && column_widths.iter().any(|w| !w.is_empty()) {
        let styles_str = column_widths.join(",");
        if let Some(table_node) = doc.node_mut(table_id) {
            table_node.attributes.set(
                AttributeKey::TableColumnWidths,
                AttributeValue::String(styles_str),
            );
        }
    }

    Ok(table_id)
}

/// Parse a `<table:table-row>` element and insert into parent table.
fn parse_table_row_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    ctx: &ParseContext,
    table_id: s1_model::NodeId,
    index: usize,
) -> Result<s1_model::NodeId, OdtError> {
    let row_id = doc.next_id();
    let row_node = Node::new(row_id, NodeType::TableRow);
    doc.insert_node(table_id, index, row_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    let mut cell_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"table-cell" => {
                parse_table_cell_into(reader, doc, e, ctx, row_id, cell_index)?;
                cell_index += 1;
            }
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"table-cell" => {
                // Empty cell
                let cell_id = doc.next_id();
                doc.insert_node(row_id, cell_index, Node::new(cell_id, NodeType::TableCell))
                    .map_err(|er| OdtError::InvalidStructure(format!("{er:?}")))?;
                cell_index += 1;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"table-row" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(row_id)
}

/// Parse a `<table:table-cell>` element and insert into parent row.
fn parse_table_cell_into(
    reader: &mut Reader<&[u8]>,
    doc: &mut DocumentModel,
    start: &quick_xml::events::BytesStart<'_>,
    ctx: &ParseContext,
    row_id: s1_model::NodeId,
    index: usize,
) -> Result<s1_model::NodeId, OdtError> {
    let cell_id = doc.next_id();
    let mut cell_node = Node::new(cell_id, NodeType::TableCell);

    // Column span
    if let Some(span) = get_attr(start, b"number-columns-spanned") {
        if let Ok(n) = span.parse::<i64>() {
            if n > 1 {
                cell_node
                    .attributes
                    .set(AttributeKey::ColSpan, AttributeValue::Int(n));
            }
        }
    }

    // Row span
    if let Some(span) = get_attr(start, b"number-rows-spanned") {
        if let Ok(n) = span.parse::<i64>() {
            if n > 1 {
                cell_node
                    .attributes
                    .set(AttributeKey::RowSpan, AttributeValue::Int(n));
            }
        }
    }

    // Apply cell style
    if let Some(style_name) = get_attr(start, b"style-name") {
        if let Some(auto_attrs) = ctx.auto_styles.get(&style_name) {
            // Extract cell-relevant attributes
            if let Some(va) = auto_attrs.get(&AttributeKey::VerticalAlign) {
                cell_node
                    .attributes
                    .set(AttributeKey::VerticalAlign, va.clone());
            }
            if let Some(bg) = auto_attrs.get(&AttributeKey::CellBackground) {
                cell_node
                    .attributes
                    .set(AttributeKey::CellBackground, bg.clone());
            }
        }
    }

    doc.insert_node(row_id, index, cell_node)
        .map_err(|e| OdtError::InvalidStructure(format!("{e:?}")))?;

    let mut child_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"p" | b"h" => {
                        let is_heading = local.as_ref() == b"h";
                        let heading_level = if is_heading {
                            get_attr(e, b"outline-level").and_then(|v| v.parse::<u8>().ok())
                        } else {
                            None
                        };
                        parse_paragraph_into(
                            reader,
                            doc,
                            e,
                            ctx,
                            cell_id,
                            child_index,
                            is_heading,
                            heading_level,
                        )?;
                        child_index += 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"table-cell" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(cell_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_ctx() -> ParseContext {
        ParseContext {
            auto_styles: HashMap::new(),
            image_map: HashMap::new(),
        }
    }

    fn parse_body_xml(xml: &str) -> DocumentModel {
        let full = format!(
            r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/"><office:body><office:text>{}</office:text></office:body></office:document-content>"#,
            xml
        );
        let mut doc = DocumentModel::new();
        let ctx = make_ctx();
        let mut reader = Reader::from_reader(full.as_bytes());

        // Advance to <office:text>
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"text" => break,
                Ok(Event::Eof) => panic!("no <office:text> found"),
                _ => {}
            }
        }

        parse_content_body(&mut reader, &mut doc, &ctx).unwrap();
        doc
    }

    #[test]
    fn parse_single_paragraph() {
        let doc = parse_body_xml("<text:p>Hello world</text:p>");
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.node_type, NodeType::Paragraph);
        assert_eq!(para.children.len(), 1); // one run

        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.node_type, NodeType::Run);
        assert_eq!(run.children.len(), 1);

        let text = doc.node(run.children[0]).unwrap();
        assert_eq!(text.text_content.as_deref(), Some("Hello world"));
    }

    #[test]
    fn parse_multiple_paragraphs() {
        let doc =
            parse_body_xml("<text:p>First</text:p><text:p>Second</text:p><text:p>Third</text:p>");
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 3);
    }

    #[test]
    fn parse_span_formatting() {
        let full = r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"><office:automatic-styles><style:style style:name="T1" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style></office:automatic-styles><office:body><office:text><text:p>Hello <text:span text:style-name="T1">bold</text:span> world</text:p></office:text></office:body></office:document-content>"#;

        let mut doc = DocumentModel::new();
        let mut reader = Reader::from_reader(full.as_bytes());

        // Parse auto styles
        let mut auto_styles = HashMap::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"automatic-styles" => {
                    auto_styles = crate::style_parser::parse_automatic_styles(&mut reader).unwrap();
                    break;
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }

        // Advance to <office:text>
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"text" => break,
                Ok(Event::Eof) => panic!("no <office:text> found"),
                _ => {}
            }
        }

        let ctx = ParseContext {
            auto_styles,
            image_map: HashMap::new(),
        };
        parse_content_body(&mut reader, &mut doc, &ctx).unwrap();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let para = doc.node(body.children[0]).unwrap();
        // Should have 3 runs: "Hello ", bold "bold", " world"
        assert!(para.children.len() >= 2);

        // Check the bold run has Bold attribute
        let bold_run = doc.node(para.children[1]).unwrap();
        assert_eq!(
            bold_run.attributes.get_bool(&AttributeKey::Bold),
            Some(true)
        );
    }

    #[test]
    fn parse_list_items() {
        let doc = parse_body_xml(
            r#"<text:list><text:list-item><text:p>Item 1</text:p></text:list-item><text:list-item><text:p>Item 2</text:p></text:list-item></text:list>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        // List items flattened as body children
        assert_eq!(body.children.len(), 2);

        let p1 = doc.node(body.children[0]).unwrap();
        assert_eq!(p1.node_type, NodeType::Paragraph);
        // Should have ListInfo attribute
        assert!(p1.attributes.get(&AttributeKey::ListInfo).is_some());
    }

    #[test]
    fn parse_table_basic() {
        let doc = parse_body_xml(
            r#"<table:table><table:table-row><table:table-cell><text:p>A1</text:p></table:table-cell><table:table-cell><text:p>B1</text:p></table:table-cell></table:table-row></table:table>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let table = doc.node(body.children[0]).unwrap();
        assert_eq!(table.node_type, NodeType::Table);
        assert_eq!(table.children.len(), 1); // 1 row

        let row = doc.node(table.children[0]).unwrap();
        assert_eq!(row.node_type, NodeType::TableRow);
        assert_eq!(row.children.len(), 2); // 2 cells
    }

    #[test]
    fn parse_heading() {
        let doc = parse_body_xml(r#"<text:h text:outline-level="1">Chapter 1</text:h>"#);
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let heading = doc.node(body.children[0]).unwrap();
        assert_eq!(heading.node_type, NodeType::Paragraph);
        assert_eq!(
            heading.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading1")
        );
    }

    #[test]
    fn parse_empty_paragraph() {
        let doc = parse_body_xml("<text:p></text:p>");
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);
        let para = doc.node(body.children[0]).unwrap();
        assert!(para.children.is_empty());
    }

    #[test]
    fn parse_toc_element() {
        let doc = parse_body_xml(
            r#"<text:table-of-content text:name="TOC" text:protected="false">
                <text:table-of-content-source text:outline-level="2"/>
                <text:index-body>
                    <text:p>Chapter One</text:p>
                    <text:p>Section A</text:p>
                </text:index-body>
            </text:table-of-content>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);

        let toc = doc.node(body.children[0]).unwrap();
        assert_eq!(toc.node_type, NodeType::TableOfContents);

        // Should have max level = 2
        assert_eq!(
            toc.attributes.get(&AttributeKey::TocMaxLevel),
            Some(&AttributeValue::Int(2))
        );

        // Should have 2 cached entry paragraphs
        assert_eq!(toc.children.len(), 2);
        let entry1 = doc.node(toc.children[0]).unwrap();
        assert_eq!(entry1.node_type, NodeType::Paragraph);
    }

    #[test]
    fn parse_toc_with_title() {
        let doc = parse_body_xml(
            r#"<text:table-of-content text:name="TOC">
                <text:table-of-content-source text:outline-level="3"/>
                <text:index-body>
                    <text:index-title text:name="Table of Contents">
                        <text:p>Table of Contents</text:p>
                    </text:index-title>
                    <text:p>Entry One</text:p>
                </text:index-body>
            </text:table-of-content>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let toc = doc.node(body.children[0]).unwrap();
        assert_eq!(toc.node_type, NodeType::TableOfContents);

        // Title should be stored
        assert_eq!(
            toc.attributes.get_string(&AttributeKey::TocTitle),
            Some("Table of Contents")
        );

        // Entry paragraphs (title paragraph excluded, only body entries)
        assert!(!toc.children.is_empty());
    }

    #[test]
    fn parse_hyperlink_external() {
        let doc = parse_body_xml(
            r#"<text:p>Click <text:a xlink:href="https://example.com" xlink:type="simple">here</text:a> now</text:p>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have 3 runs: "Click ", hyperlink "here", " now"
        assert!(para.children.len() >= 2);

        // Find the run with HyperlinkUrl
        let mut found_hyperlink = false;
        for &cid in &para.children {
            let child = doc.node(cid).unwrap();
            if let Some(url) = child.attributes.get_string(&AttributeKey::HyperlinkUrl) {
                assert_eq!(url, "https://example.com");
                // Check text
                let text = doc.node(child.children[0]).unwrap();
                assert_eq!(text.text_content.as_deref(), Some("here"));
                found_hyperlink = true;
            }
        }
        assert!(found_hyperlink, "No hyperlink run found");
    }

    #[test]
    fn parse_hyperlink_with_span() {
        let doc = parse_body_xml(
            r#"<text:p><text:a xlink:href="https://test.org"><text:span text:style-name="T1">link text</text:span></text:a></text:p>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        assert!(!para.children.is_empty());
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("https://test.org")
        );
    }

    #[test]
    fn parse_bookmark_start_end() {
        let doc = parse_body_xml(
            r#"<text:p><text:bookmark-start text:name="bm1"/>Some text<text:bookmark-end text:name="bm1"/></text:p>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // First child should be BookmarkStart
        let bs = doc.node(para.children[0]).unwrap();
        assert_eq!(bs.node_type, NodeType::BookmarkStart);
        assert_eq!(
            bs.attributes.get_string(&AttributeKey::BookmarkName),
            Some("bm1")
        );

        // Last child should be BookmarkEnd
        let be = doc.node(*para.children.last().unwrap()).unwrap();
        assert_eq!(be.node_type, NodeType::BookmarkEnd);
        assert_eq!(
            be.attributes.get_string(&AttributeKey::BookmarkName),
            Some("bm1")
        );
    }

    #[test]
    fn parse_bookmark_collapsed() {
        let doc =
            parse_body_xml(r#"<text:p><text:bookmark text:name="point1"/>Some text</text:p>"#);
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Should have BookmarkStart and BookmarkEnd as first two children
        let bs = doc.node(para.children[0]).unwrap();
        assert_eq!(bs.node_type, NodeType::BookmarkStart);
        assert_eq!(
            bs.attributes.get_string(&AttributeKey::BookmarkName),
            Some("point1")
        );

        let be = doc.node(para.children[1]).unwrap();
        assert_eq!(be.node_type, NodeType::BookmarkEnd);
        assert_eq!(
            be.attributes.get_string(&AttributeKey::BookmarkName),
            Some("point1")
        );
    }

    #[test]
    fn parse_annotation_single() {
        let doc = parse_body_xml(
            r#"<text:p>Hello <office:annotation office:name="c1"><dc:creator>Alice</dc:creator><dc:date>2024-01-15T10:30:00</dc:date><text:p>Nice work!</text:p></office:annotation>world<office:annotation-end office:name="c1"/></text:p>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Find CommentStart
        let mut found_cs = false;
        let mut found_ce = false;
        for &cid in &para.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::CommentStart {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentId),
                    Some("c1")
                );
                found_cs = true;
            }
            if child.node_type == NodeType::CommentEnd {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentId),
                    Some("c1")
                );
                found_ce = true;
            }
        }
        assert!(found_cs, "CommentStart not found");
        assert!(found_ce, "CommentEnd not found");

        // Check CommentBody on the Document root
        let root_id = doc.root_id();
        let root = doc.node(root_id).unwrap();
        let mut found_body = false;
        for &cid in &root.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::CommentBody {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentId),
                    Some("c1")
                );
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentAuthor),
                    Some("Alice")
                );
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentDate),
                    Some("2024-01-15T10:30:00")
                );
                // One paragraph child
                assert_eq!(child.children.len(), 1);
                found_body = true;
            }
        }
        assert!(found_body, "CommentBody not found");
    }

    #[test]
    fn parse_annotation_no_date() {
        let doc = parse_body_xml(
            r#"<text:p><office:annotation office:name="c2"><dc:creator>Bob</dc:creator><text:p>Comment text</text:p></office:annotation>text<office:annotation-end office:name="c2"/></text:p>"#,
        );
        let root_id = doc.root_id();
        let root = doc.node(root_id).unwrap();

        let mut found = false;
        for &cid in &root.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::CommentBody {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentAuthor),
                    Some("Bob")
                );
                assert!(child
                    .attributes
                    .get_string(&AttributeKey::CommentDate)
                    .is_none());
                found = true;
            }
        }
        assert!(found);
    }

    #[test]
    fn parse_annotation_multi_paragraph() {
        let doc = parse_body_xml(
            r#"<text:p><office:annotation office:name="c3"><dc:creator>Eve</dc:creator><text:p>First para</text:p><text:p>Second para</text:p></office:annotation>annotated<office:annotation-end office:name="c3"/></text:p>"#,
        );
        let root_id = doc.root_id();
        let root = doc.node(root_id).unwrap();

        for &cid in &root.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::CommentBody {
                assert_eq!(child.children.len(), 2);
                return;
            }
        }
        panic!("CommentBody not found");
    }

    #[test]
    fn parse_footnote_basic() {
        let doc = parse_body_xml(
            r#"<text:p>Hello<text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>Footnote text</text:p></text:note-body></text:note> world</text:p>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        // Find FootnoteRef in the paragraph children
        let mut found_ref = false;
        for &cid in &para.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::FootnoteRef {
                assert_eq!(
                    child.attributes.get(&AttributeKey::FootnoteNumber),
                    Some(&AttributeValue::Int(1))
                );
                found_ref = true;
            }
        }
        assert!(found_ref, "FootnoteRef not found in paragraph");

        // Check FootnoteBody on the Document root
        let root_id = doc.root_id();
        let root = doc.node(root_id).unwrap();
        let mut found_body = false;
        for &cid in &root.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::FootnoteBody {
                // Body should have the same note number for writer matching
                assert_eq!(
                    child.attributes.get(&AttributeKey::FootnoteNumber),
                    Some(&AttributeValue::Int(1))
                );
                assert_eq!(child.children.len(), 1);
                found_body = true;
            }
        }
        assert!(found_body, "FootnoteBody not found on document root");
    }

    #[test]
    fn parse_endnote_basic() {
        let doc = parse_body_xml(
            r#"<text:p>Text<text:note text:note-class="endnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>Endnote content</text:p></text:note-body></text:note></text:p>"#,
        );
        let root_id = doc.root_id();
        let root = doc.node(root_id).unwrap();

        let mut found = false;
        for &cid in &root.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::EndnoteBody {
                assert_eq!(
                    child.attributes.get(&AttributeKey::EndnoteNumber),
                    Some(&AttributeValue::Int(1))
                );
                found = true;
            }
        }
        assert!(found, "EndnoteBody not found");
    }

    #[test]
    fn parse_toc_source_attributes() {
        let doc = parse_body_xml(
            r#"<text:table-of-content text:name="TOC">
                <text:table-of-content-source text:outline-level="4" text:use-index-marks="true" text:use-index-source-styles="false" text:index-scope="document">
                    <text:index-title-template>Contents</text:index-title-template>
                </text:table-of-content-source>
                <text:index-body>
                    <text:p>Entry 1</text:p>
                </text:index-body>
            </text:table-of-content>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let toc = doc.node(body.children[0]).unwrap();

        assert_eq!(
            toc.attributes.get(&AttributeKey::TocMaxLevel),
            Some(&AttributeValue::Int(4))
        );
        assert_eq!(
            toc.attributes.get_bool(&AttributeKey::TocUseIndexMarks),
            Some(true)
        );
        assert_eq!(
            toc.attributes
                .get_bool(&AttributeKey::TocUseIndexSourceStyles),
            Some(false)
        );
        assert_eq!(
            toc.attributes.get_string(&AttributeKey::TocIndexScope),
            Some("document")
        );
    }

    #[test]
    fn parse_toc_source_empty_element() {
        // Test the Empty event path (self-closing source element with attributes)
        let doc = parse_body_xml(
            r#"<text:table-of-content text:name="TOC">
                <text:table-of-content-source text:outline-level="5" text:use-index-marks="false" text:index-scope="chapter"/>
                <text:index-body>
                    <text:p>Entry</text:p>
                </text:index-body>
            </text:table-of-content>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let toc = doc.node(body.children[0]).unwrap();

        assert_eq!(
            toc.attributes.get(&AttributeKey::TocMaxLevel),
            Some(&AttributeValue::Int(5))
        );
        assert_eq!(
            toc.attributes.get_bool(&AttributeKey::TocUseIndexMarks),
            Some(false)
        );
        assert_eq!(
            toc.attributes.get_string(&AttributeKey::TocIndexScope),
            Some("chapter")
        );
    }

    #[test]
    fn parse_nested_list_levels() {
        let doc = parse_body_xml(
            r#"<text:list>
                <text:list-item>
                    <text:p>Level 0 item</text:p>
                    <text:list>
                        <text:list-item>
                            <text:p>Level 1 item</text:p>
                        </text:list-item>
                    </text:list>
                </text:list-item>
            </text:list>"#,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        // Should have 2 flattened paragraphs
        assert_eq!(body.children.len(), 2);

        let p0 = doc.node(body.children[0]).unwrap();
        let li0 = p0
            .attributes
            .get_list_info(&AttributeKey::ListInfo)
            .unwrap();
        assert_eq!(li0.level, 0);

        let p1 = doc.node(body.children[1]).unwrap();
        let li1 = p1
            .attributes
            .get_list_info(&AttributeKey::ListInfo)
            .unwrap();
        assert_eq!(li1.level, 1);
    }

    #[test]
    fn parse_internal_bookmark_hyperlink() {
        // ODT-12: Internal hyperlinks to bookmarks are preserved as regular hyperlinks
        let doc = parse_body_xml(
            r##"<text:p><text:bookmark-start text:name="target"/>Target text<text:bookmark-end text:name="target"/></text:p><text:p><text:a xlink:href="#target">Go to target</text:a></text:p>"##,
        );
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 2);

        // Second paragraph should have a hyperlink run with href="#target"
        let link_para = doc.node(body.children[1]).unwrap();
        let mut found = false;
        for &cid in &link_para.children {
            let child = doc.node(cid).unwrap();
            if let Some(url) = child.attributes.get_string(&AttributeKey::HyperlinkUrl) {
                assert_eq!(url, "#target");
                found = true;
            }
        }
        assert!(found, "Internal bookmark hyperlink not preserved");
    }

    #[test]
    fn parse_annotation_end_only() {
        let doc =
            parse_body_xml(r#"<text:p>text<office:annotation-end office:name="orphan"/></text:p>"#);
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();

        let mut found = false;
        for &cid in &para.children {
            let child = doc.node(cid).unwrap();
            if child.node_type == NodeType::CommentEnd {
                assert_eq!(
                    child.attributes.get_string(&AttributeKey::CommentId),
                    Some("orphan")
                );
                found = true;
            }
        }
        assert!(found, "annotation-end should create CommentEnd");
    }
}
