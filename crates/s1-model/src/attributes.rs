//! Node attributes and formatting properties.
//!
//! Attributes use a typed key-value system to avoid stringly-typed errors.
//! The [`AttributeMap`] stores key-value pairs where keys are [`AttributeKey`]
//! variants and values are [`AttributeValue`] variants.

use std::collections::HashMap;

// ─── Supporting Types ───────────────────────────────────────────────────────

/// An RGBA color.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const RED: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse a hex color string like "FF0000" or "#FF0000".
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self::new(r, g, b))
    }

    /// Convert to hex string like "FF0000".
    pub fn to_hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Text alignment.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

/// Underline style.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnderlineStyle {
    None,
    Single,
    Double,
    Thick,
    Dotted,
    Dashed,
    Wave,
}

/// Line spacing configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum LineSpacing {
    Single,
    OnePointFive,
    Double,
    /// Exact spacing in points.
    Exact(f64),
    /// Minimum spacing in points.
    AtLeast(f64),
    /// Multiple of default line height.
    Multiple(f64),
}

/// Page orientation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

/// Table or cell width specification.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TableWidth {
    Auto,
    /// Fixed width in points.
    Fixed(f64),
    /// Percentage of available width (0.0-100.0).
    Percent(f64),
}

/// Vertical alignment within a cell.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// A tab stop definition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabStop {
    /// Position in points from the left margin.
    pub position: f64,
    /// Alignment at the tab stop.
    pub alignment: TabAlignment,
    /// Leader character between tab stops.
    pub leader: TabLeader,
}

/// Tab stop alignment.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabAlignment {
    Left,
    Center,
    Right,
    Decimal,
}

/// Tab leader character.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabLeader {
    None,
    Dot,
    Dash,
    Underscore,
}

/// List/numbering information for a paragraph.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ListInfo {
    /// Nesting depth (0-8).
    pub level: u8,
    /// Numbering format.
    pub num_format: ListFormat,
    /// References a numbering definition.
    pub num_id: u32,
    /// Override start number.
    pub start: Option<u32>,
}

/// List numbering format.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListFormat {
    Bullet,
    /// 1, 2, 3, ...
    Decimal,
    /// a, b, c, ...
    LowerAlpha,
    /// A, B, C, ...
    UpperAlpha,
    /// i, ii, iii, ...
    LowerRoman,
    /// I, II, III, ...
    UpperRoman,
    /// No visible marker.
    None,
    /// 01, 02, 03, ... (zero-padded)
    DecimalZero,
}

/// Table layout algorithm.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableLayoutMode {
    /// Column widths determined by content.
    AutoFit,
    /// Column widths are fixed.
    Fixed,
}

/// Text capitalization transform.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextTransform {
    /// No transform.
    None,
    /// UPPERCASE.
    Uppercase,
    /// lowercase.
    Lowercase,
    /// Capitalize First Letter Of Each Word.
    Capitalize,
    /// sMALL cAPS (display only).
    SmallCaps,
    /// ALL CAPS (display only).
    AllCaps,
}

/// Box margins (top, bottom, left, right) in points.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

impl Margins {
    pub const ZERO: Margins = Margins {
        top: 0.0,
        bottom: 0.0,
        left: 0.0,
        right: 0.0,
    };

    pub const fn new(top: f64, bottom: f64, left: f64, right: f64) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }

    pub fn uniform(val: f64) -> Self {
        Self {
            top: val,
            bottom: val,
            left: val,
            right: val,
        }
    }
}

/// Writing mode / text direction.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WritingMode {
    /// Left-to-right, top-to-bottom (default Western).
    LrTb,
    /// Right-to-left, top-to-bottom (Arabic, Hebrew).
    RlTb,
    /// Top-to-bottom, right-to-left (CJK vertical).
    TbRl,
    /// Top-to-bottom, left-to-right (Mongolian).
    TbLr,
    /// Bottom-to-top, left-to-right (rare, used in some rotated contexts).
    BtLr,
}

/// Border configuration for a box (paragraph, cell, table).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Borders {
    pub top: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
    pub right: Option<BorderSide>,
}

/// A single border side.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct BorderSide {
    pub style: BorderStyle,
    pub width: f64,
    pub color: Color,
    pub spacing: f64,
}

/// Border line style.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BorderStyle {
    None,
    Single,
    Double,
    Dashed,
    Dotted,
    Thick,
}

/// Unique identifier for embedded media.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaId(pub u64);

/// Type of dynamic field.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldType {
    PageNumber,
    PageCount,
    Date,
    Time,
    FileName,
    Author,
    TableOfContents,
    /// HYPERLINK field — URL stored in FieldCode.
    Hyperlink,
    /// REF or PAGEREF cross-reference field.
    CrossReference,
    /// SEQ field — sequential numbering (figures, tables).
    Sequence,
    /// MERGEFIELD — mail merge field.
    MergeField,
    /// IF conditional field.
    Conditional,
    /// STYLEREF field.
    StyleRef,
    /// Any unrecognized field type. Instruction stored in FieldCode.
    Custom,
}

// ─── Attribute Key/Value System ─────────────────────────────────────────────

/// Typed attribute keys.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AttributeKey {
    // Run attributes
    FontFamily,
    /// East Asian font family (w:rFonts/@eastAsia).
    FontFamilyEastAsia,
    /// Complex-script font family (w:rFonts/@cs).
    FontFamilyCS,
    FontSize,
    /// Complex-script font size in points (w:szCs).
    FontSizeCS,
    Bold,
    /// Complex-script bold (w:bCs).
    BoldCS,
    Italic,
    /// Complex-script italic (w:iCs).
    ItalicCS,
    Underline,
    Strikethrough,
    /// Double strikethrough (w:dstrike).
    DoubleStrikethrough,
    Color,
    HighlightColor,
    Superscript,
    Subscript,
    FontSpacing,
    /// Baseline shift in half-points (w:position).
    BaselineShift,
    Language,
    /// All caps display (w:caps).
    Caps,
    /// Small caps display (w:smallCaps).
    SmallCaps,
    /// Hidden text (w:vanish).
    Hidden,
    /// Text transform (ODF fo:text-transform).
    TextTransformStyle,
    /// Theme color reference (w:themeColor attribute on w:color).
    /// Stored alongside resolved Color for round-trip preservation.
    /// Values: "accent1"-"accent6", "dark1", "dark2", "light1", "light2", etc.
    ThemeColor,
    /// Theme tint/shade value (w:themeTint or w:themeShade, 0-255 hex string).
    ThemeTintShade,

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
    ParagraphBorders,
    Background,
    TabStops,
    StyleId,
    ListInfo,

    /// Paragraph writing mode / text direction (w:textDirection, style:writing-mode).
    ParagraphWritingMode,
    /// Widow control — prevent single line at top of page (w:widowControl).
    WidowControl,
    /// Outline level 0-9 (w:outlineLvl). Used for TOC generation.
    OutlineLevel,

    // Section attributes
    /// Index into DocumentModel.sections() for the section ending at this paragraph.
    SectionIndex,
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
    /// Page borders (w:pgBorders in sectPr).
    PageBorders,
    /// Document grid type (w:docGrid/@type): "default", "lines", "linesAndChars", "snapToChars".
    DocGridType,
    /// Document grid line pitch in points (w:docGrid/@linePitch).
    DocGridLinePitch,
    /// Line numbering configuration (w:lnNumType) — stored as "start,countBy,restart,distance".
    LineNumbering,

    // Table attributes
    TableWidth,
    TableAlignment,
    TableBorders,
    CellMargins,

    /// Table layout mode: fixed or autofit (w:tblLayout).
    TableLayout,
    /// Default cell margins for the table (w:tblCellMar) in points.
    TableDefaultCellMargins,
    /// Table indent from leading margin in points (w:tblInd).
    TableIndent,

    // Table row attributes
    /// Marks a table row as a header row that should repeat on continuation pages.
    TableHeaderRow,
    /// Explicit row height in points (w:trHeight).
    RowHeight,
    /// Row height rule: "atLeast" or "exact" (w:trHeight/@hRule).
    RowHeightRule,
    /// Row cannot split across pages (w:cantSplit).
    RowNoSplit,

    // Cell attributes
    CellWidth,
    VerticalAlign,
    CellBorders,
    CellBackground,
    ColSpan,
    RowSpan,

    /// Per-cell margins (top, bottom, left, right) in points (w:tcMar).
    CellPadding,
    /// Cell text direction / writing mode (w:textDirection).
    CellTextDirection,
    /// Cell no-wrap flag (w:noWrap).
    CellNoWrap,

    // Image attributes
    ImageMediaId,
    ImageWidth,
    ImageHeight,
    ImageAltText,
    /// "inline" (default) or "anchor" (floating)
    ImagePositionType,
    /// Text wrap style: "none", "square", "tight", "topAndBottom", "behind", "inFront"
    ImageWrapType,
    /// Horizontal offset in EMUs from anchor (for floating images)
    ImageHorizontalOffset,
    /// Vertical offset in EMUs from anchor (for floating images)
    ImageVerticalOffset,
    /// Horizontal relative-to: "column", "page", "margin", "character"
    ImageHorizontalRelativeFrom,
    /// Vertical relative-to: "paragraph", "page", "margin", "line"
    ImageVerticalRelativeFrom,
    /// Distance from text in EMUs (top, bottom, left, right as comma-separated string)
    ImageDistanceFromText,

    // Field attributes
    FieldType,
    FieldCode,

    // TOC attributes
    /// Maximum heading level included in the TOC (1-9, default 3).
    TocMaxLevel,
    /// Custom title for the TOC (e.g. "Table of Contents").
    TocTitle,
    /// Whether the TOC should use index marks (ODF: text:use-index-marks).
    TocUseIndexMarks,
    /// Whether the TOC should use index source styles (ODF: text:use-index-source-styles).
    TocUseIndexSourceStyles,
    /// Scope of the TOC index (ODF: text:index-scope, e.g. "document" or "chapter").
    TocIndexScope,

    // Additional paragraph attributes (round-trip preservation)
    /// Contextual spacing: suppress extra space between paragraphs of the same style.
    ContextualSpacing,
    /// Word wrap: controls whether East Asian text wraps at arbitrary positions.
    WordWrap,

    // Hyphenation attributes
    /// Suppress automatic hyphenation for this paragraph.
    SuppressAutoHyphens,

    // BiDi attribute
    /// Whether text direction is right-to-left.
    Bidi,

    // Equation attributes
    /// Source content for an equation (LaTeX string or raw OOXML).
    EquationSource,

    // Footnote/Endnote attributes
    /// Footnote number (auto-assigned).
    FootnoteNumber,
    /// Endnote number (auto-assigned).
    EndnoteNumber,

    // Link / annotation attributes
    HyperlinkUrl,
    HyperlinkTooltip,
    BookmarkName,
    CommentId,
    CommentAuthor,
    CommentDate,
    /// Parent comment ID for threaded replies.
    CommentParentId,
    /// Whether the comment is resolved (true/false).
    CommentResolved,

    // Revision / track changes attributes
    /// Revision type: "Insert", "Delete", or "FormatChange".
    RevisionType,
    /// Revision author name.
    RevisionAuthor,
    /// Revision date/time string (ISO 8601).
    RevisionDate,
    /// Revision ID (unique within document).
    RevisionId,
    /// The original formatting before a format change (stored as string representation).
    RevisionOriginalFormatting,

    // Shape / drawing attributes
    /// Shape type identifier (e.g., "rect", "roundRect", "ellipse", "line", "textBox").
    ShapeType,
    /// Shape width in points.
    ShapeWidth,
    /// Shape height in points.
    ShapeHeight,
    /// Shape fill color (hex string, no #).
    ShapeFillColor,
    /// Shape outline/stroke color (hex string, no #).
    ShapeStrokeColor,
    /// Shape outline width in points.
    ShapeStrokeWidth,
    /// Raw VML/DrawingML XML for preserving shapes that can't be fully modeled.
    ShapeRawXml,

    /// Comma-separated column widths in points for a table node.
    ///
    /// Stored as a string like `"72.0,108.0,72.0"` representing each column's
    /// width. Repeated columns (e.g., ODF `number-columns-repeated`) are
    /// expanded into individual values.
    TableColumnWidths,

    // Text effect attributes (round-trip preservation)
    /// Text shadow effect definition (CSS-like or raw XML string).
    TextShadow,
    /// Text outline effect definition (CSS-like or raw XML string).
    TextOutline,
    /// Text glow effect definition (CSS-like or raw XML string).
    TextGlow,
    /// Text reflection effect definition (CSS-like or raw XML string).
    TextReflection,

    // Form control attributes (DOCX SDT)
    /// Form control type: "checkbox", "dropdown", "text", or "date".
    FormType,
    /// Comma-separated list of options for dropdown form controls.
    FormOptions,
    /// Whether a checkbox form control is checked.
    FormChecked,
    /// SDT alias / display name (w:alias).
    FormAlias,
    /// SDT tag / custom property (w:tag).
    FormTag,

    // Change tracking metadata
    /// JSON string containing parsed change tracking info (regions with id,
    /// type, author, date). Used for ODT tracked-changes preservation.
    ChangeTrackingInfo,

    // Raw XML preservation
    /// Raw XML content from elements not fully modeled (e.g., SmartArt, charts,
    /// form controls, content controls). Stored for round-trip fidelity.
    RawXml,
}

/// Typed attribute values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
    TableLayoutMode(TableLayoutMode),
    TextTransform(TextTransform),
    Margins(Margins),
    WritingMode(WritingMode),
}

// ─── AttributeMap ───────────────────────────────────────────────────────────

/// A map of typed attributes. Used on every node for formatting properties.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttributeMap {
    inner: HashMap<AttributeKey, AttributeValue>,
}

impl AttributeMap {
    /// Create an empty attribute map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an attribute.
    pub fn set(&mut self, key: AttributeKey, value: AttributeValue) {
        self.inner.insert(key, value);
    }

    /// Get an attribute value.
    pub fn get(&self, key: &AttributeKey) -> Option<&AttributeValue> {
        self.inner.get(key)
    }

    /// Remove an attribute, returning the old value if present.
    pub fn remove(&mut self, key: &AttributeKey) -> Option<AttributeValue> {
        self.inner.remove(key)
    }

    /// Check if an attribute is set.
    pub fn contains(&self, key: &AttributeKey) -> bool {
        self.inner.contains_key(key)
    }

    /// Returns the number of attributes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if no attributes are set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&AttributeKey, &AttributeValue)> {
        self.inner.iter()
    }

    /// Get all keys.
    pub fn keys(&self) -> impl Iterator<Item = &AttributeKey> {
        self.inner.keys()
    }

    /// Merge another attribute map into this one.
    /// Values from `other` override values in `self` on key conflict.
    pub fn merge(&mut self, other: &AttributeMap) {
        for (key, value) in &other.inner {
            self.inner.insert(key.clone(), value.clone());
        }
    }

    /// Create a new map that is this map merged with `other` (non-destructive).
    pub fn merged_with(&self, other: &AttributeMap) -> AttributeMap {
        let mut result = self.clone();
        result.merge(other);
        result
    }

    // ─── Typed convenience getters ──────────────────────────────────────

    pub fn get_bool(&self, key: &AttributeKey) -> Option<bool> {
        match self.get(key) {
            Some(AttributeValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_f64(&self, key: &AttributeKey) -> Option<f64> {
        match self.get(key) {
            Some(AttributeValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_i64(&self, key: &AttributeKey) -> Option<i64> {
        match self.get(key) {
            Some(AttributeValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string(&self, key: &AttributeKey) -> Option<&str> {
        match self.get(key) {
            Some(AttributeValue::String(v)) => Some(v),
            _ => None,
        }
    }

    pub fn get_color(&self, key: &AttributeKey) -> Option<Color> {
        match self.get(key) {
            Some(AttributeValue::Color(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn get_alignment(&self, key: &AttributeKey) -> Option<Alignment> {
        match self.get(key) {
            Some(AttributeValue::Alignment(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get an underline style value from the map.
    pub fn get_underline_style(&self, key: &AttributeKey) -> Option<UnderlineStyle> {
        match self.get(key) {
            Some(AttributeValue::UnderlineStyle(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a line spacing value from the map.
    pub fn get_line_spacing(&self, key: &AttributeKey) -> Option<LineSpacing> {
        match self.get(key) {
            Some(AttributeValue::LineSpacing(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a borders value from the map.
    pub fn get_borders(&self, key: &AttributeKey) -> Option<&Borders> {
        match self.get(key) {
            Some(AttributeValue::Borders(v)) => Some(v),
            _ => None,
        }
    }

    /// Get tab stops from the map.
    pub fn get_tab_stops(&self, key: &AttributeKey) -> Option<&Vec<TabStop>> {
        match self.get(key) {
            Some(AttributeValue::TabStops(v)) => Some(v),
            _ => None,
        }
    }

    /// Get list info from the map.
    pub fn get_list_info(&self, key: &AttributeKey) -> Option<&ListInfo> {
        match self.get(key) {
            Some(AttributeValue::ListInfo(v)) => Some(v),
            _ => None,
        }
    }

    /// Get a media ID from the map.
    pub fn get_media_id(&self, key: &AttributeKey) -> Option<MediaId> {
        match self.get(key) {
            Some(AttributeValue::MediaId(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a field type from the map.
    pub fn get_field_type(&self, key: &AttributeKey) -> Option<FieldType> {
        match self.get(key) {
            Some(AttributeValue::FieldType(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a table layout mode from the map.
    pub fn get_table_layout(&self, key: &AttributeKey) -> Option<TableLayoutMode> {
        match self.get(key) {
            Some(AttributeValue::TableLayoutMode(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get margins from the map.
    pub fn get_margins(&self, key: &AttributeKey) -> Option<&Margins> {
        match self.get(key) {
            Some(AttributeValue::Margins(v)) => Some(v),
            _ => None,
        }
    }
}

// ─── Convenience setters (builder-style) ────────────────────────────────────

impl AttributeMap {
    /// Set bold.
    pub fn bold(mut self, v: bool) -> Self {
        self.set(AttributeKey::Bold, AttributeValue::Bool(v));
        self
    }

    /// Set italic.
    pub fn italic(mut self, v: bool) -> Self {
        self.set(AttributeKey::Italic, AttributeValue::Bool(v));
        self
    }

    /// Set font size in points.
    pub fn font_size(mut self, pts: f64) -> Self {
        self.set(AttributeKey::FontSize, AttributeValue::Float(pts));
        self
    }

    /// Set font family.
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.set(
            AttributeKey::FontFamily,
            AttributeValue::String(family.into()),
        );
        self
    }

    /// Set text color.
    pub fn color(mut self, c: Color) -> Self {
        self.set(AttributeKey::Color, AttributeValue::Color(c));
        self
    }

    /// Set text alignment.
    pub fn alignment(mut self, a: Alignment) -> Self {
        self.set(AttributeKey::Alignment, AttributeValue::Alignment(a));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_hex() {
        let c = Color::from_hex("FF0000").unwrap();
        assert_eq!(c, Color::RED);

        let c = Color::from_hex("#00FF00").unwrap();
        assert_eq!(c, Color::new(0, 255, 0));

        assert!(Color::from_hex("ZZZZZZ").is_none());
        assert!(Color::from_hex("FF").is_none());
    }

    #[test]
    fn color_to_hex() {
        assert_eq!(Color::RED.to_hex(), "FF0000");
        assert_eq!(Color::BLACK.to_hex(), "000000");
        assert_eq!(Color::WHITE.to_hex(), "FFFFFF");
    }

    #[test]
    fn attribute_map_basic() {
        let mut map = AttributeMap::new();
        assert!(map.is_empty());

        map.set(AttributeKey::Bold, AttributeValue::Bool(true));
        assert_eq!(map.len(), 1);
        assert!(map.contains(&AttributeKey::Bold));
        assert_eq!(map.get_bool(&AttributeKey::Bold), Some(true));
    }

    #[test]
    fn attribute_map_overwrite() {
        let mut map = AttributeMap::new();
        map.set(AttributeKey::FontSize, AttributeValue::Float(12.0));
        map.set(AttributeKey::FontSize, AttributeValue::Float(14.0));
        assert_eq!(map.get_f64(&AttributeKey::FontSize), Some(14.0));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn attribute_map_remove() {
        let mut map = AttributeMap::new();
        map.set(AttributeKey::Bold, AttributeValue::Bool(true));
        let old = map.remove(&AttributeKey::Bold);
        assert_eq!(old, Some(AttributeValue::Bool(true)));
        assert!(map.is_empty());
    }

    #[test]
    fn attribute_map_merge() {
        let mut base = AttributeMap::new();
        base.set(AttributeKey::Bold, AttributeValue::Bool(true));
        base.set(AttributeKey::FontSize, AttributeValue::Float(12.0));

        let mut overlay = AttributeMap::new();
        overlay.set(AttributeKey::FontSize, AttributeValue::Float(16.0));
        overlay.set(AttributeKey::Italic, AttributeValue::Bool(true));

        base.merge(&overlay);

        assert_eq!(base.get_bool(&AttributeKey::Bold), Some(true)); // kept
        assert_eq!(base.get_f64(&AttributeKey::FontSize), Some(16.0)); // overridden
        assert_eq!(base.get_bool(&AttributeKey::Italic), Some(true)); // added
        assert_eq!(base.len(), 3);
    }

    #[test]
    fn attribute_map_builder() {
        let map = AttributeMap::new()
            .bold(true)
            .italic(false)
            .font_size(14.0)
            .font_family("Arial")
            .color(Color::RED)
            .alignment(Alignment::Center);

        assert_eq!(map.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(map.get_bool(&AttributeKey::Italic), Some(false));
        assert_eq!(map.get_f64(&AttributeKey::FontSize), Some(14.0));
        assert_eq!(map.get_string(&AttributeKey::FontFamily), Some("Arial"));
        assert_eq!(map.get_color(&AttributeKey::Color), Some(Color::RED));
        assert_eq!(
            map.get_alignment(&AttributeKey::Alignment),
            Some(Alignment::Center)
        );
    }

    #[test]
    fn typed_getter_wrong_type() {
        let mut map = AttributeMap::new();
        map.set(AttributeKey::Bold, AttributeValue::Bool(true));
        assert_eq!(map.get_f64(&AttributeKey::Bold), None); // wrong type
        assert_eq!(map.get_string(&AttributeKey::Bold), None); // wrong type
    }
}
