//! XLSX writer — generate ZIP archive from Workbook model.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::XlsxError;
use crate::model::*;
use crate::shared_strings::write_shared_strings;
use crate::styles::write_styles;

/// Write a Workbook to XLSX bytes.
pub fn write(workbook: &Workbook) -> Result<Vec<u8>, XlsxError> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Build shared string table from all cells
    let mut string_table: Vec<String> = workbook.shared_strings.clone();
    let mut string_index: std::collections::HashMap<String, usize> = string_table
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    // Collect any new strings from cells
    for sheet in &workbook.sheets {
        for cell in sheet.cells.values() {
            if let CellValue::Text(ref s) = cell.value {
                if !string_index.contains_key(s) {
                    let idx = string_table.len();
                    string_table.push(s.clone());
                    string_index.insert(s.clone(), idx);
                }
            }
        }
    }

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    let mut ct = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    ct.push_str(r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#);
    ct.push_str(r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#);
    ct.push_str(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
    ct.push_str(r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#);
    ct.push_str(r#"<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>"#);
    ct.push_str(r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#);
    for (i, _) in workbook.sheets.iter().enumerate() {
        ct.push_str(&format!(
            r#"<Override PartName="/xl/worksheets/sheet{}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
            i + 1
        ));
    }
    ct.push_str("</Types>");
    zip.write_all(ct.as_bytes())?;

    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
            .as_bytes(),
    )?;

    // xl/workbook.xml
    zip.start_file("xl/workbook.xml", options)?;
    let mut wb = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    wb.push_str(r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
    wb.push_str("<sheets>");
    for (i, sheet) in workbook.sheets.iter().enumerate() {
        wb.push_str(&format!(
            r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
            quick_xml::escape::escape(&sheet.name),
            i + 1,
            i + 1
        ));
    }
    wb.push_str("</sheets></workbook>");
    zip.write_all(wb.as_bytes())?;

    // xl/_rels/workbook.xml.rels
    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    rels.push_str(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (i, _) in workbook.sheets.iter().enumerate() {
        rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{}.xml"/>"#,
            i + 1,
            i + 1
        ));
    }
    rels.push_str(r#"<Relationship Id="rIdSS" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>"#);
    rels.push_str(r#"<Relationship Id="rIdST" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#);
    rels.push_str("</Relationships>");
    zip.write_all(rels.as_bytes())?;

    // xl/sharedStrings.xml
    zip.start_file("xl/sharedStrings.xml", options)?;
    zip.write_all(write_shared_strings(&string_table).as_bytes())?;

    // xl/styles.xml
    zip.start_file("xl/styles.xml", options)?;
    zip.write_all(write_styles(&workbook.styles).as_bytes())?;

    // xl/worksheets/sheetN.xml
    for (i, sheet) in workbook.sheets.iter().enumerate() {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        zip.start_file(&path, options)?;
        let xml = write_sheet_xml(sheet, &string_index);
        zip.write_all(xml.as_bytes())?;
    }

    // Write preserved parts (unrecognized ZIP entries)
    for (path, data) in &workbook.preserved_parts {
        zip.start_file(path, options)?;
        zip.write_all(data)?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

fn write_sheet_xml(
    sheet: &Sheet,
    string_index: &std::collections::HashMap<String, usize>,
) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );

    let (max_col, max_row) = sheet.dimensions();
    if max_col > 0 && max_row > 0 {
        let start = CellRef::new(0, 0).to_a1();
        let end = CellRef::new(max_col - 1, max_row - 1).to_a1();
        xml.push_str(&format!(r#"<dimension ref="{start}:{end}"/>"#));
    }

    // Frozen pane (sheetViews)
    if let Some(ref pane) = sheet.frozen_pane {
        let top_left = CellRef::new(pane.col, pane.row).to_a1();
        xml.push_str("<sheetViews><sheetView tabSelected=\"1\" workbookViewId=\"0\">");
        xml.push_str(&format!(
            r#"<pane xSplit="{}" ySplit="{}" topLeftCell="{}" activePane="bottomRight" state="frozen"/>"#,
            pane.col, pane.row, top_left
        ));
        xml.push_str("</sheetView></sheetViews>");
    }

    // Column widths
    if !sheet.column_widths.is_empty() {
        xml.push_str("<cols>");
        // Group consecutive columns with the same width
        let mut sorted_cols: Vec<(u32, f64)> =
            sheet.column_widths.iter().map(|(&c, &w)| (c, w)).collect();
        sorted_cols.sort_by_key(|(c, _)| *c);

        let mut i = 0;
        while i < sorted_cols.len() {
            let (start_col, width) = sorted_cols[i];
            let mut end_col = start_col;
            // Merge consecutive columns with the same width
            while i + 1 < sorted_cols.len()
                && sorted_cols[i + 1].0 == end_col + 1
                && (sorted_cols[i + 1].1 - width).abs() < f64::EPSILON
            {
                end_col = sorted_cols[i + 1].0;
                i += 1;
            }
            // OOXML uses 1-indexed columns
            xml.push_str(&format!(
                r#"<col min="{}" max="{}" width="{}" customWidth="1"/>"#,
                start_col + 1,
                end_col + 1,
                width
            ));
            i += 1;
        }
        xml.push_str("</cols>");
    }

    xml.push_str("<sheetData>");

    // Group cells by row
    let mut rows: std::collections::BTreeMap<u32, Vec<(u32, &Cell)>> =
        std::collections::BTreeMap::new();
    for (cell_ref, cell) in &sheet.cells {
        rows.entry(cell_ref.row)
            .or_default()
            .push((cell_ref.col, cell));
    }

    // Also ensure rows with explicit heights appear even if they have no cells
    for &row_idx in sheet.row_heights.keys() {
        rows.entry(row_idx).or_default();
    }

    for (row_idx, mut cells) in rows {
        cells.sort_by_key(|(col, _)| *col);

        // Row element with optional height
        if let Some(&ht) = sheet.row_heights.get(&row_idx) {
            xml.push_str(&format!(
                r#"<row r="{}" ht="{}" customHeight="1">"#,
                row_idx + 1,
                ht
            ));
        } else {
            xml.push_str(&format!(r#"<row r="{}">"#, row_idx + 1));
        }

        for (col_idx, cell) in cells {
            let a1 = CellRef::new(col_idx, row_idx).to_a1();
            let mut attrs = format!(r#" r="{a1}""#);

            if cell.style_id != 0 {
                attrs.push_str(&format!(r#" s="{}""#, cell.style_id));
            }

            match &cell.value {
                CellValue::Text(s) => {
                    if let Some(&idx) = string_index.get(s) {
                        attrs.push_str(r#" t="s""#);
                        xml.push_str(&format!("<c{attrs}>"));
                        if let Some(ref f) = cell.formula {
                            xml.push_str(&format!("<f>{}</f>", quick_xml::escape::escape(f)));
                        }
                        xml.push_str(&format!("<v>{idx}</v></c>"));
                    } else {
                        attrs.push_str(r#" t="str""#);
                        xml.push_str(&format!("<c{attrs}>"));
                        if let Some(ref f) = cell.formula {
                            xml.push_str(&format!("<f>{}</f>", quick_xml::escape::escape(f)));
                        }
                        xml.push_str(&format!("<v>{}</v></c>", quick_xml::escape::escape(s)));
                    }
                }
                CellValue::Number(n) => {
                    xml.push_str(&format!("<c{attrs}>"));
                    if let Some(ref f) = cell.formula {
                        xml.push_str(&format!("<f>{}</f>", quick_xml::escape::escape(f)));
                    }
                    xml.push_str(&format!("<v>{n}</v></c>"));
                }
                CellValue::Boolean(b) => {
                    attrs.push_str(r#" t="b""#);
                    xml.push_str(&format!("<c{attrs}>"));
                    xml.push_str(&format!("<v>{}</v></c>", if *b { "1" } else { "0" }));
                }
                CellValue::Error(e) => {
                    attrs.push_str(r#" t="e""#);
                    xml.push_str(&format!("<c{attrs}><v>{e}</v></c>"));
                }
                CellValue::Date(serial) => {
                    xml.push_str(&format!("<c{attrs}><v>{serial}</v></c>"));
                }
                CellValue::Empty => {
                    xml.push_str(&format!("<c{attrs}/>"));
                }
            }
        }

        xml.push_str("</row>");
    }

    xml.push_str("</sheetData>");

    // Merged cells
    if !sheet.merged_cells.is_empty() {
        xml.push_str(&format!(
            r#"<mergeCells count="{}">"#,
            sheet.merged_cells.len()
        ));
        for range in &sheet.merged_cells {
            xml.push_str(&format!(
                r#"<mergeCell ref="{}:{}"/>"#,
                range.start.to_a1(),
                range.end.to_a1()
            ));
        }
        xml.push_str("</mergeCells>");
    }

    xml.push_str("</worksheet>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_roundtrip() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("Name".into()));
        sheet.set(1, 0, CellValue::Text("Score".into()));
        sheet.set(0, 1, CellValue::Text("Alice".into()));
        sheet.set(1, 1, CellValue::Number(95.0));
        sheet.set(0, 2, CellValue::Text("Bob".into()));
        sheet.set(1, 2, CellValue::Number(87.5));
        sheet.set(0, 3, CellValue::Boolean(true));

        let bytes = write(&wb).unwrap();
        assert!(bytes.len() > 100, "XLSX should be non-trivial");

        // Read it back
        let wb2 = crate::reader::read(&bytes).unwrap();
        assert_eq!(wb2.sheets.len(), 1);
        let s = &wb2.sheets[0];
        assert_eq!(s.name, "Sheet1");
        assert_eq!(s.get(0, 0).unwrap().value, CellValue::Text("Name".into()));
        assert_eq!(s.get(1, 1).unwrap().value, CellValue::Number(95.0));
        assert_eq!(s.get(0, 3).unwrap().value, CellValue::Boolean(true));
    }

    #[test]
    fn write_with_formulas() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Number(10.0));
        sheet.set(0, 1, CellValue::Number(20.0));
        sheet.set_formula(0, 2, "SUM(A1:A2)", CellValue::Number(30.0));

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();
        let s = &wb2.sheets[0];
        assert_eq!(s.get(0, 2).unwrap().formula.as_deref(), Some("SUM(A1:A2)"));
        assert_eq!(s.get(0, 2).unwrap().value, CellValue::Number(30.0));
    }

    #[test]
    fn write_multiple_sheets() {
        let mut wb = Workbook {
            sheets: vec![
                Sheet {
                    name: "Sheet1".into(),
                    ..Default::default()
                },
                Sheet {
                    name: "Sheet2".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        wb.sheets[0].set(0, 0, CellValue::Text("First".into()));
        wb.sheets[1].set(0, 0, CellValue::Text("Second".into()));

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();
        assert_eq!(wb2.sheets.len(), 2);
        assert_eq!(wb2.sheets[0].name, "Sheet1");
        assert_eq!(wb2.sheets[1].name, "Sheet2");
    }

    #[test]
    fn roundtrip_column_widths() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("A".into()));
        sheet.column_widths.insert(0, 20.5);
        sheet.column_widths.insert(1, 15.0);
        sheet.column_widths.insert(2, 15.0);

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();
        let s = &wb2.sheets[0];
        assert_eq!(s.column_widths.get(&0), Some(&20.5));
        assert_eq!(s.column_widths.get(&1), Some(&15.0));
        assert_eq!(s.column_widths.get(&2), Some(&15.0));
    }

    #[test]
    fn roundtrip_row_heights() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Number(1.0));
        sheet.set(0, 2, CellValue::Number(3.0));
        sheet.row_heights.insert(0, 25.5);
        sheet.row_heights.insert(2, 30.0);

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();
        let s = &wb2.sheets[0];
        assert_eq!(s.row_heights.get(&0), Some(&25.5));
        assert_eq!(s.row_heights.get(&2), Some(&30.0));
    }

    #[test]
    fn roundtrip_frozen_pane() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("Header".into()));
        sheet.frozen_pane = Some(CellRef::new(1, 2));

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();
        let s = &wb2.sheets[0];
        let pane = s.frozen_pane.unwrap();
        assert_eq!(pane.col, 1);
        assert_eq!(pane.row, 2);
    }

    #[test]
    fn roundtrip_styles() {
        let mut wb = Workbook::new();
        wb.styles = StyleSheet {
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
                    italic: false,
                    color: Some("FFFF0000".to_string()),
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
            borders: vec![
                BorderDef::default(),
                BorderDef {
                    left: Some(BorderSide {
                        style: "thin".to_string(),
                        color: Some("FF000000".to_string()),
                    }),
                    right: Some(BorderSide {
                        style: "thin".to_string(),
                        color: Some("FF000000".to_string()),
                    }),
                    top: Some(BorderSide {
                        style: "thin".to_string(),
                        color: Some("FF000000".to_string()),
                    }),
                    bottom: Some(BorderSide {
                        style: "thin".to_string(),
                        color: Some("FF000000".to_string()),
                    }),
                },
            ],
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
                    border_id: 1,
                    alignment: Some(CellAlignment {
                        horizontal: Some("center".to_string()),
                        vertical: Some("top".to_string()),
                        wrap_text: true,
                    }),
                },
            ],
        };

        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("Styled".into()));
        // Apply style 1 to a cell
        sheet.cells.get_mut(&CellRef::new(0, 0)).unwrap().style_id = 1;

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();

        // Verify styles survived round-trip
        assert_eq!(wb2.styles.number_formats.len(), 1);
        assert_eq!(wb2.styles.number_formats[0].id, 164);
        assert_eq!(wb2.styles.fonts.len(), 2);
        assert!(wb2.styles.fonts[1].bold);
        assert_eq!(wb2.styles.fills.len(), 3);
        assert_eq!(wb2.styles.fills[2].fg_color.as_deref(), Some("FFFFFF00"));
        assert_eq!(wb2.styles.borders.len(), 2);
        assert_eq!(wb2.styles.cell_formats.len(), 2);
        let xf1 = &wb2.styles.cell_formats[1];
        assert_eq!(xf1.font_id, 1);
        assert_eq!(xf1.fill_id, 2);
        assert!(xf1.alignment.is_some());

        // Verify the cell kept its style_id
        assert_eq!(wb2.sheets[0].get(0, 0).unwrap().style_id, 1);
    }

    #[test]
    fn roundtrip_preserved_parts() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("Test".into()));

        // Add a custom preserved part
        wb.preserved_parts.insert(
            "customXml/item1.xml".to_string(),
            b"<custom>data</custom>".to_vec(),
        );

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();

        assert!(wb2.preserved_parts.contains_key("customXml/item1.xml"));
        assert_eq!(
            wb2.preserved_parts.get("customXml/item1.xml").unwrap(),
            b"<custom>data</custom>"
        );
    }

    #[test]
    fn roundtrip_full_workbook() {
        // Comprehensive round-trip test with everything combined
        let mut wb = Workbook::new();

        // Styles
        wb.styles = StyleSheet {
            number_formats: vec![NumberFormat {
                id: 164,
                format_code: "0.00%".to_string(),
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
                    size: 16.0,
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
            ],
            borders: vec![BorderDef::default()],
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
                    fill_id: 0,
                    border_id: 0,
                    alignment: Some(CellAlignment {
                        horizontal: Some("right".to_string()),
                        vertical: None,
                        wrap_text: false,
                    }),
                },
            ],
        };

        // Sheet with column widths, row heights, frozen pane, merged cells
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("Product".into()));
        sheet.set(1, 0, CellValue::Text("Price".into()));
        sheet.set(2, 0, CellValue::Text("Qty".into()));
        sheet.set(0, 1, CellValue::Text("Widget".into()));
        sheet.set(1, 1, CellValue::Number(9.99));
        sheet.set(2, 1, CellValue::Number(100.0));
        sheet.set_formula(3, 1, "B2*C2", CellValue::Number(999.0));
        sheet.set(0, 2, CellValue::Boolean(true));
        sheet.set(0, 3, CellValue::Error(CellError::DivZero));

        // Apply style 1 to price header
        sheet.cells.get_mut(&CellRef::new(1, 0)).unwrap().style_id = 1;

        sheet.column_widths.insert(0, 25.0);
        sheet.column_widths.insert(1, 12.0);
        sheet.column_widths.insert(2, 12.0);
        sheet.row_heights.insert(0, 20.0);
        sheet.frozen_pane = Some(CellRef::new(0, 1));
        sheet
            .merged_cells
            .push(CellRange::new(CellRef::new(0, 4), CellRef::new(2, 4)));

        // Preserved part
        wb.preserved_parts
            .insert("docProps/custom.xml".to_string(), b"<Properties/>".to_vec());

        let bytes = write(&wb).unwrap();
        let wb2 = crate::reader::read(&bytes).unwrap();

        let s = &wb2.sheets[0];

        // Cells
        assert_eq!(
            s.get(0, 0).unwrap().value,
            CellValue::Text("Product".into())
        );
        assert_eq!(s.get(1, 1).unwrap().value, CellValue::Number(9.99));
        assert_eq!(s.get(3, 1).unwrap().formula.as_deref(), Some("B2*C2"));
        assert_eq!(s.get(0, 2).unwrap().value, CellValue::Boolean(true));
        assert_eq!(
            s.get(0, 3).unwrap().value,
            CellValue::Error(CellError::DivZero)
        );

        // Style on cell
        assert_eq!(s.get(1, 0).unwrap().style_id, 1);

        // Column widths
        assert_eq!(s.column_widths.get(&0), Some(&25.0));
        assert_eq!(s.column_widths.get(&1), Some(&12.0));

        // Row height
        assert_eq!(s.row_heights.get(&0), Some(&20.0));

        // Frozen pane
        let pane = s.frozen_pane.unwrap();
        assert_eq!(pane.col, 0);
        assert_eq!(pane.row, 1);

        // Merged cells
        assert_eq!(s.merged_cells.len(), 1);
        assert_eq!(s.merged_cells[0].start, CellRef::new(0, 4));
        assert_eq!(s.merged_cells[0].end, CellRef::new(2, 4));

        // Styles
        assert_eq!(wb2.styles.number_formats.len(), 1);
        assert_eq!(wb2.styles.fonts.len(), 2);
        assert!(wb2.styles.fonts[1].bold);
        assert!(wb2.styles.fonts[1].italic);

        // Preserved parts
        assert!(wb2.preserved_parts.contains_key("docProps/custom.xml"));
    }
}
