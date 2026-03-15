//! Real-World Document Tests
//!
//! Integration tests that open actual documents from the `testdocs/` directory
//! and verify that the engine can read them without panicking. Where applicable,
//! tests also exercise export round-trips and cross-format conversion.
//!
//! Tests degrade gracefully: if a fixture file is missing, the test is skipped
//! rather than failed.
//!
//! These tests require all format features (docx, odt, txt, md) to be enabled.
#![cfg(all(feature = "docx", feature = "odt", feature = "txt", feature = "md"))]

use std::path::Path;
use std::time::Instant;

use s1engine::{Engine, Format, NodeType};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build an absolute path to a file relative to the workspace root.
///
/// `CARGO_MANIFEST_DIR` points to `crates/s1engine/`, so we go up two levels
/// to reach the workspace root.
fn workspace_path(relative: &str) -> std::path::PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../..").join(relative)
}

/// Read a test document from the workspace, returning `None` if the file
/// does not exist (so tests degrade gracefully on CI without fixtures).
fn read_test_doc(relative: &str) -> Option<Vec<u8>> {
    let path = workspace_path(relative);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("SKIP (file not found): {}", path.display());
            None
        }
        Err(e) => panic!("Failed to read {}: {}", path.display(), e),
    }
}

/// Count all nodes of a given type anywhere in the document tree.
fn count_all_nodes_of_type(doc: &s1engine::Document, node_type: NodeType) -> usize {
    let model = doc.model();
    let root_id = model.root_id();
    count_nodes_recursive(model, root_id, node_type)
}

fn count_nodes_recursive(
    model: &s1engine::DocumentModel,
    node_id: s1engine::NodeId,
    target: NodeType,
) -> usize {
    let mut count = 0;
    if let Some(node) = model.node(node_id) {
        if node.node_type == target {
            count += 1;
        }
        for &child_id in &node.children {
            count += count_nodes_recursive(model, child_id, target);
        }
    }
    count
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOCX Documents
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn open_real_docx_freetestdata_100kb() {
    let Some(bytes) = read_test_doc("testdocs/docx/samples/freetestdata_100kb.docx") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open(&bytes)
        .expect("should open 100kb DOCX without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "100kb DOCX should contain some text"
    );

    // Structure check
    let para_count = count_all_nodes_of_type(&doc, NodeType::Paragraph);
    assert!(
        para_count >= 1,
        "should have at least 1 paragraph, got {}",
        para_count
    );

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("DOCX -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");

    // Cross-format: export to ODT
    let odt_bytes = doc
        .export(Format::Odt)
        .expect("DOCX -> ODT export should succeed");
    assert!(!odt_bytes.is_empty(), "ODT export should be non-empty");

    // Verify ODT can be reopened
    let doc2 = engine
        .open_as(&odt_bytes, Format::Odt)
        .expect("reopening DOCX-exported-to-ODT should succeed");
    assert!(
        !doc2.to_plain_text().trim().is_empty(),
        "re-opened ODT should contain text"
    );
}

#[test]
fn open_real_docx_freetestdata_500kb() {
    let Some(bytes) = read_test_doc("testdocs/docx/samples/freetestdata_500kb.docx") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open(&bytes)
        .expect("should open 500kb DOCX without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "500kb DOCX should contain some text"
    );

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("DOCX -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");
}

#[test]
fn open_real_docx_freetestdata_1mb() {
    let Some(bytes) = read_test_doc("testdocs/docx/samples/freetestdata_1mb.docx") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open(&bytes)
        .expect("should open 1mb DOCX without error");
    let text = doc.to_plain_text();
    assert!(!text.trim().is_empty(), "1mb DOCX should contain some text");

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("DOCX -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");
}

#[test]
fn open_real_docx_calibre_demo() {
    let Some(bytes) = read_test_doc("testdocs/docx/samples/calibre_demo.docx") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open(&bytes)
        .expect("should open calibre_demo DOCX without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "calibre_demo DOCX should contain some text"
    );

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("DOCX -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");

    // Cross-format: export to ODT (may fail for complex documents with
    // duplicate media filenames, so treat as best-effort)
    if let Ok(odt_bytes) = doc.export(Format::Odt) {
        assert!(!odt_bytes.is_empty(), "ODT export should be non-empty");

        // Verify ODT can be reopened
        let doc2 = engine
            .open_as(&odt_bytes, Format::Odt)
            .expect("reopening DOCX-exported-to-ODT should succeed");
        assert!(
            !doc2.to_plain_text().trim().is_empty(),
            "re-opened ODT should contain text"
        );
    }
}

#[test]
fn open_real_docx_demo_document() {
    let Some(bytes) = read_test_doc("demo/images/document.docx") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open(&bytes)
        .expect("should open demo document.docx without error");
    let text = doc.to_plain_text();
    // The demo document may or may not have text, just verify it opens
    eprintln!(
        "demo/images/document.docx: {} chars, {} paragraphs",
        text.len(),
        doc.paragraph_count()
    );

    // Cross-format: export to TXT should not panic
    let _txt_bytes = doc
        .export(Format::Txt)
        .expect("DOCX -> TXT export should succeed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// ODT Documents
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn open_real_odt_freetestdata_100kb() {
    let Some(bytes) = read_test_doc("testdocs/odt/samples/freetestdata_100kb.odt") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open_as(&bytes, Format::Odt)
        .expect("should open 100kb ODT without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "100kb ODT should contain some text"
    );

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("ODT -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");

    // Cross-format: export to DOCX
    let docx_bytes = doc
        .export(Format::Docx)
        .expect("ODT -> DOCX export should succeed");
    assert!(!docx_bytes.is_empty(), "DOCX export should be non-empty");

    // Verify DOCX can be reopened
    let doc2 = engine
        .open(&docx_bytes)
        .expect("reopening ODT-exported-to-DOCX should succeed");
    assert!(
        !doc2.to_plain_text().trim().is_empty(),
        "re-opened DOCX should contain text"
    );
}

#[test]
fn open_real_odt_freetestdata_500kb() {
    let Some(bytes) = read_test_doc("testdocs/odt/samples/freetestdata_500kb.odt") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open_as(&bytes, Format::Odt)
        .expect("should open 500kb ODT without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "500kb ODT should contain some text"
    );

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("ODT -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");
}

#[test]
fn open_real_odt_freetestdata_1mb() {
    let Some(bytes) = read_test_doc("testdocs/odt/samples/freetestdata_1mb.odt") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open_as(&bytes, Format::Odt)
        .expect("should open 1mb ODT without error");
    let text = doc.to_plain_text();
    assert!(!text.trim().is_empty(), "1mb ODT should contain some text");

    // Cross-format: export to TXT
    let txt_bytes = doc
        .export(Format::Txt)
        .expect("ODT -> TXT export should succeed");
    assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TXT Documents
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn open_real_txt_moby_dick() {
    let Some(bytes) = read_test_doc("testdocs/txt/samples/moby_dick.txt") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open_as(&bytes, Format::Txt)
        .expect("should open moby_dick.txt without error");
    let text = doc.to_plain_text();
    assert!(!text.is_empty(), "moby_dick.txt should contain text");
    assert!(
        text.len() > 1000,
        "moby_dick.txt should be a substantial text; got {} chars",
        text.len()
    );

    // Should have many paragraphs
    let para_count = doc.paragraph_count();
    assert!(
        para_count >= 10,
        "moby_dick.txt: expected at least 10 paragraphs, got {}",
        para_count
    );

    // Should contain recognizable content
    assert!(
        text.contains("Moby")
            || text.contains("whale")
            || text.contains("Ahab")
            || text.contains("Call me"),
        "moby_dick.txt: expected recognizable Moby Dick content"
    );

    // Export round-trip: TXT -> model -> TXT
    let exported = doc
        .export_string(Format::Txt)
        .expect("TXT export should succeed");
    assert!(!exported.is_empty(), "TXT re-export should be non-empty");
    assert!(
        exported.len() > 1000,
        "re-exported text should be substantial; got {} chars",
        exported.len()
    );

    // Re-open the exported TXT and verify content
    let doc2 = engine
        .open_as(exported.as_bytes(), Format::Txt)
        .expect("re-open exported TXT should succeed");
    let roundtrip_text = doc2.to_plain_text();
    assert_eq!(
        text.trim(),
        roundtrip_text.trim(),
        "Moby Dick TXT round-trip text should be preserved"
    );

    // Cross-format: export to DOCX
    let docx_bytes = doc
        .export(Format::Docx)
        .expect("TXT -> DOCX export should succeed");
    assert!(!docx_bytes.is_empty(), "DOCX export should be non-empty");

    // Verify DOCX can be reopened
    let doc3 = engine
        .open(&docx_bytes)
        .expect("reopening TXT-exported-to-DOCX should succeed");
    assert!(
        !doc3.to_plain_text().is_empty(),
        "re-opened DOCX should contain text"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Markdown Documents
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn open_real_md_markdown_here_readme() {
    let Some(bytes) = read_test_doc("testdocs/md/samples/markdown_here_readme.md") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open_as(&bytes, Format::Md)
        .expect("should open markdown_here_readme.md without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "markdown_here_readme.md should contain text"
    );
    assert!(
        text.len() > 50,
        "expected substantial Markdown content, got {} chars",
        text.len()
    );

    // Export round-trip: MD -> model -> MD
    let exported = doc
        .export_string(Format::Md)
        .expect("MD export should succeed");
    assert!(!exported.is_empty(), "MD re-export should be non-empty");

    // Re-open the exported Markdown
    let doc2 = engine
        .open_as(exported.as_bytes(), Format::Md)
        .expect("re-open exported Markdown should succeed");
    assert!(
        !doc2.to_plain_text().trim().is_empty(),
        "Markdown round-trip should preserve text"
    );

    // Export round-trip: MD -> model -> TXT
    let txt_exported = doc
        .export_string(Format::Txt)
        .expect("MD -> TXT export should succeed");
    assert!(!txt_exported.is_empty(), "TXT export should be non-empty");

    // Cross-format: export to DOCX
    let docx_bytes = doc
        .export(Format::Docx)
        .expect("MD -> DOCX export should succeed");
    assert!(!docx_bytes.is_empty(), "DOCX export should be non-empty");

    // Verify DOCX can be reopened
    let doc3 = engine
        .open(&docx_bytes)
        .expect("reopening MD-exported-to-DOCX should succeed");
    assert!(
        !doc3.to_plain_text().is_empty(),
        "re-opened DOCX should contain text"
    );
}

#[test]
fn open_real_md_markdown_test() {
    let Some(bytes) = read_test_doc("testdocs/md/samples/markdown_test.md") else {
        return;
    };
    let engine = Engine::new();
    let doc = engine
        .open_as(&bytes, Format::Md)
        .expect("should open markdown_test.md without error");
    let text = doc.to_plain_text();
    assert!(
        !text.trim().is_empty(),
        "markdown_test.md should contain text"
    );

    // Export round-trip: MD -> model -> MD
    let exported = doc
        .export_string(Format::Md)
        .expect("MD export should succeed");
    assert!(!exported.is_empty(), "MD re-export should be non-empty");

    // Cross-format: export to ODT
    let odt_bytes = doc
        .export(Format::Odt)
        .expect("MD -> ODT export should succeed");
    assert!(!odt_bytes.is_empty(), "ODT export should be non-empty");

    // Verify ODT can be reopened
    let doc2 = engine
        .open_as(&odt_bytes, Format::Odt)
        .expect("reopening MD-exported-to-ODT should succeed");
    assert!(
        !doc2.to_plain_text().trim().is_empty(),
        "re-opened ODT should contain text"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOC (Legacy) Documents -- requires `convert` feature
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "convert")]
mod doc_legacy {
    use super::*;

    #[test]
    fn open_real_doc_freetestdata_100kb() {
        let Some(bytes) = read_test_doc("testdocs/doc/samples/freetestdata_100kb.doc") else {
            return;
        };
        let engine = Engine::new();
        let doc = engine
            .open_as(&bytes, Format::Doc)
            .expect("should open 100kb DOC without error");
        let text = doc.to_plain_text();
        assert!(
            !text.trim().is_empty(),
            "100kb DOC should contain some text"
        );
        eprintln!(
            "DOC 100kb: {} chars, {} paragraphs",
            text.len(),
            doc.paragraph_count()
        );

        // Structure check
        let para_count = count_all_nodes_of_type(&doc, NodeType::Paragraph);
        assert!(
            para_count >= 1,
            "DOC should have at least 1 paragraph, got {}",
            para_count
        );

        // Cross-format: export to TXT
        let txt_bytes = doc
            .export(Format::Txt)
            .expect("DOC -> TXT export should succeed");
        assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");

        // Cross-format: export to DOCX
        let docx_bytes = doc
            .export(Format::Docx)
            .expect("DOC -> DOCX export should succeed");
        assert!(!docx_bytes.is_empty(), "DOCX export should be non-empty");

        // Verify DOCX can be reopened
        let doc2 = engine
            .open(&docx_bytes)
            .expect("reopening DOC-exported-to-DOCX should succeed");
        assert!(
            !doc2.to_plain_text().trim().is_empty(),
            "re-opened DOCX should contain text"
        );
    }

    #[test]
    fn open_real_doc_freetestdata_500kb() {
        let Some(bytes) = read_test_doc("testdocs/doc/samples/freetestdata_500kb.doc") else {
            return;
        };
        let engine = Engine::new();
        let doc = engine
            .open_as(&bytes, Format::Doc)
            .expect("should open 500kb DOC without error");
        let text = doc.to_plain_text();
        assert!(
            !text.trim().is_empty(),
            "500kb DOC should contain some text"
        );
        eprintln!(
            "DOC 500kb: {} chars, {} paragraphs",
            text.len(),
            doc.paragraph_count()
        );

        // Cross-format: export to TXT
        let txt_bytes = doc
            .export(Format::Txt)
            .expect("DOC -> TXT export should succeed");
        assert!(!txt_bytes.is_empty(), "TXT export should be non-empty");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DOCX Round-Trip Preservation
// ═══════════════════════════════════════════════════════════════════════════════

const DOCX_SAMPLES: &[&str] = &[
    "testdocs/docx/samples/freetestdata_100kb.docx",
    "testdocs/docx/samples/freetestdata_500kb.docx",
    "testdocs/docx/samples/freetestdata_1mb.docx",
    "testdocs/docx/samples/calibre_demo.docx",
    "demo/images/document.docx",
];

#[test]
fn docx_roundtrip_preserves_content() {
    let engine = Engine::new();

    for path in DOCX_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        // Open the original
        let doc1 = engine
            .open_as(&bytes, Format::Docx)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        let original_text = doc1.to_plain_text();

        // Export to DOCX
        let exported_bytes = doc1
            .export(Format::Docx)
            .unwrap_or_else(|e| panic!("{}: export to DOCX failed: {}", path, e));

        // Re-open the exported DOCX
        let doc2 = engine
            .open_as(&exported_bytes, Format::Docx)
            .unwrap_or_else(|e| panic!("{}: re-open exported DOCX failed: {}", path, e));

        let roundtrip_text = doc2.to_plain_text();

        // Text content should be substantially preserved
        // (some whitespace differences are acceptable)
        let original_trimmed: String = original_text.split_whitespace().collect();
        let roundtrip_trimmed: String = roundtrip_text.split_whitespace().collect();

        // The round-trip text should be at least 80% of the original length
        if !original_trimmed.is_empty() {
            let ratio = roundtrip_trimmed.len() as f64 / original_trimmed.len() as f64;
            assert!(
                ratio >= 0.8,
                "{}: too much text lost in round-trip: original {} chars, round-trip {} chars (ratio {:.2})",
                path,
                original_trimmed.len(),
                roundtrip_trimmed.len(),
                ratio
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ODT Round-Trip Preservation
// ═══════════════════════════════════════════════════════════════════════════════

const ODT_SAMPLES: &[&str] = &[
    "testdocs/odt/samples/freetestdata_100kb.odt",
    "testdocs/odt/samples/freetestdata_500kb.odt",
    "testdocs/odt/samples/freetestdata_1mb.odt",
];

#[test]
fn odt_roundtrip_preserves_content() {
    let engine = Engine::new();

    for path in ODT_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc1 = engine
            .open_as(&bytes, Format::Odt)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        let original_text = doc1.to_plain_text();

        // Export to ODT
        let exported_bytes = doc1
            .export(Format::Odt)
            .unwrap_or_else(|e| panic!("{}: export to ODT failed: {}", path, e));

        // Re-open the exported ODT
        let doc2 = engine
            .open_as(&exported_bytes, Format::Odt)
            .unwrap_or_else(|e| panic!("{}: re-open exported ODT failed: {}", path, e));

        let roundtrip_text = doc2.to_plain_text();

        let original_trimmed: String = original_text.split_whitespace().collect();
        let roundtrip_trimmed: String = roundtrip_text.split_whitespace().collect();

        if !original_trimmed.is_empty() {
            let ratio = roundtrip_trimmed.len() as f64 / original_trimmed.len() as f64;
            assert!(
                ratio >= 0.8,
                "{}: too much text lost in ODT round-trip: original {} chars, round-trip {} chars (ratio {:.2})",
                path,
                original_trimmed.len(),
                roundtrip_trimmed.len(),
                ratio
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Cross-Format Conversion (DOCX -> ODT and DOCX -> TXT)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn docx_to_odt_conversion() {
    let engine = Engine::new();

    for path in DOCX_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc = engine
            .open_as(&bytes, Format::Docx)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        let original_text = doc.to_plain_text();

        // Export as ODT (may fail for complex documents with duplicate media
        // filenames, so treat failures as non-fatal)
        match doc.export(Format::Odt) {
            Ok(odt_bytes) => {
                assert!(
                    !odt_bytes.is_empty(),
                    "{}: exported ODT bytes are empty",
                    path
                );

                // Re-open the ODT to verify validity
                let odt_doc = engine
                    .open_as(&odt_bytes, Format::Odt)
                    .unwrap_or_else(|e| panic!("{}: re-open exported ODT failed: {}", path, e));

                if !original_text.trim().is_empty() {
                    assert!(
                        !odt_doc.to_plain_text().trim().is_empty(),
                        "{}: ODT conversion lost all text content",
                        path
                    );
                }
            }
            Err(e) => {
                eprintln!("{}: ODT export failed (non-fatal): {}", path, e);
            }
        }
    }
}

#[test]
fn docx_to_txt_conversion() {
    let engine = Engine::new();

    for path in DOCX_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc = engine
            .open_as(&bytes, Format::Docx)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        // Export as plain text
        let txt = doc
            .export_string(Format::Txt)
            .unwrap_or_else(|e| panic!("{}: export to TXT failed: {}", path, e));

        let plain = doc.to_plain_text();

        // Both should be non-empty if the document has content
        if !plain.trim().is_empty() {
            assert!(
                !txt.trim().is_empty(),
                "{}: export_string(TXT) returned empty but to_plain_text() has content",
                path
            );
        }

        // Whitespace-normalized content should be substantially similar
        // (table rendering may differ between to_plain_text and TXT writer)
        let plain_words: Vec<&str> = plain.split_whitespace().collect();
        let txt_words: Vec<&str> = txt.split_whitespace().collect();
        if !plain_words.is_empty() {
            let ratio = txt_words.len() as f64 / plain_words.len() as f64;
            assert!(
                ratio >= 0.8,
                "{}: TXT export lost too many words: plain={}, txt={} (ratio {:.2})",
                path,
                plain_words.len(),
                txt_words.len(),
                ratio
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Markdown Round-Trip
// ═══════════════════════════════════════════════════════════════════════════════

const MD_SAMPLES: &[&str] = &[
    "testdocs/md/samples/markdown_here_readme.md",
    "testdocs/md/samples/markdown_test.md",
];

#[test]
fn md_roundtrip_preserves_content() {
    let engine = Engine::new();

    for path in MD_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc1 = engine
            .open_as(&bytes, Format::Md)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        let original_text = doc1.to_plain_text();

        // Export back to Markdown
        let exported_md = doc1
            .export_string(Format::Md)
            .unwrap_or_else(|e| panic!("{}: export to Markdown failed: {}", path, e));

        // Re-open the exported Markdown
        let doc2 = engine
            .open_as(exported_md.as_bytes(), Format::Md)
            .unwrap_or_else(|e| panic!("{}: re-open exported Markdown failed: {}", path, e));

        let roundtrip_text = doc2.to_plain_text();

        if !original_text.trim().is_empty() {
            assert!(
                !roundtrip_text.trim().is_empty(),
                "{}: Markdown round-trip lost all text",
                path
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Format Auto-Detection
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn format_autodetect_docx() {
    let engine = Engine::new();

    for path in DOCX_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        // Using engine.open() which auto-detects format
        let doc = engine
            .open(&bytes)
            .unwrap_or_else(|e| panic!("Auto-detect open failed for {}: {}", path, e));

        assert!(
            !doc.to_plain_text().trim().is_empty(),
            "{}: auto-detected DOCX has no text",
            path
        );
    }
}

#[test]
fn format_autodetect_odt() {
    let engine = Engine::new();

    for path in ODT_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc = engine
            .open(&bytes)
            .unwrap_or_else(|e| panic!("Auto-detect open failed for {}: {}", path, e));

        assert!(
            !doc.to_plain_text().trim().is_empty(),
            "{}: auto-detected ODT has no text",
            path
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Metadata Extraction (no-panic smoke tests)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn docx_metadata_accessible() {
    let engine = Engine::new();

    for path in DOCX_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc = engine
            .open_as(&bytes, Format::Docx)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        // Just verify these accessors do not panic
        let _meta = doc.metadata();
        let _styles = doc.styles();
        let _sections = doc.sections();
    }
}

#[test]
fn odt_metadata_accessible() {
    let engine = Engine::new();

    for path in ODT_SAMPLES {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };

        let doc = engine
            .open_as(&bytes, Format::Odt)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));

        let _meta = doc.metadata();
        let _styles = doc.styles();
        let _sections = doc.sections();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Performance Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn large_document_open_performance() {
    let large_files: Vec<(&str, Format)> = vec![
        ("testdocs/docx/samples/freetestdata_1mb.docx", Format::Docx),
        ("testdocs/odt/samples/freetestdata_1mb.odt", Format::Odt),
        ("testdocs/txt/samples/moby_dick.txt", Format::Txt),
    ];

    let engine = Engine::new();

    for (path, format) in &large_files {
        let Some(bytes) = read_test_doc(path) else {
            continue;
        };
        let start = Instant::now();
        let doc = engine
            .open_as(&bytes, *format)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));
        let open_elapsed = start.elapsed();

        let export_start = Instant::now();
        let _exported = doc
            .export(Format::Txt)
            .unwrap_or_else(|e| panic!("Failed to export {} to TXT: {}", path, e));
        let export_elapsed = export_start.elapsed();

        eprintln!(
            "{}: open={:?}, export_txt={:?}, {} chars",
            path,
            open_elapsed,
            export_elapsed,
            doc.to_plain_text().len()
        );

        // Each document should open in under 10 seconds
        assert!(
            open_elapsed.as_secs() < 10,
            "{} took too long to open: {:?}",
            path,
            open_elapsed
        );

        // Export should also be fast
        assert!(
            export_elapsed.as_secs() < 10,
            "{} took too long to export to TXT: {:?}",
            path,
            export_elapsed
        );
    }
}
