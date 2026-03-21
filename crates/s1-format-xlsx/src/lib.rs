//! XLSX (OOXML SpreadsheetML) reader/writer for s1engine.
//!
//! Reads and writes `.xlsx` files following ECMA-376 / ISO 29500.
//!
//! ## Architecture
//!
//! ```text
//! XLSX ZIP file
//!   ├── xl/sharedStrings.xml  → SharedStringTable
//!   ├── xl/styles.xml         → StyleSheet
//!   ├── xl/workbook.xml       → Workbook metadata
//!   ├── xl/worksheets/        → Sheet data (rows, cells)
//!   └── docProps/core.xml     → Metadata
//!
//! Workbook
//!   ├── sheets: Vec<Sheet>
//!   │     ├── name: String
//!   │     ├── cells: BTreeMap<CellRef, Cell>  (sparse)
//!   │     ├── merged_cells: Vec<CellRange>
//!   │     └── column_widths / row_heights
//!   ├── shared_strings: Vec<String>
//!   ├── styles: StyleSheet
//!   └── metadata: WorkbookMetadata
//! ```

pub mod error;
pub mod formula;
pub mod model;
pub mod reader;
pub mod shared_strings;
pub mod styles;
pub mod writer;

pub use error::XlsxError;
pub use model::*;
pub use reader::read;
pub use writer::write;
