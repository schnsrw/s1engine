# Rudra Code

## Vision

A modern, modular document and spreadsheet engine SDK built in pure Rust that serves as the foundational library for building office applications, format converters, and real-time collaborative editing platforms. Rudra Code powers the **Rudra Office** suite — a self-hostable, white-labelable alternative to Google Workspace and Microsoft Office Online.

## Goals

- **Library-first**: Embeddable SDK — consumers build their own editors on top
- **Document format support**: DOCX (primary), ODT, PDF (export + viewing), TXT, Markdown
- **Spreadsheet format support**: XLSX, ODS, CSV with a 60+ function formula engine
- **DOC support**: Via DOC-to-DOCX conversion pipeline (not native editing)
- **CRDT-ready architecture**: Document model designed from day 1 to support collaborative editing
- **Cross-platform**: Native (macOS/Linux/Windows), WASM (browser), C FFI (embedding in any language)
- **Modular**: Use only the crates you need — don't pay for what you don't use
- **Clean API**: Well-documented, ergonomic public API for programmatic document manipulation
- **Self-hostable**: Single Docker image deployment with white-labeling support
- **AI-ready**: Optional AI integration sidecar (llama.cpp with Qwen2.5-3B) for document assistance

## Non-Goals

- Full OOXML spec compliance — pragmatic subset covering ~90% of real-world documents
- Real-time collaboration server — the engine provides CRDT primitives, not the networking layer
- Backward compatibility with every legacy DOC quirk
- Presentation/slides support (deferred, may be added in a future phase)

## Architecture Layers

```
Layer 1: Core Engine (Rust)     — Format I/O, document model, operations, CRDT
Layer 2: Server (Rust/Axum)     — REST API, WebSocket collab, storage, auth, admin
Layer 3: Client SDK (JS/TS)     — Embeddable editor, headless API, React/Vue components
Layer 4: Platform Features      — White-labeling, integration mode, webhooks, AI sidecar
```

Each layer is independently usable:

- **Just need format conversion?** Use Layer 1 as a Rust library or WASM module
- **Need a document API?** Use Layers 1+2 — deploy the server via Docker
- **Need an embeddable editor?** Use Layers 1+2+3 — npm install and embed
- **Need a full branded product?** Use all 4 layers — white-label and self-host

## Why Build This?

### Problems with Existing Engines

| Approach | Problem |
|---|---|
| **Typesetting tools** | No DOCX/ODT, not an editing engine |
| **Conversion tools** | Conversion only, not an editing engine, no layout |
| **Existing office suites** | Not embeddable, not modular, difficult to self-host |

### What s1engine Offers

- **Modular** — use only the crates you need (just DOCX parsing? just PDF export? just spreadsheets?)
- **Safe** — Rust's memory safety eliminates entire classes of bugs that plague C++ document engines
- **Modern** — CRDT-ready architecture designed for collaborative editing from the ground up
- **Full office suite** — documents, spreadsheets, and PDF viewing/annotation in one engine
- **Embeddable** — library, not an application. Embed via Rust API, C FFI, or WASM
- **Portable** — same engine runs on server, desktop, and browser (via WASM)
- **Testable** — pure library with no UI dependencies, easy to unit test and fuzz (1,390+ tests)

## Target Users

1. **Application developers** building document or spreadsheet editors (web, desktop, mobile)
2. **Backend services** that need to generate, convert, or process documents and spreadsheets
3. **Platform teams** building collaborative editing products (self-hosted office suites)
4. **Enterprises** looking for a self-hosted, white-labelable alternative to cloud office suites

## Relationship to Larger Product

s1engine is a standalone, independently usable SDK. It may be integrated into rdrive/melp products but has no dependency on them. It is designed to be useful to any developer building document-related or spreadsheet-related software.
