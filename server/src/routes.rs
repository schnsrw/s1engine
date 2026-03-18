//! API route handlers.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::collab::RoomManager;
use crate::storage::{DocumentMeta, StorageBackend};
use crate::webhooks::{Webhook, WebhookRegistry};

/// Shared application state.
pub struct AppState {
    pub storage: Arc<dyn StorageBackend>,
    pub webhooks: Arc<WebhookRegistry>,
    pub rooms: Arc<RoomManager>,
}

/// Health check endpoint.
pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Server info endpoint.
pub async fn server_info() -> Json<Value> {
    Json(json!({
        "name": "s1-server",
        "version": env!("CARGO_PKG_VERSION"),
        "engine_version": "1.0.1",
        "supported_formats": {
            "read": ["docx", "odt", "txt", "md", "doc"],
            "write": ["docx", "odt", "pdf", "txt", "md"],
        },
    }))
}

/// Create a new document (upload).
pub async fn create_document(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("document").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read file: {e}")))?;

            // Validate with s1engine
            let engine = s1engine::Engine::new();
            let doc = engine
                .open(&data)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid document: {e}")))?;

            let doc_id = uuid::Uuid::new_v4().to_string();
            let text = doc.to_plain_text();
            let word_count = text.split_whitespace().count();
            let now = chrono::Utc::now().to_rfc3339();

            // Detect format from filename
            let format = filename
                .rsplit('.')
                .next()
                .unwrap_or("bin")
                .to_lowercase();

            let meta = DocumentMeta {
                id: doc_id.clone(),
                filename: filename.clone(),
                format,
                size: data.len(),
                title: doc.metadata().title.clone(),
                author: doc.metadata().creator.clone(),
                word_count,
                created_at: now.clone(),
                updated_at: now,
            };

            state
                .storage
                .put(&doc_id, &data, &meta)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {e}")))?;

            return Ok((
                StatusCode::CREATED,
                Json(json!({
                    "id": doc_id,
                    "filename": filename,
                    "size": data.len(),
                    "wordCount": word_count,
                    "title": doc.metadata().title,
                })),
            ));
        }
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

/// Pagination query params.
#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize { 1 }
fn default_page_size() -> usize { 20 }

/// List documents (paginated).
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let offset = (params.page.saturating_sub(1)) * params.page_size;
    let (docs, total) = state
        .storage
        .list(offset, params.page_size)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {e}")))?;

    Ok(Json(json!({
        "documents": docs,
        "total": total,
        "page": params.page,
        "pageSize": params.page_size,
    })))
}

/// Get a document by ID (returns bytes).
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Vec<u8>, (StatusCode, String)> {
    state
        .storage
        .get(&id)
        .map_err(|e| match e {
            crate::storage::StorageError::NotFound(_) => {
                (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
            }
            other => (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {other}")),
        })
}

/// Get document metadata by ID.
pub async fn get_document_meta(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let meta = state
        .storage
        .get_meta(&id)
        .map_err(|e| match e {
            crate::storage::StorageError::NotFound(_) => {
                (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
            }
            other => (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {other}")),
        })?;

    Ok(Json(serde_json::to_value(meta).unwrap_or_default()))
}

/// Delete a document by ID.
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .storage
        .delete(&id)
        .map_err(|e| match e {
            crate::storage::StorageError::NotFound(_) => {
                (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
            }
            other => (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {other}")),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Stateless format conversion.
pub async fn convert_document(
    mut multipart: Multipart,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut target_format = String::from("pdf");

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Read error: {e}")))?
                        .to_vec(),
                );
            }
            "format" => {
                target_format = field
                    .text()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Read error: {e}")))?;
            }
            _ => {}
        }
    }

    let data = file_data.ok_or((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))?;

    let engine = s1engine::Engine::new();
    let doc = engine
        .open(&data)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid document: {e}")))?;

    let format = match target_format.as_str() {
        "docx" => s1engine::Format::Docx,
        "odt" => s1engine::Format::Odt,
        "pdf" => s1engine::Format::Pdf,
        "txt" => s1engine::Format::Txt,
        "md" => s1engine::Format::Md,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Unsupported target format: {other}"),
            ))
        }
    };

    doc.export(format)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Export failed: {e}")))
}

// ─── Thumbnail ──────────────────────────────────────

/// Get a thumbnail of the first page of a document (PDF export, first page only).
///
/// Returns the document exported as PDF bytes. A real implementation would
/// render just the first page to a PNG using a headless renderer.
pub async fn get_thumbnail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let data = state
        .storage
        .get(&id)
        .map_err(|e| match e {
            crate::storage::StorageError::NotFound(_) => {
                (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
            }
            other => (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {other}")),
        })?;

    let engine = s1engine::Engine::new();
    let doc = engine
        .open(&data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Open failed: {e}")))?;

    // Export as PDF (first page thumbnail would require a rasterizer;
    // for now we return a full PDF — consumers can render page 1 client-side).
    doc.export(s1engine::Format::Pdf)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("PDF export failed: {e}")))
}

// ─── Webhooks ───────────────────────────────────────

/// Register a new webhook.
pub async fn register_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    let url = body["url"]
        .as_str()
        .ok_or((StatusCode::BAD_REQUEST, "Missing 'url' field".to_string()))?
        .to_string();

    let events: Vec<String> = body["events"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(|| vec!["*".to_string()]);

    let webhook = Webhook {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        events,
        active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let id = webhook.id.clone();
    state.webhooks.register(webhook);

    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

/// List all registered webhooks.
pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let hooks = state.webhooks.list();
    Json(json!({ "webhooks": hooks, "total": hooks.len() }))
}

/// Delete a webhook by ID.
pub async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if state.webhooks.remove(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, format!("Webhook not found: {id}")))
    }
}
