# Spreadsheet Format Specification v1.0

> Covers: XLSX (OOXML SpreadsheetML), ODS (OpenDocument Spreadsheet), CSV, TSV
> Reference: ECMA-376 Part 1, ISO/IEC 29500, ODF 1.2, RFC 4180 (CSV)
> Last updated: 2026-03-21

## 1. Format Overview

| Format | Standard | Structure | Formulas | Styles | Multi-sheet | Charts |
|--------|----------|-----------|----------|--------|-------------|--------|
| XLSX | ECMA-376 / ISO 29500 | ZIP + XML | Yes (600+ functions) | Full | Yes | Yes |
| ODS | ODF 1.2 / ISO 26300 | ZIP + XML | Yes (400+ functions) | Full | Yes | Yes |
| CSV | RFC 4180 | Plain text | No | No | No | No |
| TSV | LOC FDD 533 | Plain text | No | No | No | No |
| XLS | MS-XLS (binary) | OLE2 compound | Yes | Full | Yes | Yes |

## 2. XLSX (SpreadsheetML) Structure

### 2.1 ZIP Package
```
[Content_Types].xml
_rels/.rels
xl/workbook.xml           ← Workbook: sheet list, defined names
xl/worksheets/sheet1.xml  ← Sheet data: rows, cells, formulas
xl/worksheets/sheet2.xml
xl/sharedStrings.xml      ← Shared string table (deduplicated)
xl/styles.xml             ← Cell styles, number formats, fonts, fills, borders
xl/theme/theme1.xml       ← Color theme
xl/charts/chart1.xml      ← Chart definitions
xl/drawings/drawing1.xml  ← Drawing anchors
xl/tables/table1.xml      ← Structured table definitions
xl/_rels/workbook.xml.rels
docProps/core.xml          ← Metadata
docProps/app.xml
```

### 2.2 Cell Model
```xml
<row r="1">
  <c r="A1" t="s">      <!-- t="s": shared string, "n": number, "b": boolean, "e": error, "str": formula string -->
    <v>0</v>             <!-- Index into sharedStrings.xml -->
  </c>
  <c r="B1" t="n" s="4"> <!-- s="4": style index from styles.xml -->
    <v>42.5</v>
  </c>
  <c r="C1">
    <f>A1+B1</f>         <!-- Formula -->
    <v>42.5</v>          <!-- Cached result -->
  </c>
</row>
```

### 2.3 Cell Types
| Type Code | Name | Storage | Example |
|-----------|------|---------|---------|
| `s` | Shared String | Index into `sharedStrings.xml` | `"Hello"` |
| `n` | Number | Double-precision float in `<v>` | `42.5` |
| `b` | Boolean | `0` or `1` in `<v>` | `TRUE` |
| `e` | Error | Error code in `<v>` | `#DIV/0!` |
| `str` | Inline String | String directly in `<v>` | `"Inline"` |
| (none) | Number (default) | Double in `<v>` | `100` |
| `d` | Date (ISO 8601) | Date string in `<v>` | `2026-03-21` |

### 2.4 Formula Categories (for engine)
| Category | Functions | Priority |
|----------|-----------|----------|
| Arithmetic | `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT` | P0 (MVP) |
| Logic | `IF`, `AND`, `OR`, `NOT`, `IFERROR` | P0 |
| Lookup | `VLOOKUP`, `HLOOKUP`, `INDEX`, `MATCH` | P1 |
| Text | `CONCATENATE`, `LEFT`, `RIGHT`, `MID`, `LEN`, `TRIM` | P1 |
| Date/Time | `NOW`, `TODAY`, `DATE`, `YEAR`, `MONTH`, `DAY` | P1 |
| Statistical | `COUNTIF`, `SUMIF`, `AVERAGEIF`, `STDEV` | P2 |
| Financial | `PMT`, `FV`, `PV`, `NPV`, `IRR` | P3 |
| Math | `ROUND`, `ABS`, `INT`, `MOD`, `POWER`, `SQRT` | P1 |
| Reference | `ROW`, `COLUMN`, `INDIRECT`, `OFFSET` | P2 |
| Array | `SORT`, `FILTER`, `UNIQUE`, `SEQUENCE` (365) | P3 |

### 2.5 Style Model
```
styles.xml contains:
  numFmts[]    → Number formats (date, currency, percentage, custom)
  fonts[]      → Font definitions (name, size, bold, italic, color)
  fills[]      → Fill patterns (solid, pattern, gradient)
  borders[]    → Border definitions (thin, thick, double, color per side)
  cellXfs[]    → Cell format combinations (font + fill + border + numFmt + alignment)

Each cell references a style via s="index" attribute → cellXfs[index]
```

## 3. ODS (OpenDocument Spreadsheet)

### 3.1 ZIP Package
```
mimetype                      ← "application/vnd.oasis.opendocument.spreadsheet"
content.xml                   ← Sheet data + automatic styles
styles.xml                    ← Named styles
meta.xml                      ← Metadata
settings.xml                  ← Application settings
META-INF/manifest.xml
```

### 3.2 Cell Model
```xml
<table:table-cell table:style-name="ce1" office:value-type="float" office:value="42.5">
  <text:p>42.5</text:p>
</table:table-cell>

<table:table-cell table:formula="of:=[.A1]+[.B1]" office:value-type="float" office:value="42.5">
  <text:p>42.5</text:p>
</table:table-cell>
```

### 3.3 ODS vs XLSX Differences
| Feature | XLSX | ODS |
|---------|------|-----|
| Formula prefix | None | `of:=` (OpenFormula) |
| Cell reference | `A1` | `[.A1]` (dot prefix) |
| Sheet reference | `Sheet1!A1` | `[Sheet1.A1]` |
| String storage | Shared strings table | Inline in cell |
| Date storage | Serial number (days since 1900) | ISO 8601 string |
| Style system | Indexed arrays | Named automatic styles |

## 4. CSV / TSV

### 4.1 CSV (RFC 4180)
```
Delimiter: comma (,)
Line ending: CRLF (\r\n)
Quoting: double-quote (") for fields containing delimiter, newline, or quote
Escaping: double the quote ("" inside quoted field)
Header: optional first row
Encoding: UTF-8 (recommended), may be Latin-1/Windows-1252
```

### 4.2 TSV
```
Delimiter: tab (\t)
Line ending: LF (\n) or CRLF (\r\n)
Quoting: generally not used (tabs in data are rare)
Encoding: UTF-8
```

### 4.3 CSV Parsing Edge Cases
| # | Case | Input | Expected Parse |
|---|------|-------|----------------|
| 1 | Quoted comma | `"hello, world"` | Single field: `hello, world` |
| 2 | Escaped quote | `"say ""hi"""` | Single field: `say "hi"` |
| 3 | Multiline field | `"line1\nline2"` | Single field with newline |
| 4 | Empty field | `a,,c` | Three fields: `a`, `""`, `c` |
| 5 | Trailing comma | `a,b,` | Three fields: `a`, `b`, `""` |
| 6 | BOM | `\xEF\xBB\xBFa,b` | Ignore BOM, two fields |
| 7 | Mixed line endings | `a\r\nb\nc` | Three rows |
| 8 | No trailing newline | `a,b` | One row |

## 5. Data Model (s1-spreadsheet)

### 5.1 Core Types
```rust
pub struct Workbook {
    pub sheets: Vec<Sheet>,
    pub shared_strings: Vec<String>,
    pub styles: StyleSheet,
    pub defined_names: Vec<DefinedName>,
    pub metadata: DocumentMetadata,
}

pub struct Sheet {
    pub name: String,
    pub cells: BTreeMap<CellRef, Cell>,  // Sparse storage
    pub column_widths: Vec<f64>,
    pub row_heights: BTreeMap<u32, f64>,
    pub merged_cells: Vec<CellRange>,
    pub tables: Vec<Table>,
    pub charts: Vec<ChartRef>,
    pub frozen_panes: Option<FrozenPane>,
}

pub struct Cell {
    pub value: CellValue,
    pub formula: Option<String>,
    pub style_id: u32,
}

pub enum CellValue {
    Empty,
    Text(String),
    Number(f64),
    Boolean(bool),
    Error(CellError),
    Date(NaiveDateTime),
}

pub struct CellRef {
    pub col: u32,  // 0-indexed (A=0, B=1, ...)
    pub row: u32,  // 0-indexed
}

pub struct CellRange {
    pub start: CellRef,
    pub end: CellRef,
}
```

### 5.2 Formula Engine
```rust
pub trait FormulaEngine {
    fn evaluate(&self, formula: &str, context: &SheetContext) -> CellValue;
    fn parse(&self, formula: &str) -> Result<FormulaAst, FormulaError>;
    fn get_dependencies(&self, formula: &str) -> Vec<CellRef>;
}

// Dependency graph for recalculation
pub struct DependencyGraph {
    // cell → cells that depend on it
    dependents: HashMap<CellRef, Vec<CellRef>>,
    // Topological sort for evaluation order
    eval_order: Vec<CellRef>,
}
```

## 6. Implementation Phases

### Phase 6a: CSV/TSV (Sprint 1) — Foundation
| Step | Description | Effort |
|------|-------------|--------|
| 1 | CSV parser: RFC 4180 compliant with edge cases | M |
| 2 | TSV parser: tab-delimited variant | S |
| 3 | CSV writer: export from data model | S |
| 4 | CSV → DOCX table conversion (existing) | DONE |
| 5 | Auto-detect delimiter (comma vs tab vs semicolon) | S |
| 6 | Encoding detection (UTF-8, Latin-1, BOM) | S |
| 7 | Large file streaming (don't load all into memory) | M |
| 8 | Round-trip tests (CSV → model → CSV) | S |

### Phase 6b: XLSX Reader (Sprint 2)
| Step | Description | Effort |
|------|-------------|--------|
| 1 | Create `s1-format-xlsx` crate with ZIP reader | M |
| 2 | Parse `xl/sharedStrings.xml` | S |
| 3 | Parse `xl/styles.xml` (number formats, fonts, fills, borders) | L |
| 4 | Parse `xl/worksheets/sheetN.xml` (rows, cells, types) | L |
| 5 | Parse `xl/workbook.xml` (sheet names, defined names) | S |
| 6 | Formula string extraction (not evaluation) | S |
| 7 | Merged cell ranges | S |
| 8 | Column widths + row heights | S |
| 9 | Frozen panes | S |
| 10 | Round-trip test framework | M |

### Phase 6c: XLSX Writer (Sprint 3)
| Step | Description | Effort |
|------|-------------|--------|
| 1 | Generate `sharedStrings.xml` | S |
| 2 | Generate `styles.xml` | L |
| 3 | Generate `worksheetN.xml` with cells | L |
| 4 | Generate `workbook.xml` with sheet refs | S |
| 5 | ZIP packaging with `[Content_Types].xml` and relationships | M |
| 6 | Preserve unrecognized XML for round-trip | M |
| 7 | Round-trip tests (XLSX → model → XLSX → compare) | M |

### Phase 6d: Formula Engine (Sprint 4)
| Step | Description | Effort |
|------|-------------|--------|
| 1 | Formula tokenizer (operators, functions, cell refs, strings, numbers) | L |
| 2 | Formula parser → AST | L |
| 3 | P0 functions: SUM, AVERAGE, MIN, MAX, COUNT, IF, AND, OR | L |
| 4 | Cell reference resolution (A1, $A$1, A1:B10, Sheet1!A1) | M |
| 5 | Dependency graph + topological sort for recalculation | L |
| 6 | Circular reference detection | M |
| 7 | P1 functions: VLOOKUP, TEXT, DATE, ROUND, etc. | L |
| 8 | Array formula support (basic) | L |

### Phase 6e: Grid UI (Sprint 5-6)
| Step | Description | Effort |
|------|-------------|--------|
| 1 | Virtual scrolling grid (canvas or DOM) | XL |
| 2 | Cell selection (single, range, multi-range) | L |
| 3 | Cell editing (double-click to edit, Enter/Tab to navigate) | L |
| 4 | Formula bar | M |
| 5 | Column/row resize | M |
| 6 | Column/row insert/delete | M |
| 7 | Cell formatting toolbar (number format, font, alignment, borders) | L |
| 8 | Copy/paste (cells, ranges, across sheets) | L |
| 9 | Undo/redo | M |
| 10 | Sheet tabs (add, rename, delete, reorder) | M |
| 11 | Freeze panes UI | S |
| 12 | Auto-fill (drag handle) | M |
| 13 | Sort + filter | L |
| 14 | Conditional formatting (basic rules) | L |

### Phase 6f: ODS Support (Sprint 7)
| Step | Description | Effort |
|------|-------------|--------|
| 1 | Create `s1-format-ods-spreadsheet` module or extend `s1-format-odt` | M |
| 2 | Parse `content.xml` table structures | L |
| 3 | Map ODF cell types to model | M |
| 4 | OpenFormula → internal formula conversion | L |
| 5 | Write ODS from model | L |
| 6 | Round-trip tests | M |

## 7. Conversion Matrix

| From \ To | XLSX | ODS | CSV | TSV | DOCX Table | PDF Table |
|-----------|------|-----|-----|-----|------------|-----------|
| XLSX | — | L | S | S | M | M |
| ODS | L | — | S | S | M | M |
| CSV | M | M | — | S | S (DONE) | S |
| TSV | M | M | S | — | S | S |
| DOCX Table | M | M | S | S | — | S |

## 8. Performance Targets

| Metric | Target |
|--------|--------|
| Open 10k-row XLSX | <500ms |
| Open 100k-row XLSX | <3s |
| Parse 1M-row CSV | <2s (streaming) |
| Recalculate 10k formulas | <100ms |
| Grid render (visible cells) | <16ms (60fps) |
| Cell edit to display | <10ms |
| Memory: 100k cells | <50MB |

Sources:
- [XLSX Format Specification (LOC)](https://www.loc.gov/preservation/digital/formats/fdd/fdd000398.shtml)
- [OOXML SpreadsheetML Anatomy](http://officeopenxml.com/anatomyofOOXML-xlsx.php)
- [MS-XLSX Reference](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/2c5dee00-eff2-4b22-92b6-0738acd4475e)
- [CSV vs XLSX vs ODS (2026)](https://blog.fileformat.com/en/spreadsheet/csv-vs-xlsx-vs-ods-in-2026-best-spreadsheet-format-for-developers/)
- [TSV Format (LOC)](https://www.loc.gov/preservation/digital/formats/fdd/fdd000533.shtml)
- [CSV RFC 4180 (Wikipedia)](https://en.wikipedia.org/wiki/Comma-separated_values)
