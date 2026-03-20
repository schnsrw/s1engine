//! s1-server — Unified document editing server.
//!
//! Single binary that serves:
//! - Static editor files (HTML/JS/CSS/WASM) at `/`
//! - REST API at `/api/v1/`
//! - WebSocket collaborative editing at `/ws/edit/{file_id}`
//! - File sessions with auto-cleanup
//!
//! No nginx, no Node.js, no relay.js needed.

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod admin;
mod auth;
mod collab;
mod config;
mod file_sessions;
mod hooks;
mod integration;
mod plugins;
mod routes;
mod storage;
mod webhooks;

use collab::RoomManager;
use file_sessions::FileSessionManager;
use routes::AppState;
use storage::{LocalStorage, MemoryStorage};
use webhooks::WebhookRegistry;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "s1_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::load();

    // Storage backend
    let storage: Arc<dyn storage::StorageBackend> = match config.storage.as_str() {
        "memory" => {
            tracing::info!("Storage: in-memory");
            Arc::new(MemoryStorage::new())
        }
        _ => {
            tracing::info!("Storage: local ({})", config.data_dir);
            Arc::new(
                LocalStorage::new(&config.data_dir).expect("Failed to create storage directory"),
            )
        }
    };

    let webhook_registry = Arc::new(WebhookRegistry::new());
    let room_manager = Arc::new(RoomManager::new());
    let session_manager = Arc::new(FileSessionManager::new(None));

    // Clones for background tasks
    let save_rooms = room_manager.clone();
    let save_storage = storage.clone();
    let cleanup_sessions = session_manager.clone();

    let state = Arc::new(AppState {
        storage,
        webhooks: webhook_registry,
        rooms: room_manager,
        sessions: session_manager,
    });

    // Static editor files directory
    let static_dir = std::env::var("S1_STATIC_DIR").unwrap_or_else(|_| "./public".to_string());

    // Initialize admin start time
    admin::init_start_time();

    // Startup warnings
    if std::env::var("S1_JWT_SECRET")
        .unwrap_or_default()
        .is_empty()
    {
        tracing::warn!(
            "S1_JWT_SECRET not set — /edit?token= integration mode will reject all tokens"
        );
    }
    let auth_enabled = std::env::var("S1_AUTH_ENABLED")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");
    if !auth_enabled {
        tracing::warn!(
            "Authentication disabled (S1_AUTH_ENABLED=false) — all endpoints are public"
        );
    }

    let app = Router::new()
        // Health
        .route("/health", get(routes::health))
        // WebSocket editing (per file) — supports both URL patterns
        .route("/ws/edit/{file_id}", get(collab::ws_collab_handler))
        .route("/ws/collab/{file_id}", get(collab::ws_collab_handler))
        // Integration entry point: /edit?token=<jwt>
        .route("/edit", get(integration::handle_edit))
        // REST API
        .nest("/api/v1", api_routes())
        // Admin panel (protected by Basic Auth)
        .nest("/admin", admin_routes())
        .with_state(state)
        // Static editor files (fallback for SPA routing)
        .fallback_service(ServeDir::new(&static_dir).append_index_html_on_directories(true))
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024));

    // Background: save dirty collab rooms every 30s
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            save_rooms.save_dirty_rooms(save_storage.as_ref()).await;
        }
    });

    // Background: clean up expired file sessions every 60s + POST callbacks
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            // Remove editors whose last_activity is older than 5 minutes
            let stale = cleanup_sessions.cleanup_stale_editors().await;
            for (fid, _) in &stale {
                tracing::info!("Removed stale editor from session {}", fid);
            }
            let expired = cleanup_sessions.cleanup_expired().await;
            for (file_id, callback_url, data) in expired {
                tracing::info!("Session expired: {} ({} bytes)", file_id, data.len());
                // C-05: POST final document to callback URL if configured
                if let Some(url) = callback_url {
                    if !url.is_empty() {
                        tracing::info!("Posting callback for {} to {}", file_id, url);
                        let req = client
                            .post(&url)
                            .header("Content-Type", "application/octet-stream")
                            .header("X-S1-File-Id", &file_id)
                            .header("X-S1-Event", "session.closed")
                            .body(data);
                        match req.send().await {
                            Ok(resp) => tracing::info!("Callback {}: HTTP {}", url, resp.status()),
                            Err(e) => tracing::warn!("Callback {} failed: {}", url, e),
                        }
                    }
                }
            }
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("═══════════════════════════════════════");
    tracing::info!("  s1-server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("───────────────────────────────────────");
    tracing::info!("  Editor:    http://{}/", addr);
    tracing::info!("  API:       http://{}/api/v1/", addr);
    tracing::info!("  WebSocket: ws://{}/ws/edit/{{file_id}}", addr);
    tracing::info!("  Admin:     http://{}/admin/dashboard", addr);
    tracing::info!("  Static:    {}", static_dir);
    tracing::info!("═══════════════════════════════════════");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // File sessions (temp editing with TTL)
        .route("/files", post(routes::upload_file))
        .route("/files", get(routes::list_files))
        .route("/files/{id}", get(routes::get_file_info))
        .route("/files/{id}/download", get(routes::download_file))
        .route("/files/{id}", delete(routes::close_file))
        .route("/files/{id}/save", post(integration::trigger_save_callback))
        // Documents (persistent storage)
        .route("/documents", post(routes::create_document))
        .route("/documents", get(routes::list_documents))
        .route("/documents/{id}", get(routes::get_document_meta))
        .route("/documents/{id}/content", get(routes::get_document))
        .route("/documents/{id}", delete(routes::delete_document))
        .route("/documents/{id}/thumbnail", get(routes::get_thumbnail))
        // Conversion
        .route("/convert", post(routes::convert_document))
        // Webhooks
        .route("/webhooks", post(routes::register_webhook))
        .route("/webhooks", get(routes::list_webhooks))
        .route("/webhooks/{id}", delete(routes::delete_webhook))
        // Error reporting
        .route("/errors", post(routes::report_error))
        // Info
        .route("/info", get(routes::server_info))
}

fn admin_routes() -> Router<Arc<AppState>> {
    use axum::middleware;
    use axum::routing::post;

    Router::new()
        .route("/login", get(admin::admin_login_page))
        .route("/login", post(admin::admin_login_submit))
        .route("/logout", get(admin::admin_logout))
        .route("/dashboard", get(admin::admin_dashboard))
        .route("/api/stats", get(admin::admin_stats))
        .route("/api/sessions", get(admin::admin_sessions))
        .route("/api/sessions/{id}", delete(admin::admin_close_session))
        .route("/api/config", get(admin::admin_config))
        .layer(middleware::from_fn(admin::admin_auth))
}
