//! File editing sessions — temporary document storage with TTL.
#![allow(dead_code, unused_imports)]
//!
//! Manages the lifecycle of documents being actively edited:
//! 1. Upload or fetch from external URL → create session with fileId
//! 2. Editors connect via WebSocket → session stays alive
//! 3. All editors leave → session enters grace period (configurable TTL)
//! 4. After TTL → session cleaned up, file removed
//!
//! The server maintains the authoritative document state:
//! - Latest snapshot (DOCX bytes) updated every 30s or on explicit save
//! - New editors joining get the snapshot, not peer-to-peer sync

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Default TTL after last editor disconnects (5 minutes).
const DEFAULT_SESSION_TTL_SECS: u64 = 300;

/// How often to snapshot the document (30 seconds).
const SNAPSHOT_INTERVAL_SECS: u64 = 30;

/// A file editing session.
#[derive(Debug)]
pub struct FileSession {
    /// Unique file ID.
    pub file_id: String,
    /// Original filename.
    pub filename: String,
    /// Current document bytes (latest snapshot).
    pub data: Vec<u8>,
    /// Format of the document.
    pub format: String,
    /// Number of active editors.
    pub editor_count: usize,
    /// When the session was created.
    pub created_at: Instant,
    /// When the last editor disconnected (None if editors are active).
    pub last_editor_left: Option<Instant>,
    /// Session TTL after last editor leaves.
    pub ttl: Duration,
    /// Integration callback URL (for notifying host product).
    pub callback_url: Option<String>,
    /// Who uploaded / owns this session.
    pub owner_id: Option<String>,
    /// Editing mode: "edit", "view", "comment".
    pub mode: String,
    /// Connected editor user IDs.
    pub editors: Vec<EditorInfo>,
}

/// Info about a connected editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorInfo {
    pub user_id: String,
    pub user_name: String,
    pub connected_at: String,
    pub mode: String,
}

/// Public session info (returned by API).
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub file_id: String,
    pub filename: String,
    pub format: String,
    pub size: usize,
    pub editor_count: usize,
    pub editors: Vec<EditorInfo>,
    pub mode: String,
    pub created_at_secs_ago: u64,
    pub status: String, // "editing", "idle", "expired"
}

/// Manages all active file sessions.
pub struct FileSessionManager {
    sessions: Mutex<HashMap<String, FileSession>>,
    ttl: Duration,
}

impl FileSessionManager {
    pub fn new(ttl_secs: Option<u64>) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs.unwrap_or(DEFAULT_SESSION_TTL_SECS)),
        }
    }

    /// Create a new file session from uploaded bytes.
    pub async fn create(
        &self,
        file_id: String,
        filename: String,
        data: Vec<u8>,
        format: String,
        owner_id: Option<String>,
        callback_url: Option<String>,
    ) -> String {
        let session = FileSession {
            file_id: file_id.clone(),
            filename,
            data,
            format,
            editor_count: 0,
            created_at: Instant::now(),
            last_editor_left: None,
            ttl: self.ttl,
            callback_url,
            owner_id,
            mode: "edit".to_string(),
            editors: Vec::new(),
        };
        self.sessions.lock().await.insert(file_id.clone(), session);
        tracing::info!("File session created: {}", file_id);
        file_id
    }

    /// Get the latest document bytes for a file.
    pub async fn get_data(&self, file_id: &str) -> Option<Vec<u8>> {
        self.sessions
            .lock()
            .await
            .get(file_id)
            .map(|s| s.data.clone())
    }

    /// Update the document snapshot.
    pub async fn update_snapshot(&self, file_id: &str, data: Vec<u8>) {
        if let Some(session) = self.sessions.lock().await.get_mut(file_id) {
            session.data = data;
        }
    }

    /// Editor joins a session.
    pub async fn editor_join(
        &self,
        file_id: &str,
        user_id: &str,
        user_name: &str,
        mode: &str,
    ) -> bool {
        if let Some(session) = self.sessions.lock().await.get_mut(file_id) {
            session.editor_count += 1;
            session.last_editor_left = None;
            session.editors.push(EditorInfo {
                user_id: user_id.to_string(),
                user_name: user_name.to_string(),
                connected_at: chrono::Utc::now().to_rfc3339(),
                mode: mode.to_string(),
            });
            tracing::info!(
                "Editor joined {}: {} ({} total)",
                file_id,
                user_name,
                session.editor_count
            );
            true
        } else {
            false
        }
    }

    /// Editor leaves a session.
    pub async fn editor_leave(&self, file_id: &str, user_id: &str) {
        if let Some(session) = self.sessions.lock().await.get_mut(file_id) {
            session.editor_count = session.editor_count.saturating_sub(1);
            session.editors.retain(|e| e.user_id != user_id);
            if session.editor_count == 0 {
                session.last_editor_left = Some(Instant::now());
                tracing::info!("All editors left {}, TTL started", file_id);
            }
        }
    }

    /// Get session info.
    pub async fn get_info(&self, file_id: &str) -> Option<SessionInfo> {
        self.sessions.lock().await.get(file_id).map(|s| {
            let status = if s.editor_count > 0 {
                "editing"
            } else if s.last_editor_left.is_some() {
                "idle"
            } else {
                "expired"
            };
            SessionInfo {
                file_id: s.file_id.clone(),
                filename: s.filename.clone(),
                format: s.format.clone(),
                size: s.data.len(),
                editor_count: s.editor_count,
                editors: s.editors.clone(),
                mode: s.mode.clone(),
                created_at_secs_ago: s.created_at.elapsed().as_secs(),
                status: status.to_string(),
            }
        })
    }

    /// Check if a session exists.
    pub async fn exists(&self, file_id: &str) -> bool {
        self.sessions.lock().await.contains_key(file_id)
    }

    /// Get the callback URL for a session (for integration mode).
    pub async fn get_callback_url(&self, file_id: &str) -> Option<String> {
        self.sessions
            .lock()
            .await
            .get(file_id)
            .and_then(|s| s.callback_url.clone())
    }

    /// Clean up expired sessions (call periodically).
    /// Returns: Vec of (file_id, callback_url, final_data_bytes).
    pub async fn cleanup_expired(&self) -> Vec<(String, Option<String>, Vec<u8>)> {
        let mut expired = Vec::new();
        let mut sessions = self.sessions.lock().await;
        let to_remove: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| {
                if let Some(left_at) = s.last_editor_left {
                    left_at.elapsed() > s.ttl
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            if let Some(session) = sessions.remove(id) {
                expired.push((id.clone(), session.callback_url, session.data));
                tracing::info!("File session expired and cleaned up: {}", id);
            }
        }

        expired
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .map(|s| {
                let status = if s.editor_count > 0 {
                    "editing"
                } else {
                    "idle"
                };
                SessionInfo {
                    file_id: s.file_id.clone(),
                    filename: s.filename.clone(),
                    format: s.format.clone(),
                    size: s.data.len(),
                    editor_count: s.editor_count,
                    editors: s.editors.clone(),
                    mode: s.mode.clone(),
                    created_at_secs_ago: s.created_at.elapsed().as_secs(),
                    status: status.to_string(),
                }
            })
            .collect()
    }

    /// Force close a session (admin action).
    pub async fn force_close(&self, file_id: &str) -> Option<Vec<u8>> {
        self.sessions.lock().await.remove(file_id).map(|s| {
            tracing::info!("File session force-closed: {}", file_id);
            s.data
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_get() {
        let mgr = FileSessionManager::new(Some(60));
        mgr.create(
            "f1".into(),
            "test.docx".into(),
            b"hello".to_vec(),
            "docx".into(),
            None,
            None,
        )
        .await;
        assert!(mgr.exists("f1").await);
        assert_eq!(mgr.get_data("f1").await.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn editor_join_leave() {
        let mgr = FileSessionManager::new(Some(1));
        mgr.create(
            "f2".into(),
            "test.txt".into(),
            vec![],
            "txt".into(),
            None,
            None,
        )
        .await;
        mgr.editor_join("f2", "u1", "Alice", "edit").await;
        let info = mgr.get_info("f2").await.unwrap();
        assert_eq!(info.editor_count, 1);
        assert_eq!(info.status, "editing");

        mgr.editor_leave("f2", "u1").await;
        let info = mgr.get_info("f2").await.unwrap();
        assert_eq!(info.editor_count, 0);
        assert_eq!(info.status, "idle");
    }

    #[tokio::test]
    async fn snapshot_update() {
        let mgr = FileSessionManager::new(None);
        mgr.create(
            "f3".into(),
            "doc.docx".into(),
            b"v1".to_vec(),
            "docx".into(),
            None,
            None,
        )
        .await;
        mgr.update_snapshot("f3", b"v2".to_vec()).await;
        assert_eq!(mgr.get_data("f3").await.unwrap(), b"v2");
    }
}
