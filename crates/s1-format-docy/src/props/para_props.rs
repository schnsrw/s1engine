use crate::constants::*;
use crate::writer::DocyWriter;
use s1_model::{AttributeKey, AttributeMap, AttributeValue, Alignment, LineSpacing, DocumentDefaults};

/// Write paragraph properties (pPr) from an attribute map.
pub fn write(w: &mut DocyWriter, attrs: &AttributeMap) {
    // Alignment
    if let Some(AttributeValue::Alignment(a)) = attrs.get(&AttributeKey::Alignment) {
        let val = match a {
            Alignment::Right => align::RIGHT,
            Alignment::Left => align::LEFT,
            Alignment::Center => align::CENTER,
            Alignment::Justify => align::JUSTIFY,
            _ => align::LEFT, 
        };
        w.write_prop_byte(ppr::JC, val);
    }

    // Indentation (points → twips)
    if let Some(v) = attrs.get_f64(&AttributeKey::IndentLeft) {
        w.write_prop_long_signed(ppr::IND_LEFT, pts_to_twips(v));
    }
    if let Some(v) = attrs.get_f64(&AttributeKey::IndentRight) {
        w.write_prop_long_signed(ppr::IND_RIGHT, pts_to_twips(v));
    }
    if let Some(v) = attrs.get_f64(&AttributeKey::IndentFirstLine) {
        w.write_prop_long_signed(ppr::IND_FIRST_LINE, pts_to_twips(v));
    }

    // Spacing
    let has_spacing = attrs.get_f64(&AttributeKey::SpacingBefore).is_some()
        || attrs.get_f64(&AttributeKey::SpacingAfter).is_some()
        || attrs.get(&AttributeKey::LineSpacing).is_some();

    if has_spacing {
        w.write_item(ppr::SPACING, |w| {
            if let Some(v) = attrs.get_f64(&AttributeKey::SpacingBefore) {
                w.write_prop_long_signed(spacing::BEFORE, pts_to_twips(v));
            }
            if let Some(v) = attrs.get_f64(&AttributeKey::SpacingAfter) {
                w.write_prop_long_signed(spacing::AFTER, pts_to_twips(v));
            }
            if let Some(ls) = attrs.get_line_spacing(&AttributeKey::LineSpacing) {
                match ls {
                    LineSpacing::Single => {
                        w.write_prop_long(spacing::LINE, 240);
                        w.write_prop_byte(spacing::LINE_RULE, 0); // auto
                    }
                    LineSpacing::OnePointFive => {
                        w.write_prop_long(spacing::LINE, 360);
                        w.write_prop_byte(spacing::LINE_RULE, 0);
                    }
                    LineSpacing::Double => {
                        w.write_prop_long(spacing::LINE, 480);
                        w.write_prop_byte(spacing::LINE_RULE, 0);
                    }
                    LineSpacing::Multiple(v) => {
                        w.write_prop_long(spacing::LINE, (v * 240.0) as u32);
                        w.write_prop_byte(spacing::LINE_RULE, 0);
                    }
                    LineSpacing::Exact(v) => {
                        w.write_prop_long(spacing::LINE, pts_to_twips(v) as u32);
                        w.write_prop_byte(spacing::LINE_RULE, 1); // exact
                    }
                    LineSpacing::AtLeast(v) => {
                        w.write_prop_long(spacing::LINE, pts_to_twips(v) as u32);
                        w.write_prop_byte(spacing::LINE_RULE, 2); // atLeast
                    }
                    _ => { w.write_prop_long(spacing::LINE, 240); w.write_prop_byte(spacing::LINE_RULE, 0); }
                }
            }
        });
    }

    // Keep lines / keep next / page break before / widow control
    if let Some(true) = attrs.get_bool(&AttributeKey::KeepLinesTogether) {
        w.write_prop_bool(ppr::KEEP_LINES, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::KeepWithNext) {
        w.write_prop_bool(ppr::KEEP_NEXT, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::PageBreakBefore) {
        w.write_prop_bool(ppr::PAGE_BREAK_BEFORE, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::WidowControl) {
        w.write_prop_bool(ppr::WIDOW_CONTROL, true);
    }

    // Paragraph style
    if let Some(style) = attrs.get_string(&AttributeKey::StyleId) {
        w.write_prop_string2(ppr::PARA_STYLE, style);
    }

    // List/numbering
    if let Some(AttributeValue::ListInfo(li)) = attrs.get(&AttributeKey::ListInfo) {
        w.write_item(ppr::NUM_PR, |w| {
            w.write_prop_long(0, li.num_id); // NumId
            w.write_prop_long(1, li.level as u32); // Ilvl
        });
    }

    // Outline level
    if let Some(AttributeValue::Int(lvl)) = attrs.get(&AttributeKey::OutlineLevel) {
        if *lvl >= 0 && *lvl <= 8 {
            w.write_prop_byte(ppr::OUTLINE_LVL, *lvl as u8);
        }
    }

    // Bidi
    if let Some(true) = attrs.get_bool(&AttributeKey::Bidi) {
        w.write_prop_bool(ppr::BIDI, true);
    }

    // Contextual spacing
    if let Some(true) = attrs.get_bool(&AttributeKey::ContextualSpacing) {
        w.write_prop_bool(ppr::CONTEXTUAL_SPACING, true);
    }

    // Background/shading
    if let Some(AttributeValue::Color(c)) = attrs.get(&AttributeKey::Background) {
        w.write_item(ppr::SHD, |w| {
            w.write_byte(color::RGB);
            w.write_color_rgb(c.r, c.g, c.b);
        });
    }
}

/// Write default paragraph properties from document defaults.
pub fn write_defaults(w: &mut DocyWriter, defaults: &DocumentDefaults) {
    if let Some(v) = defaults.space_after {
        w.write_item(ppr::SPACING, |w| {
            w.write_prop_long_signed(spacing::AFTER, pts_to_twips(v));
        });
    }
    if let Some(v) = defaults.line_spacing_multiple {
        w.write_item(ppr::SPACING, |w| {
            w.write_prop_long(spacing::LINE, (v * 240.0) as u32);
            w.write_prop_byte(spacing::LINE_RULE, 0);
        });
    }
}
