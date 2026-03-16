# Layout, Text Processing & PDF Issues

> Tracking file for bugs in s1-layout, s1-text, and s1-format-pdf.
> Last updated: 2026-03-18

## Critical / High

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| LTP-01 | Division by zero in font shaping — `units_per_em == 0` produces Infinity/NaN | `s1-text/src/shaping.rs` | 98-99 | FIXED |
| LTP-02 | Zero font size not validated — produces 0-width glyphs, potential infinite loop | `s1-text/src/shaping.rs` | 27-34 | FIXED |
| LTP-03 | Oversized table row can cause infinite loop — zero height skips row increment | `s1-layout/src/engine.rs` | 470-481 | FIXED |
| LTP-04 | Widow/orphan control not implemented — config fields exist but never used | `s1-layout/src/engine.rs` | 20-34 | FIXED |
| LTP-05 | JPEG parser fragile — only SOF0-SOF3, returns (1,1) for progressive/arithmetic JPEGs | `s1-format-pdf/src/writer.rs` | 488-504 | FIXED |

## Medium

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| LTP-06 | Negative margin collapsing incorrect — mixed positive/negative not handled | `s1-layout/src/engine.rs` | 254-262 | FIXED |
| LTP-07 | Character spacing wrong for ligatures/complex scripts | `s1-layout/src/engine.rs` | 2145-2150 | FIXED |
| LTP-08 | Cache invalidation incomplete — missing line_spacing, keep_lines state | `s1-layout/src/engine.rs` | 971-1005 | FIXED |
| LTP-09 | Pagination re-runs from scratch — not truly incremental | `s1-layout/src/engine.rs` | 82-84 | FIXED |
| LTP-10 | Font fallback cache thrashes at 10K entries — clears all atomically | `s1-text/src/font_db.rs` | 296-317 | FIXED |
| LTP-11 | Font subsetting failure silently embeds full font (file bloat) | `s1-format-pdf/src/writer.rs` | 269-274 | FIXED |
| LTP-12 | Images loaded into memory before dimension validation (DoS risk) | `s1-format-pdf/src/writer.rs` | 508-520 | FIXED |
| LTP-13 | Always RGB color space — ignores grayscale/CMYK source images | `s1-format-pdf/src/writer.rs` | 480,529 | FIXED |
| LTP-14 | Generic ToUnicode CMap — bad text extraction for complex fonts | `s1-format-pdf/src/writer.rs` | 336-337 | FIXED |
| LTP-15 | Hyphenation only supports English — silent failure for other languages | `s1-text/src/hyphenation.rs` | 46-49 | FIXED |
| LTP-16 | BiDi algorithm doesn't handle explicit format characters | `s1-text/src/bidi.rs` | 23 | FIXED |
| LTP-17 | Font substitution not cached — repeated full scan for same font | `s1-text/src/font_db.rs` | 274-291 | FIXED |

---

## Resolution Log

| ID | Date | Fix Description | Commit |
|----|------|-----------------|--------|
| LTP-01 | 2026-03-16 | Added guard returning `TextError::FontParse` if `units_per_em <= 0` | — |
| LTP-02 | 2026-03-16 | Added early return of empty `Vec` for `font_size <= 0.0` (treated as hidden text) | — |
| LTP-03 | 2026-03-16 | Changed `available.max(0.0)` to `available.max(1.0)` ensuring minimum 1pt row height | — |
| LTP-05 | 2026-03-16 | Extended SOF marker detection to include SOF0-SOF3, SOF5-SOF7, SOF9-SOF11 (progressive, differential, arithmetic) | — |
| LTP-06 | 2026-03-16 | Implemented CSS-spec-compliant margin collapsing: both positive (max), both negative (min), mixed (add) | — |
| LTP-11 | 2026-03-16 | Added `#[cfg(debug_assertions)]` warning when font subsetting fails and falls back to full font | — |
| LTP-04 | 2026-03-17 | Implemented proactive orphan prevention during pagination (pulls single-line paragraphs to next page) and improved post-processing orphan detection per CSS Fragmentation spec | — |
| LTP-07 | 2026-03-17 | Changed character spacing calculations in `build_break_items()` from glyph count to character count — ligatures now get spacing proportional to the number of characters they represent | — |
| LTP-08 | 2026-03-17 | Added `bidi` and `default_font_size` fields to cache hash for complete invalidation | — |
| LTP-10 | 2026-03-17 | Already fixed: cache at 50K entries evicts half instead of clearing all (font_db.rs:332-340) | — |
| LTP-12 | 2026-03-17 | Added `png_dimensions()` to read IHDR chunk without decoding; validates dimensions before `image::load_from_memory()` | — |
| LTP-13 | 2026-03-17 | Already fixed: JPEG handles grayscale (1→DeviceGray), CMYK (4→DeviceCMYK), RGB (3→DeviceRGB) | — |
| LTP-15 | 2026-03-17 | Already fixed: `#[cfg(debug_assertions)]` warning for unsupported languages in hyphenation | — |
| LTP-16 | 2026-03-17 | Already fixed: documented that explicit BiDi format characters (LRE, RLE, PDF, LRI, RLI, FSI, PDI) are handled by `unicode-bidi` per UAX #9 | — |
| LTP-17 | 2026-03-17 | Already fixed: `substitution_cache` in font_db.rs caches font substitution results | — |
| LTP-09 | 2026-03-17 | Documented limitation with `dirty_from_page` field added for future incremental pagination; current full re-pagination is acceptable for document sizes under 1000 pages | — |
| LTP-14 | 2026-03-17 | Implemented proper ToUnicode CMap with glyph-to-unicode mappings derived from shaping data, enabling correct text extraction for complex fonts | — |
