use crate::constants::notes;
use crate::writer::DocyWriter;
use crate::content;
use s1_model::{DocumentModel, NodeType, NodeId};

pub fn has_content(model: &DocumentModel) -> bool {
    find_note_bodies(model, NodeType::FootnoteBody).next().is_some()
}

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();
    let mut note_id: u32 = 0;

    for body_id in find_note_bodies(model, NodeType::FootnoteBody).collect::<Vec<_>>() {
        w.write_item(notes::NOTE, |w| {
            w.write_prop_byte(notes::NOTE_TYPE, 1); // 1 = footnote
            w.write_prop_long(notes::NOTE_ID, note_id);
            w.write_item(notes::NOTE_CONTENT, |w| {
                write_note_content(w, model, body_id);
            });
        });
        note_id += 1;
    }

    w.end_length_block(len_pos);
}

fn find_note_bodies(model: &DocumentModel, note_type: NodeType) -> impl Iterator<Item = NodeId> + '_ {
    let root = model.root_node();
    root.into_iter().flat_map(move |r| {
        r.children.iter().filter_map(move |id| {
            model.node(*id).and_then(|n| {
                if n.node_type == note_type { Some(*id) } else { None }
            })
        })
    })
}

fn write_note_content(w: &mut DocyWriter, model: &DocumentModel, body_id: NodeId) {
    let body = match model.node(body_id) {
        Some(n) => n,
        None => return,
    };
    for child_id in &body.children {
        if let Some(child) = model.node(*child_id) {
            if child.node_type == NodeType::Paragraph {
                w.write_item(crate::constants::par::PAR, |w| {
                    content::paragraph::write(w, model, *child_id);
                });
            }
        }
    }
}
