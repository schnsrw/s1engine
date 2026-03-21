# Collaboration Setup

Rudra Code uses Fugue CRDT for conflict-free real-time editing.

## Architecture

```
Client A ──→ WebSocket ──→ Relay Server ──→ WebSocket ──→ Client B
              (operations broadcast to all peers)
```

## Using the Node.js Relay

```bash
# Start the relay server
node scripts/relay.js
# Serves editor at http://localhost:8787
# WebSocket at ws://localhost:8787
```

## Using the Rust Server

```bash
# Start s1-server (includes WebSocket relay)
cargo run -p s1-server
# REST API at http://localhost:8080/api/v1/
# WebSocket at ws://localhost:8080/ws/collab/{room_id}
```

## Editor Integration

The editor auto-connects when given a relay URL:

```javascript
// In editor state
state.collabUrl = 'ws://localhost:8787';
state.collabRoom = 'my-document-id';
state.userName = 'Alice';
```

## How It Works

1. Each client maintains a local CRDT document
2. Local edits generate operations with unique IDs
3. Operations are broadcast to all peers via WebSocket
4. Each peer applies remote operations to their local CRDT
5. Fugue algorithm guarantees all peers converge to the same state
