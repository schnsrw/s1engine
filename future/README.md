# s1engine — Open-Source Platform Plan

## Purpose

This directory contains the complete planning documentation for transforming s1engine from a standalone Rust document engine library into a fully integrable, white-label-ready, open-source document editing platform that other products can embed via APIs.

## Document Index

| Document | Description |
|----------|-------------|
| [VISION.md](VISION.md) | Product vision, positioning, target personas, success metrics |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Full platform architecture — server, client, APIs, storage, collaboration |
| [PHASES.md](PHASES.md) | Detailed phased roadmap with milestones, deliverables, and dependencies |
| [REST_API.md](REST_API.md) | REST API specification (OpenAPI-style) — all endpoints, auth, pagination |
| [WEBSOCKET_API.md](WEBSOCKET_API.md) | WebSocket collaboration protocol specification |
| [EDITOR_SDK.md](EDITOR_SDK.md) | JavaScript/TypeScript SDK for embedding the editor in any app |
| [WHITE_LABELING.md](WHITE_LABELING.md) | White-label strategy — theming, branding, custom domains, OEM licensing |
| [DISTRIBUTION.md](DISTRIBUTION.md) | Packaging and distribution — npm, crates.io, Docker, CDN |
| [AUTH_AND_SECURITY.md](AUTH_AND_SECURITY.md) | Authentication, authorization, permissions, multi-tenancy |
| [COLLABORATION.md](COLLABORATION.md) | Real-time collaboration architecture — CRDT, presence, conflict resolution |
| [COMPETITIVE_ANALYSIS.md](COMPETITIVE_ANALYSIS.md) | Market positioning against existing solutions |
| [PLUGIN_SYSTEM.md](PLUGIN_SYSTEM.md) | Extension and plugin architecture for customization |

## Current State (as of 2026-03-16)

The s1engine core is **complete and production-tested**:
- 14 Rust crates, 1,172+ tests, zero warnings
- Full format support: DOCX, ODT, PDF, TXT, Markdown
- CRDT-based collaboration with Fugue text + Kleppmann tree
- Layout engine with Knuth-Plass line breaking
- WASM bindings (wasm-bindgen) + C FFI (cbindgen)
- Production web editor ("Folio") — vanilla JS, multi-page, collaboration-ready
- Docker deployment with multi-stage builds

## What This Plan Adds

The core engine is a library. This plan turns it into a **platform**:

```
CURRENT:    Library (Rust) → WASM → Editor (single app)
FUTURE:     Library (Rust) → Server API → SDK → Embeddable Editor → White-label Platform
```

## Decision Log

Major architectural decisions are recorded inline in each document with `[DECISION]` tags for traceability.
