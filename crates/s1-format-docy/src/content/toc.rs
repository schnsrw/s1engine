use crate::constants::par;
use crate::writer::DocyWriter;
use crate::content::paragraph;
use s1_model::{DocumentModel, NodeType, NodeId};

pub fn write(w: &mut DocyWriter, model: &DocumentModel, toc_id: NodeId) {
    let toc = match model.node(toc_id) {
        Some(n) => n,
        None => return,
    };

    // TOC content is cached paragraphs
    for child_id in &toc.children {
        let child = match model.node(*child_id) {
            Some(n) => n,
            None => continue,
        };
        if child.node_type == NodeType::Paragraph {
            w.write_item(par::PAR, |w| {
                paragraph::write(w, model, *child_id);
            });
        }
    }
}
