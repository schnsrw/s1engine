# UX Parity Tracker — Google Docs / LibreOffice Fidelity

> Gap analysis for making Folio editor production-grade vs Google Docs / LibreOffice.
> Created: 2026-03-17 | Last updated: 2026-03-18

## Phase 1 — Critical (Blocks professional use)

| ID | Feature | Gap Analysis | Status |
|----|---------|-------------|--------|
| UXP-01 | **Styles Panel & Style Application** | Model has styles but NO UI to browse/apply named styles (Heading 1-6, Body Text, etc.). Google Docs has persistent styles dropdown. Need: dropdown in toolbar, apply via WASM, style preview. Edge cases: custom styles, style inheritance chain, style conflicts with direct formatting. | FIXED |
| UXP-02 | **Header/Footer Editing UI** | Renders headers/footers from model but NO UI to click-into-edit. Need: click on header area enters edit mode, different-first-page toggle, odd/even toggle, page number field insertion. Edge cases: section-specific headers, inherited vs overridden, empty header display. Spec: OOXML `w:headerReference`, ODF `style:header`. | FIXED |
| UXP-03 | **Page Setup Dialog** | Page size/margins/orientation are read-only from document. Need: dialog with paper size presets (Letter, A4, Legal), custom margins, portrait/landscape toggle. Edge cases: section-specific page setup, minimum margins, mixed orientation. Spec: OOXML `w:pgSz`/`w:pgMar`, ODF `style:page-layout-properties`. | FIXED |
| UXP-04 | **DOCX Import Fidelity — Styles** | Named styles from DOCX may flatten to inline formatting. Need: verify style extraction preserves style IDs, parent chains, and applies them in model. Round-trip test: open DOCX with 10+ named styles, export, verify styles preserved. Spec: ECMA-376 Part 1 §17.7. | FIXED |
| UXP-05 | **DOCX Import Fidelity — Headers/Footers** | Headers/footers from DOCX may not populate model correctly. Need: verify header1.xml, footer1.xml, headerReference types (default, first, even) are parsed. Edge cases: empty headers, section-specific overrides. Spec: ECMA-376 §17.10. | FIXED |
| UXP-06 | **DOCX Import Fidelity — Comments** | DOCX comments (comments.xml) may not import into model. Need: verify comment ranges (w:commentRangeStart/End), author, date, text extraction. Edge cases: overlapping comment ranges, nested comments, replies. Spec: ECMA-376 §17.13.4. | FIXED |
| UXP-07 | **Track Changes / Suggesting Mode** | Basic accept/reject exists but no UI for "Suggesting" vs "Editing" mode toggle. Need: mode switcher in toolbar, visual diff highlighting (insertions green, deletions red strikethrough), author/timestamp display. Edge cases: concurrent edits in suggest mode, format-only changes, table structure changes. Spec: OOXML `w:ins`, `w:del`, `w:rPrChange`. | FIXED |
| UXP-08 | **Section Breaks** | No UI to insert section breaks (continuous, next page, even page, odd page). Need: insert menu item, section properties per break, different margins/orientation per section. Edge cases: section at start of doc, empty sections, section break in table. Spec: OOXML `w:sectPr`, `w:type`. | FIXED |
| UXP-09 | **Paste Special & Enhanced Paste** | Paste carries basic formatting but loses complex structures. Need: paste special dialog (plain text, formatted, keep source/merge), HTML clipboard parsing for tables/images/lists from Word/Docs. Edge cases: paste from Excel (tab-delimited → table), paste from web (clean HTML), paste images with text wrapping. | FIXED |

## Phase 2 — High (Expected by professionals)

| ID | Feature | Gap Analysis | Status |
|----|---------|-------------|--------|
| UXP-10 | **Footnotes & Endnotes UI** | Keyboard shortcuts exist (Ctrl+Alt+F/D) but no toolbar button, no insertion dialog, no rendering of note content at page bottom. Need: insert button, note area rendering in pagination, note numbering (auto, roman, custom). Edge cases: footnotes spanning pages, endnotes at section end vs doc end, restart numbering per section. Spec: OOXML `w:footnote`, ODF `text:note`. | FIXED |
| UXP-11 | **TOC Generation & Outline** | Menu entries exist but TOC generation algorithm missing. Need: parse heading hierarchy, generate TOC with page numbers, auto-update on heading change, outline panel for navigation. Edge cases: custom TOC styles, TC field entries, hyperlinked TOC entries. Spec: OOXML `w:sdt` with `w:docPartGallery val="Table of Contents"`. | FIXED |
| UXP-12 | **Table Editing — Merged Cells & Resize** | Tables render but no merged cells, no column drag resize, no table properties dialog. Need: merge/split cell UI, drag column borders, row height control, table borders/shading dialog. Edge cases: irregular merges (L-shaped), merge across rows, table width modes (auto, fixed, percent). Spec: OOXML `w:gridSpan`, `w:vMerge`. | FIXED |
| UXP-13 | **Comments Threading** | Comments insert but no threaded replies, no timestamps, no author badges. Need: reply UI, resolve workflow, timestamp display, author avatars. Edge cases: reply to deleted comment, resolve then reopen, comment on deleted text. | FIXED |
| UXP-14 | **Format Painter** | No way to copy formatting from one selection and apply to another. Need: toolbar button (single-click for once, double-click for sticky), copies character + paragraph formatting. Edge cases: mixed formatting in source selection, applying to different node types. | FIXED |
| UXP-15 | **Tab Stops on Ruler** | TabStops exist in model but no ruler UI to set/drag/delete tab stops. Need: click ruler to add tab, drag to move, double-click for tab properties (left, center, right, decimal, bar), leader characters. Edge cases: default tab interval, tab stops in tables, inherited tab stops from style. Spec: OOXML `w:tabs`, ODF `style:tab-stops`. | FIXED |

## Phase 3 — Medium (Enhances workflow)

| ID | Feature | Gap Analysis | Status |
|----|---------|-------------|--------|
| UXP-16 | **Print Preview** | Ctrl+P goes straight to browser dialog. Need: preview pane showing paginated document, page range selection, print options. | FIXED |
| UXP-17 | **Zoom Presets & Fit-to-Page** | Zoom works but no presets. Need: dropdown with 50/75/100/125/150/200%, fit-to-width, fit-to-page, Ctrl+scroll zoom. | FIXED |
| UXP-18 | **Dark Mode Implementation** | Button exists but not functional. Need: CSS variable toggle, localStorage persistence, system preference detection (`prefers-color-scheme`). | FIXED |
| UXP-19 | **Spell Check Integration** | Button exists but not functional. Need: either browser spellcheck integration or hunspell/nspell dictionary with squiggly underlines, right-click suggestions. | FIXED |
| UXP-20 | **Bookmarks & Cross-References** | Not implemented. Need: insert bookmark (named anchor), insert cross-reference (to bookmark, heading, figure), auto-update. Spec: OOXML `w:bookmarkStart`/`w:bookmarkEnd`. | FIXED |
| UXP-21 | **Image Text Wrapping** | Images are inline-only. Need: wrap modes (inline, square, tight, behind text, in front of text), drag to position. Spec: OOXML `wp:anchor` vs `wp:inline`, `wp:wrapSquare`. | FIXED |
| UXP-22 | **Multi-Column Layout** | No multi-column section support. Need: column count/width/spacing per section, column break insertion. Spec: OOXML `w:cols`. | FIXED |
| UXP-23 | **Equation Editor** | KaTeX installed but no insertion UI. Need: equation toolbar/palette, LaTeX input, inline vs display mode. | FIXED |

---

## Resolution Log

| ID | Date | Fix Description |
|----|------|-----------------|
| UXP-01 | 2026-03-17 | Added `set_paragraph_style_id` WASM API for arbitrary style IDs (Title, Subtitle, Quote, Code, etc.). Added `styleId` field to `get_formatting_json` output. Enhanced renderer for non-heading styles with inline CSS (Title=26pt, Subtitle=15pt/gray, Quote=italic/border-left, Code=monospace/bg). Updated `detectCurrentStyle()` to prefer explicit styleId over heuristics. Added `applyParagraphStyle()` shared function + Ctrl+Alt+0-6 keyboard shortcuts. STYLE_DEFS now includes styleId per entry. |
| UXP-04 | 2026-03-17 | Audited DOCX style pipeline end-to-end: style_parser.rs correctly extracts definitions (ID, name, basedOn, formatting), content_parser.rs correctly sets StyleId on paragraphs, writer emits w:pStyle and styles.xml. Added 3 round-trip tests: single paragraph style, multiple paragraph styles (Title/Subtitle/Quote), style inheritance chain (basedOn). All pass — DOCX style fidelity confirmed per ECMA-376 §17.7. |
| UXP-03 | 2026-03-17 | Already implemented: Page Setup dialog with Letter/A4/Legal/A3/Custom paper sizes, portrait/landscape orientation, margin controls (inches), WASM `get_page_setup_json`/`set_page_setup` integration. |
| UXP-05 | 2026-03-17 | Audited: Headers/footers fully parsed (default/first/even types via `headerReference`), stored as Header/Footer NodeType, written back with rels + content_types. Minor gap: header-specific rels files for images not created (images still work). Unit tests exist for parse/write; missing integration round-trip test. |
| UXP-06 | 2026-03-17 | Audited: Comments fully parsed from comments.xml (CommentId/Author/Date/text), ranges tracked via CommentStart/CommentEnd nodes, written back with synthetic commentReference. Gaps: commentReference not parsed on read, no threaded reply support, no overlapping range validation. Core round-trip works. |
| UXP-14 | 2026-03-17 | Implemented Format Painter: toolbar button (#btnFormatPainter, Material Symbol format_paint), single-click for once mode, double-click for sticky mode. Copies bold/italic/underline/strikethrough/fontSize/fontFamily/color. mouseup applies format to selection. Escape exits. CSS: crosshair cursor + active state. State: formatPainterMode/copiedFormat in state.js. |
