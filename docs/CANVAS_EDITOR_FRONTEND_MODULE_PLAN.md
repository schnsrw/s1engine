# Canvas-First Editor Frontend Module Plan

**Status:** Draft frontend module plan  
**Last updated:** 2026-03-30  
**Applies to branch:** `feature/reimagine-port`

## Related Documents

- `CANVAS_EDITOR_HIGH_LEVEL_ARCHITECTURE.md`
- `CANVAS_EDITOR_LOW_LEVEL_DESIGN.md`
- `CANVAS_EDITOR_WASM_API_CONTRACT.md`
- `CANVAS_EDITOR_IMPLEMENTATION_ROADMAP.md`

## Purpose

This document defines how the browser-side editor code should be split once canvas becomes the primary page surface.

The repo already has a prototype `editor/src/canvas-render.js`, but the current browser architecture is still centered on monolithic files such as `editor/src/render.js`, `editor/src/input.js`, and the global `editor/src/state.js`. The goal here is to replace those ownership boundaries rather than just adding another renderer.

## Design Rules

- Keep vanilla JS module style consistent with the repo unless the project adopts a framework separately.
- Keep document truth in Rust/WASM, not in JS stores.
- Keep view state local and explicit instead of growing the global `state` object further.
- Keep the legacy DOM editor path isolated so canvas work does not keep accreting DOM assumptions.

## Target Browser Architecture

```text
editor shell
  -> document session
  -> scene store
  -> canvas viewport
  -> pointer/input controllers
  -> overlay controllers
  -> accessibility mirror
```

## Proposed Directory Layout

```text
editor/src/
  document/
    session.js
    capabilities.js
    revisions.js
  canvas/
    app.js
    host.js
    viewport.js
    scheduler.js
    world-transform.js
    page-cache.js
    scene-store.js
    renderer/
      document-renderer.js
      page-renderer.js
      text-painter.js
      table-painter.js
      image-painter.js
      shape-painter.js
      decoration-painter.js
    interaction/
      pointer.js
      hit-test.js
      drag-select.js
      wheel-zoom.js
  input/
    bridge.js
    keyboard.js
    composition.js
    clipboard.js
  selection/
    model-selection.js
    navigation.js
    painting.js
  overlay/
    comments.js
    spellcheck.js
    context-menu.js
    handles.js
    hyperlink-popup.js
  a11y/
    mirror.js
    focus-proxy.js
  legacy/
    dom-render.js
    dom-input.js
    dom-selection.js
```

## Core Runtime Modules

### `document/session.js`

Responsibilities:

- hold `engine` and `doc`
- expose document open/create/close lifecycle
- serialize all editor-facing calls into the `WasmDocument`
- expose current `document_revision` and `layout_revision`

This becomes the browser boundary point instead of letting many files call `state.doc` ad hoc.

### `canvas/scene-store.js`

Responsibilities:

- cache scene summary and page scenes by `layout_revision`
- fetch visible page scenes from WASM
- evict old page scenes when revisions change
- answer scene availability questions for the renderer

It must not invent scene data. It is a cache and coordinator only.

### `canvas/viewport.js`

Responsibilities:

- scroll state
- zoom state
- visible page range calculation
- viewport-to-world coordinate conversion
- device pixel ratio handling

### `canvas/scheduler.js`

Responsibilities:

- batch render requests into animation frames
- track dirty pages and dirty overlays
- avoid whole-document repaint for cursor or selection moves

### `canvas/renderer/*`

Responsibilities:

- page chrome and background painting
- scene item painting in deterministic z-order
- specialized painters for text, tables, images, shapes, and decorations
- caching immutable page layers where profitable

### `input/bridge.js`

Responsibilities:

- own the hidden textarea
- place it from caret geometry
- coordinate composition start/update/end
- hand browser text input to `document/session.js`

### `selection/model-selection.js`

Responsibilities:

- own primary and secondary ranges in JS view state
- expose collapsed vs expanded selection state
- keep selection synchronized with edit results returned from WASM

### `overlay/*`

Responsibilities:

- comments thread popup positioning
- spellcheck suggestions
- image and shape handles
- hyperlink and context menu overlays

These stay DOM-based but geometry-driven.

### `a11y/mirror.js`

Responsibilities:

- create a semantic reading-order mirror
- map active selection and focus for assistive tech
- keep the mirror synchronized with the current layout revision

#### Synchronization strategy

The a11y mirror updates in response to two triggers:

1. **Layout revision change** (after edit or remote sync):
   - Fetch scene summary for current visible pages.
   - For each visible page, generate semantic DOM: `<p>`, `<h1>`–`<h6>`, `<table>`, `<li>`, `<a>`, `<img alt="...">`.
   - Replace the mirror's content for changed pages only (diff by page index + layout revision).
   - Debounce: 100ms after last edit to avoid screen reader chatter.

2. **Selection/focus change** (after click, keyboard navigation, or remote cursor update):
   - Update `aria-activedescendant` on the mirror container to point to the element nearest the caret position.
   - If the caret moves to a different page, ensure that page's semantic content is in the mirror.

#### Mirror DOM structure

```html
<div id="a11y-mirror" role="document" aria-label="Document content" style="position:absolute;clip:rect(0,0,0,0);width:1px;height:1px;overflow:hidden;">
  <div role="region" aria-label="Page 1">
    <h1>Document Title</h1>
    <p>First paragraph text...</p>
    <table role="table">
      <tr><td>Cell 1</td><td>Cell 2</td></tr>
    </table>
  </div>
  <!-- more pages as needed -->
</div>
```

#### What the mirror does NOT do

- Does not replicate visual positions or coordinates.
- Does not handle pointer events (canvas handles those).
- Does not contain images (uses `<img alt="description">` with empty src).
- Does not maintain the full document — only visible pages plus one page of lookahead.

## State Ownership Plan

| State category | Owner | Notes |
|---|---|---|
| Document model | Rust/WASM | Never duplicated in JS |
| Layout revision and dirty pages | Rust/WASM with JS cache | JS stores last seen revision only |
| Visible page scenes | `canvas/scene-store.js` | Cache by page + revision |
| Scroll and zoom | `canvas/viewport.js` | Pure view state |
| Selection ranges | `selection/model-selection.js` | View state backed by Rust positions |
| Hidden input and composition | `input/bridge.js` | DOM bridge only |
| Comments, menus, handles | `overlay/*` | DOM overlay state |
| Accessibility mirror | `a11y/mirror.js` | Derived from scene/model data |

## Migration Map From Current Files

| Current file | Problem | Target split |
|---|---|---|
| `editor/src/render.js` | mixes DOM rendering, pagination assumptions, and orchestration | `document/session.js`, `canvas/scene-store.js`, `canvas/renderer/*`, `legacy/dom-render.js` |
| `editor/src/input.js` | mixes keyboard logic, DOM selection logic, and browser mutation handling | `input/bridge.js`, `input/keyboard.js`, `input/composition.js`, `input/clipboard.js`, `legacy/dom-input.js` |
| `editor/src/selection.js` | DOM selection helpers are tightly coupled to page content | `selection/model-selection.js`, `selection/navigation.js`, `selection/painting.js`, `legacy/dom-selection.js` |
| `editor/src/canvas-render.js` | useful prototype, but too monolithic and scene-format-specific | temporary shim, then split into `canvas/renderer/*`, `canvas/interaction/*`, `canvas/scene-store.js` |
| `editor/src/state.js` | giant global bucket for unrelated concerns | keep minimal app state there, move canvas/runtime state into dedicated modules |

## Recommended First File Set

The first implementation slice should create these modules first:

- `editor/src/document/session.js`
- `editor/src/canvas/app.js`
- `editor/src/canvas/viewport.js`
- `editor/src/canvas/scene-store.js`
- `editor/src/canvas/renderer/page-renderer.js`
- `editor/src/input/bridge.js`
- `editor/src/selection/model-selection.js`

That is enough to support read-only pages, click hit-test, and a single-caret typing path.

## `state.js` Reduction Plan

Keep in shared app state for now:

- `engine`
- `doc`
- `currentView`
- top-level document metadata needed by shell UI
- top-level feature flags such as canvas mode

Move out over time:

- `pageMap`, `pageElements`, `nodeToPage`, `pagesRendered`
- `_layoutCache`, `_layoutDirty`, `_layoutDebounceTimer`
- DOM selection caches
- canvas-specific visible-page and scene caches

The target is not “one better global object”. The target is explicit state ownership per subsystem.

### Reduction gates

| Gate | When | What moves out | Prerequisite |
|---|---|---|---|
| Gate 1 | Phase 1 complete (read-only canvas) | `_layoutCache`, `_layoutDirty`, `_layoutDebounceTimer` → `canvas/scene-store.js` | Scene store fetches layout from WASM |
| Gate 2 | Phase 2 complete (hit-test + caret) | DOM selection caches → removed; `pageMap` → `canvas/scene-store.js` | Selection is model-based |
| Gate 3 | Phase 3 complete (typing) | `pagesRendered`, `pageElements` → `canvas/viewport.js` | Canvas manages page visibility |
| Gate 4 | Phase 5 complete (common objects) | `nodeToPage` → removed (use `node_bounds()` from WASM) | WASM provides node geometry |
| Gate 5 | Phase 7 complete (full parity) | Remove `state.js` entirely; remaining fields move to `document/session.js` | No DOM rendering path |

Each gate is a checkpoint: do not move state until the prerequisite is met and verified by fidelity tests.

## Feature Flag Strategy

Canvas mode is activated via feature flags, allowing gradual rollout and instant rollback.

### Flag definitions

| Flag | Default | Controls |
|---|---|---|
| `canvas_render` | `false` | Use canvas for page rendering instead of DOM |
| `canvas_input` | `false` | Route input through hidden textarea + WASM instead of contenteditable |
| `canvas_selection` | `false` | Paint selection/caret from WASM geometry instead of DOM ranges |
| `canvas_a11y_mirror` | `false` | Enable semantic a11y mirror (required when `canvas_render` is true) |
| `legacy_dom_fallback` | `true` | Fall back to DOM rendering if canvas fails |

### Activation rules

- `canvas_input` requires `canvas_render` (cannot route input to canvas without canvas rendering).
- `canvas_selection` requires `canvas_render`.
- `canvas_a11y_mirror` is auto-enabled when `canvas_render` is true.
- `legacy_dom_fallback` is disabled only at Phase 7 (full parity confirmed).

### How flags are set

Flags are set via URL query parameters for development and testing:

```
?canvas_render=true&canvas_input=true
```

For production, flags are set in `editor/src/config.js` with defaults that advance as phases complete.

### Rollback

If a canvas flag causes issues, disable it by toggling the flag. The DOM path remains functional as long as `legacy_dom_fallback` is true.

## Event Delegation from Canvas to Shell

Canvas pointer events must reach the editor shell for context menus, overlays, and toolbar interactions.

### Event flow

```
canvas element
  → pointerdown/pointermove/pointerup
  → canvas/interaction/pointer.js (converts to world coordinates, hit-tests)
  → if hit target is document content → selection/model-selection.js
  → if right-click → overlay/context-menu.js (DOM context menu)
  → if hit target is image/shape → overlay/handles.js (resize handles)
  → if hit target is hyperlink → overlay/hyperlink-popup.js (link popup)
  → if double-click on word → selection (word boundary selection)
  → if triple-click → selection (paragraph selection)

keyboard events
  → input/bridge.js (hidden textarea captures)
  → if shortcut (Ctrl+B, Ctrl+Z, etc.) → input/keyboard.js → document/session.js
  → if character input → document/session.js → replace_range()
  → if Tab in table → navigation.js → move to next cell
  → if Escape → overlay/* (close active overlay)

wheel events
  → canvas/viewport.js (scroll)
  → if Ctrl+wheel → canvas/viewport.js (zoom)

clipboard events
  → input/clipboard.js → document/session.js (copy_range_*, paste_html)
```

### Shell notifications

When canvas state changes affect the shell UI, the canvas layer dispatches custom events:

```js
// After selection change → toolbar needs update
document.dispatchEvent(new CustomEvent('editor:selection-changed', { detail: formattingState }));

// After edit → dirty indicator needs update
document.dispatchEvent(new CustomEvent('editor:document-changed', { detail: { revision, dirty } }));

// After page change → page indicator needs update
document.dispatchEvent(new CustomEvent('editor:page-changed', { detail: { currentPage, pageCount } }));
```

The toolbar, status bar, and page indicator listen for these events and update themselves.

## Event Flow for the New Path

### Open document

1. `document/session.js` opens the document
2. `canvas/scene-store.js` fetches scene summary
3. `canvas/viewport.js` computes visible page range
4. `canvas/renderer/document-renderer.js` paints visible pages
5. `a11y/mirror.js` updates semantic output for visible or active content

### Pointer click

1. `canvas/interaction/pointer.js` converts client coordinates to world points
2. `canvas/interaction/hit-test.js` asks WASM for a `PositionRef`
3. `selection/model-selection.js` stores collapsed selection
4. `input/bridge.js` repositions hidden textarea
5. `selection/painting.js` schedules caret repaint

### Typing

1. browser input arrives in `input/bridge.js`
2. `document/session.js` calls `replace_range()` or `insert_text_at()`
3. returned dirty-page info updates `canvas/scene-store.js`
4. `canvas/scheduler.js` repaints only affected pages and overlays
5. `selection/model-selection.js` updates to the returned canonical selection

## Transitional Strategy for Existing Canvas Code

`editor/src/canvas-render.js` should not become the permanent architecture anchor.

Use it as a bridge only:

- keep it alive while scene APIs are unstable
- peel rendering helpers out into `canvas/renderer/*`
- stop new editor logic from accumulating there
- eventually reduce it to a thin compatibility wrapper or remove it

## Success Criteria

The browser-side migration is successful when no central editor workflow depends on `.page-content`, DOM ranges, or DOM pagination to determine document truth. At that point, the browser becomes a canvas client of the Rust engine instead of a second editor implementation.
