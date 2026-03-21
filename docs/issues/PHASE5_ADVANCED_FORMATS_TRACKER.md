# Phase 5 — Advanced Format Support Tracker

> Last updated: 2026-03-21
> **Status: COMPLETE** — All 7 items resolved. ZIP preservation handles round-trip for all.
> See `ZIP_PRESERVATION_TRACKER.md` for the `preserved_parts` mechanism.
>
> | Item | Steps | Done | Skipped |
> |------|-------|------|---------|
> | Q6 SmartArt | 5 | 4 | 1 (SVG generation — XL effort) |
> | Q7 Charts | 5 | 4 | 1 (Chart.js — XL effort) |
> | Q8 OLE Objects | 4 | 4 | 0 |
> | P4 VBA Macros | 6 | 5 | 1 (execution — out of scope) |
> | Q10 ODT Columns | 4 | 4 | 0 |
> | Q9 Custom XML | 3 | 3 | 0 |
> | Q13 ODT DB Fields | 3 | 3 | 0 |
> | **Total** | **30** | **27** | **3** |

## Q6: SmartArt Diagrams

**Status**: Preserved and detected. Placeholder rendered with diagram type.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `word/diagrams/` ZIP entries via preserved_parts | M | DONE |
| 2 | Parse `dgm:relIds` from drawing element to extract diagram type | M | DONE |
| 3 | Generate fallback SVG/image from diagram data | XL | SKIPPED (XL effort — fallback placeholder is sufficient) |
| 4 | Render diagram as styled HTML placeholder with description | S | DONE |
| 5 | Write diagram ZIP entries back via preserved_parts loop | M | DONE |

## Q7: Charts

**Status**: Preserved and detected. Chart type extracted and shown in placeholder.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `word/charts/` ZIP entries via preserved_parts | M | DONE |
| 2 | Parse `c:chart` reference to extract chart type | L | DONE |
| 3 | Render chart via Chart.js or lightweight SVG | XL | SKIPPED (XL effort — Chart.js integration deferred) |
| 4 | Show chart as image placeholder with "Chart" label | S | DONE |
| 5 | Write chart ZIP entries back via preserved_parts loop | M | DONE |

## Q8: Embedded OLE Objects

**Status**: Preserved. Preview images extracted via drawing parser when available. Placeholder fallback for non-image OLE.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `word/embeddings/` ZIP entries via preserved_parts | M | DONE |
| 2 | Extract preview image from OLE container (best-effort via drawing parser) | L | DONE |
| 3 | Show preview image / "Embedded Object" placeholder via render_drawing | S | DONE |
| 4 | Write OLE entries back via preserved_parts loop | M | DONE |

## P4: VBA Macros

**Status**: Detected, preserved, warned, macro names extracted and shown.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Detect `vbaProject.bin` → hasMacros metadata flag | S | DONE |
| 2 | Show security warning banner in editor ("This document contains macros") | S | DONE |
| 3 | Preserve via preserved_parts | S | DONE |
| 4 | Parse VBA project structure to list macro names | L | DONE |
| 5 | Display macro list in properties panel | M | DONE |
| 6 | Execution is OUT OF SCOPE (security risk) | — | N/A |

## Q10: ODT Column Widths

**Status**: Column widths parsed from auto-styles, resolved to points, and stored on table nodes.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Parse `style:column-width` from automatic styles | S | DONE |
| 2 | Apply resolved widths to table model | M | DONE |
| 3 | Render column widths in HTML table | S | DONE |
| 4 | Round-trip test | S | DONE |

## Q9: Custom XML Parts

**Status**: Fully preserved and round-trip tested.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `customXml/` via preserved_parts | M | DONE |
| 2 | Written back via preserved_parts loop | M | DONE |
| 3 | Round-trip test | S | DONE |

## Q13: ODT Database Fields

**Status**: Database display fields parsed and preserved as text runs.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Parse `text:database-display` elements | S | DONE |
| 2 | Store as read-only field placeholder (text run) | S | DONE |
| 3 | Render as styled inline element (via normal text rendering) | S | DONE |
