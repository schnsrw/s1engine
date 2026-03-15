# Collaboration Architecture

## Overview

Real-time collaboration allows multiple users to edit the same document simultaneously. s1engine uses CRDTs (Conflict-free Replicated Data Types) to ensure all clients converge to the same state without requiring a central authority to resolve conflicts.

## Current State

The existing collaboration stack:

```
Client A (WASM)  ←→  Node.js Relay (scripts/relay.js)  ←→  Client B (WASM)
                     (message passthrough, no CRDT logic)
```

- **s1-crdt** crate: Fugue text CRDT + Kleppmann tree CRDT + LWW attributes
- **relay.js**: Simple WebSocket message relay, no server-side CRDT
- **collab.js**: Client-side collaboration logic

## Target State

```
Client A (WASM)  ←→  s1-server (Rust)  ←→  Client B (WASM)
                     ┌─────────────┐
                     │ Room Manager│
                     │  ┌────────┐ │
                     │  │ CRDT   │ │   Server maintains authoritative
                     │  │ State  │ │   CRDT state for validation
                     │  └────────┘ │
                     │  ┌────────┐ │
                     │  │Storage │ │   Auto-save to persistent storage
                     │  │Backend │ │
                     │  └────────┘ │
                     └─────────────┘
```

---

## CRDT Architecture

### CRDT Components (s1-crdt crate — existing)

| Component | Algorithm | Purpose |
|-----------|-----------|---------|
| Text CRDT | Fugue (with YATA ordering) | Character-level concurrent text editing |
| Tree CRDT | Kleppmann tree moves | Node insert, delete, move with cycle detection |
| Attribute CRDT | LWW register (per key) | Formatting (bold, font-size, etc.) |
| Metadata CRDT | LWW register (per field) | Document metadata (title, author) |
| Style CRDT | LWW register (per style) | Style definitions |

### How CRDTs Work Together

```
Document Edit                    CRDT Operation
─────────────                    ──────────────
Type "hello" at offset 5    →    TextCrdt::insert(pos=5, chars="hello")
Delete paragraph            →    TreeCrdt::delete(node_id)
Bold selection              →    AttrCrdt::set(node_id, "bold", true, lamport=42)
Move paragraph up           →    TreeCrdt::move(node_id, new_parent, new_index)
Change title                →    MetadataCrdt::set("title", "New Title", lamport=43)
```

### Conflict Resolution Rules

| Conflict | Resolution | Example |
|----------|-----------|---------|
| Concurrent text inserts at same position | Deterministic ordering by (lamport, replica_id) | Alice types "A", Bob types "B" at same spot → consistent order |
| Concurrent delete + edit on same node | Delete wins (tombstone preserved) | Alice deletes paragraph, Bob edits it → paragraph deleted |
| Concurrent attribute changes on same key | Last-writer-wins (by Lamport timestamp) | Alice bolds, Bob unbolds → higher Lamport wins |
| Concurrent node moves creating cycle | Cycle detection, one move rejected | Both users move nodes into each other → one move wins |
| Concurrent node moves to same position | Deterministic ordering by replica_id | Both users move different nodes to same index → ordered by replica |

### Causal Ordering

Operations are applied in causal order using Lamport clocks and state vectors:

```
State Vector: { replica_0: 42, replica_1: 38, replica_2: 15 }

Meaning: "I have seen all operations up to counter 42 from replica 0,
          up to counter 38 from replica 1, and up to counter 15 from replica 2"
```

When receiving an operation, a client checks:
1. Have I already applied this operation? (check state vector) → Skip
2. Do I have all operations that causally precede this one? → Apply
3. Am I missing prerequisite operations? → Buffer until prerequisites arrive

---

## Room Management

### Room Lifecycle

```
                create              join (first peer)
    ┌────────┐ ───────▶ ┌────────┐ ───────────────▶ ┌────────┐
    │  None  │          │ Created│                   │ Active │
    └────────┘          └────────┘                   └───┬────┘
                                                         │
                                          last peer      │ peers join/leave
                                          leaves         │
                                     ┌──────────────◀────┘
                                     │
                                     ▼
                              ┌─────────────┐   grace period   ┌──────────┐
                              │  Draining   │ ────────────────▶│  Closed  │
                              │ (30s grace) │                  │(cleaned) │
                              └─────────────┘                  └──────────┘
                                     │
                                     │ peer reconnects
                                     ▼
                              ┌─────────────┐
                              │  Active     │  (back to active)
                              └─────────────┘
```

### Room State

```rust
struct CollabRoom {
    id: RoomId,
    document_id: DocumentId,
    tenant_id: TenantId,

    // CRDT state
    crdt_doc: CollabDocument,        // Server's CRDT document
    version: u64,                    // Monotonic version counter

    // Connected peers
    peers: HashMap<PeerId, PeerState>,

    // Timing
    created_at: Instant,
    last_activity: Instant,
    last_save: Instant,

    // Configuration
    max_peers: usize,
    auto_save_interval: Duration,
}

struct PeerState {
    peer_id: PeerId,
    user_id: String,
    user_name: String,
    user_color: String,
    permissions: Permission,
    connected_at: Instant,
    last_activity: Instant,
    sender: WebSocketSender,         // Channel to send messages

    // Awareness
    cursor: Option<CursorPosition>,
    selection: Option<SelectionRange>,
    status: PeerStatus,              // Active, Idle, Away
}
```

### Room Operations

```rust
impl CollabRoom {
    /// Peer joins the room
    fn join(&mut self, peer: PeerState) -> Result<JoinResponse> {
        if self.peers.len() >= self.max_peers {
            return Err(RoomError::Full);
        }
        if !peer.permissions.can_view() {
            return Err(RoomError::PermissionDenied);
        }

        // Send current state to new peer
        let state = self.crdt_doc.encode_state();

        // Notify existing peers
        self.broadcast(PeerJoined { peer_id, user_name, user_color });

        self.peers.insert(peer.peer_id, peer);
        Ok(JoinResponse { state, peers: self.peer_list() })
    }

    /// Apply local operation from a peer
    fn apply_op(&mut self, peer_id: PeerId, ops: Vec<CrdtOperation>) -> Result<()> {
        let peer = self.peers.get(&peer_id)?;
        if !peer.permissions.can_edit() {
            return Err(RoomError::PermissionDenied);
        }

        // Validate and apply operations
        for op in &ops {
            self.crdt_doc.apply_remote_op(op)?;
        }

        // Broadcast to all other peers
        self.broadcast_except(peer_id, RemoteOp { peer_id, operations: ops });

        self.version += 1;
        self.last_activity = Instant::now();
        Ok(())
    }

    /// Auto-save to storage
    async fn auto_save(&mut self, storage: &dyn StorageBackend) -> Result<()> {
        // Materialize CRDT state to document format
        let doc_model = self.crdt_doc.to_document_model();
        let docx_bytes = DocxWriter::write(&doc_model)?;

        // Save to storage
        storage.save(&self.document_id, &docx_bytes, metadata).await?;

        // Save CRDT state separately (for fast room recovery)
        let crdt_state = self.crdt_doc.encode_state();
        storage.save_crdt_state(&self.document_id, &crdt_state).await?;

        self.last_save = Instant::now();
        self.broadcast(Saved { version: self.version });
        Ok(())
    }
}
```

---

## Synchronization Protocol

### Initial Sync (Peer Joins)

```
Client                                    Server
  │                                          │
  │  1. connect (JWT)                        │
  │──────────────────────────────────────▶   │
  │                                          │  validate JWT
  │  2. connected (peer_id, peers)           │  create peer state
  │  ◀──────────────────────────────────     │
  │                                          │
  │  3. sync_request (state_vector={})       │
  │──────────────────────────────────────▶   │
  │                                          │  encode full state
  │  4. sync_response (full CRDT state)      │
  │  ◀──────────────────────────────────     │
  │                                          │
  │  5. Client applies state, renders doc    │
  │                                          │
```

### Incremental Sync (Reconnection)

```
Client                                    Server
  │                                          │
  │  1. connect (JWT)                        │
  │──────────────────────────────────────▶   │
  │                                          │
  │  2. connected                            │
  │  ◀──────────────────────────────────     │
  │                                          │
  │  3. sync_request (state_vector=          │
  │     {0: 40, 1: 35})                      │
  │──────────────────────────────────────▶   │
  │                                          │  compute delta
  │  4. sync_response (only ops after        │  (ops since client's
  │     client's state vector)               │   last known state)
  │  ◀──────────────────────────────────     │
  │                                          │
  │  5. Client applies delta, renders        │
  │                                          │
```

### Steady-State Editing

```
Client A                  Server                  Client B
  │                          │                        │
  │  local_op (insert "x")   │                        │
  │─────────────────────────▶│                        │
  │                          │  validate op           │
  │                          │  apply to server CRDT  │
  │                          │                        │
  │                          │  remote_op (insert "x")│
  │                          │───────────────────────▶│
  │                          │                        │  apply to local CRDT
  │                          │                        │  re-render
  │                          │                        │
  │                          │  local_op (insert "y") │
  │                          │◀───────────────────────│
  │                          │  validate op           │
  │  remote_op (insert "y")  │  apply to server CRDT  │
  │◀─────────────────────────│                        │
  │  apply to local CRDT     │                        │
  │  re-render               │                        │
```

---

## Presence & Awareness

### Cursor Broadcasting

Each client sends cursor/selection updates at a throttled rate (max 10/sec):

```json
{
  "type": "awareness",
  "data": {
    "cursor": {
      "node_id": "0:15",
      "offset": 42
    },
    "selection": {
      "anchor": { "node_id": "0:15", "offset": 30 },
      "focus": { "node_id": "0:15", "offset": 42 }
    },
    "status": "active"
  }
}
```

### Cursor Rendering

Remote cursors are rendered as colored carets with labels:

```
                                    ┌──────────┐
                                    │ Bob      │  ← colored label
                                    └──────┬───┘
The quick brown fox jumps over the lazy │dog.
                                        ↑
                                     Bob's cursor (colored line)
```

Remote selections are rendered as colored highlights:

```
The quick ████████████ jumps over the lazy dog.
          ↑ Bob's selection (colored background)
```

### Presence States

| Status | Meaning | Trigger |
|--------|---------|---------|
| `active` | User is actively editing | Keystroke, mouse click |
| `idle` | User has the tab open but isn't editing | No activity for 2 minutes |
| `away` | User's tab is hidden | `visibilitychange` event |
| `offline` | User disconnected | WebSocket close |

### Presence Timeout

```
Active → (2 min no input) → Idle → (5 min no input) → Away
                                                         │
                                  Tab hidden ────────────┘
                                  (immediate via visibilitychange)
```

---

## Offline Editing (Future)

### Architecture

```
Online:    Client ←→ Server ←→ Other Clients
Offline:   Client ←→ Local IndexedDB (buffer operations)
Reconnect: Client → merge buffered ops → Server → broadcast to others
```

### Implementation Plan

1. Client buffers all operations in IndexedDB when offline
2. On reconnection, client sends state vector to server
3. Server responds with missed operations
4. Client sends buffered operations to server
5. CRDT guarantees convergence regardless of operation order

### Challenges

- **Large operation buffers**: Compress consecutive operations
- **Stale state**: Show "offline since X" indicator
- **Conflict visibility**: Show merge diff after reconnection
- **Storage limits**: IndexedDB quota varies by browser

---

## Scaling Strategies

### Single-Node (Default)

```
┌──────────────────────────────┐
│          s1-server            │
│                              │
│  Room 1: ████               │
│  Room 2: ██████             │  All rooms in same process
│  Room 3: ██                 │
│                              │
│  In-memory room state        │
│  Auto-save to storage        │
└──────────────────────────────┘
```

Suitable for: up to ~100 concurrent rooms, ~500 peers.

### Multi-Node with Redis

```
                    Load Balancer (sticky by room_id)
                    ┌──────────────────────┐
                    │  Hash(room_id) → node │
                    └──────┬────────┬──────┘
                    ┌──────┘        └──────┐
               ┌────▼─────┐          ┌────▼─────┐
               │ s1-srv 1 │          │ s1-srv 2 │
               │ Room A   │          │ Room C   │
               │ Room B   │          │ Room D   │
               └────┬─────┘          └────┬─────┘
                    │                      │
               ┌────▼──────────────────────▼─────┐
               │            Redis                 │
               │  pub/sub channels per room       │
               │  room → node mapping             │
               │  presence aggregation            │
               └──────────────────────────────────┘
```

**Room assignment**: Consistent hashing maps `room_id` to a node. If the node goes down, rooms are redistributed.

**Cross-node communication**: Not typical (rooms are pinned to nodes). Redis is used for:
- Room directory (which node owns which room)
- Admin queries (list all rooms across nodes)
- Health monitoring

### Database-Backed (High Durability)

```
               ┌──────────────────────────────────┐
               │          s1-server                │
               │                                  │
               │  Room Manager                    │
               │  ┌────────────────────────────┐  │
               │  │  In-memory: active rooms   │  │
               │  │  + hot CRDT state          │  │
               │  └────────────┬───────────────┘  │
               │               │ write-behind     │
               │  ┌────────────▼───────────────┐  │
               │  │  PostgreSQL:               │  │
               │  │  - Operation log (append)  │  │
               │  │  - Materialized snapshots  │  │
               │  │  - Room state              │  │
               │  └────────────────────────────┘  │
               └──────────────────────────────────┘
```

Operations are appended to an operation log for durability. Periodic snapshots reduce recovery time.

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Operation broadcast latency (same node) | < 5ms |
| Operation broadcast latency (cross-node via Redis) | < 20ms |
| Sync response (10-page doc) | < 50ms |
| Sync response (100-page doc) | < 200ms |
| Max peers per room | 50 |
| Max rooms per node | 1,000 |
| Memory per room (10-page doc) | < 5 MB |
| Auto-save interval | 30 seconds (configurable) |
| Reconnection with incremental sync | < 100ms |
| CRDT state encoding (10-page doc) | < 10ms |

---

## Error Recovery

### Client Disconnects

```
1. Server detects WebSocket close
2. Server starts grace period timer (30 seconds)
3. Server notifies other peers: peer_left (reason: "disconnect")
4. If client reconnects within grace period:
   - Restore peer state
   - Send incremental sync
   - Notify peers: peer_joined
5. If grace period expires:
   - Remove peer state
   - Clean up awareness data
```

### Server Crashes

```
1. All WebSocket connections drop
2. Clients detect disconnect, start reconnection loop
3. When server restarts:
   a. Load room state from storage (CRDT snapshots)
   b. Clients reconnect with their state vectors
   c. Server sends incremental sync from last snapshot
   d. Clients send any operations they have that server doesn't
   e. Convergence achieved
```

### Split Brain (Multi-Node)

```
1. Network partition separates Node A from Node B
2. Both nodes continue accepting operations for their rooms
3. Rooms are node-pinned, so no split-brain per room
4. If load balancer misroutes (sends room A's traffic to Node B):
   - Node B rejects (not room owner)
   - Client retries to correct node
5. When partition heals:
   - Redis reconnects
   - Room directory is authoritative
   - No data loss (rooms are always on one node)
```

---

## Migration Plan (Node.js → Rust)

### Phase 1: Build Rust WebSocket server alongside existing relay
- New endpoint in s1-server: `WS /ws/collab/:room_id`
- Same message protocol as existing relay
- No CRDT on server yet (pure relay, like current Node.js)

### Phase 2: Add server-side CRDT
- Server maintains `CollabDocument` per room
- Validates operations before relaying
- Enables state sync on join (no need for peer-to-peer sync)

### Phase 3: Add persistence
- Auto-save CRDT state to storage
- Room recovery on server restart
- Operation logging for audit

### Phase 4: Remove Node.js relay
- All collaboration through Rust server
- Delete `scripts/relay.js`
- Update documentation
