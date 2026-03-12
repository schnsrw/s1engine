# s1engine

A modular document engine built in Rust. Read, write, edit, and convert documents across DOCX, ODT, PDF, and TXT formats.

Designed as an embeddable SDK for building document editors, converters, and collaborative editing applications.

## Status

**Pre-release** (`0.1.x`) -- Phases 1-4 complete, Phase 5 in progress. Core functionality works. API is not yet stable.

- Comprehensive test suite across 13 crates (run `cargo test --workspace` to verify)
- DOCX, ODT, TXT read/write with round-trip fidelity
- PDF export (text, tables, images, hyperlinks, bookmarks)
- CRDT-based collaborative editing (Fugue text, tree moves, LWW attributes)
- WASM and C FFI bindings
- Pure Rust -- zero C/C++ dependencies

## Architecture

```
Consumer Applications
        |  Rust API / C FFI / WASM
+-------v--------------------------------------------+
|                s1engine (facade)                    |
|----------------------------------------------------|
|  s1-ops       s1-layout       s1-convert           |
|  Operations   Page Layout     Format Conversion    |
|  Undo/Redo    Pagination      DOC -> DOCX          |
|----------------------------------------------------|
|  s1-crdt                s1-model                   |
|  Collaborative          Core Document Model        |
|  Editing (Fugue)        (zero external deps)       |
|----------------------------------------------------|
|  format-docx  format-odt  format-pdf  format-txt   |
|----------------------------------------------------|
|                s1-text (Pure Rust)                  |
|        rustybuzz  ttf-parser  fontdb               |
+----------------------------------------------------+
```

## Quick Start

### Open and Read

```rust
use s1engine::{Engine, Format};

let engine = Engine::new();

// Open from bytes (format auto-detected)
let data = std::fs::read("report.docx")?;
let doc = engine.open(&data)?;

println!("{}", doc.to_plain_text());
println!("Title: {:?}", doc.metadata().title);
println!("Paragraphs: {}", doc.paragraph_count());
```

### Create a Document

```rust
use s1engine::DocumentBuilder;

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

let docx_bytes = doc.export(Format::Docx)?;
let odt_bytes = doc.export(Format::Odt)?;
```

### Open from File

```rust
use s1engine::Engine;

let engine = Engine::new();
let doc = engine.open_file("input.docx")?;
let output = doc.export(s1engine::Format::Odt)?;
std::fs::write("output.odt", output)?;
```

### Cargo Feature Flags

```toml
[dependencies]
# Default: DOCX + ODT + TXT
s1engine = "0.1"

# Minimal: just DOCX parsing
s1engine = { version = "0.1", default-features = false, features = ["docx"] }

# Full: everything including PDF export and CRDT
s1engine = { version = "0.1", features = ["pdf", "convert", "crdt"] }
```

| Feature | Description | Default |
|---|---|---|
| `docx` | DOCX (OOXML) read/write | Yes |
| `odt` | ODT (ODF) read/write | Yes |
| `txt` | Plain text read/write | Yes |
| `pdf` | PDF export (requires layout + text shaping) | No |
| `convert` | Format conversion pipelines | No |
| `doc-legacy` | DOC text extraction (via OLE2) | No |
| `crdt` | CRDT collaboration primitives | No |

## Crate Structure

| Crate | Description | Tests |
|---|---|---|
| `s1engine` | Facade -- high-level public API | 46 |
| `s1-model` | Core document model (tree, nodes, attributes, styles) | 72 |
| `s1-ops` | Operations, transactions, undo/redo | 48 |
| `s1-format-docx` | DOCX reader/writer | 167 |
| `s1-format-odt` | ODT reader/writer | 63 |
| `s1-format-pdf` | PDF exporter | 21 |
| `s1-format-txt` | Plain text reader/writer | 25 |
| `s1-convert` | Format conversion (incl. DOC text extraction) | 15 |
| `s1-layout` | Page layout engine (pagination, line breaking) | 38 |
| `s1-text` | Text shaping, fonts, Unicode (pure Rust) | 39 |
| `s1-crdt` | CRDT algorithms for collaborative editing | 171 |
| `ffi/wasm` | WASM bindings (wasm-bindgen) | 12 |
| `ffi/c` | C FFI bindings (opaque handles) | 10 |

## Format Support

| Feature | DOCX | ODT | PDF | TXT | DOC |
|---|---|---|---|---|---|
| Read | Full | Full | -- | Full | Text only |
| Write | Full | Full | Export | Full | -- |
| Round-trip | Yes | Yes | -- | Yes | -- |
| Tables | Yes | Yes | Yes | Tab-separated | -- |
| Images | Yes | Yes | Yes | -- | -- |
| Lists | Yes | Yes | -- | -- | -- |
| Styles | Yes | Yes | -- | -- | -- |
| Headers/Footers | Yes | -- | Yes | -- | -- |
| Hyperlinks | Yes | -- | Yes | -- | -- |
| Comments | Yes | -- | -- | -- | -- |
| Metadata | Yes | Yes | Yes | -- | -- |

## Documentation

- [Architecture](docs/ARCHITECTURE.md) -- System design and decisions
- [Specification](docs/SPECIFICATION.md) -- Detailed technical spec
- [Roadmap](docs/ROADMAP.md) -- Development phases and milestones
- [API Design](docs/API_DESIGN.md) -- Public API surface and examples
- [Dependencies](docs/DEPENDENCIES.md) -- External libraries and rationale

## Building

```bash
# Build
cargo build

# Test
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --check
```

No system libraries required. All dependencies are pure Rust.

## Roadmap

| Phase | Status | Focus |
|---|---|---|
| 1. Foundation | Complete | Document model, operations, TXT, basic DOCX |
| 2. Rich Documents | Complete | Tables, images, lists, full DOCX, ODT |
| 3. Layout & Export | Complete | Text shaping, page layout, PDF export |
| 4. Collaboration | Complete | Fugue CRDT, tree CRDT, awareness, serialization |
| 5. Production | In Progress | WASM, C FFI, hardening, docs, release |

See [ROADMAP.md](docs/ROADMAP.md) for detailed milestones.

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
