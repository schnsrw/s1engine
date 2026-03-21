//! ODS (OpenDocument Spreadsheet) reader/writer.
//!
//! Reuses the same [`Workbook`] model as XLSX. Reads and writes `.ods` files
//! following the ODF 1.2 / 1.3 specification.
//!
//! ## Key differences from XLSX
//!
//! - Cell values use `office:value-type` attributes, not child `<v>` elements.
//! - Strings are inline `<text:p>` children, not shared string table entries.
//! - Formulas use OpenFormula syntax with `of:=` prefix and dot-notation cell refs.
//! - Repeated cells/rows use `table:number-columns-repeated` / `table:number-rows-repeated`.
//! - The `mimetype` file must be the first ZIP entry, uncompressed.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

use crate::error::XlsxError;
use crate::model::*;

// ─── ODS Reader ───────────────────────────────────────────

/// Read an ODS file from bytes and produce a [`Workbook`].
///
/// # Errors
///
/// Returns an error if the ZIP archive is invalid, content.xml is missing,
/// or the XML cannot be parsed.
pub fn read_ods(input: &[u8]) -> Result<Workbook, XlsxError> {
    let cursor = Cursor::new(input);
    let mut archive = ZipArchive::new(cursor)?;

    let mut workbook = Workbook::default();

    // Parse content.xml (required)
    let content_xml = read_zip_entry(&mut archive, "content.xml")?;
    parse_content_xml(&content_xml, &mut workbook)?;

    // Parse styles.xml (optional, we do not import named styles at this time)

    Ok(workbook)
}

/// Parse ODS `content.xml` and populate the workbook.
fn parse_content_xml(xml: &str, workbook: &mut Workbook) -> Result<(), XlsxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut in_spreadsheet = false;
    let mut in_table = false;
    let mut in_table_row = false;
    let mut in_cell = false;
    let mut in_text_p = false;

    let mut current_sheet: Option<Sheet> = None;
    let mut current_row: u32 = 0;
    let mut current_col: u32 = 0;

    // Current cell data
    let mut cell_value_type: Option<String> = None;
    let mut cell_value_attr: Option<String> = None;
    let mut cell_bool_attr: Option<String> = None;
    let mut cell_date_attr: Option<String> = None;
    let mut cell_formula: Option<String> = None;
    let mut cell_text: String = String::new();
    let mut cell_cols_repeated: u32 = 1;

    // Column width tracking
    let mut column_styles: BTreeMap<String, f64> = BTreeMap::new();
    let mut pending_col_style: Option<String> = None;
    let mut in_style = false;
    let mut current_style_name: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if is_element(&e, b"spreadsheet") {
                    in_spreadsheet = true;
                } else if is_element(&e, b"table") && in_spreadsheet {
                    in_table = true;
                    current_row = 0;
                    let name = attr_str_ods(&e, b"table:name").unwrap_or_default();
                    current_sheet = Some(Sheet {
                        name,
                        ..Default::default()
                    });
                } else if is_element(&e, b"table-row") && in_table {
                    in_table_row = true;
                    current_col = 0;
                } else if is_element(&e, b"table-cell") && in_table_row {
                    in_cell = true;
                    cell_value_type = attr_str_ods(&e, b"office:value-type");
                    cell_value_attr = attr_str_ods(&e, b"office:value");
                    cell_bool_attr = attr_str_ods(&e, b"office:boolean-value");
                    cell_date_attr = attr_str_ods(&e, b"office:date-value");
                    cell_formula = attr_str_ods(&e, b"table:formula");
                    cell_text.clear();
                    cell_cols_repeated = attr_str_ods(&e, b"table:number-columns-repeated")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(1);
                } else if is_element(&e, b"p") && in_cell {
                    in_text_p = true;
                    if !cell_text.is_empty() {
                        cell_text.push('\n');
                    }
                } else if is_element(&e, b"style") {
                    in_style = true;
                    current_style_name = attr_str_ods(&e, b"style:name");
                } else if is_element(&e, b"table-column-properties") && in_style {
                    if let Some(ref style_name) = current_style_name {
                        if let Some(width_str) = attr_str_ods(&e, b"style:column-width") {
                            if let Some(w) = parse_ods_length(&width_str) {
                                column_styles.insert(style_name.clone(), w);
                            }
                        }
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                if is_element(&e, b"table-cell") && in_table_row {
                    let repeated = attr_str_ods(&e, b"table:number-columns-repeated")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(1);

                    let vt = attr_str_ods(&e, b"office:value-type");
                    if vt.is_some() {
                        let val_attr = attr_str_ods(&e, b"office:value");
                        let bool_attr = attr_str_ods(&e, b"office:boolean-value");
                        let date_attr = attr_str_ods(&e, b"office:date-value");
                        let formula = attr_str_ods(&e, b"table:formula");
                        let value = resolve_ods_value(
                            vt.as_deref(),
                            val_attr.as_deref(),
                            bool_attr.as_deref(),
                            date_attr.as_deref(),
                            "",
                        );
                        let f = formula.map(|f| convert_ods_formula(&f));
                        if !matches!(value, CellValue::Empty) || f.is_some() {
                            if let Some(ref mut sheet) = current_sheet {
                                for i in 0..repeated {
                                    sheet.cells.insert(
                                        CellRef::new(current_col + i, current_row),
                                        Cell {
                                            value: value.clone(),
                                            formula: f.clone(),
                                            style_id: 0,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    current_col += repeated;
                } else if is_element(&e, b"table-column") && in_table {
                    let style = attr_str_ods(&e, b"table:style-name");
                    let repeated = attr_str_ods(&e, b"table:number-columns-repeated")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(1);
                    if let Some(ref s) = style {
                        pending_col_style = Some(s.clone());
                    }
                    if let Some(ref s) = style {
                        if let Some(&width) = column_styles.get(s) {
                            if let Some(ref mut sheet) = current_sheet {
                                let start = sheet.column_widths.len() as u32;
                                for i in 0..repeated {
                                    sheet.column_widths.insert(start + i, width);
                                }
                            }
                        }
                    }
                    let _ = pending_col_style;
                } else if is_element(&e, b"table-column-properties") && in_style {
                    if let Some(ref style_name) = current_style_name {
                        if let Some(width_str) = attr_str_ods(&e, b"style:column-width") {
                            if let Some(w) = parse_ods_length(&width_str) {
                                column_styles.insert(style_name.clone(), w);
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                if is_end_element(&e, b"spreadsheet") {
                    in_spreadsheet = false;
                } else if is_end_element(&e, b"table") && in_table {
                    in_table = false;
                    if let Some(sheet) = current_sheet.take() {
                        workbook.sheets.push(sheet);
                    }
                } else if is_end_element(&e, b"table-row") && in_table_row {
                    in_table_row = false;
                    current_row += 1;
                } else if is_end_element(&e, b"table-cell") && in_cell {
                    in_cell = false;
                    let value = resolve_ods_value(
                        cell_value_type.as_deref(),
                        cell_value_attr.as_deref(),
                        cell_bool_attr.as_deref(),
                        cell_date_attr.as_deref(),
                        &cell_text,
                    );
                    let formula = cell_formula.take().map(|f| convert_ods_formula(&f));

                    if !matches!(value, CellValue::Empty) || formula.is_some() {
                        if let Some(ref mut sheet) = current_sheet {
                            for i in 0..cell_cols_repeated {
                                sheet.cells.insert(
                                    CellRef::new(current_col + i, current_row),
                                    Cell {
                                        value: value.clone(),
                                        formula: formula.clone(),
                                        style_id: 0,
                                    },
                                );
                            }
                        }
                    }

                    current_col += cell_cols_repeated;
                    cell_value_type = None;
                    cell_value_attr = None;
                    cell_bool_attr = None;
                    cell_date_attr = None;
                    cell_text.clear();
                    cell_cols_repeated = 1;
                } else if is_end_element(&e, b"p") && in_text_p {
                    in_text_p = false;
                } else if is_end_element(&e, b"style") && in_style {
                    in_style = false;
                    current_style_name = None;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_p {
                    if let Ok(text) = e.unescape() {
                        cell_text.push_str(&text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(format!("ODS content.xml parse error: {e}"))),
            _ => {}
        }
    }

    Ok(())
}

/// Resolve a cell value from ODS attributes.
fn resolve_ods_value(
    value_type: Option<&str>,
    value_attr: Option<&str>,
    bool_attr: Option<&str>,
    date_attr: Option<&str>,
    text_content: &str,
) -> CellValue {
    match value_type {
        Some("float") | Some("currency") | Some("percentage") => {
            if let Some(v) = value_attr {
                if let Ok(n) = v.parse::<f64>() {
                    return CellValue::Number(n);
                }
            }
            CellValue::Empty
        }
        Some("string") => {
            if text_content.is_empty() {
                CellValue::Text(String::new())
            } else {
                CellValue::Text(text_content.to_string())
            }
        }
        Some("boolean") => {
            let b = bool_attr.map(|v| v == "true").unwrap_or(false);
            CellValue::Boolean(b)
        }
        Some("date") => {
            // ODS stores dates as ISO strings; we convert to a serial number.
            // For simplicity, store as text if parsing fails.
            if let Some(d) = date_attr {
                if let Some(serial) = iso_date_to_serial(d) {
                    CellValue::Date(serial)
                } else {
                    CellValue::Text(d.to_string())
                }
            } else {
                CellValue::Empty
            }
        }
        _ => {
            if !text_content.is_empty() {
                CellValue::Text(text_content.to_string())
            } else {
                CellValue::Empty
            }
        }
    }
}

/// Convert an ODS formula from OpenFormula syntax to standard spreadsheet syntax.
///
/// ODS formulas look like: `of:=[.A1]+[.B1]` or `of:=SUM([.A1:.A10])`
/// We convert to: `A1+B1` and `SUM(A1:A10)`.
fn convert_ods_formula(formula: &str) -> String {
    // Strip the "of:=" or "oooc:=" prefix
    let stripped = if let Some(rest) = formula.strip_prefix("of:=") {
        rest
    } else if let Some(rest) = formula.strip_prefix("oooc:=") {
        rest
    } else if let Some(rest) = formula.strip_prefix("msoxl:=") {
        rest
    } else {
        formula.strip_prefix('=').unwrap_or(formula)
    };

    // Replace [.A1] references: remove brackets and leading dots
    let mut result = String::with_capacity(stripped.len());
    let chars: Vec<char> = stripped.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            // Find matching ]
            i += 1;
            // Skip leading dot
            if i < chars.len() && chars[i] == '.' {
                i += 1;
            }
            while i < chars.len() && chars[i] != ']' {
                // Handle range separator: [.A1:.A10] → A1:A10
                if chars[i] == ':' {
                    result.push(':');
                    i += 1;
                    // Skip the dot after the colon
                    if i < chars.len() && chars[i] == '.' {
                        i += 1;
                    }
                } else {
                    result.push(chars[i]);
                    i += 1;
                }
            }
            if i < chars.len() {
                i += 1; // skip ']'
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Convert a standard formula to ODS OpenFormula syntax.
///
/// `A1+B1` → `of:=[.A1]+[.B1]`
/// `SUM(A1:A10)` → `of:=SUM([.A1:.A10])`
fn convert_to_ods_formula(formula: &str) -> String {
    let mut result = String::from("of:=");
    let chars: Vec<char> = formula.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Check for cell reference: letter(s) followed by digit(s), possibly with $
        if ch == '$' || ch.is_ascii_alphabetic() {
            let start = i;
            // Scan a potential cell reference
            let mut j = i;
            // Skip optional $
            if j < len && chars[j] == '$' {
                j += 1;
            }
            // Column letters
            let col_start = j;
            while j < len && chars[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j > col_start {
                // Skip optional $
                if j < len && chars[j] == '$' {
                    j += 1;
                }
                // Row digits
                let row_start = j;
                while j < len && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j > row_start {
                    // We have a cell reference
                    let ref_str: String = chars[start..j].iter().collect();

                    // Check if this is a function name (followed by '(')
                    // If there are only letters and the next char is '(', it's a function
                    let is_function = j < len
                        && chars[j] == '('
                        && chars[start..j].iter().all(|c| c.is_ascii_alphabetic());

                    if is_function {
                        // It's a function name, not a cell ref
                        result.push_str(&ref_str);
                        i = j;
                        continue;
                    }

                    // Check for range: A1:B2
                    if j < len && chars[j] == ':' {
                        j += 1;
                        let range_start = j;
                        // Skip optional $
                        if j < len && chars[j] == '$' {
                            j += 1;
                        }
                        let col2_start = j;
                        while j < len && chars[j].is_ascii_alphabetic() {
                            j += 1;
                        }
                        if j > col2_start {
                            if j < len && chars[j] == '$' {
                                j += 1;
                            }
                            let row2_start = j;
                            while j < len && chars[j].is_ascii_digit() {
                                j += 1;
                            }
                            if j > row2_start {
                                let ref2_str: String = chars[range_start..j].iter().collect();
                                result.push_str(&format!("[.{ref_str}:.{ref2_str}]"));
                                i = j;
                                continue;
                            }
                        }
                        // Not a valid range, just write the cell ref and the colon
                        result.push_str(&format!("[.{ref_str}]"));
                        result.push(':');
                        i = range_start;
                        continue;
                    }

                    result.push_str(&format!("[.{ref_str}]"));
                    i = j;
                    continue;
                }
            }

            // Not a cell ref — might be a function name or other identifier
            while i < len
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

/// Parse an ODS length string (e.g., "2.5cm", "72pt", "1in") to character units.
/// We approximate: 1 character unit ~ 7pt.
fn parse_ods_length(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(rest) = s.strip_suffix("cm") {
        let cm: f64 = rest.parse().ok()?;
        // 1cm = 28.3465pt, 1 char unit ≈ 7pt
        Some(cm * 28.3465 / 7.0)
    } else if let Some(rest) = s.strip_suffix("mm") {
        let mm: f64 = rest.parse().ok()?;
        Some(mm * 2.83465 / 7.0)
    } else if let Some(rest) = s.strip_suffix("in") {
        let inches: f64 = rest.parse().ok()?;
        Some(inches * 72.0 / 7.0)
    } else if let Some(rest) = s.strip_suffix("pt") {
        let pt: f64 = rest.parse().ok()?;
        Some(pt / 7.0)
    } else {
        // Try as plain number (assume pt)
        let n: f64 = s.parse().ok()?;
        Some(n / 7.0)
    }
}

/// Convert an ISO 8601 date to an Excel serial number.
/// Day 1 = 1900-01-01 (serial 1). Known Excel quirk: 1900-02-29 exists (serial 60).
fn iso_date_to_serial(date_str: &str) -> Option<f64> {
    // Parse YYYY-MM-DD or YYYY-MM-DDThh:mm:ss
    let date_part = date_str.split('T').next()?;
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    // Compute days since 1900-01-01 (Excel epoch)
    // Use a simple algorithm
    let serial = days_from_epoch(year, month, day)?;

    // Add time fraction if present
    if let Some(time_part) = date_str.split('T').nth(1) {
        let time_parts: Vec<&str> = time_part.split(':').collect();
        if time_parts.len() >= 2 {
            let h: f64 = time_parts[0].parse().ok()?;
            let m: f64 = time_parts[1].parse().ok()?;
            let s: f64 = if time_parts.len() > 2 {
                time_parts[2].trim_end_matches('Z').parse().unwrap_or(0.0)
            } else {
                0.0
            };
            return Some(serial as f64 + (h * 3600.0 + m * 60.0 + s) / 86400.0);
        }
    }

    Some(serial as f64)
}

/// Compute Excel serial date from year/month/day.
fn days_from_epoch(year: i32, month: u32, day: u32) -> Option<i64> {
    // Excel epoch: 1900-01-01 = serial 1
    // We compute Julian Day Numbers and subtract
    fn jdn(y: i32, m: u32, d: u32) -> i64 {
        let a = (14_i64 - m as i64) / 12;
        let yy = y as i64 + 4800 - a;
        let mm = m as i64 + 12 * a - 3;
        d as i64 + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
    }

    let epoch_jdn = jdn(1899, 12, 31); // serial 0
    let target_jdn = jdn(year, month, day);
    let serial = target_jdn - epoch_jdn;

    // Excel bug: it thinks 1900 is a leap year, so dates after Feb 28, 1900
    // need an extra day added
    if serial > 59 {
        Some(serial + 1)
    } else {
        Some(serial)
    }
}

/// Convert an Excel serial number back to ISO 8601 date string.
fn serial_to_iso_date(serial: f64) -> String {
    let mut day_serial = serial as i64;

    // Undo Excel's leap year bug
    if day_serial > 60 {
        day_serial -= 1;
    } else if day_serial == 60 {
        // This is the bogus Feb 29, 1900
        return "1900-02-29".to_string();
    }

    // day_serial 1 = 1900-01-01
    fn jdn_from_serial(serial: i64) -> i64 {
        fn jdn(y: i32, m: u32, d: u32) -> i64 {
            let a = (14_i64 - m as i64) / 12;
            let yy = y as i64 + 4800 - a;
            let mm = m as i64 + 12 * a - 3;
            d as i64 + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
        }
        let epoch = jdn(1899, 12, 31);
        epoch + serial
    }

    let j = jdn_from_serial(day_serial);
    // Convert JDN to calendar date
    let f = j + 1401 + (((4 * j + 274277) / 146097) * 3) / 4 - 38;
    let e = 4 * f + 3;
    let g = (e % 1461) / 4;
    let h = 5 * g + 2;
    let day = (h % 153) / 5 + 1;
    let month = ((h / 153 + 2) % 12) + 1;
    let year = e / 1461 - 4716 + (14 - month) / 12;

    format!("{year:04}-{month:02}-{day:02}")
}

/// Match the local element name from a `BytesStart` against a byte string.
fn is_element(e: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> bool {
    e.local_name().as_ref() == name
}

/// Match the local element name from a `BytesEnd` against a byte string.
fn is_end_element(e: &quick_xml::events::BytesEnd<'_>, name: &[u8]) -> bool {
    e.local_name().as_ref() == name
}

/// Get an attribute value from an ODS element (may have namespace prefix).
fn attr_str_ods(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    // ODS attributes use namespace prefixes like "office:value-type".
    // quick-xml gives us the full name including prefix.
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == key {
            return attr.unescape_value().ok().map(|v| v.to_string());
        }
    }
    // Also try matching just the local part (after the colon)
    let key_str = std::str::from_utf8(key).unwrap_or("");
    let local_key = key_str.rsplit(':').next().unwrap_or(key_str);
    for attr in e.attributes().flatten() {
        let attr_name = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let attr_local = attr_name.rsplit(':').next().unwrap_or(attr_name);
        if attr_local == local_key && attr_name.contains(':') {
            return attr.unescape_value().ok().map(|v| v.to_string());
        }
    }
    None
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

// ─── ODS Writer ───────────────────────────────────────────

/// Write a Workbook to ODS bytes.
///
/// # Errors
///
/// Returns an error if the ZIP archive cannot be written.
pub fn write_ods(workbook: &Workbook) -> Result<Vec<u8>, XlsxError> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);

    // 1. mimetype (must be first entry, uncompressed, no extra field)
    let options_stored =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let options_deflated =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("mimetype", options_stored)?;
    zip.write_all(b"application/vnd.oasis.opendocument.spreadsheet")?;

    // 2. META-INF/manifest.xml
    zip.start_file("META-INF/manifest.xml", options_deflated)?;
    zip.write_all(generate_manifest().as_bytes())?;

    // 3. meta.xml
    zip.start_file("meta.xml", options_deflated)?;
    zip.write_all(generate_meta(workbook).as_bytes())?;

    // 4. styles.xml
    zip.start_file("styles.xml", options_deflated)?;
    zip.write_all(generate_styles_xml().as_bytes())?;

    // 5. content.xml (the main data)
    zip.start_file("content.xml", options_deflated)?;
    zip.write_all(generate_content_xml(workbook).as_bytes())?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

fn generate_manifest() -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">"#);
    xml.push_str(r#"<manifest:file-entry manifest:full-path="/" manifest:version="1.2" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>"#);
    xml.push_str(
        r#"<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>"#,
    );
    xml.push_str(
        r#"<manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>"#,
    );
    xml.push_str(
        r#"<manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>"#,
    );
    xml.push_str("</manifest:manifest>");
    xml
}

fn generate_meta(workbook: &Workbook) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(
        r#"<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" office:version="1.2">"#,
    );
    xml.push_str("<office:meta>");
    if let Some(ref title) = workbook.metadata.title {
        xml.push_str(&format!(
            "<meta:title>{}</meta:title>",
            quick_xml::escape::escape(title)
        ));
    }
    if let Some(ref author) = workbook.metadata.author {
        xml.push_str(&format!(
            "<meta:initial-creator>{}</meta:initial-creator>",
            quick_xml::escape::escape(author)
        ));
    }
    xml.push_str("</office:meta>");
    xml.push_str("</office:document-meta>");
    xml
}

fn generate_styles_xml() -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(
        r#"<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.2">"#,
    );
    xml.push_str("<office:styles/>");
    xml.push_str("<office:automatic-styles/>");
    xml.push_str("</office:document-styles>");
    xml
}

fn generate_content_xml(workbook: &Workbook) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(
        r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" "#,
    );
    xml.push_str(r#"xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" "#);
    xml.push_str(r#"xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" "#);
    xml.push_str(r#"xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" "#);
    xml.push_str(r#"xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" "#);
    xml.push_str(r#"xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2" "#);
    xml.push_str(r#"office:version="1.2">"#);

    // Automatic styles for column widths
    xml.push_str("<office:automatic-styles>");
    let mut col_style_idx = 0u32;
    // We'll collect column width styles per-sheet
    let mut all_col_styles: Vec<(String, f64)> = Vec::new();
    for (si, sheet) in workbook.sheets.iter().enumerate() {
        for (&col, &width) in &sheet.column_widths {
            let style_name = format!("co_s{si}_c{col}");
            // Convert character width to cm (1 char unit ~ 7pt, 1cm = 28.3465pt)
            let cm = width * 7.0 / 28.3465;
            all_col_styles.push((style_name.clone(), cm));
            col_style_idx += 1;
        }
    }
    for (name, cm) in &all_col_styles {
        xml.push_str(&format!(
            r#"<style:style style:name="{name}" style:family="table-column"><style:table-column-properties style:column-width="{cm:.4}cm"/></style:style>"#
        ));
    }
    let _ = col_style_idx;
    xml.push_str("</office:automatic-styles>");

    xml.push_str("<office:body><office:spreadsheet>");

    for (si, sheet) in workbook.sheets.iter().enumerate() {
        xml.push_str(&format!(
            r#"<table:table table:name="{}">"#,
            quick_xml::escape::escape(&sheet.name)
        ));

        // Column definitions
        let (max_col, max_row) = sheet.dimensions();
        for c in 0..max_col {
            if let Some(&_width) = sheet.column_widths.get(&c) {
                let style_name = format!("co_s{si}_c{c}");
                xml.push_str(&format!(
                    r#"<table:table-column table:style-name="{style_name}"/>"#
                ));
            } else {
                xml.push_str("<table:table-column/>");
            }
        }

        // Rows
        for r in 0..max_row {
            // Check for row height
            if let Some(&ht) = sheet.row_heights.get(&r) {
                // Convert points to cm
                let cm = ht / 28.3465;
                xml.push_str(&format!(
                    r#"<table:table-row table:style-name="ro_r{r}" fo:min-height="{cm:.4}cm">"#
                ));
            } else {
                xml.push_str("<table:table-row>");
            }

            let mut c = 0u32;
            while c < max_col {
                if let Some(cell) = sheet.get(c, r) {
                    write_ods_cell(&mut xml, cell);
                } else {
                    xml.push_str("<table:table-cell/>");
                }
                c += 1;
            }

            xml.push_str("</table:table-row>");
        }

        // Merged cells — ODS uses table:number-columns-spanned and table:number-rows-spanned
        // on the cell itself. Since we write cells sequentially, we'd need a pre-pass.
        // For now, we skip merge info in ODS output (it's preserved in the model for XLSX).

        xml.push_str("</table:table>");
    }

    xml.push_str("</office:spreadsheet></office:body></office:document-content>");
    xml
}

/// Write a single cell in ODS format.
fn write_ods_cell(xml: &mut String, cell: &Cell) {
    match &cell.value {
        CellValue::Number(n) => {
            let formula_attr = cell
                .formula
                .as_ref()
                .map(|f| format!(r#" table:formula="{}""#, convert_to_ods_formula(f)))
                .unwrap_or_default();
            xml.push_str(&format!(
                r#"<table:table-cell office:value-type="float" office:value="{n}"{formula_attr}>"#,
            ));
            xml.push_str(&format!("<text:p>{n}</text:p>"));
            xml.push_str("</table:table-cell>");
        }
        CellValue::Text(s) => {
            let formula_attr = cell
                .formula
                .as_ref()
                .map(|f| format!(r#" table:formula="{}""#, convert_to_ods_formula(f)))
                .unwrap_or_default();
            xml.push_str(&format!(
                r#"<table:table-cell office:value-type="string"{formula_attr}>"#
            ));
            // Handle multiline text: each line gets its own <text:p>
            for line in s.split('\n') {
                xml.push_str(&format!(
                    "<text:p>{}</text:p>",
                    quick_xml::escape::escape(line)
                ));
            }
            xml.push_str("</table:table-cell>");
        }
        CellValue::Boolean(b) => {
            xml.push_str(&format!(
                r#"<table:table-cell office:value-type="boolean" office:boolean-value="{}">"#,
                if *b { "true" } else { "false" }
            ));
            xml.push_str(&format!(
                "<text:p>{}</text:p>",
                if *b { "TRUE" } else { "FALSE" }
            ));
            xml.push_str("</table:table-cell>");
        }
        CellValue::Date(serial) => {
            let iso = serial_to_iso_date(*serial);
            xml.push_str(&format!(
                r#"<table:table-cell office:value-type="date" office:date-value="{iso}">"#,
            ));
            xml.push_str(&format!("<text:p>{iso}</text:p>"));
            xml.push_str("</table:table-cell>");
        }
        CellValue::Error(e) => {
            // Write errors as text
            xml.push_str(r#"<table:table-cell office:value-type="string">"#);
            xml.push_str(&format!("<text:p>{e}</text:p>"));
            xml.push_str("</table:table-cell>");
        }
        CellValue::Empty => {
            xml.push_str("<table:table-cell/>");
        }
    }
}

// ─── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_formula_from_ods() {
        assert_eq!(convert_ods_formula("of:=[.A1]+[.B1]"), "A1+B1");
        assert_eq!(convert_ods_formula("of:=SUM([.A1:.A10])"), "SUM(A1:A10)");
        assert_eq!(
            convert_ods_formula("of:=IF([.A1]>5,\"big\",\"small\")"),
            "IF(A1>5,\"big\",\"small\")"
        );
        assert_eq!(convert_ods_formula("of:=[.B2]*[.C2]"), "B2*C2");
    }

    #[test]
    fn convert_formula_to_ods() {
        assert_eq!(convert_to_ods_formula("A1+B1"), "of:=[.A1]+[.B1]");
        assert_eq!(convert_to_ods_formula("SUM(A1:A10)"), "of:=SUM([.A1:.A10])");
        assert_eq!(convert_to_ods_formula("B2*C2"), "of:=[.B2]*[.C2]");
    }

    #[test]
    fn formula_roundtrip() {
        let formulas = ["A1+B1", "SUM(A1:A10)", "B2*C2", "100"];
        for f in formulas {
            let ods = convert_to_ods_formula(f);
            let back = convert_ods_formula(&ods);
            assert_eq!(back, f, "Formula roundtrip failed for {f}");
        }
    }

    #[test]
    fn resolve_float_value() {
        let val = resolve_ods_value(Some("float"), Some("42.5"), None, None, "42.5");
        assert_eq!(val, CellValue::Number(42.5));
    }

    #[test]
    fn resolve_string_value() {
        let val = resolve_ods_value(Some("string"), None, None, None, "Hello");
        assert_eq!(val, CellValue::Text("Hello".to_string()));
    }

    #[test]
    fn resolve_boolean_value() {
        let val = resolve_ods_value(Some("boolean"), None, Some("true"), None, "");
        assert_eq!(val, CellValue::Boolean(true));

        let val2 = resolve_ods_value(Some("boolean"), None, Some("false"), None, "");
        assert_eq!(val2, CellValue::Boolean(false));
    }

    #[test]
    fn resolve_date_value() {
        let val = resolve_ods_value(Some("date"), None, None, Some("2024-01-15"), "");
        match val {
            CellValue::Date(serial) => {
                // 2024-01-15 should be a positive serial number
                assert!(serial > 0.0);
            }
            other => panic!("Expected Date, got {other:?}"),
        }
    }

    #[test]
    fn iso_date_serial_roundtrip() {
        // 2024-01-01
        let serial = iso_date_to_serial("2024-01-01").unwrap();
        let iso = serial_to_iso_date(serial);
        assert_eq!(iso, "2024-01-01");

        // 1900-01-01 (serial 1)
        let serial = iso_date_to_serial("1900-01-01").unwrap();
        assert_eq!(serial, 1.0);
    }

    #[test]
    fn parse_ods_length_values() {
        let w = parse_ods_length("2.5cm").unwrap();
        assert!((w - 2.5 * 28.3465 / 7.0).abs() < 0.01);

        let w = parse_ods_length("72pt").unwrap();
        assert!((w - 72.0 / 7.0).abs() < 0.01);

        let w = parse_ods_length("1in").unwrap();
        assert!((w - 72.0 / 7.0).abs() < 0.01);
    }

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

        let bytes = write_ods(&wb).unwrap();
        assert!(bytes.len() > 100, "ODS should be non-trivial");

        // Read it back
        let wb2 = read_ods(&bytes).unwrap();
        assert_eq!(wb2.sheets.len(), 1);
        let s = &wb2.sheets[0];
        assert_eq!(s.name, "Sheet1");
        assert_eq!(s.get(0, 0).unwrap().value, CellValue::Text("Name".into()));
        assert_eq!(s.get(1, 1).unwrap().value, CellValue::Number(95.0));
        assert_eq!(s.get(0, 3).unwrap().value, CellValue::Boolean(true));
    }

    #[test]
    fn write_and_read_formulas() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Number(10.0));
        sheet.set(0, 1, CellValue::Number(20.0));
        sheet.set_formula(0, 2, "SUM(A1:A2)", CellValue::Number(30.0));

        let bytes = write_ods(&wb).unwrap();
        let wb2 = read_ods(&bytes).unwrap();
        let s = &wb2.sheets[0];
        assert_eq!(s.get(0, 2).unwrap().formula.as_deref(), Some("SUM(A1:A2)"));
        assert_eq!(s.get(0, 2).unwrap().value, CellValue::Number(30.0));
    }

    #[test]
    fn write_and_read_multiple_sheets() {
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

        let bytes = write_ods(&wb).unwrap();
        let wb2 = read_ods(&bytes).unwrap();
        assert_eq!(wb2.sheets.len(), 2);
        assert_eq!(wb2.sheets[0].name, "Sheet1");
        assert_eq!(wb2.sheets[1].name, "Sheet2");
        assert_eq!(
            wb2.sheets[0].get(0, 0).unwrap().value,
            CellValue::Text("First".into())
        );
        assert_eq!(
            wb2.sheets[1].get(0, 0).unwrap().value,
            CellValue::Text("Second".into())
        );
    }

    #[test]
    fn write_and_read_boolean_cells() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Boolean(true));
        sheet.set(1, 0, CellValue::Boolean(false));

        let bytes = write_ods(&wb).unwrap();
        let wb2 = read_ods(&bytes).unwrap();
        let s = &wb2.sheets[0];
        assert_eq!(s.get(0, 0).unwrap().value, CellValue::Boolean(true));
        assert_eq!(s.get(1, 0).unwrap().value, CellValue::Boolean(false));
    }

    #[test]
    fn write_and_read_mixed_types() {
        let mut wb = Workbook::new();
        let sheet = wb.sheets.first_mut().unwrap();
        sheet.set(0, 0, CellValue::Text("Header".into()));
        sheet.set(1, 0, CellValue::Number(42.0));
        sheet.set(2, 0, CellValue::Boolean(true));
        sheet.set(0, 1, CellValue::Number(3.14));
        sheet.set_formula(1, 1, "A2*2", CellValue::Number(6.28));

        let bytes = write_ods(&wb).unwrap();
        let wb2 = read_ods(&bytes).unwrap();
        let s = &wb2.sheets[0];
        assert_eq!(s.get(0, 0).unwrap().value, CellValue::Text("Header".into()));
        assert_eq!(s.get(1, 0).unwrap().value, CellValue::Number(42.0));
        assert_eq!(s.get(2, 0).unwrap().value, CellValue::Boolean(true));
        assert_eq!(s.get(0, 1).unwrap().value, CellValue::Number(3.14));
        assert_eq!(s.get(1, 1).unwrap().formula.as_deref(), Some("A2*2"));
    }

    #[test]
    fn ods_mimetype_is_first_entry() {
        let wb = Workbook::new();
        let bytes = write_ods(&wb).unwrap();

        // Verify mimetype is the first ZIP entry
        let cursor = Cursor::new(&bytes);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let first = archive.by_index(0).unwrap();
        assert_eq!(first.name(), "mimetype");
    }

    #[test]
    fn empty_workbook_ods_roundtrip() {
        let wb = Workbook::new();
        let bytes = write_ods(&wb).unwrap();
        let wb2 = read_ods(&bytes).unwrap();
        assert_eq!(wb2.sheets.len(), 1);
        assert_eq!(wb2.sheets[0].name, "Sheet1");
        assert_eq!(wb2.sheets[0].cells.len(), 0);
    }
}
