# Final Comprehensive Scan — All Issues

> Full codebase scan comparing against Google Docs, Collabora Online, OnlyOffice.
> Created: 2026-03-18 | Last updated: 2026-03-18
> Supersedes previous trackers. Only NEW/UNFIXED issues listed.

## Critical (Blocks production use)

| ID | Issue | Area | File | Status |
|----|-------|------|------|--------|
| FS-01 | 30+ `panic!()` calls in ODF/DOCX parsers violate no-panics rule | Rust | property_parser, section_parser, content_parser, content_writer, style_parser, font.rs | OK — all in test code |
| FS-02 | Missing Ctrl+Shift+V (paste without formatting) | Editor | input.js | FIXED |
| FS-03 | Missing Ctrl+K (insert link shortcut) | Editor | input.js | FIXED |
| FS-04 | Missing Ctrl+Shift+X (strikethrough shortcut) | Editor | input.js | FIXED |
| FS-05 | Missing Ctrl+. / Ctrl+, (superscript/subscript shortcuts) | Editor | input.js | FIXED |
| FS-06 | No arrow-key navigation in dropdown menus — accessibility violation | Editor | toolbar-handlers.js | FIXED |
| FS-07 | RTL text has no `dir="rtl"` attribute set — renders LTR | WASM+Editor | lib.rs, render.js | FIXED |
| FS-08 | No regex support in Find & Replace | Editor | find.js | FIXED |

## High

| ID | Issue | Area | File | Status |
|----|-------|------|------|--------|
| FS-09 | Missing Ctrl+Shift+7/8 (numbered/bullet list shortcuts) | Editor | input.js | FIXED |
| FS-10 | No image crop tool | Editor | images.js | FIXED |
| FS-11 | No read-only/viewer mode | Editor | state.js, render.js | FIXED |
| FS-12 | Toast notifications missing aria-live for screen readers | Editor | toolbar-handlers.js | FIXED |
| FS-13 | No skip-to-main-content link for accessibility | Editor | index.html | FIXED |
| FS-14 | Find & Replace missing: replace preview, find-in-selection | Editor | find.js | FIXED |
| FS-15 | No document statistics API (word count by selection, char count) | WASM | lib.rs | FIXED |
| FS-16 | No batch operation / transaction grouping WASM API | WASM | lib.rs | FIXED |
| FS-17 | Hardcoded colors in header/footer templates — breaks dark mode | Editor | toolbar-handlers.js | FIXED |
| FS-18 | Floating toolbar hardcoded 420px width — breaks on mobile | Editor | toolbar-handlers.js | FIXED |
| FS-19 | No style resolution cache in engine — repeated chain walks | Rust | tree.rs | FIXED |

## Medium

| ID | Issue | Area | File | Status |
|----|-------|------|------|--------|
| FS-20 | Image caption support missing | Editor | images.js | FIXED |
| FS-21 | Font color picker is basic HTML input — no palette/swatches | Editor | toolbar-handlers.js | FIXED |
| FS-22 | Highlight color only yellow — no color options | Editor | toolbar.js | FIXED |
| FS-23 | Dropdown positioning doesn't account for menu height at edges | Editor | toolbar-handlers.js | FIXED |
| FS-24 | No smart quotes auto-replacement | Editor | input.js | FIXED |
| FS-25 | Page breaks in pasted content dropped silently | Editor | input.js | FIXED |
| FS-26 | Footnotes/endnotes dropped on paste from external sources | Editor | input.js | FIXED |
| FS-27 | Node lookup in children uses Vec scan O(n) — slow for wide trees | Rust | tree.rs | WONTFIX |
| FS-28 | No streaming parser for large files — entire doc in memory | Rust | reader.rs | FIXED |
| FS-29 | Layout always full re-layout — no incremental dirty-page support | Rust | engine.rs | FIXED |
| FS-30 | String concatenation in XML writers — not streaming | Rust | writer.rs, content_writer.rs | FIXED |
| FS-31 | SmartArt, charts, form controls not parsed from DOCX | Rust | content_parser.rs | FIXED |
| FS-32 | Watermarks, page borders, line numbering not parsed | Rust | content_parser.rs | FIXED |
| FS-33 | Text effects (shadow, glow, outline) not modeled | Rust | attributes.rs | FIXED |
| FS-34 | ODT column widths not stored (TODO in parser) | Rust | content_parser.rs:1267 | FIXED |
| FS-35 | Peer cursor disappears after 5s even if peer still connected | Editor | collab.js | FIXED |

## Low

| ID | Issue | Area | File | Status |
|----|-------|------|------|--------|
| FS-36 | No auto-capitalize at sentence start | Editor | input.js | FIXED |
| FS-37 | Batch undo limited to typing — formatting/deletion are separate steps | Editor | input.js | FIXED |
| FS-38 | No word/line/paragraph select (triple-click) | Editor | input.js | FIXED |
| FS-39 | No multi-cursor/multi-selection support | Editor | selection.js | FIXED |
| FS-40 | Toolbar unresponsive below 480px — buttons stack | Editor | styles.css | FIXED |
| FS-41 | Find match count not announced to screen readers | Editor | find.js | FIXED |
| FS-42 | No image transparency/filter controls | Editor | images.js | FIXED |
| FS-43 | No Special Characters insertion dialog | Editor | toolbar-handlers.js | FIXED |
| FS-44 | No Format Borders & Shading dialog | Editor | toolbar-handlers.js | FIXED |

---

## Resolution Log

| ID | Date | Fix Description |
|----|------|-----------------|
