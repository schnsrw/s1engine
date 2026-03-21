# Server API Specification v1.0

> Covers: REST endpoints, WebSocket protocol, authentication, file sessions, admin panel, error handling.
> Server: `s1-server` (Rust/Axum, single binary)
> Last updated: 2026-03-21

## 1. Architecture

```
s1-server (single binary)
├── GET /             Editor static files (ServeDir)
├── GET /health       Health check
├── GET /edit         JWT integration entry point
├── /api/v1/*         REST API
├── /ws/edit/{id}     WebSocket collaboration
├── /ws/collab/{id}   WebSocket collaboration (alias)
└── /admin/*          Admin panel (cookie auth)
```

**Stack:** Axum + Tokio + tower-http (CORS, tracing, body limit)
**Max upload:** 64MB (configurable via `DefaultBodyLimit`)

## 2. REST API Endpoints

### 2.1 Health & Info

#### GET /health

Health check endpoint.

**Response:** `200 OK`
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

#### GET /api/v1/info

Server information and supported formats.

**Response:** `200 OK`
```json
{
  "name": "s1-server",
  "version": "0.1.0",
  "engine_version": "1.0.1",
  "supported_formats": {
    "read": ["docx", "odt", "txt", "md", "doc"],
    "write": ["docx", "odt", "pdf", "txt", "md"]
  }
}
```

### 2.2 File Sessions (Temporary Editing)

File sessions are temporary document editing sessions with TTL. They are the primary way to use s1-server for document editing.

#### POST /api/v1/files

Upload a file and create an editing session.

**Request:** `multipart/form-data`
- `file` (required): The document file

**Response:** `201 Created`
```json
{
  "fileId": "uuid-string",
  "filename": "report.docx",
  "size": 45231,
  "wordCount": 1520,
  "editorUrl": "/?file=uuid-string",
  "wsUrl": "/ws/edit/uuid-string"
}
```

**Errors:**
| Code | Condition |
|------|-----------|
| 400 | No file uploaded |
| 400 | Invalid document (s1engine cannot parse) |

#### GET /api/v1/files

List all active file sessions.

**Response:** `200 OK`
```json
{
  "files": [
    {
      "file_id": "uuid",
      "filename": "report.docx",
      "format": "docx",
      "size": 45231,
      "editor_count": 2,
      "editors": [
        {
          "user_id": "u-abc",
          "user_name": "Alice",
          "mode": "edit",
          "last_activity": "2026-03-21T10:30:00Z"
        }
      ],
      "mode": "edit",
      "created_at_secs_ago": 300,
      "status": "editing"
    }
  ],
  "total": 1
}
```

**Session status values:** `editing` (editors connected), `idle` (no editors, within TTL), `expired` (past TTL).

#### GET /api/v1/files/{id}

Get session info for a specific file.

**Response:** `200 OK` (same shape as list item above)

**Errors:** `404` if session not found.

#### GET /api/v1/files/{id}/download

Download the latest version of the file being edited.

**Response:** `200 OK` with document bytes
- `Content-Type`: format-appropriate MIME type
- `Content-Disposition`: `attachment; filename="original-name.docx"`

**Errors:** `404` if session not found.

#### DELETE /api/v1/files/{id}

Force close a file editing session. Returns the final document bytes.

**Response:** `200 OK` with final document bytes.

**Errors:**
| Code | Condition |
|------|-----------|
| 403 | Session is view-only |
| 404 | Session not found |

#### POST /api/v1/files/{id}/save

Trigger save callback to the host product (integration mode).

**Response:** `200 OK`
```json
{
  "fileId": "uuid",
  "callbackUrl": "https://host.example.com/save",
  "callbackStatus": 200,
  "bytesSent": 45231
}
```

**Errors:**
| Code | Condition |
|------|-----------|
| 400 | No callback URL configured for this session |
| 404 | Session not found |
| 502 | Callback POST to host failed |

### 2.3 Documents (Persistent Storage)

Documents are stored persistently (local filesystem or memory).

#### POST /api/v1/documents

Upload a document to persistent storage.

**Request:** `multipart/form-data`
- `file` (required): The document file

**Response:** `201 Created`
```json
{
  "id": "uuid",
  "filename": "report.docx",
  "size": 45231,
  "wordCount": 1520,
  "title": "Q4 Report"
}
```

#### GET /api/v1/documents

List stored documents (paginated).

**Query parameters:**
| Param | Default | Description |
|-------|---------|-------------|
| `page` | 1 | Page number |
| `page_size` | 20 | Results per page |

**Response:** `200 OK`
```json
{
  "documents": [
    {
      "id": "uuid",
      "filename": "report.docx",
      "format": "docx",
      "size": 45231,
      "title": "Q4 Report",
      "author": "Engineering",
      "word_count": 1520,
      "created_at": "2026-03-21T10:00:00Z",
      "updated_at": "2026-03-21T10:30:00Z"
    }
  ],
  "total": 42,
  "page": 1,
  "pageSize": 20
}
```

#### GET /api/v1/documents/{id}

Get document metadata.

**Response:** `200 OK` (same shape as list item)

**Errors:** `404` if not found.

#### GET /api/v1/documents/{id}/content

Download document bytes.

**Response:** `200 OK` with document bytes and appropriate `Content-Type`.

#### DELETE /api/v1/documents/{id}

Delete a document.

**Response:** `204 No Content`

**Errors:** `404` if not found.

#### GET /api/v1/documents/{id}/thumbnail

Get a thumbnail of the first page (currently returns full PDF).

**Response:** `200 OK` with PDF bytes.

### 2.4 Format Conversion

#### POST /api/v1/convert

Stateless format conversion. Upload a document, get it back in a different format.

**Request:** `multipart/form-data`
- `file` (required): Source document
- `format` (optional, default: `"pdf"`): Target format (`docx`, `odt`, `pdf`, `txt`, `md`)

**Response:** `200 OK` with converted document bytes and appropriate `Content-Type`.

**Errors:**
| Code | Condition |
|------|-----------|
| 400 | No file uploaded |
| 400 | Invalid/unparseable document |
| 400 | Unsupported target format |
| 500 | Export failed |

### 2.5 Webhooks

#### POST /api/v1/webhooks

Register a webhook.

**Request body:**
```json
{
  "url": "https://example.com/webhook",
  "events": ["session.created", "session.closed"]
}
```

If `events` is omitted, defaults to `["*"]` (all events).

**Response:** `201 Created`
```json
{
  "id": "uuid"
}
```

#### GET /api/v1/webhooks

List all webhooks.

**Response:** `200 OK`
```json
{
  "webhooks": [
    {
      "id": "uuid",
      "url": "https://example.com/webhook",
      "events": ["*"],
      "active": true,
      "created_at": "2026-03-21T10:00:00Z"
    }
  ],
  "total": 1
}
```

#### DELETE /api/v1/webhooks/{id}

Delete a webhook.

**Response:** `204 No Content`

**Errors:** `404` if not found.

### 2.6 Error Reporting

#### POST /api/v1/errors

Report a client-side error (for monitoring/observability).

**Request body:**
```json
{
  "message": "Error description (max 500 chars)"
}
```

**Response:** `204 No Content`

Messages are logged at WARN level, truncated to 500 characters.

## 3. WebSocket Protocol

WebSocket endpoints: `/ws/edit/{fileId}` and `/ws/collab/{fileId}` (alias).

**Connection URL:**
```
ws://host:port/ws/edit/{fileId}?user=Alice&uid=u-abc&access=edit
```

Query parameters:
| Param | Required | Description |
|-------|----------|-------------|
| `user` | No | Display name |
| `uid` | No | Unique user ID |
| `access` | No | Access level: `edit`, `comment`, `view` |

For the full WebSocket message protocol, see [COLLABORATION_PROTOCOL.md](../COLLABORATION_PROTOCOL.md). Key message types:

| Type | Direction | Purpose |
|------|-----------|---------|
| `joined` | Server -> Client | Welcome message with room state |
| `snapshot` | Server -> Client | Document snapshot (base64 DOCX) |
| `op` | Bidirectional | CRDT operations / fullSync |
| `awareness` | Bidirectional | Cursor/presence heartbeat (every 500ms) |
| `join` | Client -> Server | Announce presence |
| `peer-join` | Server -> Client | New peer connected |
| `peer-leave` | Server -> Client | Peer disconnected |

## 4. Authentication

### 4.1 Authentication Methods

| Method | Header / Mechanism | Use Case |
|--------|-------------------|----------|
| JWT (HS256) | `Authorization: Bearer <token>` | API access |
| API Key | `X-API-Key: <key>` | Machine-to-machine |
| Integration JWT | `GET /edit?token=<jwt>` | Embedding in host products |
| Cookie | `s1_admin=<base64(user:pass)>` | Admin panel only |
| Anonymous | (no credentials) | When `S1_AUTH_ENABLED=false` |

### 4.2 JWT Claims

**Standard API JWT (`Authorization: Bearer`):**

```json
{
  "sub": "user-id",
  "name": "Alice",
  "tenant_id": "org-123",
  "role": "user",
  "exp": 1711000000
}
```

**Integration JWT (`/edit?token=`):**

```json
{
  "file_id": "uuid",
  "user_id": "user-123",
  "user_name": "Alice",
  "permissions": "edit",
  "download_url": "https://host.example.com/files/123.docx",
  "callback_url": "https://host.example.com/save/123",
  "exp": 1711000000
}
```

### 4.3 Authentication Flow

```
Client                         s1-server
  │                               │
  ├── Authorization: Bearer JWT ──►│
  │                               ├── Decode header.payload
  │                               ├── Verify HS256 signature
  │                               ├── Check exp > now
  │                               ├── Extract user claims
  │                               ├── Inject AuthUser into request
  │                               │
  │◄── 200 OK ────────────────────┤  (success)
  │◄── 401 Unauthorized ─────────┤  (invalid/expired)
```

When `S1_AUTH_ENABLED=false` (default), all requests get anonymous access with `UserRole::Anonymous`.

### 4.4 API Key Authentication

API keys are configured in the server config. Each key maps to a user identity:

```
X-API-Key: sk-abc123 → { user_id: "apikey:MyService", role: User }
```

### 4.5 User Roles

| Role | Description |
|------|-------------|
| `Admin` | Full access to everything |
| `User` | Standard authenticated user |
| `Anonymous` | Unauthenticated, view-only |

### 4.6 Endpoint Authentication Requirements

| Endpoint | Auth Required | Minimum Permission | Enforced Today |
|----------|--------------|-------------------|----------------|
| GET /health | No | None | Yes (public) |
| POST /api/v1/files | No* | None | No enforcement |
| GET /api/v1/files | No* | Viewer | No enforcement |
| GET /api/v1/files/{id} | No* | Viewer | No enforcement |
| GET /api/v1/files/{id}/download | No* | Viewer | No enforcement |
| DELETE /api/v1/files/{id} | Yes | Editor | Partial (mode check) |
| POST /api/v1/convert | No | None | Yes (stateless) |
| WS /ws/edit/{id} | No* | Viewer | No enforcement |
| GET /admin/* | Yes | Admin | Yes (cookie auth) |
| POST /api/v1/errors | No | None | Yes (rate limited) |

*When S1_AUTH_ENABLED=true, auth middleware applies. Currently disabled by default.

## 5. Authorization (Per-Document Permissions)

### 5.1 Permission Levels

| Level | Value | Capabilities |
|-------|-------|-------------|
| Viewer | 0 | Read document, download |
| Commenter | 1 | View + add comments |
| Editor | 2 | View + comment + edit |
| Admin | 3 | Editor + manage permissions |
| Owner | 4 | Full control (delete, transfer) |

### 5.2 Permission Resolution

```
1. Admin role  → full access (all permissions granted)
2. Anonymous   → Viewer only
3. Session owner (user_id == session.owner_id) → Owner
4. Session mode:
   - "view"    → Viewer
   - "comment" → Commenter
   - "edit"    → Editor
5. Authenticated user (no session context) → Editor (default)
```

## 6. File Session Lifecycle

```
                    ┌───────────────────┐
                    │  POST /api/v1/files │
                    │  (upload file)      │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │   Session Created  │
                    │   status: idle     │
                    │   TTL: 5 min       │
                    └─────────┬─────────┘
                              │
              WS connect      │
              ┌───────────────┘
              ▼
    ┌───────────────────┐
    │  Editors Connected │◄──── More editors join
    │  status: editing   │
    │  TTL paused        │────► Editors leave
    └─────────┬─────────┘
              │
              │ All editors left
              ▼
    ┌───────────────────┐
    │  Grace Period       │
    │  status: idle       │
    │  TTL: 5 min         │
    └─────────┬─────────┘
              │
              │ TTL expires (no reconnect)
              ▼
    ┌───────────────────┐
    │  Session Expired    │
    │  Cleanup runs       │
    │  Callback POST      │──► Host product receives final bytes
    │  Session removed    │
    └───────────────────┘
```

**Timing constants:**
| Constant | Value | Description |
|----------|-------|-------------|
| `DEFAULT_SESSION_TTL_SECS` | 300 (5 min) | Grace period after last editor leaves |
| `SNAPSHOT_INTERVAL_SECS` | 30 | How often to snapshot dirty rooms |
| Stale editor cleanup | 60s interval | Removes editors with >5 min inactivity |
| Expired session cleanup | 60s interval | Removes sessions past TTL |

### 6.1 Callback on Session Close

When a session expires or is force-closed and has a `callback_url` configured:

```
POST {callback_url}
Content-Type: application/octet-stream
X-S1-File-Id: {fileId}
X-S1-Event: session.closed
Body: [final document bytes]
```

**Security**: Callback requests have no authentication or integrity protection. See section 7.4 for required HMAC signature additions, and SECURITY_SPEC.md section 2.4 for full SSRF/exfiltration analysis.

## 7. Integration Mode

### 7.1 Flow

```
Host Product                   s1-server                    Browser
    │                              │                           │
    ├── Generate JWT ─────────────►│                           │
    │   (file_id, user,            │                           │
    │    download_url,             │                           │
    │    callback_url)             │                           │
    │                              │                           │
    ├── Redirect user to ─────────►│                           │
    │   /edit?token=JWT            │                           │
    │                              ├── Validate JWT            │
    │                              ├── Fetch document from     │
    │                              │   download_url            │
    │                              ├── Create file session     │
    │                              │                           │
    │                              ├── 302 → /?file={id} ─────►│
    │                              │                           ├── Load editor
    │                              │                           ├── Connect WS
    │                              │                           ├── Edit document
    │                              │                           │
    │                              │◄── Save trigger ──────────┤
    │◄── POST callback_url ───────┤                           │
    │   (final document bytes)     │                           │
```

### 7.2 Integration JWT Claims

| Claim | Required | Description |
|-------|----------|-------------|
| `file_id` | Yes | Unique file identifier |
| `user_id` | Yes | User identifier |
| `user_name` | Yes | Display name |
| `permissions` | No (default: `edit`) | `edit`, `comment`, or `view` |
| `download_url` | No | URL to fetch the document from |
| `callback_url` | No | URL to POST final document to on save/close |
| `exp` | No | Expiration timestamp (Unix seconds) |

### 7.3 SSRF Risk: `download_url` and `callback_url`

Both `download_url` and `callback_url` cause the server to make outbound HTTP requests. This is a Server-Side Request Forgery (SSRF) vector.

**`download_url`**: Server performs `GET {download_url}` to fetch the document. An attacker can set this to an internal URL (e.g., `http://169.254.169.254/latest/meta-data/` on AWS) to probe internal services or steal cloud metadata.

**`callback_url`**: Server performs `POST {callback_url}` with the document bytes on save/close. An attacker can set this to exfiltrate document content to an external server, or probe internal services.

**Current protection**: Only a 30-second timeout. No IP validation, no domain allowlisting.

**Required mitigations** (see SECURITY_SPEC.md section 2.3 and 2.4 for full details):
1. Block requests to private/internal IP ranges (RFC 1918, link-local, loopback)
2. Domain allowlist via `S1_DOWNLOAD_ALLOWLIST` and `S1_CALLBACK_ALLOWLIST`
3. DNS rebinding protection (resolve hostname first, validate IP)
4. Sign callbacks with HMAC for receiver verification

### 7.4 Callback Security

The callback POST to `callback_url` on session close has no authentication or integrity protection.

**Current callback format:**
```
POST {callback_url}
Content-Type: application/octet-stream
X-S1-File-Id: {fileId}
X-S1-Event: session.closed
Body: [document bytes]
```

**Required additions:**
```
X-S1-Timestamp: {unix_seconds}
X-S1-Signature: HMAC-SHA256(fileId + timestamp + body, S1_JWT_SECRET)
```

The receiver should verify:
1. `X-S1-Timestamp` is within 5 minutes of current time (replay protection)
2. `X-S1-Signature` matches the expected HMAC (authenticity)
3. The source IP matches the known s1-server IP (network layer)

**NOT IMPLEMENTED**: Callbacks currently have no signature or timestamp.

## 8. Admin Panel

### 8.1 Authentication

Admin panel uses cookie-based authentication:

- Credentials set via `S1_ADMIN_USER` and `S1_ADMIN_PASS` environment variables
- Cookie: `s1_admin=base64(user:pass); Path=/admin; HttpOnly; SameSite=Strict`
- Login page at `/admin/login`; logout at `/admin/logout` (clears cookie)

### 8.2 Admin Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /admin/login | Login form |
| POST | /admin/login | Submit credentials |
| GET | /admin/logout | Clear session cookie |
| GET | /admin/dashboard | Dashboard page |
| GET | /admin/api/stats | Server statistics JSON |
| GET | /admin/api/sessions | List all sessions JSON |
| DELETE | /admin/api/sessions/{id} | Force close a session |
| GET | /admin/api/config | Sanitized server config JSON |

### 8.3 Stats Response

```json
{
  "uptime_secs": 3600,
  "version": "0.1.0",
  "active_sessions": 5,
  "active_rooms": 3,
  "total_editors": 8,
  "memory_mb": 45.2
}
```

### 8.4 Dashboard

The dashboard auto-refreshes every 10 seconds and displays:
- Uptime, session count, editor count, memory usage (card grid)
- Active sessions table (ID, filename, format, size, editors, last active, status, age, close button)
- Server configuration (sanitized, no secrets)

## 9. Error Codes and Responses

### 9.1 HTTP Error Response Format

All error responses are plain text strings (not JSON):

```
HTTP/1.1 400 Bad Request
Content-Type: text/plain

Invalid document: DOCX error: missing document.xml in archive
```

### 9.2 Error Code Reference

| HTTP Code | Meaning | Common Causes |
|-----------|---------|---------------|
| 400 | Bad Request | No file uploaded, invalid document, unsupported format, missing required field |
| 401 | Unauthorized | Invalid/expired JWT, invalid API key |
| 403 | Forbidden | Insufficient permissions, admin not configured, view-only session |
| 404 | Not Found | Document/session/webhook not found |
| 500 | Internal Server Error | Storage error, export failure |
| 502 | Bad Gateway | Callback URL unreachable, document fetch failed |

## 10. CORS Configuration

CORS is configured as permissive (`CorsLayer::permissive()`):

- All origins allowed
- All methods allowed
- All headers allowed
- Credentials allowed

This is appropriate for development and single-origin deployments. Production deployments should restrict to specific origins via configuration.

## 11. Rate Limiting

Rate limiting is not currently implemented at the server level. Recommendations for production:

| Endpoint Group | Recommended Limit |
|----------------|-------------------|
| File uploads | 10/min per IP |
| Format conversion | 20/min per IP |
| WebSocket connections | 5 concurrent per IP |
| Admin login | 5 attempts/min (brute force protection) |
| Error reporting | 60/min per IP |

Rate limiting should be implemented via a reverse proxy (nginx, Cloudflare) or an Axum middleware layer.

## 12. Configuration

### 12.1 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `S1_PORT` | `8080` | Server listen port |
| `S1_STORAGE` | `local` | Storage backend: `local` or `memory` |
| `S1_DATA_DIR` | `./data` | Local storage directory |
| `S1_STATIC_DIR` | `./public` | Static editor files directory |
| `S1_ADMIN_USER` | (none) | Admin panel username |
| `S1_ADMIN_PASS` | (none) | Admin panel password |
| `S1_JWT_SECRET` | (none) | JWT HS256 secret for integration mode |
| `S1_AUTH_ENABLED` | `false` | Enable API authentication |
| `RUST_LOG` | `s1_server=info` | Log level filter |

### 12.2 Config File

Alternatively, configure via `s1.toml`:

```toml
port = 8080
storage = "local"
data_dir = "./data"
max_upload_size = 67108864  # 64MB
```

Environment variables override config file values.

## 13. Background Tasks

| Task | Interval | Description |
|------|----------|-------------|
| Save dirty rooms | 30s | Snapshot collab room state to session storage |
| Session cleanup | 60s | Remove stale editors (>5 min inactive), expire sessions past TTL |
| Callback dispatch | On session expire | POST final document bytes to callback URL |

## 14. Implementation Status

| Feature | Status |
|---------|--------|
| File session CRUD | DONE |
| Document CRUD (persistent storage) | DONE |
| Format conversion endpoint | DONE |
| WebSocket collaboration | DONE |
| JWT authentication middleware | DONE |
| API key authentication | DONE |
| Integration mode (/edit?token=JWT) | DONE |
| Admin panel (login + dashboard) | DONE |
| Webhook registration | DONE |
| Error reporting endpoint | DONE |
| Background session cleanup | DONE |
| Background room save | DONE |
| Callback dispatch on session close | DONE |
| Rate limiting | NOT IMPLEMENTED |
| Per-route auth enforcement | NOT IMPLEMENTED (auth middleware exists but not wired to all routes) |
| S3 storage backend | NOT IMPLEMENTED |
| Binary WebSocket messages | NOT IMPLEMENTED |
