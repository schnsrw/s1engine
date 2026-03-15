//! Integration tests that verify the code examples from README.md and docs/API_DESIGN.md
//! actually compile and run correctly. This serves as CI validation for markdown docs.
//!
//! Each test corresponds to a documented API pattern and is self-contained
//! (no external files required).

use s1engine::{DocumentBuilder, Engine, Format, Node, NodeType, Operation, Transaction};

// ---------------------------------------------------------------------------
// Test 1: README quick start pattern
// ---------------------------------------------------------------------------

/// Verifies the README "Quick Start / Create a Document" pattern:
/// create engine, build a doc via builder, export to DOCX, reopen, check text.
#[test]
fn readme_quick_start() {
    let engine = Engine::new();

    // Build a document (mirrors README "Create a Document" section)
    let doc = DocumentBuilder::new()
        .title("My Report")
        .author("Engineering")
        .heading(1, "Introduction")
        .paragraph(|p| {
            p.text("This is ")
                .bold("s1engine")
                .text(" -- a document engine in Rust.")
        })
        .table(|t| {
            t.row(|r| r.cell("Name").cell("Value"))
                .row(|r| r.cell("Users").cell("15,000"))
        })
        .build();

    // Export to DOCX bytes
    let docx_bytes = doc.export(Format::Docx).unwrap();
    assert!(!docx_bytes.is_empty());

    // Reopen and verify text is preserved
    let doc2 = engine.open(&docx_bytes).unwrap();
    let text = doc2.to_plain_text();
    assert!(text.contains("Introduction"));
    assert!(text.contains("s1engine"));
    assert!(text.contains("a document engine in Rust."));
    assert!(text.contains("Users"));
    assert!(text.contains("15,000"));

    // Metadata survives the round-trip
    assert_eq!(doc2.metadata().title.as_deref(), Some("My Report"));
    assert_eq!(doc2.metadata().creator.as_deref(), Some("Engineering"));
}

// ---------------------------------------------------------------------------
// Test 2: README DocumentBuilder pattern
// ---------------------------------------------------------------------------

/// Verifies the DocumentBuilder pattern from README and docs/API_DESIGN.md:
/// headings, paragraphs with bold/italic, tables, and metadata.
#[test]
fn readme_builder_pattern() {
    let doc = DocumentBuilder::new()
        .title("Quarterly Report")
        .author("Engineering Team")
        .heading(1, "Q4 2026 Report")
        .paragraph(|p| {
            p.text("Revenue grew by ")
                .bold("23%")
                .text(" compared to Q3.")
        })
        .heading(2, "Key Metrics")
        .table(|t| {
            t.row(|r| r.cell("Metric").cell("Q3").cell("Q4"))
                .row(|r| r.cell("Users").cell("10,000").cell("15,000"))
                .row(|r| r.cell("Revenue").cell("$1.2M").cell("$1.5M"))
        })
        .heading(2, "Conclusion")
        .paragraph(|p| p.text("Strong quarter with growth across all metrics."))
        .build();

    // Verify metadata
    assert_eq!(doc.metadata().title.as_deref(), Some("Quarterly Report"));
    assert_eq!(doc.metadata().creator.as_deref(), Some("Engineering Team"));

    // Verify text content
    let text = doc.to_plain_text();
    assert!(text.contains("Q4 2026 Report"));
    assert!(text.contains("Revenue grew by 23% compared to Q3."));
    assert!(text.contains("Key Metrics"));
    assert!(text.contains("Strong quarter with growth across all metrics."));
    assert!(text.contains("Metric"));
    assert!(text.contains("$1.5M"));

    // Verify heading styles were auto-created
    assert!(doc.style_by_id("Heading1").is_some());
    assert!(doc.style_by_id("Heading2").is_some());

    // Export to DOCX (as shown in the API_DESIGN.md example)
    let docx_bytes = doc.export(Format::Docx).unwrap();
    assert!(!docx_bytes.is_empty());

    // ODT export requires the odt feature
    #[cfg(feature = "odt")]
    {
        let odt_bytes = doc.export(Format::Odt).unwrap();
        assert!(!odt_bytes.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Test 3: API_DESIGN.md Use Case 3 -- Open and edit
// ---------------------------------------------------------------------------

/// Verifies the "Edit an Existing Document" pattern from API_DESIGN.md:
/// open bytes, apply InsertText operation, undo, export.
#[test]
fn api_design_open_and_edit() {
    let engine = Engine::new();

    // Create a document with a known text node we can edit
    let mut doc = engine.create();
    let body_id = doc.body_id().unwrap();

    // Build a paragraph -> run -> text structure
    let para_id = doc.next_id();
    doc.apply(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let run_id = doc.next_id();
    doc.apply(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ))
    .unwrap();

    let text_id = doc.next_id();
    doc.apply(Operation::insert_node(
        run_id,
        0,
        Node::text(text_id, "Contract text"),
    ))
    .unwrap();

    // Export, reopen (simulating opening a file)
    let bytes = doc.export(Format::Docx).unwrap();
    let mut doc = engine.open(&bytes).unwrap();

    // Find the text node to edit
    let body_id = doc.body_id().unwrap();
    let body = doc.node(body_id).unwrap();
    let para_id = body.children[0];
    let para = doc.node(para_id).unwrap();
    let run_id = para.children[0];
    let run = doc.node(run_id).unwrap();
    let text_id = run.children[0];

    // Edit via operation (as shown in API_DESIGN.md Use Case 3)
    doc.apply(Operation::insert_text(text_id, 0, "DRAFT: "))
        .unwrap();
    assert!(doc.to_plain_text().starts_with("DRAFT: Contract text"));

    // Undo the insert
    assert!(doc.can_undo());
    doc.undo().unwrap();
    assert_eq!(doc.to_plain_text(), "Contract text");

    // Redo brings it back
    assert!(doc.can_redo());
    doc.redo().unwrap();
    assert!(doc.to_plain_text().starts_with("DRAFT: Contract text"));

    // Export the edited document
    let output = doc.export(Format::Docx).unwrap();
    assert!(!output.is_empty());
}

// ---------------------------------------------------------------------------
// Test 4: API_DESIGN.md -- Transaction pattern
// ---------------------------------------------------------------------------

/// Verifies the Transaction::with_label() pattern from API_DESIGN.md:
/// create a labeled transaction, push operations, apply_transaction.
#[test]
fn api_design_transaction() {
    let engine = Engine::new();
    let mut doc = engine.create();
    let body_id = doc.body_id().unwrap();

    // Build structure: paragraph -> run -> text
    let para_id = doc.next_id();
    let run_id = doc.next_id();
    let text_id = doc.next_id();

    let mut txn = Transaction::with_label("Add paragraph with text");
    txn.push(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ));
    txn.push(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ));
    txn.push(Operation::insert_node(
        run_id,
        0,
        Node::text(text_id, "Hello World"),
    ));

    doc.apply_transaction(&txn).unwrap();
    assert_eq!(doc.to_plain_text(), "Hello World");

    // Now apply another labeled transaction (as in the API design doc)
    let mut txn2 = Transaction::with_label("Add disclaimer");
    txn2.push(Operation::insert_text(text_id, 0, "CONFIDENTIAL: "));
    doc.apply_transaction(&txn2).unwrap();
    assert_eq!(doc.to_plain_text(), "CONFIDENTIAL: Hello World");

    // Undo reverts just the last transaction
    doc.undo().unwrap();
    assert_eq!(doc.to_plain_text(), "Hello World");

    // Undo again reverts the first transaction
    doc.undo().unwrap();
    assert_eq!(doc.to_plain_text(), "");
}

// ---------------------------------------------------------------------------
// Test 5: API_DESIGN.md Use Case 4 -- Format conversion
// ---------------------------------------------------------------------------

/// Verifies the format conversion pattern from API_DESIGN.md:
/// open DOCX bytes, export as ODT, export as TXT. Verify text preserved.
#[test]
fn api_design_format_conversion() {
    // Create a document with known content
    let doc = DocumentBuilder::new()
        .heading(1, "Report Title")
        .paragraph(|p| p.text("First paragraph of the report."))
        .paragraph(|p| p.text("Second paragraph with details."))
        .build();

    // Export as DOCX first (simulates having a DOCX file)
    let docx_bytes = doc.export(Format::Docx).unwrap();

    // Open the DOCX bytes
    let engine = Engine::new();
    let doc = engine.open(&docx_bytes).unwrap();

    // Export to ODT (requires odt feature)
    #[cfg(feature = "odt")]
    {
        let odt_bytes = doc.export(Format::Odt).unwrap();
        assert!(!odt_bytes.is_empty());

        // Verify ODT content preserved
        let odt_doc = engine.open_as(&odt_bytes, Format::Odt).unwrap();
        let odt_text = odt_doc.to_plain_text();
        assert!(odt_text.contains("Report Title"));
        assert!(odt_text.contains("First paragraph of the report."));
    }

    // Export to TXT (requires txt feature)
    #[cfg(feature = "txt")]
    {
        let txt = doc.export_string(Format::Txt).unwrap();
        assert!(txt.contains("Report Title"));
        assert!(txt.contains("First paragraph of the report."));
        assert!(txt.contains("Second paragraph with details."));
    }
}

// ---------------------------------------------------------------------------
// Test 6: API_DESIGN.md Use Case 6 -- Full DocumentBuilder
// ---------------------------------------------------------------------------

/// Verifies the full-featured DocumentBuilder from API_DESIGN.md Use Case 6:
/// heading, paragraph with bold/italic/underline/superscript/subscript,
/// hyperlink, bookmark, bullet list, numbered list, section_with_header_footer,
/// title, author.
#[test]
fn api_design_document_builder_full() {
    let doc = DocumentBuilder::new()
        .title("Styled Document")
        .author("Test Author")
        .heading(1, "Introduction")
        .paragraph(|p| {
            p.text("Normal text, ")
                .bold("bold text, ")
                .italic("italic text, ")
                .bold_italic("both, ")
                .underline("underlined, ")
                .superscript("super")
                .text(" and ")
                .subscript("sub")
        })
        .paragraph(|p| {
            p.hyperlink("https://example.com", "Click here")
                .text(" or see ")
                .bookmark_start("section1")
                .text("this section")
                .bookmark_end()
        })
        .bullet("First item")
        .bullet("Second item")
        .numbered("Step one")
        .numbered("Step two")
        .section_with_header_footer("Page Header", "Page Footer")
        .build();

    // Verify metadata
    assert_eq!(doc.metadata().title.as_deref(), Some("Styled Document"));
    assert_eq!(doc.metadata().creator.as_deref(), Some("Test Author"));

    // Verify text content
    let text = doc.to_plain_text();
    assert!(text.contains("Introduction"));
    assert!(text.contains("Normal text, "));
    assert!(text.contains("bold text, "));
    assert!(text.contains("italic text, "));
    assert!(text.contains("both, "));
    assert!(text.contains("underlined, "));
    assert!(text.contains("super"));
    assert!(text.contains("sub"));
    assert!(text.contains("Click here"));
    assert!(text.contains("this section"));
    assert!(text.contains("First item"));
    assert!(text.contains("Second item"));
    assert!(text.contains("Step one"));
    assert!(text.contains("Step two"));

    // Verify heading style exists
    assert!(doc.style_by_id("Heading1").is_some());

    // Verify bullet and numbered list numbering definitions
    assert!(!doc.numbering().is_empty());
    assert!(doc.numbering().instances.len() >= 2);

    // Verify section with header/footer
    assert_eq!(doc.sections().len(), 1);
    assert_eq!(doc.sections()[0].headers.len(), 1);
    assert_eq!(doc.sections()[0].footers.len(), 1);

    // Verify header node content
    let hdr_id = doc.sections()[0].headers[0].node_id;
    let hdr = doc.node(hdr_id).unwrap();
    assert_eq!(hdr.node_type, NodeType::Header);

    // Verify footer node content
    let ftr_id = doc.sections()[0].footers[0].node_id;
    let ftr = doc.node(ftr_id).unwrap();
    assert_eq!(ftr.node_type, NodeType::Footer);

    // Verify it can be exported to DOCX successfully
    let bytes = doc.export(Format::Docx).unwrap();
    assert!(!bytes.is_empty());
}

// ---------------------------------------------------------------------------
// Test 7: API_DESIGN.md Use Case 7 -- Collaboration (feature-gated)
// ---------------------------------------------------------------------------

/// Verifies the CRDT collaboration pattern from API_DESIGN.md:
/// create_collab, apply_local insert, verify text.
///
/// This test requires the `crdt` feature flag.
#[cfg(feature = "crdt")]
#[test]
fn api_design_collaboration() {
    let engine = Engine::new();

    // Create a collaborative document with unique replica ID
    let mut doc = engine.create_collab(42);
    assert_eq!(doc.replica_id(), 42);

    // Set up document structure: paragraph -> run -> text node
    let body_id = doc.model().body_id().unwrap();

    let para_id = doc.next_id();
    let _crdt_op = doc
        .apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

    let run_id = doc.next_id();
    let _crdt_op = doc
        .apply_local(Operation::insert_node(
            para_id,
            0,
            Node::new(run_id, NodeType::Run),
        ))
        .unwrap();

    let text_id = doc.next_id();
    let _crdt_op = doc
        .apply_local(Operation::insert_node(run_id, 0, Node::text(text_id, "")))
        .unwrap();

    // Apply a local text insert (returns CrdtOperation for broadcast)
    let crdt_op = doc
        .apply_local(Operation::insert_text(text_id, 0, "Hello"))
        .unwrap();

    // The returned CrdtOperation carries the replica's OpId
    assert_eq!(crdt_op.id.replica, 42);

    // Verify the text was applied locally
    assert_eq!(doc.to_plain_text(), "Hello");

    // State vector reflects the operations
    let sv = doc.state_vector();
    assert!(sv.get(42) > 0);
}
