# Rudra Code

**The open-source document engine that lets any product add professional editing, real-time collaboration, and format conversion — embeddable in hours, not months.**

---

## What is Rudra Code?

Rudra Code is a modular, pure-Rust SDK for document workflows. It reads, writes, edits, and converts documents across **DOCX, ODT, PDF, TXT, Markdown, XLSX, ODS, and CSV** formats — with CRDT-based real-time collaboration, a page layout engine, and a production-ready web editor.

| Capability | Description |
|---|---|
| **Multi-format** | DOCX, ODT, PDF, TXT, Markdown, XLSX, ODS, CSV, legacy DOC |
| **Pure Rust** | Zero C/C++ dependencies. Compiles to native, WASM, and C FFI |
| **Collaborative** | Fugue CRDT for conflict-free multi-user editing |
| **Layout engine** | Pagination, text shaping (rustybuzz), font subsetting, PDF export |
| **Web editor** | Production-grade browser editor with toolbar, comments, track changes |
| **Spreadsheet** | Canvas-based spreadsheet with 60+ formulas, charts, and real-time collab |
| **Embeddable** | Rust library, WASM module, C shared library, npm packages, or Docker |

## Architecture

```
Layer 1: Core Engine (Rust)     — Format I/O, document model, operations, CRDT
Layer 2: Server (Rust/Axum)     — REST API, WebSocket collab, storage, auth, admin
Layer 3: Client SDK (JS/TS)     — Embeddable editor, headless API, React/Vue components
Layer 4: Platform Features      — White-labeling, integration mode, webhooks
```

Each layer is independently usable:

- **Just need format conversion?** Use Layer 1 as a Rust library or WASM module
- **Need a document API?** Use Layers 1+2 — deploy the server via Docker
- **Need an embeddable editor?** Use Layers 1+2+3 — npm install and embed
- **Need a full branded product?** Use all 4 layers — white-label and self-host

## Quick Start

**Docker (fastest):**
```bash
docker run -p 8080:8080 rudra/server
# Editor at http://localhost:8080
# API at http://localhost:8080/api/v1
```

**Rust library:**
```bash
cargo add s1engine --features full
```

**npm (browser):**
```bash
npm install @rudra/sdk @rudra/wasm
```

> **Tip:** See the [Quick Start guide](./getting-started/quick-start.md) for a complete walkthrough, or jump to [Docker deployment](./getting-started/docker.md) for self-hosting.

## Distribution

| I want to... | Install |
|---|---|
| Process documents in Rust | `cargo add s1engine` |
| Process documents in JS (no UI) | `npm install @rudra/sdk` |
| Embed an editor in React | `npm install @rudra/react` |
| Embed an editor in Vue | `npm install @rudra/vue` |
| Run a document API server | `docker run rudra/server` |
| Self-host the full platform | `docker compose up` |
| Convert DOCX to PDF (CLI) | Download from [GitHub Releases](https://github.com/Rudra-Office/Rudra-Editor/releases) |

## Crate Structure

```
crates/
  s1-model/          Core document model (zero dependencies)
  s1-ops/            Operations, transactions, undo/redo
  s1-format-docx/    DOCX (OOXML) reader/writer
  s1-format-odt/     ODT (ODF) reader/writer
  s1-format-pdf/     PDF export
  s1-format-txt/     Plain text reader/writer
  s1-format-xlsx/    XLSX/ODS/CSV spreadsheet reader/writer
  s1-convert/        Format conversion pipelines
  s1-crdt/           Fugue CRDT for collaboration
  s1-layout/         Page layout, pagination, line breaking
  s1-text/           Text shaping (rustybuzz), font discovery
  s1engine/          Facade crate — high-level public API
```

## Links

- [GitHub Repository](https://github.com/Rudra-Office/Rudra-Editor)
- [Quick Start](./getting-started/quick-start.md)
- [API Reference](./api/rust.md)
- [Self-Hosting Guide](./guides/self-hosting.md)
- [Contributing](./contributing/setup.md)

## License

AGPL-3.0-or-later. Commercial dual-licensing available — [contact us](https://github.com/Rudra-Office/Rudra-Editor) for details.
