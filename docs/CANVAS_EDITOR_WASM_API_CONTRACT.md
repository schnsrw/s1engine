# Canvas-First Editor WASM API Contract

**Status:** Draft API contract  
**Last updated:** 2026-03-30  
**Applies to branch:** `feature/reimagine-port`

## Related Documents

- `CANVAS_EDITOR_HIGH_LEVEL_ARCHITECTURE.md`
- `CANVAS_EDITOR_LOW_LEVEL_DESIGN.md`
- `CANVAS_EDITOR_IMPLEMENTATION_ROADMAP.md`
- `WASM_DESIGN.md`

## Purpose

This document defines the concrete browser boundary for the canvas-first editor.

The current WASM surface in `ffi/wasm/src/lib.rs` is still centered on HTML output and DOM-oriented incremental rendering. The new contract adds scene, geometry, navigation, editing, composition, and clipboard APIs without removing the existing HTML APIs immediately.

## Contract Rules

### 1. Rust is authoritative

- document content and attributes live in Rust
- model positions are resolved in Rust
- layout and page geometry are resolved in Rust
- JS does not invent page breaks, caret rects, or hit-test outcomes

### 2. Geometry units are always points

Every coordinate crossing the boundary uses points.

JS is responsible for converting:

```text
points -> CSS pixels -> backing pixels
```

### 3. Structured values should use `JsValue`, not JSON strings

For canvas rendering and hot geometry queries, the canonical API should return structured objects through `serde_wasm_bindgen`.

Temporary compatibility wrappers may expose `_json` variants, but they are not the preferred long-term surface.

### 4. Boundary text offsets use UTF-16 code units

The browser and IME APIs already speak UTF-16 offsets. To avoid constant JS-side translation, model positions crossing the boundary use UTF-16 offsets.

Rust may normalize internally, but the browser contract is:

- `offset_utf16` for positions inside text-bearing nodes
- navigation APIs return canonical positions after normalization

### 5. Every mutable operation returns revisions and dirty-page information

JS must be able to repaint only what changed.

## Canonical Value Types

### PositionRef

```json
{
  "node_id": "n_1042",
  "offset_utf16": 15,
  "affinity": "downstream"
}
```

Fields:

- `node_id`: globally unique model node id for a text-bearing location
- `offset_utf16`: offset inside that node's logical text content
- `affinity`: `downstream` or `upstream` for ambiguous boundaries

### RangeRef

```json
{
  "anchor": {
    "node_id": "n_1042",
    "offset_utf16": 3,
    "affinity": "downstream"
  },
  "focus": {
    "node_id": "n_1042",
    "offset_utf16": 18,
    "affinity": "downstream"
  }
}
```

### RectPt

```json
{
  "page_index": 2,
  "x": 72.0,
  "y": 144.0,
  "width": 83.5,
  "height": 14.0
}
```

### SceneSummary

```json
{
  "protocol_version": 1,
  "document_revision": 18,
  "layout_revision": 11,
  "page_count": 7,
  "default_page_size_pt": { "width": 612.0, "height": 792.0 },
  "pages": [
    {
      "page_index": 0,
      "section_index": 0,
      "bounds_pt": { "x": 0.0, "y": 0.0, "width": 612.0, "height": 792.0 },
      "content_rect_pt": { "x": 72.0, "y": 72.0, "width": 468.0, "height": 648.0 },
      "has_header": true,
      "has_footer": true,
      "item_count": 128
    }
  ]
}
```

### PageScene

```json
{
  "page_index": 0,
  "document_revision": 18,
  "layout_revision": 11,
  "bounds_pt": { "x": 0.0, "y": 0.0, "width": 612.0, "height": 792.0 },
  "content_rect_pt": { "x": 72.0, "y": 72.0, "width": 468.0, "height": 648.0 },
  "header_rect_pt": { "x": 72.0, "y": 36.0, "width": 468.0, "height": 24.0 },
  "footer_rect_pt": { "x": 72.0, "y": 732.0, "width": 468.0, "height": 24.0 },
  "items": [ "..." ]
}
```

### SceneItem Types

Every item in `PageScene.items` has a `kind` field. The complete set of item kinds:

#### `text_run`

A shaped text run at an exact position. The primary rendering primitive.

```json
{
  "kind": "text_run",
  "node_id": "0:42",
  "bounds_pt": { "x": 84.0, "y": 102.0, "width": 120.0, "height": 14.0 },
  "baseline_y": 112.0,
  "text": "Hello world",
  "font_family": "Noto Sans",
  "font_size_pt": 11.0,
  "bold": false,
  "italic": false,
  "underline": "none",
  "strikethrough": false,
  "color": "#111111",
  "highlight_color": null,
  "hyperlink_url": null,
  "superscript": false,
  "subscript": false,
  "character_spacing": 0.0,
  "revision_type": null,
  "revision_author": null
}
```

#### `paragraph_background`

Background fill for a paragraph block.

```json
{
  "kind": "paragraph_background",
  "node_id": "0:40",
  "bounds_pt": { "x": 72.0, "y": 100.0, "width": 468.0, "height": 28.0 },
  "color": "#F0F0F0"
}
```

#### `paragraph_border`

Border segments around a paragraph.

```json
{
  "kind": "paragraph_border",
  "node_id": "0:40",
  "bounds_pt": { "x": 72.0, "y": 100.0, "width": 468.0, "height": 28.0 },
  "border_top": { "width": 1.0, "style": "solid", "color": "#000000" },
  "border_bottom": null,
  "border_left": null,
  "border_right": null
}
```

#### `list_marker`

Bullet or number glyph for a list item.

```json
{
  "kind": "list_marker",
  "node_id": "0:45",
  "bounds_pt": { "x": 84.0, "y": 130.0, "width": 12.0, "height": 14.0 },
  "marker_text": "\u2022",
  "font_family": "Symbol",
  "font_size_pt": 11.0,
  "color": "#000000",
  "list_level": 0
}
```

#### `table_cell_background`

Fill for a table cell.

```json
{
  "kind": "table_cell_background",
  "node_id": "0:80",
  "bounds_pt": { "x": 72.0, "y": 200.0, "width": 234.0, "height": 40.0 },
  "color": "#E8E8E8"
}
```

#### `table_border_segment`

A single resolved border segment of a table.

```json
{
  "kind": "table_border_segment",
  "node_id": "0:78",
  "start_pt": { "x": 72.0, "y": 200.0 },
  "end_pt": { "x": 306.0, "y": 200.0 },
  "width": 1.0,
  "style": "solid",
  "color": "#000000"
}
```

#### `image`

An inline or floating image.

```json
{
  "kind": "image",
  "node_id": "0:90",
  "bounds_pt": { "x": 72.0, "y": 300.0, "width": 200.0, "height": 150.0 },
  "media_id": 1,
  "content_type": "image/png",
  "src_base64": "iVBOR...",
  "wrap_type": "none",
  "is_floating": false
}
```

#### `shape`

A vector shape (rectangle, ellipse, line, etc.).

```json
{
  "kind": "shape",
  "node_id": "0:95",
  "bounds_pt": { "x": 300.0, "y": 400.0, "width": 150.0, "height": 100.0 },
  "shape_type": "rect",
  "fill_color": "#4472C4",
  "stroke_color": "#2F5496",
  "stroke_width": 1.0,
  "rotation_deg": 0.0,
  "flip_h": false,
  "flip_v": false,
  "is_floating": true,
  "wrap_type": "square",
  "has_text_frame": false
}
```

#### `text_box`

A shape with an embedded text frame. Contains nested text runs.

```json
{
  "kind": "text_box",
  "node_id": "0:96",
  "bounds_pt": { "x": 300.0, "y": 400.0, "width": 150.0, "height": 100.0 },
  "shape_type": "textBox",
  "fill_color": "#FFFFFF",
  "stroke_color": "#000000",
  "stroke_width": 0.5,
  "text_margins": { "top": 4.0, "bottom": 4.0, "left": 4.0, "right": 4.0 },
  "text_vertical_align": "top",
  "text_runs": [ "..." ]
}
```

#### `header_footer_anchor`

Marks the region for a header or footer.

```json
{
  "kind": "header_footer_anchor",
  "region": "header",
  "bounds_pt": { "x": 72.0, "y": 36.0, "width": 468.0, "height": 24.0 }
}
```

#### `footnote_separator`

The line separating footnotes from body content.

```json
{
  "kind": "footnote_separator",
  "bounds_pt": { "x": 72.0, "y": 650.0, "width": 120.0, "height": 1.0 }
}
```

#### `comment_anchor`

A highlight marking a comment's anchor range.

```json
{
  "kind": "comment_anchor",
  "node_id": "0:110",
  "comment_id": "c_1",
  "bounds_pt": { "x": 100.0, "y": 102.0, "width": 60.0, "height": 14.0 },
  "color": "#FFF3CD"
}
```

#### `bookmark_anchor`

A non-visual marker for a bookmark position.

```json
{
  "kind": "bookmark_anchor",
  "node_id": "0:115",
  "bookmark_name": "section_start",
  "position_pt": { "x": 84.0, "y": 102.0 }
}
```

#### `inline_image`

An image embedded within a text run (e.g., emoji, icon).

```json
{
  "kind": "inline_image",
  "node_id": "0:120",
  "bounds_pt": { "x": 200.0, "y": 102.0, "width": 14.0, "height": 14.0 },
  "media_id": 2,
  "content_type": "image/png",
  "src_base64": "..."
}
```

### HitTestResult

```json
{
  "page_index": 0,
  "kind": "text",
  "position": {
    "node_id": "n_1042",
    "offset_utf16": 6,
    "affinity": "downstream"
  },
  "node_id": "n_1042",
  "item_id": "run_991",
  "inside": true
}
```

Possible `kind` values:

- `text`
- `image`
- `shape`
- `table_cell`
- `header`
- `footer`
- `page_margin`
- `none`

### EditResult

```json
{
  "document_revision": 19,
  "layout_revision": 12,
  "dirty_pages": { "start": 0, "end": 1 },
  "selection": {
    "anchor": {
      "node_id": "n_1042",
      "offset_utf16": 12,
      "affinity": "downstream"
    },
    "focus": {
      "node_id": "n_1042",
      "offset_utf16": 12,
      "affinity": "downstream"
    }
  }
}
```

### FormattingState

Returned by `selection_formatting()` to drive toolbar state.

```json
{
  "bold": true,
  "italic": false,
  "underline": "none",
  "strikethrough": false,
  "superscript": false,
  "subscript": false,
  "font_family": "Times New Roman",
  "font_size_pt": 12.0,
  "color": "#000000",
  "highlight_color": null,
  "alignment": "left",
  "line_spacing": "single",
  "list_format": null,
  "style_id": "Normal",
  "indent_left_pt": 0.0,
  "indent_right_pt": 0.0,
  "indent_first_line_pt": 0.0,
  "spacing_before_pt": 0.0,
  "spacing_after_pt": 0.0
}
```

For mixed-formatting selections, individual fields are `null` (indeterminate).

### CompositionState

Returned by `begin_composition()` and `update_composition()`.

```json
{
  "active": true,
  "preview_text": "composing",
  "preview_range": {
    "anchor": { "node_id": "0:42", "offset_utf16": 10, "affinity": "downstream" },
    "focus": { "node_id": "0:42", "offset_utf16": 19, "affinity": "downstream" }
  },
  "underline_rects": [
    { "page_index": 0, "x": 150.0, "y": 102.0, "width": 80.0, "height": 14.0 }
  ],
  "caret_rect": { "page_index": 0, "x": 230.0, "y": 102.0, "width": 1.0, "height": 14.0 }
}
```

## Error Handling Contract

### Error response shape

All WASM methods that can fail return a JSON object with an `error` field instead of the normal result:

```json
{
  "error": {
    "code": "invalid_position",
    "message": "Node 0:999 does not exist in the document",
    "context": {
      "method": "hit_test",
      "node_id": "0:999"
    }
  }
}
```

### Error codes

| Code | Meaning | When |
|---|---|---|
| `invalid_position` | The `PositionRef` refers to a non-existent node or out-of-range offset | Any method accepting `PositionRef` |
| `invalid_range` | The `RangeRef` has invalid anchor or focus | Any method accepting `RangeRef` |
| `invalid_page_index` | Page index is out of range | `page_scene()`, `hit_test()` |
| `invalid_node_id` | Node ID does not exist | `node_bounds()`, `set_block_attrs()` |
| `layout_not_ready` | Layout has not been computed yet | Scene/geometry methods before first layout |
| `composition_active` | Cannot start a new composition while one is active | `begin_composition()` |
| `no_active_composition` | Cannot update/commit without active composition | `update_composition()`, `commit_composition()` |
| `read_only` | Document is in read-only mode | Any editing method |
| `invalid_argument` | General argument validation failure | Any method |

### JS-side handling

```js
const result = JSON.parse(doc.hit_test(pageIdx, x, y, '{}'));
if (result.error) {
  console.warn(`WASM error [${result.error.code}]: ${result.error.message}`);
  return null;
}
// use result normally
```

### Panics

WASM methods must never panic. All Rust `Result::Err` values are caught and serialized as error responses. If an unexpected panic occurs, `wasm_bindgen`'s default panic hook converts it to a JS exception — this indicates a bug and should be reported.

## Mark Types for `toggle_mark()`

The `mark` parameter in `toggle_mark(range, mark)` accepts these string values:

| Mark | Effect | Toggle behavior |
|---|---|---|
| `"bold"` | Bold weight | On if any part of range is not bold |
| `"italic"` | Italic style | On if any part of range is not italic |
| `"underline"` | Single underline | On/off toggle |
| `"strikethrough"` | Strikethrough | On/off toggle |
| `"superscript"` | Superscript | On/off toggle; clears subscript |
| `"subscript"` | Subscript | On/off toggle; clears superscript |
| `"code"` | Monospace font | On/off toggle |

For parametric formatting (font size, color, font family), use `set_run_attrs(range, attrs)` instead:

```json
{ "font_size_pt": 14.0, "color": "#FF0000", "font_family": "Arial" }
```

## Proposed `WasmDocument` Additions

### Revision and capability methods

| Method | Returns | Notes |
|---|---|---|
| `scene_protocol_version()` | `u32` | Start at `1` |
| `document_revision()` | `u64` | Bumps on every model mutation |
| `layout_revision()` | `u64` | Bumps when pagination output changes |
| `editor_capabilities()` | `JsValue` | Feature flags such as tables/comments/spellcheck support |

### Scene methods

| Method | Arguments | Returns | Notes |
|---|---|---|---|
| `scene_summary(config)` | `WasmLayoutConfig` | `JsValue` | Light page map for viewport boot |
| `page_scene(page_index, options)` | `u32`, `JsValue` | `JsValue` | Full scene for one page |
| `visible_page_scenes(start_page, end_page, options)` | `u32`, `u32`, `JsValue` | `JsValue` | Batch page fetch for viewport |
| `node_bounds(node_id)` | `string` | `JsValue` | All page rects for a node |

Recommended `options` fields:

- `include_text_runs`
- `include_backgrounds`
- `include_guides`
- `include_debug_ids`
- `include_comment_anchors`

### Geometry and navigation methods

| Method | Arguments | Returns | Notes |
|---|---|---|---|
| `hit_test(page_index, x_pt, y_pt, options)` | `u32`, `f64`, `f64`, `JsValue` | `JsValue` | Primary pointer query |
| `caret_rect(position)` | `JsValue` | `JsValue` | Rect for hidden input and caret paint |
| `selection_rects(range)` | `JsValue` | `JsValue` | Ordered rect list for canvas highlight |
| `move_position(position, direction, granularity)` | `JsValue`, `string`, `string` | `JsValue` | Returns normalized `PositionRef` |
| `move_range(range, direction, granularity, extend)` | `JsValue`, `string`, `string`, `bool` | `JsValue` | Returns normalized `RangeRef` |
| `word_boundary(position)` | `JsValue` | `JsValue` | For double-click and ctrl+arrow logic |
| `line_boundary(position, side)` | `JsValue`, `string` | `JsValue` | For home/end behavior |

Recommended enums:

- `direction`: `forward`, `backward`, `up`, `down`
- `granularity`: `character`, `word`, `line`, `paragraph`, `document`

### Editing methods

| Method | Arguments | Returns | Notes |
|---|---|---|---|
| `insert_text_at(position, text)` | `JsValue`, `string` | `JsValue` | Collapsed insertion |
| `replace_range(range, text)` | `JsValue`, `string` | `JsValue` | Primary typing path |
| `delete_range(range)` | `JsValue` | `JsValue` | Explicit range deletion |
| `insert_paragraph_break(position)` | `JsValue` | `JsValue` | Enter key |
| `toggle_mark(range, mark)` | `JsValue`, `string` | `JsValue` | See Mark Types table |
| `set_run_attrs(range, attrs)` | `JsValue`, `JsValue` | `JsValue` | Parametric run formatting |
| `set_block_attrs(node_id, attrs)` | `string`, `JsValue` | `JsValue` | Alignment, spacing, list attrs |
| `selection_formatting(range)` | `JsValue` | `JsValue` | Returns FormattingState for toolbar |
| `paste_html(position, html)` | `JsValue`, `string` | `JsValue` | Rich paste from clipboard |
| `insert_image(anchor, spec)` | `JsValue`, `JsValue` | `JsValue` | Phase 5+ |
| `insert_table(anchor, spec)` | `JsValue`, `JsValue` | `JsValue` | Phase 6 |

### Composition methods

| Method | Arguments | Returns | Notes |
|---|---|---|---|
| `begin_composition(position)` | `JsValue` | `JsValue` | Starts transient composition state |
| `update_composition(text, selection_start_utf16, selection_end_utf16)` | `string`, `u32`, `u32` | `JsValue` | Returns preview range/rects |
| `commit_composition(text)` | `string` | `JsValue` | Produces final `EditResult` |
| `cancel_composition()` | — | `JsValue` | Clears transient state |

### Clipboard and search helpers

| Method | Arguments | Returns | Notes |
|---|---|---|---|
| `copy_range_plain_text(range)` | `JsValue` | `string` | Clipboard plain text |
| `copy_range_html(range)` | `JsValue` | `string` | Rich clipboard/export fragment |
| `search_matches(query, options)` | `string`, `JsValue` | `JsValue` | Returns ranges and page-local rects |

## Compatibility Strategy

Existing methods stay during migration:

- `to_html()`
- `to_paginated_html*()`
- `render_node_html()`
- current layout JSON helpers already used by `canvas-render.js`

But they should be treated as:

- legacy DOM editor support
- export/inspection tools
- temporary migration shims

They should not remain the canonical canvas editor contract.

## Browser Flow for the First Canvas Slice

1. call `scene_summary(config)` on document open
2. render visible pages via `visible_page_scenes(start, end, options)`
3. on click, call `hit_test(page, x_pt, y_pt, options)`
4. place hidden textarea using `caret_rect(position)`
5. on typing, call `replace_range(range, text)`
6. repaint only `dirty_pages` from the returned `EditResult`

## Acceptance Requirements

The contract is acceptable when:

- it can render and edit without DOM page content ownership
- it does not require JS to infer geometry from HTML
- it returns enough dirty-page information for incremental repaint
- it can drive IME, clipboard, and selection without browser ranges as the source of truth
