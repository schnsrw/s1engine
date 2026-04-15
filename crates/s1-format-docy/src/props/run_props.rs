use crate::constants::*;
use crate::writer::DocyWriter;
use s1_model::{AttributeKey, AttributeMap, AttributeValue, UnderlineStyle, DocumentDefaults};

/// Write run properties (rPr) from an attribute map.
pub fn write(w: &mut DocyWriter, attrs: &AttributeMap) {
    // Bold
    if let Some(true) = attrs.get_bool(&AttributeKey::Bold) {
        w.write_prop_bool(rpr::BOLD, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::BoldCS) {
        w.write_prop_bool(rpr::BOLD_CS, true);
    }

    // Italic
    if let Some(true) = attrs.get_bool(&AttributeKey::Italic) {
        w.write_prop_bool(rpr::ITALIC, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::ItalicCS) {
        w.write_prop_bool(rpr::ITALIC_CS, true);
    }

    // Underline
    if let Some(AttributeValue::UnderlineStyle(us)) = attrs.get(&AttributeKey::Underline) {
        let val = match us {
            UnderlineStyle::None => 0,
            UnderlineStyle::Single => 1,
            UnderlineStyle::Double => 2,
            UnderlineStyle::Thick => 3,
            UnderlineStyle::Dotted => 4,
            UnderlineStyle::Dashed => 5,
            UnderlineStyle::Wave => 6,
            _ => 1,
        };
        w.write_prop_byte(rpr::UNDERLINE, val);
    }

    // Strikethrough
    if let Some(true) = attrs.get_bool(&AttributeKey::Strikethrough) {
        w.write_prop_bool(rpr::STRIKEOUT, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::DoubleStrikethrough) {
        w.write_prop_bool(rpr::DSTRIKEOUT, true);
    }

    // Font family
    if let Some(font) = attrs.get_string(&AttributeKey::FontFamily) {
        w.write_prop_string2(rpr::FONT_ASCII, font);
        w.write_prop_string2(rpr::FONT_HANSI, font);
    }
    if let Some(font) = attrs.get_string(&AttributeKey::FontFamilyEastAsia) {
        w.write_prop_string2(rpr::FONT_AE, font);
    }
    if let Some(font) = attrs.get_string(&AttributeKey::FontFamilyCS) {
        w.write_prop_string2(rpr::FONT_CS, font);
    }

    // Font size (points → half-points)
    if let Some(size) = attrs.get_f64(&AttributeKey::FontSize) {
        w.write_prop_long(rpr::FONT_SIZE, pts_to_half_pts(size) as u32);
    }
    if let Some(size) = attrs.get_f64(&AttributeKey::FontSizeCS) {
        w.write_prop_long(rpr::FONT_SIZE_CS, pts_to_half_pts(size) as u32);
    }

    // Color
    if let Some(AttributeValue::Color(c)) = attrs.get(&AttributeKey::Color) {
        w.write_item(rpr::COLOR, |w| {
            w.write_byte(color::RGB);
            w.write_color_rgb(c.r, c.g, c.b);
        });
    }

    // Highlight
    if let Some(AttributeValue::Color(c)) = attrs.get(&AttributeKey::HighlightColor) {
        w.write_item(rpr::HIGHLIGHT, |w| {
            w.write_byte(color::RGB);
            w.write_color_rgb(c.r, c.g, c.b);
        });
    }

    // Superscript / Subscript
    if let Some(true) = attrs.get_bool(&AttributeKey::Superscript) {
        w.write_prop_byte(rpr::VERT_ALIGN, 1); // superscript
    } else if let Some(true) = attrs.get_bool(&AttributeKey::Subscript) {
        w.write_prop_byte(rpr::VERT_ALIGN, 2); // subscript
    }

    // Caps / SmallCaps
    if let Some(true) = attrs.get_bool(&AttributeKey::Caps) {
        w.write_prop_bool(rpr::CAPS, true);
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::SmallCaps) {
        w.write_prop_bool(rpr::SMALL_CAPS, true);
    }

    // Hidden
    if let Some(true) = attrs.get_bool(&AttributeKey::Hidden) {
        w.write_prop_bool(rpr::VANISH, true);
    }

    // Font spacing (points → twips)
    if let Some(sp) = attrs.get_f64(&AttributeKey::FontSpacing) {
        w.write_prop_long_signed(rpr::SPACING, pts_to_twips(sp));
    }

    // Language
    if let Some(lang) = attrs.get_string(&AttributeKey::Language) {
        w.write_prop_string2(rpr::LANG, lang);
    }

    // Character style
    if let Some(style) = attrs.get_string(&AttributeKey::StyleId) {
        w.write_prop_string2(rpr::RSTYLE, style);
    }
}

/// Write default run properties from document defaults.
pub fn write_defaults(w: &mut DocyWriter, defaults: &DocumentDefaults) {
    if let Some(ref font) = defaults.font_family {
        w.write_prop_string2(rpr::FONT_ASCII, font);
        w.write_prop_string2(rpr::FONT_HANSI, font);
    }
    if let Some(size) = defaults.font_size {
        w.write_prop_long(rpr::FONT_SIZE, pts_to_half_pts(size) as u32);
    }
}
