//! Integration mode — JWT-based file editing for embedding in other products.
#![allow(dead_code)]

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::routes::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct IntegrationClaims {
    pub file_id: String,
    pub user_id: String,
    pub user_name: String,
    #[serde(default = "default_permissions")]
    pub permissions: String,
    pub download_url: Option<String>,
    pub callback_url: Option<String>,
    #[serde(default)]
    pub exp: u64,
}

fn default_permissions() -> String {
    "edit".to_string()
}

#[derive(Debug, Deserialize)]
pub struct EditQuery {
    pub token: Option<String>,
}

/// Handle /edit?token=<jwt>
pub async fn handle_edit(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EditQuery>,
) -> Result<Response, (StatusCode, String)> {
    let token = query
        .token
        .ok_or((StatusCode::BAD_REQUEST, "Missing token parameter".into()))?;

    let jwt_secret = std::env::var("S1_JWT_SECRET").unwrap_or_default();
    let claims = validate_integration_jwt(&token, &jwt_secret)
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {e}")))?;

    if state.sessions.exists(&claims.file_id).await {
        return Ok(Redirect::to(&format!(
            "/?file={}&mode={}",
            claims.file_id, claims.permissions
        ))
        .into_response());
    }

    let data = if let Some(url) = &claims.download_url {
        fetch_document(url).await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch document: {e}"),
            )
        })?
    } else {
        Vec::new()
    };

    let format = claims
        .download_url
        .as_deref()
        .and_then(|u| u.rsplit('.').next())
        .unwrap_or("docx")
        .to_lowercase();

    let filename = format!("{}.{}", claims.file_id, format);

    state
        .sessions
        .create(
            claims.file_id.clone(),
            filename,
            data,
            format,
            Some(claims.user_id.clone()),
            claims.callback_url.clone(),
        )
        .await;

    Ok(Redirect::to(&format!(
        "/?file={}&mode={}",
        claims.file_id, claims.permissions
    ))
    .into_response())
}

/// POST /api/v1/files/{id}/save — trigger save callback to host.
pub async fn trigger_save_callback(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let data = state
        .sessions
        .get_data(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Session not found".into()))?;

    let callback_url = state.sessions.get_callback_url(&id).await.ok_or((
        StatusCode::BAD_REQUEST,
        "No callback URL for this session".into(),
    ))?;

    let client = reqwest::Client::new();
    let resp = client
        .post(&callback_url)
        .header("Content-Type", "application/octet-stream")
        .header("X-S1-File-Id", &id)
        .header("X-S1-Event", "document.saved")
        .body(data.clone())
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Callback failed: {e}")))?;

    let status = resp.status().as_u16();
    tracing::info!(
        "Save callback {} → {} (HTTP {}, {} bytes)",
        id,
        callback_url,
        status,
        data.len()
    );

    Ok(Json(json!({
        "fileId": id,
        "callbackUrl": callback_url,
        "callbackStatus": status,
        "bytesSent": data.len(),
    })))
}

fn validate_integration_jwt(token: &str, secret: &str) -> Result<IntegrationClaims, String> {
    if secret.is_empty() {
        return Err("S1_JWT_SECRET not configured".into());
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".into());
    }

    use base64::Engine as _;
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| format!("Base64 error: {e}"))?;

    let claims: IntegrationClaims =
        serde_json::from_slice(&payload_bytes).map_err(|e| format!("Invalid claims: {e}"))?;

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| format!("HMAC error: {e}"))?;
    mac.update(signing_input.as_bytes());

    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| format!("Signature decode error: {e}"))?;

    mac.verify_slice(&sig_bytes)
        .map_err(|_| "Invalid signature".to_string())?;

    if claims.exp > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now > claims.exp {
            return Err("Token expired".into());
        }
    }

    Ok(claims)
}

async fn fetch_document(url: &str) -> Result<Vec<u8>, String> {
    let resp = reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Fetch error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Read error: {e}"))
}
