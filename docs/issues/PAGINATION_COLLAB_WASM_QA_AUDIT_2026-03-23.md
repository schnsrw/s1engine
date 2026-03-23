# Rudra Office QA Audit — Pagination, Co-Editing, WASM, Server

**Date:** 2026-03-23  
**Auditor profile:** QA review framed from a senior editor/document-product perspective, with emphasis on user experience, layout fidelity, co-editing correctness, and WASM ownership of document behavior.  
**Scope reviewed:** `editor/src`, `ffi/wasm/src`, `server/src`, `crates/s1-crdt`, and the existing rendering/collaboration specifications.

---

## Executive Summary

The repo has strong ambition around WASM-driven pagination and collaboration, but the current implementation still has several UX-critical gaps where the browser DOM, CSS clipping, and coarse paragraph replacement are doing work that should be owned by the WASM/layout/CRDT layers.

The biggest user-facing problem areas are:

1. **Page breaking for long paragraphs is not fully WASM-owned in the editor UI.** Split paragraphs are rendered from full HTML and then expected to be visually clipped in JS/CSS, which is exactly the class of behavior that produces "whole paragraph jumps" instead of stable line-by-line flow.
2. **Typing sync still force-sets whole paragraph text in the editor path.** That makes fidelity fragile for mixed formatting and also pushes collaboration toward paragraph-level reconciliation instead of character/range-level intent.
3. **Reconnect and catch-up behavior has protocol mismatches between the editor and the server.** This can leave collaborators stale or cause duplicated replay noise.
4. **The server's authoritative snapshot model is incomplete.** New joiners can be brought into a stale state unless a peer recently emitted a `fullSync`.

If the product goal is:
- "WASM decides page breaks"
- "line-by-line page carry, not full paragraph jumping"
- "co-editing must stay consistent"
- "tree must stay in sync"

then the current implementation still needs another hardening pass before the experience will feel production-grade.

---

## Priority Summary

| ID | Severity | Area | Short title |
|---|---|---|---|
| QA-01 | Critical | Pagination / UX | Paragraph pagination still depends on DOM/CSS splitting behavior instead of a pure WASM fragment model |
| QA-02 | Critical | Pagination / Editing | Split-paragraph continuation fragments are not actually finalized/applied in the render pipeline |
| QA-03 | Critical | Fidelity / Editing | Typing still uses `set_paragraph_text()` in the editor path, risking run-format collapse and paragraph-wide reflow |
| QA-04 | High | Collaboration / Server | Reconnect catch-up path is protocol-mismatched (`sync-req` from client, unsupported on server) |
| QA-05 | High | Collaboration / Server | `requestCatchup` replays are broadcast to the full room instead of only the requesting peer |
| QA-06 | High | Collaboration / Session state | Session/editor access state drifts because the client sends `access` while the server tracks `mode` |
| QA-07 | High | Collaboration / Fresh join UX | Server snapshot freshness depends on `fullSync`; there is no active periodic authoritative snapshot update |
| QA-08 | Medium | Presence / UX | Existing peers receive blank color payloads on join, which weakens stable collaborator identity cues |
| QA-09 | Medium | Architecture / Consistency | The codebase still contains a mixed ownership model: WASM layout is authoritative on paper, but editor behavior still leaks into DOM/CSS heuristics |

---

## Detailed Findings

### QA-01 — Paragraph pagination still depends on DOM/CSS splitting behavior instead of a pure WASM fragment model
**Severity:** Critical  
**Area:** Pagination, UX, fidelity

**Why this matters**
Users experience pagination correctness through typing, scrolling, selection, and co-editing. If a paragraph is rendered as a whole DOM block and then visually clipped, the UI is still paragraph-centric even when the page map says the paragraph is split. That is the exact pattern that causes page shifts to feel "all at once" rather than line-by-line.

**Evidence found**
- The editor builds split paragraph parts by calling `doc.render_node_html(originalId)` and then marking DOM nodes for split handling instead of requesting explicit WASM-produced fragment HTML for each page fragment.
- The pagination module contains `domBasedOverflowSplit()` and `applySplitParagraphClipping()`, both of which rely on CSS overflow, max-height, wrappers, and negative margins to simulate page fragments.
- Those mechanisms are DOM/CSS fallbacks, not true layout fragments owned by the engine.

**User impact**
- Long paragraphs can appear to jump page-to-page in a non-natural way.
- Cursoring and selection inside split content become fragile.
- Remote edits near a page boundary are more likely to feel visually unstable.
- The implementation contradicts the product direction that page breaking should be determined by WASM, not CSS/HTML tricks.

**Recommendation**
- Move split paragraph rendering to a **WASM fragment API** such as page-fragment HTML or page-fragment layout JSON per node/page slice.
- Make the editor mount exactly the fragment that belongs to page N, not a whole paragraph that is later clipped.
- Treat paragraph fragments like first-class layout objects in the same way table row chunks are treated.

---

### QA-02 — Split-paragraph continuation fragments are not actually finalized/applied in the render pipeline
**Severity:** Critical  
**Area:** Pagination, editing, consistency

**Why this matters**
The current implementation appears to prepare split metadata, but the finishing functions that would visually clip or wrap those fragments are not called from the active repagination/render flow. That means the system is carrying split state without fully enforcing it.

**Evidence found**
- `pagination.js` defines `domBasedOverflowSplit()` and `applySplitParagraphClipping()`.
- Repository-wide search shows these helpers are defined but not invoked by the active rendering path.
- The split paragraph insertion path only marks nodes with `data-split-first` / `data-split-continuation` metadata.

**User impact**
- Continuation blocks can render as full paragraphs rather than true page fragments.
- Duplicate text may appear across pages in edge cases.
- Editing behavior may diverge depending on whether the user interacts with the first fragment or the continuation fragment.
- This is especially risky under co-editing because different peers may re-render around the same split boundary at different times.

**Recommendation**
- Either remove the dead/incomplete CSS-splitting path entirely and replace it with fully WASM-rendered fragments, or explicitly wire the finishing logic into repagination with tests.
- Add E2E cases for: long paragraph typing at page boundary, backspace/merge across boundary, and remote insert exactly at split line.

---

### QA-03 — Typing still uses `set_paragraph_text()` in the editor path, risking run-format collapse and paragraph-wide reflow
**Severity:** Critical  
**Area:** Fidelity, editing, collaboration correctness

**Why this matters**
A professional document editor must preserve run formatting while the user types inside mixed-format content. The current editor sync path still updates a paragraph by reading the whole DOM text and calling `doc.set_paragraph_text(nodeId, newText)`.

**Evidence found**
- `editor/src/render.js` syncs paragraph edits through `doc.set_paragraph_text(nodeId, newText)`.
- The WASM binding documentation explicitly warns that `set_paragraph_text()` collapses inline formatting when an edit spans multiple runs and recommends using range-aware operations such as `insert_text_in_paragraph`, `delete_text_in_paragraph`, `format_selection`, and `replace_text` for editor-driven edits.

**User impact**
- Mixed formatting can be lost or normalized unexpectedly after typing inside bold/italic/link-rich content.
- A small text change may trigger paragraph-wide logical mutation and thus page reflow that feels heavier than necessary.
- Collaboration semantics stay paragraph-oriented instead of range-oriented, increasing the chance of visual jumps and harder-to-merge intent.

**Recommendation**
- Replace the main editor typing pipeline with character/range aware WASM mutations.
- Reserve `set_paragraph_text()` for convergence/fallback only.
- Add regression tests for editing inside multi-run paragraphs with links, comments, footnotes, and tracked changes.

---

### QA-04 — Reconnect catch-up path is protocol-mismatched (`sync-req` from client, unsupported on server)
**Severity:** High  
**Area:** Collaboration, server, resiliency

**Why this matters**
If reconnect logic is not protocol-compatible, the editor can think it requested recovery while the server silently ignores it.

**Evidence found**
- On reconnect, the client sends `{ type: 'sync-req', ... }`.
- Server-side validation does not whitelist `sync-req`.
- The WebSocket handler has no `sync-req` branch.
- The documented/implemented catch-up branch on the server is `requestCatchup`.

**User impact**
- After reconnect, a peer can remain stale until another event happens to force convergence.
- Offline-buffer replay can happen without first reconciling missed remote operations.
- Users may think co-editing is live even when they are temporarily out of sync.

**Recommendation**
- Unify on a single reconnect/catch-up message (`requestCatchup` or equivalent) across docs, client, and server.
- Add protocol contract tests for reconnect, version gap, and offline replay ordering.

---

### QA-05 — `requestCatchup` replays are broadcast to the full room instead of only the requesting peer
**Severity:** High  
**Area:** Collaboration, correctness, bandwidth

**Why this matters**
Catch-up data is recovery traffic for one peer. Broadcasting it to everyone introduces unnecessary traffic and duplicate op delivery noise.

**Evidence found**
- In the server `requestCatchup` branch, catch-up ops are emitted through the room broadcast channel rather than being written directly to the requesting socket.

**User impact**
- Non-stale peers can receive duplicate history traffic.
- Clients must spend extra effort filtering or ignoring redundant ops.
- Large rooms pay a bandwidth/performance penalty for one peer’s recovery.

**Recommendation**
- Send catch-up ops directly to the requesting socket.
- Keep room broadcast for actual new operations only.
- Add a multi-peer test: one stale peer reconnects while two healthy peers remain active.

---

### QA-06 — Session/editor access state drifts because the client sends `access` while the server tracks `mode`
**Severity:** High  
**Area:** Collaboration, admin/session UX

**Why this matters**
Access state must be consistent everywhere: the editor UI, relay auth, admin/session views, and any integration callbacks.

**Evidence found**
- The client connection URL appends `access=...`.
- The server stores file session editor state using `params.mode` when `editor_join()` is called.
- `mode` and `access` are separate query parameters, and the client is not reliably sending both.

**User impact**
- View/comment users can be tracked as editors in session state.
- Admin tooling and presence UI can misrepresent who has edit rights.
- This creates avoidable confusion in co-editing, auditing, and host-product integrations.

**Recommendation**
- Collapse to one canonical permission field for the WebSocket/session contract.
- Validate and persist exactly the same field server-side that the client uses to enforce UI permissions.

---

### QA-07 — Server snapshot freshness depends on `fullSync`; there is no active periodic authoritative snapshot update
**Severity:** High  
**Area:** Collaboration, server, fresh join experience

**Why this matters**
A new peer should join from a reliable authoritative snapshot, not from whichever client most recently happened to send a coarse sync event.

**Evidence found**
- File sessions advertise a snapshot model and define a snapshot interval constant.
- In the reviewed path, session snapshots are updated when a `fullSync` payload is received.
- The snapshot interval constant is not actively used in the collaboration path that was reviewed.

**User impact**
- A newly joined collaborator can open an outdated snapshot if the room has had many incremental edits but no recent `fullSync`.
- The user may briefly see stale content and then a burst of catch-up operations.
- That hurts trust, especially for shared docs opened from links.

**Recommendation**
- Make the server maintain a truly authoritative current state or a guaranteed-fresh snapshot cadence.
- If the server intentionally relies on peer snapshots, document the staleness window and expose a freshness timestamp.

---

### QA-08 — Existing peers receive blank color payloads on join, weakening stable collaborator identity cues
**Severity:** Medium  
**Area:** Presence, UX

**Why this matters**
In co-editing, stable visual identity is a major UX aid. Even a short period where peers appear with empty/default colors can make cursor/presence attribution feel unreliable.

**Evidence found**
- The server initially broadcasts `peer-join` with `userColor: ""` and only later updates stored peer color after the client's `join` message.

**User impact**
- Existing collaborators can see peers appear with incorrect or empty color identity.
- Cursor color, avatar color, and presence list color can flicker or mismatch.

**Recommendation**
- Delay the join broadcast until user color is known, or let the connection URL include the initial color if that is acceptable.

---

### QA-09 — Mixed ownership remains: WASM layout is authoritative in the docs, but editor behavior still leaks into DOM/CSS heuristics
**Severity:** Medium  
**Area:** Architecture consistency

**Why this matters**
The product direction is correct: the engine/tree/WASM layer should own document truth. The implementation still shows a mixed model where specs say one thing, but editor behavior falls back to DOM heuristics in critical boundary cases.

**Evidence found**
- Specs describe WASM page maps as the source of truth.
- The editor still contains DOM/CSS overflow splitting helpers and coarse paragraph sync paths.
- Collaboration also has a mixed model: CRDT text editing exists, but structural convergence and fallback behavior still rely heavily on whole-document replacement.

**User impact**
- Harder to reason about correctness.
- More edge cases where local DOM state and model state drift apart.
- Higher probability of page-jump, cursor-loss, and stale-peer bugs reappearing in future work.

**Recommendation**
- Use a stricter architectural rule: if a behavior affects page boundaries, structural fragments, or shared state, it must originate from WASM/CRDT data rather than DOM inference.
- Remove legacy/dead fallback paths once replacement APIs are ready.

---

## Recommended Next Actions

### Immediate (must-fix before claiming stable pagination/co-editing)
1. **Design and implement a WASM page-fragment API** for split paragraphs so each page receives only its own fragment.
2. **Replace editor typing sync with range-aware operations** instead of paragraph force-set.
3. **Fix the reconnect protocol mismatch** and add automated server/client protocol tests.
4. **Send catch-up traffic only to the requesting peer**.
5. **Unify `access` / `mode` semantics** across URL params, session state, and UI enforcement.

### Short-term hardening
6. Add E2E scenarios for:
   - typing at the last line of a page,
   - long-paragraph growth causing page carry,
   - deleting at the top of a continuation page,
   - two peers editing on opposite sides of a page boundary,
   - reload/reconnect while other peers continue editing.
7. Add a debug UI or logs that show: page fragment source, page break owner (WASM vs fallback), current serverVersion, and last snapshot freshness.

### Product-level recommendation
8. Stop describing the pagination/co-editing path as fully production-ready until the paragraph-fragment path is genuinely WASM-owned end-to-end.

---

## Final Assessment

The repo is promising, but **the exact areas the user called out — pagination while loading, paragraph carry between pages, and co-editing consistency — are still the highest-risk parts of the implementation**.

The core theme of this audit is simple:

> **The system will feel stable only when WASM owns fragments, page boundaries, and edit intent more directly than the DOM does.**

Right now, the project is close in concept, but not yet fully aligned in implementation.
