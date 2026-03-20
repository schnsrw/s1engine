//! API route handlers.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::collab::RoomManager;
use crate::file_sessions::FileSessionManager;
use crate::storage::{DocumentMeta, StorageBackend};
use crate::webhooks::{Webhook, WebhookRegistry};

/// Shared application state.
pub struct AppState {
    pub storage: Arc<dyn StorageBackend>,
    pub webhooks: Arc<WebhookRegistry>,
    pub rooms: Arc<RoomManager>,
    pub sessions: Arc<FileSessionManager>,
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
            let format = filename.rsplit('.').next().unwrap_or("bin").to_lowercase();

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

            state.storage.put(&doc_id, &data, &meta).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Storage error: {e}"),
                )
            })?;

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

fn default_page() -> usize {
    1
}
fn default_page_size() -> usize {
    20
}

/// List documents (paginated).
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let offset = (params.page.saturating_sub(1)) * params.page_size;
    let (docs, total) = state.storage.list(offset, params.page_size).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage error: {e}"),
        )
    })?;

    Ok(Json(json!({
        "documents": docs,
        "total": total,
        "page": params.page,
        "pageSize": params.page_size,
    })))
}

/// Get a document by ID (returns bytes with proper Content-Type).
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let meta = state.storage.get_meta(&id).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => {
            (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
        }
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage error: {other}"),
        ),
    })?;
    let data = state.storage.get(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage error: {e}"),
        )
    })?;
    let content_type = format_to_content_type(&meta.format);
    Ok((
        [
            (
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(content_type),
            ),
            (
                header::CONTENT_DISPOSITION,
                header::HeaderValue::from_str(&format!(
                    "attachment; filename=\"{}\"",
                    meta.filename
                ))
                .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
            ),
        ],
        data,
    ))
}

/// Get document metadata by ID.
pub async fn get_document_meta(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let meta = state.storage.get_meta(&id).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => {
            (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
        }
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage error: {other}"),
        ),
    })?;

    Ok(Json(serde_json::to_value(meta).unwrap_or_default()))
}

/// Delete a document by ID.
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.storage.delete(&id).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => {
            (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
        }
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage error: {other}"),
        ),
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Stateless format conversion.
pub async fn convert_document(
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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

    let bytes = doc.export(format).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Export failed: {e}"),
        )
    })?;
    let content_type = format_to_content_type(&target_format);
    Ok((
        [(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(content_type),
        )],
        bytes,
    ))
}

/// Map format string to MIME content type.
fn format_to_content_type(format: &str) -> &'static str {
    match format {
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "odt" => "application/vnd.oasis.opendocument.text",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        _ => "application/octet-stream",
    }
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
    let data = state.storage.get(&id).map_err(|e| match e {
        crate::storage::StorageError::NotFound(_) => {
            (StatusCode::NOT_FOUND, format!("Document not found: {id}"))
        }
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage error: {other}"),
        ),
    })?;

    let engine = s1engine::Engine::new();
    let doc = engine.open(&data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Open failed: {e}"),
        )
    })?;

    // Export as PDF (first page thumbnail would require a rasterizer;
    // for now we return a full PDF — consumers can render page 1 client-side).
    doc.export(s1engine::Format::Pdf).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("PDF export failed: {e}"),
        )
    })
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
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
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
pub async fn list_webhooks(State(state): State<Arc<AppState>>) -> Json<Value> {
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

// ─── File Sessions (Temp Editing) ──────────────────

/// Upload a file and create an editing session.
///
/// Returns the fileId and editor URL. The session stays alive while
/// editors are connected, then expires after configurable TTL.
pub async fn upload_file(
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
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Read error: {e}")))?;

            // Validate with s1engine
            let engine = s1engine::Engine::new();
            let doc = engine
                .open(&data)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid document: {e}")))?;

            let file_id = uuid::Uuid::new_v4().to_string();
            let format = filename.rsplit('.').next().unwrap_or("bin").to_lowercase();
            let word_count = doc.to_plain_text().split_whitespace().count();

            state
                .sessions
                .create(
                    file_id.clone(),
                    filename.clone(),
                    data.to_vec(),
                    format,
                    None,
                    None,
                )
                .await;

            return Ok((
                StatusCode::CREATED,
                Json(json!({
                    "fileId": file_id,
                    "filename": filename,
                    "size": data.len(),
                    "wordCount": word_count,
                    "editorUrl": format!("/?file={}", file_id),
                    "wsUrl": format!("/ws/edit/{}", file_id),
                })),
            ));
        }
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

/// List all active file editing sessions.
pub async fn list_files(State(state): State<Arc<AppState>>) -> Json<Value> {
    let sessions = state.sessions.list_sessions().await;
    Json(json!({ "files": sessions, "total": sessions.len() }))
}

/// Get info about a file editing session.
pub async fn get_file_info(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let info = state.sessions.get_info(&id).await.ok_or((
        StatusCode::NOT_FOUND,
        format!("File session not found: {id}"),
    ))?;

    // TODO: When auth is enabled, check permission:
    // let user = request.extensions().get::<AuthUser>();
    // if let Some(user) = user {
    //     auth::check_permission_with_session(user, info.owner_id.as_deref(), &info.mode, Permission::Viewer)?;
    // }

    Ok(Json(serde_json::to_value(info).unwrap_or_default()))
}

/// Download the latest version of a file being edited.
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let info = state.sessions.get_info(&id).await.ok_or((
        StatusCode::NOT_FOUND,
        format!("File session not found: {id}"),
    ))?;

    // TODO: When auth is enabled, check permission:
    // let user = request.extensions().get::<AuthUser>();
    // if let Some(user) = user {
    //     auth::check_permission_with_session(user, info.owner_id.as_deref(), &info.mode, Permission::Viewer)?;
    // }

    let data = state.sessions.get_data(&id).await.ok_or((
        StatusCode::NOT_FOUND,
        format!("File session not found: {id}"),
    ))?;
    let content_type = format_to_content_type(&info.format);
    Ok((
        [
            (
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(content_type),
            ),
            (
                header::CONTENT_DISPOSITION,
                header::HeaderValue::from_str(&format!(
                    "attachment; filename=\"{}\"",
                    info.filename
                ))
                .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
            ),
        ],
        data,
    ))
}

/// Force close a file editing session. Returns the final document bytes.
pub async fn close_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Vec<u8>, (StatusCode, String)> {
    // TODO: When auth is enabled, check permission:
    // let user = request.extensions().get::<AuthUser>();
    // if let Some(user) = user {
    //     let info = state.sessions.get_info(&id).await;
    //     if let Some(ref info) = info {
    //         auth::check_permission_with_session(user, info.owner_id.as_deref(), &info.mode, Permission::Editor)?;
    //     }
    // }

    state.sessions.force_close(&id).await.ok_or((
        StatusCode::NOT_FOUND,
        format!("File session not found: {id}"),
    ))
}
