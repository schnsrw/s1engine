//! Parse ODF formatting properties into `AttributeMap`.

use quick_xml::events::BytesStart;
use s1_model::{
    Alignment, AttributeKey, AttributeMap, AttributeValue, BorderSide, BorderStyle, Borders, Color,
    LineSpacing, TabAlignment, TabLeader, TabStop, TextTransform, UnderlineStyle,
};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::xml_util::{get_attr, parse_length, parse_percentage};

/// Parse `<style:text-properties>` attributes into an `AttributeMap`.
pub fn parse_text_properties(e: &BytesStart<'_>) -> AttributeMap {
    let mut attrs = AttributeMap::new();

    // Bold: fo:font-weight="bold"
    if let Some(fw) = get_attr(e, b"font-weight") {
        attrs.set(AttributeKey::Bold, AttributeValue::Bool(fw == "bold"));
    }

    // Italic: fo:font-style="italic"
    if let Some(fs) = get_attr(e, b"font-style") {
        attrs.set(AttributeKey::Italic, AttributeValue::Bool(fs == "italic"));
    }

    // Font size: fo:font-size="12pt"
    if let Some(sz) = get_attr(e, b"font-size") {
        if let Some(pts) = parse_font_size(&sz) {
            attrs.set(AttributeKey::FontSize, AttributeValue::Float(pts));
        }
    }

    // Font family: style:font-name or fo:font-family
    if let Some(ff) = get_attr(e, b"font-name").or_else(|| get_attr(e, b"font-family")) {
        // Strip quotes if present
        let ff = ff.trim_matches('\'').trim_matches('"').to_string();
        attrs.set(AttributeKey::FontFamily, AttributeValue::String(ff));
    }

    // Color: fo:color="#FF0000"
    if let Some(c) = get_attr(e, b"color") {
        if let Some(color) = Color::from_hex(c.trim_start_matches('#')) {
            attrs.set(AttributeKey::Color, AttributeValue::Color(color));
        }
    }

    // Underline: style:text-underline-style="solid"
    if let Some(ul) = get_attr(e, b"text-underline-style") {
        let style = match ul.as_str() {
            "solid" => UnderlineStyle::Single,
            "double" => UnderlineStyle::Double,
            "dotted" => UnderlineStyle::Dotted,
            "dash" => UnderlineStyle::Dashed,
            "wave" => UnderlineStyle::Wave,
            "none" => UnderlineStyle::None,
            _ => UnderlineStyle::Single,
        };
        if style != UnderlineStyle::None {
            attrs.set(
                AttributeKey::Underline,
                AttributeValue::UnderlineStyle(style),
            );
        }
    }

    // Strikethrough: style:text-line-through-style="solid"
    if let Some(lt) = get_attr(e, b"text-line-through-style") {
        if lt != "none" {
            attrs.set(AttributeKey::Strikethrough, AttributeValue::Bool(true));
        }
    }

    // Highlight/background: fo:background-color="#FFFF00"
    if let Some(bg) = get_attr(e, b"background-color") {
        if bg != "transparent" {
            if let Some(color) = Color::from_hex(bg.trim_start_matches('#')) {
                attrs.set(AttributeKey::HighlightColor, AttributeValue::Color(color));
            }
        }
    }

    // Superscript/Subscript: style:text-position="super 58%" or "sub 58%" or "33% 58%"
    if let Some(tp) = get_attr(e, b"text-position") {
        let tp = tp.to_lowercase();
        if tp.starts_with("super") {
            attrs.set(AttributeKey::Superscript, AttributeValue::Bool(true));
        } else if tp.starts_with("sub") {
            attrs.set(AttributeKey::Subscript, AttributeValue::Bool(true));
        } else if let Some(pct_str) = tp.split_whitespace().next() {
            // Positive percentage = superscript, negative = subscript
            if let Ok(pct) = pct_str.trim_end_matches('%').parse::<f64>() {
                if pct > 0.0 {
                    attrs.set(AttributeKey::Superscript, AttributeValue::Bool(true));
                } else if pct < 0.0 {
                    attrs.set(AttributeKey::Subscript, AttributeValue::Bool(true));
                }
            }
        }
    }

    // Character spacing: fo:letter-spacing="0.1cm"
    if let Some(ls) = get_attr(e, b"letter-spacing") {
        if ls != "normal" {
            if let Some(pts) = parse_length(&ls) {
                attrs.set(AttributeKey::FontSpacing, AttributeValue::Float(pts));
            }
        }
    }

    // Text transform: fo:text-transform="uppercase"
    if let Some(tt) = get_attr(e, b"text-transform") {
        let transform = match tt.as_str() {
            "uppercase" => TextTransform::Uppercase,
            "lowercase" => TextTransform::Lowercase,
            "capitalize" => TextTransform::Capitalize,
            "none" => TextTransform::None,
            _ => TextTransform::None,
        };
        attrs.set(
            AttributeKey::TextTransformStyle,
            AttributeValue::TextTransform(transform),
        );
    }

    // Complex-script font family: style:font-name-complex
    if let Some(ff_cs) = get_attr(e, b"font-name-complex") {
        let ff_cs = ff_cs.trim_matches('\'').trim_matches('"').to_string();
        attrs.set(AttributeKey::FontFamilyCS, AttributeValue::String(ff_cs));
    }

    // Complex-script font size: fo:font-size-complex or style:font-size-asian
    if let Some(sz_cs) = get_attr(e, b"font-size-complex")
        .or_else(|| get_attr(e, b"font-size-asian"))
    {
        if let Some(pts) = parse_font_size(&sz_cs) {
            attrs.set(AttributeKey::FontSizeCS, AttributeValue::Float(pts));
        }
    }

    attrs
}

/// Parse `<style:paragraph-properties>` attributes.
pub fn parse_paragraph_properties(e: &BytesStart<'_>) -> AttributeMap {
    let mut attrs = AttributeMap::new();

    // Alignment: fo:text-align
    if let Some(align) = get_attr(e, b"text-align") {
        let a = match align.as_str() {
            "start" | "left" => Alignment::Left,
            "center" => Alignment::Center,
            "end" | "right" => Alignment::Right,
            "justify" => Alignment::Justify,
            _ => Alignment::Left,
        };
        attrs.set(AttributeKey::Alignment, AttributeValue::Alignment(a));
    }

    // Spacing before: fo:margin-top
    if let Some(mt) = get_attr(e, b"margin-top") {
        if let Some(pts) = parse_length(&mt) {
            attrs.set(AttributeKey::SpacingBefore, AttributeValue::Float(pts));
        }
    }

    // Spacing after: fo:margin-bottom
    if let Some(mb) = get_attr(e, b"margin-bottom") {
        if let Some(pts) = parse_length(&mb) {
            attrs.set(AttributeKey::SpacingAfter, AttributeValue::Float(pts));
        }
    }

    // Left indent: fo:margin-left
    if let Some(ml) = get_attr(e, b"margin-left") {
        if let Some(pts) = parse_length(&ml) {
            attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(pts));
        }
    }

    // Right indent: fo:margin-right
    if let Some(mr) = get_attr(e, b"margin-right") {
        if let Some(pts) = parse_length(&mr) {
            attrs.set(AttributeKey::IndentRight, AttributeValue::Float(pts));
        }
    }

    // First-line indent: fo:text-indent
    if let Some(ti) = get_attr(e, b"text-indent") {
        if let Some(pts) = parse_length(&ti) {
            attrs.set(AttributeKey::IndentFirstLine, AttributeValue::Float(pts));
        }
    }

    // Line spacing: fo:line-height
    if let Some(lh) = get_attr(e, b"line-height") {
        if let Some(pct) = parse_percentage(&lh) {
            attrs.set(
                AttributeKey::LineSpacing,
                AttributeValue::LineSpacing(LineSpacing::Multiple(pct)),
            );
        } else if let Some(pts) = parse_length(&lh) {
            attrs.set(
                AttributeKey::LineSpacing,
                AttributeValue::LineSpacing(LineSpacing::Exact(pts)),
            );
        }
    }

    // Page break before: fo:break-before="page"
    if let Some(bb) = get_attr(e, b"break-before") {
        if bb == "page" {
            attrs.set(AttributeKey::PageBreakBefore, AttributeValue::Bool(true));
        }
    }

    // Keep with next: fo:keep-with-next="always"
    if let Some(kwn) = get_attr(e, b"keep-with-next") {
        if kwn == "always" {
            attrs.set(AttributeKey::KeepWithNext, AttributeValue::Bool(true));
        }
    }

    // Keep lines together: fo:keep-together="always"
    if let Some(kt) = get_attr(e, b"keep-together") {
        if kt == "always" {
            attrs.set(AttributeKey::KeepLinesTogether, AttributeValue::Bool(true));
        }
    }

    // Widow control: fo:widows="2" → enable, "0" → disable
    // ODF has separate widows/orphans but OOXML merges them into one flag.
    // We treat either being > 0 as WidowControl(true).
    if let Some(w) = get_attr(e, b"widows") {
        if let Ok(val) = w.parse::<u32>() {
            attrs.set(
                AttributeKey::WidowControl,
                AttributeValue::Bool(val > 0),
            );
        }
    }
    // fo:orphans — same treatment; if widows already set this, orphans can override.
    if let Some(o) = get_attr(e, b"orphans") {
        if let Ok(val) = o.parse::<u32>() {
            attrs.set(
                AttributeKey::WidowControl,
                AttributeValue::Bool(val > 0),
            );
        }
    }

    // Paragraph background/shading: fo:background-color="#..."
    if let Some(bg) = get_attr(e, b"background-color") {
        if bg != "transparent" {
            if let Some(color) = Color::from_hex(bg.trim_start_matches('#')) {
                attrs.set(AttributeKey::Background, AttributeValue::Color(color));
            }
        }
    }

    // Paragraph borders: fo:border-top, fo:border-bottom, fo:border-left, fo:border-right
    // Also fo:border (shorthand for all sides)
    let top = get_attr(e, b"border-top").and_then(|v| parse_border_value(&v));
    let bottom = get_attr(e, b"border-bottom").and_then(|v| parse_border_value(&v));
    let left = get_attr(e, b"border-left").and_then(|v| parse_border_value(&v));
    let right = get_attr(e, b"border-right").and_then(|v| parse_border_value(&v));

    // Shorthand: fo:border applies to all sides
    let all = get_attr(e, b"border").and_then(|v| parse_border_value(&v));

    let borders = Borders {
        top: top.or_else(|| all.clone()),
        bottom: bottom.or_else(|| all.clone()),
        left: left.or_else(|| all.clone()),
        right: right.or(all),
    };

    if borders.top.is_some()
        || borders.bottom.is_some()
        || borders.left.is_some()
        || borders.right.is_some()
    {
        attrs.set(
            AttributeKey::ParagraphBorders,
            AttributeValue::Borders(borders),
        );
    }

    attrs
}

/// Parse an ODF border value like "0.06pt solid #000000" → BorderSide.
fn parse_border_value(s: &str) -> Option<BorderSide> {
    if s == "none" || s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let width = parse_length(parts[0]).unwrap_or(1.0);
    let style = match parts[1] {
        "solid" => BorderStyle::Single,
        "double" => BorderStyle::Double,
        "dashed" => BorderStyle::Dashed,
        "dotted" => BorderStyle::Dotted,
        "none" => BorderStyle::None,
        _ => BorderStyle::Single,
    };
    let color = Color::from_hex(parts[2].trim_start_matches('#')).unwrap_or(Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });

    Some(BorderSide {
        style,
        width,
        color,
        spacing: 0.0,
    })
}

/// Parse children of `<style:paragraph-properties>` for tab stops.
///
/// Call this after `parse_paragraph_properties` when the element was `Event::Start`
/// (not self-closing). Reads up to the closing `</style:paragraph-properties>`.
pub fn parse_paragraph_properties_children(reader: &mut Reader<&[u8]>, attrs: &mut AttributeMap) {
    let mut tab_stops: Vec<TabStop> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"tab-stops" => {
                // Parse tab-stop children
                loop {
                    match reader.read_event() {
                        Ok(Event::Empty(ref te)) if te.local_name().as_ref() == b"tab-stop" => {
                            if let Some(ts) = parse_tab_stop_element(te) {
                                tab_stops.push(ts);
                            }
                        }
                        Ok(Event::End(ref ee)) if ee.local_name().as_ref() == b"tab-stops" => break,
                        Ok(Event::Eof) => break,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"paragraph-properties" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
    }

    if !tab_stops.is_empty() {
        attrs.set(AttributeKey::TabStops, AttributeValue::TabStops(tab_stops));
    }
}

/// Parse a single `<style:tab-stop>` element.
fn parse_tab_stop_element(e: &BytesStart<'_>) -> Option<TabStop> {
    let position = get_attr(e, b"position").and_then(|v| parse_length(&v))?;
    let alignment = get_attr(e, b"type")
        .map(|v| match v.as_str() {
            "center" => TabAlignment::Center,
            "right" => TabAlignment::Right,
            "char" => TabAlignment::Decimal,
            _ => TabAlignment::Left,
        })
        .unwrap_or(TabAlignment::Left);
    let leader = get_attr(e, b"leader-text")
        .map(|v| match v.as_str() {
            "." => TabLeader::Dot,
            "-" => TabLeader::Dash,
            "_" => TabLeader::Underscore,
            _ => TabLeader::None,
        })
        .unwrap_or_else(|| {
            // Also check style:leader-style
            get_attr(e, b"leader-style")
                .map(|v| match v.as_str() {
                    "dotted" => TabLeader::Dot,
                    "dash" => TabLeader::Dash,
                    "solid" => TabLeader::Underscore,
                    _ => TabLeader::None,
                })
                .unwrap_or(TabLeader::None)
        });

    Some(TabStop {
        position,
        alignment,
        leader,
    })
}

/// Parse a font size string (e.g. "12pt", "16px", "1cm") to points.
fn parse_font_size(s: &str) -> Option<f64> {
    parse_length(s)
}

/// Parse `<style:graphic-properties>` attributes (image wrap, etc.).
pub fn parse_graphic_properties(e: &BytesStart<'_>) -> AttributeMap {
    let mut attrs = AttributeMap::new();

    // style:wrap → ImageWrapType
    if let Some(wrap) = get_attr(e, b"wrap") {
        let wrap_type = match wrap.as_str() {
            "none" => "none",
            "left" | "right" | "parallel" => "square",
            "dynamic" => "tight",
            "run-through" => "behind",
            _ => "none",
        };
        attrs.set(
            AttributeKey::ImageWrapType,
            AttributeValue::String(wrap_type.to_string()),
        );
    }

    attrs
}

/// Parse `<style:table-cell-properties>` attributes.
#[allow(dead_code)]
pub fn parse_table_cell_properties(e: &BytesStart<'_>) -> AttributeMap {
    let mut attrs = AttributeMap::new();

    // Vertical alignment
    if let Some(va) = get_attr(e, b"vertical-align") {
        let va_enum = match va.as_str() {
            "top" => s1_model::VerticalAlignment::Top,
            "middle" => s1_model::VerticalAlignment::Center,
            "bottom" => s1_model::VerticalAlignment::Bottom,
            _ => s1_model::VerticalAlignment::Top,
        };
        attrs.set(
            AttributeKey::VerticalAlign,
            AttributeValue::VerticalAlignment(va_enum),
        );
    }

    // Background color
    if let Some(bg) = get_attr(e, b"background-color") {
        if bg != "transparent" {
            if let Some(color) = Color::from_hex(bg.trim_start_matches('#')) {
                attrs.set(AttributeKey::CellBackground, AttributeValue::Color(color));
            }
        }
    }

    // Cell borders: fo:border-top, fo:border-bottom, fo:border-left, fo:border-right
    // Also fo:border (shorthand for all sides)
    let top = get_attr(e, b"border-top").and_then(|v| parse_border_value(&v));
    let bottom = get_attr(e, b"border-bottom").and_then(|v| parse_border_value(&v));
    let left = get_attr(e, b"border-left").and_then(|v| parse_border_value(&v));
    let right = get_attr(e, b"border-right").and_then(|v| parse_border_value(&v));

    // Shorthand: fo:border applies to all sides
    let all = get_attr(e, b"border").and_then(|v| parse_border_value(&v));

    let borders = Borders {
        top: top.or_else(|| all.clone()),
        bottom: bottom.or_else(|| all.clone()),
        left: left.or_else(|| all.clone()),
        right: right.or(all),
    };

    if borders.top.is_some()
        || borders.bottom.is_some()
        || borders.left.is_some()
        || borders.right.is_some()
    {
        attrs.set(
            AttributeKey::CellBorders,
            AttributeValue::Borders(borders),
        );
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    fn parse_text_attrs(xml: &str) -> AttributeMap {
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e))
                    if e.local_name().as_ref() == b"text-properties" =>
                {
                    return parse_text_properties(&e);
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        AttributeMap::new()
    }

    fn parse_para_attrs(xml: &str) -> AttributeMap {
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e))
                    if e.local_name().as_ref() == b"paragraph-properties" =>
                {
                    return parse_paragraph_properties(&e);
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        AttributeMap::new()
    }

    #[test]
    fn parse_bold_italic() {
        let attrs = parse_text_attrs(
            r#"<style:text-properties fo:font-weight="bold" fo:font-style="italic"/>"#,
        );
        assert_eq!(attrs.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(attrs.get_bool(&AttributeKey::Italic), Some(true));
    }

    #[test]
    fn parse_font_size_and_family() {
        let attrs = parse_text_attrs(
            r#"<style:text-properties fo:font-size="12pt" style:font-name="Arial"/>"#,
        );
        assert_eq!(attrs.get_f64(&AttributeKey::FontSize), Some(12.0));
        assert_eq!(attrs.get_string(&AttributeKey::FontFamily), Some("Arial"));
    }

    #[test]
    fn parse_color() {
        let attrs = parse_text_attrs("<style:text-properties fo:color=\"#ff0000\"/>");
        match attrs.get(&AttributeKey::Color) {
            Some(AttributeValue::Color(c)) => {
                assert_eq!(c.r, 255);
                assert_eq!(c.g, 0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn parse_underline() {
        let attrs =
            parse_text_attrs(r#"<style:text-properties style:text-underline-style="solid"/>"#);
        assert!(attrs.get(&AttributeKey::Underline).is_some());
    }

    #[test]
    fn parse_alignment() {
        let attrs = parse_para_attrs(r#"<style:paragraph-properties fo:text-align="center"/>"#);
        match attrs.get(&AttributeKey::Alignment) {
            Some(AttributeValue::Alignment(a)) => assert_eq!(*a, Alignment::Center),
            other => panic!("Expected Alignment, got {:?}", other),
        }
    }

    #[test]
    fn parse_spacing() {
        let attrs = parse_para_attrs(
            r#"<style:paragraph-properties fo:margin-top="0.5cm" fo:margin-bottom="0.3cm"/>"#,
        );
        let before = attrs.get_f64(&AttributeKey::SpacingBefore).unwrap();
        assert!((before - 14.173).abs() < 0.1); // 0.5cm in points
    }

    #[test]
    fn parse_indent() {
        let attrs = parse_para_attrs(
            r#"<style:paragraph-properties fo:margin-left="1in" fo:text-indent="0.5in"/>"#,
        );
        assert!((attrs.get_f64(&AttributeKey::IndentLeft).unwrap() - 72.0).abs() < 0.01);
        assert!((attrs.get_f64(&AttributeKey::IndentFirstLine).unwrap() - 36.0).abs() < 0.01);
    }

    #[test]
    fn parse_line_spacing_percent() {
        let attrs = parse_para_attrs(r#"<style:paragraph-properties fo:line-height="150%"/>"#);
        match attrs.get(&AttributeKey::LineSpacing) {
            Some(AttributeValue::LineSpacing(LineSpacing::Multiple(m))) => {
                assert!((*m - 1.5).abs() < 0.001);
            }
            other => panic!("Expected LineSpacing::Multiple, got {:?}", other),
        }
    }

    #[test]
    fn parse_superscript() {
        let attrs = parse_text_attrs(r#"<style:text-properties style:text-position="super 58%"/>"#);
        assert_eq!(attrs.get_bool(&AttributeKey::Superscript), Some(true));
        assert_eq!(attrs.get_bool(&AttributeKey::Subscript), None);
    }

    #[test]
    fn parse_subscript() {
        let attrs = parse_text_attrs(r#"<style:text-properties style:text-position="sub 58%"/>"#);
        assert_eq!(attrs.get_bool(&AttributeKey::Subscript), Some(true));
        assert_eq!(attrs.get_bool(&AttributeKey::Superscript), None);
    }

    #[test]
    fn parse_character_spacing() {
        let attrs = parse_text_attrs(r#"<style:text-properties fo:letter-spacing="0.1cm"/>"#);
        let pts = attrs.get_f64(&AttributeKey::FontSpacing).unwrap();
        assert!((pts - 2.835).abs() < 0.01); // 0.1cm ≈ 2.835pt
    }

    #[test]
    fn parse_paragraph_shading() {
        let attrs =
            parse_para_attrs(r##"<style:paragraph-properties fo:background-color="#FFFF00"/>"##);
        match attrs.get(&AttributeKey::Background) {
            Some(AttributeValue::Color(c)) => {
                assert_eq!(c.r, 255);
                assert_eq!(c.g, 255);
                assert_eq!(c.b, 0);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn parse_keep_lines_together() {
        let attrs = parse_para_attrs(r#"<style:paragraph-properties fo:keep-together="always"/>"#);
        assert_eq!(attrs.get_bool(&AttributeKey::KeepLinesTogether), Some(true));
    }

    #[test]
    fn parse_paragraph_border_all() {
        let attrs =
            parse_para_attrs(r#"<style:paragraph-properties fo:border="0.06pt solid #000000"/>"#);
        match attrs.get(&AttributeKey::ParagraphBorders) {
            Some(AttributeValue::Borders(borders)) => {
                assert!(borders.top.is_some());
                assert!(borders.bottom.is_some());
                assert!(borders.left.is_some());
                assert!(borders.right.is_some());
                let top = borders.top.as_ref().unwrap();
                assert_eq!(top.style, s1_model::BorderStyle::Single);
                assert_eq!(top.color.r, 0);
            }
            other => panic!("Expected Borders, got {:?}", other),
        }
    }

    #[test]
    fn parse_paragraph_border_partial() {
        let attrs = parse_para_attrs(
            r#"<style:paragraph-properties fo:border-top="1pt solid #FF0000" fo:border-bottom="0.5pt dashed #0000FF"/>"#,
        );
        match attrs.get(&AttributeKey::ParagraphBorders) {
            Some(AttributeValue::Borders(borders)) => {
                assert!(borders.top.is_some());
                assert!(borders.bottom.is_some());
                assert!(borders.left.is_none());
                let top = borders.top.as_ref().unwrap();
                assert!((top.width - 1.0).abs() < 0.01);
                assert_eq!(top.color.r, 255);
                let bottom = borders.bottom.as_ref().unwrap();
                assert_eq!(bottom.style, s1_model::BorderStyle::Dashed);
            }
            other => panic!("Expected Borders, got {:?}", other),
        }
    }

    #[test]
    fn parse_tab_stop_left() {
        use s1_model::TabAlignment;
        let xml = r#"<root xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="2.54cm" style:type="left"/></style:tab-stops></style:paragraph-properties></root>"#;
        let mut reader = Reader::from_str(xml);
        let mut attrs = AttributeMap::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"paragraph-properties" => {
                    attrs = parse_paragraph_properties(e);
                    parse_paragraph_properties_children(&mut reader, &mut attrs);
                    break;
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        match attrs.get(&AttributeKey::TabStops) {
            Some(AttributeValue::TabStops(tabs)) => {
                assert_eq!(tabs.len(), 1);
                assert!((tabs[0].position - 72.0).abs() < 0.1); // 2.54cm = 1 inch = 72pt
                assert_eq!(tabs[0].alignment, TabAlignment::Left);
            }
            other => panic!("Expected TabStops, got {:?}", other),
        }
    }

    #[test]
    fn parse_tab_stop_center_right() {
        use s1_model::TabAlignment;
        let xml = r#"<root xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="5cm" style:type="center"/><style:tab-stop style:position="10cm" style:type="right"/></style:tab-stops></style:paragraph-properties></root>"#;
        let mut reader = Reader::from_str(xml);
        let mut attrs = AttributeMap::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"paragraph-properties" => {
                    attrs = parse_paragraph_properties(e);
                    parse_paragraph_properties_children(&mut reader, &mut attrs);
                    break;
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        match attrs.get(&AttributeKey::TabStops) {
            Some(AttributeValue::TabStops(tabs)) => {
                assert_eq!(tabs.len(), 2);
                assert_eq!(tabs[0].alignment, TabAlignment::Center);
                assert_eq!(tabs[1].alignment, TabAlignment::Right);
            }
            other => panic!("Expected TabStops, got {:?}", other),
        }
    }

    #[test]
    fn parse_tab_stop_with_leader() {
        use s1_model::{TabAlignment, TabLeader};
        let xml = r#"<root xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="5cm" style:type="right" style:leader-text="."/></style:tab-stops></style:paragraph-properties></root>"#;
        let mut reader = Reader::from_str(xml);
        let mut attrs = AttributeMap::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"paragraph-properties" => {
                    attrs = parse_paragraph_properties(e);
                    parse_paragraph_properties_children(&mut reader, &mut attrs);
                    break;
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        match attrs.get(&AttributeKey::TabStops) {
            Some(AttributeValue::TabStops(tabs)) => {
                assert_eq!(tabs.len(), 1);
                assert_eq!(tabs[0].alignment, TabAlignment::Right);
                assert_eq!(tabs[0].leader, TabLeader::Dot);
            }
            other => panic!("Expected TabStops, got {:?}", other),
        }
    }
}
