# API Design

## Design Principles

1. **Simple things should be simple** -- opening a document and exporting it is 3 lines of code
2. **Complex things should be possible** -- full access to the document model for advanced use cases
3. **No panics** -- all fallible operations return `Result`
4. **Zero-cost abstractions** -- high-level API compiles down to efficient code
5. **Builder pattern** for construction, **operation pattern** for editing
6. **Feature flags** for optional functionality -- don't pay for what you don't use

---

## Cargo Feature Flags

```toml
[features]
default = ["docx", "odt", "txt"]

# Format support (each pulls in its crate)
docx = ["dep:s1-format-docx"]
odt = ["dep:s1-format-odt"]
pdf = ["dep:s1-format-pdf", "dep:s1-layout", "dep:s1-text"]
txt = ["dep:s1-format-txt"]

# Conversion
convert = ["dep:s1-convert"]
doc-legacy = ["convert"]

# Collaboration
crdt = ["dep:s1-crdt"]
```

Usage:
```toml
# Minimal: just DOCX parsing
s1engine = { version = "0.1", default-features = false, features = ["docx"] }

# Full: everything
s1engine = { version = "0.1", features = ["pdf", "convert", "crdt"] }
```

---

## API Examples by Use Case

### Use Case 1: Open and Read a Document

```rust
use s1engine::Engine;

fn main() -> Result<(), s1engine::Error> {
    let engine = Engine::new();

    // Open from bytes (format auto-detected from content)
    let data = std::fs::read("report.docx")?;
    let doc = engine.open(&data)?;

    // Or open from file path (format detected from extension)
    let doc = engine.open_file("report.docx")?;

    // Read metadata
    println!("Title: {:?}", doc.metadata().title);
    println!("Author: {:?}", doc.metadata().creator);

    // Get plain text
    println!("{}", doc.to_plain_text());

    // Query structure
    println!("Paragraphs: {}", doc.paragraph_count());
    for id in doc.paragraph_ids() {
        if let Some(node) = doc.node(id) {
            println!("  {:?}: {:?}", node.node_type, node.text_content);
        }
    }

    Ok(())
}
```

### Use Case 2: Create a Document from Scratch

```rust
use s1engine::{DocumentBuilder, Format};

fn main() -> Result<(), s1engine::Error> {
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

    let docx_bytes = doc.export(Format::Docx)?;
    std::fs::write("report.docx", docx_bytes)?;

    let odt_bytes = doc.export(Format::Odt)?;
    std::fs::write("report.odt", odt_bytes)?;

    Ok(())
}
```

### Use Case 3: Edit an Existing Document

```rust
use s1engine::{Engine, Format, Operation};

fn main() -> Result<(), s1engine::Error> {
    let engine = Engine::new();
    let data = std::fs::read("contract.docx")?;
    let mut doc = engine.open(&data)?;

    // Edit via operations (preserves undo history)
    let text_id = /* find target text node */;
    doc.apply(Operation::insert_text(text_id, 0, "DRAFT: "))?;

    // Update metadata directly (not an operation)
    doc.metadata_mut().title = Some("Updated Contract".to_string());

    // Edit via transaction (atomic undo unit)
    let mut txn = s1engine::Transaction::with_label("Add disclaimer");
    txn.push(Operation::insert_text(text_id, 0, "CONFIDENTIAL: "));
    doc.apply_transaction(&txn)?;

    // Undo the last transaction
    doc.undo()?;

    // Export
    let output = doc.export(Format::Docx)?;
    std::fs::write("contract_updated.docx", output)?;
    Ok(())
}
```

### Use Case 4: Format Conversion

```rust
use s1engine::{Engine, Format};
use std::path::Path;

fn main() -> Result<(), s1engine::Error> {
    let engine = Engine::new();
    let input = std::fs::read("input.docx")?;
    let doc = engine.open(&input)?;

    // Export to different formats
    let odt = doc.export(Format::Odt)?;
    std::fs::write("output.odt", odt)?;

    let txt = doc.export_string(Format::Txt)?;
    std::fs::write("output.txt", txt)?;

    Ok(())
}
```

### Use Case 5: Batch Processing

```rust
use s1engine::Engine;

fn extract_metadata(engine: &Engine, data: &[u8]) -> Result<String, s1engine::Error> {
    let doc = engine.open(data)?;
    Ok(format!(
        "Title: {:?}, Author: {:?}, Paragraphs: {}, Text length: {}",
        doc.metadata().title,
        doc.metadata().creator,
        doc.paragraph_count(),
        doc.to_plain_text().len(),
    ))
}
```

### Use Case 6: Rich Content with Builder

```rust
use s1engine::{DocumentBuilder, Format, Color};

fn main() -> Result<(), s1engine::Error> {
    let doc = DocumentBuilder::new()
        .title("Styled Document")
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
        .build();

    std::fs::write("styled.docx", doc.export(Format::Docx)?)?;
    Ok(())
}
```

### Use Case 7: Collaboration (with `crdt` feature)

```rust
use s1engine::Engine;
// These types are available with the `crdt` feature
use s1engine::{CollabDocument, CrdtOperation, StateVector};

fn main() -> Result<(), s1engine::Error> {
    let engine = Engine::new();

    // Create a collaborative document with unique replica ID
    let mut doc = engine.create_collab(42);

    // Apply local operations (returns CrdtOps to broadcast)
    let ops = doc.apply_local(CrdtOperation::insert_text(
        doc.next_op_id(),
        /* node_id */, 0, "Hello".into(),
        None, None,
    ))?;

    // Serialize for network transport
    let encoded = s1engine::serialize_operations(&ops)?;

    // On another replica: apply remote operations
    let remote_ops = s1engine::deserialize_operations(&encoded)?;
    doc.apply_remote(remote_ops)?;

    // Incremental sync via state vectors
    let my_state = doc.state_vector();
    let changes = doc.changes_since(&remote_state_vector);

    Ok(())
}
```

---

## Error Handling

```rust
/// Top-level error type (s1engine::Error)
pub enum Error {
    /// Format parsing/writing error (DOCX, ODT, TXT, PDF)
    Format(String),

    /// Operation validation/application error
    Operation(s1_ops::OperationError),

    /// File I/O error
    Io(std::io::Error),

    /// Unsupported or unrecognized format
    UnsupportedFormat(String),

    /// CRDT error (feature-gated)
    #[cfg(feature = "crdt")]
    Crdt(s1_crdt::CrdtError),
}
```

Each internal crate has its own error type (`DocxError`, `OdtError`, `PdfError`, `TextError`, `LayoutError`, `ConvertError`, `CrdtError`) that converts into `s1engine::Error` via `From` implementations.

---

## Versioning & Stability

- **Pre-1.0** (`0.x.y`): API may change between minor versions. Use exact version pins.
- **1.0+**: Semantic versioning. Public API stable within major versions.
- **Internal crates** (`s1-model`, `s1-ops`, etc.) have independent version numbers.
- **Facade crate** (`s1engine`) re-exports the stable API -- consumers depend on this only.

---

## Naming Conventions

| Entity | Convention | Example |
|---|---|---|
| Crate names | `s1-{module}` | `s1-model`, `s1-format-docx` |
| Facade crate | `s1engine` | `s1engine` |
| Module names | `snake_case` | `line_break`, `table_layout` |
| Types | `PascalCase` | `NodeType`, `AttributeMap` |
| Functions | `snake_case` | `open_file`, `to_plain_text` |
| Constants | `SCREAMING_SNAKE` | `DEFAULT_FONT_SIZE` |
| Feature flags | `kebab-case` | `doc-legacy`, `crdt` |
| Error variants | `PascalCase` | `NodeNotFound`, `InvalidDocx` |
| C API prefix | `s1_` | `s1_engine_new`, `s1_document_free` |
