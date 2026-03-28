# Document Model (s1-model) Architecture Review

Deep review of the core document tree structure, its strengths, and gaps.

**Crate location:** `crates/s1-model/src/`
**Audit date:** 2026-03-29

---

## Document Tree Structure

```
DocumentModel
├── nodes: HashMap<NodeId, Node>     (flat storage, O(1) lookup)
├── root: NodeId                      (always NodeId(0,0))
├── id_gen: IdGenerator              (per-replica counter)
├── styles: Vec<Style>               (named styles)
├── metadata: DocumentMetadata
├── media: MediaStore                (images/media)
├── numbering: NumberingDefinitions  (list definitions)
├── sections: Vec<SectionProperties> (page layout)
├── doc_defaults: DocumentDefaults   (base formatting)
├── style_cache: HashMap             (resolved style cache)
└── preserved_parts: HashMap         (raw ZIP entries for round-trip)
```

## Node Type Hierarchy

```
Document (root, exactly 1)
├── Body
│   ├── Section
│   │   ├── Paragraph
│   │   ├── Table
│   │   ├── Image
│   │   └── TableOfContents
│   ├── Paragraph
│   │   ├── Run
│   │   │   └── Text (leaf — stores string content)
│   │   ├── LineBreak (leaf)
│   │   ├── PageBreak (leaf)
│   │   ├── ColumnBreak (leaf)
│   │   ├── Tab (leaf)
│   │   ├── Image (inline, leaf)
│   │   ├── Drawing (shape, leaf)
│   │   ├── Field (leaf)
│   │   ├── Equation (leaf)
│   │   ├── BookmarkStart (leaf)
│   │   ├── BookmarkEnd (leaf)
│   │   ├── CommentStart (leaf)
│   │   ├── CommentEnd (leaf)
│   │   ├── FootnoteRef (leaf)
│   │   └── EndnoteRef (leaf)
│   ├── Table
│   │   └── TableRow
│   │       └── TableCell
│   │           ├── Paragraph (recursive)
│   │           └── Table (nested tables)
│   ├── Image (block-level)
│   ├── TableOfContents
│   ├── PageBreak
│   └── ColumnBreak
├── Header
│   ├── Paragraph
│   └── Table
├── Footer
│   ├── Paragraph
│   └── Table
├── CommentBody
│   └── Paragraph
├── FootnoteBody
│   └── Paragraph
└── EndnoteBody
    └── Paragraph
```

## Node Structure

```rust
pub struct Node {
    pub id: NodeId,                    // Globally unique (replica_id, counter)
    pub node_type: NodeType,           // Discriminant
    pub attributes: AttributeMap,      // Formatting key-value pairs
    pub children: Vec<NodeId>,         // Ordered child references
    pub parent: Option<NodeId>,        // Back-pointer (None only for root)
    pub text_content: Option<String>,  // Only for Text nodes
}
```

## NodeId System (CRDT-Ready)

```rust
pub struct NodeId {
    pub replica: u64,    // Site ID (0 for single-user)
    pub counter: u64,    // Monotonic counter per replica
}
```

- `NodeId::ROOT = (0, 0)` — always the document root
- Globally unique across all replicas
- Foundation for CRDT collaborative editing

---

## Strengths

### 1. Zero-Dependency Core
`s1-model` has NO external dependencies — pure Rust data structures. This is enforced and critical for portability (WASM, FFI, embedded).

### 2. CRDT-Ready from Day 1
Every node has a globally unique `(replica_id, counter)` pair. This was built in from the start, not bolted on later.

### 3. Flat Storage with Tree Semantics
`HashMap<NodeId, Node>` gives O(1) node lookup while parent/children references maintain tree structure. Good for both random access and traversal.

### 4. Typed Attribute System
~80 typed `AttributeKey` variants with typed `AttributeValue` enum. No stringly-typed errors. Compile-time safety for most formatting operations.

### 5. Style Resolution with Caching
Walk parent_id chains, merge attributes in priority order (direct > character style > paragraph style > defaults). Results cached and auto-invalidated on style changes.

### 6. Comprehensive Hierarchy Validation
`insert_node()` validates parent-child relationships via `can_contain()`. Cycle detection in `move_node()`. Cannot remove root.

### 7. Unicode-Safe Text Operations
Character-based offsets (not byte offsets). Full UTF-8 support including 4-byte emoji, BiDi text, combining characters.

### 8. Round-Trip Preservation
`preserved_parts` stores unmodeled ZIP entries (VBA, SmartArt, charts, signatures) for lossless round-trip.

---

## Gaps and Limitations

### GAP 1: No Rich Attribute Variants for Some OOXML/ODF Features

**Missing AttributeValue variants:**

| Missing Variant | Needed For | Current Workaround |
|----------------|------------|-------------------|
| `Caps/SmallCaps` | `w:caps`, `w:smallCaps` (OOXML) | None — dropped |
| `TextTransform` | `fo:text-transform` (ODF) | None — dropped |
| `RowHeight` | `w:trHeight` (OOXML), `style:row-height` (ODF) | None — dropped |
| `CellMargins` | `w:tblCellMar`, `w:tcMar` (OOXML) | None — dropped |
| `TableLayout` | `w:tblLayout` (OOXML) | None — dropped |
| `WritingMode` | `style:writing-mode` (ODF), `w:textDirection` (OOXML) | None — dropped |
| `PageBorders` | `w:pgBorders` (OOXML) | None — dropped |
| `WidowOrphan` | `w:widowControl` (OOXML), `fo:widows/orphans` (ODF) | None — dropped |
| `OutlineLevel` | `w:outlineLvl` (OOXML) | None — dropped |

**These features cannot be preserved in round-trip because the model has no place to store them.**

### GAP 2: Limited List Model

The current `ListInfo` stores:
```rust
pub struct ListInfo {
    pub level: u8,
    pub num_format: ListFormat,
    pub num_id: u32,
    pub start: Option<u32>,
}
```

Missing:
- **List continuation tracking** — no way to know if two consecutive paragraphs with same `num_id` and `level` should continue numbering or restart
- **Custom bullet characters** — only enum variants (Bullet, Decimal, etc.), no custom Unicode bullets
- **List style ID** — ODF `text:list-style-name` has no equivalent

### GAP 3: No Change Tracking Semantic Model

Track changes are stored as `RevisionType`, `RevisionAuthor`, `RevisionDate` attributes on nodes, plus raw XML. But there's no structured model for:
- Revision ranges (which nodes are part of which revision)
- Revision groups (related changes)
- Conflict resolution between revisions
- Accept/reject at granular level

### GAP 4: Field Model is Minimal

```rust
pub enum FieldType {
    PageNumber, PageCount, Date, Time, FileName, Author, TableOfContents, Custom
}
```

Missing:
- **Field instructions** — OOXML field codes like `TOC \o "1-3"`, `IF`, `MERGEFIELD`
- **Field result caching** — static value for display when field can't be computed
- **Nested fields** — fields within fields
- **Cross-reference fields** — REF, PAGEREF, SEQ, STYLEREF

### GAP 5: Section Model Separation

Sections are stored in a separate `Vec<SectionProperties>` array, linked to Body/Section nodes via `SectionIndex` attribute. This creates indirection:
- Paragraphs don't directly own their section — you must find the nearest ancestor Section node, then look up its index in the sections array
- Multi-section documents require careful index management
- Section breaks between paragraphs (without Section node wrappers) can be ambiguous

### GAP 6: No Drawing/Shape Model

The model has `NodeType::Drawing` and a few shape attributes (`ShapeType`, `ShapeWidth`, `ShapeHeight`, `ShapeFillColor`, etc.) plus `ShapeRawXml`. But:
- No geometry model (paths, points, transforms)
- No text-in-shape support
- No group shape support
- VML shapes are raw XML only

### GAP 7: No Math/Equation Model

`NodeType::Equation` exists but has no structured content. Equations are leaf nodes with no child structure. The math content (fractions, roots, matrices) is not modeled.

### GAP 8: Style Cache Uses Interior Mutability

`resolve_style_chain_cached()` uses an unsafe cast for shared-reference caching. While functionally correct, this is fragile:
- Not thread-safe
- Could be replaced with `RefCell` or `OnceCell` pattern

### GAP 9: DocumentDefaults is Minimal

```rust
pub struct DocumentDefaults {
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub line_spacing_multiple: Option<f64>,
    pub space_after: Option<f64>,
    pub space_before: Option<f64>,
}
```

Missing default values for:
- Default paragraph alignment
- Default indentation
- Default language
- Theme-derived defaults (colors, fonts)

### GAP 10: No Theme/Color Model

OOXML themes (theme colors like `accent1`, `accent2`, font schemes, format schemes) are not modeled. When DOCX references `w:themeColor="accent1"`, the resolved hex color is used but the theme reference is lost.

---

## Operations Model (s1-ops) Review

### Strengths

1. **Elegant inversion protocol** — Every operation automatically returns its inverse
2. **Atomic transactions** — Rollback on failure
3. **Deep subtree undo** — DeleteNode snapshots entire subtree for restoration
4. **Precise attribute undo** — Correctly handles add + overwrite + remove semantics
5. **48 tests** including property-based tests (proptest)

### Gaps

1. **No cursor adjustment after operations** — Editor must manually update Selection positions
2. **No operation merging** — Each keystroke is a separate undo step (workaround: `merge_undo_entries()`)
3. **No operation serialization** — Can't persist or transmit operations (no serde)
4. **Rollback is best-effort** — Failed rollback leaves inconsistent state
5. **DeleteNode snapshot uses full memory** — No lazy/streaming for large subtrees

---

## CRDT Model (s1-crdt) Review

### Strengths

1. **Custom Fugue-based text CRDT** — Character-level editing with deterministic ordering
2. **Tree CRDT with tombstones** — Kleppmann-style structural operations
3. **Per-key LWW attribute registers** — No formatting conflicts
4. **Causal ordering with state vectors** — Out-of-order ops buffered automatically
5. **Comprehensive convergence tests** — Multiple replicas verified identical

### Gaps

1. **Tombstone GC not implemented** — `TombstoneTracker` built but GC never called
2. **Op log unbounded** — Grows indefinitely, no compaction
3. **No range queries** — Can't efficiently query "ops affecting this paragraph"
4. **Limited awareness** — No session IDs, no reconnection semantics

---

## Layout Engine (s1-layout) Review

### Strengths

1. **Full pagination** with orphan/widow control
2. **Knuth-Plass line breaking** with greedy fallback
3. **Table layout** with cell sizing, spanning, nested tables
4. **Incremental block caching** — Reuse layouts for unchanged content
5. **HTML export** — CSS-positioned pages

### Gaps

1. **Pagination not truly incremental** — Block cache works, but pagination always runs from page 1
2. **engine.rs is 7,354 lines** — Needs modular decomposition
3. **Limited column support** — Multi-column recognized but basic
4. **No vertical text** — CJK vertical writing not supported

---

## Recommendations

### Priority 1: Model Completeness (enables format fidelity)

Add missing AttributeKey/Value variants:
- `WidowControl(bool)`, `OrphanControl(bool)` — affects pagination quality
- `OutlineLevel(u8)` — needed for TOC generation
- `Caps(bool)`, `SmallCaps(bool)` — common formatting
- `TextTransform(enum)` — ODF text transform
- `RowHeight(f64)` — table layout
- `CellMargins(Margins)` — table layout
- `TableLayout(enum)` — fixed vs auto
- `WritingMode(enum)` — BiDi/vertical support
- `PageBorders(Borders)` — page decoration

### Priority 2: List Model Enhancement

- Add `ListStyle` to model (custom bullets, formatting per level)
- Track list continuation state
- Support all 6+ common number formats from both OOXML and ODF

### Priority 3: Field Model Enhancement

- Add `FieldInstruction(String)` attribute for raw field codes
- Add `FieldResult(String)` for cached display text
- Parse at least TOC, HYPERLINK, REF, PAGEREF field types

### Priority 4: Change Tracking Model

- Add `RevisionRange` type linking start/end markers to content spans
- Support accept/reject at the model level (not just s1engine facade)
- Structured parsing for ODF change tracking (not just raw XML)

### Priority 5: Code Health

- Split `s1-layout/engine.rs` (7.3k lines) into submodules
- Split `s1-text/font_db.rs` (20k lines) into submodules
- Replace interior mutability in style cache with safe pattern
- Add operation serialization (serde feature flag)
- Implement tombstone GC in s1-crdt
