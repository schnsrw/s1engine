# Repository Remarks and Priorities

## Status

All P0, P1, and P2 items from previous reviews have been addressed. The remaining items are P3 strategic maturity concerns that are not blockers.

---

## Closed Items

### P0: Core Correctness (all resolved)

- Subtree undo: fixed with `restore_node()`, regression tests added
- Mixed attribute undo: fixed with remove-then-merge inverse, regression tests added
- Unicode text offsets: `char_offset_to_byte()` helper, character-aware ops
- Cycle detection: `is_descendant()` guard in `move_node()`

### P1: Public Contract (all resolved)

- `docs/SPECIFICATION.md` sections 10-12 rewritten to match actual API
- `docs/ROADMAP.md` stale code snippets fixed
- `docs/API_DESIGN.md` transaction example fixed (`Transaction::with_label()`)
- `paragraph_ids()` / `paragraph_count()` documented as top-level body only
- `doc-legacy` feature now wired into facade (`Engine::open()` handles `Format::Doc`)
- `Format::Doc` variant added with OLE2 magic detection, extension, MIME type
- README import mismatch fixed (line 62: added `Format` to import)
- Markdown doc examples validated via `crates/s1engine/tests/doc_examples.rs` (7 tests)

### P2: Hardening and Maintenance (all resolved)

- Benchmark baselines published in CHANGELOG.md
- Unused `Transaction` import warning in bench file fixed
- `cargo-fuzz` targets added (6 fuzz targets in `fuzz/fuzz_targets/`)
- Hostile input tests exist (19 tests in `hostile_inputs.rs`)
- Contract tests added (19 tests in `invariants.rs`)
- Support matrix added to README (4 tables: general, block, inline, document-level)
- Hard-coded test counts removed from README

---

## Remaining P3: Strategic Maturity

These are not blockers. They are long-term quality and positioning improvements.

### 1. Real-world fidelity corpus

- Current fidelity evidence comes from synthetic round-trip tests
- No golden comparison suite against Word or LibreOffice output
- A curated corpus of real documents would strengthen fidelity claims

### 2. Facade scope positioning

- The facade is intentionally thin -- core power is in subcrates
- Docs and project ambition sometimes suggest a richer SDK
- Choose and document one posture explicitly

### 3. Support matrix confidence

- The support matrix exists and is accurate
- Over time, tie claims to named tests and fixture sets
- Separate content fidelity claims from layout/render fidelity in docs

---

## Appraiser View

- Architecture: `8.5/10`
- Implementation breadth: `8.5/10`
- Test breadth: `9.5/10`
- Fidelity confidence: `7.5/10`
- API maturity: `7.5/10`
- Documentation accuracy: `8/10`
- Release readiness: `7.5/10`
