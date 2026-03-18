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
| P3-07 | POST /api/v1/documents (upload with validation) | DONE |
| P3-08 | GET /api/v1/documents (list, paginated) | DONE |
| P3-09 | POST /api/v1/convert (stateless conversion) | DONE |
| P3-10 | StorageBackend trait + MemoryStorage + LocalStorage | DONE |
| P3-11 | GET /api/v1/documents/:id (metadata) | DONE |
| P3-12 | GET /api/v1/documents/:id/content (bytes) | DONE |
| P3-13 | DELETE /api/v1/documents/:id | DONE |

## Milestone 3.3 — Advanced Features

| ID | Task | Status |
|----|------|--------|
| P3-14 | Thumbnail generation (first page → PNG) | DONE |
| P3-15 | Webhook system (document events) | DONE |
| P3-16 | Docker image for s1-server | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P3-01 | 2026-03-18 | server/Cargo.toml + src/main.rs with Axum 0.8 + multipart, Tokio, tracing |
| P3-02 | 2026-03-18 | server/src/config.rs: TOML + env vars (S1_PORT, S1_STORAGE, S1_DATA_DIR) |
| P3-03 | 2026-03-18 | GET /health → {"status":"ok","version":"1.0.1"} |
| P3-04 | 2026-03-18 | GET /api/v1/info → engine version, supported read/write formats |
| P3-05 | 2026-03-18 | CorsLayer::permissive + TraceLayer + 64MB DefaultBodyLimit |
| P3-06 | 2026-03-18 | "server" added to workspace members |
| P3-07 | 2026-03-18 | POST upload: multipart file → s1engine validate → UUID → storage.put → return metadata |
| P3-08 | 2026-03-18 | GET list: paginated (page/pageSize query), sorted by updated_at desc |
| P3-09 | 2026-03-18 | POST convert: multipart file+format → s1engine open → export → bytes |
| P3-10 | 2026-03-19 | StorageBackend trait (put/get/get_meta/delete/list/exists), MemoryStorage + LocalStorage implementations, 2 tests |
| P3-11 | 2026-03-19 | GET /api/v1/documents/:id → DocumentMeta JSON |
| P3-12 | 2026-03-19 | GET /api/v1/documents/:id/content → raw document bytes |
| P3-13 | 2026-03-19 | DELETE /api/v1/documents/:id → 204 No Content (or 404) |
