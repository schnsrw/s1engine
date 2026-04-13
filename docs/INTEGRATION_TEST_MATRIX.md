# OnlyOffice Web + s1engine Integration Test Matrix

This document translates milestone goals into concrete automated test work.

Use this together with:
- [INTEGRATION_MILESTONES.md](/Users/sachin/Desktop/melp/rdrive/doc-engine/docs/INTEGRATION_MILESTONES.md)
- [ROADMAP.md](/Users/sachin/Desktop/melp/rdrive/doc-engine/docs/ROADMAP.md)

Rules:
- Every milestone must have in-repo automated tests before it is marked complete.
- Tests must target the active architecture:
  - `web/`
  - `server/`
  - `ffi/wasm/`
  - `crates/`
- Tests must distinguish:
  - supported and passing
  - supported and failing
  - explicitly unsupported
- Missing required fixtures for active milestones must fail CI.

---

## Canonical Test Areas

### 1. Rust Engine Tests
Purpose:
- validate model, import/export, layout, and WASM-facing engine behavior

Likely locations:
- `crates/s1engine/tests/`
- `ffi/wasm/tests/` or `ffi/wasm/examples/` promoted into tests where appropriate

### 2. Server Integration Tests
Purpose:
- validate static serving, API behavior, session lifecycle, and save/callback flows

Likely locations:
- `server/tests/`

### 3. Web Browser Integration Tests
Purpose:
- validate the `web/` app boots, loads assets, opens documents, edits, saves, and reopens correctly

Likely locations:
- `web/tests/`

### 4. Fidelity Corpus
Purpose:
- provide stable fixtures for structural and visual regression testing

Recommended canonical location:
- `tests/integration-corpus/`

Recommended structure:
- `tests/integration-corpus/text/`
- `tests/integration-corpus/formatting/`
- `tests/integration-corpus/lists/`
- `tests/integration-corpus/tables/`
- `tests/integration-corpus/images/`
- `tests/integration-corpus/sections/`
- `tests/integration-corpus/headers-footers/`
- `tests/integration-corpus/real-world/`

---

## Milestone-by-Milestone Matrix

## M0: Shell Boot and Runtime Wiring

Goal:
- prove the web shell boots and the runtime assets resolve

Tests to add:
- `server/tests/static_web_serving.rs`
- `web/tests/smoke_boot.spec.(js|ts)`
- optional: `ffi/wasm/tests/pkg_load.rs` if useful

Required checks:
- `/` serves `web/index.html`
- `/adapter.js` returns 200
- `/pkg/s1engine_wasm.js` returns 200
- `/pkg/s1engine_wasm_bg.wasm` returns 200
- one critical OnlyOffice asset returns 200
- browser can load the app without fatal boot errors
- `window._api` exists
- adapter import succeeds

Required fixtures:
- none

CI job:
- `integration-shell-smoke`

Completion gate:
- passing server smoke
- passing browser smoke

---

## M1: Text-Only DOCX Bridge

Goal:
- prove end-to-end text-only DOCX open/edit/save/re-open

Tests to add:
- `ffi/wasm/tests/text_roundtrip.rs`
- `web/tests/text_bridge_roundtrip.spec.(js|ts)`
- `crates/s1engine/tests/text_only_web_roundtrip.rs`

Required checks:
- open DOCX fixture through adapter
- visible text appears in OnlyOffice
- user edit is applied
- save path produces DOCX bytes
- saved DOCX reopens through `s1engine`
- text matches expected round-trip output
- unsupported formatting/object fidelity is explicitly not asserted here

Required fixtures:
- `tests/integration-corpus/text/simple.docx`
- `tests/integration-corpus/text/multiline-tabs.docx`

CI job:
- `integration-text-bridge`

Completion gate:
- all text fixtures round-trip

---

## M2: Serve `web/` by Default

Goal:
- server works out of the box with no manual static-dir override

Tests to add:
- extend `server/tests/static_web_serving.rs`
- add `server/tests/default_static_dir.rs`

Required checks:
- server starts with no `S1_STATIC_DIR`
- `/` resolves to `web/index.html`
- static assets for `web/` resolve
- boot smoke still passes when app is served through server

Required fixtures:
- none

CI job:
- `integration-static-serving`

Completion gate:
- fresh-start default serving passes in CI

---

## M3: Structural Import Bridge

Goal:
- preserve document structure on open for the milestone scope

Tests to add:
- `web/tests/structural_import.spec.(js|ts)`
- `crates/s1engine/tests/onlyoffice_import_scope.rs`

Required checks:
- formatted runs remain distinguishable
- headings preserve heading semantics or equivalent visible styling
- tabs and line breaks preserved
- list structure preserved for supported list cases
- unsupported objects explicitly marked unsupported

Required fixtures:
- `tests/integration-corpus/formatting/basic-runs.docx`
- `tests/integration-corpus/formatting/headings.docx`
- `tests/integration-corpus/lists/basic-lists.docx`
- `tests/integration-corpus/text/tabs-linebreaks.docx`

CI job:
- `integration-structural-import`

Completion gate:
- supported import scope passes for all milestone fixtures

---

## M4: Structural Export Bridge

Goal:
- save structured content, not flattened text

Tests to add:
- `web/tests/structural_export.spec.(js|ts)`
- `crates/s1engine/tests/onlyoffice_export_scope.rs`

Required checks:
- create/edit formatted content in web app
- save DOCX
- reopen in `s1engine`
- verify structure for:
  - paragraphs
  - runs
  - basic inline formatting
  - lists

Required fixtures:
- fixtures from M3 plus editable expected-output fixtures

CI job:
- `integration-structural-export`

Completion gate:
- open/edit/save/re-open passes for milestone scope

---

## M5: Layout and Object Fidelity

Goal:
- preserve major document objects and layout-critical structures

Tests to add:
- `web/tests/object_fidelity.spec.(js|ts)`
- `crates/s1engine/tests/web_object_roundtrip.rs`
- optional visual regression suite: `web/tests/visual_fidelity.spec.(js|ts)`

Required checks:
- tables survive
- images survive
- hyperlinks/bookmarks survive
- headers/footers survive
- sections survive
- metadata survives
- visual smoke for milestone fixtures

Required fixtures:
- `tests/integration-corpus/tables/*.docx`
- `tests/integration-corpus/images/*.docx`
- `tests/integration-corpus/headers-footers/*.docx`
- `tests/integration-corpus/sections/*.docx`
- `tests/integration-corpus/real-world/*.docx`

CI job:
- `integration-object-fidelity`

Completion gate:
- all supported object classes pass

---

## M6: Editing Semantics and Undo/Redo

Goal:
- prove the chosen source-of-truth model behaves consistently

Tests to add:
- `web/tests/undo_redo_authority.spec.(js|ts)`
- `crates/s1engine/tests/edit_authority_contract.rs`

Required checks:
- type -> undo -> redo works
- format -> undo -> redo works
- save after undo/redo works
- reopen after undo/redo works
- divergence detection between authoritative and non-authoritative sides

Required fixtures:
- small editable fixtures from M3/M4

CI job:
- `integration-editing-authority`

Completion gate:
- ADR exists
- tests encode and validate that ADR

---

## M7: Collaboration and Server Workflow

Goal:
- validate session lifecycle and multi-user editing path

Tests to add:
- `server/tests/collaboration_flow.rs`
- `web/tests/collaboration_smoke.spec.(js|ts)`

Required checks:
- two clients open same document
- edit from client A appears in B
- edit from client B appears in A
- save callback lifecycle behaves correctly
- reconnect path behaves correctly

Required fixtures:
- one small collaboration fixture

CI job:
- `integration-collaboration`

Completion gate:
- deterministic two-client test passes

---

## M8: Fidelity Validation

Goal:
- establish the lasting regression gate for the active architecture

Tests to add:
- `web/tests/open_edit_save_reopen.spec.(js|ts)`
- `web/tests/fidelity_manifest_validation.spec.(js|ts)` if using a manifest
- `server/tests/corpus_presence.rs`

Required checks:
- corpus exists at canonical path
- all active fixtures exist
- missing active fixture fails CI
- current supported feature scope has explicit regression coverage
- unsupported scope is reported separately

Required fixtures:
- full active corpus

CI job:
- `integration-fidelity-gate`

Completion gate:
- the repo has a stable, enforced regression gate for the current architecture

---

## M9: Production Packaging

Goal:
- prove the shipped form of the app actually works

Tests to add:
- `server/tests/production_packaging.rs`
- `web/tests/production_smoke.spec.(js|ts)`

Required checks:
- fresh environment can build assets
- server can serve packaged web app
- browser smoke passes against packaged app
- WASM asset resolution works in packaged mode
- required attribution/license artifacts are present

Required fixtures:
- one smoke DOCX fixture

CI job:
- `integration-production-package`

Completion gate:
- fresh-environment production smoke passes

---

## Recommended Initial Test Backlog

Implement in this order:

1. `server/tests/static_web_serving.rs`
2. `web/tests/smoke_boot.spec.(js|ts)`
3. `web/tests/text_bridge_roundtrip.spec.(js|ts)`
4. canonical corpus directory under `tests/integration-corpus/`
5. `web/tests/structural_import.spec.(js|ts)`
6. `web/tests/structural_export.spec.(js|ts)`
7. `web/tests/undo_redo_authority.spec.(js|ts)`
8. `web/tests/collaboration_smoke.spec.(js|ts)`

---

## CI Mapping Template

Suggested CI jobs:
- `integration-shell-smoke`
- `integration-text-bridge`
- `integration-static-serving`
- `integration-structural-import`
- `integration-structural-export`
- `integration-object-fidelity`
- `integration-editing-authority`
- `integration-collaboration`
- `integration-fidelity-gate`
- `integration-production-package`

Each job should:
- install only what it needs
- fail on missing required fixtures
- publish logs/artifacts when browser tests fail

---

## Completion Rule

A milestone is not complete unless:
- code for the milestone exists
- automated tests for the milestone exist
- CI runs those tests
- required fixtures for the milestone are committed
