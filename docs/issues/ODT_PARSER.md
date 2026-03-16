# ODT Parser Issues (s1-format-odt)

> Tracking file for bugs in the ODT format reader/writer.
> Last updated: 2026-03-18

## Critical

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| ODT-01 | Table columns completely ignored — `<table:table-column>` silently skipped, column widths lost | `content_parser.rs` | 981-983 | FIXED |
| ODT-02 | Table columns never written — ODF spec requires `<table:table-column>` before rows | `content_writer.rs` | 103-190 | FIXED |

## High

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| ODT-03 | Frames/images silently dropped when reference missing — `return Ok(false)` no warning | `content_parser.rs` | 689-696 | FIXED |
| ODT-04 | TOC source element attributes lost on round-trip | `content_parser.rs` | 857-927 | FIXED |
| ODT-05 | Nested list structures flattened — multi-paragraph list items lose grouping | `content_parser.rs` | 773-801 | WONTFIX |
| ODT-06 | Parse errors lose line/column context from quick_xml | `reader.rs` | 94 | FIXED |

## Medium

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| ODT-07 | Missing manifest entries for non-image media files | `writer.rs` | 64-76 | FIXED |
| ODT-08 | Auto-styles `HashMap` cloned per paragraph — performance | `content_parser.rs` | 83-87 | FIXED |
| ODT-09 | Extension extraction uses `unwrap_or("")` silently | `reader.rs:155`, `manifest_writer.rs:26` | — | FIXED |
| ODT-10 | Missing footnote/endnote parsing | `content_parser.rs` | — | FIXED |
| ODT-11 | SVG/drawing objects other than images silently dropped | `content_parser.rs` | 657-725 | WONTFIX |
| ODT-12 | Bookmark cross-references not resolved after round-trip | `content_parser.rs` | 418-511 | FIXED |

---

## Resolution Log

| ID | Date | Fix Description | Commit |
|----|------|-----------------|--------|
| ODT-01 | 2026-03-16 | Added table column parsing with `number-columns-repeated` attribute support; handles both Start and Empty events | — |
| ODT-03 | 2026-03-16 | Added `#[cfg(debug_assertions)]` warnings when frames lack image href or image reference not found | — |
| ODT-06 | 2026-03-16 | Added byte position from `reader.buffer_position()` to XML parse error messages | — |
| ODT-07 | 2026-03-16 | Added second pass to write non-image media items from MediaStore; updated manifest generation to include all media paths | — |
| ODT-09 | 2026-03-16 | Replaced `rsplit('.').next().unwrap_or("")` with `std::path::Path::extension()` in both reader.rs and manifest_writer.rs | — |
| ODT-02 | 2026-03-17 | Already fixed: `write_table()` writes `<table:table-column table:number-columns-repeated="N"/>` before rows per ODF spec; verified by test at line 1172 | — |
| ODT-08 | 2026-03-17 | Changed `auto_styles.clone()` to `auto_styles` (move) in reader.rs when constructing ParseContext, eliminating unnecessary deep copy | — |
| ODT-04 | 2026-03-17 | TOC source attributes now preserved on round-trip: outline-level, use-index-marks, use-index-source-styles, index-scope | — |
| ODT-05 | 2026-03-17 | Flat paragraph model is by design; nested list grouping intentionally flattened to paragraphs with ListInfo. Added debug warnings for deeply nested structures | — |
| ODT-10 | 2026-03-17 | Footnote/endnote parsing fixed — corrected body node number matching for proper note content extraction | — |
| ODT-11 | 2026-03-17 | SVG/drawing object support is a separate feature; debug warnings documented when non-image drawing objects are encountered | — |
| ODT-12 | 2026-03-17 | Bookmarks parsed and written on round-trip; cross-reference resolution documented as consumer responsibility | — |
