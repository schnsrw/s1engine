# Configuration

This guide covers all configuration options for both the Rudra Office client (browser editor) and the rudra-server backend. Configuration is split into two layers: the client-side `window.S1_CONFIG` object and server-side environment variables (or the `s1.toml` file).

---

## Client Configuration

The editor reads its configuration from `window.S1_CONFIG` at startup. Set this object in a `<script>` tag **before** the editor script loads. Any property not provided falls back to the default shown below.

```html
<script>
  window.S1_CONFIG = {
    // Server API URL (REST endpoints for document CRUD, conversion).
    // Empty string means the editor runs in offline/standalone mode.
    apiUrl: '',              // e.g., 'http://localhost:8080/api/v1'

    // WebSocket URL for real-time collaboration.
    // Empty string disables automatic WebSocket connection.
    relayUrl: '',            // e.g., 'ws://localhost:8080/ws/collab'

    // Editor mode:
    //   'standalone' — full local editor, no server dependency
    //   'integrated' — expects a server backend for storage and collab
    mode: 'standalone',

    // Automatically connect to the collaboration room on page load.
    // Requires relayUrl to be set and a ?room= parameter in the URL.
    autoCollab: true,

    // Branding — product name displayed in the title bar and dialogs.
    productName: 'Rudra Office',

    // ── Feature Flags ──────────────────────────────────
    enableCollab: true,      // Show the Share button and collaboration UI
    enablePdfViewer: true,   // Allow opening and viewing PDF files
    enableTemplates: true,   // Show the template gallery on new document
    enableAI: true,          // Enable AI writing assistant panel

    // AI sidecar URL (llama.cpp HTTP server).
    // The AI assistant calls this endpoint for completions.
    aiUrl: '',               // e.g., 'http://localhost:8081'
  };
</script>
```

### Property Reference

| Property | Type | Default | Description |
|---|---|---|---|
| `apiUrl` | `string` | `''` | Base URL for the REST API. When empty, the editor operates in standalone mode with no server calls. |
| `relayUrl` | `string` | `''` | WebSocket URL for collaboration. When empty, real-time editing is disabled. |
| `mode` | `string` | `'standalone'` | `'standalone'` or `'integrated'`. Integrated mode enables server-backed file management. |
| `autoCollab` | `boolean` | `true` | Connect to the collaboration room automatically when the page loads. |
| `productName` | `string` | `'Rudra Office'` | Display name used in the title bar, dialogs, and branding. |
| `enableCollab` | `boolean` | `true` | Show or hide collaboration controls (Share button, peer indicators). |
| `enablePdfViewer` | `boolean` | `true` | Allow PDF files to be opened in the viewer. |
| `enableTemplates` | `boolean` | `true` | Show the template gallery when creating a new document. |
| `enableAI` | `boolean` | `true` | Enable the AI writing assistant sidebar. |
| `aiUrl` | `string` | `''` | HTTP endpoint for the AI sidecar (llama.cpp). When empty, AI features are unavailable even if `enableAI` is true. |

### Overriding Configuration at Runtime

Because `S1_CONFIG` is a plain JavaScript object on `window`, host applications can modify it before or after the editor loads:

```javascript
// Before editor init (in the embedding page)
window.S1_CONFIG = {
  apiUrl: 'https://docs-api.example.com/api/v1',
  relayUrl: 'wss://docs-api.example.com/ws/collab',
  mode: 'integrated',
  productName: 'Acme Docs',
  enableAI: false,
};
```

For integration mode, the server typically injects this configuration into the HTML response before serving the editor.

---

## Server Configuration

The rudra-server binary reads configuration from two sources, in priority order:

1. **`s1.toml`** file in the working directory (TOML format)
2. **Environment variables** prefixed with `S1_` (fallback when no TOML file is found)

Environment variables are the recommended approach for container deployments.

### Core Settings

| Variable | TOML Key | Default | Description |
|---|---|---|---|
| `S1_PORT` | `port` | `8080` | HTTP port the server listens on. |
| `S1_STORAGE` | `storage` | `local` | Storage backend: `local`, `memory`, or `s3`. |
| `S1_DATA_DIR` | `data_dir` | `./data` | Directory for local storage. Documents are stored as `{id}.bin` with `{id}.meta.json` sidecars. |
| `S1_STATIC_DIR` | -- | `./public` | Directory containing the built editor frontend (HTML, JS, CSS, WASM). |
| `RUST_LOG` | -- | `s1_server=info,tower_http=info` | Log level filter using the `tracing` EnvFilter syntax. |

#### Example `s1.toml`

```toml
port = 8080
storage = "local"
data_dir = "/var/lib/rudra/data"
max_upload_size = 67108864  # 64 MB
```

### Authentication

| Variable | Default | Description |
|---|---|---|
| `S1_AUTH_ENABLED` | `false` | Enable authentication middleware. When `false`, all endpoints are public. |
| `S1_JWT_SECRET` | -- | HMAC-SHA256 secret for validating JWT tokens. Required for integration mode (`/edit?token=`). |
| `S1_REQUIRE_JWT_EXP` | `false` | Reject JWT tokens that lack an `exp` (expiry) claim. Recommended for production. |
| `S1_ALLOW_ANONYMOUS` | `true` | Allow unauthenticated requests (Viewer-level access). |

When authentication is enabled, the server supports two credential methods:

- **JWT Bearer tokens** -- `Authorization: Bearer <token>`. The token payload must include `sub` (user ID) and `name`. Optional claims: `role` (`admin` or `user`), `tenant_id`, `exp`.
- **API keys** -- `X-API-Key: <key>`. Configured via the `api_keys` array in the auth config.

### Admin Panel

| Variable | Default | Description |
|---|---|---|
| `S1_ADMIN_USER` | -- | Admin panel username. Both `S1_ADMIN_USER` and `S1_ADMIN_PASS` must be set to enable the admin panel. |
| `S1_ADMIN_PASS` | -- | Admin panel password. |

The admin panel is served at `/admin/dashboard` and provides real-time monitoring of active sessions, connected editors, memory usage, and server configuration. Sessions are protected by a cookie with a 1-hour timeout. Login attempts are rate-limited to 5 per IP per 60 seconds.

### Integration Mode

Integration mode allows embedding Rudra Office in another product. The host application generates a signed JWT and redirects users to `/edit?token=<jwt>`.

| Variable | Default | Description |
|---|---|---|
| `S1_JWT_SECRET` | -- | Shared secret between the host application and rudra-server for signing/validating integration JWTs. |
| `S1_CALLBACK_SECRET` | -- | HMAC-SHA256 key for signing callback request bodies. When set, the server includes an `X-S1-Signature: sha256=<hex>` header on all callback POST requests. |

#### Integration JWT Claims

| Claim | Required | Description |
|---|---|---|
| `file_id` | Yes | Unique identifier for the document. |
| `user_id` | Yes | Identifier of the editing user. |
| `user_name` | Yes | Display name shown in the collaboration UI. |
| `permissions` | No | Access level: `edit` (default), `comment`, or `view`. |
| `download_url` | No | URL from which the server fetches the document on first open. Subject to SSRF protection. |
| `callback_url` | No | URL the server POSTs the final document to when the editing session closes. |
| `exp` | No | Token expiry (Unix timestamp). Required if `S1_REQUIRE_JWT_EXP=true`. |

### Collaboration (WebSocket)

WebSocket endpoints are available at two URL patterns:

- `/ws/edit/{file_id}?user=NAME&uid=ID&mode=edit`
- `/ws/collab/{file_id}?user=NAME&uid=ID&mode=edit`

| Parameter | Default | Description |
|---|---|---|
| `user` | Random `User-XXXXXXXX` | Display name for presence. |
| `uid` | Random UUID (8 chars) | Unique peer identifier for this connection. |
| `mode` | `edit` | Editing mode for this peer. |
| `access` | `edit` | Access level: `edit` or `view`. View-only peers cannot send structural operations. |

The server manages collaboration rooms automatically:

- Rooms are created on first connection and destroyed when the last peer leaves.
- Dirty rooms (with unsaved operations) are flushed to storage every 30 seconds.
- Operation logs are capped at 10,000 entries per room, with oldest entries truncated when the limit is reached.
- A ping is sent every 15 seconds to detect dead connections.

### Storage Backends

#### Local Filesystem (default)

```bash
S1_STORAGE=local
S1_DATA_DIR=/data
```

Documents are stored as `{id}.bin` files with `{id}.meta.json` metadata sidecars. The directory is created automatically if it does not exist.

#### In-Memory

```bash
S1_STORAGE=memory
```

All data is held in memory and lost on restart. Suitable for development, testing, and stateless CI environments.

#### S3-Compatible (planned)

```bash
S1_STORAGE=s3
S1_S3_BUCKET=rudra-docs
S1_S3_REGION=us-east-1
S1_S3_ENDPOINT=https://s3.amazonaws.com
```

S3 storage uses the same `StorageBackend` trait interface. Any S3-compatible service (AWS S3, MinIO, DigitalOcean Spaces) can be used.

---

## Feature Flags

Feature flags control which UI components are visible and which server capabilities are active.

### Client-Side Flags

| Flag | Effect When Disabled |
|---|---|
| `enableCollab` | Hides the Share button, peer count indicator, and collaboration status bar. WebSocket connections are not initiated. |
| `enablePdfViewer` | PDF files cannot be opened. The editor shows an "unsupported format" message instead. |
| `enableTemplates` | The template gallery is hidden. New documents start with a blank page. |
| `enableAI` | The AI assistant panel and toolbar button are hidden. No requests are made to the AI sidecar. |

### Server-Side Flags

| Variable | Effect |
|---|---|
| `S1_AUTH_ENABLED=true` | Enforces JWT/API key authentication on all API endpoints. |
| `S1_REQUIRE_JWT_EXP=true` | Rejects tokens without an `exp` claim, preventing indefinitely-valid tokens. |
| `S1_ALLOW_ANONYMOUS=false` | Returns `401 Unauthorized` for requests without credentials. |

---

## URL Parameters

The editor reads several URL parameters to control its behavior on load.

| Parameter | Example | Description |
|---|---|---|
| `file` | `?file=abc-123` | Open an existing file session by ID. The editor fetches the document from the server and connects to the collaboration room. |
| `type` | `?type=sheet` | Hint the editor to open in spreadsheet mode (for multi-app routing). Values: `doc`, `sheet`, `slide`. |
| `room` | `?room=my-room` | Join a specific collaboration room. Used with `autoCollab: true`. |
| `mode` | `?mode=view` | Set the editor to a specific access mode: `edit`, `comment`, or `view`. |
| `token` | `/edit?token=JWT` | Integration mode entry point. The server validates the JWT and creates a file session. |

---

## Integration Callbacks

When a document editing session closes (all editors leave and the TTL expires, or an explicit save is triggered), the server sends the final document bytes to the configured callback URL.

### Callback Flow

1. Host application creates a file session via `/edit?token=JWT` with a `callback_url` in the JWT claims.
2. Users edit the document collaboratively.
3. When the session ends, the server POSTs the final document bytes to the callback URL.

### Callback Request Headers

| Header | Description |
|---|---|
| `Content-Type` | `application/octet-stream` |
| `X-S1-File-Id` | The file session ID. |
| `X-S1-Event` | Event type: `session.closed` (automatic) or `document.saved` (manual trigger via `POST /api/v1/files/{id}/save`). |
| `X-S1-Timestamp` | Unix timestamp of the callback. |
| `X-S1-Signature` | `sha256=<hex>` HMAC signature of the request body. Only present when `S1_CALLBACK_SECRET` is configured. |

### Manual Save Trigger

To trigger a callback before the session expires:

```bash
curl -X POST http://localhost:8080/api/v1/files/{file_id}/save
```

### SSRF Protection

All callback URLs and `download_url` values are checked against a blocklist before any outbound request is made. The following address ranges are blocked:

- Loopback: `127.0.0.0/8`, `::1`
- Private (RFC 1918): `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
- Link-local: `169.254.0.0/16`, `fe80::/10`
- Cloud metadata endpoints: `169.254.169.254`
- Hostnames: `localhost`, `*.local`, `*.internal`

### Signature Verification

When `S1_CALLBACK_SECRET` is set, verify the callback authenticity on the receiving end:

```python
import hmac, hashlib

def verify_signature(body: bytes, signature_header: str, secret: str) -> bool:
    expected = hmac.new(secret.encode(), body, hashlib.sha256).hexdigest()
    received = signature_header.removeprefix("sha256=")
    return hmac.compare_digest(expected, received)
```

---

## Webhooks

The server supports registering webhook endpoints that receive notifications for document lifecycle events.

### Supported Events

| Event | Trigger |
|---|---|
| `document.created` | A new document is uploaded or created. |
| `document.deleted` | A document is deleted from storage. |
| `document.exported` | A document is converted/exported to another format. |
| `*` | Wildcard: matches all events. |

### Registration

```bash
curl -X POST http://localhost:8080/api/v1/webhooks \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/hook", "events": ["document.created"]}'
```

Webhook delivery is asynchronous and fire-and-forget. Failed deliveries are logged but not retried.

---

## Complete Production Example

Below is a complete environment configuration suitable for a production deployment:

```bash
# Core
S1_PORT=8080
S1_STORAGE=local
S1_DATA_DIR=/data
S1_STATIC_DIR=/app/public

# Authentication
S1_AUTH_ENABLED=true
S1_JWT_SECRET=your-256-bit-secret-here
S1_REQUIRE_JWT_EXP=true
S1_ALLOW_ANONYMOUS=false

# Admin panel
S1_ADMIN_USER=admin
S1_ADMIN_PASS=a-strong-password-here

# Integration callbacks
S1_CALLBACK_SECRET=your-callback-signing-secret

# Logging
RUST_LOG=s1_server=info,tower_http=warn
```

The corresponding client configuration, injected by the server or embedding page:

```javascript
window.S1_CONFIG = {
  apiUrl: '/api/v1',
  relayUrl: 'wss://docs.example.com/ws/collab',
  mode: 'integrated',
  productName: 'Example Docs',
  enableCollab: true,
  enableAI: false,
};
```
