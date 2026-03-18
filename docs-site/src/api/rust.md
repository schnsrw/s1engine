# Rust API Reference

Full API documentation is auto-generated from source: `cargo doc --open`

## Core Types

### Engine

```rust
let engine = Engine::new();
let doc = engine.open(&bytes)?;       // Open from bytes
let doc = engine.open_file("path")?;  // Open from file
let doc = engine.create();             // New empty document
```

### Document

```rust
doc.to_plain_text()           // Extract text
doc.to_html()                 // Render to HTML
doc.export(Format::Pdf)?      // Export to format
doc.metadata()                // Get metadata
doc.model()                   // Access document model
doc.apply(operation)?         // Apply an operation
doc.undo()?                   // Undo last operation
doc.redo()?                   // Redo
```

### DocumentBuilder

```rust
DocumentBuilder::new()
    .title("Report")
    .heading(1, "Chapter 1")
    .text("Content here.")
    .bold("Important!")
    .table(|t| t.row(|r| r.cell("A").cell("B")))
    .build()
```

### Format

```rust
enum Format { Docx, Odt, Pdf, Txt, Md, Doc }
```

## Crate Documentation

Each crate has its own rustdoc:

```bash
cargo doc -p s1-model --open
cargo doc -p s1-ops --open
cargo doc -p s1-format-docx --open
cargo doc -p s1engine --open
```
