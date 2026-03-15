# REST API Specification

## Base URL

```
https://{host}/api/v1
```

All endpoints are versioned under `/api/v1`. Future breaking changes will increment to `/api/v2`.

## Authentication

Every request must include one of:

```
Authorization: Bearer <jwt_token>
X-API-Key: sk_live_<api_key>
```

Anonymous access can be enabled per-deployment (read-only, rate-limited).

## Common Headers

### Request Headers
| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes* | Bearer token (JWT) |
| `X-API-Key` | Yes* | API key (alternative to JWT) |
| `Content-Type` | Yes (POST/PUT) | `application/json` or `multipart/form-data` |
| `X-Tenant-Id` | No | Explicit tenant (overrides JWT `tenant_id` claim) |
| `X-Request-Id` | No | Client-generated request ID for tracing |
| `If-Match` | No | ETag for optimistic concurrency (PUT/DELETE) |

*One of `Authorization` or `X-API-Key` is required.

### Response Headers
| Header | Description |
|--------|-------------|
| `X-Request-Id` | Request ID (echoed or server-generated) |
| `X-RateLimit-Limit` | Rate limit ceiling |
| `X-RateLimit-Remaining` | Remaining requests in window |
| `X-RateLimit-Reset` | Unix timestamp when limit resets |
| `ETag` | Entity tag for caching/concurrency |

## Error Format

All errors return JSON:

```json
{
  "error": {
    "code": "DOCUMENT_NOT_FOUND",
    "message": "Document with ID 'abc-123' was not found",
    "details": {},
    "request_id": "req_abc123"
  }
}
```

### Error Codes

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `INVALID_REQUEST` | Malformed request body or parameters |
| 400 | `INVALID_FORMAT` | Unsupported or unrecognized document format |
| 400 | `FILE_TOO_LARGE` | File exceeds maximum size |
| 401 | `UNAUTHORIZED` | Missing or invalid authentication |
| 403 | `FORBIDDEN` | Insufficient permissions |
| 404 | `DOCUMENT_NOT_FOUND` | Document does not exist |
| 404 | `WEBHOOK_NOT_FOUND` | Webhook does not exist |
| 409 | `CONFLICT` | Optimistic concurrency conflict (ETag mismatch) |
| 413 | `PAYLOAD_TOO_LARGE` | Request body exceeds limit |
| 415 | `UNSUPPORTED_MEDIA_TYPE` | Content-Type not supported |
| 422 | `CONVERSION_FAILED` | Format conversion failed |
| 429 | `RATE_LIMITED` | Too many requests |
| 500 | `INTERNAL_ERROR` | Server error |
| 503 | `SERVICE_UNAVAILABLE` | Server is overloaded or in maintenance |

---

## Endpoints

### Health

#### `GET /health`

Health check. No authentication required.

**Response** `200 OK`:
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "storage": "ok",
  "collab_rooms": 12
}
```

#### `GET /health/ready`

Readiness check (for Kubernetes). Returns 200 when ready to accept traffic, 503 otherwise.

---

### Documents

#### `POST /api/v1/documents`

Create a new document.

**Option A: Upload file**
```
Content-Type: multipart/form-data

file: <binary data>
title: "My Document" (optional)
format: "docx" (optional, auto-detected)
metadata: '{"tags":["invoice"]}' (optional)
```

**Option B: Create empty**
```json
Content-Type: application/json

{
  "title": "Untitled Document",
  "format": "docx",
  "metadata": { "tags": ["draft"] }
}
```

**Response** `201 Created`:
```json
{
  "id": "doc_7a8b9c0d",
  "title": "My Document",
  "format": "docx",
  "size_bytes": 12345,
  "page_count": 3,
  "word_count": 450,
  "created_at": "2026-03-16T10:00:00Z",
  "updated_at": "2026-03-16T10:00:00Z",
  "created_by": "user-123",
  "version": 1,
  "metadata": { "tags": ["invoice"] },
  "etag": "\"a1b2c3d4\""
}
```

---

#### `GET /api/v1/documents`

List documents (paginated).

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `cursor` | string | — | Cursor for pagination (from previous response) |
| `limit` | integer | 20 | Items per page (max 100) |
| `format` | string | — | Filter by format: `docx`, `odt`, `txt`, `md` |
| `search` | string | — | Full-text search on title and metadata |
| `sort` | string | `updated_at` | Sort field: `created_at`, `updated_at`, `title` |
| `order` | string | `desc` | Sort order: `asc`, `desc` |
| `created_after` | ISO8601 | — | Filter by creation date |
| `created_before` | ISO8601 | — | Filter by creation date |
| `tag` | string | — | Filter by metadata tag |

**Response** `200 OK`:
```json
{
  "documents": [
    {
      "id": "doc_7a8b9c0d",
      "title": "My Document",
      "format": "docx",
      "size_bytes": 12345,
      "page_count": 3,
      "word_count": 450,
      "created_at": "2026-03-16T10:00:00Z",
      "updated_at": "2026-03-16T10:05:00Z",
      "created_by": "user-123",
      "version": 2,
      "metadata": { "tags": ["invoice"] }
    }
  ],
  "next_cursor": "eyJpZCI6ImRvY18xMjM0In0=",
  "has_more": true,
  "total_count": 42
}
```

---

#### `GET /api/v1/documents/:id`

Get document metadata.

**Response** `200 OK`:
```json
{
  "id": "doc_7a8b9c0d",
  "title": "My Document",
  "format": "docx",
  "size_bytes": 12345,
  "page_count": 3,
  "word_count": 450,
  "created_at": "2026-03-16T10:00:00Z",
  "updated_at": "2026-03-16T10:05:00Z",
  "created_by": "user-123",
  "version": 2,
  "metadata": { "tags": ["invoice"] },
  "permissions": {
    "current_user": "editor"
  },
  "collab": {
    "active": true,
    "room_id": "room_abc123",
    "peer_count": 3
  },
  "etag": "\"b2c3d4e5\""
}
```

---

#### `GET /api/v1/documents/:id/content`

Download raw document bytes.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `format` | string | original | Download in specific format (triggers conversion) |

**Response** `200 OK`:
```
Content-Type: application/vnd.openxmlformats-officedocument.wordprocessingml.document
Content-Disposition: attachment; filename="My Document.docx"

<binary bytes>
```

---

#### `PUT /api/v1/documents/:id`

Replace document content.

**Request:**
```
Content-Type: multipart/form-data
If-Match: "a1b2c3d4"

file: <binary data>
```

**Response** `200 OK`: Updated document metadata (same format as GET).

**Response** `409 Conflict`: ETag mismatch (document was modified since last GET).

---

#### `PATCH /api/v1/documents/:id/metadata`

Update metadata only (no content change).

**Request:**
```json
{
  "title": "Updated Title",
  "metadata": { "tags": ["invoice", "final"] }
}
```

**Response** `200 OK`: Updated document metadata.

---

#### `DELETE /api/v1/documents/:id`

Delete a document.

**Response** `204 No Content`

---

### Format Conversion

#### `POST /api/v1/convert`

Stateless format conversion. Upload a file, get it back in another format.

**Request:**
```
Content-Type: multipart/form-data

file: <binary data>
from: "docx" (optional, auto-detected)
to: "pdf" (required)
options: '{"page_size":"a4","margins":{"top":72}}' (optional)
```

**Response** `200 OK`:
```
Content-Type: application/pdf
Content-Disposition: attachment; filename="document.pdf"

<binary bytes>
```

**Supported conversion paths:**

| From | To |
|------|-----|
| DOCX | PDF, ODT, TXT, MD, HTML |
| ODT | PDF, DOCX, TXT, MD, HTML |
| TXT | DOCX, ODT, PDF |
| MD | DOCX, ODT, PDF, HTML |

**Conversion options:**
```json
{
  "page_size": "letter",        // "letter", "a4", "legal", or custom {width, height}
  "margins": {                  // in points (72pt = 1 inch)
    "top": 72,
    "bottom": 72,
    "left": 72,
    "right": 72
  },
  "font_family": "Arial",      // Default font for TXT/MD input
  "font_size": 12,              // Default font size in points
  "include_headers": true,      // Include headers/footers in output
  "include_images": true,       // Include images (false = text only)
  "pdf_quality": "standard"     // "draft", "standard", "high"
}
```

---

#### `POST /api/v1/documents/:id/export/:format`

Export a stored document to a different format.

**Path parameters:**
- `:id` — Document ID
- `:format` — Target format: `pdf`, `docx`, `odt`, `txt`, `md`, `html`

**Request body** (optional, conversion options):
```json
{
  "page_size": "a4"
}
```

**Response** `200 OK`: Converted file bytes with appropriate Content-Type.

---

#### `GET /api/v1/documents/:id/preview`

Get HTML preview of a document (paginated).

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | — | Specific page number (1-indexed) |
| `pages` | string | `all` | Page range: `1-5`, `1,3,5`, `all` |

**Response** `200 OK`:
```json
{
  "html": "<div class='s1-page'>...</div>",
  "page_count": 5,
  "pages_rendered": [1, 2, 3, 4, 5]
}
```

---

#### `GET /api/v1/documents/:id/text`

Extract plain text from a document.

**Response** `200 OK`:
```json
{
  "text": "Full document text content...",
  "word_count": 450,
  "char_count": 2340
}
```

---

#### `GET /api/v1/documents/:id/thumbnail`

Get PNG thumbnail of the first page.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | 1 | Page number |
| `width` | integer | 200 | Thumbnail width in pixels |
| `quality` | integer | 80 | JPEG quality (1-100) |

**Response** `200 OK`:
```
Content-Type: image/png

<binary PNG data>
```

---

### Sharing & Permissions

#### `GET /api/v1/documents/:id/permissions`

List all permissions for a document.

**Response** `200 OK`:
```json
{
  "permissions": [
    {
      "user_id": "user-123",
      "role": "owner",
      "granted_at": "2026-03-16T10:00:00Z",
      "granted_by": "user-123"
    },
    {
      "user_id": "user-456",
      "role": "editor",
      "granted_at": "2026-03-16T10:05:00Z",
      "granted_by": "user-123"
    }
  ]
}
```

#### `POST /api/v1/documents/:id/permissions`

Grant permission to a user.

**Request:**
```json
{
  "user_id": "user-789",
  "role": "viewer"
}
```

**Response** `201 Created`: New permission entry.

#### `DELETE /api/v1/documents/:id/permissions/:user_id`

Revoke a user's permission.

**Response** `204 No Content`

---

### Collaboration Rooms

#### `POST /api/v1/rooms`

Create a collaboration room for a document.

**Request:**
```json
{
  "document_id": "doc_7a8b9c0d",
  "max_peers": 25,
  "ttl_minutes": 480
}
```

**Response** `201 Created`:
```json
{
  "room_id": "room_x1y2z3",
  "document_id": "doc_7a8b9c0d",
  "websocket_url": "wss://host/ws/collab/room_x1y2z3",
  "token": "eyJ...",
  "created_at": "2026-03-16T10:00:00Z",
  "expires_at": "2026-03-16T18:00:00Z",
  "max_peers": 25
}
```

#### `GET /api/v1/rooms/:id`

Get room status.

**Response** `200 OK`:
```json
{
  "room_id": "room_x1y2z3",
  "document_id": "doc_7a8b9c0d",
  "status": "active",
  "peers": [
    { "peer_id": "p1", "user_name": "Alice", "connected_at": "..." },
    { "peer_id": "p2", "user_name": "Bob", "connected_at": "..." }
  ],
  "peer_count": 2,
  "created_at": "2026-03-16T10:00:00Z"
}
```

#### `DELETE /api/v1/rooms/:id`

Close a room (disconnects all peers).

**Response** `204 No Content`

---

### Webhooks

#### `POST /api/v1/webhooks`

Register a webhook.

**Request:**
```json
{
  "url": "https://example.com/webhook",
  "events": ["document.created", "document.updated", "document.deleted"],
  "secret": "whsec_mySecret123"
}
```

**Response** `201 Created`:
```json
{
  "id": "wh_abc123",
  "url": "https://example.com/webhook",
  "events": ["document.created", "document.updated", "document.deleted"],
  "status": "active",
  "created_at": "2026-03-16T10:00:00Z"
}
```

#### `GET /api/v1/webhooks`

List all webhooks.

#### `DELETE /api/v1/webhooks/:id`

Delete a webhook.

#### Webhook Payload Format

```json
{
  "id": "evt_12345",
  "event": "document.updated",
  "timestamp": "2026-03-16T10:05:00Z",
  "data": {
    "document_id": "doc_7a8b9c0d",
    "title": "My Document",
    "updated_by": "user-123",
    "version": 3
  }
}
```

**Signature verification:**

```
X-S1-Signature: sha256=<HMAC-SHA256(payload, secret)>
X-S1-Timestamp: 1700000000
```

Verify: `HMAC-SHA256(timestamp + "." + body, webhook_secret)`

---

### Admin

#### `GET /api/v1/admin/stats`

Server statistics (requires admin role).

**Response** `200 OK`:
```json
{
  "documents": {
    "total": 1234,
    "by_format": { "docx": 800, "odt": 200, "txt": 150, "md": 84 }
  },
  "storage": {
    "total_bytes": 5368709120,
    "backend": "s3"
  },
  "collab": {
    "active_rooms": 12,
    "connected_peers": 34
  },
  "api": {
    "requests_last_hour": 5600,
    "errors_last_hour": 12
  }
}
```

---

## Pagination

List endpoints use cursor-based pagination:

```
GET /api/v1/documents?limit=20
→ { "documents": [...], "next_cursor": "abc123", "has_more": true }

GET /api/v1/documents?limit=20&cursor=abc123
→ { "documents": [...], "next_cursor": "def456", "has_more": true }

GET /api/v1/documents?limit=20&cursor=def456
→ { "documents": [...], "next_cursor": null, "has_more": false }
```

**Why cursor-based (not offset-based):**
- Stable under concurrent inserts/deletes
- Better performance on large datasets
- No "page drift" when items are added/removed

## Rate Limiting

Default limits (configurable per deployment):

| Scope | Limit | Window |
|-------|-------|--------|
| Per API key | 60 requests | 1 minute |
| Per API key (burst) | 10 requests | 1 second |
| File upload | 10 requests | 1 minute |
| Conversion | 5 requests | 1 minute |
| Anonymous | 10 requests | 1 minute |

Rate limit headers are included in every response.

## Idempotency

For POST requests, include `Idempotency-Key` header to prevent duplicate creation:

```
POST /api/v1/documents
Idempotency-Key: unique-request-id-123
```

The server caches results for 24 hours by idempotency key.

## Content Types

| Format | MIME Type |
|--------|-----------|
| DOCX | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` |
| ODT | `application/vnd.oasis.opendocument.text` |
| PDF | `application/pdf` |
| TXT | `text/plain; charset=utf-8` |
| MD | `text/markdown; charset=utf-8` |
| HTML | `text/html; charset=utf-8` |
| JSON | `application/json` |
