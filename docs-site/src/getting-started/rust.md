# Rust Library

## Install

```toml
[dependencies]
s1engine = "1.0.1"
```

## Open a Document

```rust
use s1engine::Engine;

let engine = Engine::new();
let data = std::fs::read("report.docx")?;
let doc = engine.open(&data)?;

println!("Title: {:?}", doc.metadata().title);
println!("Text: {}", doc.to_plain_text());
println!("Words: {}", doc.to_plain_text().split_whitespace().count());
```

## Create a Document

```rust
use s1engine::{DocumentBuilder, Format};

let doc = DocumentBuilder::new()
    .title("My Report")
    .heading(1, "Introduction")
    .text("Built with Rudra Code.")
    .build();

std::fs::write("output.docx", doc.export(Format::Docx)?)?;
std::fs::write("output.pdf", doc.export(Format::Pdf)?)?;
```

## Convert Between Formats

```rust
let doc = engine.open(&std::fs::read("input.docx")?)?;
std::fs::write("output.odt", doc.export(Format::Odt)?)?;
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `docx` | DOCX read/write | Yes |
| `odt` | ODT read/write | Yes |
| `txt` | Plain text | Yes |
| `md` | Markdown (GFM) | Yes |
| `pdf` | PDF export | No |
| `crdt` | Collaboration | No |
| `convert` | Format conversion | No |
