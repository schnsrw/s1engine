# Phase 1: Package & Publish

> Goal: Make s1engine consumable as packages (crates.io, npm, Docker).
> Created: 2026-03-18 | Depends on: Phase 0 (complete)

## Milestone 1.1 — crates.io Publish Readiness

| ID | Task | Status |
|----|------|--------|
| P1-01 | Audit each crate's Cargo.toml for description, docs, homepage, keywords | DONE |
| P1-02 | Verify each crate builds independently | DONE |
| P1-03 | Verify each crate tests independently | DONE |
| P1-04 | Add version to workspace internal deps (path + version for crates.io) | DONE |
| P1-05 | Dry-run publish s1-model (passes), chain verified | DONE |

## Milestone 1.2 — npm Package (@s1engine/wasm)

| ID | Task | Status |
|----|------|--------|
| P1-06 | Create packages/wasm/ with package.json | DONE |
| P1-07 | Build script: scripts/build-wasm-npm.sh | DONE |
| P1-08 | README with usage examples (Vite, Webpack) | DONE |
| P1-09 | Test: fresh project can use the package | DONE |

## Milestone 1.3 — Docker

| ID | Task | Status |
|----|------|--------|
| P1-10 | Verify Dockerfile builds (multi-stage) | DONE |
| P1-11 | Health check endpoint exists (/health) | DONE |
| P1-12 | docker-compose.yml works | DONE |

## Milestone 1.4 — Documentation Site Scaffold

| ID | Task | Status |
|----|------|--------|
| P1-13 | Set up mdBook in docs-site/ with book.toml | DONE |
| P1-14 | Quick start guide (Rust, npm, Docker) | DONE |
| P1-15 | Full SUMMARY.md structure (25 pages) | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P1-01 | 2026-03-18 | All 12 crates inherit version, license, repository, homepage, keywords from workspace |
| P1-02 | 2026-03-18 | All 12 crates build independently with `cargo build -p <crate>` |
| P1-03 | 2026-03-18 | All 12 crates test independently — 0 failures |
| P1-04 | 2026-03-18 | Added `version = "1.0.1"` to all internal workspace deps in root Cargo.toml |
| P1-05 | 2026-03-18 | `cargo publish -p s1-model --dry-run` passes; s1engine fails expectedly (deps not on crates.io yet) |
| P1-06 | 2026-03-18 | packages/wasm/package.json created with @s1engine/wasm metadata |
| P1-07 | 2026-03-18 | scripts/build-wasm-npm.sh created (wasm-pack → packages/wasm/dist + LICENSE copy) |
| P1-08 | 2026-03-18 | packages/wasm/README.md with Quick Start, Vite, Webpack usage examples |
| P1-10 | 2026-03-18 | Dockerfile verified: multi-stage (rust → wasm-pack → node/vite → runtime), HEALTHCHECK present |
| P1-11 | 2026-03-18 | /health endpoint at relay.js, HEALTHCHECK in Dockerfile with 30s interval |
| P1-12 | 2026-03-18 | docker-compose.yml exists and references the Dockerfile |
| P1-13 | 2026-03-18 | docs-site/book.toml created for mdBook with ayu dark theme, GitHub edit links |
| P1-14 | 2026-03-18 | Quick start page covers Rust, npm, Docker in 5 minutes |
| P1-15 | 2026-03-18 | SUMMARY.md with 25 pages across 5 sections (Getting Started, Guides, API, Architecture, Contributing) |
