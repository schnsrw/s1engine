# DOCY Validation Plan

This document defines the minimum validation required before the DOCY open path is treated as reliable.

## Current Status

The DOCY path now opens real documents in OnlyOffice instead of falling back to the default single paragraph state.

Current evidence:
- `OpenDocumentFromBin()` loads real content for at least one real DOCX
- serializer-side structural tests now validate top-level `Document` records and the first paragraph/run containers

This is progress, not closure.

## Acceptance Gates

DOCY can be considered acceptable for default open only when all of the following pass:

1. Structural sanity
- DOCY wrapper is valid: `DOCY;v5;{size};{base64}`
- main table contains `Signature`, `Settings`, `Style`, `Document`, and `Other`
- `Document` table walks cleanly with no length overruns
- first paragraph `pPr` and first run `rPr` / `Content` walk cleanly as `Read2` / `Read1` blocks

2. Real content reconstruction
- OnlyOffice loads more than one element for representative DOCX fixtures
- first visible paragraphs match source ordering
- heading/body structure appears in the editor rather than a default empty shell

3. High-risk feature coverage
- line breaks
- page breaks
- tabs
- headings and paragraph styles
- bold / italic / underline
- multi-paragraph documents
- tables
- images

4. Regression discipline
- serializer tests run in CI
- at least one browser-side DOCY smoke validation exists
- DOCY stays behind a kill-switch until table/image fidelity is proven

## Priority Matrix

### P0

- Multi-paragraph text-only DOCX opens with correct paragraph count
- Formatted heading/body DOCX opens with visible headings
- No fallback to 1-element default document

### P1

- Line breaks survive open
- Page breaks survive open
- Tabs survive open
- Bold and italic render correctly

### P2

- Tables open without collapsing the document
- Basic cell text order is correct
- Merged cells do not corrupt the rest of the body

### P3

- Images open without parser failure
- Comments / bookmarks / note references are restored with valid ID mapping
- Header/footer linkage is validated

## Automated Tests Required

### Serializer-side

- `crates/s1-format-docy/tests/structural_validation.rs`
  - validates `Read1` / `Read2` walking on generated DOCY bytes
  - validates real DOCX fixtures, not only synthetic txt input

### Engine-side

- extend `crates/s1engine/tests/fidelity_validation.rs` with fixtures for:
  - breaks
  - tabs
  - tables
  - images

### Browser-side

- add a DOCY smoke harness that asserts:
  - `logicDoc.Content.length > 1`
  - visible text prefix matches expected fixture text
  - page count is non-zero

Recommended first browser targets:
- `text-only.docx`
- `formatted.docx`
- `complex.docx`
- `testdocs/docx/samples/calibre_demo.docx`

Current automated browser gate:
- [scripts/docy_browser_smoke.js](/Users/sachin/Desktop/melp/rdrive/doc-engine/scripts/docy_browser_smoke.js)
  - serves the repo root locally
  - opens [web/index.html](/Users/sachin/Desktop/melp/rdrive/doc-engine/web/index.html)
  - loads `complex.docx`
  - loads `testdocs/docx/samples/calibre_demo.docx`
  - asserts real element count, non-zero page count, and expected text prefix

Current browser result:
- `complex.docx` still collapses to `1` loaded element in the real browser path
- `calibre_demo.docx` still collapses to `1` loaded element in the real browser path
- DOCY is therefore not yet acceptable as the default open path for representative DOCX fixtures

## Open Risks

- table semantics are not fully validated against OnlyOffice behavior
- image/drawing serialization is still effectively unsupported
- bookmark/comment/note marker serialization is intentionally disabled pending stable ID mapping
- open success does not yet prove round-trip correctness

## Immediate Next Steps

1. Investigate why representative browser fixtures still collapse to the default `1`-element document after `OpenDocumentFromBin()`.
2. Keep DOCY enabled for open in local development only.
3. Add dedicated fixtures for:
   - line break
   - page break
   - tab
   - table
4. Do not treat save/export through DOCY as ready until images and tables are validated.
