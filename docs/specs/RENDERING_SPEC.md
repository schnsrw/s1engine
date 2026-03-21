# Rendering Specification v1.0

> Covers: rendering pipeline, incremental vs full render, pagination, virtual scrolling, images, tables.
> Last updated: 2026-03-21

## 1. Rendering Architecture

```
WASM Engine                    Browser (DOM)
─────────────────              ──────────────────────
doc.to_html()        ──►      HTML string
  │                              │
  │                         temp.innerHTML = html
  │                              │
  │                         Extract headers/footers
  │                              │
  │                         repaginate()
  │                              │
  │                         Per-page .doc-page containers
  │                              │
  │                         Post-render fixups
  │                              │
doc.get_page_map_json()  ──►   Page map (node→page assignment)
doc.render_node_html(id) ──►   Single node HTML (incremental)
```

### 1.1 Two Rendering Modes

| Mode | Method | Description |
|------|--------|-------------|
| **DOM** (default) | `to_html()` + `repaginate()` | HTML rendered into contenteditable divs, one per page |
| **Canvas** (experimental) | `renderDocumentCanvas()` | HTML5 Canvas rendering, no DOM editing |

Canvas mode is toggled via `setCanvasMode(true)` and falls back to DOM on failure.

## 2. Full Render Pipeline

A full render (`renderDocument()`) executes the following steps:

| Step | Operation | Target |
|------|-----------|--------|
| 1 | Tear down virtual scroll observer | DOM |
| 2 | Mark layout dirty, invalidate cache | State |
| 3 | `doc.to_html()` | WASM |
| 4 | Parse HTML into temp element | DOM |
| 5 | Extract `<header>`, `<footer>` elements (default + first-page) | DOM |
| 6 | Extract footnotes/endnotes sections | DOM |
| 7 | Clear `nodeIdToElement` map | State |
| 8 | Apply page dimensions from WASM | CSS |
| 9 | Clear `pageContainer`, reset page elements | DOM |
| 10 | `repaginate()` — build per-page containers from page map | DOM |
| 11 | Place headers/footers on each page | DOM |
| 12 | Substitute page number fields (`{PageNumber}`, `{PageCount}`) | DOM |
| 13 | Setup images (dimensions, context menu) | DOM |
| 14 | Render equations (MathML/KaTeX) | DOM |
| 15 | Render bookmarks | DOM |
| 16 | Apply column layout if multi-column | CSS |
| 17 | Restore cursor position (see 2.2) | DOM |
| 18 | Setup virtual scroll if large doc | DOM |
| 19 | Update undo/redo button state | UI |
| 20 | Update status bar | UI |
| 21 | Refresh page thumbnails | UI |
| 22 | Refresh track changes panel | UI |
| 23 | Mark `_rendering = false` | State |

### 2.2 Cursor Restoration Algorithm

After a full render, the cursor must be restored to its pre-render position. The algorithm:

```
1. BEFORE render:
   a. Save `cursorNodeId` = data-node-id of the paragraph containing the cursor
   b. Save `cursorOffset` = character offset within that paragraph
   c. Save `cursorParagraphIndex` = index of the paragraph among all paragraphs (fallback)

2. AFTER render (step 17):
   a. Look up element by `[data-node-id="${cursorNodeId}"]`
   b. If found:
      - Call setCursorAtOffset(element, cursorOffset)
      - Clamp offset to element text length if offset > text length
   c. If NOT found (node was deleted/merged by remote edit):
      - Find paragraph at `cursorParagraphIndex` (or last paragraph if index > count)
      - Place cursor at offset 0
   d. If NO cursor info saved (initial render, fullSync from peer):
      - Do not move cursor (let browser default apply)
      - Exception: if document was empty and is now non-empty, place cursor at start

3. FOCUS:
   - Ensure .page-content containing the cursor element has focus
   - Call element.scrollIntoView({ block: 'nearest' }) if cursor is off-screen
```

**Known limitation**: After fullSync from a peer, node IDs may change entirely. The paragraph-index fallback provides a reasonable approximation but is not guaranteed to be correct. This is acceptable because fullSync is infrequent (structural edits only).

### 2.3 When Full Render Triggers

| Trigger | Why |
|---------|-----|
| Document opened/loaded | Initial render |
| Paragraph split (Enter) | Node count changed |
| Paragraph merge (Backspace at start) | Node count changed |
| Multi-line paste | Nodes added |
| Table insert/delete | Structural change |
| Image insert/delete | Structural change |
| Page break insert | Structural change |
| Node deletion | Structural change |
| Undo/redo of structural change | Structural change |
| fullSync received from peer | Entire document replaced |
| Font change (affects layout) | Layout dirty |
| Column layout change | Layout dirty |

## 3. Incremental Render Pipeline

An incremental render (`renderSingleParagraphIfPossible(nodeId)`) updates only one paragraph.

| Step | Operation |
|------|-----------|
| 1 | Check node exists in DOM (`[data-node-id]` lookup) |
| 2 | Check node exists in WASM model (`render_node_html()`) |
| 3 | Save cursor offset within the paragraph |
| 4 | Call `renderNodeById(nodeId)` |
| 5 | `renderNodeById` calls `doc.render_node_html(nodeId)` |
| 6 | Compare new HTML with existing `innerHTML` |
| 7 | If different, update `innerHTML` (or patch attributes only) |
| 8 | Restore cursor at saved offset |
| 9 | Run lightweight fixups (images, undo/redo, status bar) |

### 3.1 When Incremental Render Triggers

| Trigger | Condition |
|---------|-----------|
| Character typed | Single paragraph, no node count change |
| Character deleted (not at boundary) | Within same paragraph |
| Formatting applied (bold, italic, etc.) | Attribute change only |
| Alignment changed | Attribute change only |
| Heading/list style changed | Attribute change only |
| CRDT text op from peer | Single paragraph update |

### 3.2 Incremental Render Fallback

If `renderSingleParagraphIfPossible()` returns `false`, a full `renderDocument()` is required. This happens when:

- The node no longer exists in DOM (was deleted/merged)
- The node no longer exists in WASM (structural change)
- `renderNodeById()` fails

**Pattern:**
```javascript
if (!renderSingleParagraphIfPossible(nodeId)) {
  renderDocument();
}
```

## 4. Pagination

### 4.1 Page Map

Pagination is driven by `doc.get_page_map_json()`, which returns the WASM layout engine's page assignment.

```json
{
  "pages": [
    {
      "pageNumber": 1,
      "nodes": ["0:1", "0:2", "0:3"],
      "tables": [{ "nodeId": "0:4", "startRow": 0, "endRow": 3 }]
    },
    {
      "pageNumber": 2,
      "nodes": ["0:5", "0:6"],
      "tables": [{ "nodeId": "0:4", "startRow": 3, "endRow": 5 }]
    }
  ]
}
```

### 4.2 Page DOM Structure

Each page is a `.doc-page` div:

```
.doc-page[data-page="1"]
  .page-header.hf-hoverable[contenteditable="false"]
  .page-content[contenteditable="true"]
    [paragraph elements with data-node-id]
  .page-footer.hf-hoverable[contenteditable="false"]
```

### 4.3 Page Dimensions

Dimensions come from the WASM document model (section properties) and are applied as inline CSS:

| Property | Source | CSS |
|----------|--------|-----|
| Page width | `state.pageDims.widthPt` | `width: Xpx` (pt * 96/72) |
| Page height | `state.pageDims.heightPt` | `min-height: Xpx` |
| Margins | `state.pageDims.margin*Pt` | padding on `.page-content` |

**Default:** US Letter (612pt x 792pt) with 72pt (1 inch) margins.

### 4.4 Fast-Path Caching

`repaginate()` caches the last page map JSON string (`_lastPageMapHash`). If the new page map is identical to the cached version and DOM pages already exist, the entire DOM reconciliation is skipped.

This cache is invalidated when:
- `_layoutDirty` is set (font change, structural edit)
- `_layoutCache` is cleared

### 4.5 Table Splitting Across Pages

Tables that span page boundaries are chunked by row:
- Each page gets a subset of rows from the same table node
- The table chunk map tracks `{ nodeId, startRow, endRow }` per page
- Table headers can repeat on continuation pages (future)

## 5. Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Keystroke to visible character | <16ms | 1 frame at 60fps; browser contentEditable handles this natively |
| Incremental paragraph re-render | <50ms | Single `render_node_html()` + DOM patch |
| Full document render (10 pages) | <200ms | `to_html()` + `repaginate()` + fixups |
| Full document render (100 pages) | <2000ms | With virtual scrolling enabled |
| `to_html()` WASM call | <50ms | For 10-page document |
| `repaginate()` DOM build | <100ms | For 10-page document |
| Page map JSON parse | <5ms | Typically <10KB JSON |
| Virtual scroll intersection check | <2ms | IntersectionObserver callback |

## 6. Virtual Scrolling

### 6.1 Activation

Virtual scrolling activates automatically for large documents:

| Threshold | Value |
|-----------|-------|
| Paragraph count | > 500 |
| Page count | > 500 |
| WASM memory | > 50MB |

### 6.2 Buffer Zone

```
BUFFER_PAGES = 2

Viewport:
  ┌──────────────────────┐
  │   Buffer (2 pages)   │  ← Pre-rendered above viewport
  │ ──────────────────── │
  │   Visible pages       │  ← Fully rendered
  │ ──────────────────── │
  │   Buffer (2 pages)   │  ← Pre-rendered below viewport
  └──────────────────────┘
  Pages outside buffer:
  - Content replaced with empty placeholder
  - Height preserved (no layout shift)
  - Images replaced with 1x1 transparent GIF
```

### 6.3 IntersectionObserver

An `IntersectionObserver` monitors page visibility. When a page enters the buffer zone, its content is restored from cache. When it exits, content is replaced with a placeholder.

### 6.4 Image Lazy Loading

Off-screen images use a transparent 1x1 pixel placeholder:
```
data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7
```

Original `src` is stored in `data-original-src` and restored when the page enters the viewport.

### 6.5 Scroll Concurrency Guard

A `_rendering` flag prevents concurrent render operations during rapid scrolling. If `renderDocument()` is called while `_rendering === true`, the call is queued or dropped.

## 7. Paragraph Rendering

### 7.1 Standard Paragraph

```html
<p data-node-id="0:5" style="margin-bottom:8pt">
  <span style="font-family:'Times New Roman';font-size:12pt">Hello </span>
  <span style="font-family:'Times New Roman';font-size:12pt;font-weight:bold">world</span>
</p>
```

### 7.2 Empty Paragraph

Empty paragraphs MUST render with a `<br>` to maintain line height and provide a cursor anchor:

```html
<p data-node-id="0:7"><br></p>
```

**Requirements:**
- Minimum height: one line (matching current font size + line spacing)
- Cursor MUST be visible at the left edge
- `<br>` is replaced by text content when user types
- No visible flicker during the transition

### 7.3 Heading Paragraph

```html
<h2 data-node-id="0:3" style="font-size:16pt;font-weight:bold">
  <span>Section Title</span>
</h2>
```

### 7.4 List Paragraph

```html
<p data-node-id="0:9" data-list-level="0" data-list-type="bullet" class="list-item">
  <span class="list-marker" contenteditable="false">&#8226;</span>
  <span>List item text</span>
</p>
```

List markers are `contenteditable="false"` to prevent editing of the marker itself.

## 8. Image Rendering

### 8.1 Inline Images

Images are rendered as `<img>` with base64 data URIs or blob URLs:

```html
<img data-node-id="0:12"
     src="data:image/png;base64,..."
     style="width:200pt;height:150pt;max-width:100%"
     alt="Image description"
     draggable="false" />
```

### 8.2 Image Dimensions

| Source | Behavior |
|--------|----------|
| Width + height in model | Render at specified dimensions |
| Width only | Calculate height from aspect ratio |
| Height only | Calculate width from aspect ratio |
| No dimensions | `max-width: 100%`, natural size |

### 8.3 Image Alignment

Image alignment is controlled by the parent paragraph's `text-align` property:

| Alignment | CSS on paragraph |
|-----------|-----------------|
| Left (default) | `text-align: left` |
| Center | `text-align: center` |
| Right | `text-align: right` |

### 8.4 Image Context Menu

Right-clicking an image shows a custom context menu with:
- Align Left / Center / Right
- Resize (Small / Medium / Large / Original)
- Delete
- Alt text editing

### 8.5 Image in Collaboration

Image operations (insert, resize, align, delete) trigger an immediate `fullSync` because they are structural changes.

## 9. Table Rendering

### 9.1 Table Structure

```html
<table data-node-id="0:20" style="width:100%;border-collapse:collapse">
  <tr data-node-id="0:21">
    <td data-node-id="0:22" style="border:1px solid #d0d0d0;padding:4pt">
      <p data-node-id="0:23">Cell content</p>
    </td>
    <td data-node-id="0:24" style="border:1px solid #d0d0d0;padding:4pt">
      <p data-node-id="0:25">Cell content</p>
    </td>
  </tr>
</table>
```

### 9.2 Table Edge Cases

| # | Scenario | Expected Behavior |
|---|----------|-------------------|
| 9.2.1 | Empty table cell | Cell contains one empty paragraph with `<br>` |
| 9.2.2 | Very wide content in cell | Cell expands; table may overflow page width with horizontal scroll |
| 9.2.3 | Nested table | Inner table rendered inside cell; editing supported but complex |
| 9.2.4 | Table spanning pages | Split by row at page boundary; header row repeats (future) |
| 9.2.5 | Single-cell table | Renders as a bordered box (sometimes used for callouts) |
| 9.2.6 | Merged cells | `colspan` / `rowspan` attributes applied |
| 9.2.7 | Table with no borders | No border CSS; cell boundaries invisible to user |
| 9.2.8 | Table wider than page | `max-width: 100%` with `overflow-x: auto` on container |
| 9.2.9 | Cell with image | Image constrained to cell width |
| 9.2.10 | Deeply nested table (3+ levels) | Supported but discouraged; performance degrades |

### 9.3 Table Cell Announcements

For accessibility, table cells announce their position (row, column) via ARIA attributes when focused. This is initialized by `initTableCellAnnouncements()`.

## 10. Header/Footer Rendering

### 10.1 Structure

Headers and footers are extracted from the WASM HTML output and placed in non-editable containers:

```html
<div class="page-header hf-hoverable" contenteditable="false"
     data-hf-kind="header" title="Double-click to edit header">
  <!-- Header content from WASM -->
</div>
```

### 10.2 Different First Page

The document may have different headers/footers for the first page:

| Page | Header Source | Footer Source |
|------|--------------|---------------|
| Page 1 (if different first) | `docFirstPageHeaderHtml` | `docFirstPageFooterHtml` |
| All other pages | `docHeaderHtml` | `docFooterHtml` |

### 10.3 Page Number Substitution

Field elements with `data-field="PageNumber"` or `data-field="PageCount"` are substituted per page during pagination.

## 11. Implementation Status

| Feature | Status |
|---------|--------|
| Full render pipeline | DONE |
| Incremental single-paragraph render | DONE |
| WASM page map pagination | DONE |
| Virtual scrolling (IntersectionObserver) | DONE |
| Image rendering (inline, alignment) | DONE |
| Table rendering | DONE |
| Header/footer with different first page | DONE |
| Page number field substitution | DONE |
| Canvas rendering mode | DONE (experimental) |
| Empty paragraph with `<br>` | DONE |
| Table splitting across pages | DONE |
| Nested table rendering | DONE |
| Incremental repagination (only reflow after edit point) | SPECIFIED (Section 12) |
| WASM dirty flags for changed paragraphs | SPECIFIED (Section 13) |
| Table header row repeat on continuation pages | SPECIFIED (Section 14) |
| Float/wrap image modes | SPECIFIED (Section 15) |

## 12. Incremental Repagination

### 12.1 Overview

Current `repaginate()` rebuilds all page containers from scratch on every call. For a single-paragraph edit in a 100-page document, this is wasteful -- only pages at or after the edit point can possibly be affected. Incremental repagination skips DOM reconstruction for pages before the edit point.

### 12.2 Algorithm

```
Input:
  - editedPageIndex: the 0-based page index containing the edited paragraph
    (derived from the paragraph's data-node-id lookup in the previous page map)
  - newPageMap: the fresh page map JSON from WASM (doc.get_page_map_json())
  - oldPageMap: the cached page map from the last repagination

Steps:
  1. Compare newPageMap.pages[0..editedPageIndex] with oldPageMap.pages[0..editedPageIndex].
     - If they are IDENTICAL (same node lists, same table chunks): skip to step 3.
     - If they DIFFER: fall back to full repagination (step 6).

  2. (Validation) For each page i < editedPageIndex:
     - Verify DOM page element `.doc-page[data-page="${i+1}"]` exists.
     - If any page is missing from DOM: fall back to full repagination.

  3. For each page i >= editedPageIndex:
     - If i < newPageMap.pages.length AND i < oldPageMap.pages.length:
       a. Compare node lists. If identical, skip this page (no DOM change).
       b. If different, rebuild this page's .page-content from newPageMap.
     - If i >= oldPageMap.pages.length (new page added):
       a. Create new .doc-page element with header, footer, page-content.
       b. Populate page-content with nodes from newPageMap.pages[i].
     - If i >= newPageMap.pages.length (page removed):
       a. Remove the .doc-page element from DOM.

  4. Update page numbers in headers/footers for all pages >= editedPageIndex.
     Substitute {PageNumber} and {PageCount} fields.

  5. Update _lastPageMapHash cache with newPageMap.

  6. FULL FALLBACK: Clear all pages, rebuild from scratch (existing behavior).
```

### 12.3 When to Use Incremental vs Full

| Condition | Mode |
|-----------|------|
| Single paragraph text edit (type/delete/format) | Incremental from edited page |
| CRDT remote op affecting single paragraph | Incremental from affected page |
| Paragraph split (Enter) | Incremental from split page |
| Paragraph merge (Backspace) | Incremental from merge page |
| Table cell edit | Incremental from table's first page |
| fullSync received | Full repagination (node IDs may all change) |
| Document open/load | Full repagination |
| Multi-paragraph paste | Full repagination |
| Font/column layout change | Full repagination |
| Page map differs before edit point | Full repagination (fallback) |

### 12.4 Data Structures

```javascript
// Stored in module state:
let _previousPageMap = null;  // Parsed JSON of last page map

// Per-invocation:
function repaginateIncremental(editedNodeId) {
  const newPageMapJson = state.doc.get_page_map_json();
  const newPageMap = JSON.parse(newPageMapJson);

  // Find which page the edited node is on
  let editedPageIndex = 0;
  for (let i = 0; i < newPageMap.pages.length; i++) {
    if (newPageMap.pages[i].nodes.includes(editedNodeId)) {
      editedPageIndex = i;
      break;
    }
  }

  // Check if pages before edit point are unchanged
  if (_previousPageMap && pagesMatchBefore(editedPageIndex, _previousPageMap, newPageMap)) {
    // Incremental path
    reconcilePagesFrom(editedPageIndex, _previousPageMap, newPageMap);
  } else {
    // Full path
    repaginateFull(newPageMap);
  }

  _previousPageMap = newPageMap;
}

function pagesMatchBefore(pageIndex, oldMap, newMap) {
  if (pageIndex > oldMap.pages.length) return false;
  for (let i = 0; i < pageIndex; i++) {
    if (JSON.stringify(oldMap.pages[i]) !== JSON.stringify(newMap.pages[i])) {
      return false;
    }
  }
  return true;
}
```

### 12.5 Performance Impact

| Document Size | Full Repagination | Incremental (edit on last page) | Speedup |
|---------------|-------------------|---------------------------------|---------|
| 10 pages | ~30ms | ~5ms | 6x |
| 50 pages | ~120ms | ~8ms | 15x |
| 100 pages | ~250ms | ~10ms | 25x |
| 500 pages | ~1200ms | ~15ms | 80x |

The improvement is proportional to the edit position: editing the first page gains nothing (all pages must be checked), while editing the last page skips almost all DOM work.

### 12.6 Implementation Priority

**HIGH** -- This is the single largest rendering optimization remaining. It directly impacts the perceived responsiveness of Enter/Backspace/structural edits in long documents.

## 13. WASM Dirty Flags for Changed Paragraphs

### 13.1 Overview

When a remote CRDT operation arrives, the editor currently does a full `renderDocument()` or calls `renderNodeById()` on the affected node (if known from the CRDT op). However, some CRDT operations (batch ops, undo, multi-node changes) affect multiple paragraphs. The editor needs to know exactly which paragraphs changed so it can update only those.

### 13.2 WASM API

```rust
// In WasmDocument or WasmCollabDocument:

/// Returns a JSON array of node IDs that have been modified since the last
/// call to this function. Each call clears the dirty set.
///
/// Returns: `["0:5", "0:12", "0:47"]`
#[wasm_bindgen]
pub fn get_dirty_paragraphs_json(&mut self) -> String {
    let ids: Vec<String> = self.dirty_nodes.drain().map(|id| id.to_string()).collect();
    serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string())
}

/// Returns true if any paragraphs have been marked dirty since the last check.
#[wasm_bindgen]
pub fn has_dirty_paragraphs(&self) -> bool {
    !self.dirty_nodes.is_empty()
}

/// Manually marks a node as dirty (useful after programmatic edits).
#[wasm_bindgen]
pub fn mark_node_dirty(&mut self, node_id: &str) {
    if let Ok(id) = NodeId::parse(node_id) {
        self.dirty_nodes.insert(id);
    }
}
```

### 13.3 Dirty Tracking Implementation (Rust Side)

```rust
// In the Document or CollabDocument struct:
use std::collections::HashSet;

struct DocumentInner {
    // ...existing fields...
    dirty_nodes: HashSet<NodeId>,
}

// Every mutation that modifies a node marks it dirty:
impl DocumentInner {
    fn apply_operation(&mut self, op: &Operation) -> Result<(), Error> {
        // ...existing apply logic...
        match op {
            Operation::InsertText { node_id, .. } => {
                self.dirty_nodes.insert(*node_id);
            }
            Operation::DeleteText { node_id, .. } => {
                self.dirty_nodes.insert(*node_id);
            }
            Operation::SetAttribute { node_id, .. } => {
                self.dirty_nodes.insert(*node_id);
            }
            Operation::SplitNode { node_id, new_node_id, .. } => {
                self.dirty_nodes.insert(*node_id);
                self.dirty_nodes.insert(*new_node_id);
            }
            Operation::MergeNode { target_node_id, source_node_id, .. } => {
                self.dirty_nodes.insert(*target_node_id);
                self.dirty_nodes.insert(*source_node_id);
            }
            // ...other operation types...
        }
        Ok(())
    }
}
```

### 13.4 Editor Usage (JavaScript Side)

```javascript
// After receiving and applying remote CRDT ops:
function handleRemoteCrdtOp(opJson) {
  try {
    state.collabDoc.apply_remote_ops(opJson);
  } catch (e) {
    console.warn("Remote CRDT op failed:", e);
    return;
  }

  // Check what changed
  const dirtyJson = state.doc.get_dirty_paragraphs_json();
  const dirtyIds = JSON.parse(dirtyJson);

  if (dirtyIds.length === 0) {
    return; // Nothing visually changed
  }

  if (dirtyIds.length <= 3) {
    // Small number of changes — incremental render each
    for (const nodeId of dirtyIds) {
      renderNodeById(nodeId);
    }
    // Check if repagination is needed (paragraph height may have changed)
    repaginateIncremental(dirtyIds[0]);
  } else {
    // Many paragraphs changed — full render is more efficient
    renderDocument();
  }
}
```

### 13.5 Dirty Flag Lifecycle

```
                    ┌──────────┐
                    │  CLEAN   │ (dirty_nodes is empty)
                    └────┬─────┘
                         │
              WASM operation applied
              (insert, delete, format, CRDT)
                         │
                         ▼
                    ┌──────────┐
                    │  DIRTY   │ (dirty_nodes has entries)
                    └────┬─────┘
                         │
              get_dirty_paragraphs_json() called
              (drains the set, returns IDs)
                         │
                         ▼
                    ┌──────────┐
                    │  CLEAN   │ (ready for next batch)
                    └──────────┘
```

### 13.6 Edge Cases

| Scenario | Behavior |
|----------|----------|
| Multiple ops before dirty check | All affected nodes accumulate in the set; single drain returns all |
| Node deleted after being marked dirty | ID remains in set; JS side checks if DOM element exists before rendering |
| fullSync replaces entire document | All nodes are dirty; `has_dirty_paragraphs()` returns true; editor should do full render instead of checking individual nodes |
| No ops applied | `get_dirty_paragraphs_json()` returns `[]` |
| Same node modified multiple times | Set deduplicates; node ID appears only once |

### 13.7 Performance Impact

| Scenario | Without Dirty Flags | With Dirty Flags | Improvement |
|----------|---------------------|-------------------|-------------|
| Single remote char insert | Full render (200ms) or guess-based incremental | Exact 1-node incremental (5ms) | 40x |
| Remote format change | Full render (200ms) | 1-node incremental (5ms) | 40x |
| Remote batch of 3 ops | Full render (200ms) | 3-node incremental (15ms) | 13x |
| Remote fullSync | Full render (200ms) | Full render (200ms) | 1x (no change) |

### 13.8 Implementation Priority

**HIGH** -- Directly improves collaborative editing responsiveness. When peer A types, peer B should see near-instant updates via targeted re-rendering rather than full document re-renders.

## 14. Table Header Row Repeat on Continuation Pages

### 14.1 Overview

When a table spans multiple pages, the first row (designated as the header row) should repeat at the top of each continuation page. This improves readability for large tables, matching the behavior of Word, Google Docs, and the OOXML `<w:tblHeader/>` element.

### 14.2 Data Model

The table header row property is stored in the document model:

```rust
// In s1-model, TableRow attributes:
AttributeKey::TableHeaderRow  // AttributeValue::Bool(true) marks a row as header

// OOXML source: <w:trPr><w:tblHeader/></w:trPr>
// ODF source: <table:table-header-rows>...</table:table-header-rows>
```

Multiple consecutive rows can be marked as header rows (e.g., a two-row header with category + column names).

### 14.3 WASM Page Map Extension

The page map JSON already includes table chunk information. For header row repeat, the page map adds a `headerRows` field:

```json
{
  "pages": [
    {
      "pageNumber": 1,
      "nodes": ["0:1"],
      "tables": [{
        "nodeId": "0:10",
        "startRow": 0,
        "endRow": 5,
        "headerRows": [0]
      }]
    },
    {
      "pageNumber": 2,
      "nodes": [],
      "tables": [{
        "nodeId": "0:10",
        "startRow": 5,
        "endRow": 10,
        "headerRows": [0],
        "isHeaderRepeat": true
      }]
    }
  ]
}
```

### 14.4 HTML Rendering

On continuation pages, header rows are cloned and inserted before the table chunk's body rows:

```html
<!-- Page 2: continuation of table 0:10 -->
<table data-node-id="0:10" data-page-chunk="2" style="width:100%;border-collapse:collapse">
  <thead class="repeated-header" contenteditable="false" aria-hidden="true">
    <tr data-node-id="0:11" data-header-repeat="true"
        style="background:#f5f5f5;border-bottom:2px solid #999">
      <td>Column A</td>
      <td>Column B</td>
    </tr>
  </thead>
  <tbody>
    <tr data-node-id="0:16">
      <td>Row 6 data</td>
      <td>Row 6 data</td>
    </tr>
    <!-- ...more rows... -->
  </tbody>
</table>
```

### 14.5 CSS Styling

```css
/* Repeated header rows on continuation pages */
.repeated-header {
  display: table-header-group;
}

.repeated-header tr {
  break-inside: avoid;
  background-color: #f5f5f5; /* Subtle distinction from body rows */
  border-bottom: 2px solid #999;
}

/* Prevent editing of repeated headers (they are clones) */
.repeated-header [contenteditable] {
  pointer-events: none;
}

/* Print support */
@media print {
  thead.repeated-header {
    display: table-header-group; /* Browser repeats thead on each printed page */
  }
}
```

### 14.6 Rendering Algorithm

```javascript
function renderTableOnPage(tableNode, pageChunk) {
  const table = document.createElement("table");
  table.setAttribute("data-node-id", tableNode.nodeId);
  table.style.cssText = "width:100%;border-collapse:collapse";

  // If this is a continuation page AND the table has header rows
  if (pageChunk.isHeaderRepeat && pageChunk.headerRows?.length > 0) {
    const thead = document.createElement("thead");
    thead.className = "repeated-header";
    thead.contentEditable = "false";
    thead.setAttribute("aria-hidden", "true");

    for (const headerRowIndex of pageChunk.headerRows) {
      // Clone the header row from the original table rendering
      const originalHeaderRow = getOriginalHeaderRow(tableNode.nodeId, headerRowIndex);
      if (originalHeaderRow) {
        const clone = originalHeaderRow.cloneNode(true);
        clone.setAttribute("data-header-repeat", "true");
        // Make all cells non-editable in the clone
        clone.querySelectorAll("td, th").forEach(cell => {
          cell.contentEditable = "false";
        });
        thead.appendChild(clone);
      }
    }
    table.appendChild(thead);
  }

  // Render body rows for this page's chunk
  const tbody = document.createElement("tbody");
  for (let r = pageChunk.startRow; r < pageChunk.endRow; r++) {
    tbody.appendChild(renderTableRow(tableNode, r));
  }
  table.appendChild(tbody);

  return table;
}
```

### 14.7 Editing Behavior

| Action | Behavior |
|--------|----------|
| Click on repeated header | No cursor placement (contenteditable=false) |
| Edit original header (page 1) | Changes propagate to all repeated headers on next repagination |
| Tab navigation in table | Skips repeated header cells; jumps to first body row |
| Select All in table | Does not include repeated header rows in selection |
| Copy table | Does not duplicate header rows in clipboard |

### 14.8 Edge Cases

| Scenario | Behavior |
|----------|----------|
| Single-row table (header only) | No body rows on continuation; just the repeated header (unusual but valid) |
| Multiple header rows (2-3 rows) | All header rows repeat on each continuation page |
| Header row taller than page | Header is not repeated (would leave no room for body rows); table continues without repeated header |
| Table with no header rows marked | No repetition; table chunks render body rows only (current behavior) |
| Nested table with header rows | Inner table's header rows repeat within the cell (if the cell spans pages) |

### 14.9 Implementation Priority

**MEDIUM** -- Important for data-heavy documents with large tables. Does not affect the common case of short tables that fit on a single page.

## 15. Float/Wrap Image Modes

### 15.1 Overview

Currently, all images render inline (within the text flow). Float/wrap modes allow images to be positioned outside the normal text flow, with text wrapping around them. This matches OOXML's `<wp:anchor>` positioning and ODF's frame anchoring.

### 15.2 Image Position Modes

| Mode | OOXML Source | ODF Source | CSS Implementation |
|------|-------------|------------|-------------------|
| **Inline** (default) | `<wp:inline>` | `<draw:frame text:anchor-type="as-char">` | `display: inline-block` (current behavior) |
| **Wrap Left** | `<wp:anchor>` + `<wp:wrapSquare wrapText="right"/>` | `<draw:frame text:anchor-type="paragraph" style:wrap="left">` | `float: left` |
| **Wrap Right** | `<wp:anchor>` + `<wp:wrapSquare wrapText="left"/>` | `<draw:frame text:anchor-type="paragraph" style:wrap="right">` | `float: right` |
| **Wrap Both** | `<wp:anchor>` + `<wp:wrapSquare wrapText="bothSides"/>` | `<draw:frame style:wrap="parallel">` | `float: left` (text wraps on right side; true both-sides wrap requires CSS Shapes) |
| **Top and Bottom** | `<wp:anchor>` + `<wp:wrapTopAndBottom/>` | `<draw:frame style:wrap="none">` | `display: block; clear: both` |
| **Behind Text** | `<wp:anchor>` + `<wp:wrapNone/>` + `behindDoc="1"` | `<draw:frame style:wrap="run-through" style:run-through="background">` | `position: absolute; z-index: -1` |
| **In Front of Text** | `<wp:anchor>` + `<wp:wrapNone/>` + `behindDoc="0"` | `<draw:frame style:wrap="run-through" style:run-through="foreground">` | `position: absolute; z-index: 10` |

### 15.3 Data Model

```rust
// In s1-model attributes:
AttributeKey::ImageWrapMode    // AttributeValue::String("inline"|"wrapLeft"|"wrapRight"|
                               //   "wrapBoth"|"topAndBottom"|"behindText"|"inFrontOfText")

AttributeKey::ImageOffsetX     // AttributeValue::Float(f64) — horizontal offset in points
AttributeKey::ImageOffsetY     // AttributeValue::Float(f64) — vertical offset in points
AttributeKey::ImageDistanceTop    // AttributeValue::Float(f64) — text distance from top edge (pt)
AttributeKey::ImageDistanceBottom // AttributeValue::Float(f64) — text distance from bottom edge (pt)
AttributeKey::ImageDistanceLeft   // AttributeValue::Float(f64) — text distance from left edge (pt)
AttributeKey::ImageDistanceRight  // AttributeValue::Float(f64) — text distance from right edge (pt)
AttributeKey::ImageAnchorH     // AttributeValue::String("column"|"page"|"margin")
AttributeKey::ImageAnchorV     // AttributeValue::String("paragraph"|"page"|"margin")
```

### 15.4 HTML Output from WASM

The `to_html()` output includes wrap-mode-specific CSS:

```html
<!-- Inline (current, unchanged) -->
<img data-node-id="0:12" src="data:image/png;base64,..."
     style="width:200pt;height:150pt;max-width:100%"
     data-wrap="inline" />

<!-- Wrap Left -->
<img data-node-id="0:12" src="data:image/png;base64,..."
     style="float:left;width:200pt;height:150pt;margin:4pt 8pt 4pt 0"
     data-wrap="wrapLeft" />

<!-- Wrap Right -->
<img data-node-id="0:12" src="data:image/png;base64,..."
     style="float:right;width:200pt;height:150pt;margin:4pt 0 4pt 8pt"
     data-wrap="wrapRight" />

<!-- Top and Bottom -->
<div style="clear:both;text-align:center">
  <img data-node-id="0:12" src="data:image/png;base64,..."
       style="display:block;width:200pt;height:150pt;margin:8pt auto"
       data-wrap="topAndBottom" />
</div>

<!-- Behind Text -->
<img data-node-id="0:12" src="data:image/png;base64,..."
     style="position:absolute;left:72pt;top:144pt;width:200pt;height:150pt;z-index:-1;pointer-events:none"
     data-wrap="behindText" />

<!-- In Front of Text -->
<img data-node-id="0:12" src="data:image/png;base64,..."
     style="position:absolute;left:72pt;top:144pt;width:200pt;height:150pt;z-index:10"
     data-wrap="inFrontOfText" />
```

### 15.5 CSS for Text Distance (Margins)

Text distance values from the document model map to CSS margins on the image element:

```css
/* Wrap modes use margin for text distance */
img[data-wrap="wrapLeft"] {
  margin-top: var(--img-dist-top, 4pt);
  margin-right: var(--img-dist-right, 8pt);
  margin-bottom: var(--img-dist-bottom, 4pt);
  margin-left: var(--img-dist-left, 0);
}

img[data-wrap="wrapRight"] {
  margin-top: var(--img-dist-top, 4pt);
  margin-right: var(--img-dist-right, 0);
  margin-bottom: var(--img-dist-bottom, 4pt);
  margin-left: var(--img-dist-left, 8pt);
}
```

In practice, the WASM `to_html()` inlines these values directly from the model attributes. The CSS custom properties above are defaults when no explicit distance is set.

### 15.6 Page-Content Container Requirements

For absolutely-positioned images (behind/in front of text), the `.page-content` container must be a positioning context:

```css
.page-content {
  position: relative; /* Required for absolute image positioning */
  overflow: hidden;   /* Prevent images from bleeding outside page */
}
```

### 15.7 Editor Interactions

| Action | Behavior |
|--------|----------|
| Click on float image | Show resize handles and alignment toolbar |
| Drag float image | Update `ImageOffsetX`/`ImageOffsetY` in model; re-render |
| Right-click float image | Context menu includes "Wrap Text" submenu with all modes |
| Delete float image | Remove node; clear float forces repagination |
| Select text near float | Text selection flows around the float (browser native behavior) |
| Type text near float | Text reflows around float (browser native behavior for CSS float) |

### 15.8 Context Menu Extension

The existing image context menu (Section 8.4) gains a "Wrap Text" submenu:

```
Right-click image:
  Align Left
  Align Center
  Align Right
  ─────────────
  Wrap Text ►  In Line with Text    [current: check mark]
               Square (Left)
               Square (Right)
               Top and Bottom
               Behind Text
               In Front of Text
  ─────────────
  Resize ►
  Delete
  Alt Text...
```

### 15.9 Pagination Impact

Float images affect page layout:

| Mode | Pagination Impact |
|------|-------------------|
| Inline | Image height added to paragraph line height; may cause page break |
| Float Left/Right | Image occupies space alongside text; text reflows; may push content to next page |
| Top and Bottom | Image height + margins added to vertical flow; clear:both may create whitespace |
| Behind/In Front | No impact on text flow (absolutely positioned); however, must be placed on correct page based on anchor paragraph |

For the WASM layout engine, float images require tracking the available text width per line segment (reduced by the float's width + margins). This is handled by the `s1-layout` engine's line-breaking algorithm, which accounts for floats as exclusion zones.

### 15.10 Edge Cases

| Scenario | Behavior |
|----------|----------|
| Float image wider than page content area | Image clamped to `max-width: 100%` of page content width |
| Two float-left images adjacent | Stack vertically (CSS float behavior) |
| Float-left + float-right on same paragraph | Both float; text wraps between them if space permits |
| Float image in table cell | Float is constrained to cell width; text in same cell wraps around it |
| Behind-text image overlapping text | Image renders behind; text remains readable (image should be faded/watermark) |
| Print mode | Float CSS properties preserved; `position:absolute` images use `@media print` rules |

### 15.11 Implementation Priority

**MEDIUM** -- Important for DOCX/ODT fidelity (many real-world documents use wrapped images). Not required for basic editing functionality.


## 16. Page Number Fields

### 16.1 Field Rendering
WASM outputs page number fields as: `<span class="field" data-field="PageNumber">PAGE</span>`

Pagination JS substitutes the placeholder text with actual page numbers via `substitutePageNumbers(container, pageNum, totalPages)`.

### 16.2 Substitution Points
Page number substitution MUST run on:
1. **Header elements** (`.page-header`) — after `innerHTML = headerHtml`
2. **Footer elements** (`.page-footer`) — after `innerHTML = footerHtml`
3. **Body content** (`.page-content`) — after content reconciliation

### 16.3 Supported Fields
| `data-field` value | Substituted with |
|---------------------|-----------------|
| `PageNumber`, `PAGE` | Current page number (1-indexed) |
| `PageCount`, `NUMPAGES` | Total page count |
| `Date`, `DATE` | Current date (future) |
| `Time`, `TIME` | Current time (future) |

### 16.4 Edge Cases
| # | Scenario | Expected Behavior |
|---|----------|-------------------|
| 16.4.1 | Cut/paste changes page count | All field spans updated on next pagination |
| 16.4.2 | Field in body content (not header) | Substituted during body content pass |
| 16.4.3 | Multiple fields in same header | Each independently substituted |
| 16.4.4 | Header from fullSync has stale number | `innerHTML = headerHtml` resets to WASM output (with placeholder), then substitutes correctly |
| 16.4.5 | Fast-path (page map unchanged) | Skip — numbers don't change if page count unchanged |
| 16.4.6 | Field in collab — peer sees correct numbers | Each peer runs substitution independently on their local page layout |

