//! Layout output types.
//!
//! These represent the fully laid-out document ready for rendering or PDF export.

use std::collections::HashMap;

use s1_model::{Color, NodeId};
use s1_text::{FontId, ShapedGlyph};

/// A fully laid-out document with pages.
#[derive(Debug, Clone)]
pub struct LayoutDocument {
    /// Pages in document order.
    pub pages: Vec<LayoutPage>,
    /// Bookmarks with resolved page positions.
    pub bookmarks: Vec<LayoutBookmark>,
}

/// A bookmark with its resolved position in the laid-out document.
#[derive(Debug, Clone)]
pub struct LayoutBookmark {
    /// Bookmark name.
    pub name: String,
    /// 0-based page index where this bookmark appears.
    pub page_index: usize,
    /// Y position on the page (in points from top).
    pub y_position: f64,
}

/// A single laid-out page.
#[derive(Debug, Clone)]
pub struct LayoutPage {
    /// 0-based page index.
    pub index: usize,
    /// Page width in points.
    pub width: f64,
    /// Page height in points.
    pub height: f64,
    /// Content area after margins.
    pub content_area: Rect,
    /// Content blocks on this page.
    pub blocks: Vec<LayoutBlock>,
    /// Header content (if any).
    pub header: Option<LayoutBlock>,
    /// Footer content (if any).
    pub footer: Option<LayoutBlock>,
}

/// A positioned block element (paragraph, table, or image).
#[derive(Debug, Clone)]
pub struct LayoutBlock {
    /// Reference back to the source document node.
    pub source_id: NodeId,
    /// Position and size in the page coordinate system.
    pub bounds: Rect,
    /// Block content.
    pub kind: LayoutBlockKind,
}

/// The kind of a layout block.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum LayoutBlockKind {
    /// A paragraph with broken lines.
    Paragraph {
        /// Lines of text.
        lines: Vec<LayoutLine>,
    },
    /// A table with rows.
    Table {
        /// Table rows with cells.
        rows: Vec<LayoutTableRow>,
    },
    /// An inline image.
    Image {
        /// Image data reference.
        media_id: String,
        /// Image bounds.
        bounds: Rect,
        /// Raw image bytes (populated during layout from MediaStore).
        image_data: Option<Vec<u8>>,
        /// MIME content type (e.g., "image/png", "image/jpeg").
        content_type: Option<String>,
    },
}

/// A line of text within a paragraph.
#[derive(Debug, Clone)]
pub struct LayoutLine {
    /// Y position of the baseline (relative to the block).
    pub baseline_y: f64,
    /// Line height.
    pub height: f64,
    /// Glyph runs on this line.
    pub runs: Vec<GlyphRun>,
}

/// A contiguous run of glyphs with uniform formatting.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    /// Reference to the source Run node.
    pub source_id: NodeId,
    /// Font used for this run.
    pub font_id: FontId,
    /// Font size in points.
    pub font_size: f64,
    /// Text color.
    pub color: Color,
    /// X offset of this run from the line start.
    pub x_offset: f64,
    /// Positioned glyphs.
    pub glyphs: Vec<ShapedGlyph>,
    /// Total advance width of this run.
    pub width: f64,
    /// Hyperlink URL if this run is part of a hyperlink.
    pub hyperlink_url: Option<String>,
}

/// A table row in the layout.
#[derive(Debug, Clone)]
pub struct LayoutTableRow {
    /// Row bounds relative to the table block.
    pub bounds: Rect,
    /// Cells in this row.
    pub cells: Vec<LayoutTableCell>,
}

/// A table cell in the layout.
#[derive(Debug, Clone)]
pub struct LayoutTableCell {
    /// Cell bounds relative to the row.
    pub bounds: Rect,
    /// Content blocks inside the cell.
    pub blocks: Vec<LayoutBlock>,
}

/// A rectangle with position and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X position (from left).
    pub x: f64,
    /// Y position (from top).
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Create a new rectangle.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The right edge (x + width).
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// The bottom edge (y + height).
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

/// Page dimensions and margins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageLayout {
    /// Page width in points.
    pub width: f64,
    /// Page height in points.
    pub height: f64,
    /// Top margin in points.
    pub margin_top: f64,
    /// Bottom margin in points.
    pub margin_bottom: f64,
    /// Left margin in points.
    pub margin_left: f64,
    /// Right margin in points.
    pub margin_right: f64,
}

impl PageLayout {
    /// US Letter default (8.5" × 11" with 1" margins).
    pub fn letter() -> Self {
        Self {
            width: 612.0,  // 8.5 * 72
            height: 792.0, // 11 * 72
            margin_top: 72.0,
            margin_bottom: 72.0,
            margin_left: 72.0,
            margin_right: 72.0,
        }
    }

    /// A4 default (210mm × 297mm with ~1" margins).
    pub fn a4() -> Self {
        Self {
            width: 595.28,  // 210mm in points
            height: 841.89, // 297mm in points
            margin_top: 72.0,
            margin_bottom: 72.0,
            margin_left: 72.0,
            margin_right: 72.0,
        }
    }

    /// Available content width.
    pub fn content_width(&self) -> f64 {
        self.width - self.margin_left - self.margin_right
    }

    /// Available content height.
    pub fn content_height(&self) -> f64 {
        self.height - self.margin_top - self.margin_bottom
    }

    /// Content area rectangle.
    pub fn content_rect(&self) -> Rect {
        Rect::new(
            self.margin_left,
            self.margin_top,
            self.content_width(),
            self.content_height(),
        )
    }
}

impl Default for PageLayout {
    fn default() -> Self {
        Self::letter()
    }
}

/// Cache for incremental layout.
///
/// Stores previously computed block layouts keyed by `(NodeId, content_hash)`.
/// When a block's content hash matches the cached value, the expensive text
/// shaping and line breaking are skipped. Pagination still runs from scratch
/// because one changed paragraph shifts all subsequent pages.
#[derive(Debug, Clone, Default)]
pub struct LayoutCache {
    entries: HashMap<NodeId, CacheEntry>,
}

/// A single cache entry mapping a content hash to a laid-out block.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Content hash at the time the block was laid out.
    content_hash: u64,
    /// The cached layout block (position-independent — bounds.y will be overwritten).
    block: LayoutBlock,
}

impl LayoutCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Look up a cached block for the given node ID and content hash.
    ///
    /// Returns `Some(&LayoutBlock)` if the cache has an entry with a matching hash.
    pub fn get(&self, node_id: NodeId, content_hash: u64) -> Option<&LayoutBlock> {
        self.entries.get(&node_id).and_then(|entry| {
            if entry.content_hash == content_hash {
                Some(&entry.block)
            } else {
                None
            }
        })
    }

    /// Insert or update a cached block for the given node ID.
    pub fn insert(&mut self, node_id: NodeId, content_hash: u64, block: LayoutBlock) {
        self.entries.insert(
            node_id,
            CacheEntry {
                content_hash,
                block,
            },
        );
    }

    /// Number of entries in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all cache entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
