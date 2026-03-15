# s1engine

A modular document engine SDK built in pure Rust. Read, write, edit, and convert documents across DOCX, ODT, PDF, TXT, and Markdown formats — with CRDT-based real-time collaboration, a page layout engine, and a fully-featured web editor.

## Highlights

- **Multi-format** — DOCX, ODT, PDF, TXT, Markdown, and legacy DOC (read)
- **Pure Rust** — Zero C/C++ dependencies. Compiles to native, WASM, and C FFI
- **Collaborative** — Fugue CRDT for real-time multi-user editing with conflict resolution
- **Layout engine** — Pagination, text shaping (rustybuzz), font subsetting, PDF export
- **Web editor included** — Production-grade browser editor (Folio) with toolbar, comments, track changes, and PDF viewer
- **Embeddable** — Use as a Rust library, WASM module, or C shared library

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
|  format-md                                         |
|----------------------------------------------------|
|                s1-text (Pure Rust)                  |
|        rustybuzz  ttf-parser  fontdb               |
+----------------------------------------------------+
```

## Quick Start

### As a Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
s1engine = "1.0"

# Optional features
# s1engine = { version = "1.0", features = ["pdf", "crdt", "convert"] }
```

Open and read a document:

```rust
use s1engine::Engine;

let engine = Engine::new();
let data = std::fs::read("report.docx")?;
let doc = engine.open(&data)?;

println!("{}", doc.to_plain_text());
println!("Title: {:?}", doc.metadata().title);
```

Create a document programmatically:

```rust
use s1engine::{DocumentBuilder, Format};

let doc = DocumentBuilder::new()
    .title("Quarterly Report")
    .author("Engineering")
    .heading(1, "Introduction")
    .paragraph(|p| {
        p.text("Built with ")
         .bold("s1engine")
         .text(" — a document SDK in Rust.")
    })
    .table(|t| {
        t.row(|r| r.cell("Metric").cell("Value"))
         .row(|r| r.cell("Users").cell("15,000"))
    })
    .build();

let docx = doc.export(Format::Docx)?;
let pdf = doc.export(Format::Pdf)?;  // requires "pdf" feature
```

Convert between formats:

```rust
let engine = Engine::new();
let doc = engine.open_file("input.docx")?;
std::fs::write("output.odt", doc.export(Format::Odt)?)?;
```

### Feature Flags

| Feature | Description | Default |
|---|---|---|
| `docx` | DOCX (OOXML) read/write | Yes |
| `odt` | ODT (ODF) read/write | Yes |
| `txt` | Plain text read/write | Yes |
| `md` | Markdown read/write (GFM tables) | Yes |
| `pdf` | PDF export with font embedding | No |
| `convert` | Format conversion pipelines | No |
| `doc-legacy` | Legacy DOC binary parsing | No |
| `crdt` | CRDT collaboration primitives | No |

## Web Editor (Folio)

s1engine ships with **Folio**, a production-grade document editor that runs in the browser via WASM.

### Features

- Full WYSIWYG editing with multi-page layout
- Toolbar with formatting, styles, tables, images, comments
- Real-time collaboration via WebSocket relay
- Track changes with accept/reject
- PDF viewer with annotations (highlight, comment, draw, text)
- Export to DOCX, ODT, PDF, TXT, Markdown
- Dark mode, keyboard shortcuts, find & replace
- Drag-and-drop file opening

### Running the Editor

```bash
# Prerequisites: Rust, wasm-pack, Node.js 18+

# Build WASM bindings
make wasm

# Start development server
cd editor && npm install && npm run dev
```

Open `http://localhost:3000` in your browser.

### Docker

```bash
# Build and run with Docker
make docker-build
make docker-run

# Or use Docker Compose
docker compose up
```

The editor is served at `http://localhost:8787`.

## Crate Structure

| Crate | Description |
|---|---|
| `s1engine` | Facade — high-level public API |
| `s1-model` | Core document model (zero external deps) |
| `s1-ops` | Operations, transactions, undo/redo |
| `s1-format-docx` | DOCX (OOXML) reader/writer |
| `s1-format-odt` | ODT (ODF) reader/writer |
| `s1-format-md` | Markdown reader/writer |
| `s1-format-pdf` | PDF export + editing (via lopdf) |
| `s1-format-txt` | Plain text reader/writer |
| `s1-convert` | Format conversion (DOC binary + cross-format) |
| `s1-layout` | Page layout, pagination, text shaping |
| `s1-text` | Font loading, shaping, Unicode (pure Rust) |
| `s1-crdt` | CRDT algorithms for collaboration |
| `ffi/wasm` | WASM bindings (wasm-bindgen) |
| `ffi/c` | C FFI bindings (cbindgen) |

## Building from Source

### Prerequisites

- Rust 1.75+ (`rustup install stable`)
- For WASM: `wasm-pack` (`cargo install wasm-pack`)
- For editor: Node.js 18+ and npm
- For Docker: Docker 20+

### Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

### Makefile Targets

```bash
make build          # Build all crates (debug)
make build-release  # Build all crates (release)
make test           # Run all tests
make clippy         # Lint with clippy
make fmt            # Format code
make check          # fmt + clippy + tests
make wasm           # Build WASM bindings (debug)
make wasm-release   # Build WASM bindings (release)
make demo           # Build WASM + start editor
make docker-build   # Build Docker image
make docker-run     # Run Docker container
make clean          # Clean build artifacts
```

## Format Support

| Feature | DOCX | ODT | Markdown | PDF | TXT | DOC |
|---|---|---|---|---|---|---|
| Read | Full | Full | Full | View* | Full | Partial |
| Write | Full | Full | Full | Export | Full | — |
| Round-trip | Full | Full | Partial | — | Full | — |
| Paragraphs | Full | Full | Full | Export | Lossy | Full |
| Tables | Full | Full | GFM | Export | Tab-sep | Partial |
| Images | Full | Full | — | Export | — | — |
| Lists | Full | Full | Full | — | Markers | — |
| Styles | Full | Full | — | Export | — | Partial |
| Comments | Full | Full | — | — | — | — |
| Headers/Footers | Full | Full | — | Export | — | — |
| Hyperlinks | Full | Full | Full | Export | — | — |
| Track Changes | Full | — | — | — | — | — |

*PDF viewing is available in the Folio web editor via PDF.js integration.

## Documentation

| Document | Description |
|---|---|
| [Architecture](docs/ARCHITECTURE.md) | System design, crate structure, core decisions |
| [Specification](docs/SPECIFICATION.md) | Detailed technical spec for every module |
| [API Design](docs/API_DESIGN.md) | Public API surface, feature flags, examples |
| [Roadmap](docs/ROADMAP.md) | Development phases and milestones |
| [Dependencies](docs/DEPENDENCIES.md) | External libraries with rationale |
| [WASM Design](docs/WASM_DESIGN.md) | WASM bindings, rendering modes, font handling |
| [Contributing](CONTRIBUTING.md) | How to contribute to the project |
| [Changelog](CHANGELOG.md) | Release history |

## Contributing

We welcome contributions. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Quick overview:**

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes (follow the coding conventions in CLAUDE.md)
4. Run `make check` to verify tests, clippy, and formatting
5. Submit a pull request

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

## Acknowledgments

s1engine uses these excellent pure-Rust libraries:

- [rustybuzz](https://github.com/nicholasgasior/rustybuzz) — Text shaping (HarfBuzz port)
- [ttf-parser](https://github.com/nicholasgasior/ttf-parser) — Font parsing
- [fontdb](https://github.com/nicholasgasior/fontdb) — Font discovery
- [pdf-writer](https://github.com/nicholasgasior/pdf-writer) — PDF generation
- [lopdf](https://github.com/nicholasgasior/lopdf) — PDF reading/editing
- [quick-xml](https://github.com/nicholasgasior/quick-xml) — XML parsing
- [pulldown-cmark](https://github.com/nicholasgasior/pulldown-cmark) — Markdown parsing
