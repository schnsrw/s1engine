# Platform Architecture

## System Overview

```
                         ┌──────────────────────────────────┐
                         │         Consumer Apps             │
                         │   React / Vue / Angular / HTML    │
                         └───────┬──────────┬───────────────┘
                                 │          │
                    ┌────────────┘          └────────────┐
                    │                                    │
         ┌──────────▼──────────┐            ┌───────────▼──────────┐
         │   @rudra/editor  │            │   @rudra/sdk       │
         │   (Embeddable UI)   │            │   (Headless JS API)   │
         │                     │            │                       │
         │  - Drop-in editor   │            │  - No UI              │
         │  - Toolbar config   │            │  - Format conversion  │
         │  - Theme system     │            │  - Doc manipulation   │
         │  - Event hooks      │            │  - Batch processing   │
         └──────────┬──────────┘            └───────────┬──────────┘
                    │                                    │
         ┌──────────▼────────────────────────────────────▼──────────┐
         │                    @rudra/wasm                         │
         │               (WebAssembly Core Engine)                  │
         │                                                          │
         │   WasmEngine / WasmDocument / WasmLayoutConfig           │
         │   (wasm-bindgen bindings to Rust core)                   │
         └──────────────────────────┬───────────────────────────────┘
                                    │ (client-side, runs in browser)
         ═══════════════════════════╪═══════════════════════════════
                                    │ (server-side, optional)
         ┌──────────────────────────▼───────────────────────────────┐
         │                   s1-server (Rust / Axum)                │
         │                                                          │
         │  ┌─────────────┐ ┌──────────────┐ ┌──────────────────┐  │
         │  │  REST API    │ │  WebSocket   │ │  Webhook Engine  │  │
         │  │  /api/docs   │ │  /ws/collab  │ │  Event dispatch  │  │
         │  └──────┬───── ┘ └──────┬───────┘ └────────┬─────────┘  │
         │         │               │                   │            │
         │  ┌──────▼───────────────▼───────────────────▼─────────┐  │
         │  │                   Service Layer                     │  │
         │  │  DocumentService / CollabService / ConvertService   │  │
         │  │  AuthService / WebhookService / StorageService      │  │
         │  └──────────────────────┬──────────────────────────────┘  │
         │                         │                                 │
         │  ┌──────────────────────▼──────────────────────────────┐  │
         │  │                 s1engine (Rust Core)                 │  │
         │  │  s1-model | s1-ops | s1-crdt | s1-layout | formats  │  │
         │  └──────────────────────┬──────────────────────────────┘  │
         │                         │                                 │
         │  ┌──────────────────────▼──────────────────────────────┐  │
         │  │              Storage Abstraction Layer               │  │
         │  │                                                      │  │
         │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐ │  │
         │  │  │ Local FS │ │   S3     │ │ Postgres │ │ Custom │ │  │
         │  │  └──────────┘ └──────────┘ └──────────┘ └────────┘ │  │
         │  └─────────────────────────────────────────────────────┘  │
         └──────────────────────────────────────────────────────────┘
```

## Architecture Principles

### 1. Every Layer is Optional

A consumer can use any combination:
- **WASM only** — client-side editing, no server needed
- **Server only** — document processing API, no editor UI
- **Server + SDK** — headless document manipulation from JS
- **Server + Editor** — full editing experience
- **Editor only** — client-side editor without server (files saved locally/to consumer's backend)

**[DECISION]**: No layer should have a hard dependency on another layer above or below it (except WASM → Rust core, which is the compilation target).

### 2. Server is Stateless (Where Possible)

The REST API server should be horizontally scalable:
- Document state lives in the storage backend, not in server memory
- Collaboration state is managed per-room with Redis or in-memory (single-node) or CRDT sync (multi-node)
- No sticky sessions required for REST endpoints
- WebSocket connections are stateful by nature but can be distributed via Redis pub/sub

### 3. Client-First Rendering

**[DECISION]**: Rendering happens on the client (WASM), not the server. This is a fundamental architectural choice (in contrast to server-side tile rendering approaches):

| | Server-Side Rendering | Client-Side Rendering (s1engine) |
|---|---|---|
| Server cost | High (CPU per user) | Low (serve static files) |
| Latency | Higher (round-trip for every keystroke) | Near-zero (local WASM) |
| Offline support | No | Yes |
| Scaling | Expensive (1 process per user) | Cheap (static hosting) |
| Consistency | Easy (single renderer) | Harder (must match across browsers) |

Server-side rendering is available for specific use cases (PDF generation, thumbnails, batch conversion) but editing is always client-side.

### 4. Format-Native Document Model

The document model is DOCX/ODF-native, not HTML-based. This means:
- Round-tripping through DOCX preserves formatting (no lossy HTML conversion)
- Pagination and page layout are first-class concepts
- Headers, footers, sections, page breaks work correctly
- No impedance mismatch between internal model and export formats

---

## Component Architecture

### A. Core Engine (Rust — existing)

```
crates/
├── s1-model/           Zero-dep document tree (nodes, attributes, styles)
├── s1-ops/             Operation system (insert, delete, format, undo/redo)
├── s1-crdt/            CRDT collaboration (Fugue text, Kleppmann tree, LWW attributes)
├── s1-layout/          Page layout engine (Knuth-Plass, pagination, tables)
├── s1-text/            Text processing (shaping, fonts, BiDi, line breaking)
├── s1-format-docx/     DOCX reader/writer
├── s1-format-odt/      ODT reader/writer
├── s1-format-pdf/      PDF export (with font embedding)
├── s1-format-txt/      Plain text reader/writer
├── s1-format-md/       Markdown reader/writer
├── s1-convert/         Format conversion pipelines
└── s1engine/           Facade crate (public API)
```

**No changes needed** to the core engine for the platform plan. It remains a pure library.

### B. Server API (Rust — new: `s1-server`)

A new crate that wraps the core engine in an HTTP server.

```
crates/s1-server/
├── src/
│   ├── lib.rs              # Server builder, configuration
│   ├── main.rs             # Binary entry point
│   ├── config.rs           # Environment-based configuration
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── documents.rs    # CRUD endpoints
│   │   ├── convert.rs      # Format conversion endpoints
│   │   ├── export.rs       # Export endpoints (PDF, DOCX, etc.)
│   │   ├── collab.rs       # WebSocket collaboration
│   │   ├── health.rs       # Health/readiness checks
│   │   └── admin.rs        # Admin endpoints (metrics, rooms)
│   ├── services/
│   │   ├── mod.rs
│   │   ├── document.rs     # Document business logic
│   │   ├── convert.rs      # Conversion orchestration
│   │   ├── collab.rs       # Collaboration room management
│   │   └── webhook.rs      # Webhook dispatch
│   ├── storage/
│   │   ├── mod.rs          # StorageBackend trait
│   │   ├── local.rs        # Local filesystem
│   │   ├── s3.rs           # S3-compatible (AWS, MinIO, R2)
│   │   ├── postgres.rs     # PostgreSQL (binary storage)
│   │   └── memory.rs       # In-memory (testing)
│   ├── auth/
│   │   ├── mod.rs          # Auth middleware
│   │   ├── jwt.rs          # JWT validation
│   │   ├── api_key.rs      # API key validation
│   │   └── permissions.rs  # Document-level permissions
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── cors.rs         # CORS configuration
│   │   ├── rate_limit.rs   # Rate limiting
│   │   ├── logging.rs      # Request logging
│   │   └── tracing.rs      # Distributed tracing
│   └── error.rs            # API error types (JSON responses)
├── Cargo.toml
└── tests/
    ├── api_tests.rs        # Integration tests
    └── fixtures/
```

**Technology choices:**

| Component | Choice | Rationale |
|-----------|--------|-----------|
| HTTP framework | Axum | Rust-native, async, tower middleware, excellent performance |
| Async runtime | Tokio | Industry standard, Axum requirement |
| Serialization | serde + serde_json | Standard Rust JSON handling |
| WebSocket | axum::extract::ws | Built into Axum, no extra dependency |
| S3 client | aws-sdk-s3 or rusoto | AWS SDK for Rust |
| Database | sqlx (PostgreSQL) | Async, compile-time checked queries |
| Auth | jsonwebtoken | JWT validation (HS256, RS256) |
| Rate limiting | governor | Token bucket, per-key |
| Tracing | tracing + tracing-subscriber | Structured logging |
| Config | figment or config | Environment + file config |
| OpenAPI | utoipa | Auto-generate OpenAPI spec from code |

**[DECISION]**: Server is built in Rust (Axum), not Node.js. Rationale:
1. Calls s1engine natively (no serialization overhead, no FFI)
2. Single binary deployment (no Node.js runtime dependency)
3. Better performance for document processing (CPU-bound)
4. Same language as core engine (one contributor skillset)
5. Memory safety (no Node.js memory leaks under load)

### C. Client SDK (JavaScript/TypeScript — new: `@rudra/editor`, `@rudra/sdk`)

```
packages/
├── wasm/                       # @rudra/wasm (existing, repackaged)
│   ├── s1engine_wasm_bg.wasm
│   ├── s1engine_wasm.js
│   └── s1engine_wasm.d.ts
│
├── sdk/                        # @rudra/sdk (new — headless API)
│   ├── src/
│   │   ├── index.ts            # Main exports
│   │   ├── engine.ts           # S1Engine class (wraps WASM)
│   │   ├── document.ts         # S1Document class
│   │   ├── types.ts            # TypeScript interfaces
│   │   ├── events.ts           # Event emitter
│   │   ├── storage.ts          # Client-side storage adapters
│   │   └── collab.ts           # Collaboration client
│   ├── package.json
│   └── tsconfig.json
│
├── editor/                     # @rudra/editor (new — embeddable UI)
│   ├── src/
│   │   ├── index.ts            # Main export: S1Editor.create()
│   │   ├── editor.ts           # Editor class (container, lifecycle)
│   │   ├── toolbar.ts          # Configurable toolbar
│   │   ├── renderer.ts         # Document renderer (from existing render.js)
│   │   ├── input-handler.ts    # Input handling (from existing input.js)
│   │   ├── collab-ui.ts        # Collaboration UI (cursors, presence)
│   │   ├── file-handler.ts     # File open/save (from existing file.js)
│   │   ├── theme.ts            # Theme system (CSS variables)
│   │   ├── config.ts           # Editor configuration schema
│   │   ├── plugins/            # Built-in plugins
│   │   │   ├── pdf-viewer.ts
│   │   │   ├── find-replace.ts
│   │   │   ├── comments.ts
│   │   │   └── track-changes.ts
│   │   └── styles/
│   │       ├── base.css        # Base styles (reset, layout)
│   │       ├── toolbar.css     # Toolbar styles
│   │       ├── editor.css      # Editor area styles
│   │       └── themes/
│   │           ├── default.css
│   │           ├── dark.css
│   │           └── minimal.css
│   ├── package.json
│   └── tsconfig.json
│
└── adapters/                   # Framework-specific wrappers
    ├── react/                  # @rudra/react
    │   ├── src/
    │   │   ├── S1Editor.tsx    # React component
    │   │   └── useS1Engine.ts  # React hook
    │   └── package.json
    ├── vue/                    # @rudra/vue
    │   ├── src/
    │   │   └── S1Editor.vue    # Vue component
    │   └── package.json
    └── web-component/          # @rudra/web-component
        ├── src/
        │   └── s1-editor.ts    # Custom element
        └── package.json
```

### D. Storage Architecture

```
┌───────────────────────────────────────────────────────┐
│                  StorageBackend Trait                   │
│                                                        │
│  async fn save(id, bytes, metadata) → Result<()>       │
│  async fn load(id) → Result<(bytes, metadata)>         │
│  async fn delete(id) → Result<()>                      │
│  async fn list(filter) → Result<Vec<DocInfo>>          │
│  async fn exists(id) → Result<bool>                    │
│  async fn metadata(id) → Result<DocMetadata>           │
│  async fn update_metadata(id, meta) → Result<()>       │
│                                                        │
├───────────────────────────────────────────────────────┤
│                                                        │
│  LocalStorage      S3Storage       PostgresStorage     │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐   │
│  │ /data/docs/  │  │ bucket/     │  │ documents    │   │
│  │  {id}.docx   │  │  {id}.docx  │  │  table       │   │
│  │  {id}.meta   │  │  {id}.meta  │  │  (id, bytes, │   │
│  └─────────────┘  └─────────────┘  │   metadata)  │   │
│                                     └──────────────┘   │
│  MemoryStorage     GCSStorage      CustomStorage       │
│  (testing)         (Google Cloud)   (user-provided)    │
└───────────────────────────────────────────────────────┘
```

**Document storage model:**

```
DocumentRecord {
    id:            UUID,              // Unique document ID
    tenant_id:     Option<String>,    // Multi-tenant isolation
    title:         String,            // Display title
    format:        Format,            // Original format (DOCX, ODT, etc.)
    content:       Vec<u8>,           // Raw document bytes (in native format)
    crdt_state:    Option<Vec<u8>>,   // Serialized CRDT state (for collab docs)
    created_at:    DateTime,
    updated_at:    DateTime,
    created_by:    Option<String>,    // User ID
    version:       u64,               // Optimistic concurrency
    metadata:      JsonValue,         // User-defined metadata (tags, etc.)
    permissions:   Vec<Permission>,   // Access control list
}
```

**[DECISION]**: Documents are stored in their native format (DOCX bytes), not in an intermediate representation. This preserves full fidelity and means consumers can download the original format at any time. CRDT state is stored separately for collaborative documents.

### E. Collaboration Architecture

See [COLLABORATION.md](COLLABORATION.md) for full details. Summary:

```
┌─────────┐    WebSocket    ┌──────────────────┐    CRDT Ops    ┌─────────┐
│ Client A │ ──────────────▶│   s1-server      │◀──────────────│ Client B │
│ (WASM)   │◀──────────────│   (Room Manager) │──────────────▶│ (WASM)   │
└─────────┘                 ├──────────────────┤                └─────────┘
                            │ Room State       │
                            │ ┌──────────────┐ │
                            │ │ CRDT Doc     │ │
                            │ │ Peer list    │ │
                            │ │ Awareness    │ │
                            │ └──────────────┘ │
                            │                  │
                            │ Persistence      │
                            │ ┌──────────────┐ │
                            │ │ Storage      │ │
                            │ │ Backend      │ │
                            │ └──────────────┘ │
                            └──────────────────┘
```

### F. Authentication & Authorization Flow

See [AUTH_AND_SECURITY.md](AUTH_AND_SECURITY.md) for full details. Summary:

```
Consumer App                    s1-server
┌──────────┐                   ┌──────────────────┐
│          │  1. Generate JWT  │                  │
│  Auth    │  (with doc perms) │   Auth           │
│  System  │ ────────────────▶ │   Middleware     │
│          │                   │   ┌────────────┐ │
│          │  2. Include JWT   │   │ Validate   │ │
│          │  in API calls     │   │ JWT/API key│ │
└──────────┘                   │   │ Extract    │ │
                               │   │ permissions│ │
                               │   └────────────┘ │
                               │         │        │
                               │   ┌─────▼──────┐ │
                               │   │ Permission │ │
                               │   │ Check      │ │
                               │   │ (per doc)  │ │
                               │   └────────────┘ │
                               └──────────────────┘
```

**[DECISION]**: s1engine does NOT manage users. The consumer's auth system generates JWTs with embedded permissions. s1engine validates the JWT and enforces permissions. This keeps s1engine stateless regarding user management.

---

## Deployment Topologies

### Topology 1: Client-Only (Simplest)

```
Browser
┌────────────────────────────┐
│  Consumer App              │
│  ┌──────────────────────┐  │
│  │  @rudra/editor    │  │
│  │  (WASM engine inside)│  │
│  └──────────────────────┘  │
│  No server needed.         │
│  Files saved locally or    │
│  to consumer's own backend │
└────────────────────────────┘
```

**Use case**: Simple document viewer/editor embedded in an existing app. No collaboration.

### Topology 2: Client + Consumer Backend

```
Browser                          Consumer's Server
┌──────────────────────┐        ┌──────────────────────┐
│  @rudra/editor    │  REST  │  Consumer's API      │
│  (WASM)              │ ──────▶│  + @rudra/sdk     │
│                      │◀────── │  (server-side WASM   │
└──────────────────────┘        │   or Rust native)    │
                                └──────────────────────┘
```

**Use case**: Consumer wants to use their own backend but needs document processing.

### Topology 3: Full Platform (Self-Hosted)

```
Browser                     s1-server              Storage
┌──────────────────┐       ┌──────────────┐       ┌──────────┐
│ @rudra/editor │ REST  │              │       │          │
│ (WASM)           │──────▶│  REST API    │──────▶│  S3      │
│                  │◀──────│  WebSocket   │◀──────│  or      │
│                  │  WS   │  Auth        │       │  Postgres│
└──────────────────┘       │  Webhooks    │       │  or      │
                           └──────────────┘       │  Local FS│
                                                  └──────────┘
```

**Use case**: Full-featured self-hosted document platform with collaboration.

### Topology 4: Multi-Node (Scaled)

```
                    Load Balancer
                    ┌───────────┐
                    │  nginx /  │
                    │  Traefik  │
                    └─────┬─────┘
              ┌───────────┼───────────┐
              │           │           │
         ┌────▼───┐  ┌───▼────┐  ┌──▼─────┐
         │ s1-srv │  │ s1-srv │  │ s1-srv  │   Stateless REST
         │  (1)   │  │  (2)   │  │  (3)    │   handlers
         └────┬───┘  └───┬────┘  └──┬─────┘
              │          │          │
         ┌────▼──────────▼──────────▼─────┐
         │            Redis                │   WebSocket pub/sub
         │    (room routing, presence)     │   Session coordination
         └────────────────┬───────────────┘
                          │
         ┌────────────────▼───────────────┐
         │         PostgreSQL / S3         │   Document storage
         └────────────────────────────────┘
```

**Use case**: High-availability deployment for SaaS products.

---

## Data Flow: Key Operations

### Open Document

```
1. Client: POST /api/documents (upload bytes)
2. Server: Detect format → parse with s1engine → store to storage backend
3. Server: Return document ID + metadata
4. Client: Initialize WASM engine with document bytes
5. Client: Render with to_paginated_html()
```

### Save Document

```
1. Client: Export document via WASM (doc.export('docx'))
2. Client: PUT /api/documents/:id (upload bytes)
3. Server: Validate → store to storage backend
4. Server: Fire webhook (document.updated)
5. Server: Return updated metadata
```

### Collaborate

```
1. Client A: POST /api/rooms (create room for document)
2. Client A: Connect WS to /ws/collab/:room_id
3. Client B: Connect WS to /ws/collab/:room_id
4. Client A: Local edit → CRDT op → broadcast via WS
5. Server: Relay op to Client B (and persist CRDT state)
6. Client B: Apply remote CRDT op → re-render
7. Periodically: Server materializes CRDT state → saves to storage
```

### Convert Format

```
1. Client: POST /api/convert { from: "docx", to: "pdf", file: <bytes> }
2. Server: Parse DOCX → s1-model → layout → PDF export
3. Server: Return PDF bytes
```

---

## Configuration Model

```toml
# s1-server.toml

[server]
host = "0.0.0.0"
port = 8080
workers = 4                    # Tokio worker threads

[storage]
backend = "s3"                 # "local", "s3", "postgres", "memory"

[storage.local]
path = "/data/documents"

[storage.s3]
bucket = "s1-documents"
region = "us-east-1"
endpoint = ""                  # Custom endpoint (MinIO, R2)
access_key = ""                # Or use IAM role
secret_key = ""

[storage.postgres]
url = "postgresql://user:pass@localhost/s1engine"

[auth]
enabled = true
jwt_secret = ""                # HS256 secret
jwt_public_key = ""            # RS256 public key (PEM)
api_key_enabled = true

[collab]
enabled = true
max_rooms = 1000
max_peers_per_room = 50
auto_save_interval = 30        # seconds
state_persistence = true

[webhooks]
enabled = true
max_retries = 3
timeout = 10                   # seconds

[rate_limit]
enabled = true
requests_per_minute = 60       # Per API key
burst = 10

[cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]

[logging]
level = "info"                 # trace, debug, info, warn, error
format = "json"                # "json" or "pretty"

[white_label]
enabled = false
# See WHITE_LABELING.md for full configuration
```

**[DECISION]**: Configuration uses TOML file + environment variable overrides (12-factor app). Every config value can be set via `S1_` prefixed env vars (e.g., `S1_SERVER_PORT=8080`).

---

## Crate Dependency Graph (with new crates)

```
s1-server (new)
├── s1engine (facade)
│   ├── s1-model (zero deps)
│   ├── s1-ops → s1-model
│   ├── s1-crdt → s1-model, s1-ops
│   ├── s1-layout → s1-model, s1-text
│   ├── s1-format-docx → s1-model
│   ├── s1-format-odt → s1-model
│   ├── s1-format-pdf → s1-model, s1-layout, s1-text
│   ├── s1-format-txt → s1-model
│   ├── s1-format-md → s1-model
│   ├── s1-convert → s1-format-docx, s1-format-odt
│   └── s1-text → s1-model
├── axum (HTTP framework)
├── tokio (async runtime)
├── sqlx (database, optional)
├── aws-sdk-s3 (S3, optional)
├── jsonwebtoken (auth)
├── governor (rate limiting)
├── tracing (logging)
├── utoipa (OpenAPI docs)
└── figment (configuration)

ffi/wasm (existing, repackaged)
└── s1engine → (same tree as above)

packages/sdk (new, TypeScript)
└── @rudra/wasm (compiled from ffi/wasm)

packages/editor (new, TypeScript)
├── @rudra/sdk
└── @rudra/wasm

packages/adapters/react (new, TypeScript)
└── @rudra/editor

packages/adapters/vue (new, TypeScript)
└── @rudra/editor
```

---

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Open 10-page DOCX | < 100ms | WASM, client-side |
| Render page (to_paginated_html) | < 50ms | Per page |
| Keystroke latency | < 16ms | 60fps editing |
| CRDT op broadcast | < 10ms | WebSocket round-trip |
| PDF export (10 pages) | < 500ms | Server-side |
| Format conversion (DOCX→PDF) | < 2s | Server-side, 50 pages |
| API response (document list) | < 50ms | Cached |
| WASM bundle size | < 3MB | gzipped |
| Editor JS bundle | < 500KB | gzipped, excl. WASM |
| Concurrent users per room | 50 | WebSocket |
| Concurrent rooms per server | 1,000 | Single node |
| Documents per storage backend | 1M+ | Depends on backend |

---

## Security Considerations

See [AUTH_AND_SECURITY.md](AUTH_AND_SECURITY.md) for full details.

- All API endpoints require authentication (JWT or API key)
- Document-level permissions (read, write, comment, admin)
- Rate limiting per API key / IP
- Input validation on all endpoints (file size limits, format validation)
- CORS configuration
- No eval() or dynamic code execution in WASM/JS
- CSP headers for editor embedding
- Audit logging for document access
