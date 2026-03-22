# Architecture

## System Overview

```
+------------------------------------------------------------+
|                    Consumer Applications                    |
|          (Web Editor, Desktop App, CLI Tool, etc.)          |
+----------------------------+-------------------------------+
                             | Public API (Rust / C FFI / WASM)
+----------------------------v-------------------------------+
|                      s1engine (facade)                      |
|               High-level API tying all modules              |
|------------------------------------------------------------+
|                                                             |
|  +-----------+  +------------+  +-----------------------+   |
|  |  s1-ops   |  | s1-layout  |  |     s1-convert        |   |
|  | Operations|  | Page Layout|  | Format Conversion     |   |
|  | Undo/Redo |  | Pagination |  | DOC text extraction   |   |
|  +-----+-----+  +-----+------+  +-----------------------+   |
|        |              |                                      |
|  +-----v--------------v-----------------------------------+ |
|  |                     s1-model                            | |
|  |           Core Document Model (Tree/DOM)                | |
|  |       Nodes, Attributes, Styles, Metadata               | |
|  |           Unique IDs (CRDT-ready)                       | |
|  +-----+---------------------------------------------------+ |
|        |                                                     |
|  +-----v---------------------------------------------------+ |
|  |   s1-crdt           |         Format I/O Layer          | |
|  |  Fugue text CRDT    | docx | odt | pdf  | txt | xlsx   | |
|  |  Tree/Attr/Meta     | R/W  | R/W | Exp  | R/W | R/W    | |
|  +----------------------+------+-----+------+-----+-------+ |
|                                                              |
|  +----------------------------------------------------------+|
|  |              s1-text (Text Processing -- Pure Rust)       ||
|  |        rustybuzz  ttf-parser  fontdb  unicode-bidi        ||
|  +----------------------------------------------------------+|
+--------------------------------------------------------------+
```

## Crate Structure

```
s1engine/
|-- Cargo.toml                    # Workspace root
|-- CLAUDE.md                     # AI development context
|-- README.md
|-- LICENSE                      # AGPL-3.0-or-later
|-- crates/
|   |-- s1-model/                 # Core document model (zero deps)
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- node.rs           # Node types (Paragraph, Run, Table, etc.)
|   |       |-- tree.rs           # DocumentModel tree and traversal
|   |       |-- attributes.rs     # AttributeKey/Value/Map
|   |       |-- styles.rs         # Style definitions and resolution
|   |       |-- metadata.rs       # DocumentMetadata
|   |       |-- id.rs             # NodeId (replica_id, counter)
|   |       |-- media.rs          # MediaStore for embedded content
|   |       |-- numbering.rs      # List numbering definitions
|   |       +-- section.rs        # SectionProperties, HeaderFooterRef
|   |
|   |-- s1-ops/                   # Operations on the document model
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- operation.rs      # Operation enum, apply(), validate()
|   |       |-- transaction.rs    # Atomic operation batches
|   |       |-- history.rs        # Undo/redo stack
|   |       +-- cursor.rs         # Position and Selection
|   |
|   |-- s1-format-docx/           # DOCX (OOXML) reader/writer
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- reader.rs         # ZIP/XML orchestration
|   |       |-- writer.rs         # DOCX ZIP packaging
|   |       |-- content_parser.rs # <w:body> XML -> model nodes
|   |       |-- content_writer.rs # Model nodes -> <w:body> XML
|   |       |-- property_parser.rs# Run/paragraph properties
|   |       |-- style_parser.rs   # styles.xml parser
|   |       |-- style_writer.rs   # styles.xml writer
|   |       |-- numbering_parser.rs # numbering.xml parser
|   |       |-- numbering_writer.rs # numbering.xml writer
|   |       |-- section_parser.rs # sectPr parser
|   |       |-- section_writer.rs # sectPr writer
|   |       |-- header_footer_parser.rs
|   |       |-- header_footer_writer.rs
|   |       |-- metadata_parser.rs # docProps/core.xml
|   |       |-- metadata_writer.rs
|   |       |-- comments_parser.rs # comments.xml
|   |       |-- comments_writer.rs
|   |       |-- xml_util.rs       # XML attribute helpers
|   |       +-- error.rs          # DocxError
|   |
|   |-- s1-format-odt/            # ODT (ODF) reader/writer
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- reader.rs
|   |       |-- writer.rs
|   |       |-- content_parser.rs
|   |       |-- content_writer.rs
|   |       |-- property_parser.rs
|   |       |-- property_writer.rs
|   |       |-- style_parser.rs
|   |       |-- style_writer.rs
|   |       |-- metadata_parser.rs
|   |       |-- metadata_writer.rs
|   |       |-- manifest_writer.rs
|   |       |-- xml_util.rs
|   |       +-- error.rs
|   |
|   |-- s1-format-pdf/            # PDF export
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- writer.rs         # PDF generation from LayoutDocument
|   |       +-- error.rs
|   |
|   |-- s1-format-txt/            # Plain text reader/writer
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- reader.rs
|   |       |-- writer.rs
|   |       +-- error.rs
|   |
|   |-- s1-format-xlsx/           # XLSX/ODS/CSV spreadsheet reader/writer
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- reader.rs         # XLSX ZIP/XML reader
|   |       |-- writer.rs         # XLSX ZIP packaging
|   |       |-- shared_strings.rs # Shared string table
|   |       |-- styles.rs         # Number formats, fonts, fills, borders
|   |       |-- formula.rs        # Formula tokenizer, parser, evaluator (60+ functions)
|   |       |-- ods.rs            # ODS (ODF spreadsheet) reader/writer
|   |       +-- error.rs
|   |
|   |-- s1-convert/               # Format conversion pipelines
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- convert.rs        # Conversion pipeline
|   |       |-- doc_reader.rs     # Legacy DOC text extraction
|   |       +-- error.rs
|   |
|   |-- s1-layout/                # Layout engine
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- engine.rs         # LayoutEngine, pagination, line breaking
|   |       |-- types.rs          # LayoutDocument, LayoutPage, LayoutBlock
|   |       |-- style_resolver.rs # Style chain resolution
|   |       +-- error.rs
|   |
|   |-- s1-text/                  # Text processing (pure Rust)
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- shaping.rs        # Text shaping via rustybuzz
|   |       |-- font.rs           # Font parsing via ttf-parser
|   |       |-- font_db.rs        # System font discovery via fontdb
|   |       |-- bidi.rs           # BiDi resolution via unicode-bidi
|   |       |-- linebreak.rs      # Line breaking via unicode-linebreak
|   |       |-- types.rs          # ShapedGlyph, GlyphRun, etc.
|   |       +-- error.rs
|   |
|   |-- s1-crdt/                  # CRDT algorithms
|   |   +-- src/
|   |       |-- lib.rs
|   |       |-- text_crdt.rs      # Fugue-based text CRDT
|   |       |-- tree_crdt.rs      # Kleppmann tree moves
|   |       |-- attr_crdt.rs      # Per-key LWW attributes
|   |       |-- metadata_crdt.rs  # LWW metadata/styles
|   |       |-- resolver.rs       # CrdtResolver coordinator
|   |       |-- collab.rs         # CollabDocument API
|   |       |-- awareness.rs      # Cursor/presence state
|   |       |-- serialize.rs      # Binary serialization
|   |       |-- compression.rs    # Op compression
|   |       |-- clock.rs          # LamportClock, VectorClock
|   |       |-- op_id.rs          # OpId (lamport + replica)
|   |       |-- state_vector.rs   # StateVector for sync
|   |       |-- crdt_op.rs        # CrdtOperation enum
|   |       |-- tombstone.rs      # TombstoneTracker
|   |       +-- error.rs
|   |
|   +-- s1engine/                 # High-level facade crate
|       +-- src/
|           |-- lib.rs            # Re-exports
|           |-- engine.rs         # Engine (factory)
|           |-- document.rs       # Document (high-level wrapper)
|           |-- builder.rs        # DocumentBuilder, ParagraphBuilder, etc.
|           |-- format.rs         # Format enum, detection
|           +-- error.rs          # Unified Error type
|
|-- ffi/
|   |-- c/                        # C FFI bindings
|   |   +-- src/lib.rs            # Opaque handles, extern "C" functions
|   +-- wasm/                     # WASM bindings
|       +-- src/lib.rs            # wasm-bindgen wrapper types
|
+-- docs/                         # Documentation
    |-- OVERVIEW.md
    |-- ARCHITECTURE.md
    |-- SPECIFICATION.md
    |-- ROADMAP.md
    |-- API_DESIGN.md
    +-- DEPENDENCIES.md
```

## Core Design Decisions

### 1. Document Model: Tree with Unique IDs

The document model is a tree structure (similar to a DOM) where every node has a globally unique ID. This is critical for CRDT support -- every element must be independently addressable.

```
Document
|-- Body
|   |-- Paragraph [id: (0, 1)]
|   |   |-- Run [id: (0, 2)] { bold: true }
|   |   |   +-- Text "Hello "
|   |   +-- Run [id: (0, 3)] { italic: true }
|   |       +-- Text "world"
|   |-- Table [id: (0, 4)]
|   |   +-- Row [id: (0, 5)]
|   |       |-- Cell [id: (0, 6)]
|   |       |   +-- Paragraph [id: (0, 7)] ...
|   |       +-- Cell [id: (0, 8)]
|   |           +-- Paragraph [id: (0, 9)] ...
|   +-- Paragraph [id: (0, 10)]
|       +-- Run [id: (0, 11)]
|           +-- Text "End of doc"
|-- Styles
|   |-- ParagraphStyle "Heading1" { font_size: 24, bold: true }
|   +-- CharacterStyle "Emphasis" { italic: true }
|-- Headers/Footers
+-- Metadata { title, author, created, modified }
```

**Node ID Strategy:**
- Each node gets a `NodeId` composed of `(replica_id, counter)`.
- For single-user mode, `replica_id` is always `0` -- no overhead.
- When CRDT is enabled, `replica_id` differentiates users, enabling merge.
- This is the same approach used by Yjs, Automerge, and Diamond Types.

### 2. Operation-Based Editing

All mutations go through an **operation** layer -- never direct tree manipulation. This is non-negotiable for undo/redo and CRDT support.

```
Operation::InsertNode  { parent: (0,1), index: 1, node: Run { ... } }
Operation::DeleteNode  { target: (0,3) }
Operation::SetAttributes { target: (0,2), attrs: { bold: false } }
Operation::InsertText  { target: (0,5), offset: 3, text: "new" }
Operation::DeleteText  { target: (0,5), offset: 0, length: 5 }
Operation::MoveNode    { target: (0,3), new_parent: (0,7), index: 0 }
```

Benefits:
- **Undo/redo** -- every operation produces its own inverse
- **Collaboration** -- operations can be broadcast and replayed via CRDT
- **History** -- full audit trail of changes
- **Validation** -- operations are validated before application

### 3. Format I/O as Separate Crates

Each format (DOCX, ODT, PDF, TXT) is an independent crate that only depends on `s1-model`. This means:
- You can use DOCX support without pulling in PDF dependencies
- Each format can be developed/tested independently
- Adding new formats doesn't touch existing code

### 4. Pure Rust Text Processing

s1-text uses pure-Rust alternatives to traditional C/C++ libraries:

| Purpose | Library | Notes |
|---|---|---|
| Text shaping | `rustybuzz` | Pure Rust port of HarfBuzz |
| Font parsing | `ttf-parser` | Zero-copy TrueType/OpenType parser |
| Font discovery | `fontdb` | System font indexing |
| BiDi | `unicode-bidi` | UAX #9 implementation |
| Line breaking | `unicode-linebreak` | UAX #14 implementation |

This eliminates all C/C++ build dependencies, enables WASM compilation, and simplifies cross-compilation.

### 5. Layout Engine: Incremental

The layout engine computes page geometry from the document model. It supports incremental re-layout via a `LayoutCache` that caches per-block results by content hash.

```
Document Model  ->  Layout Tree  ->  Render Output
(logical)           (physical)       (PDF/screen)

Paragraph (0,1) ->  LayoutBlock {
                        x: 72, y: 100,
                        width: 468, height: 36,
                        lines: [
                          Line { glyphs: [...], y: 100 },
                          Line { glyphs: [...], y: 118 },
                        ]
                    }
```

Layout features:
- Knuth-Plass optimal line breaking
- Table column width calculation
- Image placement
- Header/footer placement from section properties
- Widow/orphan control
- Page-number field substitution

### 6. CRDT Collaboration

The CRDT layer (s1-crdt) implements conflict-free replicated editing:

- **Fugue text CRDT** -- character-level concurrent text editing with YATA integration points
- **Tree CRDT** -- Kleppmann-style tree moves with cycle detection
- **LWW attribute CRDT** -- per-key last-writer-wins for formatting
- **LWW metadata CRDT** -- for document metadata and styles
- **CollabDocument** -- high-level API wrapping all CRDTs with causal ordering, state vectors, snapshot/restore, and local undo/redo
- **Binary serialization** -- compact wire format for operations
- **Operation compression** -- merge consecutive single-character inserts

### 7. Format Detection

When opening a document from bytes, s1engine auto-detects the format:

| Magic Bytes | Format |
|---|---|
| `PK\x03\x04` (ZIP header) | DOCX, ODT, XLSX, or ODS (disambiguate by ZIP contents) |
| `%PDF` | PDF (viewing via PDF.js in Rudra Office) |
| `\xD0\xCF\x11\xE0` (OLE2) | Legacy DOC -- route to converter |
| UTF-8 BOM or printable ASCII | TXT or CSV (auto-detect delimiter) |
| UTF-16 BOM | TXT (with encoding conversion) |

For ZIP files, check for `word/document.xml` (DOCX), `content.xml` + `META-INF/manifest.xml` (ODT or ODS), or `xl/workbook.xml` (XLSX).

## Dependency Graph

```
s1engine (facade)
|-- s1-model          (zero external deps, core types)
|-- s1-ops            (depends on: s1-model)
|-- s1-crdt           (depends on: s1-model, s1-ops)
|-- s1-layout         (depends on: s1-model, s1-text, fontdb)
|-- s1-format-docx    (depends on: s1-model, quick-xml, zip, base64)
|-- s1-format-odt     (depends on: s1-model, quick-xml, zip)
|-- s1-format-pdf     (depends on: s1-model, s1-layout, s1-text, pdf-writer, subsetter, image)
|-- s1-format-txt     (depends on: s1-model, encoding_rs)
|-- s1-format-xlsx    (depends on: s1-model, quick-xml, zip)
|-- s1-convert        (depends on: s1-format-docx, s1-format-odt, cfb)
+-- s1-text           (depends on: s1-model, rustybuzz, ttf-parser, fontdb, unicode-bidi, unicode-linebreak)
```

**Key principle**: `s1-model` has ZERO external dependencies. It is pure Rust data structures. Everything else is optional.

## Error Handling Strategy

- All public APIs return `Result<T, s1engine::Error>`
- `s1engine::Error` is an enum with variants per subsystem (`Format`, `Operation`, `Io`, `UnsupportedFormat`, `Crdt`)
- No panics in library code
- Invalid documents produce warnings, not crashes (lenient in parsing, strict in writing)

## Thread Safety

- `Document` is `Send + Sync` (can be shared across threads safely)
- Layout computation is parallelizable per-page
- Format I/O is single-threaded per document but multiple documents can be processed concurrently
