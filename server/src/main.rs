//! s1-server — HTTP API server for s1engine.
//!
//! Provides REST API for document management, format conversion,
//! real-time collaboration via WebSocket, and serves the web editor.

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod collab;
mod config;
mod routes;
mod storage;
mod webhooks;

use collab::RoomManager;
use routes::AppState;
use storage::{LocalStorage, MemoryStorage};
use webhooks::WebhookRegistry;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "s1_server=info,tower_http=info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::load();

    // Initialize storage backend
    let storage: Arc<dyn storage::StorageBackend> = match config.storage.as_str() {
        "memory" => {
            tracing::info!("Using in-memory storage (data lost on restart)");
            Arc::new(MemoryStorage::new())
        }
        _ => {
            tracing::info!("Using local storage at {}", config.data_dir);
            Arc::new(
                LocalStorage::new(&config.data_dir)
                    .expect("Failed to create storage directory"),
            )
        }
    };

    let webhook_registry = Arc::new(WebhookRegistry::new());
    let room_manager = Arc::new(RoomManager::new());

    // Clone for background task before moving into state
    let save_rooms = room_manager.clone();
    let save_storage = storage.clone();

    let state = Arc::new(AppState {
        storage,
        webhooks: webhook_registry,
        rooms: room_manager,
    });

    let app = Router::new()
        .route("/health", get(routes::health))
        // WebSocket collaboration
        .route("/ws/collab/{room_id}", get(collab::ws_collab_handler))
        // REST API
        .nest("/api/v1", api_routes())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024));

    // P4-08: Spawn background auto-save task for dirty rooms
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            save_rooms.save_dirty_rooms(save_storage.as_ref()).await;
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("s1-server starting on http://{}", addr);
    tracing::info!("  REST API: http://{}/api/v1/", addr);
    tracing::info!("  WebSocket: ws://{}/ws/collab/{{room_id}}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Documents
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
        // Info
        .route("/info", get(routes::server_info))
}
