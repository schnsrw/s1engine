use crate::writer::DocyWriter;
use s1_model::{AttributeKey, AttributeValue, DocumentModel, NodeId};

/// Write an inline image as pptxDrawing in a Run.
/// Binary format captured from actual sdkjs output.
pub fn write_inline_image(w: &mut DocyWriter, model: &DocumentModel, img_id: NodeId) {
    let img = match model.node(img_id) {
        Some(n) => n,
        None => return,
    };

    let width_pts = img.attributes.get_f64(&AttributeKey::ImageWidth).unwrap_or(100.0);
    let height_pts = img.attributes.get_f64(&AttributeKey::ImageHeight).unwrap_or(100.0);
    // Convert points to EMU (1 pt = 12700 EMU)
    let width_emu = (width_pts * 12700.0).round() as u32;
    let height_emu = (height_pts * 12700.0).round() as u32;
    // Convert points to mm for spPr xfrm (1 pt = 0.352778 mm, stored as EMU * 36000/914400)
    let width_mm_emu = (width_pts * 36000.0).round() as u32;
    let height_mm_emu = (height_pts * 36000.0).round() as u32;

    // Get image data as data URL
    let img_src = if let Some(AttributeValue::MediaId(mid)) = img.attributes.get(&AttributeKey::ImageMediaId) {
        if let Some(item) = model.media().get(*mid) {
            let b64 = base64::engine::Engine::encode(&base64::engine::general_purpose::STANDARD, &item.data);
            format!("data:{};base64,{}", item.content_type, b64)
        } else { return; }
    } else { return; };

    // Write Run > Content > pptxDrawing(12)
    w.write_item(crate::constants::par::RUN, |w| {
        w.write_item(crate::constants::run::CONTENT, |w| {
            // pptxDrawing = 12, Read2 Variable
            w.write_prop_item(12, |w| {
                // Type = Inline (0)
                w.write_prop_byte(0, 0); // c_oSerImageType2.Type=0, value=Inline(0)
                // Extent (14) = Variable
                w.write_prop_item(14, |w| {
                    w.write_prop_long(2, width_emu);  // CxEmu
                    w.write_prop_long(3, height_emu);  // CyEmu
                });
                // PptxData (1) = Variable
                w.write_prop_item(1, |w| {
                    write_pptx_picture_binary(w, width_mm_emu, height_mm_emu, &img_src);
                });
            });
        });
    });
}

/// Write the PPTX binary for a picture, matching the format captured from sdkjs.
/// Structure: record(0) > record(1) > record(2=Picture) > [nvPicPr, blipFill, spPr]
fn write_pptx_picture_binary(w: &mut DocyWriter, cx: u32, cy: u32, img_src: &str) {
    // Record 0 (outer container)
    pptx_rec(w, 0, |w| {
        // Record 1 (inner container)
        pptx_rec(w, 1, |w| {
            // Record 2 (Picture type)
            pptx_rec(w, 2, |w| {
                // Record 0: nvPicPr (empty)
                pptx_rec(w, 0, |_w| {});

                // Record 1: UniFill (blipFill)
                pptx_rec(w, 1, |w| {
                    // WriteUniFill → StartRecord(FILL_TYPE_BLIP=1)
                    pptx_rec(w, 1, |w| {
                        // Attributes: rotWithShape = true
                        w.write_byte(0xFA); // g_nodeAttributeStart
                        w.write_byte(1);    // attr id 1 (rotWithShape)
                        w.write_byte(1);    // value = true
                        w.write_byte(0xFB); // g_nodeAttributeEnd

                        // WriteBlip → Record 0
                        pptx_rec(w, 0, |w| {
                            // Blip attributes (empty)
                            w.write_byte(0xFA);
                            w.write_byte(0xFB);

                            // Effects record (2) — 0 effects
                            pptx_rec(w, 2, |w| {
                                w.write_long(0);
                            });

                            // Image path record (3) — _WriteString1(0, src)
                            pptx_rec(w, 3, |w| {
                                w.write_byte(0xFA); // g_nodeAttributeStart
                                // _WriteString1: type byte + WriteString2 (UTF-16LE with char count)
                                w.write_byte(0); // attr id 0
                                let utf16: Vec<u16> = img_src.encode_utf16().collect();
                                w.write_long(utf16.len() as u32);
                                for ch in &utf16 {
                                    w.write_raw(&ch.to_le_bytes());
                                }
                                w.write_byte(0xFB); // g_nodeAttributeEnd
                            });
                        });
                    });
                });

                // Record 2: spPr (shape properties)
                pptx_rec(w, 2, |w| {
                    // spPr attributes (empty)
                    w.write_byte(0xFA);
                    w.write_byte(0xFB);
                    // xfrm record (0)
                    pptx_rec(w, 0, |w| {
                        // xfrm attributes (empty)
                        w.write_byte(0xFA);
                        w.write_byte(0xFB);
                        // ext record (1) — size
                        pptx_rec(w, 1, |w| {
                            w.write_byte(0xFA); // g_nodeAttributeStart
                            // _WriteInt2(0, cx)
                            w.write_byte(0); // attr id
                            w.write_long(cx);
                            // _WriteInt2(1, cy)
                            w.write_byte(1); // attr id
                            w.write_long(cy);
                            w.write_byte(0xFB); // g_nodeAttributeEnd
                        });
                    });
                });
            });
        });
    });
}

/// Write a PPTX record: [type:1][length:4][content]
fn pptx_rec<F: FnOnce(&mut DocyWriter)>(w: &mut DocyWriter, record_type: u8, f: F) {
    w.write_byte(record_type);
    let len_pos = w.position();
    w.write_long(0); // placeholder
    f(w);
    let content_len = (w.position() - len_pos - 4) as u32;
    let bytes = content_len.to_le_bytes();
    let buf = w.as_bytes_mut();
    buf[len_pos] = bytes[0];
    buf[len_pos + 1] = bytes[1];
    buf[len_pos + 2] = bytes[2];
    buf[len_pos + 3] = bytes[3];
}
