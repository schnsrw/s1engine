# Market Positioning

## Landscape Overview

The document editing space spans several categories of solutions. s1engine's layered architecture allows it to serve use cases across all of them.

## s1engine Strengths by Category

### Category 1: Full Document Editing
s1engine provides a complete document editing experience — DOCX, ODT, PDF, and more — with native WASM embedding, client-side rendering (no server per user), and a modern UI. The modular architecture means consumers use only what they need.

### Category 2: Rich Text Editing
Unlike HTML-only editors, s1engine has a real document model with format I/O (DOCX, PDF), pagination, and print-ready layout. All format support is open-source, collaboration is self-hosted, and the document model is native (not HTML retrofitted).

### Category 3: Document Processing
s1engine is a full engine — not just format I/O. It is cross-platform (Rust + WASM + C), includes an editor UI, collaboration, and a layout engine. It provides editing + conversion + layout in a single embeddable package.

### Category 4: Modern Typesetting & Block Editors
s1engine offers WYSIWYG editing with traditional document model support (pages, headers, footers, print layout) and full format compatibility (DOCX/ODT), making it suitable for professional document workflows.

## Key Capabilities

| Feature | s1engine |
|---------|----------|
| DOCX Read/Write | Yes (open-source) |
| ODT Read/Write | Yes (open-source) |
| PDF Export | Yes (open-source) |
| Pagination/Layout | Yes (open-source) |
| Real-time Collab | Yes (open-source, self-hosted) |
| Self-hostable | Yes |
| Embeddable (component) | Yes (not iFrame) |
| White-labelable | Yes |
| Open-source license | AGPL-3.0 / commercial dual-license |
| WASM support | Yes |
| Pure Rust (no C deps) | Yes |
| Bundle size (editor) | ~2MB WASM |
| Server required | No (optional) |

## Strategic Positioning

### Where s1engine Excels

1. **"I need DOCX/PDF in my app"** — Embeddable editor with real document format support, fully open-source
2. **"I need self-hosted collaboration"** — CRDT-based collaboration, no external SaaS dependency
3. **"I need to white-label"** — Full white-labeling support, custom branding, custom domain
4. **"I want lightweight embedding"** — 2MB WASM module, embeddable as a component
5. **"I need server-side document processing + client editor"** — Same engine runs on both (Rust native + WASM)

### Areas for Growth

1. **Format fidelity** — Covering ~90% of OOXML features; continuing to improve
2. **Feature completeness** — Track changes UI, comments UI, mail merge, macros — progressing steadily
3. **Ecosystem size** — Plugin ecosystem is growing
4. **Enterprise trust** — Mitigated by comprehensive test suite (1,390+ tests) and strong documentation
5. **Mobile experience** — No native mobile SDKs yet (responsive web only initially)

### Differentiation Summary

**s1engine is the only solution that combines**: real document format support (DOCX/ODT/PDF) + embeddable component (not iFrame) + self-hosted collaboration + open-source licensing + pure Rust/WASM stack.
