//! XLSX reader — parse ZIP archive into Workbook model.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

use crate::error::XlsxError;
use crate::model::*;
use crate::shared_strings::parse_shared_strings;
use crate::styles::parse_styles;

/// Known ZIP entry paths that the reader explicitly handles.
const KNOWN_PATHS: &[&str] = &[
    "[Content_Types].xml",
    "_rels/.rels",
    "xl/workbook.xml",
    "xl/_rels/workbook.xml.rels",
    "xl/sharedStrings.xml",
    "xl/styles.xml",
];

/// Read an XLSX file from bytes and produce a [`Workbook`].
pub fn read(input: &[u8]) -> Result<Workbook, XlsxError> {
    let cursor = Cursor::new(input);
    let mut archive = ZipArchive::new(cursor)?;

    let mut workbook = Workbook::default();

    // 1. Parse shared strings
    if let Ok(xml) = read_zip_entry(&mut archive, "xl/sharedStrings.xml") {
        workbook.shared_strings = parse_shared_strings(&xml)?;
    }

    // 2. Parse styles
    if let Ok(xml) = read_zip_entry(&mut archive, "xl/styles.xml") {
        workbook.styles = parse_styles(&xml)?;
    }

    // 3. Parse workbook.xml to get sheet names
    let sheet_names = if let Ok(xml) = read_zip_entry(&mut archive, "xl/workbook.xml") {
        parse_workbook_xml(&xml)?
    } else {
        vec!["Sheet1".to_string()]
    };

    // Build set of known paths (including sheet paths)
    let mut known: std::collections::HashSet<String> =
        KNOWN_PATHS.iter().map(|s| s.to_string()).collect();
    for i in 0..sheet_names.len() {
        known.insert(format!("xl/worksheets/sheet{}.xml", i + 1));
    }

    // 4. Parse each worksheet
    for (i, name) in sheet_names.iter().enumerate() {
        let sheet_path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let mut sheet = Sheet {
            name: name.clone(),
            ..Default::default()
        };

        if let Ok(xml) = read_zip_entry(&mut archive, &sheet_path) {
            parse_sheet_xml(&xml, &workbook.shared_strings, &mut sheet)?;
        }

        workbook.sheets.push(sheet);
    }

    // 5. Collect unrecognized ZIP entries (preserved parts)
    let mut preserved_parts: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            let entry_name = entry.name().to_string();
            if !known.contains(&entry_name) {
                let mut data = Vec::new();
                if entry.read_to_end(&mut data).is_ok() {
                    preserved_parts.insert(entry_name, data);
                }
            }
        }
    }
    workbook.preserved_parts = preserved_parts;

    Ok(workbook)
}

/// Parse workbook.xml to extract sheet names.
fn parse_workbook_xml(xml: &str) -> Result<Vec<String>, XlsxError> {
    let mut names = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.local_name().as_ref() == b"sheet" => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"name" {
                        if let Ok(name) = attr.unescape_value() {
                            names.push(name.to_string());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(names)
}

/// Parse a worksheet XML into a Sheet.
fn parse_sheet_xml(
    xml: &str,
    shared_strings: &[String],
    sheet: &mut Sheet,
) -> Result<(), XlsxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut current_cell_ref: Option<CellRef> = None;
    let mut current_cell_type: String = String::new();
    let mut current_cell_style: u32 = 0;
    let mut in_value = false;
    let mut in_formula = false;
    let mut value_text = String::new();
    let mut formula_text = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"row" => {
                        // Parse row index and optional height
                        let mut row_r: Option<u32> = None;
                        let mut row_ht: Option<f64> = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"r" => {
                                    if let Ok(v) = attr.unescape_value() {
                                        row_r = v.parse().ok();
                                    }
                                }
                                b"ht" => {
                                    if let Ok(v) = attr.unescape_value() {
                                        row_ht = v.parse().ok();
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let (Some(r), Some(ht)) = (row_r, row_ht) {
                            // Store 0-indexed row height
                            sheet.row_heights.insert(r - 1, ht);
                        }
                    }
                    b"c" => {
                        // Cell element: <c r="A1" t="s" s="1">
                        current_cell_type.clear();
                        current_cell_style = 0;
                        current_cell_ref = None;
                        value_text.clear();
                        formula_text.clear();

                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"r" => {
                                    if let Ok(ref_str) = attr.unescape_value() {
                                        current_cell_ref = CellRef::from_a1(&ref_str);
                                    }
                                }
                                b"t" => {
                                    if let Ok(t) = attr.unescape_value() {
                                        current_cell_type = t.to_string();
                                    }
                                }
                                b"s" => {
                                    if let Ok(s) = attr.unescape_value() {
                                        current_cell_style = s.parse().unwrap_or(0);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    b"v" => {
                        in_value = true;
                        value_text.clear();
                    }
                    b"f" => {
                        in_formula = true;
                        formula_text.clear();
                    }
                    b"mergeCell" => {
                        parse_merge_cell_attr(&e, sheet);
                    }
                    b"pane" => {
                        // Frozen pane: <pane xSplit="1" ySplit="2" .../>
                        parse_pane_element(&e, sheet);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"c" => {
                        if let Some(cell_ref) = current_cell_ref {
                            let value =
                                resolve_cell_value(&current_cell_type, &value_text, shared_strings);
                            let formula = if formula_text.is_empty() {
                                None
                            } else {
                                Some(formula_text.clone())
                            };
                            sheet.cells.insert(
                                cell_ref,
                                Cell {
                                    value,
                                    formula,
                                    style_id: current_cell_style,
                                },
                            );
                        }
                    }
                    b"v" => in_value = false,
                    b"f" => in_formula = false,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    if in_value {
                        value_text.push_str(&text);
                    } else if in_formula {
                        formula_text.push_str(&text);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name();
                match name.as_ref() {
                    b"mergeCell" => {
                        parse_merge_cell_attr(&e, sheet);
                    }
                    b"col" => {
                        // <col min="1" max="3" width="15.5"/>
                        parse_col_element(&e, sheet);
                    }
                    b"pane" => {
                        parse_pane_element(&e, sheet);
                    }
                    b"row" => {
                        // Self-closing row with possible height
                        let mut row_r: Option<u32> = None;
                        let mut row_ht: Option<f64> = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"r" => {
                                    if let Ok(v) = attr.unescape_value() {
                                        row_r = v.parse().ok();
                                    }
                                }
                                b"ht" => {
                                    if let Ok(v) = attr.unescape_value() {
                                        row_ht = v.parse().ok();
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let (Some(r), Some(ht)) = (row_r, row_ht) {
                            sheet.row_heights.insert(r - 1, ht);
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

    Ok(())
}

/// Parse `<col>` attributes and store column widths.
fn parse_col_element(e: &quick_xml::events::BytesStart<'_>, sheet: &mut Sheet) {
    let mut min: Option<u32> = None;
    let mut max: Option<u32> = None;
    let mut width: Option<f64> = None;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"min" => {
                if let Ok(v) = attr.unescape_value() {
                    min = v.parse().ok();
                }
            }
            b"max" => {
                if let Ok(v) = attr.unescape_value() {
                    max = v.parse().ok();
                }
            }
            b"width" => {
                if let Ok(v) = attr.unescape_value() {
                    width = v.parse().ok();
                }
            }
            _ => {}
        }
    }

    if let (Some(mn), Some(mx), Some(w)) = (min, max, width) {
        // OOXML columns are 1-indexed; store 0-indexed
        for col in mn..=mx {
            sheet.column_widths.insert(col - 1, w);
        }
    }
}

/// Parse `<pane>` attributes for frozen panes.
fn parse_pane_element(e: &quick_xml::events::BytesStart<'_>, sheet: &mut Sheet) {
    let mut x_split: u32 = 0;
    let mut y_split: u32 = 0;

    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"xSplit" => {
                if let Ok(v) = attr.unescape_value() {
                    x_split = v.parse().unwrap_or(0);
                }
            }
            b"ySplit" => {
                if let Ok(v) = attr.unescape_value() {
                    y_split = v.parse().unwrap_or(0);
                }
            }
            _ => {}
        }
    }

    if x_split > 0 || y_split > 0 {
        sheet.frozen_pane = Some(CellRef::new(x_split, y_split));
    }
}

/// Parse `<mergeCell>` attribute.
fn parse_merge_cell_attr(e: &quick_xml::events::BytesStart<'_>, sheet: &mut Sheet) {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"ref" {
            if let Ok(range_str) = attr.unescape_value() {
                if let Some(range) = CellRange::from_a1(&range_str) {
                    sheet.merged_cells.push(range);
                }
            }
        }
    }
}

/// Resolve a cell value from type string and raw text.
fn resolve_cell_value(cell_type: &str, value_text: &str, shared_strings: &[String]) -> CellValue {
    match cell_type {
        "s" => {
            let idx: usize = value_text.trim().parse().unwrap_or(0);
            shared_strings
                .get(idx)
                .map(|s| CellValue::Text(s.clone()))
                .unwrap_or(CellValue::Empty)
        }
        "b" => CellValue::Boolean(value_text.trim() == "1"),
        "e" => {
            let err = match value_text.trim() {
                "#DIV/0!" => CellError::DivZero,
                "#VALUE!" => CellError::Value,
                "#REF!" => CellError::Ref,
                "#NAME?" => CellError::Name,
                "#NUM!" => CellError::Num,
                "#N/A" => CellError::NA,
                _ => CellError::Null,
            };
            CellValue::Error(err)
        }
        "str" | "inlineStr" => CellValue::Text(value_text.trim().to_string()),
        _ => {
            if let Ok(n) = value_text.trim().parse::<f64>() {
                CellValue::Number(n)
            } else if value_text.trim().is_empty() {
                CellValue::Empty
            } else {
                CellValue::Text(value_text.trim().to_string())
            }
        }
    }
}

fn read_zip_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
) -> Result<String, XlsxError> {
    let mut file = archive.by_name(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_workbook_sheets() {
        let xml = r#"<?xml version="1.0"?>
        <workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <sheets>
                <sheet name="Data" sheetId="1" r:id="rId1"/>
                <sheet name="Summary" sheetId="2" r:id="rId2"/>
            </sheets>
        </workbook>"#;
        let names = parse_workbook_xml(xml).unwrap();
        assert_eq!(names, vec!["Data", "Summary"]);
    }

    #[test]
    fn parse_sheet_cells() {
        let shared = vec!["Hello".to_string(), "World".to_string()];
        let xml = r#"<?xml version="1.0"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <sheetData>
                <row r="1">
                    <c r="A1" t="s"><v>0</v></c>
                    <c r="B1" t="n"><v>42.5</v></c>
                    <c r="C1"><f>A1+B1</f><v>42.5</v></c>
                </row>
                <row r="2">
                    <c r="A2" t="b"><v>1</v></c>
                    <c r="B2" t="e"><v>#DIV/0!</v></c>
                </row>
            </sheetData>
        </worksheet>"#;
        let mut sheet = Sheet::default();
        parse_sheet_xml(xml, &shared, &mut sheet).unwrap();

        assert_eq!(sheet.cells.len(), 5);
        assert_eq!(
            sheet.get(0, 0).unwrap().value,
            CellValue::Text("Hello".into())
        );
        assert_eq!(sheet.get(1, 0).unwrap().value, CellValue::Number(42.5));
        assert_eq!(sheet.get(2, 0).unwrap().formula.as_deref(), Some("A1+B1"));
        assert_eq!(sheet.get(0, 1).unwrap().value, CellValue::Boolean(true));
        assert_eq!(
            sheet.get(1, 1).unwrap().value,
            CellValue::Error(CellError::DivZero)
        );
    }

    #[test]
    fn parse_column_widths() {
        let xml = r#"<?xml version="1.0"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <cols>
                <col min="1" max="1" width="20.5"/>
                <col min="2" max="4" width="15.0"/>
            </cols>
            <sheetData/>
        </worksheet>"#;
        let mut sheet = Sheet::default();
        parse_sheet_xml(xml, &[], &mut sheet).unwrap();

        assert_eq!(sheet.column_widths.get(&0), Some(&20.5));
        assert_eq!(sheet.column_widths.get(&1), Some(&15.0));
        assert_eq!(sheet.column_widths.get(&2), Some(&15.0));
        assert_eq!(sheet.column_widths.get(&3), Some(&15.0));
        assert_eq!(sheet.column_widths.get(&4), None);
    }

    #[test]
    fn parse_row_heights() {
        let xml = r#"<?xml version="1.0"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <sheetData>
                <row r="1" ht="25.5">
                    <c r="A1"><v>1</v></c>
                </row>
                <row r="3" ht="30.0">
                    <c r="A3"><v>3</v></c>
                </row>
            </sheetData>
        </worksheet>"#;
        let mut sheet = Sheet::default();
        parse_sheet_xml(xml, &[], &mut sheet).unwrap();

        assert_eq!(sheet.row_heights.get(&0), Some(&25.5));
        assert_eq!(sheet.row_heights.get(&1), None);
        assert_eq!(sheet.row_heights.get(&2), Some(&30.0));
    }

    #[test]
    fn parse_frozen_pane() {
        let xml = r#"<?xml version="1.0"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <sheetViews>
                <sheetView tabSelected="1" workbookViewId="0">
                    <pane xSplit="1" ySplit="2" topLeftCell="B3" activePane="bottomRight" state="frozen"/>
                </sheetView>
            </sheetViews>
            <sheetData/>
        </worksheet>"#;
        let mut sheet = Sheet::default();
        parse_sheet_xml(xml, &[], &mut sheet).unwrap();

        let pane = sheet.frozen_pane.unwrap();
        assert_eq!(pane.col, 1);
        assert_eq!(pane.row, 2);
    }

    #[test]
    fn parse_frozen_pane_y_only() {
        let xml = r#"<?xml version="1.0"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <sheetViews>
                <sheetView>
                    <pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>
                </sheetView>
            </sheetViews>
            <sheetData/>
        </worksheet>"#;
        let mut sheet = Sheet::default();
        parse_sheet_xml(xml, &[], &mut sheet).unwrap();

        let pane = sheet.frozen_pane.unwrap();
        assert_eq!(pane.col, 0);
        assert_eq!(pane.row, 1);
    }
}
