//! Standalone helper functions and types used by the layout engine.

use s1_model::{AttributeValue, DocumentModel, LineSpacing, NodeId};
use s1_text::{FontId, FontMetrics, ShapedGlyph};

use crate::types::{InlineImage, LayoutBlock, LayoutBlockKind, Rect};

/// Shaped run info — intermediate result before line breaking.
pub(crate) struct ShapedRunInfo {
    pub(crate) source_id: NodeId,
    pub(crate) font_id: Option<FontId>,
    pub(crate) font_family: String,
    pub(crate) font_size: f64,
    pub(crate) color: s1_model::Color,
    pub(crate) glyphs: Vec<ShapedGlyph>,
    pub(crate) is_line_break: bool,
    pub(crate) metrics: Option<FontMetrics>,
    pub(crate) hyperlink_url: Option<String>,
    /// Original text content.
    pub(crate) text: String,
    /// Bold formatting.
    pub(crate) bold: bool,
    /// Italic formatting.
    pub(crate) italic: bool,
    /// Underline style ("none", "single", "double", "thick", "dotted", "dashed", "wave").
    pub(crate) underline: String,
    /// Strikethrough formatting.
    pub(crate) strikethrough: bool,
    /// Double strikethrough formatting.
    pub(crate) double_strikethrough: bool,
    /// Superscript formatting.
    pub(crate) superscript: bool,
    /// Subscript formatting.
    pub(crate) subscript: bool,
    /// Highlight/background color.
    pub(crate) highlight_color: Option<s1_model::Color>,
    /// Character spacing in points.
    pub(crate) character_spacing: f64,
    /// Baseline shift in points.
    pub(crate) baseline_shift: f64,
    /// All caps text transform.
    pub(crate) caps: bool,
    /// Small caps text transform.
    pub(crate) small_caps: bool,
    /// Hidden text (excluded from rendering).
    pub(crate) hidden: bool,
    /// Revision type for track changes.
    pub(crate) revision_type: Option<String>,
    /// Revision author for track changes.
    pub(crate) revision_author: Option<String>,
    /// Inline image data, if this run represents an inline image.
    pub(crate) inline_image: Option<InlineImage>,
}

/// Compute a content hash for a node and its descendants.
///
/// The hash includes the node's attributes, text content of all descendants,
/// and style information. Used for cache invalidation in incremental layout.
pub(crate) fn content_hash(doc: &DocumentModel, node_id: NodeId) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    hash_node(doc, node_id, &mut hash);
    hash
}

fn hash_node(doc: &DocumentModel, node_id: NodeId, hash: &mut u64) {
    if let Some(node) = doc.node(node_id) {
        // Hash node type
        *hash ^= node.node_type as u64;
        *hash = hash.wrapping_mul(0x100000001b3);

        // Hash text content
        if let Some(ref text) = node.text_content {
            for byte in text.bytes() {
                *hash ^= byte as u64;
                *hash = hash.wrapping_mul(0x100000001b3);
            }
        }

        // Hash key attributes that affect layout
        for (key, val) in node.attributes.iter() {
            // Hash the key using its debug representation
            use std::hash::{Hash, Hasher};
            struct FnvHasher(u64);
            impl Hasher for FnvHasher {
                fn finish(&self) -> u64 {
                    self.0
                }
                fn write(&mut self, bytes: &[u8]) {
                    for &b in bytes {
                        self.0 ^= b as u64;
                        self.0 = self.0.wrapping_mul(0x100000001b3);
                    }
                }
            }
            let mut key_hasher = FnvHasher(*hash);
            std::mem::discriminant(key).hash(&mut key_hasher);
            *hash = key_hasher.finish();

            // Hash the value
            match val {
                AttributeValue::Bool(b) => {
                    *hash ^= *b as u64;
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
                AttributeValue::Float(f) => {
                    *hash ^= f.to_bits();
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
                AttributeValue::Int(i) => {
                    *hash ^= *i as u64;
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
                AttributeValue::String(s) => {
                    for byte in s.bytes() {
                        *hash ^= byte as u64;
                        *hash = hash.wrapping_mul(0x100000001b3);
                    }
                }
                _ => {
                    // For complex types, just hash a sentinel
                    *hash ^= 0xDEADBEEF;
                    *hash = hash.wrapping_mul(0x100000001b3);
                }
            }
        }

        // Recurse into children
        for &child_id in &node.children {
            hash_node(doc, child_id, hash);
        }
    }
}

/// Format a border side as a CSS border value, returning None for `BorderStyle::None`.
pub(crate) fn format_border_css_opt(border: &s1_model::BorderSide) -> Option<String> {
    if border.style == s1_model::BorderStyle::None {
        return None;
    }
    Some(format_border_css(border))
}

/// Format a border side as a CSS border value.
pub(crate) fn format_border_css(border: &s1_model::BorderSide) -> String {
    let style_str = match border.style {
        s1_model::BorderStyle::None => "none",
        s1_model::BorderStyle::Single => "solid",
        s1_model::BorderStyle::Double => "double",
        s1_model::BorderStyle::Dotted => "dotted",
        s1_model::BorderStyle::Dashed => "dashed",
        s1_model::BorderStyle::Thick => "solid",
        _ => "solid",
    };
    let width = if border.width > 0.0 {
        border.width
    } else {
        1.0
    };
    format!(
        "{:.1}pt {} #{:02x}{:02x}{:02x}",
        width, style_str, border.color.r, border.color.g, border.color.b
    )
}

/// Synthesize glyphs when no font is available (fallback for headless testing).
///
/// Uses character-class-based width estimation for more accurate line breaking
/// than a single average width for all characters.
pub(crate) fn synthesize_glyphs(text: &str, font_size: f64) -> Vec<ShapedGlyph> {
    text.char_indices()
        .map(|(i, ch)| {
            let cp = ch as u32;
            let is_cjk = (0x4E00..=0x9FFF).contains(&cp)
                || (0x3400..=0x4DBF).contains(&cp)
                || (0x3000..=0x303F).contains(&cp)
                || (0xFF00..=0xFFEF).contains(&cp)
                || (0x3040..=0x309F).contains(&cp)
                || (0x30A0..=0x30FF).contains(&cp)
                || (0xAC00..=0xD7AF).contains(&cp)
                || (0x20000..=0x2A6DF).contains(&cp)
                || (0x2A700..=0x2B73F).contains(&cp)
                || (0xF900..=0xFAFF).contains(&cp);
            let width_factor = if is_cjk {
                1.0
            } else {
                match ch {
                    '\t' => 4.0,
                    'i' | 'j' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | '"' | '`' => 0.3,
                    'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '{' | '}' => 0.4,
                    'm' | 'w' | 'M' | 'W' | '@' | '%' => 0.8,
                    'A'..='Z' => 0.65,
                    ' ' => 0.3,
                    _ => 0.5,
                }
            };
            ShapedGlyph {
                glyph_id: 0,
                x_advance: font_size * width_factor,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
                cluster: i as u32,
            }
        })
        .collect()
}

/// Split a paragraph block at a given line index into two blocks.
///
/// The first block contains lines `0..split_at` and the second block contains
/// lines `split_at..`. The second block is marked `is_continuation: true` so
/// the renderer knows not to repeat list markers or first-line indent.
///
/// Both blocks share the same `source_id` (they represent the same paragraph
/// node). The first block's `split_at_line` is set to `split_at` so the JS
/// renderer knows where the split occurred. The second block's `split_at_line`
/// is also set to `split_at` (the original line index where continuation starts).
pub(crate) fn split_paragraph_block(
    block: LayoutBlock,
    split_at: usize,
    y_pos: f64,
) -> (LayoutBlock, LayoutBlock) {
    let source_id = block.source_id;
    let bounds = block.bounds;

    match block.kind {
        LayoutBlockKind::Paragraph {
            lines,
            text_align,
            background_color,
            border,
            list_marker,
            list_level,
            space_before,
            space_after,
            indent_left,
            indent_right,
            indent_first_line,
            line_height,
            bidi,
            ..
        } => {
            let (first_lines, second_lines): (Vec<_>, Vec<_>) = {
                let mut first = Vec::with_capacity(split_at);
                let mut second = Vec::with_capacity(lines.len() - split_at);
                for (i, line) in lines.into_iter().enumerate() {
                    if i < split_at {
                        first.push(line);
                    } else {
                        second.push(line);
                    }
                }
                (first, second)
            };

            let first_height: f64 = first_lines.iter().map(|l| l.height).sum();
            let second_height: f64 = second_lines.iter().map(|l| l.height).sum();

            let first_block = LayoutBlock {
                source_id,
                bounds: Rect::new(bounds.x, y_pos, bounds.width, first_height),
                kind: LayoutBlockKind::Paragraph {
                    lines: first_lines,
                    text_align: text_align.clone(),
                    background_color,
                    border: border.clone(),
                    list_marker: list_marker.clone(),
                    list_level,
                    space_before,
                    space_after: 0.0, // No space after the first part
                    indent_left,
                    indent_right,
                    indent_first_line,
                    line_height,
                    bidi,
                    is_continuation: false,
                    split_at_line: split_at,
                },
            };

            let second_block = LayoutBlock {
                source_id,
                bounds: Rect::new(bounds.x, 0.0, bounds.width, second_height),
                kind: LayoutBlockKind::Paragraph {
                    lines: second_lines,
                    text_align,
                    background_color,
                    border,
                    list_marker: None, // Don't repeat list marker on continuation
                    list_level,
                    space_before: 0.0, // No space before continuation
                    space_after,
                    indent_left,
                    indent_right,
                    indent_first_line: 0.0, // No first-line indent on continuation
                    line_height,
                    bidi,
                    is_continuation: true,
                    split_at_line: split_at,
                },
            };

            (first_block, second_block)
        }
        // Non-paragraph blocks should not reach here, but handle gracefully
        other_kind => {
            let first = LayoutBlock {
                source_id,
                bounds,
                kind: other_kind,
            };
            let second = LayoutBlock {
                source_id,
                bounds: Rect::new(bounds.x, 0.0, bounds.width, 0.0),
                kind: LayoutBlockKind::Paragraph {
                    lines: Vec::new(),
                    text_align: None,
                    background_color: None,
                    border: None,
                    list_marker: None,
                    list_level: 0,
                    space_before: 0.0,
                    space_after: 0.0,
                    indent_left: 0.0,
                    indent_right: 0.0,
                    indent_first_line: 0.0,
                    line_height: None,
                    bidi: false,
                    is_continuation: true,
                    split_at_line: 0,
                },
            };
            (first, second)
        }
    }
}

/// Sanitize a floating-point value — replace NaN/infinity with 0.0.
///
/// Prevents garbage layout when style values are malformed or produced
/// by division-by-zero or other floating-point edge cases.
pub(crate) fn sanitize_pt(val: f64) -> f64 {
    if val.is_finite() {
        val
    } else {
        0.0
    }
}

/// Compute line height from the tallest run and line spacing.
pub(crate) fn compute_line_height(max_run_height: f64, line_spacing: &LineSpacing) -> f64 {
    let h = sanitize_pt(max_run_height);
    let result = match line_spacing {
        LineSpacing::Single => h,
        LineSpacing::OnePointFive => h * 1.5,
        LineSpacing::Double => h * 2.0,
        LineSpacing::Multiple(m) => h * sanitize_pt(*m),
        LineSpacing::AtLeast(min) => h.max(sanitize_pt(*min)),
        LineSpacing::Exact(exact) => sanitize_pt(*exact),
        _ => h,
    };
    sanitize_pt(result)
}

/// Convert a `LineSpacing` value to a CSS-compatible line-height multiplier.
///
/// Returns `None` for the default single spacing (browser default is close enough).
pub(crate) fn line_spacing_to_css(spacing: &LineSpacing) -> Option<f64> {
    match spacing {
        LineSpacing::Single => None, // browser default ~1.2 is close; omit for cleaner output
        LineSpacing::OnePointFive => Some(1.5),
        LineSpacing::Double => Some(2.0),
        LineSpacing::Multiple(m) => {
            if (*m - 1.0).abs() < 0.001 {
                None // essentially single
            } else {
                Some(*m)
            }
        }
        LineSpacing::Exact(pts) => {
            // For exact spacing, emit pt-based value; we store as negative to
            // distinguish from multipliers, but CSS line-height in pt is fine.
            // We'll emit this as a raw points value — html.rs will handle the unit.
            Some(-(*pts)) // negative signals "pt" to the HTML emitter
        }
        LineSpacing::AtLeast(pts) => {
            // AtLeast is a minimum; CSS doesn't have a direct equivalent, so
            // we approximate by using the value as a minimum line-height.
            Some(-(*pts)) // negative signals "pt" to the HTML emitter
        }
        _ => None,
    }
}
