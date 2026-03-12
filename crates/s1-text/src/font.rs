//! Font loading and metrics via `ttf-parser`.

use crate::error::TextError;
use crate::types::FontMetrics;

/// A parsed font face providing metrics and glyph information.
///
/// Wraps `ttf_parser::Face` with owned data so the font can be stored
/// and used across the application lifetime.
pub struct Font {
    /// Raw font data (kept alive for the Face reference).
    _data: Vec<u8>,
    /// Parsed font face. Uses `'static` lifetime via the self-referential trick:
    /// the data is heap-allocated and never moves.
    face: ttf_parser::Face<'static>,
}

// SAFETY: Font is Send+Sync because the underlying data is immutable once parsed.
// ttf_parser::Face only reads from the data buffer.
unsafe impl Send for Font {}
unsafe impl Sync for Font {}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font")
            .field("family", &self.family_name())
            .field("units_per_em", &self.face.units_per_em())
            .finish()
    }
}

impl Font {
    /// Load a font from raw bytes (TrueType or OpenType).
    ///
    /// The font data is copied and owned by the `Font` instance.
    ///
    /// # Errors
    ///
    /// Returns `TextError::FontParse` if the data is not a valid font.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, TextError> {
        // Leak the data to get a 'static reference, then parse.
        // We keep the original Vec to drop it properly.
        let data_ptr = data.as_ptr();
        let data_len = data.len();

        // SAFETY: We're creating a slice from owned data that outlives the Face.
        // The Vec is stored in the struct and never moved/dropped before the Face.
        let static_slice = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };

        let face = ttf_parser::Face::parse(static_slice, 0)
            .map_err(|e| TextError::FontParse(format!("{e}")))?;

        Ok(Self { _data: data, face })
    }

    /// Get the font family name.
    pub fn family_name(&self) -> String {
        self.face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
            .or_else(|| {
                self.face
                    .names()
                    .into_iter()
                    .find(|name| name.name_id == ttf_parser::name_id::FAMILY)
            })
            .and_then(|name| name.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Get the font style name (e.g., "Regular", "Bold", "Italic").
    pub fn style_name(&self) -> String {
        self.face
            .names()
            .into_iter()
            .find(|name| name.name_id == ttf_parser::name_id::TYPOGRAPHIC_SUBFAMILY)
            .or_else(|| {
                self.face
                    .names()
                    .into_iter()
                    .find(|name| name.name_id == ttf_parser::name_id::SUBFAMILY)
            })
            .and_then(|name| name.to_string())
            .unwrap_or_else(|| "Regular".to_string())
    }

    /// Whether the font is bold.
    pub fn is_bold(&self) -> bool {
        self.face.is_bold()
    }

    /// Whether the font is italic.
    pub fn is_italic(&self) -> bool {
        self.face.is_italic()
    }

    /// Get font metrics scaled to the given point size.
    pub fn metrics(&self, size: f64) -> FontMetrics {
        let upem = self.face.units_per_em() as f64;
        let scale = size / upem;

        let ascent = self.face.ascender() as f64 * scale;
        let descent = self.face.descender() as f64 * scale;
        let line_gap = self.face.line_gap() as f64 * scale;

        let underline_position = self
            .face
            .underline_metrics()
            .map(|m| m.position as f64 * scale)
            .unwrap_or(-size * 0.1);
        let underline_thickness = self
            .face
            .underline_metrics()
            .map(|m| m.thickness as f64 * scale)
            .unwrap_or(size * 0.05);

        FontMetrics {
            ascent,
            descent,
            line_gap,
            units_per_em: self.face.units_per_em(),
            underline_position,
            underline_thickness,
        }
    }

    /// Get the glyph index for a Unicode character.
    pub fn glyph_index(&self, ch: char) -> Option<u16> {
        self.face.glyph_index(ch).map(|gid| gid.0)
    }

    /// Check whether the font has a glyph for the given character.
    pub fn has_glyph(&self, ch: char) -> bool {
        self.face.glyph_index(ch).is_some()
    }

    /// Get the horizontal advance width for a glyph (in font units).
    pub fn glyph_hor_advance(&self, glyph_id: u16) -> Option<u16> {
        self.face
            .glyph_hor_advance(ttf_parser::GlyphId(glyph_id))
    }

    /// Get the number of glyphs in the font.
    pub fn number_of_glyphs(&self) -> u16 {
        self.face.number_of_glyphs()
    }

    /// Get the units-per-em value.
    pub fn units_per_em(&self) -> u16 {
        self.face.units_per_em()
    }

    /// Get the raw font data bytes.
    pub fn data(&self) -> &[u8] {
        &self._data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // We'll use the system's default font for testing
    fn get_test_font_data() -> Option<Vec<u8>> {
        // Try macOS system font locations
        let paths = [
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/Times.ttc",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", // Linux
        ];
        for path in &paths {
            if let Ok(data) = std::fs::read(path) {
                return Some(data);
            }
        }
        None
    }

    #[test]
    fn font_from_bytes_invalid() {
        let result = Font::from_bytes(vec![0, 1, 2, 3]);
        assert!(result.is_err());
        match result.unwrap_err() {
            TextError::FontParse(_) => {}
            other => panic!("expected FontParse, got {other:?}"),
        }
    }

    #[test]
    fn font_from_bytes_empty() {
        let result = Font::from_bytes(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn font_metrics_scaling() {
        if let Some(data) = get_test_font_data() {
            let font = Font::from_bytes(data).unwrap();
            let m12 = font.metrics(12.0);
            let m24 = font.metrics(24.0);
            // Metrics at double size should be ~double
            let ratio = m24.ascent / m12.ascent;
            assert!((ratio - 2.0).abs() < 0.01, "ratio was {ratio}");
            assert!(m12.ascent > 0.0);
            assert!(m12.descent < 0.0);
            assert!(m12.units_per_em > 0);
        }
    }

    #[test]
    fn font_has_glyph() {
        if let Some(data) = get_test_font_data() {
            let font = Font::from_bytes(data).unwrap();
            // Most fonts have 'A'
            assert!(font.has_glyph('A'));
            assert!(font.glyph_index('A').is_some());
        }
    }

    #[test]
    fn font_family_name() {
        if let Some(data) = get_test_font_data() {
            let font = Font::from_bytes(data).unwrap();
            let name = font.family_name();
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn font_line_height() {
        if let Some(data) = get_test_font_data() {
            let font = Font::from_bytes(data).unwrap();
            let m = font.metrics(12.0);
            assert!(m.line_height() > 0.0);
        }
    }
}
