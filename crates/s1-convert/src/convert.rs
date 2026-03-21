//! Format conversion pipeline.
//!
//! Provides a unified API for converting between document formats.
//! Conversion works through the document model:
//!
//! ```text
//! Source Format → DocumentModel → Target Format
//! ```

use s1_model::{DocumentModel, Node, NodeId, NodeType};

use crate::doc_reader;
use crate::error::ConvertError;

/// Warnings generated during conversion (non-fatal issues).
#[derive(Debug, Clone, PartialEq)]
pub enum ConvertWarning {
    /// Formatting was lost during conversion.
    FormattingLost(String),
    /// An element was not supported.
    UnsupportedElement(String),
}

/// Supported source formats for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceFormat {
    /// Legacy Microsoft Word binary format (.doc).
    Doc,
    /// Office Open XML (.docx).
    Docx,
    /// Open Document Format (.odt).
    Odt,
}

/// Supported target formats for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TargetFormat {
    /// Office Open XML (.docx).
    Docx,
    /// Open Document Format (.odt).
    Odt,
}

/// Convert document data from one format to another.
///
/// Returns the converted document as bytes in the target format.
///
/// Note: DOC source format only supports basic text extraction.
/// Use [`convert_with_warnings()`] to receive diagnostic warnings about data loss.
///
/// # Supported conversions
///
/// | From | To | Notes |
/// |------|-----|-------|
/// | DOC  | DOCX | Basic text extraction only (no formatting) |
/// | DOC  | ODT  | Basic text extraction only |
/// | DOCX | ODT  | Full model round-trip |
/// | ODT  | DOCX | Full model round-trip |
///
/// # Errors
///
/// Returns `ConvertError` if the conversion is not supported or fails.
pub fn convert(data: &[u8], from: SourceFormat, to: TargetFormat) -> Result<Vec<u8>, ConvertError> {
    #[cfg(debug_assertions)]
    if matches!(from, SourceFormat::Doc) {
        eprintln!(
            "[s1-convert] Warning: DOC format only supports basic text extraction; \
             formatting, images, and styles will be lost. Use convert_with_warnings() \
             to receive structured diagnostics."
        );
    }

    // Step 1: Read source into DocumentModel
    let doc = read_source(data, from)?;

    // Step 2: Write to target format
    write_target(&doc, to)
}

/// Convert document data from one format to another, returning warnings.
///
/// This is the same as [`convert()`] but additionally returns a list of
/// [`ConvertWarning`]s describing any non-fatal data loss (e.g., formatting
/// dropped during DOC conversion).
///
/// # Errors
///
/// Returns `ConvertError` if the conversion is not supported or fails.
pub fn convert_with_warnings(
    data: &[u8],
    from: SourceFormat,
    to: TargetFormat,
) -> Result<(Vec<u8>, Vec<ConvertWarning>), ConvertError> {
    let mut warnings = Vec::new();

    if matches!(from, SourceFormat::Doc) {
        warnings.push(ConvertWarning::FormattingLost(
            "DOC format: only basic text extraction supported; formatting, images, and styles are not preserved".into()
        ));
    }

    let doc = read_source(data, from)?;
    let output = write_target(&doc, to)?;
    Ok((output, warnings))
}

/// Check if a conversion path is supported.
///
/// Currently all combinations of [`SourceFormat`] and [`TargetFormat`] are
/// supported, but this function is provided for forward-compatibility when
/// new format variants are added.
pub fn is_supported(from: SourceFormat, to: TargetFormat) -> bool {
    matches!(
        (from, to),
        (SourceFormat::Doc, TargetFormat::Docx)
            | (SourceFormat::Doc, TargetFormat::Odt)
            | (SourceFormat::Docx, TargetFormat::Odt)
            | (SourceFormat::Odt, TargetFormat::Docx)
    )
}

/// Validate that a conversion path is supported, returning an error if not.
///
/// This provides early validation before starting potentially expensive
/// read/write operations.
///
/// # Errors
///
/// Returns [`ConvertError::UnsupportedConversion`] if the path is not supported.
pub fn validate_conversion(from: SourceFormat, to: TargetFormat) -> Result<(), ConvertError> {
    if is_supported(from, to) {
        Ok(())
    } else {
        Err(ConvertError::UnsupportedConversion {
            from: format!("{from:?}"),
            to: format!("{to:?}"),
        })
    }
}

/// Convert document data from one format to a DocumentModel.
///
/// Useful when consumers want the model rather than re-encoded bytes.
pub fn convert_to_model(data: &[u8], from: SourceFormat) -> Result<DocumentModel, ConvertError> {
    read_source(data, from)
}

/// Detect the source format from file bytes.
///
/// Returns `None` if the format cannot be detected.
pub fn detect_format(data: &[u8]) -> Option<SourceFormat> {
    if doc_reader::is_doc_file(data) {
        Some(SourceFormat::Doc)
    } else if data.len() >= 4 && &data[..4] == b"PK\x03\x04" {
        // ZIP-based — could be DOCX or ODT
        // Check for DOCX content types
        if let Ok(text) = std::str::from_utf8(data) {
            if text.contains("word/") {
                return Some(SourceFormat::Docx);
            }
        }
        // Default to DOCX for ZIP files (more common)
        // In practice, consumers should specify the format explicitly
        Some(SourceFormat::Docx)
    } else {
        None
    }
}

// ─── Extended File Type Detection (Phase 6: Multi-App Suite) ────────────

/// Broad file type classification for the multi-app suite.
///
/// While [`SourceFormat`] only covers formats this engine can convert,
/// `FileType` classifies any Office/document file type, including
/// spreadsheet and presentation formats that may be handled by
/// future companion engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileType {
    /// Microsoft Word DOCX (Office Open XML).
    Docx,
    /// Microsoft Excel XLSX (Office Open XML Spreadsheet).
    Xlsx,
    /// Microsoft PowerPoint PPTX (Office Open XML Presentation).
    Pptx,
    /// OpenDocument Text (.odt).
    Odt,
    /// OpenDocument Spreadsheet (.ods).
    Ods,
    /// OpenDocument Presentation (.odp).
    Odp,
    /// Portable Document Format (.pdf).
    Pdf,
    /// Legacy Microsoft Word binary (.doc).
    Doc,
    /// Plain text (.txt).
    Txt,
    /// Markdown (.md).
    Md,
    /// Comma-separated values (.csv).
    Csv,
    /// Unknown or unrecognized format.
    Unknown,
}

impl FileType {
    /// File extension for this type (without dot).
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Xlsx => "xlsx",
            Self::Pptx => "pptx",
            Self::Odt => "odt",
            Self::Ods => "ods",
            Self::Odp => "odp",
            Self::Pdf => "pdf",
            Self::Doc => "doc",
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Csv => "csv",
            Self::Unknown => "bin",
        }
    }

    /// MIME type for this file type.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Odt => "application/vnd.oasis.opendocument.text",
            Self::Ods => "application/vnd.oasis.opendocument.spreadsheet",
            Self::Odp => "application/vnd.oasis.opendocument.presentation",
            Self::Pdf => "application/pdf",
            Self::Doc => "application/msword",
            Self::Txt => "text/plain",
            Self::Md => "text/markdown",
            Self::Csv => "text/csv",
            Self::Unknown => "application/octet-stream",
        }
    }

    /// Human-readable label for UI display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Docx => "Word Document",
            Self::Xlsx => "Excel Spreadsheet",
            Self::Pptx => "PowerPoint Presentation",
            Self::Odt => "OpenDocument Text",
            Self::Ods => "OpenDocument Spreadsheet",
            Self::Odp => "OpenDocument Presentation",
            Self::Pdf => "PDF Document",
            Self::Doc => "Word Document (Legacy)",
            Self::Txt => "Plain Text",
            Self::Md => "Markdown",
            Self::Csv => "CSV Spreadsheet",
            Self::Unknown => "Unknown",
        }
    }

    /// Whether this file type is a document (word processor) format.
    pub fn is_document(&self) -> bool {
        matches!(
            self,
            Self::Docx | Self::Odt | Self::Doc | Self::Txt | Self::Md
        )
    }

    /// Whether this file type is a spreadsheet format.
    pub fn is_spreadsheet(&self) -> bool {
        matches!(self, Self::Xlsx | Self::Ods | Self::Csv)
    }

    /// Whether this file type is a presentation format.
    pub fn is_presentation(&self) -> bool {
        matches!(self, Self::Pptx | Self::Odp)
    }

    /// Whether this file type is currently supported for reading by s1engine.
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Self::Docx | Self::Odt | Self::Doc | Self::Txt | Self::Md | Self::Pdf | Self::Csv
        )
    }
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Detect the file type from raw bytes using magic bytes and ZIP inspection.
///
/// This function performs deep inspection of ZIP-based formats by reading
/// `[Content_Types].xml` (for OOXML) or `mimetype` / `META-INF/manifest.xml`
/// (for ODF) to distinguish between document, spreadsheet, and presentation
/// formats.
///
/// # Detection strategy
///
/// 1. **OLE2 compound document** (magic `D0 CF 11 E0`) → [`FileType::Doc`]
/// 2. **ZIP archive** (magic `PK\x03\x04`) → inspect contents:
///    - Read `[Content_Types].xml` for OOXML content type strings
///    - Read `mimetype` file for ODF media type
///    - Fall back to scanning for characteristic file paths
/// 3. **PDF** (magic `%PDF`) → [`FileType::Pdf`]
/// 4. **Plain text heuristic** → [`FileType::Txt`] or [`FileType::Md`]
///
/// # Examples
///
/// ```
/// use s1_convert::detect_file_type;
///
/// let pdf_bytes = b"%PDF-1.5 fake";
/// assert_eq!(detect_file_type(pdf_bytes).extension(), "pdf");
/// ```
pub fn detect_file_type(data: &[u8]) -> FileType {
    // 1. OLE2 compound document → Doc
    if doc_reader::is_doc_file(data) {
        return FileType::Doc;
    }

    // 2. ZIP-based formats
    if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
        return detect_zip_file_type(data);
    }

    // 3. PDF
    if data.len() >= 4 && &data[0..4] == b"%PDF" {
        return FileType::Pdf;
    }

    // 4. Text-based heuristics
    detect_text_file_type(data)
}

/// Inspect a ZIP archive to determine the specific Office format.
fn detect_zip_file_type(data: &[u8]) -> FileType {
    let cursor = std::io::Cursor::new(data);
    let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
        // Corrupted ZIP — fall back to string scan
        return detect_zip_fallback(data);
    };

    // Strategy A: Read [Content_Types].xml (OOXML)
    if let Ok(mut entry) = archive.by_name("[Content_Types].xml") {
        let mut content = String::new();
        if std::io::Read::read_to_string(&mut entry, &mut content).is_ok() {
            if content.contains("wordprocessingml") {
                return FileType::Docx;
            }
            if content.contains("spreadsheetml") {
                return FileType::Xlsx;
            }
            if content.contains("presentationml") {
                return FileType::Pptx;
            }
        }
    }

    // Strategy B: Read mimetype file (ODF)
    if let Ok(mut entry) = archive.by_name("mimetype") {
        let mut mimetype = String::new();
        if std::io::Read::read_to_string(&mut entry, &mut mimetype).is_ok() {
            let mt = mimetype.trim();
            if mt.contains("opendocument.text") {
                return FileType::Odt;
            }
            if mt.contains("opendocument.spreadsheet") {
                return FileType::Ods;
            }
            if mt.contains("opendocument.presentation") {
                return FileType::Odp;
            }
        }
    }

    // Strategy C: Read META-INF/manifest.xml for ODF
    if let Ok(mut entry) = archive.by_name("META-INF/manifest.xml") {
        let mut manifest = String::new();
        if std::io::Read::read_to_string(&mut entry, &mut manifest).is_ok() {
            if manifest.contains("opendocument.text") {
                return FileType::Odt;
            }
            if manifest.contains("opendocument.spreadsheet") {
                return FileType::Ods;
            }
            if manifest.contains("opendocument.presentation") {
                return FileType::Odp;
            }
        }
    }

    // Strategy D: Check for characteristic file paths
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();

    if names.iter().any(|n| n.starts_with("word/")) {
        return FileType::Docx;
    }
    if names.iter().any(|n| n.starts_with("xl/")) {
        return FileType::Xlsx;
    }
    if names.iter().any(|n| n.starts_with("ppt/")) {
        return FileType::Pptx;
    }
    if names.iter().any(|n| n == "content.xml") {
        // Generic ODF — default to text
        return FileType::Odt;
    }

    // Unknown ZIP
    FileType::Unknown
}

/// Fallback: scan raw ZIP bytes as a string for known markers.
fn detect_zip_fallback(data: &[u8]) -> FileType {
    let s = String::from_utf8_lossy(data);
    if s.contains("word/") || s.contains("wordprocessingml") {
        FileType::Docx
    } else if s.contains("xl/") || s.contains("spreadsheetml") {
        FileType::Xlsx
    } else if s.contains("ppt/") || s.contains("presentationml") {
        FileType::Pptx
    } else if s.contains("content.xml") || s.contains("opendocument.text") {
        FileType::Odt
    } else if s.contains("opendocument.spreadsheet") {
        FileType::Ods
    } else if s.contains("opendocument.presentation") {
        FileType::Odp
    } else {
        FileType::Unknown
    }
}

/// Heuristic detection for text-based files.
fn detect_text_file_type(data: &[u8]) -> FileType {
    // Check if the data looks like UTF-8 text
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => {
            // Check if it's valid text with some non-UTF-8 (e.g., Latin-1)
            // For now, treat non-UTF-8 as unknown
            return FileType::Unknown;
        }
    };

    if text.is_empty() {
        return FileType::Txt;
    }

    // CSV heuristic: check first few lines for comma/tab-separated structure
    if looks_like_csv(text) {
        return FileType::Csv;
    }

    // Markdown heuristic: check for common Markdown patterns
    if looks_like_markdown(text) {
        return FileType::Md;
    }

    FileType::Txt
}

/// Check if text content looks like CSV data.
fn looks_like_csv(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().take(10).collect();
    if lines.len() < 2 {
        return false;
    }

    // Count commas in each line; if consistent, it's likely CSV
    let comma_counts: Vec<usize> = lines.iter().map(|l| l.matches(',').count()).collect();
    if comma_counts[0] == 0 {
        return false;
    }

    // At least 2 columns and consistent across first several lines
    let first_count = comma_counts[0];
    let consistent = comma_counts.iter().filter(|&&c| c == first_count).count();
    // Allow some variance (e.g., 80% of lines match)
    consistent * 100 / comma_counts.len() >= 70
}

/// Check if text content looks like Markdown.
fn looks_like_markdown(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().take(30).collect();
    let mut md_indicators = 0;

    for line in &lines {
        let trimmed = line.trim();
        // Headings
        if trimmed.starts_with('#') {
            md_indicators += 2;
        }
        // Lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("1. ") {
            md_indicators += 1;
        }
        // Links / images
        if trimmed.contains("](") {
            md_indicators += 2;
        }
        // Code blocks
        if trimmed.starts_with("```") {
            md_indicators += 2;
        }
        // Bold / italic
        if trimmed.contains("**") || trimmed.contains("__") {
            md_indicators += 1;
        }
    }

    md_indicators >= 3
}

// ─── CSV Import/Export (Phase 6: Spreadsheet-lite) ──────────────────────

/// Parse CSV data and convert it to a [`DocumentModel`] containing a table.
///
/// Each row in the CSV becomes a table row, and each field becomes a cell
/// containing a paragraph with the cell text. This provides
/// "spreadsheet-lite" functionality without requiring a full grid editor.
///
/// # Errors
///
/// Returns [`ConvertError::ReadError`] if the CSV data is not valid UTF-8.
pub fn csv_to_model(data: &[u8]) -> Result<DocumentModel, ConvertError> {
    let text = std::str::from_utf8(data)
        .map_err(|e| ConvertError::ReadError(format!("CSV is not valid UTF-8: {e}")))?;

    let rows = parse_csv(text);
    if rows.is_empty() {
        return Err(ConvertError::ReadError("CSV file is empty".to_string()));
    }

    let mut doc = DocumentModel::new();
    let root = doc.root_id();

    // Create body
    let body_id = doc.next_id();
    doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
        .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;

    // Create table
    let table_id = doc.next_id();
    doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
        .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;

    for (row_idx, row) in rows.iter().enumerate() {
        let row_node_id = doc.next_id();
        doc.insert_node(
            table_id,
            row_idx,
            Node::new(row_node_id, NodeType::TableRow),
        )
        .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;

        for (col_idx, cell_text) in row.iter().enumerate() {
            let cell_id = doc.next_id();
            doc.insert_node(
                row_node_id,
                col_idx,
                Node::new(cell_id, NodeType::TableCell),
            )
            .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;

            let para_id = doc.next_id();
            doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
                .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;

            let run_id = doc.next_id();
            doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
                .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;

            let text_id = doc.next_id();
            doc.insert_node(run_id, 0, Node::text(text_id, cell_text.as_str()))
                .map_err(|e| ConvertError::ReadError(format!("Model error: {e}")))?;
        }
    }

    Ok(doc)
}

/// Convert a CSV to DOCX bytes (CSV → model → DOCX).
///
/// # Errors
///
/// Returns [`ConvertError`] if parsing or DOCX export fails.
pub fn csv_to_docx(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let model = csv_to_model(data)?;
    s1_format_docx::write(&model).map_err(ConvertError::from)
}

/// Extract tables from a [`DocumentModel`] and export as CSV text.
///
/// If the document contains multiple tables, they are separated by a blank
/// line. Cells containing commas, quotes, or newlines are properly escaped
/// per RFC 4180.
pub fn model_to_csv(doc: &DocumentModel) -> String {
    let root = doc.root_id();
    let mut output = String::new();
    let mut first_table = true;

    collect_tables_csv(doc, root, &mut output, &mut first_table);

    output
}

/// Export tables from a DOCX as CSV.
///
/// # Errors
///
/// Returns [`ConvertError`] if the DOCX cannot be read.
pub fn docx_to_csv(data: &[u8]) -> Result<String, ConvertError> {
    let model = s1_format_docx::read(data).map_err(ConvertError::from)?;
    Ok(model_to_csv(&model))
}

// ─── CSV helpers ────────────────────────────────────────────────────────

/// Simple RFC 4180 CSV parser (no external dependency).
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    let mut chars = text.chars().peekable();

    loop {
        let row = parse_csv_row(&mut chars);
        if row.is_empty() && chars.peek().is_none() {
            break;
        }
        rows.push(row);
        if chars.peek().is_none() {
            break;
        }
    }

    rows
}

fn parse_csv_row(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Vec<String> {
    let mut fields = Vec::new();

    loop {
        let field = parse_csv_field(chars);
        fields.push(field);

        match chars.peek() {
            Some(',') => {
                chars.next(); // consume comma
            }
            Some('\r') => {
                chars.next(); // consume CR
                if chars.peek() == Some(&'\n') {
                    chars.next(); // consume LF
                }
                break;
            }
            Some('\n') => {
                chars.next(); // consume LF
                break;
            }
            None => break,
            _ => break,
        }
    }

    fields
}

fn parse_csv_field(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    if chars.peek() == Some(&'"') {
        // Quoted field
        chars.next(); // consume opening quote
        let mut field = String::new();
        loop {
            match chars.next() {
                Some('"') => {
                    if chars.peek() == Some(&'"') {
                        chars.next(); // escaped quote
                        field.push('"');
                    } else {
                        break; // end of quoted field
                    }
                }
                Some(c) => field.push(c),
                None => break,
            }
        }
        field
    } else {
        // Unquoted field
        let mut field = String::new();
        loop {
            match chars.peek() {
                Some(',') | Some('\r') | Some('\n') | None => break,
                Some(&c) => {
                    chars.next();
                    field.push(c);
                }
            }
        }
        field
    }
}

/// Recursively walk the document tree, collecting tables as CSV.
fn collect_tables_csv(
    doc: &DocumentModel,
    node_id: NodeId,
    output: &mut String,
    first_table: &mut bool,
) {
    let Some(node) = doc.node(node_id) else {
        return;
    };

    if node.node_type == NodeType::Table {
        if !*first_table {
            output.push('\n');
        }
        *first_table = false;
        write_table_csv(doc, node_id, output);
        return;
    }

    for &child_id in &node.children {
        collect_tables_csv(doc, child_id, output, first_table);
    }
}

/// Write a single table as CSV rows.
fn write_table_csv(doc: &DocumentModel, table_id: NodeId, output: &mut String) {
    let Some(table_node) = doc.node(table_id) else {
        return;
    };

    for &row_id in &table_node.children {
        let Some(row_node) = doc.node(row_id) else {
            continue;
        };
        if row_node.node_type != NodeType::TableRow {
            continue;
        }

        let mut first_cell = true;
        for &cell_id in &row_node.children {
            let Some(cell_node) = doc.node(cell_id) else {
                continue;
            };
            if cell_node.node_type != NodeType::TableCell {
                continue;
            }

            if !first_cell {
                output.push(',');
            }
            first_cell = false;

            let cell_text = extract_text(doc, cell_id);
            write_csv_field(output, &cell_text);
        }
        output.push('\n');
    }
}

/// Extract all text content from a subtree.
fn extract_text(doc: &DocumentModel, node_id: NodeId) -> String {
    let Some(node) = doc.node(node_id) else {
        return String::new();
    };

    if node.node_type == NodeType::Text {
        return node.text_content.clone().unwrap_or_default();
    }

    let mut text = String::new();
    for &child_id in &node.children {
        text.push_str(&extract_text(doc, child_id));
    }
    text
}

/// Write a single CSV field, quoting if necessary.
fn write_csv_field(output: &mut String, field: &str) {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        output.push('"');
        for c in field.chars() {
            if c == '"' {
                output.push_str("\"\"");
            } else {
                output.push(c);
            }
        }
        output.push('"');
    } else {
        output.push_str(field);
    }
}

fn read_source(data: &[u8], from: SourceFormat) -> Result<DocumentModel, ConvertError> {
    match from {
        SourceFormat::Doc => doc_reader::read_doc(data),
        SourceFormat::Docx => s1_format_docx::read(data).map_err(ConvertError::from),
        SourceFormat::Odt => s1_format_odt::read(data).map_err(ConvertError::from),
    }
}

fn write_target(doc: &DocumentModel, to: TargetFormat) -> Result<Vec<u8>, ConvertError> {
    match to {
        TargetFormat::Docx => s1_format_docx::write(doc).map_err(ConvertError::from),
        TargetFormat::Odt => s1_format_odt::write(doc).map_err(ConvertError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_doc_format() {
        let magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert_eq!(detect_format(&magic), Some(SourceFormat::Doc));
    }

    #[test]
    fn detect_zip_format() {
        let zip_magic = [0x50, 0x4B, 0x03, 0x04, 0, 0, 0, 0];
        let detected = detect_format(&zip_magic);
        assert!(detected.is_some());
    }

    #[test]
    fn detect_unknown_format() {
        assert_eq!(detect_format(b"random data"), None);
    }

    // ─── FileType detection tests ───────────────────────────────

    #[test]
    fn detect_file_type_pdf() {
        assert_eq!(detect_file_type(b"%PDF-1.5 fake content"), FileType::Pdf);
    }

    #[test]
    fn detect_file_type_doc() {
        let ole2 = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert_eq!(detect_file_type(&ole2), FileType::Doc);
    }

    #[test]
    fn detect_file_type_plain_text() {
        assert_eq!(detect_file_type(b"Hello, world!"), FileType::Txt);
    }

    #[test]
    fn detect_file_type_markdown() {
        let md =
            b"# Heading\n\n- item 1\n- item 2\n\n**bold text** and [link](url)\n\n## Sub heading\n";
        assert_eq!(detect_file_type(md), FileType::Md);
    }

    #[test]
    fn detect_file_type_csv() {
        let csv = b"name,age,city\nAlice,30,NYC\nBob,25,LA\nCharlie,35,CHI\n";
        assert_eq!(detect_file_type(csv), FileType::Csv);
    }

    #[test]
    fn detect_file_type_empty() {
        assert_eq!(detect_file_type(b""), FileType::Txt);
    }

    #[test]
    fn detect_file_type_docx_from_real_zip() {
        // Build a minimal DOCX and detect it
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let docx_bytes = s1_format_docx::write(&doc).unwrap();
        assert_eq!(detect_file_type(&docx_bytes), FileType::Docx);
    }

    #[test]
    fn detect_file_type_odt_from_real_zip() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let odt_bytes = s1_format_odt::write(&doc).unwrap();
        assert_eq!(detect_file_type(&odt_bytes), FileType::Odt);
    }

    #[test]
    fn file_type_properties() {
        assert!(FileType::Docx.is_document());
        assert!(!FileType::Docx.is_spreadsheet());
        assert!(!FileType::Docx.is_presentation());
        assert!(FileType::Docx.is_supported());

        assert!(FileType::Xlsx.is_spreadsheet());
        assert!(!FileType::Xlsx.is_document());
        assert!(!FileType::Xlsx.is_supported()); // not yet

        assert!(FileType::Pptx.is_presentation());
        assert!(!FileType::Pptx.is_supported()); // not yet

        assert!(FileType::Csv.is_spreadsheet());
        assert!(FileType::Csv.is_supported());
    }

    #[test]
    fn file_type_extension_and_mime() {
        assert_eq!(FileType::Xlsx.extension(), "xlsx");
        assert!(FileType::Xlsx.mime_type().contains("spreadsheetml"));
        assert_eq!(FileType::Pptx.extension(), "pptx");
        assert!(FileType::Pptx.mime_type().contains("presentationml"));
        assert_eq!(FileType::Ods.extension(), "ods");
        assert!(FileType::Ods
            .mime_type()
            .contains("opendocument.spreadsheet"));
        assert_eq!(FileType::Odp.extension(), "odp");
        assert!(FileType::Odp
            .mime_type()
            .contains("opendocument.presentation"));
    }

    #[test]
    fn file_type_display() {
        assert_eq!(format!("{}", FileType::Docx), "Word Document");
        assert_eq!(format!("{}", FileType::Xlsx), "Excel Spreadsheet");
        assert_eq!(format!("{}", FileType::Csv), "CSV Spreadsheet");
    }

    // ─── CSV import/export tests ────────────────────────────────

    #[test]
    fn csv_parse_simple() {
        let rows = parse_csv("a,b,c\n1,2,3\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a", "b", "c"]);
        assert_eq!(rows[1], vec!["1", "2", "3"]);
    }

    #[test]
    fn csv_parse_quoted_fields() {
        let rows = parse_csv("\"hello, world\",\"she said \"\"hi\"\"\"\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "hello, world");
        assert_eq!(rows[0][1], "she said \"hi\"");
    }

    #[test]
    fn csv_parse_crlf() {
        let rows = parse_csv("a,b\r\nc,d\r\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a", "b"]);
        assert_eq!(rows[1], vec!["c", "d"]);
    }

    #[test]
    fn csv_to_model_basic() {
        let csv = b"Name,Age\nAlice,30\nBob,25\n";
        let model = csv_to_model(csv).unwrap();

        // Should have root → body → table
        let root = model.root_id();
        let root_node = model.node(root).unwrap();
        assert!(!root_node.children.is_empty());

        // Find the table
        let body_id = root_node.children[0];
        let body_node = model.node(body_id).unwrap();
        let table_id = body_node.children[0];
        let table_node = model.node(table_id).unwrap();
        assert_eq!(table_node.node_type, NodeType::Table);
        assert_eq!(table_node.children.len(), 3); // header + 2 data rows
    }

    #[test]
    fn csv_to_docx_roundtrip() {
        let csv = b"X,Y\n1,2\n3,4\n";
        let docx_bytes = csv_to_docx(csv).unwrap();
        assert!(!docx_bytes.is_empty());

        // Read back and extract CSV
        let csv_text = docx_to_csv(&docx_bytes).unwrap();
        assert!(csv_text.contains("X,Y"));
        assert!(csv_text.contains("1,2"));
        assert!(csv_text.contains("3,4"));
    }

    #[test]
    fn model_to_csv_basic() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let table_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(table_id, NodeType::Table))
            .unwrap();

        // Row 1
        let row1_id = doc.next_id();
        doc.insert_node(table_id, 0, Node::new(row1_id, NodeType::TableRow))
            .unwrap();
        let cell1_id = doc.next_id();
        doc.insert_node(row1_id, 0, Node::new(cell1_id, NodeType::TableCell))
            .unwrap();
        let para1_id = doc.next_id();
        doc.insert_node(cell1_id, 0, Node::new(para1_id, NodeType::Paragraph))
            .unwrap();
        let run1_id = doc.next_id();
        doc.insert_node(para1_id, 0, Node::new(run1_id, NodeType::Run))
            .unwrap();
        let text1_id = doc.next_id();
        doc.insert_node(run1_id, 0, Node::text(text1_id, "Hello"))
            .unwrap();

        let cell2_id = doc.next_id();
        doc.insert_node(row1_id, 1, Node::new(cell2_id, NodeType::TableCell))
            .unwrap();
        let para2_id = doc.next_id();
        doc.insert_node(cell2_id, 0, Node::new(para2_id, NodeType::Paragraph))
            .unwrap();
        let run2_id = doc.next_id();
        doc.insert_node(para2_id, 0, Node::new(run2_id, NodeType::Run))
            .unwrap();
        let text2_id = doc.next_id();
        doc.insert_node(run2_id, 0, Node::text(text2_id, "World"))
            .unwrap();

        let csv = model_to_csv(&doc);
        assert_eq!(csv, "Hello,World\n");
    }

    #[test]
    fn csv_field_escaping() {
        let mut out = String::new();
        write_csv_field(&mut out, "simple");
        assert_eq!(out, "simple");

        out.clear();
        write_csv_field(&mut out, "has,comma");
        assert_eq!(out, "\"has,comma\"");

        out.clear();
        write_csv_field(&mut out, "has\"quote");
        assert_eq!(out, "\"has\"\"quote\"");

        out.clear();
        write_csv_field(&mut out, "has\nnewline");
        assert_eq!(out, "\"has\nnewline\"");
    }

    #[test]
    fn csv_empty_error() {
        let result = csv_to_model(b"");
        assert!(result.is_err());
    }

    #[test]
    fn csv_invalid_utf8() {
        let result = csv_to_model(&[0xFF, 0xFE, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn convert_docx_to_odt() {
        // Build a minimal DOCX, convert to ODT
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(
            root,
            0,
            s1_model::Node::new(body_id, s1_model::NodeType::Body),
        )
        .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(
            body_id,
            0,
            s1_model::Node::new(para_id, s1_model::NodeType::Paragraph),
        )
        .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(
            para_id,
            0,
            s1_model::Node::new(run_id, s1_model::NodeType::Run),
        )
        .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, s1_model::Node::text(text_id, "Convert me"))
            .unwrap();

        let docx_bytes = s1_format_docx::write(&doc).unwrap();

        // Convert DOCX → ODT
        let odt_bytes = convert(&docx_bytes, SourceFormat::Docx, TargetFormat::Odt).unwrap();
        assert!(!odt_bytes.is_empty());

        // Verify ODT can be read back
        let model = s1_format_odt::read(&odt_bytes).unwrap();
        // Should have content
        let root = model.root_id();
        let root_node = model.node(root).unwrap();
        assert!(!root_node.children.is_empty());
    }

    #[test]
    fn convert_odt_to_docx() {
        // Build a minimal ODT, convert to DOCX
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(
            root,
            0,
            s1_model::Node::new(body_id, s1_model::NodeType::Body),
        )
        .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(
            body_id,
            0,
            s1_model::Node::new(para_id, s1_model::NodeType::Paragraph),
        )
        .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(
            para_id,
            0,
            s1_model::Node::new(run_id, s1_model::NodeType::Run),
        )
        .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, s1_model::Node::text(text_id, "Cross format"))
            .unwrap();

        let odt_bytes = s1_format_odt::write(&doc).unwrap();

        // Convert ODT → DOCX
        let docx_bytes = convert(&odt_bytes, SourceFormat::Odt, TargetFormat::Docx).unwrap();
        assert!(!docx_bytes.is_empty());

        // Verify DOCX can be read back
        let model = s1_format_docx::read(&docx_bytes).unwrap();
        let root = model.root_id();
        let root_node = model.node(root).unwrap();
        assert!(!root_node.children.is_empty());
    }

    #[test]
    fn convert_invalid_doc() {
        let result = convert(b"not a doc", SourceFormat::Doc, TargetFormat::Docx);
        assert!(result.is_err());
    }

    #[test]
    fn convert_with_warnings_doc_source() {
        // DOC source should produce a FormattingLost warning
        // We can't easily test with real DOC data here, but we can test the
        // non-DOC path.
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(
            root,
            0,
            s1_model::Node::new(body_id, s1_model::NodeType::Body),
        )
        .unwrap();
        let docx_bytes = s1_format_docx::write(&doc).unwrap();

        let (odt_bytes, warnings) =
            convert_with_warnings(&docx_bytes, SourceFormat::Docx, TargetFormat::Odt).unwrap();
        assert!(!odt_bytes.is_empty());
        assert!(warnings.is_empty(), "DOCX→ODT should have no warnings");
    }

    #[test]
    fn is_supported_all_current_paths() {
        assert!(is_supported(SourceFormat::Doc, TargetFormat::Docx));
        assert!(is_supported(SourceFormat::Doc, TargetFormat::Odt));
        assert!(is_supported(SourceFormat::Docx, TargetFormat::Odt));
        assert!(is_supported(SourceFormat::Odt, TargetFormat::Docx));
    }

    #[test]
    fn convert_to_model_docx() {
        let mut doc = DocumentModel::new();
        let root = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(
            root,
            0,
            s1_model::Node::new(body_id, s1_model::NodeType::Body),
        )
        .unwrap();
        let docx_bytes = s1_format_docx::write(&doc).unwrap();

        let model = convert_to_model(&docx_bytes, SourceFormat::Docx).unwrap();
        assert!(model.node(model.root_id()).is_some());
    }
}
