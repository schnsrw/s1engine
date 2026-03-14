# Rendering Pipeline & Editor Issues Tracker

## Legend
- [ ] Not started
- [~] In progress
- [x] Fixed

---

## CRITICAL — Rendering Pipeline (Rust)

### C-01: Section nodes silently dropped in layout engine
- **File**: `crates/s1-layout/src/engine.rs` — `collect_body_blocks()`
- **Problem**: Body → Section → Paragraph/Table structure not traversed. Section children dropped by `_ => {}` wildcard.
- **Impact**: All content inside Section containers invisible in layout/PDF/paginated HTML.
- **Status**: [x] Fixed — added recursive `collect_body_blocks` for Section nodes

### C-02: Header/footer rendering only uses first section
- **File**: `ffi/wasm/src/lib.rs` — `to_html()`
- **Problem**: Only `sections[0]` headers/footers rendered. Multi-section docs show wrong headers/footers.
- **Impact**: Wrong headers/footers for multi-section documents.
- **Status**: [x] Fixed — now searches across all sections for best header/footer

### C-03: Empty paragraphs render as zero-height in HTML
- **File**: `ffi/wasm/src/lib.rs` — `render_paragraph()`
- **Problem**: Empty `<p></p>` collapses to zero height. No `<br>` inserted.
- **Impact**: Consecutive Enter presses don't create visible blank lines.
- **Status**: [x] Fixed — added `is_empty_paragraph()` check, inserts `<br>` for empty paragraphs

### C-04: LineBreak inside formatted runs produces malformed HTML
- **File**: `ffi/wasm/src/lib.rs` — `render_run()`
- **Problem**: `<strong><br/></strong>` instead of closing formatting before break.
- **Impact**: Formatting bleeds across line breaks.
- **Status**: [x] Fixed — close formatting tags before `<br/>`, reopen after

### C-05: PageBreak/ColumnBreak nodes ignored in layout
- **File**: `crates/s1-layout/src/engine.rs` — `collect_body_blocks()` and main loop
- **Problem**: PageBreak nodes at body level dropped by `_ => {}`.
- **Impact**: Explicit page breaks don't trigger pagination.
- **Status**: [x] Fixed — added PageBreak handling in collect_body_blocks and main layout loop

### C-06: text_len() returns byte count, not char count
- **File**: `crates/s1-model/src/node.rs` — `text_len()`
- **Problem**: Uses `str.len()` (bytes) instead of `str.chars().count()`.
- **Impact**: Unicode text corruption for non-ASCII text.
- **Status**: [x] Fixed — changed to `t.chars().count()`

### C-07: Multi-run paragraph WASM operations broken
- **File**: `ffi/wasm/src/lib.rs` — split/insert/delete/replace operations
- **Problem**: All text ops only operated on first Run's text node.
- **Impact**: Formatting corrupted when editing paragraphs with multiple runs.
- **Status**: [x] Fixed (previous session) — added `find_text_node_at_char_offset()`

### C-08: Internal clipboard replaces entire document on paste
- **File**: `editor/src/input.js`
- **Problem**: Copy/paste within editor replaced entire document content.
- **Impact**: Data loss on paste.
- **Status**: [x] Fixed (previous session) — disabled broken internal clipboard

---

## HIGH — Layout Engine & Rendering

### H-01: Track changes not rendered in paginated HTML
- **File**: `crates/s1-layout/src/types.rs`, `html.rs`, `style_resolver.rs`, `engine.rs`
- **Problem**: GlyphRun has no revision attributes. `<ins>`/`<del>` not emitted.
- **Impact**: Track changes invisible in Pages view and PDF.
- **Status**: [x] Fixed — added revision_type/revision_author to GlyphRun, renders `<ins>`/`<del>` with color coding

### H-02: Superscript/subscript not in paginated HTML
- **File**: `crates/s1-layout/src/types.rs`, `html.rs`, `engine.rs`
- **Problem**: GlyphRun lacks superscript/subscript fields.
- **Impact**: Sub/superscript text renders as normal in Pages view.
- **Status**: [x] Fixed — added superscript/subscript to GlyphRun, renders vertical-align:super/sub CSS

### H-03: Highlight/background color lost in layout
- **File**: `crates/s1-layout/src/types.rs`, `html.rs`, `style_resolver.rs`, `engine.rs`
- **Problem**: GlyphRun only has foreground `color`, no highlight/background.
- **Impact**: Highlighted text loses background color in Pages/PDF.
- **Status**: [x] Fixed — added highlight_color to GlyphRun, renders background-color CSS

### H-04: Table cell borders/backgrounds hardcoded in paginated HTML
- **File**: `crates/s1-layout/src/html.rs`, `types.rs`, `engine.rs`
- **Problem**: All cells get `border:1px solid #ccc`. Original styling lost.
- **Impact**: Custom table styling lost in paginated view.
- **Status**: [x] Fixed — LayoutTableCell now carries per-side borders and background_color from CellBorders/CellBackground attributes

### H-05: Bookmark anchors at doc top, not actual location
- **File**: `crates/s1-layout/src/html.rs`
- **Problem**: `<a id="...">` emitted before pages, not at bookmark position.
- **Impact**: Internal links jump to top.
- **Status**: [x] Fixed — bookmarks now rendered inside correct page at their Y position with absolute positioning

### H-06: LineBreak nodes ignored in layout engine run processing
- **File**: `crates/s1-layout/src/engine.rs` — `layout_paragraph()`
- **Problem**: LineBreak nodes inside Run children not handled.
- **Impact**: Shift+Enter breaks not rendered in PDF/pages view.
- **Status**: [x] Fixed — added `shape_run_with_breaks()` to split runs at inline LineBreaks

### H-07: Drawing/Field/Bookmark/Comment markers lost in layout
- **File**: `crates/s1-layout/src/engine.rs` — `layout_paragraph()`
- **Problem**: Valid paragraph children dropped by catch-all.
- **Impact**: Inline content lost.
- **Status**: [x] Fixed — explicit handling for BookmarkStart/End, CommentStart/End (skip), Field (render text), Drawing/Image (skip)

### H-08: Layout cache doesn't invalidate on indent changes
- **File**: `crates/s1-layout/src/engine.rs`
- **Problem**: Cache key doesn't include indent/margin context.
- **Impact**: Incorrect line breaking when indents differ.
- **Status**: [x] Fixed — cache hash now includes available_width, indent_left, indent_right, indent_first_line

---

## MEDIUM — Rendering Quality

### M-01: No semantic list HTML in paginated output
- **File**: `crates/s1-layout/src/html.rs`
- **Problem**: Lists render as plain paragraphs. No `<ol>`/`<ul>`/`<li>`.
- **Status**: [ ]

### M-02: No paragraph-level CSS in paginated HTML
- **File**: `crates/s1-layout/src/html.rs`
- **Problem**: No text-align, indentation, borders, or shading.
- **Status**: [ ]

### M-03: Character spacing lost in layout
- **File**: `crates/s1-layout/src/types.rs`, `style_resolver.rs`, `engine.rs`, `html.rs`
- **Status**: [x] Fixed — character_spacing added to GlyphRun, renders letter-spacing CSS

### M-04: Empty paragraph newline handling in extract_text()
- **File**: `crates/s1-model/src/tree.rs`
- **Problem**: Adds newline BEFORE children. Extra blank lines in to_plain_text().
- **Status**: [ ]

### M-05: render_node_html() doesn't handle Section/Body nodes
- **File**: `ffi/wasm/src/lib.rs`
- **Problem**: Content inside sections invisible in editor.
- **Status**: [x] Fixed (was already handled) — Section nodes render their children

### M-06: Page break CSS inconsistent between editor and layout
- **File**: `ffi/wasm/src/lib.rs` vs layout engine
- **Status**: [ ]

---

## Editor UI/UX — Comparison with Google Docs / Collabora / OnlyOffice

### E-01: Find & Replace DOM corruption
- **File**: `editor/src/find.js`
- **Problem**: `surroundContents` fails on cross-element ranges.
- **Status**: [x] Fixed — replaced with safe `extractContents` approach

### E-02: Performance — toolbar state updates on every selectionchange
- **File**: `editor/src/toolbar.js`
- **Problem**: `updateToolbarState()` fires too frequently. Expensive.
- **Status**: [x] Fixed — debounced via `requestAnimationFrame`

### E-03: Performance — status bar word count on every keystroke
- **File**: `editor/src/pagination.js`
- **Problem**: `to_plain_text()` called on every page break update.
- **Status**: [x] Fixed — debounced with 300ms timeout

### E-04: No autosave
- **File**: `editor/src/file.js`
- **Problem**: No periodic save. Data loss on browser crash.
- **Status**: [x] Fixed — 30-second autosave to IndexedDB

### E-05: No beforeunload warning
- **File**: `editor/src/file.js`
- **Problem**: No warning when closing tab with unsaved changes.
- **Status**: [x] Fixed — added `beforeunload` event handler

### E-06: No auto-recovery on startup
- **File**: `editor/src/main.js`
- **Problem**: No way to recover from autosaved document.
- **Status**: [x] Fixed — checks IndexedDB on startup, offers recovery

### E-07: Missing keyboard shortcuts
- **File**: `editor/src/input.js`
- **Problem**: No Shift+Enter (line break), Ctrl+S (save), Escape (close modals).
- **Status**: [x] Fixed (previous session)

### E-08: Table modal missing focus/backdrop/validation
- **File**: `editor/src/toolbar-handlers.js`
- **Problem**: No focus trapping, Escape close, backdrop click, input validation.
- **Status**: [x] Fixed (previous session)

### E-09: URL validation missing for hyperlinks
- **File**: `editor/src/toolbar-handlers.js`
- **Problem**: Invalid URLs not caught. No auto-prepend of `https://`.
- **Status**: [x] Fixed (previous session)

### E-10: No dark mode support
- **File**: `editor/src/styles.css`
- **Status**: [x] Fixed (previous session)

### E-11: No responsive/mobile layout
- **File**: `editor/src/styles.css`
- **Status**: [x] Fixed (previous session)

### E-12: No focus-visible accessibility styles
- **File**: `editor/src/styles.css`
- **Status**: [x] Fixed (previous session)

### E-13: ARIA accessibility gaps
- **File**: `editor/src/toolbar-handlers.js`
- **Problem**: Missing `aria-expanded` on insert menu, no ARIA live regions.
- **Status**: [x] Partially fixed — aria-expanded toggle added

### E-14: Image DPI hardcoded magic number
- **File**: `editor/src/images.js`
- **Problem**: Hardcoded `0.75` multiplier.
- **Status**: [x] Fixed — explicit `72/96` px-to-pt conversion

### E-15: Table cell focus styling missing
- **File**: `editor/src/styles.css`
- **Status**: [x] Fixed — added `focus-within` outline

### E-16: Panel transitions not smooth
- **File**: `editor/src/styles.css`
- **Problem**: Find bar and comments panel appear/disappear abruptly.
- **Status**: [x] Fixed — CSS transitions added

### E-17: Enter in replace field doesn't trigger replace
- **File**: `editor/src/find.js`
- **Status**: [x] Fixed

### E-18: Dirty flag not tracked for autosave
- **File**: `editor/src/state.js`, `render.js`, `input.js`
- **Status**: [x] Fixed — markDirty() called on content changes

### E-19: No real-time collaboration UI
- **Problem**: Missing WebSocket relay, peer cursors, share button.
- **Status**: [ ] Planned (Phase P.8 in roadmap)

### E-20: No version history
- **Status**: [ ]

### E-21: No style gallery / paragraph styles dropdown
- **Status**: [ ]

### E-22: No page ruler
- **Status**: [ ]

### E-23: No spell check integration
- **Status**: [ ]

### E-24: No "/" slash command menu
- **Status**: [ ]

### E-25: No individual track changes accept/reject
- **Status**: [ ]

### E-26: No comment threading/replies
- **Status**: [ ]

### E-27: No virtual scrolling for large documents
- **Status**: [ ]

### E-28: Context menu viewport boundary not checked
- **File**: `editor/src/toolbar-handlers.js`
- **Status**: [x] Fixed (previous session)

---

## LOW — Code Quality

### L-01: HTML escaping duplicated across files
- **File**: `ffi/wasm/src/lib.rs` and `crates/s1-layout/src/html.rs`
- **Status**: [ ]

### L-02: Field rendering code duplicated
- **File**: `ffi/wasm/src/lib.rs`
- **Status**: [ ]

### L-03: No CSS class hierarchy for semantic styling
- **File**: `crates/s1-layout/src/html.rs`
- **Status**: [ ]

---

## Summary

| Category | Total | Fixed | Remaining |
|----------|-------|-------|-----------|
| Critical (Rust) | 8 | 8 | 0 |
| High (Layout) | 8 | 8 | 0 |
| Medium (Rendering) | 6 | 2 | 4 |
| Editor UI/UX | 28 | 19 | 9 |
| Low (Code Quality) | 3 | 0 | 3 |
| **Total** | **53** | **37** | **16** |
