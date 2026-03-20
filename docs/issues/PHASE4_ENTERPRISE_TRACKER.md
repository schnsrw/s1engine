# Phase 4 — Enterprise & Compliance Tracker

> Items requiring crypto, compliance specs, or interactive UI components.
> These are multi-week items, not quick fixes.

## E5/O2: Per-Document Permission Enforcement

**Status**: PARTIAL — `check_permission_with_session()` implemented but not wired into routes.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Wire auth middleware into API routes via Axum layer | S | TODO with pattern added |
| 2 | Read `access` URL param in WebSocket handler (view/comment/edit) | S | NOT STARTED |
| 3 | Enforce edit permission on write endpoints (upload, close, save) | M | NOT STARTED |
| 4 | Enforce view permission on read endpoints (download, info) | S | NOT STARTED |
| 5 | Add permission level to WebSocket welcome message | S | NOT STARTED |
| 6 | Editor UI: disable toolbar in view/comment mode | M | NOT STARTED |
| 7 | Test: viewer can't edit, commenter can comment but not edit | M | NOT STARTED |

## Q5: Interactive Form Controls (SDT)

**Status**: DEFERRED — SDT elements parsed; TOC SDTs rendered. Interactive forms need UI.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Parse `w:sdt` with `w14:checkbox` into model as FormCheckbox node | M | NOT STARTED |
| 2 | Parse `w:sdt` with `w:dropDownList` into model as FormDropdown node | M | NOT STARTED |
| 3 | Parse `w:sdt` with `w:text` into model as FormTextInput node | M | NOT STARTED |
| 4 | Render checkboxes as `<input type="checkbox">` in HTML | S | NOT STARTED |
| 5 | Render dropdowns as `<select>` in HTML | S | NOT STARTED |
| 6 | Render text inputs as `<input type="text">` in HTML | S | NOT STARTED |
| 7 | Write form state back to SDT elements on export | L | NOT STARTED |
| 8 | Round-trip tests for each form type | M | NOT STARTED |

## P5: Digital Signatures

**Status**: DEFERRED — requires XMLDSIG crypto library.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Add `ring` or `openssl` dependency for crypto operations | S | NOT STARTED |
| 2 | Parse `_xmlsignatures/` ZIP entries on document open | M | NOT STARTED |
| 3 | Validate XML signatures (X.509 certificate chain verification) | XL | NOT STARTED |
| 4 | Show signature status in editor UI (valid/invalid/unsigned) | M | NOT STARTED |
| 5 | Sign document on export (create `_xmlsignatures/` entry) | XL | NOT STARTED |
| 6 | Certificate management UI (upload, select signing cert) | L | NOT STARTED |

## Q14: PDF/A Compliance

**Status**: DEFERRED — requires PDF spec changes.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Add PDF/A-1b metadata (`pdfaid:part`, `pdfaid:conformance`) via XMP | M | NOT STARTED |
| 2 | Embed ICC color profile (sRGB) for color management | M | NOT STARTED |
| 3 | Ensure all fonts fully embedded (no subsetting for PDF/A) | L | NOT STARTED |
| 4 | Mark document as tagged PDF (`MarkInfo` dictionary) | S | NOT STARTED |
| 5 | Add `OutputIntent` dictionary with ICC profile reference | M | NOT STARTED |
| 6 | Validation: test with veraPDF or similar PDF/A validator | M | NOT STARTED |

## Q12: ODT Change Tracking

**Status**: Raw XML preservation approach implemented.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Preserve `text:tracked-changes` as raw XML on body node | S | IN PROGRESS |
| 2 | Parse `text:changed-region` into model attributes | L | NOT STARTED |
| 3 | Map to s1-model RevisionType/Author/Date attributes | M | NOT STARTED |
| 4 | Render change tracking markers in editor HTML | M | NOT STARTED |
| 5 | Write change tracking back to ODT on export | L | NOT STARTED |
| 6 | Accept/reject changes in editor UI | M | NOT STARTED |

## M4: Error Tracking

**Status**: Automatic console.error capture added.

| Step | Description | Effort | Status |
|------|-------------|--------|--------|
| 1 | Capture `console.error` automatically | S | DONE |
| 2 | Send errors to server endpoint for aggregation | M | NOT STARTED |
| 3 | Admin dashboard: error log viewer | L | NOT STARTED |
| 4 | Rate limiting to prevent error flood | S | NOT STARTED |
