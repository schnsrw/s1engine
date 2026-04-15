use crate::constants::*;
use crate::writer::DocyWriter;
use crate::props;
use s1_model::{DocumentModel, NodeType, NodeId, AttributeKey, AttributeValue};
use base64::engine::Engine as _;

/// Write a complete paragraph: pPr + content (runs, breaks, images, etc.)
pub fn write(w: &mut DocyWriter, model: &DocumentModel, para_id: NodeId) {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return,
    };

    // Paragraph properties
    w.write_item(par::PPR, |w| {
        props::para_props::write(w, &para.attributes);
    });

    // Content
    w.write_item(par::CONTENT, |w| {
        for child_id in &para.children {
            let child = match model.node(*child_id) {
                Some(n) => n,
                None => continue,
            };

            match child.node_type {
                NodeType::Run => {
                    write_run(w, model, *child_id);
                }
                NodeType::LineBreak => {
                    // Line break as a run containing a linebreak element
                    w.write_item(run::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_byte(run::LINEBREAK);
                        });
                    });
                }
                NodeType::PageBreak => {
                    w.write_item(run::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_byte(run::PAGEBREAK);
                        });
                    });
                }
                NodeType::ColumnBreak => {
                    w.write_item(run::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_byte(run::COLUMN_BREAK);
                        });
                    });
                }
                NodeType::Tab => {
                    w.write_item(run::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_byte(run::TAB);
                        });
                    });
                }
                NodeType::Image => {
                    write_image(w, model, *child_id);
                }
                NodeType::BookmarkStart => {
                    w.write_item(par::BOOKMARK_START, |w| {
                        if let Some(name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                            w.write_prop_string2(0, name); // BookmarkName
                        }
                    });
                }
                NodeType::BookmarkEnd => {
                    w.write_item(par::BOOKMARK_END, |w| {
                        if let Some(name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                            w.write_prop_string2(0, name);
                        }
                    });
                }
                NodeType::CommentStart => {
                    w.write_item(par::COMMENT_START, |w| {
                        if let Some(id) = child.attributes.get_string(&AttributeKey::CommentId) {
                            w.write_prop_string2(0, id);
                        }
                    });
                }
                NodeType::CommentEnd => {
                    w.write_item(par::COMMENT_END, |w| {
                        if let Some(id) = child.attributes.get_string(&AttributeKey::CommentId) {
                            w.write_prop_string2(0, id);
                        }
                    });
                }
                NodeType::FootnoteRef => {
                    w.write_item(run::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_byte(run::FOOTNOTE_REFERENCE);
                        });
                    });
                }
                NodeType::EndnoteRef => {
                    w.write_item(run::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_byte(run::ENDNOTE_REFERENCE);
                        });
                    });
                }
                _ => {
                    // Skip unsupported inline elements
                }
            }
        }
    });
}

/// Write a text run: rPr + text content
fn write_run(w: &mut DocyWriter, model: &DocumentModel, run_id: NodeId) {
    let run_node = match model.node(run_id) {
        Some(n) => n,
        None => return,
    };

    w.write_item(run::RUN, |w| {
        // Run properties
        w.write_item(run::RPR, |w| {
            props::run_props::write(w, &run_node.attributes);
        });

        // Text content
        w.write_item(run::CONTENT, |w| {
            for child_id in &run_node.children {
                let child = match model.node(*child_id) {
                    Some(n) => n,
                    None => continue,
                };
                if child.node_type == NodeType::Text {
                    if let Some(ref text) = child.text_content {
                        // Write each character as text content
                        // In DOCY, text is written as type + string
                        w.write_byte(run::RUN); // text content marker
                        w.write_string(text);
                    }
                }
            }
        });
    });
}

/// Write an inline image
fn write_image(w: &mut DocyWriter, model: &DocumentModel, img_id: NodeId) {
    let img = match model.node(img_id) {
        Some(n) => n,
        None => return,
    };

    let width = img.attributes.get_f64(&AttributeKey::ImageWidth).unwrap_or(100.0);
    let height = img.attributes.get_f64(&AttributeKey::ImageHeight).unwrap_or(100.0);

    // Image as a drawing object
    w.write_item(run::RUN, |w| {
        w.write_item(run::CONTENT, |w| {
            w.write_item(run::IMAGE, |w| {
                // Width and height in EMU
                w.write_prop_long(2, pts_to_emu(width) as u32); // width
                w.write_prop_long(3, pts_to_emu(height) as u32); // height

                // Media reference
                if let Some(s1_model::AttributeValue::MediaId(mid)) =
                    img.attributes.get(&AttributeKey::ImageMediaId)
                {
                    if let Some(item) = model.media().get(*mid) {
                        // Write image data inline (base64)
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&item.data);
                        w.write_prop_string2(0, &b64); // media data
                        w.write_prop_string2(1, &item.content_type); // content type
                    }
                }
            });
        });
    });
}
