# GEMINI.md - Project Mandates & Core Instructions

This file contains foundational mandates for Gemini CLI and other AI agents working on the Rudra Office project. These instructions take absolute precedence over general defaults.

## 1. Core Engineering Mandates

- **WASM Ownership:** The Document Engine (Rust/WASM) is the single source of truth. All visual fragments and layout are dictaded by the engine.
- **CRDT-Native Operations:** ALL document mutations (text and structural) MUST use the range-aware WASM CRDT API. Never rebuild the collab state from a static model; always apply transactions to preserve Lamport history.
- **Atomic Text Sync:** Use the diff-from-truth strategy in `render.js` for all text synchronization.
- **Surgical Updates:** Maintain the modular structure of the vanilla JS editor layer.

## 2. Collaboration & Persistence

- **Version Authority**: Trust the `roomVersion` and `serverVersion` during handshake.
- **Incremental Convergence**: Rely on CRDT operations for structural convergence. `fullSync` is a rare safety net, not a standard sync mechanism.
- **Zero CDN Dependency**: All assets (fonts, icons, scripts) MUST be self-hosted to ensure offline and enterprise compatibility.

## 2. Tracking & Process Mandates

- **Improvement Pipeline:** The `docs/improvement-plan/IMPROVEMENT_TRACKER.md` is the authoritative guide for the project's evolution. Do NOT deviate from this plan without explicit user confirmation.
- **Progress Verification:** After completing a task, update the tracker to 🟢 (Completed) and, after verification, to 🔵 (Verified). Provide the test output as proof of completion.
- **Zero Regression:** Always run existing tests before and after a change to ensure no regression in core editor functionality.

## 2. Collaboration Protocol

- **Monotonic Versioning:** Always respect the server-assigned `roomVersion`.
- **Binary Sync:** Favor binary WebSocket frames for document snapshots (`fullSync`) over Base64 strings to minimize bandwidth.
- **No Data Loss:** Ensure that structural edits (splits/merges) are coordinated with the CRDT layer to prevent character loss during concurrent edits.

## 3. Technology Stack

- **Core:** Rust (Workspace)
- **Editor:** Vanilla JS / Vite (minimize framework overhead)
- **Layout:** Pure Rust (`rustybuzz`, `ttf-parser`, `fontdb`)
- **WASM:** `wasm-bindgen`
- **Server:** Rust (Axio/Tokio)

## 4. Documentation

- Maintain the `docs/improvement-plan/` trackers as the source of truth for project maturity.
- Update `CHANGELOG.md` for every significant architectural shift.
