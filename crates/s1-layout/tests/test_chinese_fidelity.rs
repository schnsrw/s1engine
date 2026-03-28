use s1_model::{DocumentModel, Node, NodeType};
use s1_layout::{LayoutEngine, LayoutConfig};
use s1_text::FontDatabase;

#[test]
fn test_chinese_layout_fidelity() {
    let mut model = DocumentModel::new();
    let root = model.root_id();
    
    // 0. Create Body
    let body_id = model.next_id();
    let body = Node::new(body_id, NodeType::Body);
    model.insert_node(root, 0, body).unwrap();
    
    // 1. Create a Paragraph
    let p1_id = model.next_id();
    let p1 = Node::new(p1_id, NodeType::Paragraph);
    model.insert_node(body_id, 0, p1).unwrap();
    
    // 2. Create a Run
    let r1_id = model.next_id();
    let r1 = Node::new(r1_id, NodeType::Run);
    model.insert_node(p1_id, 0, r1).unwrap();
    
    // 3. Create a Text node
    let t1_id = model.next_id();
    let t1 = Node::text(t1_id, "化学品及企业标识 ANTI-TERRA-205");
    model.insert_node(r1_id, 0, t1).unwrap();
    
    let db = FontDatabase::empty();
    let config = LayoutConfig::default();
    
    let mut engine = LayoutEngine::new(&model, &db, config);
    let layout = engine.layout().unwrap();
    
    assert_eq!(layout.pages.len(), 1);
    let page = &layout.pages[0];
    assert!(!page.blocks.is_empty());
    
    let mut found_text = false;
    for block in &page.blocks {
        if let s1_layout::LayoutBlockKind::Paragraph { lines, .. } = &block.kind {
            for line in lines {
                for run in &line.runs {
                    if run.text.contains("化学品") {
                        found_text = true;
                        assert!(run.width > 0.0);
                        println!("Chinese run width: {}", run.width);
                    }
                }
            }
        }
    }
    assert!(found_text, "Chinese text not found in layout output");
}

#[test]
fn test_chinese_wrapping() {
    let mut model = DocumentModel::new();
    let root = model.root_id();
    let body_id = model.next_id();
    model.insert_node(root, 0, Node::new(body_id, NodeType::Body)).unwrap();
    let p1_id = model.next_id();
    model.insert_node(body_id, 0, Node::new(p1_id, NodeType::Paragraph)).unwrap();
    let r1_id = model.next_id();
    model.insert_node(p1_id, 0, Node::new(r1_id, NodeType::Run)).unwrap();
    
    // Very long Chinese string that should wrap
    let long_text = "这是一段非常长的中文文本，旨在测试布局引擎是否能够正确处理自动换行。中文文本通常没有空格，布局引擎必须能够识别字符边界并在必要时进行换行。".repeat(5);
    let t1_id = model.next_id();
    model.insert_node(r1_id, 0, Node::text(t1_id, &long_text)).unwrap();
    
    let db = FontDatabase::empty();
    let config = LayoutConfig::default(); // default 612pt width
    let mut engine = LayoutEngine::new(&model, &db, config);
    let layout = engine.layout().unwrap();
    
    let page = &layout.pages[0];
    let block = &page.blocks[0];
    if let s1_layout::LayoutBlockKind::Paragraph { lines, .. } = &block.kind {
        println!("Long Chinese text line count: {}", lines.len());
        // It should have wrapped into multiple lines
        assert!(lines.len() > 1, "Chinese text should have wrapped into multiple lines");
        
        for (i, line) in lines.iter().enumerate() {
            let mut line_text = String::new();
            for run in &line.runs {
                line_text.push_str(&run.text);
            }
            println!("Line {}: {}", i, line_text);
            assert!(line.height > 0.0);
        }
    }
}
