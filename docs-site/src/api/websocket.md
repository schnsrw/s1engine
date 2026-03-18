# WebSocket Protocol

Real-time collaboration uses WebSocket at:

```
ws://localhost:8080/ws/collab/{room_id}
```

## Connection Flow

1. Client connects to `ws://host/ws/collab/my-doc-123`
2. Server sends welcome: `{"type":"welcome","roomId":"my-doc-123","opsCount":0}`
3. If late joiner, server sends catch-up ops: `{"type":"catchUp","op":{...}}`
4. Client sends operations as JSON text messages
5. Server validates and broadcasts to all peers in the room

## Message Types

### Client → Server

```json
{"type":"op","action":"insertText","nodeId":"0:5","offset":3,"text":"hello"}
{"type":"op","action":"formatSelection","startNode":"0:5","startOffset":0,"endNode":"0:5","endOffset":5,"key":"bold","value":"true"}
{"type":"awareness","cursor":{"nodeId":"0:5","offset":3},"user":"Alice"}
```

### Server → Client

```json
{"type":"welcome","roomId":"my-doc","opsCount":42}
{"type":"catchUp","op":{"action":"insertText",...}}
```

## Room Lifecycle

- Room created on first peer join
- All operations broadcast to all peers
- Operations logged for late-joiner catch-up
- Dirty rooms auto-saved to storage every 30 seconds
- Room closed when last peer disconnects
