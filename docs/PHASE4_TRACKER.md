# Phase 4: Collaboration Service (Rust Rewrite)

> Goal: Rewrite Node.js relay in Rust, integrate into s1-server, add production features.
> Created: 2026-03-19 | Depends on: Phase 3 (complete)

## Milestone 4.1 — WebSocket Server

| ID | Task | Status |
|----|------|--------|
| P4-01 | Add WebSocket support to s1-server (axum ws feature) | DONE |
| P4-02 | Room management (create on first join, close when empty) | DONE |
| P4-03 | Operation broadcast to all peers via tokio::broadcast | DONE |
| P4-04 | Welcome message + peer lifecycle (join/leave logging) | DONE |

## Milestone 4.2 — CRDT Integration

| ID | Task | Status |
|----|------|--------|
| P4-05 | Server-side CRDT document (s1-crdt) | OPEN |
| P4-06 | State synchronization (full state + incremental) | OPEN |
| P4-07 | Operation validation on server | OPEN |

## Milestone 4.3 — Persistence

| ID | Task | Status |
|----|------|--------|
| P4-08 | Auto-save CRDT state to storage backend | OPEN |
| P4-09 | Room state recovery on server restart | OPEN |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P4-01 | 2026-03-19 | axum ws feature + collab.rs: WebSocketUpgrade handler at /ws/collab/:room_id |
| P4-02 | 2026-03-19 | RoomManager with Mutex<HashMap<String, Room>>: get_or_create, leave, auto-close when empty |
| P4-03 | 2026-03-19 | tokio::broadcast channel (256 buffer) per room; incoming text messages broadcast to all peers |
| P4-04 | 2026-03-19 | Welcome JSON on connect; tracing::info on room create/close; debug on peer disconnect |
