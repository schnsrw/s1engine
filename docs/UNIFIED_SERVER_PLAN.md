# Unified Server Architecture — Implementation Plan

> Single binary serving: static editor + REST API + WebSocket collab + admin.
> **Status: ALL PHASES COMPLETE**
> Last updated: 2026-03-19

## Phase Status

| Phase | Description | Status |
|-------|-------------|--------|
| **A** | Static files + File sessions + CRUD + Docker + .env | **DONE** |
| **B** | WebSocket editing with file sessions + snapshot delivery | **DONE** |
| **C** | JWT integration + callbacks to host product | **DONE** |
| **D** | Single Dockerfile.unified (195MB image) | **DONE** |
| **E** | Admin panel with login form + dashboard + session mgmt | **DONE** |

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    s1-server (single binary)              │
│                                                          │
│  GET /           → Editor HTML/JS/CSS/WASM (ServeDir)    │
│  GET /health     → {"status":"ok"}                       │
│  POST /api/v1/files → Upload file, create session        │
│  GET /api/v1/files  → List active sessions               │
│  POST /api/v1/convert → Stateless format conversion      │
│  WS /ws/edit/{id}   → Collaborative editing              │
│  WS /ws/collab/{id} → (alias)                            │
│  GET /edit?token=jwt → Integration entry point            │
│  GET /admin/login    → Admin login form                   │
│  GET /admin/dashboard → Admin monitoring panel            │
│                                                          │
│  FileSessionManager: temp files with TTL + editor tracking│
│  RoomManager: WebSocket rooms with ops log + broadcast    │
│  StorageBackend: LocalFS / Memory                         │
│  WebhookRegistry: event dispatch to external URLs         │
│  AdminPanel: cookie auth + stats + session management     │
└──────────────────────────────────────────────────────────┘
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | / | Editor UI (static files) |
| GET | /health | Health check |
| POST | /api/v1/files | Upload file → fileId + editorUrl |
| GET | /api/v1/files | List active sessions |
| GET | /api/v1/files/{id} | Session info (editors, status) |
| GET | /api/v1/files/{id}/download | Download latest bytes |
| POST | /api/v1/files/{id}/save | Trigger save callback to host |
| DELETE | /api/v1/files/{id} | Force close session |
| POST | /api/v1/convert | Stateless format conversion |
| POST | /api/v1/documents | Upload to persistent storage |
| GET | /api/v1/documents | List stored documents |
| POST | /api/v1/webhooks | Register webhook |
| GET | /api/v1/info | Server info |
| GET | /edit?token=jwt | Integration entry (JWT auth) |
| WS | /ws/edit/{fileId} | WebSocket collab |
| WS | /ws/collab/{fileId} | WebSocket collab (alias) |
| GET | /admin/login | Admin login form |
| POST | /admin/login | Admin login submit |
| GET | /admin/dashboard | Admin dashboard |
| GET | /admin/api/stats | Server statistics |
| GET | /admin/api/sessions | List all sessions |
| DELETE | /admin/api/sessions/{id} | Force close session |
| GET | /admin/api/config | Server config (sanitized) |
| GET | /admin/logout | Clear admin session |

## Configuration (.env)

```bash
S1_PORT=8080
S1_STORAGE=local        # or "memory"
S1_DATA_DIR=/data
S1_STATIC_DIR=/app/public
S1_ADMIN_USER=admin
S1_ADMIN_PASS=changeme
S1_JWT_SECRET=           # for integration mode
S1_AUTH_ENABLED=false
RUST_LOG=s1_server=info
```

## Docker

```bash
docker compose up -d
# Single container: http://localhost:8080
```
