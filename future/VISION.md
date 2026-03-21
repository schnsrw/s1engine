# Product Vision

## One-Liner

s1engine is the open-source document engine that lets any product add professional-grade editing, collaboration, and format conversion — embeddable in hours, not months.

## The Problem

Building document editing into a product today is painful:

| Approach | Problem |
|----------|---------|
| Build from scratch | 2-3 years of engineering, font rendering, format parsing, collaboration |
| Monolithic office suites | Heavy C++ codebases, run as separate processes, not embeddable as a component |
| HTML-only editors (CKEditor / TinyMCE) | No real document model, no DOCX/PDF, no pagination |
| Proprietary cloud editors | Rate-limited, can't self-host, no white-labeling |
| Rich text frameworks (ProseMirror / TipTap) | Great text editors, but no document format support, no layout engine, no PDF |

**The gap**: There is no embeddable, open-source document engine that handles formats (DOCX, PDF), pagination, collaboration, AND provides a clean API for integration.

## The Solution

s1engine fills this gap with a modular, layered architecture:

```
Layer 1: Core Engine (Rust)       — Format parsing, document model, operations, CRDT
Layer 2: Server API (Rust)        — REST endpoints, storage backends, auth
Layer 3: Client SDK (JS/TS)       — Embeddable editor, headless API, React/Vue wrappers
Layer 4: Platform Features        — White-labeling, plugins, webhooks, analytics
```

Each layer is independently usable:
- **Just need format conversion?** Use Layer 1 (Rust library or WASM).
- **Need a document API for your backend?** Use Layers 1+2 (server).
- **Need an embeddable editor?** Use Layers 1+2+3 (SDK).
- **Need a full branded editor product?** Use all 4 layers.

## Target Personas

### 1. Application Developer ("Embed an editor")
**Profile**: Full-stack developer building a SaaS product (CRM, project management, knowledge base) that needs document editing.
**Need**: Drop-in editor component with `npm install` that works in React/Vue/Angular.
**Uses**: Client SDK (Layer 3) + optional Server API (Layer 2).

### 2. Backend Developer ("Process documents")
**Profile**: Backend engineer building document pipelines — generate invoices, convert uploads, extract text.
**Need**: REST API or Rust library for server-side document processing.
**Uses**: Core Engine (Layer 1) via Rust API or Server API (Layer 2).

### 3. Platform Builder ("Build a Docs product")
**Profile**: Startup/team building a document editing product.
**Need**: Full-featured editor with collaboration, white-labeling, custom branding.
**Uses**: All 4 layers, heavily customized.

### 4. Enterprise ("White-label for our customers")
**Profile**: Enterprise SaaS company that needs to offer document editing as a feature within their product, branded as theirs.
**Need**: White-label editor, custom domain, SSO integration, audit logs.
**Uses**: All 4 layers with white-label configuration.

## Success Metrics

| Metric | Target (12 months) | Target (24 months) |
|--------|--------------------|--------------------|
| GitHub stars | 2,000 | 10,000 |
| npm weekly downloads | 500 | 5,000 |
| crates.io downloads | 200 | 2,000 |
| Active integrations (known) | 10 | 50 |
| Contributors | 15 | 50 |
| Documentation completeness | 90% | 99% |
| Format fidelity (DOCX round-trip) | 90% | 95% |

## Positioning Statement

**For** developers and product teams **who** need document editing capabilities in their applications, **s1engine is** an open-source document engine **that** provides format conversion, WYSIWYG editing, real-time collaboration, and PDF export as embeddable components. s1engine is modular, self-hostable, format-native (DOCX/ODT), and designed for embedding from the ground up.

## License Strategy

| Component | License | Rationale |
|-----------|---------|-----------|
| Core engine (Rust crates) | AGPL-3.0-or-later | Ensures open-source usage under copyleft |
| Server API | AGPL-3.0-or-later | Network use triggers copyleft |
| Client SDK + Editor | AGPL-3.0-or-later | Consistent licensing |
| Relay/Collaboration Server | AGPL-3.0-or-later | Self-hostable under AGPL |
| Commercial License | Dual-license (proprietary) | Revenue stream — companies buy to avoid AGPL |

**[DECISION]**: Use AGPL-3.0 with commercial dual-licensing for all components. Companies that embed s1engine in proprietary SaaS products purchase a commercial license.

## Non-Goals (Explicit)

- We are NOT building a Notion clone (block-based, databases, etc.)
- We are NOT building spreadsheet or presentation support (document/word-processing only)
- We are NOT providing a managed hosting service initially (self-host first)
- We are NOT competing on being the "best text editor" — we compete on being the best *document* editor (formats, pagination, PDF)
- We are NOT building mobile-native editors (web-first, mobile via responsive web)
