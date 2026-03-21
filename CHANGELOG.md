# Changelog

All notable changes to Rudra Code will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

**Spreadsheet Engine (s1-format-xlsx)**
- XLSX reader/writer: cells, formulas, styles, merged cells, frozen panes, column/row sizing, preserved parts for round-trip fidelity
- ODS reader/writer: value types, formulas (OpenFormula syntax conversion), styles, round-trip (16 tests)
- CSV/TSV parser: RFC 4180, delimiter auto-detect (comma/tab/semicolon/pipe), streaming for large files, BOM handling (40 tests)
- Formula engine: tokenizer, recursive-descent parser, 60+ functions (SUM, AVERAGE, VLOOKUP, IF, INDEX, MATCH, LEFT, RIGHT, MID, LEN, TRIM, CONCATENATE, ROUND, ABS, COUNTIF, SUMIF, NOW, TODAY, etc.), dependency graph with topological sort, circular reference detection, array formula support (93 tests)
- Canvas-based spreadsheet grid UI: virtual scrolling, cell selection/editing, formula bar, sheet tabs, column/row resize/insert/delete, copy/paste, undo/redo, freeze panes, auto-fill, sort, filter
- Conditional formatting: rules, color scales, dialog
- Data validation: dropdown lists, number ranges, error toast
- Cell comments: red triangle indicator, hover tooltip, insert/edit/delete
- Charts: column/bar (grouped multi-series), line/area (multi-series), pie/doughnut (percentage labels), sparklines (inline mini-charts), chart customization (title, legend, draggable, resizable)
- Pivot tables: basic dialog with row/col/value fields, aggregation
- Spreadsheet real-time collaboration: WebSocket cell/format sync, peer cursor overlay
- Number formats: currency, percentage, date, time, scientific
- Find & Replace for spreadsheets (Ctrl+F/H)
- Named ranges, cross-sheet references (Sheet1!A1), text to columns, hyperlinks in cells
- Formula autocomplete, syntax highlighting, keyboard shortcuts help (Ctrl+/)
- Merge/unmerge cells UI, paste special (values/formulas/formatting/transpose)
- XLSX export from UI (JS ZIP builder), print to PDF via HTML table

**AI Integration**
- AI sidecar integration (llama.cpp with Qwen2.5-3B model) for document assistance
- Optional deployment — does not affect core engine operation

**Editor Enhancements**
- Styles panel with set_paragraph_style_id WASM API (Title, Subtitle, Quote, Code)
- Header/footer inline editing (double-click to edit, page number fields)
- Track changes UI (Editing/Suggesting/Viewing modes, Review menu, TC panel)
- Section breaks (next page, continuous, even, odd) with visual indicators
- TOC generation with outline panel (Ctrl+Shift+O)
- Format painter (single-click once, double-click sticky)
- Print preview overlay with page navigation
- Bookmarks & cross-references with visual markers
- Image text wrapping (7 modes), crop tool, caption support, filters/opacity
- Multi-column layout with CSS column rendering
- Equation editor with KaTeX rendering
- Special characters dialog (240 chars, 6 categories, search)
- Borders & shading dialog
- Contextual properties panel (paragraph/image/table/section)
- Color palette dropdowns (40 swatches + highlight colors)
- Read-only/viewer mode toggle
- Find & Replace: regex, replace preview, find-in-selection
- Document statistics WASM API (words, chars, paragraphs, pages)
- RTL text direction (dir="rtl" on Arabic/Hebrew paragraphs)
- Batch undo via begin_batch/end_batch WASM API
- Comment placement at correct text range (insert_comment_at_range)
- Smart quotes, auto-capitalize, triple-click paragraph select
- Tab stops on ruler (click/drag/double-click to cycle type)
- 13 new keyboard shortcuts (Ctrl+Shift+V, Ctrl+K, Ctrl+Shift+X, etc.)
- PDF annotation undo (Ctrl+Z), draggable elements, Escape tool deselect
- Skip-to-main-content accessibility link
- Toast aria-live for screen readers

### Fixed
- 106 original tracked issues (100% resolved)
- 31 editor deep scan issues (memory leaks, race conditions, event listener accumulation)
- Paste: blank lines no longer dropped (removed 3 faulty filters)
- Paste: MAX_PASTE_PARAGRAPHS raised 1K->10K with user feedback on failure
- Paste: nested list depth recursively detected, table cell formatting preserved
- Paste: multi-paragraph wrapper splitting, footnote detection
- PDF: text layer selection visibility (opacity 0.25->1)
- PDF: comment tool single-shot mode, outside-click race condition fixed
- PDF: signature typed tab renders with script font + subtle underline
- DOCX: 24 per-element Vec allocations removed from hot parse loop
- DOCX: 11 insert_node calls enriched with error context
- DOCX: round-trip tests for nested tables, mixed lists, multi-section
- ODT: auto-styles HashMap clone eliminated, column style names stored
- Layout: widow/orphan control, character spacing for ligatures
- WASM: builder depth/size limits, ConvertError preserves original error
- CRDT: 3-way convergence test, 5 error path tests, state vector bounded

### Changed
- MSRV bumped 1.85 -> 1.88 (deps require it)
- Analytics.js replaced with no-op stubs for open-source release
- Highlight palette uses muted pastels for better contrast
- Floating toolbar max-width respects viewport on mobile
- Toolbar responsive: horizontal scroll on mobile (<480px)

## [1.0.1] - 2026-03-16

### Added

**Rudra Office**
- PDF viewing via PDF.js (open, zoom, page navigation, text selection)
- PDF annotation tools: highlight, comment, freehand draw, text box
- PDF text editing: double-click to inline-edit text (overlay approach)
- PDF page thumbnails sidebar with scroll tracking
- PDF-specific toolbar (replaces doc toolbar when viewing PDFs)
- Inline comment input (replaces browser `prompt()`)
- Rich paste with formatting preservation (bold, italic, underline, font, size, color)
- Three-tier paste strategy: batch WASM paste, per-run formatting fallback, plain text fallback
- Paste support for content from external sources (other office applications)
- Shapes drawing tools (rectangle, ellipse, line, arrow, text box)

**WASM API**
- PDF editor Rust crate (`s1-format-pdf` with `pdf-editing` feature via lopdf)
- `WasmPdfEditor` class: page manipulation, annotations, form fields, text overlay
- `LayoutAnnotation` type in `s1-layout` for resolved comment/highlight positions
- `paste_formatted_runs_json` — batch paste with formatting
- `export_selection_html` — rich copy to clipboard
- `pasteWithManualFormatting` JS fallback for per-run formatting

**CI/CD**
- GitHub Actions CI: test (stable), test (MSRV 1.85), clippy, rustfmt, documentation, minimal features, all features

**Documentation**
- Rewrote README.md for open-source readability
- Added CONTRIBUTING.md with setup, workflow, and architecture rules
- Added SECURITY.md with vulnerability reporting and security measures
- Added future/ directory with architecture docs (plugin system, REST/WebSocket API, editor SDK, distribution)

### Changed
- Editor renamed from Folio to Rudra Office
- Removed Pages and Text views from editor (simplified to Editor + PDF views)
- File picker now accepts `.pdf` files
- Editor menubar hidden when viewing PDFs
- MSRV bumped from 1.75 to 1.85 (required by ecosystem dependencies using edition2024)
- Fixed GitHub repository URL in Cargo.toml
- Fixed Acknowledgments links in README to point to correct upstream crate repos

### Fixed
- **Paste formatting preservation** — `set_paragraph_text` rewritten with diff-based editing; typing after paste no longer destroys multi-run formatting
- **Underline paste value** — fixed `pasteWithManualFormatting` to use `'underline', 'true'` (was `'single'`, which WASM didn't recognize)
- **External paste** — fixed inline content from external editor `<b id="docs-internal-guid-...">` wrapper creating separate paragraphs per span
- **Consecutive inline paste** — fixed `walkBlockElements` merging adjacent inline elements into single paragraph instead of splitting
- PDF.js `standardFontDataUrl` warning resolved
- ArrayBuffer detach error when opening PDFs
- PDF sidebar thumbnails now track scroll position
- Feature-gated ODT/TXT/MD tests so `--no-default-features --features docx` CI passes
- Fixed broken doc link `[next_id]` in s1-crdt

## [1.0.0] - 2026-03-13

### Changed
- All public enums now have `#[non_exhaustive]` for forward-compatible API evolution
- All error types (except s1-model) migrated to `thiserror` derive macros
- Added missing public re-exports: `Borders`, `BorderSide`, `BorderStyle`, `TabStop`, `TabAlignment`, `TabLeader`, `MediaId`, `MediaStore`, `TableWidth`, `VerticalAlignment`
- Documented `model` and `ops` escape-hatch re-exports

### Added

**Core Engine**
- Document model with globally unique NodeIds `(replica_id, counter)` for CRDT support
- Operation-based editing with full undo/redo via inverse operations
- Transaction support for atomic multi-operation edits
- Cursor and selection model
- DocumentBuilder with fluent API (headings, paragraphs, tables, lists, formatting)
- Unicode-safe text operations (character-offset based, not byte-offset)
- Cycle detection in tree move operations

**Format Support**
- DOCX (OOXML) reader/writer: paragraphs, runs, formatting, styles, metadata, tables, images, lists, sections, headers/footers, hyperlinks, bookmarks, tab stops, paragraph borders/shading, character spacing, superscript/subscript, comments
- ODT (ODF) reader/writer: paragraphs, formatting, styles, metadata, tables, images, lists, auto-styles
- PDF export: font embedding/subsetting, text rendering, tables, images (JPEG/PNG), hyperlinks, bookmarks/outline
- TXT reader/writer: UTF-8/UTF-16/Latin-1 encoding detection
- DOC reader: OLE2/CFB heuristic text extraction
- Cross-format conversion pipeline (DOC/DOCX/ODT)
- Format auto-detection from magic bytes

**Text Processing (Pure Rust)**
- Text shaping via rustybuzz (HarfBuzz port)
- Font parsing via ttf-parser
- System font discovery via fontdb
- Bidirectional text support (UAX #9)
- Unicode line breaking (UAX #14)

**Layout Engine**
- Knuth-Plass optimal line breaking
- Block stacking with paragraph spacing
- Pagination with widow/orphan control
- Table layout with column width calculation
- Image placement
- Header/footer placement from section properties
- Page-number field substitution
- Incremental layout via content-hash-based LayoutCache

**Collaboration (CRDT)**
- Fugue-based text CRDT for concurrent character editing
- Tree CRDT with Kleppmann-style moves and cycle detection
- Per-key LWW attribute CRDT
- LWW metadata and style CRDT
- CollabDocument API with apply_local/apply_remote, snapshot/restore, changes_since
- Awareness state (cursor/presence sharing)
- Binary serialization for operations
- Operation compression (merge consecutive inserts)
- Causal ordering with buffered pending operations

**FFI Bindings**
- WASM bindings via wasm-bindgen (WasmEngine, WasmDocument, WasmDocumentBuilder)
- C FFI bindings with opaque handles and null-pointer safety

**Security**
- ZIP bomb protection: 256MB text entry limit, 64MB media entry limit
- Image dimension cap: 16384px maximum
- No panics in library code (all public APIs return Result)

### Fixed
- Subtree undo now restores complete subtree (root + all descendants)
- Mixed attribute undo properly removes added keys and restores overwritten values
- Text insert/delete uses character offsets (not byte offsets) for Unicode safety
- Tree moves reject cycles (moving a node under its own descendant)

### Performance Baselines

Measured via criterion on a single core (representative, not a guarantee):

| Operation | Time |
|---|---|
| Create empty document | ~113 ns |
| Builder (small, 4 elements) | ~2.6 us |
| Builder (50 sections) | ~67 us |
| Builder (20-row table) | ~38 us |
| Open DOCX (small) | ~34 us |
| Open DOCX (50 sections) | ~146 us |
| Open ODT (small) | ~31 us |
| Export DOCX (small) | ~79 us |
| Export DOCX (50 sections) | ~142 us |
| Export ODT (small) | ~54 us |
| Export TXT (50 sections) | ~12 us |
| DOCX round-trip (small) | ~124 us |
| Undo/redo 10 operations | ~7.2 us |

## [0.1.0]

Initial development release (pre-release). Not published to crates.io.
