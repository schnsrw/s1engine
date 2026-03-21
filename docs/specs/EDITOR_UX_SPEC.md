# Editor UX Specification v1.0

> Covers: cursor behavior, selection, typing, empty lines, images, clipboard, undo/redo.
> Reference: Google Docs, Microsoft Word Online, OnlyOffice behavior.

## 1. Cursor Behavior

### 1.1 Cursor in empty paragraph
**Spec**: Cursor MUST be visible as a blinking caret at the left edge of an empty paragraph. The paragraph MUST have a minimum height of one line (matching the current font size).

**Implementation**:
- Empty paragraph rendered with `<br>` element to ensure line height
- `setCursorAtOffset(el, 0)` places cursor at start of the element
- `.page-content` is focused before placing cursor

**Edge cases**:
| # | Scenario | Expected Behavior |
|---|----------|-------------------|
| 1.1.1 | New empty document | Cursor at start of first paragraph |
| 1.1.2 | After cutting all content | New empty paragraph created, cursor at start |
| 1.1.3 | After deleting last character | Cursor at start of now-empty paragraph |
| 1.1.4 | Empty paragraph between two paragraphs | Cursor visible at left edge, same indentation as siblings |
| 1.1.5 | Empty paragraph in a list | Cursor after list marker, paragraph has list marker visible |
| 1.1.6 | Empty paragraph in a table cell | Cursor inside the cell, cell maintains minimum height |

### 1.2 Cursor position after operations
**Spec**: After every operation, the cursor MUST be at a predictable position. The user should never have to click to get their cursor back.

| Operation | Cursor Position After |
|-----------|----------------------|
| Type character 'a' | After the 'a' |
| Delete (Backspace) | Where the deleted character was |
| Delete (Forward) | Stay in place |
| Enter (split paragraph) | Start of new (second) paragraph |
| Paste text | End of pasted content |
| Cut text | Where the selection started |
| Undo | Where cursor was before the undone action |
| Redo | Where cursor was before the redo |
| Bold/Italic/format | Selection stays selected |
| Heading change | Cursor stays in same position |
| List toggle | Cursor stays in same position |

### 1.3 Cursor must NOT:
- Jump to top-left corner (position 0,0)
- Disappear after any operation
- Move to the ruler or toolbar area
- Flicker (disappear and reappear rapidly)
- Be inside a non-editable element (header/footer/peer cursor label)

## 2. Selection

### 2.1 Selection preservation across operations
**Spec**: Selection MUST survive:
- Toolbar button clicks (bold, italic, underline, alignment)
- Color picker open/close
- Font picker open/close
- Modal open/close (link, table, comment)
- Right-click context menu open/close

**Implementation**: Selection saved by node ID + character offset (not DOM Range). Restored by finding element via `[data-node-id]` attribute.

### 2.2 Selection edge cases
| # | Scenario | Expected Behavior |
|---|----------|-------------------|
| 2.2.1 | Select all (Ctrl+A) | All content selected, blue highlight |
| 2.2.2 | Select across pages | Selection spans multiple page elements |
| 2.2.3 | Select in table | Cell-level selection (if clicking cell borders) |
| 2.2.4 | Triple-click | Select entire paragraph |
| 2.2.5 | Shift+Click | Extend selection to click point |
| 2.2.6 | Shift+Arrow | Extend selection by one character/line |
| 2.2.7 | Selection active during remote CRDT edit (same paragraph) | Selection offsets adjusted if remote edit is before selection start. If remote edit is within selection, selection expands/contracts. Selection MUST NOT be cleared. |
| 2.2.8 | Selection active during remote fullSync | Selection is cleared (document replaced). User must re-select. A toast "Document updated by [peer name]" MAY be shown. |

**Selection during remote edits (normative):**
- **CRDT path**: When a remote CRDT text op arrives for a paragraph where the local user has an active selection, the selection MUST be preserved with adjusted offsets. The `renderNodeById()` incremental render saves and restores the selection. If the selection becomes invalid (e.g., remote delete removed the selected range), the selection collapses to the nearest valid position.
- **fullSync path**: When a fullSync replaces the document, the selection is invalidated because node IDs may change. The selection is cleared. Cursor position is restored by paragraph index + character offset (best-effort). This is a known UX limitation of the fullSync approach.

## 3. Clipboard (Cut/Copy/Paste)

### 3.1 Cut
**Spec**: Selected content is copied to clipboard AND removed from document. If the entire document is selected:
1. All content is copied to clipboard (HTML + plain text)
2. All content is removed
3. An empty paragraph is created
4. Cursor is placed at start of the empty paragraph
5. Document is rendered (the empty paragraph is visible)

**Implementation**:
1. Synchronous `execCommand('copy')` FIRST (always works on HTTP)
2. Async `Clipboard API.write()` as upgrade (for rich HTML, HTTPS only)
3. Then `delete_selection()` on the WASM model
4. `renderDocument()` to show the deletion
5. Focus + `setCursorAtOffset()` to position cursor

### 3.2 Copy
**Spec**: Same as cut but content is NOT removed. Cursor and selection stay unchanged.

### 3.3 Paste
**Spec**: Content from clipboard is inserted at cursor position. If there's a selection, it's replaced.

**Flow**:
1. If selection exists → `delete_selection()` first
2. `renderDocument()` to sync DOM after deletion
3. Read clipboard: HTML (`text/html`) and plain text (`text/plain`)
4. If HTML available → `parseClipboardHtml()` → `pasteStructuredContent()`
5. If plain text only → `paste_plain_text()` (multi-line) or `insert_text_in_paragraph()` (single-line)
6. `renderDocument()` to show the pasted content
7. Place cursor after the pasted content

**Edge cases**:
| # | Scenario | Expected Behavior |
|---|----------|-------------------|
| 3.3.1 | Paste into empty document | Creates paragraph(s) with content |
| 3.3.2 | Paste after cut-all | New paragraphs created from pasted content |
| 3.3.3 | Paste multi-line text | Creates multiple paragraphs |
| 3.3.4 | Paste HTML from Word | Formats preserved (bold, italic, etc.) |
| 3.3.5 | Paste from Google Docs | Basic formatting preserved |
| 3.3.6 | Paste image from clipboard | Image inserted inline |
| 3.3.7 | Paste on HTTP (no Clipboard API) | Falls back to `execCommand` + paste event |
| 3.3.8 | Paste with pending formats | Pending formats applied to pasted text |

## 4. Images

### 4.1 Image alignment
**Spec**: Image alignment follows the parent paragraph's alignment. Setting an image to "center" sets the paragraph's `text-align: center`, and the inline image centers within it.

| Alignment | CSS Applied | Visual Result |
|-----------|-------------|---------------|
| Left | `text-align: left` on paragraph | Image at left edge |
| Center | `text-align: center` on paragraph | Image centered |
| Right | `text-align: right` on paragraph | Image at right edge |

**Implementation**: `alignImage()` in `images.js` calls `set_alignment()` on the parent paragraph node, not the image node.

### 4.2 Image sizing
**Spec**: Images render at their stored width/height from the document model. If no dimensions stored, default to `max-width: 100%` to prevent overflow.

### 4.3 Image in collab
**Spec**: Image operations (insert, resize, align, delete) use fullSync for collaboration (structural change).

## 5. Empty Lines

### 5.1 Rendering
**Spec**: An empty paragraph renders as a single blank line with:
- Height matching the current font size + line spacing
- A `<br>` element to ensure the line has height
- Cursor anchor point at the start

### 5.2 Typing into empty paragraph
**Spec**: When the user types into an empty paragraph:
1. The `<br>` is replaced by the typed character
2. The paragraph's text content updates
3. Cursor advances past the typed character
4. No visible flicker or cursor jump

### 5.3 Empty lines in collab
**Spec**: Empty paragraphs sync via fullSync. The CRDT layer doesn't natively handle empty nodes (no text to track). After fullSync, both peers see the same empty paragraphs.

## 6. Undo/Redo

### 6.1 Undo
**Spec**: `Ctrl+Z` undoes the last operation. In collab mode:
- Undo is LOCAL only (each peer has their own undo stack)
- Undo produces a CRDT inverse operation → broadcast to peers
- Peers see the undo as a new edit (they can't undo someone else's undo)

### 6.2 Redo
**Spec**: `Ctrl+Y` / `Ctrl+Shift+Z` redoes the last undone operation.
- Redo stack is cleared when a new operation is performed after undo
- WASM undo stack and UI undo buttons are synced via `can_undo()`/`can_redo()`

## 7. Rendering Performance

### 7.1 Incremental render
**Spec**: Text-only edits within a single paragraph MUST NOT trigger full document re-render. Only the affected paragraph is updated.

**Implementation**: `debouncedSync` syncs text to WASM without re-rendering DOM. The browser handles text display natively via contentEditable. Full render only on structural changes.

### 7.2 Performance targets
| Metric | Target |
|--------|--------|
| Keystroke to visible character | <16ms (1 frame at 60fps) |
| Paragraph re-render | <50ms |
| Full document render (10 pages) | <200ms |
| Full document render (100 pages) | <2000ms |
| Collab text sync to peer | <20ms |
| Collab structural sync to peer | <500ms |
