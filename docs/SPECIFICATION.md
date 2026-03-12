# Technical Specification

## 1. Document Model (`s1-model`)

### 1.1 Node Types

The document is a tree of typed nodes. Every node type maps to constructs found in both OOXML (DOCX) and ODF (ODT).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeType {
    // Root
    Document,

    // Structural
    Body,
    Section,          // Page layout properties (margins, orientation, columns)

    // Block-level
    Paragraph,
    Table,
    TableRow,
    TableCell,

    // Inline-level
    Run,              // Contiguous text with uniform formatting
    Text,             // Raw text content (leaf node)
    LineBreak,
    PageBreak,
    ColumnBreak,
    Tab,

    // Objects
    Image,            // Inline or floating image
    Drawing,          // Vector drawing / shape

    // Lists
    ListItem,         // Paragraph with list context

    // Headers/Footers
    Header,
    Footer,

    // Fields
    Field,            // Dynamic fields (page number, date, TOC, etc.)

    // Annotations
    BookmarkStart,
    BookmarkEnd,
    CommentStart,
    CommentEnd,
    CommentBody,      // Comment content container
}
```

**Node type hierarchy constraints** (enforced by validation):

| Parent | Allowed Children |
|---|---|
| `Document` | `Body`, `Header`, `Footer` |
| `Body` | `Section`, `Paragraph`, `Table` |
| `Section` | `Paragraph`, `Table` |
| `Paragraph` | `Run`, `LineBreak`, `PageBreak`, `Tab`, `Image`, `BookmarkStart`, `BookmarkEnd`, `CommentStart`, `CommentEnd`, `Field` |
| `Run` | `Text` (exactly one) |
| `Table` | `TableRow` |
| `TableRow` | `TableCell` |
| `TableCell` | `Paragraph`, `Table` (recursive) |
| `Header` | `Paragraph`, `Table` |
| `Footer` | `Paragraph`, `Table` |

### 1.2 Node Structure

```rust
/// A single node in the document tree.
#[derive(Debug, Clone)]
pub struct Node {
    /// Globally unique identifier (CRDT-ready)
    pub id: NodeId,

    /// Type of this node
    pub node_type: NodeType,

    /// Formatting and properties
    pub attributes: AttributeMap,

    /// Children (ordered). Empty for leaf nodes.
    pub children: Vec<NodeId>,

    /// Parent reference. None only for Document root.
    pub parent: Option<NodeId>,

    /// For Text nodes: the text content. None for non-text nodes.
    pub text_content: Option<String>,
}

/// CRDT-compatible unique identifier.
/// Composed of (replica_id, counter) — globally unique across all replicas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeId {
    /// Replica/site identifier. 0 for single-user mode.
    pub replica: u64,

    /// Monotonically increasing counter per replica.
    pub counter: u64,
}

impl NodeId {
    pub const ROOT: NodeId = NodeId { replica: 0, counter: 0 };
}

/// Flexible attribute storage using typed keys.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttributeMap {
    inner: HashMap<AttributeKey, AttributeValue>,
}

/// Typed attribute keys (prevents stringly-typed errors).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AttributeKey {
    // Run attributes
    FontFamily,
    FontSize,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Color,
    HighlightColor,
    Superscript,
    Subscript,
    FontSpacing,
    Language,

    // Paragraph attributes
    Alignment,
    IndentLeft,
    IndentRight,
    IndentFirstLine,
    SpacingBefore,
    SpacingAfter,
    LineSpacing,
    KeepWithNext,
    KeepLinesTogether,
    PageBreakBefore,
    Borders,
    Background,
    TabStops,
    StyleId,
    ListInfo,

    // Section attributes
    PageWidth,
    PageHeight,
    MarginTop,
    MarginBottom,
    MarginLeft,
    MarginRight,
    Columns,
    ColumnSpacing,
    Orientation,
    HeaderDistance,
    FooterDistance,

    // Table attributes
    TableWidth,
    TableAlignment,
    TableBorders,
    CellMargins,

    // Cell attributes
    CellWidth,
    VerticalAlign,
    CellBorders,
    CellBackground,
    ColSpan,
    RowSpan,

    // Image attributes
    ImageMediaId,
    ImageWidth,
    ImageHeight,
    ImageAltText,

    // Field attributes
    FieldType,
    FieldCode,

    // Link attributes
    HyperlinkUrl,
    HyperlinkTooltip,
    BookmarkName,

    // Comment attributes
    CommentId,
    CommentAuthor,
    CommentDate,
}

/// Typed attribute values.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Color(Color),
    Alignment(Alignment),
    UnderlineStyle(UnderlineStyle),
    LineSpacing(LineSpacing),
    Borders(Borders),
    TabStops(Vec<TabStop>),
    ListInfo(ListInfo),
    PageOrientation(PageOrientation),
    TableWidth(TableWidth),
    VerticalAlignment(VerticalAlignment),
    MediaId(MediaId),
    FieldType(FieldType),
}
```

### 1.3 Supporting Types

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,  // 255 = fully opaque
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };

    pub fn from_hex(hex: &str) -> Result<Color, ParseError>;
    pub fn to_hex(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle {
    Single,
    Double,
    Thick,
    Dotted,
    Dashed,
    Wave,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineSpacing {
    Single,
    OnePointFive,
    Double,
    Exact(f64),    // Exact spacing in points
    AtLeast(f64),  // Minimum spacing in points
    Multiple(f64), // Multiple of line height
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableWidth {
    Auto,
    Fixed(f64),       // in points
    Percent(f64),     // 0.0 to 100.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TabStop {
    pub position: f64,         // in points from left margin
    pub alignment: TabAlignment,
    pub leader: TabLeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAlignment { Left, Center, Right, Decimal }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabLeader { None, Dot, Dash, Underscore }

#[derive(Debug, Clone, PartialEq)]
pub struct ListInfo {
    pub level: u8,             // 0-8, nesting depth
    pub num_format: ListFormat,
    pub num_id: u32,           // References numbering definition
    pub start: Option<u32>,    // Override start number
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    Bullet,
    Decimal,           // 1, 2, 3
    LowerAlpha,        // a, b, c
    UpperAlpha,        // A, B, C
    LowerRoman,        // i, ii, iii
    UpperRoman,        // I, II, III
}

#[derive(Debug, Clone, PartialEq)]
pub struct Borders {
    pub top: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
    pub right: Option<BorderSide>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderSide {
    pub style: BorderStyle,
    pub width: f64,       // in points
    pub color: Color,
    pub spacing: f64,     // space between border and content
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None, Single, Double, Dashed, Dotted, Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    PageNumber,
    PageCount,
    Date,
    Time,
    FileName,
    Author,
    TableOfContents,
    Custom,
}
```

### 1.4 Style System

Styles form an inheritance chain. A Run's effective formatting is resolved by:

```
Direct formatting → Character style → Paragraph style → Default style
(highest priority)                                      (lowest priority)
```

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub id: String,
    pub name: String,
    pub style_type: StyleType,
    pub parent_id: Option<String>,       // Inherits from parent style
    pub next_style_id: Option<String>,   // Style for next paragraph after Enter
    pub attributes: AttributeMap,        // All attributes this style defines
    pub is_default: bool,                // Is this the default for its type?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleType {
    Paragraph,
    Character,
    Table,
    List,
}
```

**Style resolution algorithm:**
1. Start with the node's direct `attributes`
2. If the node references a character style (via `StyleId` attribute), overlay its attributes (direct wins on conflict)
3. Walk up to the node's paragraph ancestor. If paragraph references a paragraph style, overlay its attributes
4. If any style has a `parent_id`, recursively resolve the parent chain
5. Finally, overlay the document's default style for that type
6. Result: fully resolved `AttributeMap` with no `None` gaps for standard properties

### 1.5 Document Metadata

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub created: Option<String>,       // ISO 8601 datetime string
    pub modified: Option<String>,      // ISO 8601 datetime string
    pub revision: Option<u32>,
    pub language: Option<String>,      // BCP 47 language tag
    pub custom_properties: HashMap<String, String>,
}
```

### 1.6 Media Storage

```rust
#[derive(Debug, Clone, Default)]
pub struct MediaStore {
    items: HashMap<MediaId, MediaItem>,
    next_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaItem {
    pub id: MediaId,
    pub content_type: String,            // MIME type (e.g., "image/png")
    pub data: Vec<u8>,                   // Raw bytes
    pub filename: Option<String>,        // Original filename
}

impl MediaStore {
    /// Insert media. Returns existing MediaId if content already stored (dedup by hash).
    pub fn insert(&mut self, content_type: String, data: Vec<u8>) -> MediaId;

    /// Get media by ID.
    pub fn get(&self, id: MediaId) -> Option<&MediaItem>;
}
```

### 1.7 Document Tree Container

```rust
/// The complete document model.
#[derive(Debug, Clone)]
pub struct DocumentModel {
    /// All nodes, indexed by NodeId.
    nodes: HashMap<NodeId, Node>,

    /// The root node ID (always NodeId::ROOT, type Document).
    root: NodeId,

    /// ID generator for this replica.
    id_gen: IdGenerator,

    /// Named styles.
    styles: Vec<Style>,

    /// Document metadata.
    metadata: DocumentMetadata,

    /// Embedded media (images, etc.).
    media: MediaStore,

    /// Numbering/list definitions (for DOCX compatibility).
    numbering_definitions: Vec<NumberingDefinition>,
}

impl DocumentModel {
    pub fn new() -> Self;
    pub fn new_with_replica(replica_id: u64) -> Self;

    // Tree operations (used internally by s1-ops, not public)
    pub(crate) fn insert_node(&mut self, parent: NodeId, index: usize, node: Node) -> Result<(), ModelError>;
    pub(crate) fn remove_node(&mut self, id: NodeId) -> Result<Node, ModelError>;
    pub(crate) fn move_node(&mut self, id: NodeId, new_parent: NodeId, index: usize) -> Result<(), ModelError>;

    // Tree queries (public)
    pub fn node(&self, id: NodeId) -> Option<&Node>;
    pub fn root(&self) -> &Node;
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = &Node>;
    pub fn parent(&self, id: NodeId) -> Option<&Node>;
    pub fn ancestors(&self, id: NodeId) -> impl Iterator<Item = &Node>;
    pub fn descendants(&self, id: NodeId) -> impl Iterator<Item = &Node>;  // DFS
    pub fn node_count(&self) -> usize;

    // Style queries
    pub fn styles(&self) -> &[Style];
    pub fn style_by_id(&self, id: &str) -> Option<&Style>;
    pub fn resolve_attributes(&self, node_id: NodeId) -> AttributeMap; // Fully resolved

    // Metadata and media
    pub fn metadata(&self) -> &DocumentMetadata;
    pub fn media(&self) -> &MediaStore;
}

/// Generates unique NodeIds for a given replica.
#[derive(Debug, Clone)]
struct IdGenerator {
    replica: u64,
    counter: u64,
}

impl IdGenerator {
    pub fn next(&mut self) -> NodeId;
}
```

---

## 2. Operations (`s1-ops`)

### 2.1 Operation Types

```rust
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    /// Insert a new node as child of parent at given index.
    InsertNode {
        parent_id: NodeId,
        index: usize,
        node: Node,
    },

    /// Delete a node and all its descendants.
    DeleteNode {
        target_id: NodeId,
    },

    /// Move a node to a new parent/position.
    MoveNode {
        target_id: NodeId,
        new_parent_id: NodeId,
        new_index: usize,
    },

    /// Insert text into a Text node at given character offset.
    InsertText {
        target_id: NodeId,
        offset: usize,
        text: String,
    },

    /// Delete text from a Text node.
    DeleteText {
        target_id: NodeId,
        offset: usize,
        length: usize,
    },

    /// Set attributes on a node (merge with existing).
    SetAttributes {
        target_id: NodeId,
        attributes: AttributeMap,
    },

    /// Remove specific attributes from a node.
    RemoveAttributes {
        target_id: NodeId,
        keys: Vec<AttributeKey>,
    },

    /// Split a node at a given offset.
    /// For Paragraph: split into two paragraphs at the given child index.
    /// For Text: split the text content, creating a new Run + Text.
    SplitNode {
        target_id: NodeId,
        offset: usize,
    },

    /// Merge two adjacent sibling nodes into one.
    MergeNodes {
        target_id: NodeId,
        merge_with_id: NodeId,
    },

    /// Set document-level metadata.
    SetMetadata {
        key: String,
        value: Option<String>,
    },

    /// Add/replace embedded media.
    InsertMedia {
        media_item: MediaItem,
    },

    /// Set or update a style definition.
    SetStyle {
        style: Style,
    },

    /// Remove a style definition.
    RemoveStyle {
        style_id: String,
    },
}
```

### 2.2 Operation Execution

```rust
/// Apply an operation to a document model. Returns the inverse operation (for undo).
pub fn apply(model: &mut DocumentModel, op: &Operation) -> Result<Operation, OperationError>;

/// Validate an operation without applying it.
pub fn validate(model: &DocumentModel, op: &Operation) -> Result<(), OperationError>;
```

### 2.3 Transaction Model

```rust
/// A group of operations that form an atomic undo/redo unit.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: u64,
    pub operations: Vec<Operation>,
    pub inverse_operations: Vec<Operation>,  // For undo (in reverse order)
    pub timestamp: String,                    // ISO 8601
    pub description: String,                  // e.g., "Bold selection"
}
```

### 2.4 Cursor and Selection

```rust
/// A position in the document (between two characters or at node boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// The node containing this position.
    pub node_id: NodeId,
    /// Character offset within text node, or child index within container.
    pub offset: usize,
}

/// A selection is an anchor + focus. When collapsed (anchor == focus), it's a cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

impl Selection {
    pub fn collapsed(pos: Position) -> Self;
    pub fn is_collapsed(&self) -> bool;
    pub fn is_forward(&self) -> bool;
    pub fn start(&self) -> Position;  // min(anchor, focus)
    pub fn end(&self) -> Position;    // max(anchor, focus)
}
```

### 2.5 Undo/Redo History

```rust
pub struct History {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    max_depth: usize,                // Configurable limit (default: 100)
}

impl History {
    pub fn push(&mut self, txn: Transaction);
    pub fn undo(&mut self, model: &mut DocumentModel) -> Result<(), OperationError>;
    pub fn redo(&mut self, model: &mut DocumentModel) -> Result<(), OperationError>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn clear(&mut self);
}
```

---

## 3. Format: DOCX (`s1-format-docx`)

### 3.1 DOCX Structure

A .docx file is a ZIP archive containing XML files (OOXML / ECMA-376):

```
[Content_Types].xml
_rels/.rels
word/
  document.xml          <- Main document body
  styles.xml            <- Style definitions
  numbering.xml         <- List/numbering definitions
  settings.xml          <- Document settings
  fontTable.xml         <- Font declarations
  header1.xml           <- Header content
  footer1.xml           <- Footer content
  _rels/document.xml.rels  <- Relationships
  media/                <- Embedded images
    image1.png
docProps/
  core.xml              <- Dublin Core metadata (title, author, dates)
  app.xml               <- Application metadata (word count, etc.)
```

### 3.2 Reader Specification

**Input**: `&[u8]` or `impl Read + Seek`
**Output**: `Result<DocumentModel, DocxError>`

Steps:
1. Open ZIP archive (`zip` crate)
2. Parse `[Content_Types].xml` to discover parts
3. Parse `_rels/.rels` and `word/_rels/document.xml.rels` for relationships
4. Parse `docProps/core.xml` → `DocumentMetadata`
5. Parse `word/styles.xml` → `Vec<Style>`
6. Parse `word/numbering.xml` → numbering definitions
7. Parse `word/document.xml` → document tree (paragraphs, runs, tables, etc.)
8. Extract `word/media/*` → `MediaStore`
9. Parse `word/header*.xml` and `word/footer*.xml` if present
10. Assemble `DocumentModel`

**OOXML Elements Supported:**

| OOXML Element | Support Level | Maps To | Phase |
|---|---|---|---|
| `w:p` (paragraph) | Full | `Paragraph` node | 1 |
| `w:r` (run) | Full | `Run` node | 1 |
| `w:t` (text) | Full | `Text` node | 1 |
| `w:rPr` (run properties) | Most | `RunAttributes` | 1 |
| `w:pPr` (para properties) | Most | `ParagraphAttributes` | 1 |
| `w:style` | Most | `Style` | 1 |
| `w:br` (breaks) | Full | Break nodes | 1 |
| `w:tab` | Full | `Tab` node | 1 |
| `w:tbl` (table) | Full | `Table` node | 2 |
| `w:tr` (table row) | Full | `TableRow` node | 2 |
| `w:tc` (table cell) | Full | `TableCell` node | 2 |
| `w:tcPr` (cell properties) | Full | `CellAttributes` | 2 |
| `w:drawing` (images) | Basic | `Image` node | 2 |
| `w:hyperlink` | Full | Attribute on Run | 2 |
| `w:bookmarkStart/End` | Full | Bookmark nodes | 2 |
| `w:numPr` (numbering) | Full | `ListInfo` | 2 |
| `w:sectPr` (section props) | Full | `SectionAttributes` | 2 |
| `w:hdr/w:ftr` | Basic | Header/Footer | 2 |
| `w:commentRangeStart/End` | Basic | Comment nodes | 2 |
| `w:fldSimple` / `w:fldChar` | Basic | `Field` nodes | 2 |

**Elements Deferred:**
- `w:smartTag`, `w:sdt` (structured doc tags) — complex, low priority
- `mc:AlternateContent` — read preferred choice only
- VML drawings — legacy, skip or convert to basic shape
- Embedded OLE objects — skip with placeholder warning
- Math equations (OMML) — Phase 4+

### 3.3 Writer Specification

**Input**: `&DocumentModel`
**Output**: `Result<Vec<u8>, DocxError>` (ZIP bytes)

The writer produces a valid .docx that opens in Microsoft Word 2016+, LibreOffice 7+, and Google Docs.

Priorities:
1. **Round-trip fidelity**: Read a DOCX, write it back → preserve as much as possible
2. **Valid output**: Always produce a valid OOXML document (pass Open XML SDK validation)
3. **Compatibility**: Target Word 2016+ and LibreOffice 7+

### 3.4 Rust Dependencies

- `zip` — ZIP archive read/write
- `quick-xml` — Fast XML parsing and writing
- `base64` — For embedded content

---

## 4. Format: ODT (`s1-format-odt`)

### 4.1 ODT Structure

ZIP archive with XML (ODF / ISO 26300):

```
META-INF/manifest.xml
content.xml            <- Main content
styles.xml             <- Styles
meta.xml               <- Metadata
settings.xml           <- Settings
Pictures/              <- Embedded images
```

### 4.2 Mapping

| ODF Element | Maps To |
|---|---|
| `text:p` | `Paragraph` |
| `text:span` | `Run` |
| `text:h` | `Paragraph` with heading style |
| Text content | `Text` |
| `table:table` | `Table` |
| `table:table-row` | `TableRow` |
| `table:table-cell` | `TableCell` |
| `text:list` / `text:list-item` | List structure |
| `style:style` | `Style` |
| `draw:frame` + `draw:image` | `Image` |
| `text:a` | Hyperlink attribute |

### 4.3 Rust Dependencies

Same as DOCX: `zip`, `quick-xml`

---

## 5. Format: PDF Export (`s1-format-pdf`)

### 5.1 Approach

PDF export works from the **layout tree**, not directly from the document model:

```
DocumentModel → s1-layout (Layout Engine) → LayoutDocument → s1-format-pdf → PDF bytes
```

### 5.2 PDF Features by Phase

| Feature | Phase | Priority |
|---|---|---|
| Text rendering with correct glyph positioning | 3 | Must |
| Font embedding with subsetting | 3 | Must |
| Images (JPEG, PNG) | 3 | Must |
| Tables (borders, backgrounds) | 3 | Must |
| Page numbers | 3 | Should |
| Headers/Footers | 3 | Should |
| Hyperlinks (PDF annotations) | 3 | Should |
| Bookmarks / document outline | 3 | Nice |
| PDF metadata (title, author) | 3 | Nice |
| PDF/A compliance | 5 | Nice |

### 5.3 Rust Dependencies

- `pdf-writer` — Low-level PDF generation (pure Rust, proven by Typst)
- `subsetter` — Font subsetting (embed only used glyphs)
- `image` — Image decoding

---

## 6. Format: TXT (`s1-format-txt`)

### 6.1 Reader

- Detect encoding via BOM: UTF-8 BOM, UTF-16 LE/BE BOM
- If no BOM: attempt UTF-8, fall back to Latin-1
- Each line → `Paragraph` with single `Run` containing single `Text`
- Empty lines → empty `Paragraph` (no children)
- No formatting applied (all defaults)

### 6.2 Writer

- Serialize all `Text` nodes in document order
- Paragraphs separated by newline (configurable: `\n` or `\r\n`)
- Tables: columns separated by tab, rows by newline
- Strip all formatting
- Output encoding: UTF-8 (always)

---

## 7. Format Conversion (`s1-convert`)

### 7.1 DOC → DOCX Conversion

Legacy `.doc` files use Microsoft's OLE2 binary format.

**Option A: External Tool (Recommended for Phase 1-2)**
- Shell out to LibreOffice headless: `soffice --convert-to docx input.doc`
- Pros: Excellent coverage, well-tested
- Cons: Requires LibreOffice installed

**Option B: Native OLE2 Reader (Phase 3+)**
- Parse OLE2 container using `cfb` crate
- Parse Word binary format records
- Convert to `DocumentModel`
- Pros: No external dependency
- Cons: Very complex, months of work

**Decision**: Start with Option A, implement Option B later if needed.

### 7.2 General Conversion Pipeline

```rust
pub fn convert(
    input: &[u8],
    from: Format,
    to: Format,
    options: ConvertOptions,
) -> Result<Vec<u8>, ConvertError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Docx,
    Odt,
    Pdf,
    Txt,
    Doc,  // Input only — converted via pipeline
}

#[derive(Debug, Clone, Default)]
pub struct ConvertOptions {
    /// For PDF: page size override
    pub page_size: Option<(f64, f64)>,
    /// For TXT: line ending style
    pub line_ending: Option<LineEnding>,
}
```

Pipeline: `Source Format → DocumentModel → Target Format`

---

## 8. Layout Engine (`s1-layout`)

### 8.1 Layout Process

```
1. Resolve styles (compute effective attributes for every node)
2. Shape text (HarfBuzz: characters → positioned glyphs)
3. Break lines (Knuth-Plass algorithm preferred, greedy fallback)
4. Layout blocks (stack paragraphs with spacing, handle tables)
5. Paginate (break into pages, respect widows/orphans/keep-together)
6. Position headers/footers (substitute page numbers)
7. Output: LayoutDocument (pages → blocks → lines → glyph runs)
```

### 8.2 Layout Tree

```rust
pub struct LayoutDocument {
    pub pages: Vec<LayoutPage>,
}

pub struct LayoutPage {
    pub index: usize,          // 0-based page number
    pub width: f64,            // in points
    pub height: f64,
    pub content_area: Rect,    // Margins applied
    pub blocks: Vec<LayoutBlock>,
    pub header: Option<LayoutBlock>,
    pub footer: Option<LayoutBlock>,
}

pub struct LayoutBlock {
    pub source_id: NodeId,     // Link back to document model
    pub bounds: Rect,
    pub kind: LayoutBlockKind,
}

pub enum LayoutBlockKind {
    Paragraph { lines: Vec<LayoutLine> },
    Table { rows: Vec<LayoutTableRow> },
    Image { media_id: MediaId, bounds: Rect },
}

pub struct LayoutLine {
    pub baseline_y: f64,
    pub height: f64,
    pub runs: Vec<GlyphRun>,
}

pub struct GlyphRun {
    pub source_id: NodeId,     // Link to Run node
    pub font_id: FontId,
    pub font_size: f64,
    pub color: Color,
    pub glyphs: Vec<ShapedGlyph>,
}

pub struct ShapedGlyph {
    pub glyph_id: u16,        // Font glyph index
    pub x_advance: f64,
    pub y_advance: f64,
    pub x_offset: f64,
    pub y_offset: f64,
    pub cluster: u32,          // Maps back to source character index
}

pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

### 8.3 Incremental Layout

When a document is edited, only re-layout what changed:
1. Mark the modified paragraph as dirty
2. Re-layout that paragraph (re-shape, re-break lines)
3. If paragraph height changed → re-layout the page (re-stack blocks)
4. If page break changed → re-paginate from that page forward
5. Subsequent pages only re-paginate (blocks shift), no re-shaping

This is critical for editor performance — target < 5ms for single-edit re-layout.

---

## 9. Text Processing (`s1-text`)

### 9.1 HarfBuzz Integration

```rust
pub fn shape_text(
    text: &str,
    font: &Font,
    font_size: f64,
    features: &[FontFeature],       // OpenType features (e.g., liga, kern)
    language: Option<&str>,          // BCP 47
    direction: Direction,            // LTR or RTL
) -> Vec<ShapedGlyph>;

pub enum Direction { Ltr, Rtl }

pub struct FontFeature {
    pub tag: [u8; 4],    // e.g., b"liga", b"kern"
    pub value: u32,       // 0 = off, 1 = on
}
```

### 9.2 FreeType Integration

```rust
pub fn load_font(path: &Path) -> Result<Font, FontError>;
pub fn load_font_from_memory(data: &[u8]) -> Result<Font, FontError>;

pub struct Font { /* opaque */ }

impl Font {
    pub fn family_name(&self) -> &str;
    pub fn style_name(&self) -> &str;
    pub fn is_bold(&self) -> bool;
    pub fn is_italic(&self) -> bool;
    pub fn metrics(&self, size: f64) -> FontMetrics;
    pub fn glyph_index(&self, ch: char) -> Option<u16>;
    pub fn has_glyph(&self, ch: char) -> bool;
}

pub struct FontMetrics {
    pub ascent: f64,       // Distance from baseline to top
    pub descent: f64,      // Distance from baseline to bottom (negative)
    pub line_gap: f64,     // Extra spacing between lines
    pub units_per_em: u16,
}
```

### 9.3 Font Discovery

```rust
pub struct FontDatabase { /* wraps fontdb */ }

impl FontDatabase {
    pub fn new() -> Self;                   // Loads system fonts
    pub fn with_fonts_dir(path: &Path) -> Self;
    pub fn find(&self, family: &str, bold: bool, italic: bool) -> Option<Font>;
    pub fn fallback(&self, ch: char) -> Option<Font>;  // Find font that has this glyph
}
```

### 9.4 Unicode Processing

```rust
/// Determine text direction for BiDi text (Unicode UAX #9).
pub fn bidi_resolve(text: &str) -> Vec<BidiRun>;

pub struct BidiRun {
    pub start: usize,      // Byte offset in text
    pub end: usize,
    pub direction: Direction,
    pub level: u8,          // BiDi embedding level
}

/// Find valid line break opportunities (Unicode UAX #14).
pub fn line_break_opportunities(text: &str) -> Vec<BreakOpportunity>;

pub struct BreakOpportunity {
    pub offset: usize,     // Byte offset where break is allowed
    pub mandatory: bool,   // true for hard breaks (e.g., \n)
}
```

---

## 10. Engine Facade (`s1engine`)

### 10.1 Engine

`Engine` is a stateless, zero-cost factory. It holds no configuration and no
internal state. It can be freely shared across threads.

```rust
/// The main entry point for s1engine.
pub struct Engine;  // unit struct, no fields

impl Engine {
    pub fn new() -> Self;

    /// Create a new empty document.
    pub fn create(&self) -> Document;

    /// Open a document from bytes (format auto-detected from magic bytes).
    pub fn open(&self, data: &[u8]) -> Result<Document, Error>;

    /// Open a document from bytes with an explicit format.
    pub fn open_as(&self, data: &[u8], format: Format) -> Result<Document, Error>;

    /// Open a document from a file path (format detected from extension).
    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<Document, Error>;

    /// Create a new collaborative document (feature-gated: `crdt`).
    #[cfg(feature = "crdt")]
    pub fn create_collab(&self, replica_id: u64) -> CollabDocument;

    /// Open a document as a collaborative document (feature-gated: `crdt`).
    #[cfg(feature = "crdt")]
    pub fn open_collab(&self, data: &[u8], replica_id: u64) -> Result<CollabDocument, Error>;
}

impl Default for Engine { fn default() -> Self { Self::new() } }
```

### 10.2 Document

`Document` wraps `DocumentModel` with an undo/redo `History` and provides
the high-level API for querying, editing, and exporting.

```rust
pub struct Document {
    model: DocumentModel,
    history: History,
}

impl Document {
    // --- Construction ---
    pub fn new() -> Self;
    pub fn from_model(model: DocumentModel) -> Self;

    // --- Model access ---
    pub fn model(&self) -> &DocumentModel;
    pub fn model_mut(&mut self) -> &mut DocumentModel;  // escape hatch (bypasses undo)
    pub fn into_model(self) -> DocumentModel;

    // --- Metadata ---
    pub fn metadata(&self) -> &DocumentMetadata;
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata;

    // --- Content queries ---
    pub fn to_plain_text(&self) -> String;
    pub fn body_id(&self) -> Option<NodeId>;
    pub fn node(&self, id: NodeId) -> Option<&Node>;
    pub fn next_id(&mut self) -> NodeId;
    pub fn paragraph_ids(&self) -> Vec<NodeId>;   // top-level body paragraphs only
    pub fn paragraph_count(&self) -> usize;

    // --- Styles ---
    pub fn styles(&self) -> &[Style];
    pub fn style_by_id(&self, id: &str) -> Option<&Style>;
    pub fn numbering(&self) -> &NumberingDefinitions;
    pub fn sections(&self) -> &[SectionProperties];

    // --- Transactions ---
    pub fn begin_transaction(label: &str) -> TransactionBuilder;
    pub fn apply_transaction(&mut self, txn: &Transaction) -> Result<(), Error>;
    pub fn apply(&mut self, op: Operation) -> Result<(), Error>;

    // --- Undo / Redo ---
    pub fn undo(&mut self) -> Result<bool, Error>;
    pub fn redo(&mut self) -> Result<bool, Error>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn clear_history(&mut self);

    // --- Export ---
    pub fn export(&self, format: Format) -> Result<Vec<u8>, Error>;
    pub fn export_string(&self, format: Format) -> Result<String, Error>;
}

impl Default for Document { fn default() -> Self { Self::new() } }
```

Notes:

- `undo()` and `redo()` return `Result<bool, Error>`, where `true` means an
  operation was undone/redone and `false` means the stack was empty.
- `model_mut()` is an explicit escape hatch. Direct mutation bypasses
  undo/redo history and CRDT operation generation.
- `begin_transaction` is an associated function (not `&self`). It returns a
  `TransactionBuilder` from `s1-ops`.
- `export_string` is a convenience for text-oriented formats. For TXT it
  calls the writer directly; for other formats it exports bytes and
  attempts UTF-8 conversion.

### 10.3 Transaction API

```rust
// s1-ops re-exports used by s1engine:
pub struct Transaction { /* operations + label */ }
pub struct TransactionBuilder { /* builds a Transaction */ }

impl Transaction {
    pub fn new() -> Self;                           // empty, unlabeled
    pub fn with_label(label: &str) -> Self;         // empty, labeled
    pub fn push(&mut self, op: Operation);
}

impl TransactionBuilder {
    pub fn new() -> Self;
    pub fn label(self, label: &str) -> Self;
}
```

### 10.4 Format Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    Docx,
    Odt,
    Pdf,   // export only
    Txt,
}

impl Format {
    /// Detect from a file extension (case-insensitive). Errors on unknown.
    pub fn from_extension(ext: &OsStr) -> Result<Self, Error>;

    /// Detect from a file path's extension.
    pub fn from_path(path: &Path) -> Result<Self, Error>;

    /// Heuristic detection from leading bytes:
    /// ZIP magic (PK\x03\x04) -> Docx or Odt, %PDF -> Pdf, else Txt.
    pub fn detect(data: &[u8]) -> Self;

    /// File extension without dot ("docx", "odt", "pdf", "txt").
    pub fn extension(&self) -> &'static str;

    /// MIME type string.
    pub fn mime_type(&self) -> &'static str;
}
```

### 10.5 Error Enum

```rust
#[derive(Debug)]
pub enum Error {
    /// Error from a format reader/writer (DOCX, ODT, TXT).
    Format(String),
    /// Error from an operation (insert, delete, etc.).
    Operation(OperationError),
    /// I/O error (file read/write).
    Io(std::io::Error),
    /// The requested format is not supported or not enabled.
    UnsupportedFormat(String),
    /// Error from the CRDT subsystem (feature-gated: `crdt`).
    #[cfg(feature = "crdt")]
    Crdt(CrdtError),
}

impl std::fmt::Display for Error { /* ... */ }
impl std::error::Error for Error { /* ... */ }

// From conversions:
impl From<std::io::Error> for Error;
impl From<OperationError> for Error;
impl From<DocxError> for Error;       // feature: docx
impl From<OdtError> for Error;        // feature: odt
impl From<TxtError> for Error;        // feature: txt
impl From<CrdtError> for Error;       // feature: crdt
```

### 10.6 Builder API

`DocumentBuilder` is a standalone builder that constructs a `Document`
without requiring an `Engine`. It directly manipulates a `DocumentModel`
and produces a `Document` via `build()`.

```rust
pub struct DocumentBuilder { model: DocumentModel }

impl DocumentBuilder {
    pub fn new() -> Self;

    // Block-level content
    pub fn heading(self, level: u8, text: &str) -> Self;
    pub fn paragraph(self, f: impl FnOnce(ParagraphBuilder) -> ParagraphBuilder) -> Self;
    pub fn text(self, text: &str) -> Self;           // shorthand for single-run paragraph
    pub fn bullet(self, text: &str) -> Self;         // bulleted list item
    pub fn numbered(self, text: &str) -> Self;       // numbered list item
    pub fn list_item(self, text: &str, level: u8, format: ListFormat, num_id: u32) -> Self;
    pub fn table(self, f: impl FnOnce(TableBuilder) -> TableBuilder) -> Self;

    // Metadata
    pub fn title(self, title: &str) -> Self;
    pub fn author(self, author: &str) -> Self;

    // Sections
    pub fn section(self, props: SectionProperties) -> Self;
    pub fn section_with_header(self, header_text: &str) -> Self;
    pub fn section_with_footer(self, footer_text: &str) -> Self;
    pub fn section_with_header_footer(self, header_text: &str, footer_text: &str) -> Self;

    pub fn build(self) -> Document;
}
```

```rust
pub struct ParagraphBuilder<'a> { model: &'a mut DocumentModel, para_id: NodeId }

impl<'a> ParagraphBuilder<'a> {
    pub fn text(self, text: &str) -> Self;
    pub fn bold(self, text: &str) -> Self;
    pub fn italic(self, text: &str) -> Self;
    pub fn bold_italic(self, text: &str) -> Self;
    pub fn underline(self, text: &str) -> Self;
    pub fn styled(self, text: &str, font: &str, size: f64) -> Self;
    pub fn colored(self, text: &str, color: Color) -> Self;
    pub fn line_break(self) -> Self;
    pub fn superscript(self, text: &str) -> Self;
    pub fn subscript(self, text: &str) -> Self;
    pub fn hyperlink(self, url: &str, text: &str) -> Self;
    pub fn bookmark_start(self, name: &str) -> Self;
    pub fn bookmark_end(self) -> Self;
}
```

```rust
pub struct TableBuilder<'a> { model: &'a mut DocumentModel, table_id: NodeId }

impl<'a> TableBuilder<'a> {
    pub fn row(self, f: impl FnOnce(RowBuilder) -> RowBuilder) -> Self;
    pub fn width(self, width: TableWidth) -> Self;
}

pub struct RowBuilder<'a> { model: &'a mut DocumentModel, row_id: NodeId }

impl<'a> RowBuilder<'a> {
    pub fn cell(self, text: &str) -> Self;
    pub fn rich_cell(self, f: impl FnOnce(ParagraphBuilder) -> ParagraphBuilder) -> Self;
}
```

Example usage:

```rust
use s1engine::{DocumentBuilder, Format};

let doc = DocumentBuilder::new()
    .title("Quarterly Report")
    .author("Engineering Team")
    .heading(1, "Introduction")
    .paragraph(|p| {
        p.text("This is ")
         .bold("bold")
         .text(" and ")
         .italic("italic")
         .text(".")
    })
    .table(|t| {
        t.row(|r| r.cell("Name").cell("Age"))
         .row(|r| r.cell("Alice").cell("30"))
    })
    .bullet("First item")
    .bullet("Second item")
    .build();

let bytes = doc.export(Format::Docx)?;
```

### 10.7 Re-exports

The `s1engine` crate re-exports key types so consumers do not need to
depend on internal crates directly:

```rust
// Model types
pub use s1_model::{
    Alignment, AttributeKey, AttributeMap, AttributeValue, Color, DocumentMetadata,
    DocumentModel, FieldType, HeaderFooterRef, HeaderFooterType, LineSpacing,
    ListFormat, ListInfo, Node, NodeId, NodeType, PageOrientation, SectionBreakType,
    SectionProperties, Style, StyleType, UnderlineStyle,
};

// Operation types
pub use s1_ops::{
    History, Operation, OperationError, Position, Selection, Transaction, TransactionBuilder,
};

// Full crate access for advanced use
pub use s1_model as model;
pub use s1_ops as ops;

// CRDT types (feature-gated)
#[cfg(feature = "crdt")]
pub use s1_crdt as crdt;
#[cfg(feature = "crdt")]
pub use s1_crdt::{CollabDocument, CrdtError, CrdtOperation, OpId, StateVector};
```

---

## 11. FFI Bindings

### 11.1 C API (`ffi/c/src/lib.rs`)

The C API uses opaque handle types and `extern "C"` functions. All
functions are null-safe: passing null returns null, zero, or sets an
error rather than crashing. All returned handles must be freed by the
caller using the corresponding `s1_*_free` function.

**Opaque handle types:** `S1Engine`, `S1Document`, `S1Error`, `S1Bytes`, `S1String`

```c
// --- Engine lifecycle ---
S1Engine* s1_engine_new(void);
void s1_engine_free(S1Engine* engine);

// --- Document creation / opening ---
S1Document* s1_engine_create(const S1Engine* engine);
S1Document* s1_engine_open(
    const S1Engine* engine,
    const uint8_t* data,
    size_t len,
    S1Error** error_out
);

// --- Document operations ---
void s1_document_free(S1Document* doc);
S1String* s1_document_plain_text(const S1Document* doc);
S1Bytes* s1_document_export(
    const S1Document* doc,
    const char* format,       // "docx", "odt", "txt", "pdf"
    S1Error** error_out
);
S1String* s1_document_metadata_title(const S1Document* doc);
size_t s1_document_paragraph_count(const S1Document* doc);

// --- Error handling ---
const char* s1_error_message(const S1Error* error);
void s1_error_free(S1Error* error);

// --- String handling ---
const char* s1_string_ptr(const S1String* s);
void s1_string_free(S1String* s);

// --- Byte buffer handling ---
const uint8_t* s1_bytes_data(const S1Bytes* b);
size_t s1_bytes_len(const S1Bytes* b);
void s1_bytes_free(S1Bytes* b);
```

Error convention: functions that can fail accept an `S1Error** error_out`
parameter. On failure the function returns null and sets `*error_out` to
a non-null error handle. The caller must free the error with
`s1_error_free`.

Example (C):

```c
S1Engine* engine = s1_engine_new();
S1Error* err = NULL;
S1Document* doc = s1_engine_open(engine, data, len, &err);
if (!doc) {
    fprintf(stderr, "open failed: %s\n", s1_error_message(err));
    s1_error_free(err);
} else {
    S1String* text = s1_document_plain_text(doc);
    printf("%s\n", s1_string_ptr(text));
    s1_string_free(text);
    s1_document_free(doc);
}
s1_engine_free(engine);
```

### 11.2 WASM API (`ffi/wasm/src/lib.rs`)

The WASM API exposes `wasm-bindgen` classes callable from JavaScript or
TypeScript. Format strings are passed as `"docx"`, `"odt"`, `"txt"`, or
`"pdf"`. Methods that can fail throw a JavaScript error.

```typescript
// --- Engine ---
class WasmEngine {
    constructor();
    create(): WasmDocument;
    open(data: Uint8Array): WasmDocument;             // throws on error
    open_as(data: Uint8Array, format: string): WasmDocument;  // throws on error
}

// --- Document ---
class WasmDocument {
    to_plain_text(): string;                           // throws if freed
    export(format: string): Uint8Array;                // throws on error
    metadata_title(): string | undefined;
    metadata_author(): string | undefined;
    paragraph_count(): number;
    free(): void;                                      // releases memory
    is_valid(): boolean;
}

// --- Document Builder ---
class WasmDocumentBuilder {
    constructor();
    heading(level: number, text: string): WasmDocumentBuilder;
    text(text: string): WasmDocumentBuilder;
    title(title: string): WasmDocumentBuilder;
    author(author: string): WasmDocumentBuilder;
    build(): WasmDocument;                             // throws if already consumed
}

// --- Font Database ---
class WasmFontDatabase {
    constructor();                                     // empty database
    load_font(data: Uint8Array): void;
    font_count(): number;
}

// --- Free function ---
function detect_format(data: Uint8Array): string;      // returns "docx"|"odt"|"pdf"|"txt"
```

Example (JavaScript):

```javascript
import { WasmEngine, WasmDocumentBuilder, detect_format } from "s1engine-wasm";

const engine = new WasmEngine();
const doc = engine.open(docxBytes);
console.log(doc.to_plain_text());

const exported = doc.export("txt");
doc.free();

const doc2 = new WasmDocumentBuilder()
    .title("Report")
    .heading(1, "Summary")
    .text("Contents here.")
    .build();
const bytes = doc2.export("docx");
doc2.free();
```

---

## 12. Testing Strategy

### 12.1 Unit Tests

Every crate contains `#[cfg(test)] mod tests` blocks in each source file.
Coverage targets:

- Every public function has at least one test.
- Format crates (`s1-format-docx`, `s1-format-odt`, `s1-format-txt`) include
  round-trip tests: read input, write output, re-read, compare.
- `s1-crdt` uses `proptest` for convergence property-based testing.

### 12.2 Integration Tests

Integration tests live inside crate test directories, not in a top-level
`tests/` folder.

| Location | Contents |
|---|---|
| `crates/s1engine/tests/invariants.rs` | Cross-crate invariant checks (model consistency after operations, undo/redo symmetry) |
| `crates/s1engine/tests/hostile_inputs.rs` | Fuzz-style hostile input tests for all format readers (malformed ZIP, truncated XML, zero-length input, binary noise). Runs on stable Rust without `cargo-fuzz`. |
| `crates/s1-crdt/tests/convergence.rs` | 16 multi-replica convergence tests (2/3/5 replicas, partition/heal, snapshot sync, delayed delivery) |
| `crates/s1-crdt/tests/scenarios.rs` | 17 scenario tests (concurrent inserts, attribute LWW, delete+modify, undo locality) |
| `crates/s1-crdt/tests/proptests.rs` | Property-based CRDT convergence tests via `proptest` |

### 12.3 Benchmarks

Criterion benchmarks are located at `crates/s1engine/benches/engine_bench.rs`.
They cover document open, export, and round-trip timing for DOCX, ODT, and TXT.

### 12.4 FFI Tests

Both FFI crates include unit tests that exercise the full lifecycle through
the foreign interface:

- `ffi/c/src/lib.rs` -- tests for engine create/free, document open/export,
  null-pointer safety, error handling, metadata access, and format round-trips.
- `ffi/wasm/src/lib.rs` -- tests for WasmEngine, WasmDocument, WasmDocumentBuilder,
  WasmFontDatabase, format detection, export, and builder round-trips.

---

## 13. Performance Targets

| Operation | Target | How Measured |
|---|---|---|
| Open 10-page DOCX | < 50ms | `criterion` benchmark |
| Open 100-page DOCX | < 500ms | `criterion` benchmark |
| Export 10-page PDF | < 200ms | `criterion` benchmark |
| Export 100-page PDF | < 2s | `criterion` benchmark |
| Single paragraph edit + re-layout | < 5ms | `criterion` benchmark |
| Full document layout (10 pages) | < 100ms | `criterion` benchmark |
| WASM bundle size | < 2MB gzipped | CI check |
| Memory: 10-page doc loaded | < 10MB | Integration test |
| Memory: 100-page doc loaded | < 50MB | Integration test |
