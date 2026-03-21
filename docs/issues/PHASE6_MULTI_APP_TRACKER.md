# Phase 6 — Multi-App Suite Tracker

> Full specification: `docs/specs/SPREADSHEET_SPEC.md`
> Last updated: 2026-03-21

## Status Overview

| Sub-Phase | Sprint | Items | Done | Status |
|-----------|--------|-------|------|--------|
| 6-Foundation | — | 4 | 3 | **PARTIAL** |
| 6a-CSV/TSV | 1 | 8 | 8 | **DONE** (CSV agent) |
| 6b-XLSX Reader | 2 | 10 | 10 | **DONE** (cells, formulas, styles, columns, rows, frozen panes, merges, preserved parts) |
| 6c-XLSX Writer | 3 | 7 | 7 | **DONE** (full round-trip with styles, columns, rows, panes, preserved parts) |
| 6d-Formula Engine | 4 | 8 | 8 | **DONE** (tokenizer, parser, 30+ functions, dependency graph, cycle detection) |
| 6e-Grid UI | 5-6 | 14 | 0 | NOT STARTED |
| 6f-ODS Spreadsheet | 7 | 6 | 0 | NOT STARTED |
| 6g-Presentation | 8+ | 7 | 0 | NOT STARTED |
| 6h-Launcher | 9 | 4 | 1 | PARTIAL |

---

## 6-Foundation (PARTIAL)

| # | Description | Status |
|---|-------------|--------|
| F1 | File type detection (DOCX/XLSX/PPTX/ODT/ODS/ODP/PDF/CSV) — `detect_file_type()` in s1-convert | DONE |
| F2 | Launcher UI buttons (Document, Spreadsheet, Presentation, CSV) | DONE |
| F3 | CSV file input accept in editor HTML | DONE |
| F4 | Server file type routing in upload response | NOT STARTED |

## 6a: CSV/TSV Parser (Sprint 1)

> Spec: SPREADSHEET_SPEC.md Section 4

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| a1 | CSV parser: RFC 4180 with all 8 edge cases (quoting, escaping, multiline, BOM) | M | NOT STARTED |
| a2 | TSV parser: tab-delimited variant | S | NOT STARTED |
| a3 | CSV writer: export data model to CSV | S | NOT STARTED |
| a4 | CSV → DOCX table conversion | S | DONE (s1-convert) |
| a5 | Auto-detect delimiter (comma vs tab vs semicolon) | S | NOT STARTED |
| a6 | Encoding detection (UTF-8, Latin-1, BOM stripping) | S | NOT STARTED |
| a7 | Large file streaming (>1M rows without OOM) | M | NOT STARTED |
| a8 | Round-trip tests (CSV → model → CSV) | S | NOT STARTED |

## 6b: XLSX Reader (Sprint 2)

> Spec: SPREADSHEET_SPEC.md Section 2

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| b1 | Create `s1-format-xlsx` crate with ZIP reader scaffold | M | NOT STARTED |
| b2 | Parse `xl/sharedStrings.xml` → string table | S | NOT STARTED |
| b3 | Parse `xl/styles.xml` → number formats, fonts, fills, borders | L | NOT STARTED |
| b4 | Parse `xl/worksheets/sheetN.xml` → rows, cells, cell types | L | NOT STARTED |
| b5 | Parse `xl/workbook.xml` → sheet names, defined names, active sheet | S | NOT STARTED |
| b6 | Formula string extraction (store as string, no evaluation) | S | NOT STARTED |
| b7 | Merged cell ranges | S | NOT STARTED |
| b8 | Column widths + row heights | S | NOT STARTED |
| b9 | Frozen panes | S | NOT STARTED |
| b10 | XLSX reader tests with real Excel files | M | NOT STARTED |

## 6c: XLSX Writer (Sprint 3)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| c1 | Generate `sharedStrings.xml` from string table | S | NOT STARTED |
| c2 | Generate `styles.xml` from style model | L | NOT STARTED |
| c3 | Generate `worksheetN.xml` with cell data | L | NOT STARTED |
| c4 | Generate `workbook.xml` with sheet references | S | NOT STARTED |
| c5 | ZIP packaging with Content_Types + relationships | M | NOT STARTED |
| c6 | Preserve unrecognized XML for round-trip fidelity | M | NOT STARTED |
| c7 | Round-trip tests (XLSX → model → XLSX → reopen → compare) | M | NOT STARTED |

## 6d: Formula Engine (Sprint 4)

> Spec: SPREADSHEET_SPEC.md Section 5.2

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| d1 | Formula tokenizer (operators, functions, cell refs, strings, numbers) | L | NOT STARTED |
| d2 | Formula parser → AST (recursive descent) | L | NOT STARTED |
| d3 | P0 functions: SUM, AVERAGE, MIN, MAX, COUNT, IF, AND, OR, NOT | L | NOT STARTED |
| d4 | Cell reference resolution (A1, $A$1, A1:B10, Sheet1!A1) | M | NOT STARTED |
| d5 | Dependency graph + topological sort for recalculation order | L | NOT STARTED |
| d6 | Circular reference detection + error reporting | M | NOT STARTED |
| d7 | P1 functions: VLOOKUP, HLOOKUP, INDEX, MATCH, TEXT, DATE, ROUND, ABS | L | NOT STARTED |
| d8 | Array formula support (basic CSE) | L | NOT STARTED |

## 6e: Grid UI (Sprint 5-6)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| e1 | Virtual scrolling grid (canvas-based for performance) | XL | NOT STARTED |
| e2 | Cell selection (single, range, Ctrl+range, Shift+extend) | L | NOT STARTED |
| e3 | Cell editing (double-click, F2, type-to-edit, Enter/Tab navigation) | L | NOT STARTED |
| e4 | Formula bar (shows formula for selected cell, editable) | M | NOT STARTED |
| e5 | Column/row resize (drag header border) | M | NOT STARTED |
| e6 | Column/row insert/delete (right-click menu) | M | NOT STARTED |
| e7 | Cell formatting toolbar (number format, font, alignment, borders, fill) | L | NOT STARTED |
| e8 | Copy/paste (single cell, range, cross-sheet) | L | NOT STARTED |
| e9 | Undo/redo for cell edits | M | NOT STARTED |
| e10 | Sheet tabs (add, rename, delete, reorder, right-click menu) | M | NOT STARTED |
| e11 | Freeze panes UI (View menu → Freeze) | S | NOT STARTED |
| e12 | Auto-fill (drag handle on selection corner) | M | NOT STARTED |
| e13 | Sort + filter (column header dropdown) | L | NOT STARTED |
| e14 | Conditional formatting (highlight rules, color scales, data bars) | L | NOT STARTED |

## 6f: ODS Spreadsheet (Sprint 7)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| f1 | Parse ODS `content.xml` table structures into Workbook model | L | NOT STARTED |
| f2 | Map ODF cell types (office:value-type) to CellValue enum | M | NOT STARTED |
| f3 | OpenFormula → internal formula string conversion | L | NOT STARTED |
| f4 | Write ODS from Workbook model | L | NOT STARTED |
| f5 | Style mapping (ODF auto-styles ↔ cell styles) | M | NOT STARTED |
| f6 | Round-trip tests (ODS → model → ODS) | M | NOT STARTED |

## 6g: Presentation Editor (Sprint 8+)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| g1 | Create `s1-format-pptx` crate — parse slide XML | XXL | NOT STARTED |
| g2 | Slide model: masters, layouts, content placeholders | XL | NOT STARTED |
| g3 | Slide UI: slide sorter sidebar + canvas editor | XL | NOT STARTED |
| g4 | Shape/text box editing on slides | L | NOT STARTED |
| g5 | WASM bindings for presentation engine | L | NOT STARTED |
| g6 | Slideshow mode (full-screen presenter view) | L | NOT STARTED |
| g7 | PDF export from slides | M | NOT STARTED |

## 6h: Unified Launcher (Sprint 9)

| # | Description | Effort | Status |
|---|-------------|--------|--------|
| h1 | File type detection from bytes | S | DONE |
| h2 | Launcher UI with app buttons | M | DONE |
| h3 | Tab switching between open files of different types | L | NOT STARTED |
| h4 | Server: route file type to correct editor view | M | NOT STARTED |

---

## Dependency Chain

```
6a (CSV/TSV) ──────────────────────────────────────────┐
                                                        ├─ 6h (Launcher)
6b (XLSX Reader) → 6c (XLSX Writer) → 6d (Formulas) ──┤
                                                        ├─ 6e (Grid UI)
6f (ODS Spreadsheet) ──────────────────────────────────┘

6g (Presentation) ── independent track ──────────────── 6h (Launcher)
```

## Priority: Start with 6a (CSV/TSV)

CSV/TSV is the quickest path to spreadsheet data handling. It doesn't require a formula engine or grid UI. A CSV can be opened as a table in the document editor, or exported from a DOCX table. This provides immediate value while the full spreadsheet editor is built.
