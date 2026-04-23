use crate::writer::DocyWriter;
use s1_model::{AttributeKey, AttributeValue, DocumentModel, NodeId};

/// Write an inline image as pptxDrawing in a Run.
/// Format verified against actual sdkjs BinaryFileWriter output.
pub fn write_inline_image(w: &mut DocyWriter, model: &DocumentModel, img_id: NodeId) {
    let img = match model.node(img_id) {
        Some(n) => n,
        None => return,
    };

    let width_pts = img.attributes.get_f64(&AttributeKey::ImageWidth).unwrap_or(100.0);
    let height_pts = img.attributes.get_f64(&AttributeKey::ImageHeight).unwrap_or(100.0);
    let width_emu = (width_pts * 12700.0).round() as u32;
    let height_emu = (height_pts * 12700.0).round() as u32;
    // For spPr xfrm: mm * 36000. pts to mm = pts * 25.4 / 72
    let cx_sppr = (width_pts * 25.4 / 72.0 * 36000.0).round() as u32;
    let cy_sppr = (height_pts * 25.4 / 72.0 * 36000.0).round() as u32;

    let img_src = if let Some(AttributeValue::MediaId(mid)) = img.attributes.get(&AttributeKey::ImageMediaId) {
        if let Some(item) = model.media().get(*mid) {
            let b64 = base64::engine::Engine::encode(&base64::engine::general_purpose::STANDARD, &item.data);
            format!("data:{};base64,{}", item.content_type, b64)
        } else { return; }
    } else { return; };

    // Run > Content(Read1) > pptxDrawing(Read1 type=12) > Read2 props
    w.write_item(crate::constants::par::RUN, |w| {
        w.write_item(crate::constants::run::CONTENT, |w| {
            // pptxDrawing is a Read1 item (type=12) inside Run Content
            w.write_item(12, |w| {
                // Inside: Read2 properties (c_oSerImageType2)
                // Type(0) = Inline(0)
                w.write_prop_byte(0, 0);
                // Extent(14) = Variable containing Read2 EMU values
                w.write_prop_item(14, |w| {
                    w.write_prop_long(2, width_emu);  // CxEmu
                    w.write_prop_long(3, height_emu);  // CyEmu
                });
                // PptxData(1) = Variable containing PPTX binary records
                w.write_prop_item(1, |w| {
                    write_pptx_picture(w, cx_sppr, cy_sppr, &img_src);
                });
            });
        });
    });
}

/// Write PPTX binary record structure for a picture.
/// Matches format from sdkjs BinaryFileWriter capture.
fn write_pptx_picture(w: &mut DocyWriter, cx: u32, cy: u32, img_src: &str) {
    // record(0) > record(1) > record(2=Picture)
    pptx_rec(w, 0, |w| {
        pptx_rec(w, 1, |w| {
            pptx_rec(w, 2, |w| {
                // Record 0: nvPicPr
                pptx_rec(w, 0, |w| {
                    // Sub-record 0: cNvPr (id + name)
                    pptx_rec(w, 0, |w| {
                        w.write_byte(0xFA); // nodeAttrStart
                        // _WriteInt2(0, id)
                        w.write_byte(0); // attr 0 = id
                        w.write_long(1); // id=1
                        w.write_byte(0xFB); // nodeAttrEnd
                    });
                    // Sub-record 1: cNvPicPr
                    pptx_rec(w, 1, |w| {
                        w.write_byte(0xFA);
                        w.write_byte(0xFB);
                    });
                });

                // Record 1: blipFill (UniFill)
                pptx_rec(w, 1, |w| {
                    // FILL_TYPE_BLIP = 1
                    pptx_rec(w, 1, |w| {
                        // Attributes: rotWithShape
                        w.write_byte(0xFA);
                        w.write_byte(0xFB);
                        // WriteBlip > record(0)
                        pptx_rec(w, 0, |w| {
                            w.write_byte(0xFA);
                            w.write_byte(0xFB);
                            // Effects record(2): 0 effects
                            pptx_rec(w, 2, |w| {
                                w.write_long(0);
                            });
                            // Image path record(3): _WriteString1(0, src)
                            pptx_rec(w, 3, |w| {
                                w.write_byte(0xFA);
                                pptx_write_string(w, 0, img_src);
                                w.write_byte(0xFB);
                            });
                        });
                    });
                });

                // Record 2: spPr (xfrm + geometry)
                pptx_rec(w, 2, |w| {
                    w.write_byte(0xFA);
                    w.write_byte(0xFB);
                    // Sub-record 0: xfrm
                    w.write_byte(0);
                    let xfrm_len_pos = w.position();
                    w.write_long(0);
                    w.write_byte(0xFA);
                    w.write_byte(2); w.write_long(cx);
                    w.write_byte(3); w.write_long(cy);
                    w.write_byte(0xFB);
                    let xfrm_len = (w.position() - xfrm_len_pos - 4) as u32;
                    let xb = xfrm_len.to_le_bytes();
                    let buf = w.as_bytes_mut();
                    buf[xfrm_len_pos] = xb[0]; buf[xfrm_len_pos+1] = xb[1];
                    buf[xfrm_len_pos+2] = xb[2]; buf[xfrm_len_pos+3] = xb[3];
                    // Sub-record 1: geometry (preset "rect")
                    w.write_byte(1);
                    let geom_len_pos = w.position();
                    w.write_long(0);
                    w.write_byte(0xFA);
                    // _WriteString1(0, "rect")
                    pptx_write_string(w, 0, "rect");
                    w.write_byte(0xFB);
                    let geom_len = (w.position() - geom_len_pos - 4) as u32;
                    let gb = geom_len.to_le_bytes();
                    let buf2 = w.as_bytes_mut();
                    buf2[geom_len_pos] = gb[0]; buf2[geom_len_pos+1] = gb[1];
                    buf2[geom_len_pos+2] = gb[2]; buf2[geom_len_pos+3] = gb[3];
                });
            });
        });
    });
}

fn pptx_rec<F: FnOnce(&mut DocyWriter)>(w: &mut DocyWriter, record_type: u8, f: F) {
    w.write_byte(record_type);
    let len_pos = w.position();
    w.write_long(0);
    f(w);
    let content_len = (w.position() - len_pos - 4) as u32;
    let bytes = content_len.to_le_bytes();
    let buf = w.as_bytes_mut();
    buf[len_pos] = bytes[0];
    buf[len_pos + 1] = bytes[1];
    buf[len_pos + 2] = bytes[2];
    buf[len_pos + 3] = bytes[3];
}

fn pptx_write_string(w: &mut DocyWriter, id: u8, s: &str) {
    w.write_byte(id);
    let utf16: Vec<u16> = s.encode_utf16().collect();
    w.write_long(utf16.len() as u32);
    for ch in &utf16 {
        w.write_raw(&ch.to_le_bytes());
    }
}
