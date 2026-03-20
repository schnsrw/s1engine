//! Authentication and authorization middleware.
//!
//! Supports JWT tokens (HS256) and API keys. Extracts user identity
//! and permissions for use in route handlers.
//!
//! This module is not wired into the router by default. Enable authentication
//! by adding `auth_middleware` as an Axum middleware layer.
#![allow(dead_code, unused_imports)]

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Extracted user identity from JWT or API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    /// User ID (from JWT `sub` claim or API key owner).
    pub user_id: String,
    /// Display name.
    pub name: String,
    /// Tenant ID for multi-tenancy isolation.
    pub tenant_id: Option<String>,
    /// Global role (admin, user).
    pub role: UserRole,
}

/// User roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
    Anonymous,
}

/// Document-level permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    /// Can only view the document.
    Viewer = 0,
    /// Can view and add comments.
    Commenter = 1,
    /// Can view, comment, and edit.
    Editor = 2,
    /// Can do everything an editor can, plus manage permissions.
    Admin = 3,
    /// Full control (delete, transfer ownership).
    Owner = 4,
}

/// Auth configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication. If false, all requests get anonymous access.
    pub enabled: bool,
    /// JWT secret key for HS256 validation.
    pub jwt_secret: Option<String>,
    /// Allow anonymous access (read-only by default).
    pub allow_anonymous: bool,
    /// API keys (static list for now).
    pub api_keys: Vec<ApiKeyEntry>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            allow_anonymous: true,
            api_keys: Vec::new(),
        }
    }
}

/// A registered API key.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyEntry {
    pub key: String,
    pub name: String,
    pub tenant_id: Option<String>,
    pub role: Option<UserRole>,
}

/// Authentication middleware.
///
/// Checks for JWT (Authorization: Bearer ...) or API key (X-API-Key: ...).
/// If auth is disabled, passes through with anonymous identity.
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    // Extract auth config from extensions (set by the app state layer)
    let config = request
        .extensions()
        .get::<Arc<AuthConfig>>()
        .cloned()
        .unwrap_or_else(|| Arc::new(AuthConfig::default()));

    if !config.enabled {
        // Auth disabled — inject anonymous user
        request.extensions_mut().insert(AuthUser {
            user_id: "anonymous".to_string(),
            name: "Anonymous".to_string(),
            tenant_id: None,
            role: UserRole::Anonymous,
        });
        return Ok(next.run(request).await);
    }

    let headers = request.headers();

    // Try JWT (Authorization: Bearer <token>)
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                match validate_jwt(token, &config) {
                    Ok(user) => {
                        request.extensions_mut().insert(user);
                        return Ok(next.run(request).await);
                    }
                    Err(e) => {
                        tracing::warn!("JWT validation failed: {}", e);
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
            }
        }
    }

    // Try API key (X-API-Key: <key>)
    if let Some(key_header) = headers.get("X-API-Key") {
        if let Ok(key) = key_header.to_str() {
            if let Some(entry) = config.api_keys.iter().find(|k| k.key == key) {
                request.extensions_mut().insert(AuthUser {
                    user_id: format!("apikey:{}", entry.name),
                    name: entry.name.clone(),
                    tenant_id: entry.tenant_id.clone(),
                    role: entry.role.unwrap_or(UserRole::User),
                });
                return Ok(next.run(request).await);
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // No credentials — allow anonymous if configured
    if config.allow_anonymous {
        request.extensions_mut().insert(AuthUser {
            user_id: "anonymous".to_string(),
            name: "Anonymous".to_string(),
            tenant_id: None,
            role: UserRole::Anonymous,
        });
        return Ok(next.run(request).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Validate a JWT token (HS256). Returns the extracted user identity.
fn validate_jwt(token: &str, config: &AuthConfig) -> Result<AuthUser, String> {
    let secret = config
        .jwt_secret
        .as_deref()
        .ok_or("JWT secret not configured")?;

    // Decode JWT (header.payload.signature)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }

    // Decode payload (base64url)
    let payload_bytes = base64_url_decode(parts[1])?;
    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|e| format!("Invalid payload: {e}"))?;

    // Verify signature (HS256)
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| format!("HMAC error: {e}"))?;
    mac.update(signing_input.as_bytes());

    let sig_bytes = base64_url_decode(parts[2])?;
    mac.verify_slice(&sig_bytes)
        .map_err(|_| "Invalid signature".to_string())?;

    // Check expiry
    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if now > exp {
            return Err("Token expired".to_string());
        }
    }

    // Extract claims
    Ok(AuthUser {
        user_id: payload
            .get("sub")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        name: payload
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("User")
            .to_string(),
        tenant_id: payload
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        role: match payload.get("role").and_then(|v| v.as_str()) {
            Some("admin") => UserRole::Admin,
            _ => UserRole::User,
        },
    })
}

/// Base64url decode (no padding).
fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| format!("Base64 decode error: {e}"))
}

/// Session-level permission info (extracted from file session or JWT claims).
#[derive(Debug, Clone)]
pub struct DocPermission {
    /// The permission level for this user on this document.
    pub level: Permission,
    /// Whether this user is the owner of the document session.
    pub is_owner: bool,
}

/// Check if a user has at least the given permission for a document.
///
/// Permission resolution order:
/// 1. Admin role → full access
/// 2. Anonymous → Viewer only
/// 3. Session owner → Owner permission
/// 4. Session mode "view" → Viewer, "comment" → Commenter, "edit" → Editor
/// 5. Default for authenticated users without session context → Editor
#[allow(dead_code)]
pub fn check_permission(
    user: &AuthUser,
    _doc_id: &str,
    required: Permission,
) -> Result<(), StatusCode> {
    // Admins can do everything
    if user.role == UserRole::Admin {
        return Ok(());
    }

    // Anonymous users get Viewer only
    if user.role == UserRole::Anonymous {
        if required <= Permission::Viewer {
            return Ok(());
        }
        return Err(StatusCode::FORBIDDEN);
    }

    // Authenticated users: default to Editor
    // Per-document permissions are resolved via check_permission_with_session
    if required <= Permission::Editor {
        return Ok(());
    }

    Err(StatusCode::FORBIDDEN)
}

/// Check permission with session context (owner, mode).
///
/// Called from route handlers that have access to the file session info.
#[allow(dead_code)]
pub fn check_permission_with_session(
    user: &AuthUser,
    session_owner: Option<&str>,
    session_mode: &str,
    required: Permission,
) -> Result<(), StatusCode> {
    // Admins can do everything
    if user.role == UserRole::Admin {
        return Ok(());
    }

    // Anonymous users get Viewer only
    if user.role == UserRole::Anonymous {
        if required <= Permission::Viewer {
            return Ok(());
        }
        return Err(StatusCode::FORBIDDEN);
    }

    // Session owner gets full Owner permissions
    if let Some(owner_id) = session_owner {
        if user.user_id == owner_id {
            return Ok(()); // Owner can do anything
        }
    }

    // Non-owner: permission based on session mode
    let granted = match session_mode {
        "view" => Permission::Viewer,
        "comment" => Permission::Commenter,
        "edit" => Permission::Editor,
        _ => Permission::Editor, // default
    };

    if required <= granted {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_ordering() {
        assert!(Permission::Viewer < Permission::Commenter);
        assert!(Permission::Commenter < Permission::Editor);
        assert!(Permission::Editor < Permission::Admin);
        assert!(Permission::Admin < Permission::Owner);
    }

    #[test]
    fn check_anonymous_viewer() {
        let user = AuthUser {
            user_id: "anon".into(),
            name: "Anon".into(),
            tenant_id: None,
            role: UserRole::Anonymous,
        };
        assert!(check_permission(&user, "doc1", Permission::Viewer).is_ok());
        assert!(check_permission(&user, "doc1", Permission::Editor).is_err());
    }

    #[test]
    fn check_admin_full_access() {
        let user = AuthUser {
            user_id: "admin1".into(),
            name: "Admin".into(),
            tenant_id: None,
            role: UserRole::Admin,
        };
        assert!(check_permission(&user, "doc1", Permission::Owner).is_ok());
    }
}
