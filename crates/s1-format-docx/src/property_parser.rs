//! Parse OOXML run properties (`w:rPr`) and paragraph properties (`w:pPr`).

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{
    Alignment, AttributeKey, AttributeMap, AttributeValue, BorderSide, BorderStyle, Borders, Color,
    LineSpacing, ListFormat, ListInfo, Margins, TabAlignment, TabLeader, TabStop, TableWidth,
    UnderlineStyle, VerticalAlignment,
};

use crate::error::DocxError;
use crate::xml_util::{get_attr, get_val, half_points_to_points, is_toggle_on, twips_to_points};

/// Parse `<w:rPr>` — run (character) formatting properties.
pub fn parse_run_properties(reader: &mut Reader<&[u8]>) -> Result<AttributeMap, DocxError> {
    let mut attrs = AttributeMap::new();
    parse_rpr_inner(reader, &mut attrs)?;
    Ok(attrs)
}

fn parse_rpr_inner(reader: &mut Reader<&[u8]>, attrs: &mut AttributeMap) -> Result<(), DocxError> {
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"rFonts" => {
                        // Font family: prefer w:ascii, then w:hAnsi
                        if let Some(font) = get_attr(&e, b"ascii")
                            .or_else(|| get_attr(&e, b"hAnsi"))
                        {
                            attrs.set(AttributeKey::FontFamily, AttributeValue::String(font));
                        }
                        if let Some(ea) = get_attr(&e, b"eastAsia") {
                            attrs.set(AttributeKey::FontFamilyEastAsia, AttributeValue::String(ea));
                        }
                        if let Some(cs) = get_attr(&e, b"cs") {
                            attrs.set(AttributeKey::FontFamilyCS, AttributeValue::String(cs.clone()));
                            // Fallback: if no ascii/hAnsi, use cs
                            if !attrs.contains(&AttributeKey::FontFamily) {
                                attrs.set(AttributeKey::FontFamily, AttributeValue::String(cs));
                            }
                        }
                        skip_to_end(reader)?;
                    }
                    b"rStyle" => {
                        if let Some(style_id) = get_val(&e) {
                            attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
                        }
                        skip_to_end(reader)?;
                    }
                    b"rPrChange" => {
                        // Track changes — format change revision
                        attrs.set(
                            AttributeKey::RevisionType,
                            AttributeValue::String("FormatChange".into()),
                        );
                        if let Some(id) = get_attr(&e, b"id") {
                            if let Ok(id_val) = id.parse::<i64>() {
                                attrs.set(AttributeKey::RevisionId, AttributeValue::Int(id_val));
                            }
                        }
                        if let Some(author) = get_attr(&e, b"author") {
                            attrs.set(AttributeKey::RevisionAuthor, AttributeValue::String(author));
                        }
                        if let Some(date) = get_attr(&e, b"date") {
                            attrs.set(AttributeKey::RevisionDate, AttributeValue::String(date));
                        }
                        // Capture the inner <w:rPr> (old formatting) as raw XML
                        // for round-trip fidelity.
                        let old_rpr_xml = capture_inner_xml(reader, b"rPrChange")?;
                        if !old_rpr_xml.is_empty() {
                            attrs.set(
                                AttributeKey::RevisionOriginalFormatting,
                                AttributeValue::String(old_rpr_xml),
                            );
                        }
                    }
                    _ => {
                        skip_to_end(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"b" => {
                        attrs.set(AttributeKey::Bold, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"i" => {
                        attrs.set(AttributeKey::Italic, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"strike" => {
                        attrs.set(
                            AttributeKey::Strikethrough,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"u" => {
                        let style = match get_val(&e).as_deref() {
                            Some("single") => UnderlineStyle::Single,
                            Some("double") => UnderlineStyle::Double,
                            Some("thick") => UnderlineStyle::Thick,
                            Some("dotted") => UnderlineStyle::Dotted,
                            Some("dash") | Some("dashed") => UnderlineStyle::Dashed,
                            Some("wave") => UnderlineStyle::Wave,
                            Some("none") => UnderlineStyle::None,
                            _ => UnderlineStyle::Single,
                        };
                        attrs.set(
                            AttributeKey::Underline,
                            AttributeValue::UnderlineStyle(style),
                        );
                    }
                    b"sz" => {
                        if let Some(val) = get_val(&e) {
                            if let Some(pts) = half_points_to_points(&val) {
                                attrs.set(AttributeKey::FontSize, AttributeValue::Float(pts));
                            }
                        }
                    }
                    b"color" => {
                        if let Some(hex) = get_val(&e) {
                            if hex != "auto" {
                                if let Some(color) = Color::from_hex(&hex) {
                                    attrs.set(AttributeKey::Color, AttributeValue::Color(color));
                                }
                            }
                        }
                        // Preserve theme color reference for round-trip
                        if let Some(tc) = get_attr(&e, b"themeColor") {
                            attrs.set(
                                AttributeKey::ThemeColor,
                                AttributeValue::String(tc),
                            );
                        }
                        if let Some(tint) = get_attr(&e, b"themeTint") {
                            attrs.set(
                                AttributeKey::ThemeTintShade,
                                AttributeValue::String(format!("tint:{tint}")),
                            );
                        } else if let Some(shade) = get_attr(&e, b"themeShade") {
                            attrs.set(
                                AttributeKey::ThemeTintShade,
                                AttributeValue::String(format!("shade:{shade}")),
                            );
                        }
                    }
                    b"highlight" => {
                        if let Some(color_name) = get_val(&e) {
                            if let Some(color) = highlight_name_to_color(&color_name) {
                                attrs.set(
                                    AttributeKey::HighlightColor,
                                    AttributeValue::Color(color),
                                );
                            }
                        }
                    }
                    b"shd" => {
                        // Run-level shading (arbitrary highlight color)
                        if let Some(fill) = get_attr(&e, b"fill") {
                            if fill != "auto" {
                                if let Some(color) = Color::from_hex(&fill) {
                                    attrs.set(
                                        AttributeKey::HighlightColor,
                                        AttributeValue::Color(color),
                                    );
                                }
                            }
                        }
                    }
                    b"rFonts" => {
                        if let Some(font) = get_attr(&e, b"ascii")
                            .or_else(|| get_attr(&e, b"hAnsi"))
                        {
                            attrs.set(AttributeKey::FontFamily, AttributeValue::String(font));
                        }
                        if let Some(ea) = get_attr(&e, b"eastAsia") {
                            attrs.set(AttributeKey::FontFamilyEastAsia, AttributeValue::String(ea));
                        }
                        if let Some(cs) = get_attr(&e, b"cs") {
                            attrs.set(AttributeKey::FontFamilyCS, AttributeValue::String(cs.clone()));
                            if !attrs.contains(&AttributeKey::FontFamily) {
                                attrs.set(AttributeKey::FontFamily, AttributeValue::String(cs));
                            }
                        }
                    }
                    b"vertAlign" => match get_val(&e).as_deref() {
                        Some("superscript") => {
                            attrs.set(AttributeKey::Superscript, AttributeValue::Bool(true));
                        }
                        Some("subscript") => {
                            attrs.set(AttributeKey::Subscript, AttributeValue::Bool(true));
                        }
                        _ => {}
                    },
                    b"rStyle" => {
                        if let Some(style_id) = get_val(&e) {
                            attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
                        }
                    }
                    b"spacing" => {
                        if let Some(val) = get_val(&e) {
                            if let Some(pts) = twips_to_points(&val) {
                                attrs.set(AttributeKey::FontSpacing, AttributeValue::Float(pts));
                            }
                        }
                    }
                    b"lang" => {
                        if let Some(lang) = get_val(&e) {
                            attrs.set(AttributeKey::Language, AttributeValue::String(lang));
                        }
                    }
                    b"shadow" => {
                        attrs.set(
                            AttributeKey::TextShadow,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"outline" => {
                        attrs.set(
                            AttributeKey::TextOutline,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"caps" => {
                        attrs.set(AttributeKey::Caps, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"smallCaps" => {
                        attrs.set(AttributeKey::SmallCaps, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"vanish" => {
                        attrs.set(AttributeKey::Hidden, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"dstrike" => {
                        attrs.set(
                            AttributeKey::DoubleStrikethrough,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"szCs" => {
                        if let Some(val) = get_val(&e) {
                            if let Some(pts) = half_points_to_points(&val) {
                                attrs.set(AttributeKey::FontSizeCS, AttributeValue::Float(pts));
                            }
                        }
                    }
                    b"bCs" => {
                        attrs.set(AttributeKey::BoldCS, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"iCs" => {
                        attrs.set(AttributeKey::ItalicCS, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"position" => {
                        if let Some(val) = get_val(&e) {
                            if let Ok(half_pts) = val.parse::<f64>() {
                                attrs.set(
                                    AttributeKey::BaselineShift,
                                    AttributeValue::Float(half_pts / 2.0),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"rPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Parse `<w:pPr>` — paragraph formatting properties.
pub fn parse_paragraph_properties(reader: &mut Reader<&[u8]>) -> Result<AttributeMap, DocxError> {
    let mut attrs = AttributeMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"pStyle" => {
                        if let Some(style_id) = get_val(&e) {
                            attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
                        }
                        skip_to_end(reader)?;
                    }
                    b"numPr" => {
                        if let Some(list_info) = parse_num_pr(reader)? {
                            attrs.set(AttributeKey::ListInfo, AttributeValue::ListInfo(list_info));
                        }
                    }
                    b"tabs" => {
                        let tab_stops = parse_tabs(reader)?;
                        if !tab_stops.is_empty() {
                            attrs.set(AttributeKey::TabStops, AttributeValue::TabStops(tab_stops));
                        }
                    }
                    b"pBdr" => {
                        let borders = parse_borders(reader, b"pBdr")?;
                        attrs.set(
                            AttributeKey::ParagraphBorders,
                            AttributeValue::Borders(borders),
                        );
                    }
                    b"rPr" => {
                        // Default run properties for the paragraph — skip for now
                        skip_to_end(reader)?;
                    }
                    b"pPrChange" => {
                        // Track changes — paragraph property change revision
                        attrs.set(
                            AttributeKey::RevisionType,
                            AttributeValue::String("PropertyChange".into()),
                        );
                        if let Some(id) = get_attr(&e, b"id") {
                            if let Ok(id_val) = id.parse::<i64>() {
                                attrs.set(AttributeKey::RevisionId, AttributeValue::Int(id_val));
                            }
                        }
                        if let Some(author) = get_attr(&e, b"author") {
                            attrs.set(AttributeKey::RevisionAuthor, AttributeValue::String(author));
                        }
                        if let Some(date) = get_attr(&e, b"date") {
                            attrs.set(AttributeKey::RevisionDate, AttributeValue::String(date));
                        }
                        // Capture the inner <w:pPr> (old formatting) as raw XML
                        let old_ppr_xml = capture_inner_xml(reader, b"pPrChange")?;
                        if !old_ppr_xml.is_empty() {
                            attrs.set(
                                AttributeKey::RevisionOriginalFormatting,
                                AttributeValue::String(old_ppr_xml),
                            );
                        }
                    }
                    _ => {
                        skip_to_end(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"jc" => {
                        let alignment = match get_val(&e).as_deref() {
                            Some("left") | Some("start") => Some(Alignment::Left),
                            Some("center") => Some(Alignment::Center),
                            Some("right") | Some("end") => Some(Alignment::Right),
                            Some("both") | Some("justify") => Some(Alignment::Justify),
                            _ => None,
                        };
                        if let Some(a) = alignment {
                            attrs.set(AttributeKey::Alignment, AttributeValue::Alignment(a));
                        }
                    }
                    b"pStyle" => {
                        if let Some(style_id) = get_val(&e) {
                            attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
                        }
                    }
                    b"spacing" => {
                        if let Some(before) = get_attr(&e, b"before") {
                            if let Some(pts) = twips_to_points(&before) {
                                attrs.set(AttributeKey::SpacingBefore, AttributeValue::Float(pts));
                            }
                        }
                        if let Some(after) = get_attr(&e, b"after") {
                            if let Some(pts) = twips_to_points(&after) {
                                attrs.set(AttributeKey::SpacingAfter, AttributeValue::Float(pts));
                            }
                        }
                        // Line spacing
                        if let Some(line) = get_attr(&e, b"line") {
                            let rule = get_attr(&e, b"lineRule");
                            if let Ok(line_val) = line.parse::<f64>() {
                                let spacing = match rule.as_deref() {
                                    Some("exact") => LineSpacing::Exact(line_val / 20.0),
                                    Some("atLeast") => LineSpacing::AtLeast(line_val / 20.0),
                                    _ => {
                                        // "auto" — value is in 240ths of a line
                                        let multiple = line_val / 240.0;
                                        if (multiple - 1.0).abs() < 0.01 {
                                            LineSpacing::Single
                                        } else if (multiple - 1.5).abs() < 0.01 {
                                            LineSpacing::OnePointFive
                                        } else if (multiple - 2.0).abs() < 0.01 {
                                            LineSpacing::Double
                                        } else {
                                            LineSpacing::Multiple(multiple)
                                        }
                                    }
                                };
                                attrs.set(
                                    AttributeKey::LineSpacing,
                                    AttributeValue::LineSpacing(spacing),
                                );
                            }
                        }
                    }
                    b"ind" => {
                        if let Some(left) = get_attr(&e, b"left") {
                            if let Some(pts) = twips_to_points(&left) {
                                attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(pts));
                            }
                        }
                        if let Some(right) = get_attr(&e, b"right") {
                            if let Some(pts) = twips_to_points(&right) {
                                attrs.set(AttributeKey::IndentRight, AttributeValue::Float(pts));
                            }
                        }
                        if let Some(first_line) = get_attr(&e, b"firstLine") {
                            if let Some(pts) = twips_to_points(&first_line) {
                                attrs
                                    .set(AttributeKey::IndentFirstLine, AttributeValue::Float(pts));
                            }
                        }
                        // Hanging indent: stored as negative first-line indent
                        if let Some(hanging) = get_attr(&e, b"hanging") {
                            if let Some(pts) = twips_to_points(&hanging) {
                                attrs.set(
                                    AttributeKey::IndentFirstLine,
                                    AttributeValue::Float(-pts),
                                );
                            }
                        }
                    }
                    b"keepNext" => {
                        attrs.set(
                            AttributeKey::KeepWithNext,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"keepLines" => {
                        attrs.set(
                            AttributeKey::KeepLinesTogether,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"pageBreakBefore" => {
                        attrs.set(
                            AttributeKey::PageBreakBefore,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"bidi" => {
                        attrs.set(AttributeKey::Bidi, AttributeValue::Bool(is_toggle_on(&e)));
                    }
                    b"suppressAutoHyphens" => {
                        attrs.set(
                            AttributeKey::SuppressAutoHyphens,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"contextualSpacing" => {
                        attrs.set(
                            AttributeKey::ContextualSpacing,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"wordWrap" => {
                        // wordWrap defaults to true; only explicit val="false"/val="0" disables
                        attrs.set(
                            AttributeKey::WordWrap,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"shd" => {
                        // Paragraph shading/background color
                        if let Some(fill) = get_attr(&e, b"fill") {
                            if fill != "auto" {
                                if let Some(color) = Color::from_hex(&fill) {
                                    attrs.set(
                                        AttributeKey::Background,
                                        AttributeValue::Color(color),
                                    );
                                }
                            }
                        }
                    }
                    b"widowControl" => {
                        // widowControl with no val or val="true" means enabled;
                        // explicit val="false"/val="0" means disabled
                        attrs.set(
                            AttributeKey::WidowControl,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    b"textDirection" => {
                        if let Some(val) = get_val(&e) {
                            let wm = match val.as_str() {
                                "lrTb" | "lrTbV" => s1_model::WritingMode::LrTb,
                                "rlTb" => s1_model::WritingMode::RlTb,
                                "tbRl" | "tbRlV" => s1_model::WritingMode::TbRl,
                                "tbLr" | "tbLrV" => s1_model::WritingMode::TbLr,
                                "btLr" => s1_model::WritingMode::BtLr,
                                _ => s1_model::WritingMode::LrTb,
                            };
                            attrs.set(
                                AttributeKey::ParagraphWritingMode,
                                AttributeValue::WritingMode(wm),
                            );
                        }
                    }
                    b"outlineLvl" => {
                        if let Some(val) = get_val(&e) {
                            if let Ok(level) = val.parse::<i64>() {
                                attrs.set(AttributeKey::OutlineLevel, AttributeValue::Int(level));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"pPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(attrs)
}

/// Parse `<w:tblPr>` — table formatting properties.
pub fn parse_table_properties(reader: &mut Reader<&[u8]>) -> Result<AttributeMap, DocxError> {
    let mut attrs = AttributeMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tblBorders" => {
                        let borders = parse_borders(reader, b"tblBorders")?;
                        attrs.set(AttributeKey::TableBorders, AttributeValue::Borders(borders));
                    }
                    b"tblCellMar" => {
                        let margins = parse_cell_margins(reader, b"tblCellMar")?;
                        attrs.set(
                            AttributeKey::TableDefaultCellMargins,
                            AttributeValue::Margins(margins),
                        );
                    }
                    b"tblPrChange" => {
                        // Track changes — table property change revision
                        attrs.set(
                            AttributeKey::RevisionType,
                            AttributeValue::String("PropertyChange".into()),
                        );
                        if let Some(id) = get_attr(&e, b"id") {
                            if let Ok(id_val) = id.parse::<i64>() {
                                attrs.set(AttributeKey::RevisionId, AttributeValue::Int(id_val));
                            }
                        }
                        if let Some(author) = get_attr(&e, b"author") {
                            attrs.set(AttributeKey::RevisionAuthor, AttributeValue::String(author));
                        }
                        if let Some(date) = get_attr(&e, b"date") {
                            attrs.set(AttributeKey::RevisionDate, AttributeValue::String(date));
                        }
                        // Capture the inner <w:tblPr> (old table properties) as raw XML
                        let old_tbl_xml = capture_inner_xml(reader, b"tblPrChange")?;
                        if !old_tbl_xml.is_empty() {
                            attrs.set(
                                AttributeKey::RevisionOriginalFormatting,
                                AttributeValue::String(old_tbl_xml),
                            );
                        }
                    }
                    _ => {
                        skip_to_end(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tblW" => {
                        if let Some(w) = parse_width(&e) {
                            attrs.set(AttributeKey::TableWidth, AttributeValue::TableWidth(w));
                        }
                    }
                    b"tblStyle" => {
                        if let Some(style_id) = get_val(&e) {
                            attrs.set(AttributeKey::StyleId, AttributeValue::String(style_id));
                        }
                    }
                    b"jc" => {
                        let alignment = match get_val(&e).as_deref() {
                            Some("left") | Some("start") => Some(Alignment::Left),
                            Some("center") => Some(Alignment::Center),
                            Some("right") | Some("end") => Some(Alignment::Right),
                            _ => None,
                        };
                        if let Some(a) = alignment {
                            attrs.set(AttributeKey::TableAlignment, AttributeValue::Alignment(a));
                        }
                    }
                    b"tblLayout" => {
                        let mode = match get_attr(&e, b"type").as_deref() {
                            Some("fixed") => s1_model::TableLayoutMode::Fixed,
                            _ => s1_model::TableLayoutMode::AutoFit,
                        };
                        attrs.set(
                            AttributeKey::TableLayout,
                            AttributeValue::TableLayoutMode(mode),
                        );
                    }
                    b"tblInd" => {
                        if let Some(w) = get_attr(&e, b"w") {
                            if let Some(pts) = twips_to_points(&w) {
                                attrs.set(AttributeKey::TableIndent, AttributeValue::Float(pts));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"tblPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(attrs)
}

/// Parse `<w:tcPr>` — table cell formatting properties.
pub fn parse_cell_properties(reader: &mut Reader<&[u8]>) -> Result<AttributeMap, DocxError> {
    let mut attrs = AttributeMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tcBorders" => {
                        let borders = parse_borders(reader, b"tcBorders")?;
                        attrs.set(AttributeKey::CellBorders, AttributeValue::Borders(borders));
                    }
                    b"tcMar" => {
                        let margins = parse_cell_margins(reader, b"tcMar")?;
                        attrs.set(
                            AttributeKey::CellPadding,
                            AttributeValue::Margins(margins),
                        );
                    }
                    b"vMerge" => {
                        // vMerge as non-self-closing element (with val attr)
                        let val = get_val(&e);
                        let merge_val = match val.as_deref() {
                            Some("restart") => "restart",
                            _ => "continue",
                        };
                        attrs.set(
                            AttributeKey::RowSpan,
                            AttributeValue::String(merge_val.to_string()),
                        );
                        skip_to_end(reader)?;
                    }
                    b"tcPrChange" => {
                        // Track changes — table cell property change revision
                        attrs.set(
                            AttributeKey::RevisionType,
                            AttributeValue::String("PropertyChange".into()),
                        );
                        if let Some(id) = get_attr(&e, b"id") {
                            if let Ok(id_val) = id.parse::<i64>() {
                                attrs.set(AttributeKey::RevisionId, AttributeValue::Int(id_val));
                            }
                        }
                        if let Some(author) = get_attr(&e, b"author") {
                            attrs.set(AttributeKey::RevisionAuthor, AttributeValue::String(author));
                        }
                        if let Some(date) = get_attr(&e, b"date") {
                            attrs.set(AttributeKey::RevisionDate, AttributeValue::String(date));
                        }
                        // Capture the inner <w:tcPr> (old cell properties) as raw XML
                        let old_tcp_xml = capture_inner_xml(reader, b"tcPrChange")?;
                        if !old_tcp_xml.is_empty() {
                            attrs.set(
                                AttributeKey::RevisionOriginalFormatting,
                                AttributeValue::String(old_tcp_xml),
                            );
                        }
                    }
                    _ => {
                        skip_to_end(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tcW" => {
                        if let Some(w) = parse_width(&e) {
                            attrs.set(AttributeKey::CellWidth, AttributeValue::TableWidth(w));
                        }
                    }
                    b"gridSpan" => {
                        if let Some(val) = get_val(&e) {
                            if let Ok(span) = val.parse::<i64>() {
                                attrs.set(AttributeKey::ColSpan, AttributeValue::Int(span));
                            }
                        }
                    }
                    b"vMerge" => {
                        // vMerge with val="restart" starts a merge; empty vMerge continues it
                        let val = get_val(&e);
                        let merge_val = match val.as_deref() {
                            Some("restart") => "restart",
                            _ => "continue",
                        };
                        attrs.set(
                            AttributeKey::RowSpan,
                            AttributeValue::String(merge_val.to_string()),
                        );
                    }
                    b"vAlign" => {
                        let valign = match get_val(&e).as_deref() {
                            Some("top") => Some(VerticalAlignment::Top),
                            Some("center") => Some(VerticalAlignment::Center),
                            Some("bottom") => Some(VerticalAlignment::Bottom),
                            _ => None,
                        };
                        if let Some(va) = valign {
                            attrs.set(
                                AttributeKey::VerticalAlign,
                                AttributeValue::VerticalAlignment(va),
                            );
                        }
                    }
                    b"shd" => {
                        // Cell shading/background color
                        if let Some(fill) = get_attr(&e, b"fill") {
                            if fill != "auto" {
                                if let Some(color) = Color::from_hex(&fill) {
                                    attrs.set(
                                        AttributeKey::CellBackground,
                                        AttributeValue::Color(color),
                                    );
                                }
                            }
                        }
                    }
                    b"textDirection" => {
                        if let Some(val) = get_val(&e) {
                            attrs.set(
                                AttributeKey::CellTextDirection,
                                AttributeValue::String(val),
                            );
                        }
                    }
                    b"noWrap" => {
                        attrs.set(
                            AttributeKey::CellNoWrap,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"tcPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(attrs)
}

/// Parse `<w:trPr>` — table row formatting properties.
pub fn parse_row_properties(reader: &mut Reader<&[u8]>) -> Result<AttributeMap, DocxError> {
    let mut attrs = AttributeMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"trPrChange" => {
                        // Track changes — table row property change revision
                        attrs.set(
                            AttributeKey::RevisionType,
                            AttributeValue::String("PropertyChange".into()),
                        );
                        if let Some(id) = get_attr(&e, b"id") {
                            if let Ok(id_val) = id.parse::<i64>() {
                                attrs.set(AttributeKey::RevisionId, AttributeValue::Int(id_val));
                            }
                        }
                        if let Some(author) = get_attr(&e, b"author") {
                            attrs.set(AttributeKey::RevisionAuthor, AttributeValue::String(author));
                        }
                        if let Some(date) = get_attr(&e, b"date") {
                            attrs.set(AttributeKey::RevisionDate, AttributeValue::String(date));
                        }
                        // Capture the inner <w:trPr> (old row properties) as raw XML
                        let old_trp_xml = capture_inner_xml(reader, b"trPrChange")?;
                        if !old_trp_xml.is_empty() {
                            attrs.set(
                                AttributeKey::RevisionOriginalFormatting,
                                AttributeValue::String(old_trp_xml),
                            );
                        }
                    }
                    _ => {
                        skip_to_end(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"tblHeader" => {
                        attrs.set(AttributeKey::TableHeaderRow, AttributeValue::Bool(true));
                    }
                    b"trHeight" => {
                        if let Some(val) = get_val(&e) {
                            if let Some(pts) = twips_to_points(&val) {
                                attrs.set(AttributeKey::RowHeight, AttributeValue::Float(pts));
                            }
                        }
                        if let Some(rule) = get_attr(&e, b"hRule") {
                            attrs.set(
                                AttributeKey::RowHeightRule,
                                AttributeValue::String(rule),
                            );
                        }
                    }
                    b"cantSplit" => {
                        attrs.set(
                            AttributeKey::RowNoSplit,
                            AttributeValue::Bool(is_toggle_on(&e)),
                        );
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"trPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(attrs)
}

/// Parse spacing attributes from a `w:spacing` element.
pub fn parse_spacing_attrs(e: &quick_xml::events::BytesStart<'_>, attrs: &mut AttributeMap) {
    if let Some(before) = get_attr(e, b"before") {
        if let Some(pts) = twips_to_points(&before) {
            attrs.set(AttributeKey::SpacingBefore, AttributeValue::Float(pts));
        }
    }
    if let Some(after) = get_attr(e, b"after") {
        if let Some(pts) = twips_to_points(&after) {
            attrs.set(AttributeKey::SpacingAfter, AttributeValue::Float(pts));
        }
    }
    if let Some(line) = get_attr(e, b"line") {
        let rule = get_attr(e, b"lineRule");
        if let Ok(line_val) = line.parse::<f64>() {
            let spacing = match rule.as_deref() {
                Some("exact") => LineSpacing::Exact(line_val / 20.0),
                Some("atLeast") => LineSpacing::AtLeast(line_val / 20.0),
                _ => {
                    let multiple = line_val / 240.0;
                    if (multiple - 1.0).abs() < 0.01 {
                        LineSpacing::Single
                    } else if (multiple - 1.5).abs() < 0.01 {
                        LineSpacing::OnePointFive
                    } else if (multiple - 2.0).abs() < 0.01 {
                        LineSpacing::Double
                    } else {
                        LineSpacing::Multiple(multiple)
                    }
                }
            };
            attrs.set(
                AttributeKey::LineSpacing,
                AttributeValue::LineSpacing(spacing),
            );
        }
    }
}

/// Parse indent attributes from a `w:ind` element.
pub fn parse_indent_attrs(e: &quick_xml::events::BytesStart<'_>, attrs: &mut AttributeMap) {
    if let Some(left) = get_attr(e, b"left") {
        if let Some(pts) = twips_to_points(&left) {
            attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(pts));
        }
    }
    if let Some(right) = get_attr(e, b"right") {
        if let Some(pts) = twips_to_points(&right) {
            attrs.set(AttributeKey::IndentRight, AttributeValue::Float(pts));
        }
    }
    if let Some(first_line) = get_attr(e, b"firstLine") {
        if let Some(pts) = twips_to_points(&first_line) {
            attrs.set(AttributeKey::IndentFirstLine, AttributeValue::Float(pts));
        }
    }
    // Hanging indent: stored as negative first-line indent
    if let Some(hanging) = get_attr(e, b"hanging") {
        if let Some(pts) = twips_to_points(&hanging) {
            attrs.set(
                AttributeKey::IndentFirstLine,
                AttributeValue::Float(-pts),
            );
        }
    }
}

/// Parse `<w:numPr>` — list numbering reference on a paragraph.
///
/// Contains `<w:ilvl w:val="0"/>` and `<w:numId w:val="1"/>`.
/// Returns a `ListInfo` with `num_format` set to `Decimal` as a placeholder
/// (the caller resolves the actual format from numbering definitions).
pub fn parse_num_pr(reader: &mut Reader<&[u8]>) -> Result<Option<ListInfo>, DocxError> {
    let mut num_id: Option<u32> = None;
    let mut level: u8 = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"ilvl" => {
                    if let Some(v) = get_val(&e) {
                        level = v.parse().unwrap_or(0);
                    }
                }
                b"numId" => {
                    if let Some(v) = get_val(&e) {
                        num_id = v.parse().ok();
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.local_name().as_ref() == b"numPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    // numId=0 means "no list" in OOXML
    match num_id {
        Some(id) if id > 0 => Ok(Some(ListInfo {
            level,
            num_format: ListFormat::Decimal, // placeholder, resolved later
            num_id: id,
            start: None,
        })),
        _ => Ok(None),
    }
}

/// Parse `<w:tabs>` — tab stop definitions in paragraph properties.
/// Parse `<w:tabs>` children into a list of TabStop values.
pub fn parse_tabs_pub(reader: &mut Reader<&[u8]>) -> Result<Vec<TabStop>, DocxError> {
    parse_tabs(reader)
}

fn parse_tabs(reader: &mut Reader<&[u8]>) -> Result<Vec<TabStop>, DocxError> {
    let mut stops = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) if e.local_name().as_ref() == b"tab" => {
                let pos = get_attr(&e, b"pos")
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(|v| v / 20.0) // twips to points
                    .unwrap_or(0.0);

                let alignment = match get_val(&e).as_deref() {
                    Some("center") => TabAlignment::Center,
                    Some("right") => TabAlignment::Right,
                    Some("decimal") => TabAlignment::Decimal,
                    _ => TabAlignment::Left,
                };

                let leader = match get_attr(&e, b"leader").as_deref() {
                    Some("dot") => TabLeader::Dot,
                    Some("hyphen") | Some("dash") => TabLeader::Dash,
                    Some("underscore") | Some("heavy") => TabLeader::Underscore,
                    _ => TabLeader::None,
                };

                stops.push(TabStop {
                    position: pos,
                    alignment,
                    leader,
                });
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"tabs" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(stops)
}

/// Parse a width element (`<w:tblW>` or `<w:tcW>`).
///
/// OOXML width types: "auto", "dxa" (twips), "pct" (fiftieths of a percent).
fn parse_width(e: &quick_xml::events::BytesStart<'_>) -> Option<TableWidth> {
    let w_val = get_attr(e, b"w")?;
    let w_type = get_attr(e, b"type").unwrap_or_default();

    match w_type.as_str() {
        "auto" | "nil" | "" => Some(TableWidth::Auto),
        "dxa" => {
            let twips = w_val.parse::<f64>().ok()?;
            Some(TableWidth::Fixed(twips / 20.0))
        }
        "pct" => {
            // Value is in fiftieths of a percent (e.g. 5000 = 100%)
            let pct_50 = w_val.parse::<f64>().ok()?;
            Some(TableWidth::Percent(pct_50 / 50.0))
        }
        _ => Some(TableWidth::Auto),
    }
}

/// Parse a cell margins element (`<w:tblCellMar>` or `<w:tcMar>`).
/// Each child (top/bottom/start/left/end/right) has `w:w` in twips and `w:type`.
fn parse_cell_margins(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<Margins, DocxError> {
    let mut margins = Margins::ZERO;

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                let pts = get_attr(&e, b"w")
                    .and_then(|s| twips_to_points(&s))
                    .unwrap_or(0.0);
                match name.as_slice() {
                    b"top" => margins.top = pts,
                    b"bottom" => margins.bottom = pts,
                    b"left" | b"start" => margins.left = pts,
                    b"right" | b"end" => margins.right = pts,
                    _ => {}
                }
            }
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                let pts = get_attr(&e, b"w")
                    .and_then(|s| twips_to_points(&s))
                    .unwrap_or(0.0);
                match name.as_slice() {
                    b"top" => margins.top = pts,
                    b"bottom" => margins.bottom = pts,
                    b"left" | b"start" => margins.left = pts,
                    b"right" | b"end" => margins.right = pts,
                    _ => {}
                }
                skip_to_end(reader)?;
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(margins)
}

/// Parse a borders element (`<w:tblBorders>` or `<w:tcBorders>`).
/// Parse border elements (top/bottom/left/right) until the given end tag.
pub fn parse_borders(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<Borders, DocxError> {
    let mut borders = Borders {
        top: None,
        bottom: None,
        left: None,
        right: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                let side = parse_border_side(&e);
                match name.as_slice() {
                    b"top" => borders.top = side,
                    b"bottom" => borders.bottom = side,
                    b"left" | b"start" => borders.left = side,
                    b"right" | b"end" => borders.right = side,
                    // insideH/insideV are table-level; skip for now
                    _ => {}
                }
            }
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                let side = parse_border_side(&e);
                match name.as_slice() {
                    b"top" => borders.top = side,
                    b"bottom" => borders.bottom = side,
                    b"left" | b"start" => borders.left = side,
                    b"right" | b"end" => borders.right = side,
                    _ => {}
                }
                skip_to_end(reader)?;
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(borders)
}

/// Parse attributes of a single border side element (e.g. `<w:top w:val="single" w:sz="4" w:color="000000"/>`).
fn parse_border_side(e: &quick_xml::events::BytesStart<'_>) -> Option<BorderSide> {
    let style_str = get_val(e)?;
    let style = match style_str.as_str() {
        "none" | "nil" => BorderStyle::None,
        "single" => BorderStyle::Single,
        "double" => BorderStyle::Double,
        "dashed" | "dashSmallGap" => BorderStyle::Dashed,
        "dotted" => BorderStyle::Dotted,
        "thick" | "thickThinSmallGap" | "thinThickSmallGap" => BorderStyle::Thick,
        _ => BorderStyle::Single,
    };

    // w:sz is in eighths of a point
    let width = get_attr(e, b"sz")
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v / 8.0)
        .unwrap_or(0.5);

    let color = get_attr(e, b"color")
        .and_then(|hex| {
            if hex == "auto" {
                None
            } else {
                Color::from_hex(&hex)
            }
        })
        .unwrap_or(Color::BLACK);

    let spacing = get_attr(e, b"space")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Some(BorderSide {
        style,
        width,
        color,
        spacing,
    })
}

/// Skip to the matching end tag (for elements we want to ignore).
fn skip_to_end(reader: &mut Reader<&[u8]>) -> Result<(), DocxError> {
    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }
    Ok(())
}

/// Capture all inner XML content of the current element as a raw string.
///
/// The reader should be positioned just after reading the `Start` event of
/// the outer element. This function reads until the matching `End` event
/// with `end_tag` and returns the raw XML content in between.
fn capture_inner_xml(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<String, DocxError> {
    let mut xml = String::new();
    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                xml.push('<');
                xml.push_str(&name);
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    xml.push(' ');
                    xml.push_str(&key);
                    xml.push_str("=\"");
                    xml.push_str(&val);
                    xml.push('"');
                }
                xml.push('>');
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                xml.push('<');
                xml.push_str(&name);
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    xml.push(' ');
                    xml.push_str(&key);
                    xml.push_str("=\"");
                    xml.push_str(&val);
                    xml.push('"');
                }
                xml.push_str("/>");
            }
            Ok(Event::End(e)) => {
                depth -= 1;
                if depth == 0 && e.local_name().as_ref() == end_tag {
                    break;
                }
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                xml.push_str("</");
                xml.push_str(&name);
                xml.push('>');
            }
            Ok(Event::Text(t)) => {
                xml.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }
    Ok(xml)
}

/// Convert OOXML highlight color names to Color values.
fn highlight_name_to_color(name: &str) -> Option<Color> {
    match name {
        "yellow" => Some(Color::new(255, 255, 0)),
        "green" => Some(Color::new(0, 255, 0)),
        "cyan" => Some(Color::new(0, 255, 255)),
        "magenta" => Some(Color::new(255, 0, 255)),
        "blue" => Some(Color::new(0, 0, 255)),
        "red" => Some(Color::new(255, 0, 0)),
        "darkBlue" => Some(Color::new(0, 0, 139)),
        "darkCyan" => Some(Color::new(0, 139, 139)),
        "darkGreen" => Some(Color::new(0, 100, 0)),
        "darkMagenta" => Some(Color::new(139, 0, 139)),
        "darkRed" => Some(Color::new(139, 0, 0)),
        "darkYellow" => Some(Color::new(128, 128, 0)),
        "darkGray" => Some(Color::new(169, 169, 169)),
        "lightGray" => Some(Color::new(211, 211, 211)),
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bold_italic() {
        let xml = r#"<w:rPr><w:b/><w:i/></w:rPr>"#;
        let mut reader = Reader::from_str(xml);
        // Skip the opening rPr tag
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"rPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_run_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(attrs.get_bool(&AttributeKey::Italic), Some(true));
    }

    #[test]
    fn parse_bold_false() {
        let xml = r#"<w:rPr><w:b w:val="false"/></w:rPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"rPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_run_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_bool(&AttributeKey::Bold), Some(false));
    }

    #[test]
    fn parse_font_size() {
        let xml = r#"<w:rPr><w:sz w:val="24"/></w:rPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"rPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_run_properties(&mut reader).unwrap();
        // 24 half-points = 12pt
        assert_eq!(attrs.get_f64(&AttributeKey::FontSize), Some(12.0));
    }

    #[test]
    fn parse_color() {
        let xml = r#"<w:rPr><w:color w:val="FF0000"/></w:rPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"rPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_run_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_color(&AttributeKey::Color), Some(Color::RED));
    }

    #[test]
    fn parse_font_family() {
        let xml = r#"<w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"rPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_run_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_string(&AttributeKey::FontFamily), Some("Arial"));
    }

    #[test]
    fn parse_paragraph_alignment() {
        let xml = r#"<w:pPr><w:jc w:val="center"/></w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"pPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_alignment(&AttributeKey::Alignment),
            Some(Alignment::Center)
        );
    }

    #[test]
    fn parse_paragraph_spacing() {
        let xml = r#"<w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"pPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        // 240 twips = 12pt, 120 twips = 6pt
        assert_eq!(attrs.get_f64(&AttributeKey::SpacingBefore), Some(12.0));
        assert_eq!(attrs.get_f64(&AttributeKey::SpacingAfter), Some(6.0));
    }

    #[test]
    fn parse_paragraph_indent() {
        let xml = r#"<w:pPr><w:ind w:left="720" w:firstLine="360"/></w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"pPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        // 720 twips = 36pt (0.5in), 360 twips = 18pt
        assert_eq!(attrs.get_f64(&AttributeKey::IndentLeft), Some(36.0));
        assert_eq!(attrs.get_f64(&AttributeKey::IndentFirstLine), Some(18.0));
    }

    #[test]
    fn parse_paragraph_style_ref() {
        let xml = r#"<w:pPr><w:pStyle w:val="Heading1"/></w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"pPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_string(&AttributeKey::StyleId), Some("Heading1"));
    }

    #[test]
    fn parse_suppress_auto_hyphens() {
        let xml = r#"<w:pPr><w:suppressAutoHyphens/></w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"pPr" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_bool(&AttributeKey::SuppressAutoHyphens),
            Some(true)
        );
    }

    // ─── Table property tests ─────────────────────────────────────────

    fn skip_to_start(reader: &mut Reader<&[u8]>, tag: &[u8]) {
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == tag => break,
                Ok(Event::Eof) => panic!("unexpected EOF looking for tag"),
                _ => {}
            }
        }
    }

    #[test]
    fn parse_table_width_auto() {
        let xml = r#"<w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tblPr");
        let attrs = parse_table_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::TableWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Auto)) => {}
            other => panic!("Expected TableWidth::Auto, got {:?}", other),
        }
    }

    #[test]
    fn parse_table_width_dxa() {
        let xml = r#"<w:tblPr><w:tblW w:w="9360" w:type="dxa"/></w:tblPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tblPr");
        let attrs = parse_table_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::TableWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Fixed(pts))) => {
                assert!((pts - 468.0).abs() < 0.01); // 9360 twips = 468pt
            }
            other => panic!("Expected TableWidth::Fixed, got {:?}", other),
        }
    }

    #[test]
    fn parse_table_width_pct() {
        let xml = r#"<w:tblPr><w:tblW w:w="5000" w:type="pct"/></w:tblPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tblPr");
        let attrs = parse_table_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::TableWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Percent(pct))) => {
                assert!((pct - 100.0).abs() < 0.01); // 5000/50 = 100%
            }
            other => panic!("Expected TableWidth::Percent, got {:?}", other),
        }
    }

    #[test]
    fn parse_table_alignment() {
        let xml = r#"<w:tblPr><w:jc w:val="center"/></w:tblPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tblPr");
        let attrs = parse_table_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_alignment(&AttributeKey::TableAlignment),
            Some(Alignment::Center)
        );
    }

    #[test]
    fn parse_table_borders() {
        let xml = r#"<w:tblPr><w:tblBorders>
            <w:top w:val="single" w:sz="4" w:color="000000"/>
            <w:bottom w:val="double" w:sz="8" w:color="FF0000"/>
        </w:tblBorders></w:tblPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tblPr");
        let attrs = parse_table_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::TableBorders) {
            Some(AttributeValue::Borders(b)) => {
                let top = b.top.as_ref().unwrap();
                assert_eq!(top.style, s1_model::BorderStyle::Single);
                assert!((top.width - 0.5).abs() < 0.01); // 4/8 = 0.5pt

                let bottom = b.bottom.as_ref().unwrap();
                assert_eq!(bottom.style, s1_model::BorderStyle::Double);
                assert_eq!(bottom.color, Color::RED);
            }
            other => panic!("Expected Borders, got {:?}", other),
        }
    }

    #[test]
    fn parse_cell_width_and_span() {
        let xml = r#"<w:tcPr><w:tcW w:w="2880" w:type="dxa"/><w:gridSpan w:val="2"/></w:tcPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tcPr");
        let attrs = parse_cell_properties(&mut reader).unwrap();

        // 2880 twips = 144pt
        match attrs.get(&AttributeKey::CellWidth) {
            Some(AttributeValue::TableWidth(s1_model::TableWidth::Fixed(pts))) => {
                assert!((pts - 144.0).abs() < 0.01);
            }
            other => panic!("Expected CellWidth Fixed, got {:?}", other),
        }

        assert_eq!(attrs.get_i64(&AttributeKey::ColSpan), Some(2));
    }

    #[test]
    fn parse_cell_vmerge_restart() {
        let xml = r#"<w:tcPr><w:vMerge w:val="restart"/></w:tcPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tcPr");
        let attrs = parse_cell_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_string(&AttributeKey::RowSpan), Some("restart"));
    }

    #[test]
    fn parse_cell_vmerge_continue() {
        let xml = r#"<w:tcPr><w:vMerge/></w:tcPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tcPr");
        let attrs = parse_cell_properties(&mut reader).unwrap();
        assert_eq!(attrs.get_string(&AttributeKey::RowSpan), Some("continue"));
    }

    #[test]
    fn parse_cell_valign_and_shading() {
        let xml = r#"<w:tcPr><w:vAlign w:val="center"/><w:shd w:fill="FFFF00"/></w:tcPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tcPr");
        let attrs = parse_cell_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::VerticalAlign) {
            Some(AttributeValue::VerticalAlignment(s1_model::VerticalAlignment::Center)) => {}
            other => panic!("Expected Center, got {:?}", other),
        }
        assert_eq!(
            attrs.get_color(&AttributeKey::CellBackground),
            Some(Color::new(255, 255, 0))
        );
    }

    #[test]
    fn parse_paragraph_numpr() {
        let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:numPr>
                <w:ilvl w:val="0"/>
                <w:numId w:val="3"/>
            </w:numPr>
        </w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        skip_to_start(&mut reader, b"pPr");
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(info)) => {
                assert_eq!(info.level, 0);
                assert_eq!(info.num_id, 3);
            }
            other => panic!("Expected ListInfo, got {:?}", other),
        }
    }

    #[test]
    fn parse_paragraph_numpr_level_2() {
        let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:numPr>
                <w:ilvl w:val="2"/>
                <w:numId w:val="1"/>
            </w:numPr>
        </w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        skip_to_start(&mut reader, b"pPr");
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        match attrs.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(info)) => {
                assert_eq!(info.level, 2);
                assert_eq!(info.num_id, 1);
            }
            other => panic!("Expected ListInfo, got {:?}", other),
        }
    }

    #[test]
    fn parse_paragraph_numpr_zero_numid_ignored() {
        // numId=0 means "remove from list" in OOXML — should produce no ListInfo
        let xml = r#"<w:pPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:numPr>
                <w:ilvl w:val="0"/>
                <w:numId w:val="0"/>
            </w:numPr>
        </w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        skip_to_start(&mut reader, b"pPr");
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        assert!(attrs.get(&AttributeKey::ListInfo).is_none());
    }

    #[test]
    fn parse_ppr_change() {
        let xml = r#"<w:pPr><w:jc w:val="center"/><w:pPrChange w:id="10" w:author="Alice" w:date="2026-01-01T12:00:00Z"><w:pPr><w:jc w:val="left"/></w:pPr></w:pPrChange></w:pPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"pPr");
        let attrs = parse_paragraph_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionType).as_deref(),
            Some("PropertyChange")
        );
        assert_eq!(attrs.get_i64(&AttributeKey::RevisionId), Some(10));
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionAuthor).as_deref(),
            Some("Alice")
        );
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionDate).as_deref(),
            Some("2026-01-01T12:00:00Z")
        );
        // Current alignment should still be parsed
        assert_eq!(
            attrs.get_alignment(&AttributeKey::Alignment),
            Some(Alignment::Center)
        );
    }

    #[test]
    fn parse_tcpr_change() {
        let xml = r#"<w:tcPr><w:tcW w:w="2880" w:type="dxa"/><w:tcPrChange w:id="20" w:author="Bob" w:date="2026-02-15T08:00:00Z"><w:tcPr><w:tcW w:w="1440" w:type="dxa"/></w:tcPr></w:tcPrChange></w:tcPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tcPr");
        let attrs = parse_cell_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionType).as_deref(),
            Some("PropertyChange")
        );
        assert_eq!(attrs.get_i64(&AttributeKey::RevisionId), Some(20));
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionAuthor).as_deref(),
            Some("Bob")
        );
        // Cell width should still be parsed
        assert!(attrs.get(&AttributeKey::CellWidth).is_some());
    }

    #[test]
    fn parse_tblpr_change() {
        let xml = r#"<w:tblPr><w:jc w:val="center"/><w:tblPrChange w:id="30" w:author="Carol" w:date="2026-03-10T10:00:00Z"><w:tblPr><w:jc w:val="left"/></w:tblPr></w:tblPrChange></w:tblPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"tblPr");
        let attrs = parse_table_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionType).as_deref(),
            Some("PropertyChange")
        );
        assert_eq!(attrs.get_i64(&AttributeKey::RevisionId), Some(30));
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionAuthor).as_deref(),
            Some("Carol")
        );
        assert_eq!(
            attrs.get_alignment(&AttributeKey::TableAlignment),
            Some(Alignment::Center)
        );
    }

    #[test]
    fn parse_trpr_change() {
        let xml = r#"<w:trPr><w:tblHeader/><w:trPrChange w:id="40" w:author="Dave" w:date="2026-03-12T14:00:00Z"><w:trPr/></w:trPrChange></w:trPr>"#;
        let mut reader = Reader::from_str(xml);
        skip_to_start(&mut reader, b"trPr");
        let attrs = parse_row_properties(&mut reader).unwrap();
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionType).as_deref(),
            Some("PropertyChange")
        );
        assert_eq!(attrs.get_i64(&AttributeKey::RevisionId), Some(40));
        assert_eq!(
            attrs.get_string(&AttributeKey::RevisionAuthor).as_deref(),
            Some("Dave")
        );
        assert_eq!(attrs.get_bool(&AttributeKey::TableHeaderRow), Some(true));
    }
}
