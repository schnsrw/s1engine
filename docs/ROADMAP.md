
# Development Roadmap

## Phase Overview

```
Phase 0: Planning           ████████████████████  COMPLETE
Phase 1: Foundation         ████████████████████  COMPLETE
Phase 2: Rich Documents     ████████████████████  COMPLETE (6/6 milestones)
Phase 3: Layout & Export    ████████████████████  COMPLETE (all milestones)
Phase 4: Collaboration      ████████████████████  COMPLETE (4/4 milestones)
Phase 5: Production Ready   ████████████████████  COMPLETE (WASM, C FFI, hardening)
```

---

## Phase 0: Planning & Specification (COMPLETE)

**Completed**: 2026-03-11

Deliverables:
- [x] Project vision and goals (`docs/OVERVIEW.md`)
- [x] System architecture (`docs/ARCHITECTURE.md`)
- [x] Technical specification (`docs/SPECIFICATION.md`)
- [x] Development roadmap (`docs/ROADMAP.md`)
- [x] API design (`docs/API_DESIGN.md`)
- [x] Dependency analysis (`docs/DEPENDENCIES.md`)
- [x] AI development context (`CLAUDE.md`)
- [x] Project README (`README.md`)
- [x] License files (`LICENSE-MIT`, `LICENSE-APACHE`)

---

## Phase 1: Foundation (COMPLETE)

**Completed**: 2026-03-11
**Tests**: 206 passing across 6 crates

**Goal**: Core document model, basic operations, TXT and minimal DOCX support. Prove the architecture works.

### Milestone 1.1: Project Setup (COMPLETE)
- [x] Initialize Cargo workspace with all crate stubs
- [x] Configure workspace `Cargo.toml` with shared settings
- [x] Set MSRV — Rust 1.75+
- [x] `.gitignore` for Rust project
- [x] Verify `cargo build` and `cargo test` pass on all crates

### Milestone 1.2: Document Model — `s1-model` (COMPLETE — 52 tests)
- [x] `NodeId` with replica/counter, `NodeId::ROOT` constant
- [x] `NodeType` enum with all variants
- [x] `Node` struct with id, type, attributes, children, parent, text_content
- [x] `IdGenerator` per-replica counter
- [x] `AttributeKey` and `AttributeValue` enums
- [x] `AttributeMap` with typed get/set methods and builder pattern
- [x] All supporting types: `Color`, `Alignment`, `LineSpacing`, `Borders`, etc.
- [x] `Style` with id, name, type, parent inheritance
- [x] Style resolution algorithm (direct → character → paragraph → default)
- [x] `DocumentMetadata` with all fields
- [x] `MediaStore` with insert (dedup by hash) and get
- [x] `DocumentModel` container with tree operations
- [x] Tree queries: node, root, children, parent, ancestors, descendants
- [x] Node type hierarchy validation (enforce parent/child constraints)
- [x] Unit tests for every type, constructor, and method

### Milestone 1.3: Operations — `s1-ops` (COMPLETE — 37 tests)
- [x] `Operation` enum with 10 variants
- [x] `apply()` function: execute operation on `DocumentModel`
- [x] `validate()` function: check operation validity without applying
- [x] Operation inversion: every `apply()` returns the inverse `Operation`
- [x] `Transaction` grouping with label, `TransactionBuilder`
- [x] `apply_transaction()` with rollback on failure
- [x] `History` with undo/redo stacks, configurable max depth
- [x] `Position` and `Selection` types
- [x] Unit tests for every operation type (apply + invert)

### Milestone 1.4: TXT Format — `s1-format-txt` (COMPLETE — 25 tests)
- [x] TXT reader: encoding detection (UTF-8, UTF-8 BOM, UTF-16 LE/BE, Latin-1 fallback)
- [x] TXT reader: lines → paragraphs with single run + text
- [x] TXT reader: handle `\n`, `\r\n`, `\r` line endings
- [x] TXT writer: serialize document text, tables as tab-separated
- [x] Round-trip tests: read → write → read → compare

### Milestone 1.5: Basic DOCX Reader — `s1-format-docx` (COMPLETE — 37 reader tests)
- [x] ZIP archive opening via `zip` crate
- [x] Parse `[Content_Types].xml`
- [x] Parse relationships (`_rels/.rels`, `word/_rels/document.xml.rels`)
- [x] Parse `docProps/core.xml` → `DocumentMetadata` (Dublin Core)
- [x] Parse `word/document.xml` → paragraphs, runs, text, breaks, tabs
- [x] Parse `w:rPr`: bold, italic, underline (7 styles), strikethrough, font, size, color, highlight, super/subscript, language
- [x] Parse `w:pPr`: alignment, spacing (before/after/line with lineRule), indent, style ref, keepNext/keepLines/pageBreakBefore
- [x] Parse `word/styles.xml` → styles with parent resolution
- [x] Run splitting: DOCX breaks/tabs inside runs → s1-model paragraph children
- [x] Graceful handling of unknown elements (silently skipped)

### Milestone 1.6: Basic DOCX Writer — `s1-format-docx` (COMPLETE — 27 writer tests)
- [x] Generate `[Content_Types].xml`, `_rels/.rels`, `word/_rels/document.xml.rels`
- [x] Generate `word/document.xml` from model (paragraphs, runs, text, breaks, tabs)
- [x] Write `w:rPr` and `w:pPr` properties (all Phase 1 attributes)
- [x] Generate `word/styles.xml` with inheritance
- [x] Generate `docProps/core.xml` metadata
- [x] Package into valid ZIP via `zip` crate
- [x] Round-trip tests: read DOCX → write DOCX → read again → compare (6 tests)

### Milestone 1.7: Facade — `s1engine` (COMPLETE — 28 tests)
- [x] `Engine::new()`, `Engine::create()`, `Engine::open()`, `Engine::open_as()`, `Engine::open_file()`
- [x] `Document` wrapper with model access, metadata, paragraph queries
- [x] `Document::export()` and `Document::export_string()`
- [x] `Document::apply_transaction()`, `undo()`, `redo()`, `can_undo()`, `can_redo()`
- [x] `Format` enum with extension/path/magic-byte detection, MIME types
- [x] Unified `Error` type (Format, Operation, Io, UnsupportedFormat)
- [x] Re-exports of key model/ops types for consumer convenience
- [x] `DocumentBuilder` — fluent API: heading, paragraph, text, bold, italic, underline, styled, colored, line_break
- [x] `ParagraphBuilder` — inline content builder with formatting methods
- [x] Integration tests: create → export DOCX → reopen, open TXT → export, builder → DOCX round-trip

### Phase 1 Deliverable
```rust
use s1engine::{DocumentBuilder, Engine, Format};

// Builder API
let doc = DocumentBuilder::new()
    .title("Report")
    .author("Alice")
    .heading(1, "Introduction")
    .paragraph(|p| p.text("This is ").bold("important").text(" content."))
    .build();

let bytes = doc.export(Format::Docx)?;

// Open and re-export
let engine = Engine::new();
let doc = engine.open(&bytes)?;
println!("{}", doc.to_plain_text());
```

---

## Phase 2: Rich Documents (COMPLETE)

**Completed**: 2026-03-12
**Goal**: Full DOCX support for common features, ODT support, tables, images, lists.

### Milestone 2.1: Tables (COMPLETE — 19 new tests)
- [x] DOCX table reading: `w:tbl`, `w:tr`, `w:tc` → Table/Row/Cell nodes
- [x] Merged cells: `w:gridSpan` (col span), `w:vMerge` (row span)
- [x] Table properties: borders, widths (auto/dxa/pct), alignment
- [x] Cell properties: borders, background (shd), vertical alignment, width
- [x] Nested tables (table inside a cell)
- [x] DOCX table writing with tblGrid generation
- [x] Round-trip tests (write → read → verify structure + properties)
- [x] `DocumentBuilder::table()` + `TableBuilder`/`RowBuilder` fluent API
- [x] Builder DOCX round-trip test

### Milestone 2.2: Images (COMPLETE — 7 new tests)
- [x] Read inline images: `w:drawing` → `wp:inline` → `a:blip` → Image node
- [x] Extract image data from `word/media/` via relationship resolution
- [x] Store in `MediaStore` with deduplication (content hash)
- [x] ParseContext pattern: thread rels + media through all parse functions
- [x] EMU ↔ points conversion helpers (`emu_to_points`, `points_to_emu`)
- [x] MIME type ↔ extension mapping (`mime_for_extension`, `extension_for_mime`)
- [x] Write images: `ImageRelEntry` collection, inline drawing XML generation
- [x] Write ZIP with `word/media/*` files, updated `[Content_Types].xml` and relationships
- [x] Round-trip test: build image model → write DOCX → read back → verify structure + media bytes
- [ ] Read floating images: `wp:anchor` (deferred to Milestone 2.6)
- [ ] Image sizing and DPI handling (deferred)

### Milestone 2.3: Lists (COMPLETE — 30 new tests)
- [x] `NumberingDefinitions` model: `AbstractNumbering`, `NumberingInstance`, `NumberingLevel`, `LevelOverride`
- [x] `numbering_parser.rs`: Parse `word/numbering.xml` — abstract nums, levels, instances, overrides
- [x] `numbering_writer.rs`: Write `word/numbering.xml` back with full fidelity
- [x] `property_parser.rs`: Parse `w:numPr` (ilvl + numId) in paragraph properties
- [x] `content_parser.rs`: Resolve list format from numbering definitions via `ParseContext`
- [x] `content_writer.rs`: Write `w:numPr` in paragraph properties
- [x] `reader.rs`/`writer.rs`: Read/write `word/numbering.xml` in ZIP, content types, relationships
- [x] Support: bulleted, decimal, lowerAlpha, upperAlpha, lowerRoman, upperRoman
- [x] Multi-level lists with per-level format definitions
- [x] Level overrides (start override, full level def override)
- [x] Builder API: `.bullet()`, `.numbered()`, `.list_item()` with auto-created numbering defs
- [x] Round-trip tests: bullet list, numbered list, multi-level list

### Milestone 2.4: Headers, Footers, Sections (COMPLETE — 29 new tests)
- [x] `SectionProperties` model: page size, margins, orientation, columns, break type, header/footer refs, title page
- [x] `SectionBreakType` enum: NextPage, Continuous, EvenPage, OddPage
- [x] `HeaderFooterType` enum: Default, First, Even
- [x] `HeaderFooterRef`: type + NodeId reference to Header/Footer node
- [x] `section_parser.rs`: Parse `w:sectPr` → `RawSectionProperties` with rId strings
- [x] Two-phase rId resolution: section parser returns rIds → reader resolves to NodeIds after parsing header/footer XML
- [x] `section_writer.rs`: Write `w:sectPr` with header/footer references, page size, margins, columns, break type, titlePg
- [x] `header_footer_parser.rs`: Parse `word/header*.xml` and `word/footer*.xml` → Header/Footer nodes as Document root children
- [x] `header_footer_writer.rs`: Write header/footer XML with paragraph content and field support
- [x] Field support: `w:fldSimple` for PAGE, NUMPAGES fields; `FieldType` enum
- [x] `content_parser.rs`: Handle `w:sectPr` in body and in `w:pPr`, `w:fldSimple` fields, `SectionIndex` attribute on paragraphs
- [x] `writer.rs`: Full integration — generate header/footer XML parts, inject sectPr, content types, relationships
- [x] Default / first-page / even-odd headers with `w:titlePg`
- [x] Section breaks: next page, continuous, even/odd
- [x] Builder API: `.section()`, `.section_with_header()`, `.section_with_footer()`, `.section_with_header_footer()`
- [x] Round-trip tests: section properties, header/footer content, first-page header, section breaks
- [x] Builder DOCX round-trip test

### Milestone 2.5: ODT Format — `s1-format-odt` (COMPLETE — 63 tests)
- [x] ODT reader: `content.xml` → document model (paragraphs, headings, spans, formatting)
- [x] ODF style system mapping → `s1-model` styles (named styles from `styles.xml`, automatic styles from `content.xml`)
- [x] ODT writer: document model → `content.xml` + `styles.xml` + `meta.xml` + `META-INF/manifest.xml`
- [x] Tables in ODT (read/write: `table:table`, `table:table-row`, `table:table-cell`)
- [x] Images in ODT (read/write: `draw:frame` + `draw:image` with `xlink:href`)
- [x] Lists in ODT (read/write: `text:list` + `text:list-item`, flattened to paragraphs with ListInfo)
- [x] Property parsing/writing: bold, italic, font-size, font-name, color, underline, strikethrough, alignment, margins, indent, line-height
- [x] Metadata: title, creator, description, keywords, language
- [x] Round-trip tests (ODT → model → ODT → compare)
- [x] s1engine facade integration (feature-gated `odt` support, 2 integration tests)
- [ ] Cross-format test: DOCX → model → ODT → model → compare content

### Milestone 2.6: Advanced DOCX Features (COMPLETE — 43 new tests)
- [x] Hyperlinks: external (rId resolution, relationship entries), internal (w:anchor), tooltip support
- [x] Bookmarks: BookmarkStart/BookmarkEnd read/write/round-trip
- [x] Comments: comments_parser.rs/comments_writer.rs, CommentBody nodes, commentRangeStart/End in document.xml, word/comments.xml in ZIP, builder API
- [x] Tab stops: parse_tabs/write tabs with left/center/right/decimal alignment and none/dot/dash/underscore leaders
- [x] Paragraph borders and shading: pBdr parsing/writing, Background attribute for shading
- [x] Character spacing: FontSpacing in run properties (twips ↔ points)
- [x] Superscript/subscript: vertAlign read/write/round-trip
- [x] Builder API: .hyperlink(), .bookmark_start()/.bookmark_end(), .superscript(), .subscript()
- [x] 11 content_parser tests, 8 content_writer tests, 10 writer round-trip tests, 4 comments_parser tests, 4 comments_writer tests, 5 builder tests, 1 builder round-trip test

### Phase 2 Deliverable
Full DOCX and ODT read/write covering text, formatting, tables, images, lists, headers/footers, sections, hyperlinks, bookmarks, comments, tab stops, paragraph borders/shading, character spacing, superscript/subscript.

---

## Phase 3: Layout & Export (COMPLETE)

**Completed**: 2026-03-12
**Goal**: Text shaping, page layout, PDF export, DOC conversion.

### Milestone 3.1: Text Processing — `s1-text` (COMPLETE — 39 tests)
- [x] Pure-Rust text shaping via `rustybuzz` (HarfBuzz port)
- [x] Font parsing via `ttf-parser` (TrueType/OpenType)
- [x] `FontDatabase` wrapping `fontdb` for system font discovery
- [x] Font fallback chain (missing glyph → try fallback fonts)
- [x] Text shaping pipeline: `&str + Font → Vec<ShapedGlyph>`
- [x] BiDi text support via `unicode-bidi`
- [x] Line break opportunities via `unicode-linebreak`
- [x] Font metrics (ascent, descent, line gap, underline)
- [x] OpenType feature support (ligatures, kerning, etc.)

### Milestone 3.2: Layout Engine — `s1-layout` (COMPLETE — 30 tests)
- [x] Style resolution: compute effective attributes for every node
- [x] Knuth-Plass optimal line breaking (with greedy fallback)
- [x] Paragraph layout → `Vec<LayoutLine>` with glyph runs
- [x] Block stacking (paragraphs with spacing-before/after)
- [x] Page breaking / pagination
- [x] Table layout: equal column width algorithm
- [x] Table cell layout (paragraphs inside cells)
- [x] Image placement (inline sizing with content-width constraint)
- [x] Page-break-before support
- [x] `LayoutDocument` output with pages, blocks, lines, glyph runs
- [x] Widow/orphan control (configurable min_orphan_lines, min_widow_lines)
- [x] Header/footer placement from SectionProperties
- [x] Page-number field substitution (PAGE/NUMPAGES)
- [x] Section page size resolution (reads from DocumentModel.sections())

### Milestone 3.3: Incremental Layout (COMPLETE — 8 tests)
- [x] Content-hash-based `LayoutCache` (FNV-1a hash of node attributes + descendant text)
- [x] `LayoutEngine::new_with_cache()` for cache-enabled layout
- [x] Per-block cache lookup before full layout, result stored after
- [x] Cache invalidation on text/style/insert changes
- [x] Tests: cache hit, cache miss on text/style change, pagination still correct, table cache, empty cache, invalidation on insert

### Milestone 3.4: PDF Export — `s1-format-pdf` (COMPLETE — 8 core tests)
- [x] PDF page generation from `LayoutDocument`
- [x] Text rendering with correct glyph positioning (CID fonts)
- [x] Font embedding with subsetting via `subsetter` (only used glyphs)
- [x] Font compression (FlateDecode)
- [x] Table borders rendering
- [x] Multi-page support
- [x] PDF metadata (title, author, subject)
- [x] Image placeholder rendering

### Milestone 3.5: Format Conversion — `s1-convert` (COMPLETE)
- [x] DOC reader: OLE2/CFB container via `cfb` crate with heuristic text extraction
- [x] DOC magic byte detection (`is_doc_file`)
- [x] Cross-format conversion pipeline: Source → DocumentModel → Target
- [x] Supported conversions: DOC→DOCX/ODT (text only), DOCX↔ODT (full model)
- [x] `convert()`, `convert_to_model()`, `detect_format()` API
- [x] SourceFormat (Doc, Docx, Odt), TargetFormat (Docx, Odt) enums
- [x] 15 tests (doc reader, format detection, cross-format round-trips)

### Milestone 3.6: PDF Polish (COMPLETE — 13 tests)
- [x] Image embedding: JPEG pass-through (DCTDecode), PNG decode to RGB + FlateDecode, deduplication
- [x] Hyperlink annotations: /Link with /URI action, computed from GlyphRun positions
- [x] Bookmarks / document outline: outline tree with /Dest [page /XYZ x y null]
- [x] Image dimension caps (16384px max)

### Phase 3 Deliverable
```rust
let doc = engine.open_file("report.docx")?;

// Page layout via the layout engine
let mut layout_engine = s1_layout::LayoutEngine::new(doc.model(), &font_db);
let layout_doc = layout_engine.layout()?;
println!("Pages: {}", layout_doc.pages.len());

// PDF export (layout -> PDF bytes -> write to file)
let pdf_bytes = s1_format_pdf::write_pdf(&layout_doc, &font_db, Some(doc.metadata()))?;
std::fs::write("report.pdf", pdf_bytes)?;

// DOCX export via Document API
let docx_bytes = doc.export(Format::Docx)?;
std::fs::write("report_copy.docx", docx_bytes)?;

// DOC conversion (legacy DOC -> DOCX bytes)
let doc_data = std::fs::read("legacy.doc")?;
let docx_bytes = s1_convert::convert(&doc_data, SourceFormat::Doc, TargetFormat::Docx)?;
std::fs::write("modern.docx", docx_bytes)?;
```

---

## Phase 4: Collaboration Foundation (COMPLETE)

**Completed**: 2026-03-12
**Tests**: 171 passing (138 unit + 16 convergence + 17 scenario integration tests)
**Approach**: Custom CRDT in new `s1-crdt` crate — no external CRDT deps. s1-model already had the right primitives (NodeId, Operations, inversion). Zero impact on existing 491 tests.

### Milestone 4.1: Core CRDT Primitives (COMPLETE — 25 tests)
- [x] `LamportClock` — scalar logical clock: `tick()`, `update(remote_ts)`, `current()`
- [x] `VectorClock` — `HashMap<u64, u64>` replica → highest timestamp: `merge()`, `dominates()`, `concurrent_with()`
- [x] `OpId { replica: u64, lamport: u64 }` — total order (lamport first, replica tiebreak)
- [x] `StateVector` — tracks highest OpId.lamport per replica: `includes()`, `diff()`, `merge()`
- [x] `CrdtOperation` — wraps `s1_ops::Operation` with `id`, `deps`, `origin_left/right`, `parent_op`
- [x] `CrdtError` — CausalityViolation, DuplicateOperation, InvalidOperation, etc.

### Milestone 4.2: CRDT Algorithms (COMPLETE — 40 tests)
- [x] **Fugue/YATA-based Text CRDT** (`text_crdt.rs`) — per-character OpId tracking, origin_left/right for deterministic concurrent insert ordering, YATA position comparison for convergence, tombstone deletes, `materialize()`, `offset_to_op_id()`
- [x] **Kleppmann Tree CRDT** (`tree_crdt.rs`) — insert/delete/move with tombstones, cycle detection for moves (drop cyclic), LWW among concurrent non-cyclic moves, `visible_children()`
- [x] **LWW Attribute CRDT** (`attr_crdt.rs`) — per-node per-key Last-Writer-Wins registers, concurrent different keys both apply, same key highest OpId wins
- [x] **LWW Metadata CRDT** (`metadata_crdt.rs`) — per-key LWW for document metadata
- [x] **CrdtResolver** (`resolver.rs`) — central conflict resolution coordinator, delegates to sub-CRDTs, returns per-character `Vec<Operation>` for text inserts, duplicate operation detection
- [x] **TombstoneTracker** (`tombstone.rs`) — tombstone management with GC support

### Milestone 4.3: Collaboration API (COMPLETE — 40 tests)
- [x] **CollabDocument** (`collab.rs`) — main consumer API wrapping DocumentModel + History + CrdtResolver
  - `apply_local(op)` → generate CrdtOperation for broadcast
  - `apply_remote(crdt_op)` → integrate with causal ordering (pending buffer for out-of-order)
  - `changes_since(sv)` → delta for incremental sync
  - `snapshot()` / `from_snapshot()` → initial sync (preserves resolver state)
  - `fork(new_replica_id)` → create new replica (no phantom state entries)
  - `undo()` / `redo()` → local only, generates CrdtOps for broadcast
- [x] **AwarenessState** (`awareness.rs`) — cursor/presence sharing, stale cursor removal
- [x] **Binary serialization** (`serialize.rs`) — custom varint-based format for CrdtOperation, StateVector, Snapshot
- [x] **Operation compression** (`compression.rs`) — merge consecutive single-char inserts from same replica

### Milestone 4.4: Collaboration Testing (COMPLETE — 33 integration tests)
- [x] **Convergence tests** (16 tests) — 2/3/5 replicas with random ops, delayed delivery, partition-and-heal, snapshot sync, fork-diverge-converge, incremental delta sync, idempotent sync
- [x] **Scenario tests** (17 tests) — concurrent insert at same offset (both preserved, deterministic order), concurrent bold + italic (both apply), concurrent same attribute (LWW), delete + modify (delete wins), undo local-only, multi-char insert sync, awareness cursor sharing

### Phase 4 Deliverable
```rust
use s1_crdt::CollabDocument;

let mut doc_a = CollabDocument::new(1);
let op = doc_a.apply_local(insert_text(node, 0, "Hello"))?;

let mut doc_b = doc_a.fork(2);
let op_b = doc_b.apply_local(insert_text(node, 0, "World"))?;

doc_a.apply_remote(op_b)?;
doc_b.apply_remote(op)?;

// Both replicas converge to same state
assert_eq!(doc_a.text_content(node), doc_b.text_content(node));
```

---

## Phase 5: Production Ready (COMPLETE)

**Completed**: 2026-03-12
**Goal**: WASM, C FFI, hardening, documentation, release.

### Milestone 5.1: WASM Bindings (COMPLETE — 12 tests)
- [x] `wasm-bindgen` API: WasmEngine, WasmDocument, WasmDocumentBuilder, WasmFontDatabase
- [x] WASM-compatible font loading (`FontDatabase::empty()` + `load_font_data()`, `#[cfg(not(target_arch = "wasm32"))]` guard)
- [x] Format detection, open/export, plain text, metadata, paragraph count
- [x] Document free/validity checking
- [x] DOCX export round-trip tests

### Milestone 5.2: C FFI Bindings (COMPLETE — 10 tests)
- [x] Opaque handles: S1Engine, S1Document, S1Error, S1Bytes, S1String
- [x] `extern "C"` functions: s1_engine_new/free/create/open, s1_document_free/export/plain_text/metadata_title/paragraph_count
- [x] Error handling: s1_error_message/free
- [x] Null-safety on all functions
- [x] Format roundtrip (DOCX → open → export TXT → verify)

### Milestone 5.3: Performance & Hardening (COMPLETE — 4 proptest tests)
- [x] Proptest: model tree invariants (random ops never produce invalid state)
- [x] Proptest: insert/delete text inversion roundtrip
- [x] Proptest: CRDT concurrent text inserts converge
- [x] ZIP bomb protection: 256MB max decompressed entry, 64MB max media entry (DOCX + ODT readers)
- [x] Image dimension caps: 16384px max (PDF writer)

### Milestone 5.4-5.5: Documentation & Release
- [x] CLAUDE.md project state fully updated
- [x] ROADMAP.md fully updated
- [ ] README.md update with quick start and badges
- [ ] CHANGELOG.md
- [ ] Doc comment audit on all public items
- [ ] User guide (`docs/GUIDE.md`)
- [ ] `cargo publish` in dependency order
- [ ] `wasm-pack publish` for NPM

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| OOXML spec complexity | High | High | Pragmatic subset; test against real files, not spec |
| CRDT for tree structures | High | High | **RESOLVED**: Custom Fugue text + Kleppmann tree CRDTs in s1-crdt |
| C++ FFI complexity | Medium | Medium | Use Rust wrappers; watch rustybuzz/icu4x for pure Rust |
| Performance targets | Medium | Medium | Profile early; incremental layout is key |
| DOC binary format | High | Medium | Use LibreOffice headless conversion, not native parsing |
| Cross-platform fonts | Medium | Medium | Use fontdb; test on all platforms in CI |
| WASM bundle size | Medium | Low | Feature flags, tree-shaking, split crates |

---

## Dependencies by Phase

### Phase 1 Rust Crates
| Crate | Purpose |
|---|---|
| `quick-xml` | XML parsing/writing |
| `zip` | ZIP archive handling |
| `encoding_rs` | Text encoding detection |
| `thiserror` | Error type derivation |
| `proptest` | Property-based testing (dev) |
| `pretty_assertions` | Better test diffs (dev) |

### Phase 2 Rust Crates
| Crate | Purpose |
|---|---|
| `criterion` | Benchmarking (dev) |

### Phase 3 Rust Crates (pure Rust — no C/C++ FFI)
| Crate | Purpose |
|---|---|
| `rustybuzz` | Text shaping (pure Rust HarfBuzz port) |
| `ttf-parser` | Font parsing (pure Rust) |
| `fontdb` | System font discovery |
| `unicode-bidi` | BiDi algorithm |
| `unicode-linebreak` | Line breaking (UAX #14) |
| `pdf-writer` | PDF generation |
| `subsetter` | Font subsetting |
| `image` | Image decoding |
| `cfb` | OLE2 compound file (DOC) |

### Phase 5 Crates
| Crate | Purpose |
|---|---|
| `wasm-bindgen` | WASM FFI |
| `cbindgen` | C header generation |
