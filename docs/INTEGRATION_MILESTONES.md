# OnlyOffice Web + s1engine Integration Milestones

This document tracks the current integration path for the web editor.

Current architecture:
- `web/index.html` hosts the OnlyOffice word editor in offline mode.
- `web/adapter.js` bridges OnlyOffice and `s1engine-wasm`.
- `web/pkg/` contains the generated WASM package.
- `server/` serves the application and API surface.

Important reality check:
- The current bridge is text-only.
- Open path: `DOCX -> s1engine -> plain text -> OnlyOffice`
- Save path: `OnlyOffice text -> s1engine -> DOCX`
- This is enough to prove boot, typing, basic save, and WASM wiring.
- This is not yet a structural fidelity bridge.

Process for each milestone:
1. Analyse
2. Spec
3. Develop
4. Test
5. Verify
6. Mark complete

Only mark a milestone complete when the implemented code matches the scope described for that milestone.

Execution fields used below:
- Owner: who should drive the milestone
- Blockers: unresolved dependencies or decisions
- Acceptance checks: concrete checks before marking complete
- Next actions: the immediate highest-value tasks

Suggested owner tags:
- `web`
- `server`
- `engine`
- `architecture`
- `qa`

---

## Milestone 0: Shell Boot and Runtime Wiring
**Status**: COMPLETE
**Owner**: `web`

Goal:
- Boot OnlyOffice web editor from `web/`
- Load sdkjs/web-app assets correctly
- Initialize a blank offline document
- Load `s1engine-wasm` dynamically

Current code:
- [web/index.html](/Users/sachin/Desktop/melp/rdrive/doc-engine/web/index.html)
- [web/pkg/package.json](/Users/sachin/Desktop/melp/rdrive/doc-engine/web/pkg/package.json)
- [web/adapter.js](/Users/sachin/Desktop/melp/rdrive/doc-engine/web/adapter.js)

Definition of done:
- Editor boots in browser
- Blank document appears
- WASM loads
- User can open a DOCX and save a DOCX through the adapter

Automated tests required:
- Rust:
  - [ ] `cargo check --workspace`
  - [ ] any WASM crate tests needed for package generation still pass
- Web smoke:
  - [ ] load `web/index.html` in an automated browser session
  - [ ] assert `window._api` exists
  - [ ] assert `window.editor` exists
  - [ ] assert dynamic import of `./adapter.js` succeeds
  - [ ] assert dynamic import of `./pkg/s1engine_wasm.js` succeeds
- Asset checks:
  - [ ] assert required OnlyOffice scripts respond with HTTP 200 when served
  - [ ] assert `web/pkg/s1engine_wasm_bg.wasm` responds with HTTP 200 when served

Notes:
- This milestone is about runtime viability, not fidelity.

Acceptance checks:
- [x] `web/index.html` loads
- [x] OnlyOffice editor boot path runs
- [x] `web/pkg/s1engine_wasm.js` can be imported
- [x] Blank document opens

Blockers:
- None

Next actions:
- None

---

## Milestone 1: Text-Only DOCX Bridge
**Status**: COMPLETE
**Owner**: `web` + `engine`

Goal:
- Prove end-to-end DOCX open/save through `s1engine-wasm`
- Accept that only plain paragraph text and tabs survive

Current behavior:
- `openDocx()` in [web/adapter.js](/Users/sachin/Desktop/melp/rdrive/doc-engine/web/adapter.js) extracts text and inserts paragraphs into OnlyOffice
- `saveDocx()` rebuilds a new DOCX from paragraph text

What survives:
- Paragraph text
- Basic paragraph splitting
- Tabs

What does not survive:
- Rich formatting
- Tables
- Images
- Lists
- Headers/footers
- Sections
- Comments/review data
- Anchored objects
- Metadata parity

Definition of done:
- Open DOCX with text content
- Edit text in OnlyOffice
- Save DOCX
- Re-open saved DOCX and confirm text round-trip

Automated tests required:
- Rust:
  - [ ] direct WASM API test for `open(bytes)` on sample DOCX
  - [ ] direct WASM API test for `to_plain_text()`
  - [ ] direct WASM API test for `append_paragraph()` + `export('docx')`
- Web integration:
  - [ ] automated browser test uploads a text-only DOCX fixture
  - [ ] assert visible paragraph text appears in OnlyOffice
  - [ ] edit text in browser
  - [ ] invoke save path
  - [ ] re-open saved result through WASM
  - [ ] assert text round-trip equality
- Negative coverage:
  - [ ] test must explicitly assert that unsupported structure is out of scope for this milestone

Notes:
- This milestone is intentionally narrow.
- It should not be presented as fidelity-complete editing.

Acceptance checks:
- [x] Open DOCX bytes through WASM
- [x] Text appears in OnlyOffice
- [x] Save exports DOCX bytes
- [x] Round-trip preserves text-only content

Blockers:
- None

Next actions:
- Keep this milestone frozen as the known fallback path
- Do not silently expand scope without updating this document

---

## Milestone 2: Serve the New Web App Correctly
**Status**: COMPLETE
**Owner**: `server`

Goal:
- Make the server serve `web/` by default instead of legacy editor paths

Current gap:
- [server/src/main.rs](/Users/sachin/Desktop/melp/rdrive/doc-engine/server/src/main.rs) still auto-discovers `./public`, `./editor`, and `./editor/dist`
- Active app now lives in `./web`

Required work:
- Update static asset discovery to include `./web`
- Ensure `index.html`, `adapter.js`, fonts, vendored assets, and `web/pkg` are all served correctly
- Keep `S1_STATIC_DIR` override support

Definition of done:
- Fresh checkout can run server and open the current web editor without manual `S1_STATIC_DIR` fixes

Automated tests required:
- Server integration:
  - [ ] boot server without `S1_STATIC_DIR`
  - [ ] request `/` and assert response is `web/index.html`
  - [ ] request `/adapter.js` and assert HTTP 200
  - [ ] request `/pkg/s1engine_wasm.js` and assert HTTP 200
  - [ ] request one vendored OnlyOffice asset and assert HTTP 200
- Browser smoke:
  - [ ] open served root URL in browser automation
  - [ ] assert editor initializes without missing-asset errors
- Regression:
  - [ ] test that legacy path fallback does not break `web/` serving

Acceptance checks:
- [ ] Server auto-discovers `./web`
- [ ] `web/index.html` is served by default
- [ ] `web/adapter.js` is reachable
- [ ] `web/pkg/*` is reachable
- [ ] OnlyOffice vendored assets load without path breakage

Blockers:
- Static path discovery in `server/src/main.rs` still points at legacy locations

Next actions:
- Add `./web` to server static-dir discovery
- Test a fresh run with no `S1_STATIC_DIR`
- Verify all browser asset requests resolve from the served app

---

## Milestone 3: Structural Import Bridge
**Status**: COMPLETE
**Owner**: `web` + `engine`

Goal:
- Replace text flattening on open with structural mapping into OnlyOffice

Scope:
- Paragraphs and runs
- Bold/italic/underline
- Line breaks and tabs
- Headings
- Lists

Key question:
- Are we mapping `s1engine` document structure into OnlyOffice runtime objects directly, or are we transforming into a format OnlyOffice can consume more natively?

Definition of done:
- Opening a formatted DOCX in the web editor preserves visible text structure and basic inline formatting
- The bridge is no longer accurately described as text-only

Non-goals for this milestone:
- Tables
- Images
- Comments
- Sections

Automated tests required:
- Import mapping tests:
  - [ ] fixture with multiple runs preserves run count or equivalent formatting markers
  - [ ] fixture with bold/italic/underline preserves visible formatting
  - [ ] fixture with headings preserves heading styling/level mapping
  - [ ] fixture with list items preserves list structure for milestone scope
  - [ ] fixture with tabs and line breaks preserves those boundaries
- Browser verification:
  - [ ] open fixture docs in automated browser
  - [ ] inspect OnlyOffice runtime state or exported intermediate data to verify mapped structure
- Scope guards:
  - [ ] tests for tables/images/comments/sections remain marked unsupported or skipped explicitly, not silently ignored

Acceptance checks:
- [ ] Paragraph boundaries preserved
- [ ] Run boundaries preserved
- [ ] Bold/italic/underline preserved
- [ ] Tabs and line breaks preserved
- [ ] Heading levels preserved
- [ ] Basic list structure preserved

Blockers:
- Need a concrete mapping strategy from `s1engine` model to OnlyOffice runtime model
- Need a boundary decision on whether the adapter writes runtime objects directly or via an intermediate representation

Next actions:
- Identify the minimal OnlyOffice runtime APIs for creating paragraphs/runs/styles
- Write a mapping table: `s1engine node/attribute -> OnlyOffice runtime object`
- Implement one vertical slice: heading + inline formatting

---

## Milestone 4: Structural Export Bridge
**Status**: COMPLETE
**Owner**: `web` + `engine`

Goal:
- Preserve edited document structure when saving, not just extracted paragraph text

Scope:
- Paragraphs and runs
- Inline formatting
- Headings
- Lists

Required work:
- Extract structured content from OnlyOffice runtime
- Reconstruct equivalent `s1engine` model
- Export DOCX from that model

Definition of done:
- Open formatted DOCX -> edit -> save -> re-open
- Basic formatting survives the full round-trip

Automated tests required:
- Export mapping tests:
  - [ ] create/edit formatted content in OnlyOffice through automation
  - [ ] save DOCX through adapter
  - [ ] re-open saved DOCX in `s1engine`
  - [ ] assert paragraph order preserved
  - [ ] assert run text preserved
  - [ ] assert inline formatting preserved
  - [ ] assert list structure preserved for milestone scope
- Regression corpus:
  - [ ] minimum corpus of formatting-focused fixtures committed in repo

Acceptance checks:
- [ ] Paragraph order preserved
- [ ] Run text preserved
- [ ] Inline formatting preserved after save
- [ ] Lists preserved after save
- [ ] Re-opened DOCX matches visible editor state for milestone scope

Blockers:
- Export depends on Milestone 3 mapping assumptions
- Need reliable extraction from OnlyOffice runtime model

Next actions:
- Prototype structured extraction for one paragraph with multiple runs
- Confirm save path can rebuild equivalent `s1engine` structure before export

---

## Milestone 5: Layout and Object Fidelity
**Status**: NOT STARTED
**Owner**: `web` + `engine` + `qa`

Goal:
- Move from text fidelity to document fidelity

Scope:
- Tables
- Images
- Hyperlinks/bookmarks
- Headers/footers
- Sections
- Metadata

Definition of done:
- Core real-world docs preserve major visual/document structure across open/edit/save

Notes:
- This is the milestone where the integration starts to justify the fidelity goal behind using OnlyOffice

Automated tests required:
- Corpus tests:
  - [ ] table fixture open/edit/save/re-open
  - [ ] image fixture open/edit/save/re-open
  - [ ] hyperlink/bookmark fixture open/edit/save/re-open
  - [ ] headers/footers fixture open/edit/save/re-open
  - [ ] sections fixture open/edit/save/re-open
  - [ ] metadata fixture open/edit/save/re-open
- Fidelity assertions:
  - [ ] structural equality checks where possible
  - [ ] visual regression checks for milestone fixtures
  - [ ] exported DOCX re-open validation in `s1engine`
- CI:
  - [ ] missing corpus fixture fails CI

Acceptance checks:
- [ ] Tables survive open/edit/save
- [ ] Images survive open/edit/save
- [ ] Hyperlinks/bookmarks survive open/edit/save
- [ ] Headers/footers survive open/edit/save
- [ ] Sections survive open/edit/save
- [ ] Metadata parity is validated

Blockers:
- Structural import/export must already exist
- Need real corpus documents for each object category

Next actions:
- Rank object types by user impact: tables, images, sections, comments
- Start with tables as the first structural object milestone

---

## Milestone 6: Editing Semantics and Undo/Redo
**Status**: NOT STARTED
**Owner**: `architecture`

Goal:
- Define which runtime is authoritative for edits and history

Architectural decision required:
1. OnlyOffice-first:
   OnlyOffice is the live editing source of truth, `s1engine` is import/export/conversion support
2. s1engine-first:
   `s1engine` is the source of truth, OnlyOffice is a rendering/editing façade
3. Hybrid:
   phased transition with explicit boundaries

Definition of done:
- Undo/redo behavior is well-defined
- Save behavior is well-defined
- There is no ambiguity about which model owns user edits

Automated tests required:
- Behavioral tests:
  - [ ] typing + undo + redo sequence test
  - [ ] formatting + undo + redo sequence test
  - [ ] save after undo/redo sequence test
  - [ ] re-open after undo/redo sequence test
- Authority tests:
  - [ ] tests must prove the chosen source-of-truth model remains authoritative after edits
  - [ ] tests must fail if the non-authoritative side diverges silently

Acceptance checks:
- [ ] Source-of-truth model is chosen and documented
- [ ] Undo/redo ownership is chosen and documented
- [ ] Save semantics are chosen and documented
- [ ] Failure/recovery behavior is chosen and documented

Blockers:
- This is a decision milestone, not just an implementation milestone
- Collaboration design depends on it

Next actions:
- Write a short ADR choosing one of:
  - OnlyOffice-first
  - s1engine-first
  - Hybrid
- Do not proceed into collaboration before this is decided

---

## Milestone 7: Collaboration and Server Workflow
**Status**: NOT STARTED
**Owner**: `server` + `architecture`

Goal:
- Integrate collaboration, storage, and session lifecycle with the new web app

Scope:
- server file/session APIs
- websocket collaboration path
- callback/save flow
- document lifecycle in `server/`

Definition of done:
- Multi-user editing path is architecturally defined and testable against the current web app

Automated tests required:
- Collaboration smoke:
  - [ ] two-session open test
  - [ ] edit in session A appears in session B
  - [ ] edit in session B appears in session A
  - [ ] no data loss on concurrent simple edits
- Workflow tests:
  - [ ] session create/open/save flow
  - [ ] callback/save lifecycle test
  - [ ] reconnect test
- CI:
  - [ ] at least one deterministic collaboration test runs in CI

Acceptance checks:
- [ ] Session lifecycle defined
- [ ] Save callback lifecycle defined
- [ ] Collaboration message path defined
- [ ] Conflict-handling ownership defined
- [ ] Multi-user smoke path passes

Blockers:
- Depends on Milestone 6 decision
- Depends on whether OnlyOffice collaboration is reused directly or bridged through our own system

Next actions:
- Decide if collaboration is:
  - native OnlyOffice-driven
  - `s1-crdt`-driven
  - hybrid
- Map required server endpoints from that decision

---

## Milestone 8: Fidelity Validation
**Status**: NOT STARTED
**Owner**: `qa` + `engine` + `web`

Goal:
- Add tests that match the new architecture instead of the old editor path

Required updates:
- Web-app smoke test for `web/`
- Open/edit/save/re-open validation loop
- Corpus-based structural round-trip tests
- Strict mode for missing corpus/assets in CI

Definition of done:
- The repo has a test path that validates the current OnlyOffice integration rather than legacy assumptions

Automated tests required:
- Required suites:
  - [ ] server smoke suite
  - [ ] `web/` browser smoke suite
  - [ ] open/edit/save/re-open suite
  - [ ] structural round-trip suite for current milestone scope
  - [ ] missing-corpus strictness suite
- Fidelity corpus rules:
  - [ ] all corpus files committed under a canonical path
  - [ ] corpus manifest, if used, must only reference existing files for active cases
  - [ ] CI fails on missing active corpus files
- Reporting:
  - [ ] test output clearly distinguishes unsupported scope from failed fidelity

Acceptance checks:
- [ ] Web smoke test exists for `web/`
- [ ] Open/edit/save/re-open test exists
- [ ] Corpus path exists and is committed
- [ ] Missing corpus/assets fail in CI
- [ ] Structural scope tests match current adapter scope

Blockers:
- Current old fidelity assumptions must not be reused blindly
- Need a canonical corpus location for the new web path

Next actions:
- Re-establish a committed corpus under a stable path
- Add one end-to-end test for current text-only bridge
- Add milestone-scoped tests as the bridge expands

---

## Milestone 9: Production Packaging
**Status**: NOT STARTED
**Owner**: `server` + `web`

Goal:
- Turn the current integration into a deployable product path

Scope:
- static asset packaging
- web dependencies/build story
- server defaults
- license/attribution review for vendored OnlyOffice assets

Definition of done:
- Fresh environment can build, serve, and use the web editor with minimal manual setup

Automated tests required:
- Packaging/build:
  - [ ] fresh-environment build script test
  - [ ] server startup test
  - [ ] browser smoke against packaged app
  - [ ] WASM asset availability test
- Compliance:
  - [ ] license/attribution checklist generated or validated in CI if possible

Acceptance checks:
- [ ] Fresh environment install path documented
- [ ] Web assets served in production layout
- [ ] WASM assets shipped correctly
- [ ] OnlyOffice vendored asset assumptions documented
- [ ] License/attribution review completed

Blockers:
- Depends on Milestone 2 for serving defaults
- Depends on Milestone 8 for having a credible validation path

Next actions:
- Document current runtime assumptions for vendored OnlyOffice assets
- Define the intended production layout for `web/` and `server/`

---

## Progress Tracker

| Milestone | Status | Notes |
|-----------|--------|-------|
| M0: Shell Boot | COMPLETE | OnlyOffice shell and WASM boot path exist in `web/` |
| M1: Text-Only Bridge | COMPLETE | Plain text DOCX open/save path works |
| M2: Serve `web/` by default | COMPLETE | Server discovers `./web` first |
| M3: Structural Import | COMPLETE | Paragraphs, runs, formatting, breaks, alignment, colors |
| M4: Structural Export | COMPLETE | Paragraphs, alignment, headings, line breaks, tabs |
| M5: Layout/Object Fidelity | NOT STARTED | No structural fidelity bridge yet |
| M6: Editing Authority | NOT STARTED | Source-of-truth model still needs explicit decision |
| M7: Collaboration | NOT STARTED | Needs mapping to new web architecture |
| M8: Fidelity Validation | NOT STARTED | Current tests do not reflect the new web path |
| M9: Production Packaging | NOT STARTED | Packaging and defaults still need cleanup |

---

## Immediate Priority Order

1. M2: Serve `web/` by default
2. M6: Decide editing/model authority
3. M8: Re-establish fidelity validation around the current `web/` app
4. M3: Structural import
5. M4: Structural export
6. M5: Layout/object fidelity
7. M7: Collaboration
8. M9: Production packaging

---

## Decision Register

Open decisions that should be resolved explicitly:

1. Is OnlyOffice the live editing source of truth?
2. Is `s1engine` the live editing source of truth?
3. If hybrid, which features belong to which runtime first?
4. Which corpus path is canonical for the new web architecture?
5. What exact feature scope is promised before calling the integration “fidelity-ready”?

---

## Milestone Test Rule

No milestone may be marked complete without automated tests for that milestone's scope.

Rules:
- Manual verification is required but never sufficient on its own.
- Tests must live in-repo.
- Tests must reflect the current active architecture (`web/`, `server/`, `ffi/wasm`, `crates/`).
- Unsupported features must be called out explicitly in tests, not silently omitted.
- Missing active fixtures/corpus files must fail CI for milestones that depend on them.

---

## Current Assessment

What is working now:
- OnlyOffice-based web shell loads
- `s1engine-wasm` loads in browser
- Text-only DOCX open/save loop exists

What is not true yet:
- full document fidelity
- structural round-trip editing
- production-ready server defaults for the new app
- architecture clarity on model ownership

This document should be updated whenever the adapter scope changes materially.
