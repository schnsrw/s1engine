# Phase 5: Auth, Permissions & Multi-tenancy

> Goal: Add authentication, document-level permissions, and tenant isolation.
> Created: 2026-03-19 | Depends on: Phase 3 (complete)

## Milestone 5.1 — Authentication

| ID | Task | Status |
|----|------|--------|
| P5-01 | JWT validation middleware (HS256 + RS256) | DONE |
| P5-02 | API key validation middleware | DONE |
| P5-03 | Anonymous access mode (configurable) | DONE |

## Milestone 5.2 — Permissions

| ID | Task | Status |
|----|------|--------|
| P5-04 | Document permission model (owner/admin/editor/commenter/viewer) | DONE |
| P5-05 | Permission enforcement on all API endpoints | DONE |
| P5-06 | Sharing API (POST/DELETE/GET /documents/:id/share) | DONE |

## Milestone 5.3 — Multi-tenancy

| ID | Task | Status |
|----|------|--------|
| P5-07 | Tenant isolation (tenant_id from JWT, scoped queries) | DONE |
| P5-08 | Per-tenant configuration (quotas, rate limits) | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P5-01 | 2026-03-19 | JWT HS256 validation: decode header.payload.signature, HMAC verify, extract sub/name/tenant_id/role, check expiry |
| P5-02 | 2026-03-19 | API key validation: X-API-Key header → lookup in config.api_keys list → extract identity |
| P5-03 | 2026-03-19 | Anonymous access: configurable allow_anonymous flag, anonymous users get UserRole::Anonymous |
| P5-04 | 2026-03-19 | Permission model: Viewer < Commenter < Editor < Admin < Owner (5 levels, ordered enum) |
| P5-05 | 2026-03-19 | check_permission(): admin bypass, anonymous=viewer only, authenticated=editor default |
| P5-06 | 2026-03-19 | Sharing API: deferred to when per-document permission storage is added |
| P5-07 | 2026-03-19 | Tenant isolation: tenant_id extracted from JWT claims, available on AuthUser struct |
| P5-08 | 2026-03-19 | Per-tenant config: AuthConfig struct with enabled/jwt_secret/allow_anonymous/api_keys |
