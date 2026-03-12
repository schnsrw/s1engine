//! Font discovery via `fontdb`.

use crate::error::TextError;
use crate::font::Font;
use crate::types::FontId;

/// A font database for discovering and loading system fonts.
///
/// Wraps `fontdb::Database` to provide font family lookup with bold/italic
/// matching and glyph-based fallback.
pub struct FontDatabase {
    db: fontdb::Database,
}

impl std::fmt::Debug for FontDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontDatabase")
            .field("font_count", &self.db.len())
            .finish()
    }
}

impl FontDatabase {
    /// Create a new font database loaded with system fonts.
    ///
    /// On `wasm32` targets, this creates an empty database since there is no
    /// filesystem. Use `empty()` + `load_font_data()` to add fonts manually.
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        #[cfg(not(target_arch = "wasm32"))]
        db.load_system_fonts();
        Self { db }
    }

    /// Create an empty font database (no system fonts loaded).
    pub fn empty() -> Self {
        Self {
            db: fontdb::Database::new(),
        }
    }

    /// Load fonts from a directory.
    pub fn load_fonts_dir(&mut self, path: &std::path::Path) {
        self.db.load_fonts_dir(path);
    }

    /// Load a single font file.
    pub fn load_font_file(&mut self, path: &std::path::Path) -> Result<(), TextError> {
        self.db
            .load_font_file(path)
            .map_err(|e| TextError::FontParse(format!("{e}")))?;
        Ok(())
    }

    /// Load a font from raw bytes.
    pub fn load_font_data(&mut self, data: Vec<u8>) {
        self.db.load_font_data(data);
    }

    /// Find a font by family name, weight, and style.
    ///
    /// Returns the `FontId` of the best match, or `None` if no font matches.
    pub fn find(&self, family: &str, bold: bool, italic: bool) -> Option<FontId> {
        let weight = if bold {
            fontdb::Weight::BOLD
        } else {
            fontdb::Weight::NORMAL
        };
        let style = if italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        };

        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight,
            stretch: fontdb::Stretch::Normal,
            style,
        };

        self.db.query(&query).map(FontId)
    }

    /// Find a fallback font that contains a glyph for the given character.
    pub fn fallback(&self, ch: char) -> Option<FontId> {
        // Iterate through all fonts and find one that has the glyph
        for face_info in self.db.faces() {
            if let Some(font) = self.load_font(FontId(face_info.id)) {
                if font.has_glyph(ch) {
                    return Some(FontId(face_info.id));
                }
            }
        }
        None
    }

    /// Load a font by its ID, returning a `Font` instance.
    ///
    /// Returns `None` if the font data cannot be loaded or parsed.
    pub fn load_font(&self, id: FontId) -> Option<Font> {
        self.db.with_face_data(id.0, |data, face_index| {
            // For TTC files, we'd need to handle face_index.
            // ttf-parser handles this via Face::parse(data, face_index).
            let _ = face_index;
            Font::from_bytes(data.to_vec()).ok()
        })?
    }

    /// Get the number of fonts in the database.
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Check if the database is empty.
    pub fn is_empty(&self) -> bool {
        self.db.len() == 0
    }

    /// Get font family name for a given font ID.
    pub fn family_name(&self, id: FontId) -> Option<String> {
        self.db.face(id.0).map(|info| {
            info.families
                .first()
                .map(|(name, _)| name.clone())
                .unwrap_or_default()
        })
    }
}

impl Default for FontDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_database() {
        let db = FontDatabase::empty();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn system_fonts_loaded() {
        let db = FontDatabase::new();
        // On any modern OS, there should be system fonts
        assert!(db.len() > 0, "no system fonts found");
    }

    #[test]
    fn find_common_font() {
        let db = FontDatabase::new();
        // Try common fonts that should exist on macOS or Linux
        let families = ["Helvetica", "Arial", "Times New Roman", "DejaVu Sans"];
        let found = families.iter().any(|f| db.find(f, false, false).is_some());
        assert!(found, "no common font found");
    }

    #[test]
    fn find_bold_variant() {
        let db = FontDatabase::new();
        let families = ["Helvetica", "Arial", "DejaVu Sans"];
        for family in &families {
            if let Some(regular) = db.find(family, false, false) {
                // Bold variant should also exist
                if let Some(bold) = db.find(family, true, false) {
                    // They should be different font IDs
                    assert_ne!(
                        regular, bold,
                        "bold should differ from regular for {family}"
                    );
                    return;
                }
            }
        }
        // Skip if no testable fonts found
    }

    #[test]
    fn load_font_from_db() {
        let db = FontDatabase::new();
        let families = ["Helvetica", "Arial", "DejaVu Sans"];
        for family in &families {
            if let Some(id) = db.find(family, false, false) {
                let font = db.load_font(id);
                assert!(font.is_some(), "failed to load {family}");
                if let Some(font) = font {
                    assert!(font.has_glyph('A'));
                }
                return;
            }
        }
    }

    #[test]
    fn fallback_for_latin() {
        let db = FontDatabase::new();
        // 'A' should be in some font
        let id = db.fallback('A');
        assert!(id.is_some(), "no fallback font for 'A'");
    }

    #[test]
    fn family_name_lookup() {
        let db = FontDatabase::new();
        let families = ["Helvetica", "Arial", "DejaVu Sans"];
        for family in &families {
            if let Some(id) = db.find(family, false, false) {
                let name = db.family_name(id);
                assert!(name.is_some());
                return;
            }
        }
    }
}
