//! Node attributes and formatting properties.
//!
//! Attributes use a typed key-value system to avoid stringly-typed errors.
//! The [`AttributeMap`] stores key-value pairs where keys are [`AttributeKey`]
//! variants and values are [`AttributeValue`] variants.

use std::collections::HashMap;

// ─── Supporting Types ───────────────────────────────────────────────────────

/// An RGBA color.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

/// Underline style.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

/// Table or cell width specification.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// A tab stop definition.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabAlignment {
    Left,
    Center,
    Right,
    Decimal,
}

/// Tab leader character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabLeader {
    None,
    Dot,
    Dash,
    Underscore,
}

/// List/numbering information for a paragraph.
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
}

/// Border configuration for a box (paragraph, cell, table).
#[derive(Debug, Clone, PartialEq)]
pub struct Borders {
    pub top: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
    pub right: Option<BorderSide>,
}

/// A single border side.
#[derive(Debug, Clone, PartialEq)]
pub struct BorderSide {
    pub style: BorderStyle,
    pub width: f64,
    pub color: Color,
    pub spacing: f64,
}

/// Border line style.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaId(pub u64);

/// Type of dynamic field.
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
    Custom,
}

// ─── Attribute Key/Value System ─────────────────────────────────────────────

/// Typed attribute keys.
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
    ParagraphBorders,
    Background,
    TabStops,
    StyleId,
    ListInfo,

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

    // TOC attributes
    /// Maximum heading level included in the TOC (1-9, default 3).
    TocMaxLevel,
    /// Custom title for the TOC (e.g. "Table of Contents").
    TocTitle,

    // Link / annotation attributes
    HyperlinkUrl,
    HyperlinkTooltip,
    BookmarkName,
    CommentId,
    CommentAuthor,
    CommentDate,
}

/// Typed attribute values.
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
}

// ─── AttributeMap ───────────────────────────────────────────────────────────

/// A map of typed attributes. Used on every node for formatting properties.
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
