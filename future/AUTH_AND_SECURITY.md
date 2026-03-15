# Authentication, Authorization & Security

## Design Principles

1. **s1engine does NOT manage users** — consumers bring their own auth system
2. **JWT-based identity** — consumers generate JWTs with embedded permissions
3. **s1engine validates and enforces** — checks JWT, extracts permissions, gates access
4. **Defense in depth** — input validation, rate limiting, audit logging, encryption

---

## Authentication

### Authentication Flow

```
┌──────────┐     ┌──────────────┐     ┌─────────────┐
│ Consumer │     │  Consumer's  │     │  s1-server   │
│ Frontend │     │  Auth Server │     │              │
└────┬─────┘     └──────┬───────┘     └──────┬───────┘
     │                  │                     │
     │  1. Login        │                     │
     │─────────────────▶│                     │
     │                  │                     │
     │  2. JWT (with    │                     │
     │     s1 perms)    │                     │
     │◀─────────────────│                     │
     │                  │                     │
     │  3. API call with JWT                  │
     │───────────────────────────────────────▶│
     │                                        │
     │                  4. Validate JWT        │
     │                     Extract permissions │
     │                     Check access        │
     │                                        │
     │  5. Response (200 or 403)              │
     │◀───────────────────────────────────────│
```

### JWT Token Structure

s1engine expects JWTs with these claims:

```json
{
  // Standard claims
  "iss": "https://auth.acme.com",
  "sub": "user-123",
  "exp": 1700000000,
  "iat": 1699996400,

  // s1engine claims (in configurable namespace)
  "s1": {
    "tenant_id": "org-456",
    "user_name": "Alice Smith",
    "user_color": "#4285f4",
    "role": "member",
    "permissions": {
      "*": "viewer",
      "doc_abc": "editor",
      "doc_xyz": "owner"
    }
  }
}
```

**Claim namespace is configurable:**

```toml
# s1-server.toml
[auth.jwt]
claims_namespace = "s1"           # default
# Or for flat claims:
# claims_namespace = ""           # claims at root level

# Or map from existing claims:
[auth.jwt.claim_mapping]
tenant_id = "org_id"              # map "org_id" claim → tenant_id
user_name = "name"                # map "name" claim → user_name
permissions = "s1_permissions"    # map "s1_permissions" → permissions
```

This allows consumers to add s1engine permissions to their existing JWT structure without changing their auth server.

### JWT Validation

```toml
[auth.jwt]
# Algorithm
algorithm = "RS256"               # RS256, RS384, RS512, HS256, HS384, HS512

# For HMAC (HS256)
secret = "your-secret"            # or S1_AUTH_JWT_SECRET env var

# For RSA (RS256)
public_key_file = "/etc/s1/jwt-public-key.pem"
# Or fetch from JWKS endpoint (auto-rotating keys)
jwks_url = "https://auth.acme.com/.well-known/jwks.json"
jwks_refresh_interval = 3600      # seconds

# Validation
issuer = "https://auth.acme.com"  # optional, validate iss claim
audience = "s1engine"             # optional, validate aud claim
clock_skew = 30                   # seconds, tolerance for exp/nbf
```

### API Key Authentication

For server-to-server communication (no user context):

```
X-API-Key: sk_live_abc123def456
```

API keys are managed by the consumer and stored in s1engine's configuration or database:

```toml
# s1-server.toml
[[auth.api_keys]]
key = "sk_live_abc123def456"      # or hash of key
tenant_id = "org-456"
name = "Production Backend"
permissions = { "*": "editor" }   # all documents
rate_limit = 120                  # requests per minute
created_at = "2026-03-16"
```

Or via admin API:

```
POST /api/v1/admin/api-keys
{
  "name": "Production Backend",
  "permissions": { "*": "editor" },
  "rate_limit": 120,
  "expires_at": "2027-03-16T00:00:00Z"
}
```

### Anonymous Access

```toml
[auth]
allow_anonymous = true            # default: false
anonymous_role = "viewer"         # read-only for anonymous users
anonymous_rate_limit = 10         # requests per minute (strict)
```

---

## Authorization (Permissions)

### Permission Model

```
Hierarchy (most → least permissive):

  owner > admin > editor > commenter > viewer

Each level inherits all permissions from levels below it.
```

| Permission | viewer | commenter | editor | admin | owner |
|------------|--------|-----------|--------|-------|-------|
| Read document | Yes | Yes | Yes | Yes | Yes |
| Export document | Yes | Yes | Yes | Yes | Yes |
| Add comments | No | Yes | Yes | Yes | Yes |
| Edit content | No | No | Yes | Yes | Yes |
| Delete content | No | No | Yes | Yes | Yes |
| Manage permissions | No | No | No | Yes | Yes |
| Delete document | No | No | No | No | Yes |
| Transfer ownership | No | No | No | No | Yes |

### Permission Resolution

Permissions are resolved in this order (first match wins):

```
1. Document-specific permission (from JWT: permissions["doc_abc"] = "editor")
2. Wildcard permission (from JWT: permissions["*"] = "viewer")
3. Tenant default (from server config: default_role = "viewer")
4. Anonymous default (from server config: anonymous_role = "viewer")
5. Deny (if none of the above match)
```

### Permission Enforcement Points

```
REST API:
  ├── Middleware extracts auth (JWT or API key)
  ├── Middleware resolves tenant
  ├── Route handler checks permission for specific action
  └── Returns 403 if denied

WebSocket:
  ├── Token validated on connection
  ├── Permission checked on connect (must have at least "viewer")
  ├── Write operations checked per message (must have "editor")
  └── Connection closed if permission revoked

Storage:
  ├── All queries are tenant-scoped
  └── Cross-tenant access is impossible (enforced at query level)
```

### Dynamic Permission Updates

When a user's permissions change (e.g., removed as editor):

1. JWT expires naturally (short-lived tokens recommended: 15-60 minutes)
2. Next API call with new JWT reflects updated permissions
3. Active WebSocket connections are re-validated on next operation
4. Optional: server can push `permission_changed` event to force re-auth

---

## Multi-Tenancy

### Tenant Isolation

Every piece of data belongs to a tenant:

```sql
-- PostgreSQL schema
CREATE TABLE documents (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT NOT NULL,              -- isolation key
    title       TEXT NOT NULL,
    format      TEXT NOT NULL,
    content     BYTEA NOT NULL,
    metadata    JSONB DEFAULT '{}',
    created_by  TEXT,
    created_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW(),
    version     BIGINT DEFAULT 1
);

-- Index for tenant-scoped queries
CREATE INDEX idx_documents_tenant ON documents(tenant_id);
CREATE INDEX idx_documents_tenant_updated ON documents(tenant_id, updated_at DESC);

-- Row-level security (PostgreSQL)
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON documents
    USING (tenant_id = current_setting('app.tenant_id'));
```

For S3:
```
Bucket: s1-documents
└── {tenant_id}/
    ├── {doc_id}.docx
    ├── {doc_id}.meta.json
    └── ...
```

### Tenant Configuration

```toml
# Per-tenant limits (server config or database)
[tenants.defaults]
max_documents = 1000
max_storage_bytes = 1073741824    # 1 GB
max_file_size_bytes = 52428800    # 50 MB
max_collab_rooms = 10
max_peers_per_room = 25
rate_limit_per_minute = 60
```

### Cross-Tenant Access Prevention

- **Database**: Row-level security or WHERE clause on every query
- **S3**: Prefix-based access, no wildcard listings
- **API**: Tenant extracted from JWT, no way to override
- **WebSocket**: Room IDs are tenant-scoped
- **Logs**: Tenant ID included in every log entry
- **Admin API**: Requires explicit admin role, scoped to requesting tenant

---

## Security Measures

### Input Validation

```rust
// Every API endpoint validates input
struct CreateDocumentRequest {
    #[validate(length(max = 255))]
    title: String,

    #[validate(custom = "validate_format")]
    format: Option<String>,

    #[validate(length(max = 52_428_800))]  // 50 MB
    file: Option<Vec<u8>>,
}
```

**File upload security:**
- Maximum file size enforced (configurable, default 50 MB)
- Format validation (magic bytes check, not just extension)
- ZIP bomb detection (for DOCX/ODT — check uncompressed ratio)
- Path traversal prevention (sanitize filenames)
- Content-Type validation
- Virus scanning integration point (webhook after upload)

### Rate Limiting

```
Per API Key:
  ├── Global: 60 req/min (configurable)
  ├── Burst: 10 req/sec
  └── Heavy operations (convert, export): 5 req/min

Per IP (anonymous):
  ├── Global: 10 req/min
  └── Heavy operations: 2 req/min

Per WebSocket:
  ├── Messages: 100/sec
  ├── Awareness: 10/sec
  └── Sync requests: 1/5sec
```

Implementation: Token bucket algorithm via `governor` crate.

### CORS

```toml
[cors]
allowed_origins = ["https://app.acme.com"]  # or ["*"] for development
allowed_methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]
allowed_headers = ["Authorization", "Content-Type", "X-API-Key", "X-Request-Id"]
expose_headers = ["X-Request-Id", "X-RateLimit-Remaining", "ETag"]
max_age = 86400                              # preflight cache (seconds)
allow_credentials = true
```

### Content Security Policy (for editor embedding)

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-eval' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:;
  connect-src 'self' wss://*.s1engine.dev;
  font-src 'self' data:;
  worker-src 'self' blob:;
  frame-ancestors 'self' https://*.acme.com;
```

`frame-ancestors` is configurable per tenant for embedding control.

### Encryption

| Data State | Encryption |
|------------|-----------|
| In transit | TLS 1.2+ (mandatory for production) |
| At rest (S3) | S3 server-side encryption (SSE-S3 or SSE-KMS) |
| At rest (Postgres) | Transparent Data Encryption (TDE) or disk-level |
| At rest (local FS) | OS-level disk encryption |
| JWT tokens | Signed (RS256 preferred over HS256) |
| API keys | Stored as bcrypt hashes (raw key never stored) |
| Webhook secrets | HMAC-SHA256 signatures |

### Audit Logging

Every security-relevant action is logged:

```json
{
  "timestamp": "2026-03-16T10:05:00Z",
  "event": "document.accessed",
  "tenant_id": "org-456",
  "user_id": "user-123",
  "document_id": "doc_abc",
  "action": "read",
  "ip": "192.168.1.100",
  "user_agent": "Mozilla/5.0...",
  "request_id": "req_xyz",
  "result": "allowed"
}
```

Events logged:
- `auth.login` — JWT validated
- `auth.failed` — JWT validation failed
- `auth.api_key_used` — API key authentication
- `document.created` — Document created
- `document.accessed` — Document read
- `document.updated` — Document modified
- `document.deleted` — Document deleted
- `document.exported` — Document exported
- `document.shared` — Permission granted
- `document.unshared` — Permission revoked
- `collab.joined` — Peer joined room
- `collab.left` — Peer left room
- `admin.api_key_created` — API key created
- `admin.api_key_revoked` — API key revoked

### Vulnerability Mitigation

| Threat | Mitigation |
|--------|-----------|
| SQL Injection | Parameterized queries (sqlx compile-time checked) |
| XSS | No user HTML rendered on server; client sanitizes via WASM |
| CSRF | Token-based auth (no cookies by default) |
| Path Traversal | Filename sanitization, no user-controlled paths |
| Zip Bombs | Uncompressed size ratio check (max 100:1) |
| DoS | Rate limiting, max file sizes, request timeouts |
| Man-in-the-Middle | TLS required in production |
| JWT Replay | Short expiry (15 min), optional `jti` claim dedup |
| Broken Auth | No default credentials, key rotation support |
| Insecure Deserialization | Typed deserialization (serde), no arbitrary code execution |

### Security Headers

```
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY (or ALLOW-FROM for embedding)
X-XSS-Protection: 0 (rely on CSP instead)
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
```

---

## Integration Examples

### Auth0

```javascript
// Consumer's frontend
import { S1Editor } from '@s1engine/editor'
import { useAuth0 } from '@auth0/auth0-react'

function Editor() {
  const { getAccessTokenSilently } = useAuth0()

  useEffect(() => {
    async function init() {
      const token = await getAccessTokenSilently({
        audience: 'https://api.s1engine.dev',
        scope: 's1:editor'
      })

      const editor = await S1Editor.create(container, {
        collab: {
          serverUrl: 'wss://api.example.com/ws/collab',
          roomId: 'doc-123',
          token: token
        }
      })
    }
    init()
  }, [])
}
```

```toml
# s1-server.toml
[auth.jwt]
algorithm = "RS256"
jwks_url = "https://your-tenant.auth0.com/.well-known/jwks.json"
issuer = "https://your-tenant.auth0.com/"
audience = "https://api.s1engine.dev"
claims_namespace = "s1"
```

### Firebase Auth

```javascript
import { getAuth, getIdToken } from 'firebase/auth'

const auth = getAuth()
const token = await getIdToken(auth.currentUser, true)
// Include custom claims with s1 permissions via Firebase Admin SDK
```

### Custom JWT Server

```javascript
// Consumer's auth server (Node.js example)
import jwt from 'jsonwebtoken'

function generateS1Token(user, documentId, permission) {
  return jwt.sign({
    sub: user.id,
    s1: {
      tenant_id: user.orgId,
      user_name: user.displayName,
      permissions: {
        [documentId]: permission  // "viewer", "editor", "owner"
      }
    }
  }, process.env.JWT_SECRET, {
    algorithm: 'HS256',
    expiresIn: '15m'
  })
}
```
