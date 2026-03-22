# Rudra Code

[![CI](https://github.com/Rudra-Office/Rudra-Editor/actions/workflows/ci.yml/badge.svg)](https://github.com/Rudra-Office/Rudra-Editor/actions)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

A modular document and spreadsheet engine SDK built in pure Rust. Supports DOCX, ODT, PDF, TXT, Markdown, XLSX, ODS, and CSV with real-time CRDT collaboration, a page layout engine, and a production-ready web editor.

**1,390+ tests** · **Zero C/C++ dependencies** · **Pure Rust + WASM**

---

## Quick Start

**Docker** (fastest):
```bash
docker compose up
# Editor at http://localhost:8787
```

**Rust library**:
```bash
cargo add s1engine
```

```rust
use s1engine::Engine;

let engine = Engine::new();
let doc = engine.open(&std::fs::read("report.docx")?)?;
println!("{}", doc.to_plain_text());

// Convert to PDF
std::fs::write("output.pdf", doc.export(s1engine::Format::Pdf)?)?;
```

**npm (browser)**:
```bash
npm install @rudra/sdk @rudra/wasm
```

---

## What It Does

| Capability | Details |
|---|---|
| **Document formats** | Read/write DOCX, ODT, PDF (export), TXT, Markdown, legacy DOC (read) |
| **Spreadsheet formats** | Read/write XLSX, ODS, CSV with 60+ formula engine |
| **Collaboration** | Fugue CRDT for real-time multi-user editing (documents and spreadsheets) |
| **Layout engine** | Pagination, text shaping (rustybuzz), font subsetting, PDF export |
| **Web editor** | Browser-based document editor, spreadsheet editor, PDF viewer with annotations |
| **Charts** | Column, bar, line, area, pie, doughnut — rendered on canvas |
| **AI assistant** | Optional sidecar (llama.cpp + Qwen2.5-3B) for writing, grammar, formulas |
| **Self-hosting** | Single Docker image with white-labeling, admin panel, JWT auth |
| **Embeddable** | Rust library, WASM module, C FFI, npm packages, React/Vue components |

---

## Rudra Office

The web editor built on the engine. Documents, spreadsheets, and PDFs in one interface.

- **Document editor** — Paginated canvas with formatting toolbar, templates, tables, images, comments, track changes
- **Spreadsheet editor** — Canvas grid with formulas, charts, pivot tables, conditional formatting, data validation, cell comments
- **PDF viewer** — View, annotate (highlight, draw, comment, redact), and export
- **Collaboration** — Real-time co-editing with peer cursors, presence indicators, and "typing..." status
- **Multi-file tabs** — Open multiple documents/spreadsheets simultaneously, switch between them
- **Dark mode** — Full dark theme across all views

### Run locally

```bash
# Prerequisites: Rust, wasm-pack, Node.js 18+
make wasm && cd editor && npm install && npm run dev
```

### Docker

```bash
docker compose up        # Full stack: editor + server + collab
# Or standalone:
docker run -p 8787:8787 rudra/server
```

---

## Architecture

```
+----------------------------------------------------+
|                s1engine (facade)                    |
|----------------------------------------------------|
|  s1-ops        s1-layout        s1-convert         |
|  Operations    Page Layout      Format Conversion   |
|  Undo/Redo     Pagination       DOC -> DOCX         |
|----------------------------------------------------|
|  s1-crdt                 s1-model                   |
|  Collaborative           Core Document Model        |
|  Editing (Fugue)         (zero external deps)       |
|----------------------------------------------------|
|  format-docx   format-odt   format-pdf   format-txt|
|  format-md     format-xlsx  (XLSX/ODS/CSV)         |
|----------------------------------------------------|
|                s1-text (Pure Rust)                  |
|        rustybuzz · ttf-parser · fontdb             |
+----------------------------------------------------+
        |           |            |
    Rust API     C FFI       WASM/JS
```

### Crates

| Crate | Purpose |
|---|---|
| `s1engine` | Facade crate — high-level API |
| `s1-model` | Document tree model (zero deps) |
| `s1-ops` | Operations, transactions, undo/redo |
| `s1-crdt` | Fugue CRDT for collaboration |
| `s1-format-docx` | DOCX reader/writer |
| `s1-format-odt` | ODT reader/writer |
| `s1-format-pdf` | PDF export |
| `s1-format-txt` | Plain text reader/writer |
| `s1-format-md` | Markdown reader/writer |
| `s1-format-xlsx` | XLSX/ODS/CSV with formula engine |
| `s1-convert` | Cross-format conversion pipelines |
| `s1-layout` | Page layout, pagination, line breaking |
| `s1-text` | Text shaping, font loading (pure Rust) |

### Feature Flags

```toml
s1engine = "1.0"                                    # DOCX + ODT + TXT (default)
s1engine = { version = "1.0", features = ["full"] } # Everything
```

| Flag | What it adds | Default |
|---|---|---|
| `docx` | DOCX read/write | Yes |
| `odt` | ODT read/write | Yes |
| `txt` | Plain text | Yes |
| `md` | Markdown (GFM) | Yes |
| `xlsx` | XLSX/ODS/CSV + formulas | No |
| `pdf` | PDF export | No |
| `crdt` | Collaboration primitives | No |
| `convert` | Format conversion | No |
| `doc-legacy` | Legacy DOC parsing | No |
| `full` | All of the above | No |

---

## Format Support

### Documents

| | DOCX | ODT | Markdown | PDF | TXT | DOC |
|---|---|---|---|---|---|---|
| **Read** | Yes | Yes | Yes | View | Yes | Partial |
| **Write** | Yes | Yes | Yes | Export | Yes | -- |
| **Round-trip** | Yes | Yes | Partial | -- | Yes | -- |
| **Tables** | Yes | Yes | GFM | Export | Tab-sep | Partial |
| **Images** | Yes | Yes | -- | Export | -- | -- |
| **Styles** | Yes | Yes | -- | Export | -- | Partial |
| **Comments** | Yes | Yes | -- | -- | -- | -- |
| **Track Changes** | Yes | -- | -- | -- | -- | -- |

### Spreadsheets

| | XLSX | ODS | CSV |
|---|---|---|---|
| **Read/Write** | Yes | Yes | Yes |
| **Formulas** | 60+ functions | 60+ | -- |
| **Styles** | Yes | Yes | -- |
| **Merged Cells** | Yes | Yes | -- |
| **Charts** | UI | UI | -- |
| **Multi-sheet** | Yes | Yes | -- |
| **Frozen Panes** | Yes | Yes | -- |

---

## Building from Source

```bash
# Prerequisites: Rust 1.88+, wasm-pack (for WASM), Node.js 18+ (for editor)

cargo build --workspace          # Build all crates
cargo test --workspace           # Run 1,390+ tests
cargo clippy --workspace -- -D warnings  # Lint
```

See `make help` for all build targets.

---

## Documentation

- **[Documentation Site](https://rudra-office.github.io/Rudra-Editor/)** — Guides, API reference, deployment
- [Architecture](docs/ARCHITECTURE.md) — System design and crate structure
- [Specification](docs/SPECIFICATION.md) — Technical spec for every module
- [API Design](docs/API_DESIGN.md) — Public API surface and examples
- [Roadmap](docs/ROADMAP.md) — Development phases and milestones
- [Contributing](CONTRIBUTING.md) — How to contribute

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes and run `make check`
4. Submit a pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions and architecture rules.

---

## License

[AGPL-3.0-or-later](LICENSE). Commercial dual-licensing available for proprietary use — [contact us](https://github.com/Rudra-Office/Rudra-Editor/discussions).

## Acknowledgments

Built on pure-Rust libraries: [rustybuzz](https://github.com/RazrFalcon/rustybuzz) (text shaping), [ttf-parser](https://github.com/RazrFalcon/ttf-parser) (fonts), [fontdb](https://github.com/RazrFalcon/fontdb) (font discovery), [pdf-writer](https://github.com/typst/pdf-writer) (PDF), [lopdf](https://github.com/J-F-Liu/lopdf) (PDF editing), [quick-xml](https://github.com/tafia/quick-xml) (XML), [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) (Markdown).
