use crate::constants::comments as ct;
use crate::writer::DocyWriter;
use s1_model::{DocumentModel, NodeType, AttributeKey};

pub fn has_content(model: &DocumentModel) -> bool {
    let root = match model.root_node() {
        Some(n) => n,
        None => return false,
    };
    root.children.iter().any(|id| {
        model.node(*id).map_or(false, |n| n.node_type == NodeType::CommentBody)
    })
}

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();

    let root = match model.root_node() {
        Some(n) => n,
        None => { w.end_length_block(len_pos); return; }
    };

    let mut comment_id: u32 = 0;
    for child_id in &root.children {
        let child = match model.node(*child_id) {
            Some(n) if n.node_type == NodeType::CommentBody => n,
            _ => continue,
        };

        w.write_item(ct::COMMENT, |w| {
            w.write_prop_long(ct::ID, comment_id);

            if let Some(author) = child.attributes.get_string(&AttributeKey::CommentAuthor) {
                w.write_prop_string2(ct::USER_NAME, author);
            }
            if let Some(date) = child.attributes.get_string(&AttributeKey::CommentDate) {
                w.write_prop_string2(ct::DATE, date);
            }

            // Extract text from comment body paragraphs
            let mut text = String::new();
            for para_id in &child.children {
                if let Some(para) = model.node(*para_id) {
                    if para.node_type == NodeType::Paragraph {
                        for run_id in &para.children {
                            if let Some(run) = model.node(*run_id) {
                                for text_id in &run.children {
                                    if let Some(t) = model.node(*text_id) {
                                        if let Some(ref tc) = t.text_content {
                                            text.push_str(tc);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !text.is_empty() {
                w.write_prop_string2(ct::TEXT, &text);
            }
        });

        comment_id += 1;
    }

    w.end_length_block(len_pos);
}
