use crate::constants::*;
use crate::writer::DocyWriter;
use crate::props;
use s1_model::{AttributeKey, AttributeValue, DocumentModel, NodeType, NodeId};
use base64::engine::Engine as _;

use crate::tables::headers_footers::HdrFtrEntry;
use s1_model::SectionProperties;

/// Write a complete paragraph: pPr + content (runs, breaks, images, etc.)
/// If `sect_pr` is Some, the section properties are written inside pPr
/// (this is how sdkjs handles mid-document section breaks).
pub fn write(w: &mut DocyWriter, model: &DocumentModel, para_id: NodeId) {
    write_with_section(w, model, para_id, None, &[], &[]);
}

pub fn write_with_section(
    w: &mut DocyWriter,
    model: &DocumentModel,
    para_id: NodeId,
    sect_pr: Option<&SectionProperties>,
    all_headers: &[HdrFtrEntry],
    all_footers: &[HdrFtrEntry],
) {
    let para = match model.node(para_id) {
        Some(n) => n,
        None => return,
    };

    // Paragraph properties
    w.write_item(par::PPR, |w| {
        props::para_props::write(w, &para.attributes);
        // Mid-document section properties inside pPr (sdkjs c_oSerProp_pPrType.SectPr=31)
        if let Some(sec) = sect_pr {
            w.write_prop_item(ppr::PPR_SECT_PR, |w| {
                crate::content::section::write_with_hdr_ftr(w, sec, all_headers, all_footers);
            });
        }
    });

    // Content — group consecutive runs with the same HyperlinkUrl
    w.write_item(par::CONTENT, |w| {
        let children: Vec<NodeId> = para.children.clone();
        let mut i = 0;
        while i < children.len() {
            let child_id = children[i];
            let child = match model.node(child_id) {
                Some(n) => n,
                None => { i += 1; continue; }
            };

            match child.node_type {
                NodeType::Run => {
                    // Check for hyperlink
                    if let Some(url) = child.attributes.get_string(&AttributeKey::HyperlinkUrl) {
                        let url = url.to_string();
                        let start = i;
                        // Collect consecutive runs with same URL
                        while i < children.len() {
                            if let Some(n) = model.node(children[i]) {
                                if n.node_type == NodeType::Run
                                    && n.attributes.get_string(&AttributeKey::HyperlinkUrl)
                                        == Some(&url)
                                {
                                    i += 1;
                                    continue;
                                }
                            }
                            break;
                        }
                        // Write hyperlink wrapper
                        w.write_item(par::HYPERLINK, |w| {
                            if url.starts_with('#') {
                                w.write_string_item(hyperlink::ANCHOR, &url[1..]);
                            } else {
                                w.write_string_item(hyperlink::LINK, &url);
                            }
                            w.write_item(hyperlink::CONTENT, |w| {
                                for &run_id in &children[start..i] {
                                    w.write_item(par::RUN, |w| {
                                        write_run_content(w, model, run_id);
                                    });
                                }
                            });
                        });
                        continue; // i already advanced
                    }
                    // Normal run (not a hyperlink)
                    w.write_item(par::RUN, |w| {
                        write_run_content(w, model, child_id);
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
                    // Image serialization via pptxDrawing is complex and fragile.
                    // Images are injected via adapter.js post-load instead.
                }
                NodeType::Drawing => {
                    // Drawing/shape objects not yet supported
                }
                NodeType::Field => {
                    // Write field as page number if applicable
                    if let Some(AttributeValue::FieldType(ft)) = child.attributes.get(&AttributeKey::FieldType) {
                        match ft {
                            s1_model::FieldType::PageNumber => {
                                w.write_item(par::RUN, |w| {
                                    w.write_item(run::CONTENT, |w| {
                                        w.write_item(run::PAGENUM, |_| {});
                                    });
                                });
                            }
                            _ => {} // Other field types not yet supported
                        }
                    }
                }
                NodeType::BookmarkStart => {
                    // Bookmark: Read1 format. ID=WriteItem(0, Long), Name=WriteByte(1)+WriteString2
                    if let Some(name) = child.attributes.get_string(&AttributeKey::BookmarkName) {
                        w.write_item(par::BOOKMARK_START, |w| {
                            w.write_item(bookmark::ID, |w| {
                                w.write_long(child_id.counter as u32);
                            });
                            w.write_string_item(bookmark::NAME, name);
                        });
                    }
                }
                NodeType::BookmarkEnd => {
                    w.write_item(par::BOOKMARK_END, |w| {
                        w.write_item(bookmark::ID, |w| {
                            w.write_long(child_id.counter as u32);
                        });
                    });
                }
                NodeType::CommentStart => {
                    // Comment anchor: Read1 format. c_oSer_CommentsType.Id = 1
                    if let Some(AttributeValue::Int(cid)) = child.attributes.get(&AttributeKey::CommentId) {
                        w.write_item(par::COMMENT_START, |w| {
                            w.write_item(comments::ID, |w| {
                                w.write_long(*cid as u32);
                            });
                        });
                    }
                }
                NodeType::CommentEnd => {
                    if let Some(AttributeValue::Int(cid)) = child.attributes.get(&AttributeKey::CommentId) {
                        w.write_item(par::COMMENT_END, |w| {
                            w.write_item(comments::ID, |w| {
                                w.write_long(*cid as u32);
                            });
                        });
                    }
                }
                NodeType::FootnoteRef => {
                    // Note ref: Read1 format. c_oSerNotes.RefId = 5
                    if let Some(AttributeValue::Int(note_id)) = child.attributes.get(&AttributeKey::FootnoteNumber) {
                        w.write_item(par::RUN, |w| {
                            w.write_item(run::CONTENT, |w| {
                                w.write_item(run::FOOTNOTE_REFERENCE, |w| {
                                    w.write_item(notes::REF_ID, |w| {
                                        w.write_long(*note_id as u32);
                                    });
                                });
                            });
                        });
                    }
                }
                NodeType::EndnoteRef => {
                    if let Some(AttributeValue::Int(note_id)) = child.attributes.get(&AttributeKey::EndnoteNumber) {
                        w.write_item(par::RUN, |w| {
                            w.write_item(run::CONTENT, |w| {
                                w.write_item(run::ENDNOTE_REFERENCE, |w| {
                                    w.write_item(notes::REF_ID, |w| {
                                        w.write_long(*note_id as u32);
                                    });
                                });
                            });
                        });
                    }
                }
                _ => {
                    // Skip unsupported inline elements
                }
            }
            i += 1;
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
