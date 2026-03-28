# Improvement Plan

Prioritized, phased plan to bring s1engine format fidelity from ~50% to ~90%+ of OOXML/ODF spec coverage.

**Audit date:** 2026-03-29
**Current state (after Phase 1-3):** ~72% DOCX coverage, ~58% ODT coverage, 1,660+ tests passing

## Phase Completion Status

| Phase | Status | Date |
|-------|--------|------|
| Phase 1: Model Completeness | **COMPLETE** | 2026-03-29 |
| Phase 2: DOCX Fidelity | **COMPLETE** | 2026-03-29 |
| Phase 3: ODT Fidelity | **COMPLETE** | 2026-03-29 |
| Phase 4: Advanced Features | PENDING | — |
| Phase 5: Code Health | PENDING | — |

---

## Guiding Principles

1. **Model first** — If it's not in `s1-model`, no format crate can preserve it
2. **One feature at a time** — Don't rush, test thoroughly
3. **Round-trip is king** — Every feature must survive read -> edit -> write
4. **Spec-first** — Write spec before code (per CLAUDE.md process)
5. **Test with real documents** — Not just synthetic fixtures

---

## Phase 1: Model Completeness (Foundation)

**Goal:** Add missing AttributeKey/Value variants so format crates can store what they parse.

**Duration estimate:** Focus work

### 1.1 Paragraph/Page Layout Attributes

| Attribute | Key | Value Type | Needed By |
|-----------|-----|-----------|-----------|
| Widow control | `WidowControl` | `Bool` | DOCX, ODT |
| Orphan control | `OrphanControl` | `Bool` | ODT |
| Outline level | `OutlineLevel` | `Int(0-9)` | DOCX |
| Page borders | `PageBorders` | `Borders` | DOCX |
| Writing mode | `WritingMode` | new enum `WritingMode` | DOCX, ODT |

### 1.2 Text Formatting Attributes

| Attribute | Key | Value Type | Needed By |
|-----------|-----|-----------|-----------|
| All caps | `Caps` | `Bool` | DOCX |
| Small caps | `SmallCaps` | `Bool` | DOCX |
| Double strikethrough | `DoubleStrikethrough` | `Bool` | DOCX |
| Hidden text | `Hidden` | `Bool` | DOCX |
| Text transform | `TextTransform` | new enum | ODT |
| Baseline position | `BaselineShift` | `Float` (half-pts) | DOCX |
| Complex-script size | `FontSizeCS` | `Float` | DOCX |
| East Asian font | `FontFamilyEastAsia` | `String` | DOCX |
| Complex-script font | `FontFamilyCS` | `String` | DOCX |

### 1.3 Table Attributes

| Attribute | Key | Value Type | Needed By |
|-----------|-----|-----------|-----------|
| Row height | `RowHeight` | `Float` | DOCX, ODT |
| Row can split | `RowCanSplit` | `Bool` | DOCX |
| Table layout | `TableLayoutMode` | new enum (Fixed/Auto) | DOCX |
| Default cell margins | `TableCellMargins` | new struct `Margins` | DOCX |
| Per-cell margins | `CellMarginTop/Bottom/Left/Right` | `Float` | DOCX |
| Cell no-wrap | `CellNoWrap` | `Bool` | DOCX |
| Cell text direction | `CellTextDirection` | `WritingMode` | DOCX |
| Table indent | `TableIndent` | `Float` | DOCX |

### 1.4 Tests for Phase 1

- Unit tests for every new AttributeKey/Value variant
- Round-trip tests: create model with new attributes -> write DOCX -> read back -> compare
- Property tests for new enum types

---

## Phase 2: DOCX Fidelity (High-Impact Features)

**Goal:** Raise DOCX coverage from ~57% to ~75%.

### 2.1 Run Properties

| Feature | OOXML Element | Priority |
|---------|--------------|----------|
| All caps | `w:caps` | HIGH |
| Small caps | `w:smallCaps` | HIGH |
| Hidden text | `w:vanish` | MEDIUM |
| Double strikethrough | `w:dstrike` | LOW |
| Complex-script size | `w:szCs` | HIGH (BiDi) |
| East Asian font | `w:rFonts/@eastAsia` | HIGH (CJK) |
| Baseline position | `w:position` | MEDIUM |
| Kerning threshold | `w:kern` | LOW |

### 2.2 Paragraph Properties

| Feature | OOXML Element | Priority |
|---------|--------------|----------|
| Widow/orphan control | `w:widowControl` | HIGH |
| Outline level | `w:outlineLvl` | HIGH |
| Text direction | `w:textDirection` | MEDIUM |
| Mirror indents | `w:mirrorIndents` | LOW |

### 2.3 Table Properties

| Feature | OOXML Element | Priority |
|---------|--------------|----------|
| Row height | `w:trHeight` | HIGH |
| Table cell margins | `w:tblCellMar` | HIGH |
| Table layout mode | `w:tblLayout` | HIGH |
| Per-cell margins | `w:tcMar` | MEDIUM |
| Row no-split | `w:cantSplit` | MEDIUM |
| Table indent | `w:tblInd` | LOW |

### 2.4 Section Properties

| Feature | OOXML Element | Priority |
|---------|--------------|----------|
| Page borders | `w:pgBorders` | MEDIUM |
| Document grid | `w:docGrid` | MEDIUM (CJK) |
| Line numbering | `w:lnNumType` | LOW |

### 2.5 Fields (Critical Gap)

| Feature | Priority | Notes |
|---------|----------|-------|
| Parse field instructions | HIGH | Store raw instruction string |
| Cache field results | HIGH | Display value for unsupported fields |
| TOC field | HIGH | Generate from headings |
| HYPERLINK field | HIGH | Alternative to `w:hyperlink` |
| REF/PAGEREF | MEDIUM | Cross-references |
| DATE/TIME | MEDIUM | Common fields |
| SEQ | LOW | Figure/table numbering |

### 2.6 Tests for Phase 2

- Create test DOCX files with each feature using Microsoft Word
- Round-trip test: read -> write -> read -> binary compare
- Fidelity test: read -> check all attributes populated correctly
- Regression: ensure existing tests still pass

---

## Phase 3: ODT Fidelity (Parity with DOCX)

**Goal:** Raise ODT coverage from ~43% to ~70%.

### 3.1 Critical Fixes

| Feature | ODF Element | Priority |
|---------|------------|----------|
| Ordered lists | `text:list-level-style-number` | HIGH |
| List style definitions | `text:list-style` | HIGH |
| Default styles | `style:default-style` | HIGH |
| Cell borders | `fo:border-*` in cells | HIGH |
| Text transform | `fo:text-transform` | MEDIUM |
| Widow/orphan | `fo:widows`, `fo:orphans` | MEDIUM |

### 3.2 Image/Frame Improvements

| Feature | ODF Element | Priority |
|---------|------------|----------|
| Anchor type | `text:anchor-type` | HIGH |
| Text wrapping | `style:wrap` | HIGH |
| Image positioning | Absolute/relative | MEDIUM |

### 3.3 Fields

| Feature | ODF Element | Priority |
|---------|------------|----------|
| Date field | `text:date` | MEDIUM |
| Time field | `text:time` | MEDIUM |
| Author field | `text:author-name` | LOW |
| File name | `text:file-name` | LOW |

### 3.4 Change Tracking

| Feature | Priority | Notes |
|---------|----------|-------|
| Parse `text:insertion` | HIGH | Currently raw XML only |
| Parse `text:deletion` | HIGH | Currently raw XML only |
| Parse `text:format-change` | MEDIUM | |
| Insert/delete markers | MEDIUM | `text:change-start/end` |

### 3.5 Tests for Phase 3

- Create test ODT files with LibreOffice
- Round-trip tests for each new feature
- Cross-format test: DOCX -> ODT -> DOCX -> compare

---

## Phase 4: Advanced Features

**Goal:** Reach ~85%+ coverage for production-grade use.

### 4.1 Number Format Expansion

Add remaining common formats to `ListFormat`:
- `none` (no marker)
- `ideographDigital`, `japaneseCounting` (CJK)
- `arabicAlpha`, `arabicAbjad` (Arabic)
- `hebrew1`, `hebrew2` (Hebrew)
- `hindiNumbers`, `thaiNumbers` (Indic)
- Custom bullet characters

### 4.2 Content Controls (SDT)

| Feature | Priority |
|---------|----------|
| SDT data binding | MEDIUM |
| Date picker | MEDIUM |
| Combo box | MEDIUM |
| SDT tag/alias | LOW |

### 4.3 Themes

| Feature | Priority |
|---------|----------|
| Theme color model | MEDIUM |
| Theme font model | MEDIUM |
| Color scheme resolution | MEDIUM |

### 4.4 Drawing/Shapes

| Feature | Priority |
|---------|----------|
| Basic shapes (rect, oval, line) | LOW |
| Text boxes | MEDIUM |
| Group shapes | LOW |

### 4.5 Math (OMML/MathML)

| Feature | Priority |
|---------|----------|
| Basic equation parsing | LOW |
| Fraction, root, matrix | LOW |
| OMML <-> MathML conversion | LOW |

---

## Phase 5: Code Health & Performance

### 5.1 Code Splitting

| File | Current | Target |
|------|---------|--------|
| `s1-layout/engine.rs` | 7,354 lines | Split into: pagination.rs, table_layout.rs, block_layout.rs, style_apply.rs |
| `s1-text/font_db.rs` | 20,792 lines | Split into: discovery.rs, substitution.rs, cache.rs, fallback.rs |
| `s1-crdt/resolver.rs` | 20,867 lines | Split into: text_resolve.rs, tree_resolve.rs, attr_resolve.rs, sync.rs |

### 5.2 Safety

| Issue | Fix |
|-------|-----|
| Style cache interior mutability | Replace with `RefCell<HashMap>` or `OnceCell` |
| 19 panics across test code | Convert to `assert!` / `Result` patterns |
| Transaction rollback silent failure | Add rollback error reporting |

### 5.3 CRDT Health

| Issue | Fix |
|-------|-----|
| Tombstone GC never called | Implement periodic GC based on state vector |
| Op log unbounded growth | Add log compaction / snapshot strategy |
| No operation serialization | Add serde feature flag to s1-ops |

### 5.4 Layout Performance

| Issue | Fix |
|-------|-----|
| Pagination not truly incremental | Use `dirty_from_page` to skip unchanged prefix |
| Full style resolution on every layout | Persistent style resolution cache across layouts |

---

## XLSX Decision

**Recommendation:** Separate `s1-format-xlsx` from the main engine.

**Rationale:**
- XLSX is a spreadsheet format — fundamentally different data model from documents
- It's NOT integrated into s1engine facade (only used in WASM FFI directly)
- Zero coupling with core crates (s1-model, s1-ops, s1-layout)
- Keeping it adds confusion about project scope

**Action:**
- Move `s1-format-xlsx` to a separate repository or feature-gate it
- Remove from default workspace members
- Clean up WASM FFI to feature-gate spreadsheet functions
- Keep `FileType::Xlsx` detection in s1-convert for user-facing error messages

---

## Success Metrics

| Metric | Current | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|--------|---------|---------|---------|---------|---------|
| DOCX feature coverage | ~57% | ~57% | ~75% | ~75% | ~85% |
| ODT feature coverage | ~43% | ~43% | ~43% | ~70% | ~80% |
| Round-trip tests | basic | +model | +docx | +odt | +advanced |
| AttributeKey variants | ~80 | ~95 | ~95 | ~95 | ~100+ |
| Real-world doc tests | few | few | 10+ | 20+ | 50+ |

---

## Testing Strategy

### For Every New Feature:

1. **Spec**: Write feature spec in `docs/specs/`
2. **Unit test**: Test attribute parsing in isolation
3. **Round-trip test**: Read DOCX/ODT -> write -> read -> compare attributes
4. **Cross-format test**: DOCX -> ODT -> DOCX (or vice versa)
5. **Real-world test**: Use a document created in Word/LibreOffice, verify parsing
6. **Fidelity test**: Open original and round-tripped in Word/LibreOffice, visual compare

### Test Fixture Collection:

Build a library of test documents covering:
- Simple formatting (bold, italic, underline)
- Complex tables (merged cells, nested, borders)
- Lists (multi-level, mixed bullet/number)
- Headers/footers with fields
- Track changes (insert, delete, format)
- Images (inline, floating, various wraps)
- Sections (different page sizes, orientations)
- CJK content (Chinese, Japanese, Korean)
- BiDi content (Arabic, Hebrew)
- Complex fields (TOC, cross-references)

Store in `tests/fixtures/fidelity/` organized by format and feature.
