# Phase 2: WASM Ownership (Layout & Pagination)

## Goal
Shift all visual document logic from the DOM/CSS layer to the Rust/WASM engine. Achieve "True WYSIWYG" where the editor matches the PDF export perfectly.

## Key Objectives

### 1. The Page-Fragment API
Instead of the editor trying to "guess" where a paragraph splits across pages using CSS height, the WASM engine will dictate the fragments.
- **Action:** Engine provides `get_page_fragments(page_index)` which returns a list of NodeIDs and their specific text/layout slices for that page.
- **Action:** Editor renders these fragments as independent DOM nodes per page.

### 2. Eliminating DOM Heuristics
Remove all JS code that measures DOM elements to determine page breaks.
- **Action:** Deprecate `domBasedOverflowSplit()` and `applySplitParagraphClipping()`.
- **Action:** Use WASM-calculated line-heights and glyph-widths for cursor positioning.

### 3. Engine-Driven Typography
- **Action:** Ensure `s1-text` (rustybuzz) is used for shaping in the editor view.
- **Action:** Support Right-to-Left (RTL) and complex scripts (Hindi/Arabic) natively through the engine layout.

### 4. Seamless Pagination
- **Action:** Implement "Line-by-Line" page carry. As a user types, lines should move to the next page individually, not as whole paragraphs.
