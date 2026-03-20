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
use std::sync::Arc;
use std::time::Instant;

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

/// Admin auth middleware — checks for `s1_admin` cookie.
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
            for cookie in cookies.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("s1_admin=") {
                    use base64::Engine as _;
                    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(token) {
                        if let Ok(creds) = String::from_utf8(decoded) {
                            let expected = format!("{}:{}", admin_user, admin_pass);
                            if creds == expected {
                                return Ok(next.run(request).await);
                            }
                        }
                    }
                }
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

/// Handle login POST.
pub async fn admin_login_submit(
    axum::extract::Form(form): axum::extract::Form<LoginForm>,
) -> Response {
    let admin_user = std::env::var("S1_ADMIN_USER").unwrap_or_default();
    let admin_pass = std::env::var("S1_ADMIN_PASS").unwrap_or_default();

    if form.username == admin_user && form.password == admin_pass {
        use base64::Engine as _;
        let token = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", form.username, form.password));
        (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, "/admin/dashboard"),
                (
                    header::SET_COOKIE,
                    &format!("s1_admin={}; Path=/admin; HttpOnly; SameSite=Strict", token),
                ),
            ],
            "",
        )
            .into_response()
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
