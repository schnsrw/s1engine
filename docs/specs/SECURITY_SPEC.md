# Security Specification v1.0

> **WARNING**: Authentication is disabled by default (S1_AUTH_ENABLED=false). When disabled, ALL endpoints are public. Enable auth before production deployment.

> Covers: authentication, authorization, input validation, WebSocket security, admin panel, DOCX security.
> Last updated: 2026-03-21

## 1. Threat Model

### 1.1 Actors

| Actor | Description | Trust Level |
|-------|-------------|-------------|
| Authenticated User | Has valid JWT or API key | Medium |
| Anonymous User | No credentials, auth disabled | Low |
| Integration Host | External product embedding s1-server | High |
| Admin | Has admin credentials | High |
| Malicious User | Attempting to exploit the system | None |

### 1.2 Assets to Protect

| Asset | Sensitivity |
|-------|-------------|
| Document content | High (may contain confidential data) |
| User identity/session | Medium |
| Admin credentials | Critical |
| JWT signing secret | Critical |
| Server configuration | Medium |
| System resources (CPU, memory, disk) | Medium |

### 1.3 Attack Surface

| Surface | Exposure |
|---------|----------|
| REST API | Public (if auth disabled) |
| WebSocket | Public (per-session) |
| Admin panel | Protected by cookie auth |
| File upload | Accepts arbitrary binary data |
| Document parsing | Processes untrusted document formats |
| Integration JWT | Tokens from external systems |

## 2. Authentication

### 2.1 JWT Validation (HS256)

**Implementation:** `server/src/auth.rs` — `validate_jwt()`

| Check | Implementation | Status |
|-------|---------------|--------|
| Format validation | Verify 3 dot-separated parts | DONE |
| Signature verification | HMAC-SHA256 with `S1_JWT_SECRET` | DONE |
| Expiration check | `exp` claim vs current UTC time | DONE |
| Claims extraction | `sub`, `name`, `tenant_id`, `role` | DONE |
| Algorithm restriction | Only HS256 accepted | DONE |
| Timing attack protection | `hmac::Mac::verify_slice` (constant-time) | DONE |

**Security requirements:**
- `S1_JWT_SECRET` MUST be at least 32 bytes in production
- Tokens without `exp` claim are accepted by default (no expiration) -- production should enable `S1_REQUIRE_JWT_EXP=true`
- Clock skew tolerance: none (server time must be accurate)

**JWT `exp` requirement:**
- **Default behavior**: Tokens without `exp` are accepted indefinitely. This is convenient for development but dangerous in production.
- **Enforcement**: Set `S1_REQUIRE_JWT_EXP=true` to reject tokens without an `exp` claim with `401 Unauthorized`. This applies to both API JWT (`auth.rs`) and integration JWT (`integration.rs`).
- **Maximum token lifetime**: Recommended max 24 hours for API tokens, 1 hour for integration tokens.
- **DONE**: `S1_REQUIRE_JWT_EXP=true` enforcement implemented in both `validate_jwt()` and `validate_integration_jwt()`.

### 2.2 API Key Authentication

**Implementation:** `server/src/auth.rs` — `auth_middleware()`

API keys are checked via the `X-API-Key` header against a static list in `AuthConfig.api_keys`.

| Check | Implementation | Status |
|-------|---------------|--------|
| Header extraction | `X-API-Key` header | DONE |
| Key comparison | Exact string match (iterate list) | DONE |
| User identity mapping | Key -> name + tenant_id + role | DONE |

**Security considerations:**
- API keys are compared via normal string equality (not constant-time). This is acceptable because keys are not secrets derived from user input -- they are pre-configured server-side.
- Keys should be rotated periodically.
- Future: rate limiting per API key.

### 2.3 Integration JWT (/edit?token=)

**Implementation:** `server/src/integration.rs` — `validate_integration_jwt()`

Separate JWT validation for the integration entry point, with different claims:

| Claim | Validated | Required |
|-------|-----------|----------|
| `file_id` | Extracted | Yes |
| `user_id` | Extracted | Yes |
| `user_name` | Extracted | Yes |
| `permissions` | Extracted (default: "edit") | No |
| `download_url` | Used to fetch document | No |
| `callback_url` | Stored for save callbacks | No |
| `exp` | Checked against system time | No |
| Signature | HS256 with `S1_JWT_SECRET` | Yes |

**Security risk:** The `download_url` claim causes the server to make an outbound HTTP request (SSRF vector). Mitigations:

| Mitigation | Status | Description |
|-----------|--------|-------------|
| Request timeout | DONE | 30-second timeout on fetch |
| Domain allowlist (`S1_DOWNLOAD_ALLOWLIST`) | NOT IMPLEMENTED | Reject URLs not matching allowlisted domains |
| Private IP block | DONE | `is_private_ip()` blocks RFC 1918 (10.x, 172.16-31.x, 192.168.x), link-local (169.254.x), loopback (127.x), localhost, .local, .internal, IPv6 equivalents, and cloud metadata endpoints |
| DNS rebinding protection | NOT IMPLEMENTED | Resolve hostname before request, verify resolved IP is not private |
| Redirect following limit | NOT IMPLEMENTED | Max 3 redirects; re-validate each redirect target against IP blocklist |
| Response size limit | NOT IMPLEMENTED | Cap response body at 64MB (match upload limit) |

Private IP blocking is applied to both `download_url` (in `fetch_document()`) and `callback_url` (in `trigger_save_callback()` and the background cleanup task). The `is_private_ip()` function checks raw IPs, URLs with schemes, and common private hostnames.

### 2.4 Callback URL Security

The `callback_url` JWT claim causes the server to POST document bytes to an external URL on session close/save. This has the same SSRF risks as `download_url`, plus data exfiltration risk.

| Risk | Description | Mitigation |
|------|-------------|-----------|
| SSRF (outbound POST) | Server can be directed to POST to internal services | Same `is_private_ip()` blocklist as `download_url` — DONE |
| Data exfiltration | Attacker-controlled callback receives document content | Domain allowlist (`S1_CALLBACK_ALLOWLIST`) (NOT IMPLEMENTED) |
| Callback impersonation | No HMAC/signature on callback request | `X-S1-Signature: sha256=HMAC-SHA256(body, S1_CALLBACK_SECRET)` header — DONE |
| Callback replay | Intercepted callback could be replayed | `X-S1-Timestamp` header included with Unix timestamp — DONE (server-side; receiver-side validation is consumer responsibility) |

**Current state**: Callback requests are protected by:
1. Private IP blocklist (`is_private_ip()`) — blocks RFC 1918, loopback, link-local, metadata endpoints
2. HMAC signature (`X-S1-Signature` header) when `S1_CALLBACK_SECRET` env var is set
3. Timestamp header (`X-S1-Timestamp`) for replay detection by the receiver
4. 30-second timeout on HTTP requests

**Remaining mitigations**:
1. Domain allowlist configuration (`S1_CALLBACK_ALLOWLIST`) for restricting callback targets
2. DNS rebinding protection (resolve hostname before request)

### 2.5 Admin Cookie Authentication

**Implementation:** `server/src/admin.rs` — `admin_auth()`

| Property | Value |
|----------|-------|
| Cookie name | `s1_admin` |
| Cookie value | `base64(username:password)` |
| Cookie path | `/admin` |
| HttpOnly | Yes |
| SameSite | Strict |
| Secure | No (should be Yes in production with HTTPS) |

**Security considerations:**
- Credentials are effectively base64-encoded Basic Auth in a cookie. This is simple but not ideal for production.
- No session tokens, no CSRF tokens. The `SameSite=Strict` attribute provides CSRF protection.
- No brute force protection on `/admin/login`. Future: rate limiting, account lockout.
- Admin credentials are read from environment variables on every request (not cached).

## 3. Authorization

### 3.1 Permission Model

```
Owner > Admin > Editor > Commenter > Viewer
  4       3        2         1          0
```

### 3.2 Permission Resolution

| Context | Resolution |
|---------|-----------|
| Admin role user | All permissions granted (Owner level) |
| Anonymous user | Viewer only |
| Session owner (user_id matches) | Owner |
| Session mode "edit" | Editor |
| Session mode "comment" | Commenter |
| Session mode "view" | Viewer |
| Authenticated, no session context | Editor (default) |

### 3.3 Unified Auth Model

| User Role (from JWT/API key) | Default Document Permission | Can Be Elevated | Can Be Restricted |
|------------------------------|---------------------------|-----------------|-------------------|
| Admin | Owner | N/A (already max) | No |
| User | Editor (default) | To Admin (by admin) | To Viewer/Commenter (per-doc) |
| Anonymous | Viewer | No | No |

Roles come from authentication (JWT `role` claim). Permissions come from the document session (share link `access` param or integration JWT `permissions` claim). The LOWER of role and document permission applies.

### 3.4 Endpoint Authorization Matrix

| Endpoint | Required Permission | Currently Enforced |
|----------|--------------------|--------------------|
| GET /health | None | No auth needed |
| GET /api/v1/info | None | No auth needed |
| POST /api/v1/files | Editor | NO (not enforced) |
| GET /api/v1/files | Viewer | NO (not enforced) |
| GET /api/v1/files/{id} | Viewer | NO (not enforced) |
| GET /api/v1/files/{id}/download | Viewer | NO (not enforced) |
| DELETE /api/v1/files/{id} | Editor | Session mode check only |
| POST /api/v1/files/{id}/save | Editor | NO (not enforced) |
| POST /api/v1/documents | Editor | NO (not enforced) |
| GET /api/v1/documents | Viewer | NO (not enforced) |
| DELETE /api/v1/documents/{id} | Admin | NO (not enforced) |
| POST /api/v1/convert | Viewer | NO (not enforced) |
| WS /ws/edit/{id} | Per `access` query param | Access level from URL param |
| /admin/* | Admin cookie | Cookie check via middleware |

**Gap:** The auth middleware exists but is not wired into the main router. When `S1_AUTH_ENABLED=false` (default), all endpoints are public. When enabled, per-route permission checks should be added.

## 4. Input Validation

### 4.1 File Upload Validation

| Check | Implementation | Status |
|-------|---------------|--------|
| Max file size | 64MB (DefaultBodyLimit) | DONE |
| Document parsing | s1engine validates on upload | DONE |
| Filename sanitization | Used for Content-Disposition only | PARTIAL |
| Content type verification | Format detected from content, not Content-Type header | DONE |

**Missing validations:**
- Filename length limit
- File extension allowlist
- Zip bomb detection (nested ZIP archives)
- XML entity expansion limit (handled by quick-xml defaults)

### 4.2 XSS Prevention

| Vector | Mitigation | Status |
|--------|-----------|--------|
| Document content rendered as HTML | `to_html()` produces sanitized HTML from model (no raw user HTML) | DONE |
| User names in collaboration | Displayed via `textContent` (not `innerHTML`) | DONE |
| Admin dashboard | User-provided values escaped via `esc()` function | DONE |
| Error messages in HTML | Messages use `textContent` or text responses | DONE |
| `data-node-id` attributes | Numeric IDs only (format: `replica:counter`) | DONE |

**Key defense:** The WASM engine's `to_html()` generates HTML from the document model tree, not from raw document XML. Malicious content in a DOCX/ODT file is parsed into the model (plain text, attributes) and re-rendered as safe HTML. There is no path for raw XML/HTML injection from document content to the browser DOM.

### 4.3 SQL Injection

Not applicable. s1-server does not use a SQL database. Storage backends are filesystem-based or in-memory.

### 4.4 CSRF Protection

| Mechanism | Protection |
|-----------|-----------|
| Admin panel cookies | `SameSite=Strict` prevents cross-site cookie sending |
| REST API | Stateless (JWT/API key), no cookies = no CSRF risk |
| WebSocket | Same-origin check (see section 5) |

### 4.5 Path Traversal

| Vector | Mitigation |
|--------|-----------|
| File upload path | UUIDs used as file IDs (no user-controlled paths) |
| Download filename | Original filename used only in Content-Disposition header |
| Static file serving | `ServeDir` from Axum/tower-http handles path traversal safely |
| Storage backend | File IDs are UUIDs, stored in flat directory |

### 4.6 Resource Exhaustion

| Attack | Current Protection | Recommended |
|--------|-------------------|-------------|
| Large file upload | 64MB body limit | Keep or reduce |
| Many concurrent WebSocket connections | No limit | Rate limit per IP |
| ops_log growth | Truncates at 10,000 entries | Sufficient |
| Offline buffer flood | Client-side 10,000 op limit | Add server-side check |
| Many file sessions | No limit | Add max session count |
| Rapid reconnect flapping | Client-side exponential backoff | Add server-side tracking |

## 5. WebSocket Security

### 5.1 Access Parameter -- MUST NOT Trust Client

The `?access=edit` URL parameter is for **client-side UX only** (toolbar state). The server MUST NOT use this parameter for access control decisions.

**Current state**: Server reads `access` from WsParams and includes it in `joined` message. Client uses it to enable/disable toolbar.

**Required**: Server MUST validate access against:
1. FileSession.mode (set by share link creator)
2. Integration JWT permissions claim (if present)
3. User role (from auth middleware)

**NOT IMPLEMENTED**: Server currently trusts client `access` param. Until fixed, access control is client-side only (bypassable).

### 5.2 Connection Authentication

WebSocket connections are authenticated via URL query parameters:

```
/ws/edit/{fileId}?user=Alice&uid=u-abc&access=edit
```

**Current state:** The `access` parameter is trusted from the client. In production, WebSocket connections should validate the user against the file session permissions.

### 5.3 Origin Checking

Not currently implemented. Axum's WebSocket upgrade does not check the `Origin` header by default.

**Recommendation:** Add origin validation middleware that checks the `Origin` header against an allowlist of trusted origins.

### 5.4 Message Validation

| Check | Implementation | Status |
|-------|---------------|--------|
| JSON parse | All messages parsed as JSON | DONE |
| Message type field | Validated in match statement | DONE |
| Room membership | Messages only forwarded to room members | DONE |
| Sender identity | `_sender` field added server-side | DONE |
| Op data validation | CRDT ops validated by WASM engine | DONE |
| Message size limit | No explicit limit | NOT IMPLEMENTED |

**Recommendation:** Add a maximum message size (e.g., 10MB) to prevent memory exhaustion from oversized fullSync messages.

### 5.5 Echo Filtering

The server adds `_sender` to every forwarded message. Clients filter out messages from their own `peerId` to prevent echo loops.

### 5.6 Server Ping/Pong

Server sends ping frames every 15 seconds. Clients that don't respond with pong are considered disconnected.

## 6. Content Security Policy

Not currently implemented. The server serves static files with default headers.

**Recommended CSP for production:**
```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:;
  connect-src 'self' ws: wss:;
  font-src 'self';
  object-src 'none';
  base-uri 'self';
  frame-ancestors 'none';
```

Key considerations:
- `'wasm-unsafe-eval'` is required for WASM execution
- `'unsafe-inline'` is needed for inline styles (document formatting)
- `data:` and `blob:` for `img-src` are required for embedded document images
- `ws:` / `wss:` for WebSocket connections

## 7. Admin Panel Security

### 7.1 Current State

| Property | Status |
|----------|--------|
| Cookie-based auth | DONE |
| HttpOnly cookie | DONE |
| SameSite=Strict | DONE |
| Brute force protection | DONE (rate limiter: 5 attempts per IP per 60s) |
| Session expiration | DONE (Max-Age=3600 on cookies + `s1_admin_ts` timestamp check) |
| HTTPS-only cookie (Secure flag) | NOT IMPLEMENTED |
| Rate limiting on login | DONE (in-memory HashMap<IpAddr, (count, window_start)>) |
| Audit logging | NOT IMPLEMENTED |

### 7.2 Recommended Improvements

| Improvement | Priority | Status |
|-------------|----------|--------|
| Add `Secure` flag to cookie (HTTPS only) | HIGH | NOT IMPLEMENTED |
| Add login rate limiting (5 attempts/min) | HIGH | DONE |
| Add session expiration (1 hour) | MEDIUM | DONE |
| Use proper session tokens instead of base64(user:pass) | MEDIUM | NOT IMPLEMENTED |
| Add audit log for admin actions | LOW | NOT IMPLEMENTED |
| TOTP/2FA for admin login | LOW | NOT IMPLEMENTED |

## 8. Data at Rest

### 8.1 Current State

| Storage | Encryption | Status |
|---------|-----------|--------|
| Local filesystem | None (plain files) | NOT ENCRYPTED |
| In-memory storage | N/A (volatile) | N/A |
| File sessions | None (in-memory + optional disk) | NOT ENCRYPTED |
| ops_log (collab rooms) | None (in-memory) | N/A |

### 8.2 Future: Storage Encryption

For production deployments with sensitive documents:
- Encrypt document bytes before writing to disk (AES-256-GCM)
- Encryption key from environment variable or KMS
- Per-document keys (wrapped by master key) for key rotation
- Encrypted at rest, decrypted in memory for editing

## 9. DOCX/ODT Security

### 9.1 Macro Detection

DOCX files can contain VBA macros in `vbaProject.bin`. The s1engine parser does not execute macros but should detect and report their presence.

| Check | Status |
|-------|--------|
| Detect `vbaProject.bin` in DOCX ZIP | DONE (reader.rs) |
| Strip macros on import | DEFERRED (macros preserved for round-trip) |
| Warn user about macros | DONE (showToast in file.js) |
| Block macro-enabled documents (.docm) | DEFERRED (would break round-trip) |

### 9.2 External References

DOCX/ODT documents can reference external resources:

| Reference Type | Risk | Mitigation |
|----------------|------|-----------|
| External images (`http://...`) | Privacy leak (tracking pixel), SSRF | Strip or warn on external URLs |
| OLE objects | Code execution | Not supported by s1engine |
| External hyperlinks | Phishing | Display but do not auto-follow |
| External fonts | Privacy leak | Not loaded (WASM has no network) |
| XML external entities (XXE) | File disclosure, SSRF | `quick-xml` does not expand external entities by default |

### 9.3 XML Parsing Safety

The `quick-xml` crate used for DOCX/ODT parsing has the following safety properties:

| Property | Status |
|----------|--------|
| External entity expansion | Disabled by default |
| DTD processing | Not supported (safe) |
| Entity expansion limit | Bounded by quick-xml defaults |
| Buffer size limit | Configurable |
| Stack overflow (deeply nested XML) | Bounded by recursion limits |

### 9.4 Digital Signatures

DOCX files may contain digital signatures (`_xmlsignatures/`). s1engine does not validate or preserve signatures.

| Feature | Status |
|---------|--------|
| Detect signed documents | DONE (signature_parser.rs) |
| Validate signatures | DONE (x509-cert parsing, ring crypto feature) |
| Preserve signatures on export | DONE (preserved_parts) |
| Warn that editing breaks signature | SPECIFIED (showToast on edit of signed doc) |

## 10. Security Checklist for Production Deployment

| Item | Status |
|------|--------|
| Set strong `S1_JWT_SECRET` (32+ bytes) | Manual |
| Set strong `S1_ADMIN_PASS` | Manual |
| Enable `S1_AUTH_ENABLED=true` | Manual |
| Deploy behind HTTPS (TLS termination) | Manual |
| Set restrictive CORS origins | NOT IMPLEMENTED (currently permissive) |
| Add Content-Security-Policy header | NOT IMPLEMENTED |
| Add rate limiting (reverse proxy or middleware) | NOT IMPLEMENTED |
| Restrict WebSocket origins | NOT IMPLEMENTED |
| Add message size limits on WebSocket | NOT IMPLEMENTED |
| Enable storage encryption | NOT IMPLEMENTED |
| Monitor error reporting endpoint | Manual |
| Rotate API keys periodically | Manual |
| Set up log aggregation | Manual |

## 11. Implementation Status

| Feature | Status |
|---------|--------|
| JWT authentication (HS256) | DONE |
| API key authentication | DONE |
| Integration JWT (separate claims) | DONE |
| Admin cookie authentication | DONE |
| Permission model (5 levels) | DONE |
| Permission resolution logic | DONE |
| XSS prevention in rendering | DONE |
| Safe XML parsing (no XXE) | DONE (via quick-xml) |
| File size limits | DONE (64MB) |
| Echo filtering on WebSocket | DONE |
| Server ping/pong for stale detection | DONE |
| Per-route auth enforcement | NOT IMPLEMENTED |
| WebSocket origin checking | NOT IMPLEMENTED |
| WebSocket message size limit | NOT IMPLEMENTED |
| Rate limiting | NOT IMPLEMENTED |
| Content Security Policy | NOT IMPLEMENTED |
| Storage encryption | NOT IMPLEMENTED |
| Macro detection in DOCX | NOT IMPLEMENTED |
| Digital signature handling | NOT IMPLEMENTED |
| Admin brute force protection | DONE (5 attempts/IP/60s) |
| Admin session timeout | DONE (1h via Max-Age + timestamp cookie) |
| Private IP block (SSRF) | DONE (`is_private_ip()` on download_url + callback_url) |
| Callback HMAC signing | DONE (`X-S1-Signature` header with `S1_CALLBACK_SECRET`) |
| JWT exp enforcement | DONE (`S1_REQUIRE_JWT_EXP=true` rejects tokens without exp) |
| HTTPS cookie flag | NOT IMPLEMENTED |
