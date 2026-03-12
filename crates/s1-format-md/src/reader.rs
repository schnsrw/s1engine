//! Markdown reader using pulldown-cmark.
//!
//! Converts a Markdown string into a [`DocumentModel`].

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use s1_model::{
    AttributeKey, AttributeValue, DocumentModel, ListFormat, ListInfo, Node, NodeId, NodeType,
};

use crate::MdError;

/// Read a Markdown string into a [`DocumentModel`].
pub fn read(input: &str) -> Result<DocumentModel, MdError> {
    let mut doc = DocumentModel::new();
    let body_id = doc
        .body_id()
        .ok_or_else(|| MdError::Model("no body".into()))?;

    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(input, opts);

    let mut ctx = ReadContext {
        body_id,
        body_child_index: 0,
        container_stack: Vec::new(),
        bold: false,
        italic: false,
        strikethrough: false,
        code: false,
        link_url: None,
        list_stack: Vec::new(),
        numbering_counter: 0,
        in_table: false,
        table_id: None,
        table_row_id: None,
        table_child_index: 0,
        row_child_index: 0,
        cell_para_id: None,
        cell_child_index: 0,
    };

    for event in parser {
        process_event(&mut doc, &mut ctx, event)?;
    }

    Ok(doc)
}

struct ReadContext {
    body_id: NodeId,
    body_child_index: usize,
    container_stack: Vec<(NodeId, usize)>,
    bold: bool,
    italic: bool,
    strikethrough: bool,
    code: bool,
    link_url: Option<String>,
    list_stack: Vec<ListState>,
    numbering_counter: u32,
    in_table: bool,
    table_id: Option<NodeId>,
    table_row_id: Option<NodeId>,
    table_child_index: usize,
    row_child_index: usize,
    cell_para_id: Option<NodeId>,
    cell_child_index: usize,
}

struct ListState {
    num_id: u32,
    ordered: bool,
}

fn process_event(
    doc: &mut DocumentModel,
    ctx: &mut ReadContext,
    event: Event<'_>,
) -> Result<(), MdError> {
    match event {
        Event::Start(tag) => match tag {
            Tag::Paragraph => {
                if !ctx.in_table {
                    let para_id = doc.next_id();
                    insert_node(doc, ctx.body_id, ctx.body_child_index, para_id, NodeType::Paragraph)?;
                    ctx.body_child_index += 1;
                    ctx.container_stack.push((para_id, 0));
                }
            }
            Tag::Heading { level, .. } => {
                let para_id = doc.next_id();
                let mut para = Node::new(para_id, NodeType::Paragraph);
                let level_num = heading_level_to_u8(level);
                para.attributes.set(
                    AttributeKey::StyleId,
                    AttributeValue::String(format!("Heading{}", level_num)),
                );
                doc.insert_node(ctx.body_id, ctx.body_child_index, para)
                    .map_err(|e| MdError::Model(e.to_string()))?;
                ctx.body_child_index += 1;
                ctx.container_stack.push((para_id, 0));
            }
            Tag::Emphasis => {
                ctx.italic = true;
            }
            Tag::Strong => {
                ctx.bold = true;
            }
            Tag::Strikethrough => {
                ctx.strikethrough = true;
            }
            Tag::CodeBlock(_kind) => {
                let para_id = doc.next_id();
                insert_node(doc, ctx.body_id, ctx.body_child_index, para_id, NodeType::Paragraph)?;
                ctx.body_child_index += 1;
                ctx.container_stack.push((para_id, 0));
                ctx.code = true;
            }
            Tag::Link { dest_url, .. } => {
                ctx.link_url = Some(dest_url.to_string());
            }
            Tag::Image { dest_url, title, .. } => {
                // Store image reference as attributes on a placeholder image node
                if let Some((parent_id, _)) = ctx.container_stack.last().copied() {
                    let img_id = doc.next_id();
                    let mut img = Node::new(img_id, NodeType::Image);
                    img.attributes.set(
                        AttributeKey::ImageAltText,
                        AttributeValue::String(title.to_string()),
                    );
                    // Store source URL in a generic attribute
                    img.attributes.set(
                        AttributeKey::HyperlinkUrl,
                        AttributeValue::String(dest_url.to_string()),
                    );
                    let child_idx = ctx.container_stack.last().map(|c| c.1).unwrap_or(0);
                    doc.insert_node(parent_id, child_idx, img)
                        .map_err(|e| MdError::Model(e.to_string()))?;
                    if let Some(last) = ctx.container_stack.last_mut() {
                        last.1 += 1;
                    }
                }
            }
            Tag::List(first_item) => {
                ctx.numbering_counter += 1;
                ctx.list_stack.push(ListState {
                    num_id: ctx.numbering_counter,
                    ordered: first_item.is_some(),
                });
            }
            Tag::Item => {
                let para_id = doc.next_id();
                let mut para = Node::new(para_id, NodeType::Paragraph);

                if let Some(list_state) = ctx.list_stack.last() {
                    let level = ctx.list_stack.len() as u8;
                    let num_format = if list_state.ordered {
                        ListFormat::Decimal
                    } else {
                        ListFormat::Bullet
                    };
                    para.attributes.set(
                        AttributeKey::ListInfo,
                        AttributeValue::ListInfo(ListInfo {
                            level,
                            num_format,
                            num_id: list_state.num_id,
                            start: None,
                        }),
                    );
                }

                doc.insert_node(ctx.body_id, ctx.body_child_index, para)
                    .map_err(|e| MdError::Model(e.to_string()))?;
                ctx.body_child_index += 1;
                ctx.container_stack.push((para_id, 0));
            }
            Tag::BlockQuote(_) => {
                ctx.container_stack.push((ctx.body_id, ctx.body_child_index));
            }
            Tag::Table(_alignments) => {
                let table_id = doc.next_id();
                insert_node(doc, ctx.body_id, ctx.body_child_index, table_id, NodeType::Table)?;
                ctx.body_child_index += 1;
                ctx.in_table = true;
                ctx.table_id = Some(table_id);
                ctx.table_child_index = 0;
            }
            Tag::TableHead => {
                if let Some(table_id) = ctx.table_id {
                    let row_id = doc.next_id();
                    insert_node(doc, table_id, ctx.table_child_index, row_id, NodeType::TableRow)?;
                    ctx.table_child_index += 1;
                    ctx.table_row_id = Some(row_id);
                    ctx.row_child_index = 0;
                }
            }
            Tag::TableRow => {
                if let Some(table_id) = ctx.table_id {
                    let row_id = doc.next_id();
                    insert_node(doc, table_id, ctx.table_child_index, row_id, NodeType::TableRow)?;
                    ctx.table_child_index += 1;
                    ctx.table_row_id = Some(row_id);
                    ctx.row_child_index = 0;
                }
            }
            Tag::TableCell => {
                if let Some(row_id) = ctx.table_row_id {
                    let cell_id = doc.next_id();
                    insert_node(doc, row_id, ctx.row_child_index, cell_id, NodeType::TableCell)?;
                    ctx.row_child_index += 1;

                    let para_id = doc.next_id();
                    insert_node(doc, cell_id, 0, para_id, NodeType::Paragraph)?;
                    ctx.cell_para_id = Some(para_id);
                    ctx.cell_child_index = 0;
                }
            }
            _ => {}
        },

        Event::End(tag_end) => match tag_end {
            TagEnd::Paragraph => {
                if !ctx.in_table {
                    ctx.container_stack.pop();
                }
            }
            TagEnd::Heading(_) => {
                ctx.container_stack.pop();
            }
            TagEnd::Emphasis => {
                ctx.italic = false;
            }
            TagEnd::Strong => {
                ctx.bold = false;
            }
            TagEnd::Strikethrough => {
                ctx.strikethrough = false;
            }
            TagEnd::CodeBlock => {
                ctx.code = false;
                ctx.container_stack.pop();
            }
            TagEnd::Link => {
                ctx.link_url = None;
            }
            TagEnd::Image => {}
            TagEnd::List(_) => {
                ctx.list_stack.pop();
            }
            TagEnd::Item => {
                ctx.container_stack.pop();
            }
            TagEnd::BlockQuote(_) => {
                ctx.container_stack.pop();
            }
            TagEnd::Table => {
                ctx.in_table = false;
                ctx.table_id = None;
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                ctx.table_row_id = None;
            }
            TagEnd::TableCell => {
                ctx.cell_para_id = None;
            }
            _ => {}
        },

        Event::Text(text) => {
            emit_text(doc, ctx, &text)?;
        }

        Event::Code(code) => {
            let old_code = ctx.code;
            ctx.code = true;
            emit_text(doc, ctx, &code)?;
            ctx.code = old_code;
        }

        Event::SoftBreak => {
            emit_text(doc, ctx, " ")?;
        }

        Event::HardBreak => {
            if let Some(&(parent_id, child_idx)) = ctx.container_stack.last() {
                let br_id = doc.next_id();
                insert_node(doc, parent_id, child_idx, br_id, NodeType::LineBreak)?;
                if let Some(last) = ctx.container_stack.last_mut() {
                    last.1 += 1;
                }
            }
        }

        Event::Rule => {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::PageBreakBefore,
                AttributeValue::Bool(true),
            );
            doc.insert_node(ctx.body_id, ctx.body_child_index, para)
                .map_err(|e| MdError::Model(e.to_string()))?;
            ctx.body_child_index += 1;
        }

        _ => {}
    }

    Ok(())
}

/// Emit text content into the current container as a Run node.
fn emit_text(doc: &mut DocumentModel, ctx: &mut ReadContext, text: &str) -> Result<(), MdError> {
    let (parent_id, child_idx) = if ctx.in_table {
        if let Some(para_id) = ctx.cell_para_id {
            (para_id, ctx.cell_child_index)
        } else {
            return Ok(());
        }
    } else if let Some(&(parent_id, child_idx)) = ctx.container_stack.last() {
        (parent_id, child_idx)
    } else {
        return Ok(());
    };

    let run_id = doc.next_id();
    let mut run = Node::new(run_id, NodeType::Run);
    if ctx.bold {
        run.attributes.set(AttributeKey::Bold, AttributeValue::Bool(true));
    }
    if ctx.italic {
        run.attributes.set(AttributeKey::Italic, AttributeValue::Bool(true));
    }
    if ctx.strikethrough {
        run.attributes.set(AttributeKey::Strikethrough, AttributeValue::Bool(true));
    }
    if ctx.code {
        run.attributes.set(
            AttributeKey::FontFamily,
            AttributeValue::String("monospace".into()),
        );
    }
    if let Some(ref url) = ctx.link_url {
        run.attributes.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String(url.clone()),
        );
    }

    doc.insert_node(parent_id, child_idx, run)
        .map_err(|e| MdError::Model(e.to_string()))?;

    let text_id = doc.next_id();
    doc.insert_node(run_id, 0, Node::text(text_id, text))
        .map_err(|e| MdError::Model(e.to_string()))?;

    if ctx.in_table {
        ctx.cell_child_index += 1;
    } else if let Some(last) = ctx.container_stack.last_mut() {
        last.1 += 1;
    }

    Ok(())
}

fn insert_node(
    doc: &mut DocumentModel,
    parent_id: NodeId,
    child_index: usize,
    node_id: NodeId,
    node_type: NodeType,
) -> Result<(), MdError> {
    doc.insert_node(parent_id, child_index, Node::new(node_id, node_type))
        .map_err(|e| MdError::Model(e.to_string()))
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_empty() {
        let doc = read("").unwrap();
        assert_eq!(doc.to_plain_text(), "");
    }

    #[test]
    fn read_single_paragraph() {
        let doc = read("Hello world").unwrap();
        assert_eq!(doc.to_plain_text(), "Hello world");
    }

    #[test]
    fn read_multiple_paragraphs() {
        let doc = read("First\n\nSecond\n\nThird").unwrap();
        let text = doc.to_plain_text();
        assert!(text.contains("First"));
        assert!(text.contains("Second"));
        assert!(text.contains("Third"));
    }

    #[test]
    fn read_heading_levels() {
        let doc = read("# H1\n\n## H2\n\n### H3").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let h1 = doc.node(body.children[0]).unwrap();
        assert_eq!(
            h1.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading1")
        );

        let h2 = doc.node(body.children[1]).unwrap();
        assert_eq!(
            h2.attributes.get_string(&AttributeKey::StyleId),
            Some("Heading2")
        );
    }

    #[test]
    fn read_bold() {
        let doc = read("**bold text**").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.attributes.get_bool(&AttributeKey::Bold), Some(true));
    }

    #[test]
    fn read_italic() {
        let doc = read("*italic text*").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.attributes.get_bool(&AttributeKey::Italic), Some(true));
    }

    #[test]
    fn read_bold_italic() {
        let doc = read("***bold italic***").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(run.attributes.get_bool(&AttributeKey::Italic), Some(true));
    }

    #[test]
    fn read_strikethrough() {
        let doc = read("~~struck~~").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.attributes.get_bool(&AttributeKey::Strikethrough), Some(true));
    }

    #[test]
    fn read_inline_code() {
        let doc = read("`code`").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(run.attributes.get_string(&AttributeKey::FontFamily), Some("monospace"));
    }

    #[test]
    fn read_code_block() {
        let doc = read("```\nfn main() {}\n```").unwrap();
        let text = doc.to_plain_text();
        assert!(text.contains("fn main()"));
    }

    #[test]
    fn read_hyperlink() {
        let doc = read("[Click here](https://example.com)").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes.get_string(&AttributeKey::HyperlinkUrl),
            Some("https://example.com")
        );
        assert_eq!(doc.to_plain_text(), "Click here");
    }

    #[test]
    fn read_unordered_list() {
        let doc = read("- Item 1\n- Item 2").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert!(body.children.len() >= 2);

        let item1 = doc.node(body.children[0]).unwrap();
        match item1.attributes.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(info)) => {
                assert_eq!(info.num_format, ListFormat::Bullet);
            }
            other => panic!("Expected ListInfo, got {:?}", other),
        }
    }

    #[test]
    fn read_ordered_list() {
        let doc = read("1. First\n2. Second").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let item1 = doc.node(body.children[0]).unwrap();
        match item1.attributes.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(info)) => {
                assert_eq!(info.num_format, ListFormat::Decimal);
            }
            other => panic!("Expected ListInfo, got {:?}", other),
        }
    }

    #[test]
    fn read_nested_list() {
        let doc = read("- Outer\n  - Inner").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let mut found_nested = false;
        for &child_id in &body.children {
            if let Some(node) = doc.node(child_id) {
                if let Some(AttributeValue::ListInfo(info)) = node.attributes.get(&AttributeKey::ListInfo) {
                    if info.level >= 2 {
                        found_nested = true;
                    }
                }
            }
        }
        assert!(found_nested, "Expected nested list item at level >= 2");
    }

    #[test]
    fn read_gfm_table() {
        let doc = read("| A | B |\n|---|---|\n| 1 | 2 |").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let table = doc.node(body.children[0]).unwrap();
        assert_eq!(table.node_type, NodeType::Table);
        assert!(table.children.len() >= 2);
    }

    #[test]
    fn read_line_break() {
        let doc = read("Line 1  \nLine 2").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let has_break = para.children.iter().any(|&id| {
            doc.node(id).map(|n| n.node_type == NodeType::LineBreak).unwrap_or(false)
        });
        assert!(has_break, "Expected a LineBreak node");
    }

    #[test]
    fn read_thematic_break() {
        let doc = read("Before\n\n---\n\nAfter").unwrap();
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();

        let has_rule = body.children.iter().any(|&id| {
            doc.node(id)
                .map(|n| n.attributes.get_bool(&AttributeKey::PageBreakBefore) == Some(true))
                .unwrap_or(false)
        });
        assert!(has_rule, "Expected a thematic break paragraph");
    }

    #[test]
    fn read_mixed_formatting() {
        let doc = read("Normal **bold** and *italic*").unwrap();
        let text = doc.to_plain_text();
        assert!(text.contains("Normal"));
        assert!(text.contains("bold"));
        assert!(text.contains("italic"));
    }

    #[test]
    fn read_unicode() {
        let doc = read("こんにちは **世界**").unwrap();
        let text = doc.to_plain_text();
        assert!(text.contains("こんにちは"));
        assert!(text.contains("世界"));
    }
}
