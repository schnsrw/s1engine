//! Core types for text processing.

/// Text direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direction {
    /// Left-to-right (Latin, CJK, etc.)
    Ltr,
    /// Right-to-left (Arabic, Hebrew, etc.)
    Rtl,
}

/// An OpenType font feature tag with value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFeature {
    /// 4-byte OpenType feature tag (e.g., `b"liga"`, `b"kern"`).
    pub tag: [u8; 4],
    /// Feature value: 0 = off, 1 = on, >1 for alternates.
    pub value: u32,
}

impl FontFeature {
    /// Create a new font feature.
    pub fn new(tag: [u8; 4], value: u32) -> Self {
        Self { tag, value }
    }

    /// Create an enabled feature.
    pub fn enabled(tag: [u8; 4]) -> Self {
        Self { tag, value: 1 }
    }

    /// Create a disabled feature.
    pub fn disabled(tag: [u8; 4]) -> Self {
        Self { tag, value: 0 }
    }
}

/// A shaped glyph with positioning information.
///
/// Produced by the text shaping pipeline. All measurements are in font units
/// (scaled by font_size / units_per_em for actual rendering).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// Font glyph index (used for rendering and PDF embedding).
    pub glyph_id: u16,
    /// Horizontal advance (how far to move after this glyph).
    pub x_advance: f64,
    /// Vertical advance (usually 0 for horizontal text).
    pub y_advance: f64,
    /// Horizontal offset from the current position.
    pub x_offset: f64,
    /// Vertical offset from the current position.
    pub y_offset: f64,
    /// Character cluster index — maps back to source text byte offset.
    pub cluster: u32,
}

/// Font metrics at a specific size.
///
/// All values are in points (1/72 inch) when computed for a given font size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    /// Distance from baseline to top of tallest glyph (positive).
    pub ascent: f64,
    /// Distance from baseline to bottom of lowest glyph (negative).
    pub descent: f64,
    /// Extra spacing recommended between lines.
    pub line_gap: f64,
    /// Font design units per em square.
    pub units_per_em: u16,
    /// Underline position (negative = below baseline).
    pub underline_position: f64,
    /// Underline thickness.
    pub underline_thickness: f64,
}

impl FontMetrics {
    /// Total line height (ascent - descent + line_gap).
    pub fn line_height(&self) -> f64 {
        self.ascent - self.descent + self.line_gap
    }
}

/// A bidirectional text run resolved by the Unicode BiDi algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidiRun {
    /// Byte offset in the source text where this run starts.
    pub start: usize,
    /// Byte offset in the source text where this run ends.
    pub end: usize,
    /// Resolved text direction for this run.
    pub direction: Direction,
    /// BiDi embedding level (even = LTR, odd = RTL).
    pub level: u8,
}

/// A line break opportunity in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakOpportunity {
    /// Byte offset in the source text where a break is allowed.
    pub offset: usize,
    /// Whether this is a mandatory break (e.g., at `\n`).
    pub mandatory: bool,
}

/// Identifier for a font in the font database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(pub fontdb::ID);
