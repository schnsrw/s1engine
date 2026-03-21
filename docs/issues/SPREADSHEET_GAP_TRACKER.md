# Rudra Office Spreadsheet — Gap Tracker

> Feature completeness tracker
> Current: ~92% feature-complete. Target: 80%+
> Last updated: 2026-03-22

## Phase S1: Make It Usable (Sprint 1) — CRITICAL

| # | Feature | Severity | Status |
|---|---------|----------|--------|
| S1.1 | **Menu bar** | CRITICAL | DONE |
| S1.2 | **Cell formatting toolbar** + canvas rendering (bold/italic/color/fill/borders/alignment) | CRITICAL | DONE |
| S1.3 | **XLSX export** from UI (JS ZIP builder) | CRITICAL | DONE |
| S1.4 | **Number formats** — currency, percentage, date, time, scientific | CRITICAL | DONE |
| S1.5 | **Find & Replace** (Ctrl+F/H, highlight matches, replace all) | HIGH | DONE |
| S1.6 | **Print to PDF** — HTML table print via iframe | HIGH | DONE |
| S1.7 | **Merge cells UI** — merge/unmerge from toolbar + menu | HIGH | DONE |
| S1.8 | **Paste Special** — values, formulas, formatting, transpose | HIGH | DONE |
| S1.9 | **View switching** — doc toolbar ↔ sheet toolbar | CRITICAL | DONE |
| S1.10 | **Custom modals** replacing prompt/confirm | HIGH | DONE |

## Phase S2: Data Intelligence (Sprint 2)

| # | Feature | Severity | Status |
|---|---------|----------|--------|
| S2.1 | **Conditional formatting** — rules, color scales, dialog | CRITICAL | DONE |
| S2.2 | **Data validation** — dropdown lists, number ranges, error toast | CRITICAL | DONE |
| S2.3 | **Cell comments** — red triangle, hover tooltip, insert/edit/delete | MEDIUM | DONE |
| S2.4 | **Multi-level sort** — sort dialog with add/remove criteria | MEDIUM | DONE |
| S2.5 | **Advanced filter** — text/number/date filter types, multi-filter AND logic | MEDIUM | DONE |
| S2.6 | **Remove duplicates** — column selection dialog, preview, remove | HIGH | DONE |
| S2.7 | **Named ranges** — define, use in formulas, name box dropdown | MEDIUM | DONE |
| S2.8 | **Sheet management** — double-click rename, right-click menu, drag reorder | MEDIUM | DONE |

## Phase S3: Visualization (Sprint 3)

| # | Feature | Severity | Status |
|---|---------|----------|--------|
| S3.1 | **Column/Bar charts** — grouped multi-series, Y-axis scale, gridlines | CRITICAL | DONE |
| S3.2 | **Line/Area charts** — connected dots, multi-series, fill | CRITICAL | DONE |
| S3.3 | **Pie/Doughnut charts** — proportional slices, percentage labels, legend | CRITICAL | DONE |
| S3.4 | **Chart customization** — title, legend, draggable, resizable, close | HIGH | DONE |
| S3.5 | **Insert images** — floating, draggable, resizable overlay | HIGH | DONE |
| S3.6 | **Sparklines** — inline line/bar mini-charts in cells | MEDIUM | DONE |

## Phase S4: Collaboration & Polish (Sprint 4)

| # | Feature | Severity | Status |
|---|---------|----------|--------|
| S4.1 | **Real-time collab** — WebSocket cell/format sync, peer cursor overlay | CRITICAL | DONE |
| S4.2 | **Sharing** — share link with ?type=sheet, same flow as docs | HIGH | DONE |
| S4.3 | **Zoom** — Ctrl+mousewheel, dropdown, CSS transform, persist | MEDIUM | DONE |
| S4.4 | **Formula autocomplete** — dropdown with function names + descriptions | MEDIUM | DONE |
| S4.5 | **Formula syntax highlighting** — color-coded cell references | MEDIUM | DONE |
| S4.6 | **Keyboard shortcuts help** — Ctrl+/ modal with all shortcuts | MEDIUM | DONE |
| S4.7 | **Accessibility** — ARIA labels on canvas, screen reader support | HIGH | DONE |
| S4.8 | **Missing formulas** — FIND, SEARCH, SUBSTITUTE, SIN, COS, TAN, LOG, EXP, TEXT, VALUE + more | MEDIUM | DONE |

## Phase S5: Advanced (Sprint 5+)

| # | Feature | Severity | Status |
|---|---------|----------|--------|
| S5.1 | **Pivot tables** — basic dialog, row/col/value fields, aggregation, new sheet | HIGH | DONE |
| S5.2 | **Text to columns** — delimiter selection, preview, split | HIGH | DONE |
| S5.3 | **Cross-sheet references** — Sheet1!A1 in formulas, SheetLookup trait | MEDIUM | DONE |
| S5.4 | **Array formulas** — CSE (Ctrl+Shift+Enter) | MEDIUM | DONE |
| S5.5 | **Hyperlinks** — Ctrl+Click to follow, blue underline, insert dialog | MEDIUM | DONE |
| S5.6 | **Shapes/drawings** — basic shapes, text boxes | LOW | NOT STARTED |
| S5.7 | **Mobile/touch** — responsive layout, touch gestures | MEDIUM | NOT STARTED |

## Summary

| Phase | Items | Priority | Status |
|-------|-------|----------|--------|
| S1: Usable | 10 | CRITICAL | **DONE** |
| S2: Data Intelligence | 8 | HIGH | **DONE** |
| S3: Visualization | 6 | CRITICAL | **DONE** |
| S4: Collaboration | 8 | HIGH | **DONE** |
| S5: Advanced | 7 | MEDIUM | **MOSTLY DONE** (6/7) |
| **Total** | **51** | | **48/51 DONE (94%)** |

## Additional Items (User Reported)

| # | Feature | Severity | Status |
|---|---------|----------|--------|
| U1 | **Click column header → select entire column** | CRITICAL | DONE (selection agent) |
| U2 | **Click row header → select entire row** | CRITICAL | DONE (selection agent) |
| U3 | **Click top-left corner → select all** | CRITICAL | DONE (selection agent) |
| U4 | **Drag column header border → resize column** | CRITICAL | DONE (selection agent) |
| U5 | **Drag row header border → resize row** | CRITICAL | DONE (selection agent) |
| U6 | **Double-click column border → auto-fit width** | HIGH | DONE (selection agent) |
| U7 | **Selected header highlighting** (blue background + bold text) | HIGH | DONE (selection agent) |
| U8 | **Cell formatting** — bold/italic/color/fill/borders render on canvas | CRITICAL | DONE (format agent) |
| U9 | **Number formats** — currency/percentage/date display | CRITICAL | DONE (format agent) |
| U10 | **Find & Replace** (Ctrl+F/Ctrl+H) | HIGH | DONE (format agent) |
| U11 | **Format preservation on copy/paste** | HIGH | DONE |
| U12 | **Table detection in imported data** | MEDIUM | NOT STARTED |
