use base64::Engine as _;
use s1_format_docy as docy;
use s1engine::{Engine, Format};
use std::path::{Path, PathBuf};

fn engine() -> Engine {
    Engine::new()
}

fn workspace_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn decode_docy_bytes(docy_str: &str) -> Vec<u8> {
    let parts: Vec<&str> = docy_str.splitn(4, ';').collect();
    assert_eq!(parts.len(), 4, "invalid DOCY wrapper");
    assert_eq!(parts[0], "DOCY");
    base64::engine::general_purpose::STANDARD
        .decode(parts[3])
        .expect("invalid DOCY base64")
}

fn document_table(binary: &[u8]) -> (usize, usize) {
    let table_count = binary[0] as usize;
    for i in 0..table_count {
        let pos = 1 + i * 5;
        let table_type = binary[pos];
        let offset = u32::from_le_bytes([
            binary[pos + 1],
            binary[pos + 2],
            binary[pos + 3],
            binary[pos + 4],
        ]) as usize;
        if table_type == 6 {
            let len = u32::from_le_bytes([
                binary[offset],
                binary[offset + 1],
                binary[offset + 2],
                binary[offset + 3],
            ]) as usize;
            return (offset + 4, len);
        }
    }
    panic!("document table not found");
}

fn walk_read1_items(binary: &[u8], start: usize, len: usize) -> Vec<(u8, usize, usize)> {
    let mut items = Vec::new();
    let mut p = start;
    let end = start + len;
    while p < end {
        assert!(p + 5 <= end, "truncated Read1 item header at {}", p);
        let item_type = binary[p];
        let item_len = u32::from_le_bytes([
            binary[p + 1],
            binary[p + 2],
            binary[p + 3],
            binary[p + 4],
        ]) as usize;
        let next = p + 5 + item_len;
        assert!(
            next <= end,
            "Read1 item type={} at {} overruns container: next={} end={}",
            item_type,
            p,
            next,
            end
        );
        items.push((item_type, item_len, p));
        p = next;
    }
    assert_eq!(p, end, "Read1 walk ended at {} expected {}", p, end);
    items
}

fn walk_read2_items(binary: &[u8], start: usize, len: usize) -> Vec<(u8, u8, usize, usize)> {
    let mut items = Vec::new();
    let mut p = start;
    let end = start + len;
    while p < end {
        assert!(p + 2 <= end, "truncated Read2 item header at {}", p);
        let item_type = binary[p];
        let len_type = binary[p + 1];
        let (payload_len, header_len) = match len_type {
            0 => (0usize, 2usize),
            1 => (1usize, 2usize),
            2 => (2usize, 2usize),
            3 => (3usize, 2usize),
            4 => (4usize, 2usize),
            5 => (8usize, 2usize),
            6 => {
                assert!(p + 6 <= end, "truncated variable Read2 header at {}", p);
                let l = u32::from_le_bytes([
                    binary[p + 2],
                    binary[p + 3],
                    binary[p + 4],
                    binary[p + 5],
                ]) as usize;
                (l, 6usize)
            }
            other => panic!("unknown Read2 lenType={} at {}", other, p),
        };
        let next = p + header_len + payload_len;
        assert!(
            next <= end,
            "Read2 item type={} lenType={} at {} overruns container: next={} end={}",
            item_type,
            len_type,
            p,
            next,
            end
        );
        items.push((item_type, len_type, payload_len, p));
        p = next;
    }
    assert_eq!(p, end, "Read2 walk ended at {} expected {}", p, end);
    items
}

fn validate_first_paragraph(binary: &[u8], doc_start: usize, doc_len: usize) {
    let top = walk_read1_items(binary, doc_start, doc_len);
    let first_para = top
        .iter()
        .find(|(item_type, _, _)| *item_type == 0)
        .expect("no paragraph in document table");
    let para_start = first_para.2 + 5;
    let para_len = first_para.1;
    let para_items = walk_read1_items(binary, para_start, para_len);
    for (item_type, item_len, item_pos) in para_items {
        match item_type {
            1 => {
                let _ = walk_read2_items(binary, item_pos + 5, item_len);
            }
            2 => {
                let run_items = walk_read1_items(binary, item_pos + 5, item_len);
                for (run_item_type, run_item_len, run_item_pos) in run_items {
                    if run_item_type == 5 {
                        let run_children = walk_read1_items(binary, run_item_pos + 5, run_item_len);
                        for (run_child_type, run_child_len, run_child_pos) in run_children {
                            match run_child_type {
                                1 => {
                                    let _ = walk_read2_items(binary, run_child_pos + 5, run_child_len);
                                }
                                8 => {
                                    let _ = walk_read1_items(binary, run_child_pos + 5, run_child_len);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn validate_docx_fixture(path: &Path) {
    let bytes = std::fs::read(path).unwrap_or_else(|_| panic!("missing fixture {}", path.display()));
    let doc = engine().open(&bytes).expect("failed to open fixture");
    let docy_str = docy::write(doc.model());
    let binary = decode_docy_bytes(&docy_str);
    let (doc_start, doc_len) = document_table(&binary);
    let top_items = walk_read1_items(&binary, doc_start, doc_len);
    let paragraph_count = top_items.iter().filter(|(t, _, _)| *t == 0).count();
    assert!(paragraph_count > 1, "expected multiple paragraphs in {}", path.display());
    validate_first_paragraph(&binary, doc_start, doc_len);
}

#[test]
fn structural_validation_simple_txt() {
    let doc = engine()
        .open_as(b"Hello World\nSecond paragraph\nThird paragraph", Format::Txt)
        .unwrap();
    let binary = decode_docy_bytes(&docy::write(doc.model()));
    let (doc_start, doc_len) = document_table(&binary);
    let top_items = walk_read1_items(&binary, doc_start, doc_len);
    let paragraph_count = top_items.iter().filter(|(t, _, _)| *t == 0).count();
    assert_eq!(paragraph_count, 3);
    validate_first_paragraph(&binary, doc_start, doc_len);
}

#[test]
fn structural_validation_complex_docx() {
    validate_docx_fixture(&workspace_path("complex.docx"));
}

#[test]
fn structural_validation_calibre_demo_docx() {
    validate_docx_fixture(&workspace_path("testdocs/docx/samples/calibre_demo.docx"));
}
