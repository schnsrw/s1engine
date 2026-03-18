# Phase 3: Server API (REST + Storage)

> Goal: Rust-based HTTP server for document management and format conversion.
> Created: 2026-03-18 | Depends on: Phase 1 (complete)

## Milestone 3.1 — Server Scaffold

| ID | Task | Status |
|----|------|--------|
| P3-01 | Create server/ crate with Axum + Tokio | DONE |
| P3-02 | Configuration system (TOML + env vars) | DONE |
| P3-03 | Health check endpoint (GET /health) | DONE |
| P3-04 | Server info endpoint (GET /api/v1/info) | DONE |
| P3-05 | CORS + tracing middleware | DONE |
| P3-06 | Add to workspace members | DONE |

## Milestone 3.2 — Document CRUD

| ID | Task | Status |
|----|------|--------|
| P3-07 | POST /api/v1/documents (upload) | DONE |
| P3-08 | GET /api/v1/documents (list — placeholder) | DONE |
| P3-09 | POST /api/v1/convert (stateless conversion) | DONE |
| P3-10 | Storage backend trait definition | OPEN |
| P3-11 | LocalStorage implementation | OPEN |
| P3-12 | GET /api/v1/documents/:id | OPEN |
| P3-13 | DELETE /api/v1/documents/:id | OPEN |

## Milestone 3.3 — Advanced Features

| ID | Task | Status |
|----|------|--------|
| P3-14 | Thumbnail generation (first page → PNG) | OPEN |
| P3-15 | Webhook system (document.created, document.exported) | OPEN |
| P3-16 | Docker image for s1-server | OPEN |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P3-01 | 2026-03-18 | server/Cargo.toml + src/main.rs with Axum 0.8, Tokio, tracing |
| P3-02 | 2026-03-18 | server/src/config.rs: Config struct with TOML + env var loading, defaults |
| P3-03 | 2026-03-18 | GET /health returns {"status":"ok","version":"1.0.1"} |
| P3-04 | 2026-03-18 | GET /api/v1/info returns engine version + supported formats |
| P3-05 | 2026-03-18 | CorsLayer::permissive + TraceLayer + 64MB body limit |
| P3-06 | 2026-03-18 | Added "server" to workspace members in root Cargo.toml |
| P3-07 | 2026-03-18 | POST /api/v1/documents: multipart upload, s1engine validation, returns metadata |
| P3-08 | 2026-03-18 | GET /api/v1/documents: placeholder returning empty list |
| P3-09 | 2026-03-18 | POST /api/v1/convert: multipart file + format → converted bytes |
