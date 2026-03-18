//! API route handlers.

use axum::{extract::Multipart, http::StatusCode, Json};
use serde_json::{json, Value};

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

/// Create a new document (upload or empty).
pub async fn create_document(
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("document").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read file: {e}")))?;

            // Open with s1engine to validate
            let engine = s1engine::Engine::new();
            let doc = engine
                .open(&data)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid document: {e}")))?;

            let doc_id = uuid::Uuid::new_v4().to_string();
            let text = doc.to_plain_text();
            let word_count = text.split_whitespace().count();

            return Ok(Json(json!({
                "id": doc_id,
                "filename": filename,
                "size": data.len(),
                "wordCount": word_count,
                "title": doc.metadata().title,
            })));
        }
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

/// List documents (placeholder — needs storage backend).
pub async fn list_documents() -> Json<Value> {
    Json(json!({
        "documents": [],
        "total": 0,
        "page": 1,
        "pageSize": 20,
    }))
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
