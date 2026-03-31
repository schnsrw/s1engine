# Canvas-First Editor Low-Level Design

**Status:** Draft low-level design  
**Last updated:** 2026-03-30

## Related Documents

- `CANVAS_EDITOR_HIGH_LEVEL_ARCHITECTURE.md`
- `CANVAS_EDITOR_ELEMENTS_SPEC.md`

## Purpose

This document defines the low-level runtime design for implementing a canvas-first editor on top of the current s1engine Rust/WASM stack.

It assumes the high-level direction in `CANVAS_EDITOR_HIGH_LEVEL_ARCHITECTURE.md` is accepted.

## Current Baseline

### Current strengths

- Rust already owns the document model, operations, and layout engine.
- `s1-layout` already produces page-relative geometry.
- `s1-text` already performs shaping, bidi, hyphenation, and line-break support.
- the WASM bridge already exposes document mutation and layout entry points.

### Current blockers

- WASM surface is HTML-heavy.
- editor event handling is deeply tied to `.page-content` contenteditable containers.
- per-node HTML rendering is used as the incremental update protocol.
- DOM range state is used as the editing position model.

## Target Runtime Pipeline

```text
Document Mutation
  -> s1-ops transaction
  -> DocumentModel revision update
  -> layout invalidation
  -> s1-layout recompute (full or incremental)
  -> scene serialization to WASM
  -> JS viewport receives scene diff/full scene
  -> canvas paints pages
  -> DOM overlays update from geometry APIs
```

## Rendering Data Model

### 1. Canonical layout data

The existing `LayoutDocument` remains the internal Rust output of pagination.

### 2. UI-facing render scene

Add a UI-facing scene representation derived from layout output.

Suggested shape:

```text
RenderScene
  revision: u64
  page_count: u32
  page_size_pt: { width, height }
  visible_scale_hint: f32
  pages: Vec<ScenePage>

ScenePage
  index: u32
  section_index: u32
  bounds_pt: Rect
  content_rect_pt: Rect
  header_rect_pt: Option<Rect>
  footer_rect_pt: Option<Rect>
  items: Vec<SceneItem>

SceneItem
  TextRun
  ParagraphBackground
  ParagraphBorder
  ListMarker
  TableGrid
  TableCellBackground
  TableCellBorder
  Image
  Shape
  HeaderFooterAnchor
  BookmarkAnchor
  CommentAnchor
```

The scene should represent static page content only. It should not include transient UI state like active selection, blink state, drag handles, or ruler hover feedback.

## WASM API Additions

### Rendering APIs

Add scene-oriented APIs alongside existing HTML APIs.

Suggested APIs:

- `layout_scene_json(config)`
- `layout_scene_json_with_fonts(font_db, config)`
- `page_scene_json(page_index)`
- `visible_page_scenes_json(start_page, end_page)`
- `document_revision()`

### Geometry APIs

These are required to break the dependency on DOM ranges.

Suggested APIs:

- `hit_test(page_index, x_pt, y_pt) -> PositionHit`
- `caret_rect(position) -> Rect`
- `selection_rects(anchor, focus) -> Vec<Rect>`
- `word_boundary_at(position)`
- `line_boundary_at(position)`
- `node_bounds(node_id) -> Rect | Vec<Rect>`

### Editing APIs

The browser should talk in positions/ranges, not DOM nodes.

Suggested APIs:

- `insert_text_at(position, text)`
- `delete_range(anchor, focus)`
- `replace_range(anchor, focus, text)`
- `toggle_mark_on_range(anchor, focus, mark)`
- `set_block_attrs(node_id, attrs)`
- `insert_paragraph_break(position)`
- `insert_table(position, spec)`
- `insert_image(position_or_anchor, spec)`
- `move_selection(direction, granularity, extend)`

### IME / composition APIs

Composition must not mutate the document model on every intermediate browser event unless explicitly desired.

Suggested APIs:

- `begin_composition(position)`
- `update_composition(text, selection_start, selection_end)`
- `commit_composition(text)`
- `cancel_composition()`

## Frontend Subsystems

### 1. CanvasViewportController

Responsibilities:

- page scroll model
- zoom factor
- device pixel ratio scaling
- visible page calculation
- dirty rect scheduling
- offscreen page culling

Rules:

- internal world coordinates remain in points
- CSS pixels are derived from points and zoom
- backing canvas resolution multiplies by devicePixelRatio

### 2. CanvasPageRenderer

Responsibilities:

- paint page background
- paint margins/guides/rulers
- paint scene items in z-order
- cache immutable page layers when possible
- repaint only dirty pages/regions

Recommended paint order:

1. page paper/background
2. page shadow / gap chrome
3. guides / margin indicators
4. paragraph and cell backgrounds
5. table fills and borders
6. images / behind-text shapes
7. text runs and list markers
8. in-front shapes / handles
9. selection fills
10. cursor(s)
11. composition underline / spell marks

### 3. InputBridge

Responsibilities:

- maintain hidden textarea/input
- translate browser key/input/composition events to model operations
- request caret geometry from WASM for IME anchoring
- handle clipboard bridge

Notes:

- hidden input should be positioned near the active caret rect
- browser selection should not represent document selection
- clipboard serialization should come from model ranges, not DOM selection extraction

### 4. SelectionController

Responsibilities:

- primary caret position
- anchor/focus range state
- multi-cursor state
- selection painting requests
- keyboard navigation granularity

Selection source of truth should be model positions, not DOM nodes.

### 5. OverlayManager

Responsibilities:

- context menus
- comment cards
- spellcheck suggestion popups
- resize/rotate handles for shapes and images
- hyperlink popups

These may remain DOM overlays positioned from canvas/world geometry.

### 6. AccessibilityMirror

Responsibilities:

- expose semantic reading order
- map current page/range/focus to assistive tech
- support screen reader traversal of paragraphs, headings, tables, and links

This layer should be derived from the model/layout scene, not hand-maintained from the page DOM editor.

## Proposed Module Split

### Rust side

- `s1-model`: document nodes, attributes, semantic ranges, anchors
- `s1-ops`: transactions, transforms, undo/redo, selection movement semantics
- `s1-text`: shaping, bidi, break opportunities, font metrics
- `s1-layout`: pagination, line boxes, table geometry, page object placement
- `ffi/wasm`: serialized scene, geometry queries, editing entry points

### Browser side

- `editor/src/canvas/viewport.*`: scroll, zoom, DPR handling, page visibility
- `editor/src/canvas/renderer.*`: scene painting, caches, invalidation scheduling
- `editor/src/canvas/selection.*`: canvas selection and caret state
- `editor/src/input/bridge.*`: hidden textarea, keyboard, composition, clipboard
- `editor/src/overlay/*`: comments, spellcheck popups, resize handles, menus
- `editor/src/a11y/*`: semantic mirror and assistive-tech synchronization

This split keeps the browser thin and prevents layout logic from drifting into JS.

## Pagination and Incrementality

### Rust-side ownership

Pagination remains fully owned by `s1-layout`.

JS may request:

- full scene
- affected pages only
- page-map summary
- geometry for a given revision

JS must not decide page breaks.

### Incremental strategy

Recommended invalidation levels:

1. **paint-only dirty**: selection, cursor, hover, guides
2. **page-scene dirty**: formatting or object changes that do not shift later pages
3. **pagination dirty from page N**: content height changes, section changes, table changes
4. **document-wide dirty**: style/global setting changes that alter all pages

Use the current layout cache machinery as the foundation for page-scene caching.

## Zoom Model

Keep all engine geometry in points.

Frontend transform chain:

```text
point-space -> zoomed CSS pixel space -> device backing pixel space
```

Rules:

- do not mutate model/layout units for zoom
- canvas size = css_size * devicePixelRatio
- text/object painting should happen in world coordinates with a transform, not by rewriting scene coordinates

## Tables and Borders

Table rendering should be canvas-painted from explicit border geometry, not inferred from nested DOM boxes.

Recommended internal representation per cell/table:

- content rect
- padding rect
- background rect
- resolved border segments (top/right/bottom/left)
- collapsed-border resolution result if applicable

The renderer should paint borders segment-first to avoid DOM-style inconsistencies.

## Images and Shapes

### Images

Rust scene should provide:

- source reference / decoded handle key
- object bounds
- clipping rect
- wrap mode
- z order
- transform metadata if rotation/crop is supported

### Shapes / text boxes

Do not treat shapes as ad hoc DOM widgets.

Scene should provide:

- geometry primitive or path
- fill/stroke
- z layer
- anchor behavior
- optional embedded text frame

Text inside text boxes should still route through Rust shaping/layout logic.

## Spellcheck and Comments

### Spellcheck

Canvas cannot rely on browser-native red underlines without a DOM text surface.

Options:

1. custom spellcheck engine + canvas painting
2. hidden mirror text surface for browser spellcheck only

Recommendation: plan for custom spellcheck rendering, even if a hidden mirror is used temporarily.

### Comments

Comment markers/anchors should be part of scene geometry.
Comment threads/panels can remain DOM overlays.

## Performance Targets

Recommended editor targets:

- steady 60 FPS for cursor/selection/scroll interactions
- no full-document repaint on caret move
- visible-page-only painting under normal scroll
- repaint of changed page(s) only after local edit when possible
- no HTML diffing requirement for editor correctness

## Locked Decisions

### Scene serialization: JSON (not binary)

**Decision:** Scene data crosses the WASM boundary as JSON strings, parsed with `JSON.parse()` on the JS side.

**Rationale:**

- `JSON.parse()` is the fastest deserialization path in every browser engine (V8, SpiderMonkey, JSC). It outperforms `serde_wasm_bindgen` for payloads above ~1 KB.
- JSON is human-readable, which simplifies debugging, fidelity comparison tooling, and snapshot testing.
- The overhead of JSON serialization in Rust (via `serde_json`) is negligible compared to layout computation.
- Binary formats (MessagePack, FlatBuffers) add dependencies, tooling complexity, and debugging friction for marginal throughput gains on the data sizes we produce (typical page scene is 5–50 KB).
- Temporary `_json` wrappers can be replaced later if profiling proves JSON is a bottleneck, but the protocol shape (field names, nesting) remains stable regardless of encoding.

**Convention:** All scene/geometry API methods return `String` (JSON). JS callers use `JSON.parse(result)`. Method names end with no suffix (not `_json`) since JSON is the canonical encoding.

### Scene module location: `s1-layout` (not a separate crate)

**Decision:** Scene serialization lives inside `s1-layout` as a `scene` submodule alongside the existing `html` submodule.

**Rationale:**

- The scene is a direct projection of `LayoutDocument` — adding a separate crate would create a circular dependency or force `s1-layout` to export unstable internals.
- The `s1-layout` crate already contains `layout_to_html()` as a serialization path; `layout_to_scene_json()` is the canvas equivalent.
- A separate crate adds coordination overhead with no architectural benefit at this scale.

### Model position format across WASM boundary

**Decision:** Positions use the `PositionRef` JSON shape defined in `CANVAS_EDITOR_WASM_API_CONTRACT.md`:

```json
{ "node_id": "0:42", "offset_utf16": 15, "affinity": "downstream" }
```

- `node_id` is the `NodeId` serialized as `"replica_id:counter"` (matching existing `data-node-id` attributes).
- `offset_utf16` is the character offset in UTF-16 code units within the text-bearing node.
- `affinity` is `"downstream"` (default) or `"upstream"` for line-wrap ambiguity.

Rust internally converts UTF-16 offsets to byte/char offsets as needed. The boundary contract is always UTF-16.

### Accessibility mirror: semantic output div approach

**Decision:** The a11y mirror is a hidden DOM tree (`aria-hidden="false"`, visually clipped) that reflects the reading order of visible pages.

**Strategy:**

1. On layout revision change, `a11y/mirror.js` receives the scene summary.
2. For each visible page, it creates semantic elements: `<p>`, `<h1>`–`<h6>`, `<table>`, `<li>`, `<a>`, `<img alt="...">`.
3. Elements are positioned off-screen but in correct DOM order for screen reader traversal.
4. The mirror does NOT attempt to replicate visual layout — it provides reading order and landmarks only.
5. Active selection/focus is synchronized: `aria-activedescendant` or focus management tracks the model position.
6. Mirror updates are debounced (100ms) to avoid screen reader chatter during rapid edits.

**What it exposes:**
- Document headings and heading levels
- Paragraph text content
- Table structure (rows, cells, headers)
- Image alt text
- Link targets
- List structure and nesting
- Current selection/focus position

**What it does NOT do:**
- Replicate exact visual positions
- Handle pointer interaction (canvas handles that)
- Own editing input (hidden textarea handles that)

## Incremental Scene Updates

### Invalidation model

Every `EditResult` returned from a mutation includes `dirty_pages: { start, end }` — the inclusive range of page indices whose scene data changed.

JS uses this to:

1. Evict cached page scenes for `[start..end]` from `scene-store.js`.
2. Re-fetch only dirty page scenes via `visible_page_scenes(start, end)`.
3. Repaint only dirty pages on the canvas.

### Invalidation levels (refined)

| Level | Trigger | JS action |
|---|---|---|
| **paint-only** | cursor blink, selection change, hover | Repaint overlay layer only, no WASM call |
| **page-scene dirty** | text edit, formatting change within a page | Re-fetch dirty pages from `dirty_pages` range |
| **pagination dirty from page N** | content height change, section break, table reflow | Re-fetch scene summary + all pages from N onward |
| **document-wide dirty** | style definition change, global setting | Re-fetch full scene summary + all visible pages |

### Revision-based cache coherence

- `scene-store.js` stores `(page_index, layout_revision) → PageScene`.
- On edit, the returned `layout_revision` is compared to cached revision. Stale entries are evicted.
- If `layout_revision` has not changed (e.g., a metadata-only edit), no scene re-fetch is needed.

### No delta encoding for now

Full page scenes are re-fetched for dirty pages. Delta encoding (sending only changed items) is deferred until profiling shows it's needed. The typical dirty-page set is 1–3 pages, and each page scene is 5–50 KB — well within acceptable latency.

## Hidden Input Positioning Algorithm

The hidden `<textarea>` must be positioned near the caret for IME candidate windows to appear correctly.

### Algorithm

1. On selection change, call `caret_rect(position)` → `RectPt { page_index, x, y, width, height }`.
2. Convert page-local point coordinates to viewport CSS pixels:
   ```
   css_x = (page_offset_x + rect.x) * zoom
   css_y = (page_offset_y + rect.y) * zoom - scroll_y
   ```
3. Position the hidden textarea at `(css_x, css_y)` with `position: fixed`.
4. Set textarea height to match caret height for correct IME popup alignment.
5. The textarea is `opacity: 0`, `width: 1px`, `overflow: hidden` — invisible but positioned.

### Edge cases

- **Caret at page bottom:** Ensure IME popup doesn't overflow viewport; browser handles this natively.
- **Zoom change:** Reposition on zoom since CSS pixel coordinates shift.
- **Scroll:** Reposition on scroll since page offset changes.
- **Multi-page selection:** Position at focus (not anchor) end of selection.

## Spellcheck Strategy

**Decision:** Custom spellcheck rendering on canvas, with pluggable spellcheck engine.

### Architecture

1. **Spellcheck engine** is external (browser-native via hidden mirror, or a WASM-based dictionary checker). The editor does not ship its own dictionary.
2. **Spellcheck results** are ranges of misspelled words, stored in JS view state (not in the Rust model).
3. **Canvas rendering** paints red wavy underlines at the geometry positions of misspelled ranges (obtained via `selection_rects()` for each misspelled range).
4. **Suggestion popup** is a DOM overlay positioned from the misspelled word's geometry.

### Initial approach (hidden mirror)

For the first implementation, use a hidden `<div contenteditable>` containing the text of the active paragraph. The browser's native spellcheck marks misspelled words. JS reads the misspelled ranges from the DOM and translates them to model ranges.

This avoids shipping a dictionary while still getting canvas-painted underlines.

### Future approach (WASM spellcheck)

Replace the hidden mirror with a WASM-based spellcheck engine (e.g., hunspell compiled to WASM or a custom dictionary). This eliminates the hidden DOM surface entirely.

## Clipboard Strategy

### Copy

1. On Ctrl+C / Cmd+C, call `copy_range_plain_text(range)` and `copy_range_html(range)` from WASM.
2. Write both formats to the clipboard via the Clipboard API:
   ```js
   navigator.clipboard.write([
     new ClipboardItem({
       'text/plain': plainBlob,
       'text/html': htmlBlob
     })
   ]);
   ```

### Cut

1. Copy (as above).
2. Call `delete_range(range)` to remove the selected content.
3. Repaint dirty pages from the `EditResult`.

### Paste

1. Read clipboard via `navigator.clipboard.read()`.
2. If `text/html` is available and rich paste is desired:
   - Pass HTML to a new WASM method `paste_html(position, html)` which parses the HTML fragment and inserts structured content.
3. If plain text only:
   - Call `insert_text_at(position, text)` or `replace_range(range, text)`.
4. Repaint dirty pages from the `EditResult`.

### Paste sanitization

- The Rust-side `paste_html()` method sanitizes the HTML (strips scripts, external images, dangerous attributes).
- Only structural elements are preserved: paragraphs, runs with formatting, tables, lists, images (as data URIs).
- Unknown elements are flattened to plain text.

## Toolbar State Update Flow

### Problem

The toolbar (bold/italic/alignment buttons, font picker, etc.) must reflect the current document state at the selection. In the DOM editor, this was read from DOM element styles. In canvas mode, DOM has no styled elements to query.

### Solution

1. On every selection change, `selection/model-selection.js` calls a new WASM method:
   ```
   selection_formatting(range) → FormattingState
   ```
2. `FormattingState` returns the resolved formatting at the selection:
   ```json
   {
     "bold": true,
     "italic": false,
     "underline": "none",
     "font_family": "Times New Roman",
     "font_size_pt": 12.0,
     "color": "#000000",
     "alignment": "left",
     "line_spacing": "single",
     "list_format": null,
     "style_id": "Normal"
   }
   ```
3. `toolbar-handlers.js` updates button active states and dropdown values from `FormattingState`.
4. For mixed-formatting selections (e.g., partially bold), values are `null` (indeterminate).

### Debouncing

Toolbar state updates are debounced to 50ms to avoid excessive WASM calls during rapid cursor movement.

## Undo/Redo Ownership

**Decision:** Undo/redo is fully owned by the Rust `s1-ops` transaction stack. JS never maintains a parallel undo history.

### Flow

1. User presses Ctrl+Z → `input/bridge.js` intercepts.
2. Calls `document/session.js` → `doc.undo()`.
3. Rust undoes the last transaction, returns `EditResult` with dirty pages and new selection.
4. JS repaints dirty pages, updates selection, updates toolbar state.

### Redo

Same flow with `doc.redo()`.

### Batch operations

Multi-step UI operations (e.g., find-and-replace-all) use `begin_batch()` / `end_batch()` so they undo as a single unit.

## Dirty Document Tracking

The editor needs to know if the document has unsaved changes (for the dirty indicator, close confirmation, etc.).

### Strategy

1. `document/session.js` stores `lastSavedRevision` (the `document_revision` at last save).
2. On every edit, compare `doc.document_revision()` to `lastSavedRevision`.
3. If different, the document is dirty → update the title bar dirty indicator.
4. On save, update `lastSavedRevision = doc.document_revision()`.

This is simpler and more reliable than tracking individual edits.

## Collaborative Peer Sync with Scene Invalidation

### Problem

When a remote peer sends CRDT operations, the local document model updates, which may invalidate pages. The canvas must repaint without knowing which specific pages changed.

### Strategy

1. Remote operations arrive via the collaboration transport (WebSocket relay).
2. `collab.js` applies them to the CRDT document: `collab_doc.apply_remote_ops(ops)`.
3. The CRDT layer returns a `SyncResult` with `affected_node_ids`.
4. Call `doc.layout_revision()` — if it changed, the layout is dirty.
5. Call `scene_summary()` to get the new page count and page sizes.
6. Compare with the cached scene summary to identify which pages changed (page count, item counts).
7. Re-fetch and repaint changed pages.

### Collaborative cursors

- Each peer's cursor position is broadcast via awareness protocol.
- Remote cursors are stored in `selection/model-selection.js` as secondary ranges.
- `selection/painting.js` paints them with distinct colors (assigned by peer ID hash).
- `caret_rect()` is called for each remote cursor position to get paint coordinates.

## Performance Instrumentation

### Metrics to track

| Metric | Where measured | Target |
|---|---|---|
| Layout time (ms) | Rust `s1-layout` | < 50ms for single-page edit |
| Scene serialization time (ms) | Rust `layout_to_scene_json()` | < 10ms per page |
| WASM boundary crossing (ms) | JS before/after WASM call | < 5ms overhead |
| Canvas paint time (ms) | JS `requestAnimationFrame` callback | < 16ms (60 FPS) |
| Hit-test latency (ms) | JS click → position resolved | < 5ms |
| Keypress-to-paint latency (ms) | JS input event → canvas repainted | < 50ms |
| Scene store cache hit rate (%) | `scene-store.js` | > 90% during scrolling |

### Instrumentation approach

- Wrap key WASM calls in `performance.mark()` / `performance.measure()`.
- Add a `--perf` query parameter that enables a performance overlay (FPS counter, layout time, paint time).
- Log slow frames (> 20ms paint) to console in debug mode.
- Expose `performance.getEntriesByType('measure')` for automated benchmark collection.

### Baseline capture

Before starting canvas migration, capture baseline measurements of the current DOM editor for:
- Document open time (WASM load + first render)
- Typing latency (keypress → visual update)
- Scroll FPS
- Large document (100+ pages) responsiveness

These baselines are stored in `tests/fidelity/baselines/` for regression comparison.

## Risks and Mitigations

### Hard problems

| Risk | Mitigation |
|---|---|
| Accessibility parity with canvas | Semantic mirror provides reading order; test with VoiceOver/NVDA early in Phase 1 |
| Browser IME edge cases | Hidden textarea approach is battle-tested (Google Docs, VS Code); test CJK/Korean/Japanese/Arabic |
| Custom spellcheck UX | Start with hidden mirror for browser-native spellcheck; migrate to WASM spellcheck later |
| Clipboard fidelity for rich ranges | Use `copy_range_html()` from Rust for guaranteed round-trip fidelity |
| Selection across tables/headers/shapes | Implement incrementally by element wave; tables are Wave 3 |

## Recommended First Implementation Slice

The first low-risk slice should be:

1. scene serialization for read-only pages
2. canvas page painting for visible pages
3. page hit-testing and caret rect API
4. hidden-input bridge for a single caret
5. selection painting for plain paragraphs only

Do not start with tables, comments, shapes, collaborative cursors, or review UI first.

This slice removes the riskiest DOM ownership while keeping the migration bounded.
