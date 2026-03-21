# Admin Panel Specification v1.0

## Current State

The admin panel at `/admin/dashboard` provides:
- Login form (cookie-based auth)
- Server stats (uptime, sessions, editors, memory)
- Session list (file_id, filename, format, size, editors, status, last active)
- Session close action
- Auto-refresh every 10s

## Enhancements Needed

### 1. Real-Time Dashboard

| Feature | Current | Target |
|---------|---------|--------|
| Refresh interval | 10s polling | WebSocket push (instant updates) |
| Session count | Number only | Sparkline chart (last 1h) |
| Editor count | Number only | Per-session breakdown |
| Memory usage | Single number | Trend graph |
| Network traffic | Not shown | Bytes in/out per second |

### 2. Session Detail View

Click a session row → expanded view showing:
- All connected editors (name, role, last activity, cursor position)
- Document preview (first paragraph text)
- ops_log size and version counter
- CRDT state: tombstone count, pending ops
- Collaboration health: version gaps between peers
- Action buttons: force sync, kick editor, export snapshot

### 3. Error Log Viewer

| Feature | Description |
|---------|-------------|
| Source | Client errors via `POST /api/v1/errors` |
| Display | Paginated table: timestamp, message, count |
| Grouping | Identical errors grouped with count |
| Filtering | By time range, severity, keyword |
| Export | CSV download of error log |

### 4. Document History

| Feature | Description |
|---------|-------------|
| Timeline | Show when documents were opened, edited, saved, closed |
| Snapshots | Periodic document snapshots with version numbers |
| Diff | Compare two snapshots (visual diff of text changes) |
| Restore | Roll back to a previous snapshot |

### 5. User Management

| Feature | Description |
|---------|-------------|
| User list | All users who have connected (from JWT claims) |
| Per-user stats | Documents edited, total edits, last seen |
| Active sessions | Which documents each user is currently editing |
| Kick user | Force disconnect a specific user |

### 6. System Health

| Metric | Source | Alert Threshold |
|--------|--------|----------------|
| CPU usage | `/proc/stat` or `sysinfo` crate | >80% |
| Memory | RSS from process info | >75% of available |
| Disk | Storage directory size | >90% capacity |
| WebSocket connections | Room manager | >100 per room |
| ops_log size | Per room | >8000 (nearing truncation) |
| Session count | FileSessionManager | >50 concurrent |

### 7. API Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | /admin/login | Login form | DONE |
| POST | /admin/login | Submit credentials | DONE |
| GET | /admin/logout | Clear session | DONE |
| GET | /admin/dashboard | Dashboard HTML | DONE |
| GET | /admin/api/stats | Server statistics | DONE |
| GET | /admin/api/sessions | List all sessions | DONE |
| DELETE | /admin/api/sessions/{id} | Force close session | DONE |
| GET | /admin/api/config | Sanitized config | DONE |
| GET | /admin/api/errors | Error log (paginated, ring buffer of last 100) | DONE |
| GET | /admin/api/sessions/{id}/editors | Detailed editor info (user_id, name, mode, last_activity) | DONE |
| POST | /admin/api/sessions/{id}/sync | Force fullSync broadcast to all peers in room | DONE |
| POST | /admin/api/sessions/{id}/kick/{uid} | Kick editor: editor_leave + broadcast peer-leave | DONE |
| GET | /admin/api/health | System health: uptime, memory, sessions, rooms, editors, PID | DONE |
| GET | /admin/api/rooms/{id}/version | Room version counter + ops_log count | DONE |

### 8. Security

- Cookie auth with `s1_admin` (base64 user:pass) — DONE
- `HttpOnly` flag on cookie — DONE
- Admin-only middleware on all /admin/ routes — DONE
- HTML-escaped user data in dashboard — DONE
- Rate limiting on login attempts (5 per IP per 60s, in-memory HashMap) — DONE
- Session timeout (auto-logout after 1h via `s1_admin_ts` cookie + Max-Age=3600) — DONE

### 9. Admin MVP Scope

The current admin panel (sections 1-8 above) covers the MVP. The following defines what is in scope for the MVP and what is deferred.

**MVP (DONE):**
- Login/logout with cookie auth
- Dashboard with stats cards (uptime, sessions, editors, memory)
- Session list with close action
- Auto-refresh (10s polling)
- Sanitized config display

**Post-MVP priorities (in order):**

| Priority | Feature | Complexity | Depends On |
|----------|---------|-----------|-----------|
| P0 | Login rate limiting (5 attempts/min) | Low | None | DONE |
| P0 | Session timeout (auto-logout after 1h) | Low | None | DONE |
| P1 | Error log viewer (section 3) | Medium | `POST /api/v1/errors` storage | DONE |
| P1 | Session detail view (section 2) | Medium | `/admin/api/sessions/{id}/editors` | DONE |
| P2 | System health metrics (section 6) | Medium | `sysinfo` crate | DONE |
| P2 | WebSocket push for real-time updates (section 1) | Medium | Admin WS endpoint |
| P3 | User management (section 5) | High | Auth system + user storage |
| P3 | Document history/snapshots (section 4) | High | Snapshot storage |

**Explicit non-goals for admin MVP:**
- Multi-user admin accounts (single admin user is sufficient)
- Role-based admin permissions (all admins have full access)
- Custom branding/theming of admin panel

## 10. Full Configuration Control Panel

### 10.1 Server Settings (runtime-editable)

| Setting | Env Var | Default | Editable | Description |
|---------|---------|---------|----------|-------------|
| Log level | `RUST_LOG` | info | **Yes** | Runtime log level change |
| Max upload size | `S1_MAX_UPLOAD_MB` | 64 | **Yes** | File upload limit (MB) |
| Session TTL | `S1_SESSION_TTL` | 300 | **Yes** | Seconds after last editor before cleanup |
| Max editors/session | `S1_MAX_EDITORS` | 50 | **Yes** | Concurrent editor cap |
| Stale timeout | `S1_STALE_TIMEOUT` | 300 | **Yes** | Inactivity before editor removed |
| ops_log max | `S1_OPSLOG_MAX` | 10000 | **Yes** | Max ops before truncation |
| fullSync debounce | `S1_FULLSYNC_DEBOUNCE` | 1500 | **Yes** | Debounce ms for structural sync |
| Ping interval | `S1_PING_INTERVAL` | 15 | **Yes** | WS keepalive seconds |
| Snapshot interval | `S1_SNAPSHOT_INTERVAL` | 30 | **Yes** | Auto-save frequency seconds |
| CORS origins | `S1_CORS_ORIGINS` | * | **Yes** | Allowed origins |
| Auth enabled | `S1_AUTH_ENABLED` | false | **Yes** | Toggle authentication |
| Allow anonymous | `S1_ALLOW_ANONYMOUS` | true | **Yes** | Allow unauthenticated access |
| Require JWT exp | `S1_REQUIRE_JWT_EXP` | false | **Yes** | Reject tokens without expiry |
| Callback allowlist | `S1_CALLBACK_ALLOWLIST` | (empty) | **Yes** | Allowed callback domains |
| Download allowlist | `S1_DOWNLOAD_ALLOWLIST` | (empty) | **Yes** | Allowed download domains |

### 10.2 Feature Flags

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| PDF export | `S1_FEATURE_PDF` | true | PDF conversion |
| ODT support | `S1_FEATURE_ODT` | true | ODT read/write |
| Markdown | `S1_FEATURE_MD` | true | Markdown format |
| CRDT collab | `S1_CRDT_ENABLED` | true | CRDT text ops |
| Track changes | `S1_FEATURE_TRACKCHANGES` | true | Change tracking |
| Comments | `S1_FEATURE_COMMENTS` | true | Comment insertion |
| Signatures | `S1_FEATURE_SIGNATURES` | true | Digital signature display |
| Form controls | `S1_FEATURE_FORMS` | true | Interactive forms |
| Equations | `S1_FEATURE_EQUATIONS` | true | LaTeX equations |
| Image upload | `S1_FEATURE_IMAGES` | true | Image insertion |
| Tables | `S1_FEATURE_TABLES` | true | Table creation |
| Webhooks | `S1_FEATURE_WEBHOOKS` | true | Webhook API |
| Integration mode | `S1_INTEGRATION_ENABLED` | true | JWT /edit endpoint |

### 10.3 Config API

| Method | Path | Description |
|--------|------|-------------|
| GET | /admin/api/config | All settings (passwords sanitized) |
| PUT | /admin/api/config | Update runtime setting `{key, value}` |
| GET | /admin/api/features | All feature flags as JSON |
| PUT | /admin/api/features/{flag} | Toggle feature `{enabled: bool}` |
| GET | /admin/api/audit | Admin action log (last 1000 entries) |

### 10.4 Config Change Propagation

1. Admin updates setting via PUT
2. Server updates in-memory config
3. If affects editors → broadcast `config-update` to all WS clients
4. Editors update UI (disable features, change limits)
5. Logged in audit trail (admin, timestamp, old→new)
6. NOT persisted to .env (ephemeral — restart resets to env vars)

### 10.5 Audit Trail

Every admin action logged: `{timestamp, admin, action, target, old_value, new_value}`

Actions: login, logout, config change, feature toggle, session close, editor kick, force sync, error export.

Storage: in-memory ring buffer (1000 entries). `GET /admin/api/audit`.

### 10.6 Dashboard Tabs

| Tab | Content | Status |
|-----|---------|--------|
| Sessions | Session list + detail view | DONE |
| Errors | Error log with grouping | DONE (API) |
| Config | Runtime settings editor | SPECIFIED |
| Features | Toggle switches per feature | SPECIFIED |
| Health | System metrics + sparklines | DONE (API) |
| Audit | Admin action log | SPECIFIED |
