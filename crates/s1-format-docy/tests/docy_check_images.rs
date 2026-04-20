use s1engine::Engine;
use s1_model::NodeType;

#[test]
fn check_images_in_files() {
    let files = [
        "/Users/sachin/Downloads/Aruljothi.docx",
        "/Users/sachin/Downloads/Chat Reaction.docx",
        "/Users/sachin/Downloads/Nishtriya.docx",
    ];
    for path in &files {
        let engine = Engine::new();
        let bytes = std::fs::read(path).unwrap();
        let doc = engine.open(&bytes).unwrap();
        let model = doc.model();
        let name = path.rsplit('/').next().unwrap();
        
        // Walk all body paragraphs for Image/Drawing
        let mut images = 0;
        let mut drawings = 0;
        let body_id = model.body_id().unwrap();
        let body = model.node(body_id).unwrap();
        for cid in &body.children {
            if let Some(n) = model.node(*cid) {
                for kid in &n.children {
                    if let Some(k) = model.node(*kid) {
                        match k.node_type {
                            NodeType::Image => images += 1,
                            NodeType::Drawing => drawings += 1,
                            _ => {}
                        }
                    }
                }
            }
        }
        // Also check headers/footers
        let root = model.root_node().unwrap();
        for rid in &root.children {
            if let Some(r) = model.node(*rid) {
                if r.node_type == NodeType::Header || r.node_type == NodeType::Footer {
                    for pid in &r.children {
                        if let Some(p) = model.node(*pid) {
                            for kid in &p.children {
                                if let Some(k) = model.node(*kid) {
                                    match k.node_type {
                                        NodeType::Image => images += 1,
                                        NodeType::Drawing => drawings += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("{}: images={} drawings={} media={}", name, images, drawings, model.media().len());
    }
}
