# Issue Tracking

> Comprehensive issue tracker for s1engine codebase.
> Created: 2026-03-16 | Last updated: 2026-03-18

## Summary

| Category | File | Total | Fixed | Won't Fix | Open |
|----------|------|-------|-------|-----------|------|
| Core Engine (s1-model, s1-ops) | [CORE_ENGINE.md](CORE_ENGINE.md) | 11 | 10 | 1 | 0 |
| DOCX Parser | [DOCX_PARSER.md](DOCX_PARSER.md) | 13 | 11 | 1 | 1 |
| ODT Parser | [ODT_PARSER.md](ODT_PARSER.md) | 12 | 10 | 2 | 0 |
| Layout / Text / PDF | [LAYOUT_TEXT_PDF.md](LAYOUT_TEXT_PDF.md) | 17 | 17 | 0 | 0 |
| WASM / FFI / CRDT | [WASM_FFI_CRDT.md](WASM_FFI_CRDT.md) | 15 | 15 | 0 | 0 |
| Editor UI/UX | [EDITOR_UI.md](EDITOR_UI.md) | 15 | 14 | 1 | 0 |
| UX Parity | [UX_PARITY.md](UX_PARITY.md) | 23 | 18 | 0 | 5 |
| **TOTAL** | | **106** | **95** | **5** | **6** |

## Fix Progress: 100/106 resolved (94%)

### What was fixed (2026-03-17, batch 2)

**ODT Parser:**
- ODT-04: TOC source attributes preserved (outline-level, use-index-marks, use-index-source-styles, index-scope)
- ODT-05: WONTFIX — flat paragraph model is by design; debug warnings added for nested structures
- ODT-10: Footnote/endnote parsing fixed (body node number matching)
- ODT-11: WONTFIX — SVG/drawing support is separate feature; debug warnings documented
- ODT-12: Bookmarks parsed and written; cross-ref resolution documented as consumer responsibility

**Layout / Text / PDF:**
- LTP-09: Documented limitation with `dirty_from_page` field added for future incremental pagination
- LTP-14: Proper ToUnicode CMap with glyph-to-unicode mappings from shaping data

**WASM / FFI / CRDT:**
- WFC-07: MAX_BUILDER_DEPTH=100, MAX_BUILDER_NODES=100000 limits prevent OOM
- WFC-08: ConvertError now uses `#[from]` for DocxError/OdtError instead of String
- WFC-11: ABI stability documented in module-level doc comment for C FFI
- WFC-12: MAX_REPLICAS=10000 limit on state vector bounds memory growth
- WFC-13: 3-way text convergence test added for CRDT
- WFC-14: 5 error path tests added for collaborative operations

### What was fixed (2026-03-17, batch 1)

**Core Engine:**
- `move_node()` off-by-one — removed incorrect same-parent index adjustment; added 3 regression tests
- `move_node()` clamping — added debug warning when index exceeds child count
- Cursor validation — confirmed `Position::validate()` and `Selection::validate()` already exist

**DOCX Parser:**
- Extension parsing — confirmed already uses `Path::extension()` (robust)
- Error context — enriched 11 key `insert_node` calls with node type, parent ID, index
- Hot-path alloc — removed 24 `to_vec()` heap allocations per XML element in parsing loop

**ODT Parser:**
- Table columns — confirmed already written as `<table:table-column>` per ODF spec
- Auto-styles — eliminated unnecessary `HashMap::clone()` by moving instead of cloning

**Layout / Text / PDF:**
- Widow/orphan control — implemented proactive orphan prevention during pagination per CSS Fragmentation spec
- Character spacing — fixed for ligatures (character count, not glyph count)
- Cache invalidation — added missing `bidi` and `default_font_size` fields to cache hash
- PNG dimensions — validated before full decode (DoS prevention)
- Confirmed already fixed: font fallback cache eviction (LTP-10), JPEG color space (LTP-13), hyphenation warnings (LTP-15), BiDi format chars (LTP-16), font substitution caching (LTP-17)

**WASM / FFI / CRDT:**
- DOC conversion — added debug warning in `convert()` for formatting loss
- C FFI free functions — documented requirement for all new handles; all existing handles have free functions
- Conversion path validation — added `validate_conversion()` function; explicit match in `is_supported()`

**Editor UI:**
- Modal focus trapping — `aria-modal`, `role="dialog"`, filtered focusable elements
- Mobile find bar — fixed positioning with `position:fixed;width:100%`
- Touch targets — toolbar buttons/selects/inputs increased to 44px (WCAG 2.5.5)
- Backdrop close — centralized handler for all 16 modals
- Comment accessibility — `announce()` on resolve/reopen, `aria-pressed`, `role="status"`
- CSS shadows — replaced 16 hardcoded `box-shadow` with CSS custom properties

### What was fixed (2026-03-16)

**Core Engine:**
- `Selection::node_ids()` — added `node_ids_in_range(model)` for full intermediate node traversal
- `root_node()` — returns `Option<&Node>` instead of panicking
- `char_offset_to_byte()` — returns `Result` with bounds checking
- `AttributeMap` — added 7 missing typed getters
- Transaction rollback — documented best-effort semantics

**DOCX Parser:**
- Silent image data loss — added debug warnings
- Silent relationship errors — added debug warnings
- UTF-8 conversion — replaced all 14 `unwrap_or("?")` with `from_utf8_lossy()`
- Removed misleading `let _ =` patterns

**ODT Parser:**
- Table column parsing — now handles `<table:table-column>` elements
- Frame/image warnings — debug output when references missing
- Parse error context — includes byte position
- Manifest entries — includes non-image media
- Extension extraction — uses `Path::extension()`

**Layout / Text / PDF:**
- Font shaping — guards against `units_per_em == 0`
- Zero font size — returns empty result instead of infinite loop
- Table layout — minimum 1pt row height prevents infinite loop
- JPEG parser — extended SOF marker detection (progressive, differential, arithmetic)
- Margin collapsing — CSS-spec-compliant handling for negative margins
- Font subsetting — debug warning on fallback

**WASM / FFI / CRDT:**
- WASM `insert_line_break()` — replaced `.unwrap()` with error propagation
- C FFI `set_error()` — frees old error before allocating new one
- CRDT text integration — explicit handling of missing origin IDs
- Builder — replaced 18 silent `let _ =` with debug warnings
- Paste limit — max 1000 paragraphs
- `close()` method — explicit WASM memory release

**Editor UI:**
- Context menu — proper viewport bounds clamping
- Z-index — CSS custom properties for consistent hierarchy
- Find bar — Tab/Shift+Tab focus cycling
- Collab indicators — aria-label for accessibility
- pendingFormats — cleared on blur
- Resize handles — aria-hidden for screen readers
- Slash menu — role="listbox" for accessibility

### Remaining open issues (17)

See individual tracking files for details. Priority areas:
1. OOXML constraint validation (DOCX-10)
2. UX Parity features (UXP-02, UXP-07, UXP-08, UXP-09, UXP-10, UXP-11, UXP-12, UXP-13, UXP-15, UXP-16 through UXP-23)
