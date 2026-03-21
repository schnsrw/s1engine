# Rudra Code Specification Index

> Each feature area has its own specification document.
> Process: Research → Spec → Break → Fix → Implement → Test → Verify
> Last updated: 2026-03-20

## Specifications

| # | Area | Document | Status |
|---|------|----------|--------|
| 1 | **Collaboration Protocol** | [COLLABORATION_PROTOCOL.md](COLLABORATION_PROTOCOL.md) | v3.1 — Complete (22 edge cases, full checklists) |
| 2 | **Editor UX** | [specs/EDITOR_UX_SPEC.md](specs/EDITOR_UX_SPEC.md) | v1.0 — Cursor, selection, clipboard, images, undo |
| 3 | **Rendering Engine** | [specs/RENDERING_SPEC.md](specs/RENDERING_SPEC.md) | v1.0 — Incremental render, pagination, performance |
| 4 | **DOCX Format Fidelity** | [specs/DOCX_FIDELITY_SPEC.md](specs/DOCX_FIDELITY_SPEC.md) | v1.0 — Feature matrix, round-trip, compatibility |
| 5 | **ODT Format Fidelity** | [specs/ODT_FIDELITY_SPEC.md](specs/ODT_FIDELITY_SPEC.md) | v1.0 — Feature matrix, limitations |
| 6 | **PDF Export** | [specs/PDF_EXPORT_SPEC.md](specs/PDF_EXPORT_SPEC.md) | v1.0 — Pipeline, targets, edge cases |
| 7 | **Server API** | [specs/SERVER_API_SPEC.md](specs/SERVER_API_SPEC.md) | v1.0 — REST + WebSocket + auth |
| 8 | **Security Model** | [specs/SECURITY_SPEC.md](specs/SECURITY_SPEC.md) | v1.0 — Auth, authz, input validation, DOCX security |
| 9 | **Performance** | [specs/PERFORMANCE_SPEC.md](specs/PERFORMANCE_SPEC.md) | v1.0 — Targets, benchmarks, profiling |
| 10 | **Admin Panel** | [specs/ADMIN_PANEL_SPEC.md](specs/ADMIN_PANEL_SPEC.md) | v1.0 — Dashboard, errors, health, enhancement plan |
| 11 | **E2E Test Plan** | [specs/E2E_TEST_PLAN.md](specs/E2E_TEST_PLAN.md) | v1.0 — Automated + manual scenarios |
| 12 | **Spreadsheet Formats** | [specs/SPREADSHEET_SPEC.md](specs/SPREADSHEET_SPEC.md) | v1.0 — XLSX, ODS, CSV, TSV, data model, formula engine |

## Trackers

| Document | Purpose |
|----------|---------|
| [issues/COMPREHENSIVE_ISSUE_TRACKER.md](issues/COMPREHENSIVE_ISSUE_TRACKER.md) | Master issue tracker (92 items) |
| [issues/PHASED_ROADMAP.md](issues/PHASED_ROADMAP.md) | 6-phase roadmap with status |
| [issues/PHASE4_ENTERPRISE_TRACKER.md](issues/PHASE4_ENTERPRISE_TRACKER.md) | Enterprise features (46 steps) |
| [issues/PHASE5_ADVANCED_FORMATS_TRACKER.md](issues/PHASE5_ADVANCED_FORMATS_TRACKER.md) | Advanced format support (30 steps) |
| [issues/PHASE6_MULTI_APP_TRACKER.md](issues/PHASE6_MULTI_APP_TRACKER.md) | Multi-app suite (20 steps) |
| [issues/ZIP_PRESERVATION_TRACKER.md](issues/ZIP_PRESERVATION_TRACKER.md) | ZIP entry round-trip preservation |

## Development Process

```
1. RESEARCH    → Study standard + existing implementations
2. SPECIFY     → Write spec: happy path + edge cases + errors + targets
3. BREAK       → Adversarial thinking: what breaks this?
4. FIX SPEC    → Update spec to handle breaks
5. IMPLEMENT   → Code to spec (spec items = test cases)
6. TEST        → Unit + integration + E2E for every spec item
7. VERIFY      → Real documents from various office applications
```
