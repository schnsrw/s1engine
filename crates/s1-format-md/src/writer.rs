//! Markdown writer.
//!
//! Converts a [`DocumentModel`] into a Markdown string.

use s1_model::{AttributeKey, AttributeValue, DocumentModel, ListFormat, NodeId, NodeType};

/// Write a document model to a Markdown string.
pub fn write(doc: &DocumentModel) -> String {
    let body_id = match doc.body_id() {
        Some(id) => id,
        None => return String::new(),
    };

    let mut out = String::new();
    let body = match doc.node(body_id) {
        Some(n) => n,
        None => return String::new(),
    };

    let children: Vec<NodeId> = body.children.clone();
    for (i, &child_id) in children.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        write_block(doc, child_id, &mut out);
    }

    out
}

/// Extract heading level from StyleId (e.g. "Heading1" -> 1).
fn heading_level(style_id: &str) -> Option<u8> {
    style_id.strip_prefix("Heading")?.parse::<u8>().ok()
}

/// Write a block-level node.
fn write_block(doc: &DocumentModel, node_id: NodeId, out: &mut String) {
    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return,
    };

    match node.node_type {
        NodeType::Paragraph => {
            // Check for heading via StyleId
            if let Some(style_id) = node.attributes.get_string(&AttributeKey::StyleId) {
                if let Some(level) = heading_level(style_id) {
                    for _ in 0..level {
                        out.push('#');
                    }
                    out.push(' ');
                }
            }

            // Check for list item
            if let Some(AttributeValue::ListInfo(info)) =
                node.attributes.get(&AttributeKey::ListInfo)
            {
                let indent = if info.level > 1 {
                    "  ".repeat((info.level - 1) as usize)
                } else {
                    String::new()
                };
                out.push_str(&indent);
                match info.num_format {
                    ListFormat::Decimal => out.push_str("1. "),
                    _ => out.push_str("- "),
                }
            }

            // Check for thematic break (PageBreakBefore on empty paragraph)
            if node.attributes.get_bool(&AttributeKey::PageBreakBefore) == Some(true)
                && node.children.is_empty()
            {
                out.push_str("---\n");
                return;
            }

            // Write inline content
            let children: Vec<NodeId> = node.children.clone();
            for &child_id in &children {
                write_inline(doc, child_id, out);
            }
            out.push('\n');
        }

        NodeType::Table => {
            write_table(doc, node_id, out);
        }

        NodeType::TableOfContents => {
            let children: Vec<NodeId> = node.children.clone();
            for &child_id in &children {
                if let Some(child) = doc.node(child_id) {
                    if child.node_type == NodeType::Paragraph {
                        let mut text = String::new();
                        let para_children: Vec<NodeId> = child.children.clone();
                        for &inline_id in &para_children {
                            write_inline_text(doc, inline_id, &mut text);
                        }
                        out.push_str(&text);
                        out.push('\n');
                    }
                }
            }
        }

        NodeType::Section | NodeType::Body | NodeType::Document => {
            let children: Vec<NodeId> = node.children.clone();
            for &child_id in &children {
                write_block(doc, child_id, out);
            }
        }

        _ => {
            let mut text = String::new();
            write_inline_text(doc, node_id, &mut text);
            if !text.is_empty() {
                out.push_str(&text);
                out.push('\n');
            }
        }
    }
}

/// Write inline content with Markdown formatting markers.
fn write_inline(doc: &DocumentModel, node_id: NodeId, out: &mut String) {
    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return,
    };

    match node.node_type {
        NodeType::Text => {
            if let Some(text) = &node.text_content {
                out.push_str(text);
            }
        }
        NodeType::LineBreak => {
            out.push_str("  \n");
        }
        NodeType::Run => {
            let bold = node.attributes.get_bool(&AttributeKey::Bold) == Some(true);
            let italic = node.attributes.get_bool(&AttributeKey::Italic) == Some(true);
            let strike = node.attributes.get_bool(&AttributeKey::Strikethrough) == Some(true);
            let code = node
                .attributes
                .get_string(&AttributeKey::FontFamily)
                .map(|f| f == "monospace")
                .unwrap_or(false);
            let url = node.attributes.get_string(&AttributeKey::HyperlinkUrl);

            // Collect inner text
            let mut inner = String::new();
            let children: Vec<NodeId> = node.children.clone();
            for &child_id in &children {
                write_inline_text(doc, child_id, &mut inner);
            }

            if inner.is_empty() {
                return;
            }

            // Hyperlink wrapping
            if let Some(href) = url {
                if code {
                    out.push('[');
                    out.push('`');
                    out.push_str(&inner);
                    out.push('`');
                    out.push_str("](");
                    out.push_str(href);
                    out.push(')');
                } else {
                    out.push('[');
                    // Apply formatting inside the link text
                    push_formatted(out, &inner, bold, italic, strike);
                    out.push_str("](");
                    out.push_str(href);
                    out.push(')');
                }
            } else if code {
                out.push('`');
                out.push_str(&inner);
                out.push('`');
            } else {
                push_formatted(out, &inner, bold, italic, strike);
            }
        }
        _ => {
            let children: Vec<NodeId> = node.children.clone();
            for &child_id in &children {
                write_inline(doc, child_id, out);
            }
        }
    }
}

/// Push text with bold/italic/strikethrough markers.
fn push_formatted(out: &mut String, text: &str, bold: bool, italic: bool, strike: bool) {
    if bold && italic {
        out.push_str("***");
    } else if bold {
        out.push_str("**");
    } else if italic {
        out.push('*');
    }
    if strike {
        out.push_str("~~");
    }

    out.push_str(text);

    if strike {
        out.push_str("~~");
    }
    if bold && italic {
        out.push_str("***");
    } else if bold {
        out.push_str("**");
    } else if italic {
        out.push('*');
    }
}

/// Extract plain text from inline nodes (no formatting markers).
fn write_inline_text(doc: &DocumentModel, node_id: NodeId, out: &mut String) {
    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return,
    };

    match node.node_type {
        NodeType::Text => {
            if let Some(text) = &node.text_content {
                out.push_str(text);
            }
        }
        _ => {
            let children: Vec<NodeId> = node.children.clone();
            for &child_id in &children {
                write_inline_text(doc, child_id, out);
            }
        }
    }
}

/// Write a table in GFM format.
fn write_table(doc: &DocumentModel, table_id: NodeId, out: &mut String) {
    let table = match doc.node(table_id) {
        Some(n) => n,
        None => return,
    };

    let rows: Vec<NodeId> = table.children.clone();
    for (row_idx, &row_id) in rows.iter().enumerate() {
        let row = match doc.node(row_id) {
            Some(n) => n,
            None => continue,
        };

        let cells: Vec<NodeId> = row.children.clone();
        out.push('|');
        for &cell_id in &cells {
            out.push(' ');
            let mut cell_text = String::new();
            write_cell_text(doc, cell_id, &mut cell_text);
            out.push_str(cell_text.trim());
            out.push_str(" |");
        }
        out.push('\n');

        // After header row, add separator
        if row_idx == 0 {
            out.push('|');
            for _ in &cells {
                out.push_str("---|");
            }
            out.push('\n');
        }
    }
}

fn write_cell_text(doc: &DocumentModel, node_id: NodeId, out: &mut String) {
    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return,
    };

    if let Some(text) = &node.text_content {
        out.push_str(text);
    }

    let children: Vec<NodeId> = node.children.clone();
    for &child_id in &children {
        write_cell_text(doc, child_id, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{ListInfo, Node};

    fn make_para_doc(lines: &[&str]) -> DocumentModel {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        for (i, line) in lines.iter().enumerate() {
            let para_id = doc.next_id();
            doc.insert_node(body_id, i, Node::new(para_id, NodeType::Paragraph))
                .unwrap();

            if !line.is_empty() {
                let run_id = doc.next_id();
                doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                    .unwrap();
                let text_id = doc.next_id();
                doc.insert_node(run_id, 0, Node::text(text_id, *line))
                    .unwrap();
            }
        }
        doc
    }

    #[test]
    fn write_empty() {
        let doc = DocumentModel::new();
        assert_eq!(write(&doc), "");
    }

    #[test]
    fn write_paragraph() {
        let doc = make_para_doc(&["Hello world"]);
        let md = write(&doc);
        assert!(md.contains("Hello world"));
    }

    #[test]
    fn write_heading_levels() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        for (i, level) in [1u8, 2, 3].iter().enumerate() {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::StyleId,
                AttributeValue::String(format!("Heading{}", level)),
            );
            doc.insert_node(body_id, i, para).unwrap();

            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, &format!("H{}", level)))
                .unwrap();
        }

        let md = write(&doc);
        assert!(md.contains("# H1"));
        assert!(md.contains("## H2"));
        assert!(md.contains("### H3"));
    }

    #[test]
    fn write_bold() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Bold, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run).unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "bold"))
            .unwrap();

        assert!(write(&doc).contains("**bold**"));
    }

    #[test]
    fn write_italic() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Italic, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run).unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "italic"))
            .unwrap();

        assert!(write(&doc).contains("*italic*"));
    }

    #[test]
    fn write_bold_italic() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Bold, AttributeValue::Bool(true));
        run.attributes
            .set(AttributeKey::Italic, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run).unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "both"))
            .unwrap();

        assert!(write(&doc).contains("***both***"));
    }

    #[test]
    fn write_strikethrough() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Strikethrough, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run).unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "struck"))
            .unwrap();

        assert!(write(&doc).contains("~~struck~~"));
    }

    #[test]
    fn write_hyperlink() {
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
        doc.insert_node(run_id, 0, Node::text(text_id, "Link"))
            .unwrap();

        assert!(write(&doc).contains("[Link](https://example.com)"));
    }

    #[test]
    fn write_unordered_list() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        for i in 0..2 {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::ListInfo,
                AttributeValue::ListInfo(ListInfo {
                    level: 1,
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
            doc.insert_node(run_id, 0, Node::text(text_id, &format!("Item {}", i + 1)))
                .unwrap();
        }

        let md = write(&doc);
        assert!(md.contains("- Item 1"));
        assert!(md.contains("- Item 2"));
    }

    #[test]
    fn write_ordered_list() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        for i in 0..2 {
            let para_id = doc.next_id();
            let mut para = Node::new(para_id, NodeType::Paragraph);
            para.attributes.set(
                AttributeKey::ListInfo,
                AttributeValue::ListInfo(ListInfo {
                    level: 1,
                    num_format: ListFormat::Decimal,
                    num_id: 1,
                    start: None,
                }),
            );
            doc.insert_node(body_id, i, para).unwrap();

            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .unwrap();
            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, &format!("Item {}", i + 1)))
                .unwrap();
        }

        let md = write(&doc);
        assert!(md.contains("1. Item 1"));
        assert!(md.contains("1. Item 2"));
    }

    #[test]
    fn write_table() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        for (row_idx, row_data) in [["A", "B"], ["1", "2"]].iter().enumerate() {
            let row_id = doc.next_id();
            doc.insert_node(table_id, row_idx, Node::new(row_id, NodeType::TableRow))
                .unwrap();
            for (j, text) in row_data.iter().enumerate() {
                let cell_id = doc.next_id();
                doc.insert_node(row_id, j, Node::new(cell_id, NodeType::TableCell))
                    .unwrap();
                let p_id = doc.next_id();
                doc.insert_node(cell_id, 0, Node::new(p_id, NodeType::Paragraph))
                    .unwrap();
                let r_id = doc.next_id();
                doc.insert_node(p_id, 0, Node::new(r_id, NodeType::Run))
                    .unwrap();
                let t_id = doc.next_id();
                doc.insert_node(r_id, 0, Node::text(t_id, *text)).unwrap();
            }
        }

        let md = write(&doc);
        assert!(md.contains("| A | B |"));
        assert!(md.contains("|---|---|"));
        assert!(md.contains("| 1 | 2 |"));
    }

    #[test]
    fn write_line_break() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run1_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run1_id, NodeType::Run))
            .unwrap();
        let t1_id = doc.next_id();
        doc.insert_node(run1_id, 0, Node::text(t1_id, "Line 1"))
            .unwrap();

        let br_id = doc.next_id();
        doc.insert_node(para_id, 1, Node::new(br_id, NodeType::LineBreak))
            .unwrap();

        let run2_id = doc.next_id();
        doc.insert_node(para_id, 2, Node::new(run2_id, NodeType::Run))
            .unwrap();
        let t2_id = doc.next_id();
        doc.insert_node(run2_id, 0, Node::text(t2_id, "Line 2"))
            .unwrap();

        assert!(write(&doc).contains("Line 1  \nLine 2"));
    }

    #[test]
    fn write_unicode() {
        let doc = make_para_doc(&["こんにちは", "café"]);
        let md = write(&doc);
        assert!(md.contains("こんにちは"));
        assert!(md.contains("café"));
    }
}
