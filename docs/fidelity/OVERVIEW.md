# s1engine Fidelity Audit — Executive Summary

**Date:** 2026-03-29 (updated after Phase 1-3 implementation)
**Scope:** All crates in s1engine document engine

---

## Current State

s1engine is a modular document engine with 14 crates, **1,660+ tests passing**, zero clippy warnings. The core architecture (model, operations, CRDT) is solid. Format fidelity has been significantly improved through a 3-phase effort.

### Coverage Summary

| Crate | Maturity | Coverage vs Spec |
|-------|----------|-----------------|
| **s1-model** | HIGH | Comprehensive tree model with 100+ attribute variants |
| **s1-ops** | HIGH | Complete operation/undo/redo system (48 tests) |
| **s1-crdt** | HIGH | Fugue text CRDT + tree CRDT (182 tests) |
| **s1-format-docx** | HIGH | **~72%** of OOXML WordprocessingML spec |
| **s1-format-odt** | MEDIUM-HIGH | **~58%** of ODF 1.2 text document spec |
| **s1-format-pdf** | MEDIUM | Export-only, full visual fidelity via layout |
| **s1-format-txt** | HIGH | Complete for plain text (encoding detection) |
| **s1-format-md** | HIGH | Complete for CommonMark + GFM |
| **s1-layout** | MEDIUM | Full pagination, needs incremental improvement |
| **s1-text** | HIGH | Pure-Rust shaping, fonts, BiDi, line breaking |
| **s1-convert** | MEDIUM | DOC reader is basic, CSV is complete |
| **s1engine** | HIGH | Clean facade with builder, undo, export |

### What Works Well

- Text content, basic formatting (bold/italic/underline/color/font/size)
- Paragraph properties (alignment, spacing, indentation, borders, tabs)
- Tables (structure, cell merge, borders, shading)
- Lists (basic numbered/bulleted)
- Images (inline and floating with positioning)
- Headers/footers (per-section, with page number fields)
- Styles (paragraph + character with inheritance)
- Comments, footnotes, endnotes
- Bookmarks, hyperlinks
- Track changes (basic insert/delete/format with metadata)
- Metadata (title, author, subject, dates)
- Document-level settings (page size, margins, orientation)

### What's Missing

| Gap | Impact | Details |
|-----|--------|---------|
| ~~**Advanced fields**~~ | ~~HIGH~~ | DONE: HYPERLINK, REF, SEQ, MERGEFIELD, IF, STYLEREF types added |
| ~~**Widow/orphan control**~~ | ~~HIGH~~ | DONE: DOCX + ODT read/write |
| ~~**Table cell margins**~~ | ~~HIGH~~ | DONE: tblCellMar + tcMar read/write |
| ~~**Table layout mode**~~ | ~~HIGH~~ | DONE: Fixed/AutoFit enum |
| ~~**Row height**~~ | ~~HIGH~~ | DONE: trHeight with hRule |
| ~~**ODT ordered lists**~~ | ~~HIGH~~ | DONE: Heuristic detection via continue-numbering/start-value |
| **ODT change tracking** | HIGH | Still raw XML preserved |
| ~~**Complex-script fonts**~~ | ~~MEDIUM~~ | DONE: FontFamilyCS, FontSizeCS, BoldCS, ItalicCS |
| ~~**Text caps/smallcaps**~~ | ~~MEDIUM~~ | DONE: Caps, SmallCaps attributes |
| ~~**Outline levels**~~ | ~~MEDIUM~~ | DONE: OutlineLevel attribute |
| **Theme colors/fonts** | MEDIUM | Office theme references still lost |
| **Drawing shapes** | LOW | Only images, no shapes/WordArt |
| **Math equations** | LOW | Content dropped |

---

## Conversion Matrix (Quick Reference)

```
FROM ╲ TO │  DOCX    ODT     PDF     TXT     MD      CSV
──────────┼──────────────────────────────────────────────
DOCX      │   —      HIGH    FULL    BASIC   PARTIAL LOSSY
ODT       │  HIGH     —      FULL    BASIC   PARTIAL LOSSY
TXT       │  FULL    FULL    FULL     —      FULL    LOSSY
MD        │  FULL    FULL    FULL    BASIC    —      LOSSY
DOC       │ PARTIAL  PARTIAL PARTIAL BASIC   BASIC   LOSSY
CSV       │  FULL    FULL    FULL    BASIC   PARTIAL  —
```

PDF is export-only (no reader). DOC is read-only (no writer).

See [CONVERSION_MATRIX.md](CONVERSION_MATRIX.md) for full details with feature-by-feature breakdown.

---

## Architecture Diagram

```
┌─────────────────────────────────────────┐
│              s1engine (facade)           │
│  Engine, Document, Builder, Format      │
└────────┬───────────┬───────────┬────────┘
         │           │           │
    ┌────┴──┐   ┌────┴──┐   ┌───┴───┐
    │s1-ops │   │s1-crdt│   │s1-conv│
    │undo   │   │collab │   │DOC/CSV│
    └───┬───┘   └───┬───┘   └───┬───┘
        │           │            │
   ┌────┴───────────┴────────────┴────┐
   │         s1-model (ZERO deps)      │
   │  DocumentModel, Node, Attributes  │
   └──────────────────────────────────┘
         │           │          │
    ┌────┴──┐   ┌────┴──┐  ┌───┴────┐
    │ DOCX  │   │  ODT  │  │TXT/MD  │
    │reader │   │reader │  │reader  │
    │writer │   │writer │  │writer  │
    └───────┘   └───────┘  └────────┘
                    │
              ┌─────┴──────┐
              │  s1-layout  │
              │ pagination  │
              └─────┬──────┘
                    │
              ┌─────┴──────┐
              │s1-format-pdf│
              │  PDF export │
              └────────────┘
```

---

## XLSX Decision

**Recommendation: Separate out.**

`s1-format-xlsx` is a standalone crate with zero coupling to the core engine. It's only used directly in the WASM FFI layer. Spreadsheets have a fundamentally different data model (cells, formulas, sheets) from documents (paragraphs, runs, styles).

Options:
- A) Feature-gate it (keep in workspace but off by default)
- B) Move to separate repository
- C) Keep but document as separate product

---

## Improvement Plan (Summary)

| Phase | Focus | Outcome |
|-------|-------|---------|
| **Phase 1** | Model completeness | Add ~15 missing attribute variants |
| **Phase 2** | DOCX fidelity | Raise from ~57% to ~75% |
| **Phase 3** | ODT fidelity | Raise from ~43% to ~70% |
| **Phase 4** | Advanced features | Themes, shapes, math, SDT binding |
| **Phase 5** | Code health | Split large files, fix safety, CRDT GC |

See [IMPROVEMENT_PLAN.md](IMPROVEMENT_PLAN.md) for full details with specific features per phase.

---

## Files in This Directory

| File | Contents |
|------|----------|
| `README.md` | Index and methodology |
| `OVERVIEW.md` | This file — executive summary |
| `CONVERSION_MATRIX.md` | Full FROM->TO matrix with feature tracking |
| `DOCX_FIDELITY.md` | Element-by-element DOCX audit |
| `ODT_FIDELITY.md` | Element-by-element ODT audit |
| `DOCUMENT_MODEL_REVIEW.md` | Core model architecture + gaps |
| `IMPROVEMENT_PLAN.md` | Phased improvement plan |
| `OOXML_SPEC_CHECKLIST.md` | Full ECMA-376 spec checklist (~800 features) |
| `ODF_SPEC_CHECKLIST.md` | Full ODF 1.2/1.3 spec checklist (~810 features) |
