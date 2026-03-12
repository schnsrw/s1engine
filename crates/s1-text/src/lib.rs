//! Text processing (shaping, fonts, Unicode) for s1engine.
//!
//! This crate provides the text processing pipeline for the s1engine document
//! engine. It handles:
//!
//! - **Font loading and metrics** — Parse TrueType/OpenType fonts, extract metrics
//! - **Font discovery** — Find system fonts by family, weight, style
//! - **Text shaping** — Convert text + font → positioned glyphs (via `rustybuzz`)
//! - **BiDi text** — Resolve bidirectional text runs (Unicode UAX #9)
//! - **Line breaking** — Find valid line break opportunities (Unicode UAX #14)
//!
//! # Architecture
//!
//! Uses pure-Rust implementations for all functionality:
//! - `rustybuzz` — HarfBuzz-compatible text shaping (pure Rust port)
//! - `ttf-parser` — Font parsing (TrueType, OpenType, WOFF)
//! - `fontdb` — System font discovery and indexing
//! - `unicode-bidi` — Unicode Bidirectional Algorithm
//! - `unicode-linebreak` — Unicode Line Breaking Algorithm

pub mod bidi;
pub mod error;
pub mod font;
pub mod font_db;
pub mod linebreak;
pub mod shaping;
pub mod types;

// Re-export core types for convenience.
pub use bidi::{bidi_resolve, paragraph_direction};
pub use error::TextError;
pub use font::Font;
pub use font_db::FontDatabase;
pub use linebreak::line_break_opportunities;
pub use shaping::{measure_shaped_width, shape_text};
pub use types::{
    BidiRun, BreakOpportunity, Direction, FontFeature, FontId, FontMetrics, ShapedGlyph,
};
