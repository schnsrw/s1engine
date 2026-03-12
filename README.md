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
use s1engine::{DocumentBuilder, Format};

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

## Format Support Matrix

Detailed per-feature support across all document formats. Classification key:

- **Full** -- read + write with round-trip fidelity
- **Read** -- read only (data imported but not written back in this format)
- **Write** -- write/export only
- **Partial** -- some aspects work (see notes)
- **Lossy** -- data survives but loses fidelity
- **--** -- not supported

### General

| Capability | DOCX | ODT | PDF | TXT | DOC (legacy) |
|---|---|---|---|---|---|
| Read | Yes | Yes | -- | Yes | Text only |
| Write | Yes | Yes | Export only | Yes | -- |
| Round-trip | Yes | Yes | -- | Yes | -- |

### Block-Level Content

| Feature | DOCX | ODT | PDF | TXT | DOC |
|---|---|---|---|---|---|
| Paragraphs (text) | Full | Full | Write | Lossy | Partial |
| Paragraph alignment | Full | Full | Write | -- | -- |
| Paragraph spacing | Full | Full | Write | -- | -- |
| Paragraph indent | Full | Full | Write | -- | -- |
| Headings / styles | Full | Full | Write | -- | -- |
| Tables (basic) | Full | Full | Write | Lossy (tab-separated) | -- |
| Tables (merged cells) | Full | Full | Write | -- | -- |
| Tables (nested) | Full | -- | Write | -- | -- |
| Lists (bullet) | Full | Full | -- | -- | -- |
| Lists (numbered) | Full | Full | -- | -- | -- |
| Lists (multilevel) | Full | Full | -- | -- | -- |
| Page breaks | Full | Full | Write | -- | -- |
| Sections (page size, margins) | Full | -- | Write | -- | -- |
| Sections (orientation) | Full | -- | Write | -- | -- |
| Headers / footers | Full | -- | Write | -- | -- |

### Inline / Character-Level Content

| Feature | DOCX | ODT | PDF | TXT | DOC |
|---|---|---|---|---|---|
| Bold / italic | Full | Full | Write | -- | -- |
| Underline | Full | Full | Write | -- | -- |
| Font family | Full | Full | Write | -- | -- |
| Font size | Full | Full | Write | -- | -- |
| Font color | Full | Full | Write | -- | -- |
| Strikethrough | Full | Full | -- | -- | -- |
| Highlight color | Full | Full | -- | -- | -- |
| Superscript / subscript | Full | -- | -- | -- | -- |
| Character spacing | Full | -- | -- | -- | -- |
| Line breaks | Full | Full | Write | -- | -- |
| Tab characters | Full | Full | Write | Lossy | Partial |
| Images (inline) | Full | Full | Write | -- | -- |
| Images (floating/anchored) | Read | -- | -- | -- | -- |
| Hyperlinks (external) | Full | Partial (text only, URL lost) | Write | -- | -- |
| Hyperlinks (internal anchor) | Full | -- | -- | -- | -- |
| Bookmarks | Full | -- | Write | -- | -- |

### Document-Level Features

| Feature | DOCX | ODT | PDF | TXT | DOC |
|---|---|---|---|---|---|
| Metadata (title, author) | Full | Full | Write | -- | -- |
| Comments | Full | -- | -- | -- | -- |
| Tab stops (custom positions) | Full | -- | -- | -- | -- |
| Paragraph borders | Full | -- | -- | -- | -- |
| Paragraph shading | Full | -- | -- | -- | -- |
| Style inheritance | Full | Full | -- | -- | -- |

### Notes

- **DOCX**: Most complete format support. Floating images are read into the model but written back as inline.
- **ODT**: Hyperlink elements (`text:a`) are parsed but the URL is not preserved -- only the visible text survives. No support for bookmarks, comments, headers/footers, or sections.
- **PDF**: Export-only path: DocumentModel passes through the layout engine (`s1-layout`) before PDF generation. Supports font embedding with subsetting, table borders, image embedding, hyperlink annotations, and document outline (bookmarks).
- **TXT**: Inherently lossy -- all formatting, styles, and structure are stripped. Tables render as tab-separated columns. Encoding detection supports UTF-8, UTF-16 LE/BE (BOM), and Latin-1 fallback.
- **DOC**: Legacy binary format read via heuristic text extraction (`s1-convert`). Only paragraph text and tabs are extracted. No formatting, tables, images, or other structures.

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
