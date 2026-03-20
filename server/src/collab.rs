//! Real-time collaborative editing via WebSocket.
//!
//! Each file being edited gets a room keyed by `file_id`.
//! The server maintains authoritative document state via file sessions.
//! New editors receive the latest snapshot on connect.
//!
//! Protocol messages sent to clients:
//!   - `joined`     — sent to the connecting peer with peerId + peer list
//!   - `peer-join`  — broadcast when a new peer joins
//!   - `peer-leave` — broadcast when a peer leaves
//!   - `snapshot`   — latest document bytes (base64) sent to new joiners
//!   - `op`         — forwarded edit operation with sender's peerId
//!   - `awareness`  — forwarded cursor/selection data with sender's peerId

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::routes::AppState;
use crate::storage::StorageBackend;

/// Info about a connected peer in a room.
#[derive(Debug, Clone)]
struct PeerInfo {
    peer_id: String,
    user_name: String,
    user_color: String,
}

/// A collaboration room for a document.
struct Room {
    tx: broadcast::Sender<String>,
    peers: Vec<PeerInfo>,
    ops_log: Vec<String>,
    #[allow(dead_code)]
    doc_id: String,
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

    /// Join a room. Returns (broadcast sender, catch-up ops, current peer list).
    async fn join(
        &self,
        room_id: &str,
        peer: PeerInfo,
    ) -> (broadcast::Sender<String>, Vec<String>, Vec<PeerInfo>) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            let existing_peers = room.peers.clone();
            room.peers.push(peer);
            let catch_up = room.ops_log.clone();
            (room.tx.clone(), catch_up, existing_peers)
        } else {
            let (tx, _) = broadcast::channel(512);
            let room = Room {
                tx: tx.clone(),
                peers: vec![peer],
                ops_log: Vec::new(),
                doc_id: room_id.to_string(),
                dirty: false,
            };
            rooms.insert(room_id.to_string(), room);
            tracing::info!("Room created: {}", room_id);
            (tx, Vec::new(), Vec::new())
        }
    }

    async fn update_peer_color(&self, room_id: &str, peer_id: &str, color: &str) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            if let Some(peer) = room.peers.iter_mut().find(|p| p.peer_id == peer_id) {
                peer.user_color = color.to_string();
            }
        }
    }

    async fn record_op(&self, room_id: &str, op: &str) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            room.ops_log.push(op.to_string());
            room.dirty = true;
            if room.ops_log.len() > 10000 {
                tracing::warn!(
                    "Room {} ops_log at {} entries — truncating oldest 5000. Late joiners may miss history.",
                    room_id,
                    room.ops_log.len()
                );
                room.ops_log.drain(..5_000);
            }
        }
    }

    fn validate_op(msg: &str) -> bool {
        serde_json::from_str::<serde_json::Value>(msg)
            .map(|v| v.get("type").is_some() || v.get("action").is_some())
            .unwrap_or(false)
    }

    async fn leave(&self, room_id: &str, peer_id: &str) -> bool {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            room.peers.retain(|p| p.peer_id != peer_id);
            if room.peers.is_empty() {
                rooms.remove(room_id);
                tracing::info!("Room closed: {}", room_id);
                return true;
            }
        }
        false
    }

    pub async fn save_dirty_rooms(&self, storage: &dyn StorageBackend) {
        let mut rooms = self.rooms.lock().await;
        for (room_id, room) in rooms.iter_mut() {
            if room.dirty && !room.ops_log.is_empty() {
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
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn room_count(&self) -> usize {
        self.rooms.lock().await.len()
    }
}

/// Query params for WebSocket connection.
#[derive(Debug, Deserialize)]
pub struct WsParams {
    /// User name for presence display.
    #[serde(default = "default_user_name")]
    pub user: String,
    /// User ID for session tracking.
    #[serde(default = "default_user_id")]
    pub uid: String,
    /// Editing mode.
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Access level: "view" or "edit". Controls whether the peer can send structural ops.
    #[serde(default = "default_mode")]
    pub access: String,
}

fn default_user_name() -> String {
    format!("User-{}", rand_id())
}
fn default_user_id() -> String {
    rand_id()
}
fn default_mode() -> String {
    "edit".to_string()
}
fn rand_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

/// WebSocket upgrade handler.
///
/// Route: `GET /ws/edit/{file_id}?user=Alice&uid=u123&mode=edit`
pub async fn ws_collab_handler(
    ws: WebSocketUpgrade,
    Path(file_id): Path<String>,
    Query(params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, file_id, params, state))
}

/// Handle a WebSocket connection for collaborative editing.
async fn handle_socket(socket: WebSocket, file_id: String, params: WsParams, state: Arc<AppState>) {
    let has_session = state.sessions.exists(&file_id).await;
    let peer_id = params.uid.clone();
    let user_name = params.user.clone();

    if params.access == "view" {
        tracing::info!(
            "Viewer {} connected to {} (read-only)",
            params.user,
            file_id
        );
    }

    // Build peer info
    let peer_info = PeerInfo {
        peer_id: peer_id.clone(),
        user_name: user_name.clone(),
        user_color: String::new(), // Will be set from client's join message
    };

    // Join the room
    let (tx, catch_up_ops, existing_peers) = state.rooms.join(&file_id, peer_info).await;
    let mut rx = tx.subscribe();

    // Track editor in file session
    if has_session {
        state
            .sessions
            .editor_join(&file_id, &peer_id, &user_name, &params.mode)
            .await;
    }

    let (mut sender, mut receiver) = socket.split();

    // 1. Send "joined" to this peer with their peerId and current peer list
    let peer_list: Vec<serde_json::Value> = existing_peers
        .iter()
        .map(|p| {
            serde_json::json!({
                "peerId": p.peer_id,
                "userName": p.user_name,
                "userColor": p.user_color,
            })
        })
        .collect();

    let joined_msg = serde_json::json!({
        "type": "joined",
        "peerId": peer_id,
        "room": file_id,
        "peers": peer_list,
        "access": params.access,
    });
    let _ = sender
        .send(Message::Text(joined_msg.to_string().into()))
        .await;

    // 2. Broadcast "peer-join" to all existing peers (with _sender so echo is filtered)
    let peer_join_msg = serde_json::json!({
        "type": "peer-join",
        "peerId": peer_id,
        "userName": user_name,
        "userColor": "",
        "_sender": peer_id,
    });
    let _ = tx.send(peer_join_msg.to_string());

    // 3. Send document snapshot if session has data
    if has_session {
        if let Some(data) = state.sessions.get_data(&file_id).await {
            use base64::Engine as _;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let snapshot = serde_json::json!({
                "type": "snapshot",
                "data": b64,
                "size": data.len(),
            });
            let _ = sender
                .send(Message::Text(snapshot.to_string().into()))
                .await;
        }
    }

    // 4. Send catch-up ops
    for op in &catch_up_ops {
        let msg = serde_json::json!({
            "type": "op",
            "peerId": "server",
            "data": op,
        });
        if sender
            .send(Message::Text(msg.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
    }

    // Broadcast → this peer (filter out own messages) + periodic ping
    let my_peer_id = peer_id.clone();
    let mut send_task = tokio::spawn(async move {
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            // Don't echo messages back to the sender
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                                if parsed.get("_sender").and_then(|s| s.as_str()) == Some(my_peer_id.as_str()) {
                                    continue;
                                }
                            }
                            if sender.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                _ = ping_interval.tick() => {
                    // Send ping to detect dead connections
                    if sender.send(Message::Ping(vec![].into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // This peer → parse → wrap with peerId → broadcast + record + snapshot update
    let rooms = state.rooms.clone();
    let sessions_for_recv = state.sessions.clone();
    let tx_clone = tx.clone();
    let file_id_recv = file_id.clone();
    let sender_peer_id = peer_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let text_str = text.to_string();
                    if !RoomManager::validate_op(&text_str) {
                        continue;
                    }

                    // Update last-activity timestamp for this editor
                    sessions_for_recv
                        .update_activity(&file_id_recv, &sender_peer_id)
                        .await;

                    // Parse the incoming message to determine its type
                    let parsed: serde_json::Value = match serde_json::from_str(&text_str) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let msg_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    match msg_type {
                        "join" => {
                            // Extract peer color from join message and update room state
                            if let Some(color) = parsed.get("userColor").and_then(|c| c.as_str()) {
                                rooms
                                    .update_peer_color(&file_id_recv, &sender_peer_id, color)
                                    .await;
                            }
                            continue;
                        }
                        "op" => {
                            // Forward op with sender's peerId attached
                            let data = parsed.get("data").cloned().unwrap_or_default();
                            let forwarded = serde_json::json!({
                                "type": "op",
                                "peerId": sender_peer_id,
                                "data": data,
                                "_sender": sender_peer_id,
                            });
                            let forwarded_str = forwarded.to_string();

                            // Record the raw op data in the ops log (not the envelope)
                            if let Some(data_str) = data.as_str() {
                                rooms.record_op(&file_id_recv, data_str).await;

                                // Check for fullSync to update session snapshot
                                if let Ok(inner) =
                                    serde_json::from_str::<serde_json::Value>(data_str)
                                {
                                    if inner.get("action").and_then(|a| a.as_str())
                                        == Some("fullSync")
                                    {
                                        if let Some(b64) =
                                            inner.get("docBase64").and_then(|d| d.as_str())
                                        {
                                            use base64::Engine as _;
                                            if let Ok(bytes) =
                                                base64::engine::general_purpose::STANDARD
                                                    .decode(b64)
                                            {
                                                sessions_for_recv
                                                    .update_snapshot(&file_id_recv, bytes)
                                                    .await;
                                                tracing::debug!(
                                                    "Updated snapshot for {} from fullSync",
                                                    file_id_recv
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            let _ = tx_clone.send(forwarded_str);
                        }
                        "awareness" => {
                            // Forward awareness with sender's peerId
                            let data = parsed.get("data").cloned().unwrap_or_default();
                            let forwarded = serde_json::json!({
                                "type": "awareness",
                                "peerId": sender_peer_id,
                                "data": data,
                                "_sender": sender_peer_id,
                            });
                            let _ = tx_clone.send(forwarded.to_string());
                        }
                        _ => {
                            // Unknown type — still broadcast for extensibility
                            let _ = tx_clone.send(text_str);
                        }
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

    // Cleanup: broadcast peer-leave, remove from session, leave room
    let leave_msg = serde_json::json!({
        "type": "peer-leave",
        "peerId": peer_id,
        "_sender": peer_id,
    });
    let _ = tx.send(leave_msg.to_string());

    if has_session {
        state.sessions.editor_leave(&file_id, &peer_id).await;
    }
    let room_closed = state.rooms.leave(&file_id, &peer_id).await;

    tracing::info!(
        "Editor {} ({}) disconnected from {} (room {})",
        user_name,
        peer_id,
        file_id,
        if room_closed {
            "closed"
        } else {
            "still active"
        }
    );
}
