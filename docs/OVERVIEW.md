# Rudra Code

## Vision

A modern, modular document engine SDK built in Rust (with C++ interop where necessary) that serves as the foundational library for building document editors, converters, and real-time collaborative editing applications.

## Goals

- **Library-first**: Embeddable SDK — consumers build their own editors on top
- **Format support**: DOCX (primary), ODT, PDF (export), TXT
- **DOC support**: Via DOC-to-DOCX conversion pipeline (not native editing)
- **CRDT-ready architecture**: Document model designed from day 1 to support collaborative editing
- **Cross-platform**: Native (macOS/Linux/Windows), WASM (browser), C FFI (embedding in any language)
- **Modular**: Use only the crates you need — don't pay for what you don't use
- **Clean API**: Well-documented, ergonomic public API for programmatic document manipulation

## Non-Goals (Initial Scope)

- Full GUI editor — consumers build their own UI on top of this engine
- PDF editing/annotation — PDF is export-only
- Spreadsheet or presentation support — this is a document (word-processing) engine
- Full OOXML spec compliance — pragmatic subset covering ~90% of real-world documents
- Real-time collaboration server — the engine provides CRDT primitives, not the networking layer
- Backward compatibility with every legacy DOC quirk

## Why Build This?

### Problems with Existing Engines

| Approach | Problem |
|---|---|
| **Typesetting tools** | No DOCX/ODT, not an editing engine |
| **Conversion tools** | Conversion only, not an editing engine, no layout |

### What s1engine Offers

- **Modular** — use only the crates you need (just DOCX parsing? just PDF export? just the model?)
- **Safe** — Rust's memory safety eliminates entire classes of bugs that plague C++ document engines
- **Modern** — CRDT-ready architecture designed for collaborative editing from the ground up
- **Embeddable** — library, not an application. Embed via Rust API, C FFI, or WASM
- **Portable** — same engine runs on server, desktop, and browser (via WASM)
- **Testable** — pure library with no UI dependencies, easy to unit test and fuzz

## Target Users

1. **Application developers** building document editors (web, desktop, mobile)
2. **Backend services** that need to generate, convert, or process documents
3. **Platform teams** building collaborative editing products (self-hosted document editors)

## Relationship to Larger Product

s1engine is a standalone, independently usable SDK. It may be integrated into rdrive/melp products but has no dependency on them. It is designed to be useful to any developer building document-related software.
