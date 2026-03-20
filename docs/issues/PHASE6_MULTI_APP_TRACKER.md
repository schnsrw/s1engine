# Phase 6 — Multi-App Suite Tracker

> Spreadsheet and presentation editors alongside documents.
> This is the longest-term initiative — 3+ sprints of new product development.

## P2a: Spreadsheet Editor

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Create `s1-format-xlsx` crate — OOXML spreadsheet parser | XXL | NOT STARTED |
| 2 | Cell model: grid with types (text, number, date, formula) | XL | NOT STARTED |
| 3 | Formula engine: basic functions (SUM, AVERAGE, IF, VLOOKUP) | XXL | NOT STARTED |
| 4 | Grid UI: virtual scrolling canvas with cell selection | XL | NOT STARTED |
| 5 | Cell formatting: number formats, borders, fill colors | L | NOT STARTED |
| 6 | WASM bindings for spreadsheet | L | NOT STARTED |
| 7 | Editor integration: tab switching, file type detection | M | NOT STARTED |
| 8 | CSV import/export | M | NOT STARTED |
| 9 | Charts from spreadsheet data | XL | NOT STARTED |

## P2b: Presentation Editor

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Create `s1-format-pptx` crate — OOXML presentation parser | XXL | NOT STARTED |
| 2 | Slide model: slide master, layouts, content placeholders | XL | NOT STARTED |
| 3 | Slide UI: slide sorter, canvas editor, transitions | XL | NOT STARTED |
| 4 | Shape/text box editing on slides | L | NOT STARTED |
| 5 | WASM bindings for presentation | L | NOT STARTED |
| 6 | Slideshow mode (full-screen presenter view) | L | NOT STARTED |
| 7 | PDF export from slides | M | NOT STARTED |

## P1: Unified Launcher

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | File type detection from bytes (DOCX vs XLSX vs PPTX) | S | NOT STARTED |
| 2 | Launcher UI: "New Document / Spreadsheet / Presentation" | M | NOT STARTED |
| 3 | Tab switching between open files of different types | L | NOT STARTED |
| 4 | Server: file type routing to correct editor | M | NOT STARTED |

---

## Dependency Chain

```
P2a (Spreadsheet) ─┐
                    ├─ P1 (Launcher) ─ Complete suite
P2b (Presentation) ─┘
```

Both P2a and P2b can be developed in parallel. P1 integrates them.
