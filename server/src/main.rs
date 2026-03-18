//! s1-server — HTTP API server for s1engine.
//!
//! Provides REST API for document management, format conversion,
//! and serves the web editor as static files.
//!
//! # Usage
//!
//! ```bash
//! s1-server                    # Start with defaults (port 8080)
//! s1-server --port 3000        # Custom port
//! s1-server --config s1.toml   # Custom config file
//! ```

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod routes;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "s1_server=info,tower_http=info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::load();

    let app = Router::new()
        // Health check
        .route("/health", get(routes::health))
        // API v1
        .nest("/api/v1", api_routes())
        // CORS
        .layer(CorsLayer::permissive())
        // Request tracing
        .layer(TraceLayer::new_for_http())
        // Body limit (64MB for document uploads)
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("s1-server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn api_routes() -> Router {
    Router::new()
        // Document operations
        .route("/documents", post(routes::create_document))
        .route("/documents", get(routes::list_documents))
        // Conversion
        .route("/convert", post(routes::convert_document))
        // Info
        .route("/info", get(routes::server_info))
}
