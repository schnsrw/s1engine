//! Comprehensive test: load every testdoc DOCX through the full pipeline
//! (read -> model -> layout -> HTML + PDF + round-trip DOCX) and report issues.
//! Also tests cut-all-and-paste (clear -> rebuild) fidelity.

use std::path::PathBuf;

fn testdocs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdocs")
}

fn collect_docx_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_docx_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("docx") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn count_paragraphs(model: &s1_model::DocumentModel, node_id: s1_model::NodeId) -> u32 {
    let mut count = 0;
    if let Some(node) = model.node(node_id) {
        if node.node_type == s1_model::NodeType::Paragraph {
            count += 1;
        }
        for &child in &node.children {
            count += count_paragraphs(model, child);
        }
    }
    count
}

fn extract_text(model: &s1_model::DocumentModel, node_id: s1_model::NodeId) -> String {
    let mut text = String::new();
    if let Some(node) = model.node(node_id) {
        if let Some(ref t) = node.text_content {
            text.push_str(t);
        }
        for &child in &node.children {
            text.push_str(&extract_text(model, child));
        }
    }
    text
}

#[cfg(all(feature = "docx", feature = "layout", feature = "pdf"))]
#[test]
fn analyze_all_testdocs() {
    let dir = testdocs_dir();
    if !dir.exists() {
        eprintln!("SKIP: testdocs directory not found at {}", dir.display());
        return;
    }

    let files = collect_docx_files(&dir);
    assert!(
        !files.is_empty(),
        "No DOCX files found in {}",
        dir.display()
    );

    let engine = s1engine::Engine::new();
    let font_db = s1_text::FontDatabase::new();

    let mut total_issues: Vec<String> = Vec::new();
    let sep = "=".repeat(70);

    for file_path in &files {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
        eprintln!("\n{sep}");
        eprintln!("FILE: {file_name}");
        eprintln!("{sep}");

        let data = match std::fs::read(file_path) {
            Ok(d) => d,
            Err(e) => {
                total_issues.push(format!("[{file_name}] READ ERROR: {e}"));
                continue;
            }
        };

        // -- Step 1: Parse DOCX --
        let doc = match engine.open(&data) {
            Ok(d) => d,
            Err(e) => {
                total_issues.push(format!("[{file_name}] PARSE ERROR: {e}"));
                continue;
            }
        };

        let model = doc.model();
        let body_id = match model.body_id() {
            Some(id) => id,
            None => {
                total_issues.push(format!("[{file_name}] NO BODY NODE"));
                continue;
            }
        };
        let body_node = model.node(body_id).unwrap();
        let child_count = body_node.children.len();
        let sections = model.sections();

        eprintln!(
            "  Parsed OK: {child_count} body children, {} sections",
            sections.len()
        );

        // Validate sections
        for (i, sec) in sections.iter().enumerate() {
            if sec.page_width < 100.0 || sec.page_width > 2000.0 {
                total_issues.push(format!(
                    "[{file_name}] SECTION {i}: unreasonable page_width = {:.1}pt",
                    sec.page_width
                ));
            }
            if sec.page_height < 100.0 || sec.page_height > 3000.0 {
                total_issues.push(format!(
                    "[{file_name}] SECTION {i}: unreasonable page_height = {:.1}pt",
                    sec.page_height
                ));
            }
            if sec.margin_left + sec.margin_right >= sec.page_width {
                total_issues.push(format!(
                    "[{file_name}] SECTION {i}: L+R margins ({:.1} + {:.1}) >= page_width ({:.1})",
                    sec.margin_left, sec.margin_right, sec.page_width
                ));
            }
            if sec.margin_top + sec.margin_bottom >= sec.page_height {
                total_issues.push(format!(
                    "[{file_name}] SECTION {i}: T+B margins ({:.1} + {:.1}) >= page_height ({:.1})",
                    sec.margin_top, sec.margin_bottom, sec.page_height
                ));
            }
            eprintln!(
                "  Section {i}: {:.0}x{:.0}pt, margins T={:.0} B={:.0} L={:.0} R={:.0}, cols={}",
                sec.page_width,
                sec.page_height,
                sec.margin_top,
                sec.margin_bottom,
                sec.margin_left,
                sec.margin_right,
                sec.columns
            );
        }

        // Walk tree stats
        let para_count = count_paragraphs(model, body_id);
        let original_text = extract_text(model, body_id);
        eprintln!(
            "  Content: {para_count} paragraphs, {} chars text",
            original_text.len()
        );

        // -- Step 2: Layout --
        let layout = match doc.layout(&font_db) {
            Ok(l) => l,
            Err(e) => {
                total_issues.push(format!("[{file_name}] LAYOUT ERROR: {e}"));
                continue;
            }
        };

        eprintln!("  Layout OK: {} pages", layout.pages.len());

        if layout.pages.is_empty() && para_count > 0 {
            total_issues.push(format!(
                "[{file_name}] LAYOUT: 0 pages but {para_count} paragraphs"
            ));
        }

        // Validate layout pages
        for (pi, page) in layout.pages.iter().enumerate() {
            if page.width <= 0.0 || page.height <= 0.0 {
                total_issues.push(format!(
                    "[{file_name}] PAGE {pi}: invalid dimensions {:.1}x{:.1}",
                    page.width, page.height
                ));
            }

            let ca = &page.content_area;
            if ca.width <= 0.0 || ca.height <= 0.0 {
                total_issues.push(format!(
                    "[{file_name}] PAGE {pi}: invalid content area {:.1}x{:.1}",
                    ca.width, ca.height
                ));
            }

            // Check blocks
            let mut prev_bottom = ca.y;
            for (bi, block) in page.blocks.iter().enumerate() {
                let b = &block.bounds;

                if b.width < 0.0 || b.height < 0.0 {
                    total_issues.push(format!(
                        "[{file_name}] P{pi} B{bi}: negative bounds {:.1}x{:.1}",
                        b.width, b.height
                    ));
                }

                if b.height < 0.001 {
                    total_issues.push(format!("[{file_name}] P{pi} B{bi}: zero-height block"));
                }

                // Block overflows page content area (with generous tolerance
                // for spacing that leaks into bottom margin). Only flag truly
                // egregious overflows (>50% of bottom margin past content area,
                // or past page height).
                let block_bottom = b.y + b.height;
                let block_type = match &block.kind {
                    s1_layout::LayoutBlockKind::Paragraph { .. } => "Para",
                    s1_layout::LayoutBlockKind::Table { .. } => "Table",
                    s1_layout::LayoutBlockKind::Image { .. } => "Image",
                    _ => "Other",
                };
                let overflow_threshold = page.height + 2.0;
                if block_bottom > overflow_threshold {
                    total_issues.push(format!(
                        "[{file_name}] P{pi} B{bi}: overflows page ({:.1} > {:.1}) [{block_type} y={:.1} h={:.1}]",
                        block_bottom, page.height, b.y, b.height
                    ));
                }

                // Overlap with previous block (>1pt tolerance).
                // Skip overlap check if blocks have different x offsets
                // (multi-column layout — side by side, not overlapping).
                let prev_x = if bi > 0 {
                    page.blocks[bi - 1].bounds.x
                } else {
                    b.x
                };
                let same_column = (b.x - prev_x).abs() < 1.0;
                if same_column && b.y + 1.0 < prev_bottom && bi > 0 {
                    total_issues.push(format!(
                        "[{file_name}] P{pi} B{bi}: overlaps prev (y={:.1}, prev_bot={:.1}) [{block_type} h={:.1}]",
                        b.y, prev_bottom, b.height
                    ));
                }
                // Reset tracking when column changes
                prev_bottom = if same_column {
                    block_bottom
                } else {
                    b.y + b.height
                };

                // Check paragraph lines
                if let s1_layout::LayoutBlockKind::Paragraph { ref lines, .. } = block.kind {
                    if lines.is_empty() {
                        total_issues.push(format!("[{file_name}] P{pi} B{bi}: paragraph 0 lines"));
                    }
                    for (li, line) in lines.iter().enumerate() {
                        if line.height <= 0.0 {
                            total_issues.push(format!(
                                "[{file_name}] P{pi} B{bi} L{li}: height {:.2} <= 0",
                                line.height
                            ));
                        }
                        if line.height > 500.0 {
                            total_issues.push(format!(
                                "[{file_name}] P{pi} B{bi} L{li}: tall line {:.1}pt",
                                line.height
                            ));
                        }
                    }
                }

                // Check table rows
                if let s1_layout::LayoutBlockKind::Table { ref rows, .. } = block.kind {
                    for (ri, row) in rows.iter().enumerate() {
                        if row.bounds.height <= 0.0 {
                            total_issues
                                .push(format!("[{file_name}] P{pi} B{bi} R{ri}: zero-height row"));
                        }
                        for (ci, cell) in row.cells.iter().enumerate() {
                            if cell.bounds.width <= 0.0 {
                                total_issues.push(format!(
                                    "[{file_name}] P{pi} B{bi} R{ri} C{ci}: zero-width cell"
                                ));
                            }
                        }
                    }
                }
            }

            // Dump blocks on pages with issues (once per page)
            let page_has_issue = page.blocks.iter().enumerate().any(|(bi, block)| {
                let bb = block.bounds.y + block.bounds.height;
                let is_overflow = bb > page.height + 2.0;
                let is_overlap = bi > 0 && {
                    let prev_bb: f64 = page.blocks[..bi]
                        .iter()
                        .map(|b| b.bounds.y + b.bounds.height)
                        .last()
                        .unwrap_or(0.0);
                    block.bounds.y + 1.0 < prev_bb
                };
                is_overflow || is_overlap
            });
            if page_has_issue {
                eprintln!(
                    "\n  DIAG [{file_name}] P{pi}: page {:.0}x{:.0}, ca y={:.1} h={:.1} bot={:.1}, {} blocks",
                    page.width, page.height, ca.y, ca.height, ca.y + ca.height, page.blocks.len()
                );
                for (di, db) in page.blocks.iter().enumerate() {
                    let dt = match &db.kind {
                        s1_layout::LayoutBlockKind::Paragraph { .. } => "Para",
                        s1_layout::LayoutBlockKind::Table { .. } => "Table",
                        s1_layout::LayoutBlockKind::Image { .. } => "Image",
                        _ => "Other",
                    };
                    eprintln!(
                        "    B{di}: {dt:5} y={:7.1} h={:7.1} bot={:7.1}",
                        db.bounds.y,
                        db.bounds.height,
                        db.bounds.y + db.bounds.height
                    );
                }
            }
        }

        // -- Step 3: HTML generation --
        let html = s1_layout::layout_to_html(&layout);
        let html_page_count = html.matches("class=\"s1-page\"").count();
        if html_page_count != layout.pages.len() {
            total_issues.push(format!(
                "[{file_name}] HTML: {html_page_count} divs vs {} layout pages",
                layout.pages.len()
            ));
        }
        eprintln!("  HTML OK: {} bytes, {html_page_count} pages", html.len());

        // -- Step 4: PDF export --
        match doc.export_pdf(&font_db) {
            Ok(pdf_bytes) => {
                if !pdf_bytes.starts_with(b"%PDF") {
                    total_issues.push(format!("[{file_name}] PDF: missing header"));
                }
                eprintln!("  PDF OK: {} bytes", pdf_bytes.len());
            }
            Err(e) => {
                total_issues.push(format!("[{file_name}] PDF ERROR: {e}"));
            }
        }

        // -- Step 5: DOCX round-trip --
        match doc.export(s1engine::Format::Docx) {
            Ok(docx_bytes) => match engine.open(&docx_bytes) {
                Ok(doc2) => {
                    let m2 = doc2.model();
                    let rt_para_count = m2
                        .body_id()
                        .map(|bid| count_paragraphs(m2, bid))
                        .unwrap_or(0);
                    let rt_text = m2
                        .body_id()
                        .map(|bid| extract_text(m2, bid))
                        .unwrap_or_default();

                    if rt_para_count == 0 && para_count > 0 {
                        total_issues.push(format!(
                            "[{file_name}] ROUNDTRIP: lost ALL paras ({para_count} -> 0)"
                        ));
                    } else if para_count > 2 && rt_para_count < para_count / 2 {
                        total_issues.push(format!(
                            "[{file_name}] ROUNDTRIP: lost >50% paras ({para_count} -> {rt_para_count})"
                        ));
                    }

                    let orig_trimmed = original_text.trim();
                    let rt_trimmed = rt_text.trim();
                    if !orig_trimmed.is_empty() && rt_trimmed.is_empty() {
                        total_issues.push(format!(
                            "[{file_name}] ROUNDTRIP: lost ALL text ({} -> 0 chars)",
                            orig_trimmed.len()
                        ));
                    } else if orig_trimmed.len() > 10
                        && (rt_trimmed.len() as f64) < (orig_trimmed.len() as f64 * 0.5)
                    {
                        total_issues.push(format!(
                            "[{file_name}] ROUNDTRIP: lost >50% text ({} -> {} chars)",
                            orig_trimmed.len(),
                            rt_trimmed.len()
                        ));
                    }

                    // Layout round-tripped doc, compare page counts
                    if let Ok(layout2) = doc2.layout(&font_db) {
                        let page_diff =
                            (layout2.pages.len() as i64 - layout.pages.len() as i64).abs();
                        if layout.pages.len() > 1 && page_diff > 1 {
                            total_issues.push(format!(
                                "[{file_name}] RT LAYOUT: pages {} -> {}",
                                layout.pages.len(),
                                layout2.pages.len()
                            ));
                        }
                        eprintln!(
                            "  Roundtrip: {para_count}->{rt_para_count} paras, {}->{} pages, {}->{} chars",
                            layout.pages.len(),
                            layout2.pages.len(),
                            orig_trimmed.len(),
                            rt_trimmed.len()
                        );
                    }
                }
                Err(e) => {
                    total_issues.push(format!("[{file_name}] RT READ ERROR: {e}"));
                }
            },
            Err(e) => {
                total_issues.push(format!("[{file_name}] RT WRITE ERROR: {e}"));
            }
        }

        // -- Step 6: Cut-all-paste simulation --
        // Build a fresh doc with same sections, insert plain-text paragraphs
        {
            let mut new_doc = engine.create();
            {
                let new_model = new_doc.model_mut();
                // Copy section properties
                let new_sections = new_model.sections_mut();
                new_sections.clear();
                for sec in sections.iter() {
                    new_sections.push(sec.clone());
                }

                let new_body_id = match new_model.body_id() {
                    Some(id) => id,
                    None => continue,
                };

                // Collect paragraph texts from original
                fn collect_para_texts(
                    model: &s1_model::DocumentModel,
                    node_id: s1_model::NodeId,
                    texts: &mut Vec<String>,
                ) {
                    if let Some(node) = model.node(node_id) {
                        if node.node_type == s1_model::NodeType::Paragraph {
                            texts.push(extract_text(model, node_id));
                        }
                        for &c in &node.children {
                            collect_para_texts(model, c, texts);
                        }
                    }
                }

                let mut para_texts: Vec<String> = Vec::new();
                collect_para_texts(model, body_id, &mut para_texts);

                // Insert paragraphs with text into new doc
                for (idx, text) in para_texts.iter().enumerate() {
                    let para_id = new_model.next_id();
                    let para = s1_model::Node::new(para_id, s1_model::NodeType::Paragraph);
                    if new_model.insert_node(new_body_id, idx, para).is_err() {
                        continue;
                    }
                    if !text.is_empty() {
                        let run_id = new_model.next_id();
                        let run = s1_model::Node::new(run_id, s1_model::NodeType::Run);
                        let _ = new_model.insert_node(para_id, 0, run);

                        let text_id = new_model.next_id();
                        let mut text_node = s1_model::Node::new(text_id, s1_model::NodeType::Text);
                        text_node.text_content = Some(text.clone());
                        let _ = new_model.insert_node(run_id, 0, text_node);
                    }
                }
            }

            match new_doc.layout(&font_db) {
                Ok(new_layout) => {
                    let new_page_count = new_layout.pages.len();
                    let orig_page_count = layout.pages.len();

                    let new_body = new_doc.model().body_id().unwrap();
                    let new_text = extract_text(new_doc.model(), new_body);
                    let orig_clean: String = original_text
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect();
                    let new_clean: String =
                        new_text.chars().filter(|c| !c.is_whitespace()).collect();
                    if !orig_clean.is_empty() && new_clean.is_empty() {
                        total_issues.push(format!("[{file_name}] CUT-PASTE: lost all text"));
                    }
                    eprintln!(
                        "  Cut-paste: {orig_page_count} -> {new_page_count} pages, text={}",
                        if orig_clean == new_clean {
                            "MATCH"
                        } else {
                            "PARTIAL"
                        }
                    );
                }
                Err(e) => {
                    total_issues.push(format!("[{file_name}] CUT-PASTE LAYOUT ERROR: {e}"));
                }
            }
        }
    }

    // -- Final report --
    eprintln!("\n{sep}");
    eprintln!(
        "ANALYSIS COMPLETE: {} files, {} issues",
        files.len(),
        total_issues.len()
    );
    eprintln!("{sep}");
    for (i, issue) in total_issues.iter().enumerate() {
        eprintln!("  {}. {issue}", i + 1);
    }
    if !total_issues.is_empty() {
        eprintln!("\n!!! {} ISSUES FOUND !!!", total_issues.len());
    }
}
