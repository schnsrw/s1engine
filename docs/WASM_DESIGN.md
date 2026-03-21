# WASM Bindings Design Document

## Overview

s1engine provides WebAssembly bindings via `s1engine-wasm`, enabling document processing directly in the browser or Node.js. The WASM package exposes a JavaScript-friendly API for creating, opening, editing, rendering, and exporting documents without any server round-trips.

**Package:** `s1engine-wasm` (crate type: `cdylib` + `rlib`)
**Build tool:** `wasm-pack` (via `scripts/build-wasm.sh`)
**JS interop:** `wasm-bindgen` + `js-sys`

## Architecture

```
Browser / Node.js
       ↓ JavaScript API
+------+------------------+
|  s1engine-wasm           |
|  (wasm-bindgen)          |
|  WasmEngine              |
|  WasmDocument            |
|  WasmDocumentBuilder     |
|  WasmFontDatabase        |
|  WasmLayoutConfig        |
+-----------+--------------+
            ↓
+--------------------------------------+
|  s1engine (facade)                    |
|  Engine, Document, DocumentBuilder   |
+----+----------+----------+-----------+
     ↓          ↓          ↓
  s1-format-*  s1-layout  s1-format-pdf
  (docx,odt,   (layout    (PDF export)
   md,txt)      engine)
     ↓
  s1-model + s1-ops
```

### Crate Dependencies

```toml
[dependencies]
s1engine = { workspace = true, features = ["layout", "pdf"] }
s1-model  = { workspace = true }
s1-layout = { workspace = true }
s1-text   = { workspace = true }
s1-format-pdf = { workspace = true }
wasm-bindgen = "0.2"
js-sys = "0.3"
```

The WASM build enables the `layout` and `pdf` feature flags on s1engine, pulling in the full layout engine and PDF export pipeline.

## Public API Surface

### WasmEngine

Entry point for all document operations.

```javascript
const engine = new WasmEngine();

// Open from bytes (format auto-detected from magic bytes)
const doc = engine.open(uint8Array);

// Open with explicit format
const doc = engine.open_as(uint8Array, "docx");

// Create empty document
const doc = engine.create();
```

**Methods:**

| Method | Arguments | Returns | Description |
|---|---|---|---|
| `new()` | — | `WasmEngine` | Constructor |
| `create()` | — | `WasmDocument` | Create empty document |
| `open(data)` | `Uint8Array` | `WasmDocument` | Auto-detect format and open |
| `open_as(data, format)` | `Uint8Array`, `string` | `WasmDocument` | Open with explicit format |

### WasmDocument

Handle for reading, rendering, and exporting documents.

```javascript
// Basic operations
const text = doc.to_plain_text();
const title = doc.metadata_title();
const author = doc.metadata_author();
const count = doc.paragraph_count();

// HTML rendering (tree-to-HTML, no layout engine)
const html = doc.to_html();

// Paginated HTML (layout engine, CSS-positioned pages)
const paginatedHtml = doc.to_paginated_html();
const paginatedHtml = doc.to_paginated_html_with_config(config);
const paginatedHtml = doc.to_paginated_html_with_fonts(fontDb);
const paginatedHtml = doc.to_paginated_html_with_fonts_and_config(fontDb, config);

// PDF export
const pdfBytes = doc.to_pdf();
const pdfBytes = doc.to_pdf_with_fonts(fontDb);
const dataUrl = doc.to_pdf_data_url();
const dataUrl = doc.to_pdf_data_url_with_fonts(fontDb);

// Format export
const docxBytes = doc.export("docx");
const odtBytes = doc.export("odt");
const txtBytes = doc.export("txt");
const mdBytes = doc.export("md");

// Track changes
const changeCount = doc.tracked_changes_count();
doc.accept_all_changes();
doc.reject_all_changes();

// Memory management
doc.free();       // Release WASM memory
doc.is_valid();   // Check if handle is still valid
```

**Methods:**

| Method | Arguments | Returns | Description |
|---|---|---|---|
| `to_plain_text()` | — | `string` | Extract all text content |
| `to_html()` | — | `string` | Tree-based HTML (formatting, tables, images, headers/footers) |
| `to_paginated_html()` | — | `string` | Layout-engine HTML with page boundaries |
| `to_paginated_html_with_config(config)` | `WasmLayoutConfig` | `string` | Paginated HTML with custom page dimensions |
| `to_paginated_html_with_fonts(fontDb)` | `WasmFontDatabase` | `string` | Paginated HTML with loaded fonts |
| `to_paginated_html_with_fonts_and_config(fontDb, config)` | `WasmFontDatabase`, `WasmLayoutConfig` | `string` | Paginated HTML with fonts + custom config |
| `to_pdf()` | — | `Uint8Array` | Export PDF (fallback metrics) |
| `to_pdf_with_fonts(fontDb)` | `WasmFontDatabase` | `Uint8Array` | Export PDF with loaded fonts |
| `to_pdf_data_url()` | — | `string` | PDF as `data:application/pdf;base64,...` |
| `to_pdf_data_url_with_fonts(fontDb)` | `WasmFontDatabase` | `string` | PDF data URL with loaded fonts |
| `export(format)` | `string` | `Uint8Array` | Export to named format |
| `metadata_title()` | — | `string?` | Document title |
| `metadata_author()` | — | `string?` | Document author/creator |
| `paragraph_count()` | — | `number` | Count of paragraphs |
| `tracked_changes_count()` | — | `number` | Count of tracked changes |
| `accept_all_changes()` | — | `void` | Accept all insertions/deletions |
| `reject_all_changes()` | — | `void` | Reject all insertions/deletions |
| `free()` | — | `void` | Release document memory |
| `is_valid()` | — | `boolean` | Check if document handle is live |

### WasmDocumentBuilder

Fluent builder for constructing documents programmatically.

```javascript
const doc = new WasmDocumentBuilder()
    .title("My Report")
    .author("Engineering")
    .heading(1, "Introduction")
    .text("This is the body text.")
    .heading(2, "Details")
    .text("More content here.")
    .build();
```

**Methods:**

| Method | Arguments | Returns | Description |
|---|---|---|---|
| `new()` | — | `WasmDocumentBuilder` | Constructor |
| `title(title)` | `string` | `self` | Set document title |
| `author(author)` | `string` | `self` | Set document author |
| `heading(level, text)` | `number`, `string` | `self` | Add heading (1-6) |
| `text(text)` | `string` | `self` | Add plain text paragraph |
| `build()` | — | `WasmDocument` | Build and return document |

### WasmLayoutConfig

Configuration for the layout engine's page dimensions and margins.

```javascript
// US Letter defaults (612pt x 792pt, 72pt margins)
const config = new WasmLayoutConfig();

// A4
config.set_page_width(595.28);
config.set_page_height(841.89);
config.set_margin_top(56.7);    // 2cm
config.set_margin_bottom(56.7);
config.set_margin_left(56.7);
config.set_margin_right(56.7);
```

**Defaults:** US Letter (8.5" x 11") with 1-inch (72pt) margins on all sides.

| Method | Arguments | Returns | Description |
|---|---|---|---|
| `new()` | — | `WasmLayoutConfig` | US Letter defaults |
| `set_page_width(w)` | `number` (pt) | `void` | Page width |
| `set_page_height(h)` | `number` (pt) | `void` | Page height |
| `set_margin_top(m)` | `number` (pt) | `void` | Top margin |
| `set_margin_bottom(m)` | `number` (pt) | `void` | Bottom margin |
| `set_margin_left(m)` | `number` (pt) | `void` | Left margin |
| `set_margin_right(m)` | `number` (pt) | `void` | Right margin |
| `page_width()` | — | `number` | Get page width |
| `page_height()` | — | `number` | Get page height |
| `margin_top()` | — | `number` | Get top margin |
| `margin_bottom()` | — | `number` | Get bottom margin |
| `margin_left()` | — | `number` | Get left margin |
| `margin_right()` | — | `number` | Get right margin |

### WasmFontDatabase

Font loader for WASM environments. Since WASM has no filesystem, fonts must be loaded from bytes.

```javascript
const fontDb = new WasmFontDatabase();

// Fetch and load a web font
const response = await fetch('/fonts/NotoSans-Regular.ttf');
const fontData = new Uint8Array(await response.arrayBuffer());
fontDb.load_font(fontData);

console.log(`Loaded ${fontDb.font_count()} font faces`);

// Use with layout
const html = doc.to_paginated_html_with_fonts(fontDb);
const pdf = doc.to_pdf_with_fonts(fontDb);
```

| Method | Arguments | Returns | Description |
|---|---|---|---|
| `new()` | — | `WasmFontDatabase` | Empty font database |
| `load_font(data)` | `Uint8Array` | `void` | Load TTF/OTF font |
| `font_count()` | — | `number` | Number of loaded faces |

### Standalone Functions

```javascript
import { detect_format } from 's1engine-wasm';

// Returns "docx", "odt", "pdf", or "txt"
const format = detect_format(uint8Array);
```

## Rendering Modes

### 1. Tree-to-HTML (`to_html()`)

Direct conversion of the document model tree to semantic HTML. No layout engine involved.

**Output:**
- Standard HTML elements: `<p>`, `<h1>`-`<h6>`, `<table>`, `<img>`, `<strong>`, `<em>`, etc.
- Inline styles for formatting (font, color, alignment)
- Base64-encoded images
- Headers and footers from first section
- Track changes visual indicators (green underline for insertions, red strikethrough for deletions)
- Hyperlinks with styling

**Pros:** Fast, lightweight, searchable, selectable text.
**Cons:** No pagination, no page boundaries, no exact positioning.

**Use for:** Quick preview, content editing, web display where page layout doesn't matter.

### 2. Paginated HTML (`to_paginated_html()`)

Full layout engine pipeline producing CSS-positioned HTML with real page boundaries.

**Output structure:**
```html
<div class="s1-document" style="display:flex;flex-direction:column;align-items:center;">
  <div class="s1-page" style="width:612pt;height:792pt;position:relative;
       background:white;margin:20px auto;box-shadow:0 2px 8px rgba(0,0,0,0.3);
       overflow:hidden">
    <!-- Header block -->
    <div class="s1-block" style="position:absolute;left:72pt;top:20pt;width:468pt">
      <div class="s1-line" style="height:12pt;position:relative">
        <span style="font-size:10pt;position:absolute;left:0pt">Header Text</span>
      </div>
    </div>
    <!-- Content blocks -->
    <div class="s1-block" style="position:absolute;left:72pt;top:72pt;width:468pt">
      <div class="s1-line" style="height:14.4pt;position:relative">
        <span style="font-size:12pt;position:absolute;left:0pt">Hello world</span>
      </div>
    </div>
    <!-- Tables -->
    <div class="s1-table" style="position:absolute;left:72pt;top:100pt;width:468pt">
      <div class="s1-table-row" style="position:relative;height:20pt">
        <div class="s1-table-cell" style="position:absolute;left:0pt;top:0pt;
             width:234pt;height:20pt;border:1px solid #ccc;overflow:hidden">
          <!-- Cell content -->
        </div>
      </div>
    </div>
    <!-- Images -->
    <img class="s1-image" src="data:image/png;base64,..."
         style="position:absolute;left:72pt;top:200pt;width:200pt;height:150pt" alt=""/>
    <!-- Footer block -->
    <div class="s1-block" style="position:absolute;left:72pt;top:760pt;width:468pt">
      ...
    </div>
  </div>
  <!-- More pages... -->
</div>
```

**CSS classes:**
- `.s1-document` — top-level flex container
- `.s1-page` — individual page div (relative positioning)
- `.s1-block` — paragraph or container block (absolute positioning)
- `.s1-line` — text line (relative positioning)
- `.s1-table` — table container
- `.s1-table-row` — table row
- `.s1-table-cell` — table cell with border
- `.s1-image` — positioned image
- `.s1-image-placeholder` — placeholder when image data unavailable

**Layout pipeline:**
1. `s1-layout` resolves styles for all nodes
2. Text is shaped using `s1-text` (rustybuzz if fonts loaded, fallback metrics otherwise)
3. Knuth-Plass optimal line breaking (or greedy fallback)
4. Blocks are stacked with paragraph spacing
5. Pagination splits content across pages (row-by-row for tables)
6. Headers/footers placed per section
7. Page number fields substituted
8. `layout_to_html()` converts positioned layout to CSS divs

**Pros:** Accurate page layout, page boundaries, WYSIWYG rendering.
**Cons:** Requires fonts for accurate metrics, heavier computation.

**Use for:** Print preview, document viewer, PDF-like display.

### 3. PDF Export (`to_pdf()`)

Full layout-to-PDF pipeline producing valid PDF 1.4 output.

**Pipeline:**
1. Same layout engine as paginated HTML
2. PDF generation with CIDFont embedding and subsetting
3. Glyph positioning, table borders, image embedding
4. Hyperlink annotations, bookmarks/outline, metadata

**Output:** Raw PDF bytes (`Uint8Array`) or base64 data URL.

**Use for:** Download, print, sharing.

## Font Handling in WASM

### The Problem

WASM has no filesystem access. The layout engine needs font metrics for text shaping and line breaking. Without fonts, character widths are unknown.

### The Solution: Fallback Metrics

When no fonts are loaded via `WasmFontDatabase`, the layout engine uses approximate character widths based on a monospace fallback model. This produces "good enough" layout for most documents -- text will be positioned reasonably, but character-level kerning and exact line breaks may differ from native rendering.

This is a common approach used by lightweight document viewers.

### Loading Fonts

For accurate layout, load fonts via `WasmFontDatabase`:

```javascript
const fontDb = new WasmFontDatabase();

// Load system-like fonts from your server or CDN
const fonts = [
  '/fonts/NotoSans-Regular.ttf',
  '/fonts/NotoSans-Bold.ttf',
  '/fonts/NotoSans-Italic.ttf',
  '/fonts/NotoSerif-Regular.ttf',
];

for (const url of fonts) {
  const resp = await fetch(url);
  const data = new Uint8Array(await resp.arrayBuffer());
  fontDb.load_font(data);
}

// Now use the font database for accurate layout
const html = doc.to_paginated_html_with_fonts(fontDb);
const pdf = doc.to_pdf_with_fonts(fontDb);
```

### Font Matching

The font database uses `fontdb` for matching. When the document specifies a font family (e.g., "Times New Roman"), the engine:

1. Searches loaded fonts for an exact match
2. Falls back to a generic serif/sans-serif/monospace match
3. Falls back to the first loaded font
4. Falls back to estimated character widths if no fonts are loaded

### Recommendations

- Load at least one serif and one sans-serif font for reasonable coverage
- For CJK documents, load a CJK font (e.g., Noto Sans CJK)
- Font loading can be done lazily after the initial page render
- Font files are typically 100KB-5MB each; consider subsetting for web delivery

## Track Changes Support

The WASM bindings expose track change operations:

```javascript
// Count tracked changes
const count = doc.tracked_changes_count();

// Accept all (insertions kept, deletions removed)
doc.accept_all_changes();

// Reject all (insertions removed, deletions restored)
doc.reject_all_changes();
```

### Visual Indicators in `to_html()`

When rendering via `to_html()`, tracked changes are styled automatically:

- **Insertions:** Green text with underline (`color:#22863a; text-decoration:underline`)
- **Deletions:** Red text with strikethrough (`color:#cb2431; text-decoration:line-through`)
- **Format changes:** Yellow dotted bottom border (`border-bottom:2px dotted #b08800`)

## Supported Formats

| Format | Open | Export | MIME Type |
|---|---|---|---|
| DOCX | Yes | Yes | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` |
| ODT | Yes | Yes | `application/vnd.oasis.opendocument.text` |
| TXT | Yes | Yes | `text/plain` |
| Markdown | Yes | Yes | `text/markdown` |
| DOC | Yes | No | `application/msword` |
| PDF | No | Yes | `application/pdf` |

Format auto-detection uses magic bytes:
- `PK\x03\x04` → ZIP-based (DOCX or ODT, further detection from ZIP contents)
- `%PDF` → PDF
- `\xD0\xCF\x11\xE0` → OLE2 (DOC)
- Everything else → TXT (with UTF-8/UTF-16/Latin-1 encoding detection)

## Memory Management

WASM linear memory is managed through the `free()` method on `WasmDocument`:

```javascript
const doc = engine.open(data);
// ... use document ...
doc.free();  // Release WASM memory

// After free(), all methods throw:
doc.to_plain_text();  // Error: "Document has been freed"
doc.is_valid();       // false
```

**Best practices:**
- Call `free()` when done with a document, especially in single-page apps that process many files
- `WasmEngine`, `WasmDocumentBuilder`, `WasmFontDatabase`, and `WasmLayoutConfig` are lightweight and don't need explicit cleanup
- The `WasmFontDatabase` can be shared across multiple document operations

## Build Process

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-pack` (`cargo install wasm-pack`)

### Building

```bash
# Debug build (fast compilation, larger output)
./scripts/build-wasm.sh --dev

# Release build (optimized, smaller output)
./scripts/build-wasm.sh

# Via Makefile
make wasm           # Debug
make wasm-release   # Release
```

Output directory: `ffi/wasm/pkg/`

### Output Files

```
ffi/wasm/pkg/
  s1engine_wasm_bg.wasm   # WASM binary
  s1engine_wasm.js         # JS glue code (ESM)
  s1engine_wasm.d.ts       # TypeScript declarations
  package.json             # npm package metadata
```

### Browser Demo

```bash
make demo          # Build WASM + start server at localhost:8080
make demo-only     # Start server without rebuilding
```

The demo (`demo/index.html`) includes:
- **HTML tab** — tree-based HTML rendering with formatting
- **Pages tab** — paginated layout-engine view with page boundaries, shadows, and page navigation
- **Text tab** — plain text extraction
- File opening via drag-and-drop or file picker
- Export to DOCX, ODT, TXT, Markdown
- PDF download via `to_pdf_data_url()`
- Format auto-detection

## Error Handling

All WASM methods that can fail return `Result<T, JsError>`. In JavaScript, these throw standard `Error` objects:

```javascript
try {
  const doc = engine.open(corruptedData);
} catch (e) {
  console.error("Failed to open:", e.message);
  // e.g.: "DOCX error: missing document.xml in archive"
}
```

Error messages include context (format type, missing file, invalid structure) and are suitable for display to end users.

## Size Budget

| Component | Approximate WASM Size (release) |
|---|---|
| Core model + ops | ~100 KB |
| DOCX reader/writer | ~150 KB |
| ODT reader/writer | ~120 KB |
| TXT + Markdown | ~40 KB |
| Layout engine | ~80 KB |
| Text shaping (rustybuzz) | ~400 KB |
| PDF export | ~60 KB |
| **Total (gzip)** | **~350-500 KB** |

The text shaping engine (rustybuzz) is the largest component. It's included because the `layout` and `pdf` features are enabled. If only tree-based HTML rendering is needed, a slimmer build without layout/pdf features would reduce the binary significantly.

## Limitations

1. **No system fonts** — Fonts must be loaded manually via `WasmFontDatabase`. Without fonts, layout uses approximate character widths.

2. **No async API** — All operations are synchronous. For large documents, consider running in a Web Worker to avoid blocking the main thread.

3. **No filesystem** — Cannot use `open_file()` or write to disk. Use `Uint8Array` for all I/O.

4. **No CRDT in WASM** — The collaborative editing (CRDT) module is not currently exposed in WASM bindings. Transport and conflict resolution must be handled server-side.

5. **Single-threaded** — WASM runs on a single thread. Layout of very large documents (100+ pages) may take noticeable time.

6. **No incremental layout** — Each call to `to_paginated_html()` re-layouts the entire document. For interactive editing, batch changes before re-rendering.

## Future Work

- **CRDT bindings** — Expose `CollabDocument` for browser-based collaborative editing
- **Incremental rendering** — Only re-layout changed pages
- **Web Worker support** — Off-main-thread document processing
- **Streaming export** — Progressive PDF/DOCX generation for large documents
- **Edit operations** — Expose `Transaction` API for document editing from JavaScript
- **Custom HTML rendering** — Allow consumers to provide custom element renderers
