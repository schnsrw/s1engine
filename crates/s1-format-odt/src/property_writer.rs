//! Write s1-model attributes as ODF style property elements.

use s1_model::{
    Alignment, AttributeKey, AttributeMap, AttributeValue, BorderStyle, LineSpacing, TabAlignment,
    TabLeader, UnderlineStyle, VerticalAlignment,
};

use crate::xml_util::{escape_xml, points_to_cm};

/// Generate `<style:text-properties .../>` from text-level attributes.
///
/// Returns an empty string if no text properties are present.
pub fn write_text_properties(attrs: &AttributeMap) -> String {
    let mut props = Vec::new();

    if let Some(true) = attrs.get_bool(&AttributeKey::Bold) {
        props.push(r#"fo:font-weight="bold""#.to_string());
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::Italic) {
        props.push(r#"fo:font-style="italic""#.to_string());
    }
    if let Some(size) = attrs.get_f64(&AttributeKey::FontSize) {
        props.push(format!(r#"fo:font-size="{size}pt""#));
    }
    if let Some(family) = attrs.get_string(&AttributeKey::FontFamily) {
        props.push(format!(r#"style:font-name="{}""#, escape_xml(family)));
    }
    if let Some(color) = attrs.get_color(&AttributeKey::Color) {
        props.push(format!("fo:color=\"#{}\"", color.to_hex()));
    }
    if let Some(AttributeValue::UnderlineStyle(style)) = attrs.get(&AttributeKey::Underline) {
        let val = match style {
            UnderlineStyle::Single | UnderlineStyle::Thick => "solid",
            UnderlineStyle::Double => "double",
            UnderlineStyle::Dotted => "dotted",
            UnderlineStyle::Dashed => "dash",
            UnderlineStyle::Wave => "wave",
            UnderlineStyle::None => "none",
        };
        if *style != UnderlineStyle::None {
            props.push(format!(r#"style:text-underline-style="{val}""#));
        }
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::Strikethrough) {
        props.push(r#"style:text-line-through-style="solid""#.to_string());
    }
    if let Some(color) = attrs.get_color(&AttributeKey::HighlightColor) {
        props.push(format!("fo:background-color=\"#{}\"", color.to_hex()));
    }
    // Superscript
    if let Some(true) = attrs.get_bool(&AttributeKey::Superscript) {
        props.push(r#"style:text-position="super 58%""#.to_string());
    }
    // Subscript
    if let Some(true) = attrs.get_bool(&AttributeKey::Subscript) {
        props.push(r#"style:text-position="sub 58%""#.to_string());
    }
    // Character spacing
    if let Some(pts) = attrs.get_f64(&AttributeKey::FontSpacing) {
        props.push(format!(r#"fo:letter-spacing="{}""#, points_to_cm(pts)));
    }

    if props.is_empty() {
        String::new()
    } else {
        format!("<style:text-properties {}/>", props.join(" "))
    }
}

/// Generate `<style:paragraph-properties .../>` from paragraph-level attributes.
///
/// Returns an empty string if no paragraph properties are present.
pub fn write_paragraph_properties(attrs: &AttributeMap) -> String {
    let mut props = Vec::new();

    if let Some(AttributeValue::Alignment(a)) = attrs.get(&AttributeKey::Alignment) {
        let val = match a {
            Alignment::Left => "start",
            Alignment::Center => "center",
            Alignment::Right => "end",
            Alignment::Justify => "justify",
        };
        props.push(format!(r#"fo:text-align="{val}""#));
    }
    if let Some(pts) = attrs.get_f64(&AttributeKey::SpacingBefore) {
        props.push(format!(r#"fo:margin-top="{}""#, points_to_cm(pts)));
    }
    if let Some(pts) = attrs.get_f64(&AttributeKey::SpacingAfter) {
        props.push(format!(r#"fo:margin-bottom="{}""#, points_to_cm(pts)));
    }
    if let Some(pts) = attrs.get_f64(&AttributeKey::IndentLeft) {
        props.push(format!(r#"fo:margin-left="{}""#, points_to_cm(pts)));
    }
    if let Some(pts) = attrs.get_f64(&AttributeKey::IndentRight) {
        props.push(format!(r#"fo:margin-right="{}""#, points_to_cm(pts)));
    }
    if let Some(pts) = attrs.get_f64(&AttributeKey::IndentFirstLine) {
        props.push(format!(r#"fo:text-indent="{}""#, points_to_cm(pts)));
    }
    if let Some(AttributeValue::LineSpacing(ls)) = attrs.get(&AttributeKey::LineSpacing) {
        match ls {
            LineSpacing::Multiple(m) => {
                let pct = m * 100.0;
                props.push(format!(r#"fo:line-height="{pct:.0}%""#));
            }
            LineSpacing::Exact(pts) => {
                props.push(format!(r#"fo:line-height="{}""#, points_to_cm(*pts)));
            }
            LineSpacing::Single => {
                props.push(r#"fo:line-height="100%""#.to_string());
            }
            LineSpacing::OnePointFive => {
                props.push(r#"fo:line-height="150%""#.to_string());
            }
            LineSpacing::Double => {
                props.push(r#"fo:line-height="200%""#.to_string());
            }
            LineSpacing::AtLeast(pts) => {
                props.push(format!(
                    r#"style:line-height-at-least="{}""#,
                    points_to_cm(*pts)
                ));
            }
        }
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::PageBreakBefore) {
        props.push(r#"fo:break-before="page""#.to_string());
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::KeepWithNext) {
        props.push(r#"fo:keep-with-next="always""#.to_string());
    }
    if let Some(true) = attrs.get_bool(&AttributeKey::KeepLinesTogether) {
        props.push(r#"fo:keep-together="always""#.to_string());
    }
    if let Some(color) = attrs.get_color(&AttributeKey::Background) {
        props.push(format!("fo:background-color=\"#{}\"", color.to_hex()));
    }

    // Paragraph borders
    if let Some(AttributeValue::Borders(borders)) = attrs.get(&AttributeKey::ParagraphBorders) {
        if let Some(ref side) = borders.top {
            props.push(format!(r#"fo:border-top="{}""#, border_side_to_odf(side)));
        }
        if let Some(ref side) = borders.bottom {
            props.push(format!(r#"fo:border-bottom="{}""#, border_side_to_odf(side)));
        }
        if let Some(ref side) = borders.left {
            props.push(format!(r#"fo:border-left="{}""#, border_side_to_odf(side)));
        }
        if let Some(ref side) = borders.right {
            props.push(format!(r#"fo:border-right="{}""#, border_side_to_odf(side)));
        }
    }

    // Tab stops — these require child elements
    let tab_stops_xml = if let Some(AttributeValue::TabStops(tabs)) = attrs.get(&AttributeKey::TabStops) {
        if tabs.is_empty() {
            String::new()
        } else {
            let mut xml = String::from("<style:tab-stops>");
            for ts in tabs {
                xml.push_str("<style:tab-stop");
                xml.push_str(&format!(r#" style:position="{}""#, points_to_cm(ts.position)));
                let tab_type = match ts.alignment {
                    TabAlignment::Left => "left",
                    TabAlignment::Center => "center",
                    TabAlignment::Right => "right",
                    TabAlignment::Decimal => "char",
                };
                xml.push_str(&format!(r#" style:type="{tab_type}""#));
                match ts.leader {
                    TabLeader::None => {}
                    TabLeader::Dot => xml.push_str(r#" style:leader-text=".""#),
                    TabLeader::Dash => xml.push_str(r#" style:leader-text="-""#),
                    TabLeader::Underscore => xml.push_str(r#" style:leader-text="_""#),
                }
                xml.push_str("/>");
            }
            xml.push_str("</style:tab-stops>");
            xml
        }
    } else {
        String::new()
    };

    if props.is_empty() && tab_stops_xml.is_empty() {
        String::new()
    } else if tab_stops_xml.is_empty() {
        format!("<style:paragraph-properties {}/>", props.join(" "))
    } else {
        format!(
            "<style:paragraph-properties {}>{}</style:paragraph-properties>",
            props.join(" "),
            tab_stops_xml
        )
    }
}

/// Convert a BorderSide to ODF border value like "0.06pt solid #000000".
fn border_side_to_odf(side: &s1_model::BorderSide) -> String {
    let style = match side.style {
        BorderStyle::None => "none",
        BorderStyle::Single => "solid",
        BorderStyle::Double => "double",
        BorderStyle::Dashed => "dashed",
        BorderStyle::Dotted => "dotted",
        BorderStyle::Thick => "solid",
    };
    format!("{:.2}pt {} #{}", side.width, style, side.color.to_hex())
}

/// Generate `<style:table-cell-properties .../>` from cell-level attributes.
///
/// Returns an empty string if no cell properties are present.
pub fn write_table_cell_properties(attrs: &AttributeMap) -> String {
    let mut props = Vec::new();

    if let Some(AttributeValue::VerticalAlignment(va)) = attrs.get(&AttributeKey::VerticalAlign) {
        let val = match va {
            VerticalAlignment::Top => "top",
            VerticalAlignment::Center => "middle",
            VerticalAlignment::Bottom => "bottom",
        };
        props.push(format!(r#"style:vertical-align="{val}""#));
    }
    if let Some(color) = attrs.get_color(&AttributeKey::CellBackground) {
        props.push(format!("fo:background-color=\"#{}\"", color.to_hex()));
    }

    if props.is_empty() {
        String::new()
    } else {
        format!("<style:table-cell-properties {}/>", props.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::Color;

    #[test]
    fn write_bold_italic() {
        let attrs = AttributeMap::new().bold(true).italic(true);
        let xml = write_text_properties(&attrs);
        assert!(xml.contains(r#"fo:font-weight="bold""#));
        assert!(xml.contains(r#"fo:font-style="italic""#));
    }

    #[test]
    fn write_font_size_and_family() {
        let attrs = AttributeMap::new().font_size(12.0).font_family("Arial");
        let xml = write_text_properties(&attrs);
        assert!(xml.contains(r#"fo:font-size="12pt""#));
        assert!(xml.contains(r#"style:font-name="Arial""#));
    }

    #[test]
    fn write_color() {
        let attrs = AttributeMap::new().color(Color::new(255, 0, 0));
        let xml = write_text_properties(&attrs);
        assert!(xml.contains("fo:color=\"#FF0000\""));
    }

    #[test]
    fn write_alignment_center() {
        let attrs = AttributeMap::new().alignment(Alignment::Center);
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains(r#"fo:text-align="center""#));
    }

    #[test]
    fn write_margins() {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::SpacingBefore, AttributeValue::Float(14.0));
        attrs.set(AttributeKey::SpacingAfter, AttributeValue::Float(7.0));
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains("fo:margin-top="));
        assert!(xml.contains("fo:margin-bottom="));
    }

    #[test]
    fn write_empty_attrs() {
        let attrs = AttributeMap::new();
        assert!(write_text_properties(&attrs).is_empty());
        assert!(write_paragraph_properties(&attrs).is_empty());
        assert!(write_table_cell_properties(&attrs).is_empty());
    }

    #[test]
    fn write_line_spacing_percent() {
        let mut attrs = AttributeMap::new();
        attrs.set(
            AttributeKey::LineSpacing,
            AttributeValue::LineSpacing(LineSpacing::Multiple(1.5)),
        );
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains(r#"fo:line-height="150%""#));
    }

    #[test]
    fn write_superscript() {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::Superscript, AttributeValue::Bool(true));
        let xml = write_text_properties(&attrs);
        assert!(xml.contains(r#"style:text-position="super 58%""#));
    }

    #[test]
    fn write_subscript() {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::Subscript, AttributeValue::Bool(true));
        let xml = write_text_properties(&attrs);
        assert!(xml.contains(r#"style:text-position="sub 58%""#));
    }

    #[test]
    fn write_character_spacing() {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::FontSpacing, AttributeValue::Float(2.0));
        let xml = write_text_properties(&attrs);
        assert!(xml.contains("fo:letter-spacing="));
    }

    #[test]
    fn write_paragraph_shading() {
        let mut attrs = AttributeMap::new();
        attrs.set(
            AttributeKey::Background,
            AttributeValue::Color(Color::new(255, 255, 0)),
        );
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains("fo:background-color=\"#FFFF00\""));
    }

    #[test]
    fn write_keep_lines_together() {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::KeepLinesTogether, AttributeValue::Bool(true));
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains(r#"fo:keep-together="always""#));
    }

    #[test]
    fn write_tab_stops() {
        use s1_model::{TabAlignment, TabLeader, TabStop};
        let mut attrs = AttributeMap::new();
        attrs.set(
            AttributeKey::TabStops,
            AttributeValue::TabStops(vec![
                TabStop { position: 72.0, alignment: TabAlignment::Left, leader: TabLeader::None },
                TabStop { position: 144.0, alignment: TabAlignment::Right, leader: TabLeader::Dot },
            ]),
        );
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains("<style:tab-stops>"));
        assert!(xml.contains(r#"style:type="left""#));
        assert!(xml.contains(r#"style:type="right""#));
        assert!(xml.contains(r#"style:leader-text=".""#));
        assert!(xml.contains("</style:tab-stops>"));
        // Should be a non-self-closing element
        assert!(xml.contains("</style:paragraph-properties>"));
    }

    #[test]
    fn write_paragraph_borders() {
        use s1_model::{BorderSide, BorderStyle, Borders};
        let mut attrs = AttributeMap::new();
        attrs.set(
            AttributeKey::ParagraphBorders,
            AttributeValue::Borders(Borders {
                top: Some(BorderSide { style: BorderStyle::Single, width: 1.0, color: Color::new(0, 0, 0), spacing: 0.0 }),
                bottom: Some(BorderSide { style: BorderStyle::Dashed, width: 0.5, color: Color::new(255, 0, 0), spacing: 0.0 }),
                left: None,
                right: None,
            }),
        );
        let xml = write_paragraph_properties(&attrs);
        assert!(xml.contains("fo:border-top="));
        assert!(xml.contains("fo:border-bottom="));
        assert!(!xml.contains("fo:border-left="));
        assert!(xml.contains("solid"));
        assert!(xml.contains("dashed"));
    }
}
