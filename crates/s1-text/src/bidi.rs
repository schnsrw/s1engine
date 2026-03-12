//! Bidirectional text support via `unicode-bidi`.

use crate::types::{BidiRun, Direction};

/// Resolve bidirectional text runs using the Unicode BiDi algorithm (UAX #9).
///
/// Takes a string of text and returns a sequence of `BidiRun` values,
/// each describing a contiguous run of text with the same direction.
///
/// # Arguments
///
/// * `text` — The text to analyze.
///
/// # Returns
///
/// A vector of `BidiRun` values sorted by visual order. For pure LTR text,
/// this returns a single run covering the entire text.
pub fn bidi_resolve(text: &str) -> Vec<BidiRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let bidi_info = unicode_bidi::BidiInfo::new(text, None);

    let mut runs = Vec::new();

    for para in &bidi_info.paragraphs {
        let line = para.range.clone();
        let (_levels, visual_runs) = bidi_info.visual_runs(para, line);

        for run_range in visual_runs {
            // Get the embedding level for the first character in the run
            let level = bidi_info.levels[run_range.start];
            let direction = if level.is_rtl() {
                Direction::Rtl
            } else {
                Direction::Ltr
            };

            runs.push(BidiRun {
                start: run_range.start,
                end: run_range.end,
                direction,
                level: level.number(),
            });
        }
    }

    runs
}

/// Determine the base direction of a paragraph.
///
/// Uses the first strong character (L, R, or AL) to determine paragraph direction.
/// Falls back to LTR if no strong character is found.
pub fn paragraph_direction(text: &str) -> Direction {
    let bidi_info = unicode_bidi::BidiInfo::new(text, None);
    if let Some(para) = bidi_info.paragraphs.first() {
        if para.level.is_rtl() {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    } else {
        Direction::Ltr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bidi_pure_ltr() {
        let runs = bidi_resolve("Hello World");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].direction, Direction::Ltr);
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs[0].end, 11);
    }

    #[test]
    fn bidi_empty() {
        let runs = bidi_resolve("");
        assert!(runs.is_empty());
    }

    #[test]
    fn bidi_pure_rtl() {
        // Arabic text
        let text = "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}"; // العربية
        let runs = bidi_resolve(text);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].direction, Direction::Rtl);
    }

    #[test]
    fn bidi_mixed_ltr_rtl() {
        // English + Arabic
        let text = "Hello \u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629} World";
        let runs = bidi_resolve(text);
        // Should have multiple runs with different directions
        assert!(
            runs.len() >= 2,
            "expected multiple bidi runs, got {}",
            runs.len()
        );
        // At least one LTR and one RTL run
        let has_ltr = runs.iter().any(|r| r.direction == Direction::Ltr);
        let has_rtl = runs.iter().any(|r| r.direction == Direction::Rtl);
        assert!(has_ltr, "expected LTR run");
        assert!(has_rtl, "expected RTL run");
    }

    #[test]
    fn paragraph_direction_ltr() {
        assert_eq!(paragraph_direction("Hello"), Direction::Ltr);
    }

    #[test]
    fn paragraph_direction_rtl() {
        let text = "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}";
        assert_eq!(paragraph_direction(text), Direction::Rtl);
    }

    #[test]
    fn paragraph_direction_empty() {
        assert_eq!(paragraph_direction(""), Direction::Ltr);
    }

    #[test]
    fn bidi_run_levels() {
        let runs = bidi_resolve("Hello");
        assert_eq!(runs[0].level, 0); // LTR base level is 0
    }

    #[test]
    fn bidi_rtl_level() {
        let text = "\u{0627}\u{0644}\u{0639}"; // Arabic
        let runs = bidi_resolve(text);
        assert!(runs[0].level % 2 == 1, "RTL level should be odd");
    }
}
