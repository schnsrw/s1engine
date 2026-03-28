#[test]
fn inspect_complex_header() {
    use s1_model::AttributeKey;
    let data = match std::fs::read("complex.docx") {
        Ok(d) => d,
        Err(_) => { eprintln!("SKIP: complex.docx not found"); return; }
    };
    let m = s1_format_docx::reader::read(&data).unwrap();
    let sects = m.sections();
    eprintln!("sections: {}, media: {}", sects.len(), m.media().len());
    for (i, s) in sects.iter().enumerate() {
        for h in &s.headers {
            eprintln!("sect {} hdr {:?} -> {:?}", i, h.hf_type, h.node_id);
            fn dump(m: &s1_model::DocumentModel, id: s1_model::NodeId, depth: usize) {
                if let Some(n) = m.node(id) {
                    let media = n.attributes.get(&AttributeKey::ImageMediaId).is_some();
                    let pad = "  ".repeat(depth);
                    eprintln!("{}{:?} {:?} media={} attrs={} children={}",
                        pad, n.node_type, id, media, n.attributes.len(), n.children.len());
                    for c in &n.children { dump(m, *c, depth + 1); }
                }
            }
            dump(&m, h.node_id, 1);
        }
        if i > 0 { break; }
    }
}
