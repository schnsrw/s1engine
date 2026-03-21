# s1engine Collaboration Protocol Specification v3.1

## 1. Design Goals

| Goal | Target | Status |
|------|--------|--------|
| Text change latency | <20ms peer-to-peer | DONE (CRDT native) |
| Conflict resolution | Zero lost edits | DONE (Fugue CRDT) |
| Convergence | All peers identical within 2s | DONE (fullSync fallback) |
| Offline tolerance | Buffer 10k ops, resync on reconnect | DONE |
| Reconnect recovery | Version check on every heartbeat | DONE |
| Bandwidth optimization | Binary DOCX for fullSync | SPECIFIED (not yet implemented) |
| Version authority | Server-assigned monotonic counter | DONE |

## 2. Architecture

```
┌──────────────┐          ┌────────────────────┐          ┌──────────────┐
│   Peer A      │◄────────►│     s1-server       │◄────────►│   Peer B      │
│               │   WS     │                    │   WS     │               │
│ WasmCollabDoc │          │ Room {              │          │ WasmCollabDoc │
│ localVer=5    │          │   version: 7        │          │ localVer=3    │
│ serverVer=7   │          │   ops_log: [(v,op)] │          │ serverVer=7   │
│ replicaId=42  │          │   peers: [A,B]     │          │ replicaId=99  │
│ CRDT state    │          │   snapshot: bytes   │          │ CRDT state    │
│ offlineBuffer │          │ }                   │          │ offlineBuffer │
└──────────────┘          └────────────────────┘          └──────────────┘
```

### 2.1 CRDT Layer Availability

The CRDT layer (WasmCollabDocument) is **optional**. It requires `engine.open_collab()` which may not be available in all WASM builds.

**When CRDT is available:**
- Text insert/delete use native CRDT ops (<20ms)
- State vector exchange for delta sync
- Awareness via `collabDoc.set_cursor()`

**When CRDT is NOT available (fallback mode):**
- ALL operations use best-effort broadcastOp + debounced fullSync (1.5s)
- No state vectors, no delta sync
- Cursor broadcast via manual JSON payload
- Still functional but higher latency for text sync

**Detection**: `typeof state.engine.open_collab === 'function'`

### Server is the version authority
- Server assigns `roomVersion` to every op (monotonically increasing)
- Peers track `serverVersion` (highest version seen from server)
- Peers track `localVersion` (own edit counter, independent of server)
- Every server→client message includes `serverVersion`
- Presence heartbeats include `clientVersion` so server knows each peer's state

## 3. State

### 3.1 Per Peer (client-side)
```javascript
peerId: string           // Unique per WS connection (random)
replicaId: number        // Unique for CRDT (random u64, stable per session)
userName: string
userColor: string        // From PEER_COLORS palette
accessLevel: 'edit'|'comment'|'view'
localVersion: number     // Increments on every local edit
serverVersion: number    // Highest version seen from server
lastSyncedVersion: number // Version of last applied fullSync
collabDoc: WasmCollabDocument  // Fugue CRDT state
connected: boolean
offlineBuffer: Op[]      // Max 10,000 ops, with warnings at 8,000
```

### 3.2 Per Room (server-side)
```rust
tx: broadcast::Sender<String>  // Broadcast channel (capacity 512)
peers: Vec<PeerInfo>            // Connected peers
ops_log: Vec<(u64, String)>    // (version, op_json) — max 10k, truncates oldest 5k
version: u64                    // Monotonic counter
doc_id: String                  // File session ID
dirty: bool                     // Has unsaved ops
```

### 3.3 Per Peer in Room (server-side)
```rust
peer_id: String
user_name: String
user_color: String
// Also tracked in FileSession.editors:
//   mode: String (edit/comment/view)
//   last_activity: String (ISO 8601)
```

## 4. Message Protocol

### 4.1 Connection Handshake

```
Step 1: Client opens WebSocket
  Client → Server:  WS /ws/collab/{fileId}?user=Alice&uid=u-abc&access=edit

Step 2: Server sends welcome
  Server → Client:  {
    type: "joined",
    peerId: "u-abc",
    room: "fileId",
    peers: [{ peerId, userName, userColor }],
    access: "edit",
    serverVersion: 42
  }

Step 3: Server sends document snapshot (if session has data)
  Server → Client:  {
    type: "snapshot",
    data: "<base64 DOCX bytes>",
    size: 12345
  }

Step 4: Server sends catch-up ops (versioned)
  Server → Client:  (for each op in ops_log)
    { type: "op", peerId: "server", data: "<op json>", serverVersion: N }

Step 5: Client announces presence
  Client → Server:  {
    type: "join",
    room: "fileId",
    userName: "Alice",
    userColor: "#4285f4",
    clientVersion: 0
  }

Step 6: Server broadcasts peer-join to existing peers
  Server → Others:  {
    type: "peer-join",
    peerId: "u-abc",
    userName: "Alice",
    userColor: "#4285f4",
    _sender: "u-abc"
  }
```

### 4.2 Text Editing (CRDT — Tier 1)

```
1. User types 'a' at offset 5 in paragraph P
2. collabDoc.apply_local_insert_text(P, 5, 'a')
   → Returns CrdtOperation JSON
3. Client → Server:  { type: "op", room: R, data: JSON.stringify({
     action: "crdtOp", ops: "<CrdtOperation JSON>"
   })}
4. Server:
   a. Validates JSON
   b. roomVersion++
   c. Stores in ops_log: (roomVersion, op_data)
   d. Forwards with version: { type: "op", peerId: "u-abc", data: ..., serverVersion: N, _sender: "u-abc" }
5. Other peers:
   a. Parse crdtOp
   b. collabDoc.apply_remote_ops(json)
   c. renderNodeById(affected paragraph)
```

### 4.3 Structural Editing (fullSync — Tier 2)

```
1. User presses Enter (split paragraph)
2. doc.split_paragraph(nodeId, offset)
3. broadcastOp({ action: "splitParagraph", nodeId, offset })
4. scheduleDebouncedFullSync() → fires after 1.5s (5s in CRDT mode)
5. fullSync sends entire document:
   Client → Server:  { type: "op", room: R, data: JSON.stringify({
     action: "fullSync", docBase64: "<base64>", version: localVersion, replicaId: peerId
   })}
6. Server increments roomVersion, stores in ops_log, forwards
7. Receiving peer:
   a. Checks version > lastSyncedVersion (ignores stale)
   b. state.doc = state.engine.open(bytes)
   c. lastSyncedVersion = version
   d. renderDocument()
```

### 4.4 Presence/Heartbeat (every 500ms)

```
Client → Server:  {
  type: "awareness",
  room: R,
  data: JSON.stringify({
    peerId: "u-abc",
    nodeId: "0:5",          // Paragraph containing cursor
    offset: 12,             // Character offset from paragraph start
    userName: "Alice",
    userColor: "#4285f4",
    clientVersion: 42       // CRITICAL: tells server our version
  })
}

For view-only users (no cursor position):
Client → Server:  {
  type: "awareness",
  room: R,
  data: JSON.stringify({
    peerId: "u-abc",
    userName: "Alice",
    userColor: "#4285f4",
    clientVersion: 42,
    heartbeat: true
  })
}

Server forwards to others with serverVersion:
Server → Others:  { type: "awareness", peerId: "u-abc", data: ..., serverVersion: 42, _sender: "u-abc" }
```

**Dual purpose**: Heartbeat serves as both cursor sync AND version check. If server's roomVersion > peer's clientVersion, the peer is behind.

### 4.5 Version Gap Detection

Client checks on EVERY received message:
```javascript
if (msg.serverVersion) {
  if (msg.serverVersion > serverVersion + 1) {
    // GAP DETECTED: we missed ops
    console.warn('Version gap:', serverVersion, '→', msg.serverVersion);
    sendOp({ action: 'requestFullSync', myVersion: serverVersion });
  }
  serverVersion = msg.serverVersion;
}
```

Any peer receiving `requestFullSync` with `myVersion < serverVersion` responds:
```javascript
case 'requestFullSync':
  if (op.myVersion < serverVersion) {
    sendFullSync();  // Send our current state
  }
  break;
```

### 4.6 Reconnect Recovery

```
Timeline:
  T=0:   Connection drops
  T=0:   Client detects via ws.readyState !== OPEN (checked every 500ms in heartbeat)
  T=0:   connected = false, start buffering ops
  T=2s:  First reconnect attempt (exponential backoff: 2s, 4s, 8s, 16s, 30s)
  T=2s:  WS reconnect to /ws/collab/{fileId}?user=X&uid=Y&access=edit
  T=2s:  Server sends "joined" with current serverVersion=50
  T=2s:  Client compares: myServerVersion=42, server=50 → 8 ops behind
  T=2s:  Server sends catch-up ops (version 43-50) from ops_log
  T=2s:  Client applies catch-up ops
  T=2s:  Client flushes offline buffer (ops made while disconnected)
  T=2s:  Client sends CRDT stateVector for delta sync
  T=2s:  Existing peers respond with changes_since
  T=2.5s: All peers converged
```

### 4.7 Disconnect Cleanup

```
Server-side:
  T=0:    Peer disconnects
  T=0:    Server broadcasts peer-leave to room
  T=0:    FileSession.editor_leave(fileId, peerId) — decrements editor count
  T=0:    RoomManager.leave(fileId, peerId) — removes from peers list
  T=5min: Stale editor cleanup (if last_activity > 5min)
  T=5min: If editor_count == 0: start session TTL (5 min default)
  T=10min: Session cleanup — session removed, data lost

Client-side:
  T=0:    ws.onclose fires
  T=0:    connected = false
  T=0:    updateConnectionStatus('disconnected')
  T=0:    Clear peer cursors from DOM
  T=2s:   scheduleReconnect (exponential backoff)
  T=max:  After 5 failed attempts: give up, show "Connection lost" toast
```

## 5. Sync Decision Matrix

| Operation | Node Count Change | Sync Method | Debounce |
|-----------|-------------------|-------------|----------|
| Type character | 0 | CRDT native | None (immediate) |
| Delete character | 0 | CRDT native | None |
| Format selection | 0 | Best-effort op | 1.5s fullSync |
| Set heading/list | 0 | Best-effort op | 1.5s fullSync |
| Set alignment | 0 | Best-effort op | 1.5s fullSync |
| Split paragraph (Enter) | +1 | Immediate fullSync | None |
| Merge paragraphs (Backspace) | -1 | Immediate fullSync | None |
| Cut (same paragraph) | 0 | CRDT delete | None |
| Cut (cross-paragraph) | -N | Immediate fullSync | None |

**Cut clarification**: "Same paragraph" means the selection start and end are within a single paragraph node. The node count does not change, so CRDT character-level delete ops are sufficient. "Cross-paragraph" means the selection spans two or more paragraphs. Intermediate paragraphs are deleted (node count decreases), and the remaining text from the last paragraph is merged into the first. This structural change requires immediate fullSync because CRDT does not yet handle node deletion/merge.
| Paste (single line) | 0 | Best-effort op | 1.5s fullSync |
| Paste (multi-line) | +N | Immediate fullSync | None |
| Insert table | +N | Immediate fullSync | None |
| Insert image | +1 | Immediate fullSync | None |
| Insert page break | +1 | Immediate fullSync | None |
| Delete node | -1 | Immediate fullSync | None |
| Undo/redo | Varies | CRDT inverse op | 1.5s fullSync |

**Rule**: `nodeCountChange != 0` → immediate fullSync. Otherwise → CRDT or debounced.

## 6. Edge Cases (22 total)

### Category A: Concurrent Editing

| # | Scenario | Resolution |
|---|----------|------------|
| 6.1 | Same paragraph, same position | CRDT Fugue interleaves by replicaId (deterministic) |
| 6.2 | Same paragraph, different positions | CRDT handles independently |
| 6.3 | Different paragraphs | Independent CRDT ops, no conflict |
| 6.4 | A formats, B types in same range | Format applies to existing chars; new chars get/don't get format based on position |
| 6.5 | A and B both bold overlapping ranges | Both formats apply (commutative CRDT attribute ops) |

### Category B: Structural Conflicts

| # | Scenario | Resolution |
|---|----------|------------|
| 6.6 | A cuts all, B types | A's fullSync arrives → B's CRDT ops for old nodes dropped. B's local edits survive in B's doc; B's next fullSync sends them. **Merge algorithm**: (1) A's fullSync contains a single empty paragraph. (2) B receives fullSync and replaces their document. (3) B's locally typed characters that were in-flight are lost on B's side. (4) B continues typing into the empty paragraph. (5) B's next fullSync or CRDT op sends the new text to A. **Net result**: A sees empty doc, then B's new text arrives. No data loss for characters typed AFTER fullSync receipt. Characters typed between A's cut and B's receipt of fullSync are lost (acceptable -- same behavior as Google Docs). |
| 6.7 | A splits para, B types in same para | fullSync resolves; B's text goes to one of the two resulting paragraphs |
| 6.8 | A deletes para, B types in it | fullSync with deletion wins; B's text lost (acceptable — CRDT would handle this better once structural ops use CRDT) |
| 6.9 | A inserts table at position, B inserts image at same position | fullSync from last editor wins (version ordering) |

### Category C: Network Issues

| # | Scenario | Resolution |
|---|----------|------------|
| 6.10 | WS message lost | Version gap detected → requestFullSync → peer responds |
| 6.11 | Disconnect during edit | Offline buffer → flush on reconnect → version catchup |
| 6.12 | Rapid reconnect flapping | Exponential backoff (2-30s), don't clear buffer until confirmed |
| 6.13 | Server restart | All peers disconnect → first reconnector uploads state → others sync |
| 6.14 | Very old peer reconnects (>1000 versions behind) | Server sends snapshot instead of 1000 catch-up ops |

### Category D: Client Issues

| # | Scenario | Resolution |
|---|----------|------------|
| 6.15 | Tab duplication | New peerId per connection; both tabs are independent peers |
| 6.16 | Browser crash (no clean disconnect) | Server ping timeout → cleanup after 15s of no pong |
| 6.17 | Cursor jump after remote edit | Save cursor by paragraph nodeId + offset, restore after render |
| 6.18 | Selection invalidated by remote edit | Non-CRDT path: selection cleared by fullSync (user re-selects) |
| 6.19 | Paste from external source | parseClipboardHtml normalizes → fullSync broadcasts canonical state |

### Category E: Resource Limits

| # | Scenario | Resolution |
|---|----------|------------|
| 6.20 | Document >5MB | Increase fullSync debounce to 10s; rely on CRDT for text ops |
| 6.21 | ops_log >10k entries | Truncate oldest 5k with warning; late joiners get snapshot |
| 6.22 | >10 concurrent peers | Broadcast channel capacity 512; ops buffered if slow |

### 6.23 CRDT op arrives after fullSync changed node IDs

When fullSync replaces the document, all in-flight CRDT ops referencing old node IDs are silently dropped. The CRDT state is reset from the new document. This is acceptable because fullSync is the authoritative state.

**Sequence:**
1. Peer A sends CRDT op for node `42:5`
2. Peer B sends fullSync (which reassigns node IDs)
3. Peer C receives fullSync first, replaces document (node `42:5` no longer exists)
4. Peer C receives CRDT op for `42:5` -- try/catch in `applyRemoteOp` catches "node not found", op is dropped
5. Peer C's state matches Peer B's fullSync (correct)

**Design decision:** fullSync always wins over in-flight CRDT ops. This simplifies convergence at the cost of occasionally dropping a few characters that will be re-sent by the original author on their next keystroke or fullSync.

## 7. Error Recovery

| Error | Detection | Recovery |
|-------|-----------|----------|
| WASM op fails (node not found) | try/catch in applyRemoteOp | Silent drop + wait for fullSync |
| fullSync decode fails | try/catch on base64+open | Request fullSync from another peer |
| WebSocket error | ws.onerror + ws.onclose | Exponential backoff reconnect |
| Version gap | serverVersion > myVersion+1 | requestFullSync |
| Offline buffer full (10k) | Buffer length check | Show warning toast; oldest ops dropped |
| Document corruption | Tree validation after remote ops | Request fullSync from highest-version peer |

## 8. Monitoring & Observability

| Metric | Where | Purpose |
|--------|-------|---------|
| `serverVersion` per room | Server logs | Track room activity |
| `ops_log.len()` per room | Server logs (on truncation) | Memory pressure |
| `editor_count` per session | Admin API `/admin/api/sessions` | Active usage |
| `last_activity` per editor | Admin API + dashboard | Stale detection |
| Client errors | POST `/api/v1/errors` | Client-side crash tracking |
| Reconnect count per peer | Client console.debug | Network quality |
| fullSync frequency | Client console.debug | Structural edit rate |
| CRDT op count | `collabDoc.op_log_size()` | CRDT state size |

## 9. Implementation Checklist

### Server (collab.rs)

| Component | Status |
|-----------|--------|
| Room with `version: u64` counter | DONE |
| `ops_log: Vec<(u64, String)>` versioned | DONE |
| `record_op()` returns version | DONE |
| `get_version(room_id)` | DONE |
| `get_ops_since(room_id, from_version)` | DONE |
| `serverVersion` on every forwarded `op` | DONE |
| `serverVersion` on every forwarded `awareness` | DONE |
| `serverVersion` in `joined` message | DONE |
| `requestCatchup` handler | DONE |
| Version-aware catch-up on join | DONE |
| `_sender` echo filtering | DONE |
| `peer-join`/`peer-leave` broadcast | DONE |
| Server ping every 15s | DONE |
| Stale editor cleanup (5min) | DONE |
| `update_activity()` on every message | DONE |
| Binary WebSocket (`Message::Binary`) | SPECIFIED (Section 10) |
| Debounce `peer-leave` broadcast (5s) | SPECIFIED (Section 11) |
| Snapshot storage on fullSync via `update_snapshot` | DONE |

### Client (collab.js)

| Component | Status |
|-----------|--------|
| CRDT text insert/delete via `beforeinput` | DONE |
| `broadcastCrdtOp()` for native CRDT ops | DONE |
| `localVersion` counter (increments on edit) | DONE |
| `serverVersion` tracking from every message | DONE |
| `lastSyncedVersion` for fullSync ordering | DONE |
| `clientVersion` in awareness heartbeat | DONE |
| `clientVersion` in join message | DONE |
| Version gap detection (`serverVersion > myVersion + 1`) | DONE |
| `requestFullSync` on gap detection | DONE |
| `requestFullSync` handler (respond to behind peers) | DONE |
| Debounced fullSync (1.5s / 5s in CRDT mode) | DONE |
| Version in fullSync messages | DONE |
| Version-aware fullSync receiver (ignore stale) | DONE |
| Offline buffer (10k max, warnings at 8k) | DONE |
| Exponential backoff reconnect (2-30s, 5 attempts) | DONE |
| View-only heartbeat (no cursor) | DONE |
| CRDT awareness via `collabDoc.set_cursor()` | DONE |
| State vector exchange on join | DONE |
| Binary WebSocket for fullSync | SPECIFIED (Section 10) |
| Cursor restore after fullSync | PARTIAL (saves before, restores after) |

## 10. Binary WebSocket for fullSync

### 10.1 Overview

The current fullSync implementation encodes the entire document as base64 text, adding approximately 33% bandwidth overhead. Binary WebSocket frames eliminate this overhead by transmitting raw document bytes alongside a small JSON header.

### 10.2 Binary Frame Format

Each binary WebSocket frame uses a header-length-prefixed format:

```
┌────────────────────┬──────────────────────────┬───────────────────────┐
│  4 bytes (u32 BE)  │  JSON header (UTF-8)      │  Raw document bytes   │
│  Header length     │  Variable length           │  Variable length      │
└────────────────────┴──────────────────────────┴───────────────────────┘
```

**Byte layout:**
- **Bytes 0-3**: Header length as a big-endian unsigned 32-bit integer (maximum header size: 4GB, practically <1KB).
- **Bytes 4 to 4+headerLength-1**: UTF-8 encoded JSON header.
- **Bytes 4+headerLength to end**: Raw document bytes (DOCX ZIP archive).

### 10.3 JSON Header Schema

```json
{
  "action": "fullSync",
  "version": 42,
  "room": "file-abc-123",
  "replicaId": "u-abc",
  "size": 524288
}
```

| Field | Type | Description |
|-------|------|-------------|
| `action` | string | Always `"fullSync"` for binary frames |
| `version` | u64 | Sender's `localVersion` at time of sync |
| `room` | string | Room/file identifier |
| `replicaId` | string | Sender's peer ID |
| `size` | u64 | Byte length of the raw document payload (for validation) |

### 10.4 Server Handling

```
Server receives Message::Binary(bytes):
  1. Read bytes[0..4] as u32 big-endian → headerLen
  2. Read bytes[4..4+headerLen] as UTF-8 → parse JSON header
  3. Validate:
     a. header.action == "fullSync"
     b. header.room matches the WebSocket's associated room
     c. header.size == bytes.len() - 4 - headerLen
     d. If validation fails → send error JSON text message, drop frame
  4. roomVersion++
  5. Store in ops_log: (roomVersion, BinaryOp { header, payload_hash })
     Note: ops_log stores only the header + hash, NOT the full payload.
     The full payload is stored as the room's latest snapshot.
  6. Forward the ENTIRE binary frame to all other peers in the room:
     - Inject `serverVersion` into the header before forwarding:
       Re-serialize header with added `serverVersion` field
       Reconstruct binary frame: [4 bytes new header len][new header JSON][original raw bytes]
     - Send as Message::Binary to each peer
  7. Update room snapshot (for late joiners):
     room.snapshot = raw document bytes (the payload portion)
```

### 10.5 Client Sending

```javascript
function sendBinaryFullSync() {
  const docBytes = state.doc.export("docx"); // Uint8Array
  const header = JSON.stringify({
    action: "fullSync",
    version: state.localVersion,
    room: state.room,
    replicaId: state.peerId,
    size: docBytes.byteLength
  });
  const headerBytes = new TextEncoder().encode(header);
  const headerLen = new DataView(new ArrayBuffer(4));
  headerLen.setUint32(0, headerBytes.byteLength, false); // big-endian

  const frame = new Uint8Array(4 + headerBytes.byteLength + docBytes.byteLength);
  frame.set(new Uint8Array(headerLen.buffer), 0);
  frame.set(headerBytes, 4);
  frame.set(docBytes, 4 + headerBytes.byteLength);

  if (ws.readyState === WebSocket.OPEN) {
    ws.binaryType = "arraybuffer";
    ws.send(frame.buffer);
  }
}
```

### 10.6 Client Receiving

```javascript
ws.binaryType = "arraybuffer";

ws.onmessage = function(e) {
  if (e.data instanceof ArrayBuffer) {
    // Binary frame
    const view = new DataView(e.data);
    const headerLen = view.getUint32(0, false); // big-endian
    const headerStr = new TextDecoder().decode(
      new Uint8Array(e.data, 4, headerLen)
    );
    const header = JSON.parse(headerStr);
    const docBytes = new Uint8Array(e.data, 4 + headerLen);

    // Validate
    if (header.action !== "fullSync") return;
    if (header.size !== docBytes.byteLength) {
      console.error("Binary fullSync size mismatch");
      return;
    }

    // Version check (same as text fullSync)
    if (header.version <= state.lastSyncedVersion) return; // stale

    // Update server version
    if (header.serverVersion) {
      state.serverVersion = header.serverVersion;
    }

    // Apply document
    state.doc = state.engine.open(docBytes);
    state.lastSyncedVersion = header.version;
    renderDocument();
  } else {
    // Text frame — existing JSON message handling
    const msg = JSON.parse(e.data);
    handleTextMessage(msg);
  }
};
```

### 10.7 Fallback to Base64 Text

Binary WebSocket may fail in certain environments (e.g., some reverse proxies strip binary frames, or older WebSocket libraries do not support them).

**Fallback state machine:**

```
                     ┌──────────────┐
        Start ──────►│ TRY_BINARY   │
                     └──────┬───────┘
                            │
                  send binary fullSync
                            │
                ┌───────────┴───────────┐
                │                       │
           success                  failure (ws.onerror
                │                   or no ACK within 5s)
                ▼                       │
        ┌──────────────┐               ▼
        │ BINARY_MODE  │       ┌──────────────┐
        │ (all future   │       │ TEXT_MODE     │
        │  fullSync     │       │ (base64 JSON  │
        │  use binary)  │       │  forever)     │
        └──────────────┘       └──────────────┘
```

**Detection:**
1. On first fullSync, attempt to send as binary.
2. Set a 5-second timer. If the server echoes back or acknowledges (via `serverVersion` increment visible in next awareness), mark `binarySupported = true`.
3. If no acknowledgment within 5s or an error occurs, fall back to `binarySupported = false` and use base64 text for all future fullSync messages.
4. Log the mode to console: `console.info("fullSync mode:", binarySupported ? "binary" : "text/base64")`.

### 10.8 Bandwidth Savings

| Document Size | Base64 Text | Binary | Savings |
|---------------|-------------|--------|---------|
| 100KB | 137KB | 100.1KB | 27% |
| 500KB | 683KB | 500.2KB | 27% |
| 2MB | 2.73MB | 2.00MB | 27% |
| 10MB | 13.7MB | 10.0MB | 27% |

The JSON header adds approximately 100-200 bytes of overhead per frame, which is negligible for typical document sizes.

## 11. Debounced Peer-Leave Protocol

### 11.1 Problem

When a WebSocket connection drops briefly (network hiccup, browser tab suspension), the server immediately broadcasts `peer-leave`, causing other peers to remove the cursor and show a "left" notification. If the peer reconnects within seconds, this creates annoying UI flicker: leave notification followed immediately by a join notification.

### 11.2 Solution

The server waits 5 seconds after WebSocket close before broadcasting `peer-leave`. If the peer reconnects within that window, the leave is cancelled silently.

### 11.3 State Machine

```
                                    ┌─────────────────┐
              WS connected ────────►│    CONNECTED     │
                                    └────────┬────────┘
                                             │
                                    ws.onclose fires
                                             │
                                             ▼
                                    ┌─────────────────┐
                                    │ PENDING_LEAVE    │
                                    │ timer = 5s       │
                                    │ peer still in    │
                                    │ room.peers list  │
                                    └────────┬────────┘
                                             │
                              ┌──────────────┴──────────────┐
                              │                             │
                  peer reconnects                   5s timer expires
                  (same uid in room)                        │
                              │                             │
                              ▼                             ▼
                   ┌─────────────────┐           ┌─────────────────┐
                   │   RECONNECTED   │           │     LEFT        │
                   │ cancel timer    │           │ broadcast       │
                   │ keep peer in    │           │ peer-leave      │
                   │ room.peers      │           │ remove from     │
                   │ update ws handle│           │ room.peers      │
                   │ NO peer-leave   │           │ clean up state  │
                   │ NO peer-join    │           └─────────────────┘
                   │ (silent rejoin) │
                   └────────┬────────┘
                            │
                   Send catch-up ops
                   (version gap fill)
                            │
                            ▼
                   ┌─────────────────┐
                   │   CONNECTED     │
                   │ (back to normal)│
                   └─────────────────┘
```

### 11.4 Server Implementation (Pseudocode)

```rust
// In Room struct:
struct Room {
    peers: HashMap<String, PeerState>,
    pending_leaves: HashMap<String, tokio::task::JoinHandle<()>>,
    // ...existing fields...
}

enum PeerState {
    Connected { ws_tx: SplitSink, info: PeerInfo },
    PendingLeave { info: PeerInfo, disconnected_at: Instant },
}

// On WebSocket close:
fn on_ws_close(room: &mut Room, peer_id: &str) {
    if let Some(peer) = room.peers.get_mut(peer_id) {
        // Transition to PendingLeave — do NOT remove from peers yet
        let info = peer.info().clone();
        *peer = PeerState::PendingLeave {
            info,
            disconnected_at: Instant::now(),
        };

        // Schedule delayed leave
        let room_id = room.id.clone();
        let pid = peer_id.to_string();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            // If still pending after 5s, execute the leave
            execute_peer_leave(&room_id, &pid).await;
        });

        room.pending_leaves.insert(peer_id.to_string(), handle);
    }
}

// On WebSocket reconnect (same uid joins same room):
fn on_ws_reconnect(room: &mut Room, peer_id: &str, new_ws_tx: SplitSink) {
    // Cancel pending leave timer
    if let Some(handle) = room.pending_leaves.remove(peer_id) {
        handle.abort(); // Cancel the 5s timer
    }

    // Update peer state back to Connected with new WS handle
    if let Some(peer) = room.peers.get_mut(peer_id) {
        let info = peer.info().clone();
        *peer = PeerState::Connected { ws_tx: new_ws_tx, info };
    }

    // Send catch-up ops (version gap fill) — no peer-join broadcast
    send_catchup_ops(room, peer_id);
}

// Executed after 5s timeout:
async fn execute_peer_leave(room_id: &str, peer_id: &str) {
    let room = get_room(room_id);
    // Only proceed if peer is still in PendingLeave state
    if matches!(room.peers.get(peer_id), Some(PeerState::PendingLeave { .. })) {
        room.peers.remove(peer_id);
        room.pending_leaves.remove(peer_id);
        broadcast_to_room(room, json!({
            "type": "peer-leave",
            "peerId": peer_id,
            "_sender": peer_id
        }));
        // FileSession cleanup
        editor_leave(room_id, peer_id);
    }
}
```

### 11.5 Client Behavior

The client does NOT need to change. From the client's perspective:
- Reconnect uses the same `uid` query parameter, so the server can match it to the pending-leave peer.
- If reconnection happens within 5s, the client receives catch-up ops as usual (no `peer-join` or `peer-leave` messages are broadcast).
- If reconnection happens after 5s, the server has already broadcast `peer-leave`, and the reconnecting client is treated as a new join (receives `joined` + `peer-join` broadcast).

### 11.6 Edge Cases

| Scenario | Behavior |
|----------|----------|
| Peer reconnects at exactly 5s | Race condition: if timer fires first, treated as new join. If reconnect message arrives first, silent rejoin. Both are correct outcomes. |
| Peer reconnects to different room | Not a reconnect — treated as new connection. Original room's 5s timer still runs. |
| Server restarts during 5s window | All pending-leave timers lost. All peers are disconnected. First reconnector uploads state. |
| Multiple rapid disconnect/reconnect cycles | Each disconnect resets the 5s timer. Only the final disconnect starts a fresh timer. |
| Peer's browser crashes (no clean close) | Server detects via ping timeout (15s). Then 5s peer-leave timer starts. Total: 20s before peer-leave broadcast. |

## 12. Future Improvements

| Improvement | Priority | Description |
|-------------|----------|-------------|
| Binary WebSocket | HIGH | Eliminate base64 overhead (33% bandwidth savings) |
| Protobuf for ops | MEDIUM | Structured binary format instead of JSON |
| CRDT for structural ops | HIGH | Move split/merge/table to CRDT (eliminate fullSync for most cases) |
| Operational compression | LOW | Batch consecutive single-char inserts into multi-char ops |
| Peer-to-peer fallback | LOW | WebRTC data channel if server is slow/unavailable |
| Conflict-free undo | MEDIUM | Undo that works correctly across concurrent edits |
| Awareness protocol optimization | LOW | Delta-compressed cursor updates (only send changes) |
