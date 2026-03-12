//! Page layout engine for s1engine.
//!
//! Converts a `DocumentModel` into a `LayoutDocument` containing positioned
//! pages, blocks, lines, and glyphs ready for rendering or PDF export.
//!
//! # Layout Process
//!
//! 1. **Style resolution** — compute effective attributes for every node
//! 2. **Text shaping** — characters → positioned glyphs (via `s1-text`)
//! 3. **Line breaking** — Knuth-Plass optimal or greedy fallback
//! 4. **Block stacking** — paragraphs with spacing-before/after
//! 5. **Pagination** — break into pages, handle page-break-before
//! 6. **Output** — `LayoutDocument` with pages → blocks → lines → glyph runs

pub mod engine;
pub mod error;
pub mod style_resolver;
pub mod types;

pub use engine::{LayoutConfig, LayoutEngine};
pub use error::LayoutError;
pub use style_resolver::{
    resolve_paragraph_style, resolve_run_style, ResolvedParagraphStyle, ResolvedRunStyle,
};
pub use types::{
    GlyphRun, LayoutBlock, LayoutBlockKind, LayoutBookmark, LayoutCache, LayoutDocument,
    LayoutLine, LayoutPage, LayoutTableCell, LayoutTableRow, PageLayout, Rect,
};
