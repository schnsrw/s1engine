# s1engine

s1engine is the open-source document engine that lets any product add Google Docs-grade editing, collaboration, and format conversion — embeddable in hours, not months.

## What is s1engine?

A modular Rust SDK for document workflows. It reads, writes, edits, and converts documents across DOCX, ODT, PDF, TXT, and Markdown formats — with CRDT-based collaboration, a page layout engine, and a production web editor.

## Key Features

- **Multi-format**: DOCX, ODT, PDF, TXT, Markdown, and legacy DOC (read)
- **Pure Rust**: Zero C/C++ dependencies. Compiles to native, WASM, and C FFI
- **Collaborative**: Fugue CRDT for multi-user editing with conflict resolution
- **Layout engine**: Pagination, text shaping, font subsetting, PDF export
- **Web editor**: Production-grade browser editor with toolbar, comments, track changes
- **Embeddable**: Use as a Rust library, WASM module, C shared library, or Docker container

## Architecture

```
Layer 1: Core Engine (Rust)     — Format parsing, document model, operations, CRDT
Layer 2: Server API (Rust)      — REST endpoints, storage backends, auth (coming soon)
Layer 3: Client SDK (JS/TS)     — Embeddable editor, headless API (coming soon)
Layer 4: Platform Features      — White-labeling, plugins, webhooks (coming soon)
```

Each layer is independently usable:
- **Just need format conversion?** Use Layer 1 (Rust library or WASM)
- **Need a document API?** Use Layers 1+2 (server)
- **Need an embeddable editor?** Use Layers 1+2+3 (SDK)
- **Need a full branded product?** Use all 4 layers

## Quick Links

- [Quick Start](./getting-started/quick-start.md)
- [GitHub Repository](https://github.com/schnsrw/s1engine)
- [API Reference](./api/rust.md)
- [Contributing](./contributing/setup.md)
