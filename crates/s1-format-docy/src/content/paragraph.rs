use crate::constants::*;
use crate::writer::DocyWriter;
use crate::props;
use s1_model::{AttributeKey, DocumentModel, NodeType, NodeId};
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
                    // Runs in paragraph Content use c_oSerParType.Run (5),
                    // NOT c_oSerRunType.run (0).
                    w.write_item(par::RUN, |w| {
                        write_run_content(w, model, *child_id);
                    });
                }
                NodeType::LineBreak => {
                    w.write_item(par::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_item(run::LINEBREAK, |_| {});
                        });
                    });
                }
                NodeType::PageBreak => {
                    w.write_item(par::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_item(run::PAGEBREAK, |_| {});
                        });
                    });
                }
                NodeType::ColumnBreak => {
                    w.write_item(par::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_item(run::COLUMN_BREAK, |_| {});
                        });
                    });
                }
                NodeType::Tab => {
                    w.write_item(par::RUN, |w| {
                        w.write_item(run::CONTENT, |w| {
                            w.write_item(run::TAB, |_| {});
                        });
                    });
                }
                NodeType::Image => {
                    // Image drawing serialization disabled — format doesn't match
                    // sdkjs pptxDrawing structure yet. Skip silently.
                }
                NodeType::BookmarkStart => {}
                NodeType::BookmarkEnd => {}
                NodeType::CommentStart => {}
                NodeType::CommentEnd => {}
                NodeType::FootnoteRef => {
                    // Note references need a stable DOCY note-id mapping. Skip until
                    // the serializer and note tables share that mapping.
                }
                NodeType::EndnoteRef => {
                    // Note references need a stable DOCY note-id mapping. Skip until
                    // the serializer and note tables share that mapping.
                }
                _ => {
                    // Skip unsupported inline elements
                }
            }
        }
    });
}

/// Write run internals: rPr + text content.
/// Called inside a c_oSerParType.Run (5) item.
fn write_run_content(w: &mut DocyWriter, model: &DocumentModel, run_id: NodeId) {
    let run_node = match model.node(run_id) {
        Some(n) => n,
        None => return,
    };

    // Run properties (c_oSerRunType.rPr = 1)
    w.write_item(run::RPR, |w| {
        props::run_props::write(w, &run_node.attributes);
    });

    // Text content (c_oSerRunType.Content = 8)
    w.write_item(run::CONTENT, |w| {
        for child_id in &run_node.children {
            let child = match model.node(*child_id) {
                Some(n) => n,
                None => continue,
            };
            if child.node_type == NodeType::Text {
                if let Some(ref text) = child.text_content {
                    // c_oSerRunType.run (0) = text content marker + WriteString2
                    w.write_byte(run::RUN); // 0
                    w.write_string(text);   // UTF-16LE with length prefix
                }
            }
        }
    });
}

/// Write an inline image
#[allow(dead_code)]
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
                        w.write_string_item(0, &b64); // media data
                        w.write_string_item(1, &item.content_type); // content type
                    }
                }
            });
        });
    });
}
