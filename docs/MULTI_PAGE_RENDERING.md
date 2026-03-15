# Multi-Page Rendering Architecture

## Context

The current editor uses a **single `contenteditable` div** (`#docPage`) with CSS-injected page breaks via `pagination.js`. This causes critical problems:

1. **Page breaks are editable** — users can delete page dividers, headers, and footers with Backspace/Delete
2. **CSS pages don't align with WASM layout** — the JS pagination dividers don't match `get_page_map_json()` output
3. **Fragile DOM** — decorative elements (`.page-break`, `.page-bottom-spacer`, `.editor-header`, `.editor-footer`) live inside the contenteditable and participate in editing operations

The fix: **separate `contenteditable` per page**, with WASM as the authoritative layout source. Each page is an isolated editing container. Page breaks are structural (between pages), not decorative (inside one page).

---

## Architecture Overview

```
#pageContainer                          ← scroll container (not editable)
  ├─ .doc-page[data-page="1"]           ← visual page (white box, shadow)
  │    ├─ .page-header                  ← header (not editable, rendered from WASM)
  │    ├─ .page-content[contenteditable] ← editable zone, contains only this page's nodes
  │    └─ .page-footer                  ← footer (not editable, rendered from WASM)
  ├─ .doc-page[data-page="2"]
  │    ├─ .page-header
  │    ├─ .page-content[contenteditable]
  │    └─ .page-footer
  └─ ...
```

**Key invariant**: WASM's `get_page_map_json()` is the single source of truth for which nodes belong on which page. JS never decides pagination — it only reconciles DOM to match WASM's assignment.

---

## Files Requiring Changes

| File | Change Type | Description |
|------|-------------|-------------|
| `editor/src/pagination.js` | **Full rewrite** | Replace CSS break injection with per-page DOM container management |
| `editor/src/render.js` | **Major** | Render nodes into correct page containers instead of single div |
| `editor/src/input.js` | **Major** | Event delegation on `#pageContainer`, cross-page editing handlers |
| `editor/src/selection.js` | **Major** | Multi-container selection, cross-page selection overlay |
| `editor/src/styles.css` | **Moderate** | Page container layout, remove old page-break CSS |
| `editor/index.html` | **Moderate** | Replace `#docPage` with `#pageContainer` template |
| `editor/src/state.js` | **Minor** | Add `pageMap`, `pageElements`, `nodeToPage` state fields |
| `editor/src/collab.js` | **Minor** | Update DOM references from `#docPage` to page containers |
| `editor/src/file.js` | **Minor** | Update `newDocument`/`openFile` to use new render path |
| `editor/src/toolbar-handlers.js` | **Minor** | Update any direct `#docPage` references |
| `editor/src/find.js` | **Minor** | Cross-page find/replace highlight |
| `editor/src/ruler.js` | **Minor** | Update page width source |
| `editor/src/images.js` | **Minor** | Image context menu within page containers |

---

## Implementation Phases

### Phase 1: Per-Page Rendering (Foundation)

**Goal**: Replace single contenteditable with multiple page containers. Editing works within a single page. No cross-page editing yet.

#### 1.1 State & DOM Structure

**`state.js`** — Add new state fields:
```js
pageMap: null,        // parsed get_page_map_json() result
pageElements: [],     // array of .doc-page DOM elements
nodeToPage: new Map() // nodeId → pageNumber for O(1) lookup
```

**`index.html`** — Replace:
```html
<!-- OLD -->
<div class="doc-page" id="docPage" contenteditable="true"></div>

<!-- NEW -->
<div id="pageContainer"></div>
```

#### 1.2 Page Manager (`pagination.js` rewrite)

Core functions:

- **`buildPages(pageMap)`** — Creates/updates `.doc-page` elements in `#pageContainer`. Each page gets:
  - `.page-header` (non-editable, innerHTML from WASM header rendering)
  - `.page-content` with `contenteditable="true"`
  - `.page-footer` (non-editable, innerHTML from WASM footer rendering)
  - `data-page` attribute for page number
  - Inline style for width/height from `pageMap.pages[i].width/height`

- **`reconcileNodes(pageMap)`** — For each page in `pageMap.pages`, move/render `[data-node-id]` elements into the correct `.page-content`. Uses `render_node_html()` for nodes not yet in DOM. Moves existing DOM elements if they're on the wrong page (preserves editing state).

- **`repaginate()`** — Calls `doc.get_page_map_json()`, parses result, calls `buildPages()` then `reconcileNodes()`. Updates `state.pageMap`, `state.pageElements`, `state.nodeToPage`.

- **Debounced repagination** — `scheduleRepaginate()` with 300ms debounce. Called after every text sync. Immediate repagination on structural changes (Enter, Delete paragraph, insert table, etc.).

#### 1.3 Render Integration (`render.js` changes)

- **`renderDocument()`** — Instead of `docPage.innerHTML = doc.to_html()`, call `repaginate()` which builds pages and places nodes.
- **`renderNodeById(nodeId)`** — Look up which page the node is on via `state.nodeToPage`, find the element within that page's `.page-content`, update its HTML.
- **`populateNodeIdMap()`** — Scan all `.page-content` containers for `[data-node-id]` elements.
- **`syncParagraphText()`** — Unchanged (operates on individual `[data-node-id]` elements regardless of container).

#### 1.4 Input Delegation (`input.js` changes)

- Move all event listeners from `$('docPage')` to `$('pageContainer')` with event delegation.
- `beforeinput` handler: identify which `.page-content` the event targets. Remove the old page-break deletion prevention (no longer needed).
- `focusin`/`focusout` on `#pageContainer` — track `state.activePageNum` for the currently focused page.

#### 1.5 Selection (`selection.js` changes)

- **`PAGE()`** helper → `activePage()` — returns the currently focused `.page-content` element.
- `getSelectionInfo()` — walk from anchor/focus nodes up to find `[data-node-id]` within whichever `.page-content` contains them.
- `setCursorAtOffset()` — unchanged (works on any element regardless of container).

#### 1.6 CSS (`styles.css` changes)

- Remove `.page-break`, `.page-bottom-spacer`, `.editor-header`, `.editor-footer` styles.
- Add `#pageContainer` styles: gray background, flex column, centered pages with gap.
- `.doc-page`: white background, box-shadow, page dimensions from inline style.
- `.page-header`, `.page-footer`: non-editable zones with border styling.
- `.page-content`: min-height to fill remaining page space.

---

### Phase 2: Cross-Page Editing

**Goal**: Backspace at start of page merges with previous page's last paragraph. Delete at end merges with next page's first paragraph. Arrow keys navigate across page boundaries.

#### 2.1 Cross-Page Backspace/Delete

In `input.js`:
- **Backspace at offset 0 of first node on a page**: Execute `merge_paragraphs()` via WASM (current paragraph with previous page's last paragraph), then `repaginate()` immediately. WASM handles the model change; repagination moves nodes to correct pages.
- **Delete at end of last node on a page**: Same approach — `merge_paragraphs()` with next page's first paragraph.
- Detection: Check if cursor is at start/end of first/last `[data-node-id]` in the current `.page-content`.

#### 2.2 Cross-Page Arrow Navigation

- **ArrowDown at last line of a page**: Move focus to first line of next page's `.page-content`. Use `setCursorAtOffset()` on the first text node.
- **ArrowUp at first line of a page**: Move focus to last line of previous page.
- Detection: Compare cursor's bounding rect with the `.page-content` container's bounding rect. If cursor rect is at the bottom edge, we're at the last line.

#### 2.3 Tab/Shift+Tab in Tables Across Pages

- If Tab at last cell of a table that spans pages, navigate to the continuation on the next page.
- Use `state.nodeToPage` to find where the next cell lives.

---

### Phase 3: Cross-Page Selection

**Goal**: Users can click-drag to select text spanning multiple pages. Selection highlighting works across page boundaries.

#### 3.1 Synthetic Selection Overlay

Since native `Selection` only works within one `contenteditable`, cross-page selection requires a synthetic approach:

- **Selection state** in `state.js`:
  ```js
  crossPageSelection: {
    active: false,
    anchorPageNum: null,
    anchorNodeId: null,
    anchorOffset: null,
    focusPageNum: null,
    focusNodeId: null,
    focusOffset: null
  }
  ```

- **Mousedown** on any `.page-content`: Record anchor (page, node, offset). Set `crossPageSelection.active = false`.
- **Mousemove** (with button down) crossing into a different `.page-content`: Set `crossPageSelection.active = true`. Track focus position. Apply `.cross-selected` CSS class to selected ranges.
- **Mouseup**: If cross-page selection is active, finalize the selection state.

#### 3.2 Selection Highlighting

- For pages fully within the selection range: add `.fully-selected` class to `.page-content` (CSS `::selection`-like background).
- For anchor/focus pages: use Range API within that page to highlight partial selection, plus `mark` elements or CSS custom highlights for the selected portions.

#### 3.3 Formatting Cross-Page Selections

- When user applies formatting (bold, etc.) with an active cross-page selection:
  - Compute the full node range from anchor to focus across pages.
  - Call WASM `format_selection()` with the computed range.
  - Clear cross-page selection state, repaginate, re-render affected nodes.

#### 3.4 Copy/Cut Cross-Page Selections

- Gather text from the selection range across pages.
- Construct clipboard data (plain text + HTML).
- For cut: call WASM delete operations for the range, then repaginate.

---

### Phase 4: Performance & Polish

**Goal**: Smooth editing experience for large documents (100+ pages).

#### 4.1 Page-Level Virtual Scrolling

- Only render `.page-content` innerHTML for pages within the viewport +/- 2 pages.
- Off-screen pages show as empty white boxes with correct dimensions (placeholder).
- On scroll, hydrate entering pages (render nodes), dehydrate leaving pages (clear innerHTML, preserve in WASM model).
- `IntersectionObserver` on each `.doc-page` for efficient viewport tracking.

#### 4.2 Incremental Reconciliation

- When `repaginate()` runs, diff the new `pageMap` against the old one.
- Only move nodes that changed pages. Don't rebuild pages that haven't changed.
- Track `pageMap` version/hash for quick no-op detection.

#### 4.3 Smooth Repagination

- During typing, the current page may overflow. Options:
  - **Eager overflow**: If content exceeds page height, immediately split last node to next page (complex).
  - **Deferred**: Let content overflow briefly, repaginate on debounce (simpler, slight visual glitch).
- Recommended: Deferred approach with a subtle CSS `overflow: hidden` + scroll indicator on the overflowing page content, resolved on repaginate.

#### 4.4 Page Number Display

- Page numbers shown in `.page-footer` via WASM header/footer rendering (already has `{PAGE}` / `{NUMPAGES}` field substitution).
- Also show "Page X of Y" in status bar, updated on repaginate.

---

### Phase 5: Multi-Section Support

**Goal**: Different page sizes/orientations per section (e.g., landscape table section in a portrait document).

- WASM `get_page_map_json()` already returns per-page `width`/`height`.
- Each `.doc-page` gets inline `width`/`height` from the page map — this already supports variable page sizes.
- Section breaks create natural page boundaries.
- Different headers/footers per section already supported by WASM rendering.

---

## Reconciliation Algorithm (Core Logic)

```
repaginate():
  1. pageMap = JSON.parse(doc.get_page_map_json())
  2. oldNodeToPage = state.nodeToPage  // previous assignment
  3. newNodeToPage = new Map()

  4. For each page in pageMap.pages:
       For each nodeId in page.nodeIds:
         newNodeToPage.set(nodeId, page.pageNum)

  5. Ensure correct number of .doc-page elements exist
     (create new pages, remove excess pages)

  6. For each page in pageMap.pages:
       pageEl = state.pageElements[page.pageNum - 1]
       contentEl = pageEl.querySelector('.page-content')

       For each nodeId in page.nodeIds:
         existingEl = state.nodeIdToElement.get(nodeId)
         if existingEl && existingEl.parentElement === contentEl:
           continue  // already in correct page
         else if existingEl:
           contentEl.appendChild(existingEl)  // move to correct page
         else:
           html = doc.render_node_html(nodeId)
           contentEl.insertAdjacentHTML('beforeend', html)

       // Remove nodes that no longer belong on this page
       For each child of contentEl with [data-node-id]:
         if child.dataset.nodeId not in page.nodeIds:
           // node moved to another page or deleted — will be placed by that page's loop

  7. Update headers/footers for each page
  8. state.nodeToPage = newNodeToPage
  9. state.pageMap = pageMap
  10. populateNodeIdMap()  // rebuild nodeIdToElement
```

---

## Migration Strategy

To avoid a big-bang rewrite:

1. **Phase 1 first** — get per-page rendering working with single-page editing. This is the minimum viable change.
2. **Feature flag** — `state.multiPageMode` toggle during development. Old path stays functional until new path is solid.
3. **Phase 2** before Phase 3 — cross-page editing (Backspace/Delete/arrows) is more critical than cross-page selection.
4. **Phase 3 can be simplified initially** — start with "click selects within one page only" and add cross-page selection as a polish step.

---

## Verification Plan

### Phase 1 Verification
- Open a multi-page DOCX -> pages render as separate white boxes with correct dimensions
- Type text on page 1 -> text appears, stays on page 1
- Add enough text to overflow page 1 -> after debounce, last paragraph moves to page 2
- Headers/footers display correctly and are NOT editable (Backspace does nothing)
- Page numbers in footers are correct
- Zoom works with new page containers
- All formatting toolbar actions work within a single page

### Phase 2 Verification
- Backspace at start of page 2 first paragraph -> merges with page 1 last paragraph
- Delete at end of page 1 last paragraph -> merges with page 2 first paragraph
- Arrow down from last line of page 1 -> cursor moves to page 2 first line
- Arrow up from first line of page 2 -> cursor moves to page 1 last line
- Tab from last table cell on page 1 -> moves to continuation on page 2

### Phase 3 Verification
- Click on page 1, drag to page 3 -> text highlighted across all three pages
- Ctrl+A -> all text selected across all pages
- Apply bold to cross-page selection -> formatting applied correctly
- Copy cross-page selection -> clipboard has correct text
- Cut cross-page selection -> text removed, pages repaginate

### Phase 4 Verification
- Open 100-page document -> only visible pages render content
- Scroll quickly -> pages hydrate smoothly without flicker
- Type rapidly -> no lag from repagination (debounced)

---

## Existing Code to Reuse

- **`render_node_html(nodeId)`** — WASM API already exists for per-node rendering (`ffi/wasm/src/lib.rs`)
- **`get_page_map_json()`** — WASM API already returns page-to-node mapping (`ffi/wasm/src/lib.rs`)
- **`syncParagraphText()`** in `render.js` — works on individual nodes, no change needed
- **`state.nodeIdToElement`** map in `state.js` — reusable for O(1) node lookup
- **`merge_paragraphs()`** WASM API — for cross-page Backspace/Delete
- **`split_paragraph()`** WASM API — for Enter creating new paragraphs
- **`format_selection()`** WASM API — for formatting across pages
- **Header/footer HTML** already stored in `state.docHeaderHtml` / `state.docFooterHtml`