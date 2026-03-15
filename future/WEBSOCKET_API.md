# WebSocket Collaboration Protocol

## Overview

The WebSocket protocol enables real-time collaborative document editing. Clients connect to a room (associated with a document) and exchange CRDT operations and presence information.

## Connection

### Endpoint

```
wss://{host}/ws/collab/{room_id}?token={jwt_token}
```

### Handshake

1. Client opens WebSocket connection with JWT token as query parameter
2. Server validates token and extracts user identity + permissions
3. Server sends `connected` message with room state
4. Client sends `sync_request` to get current document state
5. Server sends `sync_response` with full CRDT state
6. Client is now ready to send/receive operations

```
Client                              Server
  │                                    │
  │──── WebSocket Connect ────────────▶│
  │     (with JWT token)               │
  │                                    │  ← validate token
  │◀──── connected ───────────────────│  ← send room info
  │                                    │
  │──── sync_request ─────────────────▶│
  │                                    │  ← prepare state
  │◀──── sync_response ──────────────│  ← full CRDT state
  │                                    │
  │◀──── peer_joined ────────────────│  ← notify about peers
  │                                    │
  │     (editing session active)       │
  │                                    │
```

## Message Format

All messages are JSON-encoded. Binary messages are used for CRDT state transfer (sync_response).

### Envelope

```json
{
  "type": "message_type",
  "seq": 42,
  "timestamp": 1700000000,
  "data": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Message type identifier |
| `seq` | integer | Sequence number (per-client, monotonically increasing) |
| `timestamp` | integer | Unix timestamp (seconds) |
| `data` | object | Type-specific payload |

---

## Server → Client Messages

### `connected`

Sent immediately after successful WebSocket handshake.

```json
{
  "type": "connected",
  "data": {
    "peer_id": "peer_abc123",
    "room_id": "room_x1y2z3",
    "document_id": "doc_7a8b9c0d",
    "user": {
      "id": "user-123",
      "name": "Alice",
      "color": "#4285f4"
    },
    "permissions": "editor",
    "peers": [
      {
        "peer_id": "peer_def456",
        "user": { "id": "user-456", "name": "Bob", "color": "#ea4335" },
        "connected_at": "2026-03-16T10:00:00Z"
      }
    ],
    "server_version": "1.0.0",
    "protocol_version": 1
  }
}
```

### `sync_response`

Response to client's `sync_request`. Contains full CRDT document state.

```json
{
  "type": "sync_response",
  "data": {
    "state_vector": { "0": 42, "1": 38 },
    "document_state": "<base64-encoded CRDT binary state>",
    "encoding": "base64",
    "version": 42
  }
}
```

For large documents, the state may be sent as a binary WebSocket frame instead of JSON.

### `remote_op`

A CRDT operation from another peer.

```json
{
  "type": "remote_op",
  "data": {
    "peer_id": "peer_def456",
    "operations": [
      {
        "op_id": { "lamport": 43, "replica": 1 },
        "action": "insert_text",
        "node_id": "0:15",
        "offset": 5,
        "text": "hello",
        "parent_op": { "lamport": 42, "replica": 1 }
      }
    ],
    "state_vector": { "0": 42, "1": 43 }
  }
}
```

### `awareness_update`

Presence information from another peer.

```json
{
  "type": "awareness_update",
  "data": {
    "peer_id": "peer_def456",
    "user": { "id": "user-456", "name": "Bob", "color": "#ea4335" },
    "cursor": {
      "node_id": "0:15",
      "offset": 10
    },
    "selection": {
      "anchor": { "node_id": "0:15", "offset": 5 },
      "focus": { "node_id": "0:15", "offset": 15 }
    },
    "status": "active"
  }
}
```

### `peer_joined`

A new peer connected to the room.

```json
{
  "type": "peer_joined",
  "data": {
    "peer_id": "peer_ghi789",
    "user": { "id": "user-789", "name": "Carol", "color": "#34a853" },
    "connected_at": "2026-03-16T10:05:00Z"
  }
}
```

### `peer_left`

A peer disconnected from the room.

```json
{
  "type": "peer_left",
  "data": {
    "peer_id": "peer_def456",
    "reason": "disconnect"
  }
}
```

### `error`

Server-side error.

```json
{
  "type": "error",
  "data": {
    "code": "PERMISSION_DENIED",
    "message": "You do not have write permission for this document",
    "fatal": false
  }
}
```

Fatal errors close the connection. Non-fatal errors are informational.

### `saved`

Confirmation that the document was auto-saved.

```json
{
  "type": "saved",
  "data": {
    "version": 43,
    "saved_at": "2026-03-16T10:05:00Z"
  }
}
```

### `pong`

Response to client's `ping`.

```json
{
  "type": "pong",
  "data": {
    "server_time": 1700000000
  }
}
```

---

## Client → Server Messages

### `sync_request`

Request full document state (sent on initial connection or after reconnect).

```json
{
  "type": "sync_request",
  "data": {
    "state_vector": {}
  }
}
```

If the client has partial state (reconnecting), include their state vector for incremental sync:

```json
{
  "type": "sync_request",
  "data": {
    "state_vector": { "0": 40, "1": 35 }
  }
}
```

### `local_op`

Send local CRDT operations to the server for broadcast.

```json
{
  "type": "local_op",
  "data": {
    "operations": [
      {
        "op_id": { "lamport": 44, "replica": 0 },
        "action": "insert_text",
        "node_id": "0:15",
        "offset": 10,
        "text": " world",
        "parent_op": { "lamport": 43, "replica": 0 }
      }
    ]
  }
}
```

### `awareness`

Send local cursor/selection state.

```json
{
  "type": "awareness",
  "data": {
    "cursor": {
      "node_id": "0:15",
      "offset": 16
    },
    "selection": null,
    "status": "active"
  }
}
```

### `ping`

Client heartbeat (recommended every 30 seconds).

```json
{
  "type": "ping",
  "data": {}
}
```

---

## Operation Types

The `action` field in operations maps to CRDT operation types:

| Action | Description | Fields |
|--------|-------------|--------|
| `insert_text` | Insert text at position | `node_id`, `offset`, `text` |
| `delete_text` | Delete text range | `node_id`, `offset`, `length` |
| `insert_node` | Insert new node | `parent_id`, `index`, `node_type`, `attributes` |
| `delete_node` | Delete a node | `node_id` |
| `move_node` | Move node to new parent | `node_id`, `new_parent_id`, `index` |
| `set_attributes` | Set formatting attributes | `node_id`, `attributes` |
| `split_node` | Split node at offset | `node_id`, `offset` |
| `merge_nodes` | Merge two adjacent nodes | `node_id_1`, `node_id_2` |
| `set_metadata` | Update document metadata | `key`, `value` |

---

## Connection Lifecycle

### Normal Session

```
1. Connect → connected
2. sync_request → sync_response
3. Local edits → local_op (repeated)
4. Remote edits ← remote_op (received)
5. Cursor moves → awareness (repeated)
6. Heartbeat → ping/pong (every 30s)
7. Auto-save ← saved (periodic)
8. Disconnect → peer_left (to others)
```

### Reconnection

```
1. WebSocket disconnects (network issue)
2. Client waits 1s, then reconnects (exponential backoff: 1s, 2s, 4s, 8s, max 30s)
3. Connect → connected
4. sync_request (with last known state_vector) → sync_response (incremental)
5. Apply missing ops
6. Resume normal session
```

### Conflict Resolution

All conflict resolution is handled by the CRDT layer:
- **Text conflicts**: Fugue CRDT provides deterministic ordering
- **Attribute conflicts**: Last-writer-wins per attribute key (by Lamport timestamp)
- **Structure conflicts**: Kleppmann tree CRDT with cycle detection
- **No manual conflict resolution needed** — CRDTs guarantee convergence

---

## Rate Limiting

| Message Type | Limit | Window |
|-------------|-------|--------|
| `local_op` | 100 messages | 1 second |
| `awareness` | 10 messages | 1 second |
| `ping` | 1 message | 10 seconds |
| `sync_request` | 1 message | 5 seconds |

Exceeding rate limits triggers a non-fatal `error` message with code `RATE_LIMITED`.

## Message Size Limits

| Message | Max Size |
|---------|----------|
| `local_op` | 64 KB |
| `awareness` | 1 KB |
| `sync_response` (server) | 16 MB |
| Total per second (per client) | 256 KB |

## Connection Limits

| Limit | Default | Configurable |
|-------|---------|-------------|
| Max peers per room | 50 | Yes |
| Max rooms per server | 1,000 | Yes |
| Connection timeout (idle) | 5 minutes | Yes |
| Reconnection grace period | 30 seconds | Yes |

During the grace period after disconnect, the peer's state is preserved. If they reconnect within the grace period, they resume seamlessly without a full sync.

---

## Binary Protocol (Future)

For performance optimization, a binary message format may be introduced in protocol v2:

```
Header (4 bytes):
  [0]    Message type (u8)
  [1]    Flags (u8)
  [2-3]  Payload length (u16, big-endian)

Payload:
  Variable length, type-specific binary encoding
```

This will be backward-compatible: clients negotiate protocol version during handshake.
