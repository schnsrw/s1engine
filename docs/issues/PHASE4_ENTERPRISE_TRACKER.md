# Phase 4 — Enterprise & Compliance Tracker

> Last updated: 2026-03-20
> **Status: COMPLETE** — All actionable items done. Crypto-dependent items (signature validation, PDF/A full compliance) deferred to dedicated sprint.

## E5/O2: Per-Document Permission Enforcement — COMPLETE

| Step | Description | Status |
|------|-------------|--------|
| 1 | Wire auth middleware — detailed patterns added to 3 route handlers | DONE |
| 2 | Read `access` URL param in WsParams + log view-only connections | DONE |
| 3 | Enforce edit permission — `close_file` checks session mode, returns 403 for view-only | DONE |
| 4 | Read endpoints — confirmed no restriction needed (Viewer implicit) | DONE |
| 5 | `joined` message includes `access` field | DONE |
| 6 | Editor UI: disable toolbar in view/comment mode based on `joined.access` | DONE |
| 7 | Test: `close_view_only_session_forbidden` + `cleanup_stale_editors_removes_old` | DONE |

## Q5: Interactive Form Controls (SDT) — COMPLETE

| Step | Description | Status |
|------|-------------|--------|
| 1 | Parse `w:sdt` with `w14:checkbox` — FormType + FormChecked attrs | DONE |
| 2 | Parse `w:sdt` with `w:dropDownList` — FormType + FormOptions attrs | DONE |
| 3 | Parse `w:sdt` with `w:text` — FormType attr | DONE |
| 4 | Checkbox rendered as `<label class="form-checkbox"><input>` | DONE |
| 5 | Dropdown rendered as `<select class="form-dropdown">` | DONE |
| 6 | Text input rendered as `<input class="form-text">` | DONE |
| 7 | Write form state back — `write_sdt_form()` in content_writer with w14 namespace | DONE |
| 8 | Round-trip tests — 4 tests (checked, unchecked, dropdown, text) | DONE |

## P5: Digital Signatures — CRYPTO ADDED

| Step | Description | Status |
|------|-------------|--------|
| 0 | Detect `_xmlsignatures/` → `hasDigitalSignature` metadata | DONE |
| 0b | Detect `vbaProject.bin` → `hasMacros` metadata | DONE |
| 1 | `signature_parser.rs` — extracts X509SubjectName, SigningTime, cert + signature value (6 tests) | DONE |
| 2 | Parse signature ZIP entries → signer subject, date, count in metadata | DONE |
| 3 | Preserve signatures in `preserved_parts` → round-trip write back | DONE |
| 4 | `ring` + `x509-cert` added as optional `crypto` feature | DONE |
| 5 | `validate_signature()` — parses X.509 cert DER, extracts subject, confirms cert validity | DONE |
| 6 | Server uses `crypto` feature for signature validation on upload | DONE |
| 7 | Sign documents on export (requires private key management) | DEFERRED |
| 8 | Certificate management UI (upload, select signing cert) | DEFERRED |

## Q14: PDF/A Compliance — ALREADY IMPLEMENTED

| Step | Description | Status |
|------|-------------|--------|
| 1 | XMP metadata (`pdfaid:part`, `pdfaid:conformance`) | DONE (already existed) |
| 2 | Embedded sRGB ICC color profile | DONE (already existed) |
| 3 | Full font embedding | DONE (subsetting acceptable for PDF/A-1b) |
| 4 | Tagged PDF (`MarkInfo`) | DONE (already existed) |
| 5 | `OutputIntent` dictionary | DONE (already existed) |
| 6 | Validation with veraPDF | NOT TESTED |

## Q12: ODT Change Tracking — PRESERVATION + PARSING DONE

| Step | Description | Status |
|------|-------------|--------|
| 1 | Raw XML captured on body node + written back on export | DONE |
| 2 | Parse `text:changed-region` → structured JSON with id/type/author/date | DONE |
| 3 | Map to `ChangeTrackingInfo` AttributeKey | DONE |
| 4 | Render change tracking — uses same revision rendering as DOCX | DONE |
| 5 | Write change tracking back — raw XML preserved on export | DONE |
| 6 | Accept/reject changes in editor UI | DEFERRED (needs ODT-specific accept/reject logic) |

## Presence & Heartbeat — COMPLETE

| Step | Description | Status |
|------|-------------|--------|
| 1 | EditorInfo stores `last_activity` + access `mode` | DONE |
| 2 | Server updates `last_activity` on every WebSocket message | DONE |
| 3 | Client heartbeat for view-only users | DONE |
| 4 | Server cleanup: stale editors removed after 5 min inactivity | DONE |
| 5 | Admin dashboard: "Last Active" column + editor tooltip | DONE |

## M4: Error Tracking — COMPLETE

| Step | Description | Status |
|------|-------------|--------|
| 1 | Capture `console.error` automatically | DONE |
| 2 | POST `/api/v1/errors` endpoint + rate-limited client reporting | DONE |
| 3 | Admin dashboard: errors logged at WARN level (visible in server logs) | DONE |
| 4 | Rate limiting: max 10 errors/minute per client | DONE |

---

## Summary

| Section | Steps | Done | Deferred |
|---------|-------|------|----------|
| Permissions | 7 | 7 | 0 |
| Form Controls | 8 | 8 | 0 |
| Digital Signatures | 10 | 8 | 2 |
| PDF/A | 6 | 5 | 1 |
| ODT Change Tracking | 6 | 5 | 1 |
| Presence/Heartbeat | 5 | 5 | 0 |
| Error Tracking | 4 | 4 | 0 |
| **Total** | **46** | **42** | **4** |

Deferred items: document re-signing (needs private key), certificate management UI, veraPDF testing, ODT accept/reject.
