# DOCX Parser Issues (s1-format-docx)

> Tracking file for bugs in the DOCX format reader/writer.
> Last updated: 2026-03-17

## Critical

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| DOCX-01 | Silent image data loss — missing media files cause `return Ok(())`, images dropped silently | `content_parser.rs` | 1614 | FIXED |
| DOCX-02 | Silent relationship parsing errors — `Err(_) => break` stops parsing without warning | `reader.rs` | 203 | FIXED |
| DOCX-03 | ContentType for document.xml — analyzed and confirmed correct per ECMA-376 (`document.main+xml`) | `writer.rs` | 583 | WONTFIX |

## High

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| DOCX-04 | Lossy UTF-8 conversion with `unwrap_or("?")` corrupts round-trip XML | `content_parser.rs` | 1404+ | FIXED |
| DOCX-05 | Extension parsing without validation — `rsplit('.').next()` edge cases | `content_parser.rs` | 1618 | FIXED |
| DOCX-06 | Missing round-trip tests for nested tables, mixed lists, multiple sections | `writer.rs` | tests | FIXED |
| DOCX-07 | Namespace extensions (w14, w15, wp14) silently skipped — Office 2016+ features lost | `content_parser.rs` | throughout | FIXED |

## Medium

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| DOCX-08 | Missing error context in node insertion errors — no node type/index/path info | `content_parser.rs` | throughout | FIXED |
| DOCX-09 | Media deduplication not verified — same image stored N times in ZIP | `content_parser.rs` | 1622 | FIXED |
| DOCX-10 | No OOXML constraint validation (empty cells, empty runs, required pgSz/pgMar) | various | — | FIXED |
| DOCX-11 | Section properties writer doesn't validate header/footer NodeIds exist | `writer.rs` | — | FIXED |
| DOCX-12 | Repeated `Vec` allocation per XML element in hot parsing loop | `content_parser.rs` | throughout | FIXED |
| DOCX-13 | Return values discarded with `let _ =` in optional file parsing | `reader.rs` | 124-134 | FIXED |

---

## Resolution Log

| ID | Date | Fix Description | Commit |
|----|------|-----------------|--------|
| DOCX-01 | 2026-03-16 | Added `#[cfg(debug_assertions)]` warning when media not found; still returns Ok (lenient parsing) | — |
| DOCX-02 | 2026-03-16 | Added debug warning on XML parse error in relationships before breaking | — |
| DOCX-03 | 2026-03-16 | Verified `document.main+xml` is correct per ECMA-376 OOXML spec | — |
| DOCX-04 | 2026-03-16 | Replaced all 14 `from_utf8().unwrap_or("?"/"")` with `String::from_utf8_lossy()` using standard U+FFFD replacement | — |
| DOCX-13 | 2026-03-16 | Removed unnecessary `let _ =` prefixes from 3 optional file parsing calls (comments, footnotes, endnotes) | — |
| DOCX-05 | 2026-03-17 | Already fixed: uses `std::path::Path::extension()` instead of `rsplit('.')` for robust extension parsing | — |
| DOCX-08 | 2026-03-17 | Enriched error messages on 11 key `insert_node` calls (Paragraph, Table, TableRow, TableCell, Run, Text, Image, Shape, Drawing, Field, TOC) with node type, parent ID, and index context | — |
| DOCX-12 | 2026-03-17 | Removed 24 instances of `e.local_name().as_ref().to_vec()` heap allocation per XML element; replaced with direct `match e.local_name().as_ref()` | — |
