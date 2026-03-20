# Phase 5 — Advanced Format Support Tracker

> Complex OOXML/ODF features found in enterprise documents.
> **ZIP preservation infrastructure now handles round-trip for most items.**
> See `ZIP_PRESERVATION_TRACKER.md` for the `preserved_parts` mechanism.

## Q6: SmartArt Diagrams

**Status**: Dropped on import.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `word/diagrams/` ZIP entries via preserved_parts | M | DONE |
| 2 | Parse `dgm:relIds` from drawing element to find diagram parts | M | NOT STARTED |
| 3 | Generate fallback SVG/image from diagram data | XL | NOT STARTED |
| 4 | Render diagram as styled HTML placeholder with description | S | NOT STARTED |
| 5 | Write diagram ZIP entries back via preserved_parts loop | M | DONE |

## Q7: Charts

**Status**: Chart reference preserved as raw XML via DrawingML round-trip.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `word/charts/` ZIP entries via preserved_parts | M | DONE |
| 2 | Parse `c:chart` reference to extract chart type and data | L | NOT STARTED |
| 3 | Render chart via Chart.js or lightweight SVG | XL | NOT STARTED |
| 4 | Show chart as image placeholder with "Chart" label | S | CAN DO NOW |
| 5 | Write chart ZIP entries back via preserved_parts loop | M | DONE |

## Q8: Embedded OLE Objects

**Status**: Dropped on import.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `word/embeddings/` ZIP entries via preserved_parts | M | DONE |
| 2 | Extract preview image from OLE container | L | NOT STARTED |
| 3 | Show preview image with "Embedded object" overlay | S | NOT STARTED |
| 4 | Write OLE entries back via preserved_parts loop | M | DONE |

## P4: VBA Macros

**Status**: `vbaProject.bin` preserved in ZIP but not accessible.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Detect `vbaProject.bin` → hasMacros metadata flag | S | DONE |
| 2 | Show security warning banner in editor ("This document contains macros") | S | NOT STARTED |
| 3 | Preserve via preserved_parts | S | DONE |
| 4 | Parse VBA project structure to list macro names | L | NOT STARTED |
| 5 | Display macro list in properties panel | M | NOT STARTED |
| 6 | Execution is OUT OF SCOPE (security risk) | — | N/A |

## Q10: ODT Column Widths

**Status**: Style names stored but actual widths not resolved.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Parse `style:column-width` from automatic styles | S | NOT STARTED |
| 2 | Apply resolved widths to table model | M | NOT STARTED |
| 3 | Render column widths in HTML table | S | NOT STARTED |
| 4 | Round-trip test | S | NOT STARTED |

## Q9: Custom XML Parts

**Status**: Dropped on import.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `customXml/` via preserved_parts | M | DONE |
| 2 | Written back via preserved_parts loop | M | DONE |
| 3 | Round-trip test | S | NOT STARTED |

## Q13: ODT Database Fields

**Status**: Not supported.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Parse `text:database-display` elements | S | NOT STARTED |
| 2 | Store as read-only field placeholder | S | NOT STARTED |
| 3 | Render as styled inline element | S | NOT STARTED |
