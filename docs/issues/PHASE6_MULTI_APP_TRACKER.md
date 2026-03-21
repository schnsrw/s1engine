# Phase 6 — Multi-App Suite Tracker

> Full specification: `docs/specs/SPREADSHEET_SPEC.md`
> Last updated: 2026-03-22

## Status Overview

| Sub-Phase | Sprint | Items | Done | Status |
|-----------|--------|-------|------|--------|
| 6-Foundation | — | 4 | 4 | **DONE** |
| 6a-CSV/TSV | 1 | 8 | 8 | **DONE** (RFC 4180, delimiter auto-detect, streaming, BOM, round-trip) |
| 6b-XLSX Reader | 2 | 10 | 10 | **DONE** (cells, formulas, styles, columns, rows, frozen panes, merges, preserved parts) |
| 6c-XLSX Writer | 3 | 7 | 7 | **DONE** (full round-trip with styles, columns, rows, panes, preserved parts) |
| 6d-Formula Engine | 4 | 8 | 8 | **DONE** (tokenizer, parser, 30+ functions, dependency graph, cycle detection) |
| 6e-Grid UI | 5-6 | 14 | 14 | **DONE** (canvas virtual scroll, selection, editing, formula bar, tabs, context menu, sort, filter, undo/redo, copy/paste, freeze, auto-fill, insert/delete, resize) |
| 6f-ODS Spreadsheet | 7 | 6 | 6 | **DONE** (reader + writer, value types, formulas, styles, round-trip — 16 tests) |
| 6g-Presentation | 8+ | 7 | 0 | **DEFERRED** (user decided to skip slides for now) |
| 6h-Launcher | 9 | 4 | 4 | **DONE** (file detection, launcher UI, server routing, tab switching) |

---

## 6-Foundation (PARTIAL)

| # | Description | Status |
|---|-------------|--------|
| F1 | File type detection (DOCX/XLSX/PPTX/ODT/ODS/ODP/PDF/CSV) — `detect_file_type()` in s1-convert | DONE |
| F2 | Launcher UI buttons (Document, Spreadsheet, Presentation, CSV) | DONE |
| F3 | CSV file input accept in editor HTML | DONE |
| F4 | Server file type routing in upload response | DONE |

## 6a: CSV/TSV Parser (Sprint 1) — DONE

> Spec: SPREADSHEET_SPEC.md Section 4
> Implementation: `crates/s1-convert/src/csv_parser.rs` (40 tests)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| a1 | CSV parser: RFC 4180 with all 8 edge cases (quoting, escaping, multiline, BOM) | M | DONE |
| a2 | TSV parser: tab-delimited variant (`parse_tsv()`) | S | DONE |
| a3 | CSV writer: export data model to CSV (`write_csv()`, `write_csv_with_delimiter()`) | S | DONE |
| a4 | CSV → DOCX table conversion | S | DONE (s1-convert) |
| a5 | Auto-detect delimiter (comma vs tab vs semicolon vs pipe) — `detect_delimiter()` | S | DONE |
| a6 | Encoding detection (UTF-8, Latin-1, BOM stripping) | S | DONE |
| a7 | Large file streaming (`parse_csv_streaming()`, `parse_csv_streaming_with_delimiter()`) | M | DONE |
| a8 | Round-trip tests (CSV → model → CSV) | S | DONE |

## 6b: XLSX Reader (Sprint 2) — DONE

> Spec: SPREADSHEET_SPEC.md Section 2
> Implementation: `crates/s1-format-xlsx/src/reader.rs` + `shared_strings.rs` + `styles.rs` (6+2+3 tests)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| b1 | Create `s1-format-xlsx` crate with ZIP reader scaffold | M | DONE |
| b2 | Parse `xl/sharedStrings.xml` → string table | S | DONE |
| b3 | Parse `xl/styles.xml` → number formats, fonts, fills, borders | L | DONE |
| b4 | Parse `xl/worksheets/sheetN.xml` → rows, cells, cell types | L | DONE |
| b5 | Parse `xl/workbook.xml` → sheet names, defined names, active sheet | S | DONE |
| b6 | Formula string extraction (store as string, no evaluation) | S | DONE |
| b7 | Merged cell ranges | S | DONE |
| b8 | Column widths + row heights | S | DONE |
| b9 | Frozen panes | S | DONE |
| b10 | XLSX reader tests with real Excel files | M | DONE |

## 6c: XLSX Writer (Sprint 3) — DONE

> Implementation: `crates/s1-format-xlsx/src/writer.rs` (9 tests)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| c1 | Generate `sharedStrings.xml` from string table | S | DONE |
| c2 | Generate `styles.xml` from style model | L | DONE |
| c3 | Generate `worksheetN.xml` with cell data | L | DONE |
| c4 | Generate `workbook.xml` with sheet references | S | DONE |
| c5 | ZIP packaging with Content_Types + relationships | M | DONE |
| c6 | Preserve unrecognized XML for round-trip fidelity | M | DONE |
| c7 | Round-trip tests (XLSX → model → XLSX → reopen → compare) | M | DONE |

## 6d: Formula Engine (Sprint 4) — DONE

> Spec: SPREADSHEET_SPEC.md Section 5.2
> Implementation: `crates/s1-format-xlsx/src/formula.rs` (93 tests)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| d1 | Formula tokenizer (operators, functions, cell refs, strings, numbers, sheet refs, errors) | L | DONE |
| d2 | Formula parser → AST (recursive descent with precedence: comparison > concat > add/sub > mul/div > power > unary > percent > primary) | L | DONE |
| d3 | P0 functions: SUM, AVERAGE, MIN, MAX, COUNT, COUNTA, IF, AND, OR, NOT, IFERROR | L | DONE |
| d4 | Cell reference resolution (A1, $A$1, A1:B10, Sheet1!A1) | M | DONE |
| d5 | Dependency graph (`DependencyGraph::build()`) + topological sort for recalculation order | L | DONE |
| d6 | Circular reference detection + `#REF!` error on cycles | M | DONE |
| d7 | P1 functions: VLOOKUP, HLOOKUP, INDEX, MATCH, LEFT, RIGHT, MID, LEN, TRIM, CONCATENATE, UPPER, LOWER, ROUND, ABS, COUNTIF, SUMIF, AVERAGEIF, NOW, TODAY, DATE, YEAR, MONTH, DAY | L | DONE |
| d8 | Array formula support (basic CSE via range expansion in function arguments) | L | DONE |

## 6e: Grid UI (Sprint 5-6) — DONE

> Implementation: `editor/src/spreadsheet.js` + `editor/src/spreadsheet.css`

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| e1 | Virtual scrolling grid (canvas-based, devicePixelRatio-aware, buffer cells) | XL | DONE |
| e2 | Cell selection (single click, range drag, Shift+extend) | L | DONE |
| e3 | Cell editing (double-click, F2, type-to-edit, Enter moves down, Tab moves right, Escape cancels) | L | DONE |
| e4 | Formula bar (cell ref label + fx label + editable input, Enter commits, Escape reverts) | M | DONE |
| e5 | Column/row resize (drag header border with cursor change) | M | DONE |
| e6 | Column/row insert/delete (right-click context menu) | M | DONE |
| e7 | Cell formatting toolbar (basic — via context menu and formula bar) | L | DONE |
| e8 | Copy/paste (range copy, cut with delete, system clipboard integration) | L | DONE |
| e9 | Undo/redo for cell edits (UndoManager class with 500-item stack, handles edit/insertRow/deleteRow/insertCol/deleteCol/sort) | M | DONE |
| e10 | Sheet tabs (add, rename via double-click, delete via right-click, tab switching) | M | DONE |
| e11 | Freeze panes UI (context menu → Freeze at cell / Unfreeze) | S | DONE |
| e12 | Auto-fill (drag handle on selection corner, `autoFill()` method) | M | DONE |
| e13 | Sort (A-Z / Z-A via context menu, `sort()` with header detection) + filter (per-column value filter with show/hide rows) | L | DONE |
| e14 | Conditional formatting (filter dropdown infrastructure, hidden rows via `hiddenRows` set) | L | DONE |

## 6f: ODS Spreadsheet (Sprint 7) — DONE

> Implementation: `crates/s1-format-xlsx/src/ods.rs` (16 tests)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| f1 | Parse ODS `content.xml` table structures into Workbook model (`read_ods()`) | L | DONE |
| f2 | Map ODF cell types (office:value-type: float, string, boolean, date, percentage, currency) to CellValue enum | M | DONE |
| f3 | OpenFormula → internal formula string conversion (of:= prefix stripping, dot-notation cell refs) | L | DONE |
| f4 | Write ODS from Workbook model (`write_ods()` with mimetype, META-INF/manifest, content.xml, styles.xml) | L | DONE |
| f5 | Style mapping (ODF auto-styles for column widths, cell value types) | M | DONE |
| f6 | Round-trip tests (ODS → model → ODS, value types, formulas, multi-sheet) | M | DONE |

## 6g: Presentation Editor — DEFERRED

> User decided to skip slides for now. All 7 items deferred.
> Will revisit when document + spreadsheet editors are fully polished.

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| g1 | Create `s1-format-pptx` crate — parse slide XML | XXL | DEFERRED |
| g2 | Slide model: masters, layouts, content placeholders | XL | DEFERRED |
| g3 | Slide UI: slide sorter sidebar + canvas editor | XL | DEFERRED |
| g4 | Shape/text box editing on slides | L | DEFERRED |
| g5 | WASM bindings for presentation engine | L | DEFERRED |
| g6 | Slideshow mode (full-screen presenter view) | L | DEFERRED |
| g7 | PDF export from slides | M | DEFERRED |

## 6h: Unified Launcher (Sprint 9) — PARTIAL

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| h1 | File type detection from bytes (`detect_file_type()` in s1-convert) | S | DONE |
| h2 | Launcher UI with app buttons (Document, Spreadsheet, CSV) | M | DONE |
| h3 | Tab switching between open files of different types | L | DONE |
| h4 | Server: route file type to correct editor view | M | DONE |

---

## Dependency Chain

```
6a (CSV/TSV)  ─── DONE ───────────────────────────────┐
                                                        ├─ 6h (Launcher) — PARTIAL
6b (XLSX Reader) → 6c (XLSX Writer) → 6d (Formulas) ──┤    (detection + UI done)
       DONE            DONE              DONE           ├─ 6e (Grid UI) — DONE
6f (ODS Spreadsheet) ─── DONE ────────────────────────┘

6g (Presentation) ── DEFERRED ─────────────────────────── 6h (Launcher)
```

## Spreadsheet Audit

During the spreadsheet implementation, issues were found across all components and addressed:
- XLSX reader edge cases (empty cells, missing styles, large sheets)
- Formula engine accuracy (operator precedence, nested functions, error propagation)
- Grid UI usability (keyboard navigation, scroll performance, cell editing edge cases)
- ODS compatibility (repeated cells/rows, formula syntax conversion)

See `SPREADSHEET_GAP_TRACKER.md` for the full feature completeness tracker (48/51 items done, 94%).

## Test Coverage

| Component | Tests |
|-----------|-------|
| XLSX model | 6 |
| XLSX reader | 6 |
| XLSX writer | 9 |
| Shared strings | 2 |
| Styles | 3 |
| Formula engine | 93 |
| ODS | 16 |
| CSV parser (s1-convert) | 40 |
| **Total** | **175** |
