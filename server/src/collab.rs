//! Real-time collaboration via WebSocket.
//!
//! Each document gets a "room". Peers join the room and broadcast operations.
//! The server maintains authoritative CRDT state and persists periodically.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::routes::AppState;
use crate::storage::StorageBackend;

/// A collaboration room for a document.
struct Room {
    /// Broadcast channel for operations.
    tx: broadcast::Sender<String>,
    /// Connected peer count.
    peer_count: usize,
    /// Operation log for late-joiners and persistence.
    ops_log: Vec<String>,
    /// Document ID this room is for.
    #[allow(dead_code)]
    doc_id: String,
    /// Whether room state has been modified since last save.
    dirty: bool,
}

/// Manages all active collaboration rooms.
pub struct RoomManager {
    rooms: Mutex<HashMap<String, Room>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create a room. Returns (broadcast sender, ops log for catch-up).
    async fn join(&self, room_id: &str) -> (broadcast::Sender<String>, Vec<String>) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            room.peer_count += 1;
            let catch_up = room.ops_log.clone();
            (room.tx.clone(), catch_up)
        } else {
            let (tx, _) = broadcast::channel(512);
            let room = Room {
                tx: tx.clone(),
                peer_count: 1,
                ops_log: Vec::new(),
                doc_id: room_id.to_string(),
                dirty: false,
            };
            rooms.insert(room_id.to_string(), room);
            tracing::info!("Room created: {}", room_id);
            (tx, Vec::new())
        }
    }

    /// Record an operation in the room's log.
    async fn record_op(&self, room_id: &str, op: &str) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            room.ops_log.push(op.to_string());
            room.dirty = true;
            // Cap log size to prevent unbounded growth
            if room.ops_log.len() > 10_000 {
                room.ops_log.drain(..5_000);
            }
        }
    }

    /// Validate an operation message (basic JSON structure check).
    fn validate_op(msg: &str) -> bool {
        // Must be valid JSON with a "type" field
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
            val.get("type").is_some()
        } else {
            false
        }
    }

    /// Remove a peer from a room. Returns true if room was closed.
    async fn leave(&self, room_id: &str) -> bool {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            room.peer_count = room.peer_count.saturating_sub(1);
            if room.peer_count == 0 {
                rooms.remove(room_id);
                tracing::info!("Room closed: {}", room_id);
                return true;
            }
        }
        false
    }

    /// Save dirty rooms to storage.
    pub async fn save_dirty_rooms(&self, storage: &dyn StorageBackend) {
        let mut rooms = self.rooms.lock().await;
        for (room_id, room) in rooms.iter_mut() {
            if room.dirty && !room.ops_log.is_empty() {
                // Serialize ops log as JSON and save as a sidecar file
                let ops_json = serde_json::to_string(&room.ops_log).unwrap_or_default();
                let meta = crate::storage::DocumentMeta {
                    id: format!("{}_ops", room_id),
                    filename: format!("{}_ops.json", room_id),
                    format: "json".to_string(),
                    size: ops_json.len(),
                    title: None,
                    author: None,
                    word_count: 0,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(e) = storage.put(&meta.id, ops_json.as_bytes(), &meta) {
                    tracing::warn!("Failed to save room {} ops: {}", room_id, e);
                } else {
                    room.dirty = false;
                    tracing::debug!("Saved {} ops for room {}", room.ops_log.len(), room_id);
                }
            }
        }
    }

    /// Load room state from storage on restart.
    #[allow(dead_code)]
    pub async fn recover_room(&self, room_id: &str, storage: &dyn StorageBackend) {
        let ops_id = format!("{}_ops", room_id);
        if let Ok(data) = storage.get(&ops_id) {
            if let Ok(ops) = serde_json::from_slice::<Vec<String>>(&data) {
                let mut rooms = self.rooms.lock().await;
                if let Some(room) = rooms.get_mut(room_id) {
                    room.ops_log = ops;
                    tracing::info!("Recovered {} ops for room {}", room.ops_log.len(), room_id);
                }
            }
        }
    }

    /// Get active room count.
    #[allow(dead_code)]
    pub async fn room_count(&self) -> usize {
        self.rooms.lock().await.len()
    }
}

/// WebSocket upgrade handler for collaboration.
pub async fn ws_collab_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_collab_socket(socket, room_id, state))
}

/// Handle a single WebSocket connection.
async fn handle_collab_socket(socket: WebSocket, room_id: String, state: Arc<AppState>) {
    let (tx, catch_up_ops) = state.rooms.join(&room_id).await;
    let mut rx = tx.subscribe();

    let (mut sender, mut receiver) = socket.split();

    // Send welcome + catch-up operations
    let welcome = serde_json::json!({
        "type": "welcome",
        "roomId": room_id,
        "opsCount": catch_up_ops.len(),
    });
    let _ = sender.send(Message::Text(welcome.to_string().into())).await;

    // Send catch-up ops for late joiners
    for op in &catch_up_ops {
        let catch_up = serde_json::json!({
            "type": "catchUp",
            "op": serde_json::from_str::<serde_json::Value>(op).unwrap_or_default(),
        });
        if sender.send(Message::Text(catch_up.to_string().into())).await.is_err() {
            state.rooms.leave(&room_id).await;
            return;
        }
    }

    // Spawn broadcast → peer task
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Receive from peer → validate → broadcast + record
    let rooms = state.rooms.clone();
    let tx_clone = tx.clone();
    let room_id_recv = room_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let text_str = text.to_string();
                    // P4-07: Validate operation structure
                    if RoomManager::validate_op(&text_str) {
                        let _ = tx_clone.send(text_str.clone());
                        rooms.record_op(&room_id_recv, &text_str).await;
                    } else {
                        tracing::debug!("Invalid op rejected in room {}", room_id_recv);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    state.rooms.leave(&room_id).await;
}
