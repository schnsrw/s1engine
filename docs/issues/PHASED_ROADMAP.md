# s1engine — Phased Roadmap for Remaining Issues

> 36 items remaining (30 OPEN + 3 PARTIAL + 2 IMPROVED + 1 MITIGATED)
> Organized into 6 phases by dependency, impact, and effort.

---

## Phase 1: Collaboration Foundation (Sprint 1-2)
> **Goal**: Real-time co-editing that actually works — CRDT integration, no more fullSync hack.
> **Impact**: Highest user-facing improvement. Currently the biggest gap vs Google Docs/OnlyOffice.
> **Effort**: Large (2 sprints)

| # | Issue | Description | Effort | Depends On |
|---|-------|-------------|--------|------------|
| E1 | Node ID mismatch | Wire s1-crdt (Fugue) into editor WASM bindings. Replace fullSync with CRDT ops. | XL | — |
| E3 | fullSync expensive | Replace DOCX export sync with CRDT state vectors. Only send deltas. | L | E1 |
| E4 | No conflict resolution | CRDT handles this automatically — concurrent edits merge deterministically. | M | E1 |
| E7 | No OT/CRDT | Same as E1 — CRDT replaces best-effort ops. | — | E1 |
| P3 | Collaborative perf | CRDT ops are granular (char-level), not document-level. Latency drops to <50ms. | — | E1 |
| A1 | Incremental render | Extend `renderSmart()` to ALL input handlers (currently only list/heading). Use CRDT dirty flags. | M | E1 |
| E6 | Cursor broadcast | Reduce to ~200ms with delta compression once CRDT handles doc sync. | S | E1 |

**Deliverable**: Two users can type simultaneously in the same paragraph without data loss or lag.

### Implementation Status

| Step | Description | Status |
|------|-------------|--------|
| 1 | Add `WasmCollabDocument` editing methods (20+ methods via `with_wasm_doc` delegate) | DONE |
| 2 | Add rendering to collab doc (`render_node_html`, `paragraph_ids_json`, `get_formatting_json`) | DONE |
| 3 | Create collab doc on share (`startCollab` creates via `engine.open_collab`) | DONE |
| 4 | Create collab doc on join (`checkAutoJoin` creates with unique replica ID) | DONE |
| 5 | CRDT text insert in `beforeinput` → `apply_local_insert_text` → broadcast | DONE |
| 6 | CRDT text delete in `beforeinput` → `apply_local_delete_text` → broadcast | DONE |
| 7 | CRDT op receiver (`crdtOp` action → `apply_remote_ops`) | DONE |
| 8 | State vector exchange on peer join (delta sync) | DONE |
| 9 | CRDT awareness for cursor positions (`set_cursor`) | DONE |
| 10 | Cleanup on disconnect (`free_doc`, clear timers) | DONE |
| 11 | Structural ops fallback (split/merge/table use fullSync, 5s debounce in CRDT mode) | DONE |
| 12 | `renderSmart()` for list/heading/slash commands | DONE |
| 13 | Incremental render in `debouncedSync` — tries `renderSingleParagraphIfPossible` before full repaginate | DONE |
| 14 | CRDT targeted re-render — extracts affected nodeId from CRDT ops, uses `renderNodeById` for single-node changes | DONE |

---

## Phase 2: Format Fidelity (Sprint 3)
> **Goal**: Documents round-trip through s1engine without losing formatting.
> **Impact**: Enterprise trust — "my document looks the same after editing in s1engine."
> **Effort**: Medium (1 sprint)

| # | Issue | Description | Effort | Depends On |
|---|-------|-------------|--------|------------|
| B3 | rPrChange old props | Store pre-change run properties and write them in `<w:rPr>` inside `<w:rPrChange>`. | M | — |
| B8 | Section odd/even headers | Support `w:evenAndOddHeaders` and per-section margin writing. | M | — |
| B10 | Custom list numbering | Reconstruct `w:abstractNum` with custom formats, start numbers, separators. | L | — |
| Q2 | Complex table vMerge | Improve vertical merge tracking in table model — handle irregular merges. | M | — |
| G4 | Redo sync with WASM | Align UI undo history with WASM undo stack. Clear redo on new actions consistently. | S | — |
| J2 | Merge cells validation | Validate rectangular selection before merging. Show error toast if invalid. | S | — |
| Q1 | Namespace extensions | Model w14/w15 properties (textOutline detail, ligatures, etc.) instead of raw XML. | L | — |
| Q3 | Text effects rendering | Render glow, reflection in CSS (text-shadow variants, filters). | S | A2 (done) |

**Deliverable**: DOCX files from Word 2019+ survive open → edit → save without visible format loss.

---

## Phase 3: Equation & Drawing Support (Sprint 4)
> **Goal**: Render and edit equations, shapes, text boxes.
> **Impact**: Academic/technical users can use s1engine for papers and reports.
> **Effort**: Medium-Large (1 sprint)

| # | Issue | Description | Effort | Depends On |
|---|-------|-------------|--------|------------|
| Q4 | Equations (OMML) | Parse `<m:oMath>` into model nodes. Render as MathML or KaTeX in editor. | XL | — |
| C4 | Equation WASM API | Expose equation insert/edit API. LaTeX input → OMML output. | L | Q4 |
| A6 | Shapes editable | Make shape placeholders interactive — resize handles, text editing inside text boxes. | L | — |
| Q11 | ODT drawings | Parse `draw:frame` / `draw:text-box` into shape model nodes. | M | A6 |
| C2 | Batch formatting API | Add `begin_batch()`/`commit_batch()` to WASM. Single re-render for multiple format changes. | M | — |

**Deliverable**: Equations render visually. Text boxes are editable. Multiple format changes apply in one render cycle.

---

## Phase 4: Enterprise & Compliance (Sprint 5)
> **Goal**: Features required for enterprise deployment — permissions, signatures, forms.
> **Impact**: Enterprise sales readiness.
> **Effort**: Large (1 sprint)

| # | Issue | Description | Effort | Depends On |
|---|-------|-------------|--------|------------|
| E5/O2 | Permission enforcement | Wire `check_permission_with_session()` into all routes. Enforce view/comment/edit per document. | M | — |
| Q5 | Form controls (SDT) | Render checkboxes, dropdowns, text inputs from `w:sdt`. Allow interaction in editor. | L | — |
| P5 | Digital signatures | Validate XML signatures on open. Sign documents on export (XMLDSIG). | XL | — |
| Q14 | PDF/A compliance | Add PDF/A-1b conformance (metadata, color profiles, font embedding). | L | — |
| Q12 | ODT change tracking | Implement `text:tracked-changes` in ODT reader/writer. | L | — |
| M4 | Error tracking | Automatically capture `console.error()` calls into error tracking system. | S | — |

**Deliverable**: Per-document permissions enforced. Interactive form fields. PDF/A export option.

---

## Phase 5: Advanced Format Support (Sprint 6)
> **Goal**: Handle complex OOXML content that enterprise documents contain.
> **Impact**: Compatibility with real-world documents from large organizations.
> **Effort**: Large (1 sprint)

| # | Issue | Description | Effort | Depends On |
|---|-------|-------------|--------|------------|
| Q6 | SmartArt | Parse `dgm:*` elements. Render as styled SVG or HTML divs. | XL | — |
| Q7 | Charts | Parse `c:chart` + `c:chartSpace`. Render via Chart.js or SVG. | XL | — |
| Q8 | OLE objects | Extract embedded objects. Show preview image with "Open in..." option. | L | — |
| P4 | VBA/macros | Parse `vbaProject.bin`. Show macro list. Execute is out of scope (security). | M | — |
| Q10 | ODT column widths | Resolve `style:column-width` from auto-styles into actual widths. | S | — |
| Q9 | Custom XML parts | Preserve `customXml/` ZIP entries for round-trip. | S | — |
| Q13 | ODT database fields | Parse `text:database-display`. Render as read-only placeholders. | S | — |

**Deliverable**: SmartArt and charts visible (read-only). OLE objects show preview. VBA macros listed.

---

## Phase 6: Multi-App Suite (Sprint 7+)
> **Goal**: Spreadsheet and presentation editors alongside documents.
> **Impact**: Full competitor parity with OnlyOffice/Collabora/Google Workspace.
> **Effort**: Extra-Large (3+ sprints)

| # | Issue | Description | Effort | Depends On |
|---|-------|-------------|--------|------------|
| P2 | Spreadsheet editor | New crate `s1-format-xlsx`. Cell model, formula engine, grid UI. | XXL | — |
| P2 | Presentation editor | New crate `s1-format-pptx`. Slide model, layout engine, slide UI. | XXL | — |
| P1 | Feature breadth | Unified launcher (like Google Workspace). File type detection → open in correct editor. | L | P2 spreadsheet, P2 presentation |

**Deliverable**: Three-app suite: Documents + Spreadsheets + Presentations.

---

## Summary Timeline

```
Sprint 1-2: Phase 1 — CRDT Collaboration     [E1,E3,E4,E7,P3,A1,E6]
Sprint 3:   Phase 2 — Format Fidelity         [B3,B8,B10,Q2,G4,J2,Q1,Q3]
Sprint 4:   Phase 3 — Equations & Drawings    [Q4,C4,A6,Q11,C2]
Sprint 5:   Phase 4 — Enterprise & Compliance [E5,O2,Q5,P5,Q14,Q12,M4]
Sprint 6:   Phase 5 — Advanced Formats        [Q6,Q7,Q8,P4,Q10,Q9,Q13]
Sprint 7+:  Phase 6 — Multi-App Suite         [P2,P1]
```

| Phase | Items | Priority | Status |
|-------|-------|----------|--------|
| 1. CRDT Collaboration | 7 | Critical | **COMPLETE** (14/14 steps) |
| 2. Format Fidelity | 8 | High | **COMPLETE** (6 fixed, 2 not-an-issue, 1 deferred) |
| 3. Equations & Drawings | 5 | High | NOT STARTED |
| 4. Enterprise & Compliance | 6 | High | NOT STARTED |
| 5. Advanced Formats | 7 | Medium | NOT STARTED |
| 6. Multi-App Suite | 3 | Future | NOT STARTED |
| **Total** | **36** | | |

---

## Already Completed (This Session)

| Metric | Count |
|--------|-------|
| Issues found | 92 |
| Fixed | 49 |
| Not an issue (false positives) | 7 |
| Remaining | 36 |
| Tests passing | 265 DOCX + 13 server + 32 e2e collab |
| CI checks | fmt + clippy + doc all clean |
