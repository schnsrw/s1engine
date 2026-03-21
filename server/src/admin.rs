//! Admin panel — server monitoring and management.
//!
//! Serves a login form at `/admin/login` and dashboard at `/admin/dashboard`.
//! Uses a simple cookie-based session (token = base64(user:pass)).

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::routes::AppState;

static mut START_TIME: Option<Instant> = None;

pub fn init_start_time() {
    unsafe {
        START_TIME = Some(Instant::now());
    }
}

fn uptime_secs() -> u64 {
    unsafe { START_TIME.map(|t| t.elapsed().as_secs()).unwrap_or(0) }
}

// ─── Error Log (Ring Buffer) ─────────────────────────

/// A single error log entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorEntry {
    pub timestamp: String,
    pub message: String,
    pub source: String,
}

/// In-memory ring buffer for the last N errors.
pub struct ErrorLog {
    entries: Mutex<Vec<ErrorEntry>>,
    capacity: usize,
}

impl ErrorLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(capacity)),
            capacity,
        }
    }

    pub async fn push(&self, message: String, source: String) {
        let mut entries = self.entries.lock().await;
        if entries.len() >= self.capacity {
            entries.remove(0);
        }
        entries.push(ErrorEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            message,
            source,
        });
    }

    pub async fn recent(&self, limit: usize, offset: usize) -> (Vec<ErrorEntry>, usize) {
        let entries = self.entries.lock().await;
        let total = entries.len();
        let page: Vec<ErrorEntry> = entries
            .iter()
            .rev()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        (page, total)
    }
}

// ─── Rate Limiter ────────────────────────────────────

/// Simple in-memory rate limiter: max N attempts per IP per window.
pub struct RateLimiter {
    attempts: Mutex<HashMap<IpAddr, (u32, Instant)>>,
    max_attempts: u32,
    window: std::time::Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window_secs: u64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window: std::time::Duration::from_secs(window_secs),
        }
    }

    /// Check if the IP is allowed. Returns Ok(()) if allowed, Err(remaining_secs) if blocked.
    pub async fn check_and_increment(&self, ip: IpAddr) -> Result<(), u64> {
        let mut attempts = self.attempts.lock().await;
        let now = Instant::now();

        if let Some((count, window_start)) = attempts.get_mut(&ip) {
            if now.duration_since(*window_start) > self.window {
                // Window expired, reset
                *count = 1;
                *window_start = now;
                Ok(())
            } else if *count >= self.max_attempts {
                let remaining = self.window.as_secs() - now.duration_since(*window_start).as_secs();
                Err(remaining)
            } else {
                *count += 1;
                Ok(())
            }
        } else {
            attempts.insert(ip, (1, now));
            Ok(())
        }
    }

    /// Reset the counter for an IP (on successful login).
    pub async fn reset(&self, ip: IpAddr) {
        self.attempts.lock().await.remove(&ip);
    }
}

/// Session timeout duration: 1 hour (3600 seconds).
const SESSION_MAX_AGE_SECS: u64 = 3600;

/// Admin auth middleware — checks for `s1_admin` cookie.
///
/// Validates credentials and enforces session timeout via the `s1_admin_ts`
/// cookie which records when the session was created.
pub async fn admin_auth(request: Request, next: Next) -> Result<Response, StatusCode> {
    let admin_user = std::env::var("S1_ADMIN_USER").unwrap_or_default();
    let admin_pass = std::env::var("S1_ADMIN_PASS").unwrap_or_default();

    if admin_user.is_empty() || admin_pass.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check cookie
    let path = request.uri().path().to_string();

    // Allow login page and login POST without auth
    if path.ends_with("/login") {
        return Ok(next.run(request).await);
    }

    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(cookies) = cookie_header.to_str() {
            let mut auth_valid = false;
            let mut session_ts: Option<i64> = None;

            for cookie in cookies.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("s1_admin=") {
                    use base64::Engine as _;
                    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(token) {
                        if let Ok(creds) = String::from_utf8(decoded) {
                            let expected = format!("{}:{}", admin_user, admin_pass);
                            if creds == expected {
                                auth_valid = true;
                            }
                        }
                    }
                }
                if let Some(ts_str) = cookie.strip_prefix("s1_admin_ts=") {
                    session_ts = ts_str.trim().parse::<i64>().ok();
                }
            }

            if auth_valid {
                // Enforce session timeout
                if let Some(ts) = session_ts {
                    let now = chrono::Utc::now().timestamp();
                    if (now - ts) > SESSION_MAX_AGE_SECS as i64 {
                        // Session expired — clear cookies and redirect to login
                        return Ok((
                            StatusCode::SEE_OTHER,
                            [
                                (header::LOCATION, "/admin/login".to_string()),
                                (
                                    header::SET_COOKIE,
                                    "s1_admin=; Path=/admin; Max-Age=0; HttpOnly".to_string(),
                                ),
                            ],
                            "",
                        )
                            .into_response());
                    }
                }
                return Ok(next.run(request).await);
            }
        }
    }

    // Not authenticated — redirect to login
    Ok(Redirect::to("/admin/login").into_response())
}

/// Login page.
pub async fn admin_login_page() -> Html<&'static str> {
    Html(LOGIN_HTML)
}

/// Extract client IP from request headers (X-Forwarded-For, X-Real-IP) or fallback.
fn extract_client_ip(headers: &axum::http::HeaderMap) -> IpAddr {
    // Try X-Forwarded-For first (first IP in chain)
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }
    // Try X-Real-IP
    if let Some(xri) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = xri.trim().parse::<IpAddr>() {
            return ip;
        }
    }
    // Fallback to unspecified
    IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
}

/// Handle login POST with rate limiting.
pub async fn admin_login_submit(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(form): axum::extract::Form<LoginForm>,
) -> Response {
    let ip = extract_client_ip(&headers);

    // Rate limiting: max 5 attempts per IP per minute
    if let Err(remaining) = state.login_limiter.check_and_increment(ip).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Html(format!(
                r#"<!DOCTYPE html><html><body style="font-family:sans-serif;text-align:center;padding:60px">
                <h2>Too many login attempts</h2><p>Try again in {} seconds.</p>
                <a href="/admin/login">Back to login</a></body></html>"#,
                remaining
            )),
        )
            .into_response();
    }

    let admin_user = std::env::var("S1_ADMIN_USER").unwrap_or_default();
    let admin_pass = std::env::var("S1_ADMIN_PASS").unwrap_or_default();

    if form.username == admin_user && form.password == admin_pass {
        // Successful login — reset rate limiter for this IP
        state.login_limiter.reset(ip).await;

        use base64::Engine as _;
        let token = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", form.username, form.password));
        let now_ts = chrono::Utc::now().timestamp();

        // Set both auth cookie and timestamp cookie (for session timeout)
        let auth_cookie = format!(
            "s1_admin={}; Path=/admin; HttpOnly; SameSite=Strict; Max-Age={}",
            token, SESSION_MAX_AGE_SECS
        );
        let ts_cookie = format!(
            "s1_admin_ts={}; Path=/admin; HttpOnly; SameSite=Strict; Max-Age={}",
            now_ts, SESSION_MAX_AGE_SECS
        );

        // We need to return two Set-Cookie headers. Build a response manually.
        let mut resp = (StatusCode::SEE_OTHER, "").into_response();
        resp.headers_mut()
            .insert(header::LOCATION, "/admin/dashboard".parse().unwrap());
        resp.headers_mut()
            .append(header::SET_COOKIE, auth_cookie.parse().unwrap());
        resp.headers_mut()
            .append(header::SET_COOKIE, ts_cookie.parse().unwrap());
        resp
    } else {
        Html(LOGIN_ERROR_HTML).into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// Admin dashboard.
pub async fn admin_dashboard() -> Html<String> {
    Html(ADMIN_HTML.to_string())
}

/// Stats JSON.
pub async fn admin_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let sessions = state.sessions.list_sessions().await;
    let rooms = state.rooms.room_count().await;
    let total_editors: usize = sessions.iter().map(|s| s.editor_count).sum();
    Json(json!({
        "uptime_secs": uptime_secs(),
        "version": env!("CARGO_PKG_VERSION"),
        "active_sessions": sessions.len(),
        "active_rooms": rooms,
        "total_editors": total_editors,
        "memory_mb": get_memory_mb(),
    }))
}

/// List sessions.
pub async fn admin_sessions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let sessions = state.sessions.list_sessions().await;
    Json(json!({ "sessions": sessions }))
}

/// Config (sanitized).
pub async fn admin_config() -> Json<serde_json::Value> {
    Json(json!({
        "port": std::env::var("S1_PORT").unwrap_or_else(|_| "8080".into()),
        "storage": std::env::var("S1_STORAGE").unwrap_or_else(|_| "local".into()),
        "data_dir": std::env::var("S1_DATA_DIR").unwrap_or_else(|_| "./data".into()),
        "static_dir": std::env::var("S1_STATIC_DIR").unwrap_or_else(|_| "./public".into()),
        "admin_configured": true,
        "log_level": std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
    }))
}

/// Force close session.
pub async fn admin_close_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .sessions
        .force_close(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or((StatusCode::NOT_FOUND, format!("Session not found: {id}")))
}

// ─── New Admin API Endpoints ─────────────────────────

/// GET /admin/api/errors — Return the last errors from the in-memory ring buffer.
///
/// Query parameters:
/// - `limit` (default 50, max 100): number of entries to return
/// - `offset` (default 0): pagination offset
///
/// Returns JSON:
/// ```json
/// { "errors": [...], "total": N, "limit": 50, "offset": 0 }
/// ```
pub async fn admin_errors(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ErrorQueryParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    let (entries, total) = state.error_log.recent(limit, offset).await;
    Json(json!({
        "errors": entries,
        "total": total,
        "limit": limit,
        "offset": offset,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct ErrorQueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /admin/api/sessions/{id}/editors — Return detailed editor info for a session.
///
/// Returns JSON:
/// ```json
/// {
///   "file_id": "...",
///   "editors": [
///     { "user_id": "...", "user_name": "...", "connected_at": "...", "mode": "edit", "last_activity": "..." }
///   ],
///   "editor_count": N
/// }
/// ```
pub async fn admin_session_editors(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let info = state
        .sessions
        .get_info(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Session not found: {id}")))?;

    Ok(Json(json!({
        "file_id": id,
        "editors": info.editors,
        "editor_count": info.editor_count,
    })))
}

/// POST /admin/api/sessions/{id}/sync — Force a fullSync broadcast to all peers in a room.
///
/// Sends a `requestFullSync` message to all connected peers so they re-send
/// the complete document state. Returns 204 on success, 404 if room not found.
pub async fn admin_force_sync(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if state.rooms.broadcast_sync(&id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, format!("Room not found: {id}")))
    }
}

/// POST /admin/api/sessions/{id}/kick/{uid} — Kick an editor from a session.
///
/// Removes the editor from the file session manager and broadcasts a
/// `peer-leave` message to all remaining peers in the room.
/// Returns 204 on success, 404 if session or editor not found.
pub async fn admin_kick_editor(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((id, uid)): axum::extract::Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify the session exists
    let info = state
        .sessions
        .get_info(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Session not found: {id}")))?;

    // Verify the editor exists in the session
    if !info.editors.iter().any(|e| e.user_id == uid) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Editor {} not found in session {}", uid, id),
        ));
    }

    // Remove from file session
    state.sessions.editor_leave(&id, &uid).await;

    // Broadcast peer-leave to the room
    state.rooms.broadcast_peer_leave(&id, &uid).await;

    tracing::info!("Admin kicked editor {} from session {}", uid, id);
    Ok(StatusCode::NO_CONTENT)
}

/// GET /admin/api/health — Return system health metrics.
///
/// Returns JSON:
/// ```json
/// {
///   "status": "ok",
///   "uptime_secs": N,
///   "memory_mb": F,
///   "cpu_usage_percent": null,
///   "active_sessions": N,
///   "active_rooms": N,
///   "total_editors": N,
///   "pid": N
/// }
/// ```
///
/// Note: `cpu_usage_percent` requires the `sysinfo` crate and is reported as null
/// in this implementation. Memory is derived from /proc/self/status on Linux.
pub async fn admin_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let sessions = state.sessions.list_sessions().await;
    let rooms = state.rooms.room_count().await;
    let total_editors: usize = sessions.iter().map(|s| s.editor_count).sum();

    Json(json!({
        "status": "ok",
        "uptime_secs": uptime_secs(),
        "memory_mb": get_memory_mb(),
        "cpu_usage_percent": serde_json::Value::Null,
        "active_sessions": sessions.len(),
        "active_rooms": rooms,
        "total_editors": total_editors,
        "pid": std::process::id(),
    }))
}

/// GET /admin/api/rooms/{id}/version — Return room version and op count.
///
/// Returns JSON:
/// ```json
/// { "room_id": "...", "version": N, "ops_count": N }
/// ```
pub async fn admin_room_version(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let (version, ops_count) = state
        .rooms
        .get_room_version_info(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Room not found: {id}")))?;

    Ok(Json(json!({
        "room_id": id,
        "version": version,
        "ops_count": ops_count,
    })))
}

/// Logout — clear cookie.
pub async fn admin_logout() -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, "/admin/login"),
            (
                header::SET_COOKIE,
                "s1_admin=; Path=/admin; Max-Age=0; HttpOnly",
            ),
        ],
        "",
    )
        .into_response()
}

fn get_memory_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(v) = kb.parse::<f64>() {
                            return v / 1024.0;
                        }
                    }
                }
            }
        }
    }
    0.0
}

const LOGIN_HTML: &str = r##"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>s1-server Admin Login</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#f5f5f5;display:flex;align-items:center;justify-content:center;height:100vh}
.login-box{background:#fff;padding:32px;border-radius:12px;box-shadow:0 2px 12px rgba(0,0,0,.1);width:340px}
h2{font-size:18px;margin-bottom:20px;color:#333;text-align:center}
label{display:block;font-size:13px;color:#666;margin-bottom:4px}
input{width:100%;padding:10px 12px;border:1px solid #ddd;border-radius:6px;font-size:14px;margin-bottom:16px}
input:focus{outline:none;border-color:#1a73e8}
button{width:100%;padding:10px;background:#1a73e8;color:#fff;border:none;border-radius:6px;font-size:14px;cursor:pointer}
button:hover{background:#1557b0}
.logo{text-align:center;margin-bottom:16px;font-size:24px;color:#1a73e8}
</style></head><body>
<div class="login-box">
<div class="logo">s1-server</div>
<h2>Admin Login</h2>
<form method="POST" action="/admin/login">
<label for="username">Username</label>
<input type="text" id="username" name="username" required autofocus>
<label for="password">Password</label>
<input type="password" id="password" name="password" required>
<button type="submit">Sign In</button>
</form>
</div></body></html>"##;

const LOGIN_ERROR_HTML: &str = r##"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Login Failed</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#f5f5f5;display:flex;align-items:center;justify-content:center;height:100vh}
.login-box{background:#fff;padding:32px;border-radius:12px;box-shadow:0 2px 12px rgba(0,0,0,.1);width:340px}
h2{font-size:18px;margin-bottom:20px;color:#333;text-align:center}
.error{background:#fde8e8;color:#c62828;padding:10px;border-radius:6px;margin-bottom:16px;font-size:13px;text-align:center}
label{display:block;font-size:13px;color:#666;margin-bottom:4px}
input{width:100%;padding:10px 12px;border:1px solid #ddd;border-radius:6px;font-size:14px;margin-bottom:16px}
button{width:100%;padding:10px;background:#1a73e8;color:#fff;border:none;border-radius:6px;font-size:14px;cursor:pointer}
button:hover{background:#1557b0}
.logo{text-align:center;margin-bottom:16px;font-size:24px;color:#1a73e8}
</style></head><body>
<div class="login-box">
<div class="logo">s1-server</div>
<h2>Admin Login</h2>
<div class="error">Invalid username or password</div>
<form method="POST" action="/admin/login">
<label for="username">Username</label>
<input type="text" id="username" name="username" required autofocus>
<label for="password">Password</label>
<input type="password" id="password" name="password" required>
<button type="submit">Sign In</button>
</form>
</div></body></html>"##;

const ADMIN_HTML: &str = r##"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>s1-server Admin</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#f5f5f5;color:#333}
.header{background:#1a73e8;color:#fff;padding:12px 24px;display:flex;align-items:center;gap:12px}
.header h1{font-size:16px;font-weight:500}
.header .logout{margin-left:auto;color:#fff;text-decoration:none;font-size:13px;opacity:.8}
.header .logout:hover{opacity:1}
.container{max-width:1100px;margin:20px auto;padding:0 20px}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:12px;margin-bottom:20px}
.card{background:#fff;border-radius:8px;padding:16px;box-shadow:0 1px 3px rgba(0,0,0,.08)}
.card-label{font-size:11px;color:#888;text-transform:uppercase;letter-spacing:.5px}
.card-value{font-size:26px;font-weight:600;margin-top:2px;color:#1a73e8}
.section{background:#fff;border-radius:8px;padding:16px;box-shadow:0 1px 3px rgba(0,0,0,.08);margin-bottom:12px}
.section h2{font-size:14px;margin-bottom:10px}
table{width:100%;border-collapse:collapse;font-size:12px}
th{text-align:left;padding:6px 8px;border-bottom:2px solid #eee;color:#888;font-weight:500}
td{padding:6px 8px;border-bottom:1px solid #f5f5f5}
.badge{display:inline-block;padding:2px 8px;border-radius:10px;font-size:10px;font-weight:600}
.badge-editing{background:#e8f5e9;color:#2e7d32}
.badge-idle{background:#fff3e0;color:#e65100}
.btn-sm{padding:3px 10px;border:none;border-radius:4px;cursor:pointer;font-size:11px;background:#ef5350;color:#fff}
.btn-sm:hover{background:#c62828}
pre{font-size:11px;background:#f9f9f9;padding:10px;border-radius:4px;overflow-x:auto}
</style></head><body>
<div class="header">
<h1>s1-server Admin</h1>
<span id="ver" style="font-size:11px;opacity:.6"></span>
<a href="/admin/logout" class="logout">Logout</a>
</div>
<div class="container">
<div class="cards" id="cards"></div>
<div class="section"><h2>Active Sessions</h2>
<table><thead><tr><th>ID</th><th>File</th><th>Fmt</th><th>Size</th><th>Editors</th><th>Last Active</th><th>Status</th><th>Age</th><th></th></tr></thead>
<tbody id="sessions"></tbody></table></div>
<div class="section"><h2>Config</h2><pre id="config"></pre></div>
</div>
<script>
async function r(){
 const s=await(await fetch('/admin/api/stats')).json();
 const ss=await(await fetch('/admin/api/sessions')).json();
 const c=await(await fetch('/admin/api/config')).json();
 document.getElementById('ver').textContent='v'+s.version;
 document.getElementById('cards').innerHTML=`
  <div class="card"><div class="card-label">Uptime</div><div class="card-value">${fu(s.uptime_secs)}</div></div>
  <div class="card"><div class="card-label">Sessions</div><div class="card-value">${s.active_sessions}</div></div>
  <div class="card"><div class="card-label">Editors</div><div class="card-value">${s.total_editors}</div></div>
  <div class="card"><div class="card-label">Memory</div><div class="card-value">${s.memory_mb.toFixed(1)}MB</div></div>`;
 const tb=document.getElementById('sessions');
 function esc(s){const d=document.createElement('div');d.textContent=s;return d.innerHTML}
 function la(eds){if(!eds||!eds.length)return'-';const acts=eds.filter(e=>e.last_activity).map(e=>e.last_activity);if(!acts.length)return'-';acts.sort();const t=new Date(acts[acts.length-1]);const ago=Math.floor((Date.now()-t.getTime())/1000);return ago<0?'now':fu(ago)+' ago'}
 function edTip(eds){if(!eds||!eds.length)return'';return eds.map(e=>esc(e.user_name)+' ('+esc(e.mode)+')'+( e.last_activity?' last: '+e.last_activity:'')).join('\\n')}
 tb.innerHTML=ss.sessions.length?ss.sessions.map(s=>`<tr>
  <td style="font-family:monospace;font-size:10px">${esc(s.file_id.substring(0,8))}</td>
  <td>${esc(s.filename)}</td><td>${esc(s.format)}</td><td>${fs(s.size)}</td>
  <td title="${edTip(s.editors)}">${s.editor_count}</td>
  <td>${la(s.editors)}</td>
  <td><span class="badge badge-${esc(s.status)}">${esc(s.status)}</span></td>
  <td>${fu(s.created_at_secs_ago)}</td>
  <td><button class="btn-sm" onclick="cl('${esc(s.file_id)}')">Close</button></td></tr>`).join('')
  :'<tr><td colspan="9" style="text-align:center;color:#ccc;padding:16px">No active sessions</td></tr>';
 document.getElementById('config').textContent=JSON.stringify(c,null,2);
}
async function cl(id){if(!confirm('Close session?'))return;await fetch('/admin/api/sessions/'+encodeURIComponent(id),{method:'DELETE'});r()}
function fu(s){return s<60?s+'s':s<3600?Math.floor(s/60)+'m':Math.floor(s/3600)+'h'}
function fs(b){return b<1024?b+'B':b<1048576?(b/1024).toFixed(1)+'KB':(b/1048576).toFixed(1)+'MB'}
r();setInterval(r,10000);
</script></body></html>"##;
