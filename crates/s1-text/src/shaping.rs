//! Text shaping via `rustybuzz`.

use std::str::FromStr;

use crate::error::TextError;
use crate::font::Font;
use crate::types::{Direction, FontFeature, ShapedGlyph};

/// Shape a string of text using the given font, returning positioned glyphs.
///
/// This is the core text shaping function. It takes a string and a font,
/// runs the HarfBuzz-compatible shaping algorithm (via `rustybuzz`), and
/// returns a sequence of glyphs with precise positioning.
///
/// # Arguments
///
/// * `text` — The text to shape.
/// * `font` — The font to use for shaping.
/// * `font_size` — Font size in points. Glyph advances/offsets are scaled to this size.
/// * `features` — OpenType features to apply (e.g., ligatures, kerning).
/// * `language` — Optional BCP 47 language tag for language-specific shaping.
/// * `direction` — Text direction (LTR or RTL).
///
/// # Errors
///
/// Returns `TextError::ShapingFailed` if the font cannot be used for shaping.
pub fn shape_text(
    text: &str,
    font: &Font,
    font_size: f64,
    features: &[FontFeature],
    language: Option<&str>,
    direction: Direction,
) -> Result<Vec<ShapedGlyph>, TextError> {
    // Create a rustybuzz Face from the font data
    let rb_face = rustybuzz::Face::from_slice(font.data(), 0)
        .ok_or_else(|| TextError::ShapingFailed("failed to create shaping face".into()))?;

    // Create shaping plan
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);

    // Set direction
    buffer.set_direction(match direction {
        Direction::Ltr => rustybuzz::Direction::LeftToRight,
        Direction::Rtl => rustybuzz::Direction::RightToLeft,
    });

    // Set language if provided
    if let Some(lang) = language {
        if let Ok(rb_lang) = rustybuzz::Language::from_str(lang) {
            buffer.set_language(rb_lang);
        }
    }

    // Convert font features
    let rb_features: Vec<rustybuzz::Feature> = features
        .iter()
        .map(|f| {
            let tag = ttf_parser::Tag::from_bytes(&f.tag);
            rustybuzz::Feature::new(tag, f.value, ..)
        })
        .collect();

    // Shape the text
    let output = rustybuzz::shape(&rb_face, &rb_features, buffer);

    // Scale factor: font design units → points
    let upem = font.units_per_em() as f64;
    let scale = font_size / upem;

    // Convert output glyphs
    let info = output.glyph_infos();
    let positions = output.glyph_positions();

    let glyphs = info
        .iter()
        .zip(positions.iter())
        .map(|(gi, gp)| ShapedGlyph {
            glyph_id: gi.glyph_id as u16,
            x_advance: gp.x_advance as f64 * scale,
            y_advance: gp.y_advance as f64 * scale,
            x_offset: gp.x_offset as f64 * scale,
            y_offset: gp.y_offset as f64 * scale,
            cluster: gi.cluster,
        })
        .collect();

    Ok(glyphs)
}

/// Measure the total advance width of shaped text.
pub fn measure_shaped_width(glyphs: &[ShapedGlyph]) -> f64 {
    glyphs.iter().map(|g| g.x_advance).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_db::FontDatabase;

    fn get_test_font() -> Option<Font> {
        let db = FontDatabase::new();
        let families = ["Helvetica", "Arial", "DejaVu Sans", "Liberation Sans"];
        for family in &families {
            if let Some(id) = db.find(family, false, false) {
                return db.load_font(id);
            }
        }
        None
    }

    #[test]
    fn shape_simple_text() {
        if let Some(font) = get_test_font() {
            let glyphs = shape_text("Hello", &font, 12.0, &[], None, Direction::Ltr).unwrap();
            assert_eq!(glyphs.len(), 5); // One glyph per character
                                         // All glyphs should have positive advance
            for g in &glyphs {
                assert!(g.x_advance > 0.0, "glyph {g:?} has non-positive advance");
            }
        }
    }

    #[test]
    fn shape_empty_text() {
        if let Some(font) = get_test_font() {
            let glyphs = shape_text("", &font, 12.0, &[], None, Direction::Ltr).unwrap();
            assert!(glyphs.is_empty());
        }
    }

    #[test]
    fn shape_with_size_scaling() {
        if let Some(font) = get_test_font() {
            let g12 = shape_text("A", &font, 12.0, &[], None, Direction::Ltr).unwrap();
            let g24 = shape_text("A", &font, 24.0, &[], None, Direction::Ltr).unwrap();
            assert_eq!(g12.len(), 1);
            assert_eq!(g24.len(), 1);
            // At double size, advance should be double
            let ratio = g24[0].x_advance / g12[0].x_advance;
            assert!(
                (ratio - 2.0).abs() < 0.01,
                "expected 2x scaling, got {ratio}"
            );
        }
    }

    #[test]
    fn shape_rtl_text() {
        if let Some(font) = get_test_font() {
            // Simple ASCII in RTL mode — glyphs should still be produced
            let glyphs = shape_text("test", &font, 12.0, &[], None, Direction::Rtl).unwrap();
            assert_eq!(glyphs.len(), 4);
        }
    }

    #[test]
    fn shape_unicode_text() {
        if let Some(font) = get_test_font() {
            let glyphs = shape_text("café", &font, 12.0, &[], None, Direction::Ltr).unwrap();
            // 'c', 'a', 'f', 'é' = 4 glyphs
            assert_eq!(glyphs.len(), 4);
        }
    }

    #[test]
    fn shape_cluster_mapping() {
        if let Some(font) = get_test_font() {
            let glyphs = shape_text("ABC", &font, 12.0, &[], None, Direction::Ltr).unwrap();
            // Clusters should map back to source bytes: 0, 1, 2
            assert_eq!(glyphs[0].cluster, 0);
            assert_eq!(glyphs[1].cluster, 1);
            assert_eq!(glyphs[2].cluster, 2);
        }
    }

    #[test]
    fn measure_width() {
        if let Some(font) = get_test_font() {
            let glyphs = shape_text("Hello World", &font, 12.0, &[], None, Direction::Ltr).unwrap();
            let width = measure_shaped_width(&glyphs);
            assert!(width > 0.0);
        }
    }

    #[test]
    fn shape_with_feature() {
        if let Some(font) = get_test_font() {
            // Enable kerning explicitly
            let features = vec![FontFeature::enabled(*b"kern")];
            let glyphs = shape_text("AV", &font, 12.0, &features, None, Direction::Ltr).unwrap();
            assert_eq!(glyphs.len(), 2);
        }
    }
}
