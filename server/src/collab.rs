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

/// Maximum allowed WebSocket message size (256 KB).
const MAX_WS_MESSAGE_SIZE: usize = 256 * 1024;

/// Maximum messages per second per client before rate-limiting kicks in.
const MAX_MESSAGES_PER_SECOND: u32 = 100;

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
    /// Each entry is (server_version, op_json, timestamp).
    ops_log: Vec<(u64, String, std::time::Instant)>,
    /// Monotonically increasing version counter.
    version: u64,
    #[allow(dead_code)]
    doc_id: String,
    dirty: bool,
}

/// Per-client rate-limit state: message count in the current 1-second window.
struct RateWindow {
    count: u32,
    window_start: std::time::Instant,
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

    /// Join a room. Returns (broadcast sender, catch-up ops with versions, current peer list).
    async fn join(
        &self,
        room_id: &str,
        peer: PeerInfo,
    ) -> (broadcast::Sender<String>, Vec<(u64, String)>, Vec<PeerInfo>) {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            let existing_peers = room.peers.clone();
            room.peers.push(peer);
            let catch_up: Vec<(u64, String)> = room.ops_log.iter().map(|(v, s, _)| (*v, s.clone())).collect();
            (room.tx.clone(), catch_up, existing_peers)
        } else {
            let (tx, _) = broadcast::channel(512);
            let room = Room {
                tx: tx.clone(),
                peers: vec![peer],
                ops_log: Vec::new(),
                version: 0,
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

    /// Record an op with an incremented server version. Returns the new version.
    ///
    /// Eviction policy:
    /// 1. Time-based: entries older than 1 hour are evicted.
    /// 2. Count-based: if the log exceeds 10,000 entries, the oldest 5,000 are removed.
    async fn record_op(&self, room_id: &str, op: &str) -> u64 {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            room.version += 1;
            let version = room.version;
            let now = std::time::Instant::now();
            room.ops_log.push((version, op.to_string(), now));
            room.dirty = true;

            // Time-based eviction: remove entries older than 1 hour
            let one_hour = std::time::Duration::from_secs(3600);
            if let Some((_, _, oldest_ts)) = room.ops_log.first() {
                if oldest_ts.elapsed() > one_hour {
                    let cutoff = now - one_hour;
                    let evict_count = room.ops_log.partition_point(|(_, _, ts)| *ts < cutoff);
                    if evict_count > 0 {
                        tracing::debug!(
                            "Room {} evicting {} time-expired ops (>1h old)",
                            room_id,
                            evict_count
                        );
                        room.ops_log.drain(..evict_count);
                    }
                }
            }

            // Count-based cap (existing logic)
            if room.ops_log.len() > 10000 {
                tracing::warn!(
                    "Room {} ops_log at {} entries — truncating oldest 5000. Late joiners may miss history.",
                    room_id,
                    room.ops_log.len()
                );
                room.ops_log.drain(..5_000);
            }
            version
        } else {
            0
        }
    }

    /// Validate an incoming WebSocket message.
    ///
    /// Rejects messages that are too large, fail to parse as JSON, or
    /// contain an unrecognised `type` / `action` field.
    fn validate_op(msg: &str) -> bool {
        if msg.len() > MAX_WS_MESSAGE_SIZE {
            return false;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
            if let Some(t) = val.get("type").and_then(|v| v.as_str()) {
                return matches!(
                    t,
                    "op" | "join" | "awareness" | "awareness-batch" | "sync"
                        | "fullSync" | "requestFullSync" | "requestCatchup"
                );
            }
            if let Some(a) = val.get("action").and_then(|v| v.as_str()) {
                return matches!(
                    a,
                    "op" | "insert" | "delete" | "format" | "structural"
                        | "ssSetCell" | "ssFormat" | "ssSync" | "ssCursor"
                        | "peer-join" | "peer-leave"
                );
            }
        }
        false
    }

    /// Get the current server version for a room.
    async fn get_version(&self, room_id: &str) -> u64 {
        let rooms = self.rooms.lock().await;
        rooms.get(room_id).map_or(0, |room| room.version)
    }

    /// Get all ops recorded after `from_version`.
    async fn get_ops_since(&self, room_id: &str, from_version: u64) -> Vec<(u64, String)> {
        let rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get(room_id) {
            room.ops_log
                .iter()
                .filter(|(v, _, _)| *v > from_version)
                .map(|(v, s, _)| (*v, s.clone()))
                .collect()
        } else {
            Vec::new()
        }
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
                // Serialize only the op strings (strip version tuples) for storage compatibility
                let ops_only: Vec<&str> = room.ops_log.iter().map(|(_, op, _)| op.as_str()).collect();
                let ops_json = serde_json::to_string(&ops_only).unwrap_or_default();
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

    /// Get version and op count for a room.
    pub async fn get_room_version_info(&self, room_id: &str) -> Option<(u64, usize)> {
        let rooms = self.rooms.lock().await;
        rooms
            .get(room_id)
            .map(|room| (room.version, room.ops_log.len()))
    }

    /// Get the peer list for a room.
    #[allow(dead_code)]
    pub async fn get_room_peers(&self, room_id: &str) -> Vec<(String, String, String)> {
        let rooms = self.rooms.lock().await;
        rooms
            .get(room_id)
            .map(|room| {
                room.peers
                    .iter()
                    .map(|p| (p.peer_id.clone(), p.user_name.clone(), p.user_color.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Broadcast a fullSync request to all peers in a room.
    /// Returns true if the room exists and the message was sent.
    pub async fn broadcast_sync(&self, room_id: &str) -> bool {
        let rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get(room_id) {
            let msg = serde_json::json!({
                "type": "requestFullSync",
                "peerId": "admin",
                "_sender": "admin",
            });
            let _ = room.tx.send(msg.to_string());
            true
        } else {
            false
        }
    }

    /// Broadcast a peer-leave message for a specific peer.
    /// Returns true if the room exists.
    pub async fn broadcast_peer_leave(&self, room_id: &str, peer_id: &str) -> bool {
        let mut rooms = self.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_id) {
            // Remove the peer from the room
            room.peers.retain(|p| p.peer_id != peer_id);
            let msg = serde_json::json!({
                "type": "peer-leave",
                "peerId": peer_id,
                "_sender": "admin",
            });
            let _ = room.tx.send(msg.to_string());
            if room.peers.is_empty() {
                rooms.remove(room_id);
                tracing::info!("Room closed (admin kick): {}", room_id);
            }
            true
        } else {
            false
        }
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

    // Get the current room version to include in the joined message
    let current_room_version = state.rooms.get_version(&file_id).await;

    let joined_msg = serde_json::json!({
        "type": "joined",
        "peerId": peer_id,
        "room": file_id,
        "peers": peer_list,
        "access": params.access,
        "serverVersion": current_room_version,
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

    // 4. Send catch-up ops (with server version)
    for (version, op) in &catch_up_ops {
        let msg = serde_json::json!({
            "type": "op",
            "peerId": "server",
            "data": op,
            "serverVersion": version,
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
        // Per-client rate limiting state
        let mut rate = RateWindow {
            count: 0,
            window_start: std::time::Instant::now(),
        };

        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let text_str = text.to_string();

                    // Reject oversized messages before parsing
                    if text_str.len() > MAX_WS_MESSAGE_SIZE {
                        tracing::warn!(
                            "Dropping oversized WS message ({} bytes) from {}",
                            text_str.len(),
                            sender_peer_id
                        );
                        continue;
                    }

                    // Per-client rate limiting: max MAX_MESSAGES_PER_SECOND msgs/sec
                    let now = std::time::Instant::now();
                    if now.duration_since(rate.window_start).as_secs() >= 1 {
                        rate.count = 0;
                        rate.window_start = now;
                    }
                    rate.count += 1;
                    if rate.count > MAX_MESSAGES_PER_SECOND {
                        tracing::warn!(
                            "Rate limit exceeded for peer {} ({} msgs/sec)",
                            sender_peer_id,
                            rate.count
                        );
                        continue;
                    }

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
                        "requestCatchup" => {
                            // Client is behind — send all ops since their version
                            if let Some(from_version) =
                                parsed.get("fromVersion").and_then(|v| v.as_u64())
                            {
                                let ops = rooms.get_ops_since(&file_id_recv, from_version).await;
                                for (version, op_data) in ops {
                                    let catchup = serde_json::json!({
                                        "type": "op",
                                        "peerId": "server",
                                        "data": op_data,
                                        "serverVersion": version,
                                    });
                                    let _ = tx_clone.send(catchup.to_string());
                                }
                            }
                            continue;
                        }
                        "op" => {
                            // Forward op with sender's peerId attached
                            let data = parsed.get("data").cloned().unwrap_or_default();

                            // Record the raw op data in the ops log and get the new version
                            let mut room_version: u64 = 0;
                            if let Some(data_str) = data.as_str() {
                                room_version = rooms.record_op(&file_id_recv, data_str).await;

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

                            let forwarded = serde_json::json!({
                                "type": "op",
                                "peerId": sender_peer_id,
                                "data": data,
                                "serverVersion": room_version,
                                "_sender": sender_peer_id,
                            });
                            let _ = tx_clone.send(forwarded.to_string());
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
