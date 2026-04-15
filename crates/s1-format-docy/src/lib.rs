//! s1-format-docy — OnlyOffice DOCY binary format writer.
//!
//! Transforms s1engine DocumentModel into DOCY binary format that
//! sdkjs BinaryFileReader can parse natively, enabling full-fidelity
//! rendering of headers, footers, images, tables, TOC, comments, etc.
//!
//! Architecture:
//! ```text
//! DOCX → s1-format-docx::read() → DocumentModel → s1-format-docy::write() → DOCY → sdkjs
//! ```

mod constants;
mod writer;
mod tables;
mod props;
mod content;

use base64::engine::Engine as _;
use s1_model::DocumentModel;

/// Write a DocumentModel as a DOCY binary string.
///
/// Returns the complete DOCY string: `DOCY;v5;{size};{base64_data}`
/// Ready to pass to `sdkjs OpenDocumentFromBin()`.
pub fn write(model: &DocumentModel) -> String {
    let mut w = writer::DocyWriter::new();

    // Main table
    let mut mt = w.begin_main_table();

    // Table 1: Signature (required)
    w.register_table(&mut mt, constants::table_type::SIGNATURE);
    tables::signature::write(&mut w);

    // Table 2: Settings (required)
    w.register_table(&mut mt, constants::table_type::SETTINGS);
    tables::settings::write(&mut w, model);

    // Table 3: Numbering (if lists exist)
    if tables::numbering::has_content(model) {
        w.register_table(&mut mt, constants::table_type::NUMBERING);
        tables::numbering::write(&mut w, model);
    }

    // Table 4: Styles (required)
    w.register_table(&mut mt, constants::table_type::STYLE);
    tables::styles::write(&mut w, model);

    // Table 5: Document content (required)
    w.register_table(&mut mt, constants::table_type::DOCUMENT);
    tables::document::write(&mut w, model);

    // Table 6: Headers/Footers (if present)
    if tables::headers_footers::has_content(model) {
        w.register_table(&mut mt, constants::table_type::HDR_FTR);
        tables::headers_footers::write(&mut w, model);
    }

    // Table 7: Comments (if present)
    if tables::comments::has_content(model) {
        w.register_table(&mut mt, constants::table_type::COMMENTS);
        tables::comments::write(&mut w, model);
    }

    // Table 8: Footnotes (if present)
    if tables::footnotes::has_content(model) {
        w.register_table(&mut mt, constants::table_type::FOOTNOTES);
        tables::footnotes::write(&mut w, model);
    }

    // Table 9: Endnotes (if present)
    if tables::endnotes::has_content(model) {
        w.register_table(&mut mt, constants::table_type::ENDNOTES);
        tables::endnotes::write(&mut w, model);
    }

    // Table 10: Other (theme — can be empty)
    w.register_table(&mut mt, constants::table_type::OTHER);
    tables::other::write(&mut w);

    w.end_main_table(&mt);

    // Encode as DOCY string
    let binary = w.into_bytes();
    let b64 = base64::engine::general_purpose::STANDARD.encode(&binary);
    format!(
        "{};v{};{};{}",
        constants::DOCY_SIGNATURE,
        constants::DOCY_VERSION,
        binary.len(),
        b64
    )
}

// Re-export for WASM integration
pub use writer::DocyWriter;
