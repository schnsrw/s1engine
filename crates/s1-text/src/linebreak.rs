//! Line break opportunities via `unicode-linebreak`.

use crate::types::BreakOpportunity;

/// Find all valid line break opportunities in a string.
///
/// Uses the Unicode Line Breaking Algorithm (UAX #14) to determine where
/// text may be broken across lines.
///
/// # Returns
///
/// A vector of `BreakOpportunity` values. Each opportunity indicates a byte
/// offset where a line break is allowed or mandatory.
pub fn line_break_opportunities(text: &str) -> Vec<BreakOpportunity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut opportunities = Vec::new();

    for (offset, break_class) in unicode_linebreak::linebreaks(text) {
        let mandatory = matches!(break_class, unicode_linebreak::BreakOpportunity::Mandatory);
        opportunities.push(BreakOpportunity { offset, mandatory });
    }

    opportunities
}

/// Check if a break is allowed at the given byte offset.
pub fn can_break_at(text: &str, offset: usize) -> bool {
    for (break_offset, _) in unicode_linebreak::linebreaks(text) {
        if break_offset == offset {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linebreak_empty() {
        let breaks = line_break_opportunities("");
        assert!(breaks.is_empty());
    }

    #[test]
    fn linebreak_single_word() {
        let breaks = line_break_opportunities("hello");
        // End of text is typically a mandatory break
        assert!(!breaks.is_empty());
        let last = breaks.last().unwrap();
        assert_eq!(last.offset, 5); // end of "hello"
        assert!(last.mandatory);
    }

    #[test]
    fn linebreak_space_separated() {
        let breaks = line_break_opportunities("hello world");
        // Should have a break opportunity after "hello " (at byte 6)
        let has_break_at_space = breaks.iter().any(|b| b.offset == 6 && !b.mandatory);
        assert!(
            has_break_at_space,
            "expected break after space, got: {breaks:?}"
        );
    }

    #[test]
    fn linebreak_multiple_words() {
        let breaks = line_break_opportunities("one two three");
        // Should have breaks after "one " and "two "
        let non_mandatory: Vec<_> = breaks.iter().filter(|b| !b.mandatory).collect();
        assert!(
            non_mandatory.len() >= 2,
            "expected at least 2 optional breaks, got: {non_mandatory:?}"
        );
    }

    #[test]
    fn linebreak_newline_is_mandatory() {
        let breaks = line_break_opportunities("line1\nline2");
        let mandatory: Vec<_> = breaks.iter().filter(|b| b.mandatory).collect();
        assert!(
            !mandatory.is_empty(),
            "expected mandatory break at newline"
        );
    }

    #[test]
    fn linebreak_hyphen() {
        let breaks = line_break_opportunities("well-known");
        // Break opportunity after the hyphen
        let break_after_hyphen = breaks.iter().any(|b| b.offset == 5);
        assert!(
            break_after_hyphen,
            "expected break after hyphen, got: {breaks:?}"
        );
    }

    #[test]
    fn can_break_at_space() {
        assert!(can_break_at("hello world", 6)); // after "hello "
    }

    #[test]
    fn cannot_break_mid_word() {
        // Should not break in the middle of "hello"
        assert!(!can_break_at("hello world", 3));
    }

    #[test]
    fn linebreak_unicode() {
        let breaks = line_break_opportunities("café latte");
        // Should have break opportunity at the space
        let has_break = breaks.iter().any(|b| !b.mandatory && b.offset > 0);
        assert!(has_break);
    }
}
