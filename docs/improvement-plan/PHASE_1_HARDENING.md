# Phase 1: Hardening (Stability & Fidelity)

## Goal
Eliminate data loss, prevent formatting collapse during typing, and stabilize the collaboration handshake. Move the editor from "coarse sync" to "atomic sync."

## Key Objectives

### 1. Range-Aware Editing (CRDT Integration)
The current editor uses a "read-all, write-all" approach for paragraphs. This is the #1 cause of lost formatting (bold/italic/links) and cursor jumps.
- **Action:** Rewrite the `input.js` and `render.js` sync path to use `doc.insert_text_in_paragraph(nodeId, offset, text)` and `doc.delete_text_in_paragraph(nodeId, offset, length)`.
- **Validation:** Type inside a paragraph with mixed bold and italic text; verify no formatting is lost.

### 2. Protocol Alignment
The client and server currently speak slightly different languages for reconnection.
- **Action:** Standardize on `requestCatchup` for all recovery.
- **Action:** Ensure the server responds with a version-aware `joined` message that includes the latest `serverVersion`.

### 3. Collaboration Resiliency
- **Unicast Recovery:** Ensure that when a peer requests missed ops, only that peer receives the replay.
- **Profile Validation:** Ensure `userName` and `userColor` are persisted correctly on the server to prevent "ghost" cursors or color flickering.

### 4. Cursor & Re-render Stability (H-06, H-07)
The editor currently re-renders the full document on many operations, which resets cursor position and scroll.
- **Action:** Save cursor position (nodeId + offset) and scroll position before every re-render, restore after.
- **Action:** For single-paragraph edits, use `renderNodeById()` instead of `renderDocument()`. Only fall through to full re-render when structural changes affect page layout.
- **Validation:** Type in a 50-page document — cursor should never jump, scroll should never shift.

### 5. Pagination Fragment Wiring (H-08)
`applySplitParagraphClipping()` exists in `pagination.js` but is never called from the active render path.
- **Action:** Call it at the end of `repaginate()` so split paragraphs are visually clipped to their page slice.
- **Validation:** Long paragraph spanning two pages renders correctly without duplicate text.

### 6. Missing Co-editing Op Handlers (H-09, H-10)
15+ operations are broadcast by the editor but have no handler on the receiving side, causing silent failures.
- **Action:** Add remote op handlers for: `insertColumnBreak`, `insertCommentReply`, `setImageWrapMode`, `insertFootnote`, `insertEndnote`, `mergeCells`, `splitCell`, `setParagraphSpacing`, `setParagraphKeep`, `setPageSetup`, `setSectionColumns`, `setTabStops`, `insertEquation`.
- **Action:** Add `normalizeRemoteOp()` to translate field name mismatches between peers (`removeNode`→`deleteNode`, `tableNodeId`→`tableId`, `rowIndex`→`index`).
- **Note:** The `codex/identify-issues-in-co-editing-experience` branch already implements all of these.

### 7. Regression Suite (H-05)
- Create Playwright tests that simulate high-latency connections.
- Test "Concurrent Typing" where two users type in the same paragraph simultaneously.
- Test typing at the last line of a page (page carry behavior).
- Test backspace at the top of a continuation page (merge across boundary).
- Test reconnect while other peers continue editing.
