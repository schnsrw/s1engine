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

/// Handle `/edit?token=JWT`
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
///
/// Includes SSRF protection (private IP block) and optional HMAC signature
/// via `X-S1-Signature` header (when `S1_CALLBACK_SECRET` is set).
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

    // SSRF mitigation: block private/internal IP addresses
    if is_private_ip(&callback_url) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Callback URL resolves to a private/internal address".into(),
        ));
    }

    let client = reqwest::Client::new();
    let mut req = client
        .post(&callback_url)
        .header("Content-Type", "application/octet-stream")
        .header("X-S1-File-Id", &id)
        .header("X-S1-Event", "document.saved")
        .header("X-S1-Timestamp", chrono::Utc::now().timestamp().to_string())
        .timeout(std::time::Duration::from_secs(30));

    // Add HMAC signature if S1_CALLBACK_SECRET is configured
    if let Some(signature) = compute_callback_signature(&data) {
        req = req.header("X-S1-Signature", format!("sha256={}", signature));
    }

    let resp = req
        .body(data.clone())
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Callback failed: {e}")))?;

    let status = resp.status().as_u16();
    tracing::info!(
        "Save callback {} -> {} (HTTP {}, {} bytes)",
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
    } else {
        // No `exp` claim (exp == 0) — reject if S1_REQUIRE_JWT_EXP=true
        let require_exp = std::env::var("S1_REQUIRE_JWT_EXP")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");
        if require_exp {
            return Err("Token missing required 'exp' claim".into());
        }
    }

    Ok(claims)
}

/// Check if a URL points to a private/internal IP address (SSRF mitigation).
///
/// Blocks:
/// - Loopback: 127.0.0.0/8, ::1
/// - Private RFC 1918: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
/// - Link-local: 169.254.0.0/16, fe80::/10
/// - Multicast, broadcast, unspecified
///
/// Returns `true` if the address is private/internal and should be blocked.
pub fn is_private_ip(addr: &str) -> bool {
    use std::net::IpAddr;

    // Try to parse as IP directly
    let ip: Option<IpAddr> = addr.parse().ok();

    // If not a raw IP, try to extract the host from a URL
    let ip = ip.or_else(|| {
        // Strip scheme if present
        let host = addr.split("://").nth(1).unwrap_or(addr);
        // Strip path/port
        let host = host.split('/').next().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        // Strip brackets for IPv6
        let host = host.trim_start_matches('[').trim_end_matches(']');
        host.parse().ok()
    });

    match ip {
        Some(IpAddr::V4(v4)) => {
            v4.is_loopback()              // 127.0.0.0/8
                || v4.is_private()         // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || v4.is_link_local()      // 169.254.0.0/16
                || v4.is_broadcast()       // 255.255.255.255
                || v4.is_unspecified()     // 0.0.0.0
                || v4.is_multicast()       // 224.0.0.0/4
                || v4.is_documentation() // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
        }
        Some(IpAddr::V6(v6)) => {
            v6.is_loopback()        // ::1
                || v6.is_unspecified()  // ::
                || v6.is_multicast()    // ff00::/8
                // fe80::/10 link-local
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // fc00::/7 unique local
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // IPv4-mapped addresses (::ffff:a.b.c.d) — check the mapped v4
                || v6.to_ipv4_mapped().is_some_and(|v4| {
                    v4.is_loopback()
                        || v4.is_private()
                        || v4.is_link_local()
                        || v4.is_broadcast()
                        || v4.is_unspecified()
                })
        }
        None => {
            // Could not parse as IP — check common private hostnames
            let lower = addr.to_lowercase();
            let host = lower.split("://").nth(1).unwrap_or(&lower);
            let host = host.split('/').next().unwrap_or(host);
            let host = host.split(':').next().unwrap_or(host);
            host == "localhost"
                || host.ends_with(".local")
                || host.ends_with(".internal")
                || host == "metadata.google.internal"
                || host == "169.254.169.254"
        }
    }
}

/// Compute HMAC-SHA256 signature for callback request body.
///
/// Uses `S1_CALLBACK_SECRET` env var as the signing key.
/// Returns `None` if the secret is not configured.
pub fn compute_callback_signature(body: &[u8]) -> Option<String> {
    let secret = std::env::var("S1_CALLBACK_SECRET").ok()?;
    if secret.is_empty() {
        return None;
    }

    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(body);
    let result = mac.finalize().into_bytes();

    // Hex-encode the signature
    Some(result.iter().map(|b| format!("{:02x}", b)).collect())
}

async fn fetch_document(url: &str) -> Result<Vec<u8>, String> {
    // SSRF mitigation: block private/internal IP addresses
    if is_private_ip(url) {
        return Err("Blocked: URL resolves to a private/internal address".into());
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── is_private_ip tests ─────────────────────────

    #[test]
    fn blocks_loopback_v4() {
        assert!(is_private_ip("127.0.0.1"));
        assert!(is_private_ip("127.255.255.255"));
    }

    #[test]
    fn blocks_loopback_v6() {
        assert!(is_private_ip("::1"));
    }

    #[test]
    fn blocks_rfc1918_class_a() {
        assert!(is_private_ip("10.0.0.1"));
        assert!(is_private_ip("10.255.255.255"));
    }

    #[test]
    fn blocks_rfc1918_class_b() {
        assert!(is_private_ip("172.16.0.1"));
        assert!(is_private_ip("172.31.255.255"));
    }

    #[test]
    fn blocks_rfc1918_class_c() {
        assert!(is_private_ip("192.168.0.1"));
        assert!(is_private_ip("192.168.255.255"));
    }

    #[test]
    fn blocks_link_local() {
        assert!(is_private_ip("169.254.0.1"));
        assert!(is_private_ip("169.254.169.254"));
    }

    #[test]
    fn blocks_unspecified() {
        assert!(is_private_ip("0.0.0.0"));
    }

    #[test]
    fn allows_public_ip() {
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("1.1.1.1"));
        assert!(!is_private_ip("203.0.114.1")); // Just outside documentation range
    }

    #[test]
    fn blocks_url_with_private_ip() {
        assert!(is_private_ip("http://10.0.0.1/file.docx"));
        assert!(is_private_ip("https://192.168.1.100:8080/callback"));
        assert!(is_private_ip("http://127.0.0.1:3000/api"));
    }

    #[test]
    fn allows_url_with_public_ip() {
        assert!(!is_private_ip("https://8.8.8.8/file.docx"));
        assert!(!is_private_ip("http://1.2.3.4:8080/callback"));
    }

    #[test]
    fn blocks_localhost_hostname() {
        assert!(is_private_ip("http://localhost/file.docx"));
        assert!(is_private_ip("http://localhost:8080/file.docx"));
    }

    #[test]
    fn blocks_metadata_endpoint() {
        assert!(is_private_ip("http://169.254.169.254/latest/meta-data/"));
    }

    #[test]
    fn blocks_internal_hostnames() {
        assert!(is_private_ip("http://myservice.local/api"));
        assert!(is_private_ip(
            "http://metadata.google.internal/computeMetadata/v1/"
        ));
    }

    #[test]
    fn allows_public_hostname() {
        // Non-IP hostnames that don't match known private patterns
        assert!(!is_private_ip("http://example.com/file.docx"));
        assert!(!is_private_ip("https://api.github.com/repos"));
    }

    // ─── compute_callback_signature tests ────────────

    /// Test HMAC signature: no secret returns None, with secret returns valid hex,
    /// same inputs produce same output, different inputs produce different output.
    ///
    /// Combined into one test to avoid env var race conditions in parallel tests.
    #[test]
    fn callback_signature_behavior() {
        // Without secret → None
        std::env::remove_var("S1_CALLBACK_SECRET");
        assert!(compute_callback_signature(b"test data").is_none());

        // With secret → valid hex string
        std::env::set_var("S1_CALLBACK_SECRET", "test-secret-key-123");
        let sig = compute_callback_signature(b"hello world");
        assert!(sig.is_some());
        let sig = sig.unwrap();
        assert_eq!(sig.len(), 64); // HMAC-SHA256 = 32 bytes = 64 hex chars
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));

        // Deterministic: same data + same key → same signature
        let sig2 = compute_callback_signature(b"hello world");
        assert_eq!(Some(sig.clone()), sig2);

        // Different data → different signature
        let sig3 = compute_callback_signature(b"different data");
        assert_ne!(Some(sig), sig3);

        // Clean up
        std::env::remove_var("S1_CALLBACK_SECRET");
    }
}
