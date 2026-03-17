//! Font discovery via `fontdb`.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::TextError;
use crate::font::Font;
use crate::types::FontId;

/// Embedded fallback font for WASM and headless environments.
///
/// This is a variable-weight Noto Sans font (OFL-licensed) that provides
/// broad Latin, Cyrillic, Greek, and many other script coverages. It is
/// loaded automatically in WASM builds so that text rendering works even
/// without user-loaded fonts.
#[cfg(feature = "embedded-font")]
static EMBEDDED_FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// Common font substitution table.
///
/// Maps document fonts that are often unavailable on other platforms to
/// widely-available alternatives. Used when the exact font isn't found.
const FONT_SUBSTITUTIONS: &[(&str, &[&str])] = &[
    // Microsoft Office fonts → alternatives
    (
        "Calibri",
        &[
            "Carlito",
            "Helvetica",
            "Arial",
            "Liberation Sans",
            "Noto Sans",
        ],
    ),
    (
        "Cambria",
        &[
            "Caladea",
            "Times New Roman",
            "Liberation Serif",
            "Noto Serif",
        ],
    ),
    (
        "Consolas",
        &[
            "Inconsolata",
            "Menlo",
            "DejaVu Sans Mono",
            "Liberation Mono",
        ],
    ),
    (
        "Segoe UI",
        &["Helvetica", "Arial", "Noto Sans", "Liberation Sans"],
    ),
    ("Verdana", &["DejaVu Sans", "Liberation Sans", "Noto Sans"]),
    ("Tahoma", &["DejaVu Sans", "Liberation Sans", "Noto Sans"]),
    (
        "Trebuchet MS",
        &["Liberation Sans", "DejaVu Sans", "Noto Sans"],
    ),
    (
        "Georgia",
        &["Liberation Serif", "DejaVu Serif", "Noto Serif"],
    ),
    (
        "Palatino Linotype",
        &["Palatino", "Liberation Serif", "Noto Serif"],
    ),
    (
        "Book Antiqua",
        &["Palatino", "Liberation Serif", "Noto Serif"],
    ),
    (
        "Garamond",
        &["EB Garamond", "Liberation Serif", "Noto Serif"],
    ),
    (
        "Century Gothic",
        &["URW Gothic", "Liberation Sans", "Noto Sans"],
    ),
    // macOS fonts → alternatives on Linux/Windows
    (
        "Helvetica Neue",
        &["Helvetica", "Arial", "Liberation Sans", "Noto Sans"],
    ),
    ("Helvetica", &["Arial", "Liberation Sans", "Noto Sans"]),
    (
        "Times",
        &["Times New Roman", "Liberation Serif", "Noto Serif"],
    ),
    // CJK font fallbacks
    ("SimSun", &["Noto Serif CJK SC", "WenQuanYi Zen Hei"]),
    ("SimHei", &["Noto Sans CJK SC", "WenQuanYi Zen Hei"]),
    ("MS Mincho", &["Noto Serif CJK JP", "IPAMincho"]),
    ("MS Gothic", &["Noto Sans CJK JP", "IPAGothic"]),
    ("Malgun Gothic", &["Noto Sans CJK KR"]),
    // Arabic
    (
        "Arabic Typesetting",
        &["Noto Naskh Arabic", "Noto Sans Arabic"],
    ),
    ("Sakkal Majalla", &["Noto Naskh Arabic", "Noto Sans Arabic"]),
    // Indic
    ("Mangal", &["Noto Sans Devanagari", "Noto Serif Devanagari"]),
];

/// Script-preferred font families for fallback ordering.
///
/// When a glyph is missing, fonts for the detected script are tried first.
fn script_preferred_fonts(script: unicode_script::Script) -> &'static [&'static str] {
    use unicode_script::Script;
    match script {
        Script::Arabic => &[
            "Noto Naskh Arabic",
            "Noto Sans Arabic",
            "Geeza Pro",
            "Arial Unicode MS",
        ],
        Script::Hebrew => &[
            "Noto Sans Hebrew",
            "Noto Serif Hebrew",
            "Arial Hebrew",
            "Arial Unicode MS",
        ],
        Script::Devanagari => &[
            "Noto Sans Devanagari",
            "Noto Serif Devanagari",
            "Kohinoor Devanagari",
        ],
        Script::Bengali => &["Noto Sans Bengali", "Noto Serif Bengali"],
        Script::Tamil => &["Noto Sans Tamil", "Noto Serif Tamil"],
        Script::Telugu => &["Noto Sans Telugu", "Noto Serif Telugu"],
        Script::Thai => &["Noto Sans Thai", "Noto Serif Thai", "Thonburi"],
        Script::Han => &[
            "Noto Sans CJK SC",
            "Noto Serif CJK SC",
            "PingFang SC",
            "Hiragino Sans",
            "WenQuanYi Zen Hei",
        ],
        Script::Hangul => &["Noto Sans CJK KR", "Apple SD Gothic Neo"],
        Script::Hiragana | Script::Katakana => &["Noto Sans CJK JP", "Hiragino Sans", "Yu Gothic"],
        Script::Cyrillic => &["Noto Sans", "DejaVu Sans", "Liberation Sans"],
        Script::Greek => &["Noto Sans", "DejaVu Sans", "Liberation Sans"],
        Script::Armenian => &["Noto Sans Armenian", "Noto Serif Armenian"],
        Script::Georgian => &["Noto Sans Georgian", "Noto Serif Georgian"],
        Script::Ethiopic => &["Noto Sans Ethiopic"],
        Script::Khmer => &["Noto Sans Khmer", "Noto Serif Khmer"],
        Script::Myanmar => &["Noto Sans Myanmar"],
        Script::Lao => &["Noto Sans Lao", "Noto Serif Lao"],
        Script::Tibetan => &["Noto Sans Tibetan"],
        _ => &[],
    }
}

/// A font database for discovering and loading system fonts.
///
/// Wraps `fontdb::Database` to provide font family lookup with bold/italic
/// matching and glyph-based fallback.
pub struct FontDatabase {
    db: fontdb::Database,
    /// Cache for fallback lookups: char → FontId.
    fallback_cache: Mutex<HashMap<char, Option<FontId>>>,
    /// Cache for substitution lookups: (family_lower, bold, italic) → `Option<FontId>`.
    substitution_cache: Mutex<HashMap<(String, bool, bool), Option<FontId>>>,
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
    /// On `wasm32` targets with the `embedded-font` feature enabled, the
    /// built-in Noto Sans font is loaded automatically so that basic text
    /// rendering works without any user-loaded fonts. Without the feature,
    /// the database starts empty. Use `load_font_data()` to add fonts manually.
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        #[cfg(not(target_arch = "wasm32"))]
        db.load_system_fonts();

        #[allow(unused_mut)]
        let mut font_db = Self {
            db,
            fallback_cache: Mutex::new(HashMap::new()),
            substitution_cache: Mutex::new(HashMap::new()),
        };

        // On WASM, load embedded fallback font if available
        #[cfg(all(target_arch = "wasm32", feature = "embedded-font"))]
        font_db.load_font_data(EMBEDDED_FONT.to_vec());

        font_db
    }

    /// Create an empty font database (no system fonts loaded).
    pub fn empty() -> Self {
        Self {
            db: fontdb::Database::new(),
            fallback_cache: Mutex::new(HashMap::new()),
            substitution_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Create a font database with an embedded fallback font.
    ///
    /// This loads a built-in font (Noto Sans) that provides broad Latin,
    /// Cyrillic, Greek, and many other script coverages. Useful for WASM
    /// environments where no system fonts are available, or for headless
    /// testing.
    ///
    /// # Errors
    ///
    /// This method is only available when the `embedded-font` feature is
    /// enabled (on by default).
    #[cfg(feature = "embedded-font")]
    pub fn with_embedded_fallback() -> Self {
        let mut db = Self::empty();
        db.load_font_data(EMBEDDED_FONT.to_vec());
        db
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

    /// Find a font by family, trying substitution table if exact match fails.
    ///
    /// This extends `find()` with a substitution table that maps common
    /// unavailable fonts to available alternatives (e.g., Calibri → Carlito).
    /// Results are cached to avoid repeated substitution table scans.
    pub fn find_with_substitution(&self, family: &str, bold: bool, italic: bool) -> Option<FontId> {
        // Try exact match first
        if let Some(id) = self.find(family, bold, italic) {
            return Some(id);
        }

        // Check substitution cache
        let cache_key = (family.to_lowercase(), bold, italic);
        if let Ok(cache) = self.substitution_cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return *cached;
            }
        }

        // Try substitution table
        let mut result = None;
        for &(src, alternatives) in FONT_SUBSTITUTIONS {
            if src.to_lowercase() == cache_key.0 {
                for &alt in alternatives {
                    if let Some(id) = self.find(alt, bold, italic) {
                        result = Some(id);
                        break;
                    }
                }
                break;
            }
        }

        // Cache the substitution result (even None to avoid repeated misses)
        if let Ok(mut cache) = self.substitution_cache.lock() {
            cache.insert(cache_key, result);
        }

        result
    }

    /// Find a fallback font that contains a glyph for the given character.
    ///
    /// Results are cached to avoid repeated O(n) linear scans.
    pub fn fallback(&self, ch: char) -> Option<FontId> {
        // Check cache
        if let Ok(cache) = self.fallback_cache.lock() {
            if let Some(&cached) = cache.get(&ch) {
                return cached;
            }
        }

        // Linear scan through all fonts
        let result = self.fallback_uncached(ch);

        // Cache the result
        if let Ok(mut cache) = self.fallback_cache.lock() {
            // Cap cache size to prevent unbounded memory growth
            if cache.len() > 50_000 {
                // Evict oldest half instead of clearing entire cache.
                // HashMap has no insertion order, so we remove an arbitrary
                // half of entries. This is cheaper than a full clear for
                // workloads that repeatedly query the same characters.
                let to_remove: Vec<_> = cache.keys().take(cache.len() / 2).cloned().collect();
                for key in to_remove {
                    cache.remove(&key);
                }
            }
            cache.insert(ch, result);
        }

        result
    }

    /// Find a fallback font for a character with script-aware ordering.
    ///
    /// Tries script-preferred fonts first (e.g., Noto Sans Arabic for Arabic
    /// characters) before falling back to a linear scan of all fonts.
    pub fn fallback_for_script(&self, ch: char, script: unicode_script::Script) -> Option<FontId> {
        // Check cache first
        if let Ok(cache) = self.fallback_cache.lock() {
            if let Some(&cached) = cache.get(&ch) {
                return cached;
            }
        }

        // Try script-preferred fonts first
        let preferred = script_preferred_fonts(script);
        for &family in preferred {
            if let Some(id) = self.find(family, false, false) {
                if let Some(font) = self.load_font(id) {
                    if font.has_glyph(ch) {
                        // Cache and return
                        if let Ok(mut cache) = self.fallback_cache.lock() {
                            if cache.len() > 10_000 {
                                cache.clear();
                            }
                            cache.insert(ch, Some(id));
                        }
                        return Some(id);
                    }
                }
            }
        }

        // Fall back to general scan
        let result = self.fallback_uncached(ch);
        if let Ok(mut cache) = self.fallback_cache.lock() {
            if cache.len() > 50_000 {
                // Evict oldest half instead of clearing entire cache.
                // HashMap has no insertion order, so we remove an arbitrary
                // half of entries. This is cheaper than a full clear for
                // workloads that repeatedly query the same characters.
                let to_remove: Vec<_> = cache.keys().take(cache.len() / 2).cloned().collect();
                for key in to_remove {
                    cache.remove(&key);
                }
            }
            cache.insert(ch, result);
        }
        result
    }

    /// Uncached fallback — linear scan through all fonts.
    fn fallback_uncached(&self, ch: char) -> Option<FontId> {
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

    #[test]
    fn substitution_finds_alternative() {
        let db = FontDatabase::new();
        // Calibri is unlikely to be on Linux/macOS, but one of its
        // alternatives (Helvetica, Arial, Liberation Sans) should be.
        let id = db.find_with_substitution("Calibri", false, false);
        // If Calibri itself exists, that's fine too; we just need a result
        // On CI/systems without fonts, substitution may not find anything;
        // only assert if the system has at least some fonts loaded.
        if db.len() > 0 {
            assert!(
                id.is_some() || db.find("Calibri", false, false).is_some(),
                "substitution should find an alternative for Calibri when fonts are available"
            );
        }
    }

    #[test]
    fn fallback_cache_works() {
        let db = FontDatabase::new();
        // First call — uncached
        let id1 = db.fallback('A');
        // Second call — should hit cache
        let id2 = db.fallback('A');
        assert_eq!(id1, id2);
    }

    #[test]
    fn script_preferred_fallback() {
        let db = FontDatabase::new();
        // This tests that the script-aware fallback path doesn't panic.
        // Whether it finds a font depends on the system.
        let _ = db.fallback_for_script('A', unicode_script::Script::Latin);
        let _ = db.fallback_for_script('你', unicode_script::Script::Han);
    }

    #[cfg(feature = "embedded-font")]
    #[test]
    fn embedded_font_loads() {
        let db = FontDatabase::with_embedded_fallback();
        assert!(db.len() > 0, "embedded font should load at least one face");
    }

    #[cfg(feature = "embedded-font")]
    #[test]
    fn embedded_font_has_basic_latin() {
        let db = FontDatabase::with_embedded_fallback();
        // The embedded Noto Sans font must contain basic Latin glyphs
        let id = db.fallback('A');
        assert!(id.is_some(), "embedded font should have glyph for 'A'");
        let id = db.fallback('z');
        assert!(id.is_some(), "embedded font should have glyph for 'z'");
        let id = db.fallback('0');
        assert!(id.is_some(), "embedded font should have glyph for '0'");
    }

    #[cfg(feature = "embedded-font")]
    #[test]
    fn embedded_font_findable_by_name() {
        let db = FontDatabase::with_embedded_fallback();
        // Should be findable as "Noto Sans"
        let id = db.find("Noto Sans", false, false);
        assert!(
            id.is_some(),
            "embedded font should be findable as 'Noto Sans'"
        );
    }

    #[cfg(feature = "embedded-font")]
    #[test]
    fn embedded_font_substitution_works() {
        let db = FontDatabase::with_embedded_fallback();
        // Calibri should substitute to Noto Sans (last entry in substitution list)
        let id = db.find_with_substitution("Calibri", false, false);
        assert!(
            id.is_some(),
            "Calibri should substitute to embedded Noto Sans"
        );
    }
}
