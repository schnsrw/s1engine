use crate::constants::notes;
use crate::writer::DocyWriter;
use crate::content;
use s1_model::{DocumentModel, NodeType, NodeId};

pub fn has_content(model: &DocumentModel) -> bool {
    let root = match model.root_node() {
        Some(n) => n,
        None => return false,
    };
    root.children.iter().any(|id| {
        model.node(*id).map_or(false, |n| n.node_type == NodeType::EndnoteBody)
    })
}

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();
    let root = match model.root_node() {
        Some(n) => n,
        None => { w.end_length_block(len_pos); return; }
    };

    let mut note_id: u32 = 0;
    for child_id in &root.children {
        let child = match model.node(*child_id) {
            Some(n) if n.node_type == NodeType::EndnoteBody => n,
            _ => continue,
        };

        w.write_item(notes::NOTE, |w| {
            w.write_prop_byte(notes::NOTE_TYPE, 2); // 2 = endnote
            w.write_prop_long(notes::NOTE_ID, note_id);
            w.write_item(notes::NOTE_CONTENT, |w| {
                for para_id in &child.children {
                    if let Some(para) = model.node(*para_id) {
                        if para.node_type == NodeType::Paragraph {
                            w.write_item(crate::constants::par::PAR, |w| {
                                content::paragraph::write(w, model, *para_id);
                            });
                        }
                    }
                }
            });
        });
        note_id += 1;
    }

    w.end_length_block(len_pos);
}
