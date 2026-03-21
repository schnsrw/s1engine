//! Parse and write `xl/styles.xml`.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::model::*;
use crate::XlsxError;

/// Parse styles from `xl/styles.xml` XML content.
pub fn parse_styles(xml: &str) -> Result<StyleSheet, XlsxError> {
    let mut styles = StyleSheet::default();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    // Section flags
    let mut in_num_fmts = false;
    let mut in_fonts = false;
    let mut in_fills = false;
    let mut in_borders = false;
    let mut in_cell_xfs = false;

    // Current builders
    let mut cur_font: Option<FontDef> = None;
    let mut cur_fill: Option<FillDef> = None;
    let mut cur_border: Option<BorderDef> = None;
    let mut cur_border_side: Option<String> = None;
    let mut cur_border_style: Option<String> = None;
    let mut cur_xf: Option<CellFormat> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                handle_start_or_empty(
                    e,
                    false,
                    &mut styles,
                    &mut in_num_fmts,
                    &mut in_fonts,
                    &mut in_fills,
                    &mut in_borders,
                    &mut in_cell_xfs,
                    &mut cur_font,
                    &mut cur_fill,
                    &mut cur_border,
                    &mut cur_border_side,
                    &mut cur_border_style,
                    &mut cur_xf,
                );
            }
            Ok(Event::Empty(ref e)) => {
                handle_start_or_empty(
                    e,
                    true,
                    &mut styles,
                    &mut in_num_fmts,
                    &mut in_fonts,
                    &mut in_fills,
                    &mut in_borders,
                    &mut in_cell_xfs,
                    &mut cur_font,
                    &mut cur_fill,
                    &mut cur_border,
                    &mut cur_border_side,
                    &mut cur_border_style,
                    &mut cur_xf,
                );
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"numFmts" => in_num_fmts = false,
                    b"fonts" => in_fonts = false,
                    b"fills" => in_fills = false,
                    b"borders" => in_borders = false,
                    b"cellXfs" => in_cell_xfs = false,
                    b"font" if in_fonts => {
                        if let Some(font) = cur_font.take() {
                            styles.fonts.push(font);
                        }
                    }
                    b"fill" if in_fills => {
                        if let Some(fill) = cur_fill.take() {
                            styles.fills.push(fill);
                        }
                    }
                    b"border" if in_borders => {
                        if let Some(border) = cur_border.take() {
                            styles.borders.push(border);
                        }
                        cur_border_side = None;
                        cur_border_style = None;
                    }
                    b"left" | b"right" | b"top" | b"bottom" if cur_border.is_some() => {
                        // If the side had a style attribute but no <color> child,
                        // commit the border side now.
                        if let Some(style) = cur_border_style.take() {
                            if let (Some(ref mut border), Some(ref side)) =
                                (&mut cur_border, &cur_border_side)
                            {
                                let existing = match side.as_str() {
                                    "left" => border.left.is_some(),
                                    "right" => border.right.is_some(),
                                    "top" => border.top.is_some(),
                                    "bottom" => border.bottom.is_some(),
                                    _ => true,
                                };
                                if !existing {
                                    let bs = BorderSide { style, color: None };
                                    match side.as_str() {
                                        "left" => border.left = Some(bs),
                                        "right" => border.right = Some(bs),
                                        "top" => border.top = Some(bs),
                                        "bottom" => border.bottom = Some(bs),
                                        _ => {}
                                    }
                                }
                            }
                        }
                        cur_border_side = None;
                    }
                    b"xf" if in_cell_xfs => {
                        if let Some(xf) = cur_xf.take() {
                            styles.cell_formats.push(xf);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(styles)
}

/// Handle both `Start` and `Empty` XML events for style elements.
#[allow(clippy::too_many_arguments)]
fn handle_start_or_empty(
    e: &quick_xml::events::BytesStart<'_>,
    is_empty: bool,
    styles: &mut StyleSheet,
    in_num_fmts: &mut bool,
    in_fonts: &mut bool,
    in_fills: &mut bool,
    in_borders: &mut bool,
    in_cell_xfs: &mut bool,
    cur_font: &mut Option<FontDef>,
    cur_fill: &mut Option<FillDef>,
    cur_border: &mut Option<BorderDef>,
    cur_border_side: &mut Option<String>,
    cur_border_style: &mut Option<String>,
    cur_xf: &mut Option<CellFormat>,
) {
    let local = e.local_name();
    match local.as_ref() {
        // Section openers (only via Start)
        b"numFmts" if !is_empty => *in_num_fmts = true,
        b"fonts" if !is_empty => *in_fonts = true,
        b"fills" if !is_empty => *in_fills = true,
        b"borders" if !is_empty => *in_borders = true,
        b"cellXfs" if !is_empty => *in_cell_xfs = true,

        // Number format
        b"numFmt" if *in_num_fmts => {
            styles.number_formats.push(NumberFormat {
                id: attr_u32(e, b"numFmtId").unwrap_or(0),
                format_code: attr_str(e, b"formatCode").unwrap_or_default(),
            });
        }

        // Font elements
        b"font" if *in_fonts && !is_empty => {
            *cur_font = Some(FontDef::default());
        }
        b"sz" if cur_font.is_some() => {
            if let Some(ref mut f) = cur_font {
                f.size = attr_f64(e, b"val").unwrap_or(11.0);
            }
        }
        b"name" if cur_font.is_some() => {
            if let Some(ref mut f) = cur_font {
                f.name = attr_str(e, b"val").unwrap_or_default();
            }
        }
        b"b" if cur_font.is_some() => {
            if let Some(ref mut f) = cur_font {
                f.bold = attr_str(e, b"val").as_deref() != Some("0");
            }
        }
        b"i" if cur_font.is_some() => {
            if let Some(ref mut f) = cur_font {
                f.italic = attr_str(e, b"val").as_deref() != Some("0");
            }
        }
        b"color" if cur_font.is_some() && cur_border_side.is_none() => {
            if let Some(ref mut f) = cur_font {
                f.color = attr_str(e, b"rgb").or_else(|| attr_str(e, b"theme"));
            }
        }

        // Fill elements
        b"fill" if *in_fills && !is_empty => {
            *cur_fill = Some(FillDef::default());
        }
        b"patternFill" if cur_fill.is_some() => {
            if let Some(ref mut fill) = cur_fill {
                fill.pattern = attr_str(e, b"patternType").unwrap_or_default();
            }
        }
        b"fgColor" if cur_fill.is_some() => {
            if let Some(ref mut fill) = cur_fill {
                fill.fg_color = attr_str(e, b"rgb").or_else(|| attr_str(e, b"theme"));
            }
        }
        b"bgColor" if cur_fill.is_some() => {
            if let Some(ref mut fill) = cur_fill {
                fill.bg_color = attr_str(e, b"rgb").or_else(|| attr_str(e, b"theme"));
            }
        }

        // Border elements
        b"border" if *in_borders && !is_empty => {
            *cur_border = Some(BorderDef::default());
        }
        b"left" | b"right" | b"top" | b"bottom" if cur_border.is_some() => {
            let side_name = String::from_utf8_lossy(local.as_ref()).to_string();
            let style = attr_str(e, b"style");

            if is_empty {
                // Self-closing side: <left style="thin"/>
                if let Some(ref sty) = style {
                    if let Some(ref mut border) = cur_border {
                        let bs = BorderSide {
                            style: sty.clone(),
                            color: None,
                        };
                        match side_name.as_str() {
                            "left" => border.left = Some(bs),
                            "right" => border.right = Some(bs),
                            "top" => border.top = Some(bs),
                            "bottom" => border.bottom = Some(bs),
                            _ => {}
                        }
                    }
                }
            } else {
                // Opening tag: remember side+style for child <color>
                *cur_border_side = Some(side_name);
                *cur_border_style = style;
            }
        }
        b"color" if cur_border.is_some() && cur_border_side.is_some() => {
            let color = attr_str(e, b"rgb")
                .or_else(|| attr_str(e, b"indexed").map(|v| format!("indexed:{v}")));
            if let (Some(ref mut border), Some(ref side)) = (cur_border, cur_border_side) {
                let bs = BorderSide {
                    style: cur_border_style.take().unwrap_or_default(),
                    color,
                };
                match side.as_str() {
                    "left" => border.left = Some(bs),
                    "right" => border.right = Some(bs),
                    "top" => border.top = Some(bs),
                    "bottom" => border.bottom = Some(bs),
                    _ => {}
                }
            }
        }

        // cellXfs entries
        b"xf" if *in_cell_xfs => {
            let xf = CellFormat {
                number_format_id: attr_u32(e, b"numFmtId").unwrap_or(0),
                font_id: attr_u32(e, b"fontId").unwrap_or(0),
                fill_id: attr_u32(e, b"fillId").unwrap_or(0),
                border_id: attr_u32(e, b"borderId").unwrap_or(0),
                alignment: None,
            };
            if is_empty {
                styles.cell_formats.push(xf);
            } else {
                *cur_xf = Some(xf);
            }
        }
        b"alignment" if cur_xf.is_some() => {
            if let Some(ref mut xf) = cur_xf {
                xf.alignment = Some(CellAlignment {
                    horizontal: attr_str(e, b"horizontal"),
                    vertical: attr_str(e, b"vertical"),
                    wrap_text: attr_str(e, b"wrapText").as_deref() == Some("1"),
                });
            }
        }

        _ => {}
    }
}

/// Generate `xl/styles.xml` from the StyleSheet model.
pub fn write_styles(styles: &StyleSheet) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );

    // numFmts
    if !styles.number_formats.is_empty() {
        xml.push_str(&format!(
            r#"<numFmts count="{}">"#,
            styles.number_formats.len()
        ));
        for nf in &styles.number_formats {
            xml.push_str(&format!(
                r#"<numFmt numFmtId="{}" formatCode="{}"/>"#,
                nf.id,
                quick_xml::escape::escape(&nf.format_code)
            ));
        }
        xml.push_str("</numFmts>");
    }

    // fonts (minimum 1)
    let font_count = styles.fonts.len().max(1);
    xml.push_str(&format!(r#"<fonts count="{font_count}">"#));
    if styles.fonts.is_empty() {
        xml.push_str(r#"<font><sz val="11"/><name val="Calibri"/></font>"#);
    } else {
        for font in &styles.fonts {
            xml.push_str("<font>");
            if font.bold {
                xml.push_str("<b/>");
            }
            if font.italic {
                xml.push_str("<i/>");
            }
            xml.push_str(&format!(r#"<sz val="{}"/>"#, font.size));
            if let Some(ref c) = font.color {
                xml.push_str(&format!(
                    r#"<color rgb="{}"/>"#,
                    quick_xml::escape::escape(c)
                ));
            }
            xml.push_str(&format!(
                r#"<name val="{}"/>"#,
                quick_xml::escape::escape(&font.name)
            ));
            xml.push_str("</font>");
        }
    }
    xml.push_str("</fonts>");

    // fills (minimum 2 per OOXML)
    let fill_count = styles.fills.len().max(2);
    xml.push_str(&format!(r#"<fills count="{fill_count}">"#));
    if styles.fills.is_empty() {
        xml.push_str(r#"<fill><patternFill patternType="none"/></fill>"#);
        xml.push_str(r#"<fill><patternFill patternType="gray125"/></fill>"#);
    } else {
        for fill in &styles.fills {
            xml.push_str("<fill>");
            let pattern = if fill.pattern.is_empty() {
                "none"
            } else {
                &fill.pattern
            };
            if fill.fg_color.is_some() || fill.bg_color.is_some() {
                xml.push_str(&format!(r#"<patternFill patternType="{pattern}">"#));
                if let Some(ref fg) = fill.fg_color {
                    xml.push_str(&format!(
                        r#"<fgColor rgb="{}"/>"#,
                        quick_xml::escape::escape(fg)
                    ));
                }
                if let Some(ref bg) = fill.bg_color {
                    xml.push_str(&format!(
                        r#"<bgColor rgb="{}"/>"#,
                        quick_xml::escape::escape(bg)
                    ));
                }
                xml.push_str("</patternFill>");
            } else {
                xml.push_str(&format!(r#"<patternFill patternType="{pattern}"/>"#));
            }
            xml.push_str("</fill>");
        }
    }
    xml.push_str("</fills>");

    // borders (minimum 1)
    let border_count = styles.borders.len().max(1);
    xml.push_str(&format!(r#"<borders count="{border_count}">"#));
    if styles.borders.is_empty() {
        xml.push_str("<border><left/><right/><top/><bottom/><diagonal/></border>");
    } else {
        for border in &styles.borders {
            xml.push_str("<border>");
            write_border_side(&mut xml, "left", &border.left);
            write_border_side(&mut xml, "right", &border.right);
            write_border_side(&mut xml, "top", &border.top);
            write_border_side(&mut xml, "bottom", &border.bottom);
            xml.push_str("<diagonal/>");
            xml.push_str("</border>");
        }
    }
    xml.push_str("</borders>");

    // cellStyleXfs (minimal default)
    xml.push_str(
        r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
    );

    // cellXfs
    let xf_count = styles.cell_formats.len().max(1);
    xml.push_str(&format!(r#"<cellXfs count="{xf_count}">"#));
    if styles.cell_formats.is_empty() {
        xml.push_str(r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>"#);
    } else {
        for xf in &styles.cell_formats {
            xml.push_str(&format!(
                r#"<xf numFmtId="{}" fontId="{}" fillId="{}" borderId="{}" xfId="0""#,
                xf.number_format_id, xf.font_id, xf.fill_id, xf.border_id
            ));
            if let Some(ref align) = xf.alignment {
                xml.push_str(" applyAlignment=\"1\">");
                xml.push_str("<alignment");
                if let Some(ref h) = align.horizontal {
                    xml.push_str(&format!(r#" horizontal="{h}""#));
                }
                if let Some(ref v) = align.vertical {
                    xml.push_str(&format!(r#" vertical="{v}""#));
                }
                if align.wrap_text {
                    xml.push_str(r#" wrapText="1""#);
                }
                xml.push_str("/>");
                xml.push_str("</xf>");
            } else {
                xml.push_str("/>");
            }
        }
    }
    xml.push_str("</cellXfs>");

    xml.push_str("</styleSheet>");
    xml
}

fn write_border_side(xml: &mut String, name: &str, side: &Option<BorderSide>) {
    match side {
        Some(bs) => {
            if let Some(ref color) = bs.color {
                xml.push_str(&format!(r#"<{name} style="{}">"#, bs.style));
                xml.push_str(&format!(
                    r#"<color rgb="{}"/>"#,
                    quick_xml::escape::escape(color)
                ));
                xml.push_str(&format!("</{name}>"));
            } else {
                xml.push_str(&format!(r#"<{name} style="{}"/>"#, bs.style));
            }
        }
        None => {
            xml.push_str(&format!("<{name}/>"));
        }
    }
}

// ─── XML attribute helpers ───────────────────────────

fn attr_str(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == key {
            return attr.unescape_value().ok().map(|v| v.to_string());
        }
    }
    None
}

fn attr_f64(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<f64> {
    attr_str(e, key).and_then(|v| v.parse().ok())
}

fn attr_u32(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<u32> {
    attr_str(e, key).and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_complete_styles() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<numFmts count="1">
  <numFmt numFmtId="164" formatCode="#,##0.00"/>
</numFmts>
<fonts count="2">
  <font><sz val="11"/><name val="Calibri"/></font>
  <font><b/><i/><sz val="14"/><color rgb="FF0000FF"/><name val="Arial"/></font>
</fonts>
<fills count="3">
  <fill><patternFill patternType="none"/></fill>
  <fill><patternFill patternType="gray125"/></fill>
  <fill><patternFill patternType="solid"><fgColor rgb="FFFFFF00"/><bgColor rgb="FF000000"/></patternFill></fill>
</fills>
<borders count="2">
  <border><left/><right/><top/><bottom/><diagonal/></border>
  <border>
    <left style="thin"><color rgb="FF000000"/></left>
    <right style="thin"><color rgb="FF000000"/></right>
    <top style="medium"/>
    <bottom style="medium"/>
    <diagonal/>
  </border>
</borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="2">
  <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
  <xf numFmtId="164" fontId="1" fillId="2" borderId="1" xfId="0" applyAlignment="1">
    <alignment horizontal="center" vertical="top" wrapText="1"/>
  </xf>
</cellXfs>
</styleSheet>"##;

        let styles = parse_styles(xml).unwrap();

        // Number formats
        assert_eq!(styles.number_formats.len(), 1);
        assert_eq!(styles.number_formats[0].id, 164);
        assert_eq!(styles.number_formats[0].format_code, "#,##0.00");

        // Fonts
        assert_eq!(styles.fonts.len(), 2);
        assert_eq!(styles.fonts[0].name, "Calibri");
        assert!(!styles.fonts[0].bold);
        assert_eq!(styles.fonts[1].name, "Arial");
        assert!(styles.fonts[1].bold);
        assert!(styles.fonts[1].italic);
        assert_eq!(styles.fonts[1].size, 14.0);
        assert_eq!(styles.fonts[1].color.as_deref(), Some("FF0000FF"));

        // Fills
        assert_eq!(styles.fills.len(), 3);
        assert_eq!(styles.fills[0].pattern, "none");
        assert_eq!(styles.fills[2].pattern, "solid");
        assert_eq!(styles.fills[2].fg_color.as_deref(), Some("FFFFFF00"));
        assert_eq!(styles.fills[2].bg_color.as_deref(), Some("FF000000"));

        // Borders
        assert_eq!(styles.borders.len(), 2);
        assert!(styles.borders[0].left.is_none());
        let b1 = &styles.borders[1];
        assert_eq!(b1.left.as_ref().unwrap().style, "thin");
        assert_eq!(b1.left.as_ref().unwrap().color.as_deref(), Some("FF000000"));

        // Cell formats
        assert_eq!(styles.cell_formats.len(), 2);
        let xf1 = &styles.cell_formats[1];
        assert_eq!(xf1.number_format_id, 164);
        assert_eq!(xf1.font_id, 1);
        assert_eq!(xf1.fill_id, 2);
        assert_eq!(xf1.border_id, 1);
        let align = xf1.alignment.as_ref().unwrap();
        assert_eq!(align.horizontal.as_deref(), Some("center"));
        assert_eq!(align.vertical.as_deref(), Some("top"));
        assert!(align.wrap_text);
    }

    #[test]
    fn write_styles_roundtrip() {
        let styles = StyleSheet {
            number_formats: vec![NumberFormat {
                id: 164,
                format_code: "#,##0.00".to_string(),
            }],
            fonts: vec![
                FontDef {
                    name: "Calibri".to_string(),
                    size: 11.0,
                    bold: false,
                    italic: false,
                    color: None,
                },
                FontDef {
                    name: "Arial".to_string(),
                    size: 14.0,
                    bold: true,
                    italic: true,
                    color: Some("FF0000FF".to_string()),
                },
            ],
            fills: vec![
                FillDef {
                    pattern: "none".to_string(),
                    fg_color: None,
                    bg_color: None,
                },
                FillDef {
                    pattern: "gray125".to_string(),
                    fg_color: None,
                    bg_color: None,
                },
                FillDef {
                    pattern: "solid".to_string(),
                    fg_color: Some("FFFFFF00".to_string()),
                    bg_color: None,
                },
            ],
            borders: vec![BorderDef {
                left: None,
                right: None,
                top: Some(BorderSide {
                    style: "thin".to_string(),
                    color: Some("FF000000".to_string()),
                }),
                bottom: None,
            }],
            cell_formats: vec![
                CellFormat {
                    number_format_id: 0,
                    font_id: 0,
                    fill_id: 0,
                    border_id: 0,
                    alignment: None,
                },
                CellFormat {
                    number_format_id: 164,
                    font_id: 1,
                    fill_id: 2,
                    border_id: 0,
                    alignment: Some(CellAlignment {
                        horizontal: Some("center".to_string()),
                        vertical: None,
                        wrap_text: true,
                    }),
                },
            ],
        };

        let xml = write_styles(&styles);
        let parsed = parse_styles(&xml).unwrap();

        assert_eq!(parsed.number_formats.len(), 1);
        assert_eq!(parsed.number_formats[0].id, 164);

        assert_eq!(parsed.fonts.len(), 2);
        assert_eq!(parsed.fonts[0].name, "Calibri");
        assert!(parsed.fonts[1].bold);
        assert!(parsed.fonts[1].italic);

        assert_eq!(parsed.fills.len(), 3);
        assert_eq!(parsed.fills[2].fg_color.as_deref(), Some("FFFFFF00"));

        assert_eq!(parsed.borders.len(), 1);
        assert!(parsed.borders[0].top.is_some());

        assert_eq!(parsed.cell_formats.len(), 2);
        assert_eq!(parsed.cell_formats[1].number_format_id, 164);
        assert!(parsed.cell_formats[1].alignment.is_some());
    }

    #[test]
    fn write_default_styles() {
        let styles = StyleSheet::default();
        let xml = write_styles(&styles);
        let parsed = parse_styles(&xml).unwrap();
        // Should have default entries
        assert!(parsed.fonts.len() >= 1);
        assert!(parsed.fills.len() >= 2);
        assert!(parsed.borders.len() >= 1);
        assert!(parsed.cell_formats.len() >= 1);
    }
}
