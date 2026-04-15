use crate::constants::*;
use crate::writer::DocyWriter;
use crate::content;
use s1_model::{DocumentModel, NodeType};

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();

    let body_id = match model.body_id() {
        Some(id) => id,
        None => {
            w.end_length_block(len_pos);
            return;
        }
    };

    let body = match model.node(body_id) {
        Some(n) => n,
        None => {
            w.end_length_block(len_pos);
            return;
        }
    };

    // Write each body child
    let mut last_section_idx: Option<usize> = None;
    for child_id in &body.children {
        let child = match model.node(*child_id) {
            Some(n) => n,
            None => continue,
        };

        match child.node_type {
            NodeType::Paragraph => {
                w.write_item(par::PAR, |w| {
                    content::paragraph::write(w, model, *child_id);
                });

                // Check if this paragraph ends a section
                if let Some(s1_model::AttributeValue::Int(idx)) = child.attributes.get(&s1_model::AttributeKey::SectionIndex) {
                    let idx = *idx;
                    if Some(idx as usize) != last_section_idx {
                        // Write section properties after the paragraph
                        if let Some(sec) = model.sections().get(idx as usize) {
                            w.write_item(par::SECT_PR, |w| {
                                content::section::write(w, sec);
                            });
                        }
                        last_section_idx = Some(idx as usize);
                    }
                }
            }
            NodeType::Table => {
                w.write_item(par::TABLE, |w| {
                    content::table::write(w, model, *child_id);
                });
            }
            NodeType::TableOfContents => {
                w.write_item(par::SDT, |w| {
                    content::toc::write(w, model, *child_id);
                });
            }
            _ => {
                // Skip unsupported top-level elements
            }
        }
    }

    // Write final section properties if not already written
    if let Some(sections) = Some(model.sections()) {
        if let Some(last_sec) = sections.last() {
            let last_idx = sections.len() - 1;
            if last_section_idx != Some(last_idx) {
                w.write_item(par::SECT_PR, |w| {
                    content::section::write(w, last_sec);
                });
            }
        }
    }

    w.end_length_block(len_pos);
}
