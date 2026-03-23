# Phase 4: Editor UX & Feature Parity

## Goal
Close the gap between Rudra Office and production document editors (Google Docs, OnlyOffice, Collabora Online). Move from 65% → 90% by implementing the features and polish that professional users expect.

## Current Gap Assessment

| Area | Current | Target | Key Missing Pieces |
|------|---------|--------|-------------------|
| Comments & Review | 20% | 85% | No threading UI, no resolve, no inline edit |
| Track Changes | 25% | 85% | Accept/reject UI incomplete, no sidebar workflow |
| Tables (doc) | 30% | 80% | No Tab nav, no resize handles, no sort |
| Page Layout | 40% | 85% | No per-section H/F, section breaks stub-only |
| Paste Fidelity | 45% | 80% | No Paste Special, formatting lossy |
| PDF Editing | 40% | 75% | Features exist in WASM but not wired to UI |
| Conflict Awareness | 10% | 70% | No "X is editing here" indicators |
| Mobile Polish | 20% | 60% | No selection handles, no cursor blink |

---

## Key Objectives

### 1. Comments & Track Changes (U-01, U-02) — Critical

**Comments threading:**
- Reply button on each comment → opens inline reply input
- Resolve/unresolve toggle (checkbox or button)
- Click comment to scroll to anchored text
- Comment count badge in status bar
- Sidebar panel showing all comments with filter (all / open / resolved)
- **Files:** `input.js` (comment handlers), `toolbar-handlers.js` (comment panel), `styles.css`
- **WASM:** `insert_comment_reply()`, `resolve_comment()` already exist or need adding

**Track changes:**
- Sidebar showing all pending changes with accept/reject buttons per change
- Accept All / Reject All bulk actions
- Visual diff highlighting in document (insertions green, deletions red strikethrough)
- Navigate between changes (previous/next buttons)
- **Files:** `toolbar-handlers.js:5604-5885` (existing track changes handlers), `file.js:1059-1071`
- **WASM:** `accept_change()`, `reject_change()`, `get_tracked_changes_json()` exist

### 2. Document Tables (U-04, U-07, U-08) — High

**Tab navigation:**
- Tab key moves to next cell, Shift+Tab to previous
- Tab at last cell creates new row
- Arrow keys move between cells when cursor is at cell boundary
- **File:** `input.js` — intercept Tab in `keydown` handler when cursor is inside a `<td>`

**Column resize:**
- Drag handles on column borders (visible on hover)
- Visual resize indicator line during drag
- Minimum column width constraint
- **File:** new handler in `input.js` or separate `table-resize.js`

**Sort:**
- Right-click context menu → Sort Ascending / Sort Descending
- Operates on selected column
- **WASM:** needs `sort_table_column(tableId, colIndex, ascending)` or JS-side sort + reorder ops

### 3. Page Layout (U-03, U-05, U-09) — High

**Per-section headers/footers:**
- Different first page header/footer
- Odd/even page headers/footers
- Section-level header/footer editing
- **WASM:** Section model already supports this, editor rendering doesn't use it
- **Files:** `render.js` (header/footer rendering), `input.js:130-157` (edit mode)

**Footnote/endnote editing:**
- Click footnote reference → scroll to footnote area
- Click footnote text → editable contenteditable region
- Delete footnote reference → removes footnote
- Renumbering on insert/delete
- **WASM:** `insert_footnote()`, `insert_endnote()` exist; need `edit_footnote()`, `delete_footnote()`

**Section breaks:**
- Insert → Section Break submenu (Next Page, Continuous, Even Page, Odd Page)
- Visual indicator in document flow
- **WASM:** `insert_section_break()` exists, editor UI stubs need wiring

### 4. Paste & Clipboard (U-06) — High

**Paste Special dialog:**
- Ctrl+Shift+V opens modal with options:
  - Keep Source Formatting
  - Match Destination Formatting
  - Plain Text Only
  - Values Only (for spreadsheet paste)
- **Files:** `input.js` paste handler, new modal in HTML

### 5. Collaboration UX (U-11) — Medium

**Conflict indicators:**
- When remote user is editing a paragraph, show subtle colored border/badge
- "Alice is typing..." indicator near the affected paragraph
- Fade out after 3 seconds of inactivity
- **Files:** `collab.js` awareness handler, `styles.css` for indicator styles

### 6. PDF Integration (U-15) — High

**Wire `_wasmPdfEditor` on PDF open:**
- Initialize in `file.js` PDF open path
- Gate page operations UI on editor availability
- Free on document switch/close
- **Files:** `file.js:513-573`, `pdf-pages.js`

### 7. Production Polish (U-13, U-14, U-16) — Medium/Low

**Cursor blinking:** CSS animation on primary cursor element
**Import fidelity:** Call `fidelity_report_json()` after open, show toast if placeholders > 0
**Self-host fonts:** Bundle NotoSans + MaterialSymbols, remove Google Fonts/jsDelivr CDN links

---

## Dependencies

| Task | Depends On |
|------|-----------|
| U-01 (Comments) | Phase 1 complete (stable typing) |
| U-02 (Track Changes) | Phase 1 complete |
| U-03 (Per-section H/F) | Phase 2 W-01 (page fragment API) |
| U-04 (Tab nav tables) | None — can start immediately |
| U-05 (Footnote editing) | WASM API additions needed |
| U-11 (Conflict indicators) | Phase 1 H-01 (range-aware ops) |
| U-15 (PDF wiring) | None — can start immediately |

## Estimated Effort

| Priority | Tasks | Estimate |
|----------|-------|----------|
| Critical | U-01, U-02 | 2-3 weeks |
| High | U-03, U-04, U-05, U-06, U-15 | 2-3 weeks |
| Medium | U-07, U-08, U-09, U-11, U-14, U-16 | 2-3 weeks |
| Low | U-10, U-12, U-13, U-17 | 1-2 weeks |
