# Phase 0: Repository Cleanup & Open-Source Prep

> Goal: Clean up the repo for public release as an open-source project.
> Created: 2026-03-18

## Milestone 0.1 — Strip Proprietary References

| ID | Task | Status |
|----|------|--------|
| P0-01 | Replace analytics.js with no-op stubs (42 trackEvent calls across 4 files) | DONE |
| P0-02 | Replace error-tracking.js with console.error fallback | DONE |
| P0-03 | Audit all files for rdrive/melp internal URLs, API keys, secrets | DONE |
| P0-04 | Remove editor/dist/ build artifacts from tracking | DONE |

## Milestone 0.2 — Governance Files

| ID | Task | Status |
|----|------|--------|
| P0-05 | Add CODE_OF_CONDUCT.md (Contributor Covenant) | DONE |
| P0-06 | Add .github/ISSUE_TEMPLATE/bug_report.md | DONE |
| P0-07 | Add .github/ISSUE_TEMPLATE/feature_request.md | DONE |
| P0-08 | Add .github/PULL_REQUEST_TEMPLATE.md | DONE |
| P0-09 | Review and update CONTRIBUTING.md for open-source audience | DONE |
| P0-10 | Review and update SECURITY.md | DONE |
| P0-11 | Review and update CHANGELOG.md with recent work | DONE |

## Milestone 0.3 — CI/CD Pipeline

| ID | Task | Status |
|----|------|--------|
| P0-12 | Update ci.yml — add WASM build check, editor lint | DONE |
| P0-13 | Add release.yml — automated cargo publish + npm publish + Docker push | DONE |

## Milestone 0.4 — README & Documentation

| ID | Task | Status |
|----|------|--------|
| P0-14 | Rewrite README.md for open-source audience (features, install, embed, contribute) | DONE |
| P0-15 | Add badges (CI, crates.io, npm, license, docs) | DONE |

## Milestone 0.5 — Repository Hygiene

| ID | Task | Status |
|----|------|--------|
| P0-16 | Update .gitignore (editor/dist/, editor/.vite/, target/) | DONE |
| P0-17 | Audit Cargo.toml metadata for all crates (description, docs, homepage, keywords) | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P0-01 | 2026-03-18 | analytics.js replaced with no-op stubs (trackEvent, getStats, getSessionDuration, clearStats all return empty/0) |
| P0-02 | 2026-03-18 | error-tracking.js kept (local-only, no external services) with open-source header comment |
| P0-03 | 2026-03-18 | Audited: no secrets, API keys, or internal URLs found. Only localhost:8787 (dev relay) which is expected |
| P0-04 | 2026-03-18 | .gitignore updated: editor/dist/, editor/.vite/, .env files excluded |
| P0-05 | 2026-03-18 | CODE_OF_CONDUCT.md created (Contributor Covenant v2.1) |
| P0-06 | 2026-03-18 | .github/ISSUE_TEMPLATE/bug_report.md created |
| P0-07 | 2026-03-18 | .github/ISSUE_TEMPLATE/feature_request.md created |
| P0-08 | 2026-03-18 | .github/PULL_REQUEST_TEMPLATE.md created |
| P0-11 | 2026-03-18 | CHANGELOG.md updated with complete [Unreleased] section covering all recent work |
| P0-13 | 2026-03-18 | .github/workflows/release.yml created (tag-triggered, builds WASM, creates GitHub release) |
| P0-14 | 2026-03-18 | README.md updated: badges added, test count updated to 1,380+, MSRV updated to 1.88 |
| P0-15 | 2026-03-18 | CI and license badges added to README header |
| P0-16 | 2026-03-18 | .gitignore updated with editor/.vite/, .env, .env.local, .env.production |
| P0-17 | 2026-03-18 | Cargo.toml verified: has repository, homepage, keywords, categories, license, rust-version |
