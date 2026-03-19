# Competitive Analysis

## Landscape Overview

The document editing space has several categories of solutions. s1engine competes across multiple categories because of its layered architecture.

## Detailed Comparison

### Category 1: Full Document Suites

#### LibreOffice Online (Collabora CODE)
- **What**: LibreOffice wrapped in a web container, rendered server-side via tile rendering
- **License**: MPL 2.0 (LibreOffice), proprietary (Collabora)
- **Strengths**: Best format fidelity (decades of OOXML/ODF work), full feature set
- **Weaknesses**: Monolithic C++, runs as separate Docker container, not embeddable as a component, heavy resource usage (each user = process), UI feels dated
- **Integration model**: iFrame embed with WOPI protocol
- **s1engine advantage**: Native embedding (not iFrame), client-side rendering (no server per user), modern UI, modular (use only what you need)

#### OnlyOffice
- **What**: Full office suite with web editor, document server
- **License**: AGPL v3 (forces open-sourcing) or commercial
- **Strengths**: Good format support, collaboration, familiar UI
- **Weaknesses**: AGPL license is toxic for commercial products, heavy JS client (~5MB), tightly coupled architecture, must run their document server
- **Integration model**: iFrame embed with WOPI or their JS API
- **s1engine advantage**: AGPL-3.0 with commercial dual-license (same model as OnlyOffice), lightweight WASM (~2MB), embeddable as component (not iFrame), no document server required for basic editing

#### Google Docs
- **What**: Proprietary cloud document editor
- **License**: Proprietary, no self-hosting
- **Strengths**: Best-in-class collaboration, ubiquitous, free tier
- **Weaknesses**: Vendor lock-in, no self-hosting, limited API (can't embed editor), no white-labeling, data sovereignty issues
- **Integration model**: iFrame (view-only) or API (limited)
- **s1engine advantage**: Self-hostable, full API access, white-labelable, no vendor lock-in, data stays in your infrastructure

### Category 2: Rich Text Editors (HTML-based)

#### ProseMirror / TipTap
- **What**: Schema-based rich text editor framework (ProseMirror) with batteries-included wrapper (TipTap)
- **License**: MIT (ProseMirror), open-core (TipTap — collab is paid)
- **Strengths**: Excellent editing model, great plugin system, React/Vue components, large community
- **Weaknesses**: HTML-only (no DOCX/PDF), no pagination/page layout, no document model (just rich text), collaboration requires their paid cloud
- **Integration model**: npm package, component-level
- **s1engine advantage**: Real document model (DOCX, PDF, pages), format I/O, pagination, print-ready layout

#### CKEditor 5
- **What**: Commercial rich text editor with collaboration
- **License**: GPL or commercial (collaboration is paid SaaS only)
- **Strengths**: Mature, good toolbar/plugin system, pagination plugin, Word import/export (premium)
- **Weaknesses**: DOCX support is premium/paid, pagination is premium, collaboration requires their cloud, GPL for open-source version
- **Integration model**: npm package, component-level
- **s1engine advantage**: All format support in open-source, self-hosted collaboration, no SaaS dependency, native document model (not HTML retrofitted)

#### Quill / Draft.js / Slate
- **What**: React-oriented rich text editor frameworks
- **License**: MIT / BSD
- **Strengths**: Easy to embed in React apps, simple API
- **Weaknesses**: No document format support, no pagination, limited formatting, collaboration is DIY, not designed for documents (designed for comments/posts)
- **Integration model**: npm/React component
- **s1engine advantage**: Full document editing (not just rich text), format support, pagination, collaboration built-in

### Category 3: Document Processing Libraries

#### Apache POI (Java)
- **What**: Java library for reading/writing Office formats
- **License**: Apache 2.0
- **Strengths**: Mature, comprehensive OOXML support, widely used in enterprise
- **Weaknesses**: Java-only, no editing/UI, no collaboration, no layout engine, heavy
- **s1engine advantage**: Multi-platform (Rust + WASM + C), includes editor UI, collaboration, layout, cross-language

#### python-docx / docx4j
- **What**: Language-specific DOCX libraries
- **License**: MIT (python-docx), Apache 2.0 (docx4j)
- **Strengths**: Simple API for their language
- **Weaknesses**: Single language, read/write only, no layout, no collaboration
- **s1engine advantage**: Full engine (not just format I/O), cross-platform, includes all layers

#### Pandoc
- **What**: Universal document converter (Haskell)
- **License**: GPL
- **Strengths**: Best-in-class format conversion, supports 40+ formats
- **Weaknesses**: Conversion only (no editing), GPL license, no layout, loses formatting details
- **s1engine advantage**: Editing + conversion + layout, permissive license, embeddable

### Category 4: Emerging Alternatives

#### Typst
- **What**: Modern typesetting system (Rust)
- **License**: Apache 2.0
- **Strengths**: Beautiful output, fast compilation, Rust-based, growing community
- **Weaknesses**: Typesetting only (not a document editor), no DOCX/ODT I/O, requires learning Typst markup, not WYSIWYG
- **s1engine advantage**: WYSIWYG editing, format I/O (DOCX/ODT), embeddable editor, collaboration

#### BlockNote / Notion-like editors
- **What**: Block-based editors inspired by Notion
- **License**: Various (MPL, MIT)
- **Strengths**: Modern UX, block model, good for knowledge bases
- **Weaknesses**: Not document editors (no pages, no DOCX, no print layout), different paradigm
- **s1engine advantage**: Traditional document model (pages, headers, footers, print layout), format compatibility

## Competitive Matrix

| Feature | s1engine | LibreOffice | OnlyOffice | TipTap | CKEditor | Google Docs |
|---------|----------|-------------|------------|--------|----------|-------------|
| DOCX Read/Write | Yes (OSS) | Yes | Yes | No | Paid | API only |
| ODT Read/Write | Yes (OSS) | Yes | Limited | No | No | No |
| PDF Export | Yes (OSS) | Yes | Yes | No | No | Yes |
| Pagination/Layout | Yes (OSS) | Yes | Yes | No | Paid | Yes |
| Real-time Collab | Yes (OSS) | Yes (paid) | Yes (AGPL) | Paid | Paid | Yes |
| Self-hostable | Yes | Yes | Yes (AGPL) | Partial | No (collab) | No |
| Embeddable (component) | Yes | No (iFrame) | No (iFrame) | Yes | Yes | No |
| White-labelable | Yes | Limited | Yes (paid) | Yes | Yes (paid) | No |
| Open-source license | AGPL/commercial | MPL/paid | AGPL/paid | MIT/paid | GPL/paid | Proprietary |
| WASM support | Yes | No | No | N/A | N/A | No |
| Pure Rust (no C deps) | Yes | No (C++) | No (C++) | N/A | N/A | N/A |
| Bundle size (editor) | ~2MB WASM | N/A (server) | ~5MB JS | ~500KB | ~1MB | N/A |
| Server required | No (optional) | Yes | Yes | No (optional) | No (optional) | Yes |

## Strategic Positioning

### Where s1engine Wins

1. **"I need DOCX/PDF in my app"** — No other embeddable editor handles real document formats in open-source
2. **"I need self-hosted collaboration"** — OnlyOffice forces AGPL, CKEditor/TipTap collab is SaaS-only
3. **"I need to white-label"** — Google Docs can't, LibreOffice is clunky, OnlyOffice charges heavily
4. **"I want lightweight embedding"** — 2MB WASM vs iFrame containers or 5MB JS bundles
5. **"I need server-side document processing + client editor"** — Same engine runs on both (Rust native + WASM)

### Where s1engine Doesn't Win (Yet)

1. **Format fidelity** — LibreOffice has 20+ years of OOXML work. We cover ~90%, they cover ~99%
2. **Feature completeness** — Track changes UI, comments UI, mail merge, macros — these take time
3. **Ecosystem size** — CKEditor/TipTap have large plugin ecosystems. We're starting from zero
4. **Enterprise trust** — New project vs established players. Mitigated by permissive license and test coverage
5. **Mobile experience** — No native mobile SDKs (responsive web only initially)

### Differentiation Summary

**s1engine is the only solution that combines**: real document format support (DOCX/ODT/PDF) + embeddable component (not iFrame) + self-hosted collaboration + permissive open-source license + pure Rust/WASM stack.

No other solution offers all five together.
