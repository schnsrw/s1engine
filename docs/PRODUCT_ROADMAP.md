# Rudra Code — Product Roadmap

> From document engine to enterprise editing platform.
> Created: 2026-03-19

## Vision

Rudra Code becomes the open-source alternative to OnlyOffice/Collabora — a single Docker image that any product can embed for document editing, collaboration, and conversion. AGPL-3.0 licensed (with commercial dual-license), pure Rust, sub-200MB image.

---

## Near-Term (Current Sprint)

### Unified Server Architecture
Single binary serving everything: static editor + REST API + WebSocket collab.

**Phases A-D** (see `UNIFIED_SERVER_PLAN.md`):
- A: File sessions with TTL + static file serving
- B: Server-authoritative WebSocket editing
- C: JWT integration mode (like OnlyOffice Document Server API)
- D: Single Docker image

### Documentation Wiki
Comprehensive docs for operators and integrators:

| Topic | Content |
|-------|---------|
| **Deployment Guide** | Docker, Docker Compose, Kubernetes, bare metal |
| **Reverse Proxy** | Nginx, Caddy, Traefik, Apache configs with WebSocket passthrough |
| **TLS/SSL** | Let's Encrypt, custom certs, termination at proxy vs server |
| **White-Labeling** | CSS theming, branding config, custom domain, removing s1 branding |
| **Configuration** | All env vars, TOML config, feature flags, limits |
| **Integration API** | JWT format, callback protocol, iframe embedding, SDK usage |
| **Plugin Development** | Client plugin API, server hooks, custom toolbar, side panels |
| **Monitoring** | Prometheus metrics, health checks, logging, alerting |
| **Scaling** | Single node limits, when to cluster, resource planning |
| **Security** | CSP headers, CORS, auth, input validation, file size limits |
| **Troubleshooting** | Common issues, debug mode, log levels |

### Reverse Proxy Support
Essential for production deployment:

```nginx
# Nginx example
server {
    listen 443 ssl;
    server_name docs.myapp.com;

    ssl_certificate /etc/letsencrypt/live/docs.myapp.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/docs.myapp.com/privkey.pem;

    location / {
        proxy_pass http://s1-server:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400; # WebSocket keepalive
    }
}
```

---

## Mid-Term (Next Quarter)

### Clustering & Horizontal Scaling
Make s1-server stateless for multi-instance deployment.

| Component | Current | Clustered |
|-----------|---------|-----------|
| File sessions | In-memory per server | Shared blob storage (S3/MinIO) |
| WebSocket rooms | In-memory per server | Redis pub/sub for cross-node relay |
| Room state | In-memory | Redis or shared storage |
| Document snapshots | Local filesystem | S3-compatible object store |
| Session metadata | In-memory HashMap | Redis or PostgreSQL |

**Architecture:**
```
Load Balancer (sticky sessions for WS, or Redis routing)
    ├── s1-server instance 1
    ├── s1-server instance 2
    └── s1-server instance 3
         │
    ┌────┴────┐
    │  Redis   │ (pub/sub for room ops, session state)
    └────┬────┘
    ┌────┴────┐
    │   S3    │ (document snapshots, file storage)
    └─────────┘
```

**Implementation:**
1. `StorageBackend::S3` implementation (aws-sdk-s3 or rusoto)
2. Redis-backed `RoomManager` (redis pub/sub for op relay)
3. Redis-backed `FileSessionManager` (session metadata)
4. Sticky session support (WebSocket affinity via room hash)
5. Health endpoint reports instance ID + cluster status

### Rate Limiting & Security Hardening
- Per-IP rate limits on upload/convert endpoints
- Max concurrent sessions per tenant
- File size limits per session type
- Request signing for callback URLs
- CORS configuration per tenant

---

## Long-Term (Future Roadmap)

### Additional Document Types

Each format gets its own WASM crate — browser only loads what's needed.

| Format | Crate | WASM Size | Status |
|--------|-------|-----------|--------|
| **Documents** (DOCX/ODT) | `s1-format-docx`, `s1-format-odt` | ~2MB | Done |
| **Spreadsheets** (XLSX/ODS) | `s1-format-xlsx`, `s1-format-ods` | ~1.5MB | Planned |
| **Presentations** (PPTX/ODP) | `s1-format-pptx`, `s1-format-odp` | ~1.5MB | Planned |
| **Forms** (PDF forms, HTML forms) | `s1-format-forms` | ~500KB | Planned |
| **Diagrams** (draw.io/SVG) | `s1-format-diagram` | ~1MB | Planned |

**Key design principle**: Browser loads ONLY the crate for the format being edited. A user opening a DOCX never downloads the spreadsheet WASM.

```
Editor shell (shared toolbar, collab, layout)     ~500KB
  + s1-format-docx.wasm (if editing .docx)        ~2MB
  + s1-format-xlsx.wasm (if editing .xlsx)         ~1.5MB
  + s1-format-pptx.wasm (if editing .pptx)         ~1.5MB
```

**Implementation approach per format:**

#### Spreadsheets (s1-format-xlsx)
- New `s1-model-sheet` crate: Cell model, formulas, sheets, charts
- Parser: OpenXML SpreadsheetML (xlsx) + ODS
- Renderer: CSS grid-based cell layout
- Features: formulas, conditional formatting, charts, pivot tables (progressive)
- Editor: separate `editor-sheet/` with cell grid, formula bar, sheet tabs

#### Presentations (s1-format-pptx)
- New `s1-model-slide` crate: Slide model, shapes, transitions, animations
- Parser: OpenXML PresentationML (pptx) + ODP
- Renderer: Canvas-based slide rendering
- Features: master slides, layouts, transitions, speaker notes
- Editor: separate `editor-slide/` with slide panel, canvas, notes

#### Diagrams (s1-format-diagram)
- New `s1-model-diagram` crate: Shape graph, connectors, layers
- Parser: draw.io XML, SVG, Visio (vsdx)
- Renderer: SVG-based with interactive handles
- Editor: separate `editor-diagram/` with shape palette, canvas, layers

### LLM Integration

AI-assisted editing via pluggable LLM backends:

| Feature | Description |
|---------|-------------|
| **Content Generation** | "Write an introduction for..." → generates paragraph |
| **Rewriting** | Select text → "Make it more formal" / "Simplify" / "Translate to Spanish" |
| **Summarization** | Select document → generate executive summary |
| **Grammar & Style** | Real-time suggestions (like Grammarly) |
| **Template Fill** | "Fill this template with data from..." |
| **Format Conversion AI** | Lossy format round-trip repair via LLM |

**Architecture:**
```
Editor → Plugin API → LLM Provider Interface
                          ├── OpenAI API
                          ├── Anthropic Claude API
                          ├── Local Ollama/vLLM
                          └── Custom endpoint
```

**Implementation:**
- `s1-ai` crate: Provider trait, prompt templates, streaming responses
- Editor plugin: AI sidebar panel, inline suggestions, /ai slash command
- Server endpoint: `POST /api/v1/ai/complete` (proxies to configured LLM)
- Configurable: provider, model, API key, temperature, max tokens
- Privacy: option to run fully local (Ollama) — no data leaves the server

### Plugin Marketplace
- Plugin discovery and installation
- Versioning and compatibility checking
- Community-contributed plugins
- Plugin sandboxing (iframe isolation for untrusted plugins)

---

## Architecture Principles (All Phases)

1. **Format-per-WASM**: Each document type is a separate WASM module. Browser loads only what's needed.
2. **Single binary server**: No sidecar processes, no separate relay, no nginx. One binary does everything.
3. **Stateless-ready**: In-memory by default for simplicity, but every state component has a shared-storage backend for clustering.
4. **AGPL-3.0 + commercial dual-license**: Same model as OnlyOffice. Free for open-source use; commercial license for proprietary embedding.
5. **Pure Rust**: No C/C++ dependencies in the core. Compiles anywhere Rust does.
6. **Progressive features**: Basic editing works immediately. Advanced features (collab, AI, forms) load on demand.

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Docker image size | < 100MB |
| Cold start time | < 2 seconds |
| WASM load time (documents) | < 1 second on 3G |
| Concurrent editors per room | 50+ |
| Document open time (10 pages) | < 100ms |
| Format round-trip fidelity | > 95% for common DOCX features |
| Test coverage | > 1,400 tests |
| Stars (6 months) | 1,000+ |
| Monthly active deployments | 100+ |
