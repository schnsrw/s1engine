//! Spreadsheet data model — the in-memory representation of an XLSX workbook.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

/// A complete workbook with multiple sheets.
#[derive(Debug, Clone, Default)]
pub struct Workbook {
    /// Ordered list of sheets.
    pub sheets: Vec<Sheet>,
    /// Shared string table (deduplicated text values).
    pub shared_strings: Vec<String>,
    /// Style definitions.
    pub styles: StyleSheet,
    /// Defined names (named ranges, print areas).
    pub defined_names: Vec<DefinedName>,
    /// Workbook metadata.
    pub metadata: WorkbookMetadata,
    /// Preserved ZIP entries not recognized by the reader (round-trip fidelity).
    pub preserved_parts: HashMap<String, Vec<u8>>,
}

/// A single worksheet.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    /// Sheet name (tab label).
    pub name: String,
    /// Sparse cell storage keyed by (col, row).
    pub cells: BTreeMap<CellRef, Cell>,
    /// Column widths in character units (default ~8.43).
    pub column_widths: BTreeMap<u32, f64>,
    /// Row heights in points.
    pub row_heights: BTreeMap<u32, f64>,
    /// Merged cell ranges.
    pub merged_cells: Vec<CellRange>,
    /// Frozen pane position.
    pub frozen_pane: Option<CellRef>,
    /// Sheet-level tab color (hex RGB).
    pub tab_color: Option<String>,
}

/// A single cell.
#[derive(Debug, Clone)]
pub struct Cell {
    /// The cell's value.
    pub value: CellValue,
    /// Formula string (e.g., "SUM(A1:A10)"), if any.
    pub formula: Option<String>,
    /// Style index (references StyleSheet.cell_formats).
    pub style_id: u32,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            value: CellValue::Empty,
            formula: None,
            style_id: 0,
        }
    }
}

/// Cell value types matching OOXML cell types.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CellValue {
    /// No value.
    #[default]
    Empty,
    /// Text string.
    Text(String),
    /// Numeric value (f64 covers integers and floats).
    Number(f64),
    /// Boolean (TRUE/FALSE).
    Boolean(bool),
    /// Error value (#DIV/0!, #VALUE!, etc.).
    Error(CellError),
    /// Date/time (stored as serial number in XLSX, converted here).
    Date(f64),
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellValue::Empty => write!(f, ""),
            CellValue::Text(s) => write!(f, "{s}"),
            CellValue::Number(n) => {
                if *n == (*n as i64) as f64 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            CellValue::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            CellValue::Error(e) => write!(f, "{e}"),
            CellValue::Date(serial) => write!(f, "{serial}"),
        }
    }
}

/// Spreadsheet error values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellError {
    DivZero,
    Value,
    Ref,
    Name,
    Num,
    NA,
    Null,
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellError::DivZero => write!(f, "#DIV/0!"),
            CellError::Value => write!(f, "#VALUE!"),
            CellError::Ref => write!(f, "#REF!"),
            CellError::Name => write!(f, "#NAME?"),
            CellError::Num => write!(f, "#NUM!"),
            CellError::NA => write!(f, "#N/A"),
            CellError::Null => write!(f, "#NULL!"),
        }
    }
}

/// Cell reference (column + row, both 0-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellRef {
    pub col: u32,
    pub row: u32,
}

impl CellRef {
    pub fn new(col: u32, row: u32) -> Self {
        Self { col, row }
    }

    /// Parse "A1" style reference.
    pub fn from_a1(s: &str) -> Option<Self> {
        let s = s.trim();
        let mut col: u32 = 0;
        let mut row_start = 0;
        for (i, ch) in s.chars().enumerate() {
            if ch.is_ascii_alphabetic() {
                col = col * 26 + (ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1);
                row_start = i + 1;
            } else {
                break;
            }
        }
        if col == 0 || row_start == 0 {
            return None;
        }
        let row: u32 = s[row_start..].parse().ok()?;
        if row == 0 {
            return None;
        }
        Some(Self {
            col: col - 1,
            row: row - 1,
        })
    }

    /// Convert to "A1" style string.
    pub fn to_a1(&self) -> String {
        let mut col_str = String::new();
        let mut c = self.col + 1;
        while c > 0 {
            c -= 1;
            col_str.insert(0, (b'A' + (c % 26) as u8) as char);
            c /= 26;
        }
        format!("{}{}", col_str, self.row + 1)
    }
}

/// A range of cells (e.g., A1:C10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    pub start: CellRef,
    pub end: CellRef,
}

impl CellRange {
    pub fn new(start: CellRef, end: CellRef) -> Self {
        Self { start, end }
    }

    /// Parse "A1:C10" style range.
    pub fn from_a1(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        Some(Self {
            start: CellRef::from_a1(parts[0])?,
            end: CellRef::from_a1(parts[1])?,
        })
    }
}

/// Cell style definitions.
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    /// Number format definitions.
    pub number_formats: Vec<NumberFormat>,
    /// Font definitions.
    pub fonts: Vec<FontDef>,
    /// Fill definitions.
    pub fills: Vec<FillDef>,
    /// Border definitions.
    pub borders: Vec<BorderDef>,
    /// Combined cell format entries (cellXfs).
    pub cell_formats: Vec<CellFormat>,
}

#[derive(Debug, Clone)]
pub struct NumberFormat {
    pub id: u32,
    pub format_code: String,
}

#[derive(Debug, Clone, Default)]
pub struct FontDef {
    pub name: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FillDef {
    pub pattern: String,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BorderDef {
    pub left: Option<BorderSide>,
    pub right: Option<BorderSide>,
    pub top: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
}

#[derive(Debug, Clone)]
pub struct BorderSide {
    pub style: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CellFormat {
    pub number_format_id: u32,
    pub font_id: u32,
    pub fill_id: u32,
    pub border_id: u32,
    pub alignment: Option<CellAlignment>,
}

#[derive(Debug, Clone)]
pub struct CellAlignment {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap_text: bool,
}

/// A defined name (named range, print area, etc.).
#[derive(Debug, Clone)]
pub struct DefinedName {
    pub name: String,
    pub value: String,
    pub sheet_index: Option<u32>,
}

/// Workbook-level metadata.
#[derive(Debug, Clone, Default)]
pub struct WorkbookMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub active_sheet: u32,
}

// ─── Workbook convenience methods ────────────────────

impl Workbook {
    /// Create a new empty workbook with one sheet.
    pub fn new() -> Self {
        Self {
            sheets: vec![Sheet {
                name: "Sheet1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Get a sheet by name.
    pub fn sheet_by_name(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }

    /// Get a mutable sheet by name.
    pub fn sheet_by_name_mut(&mut self, name: &str) -> Option<&mut Sheet> {
        self.sheets.iter_mut().find(|s| s.name == name)
    }

    /// Export as CSV (first sheet only).
    pub fn to_csv(&self, delimiter: char) -> String {
        let sheet = match self.sheets.first() {
            Some(s) => s,
            None => return String::new(),
        };
        sheet.to_csv(delimiter)
    }
}

impl Sheet {
    /// Get a cell value.
    pub fn get(&self, col: u32, row: u32) -> Option<&Cell> {
        self.cells.get(&CellRef::new(col, row))
    }

    /// Set a cell value.
    pub fn set(&mut self, col: u32, row: u32, value: CellValue) {
        self.cells.insert(
            CellRef::new(col, row),
            Cell {
                value,
                formula: None,
                style_id: 0,
            },
        );
    }

    /// Set a cell with formula.
    pub fn set_formula(&mut self, col: u32, row: u32, formula: &str, cached_value: CellValue) {
        self.cells.insert(
            CellRef::new(col, row),
            Cell {
                value: cached_value,
                formula: Some(formula.to_string()),
                style_id: 0,
            },
        );
    }

    /// Get the dimensions (max col, max row) of used cells.
    pub fn dimensions(&self) -> (u32, u32) {
        let mut max_col = 0u32;
        let mut max_row = 0u32;
        for ref_cell in self.cells.keys() {
            max_col = max_col.max(ref_cell.col + 1);
            max_row = max_row.max(ref_cell.row + 1);
        }
        (max_col, max_row)
    }

    /// Export sheet as CSV.
    pub fn to_csv(&self, delimiter: char) -> String {
        let (cols, rows) = self.dimensions();
        let mut out = String::new();
        for r in 0..rows {
            for c in 0..cols {
                if c > 0 {
                    out.push(delimiter);
                }
                if let Some(cell) = self.get(c, r) {
                    let text = cell.value.to_string();
                    // Quote if contains delimiter, newline, or quote
                    if text.contains(delimiter)
                        || text.contains('\n')
                        || text.contains('\r')
                        || text.contains('"')
                    {
                        out.push('"');
                        out.push_str(&text.replace('"', "\"\""));
                        out.push('"');
                    } else {
                        out.push_str(&text);
                    }
                }
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_ref_from_a1() {
        assert_eq!(CellRef::from_a1("A1"), Some(CellRef::new(0, 0)));
        assert_eq!(CellRef::from_a1("B2"), Some(CellRef::new(1, 1)));
        assert_eq!(CellRef::from_a1("Z1"), Some(CellRef::new(25, 0)));
        assert_eq!(CellRef::from_a1("AA1"), Some(CellRef::new(26, 0)));
        assert_eq!(CellRef::from_a1("AZ1"), Some(CellRef::new(51, 0)));
        assert_eq!(CellRef::from_a1("BA1"), Some(CellRef::new(52, 0)));
        assert_eq!(
            CellRef::from_a1("XFD1048576"),
            Some(CellRef::new(16383, 1048575))
        );
    }

    #[test]
    fn cell_ref_to_a1() {
        assert_eq!(CellRef::new(0, 0).to_a1(), "A1");
        assert_eq!(CellRef::new(1, 1).to_a1(), "B2");
        assert_eq!(CellRef::new(25, 0).to_a1(), "Z1");
        assert_eq!(CellRef::new(26, 0).to_a1(), "AA1");
        assert_eq!(CellRef::new(52, 0).to_a1(), "BA1");
    }

    #[test]
    fn cell_ref_roundtrip() {
        for col in [0, 1, 25, 26, 51, 52, 701, 702, 16383] {
            for row in [0, 1, 99, 1048575] {
                let r = CellRef::new(col, row);
                let a1 = r.to_a1();
                let parsed = CellRef::from_a1(&a1).unwrap();
                assert_eq!(r, parsed, "Failed for {a1}");
            }
        }
    }

    #[test]
    fn cell_range_parse() {
        let range = CellRange::from_a1("A1:C10").unwrap();
        assert_eq!(range.start, CellRef::new(0, 0));
        assert_eq!(range.end, CellRef::new(2, 9));
    }

    #[test]
    fn sheet_csv_export() {
        let mut sheet = Sheet::default();
        sheet.set(0, 0, CellValue::Text("Name".into()));
        sheet.set(1, 0, CellValue::Text("Age".into()));
        sheet.set(0, 1, CellValue::Text("Alice".into()));
        sheet.set(1, 1, CellValue::Number(30.0));
        sheet.set(0, 2, CellValue::Text("Bob, Jr.".into()));
        sheet.set(1, 2, CellValue::Number(25.0));

        let csv = sheet.to_csv(',');
        assert!(csv.contains("Name,Age"));
        assert!(csv.contains("Alice,30"));
        assert!(csv.contains("\"Bob, Jr.\",25"));
    }

    #[test]
    fn cell_error_display() {
        assert_eq!(CellError::DivZero.to_string(), "#DIV/0!");
        assert_eq!(CellError::NA.to_string(), "#N/A");
    }
}
