# Changelog

All notable changes to s1engine will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
