# Changelog

All notable changes to s1engine will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## [0.1.0] - Unreleased

Initial development release. Not yet published to crates.io.
