use crate::constants::hdr_ftr;
use crate::writer::DocyWriter;
use crate::content;
use s1_model::{DocumentModel, NodeType, NodeId};

pub fn has_content(model: &DocumentModel) -> bool {
    let root = match model.root_node() {
        Some(n) => n,
        None => return false,
    };
    root.children.iter().any(|id| {
        model.node(*id).map_or(false, |n| {
            n.node_type == NodeType::Header || n.node_type == NodeType::Footer
        })
    })
}

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();

    // Collect headers and footers from document root children
    let root = match model.root_node() {
        Some(n) => n,
        None => { w.end_length_block(len_pos); return; }
    };

    let mut headers: Vec<NodeId> = Vec::new();
    let mut footers: Vec<NodeId> = Vec::new();

    for child_id in &root.children {
        if let Some(child) = model.node(*child_id) {
            match child.node_type {
                NodeType::Header => headers.push(*child_id),
                NodeType::Footer => footers.push(*child_id),
                _ => {}
            }
        }
    }

    // Write headers
    if !headers.is_empty() {
        w.write_item(hdr_ftr::HEADER, |w| {
            // Write as "odd" (default) header
            for hdr_id in &headers {
                w.write_item(hdr_ftr::ODD, |w| {
                    w.write_item(hdr_ftr::CONTENT, |w| {
                        write_hdr_ftr_content(w, model, *hdr_id);
                    });
                });
            }
        });
    }

    // Write footers
    if !footers.is_empty() {
        w.write_item(hdr_ftr::FOOTER, |w| {
            for ftr_id in &footers {
                w.write_item(hdr_ftr::ODD, |w| {
                    w.write_item(hdr_ftr::CONTENT, |w| {
                        write_hdr_ftr_content(w, model, *ftr_id);
                    });
                });
            }
        });
    }

    w.end_length_block(len_pos);
}

fn write_hdr_ftr_content(w: &mut DocyWriter, model: &DocumentModel, node_id: NodeId) {
    let node = match model.node(node_id) {
        Some(n) => n,
        None => return,
    };
    // Header/footer children are paragraphs
    for child_id in &node.children {
        if let Some(child) = model.node(*child_id) {
            if child.node_type == NodeType::Paragraph {
                w.write_item(crate::constants::par::PAR, |w| {
                    content::paragraph::write(w, model, *child_id);
                });
            }
        }
    }
}
