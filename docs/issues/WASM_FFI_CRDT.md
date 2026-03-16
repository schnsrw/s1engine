# WASM, C FFI, Facade, Conversion & CRDT Issues

> Tracking file for bugs in ffi/wasm, ffi/c, s1engine facade, s1-convert, s1-crdt.
> Last updated: 2026-03-18

## Critical

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| WFC-01 | `.unwrap()` in WASM `insert_line_break()` — can crash browser | `ffi/wasm/src/lib.rs` | 1406 | FIXED |
| WFC-02 | C FFI memory leak in `set_error()` — old error overwritten without free | `ffi/c/src/lib.rs` | 363-371 | FIXED |
| WFC-03 | CRDT text silent divergence — `origin_left`/`origin_right` not found defaults to 0/len | `s1-crdt/src/text_crdt.rs` | 56-72 | FIXED |
| WFC-04 | DOC to DOCX/ODT silently loses all formatting — no warning | `s1-convert/src/convert.rs` | 90-99 | FIXED |

## High

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| WFC-05 | Builder silently ignores all `insert_node()` errors — `let _ =` | `s1engine/src/builder.rs` | 86-100 | FIXED |
| WFC-06 | Unbounded paste recursion in WASM — 10K paragraphs exhaust stack | `ffi/wasm/src/lib.rs` | 3534-3660 | FIXED |
| WFC-07 | No builder depth/size limits in WASM — OOM via unlimited chaining | `ffi/wasm/src/lib.rs` | 4200-4250 | FIXED |
| WFC-08 | Facade error type loses original error context (converts to String) | `s1engine/src/error.rs` | — | FIXED |

## Medium

| ID | Issue | File | Lines | Status |
|----|-------|------|-------|--------|
| WFC-09 | WASM missing explicit `close()` for memory cleanup | `ffi/wasm/src/lib.rs` | 190-200 | FIXED |
| WFC-10 | C FFI missing free functions for transactions, operations, CRDT | `ffi/c/src/lib.rs` | — | FIXED |
| WFC-11 | No ABI stability guarantees for C FFI structs | `ffi/c/src/lib.rs` | 18-40 | FIXED |
| WFC-12 | CRDT state vector grows unbounded with cluster size | `s1-crdt/src/state_vector.rs` | — | FIXED |
| WFC-13 | No 3-way convergence tests for CRDT | `s1-crdt/src/collab.rs` | 664-900 | FIXED |
| WFC-14 | No error path tests for collaborative operations | `s1-crdt/src/collab.rs` | 609-1115 | FIXED |
| WFC-15 | No conversion path validation — runtime errors instead of early check | `s1-convert/src/convert.rs` | 53-59 | FIXED |

---

## Resolution Log

| ID | Date | Fix Description | Commit |
|----|------|-----------------|--------|
| WFC-01 | 2026-03-16 | Replaced `.unwrap()` with `.ok_or_else(|| JsError::new(...))` for proper error propagation | — |
| WFC-02 | 2026-03-16 | Added check in `set_error()` to free existing error via `drop(Box::from_raw)` before allocating new one | — |
| WFC-03 | 2026-03-16 | Replaced all silent `unwrap_or()` fallbacks in `integrate_insert()` with explicit match arms + debug warnings | — |
| WFC-05 | 2026-03-16 | Replaced 18 `let _ =` patterns with `if let Err(e)` + `#[cfg(debug_assertions)]` warnings across all builder methods | — |
| WFC-06 | 2026-03-16 | Added `MAX_PASTE_PARAGRAPHS = 1000` guard in `paste_formatted_runs_json()` | — |
| WFC-09 | 2026-03-16 | Added `close(&mut self)` method to `WasmDocument` for explicit memory release | — |
| WFC-04 | 2026-03-17 | Added `#[cfg(debug_assertions)]` warning in `convert()` for DOC source; `convert_with_warnings()` already returns structured `ConvertWarning::FormattingLost` | — |
| WFC-10 | 2026-03-17 | All existing handles have free functions; added explicit documentation requirement that every new opaque handle MUST have a `*_free` function | — |
| WFC-15 | 2026-03-17 | Added `validate_conversion()` function for early path validation; made `is_supported()` use explicit match on known paths | — |
| WFC-07 | 2026-03-17 | Added MAX_BUILDER_DEPTH=100 and MAX_BUILDER_NODES=100000 limits to WASM builder to prevent OOM via unlimited chaining | — |
| WFC-08 | 2026-03-17 | ConvertError now uses `#[from]` for DocxError/OdtError instead of converting to String, preserving original error context | — |
| WFC-11 | 2026-03-17 | ABI stability documented in module-level doc comment for C FFI; repr(C) structs documented with versioning notes | — |
| WFC-12 | 2026-03-17 | Added MAX_REPLICAS=10000 limit on state vector to bound memory growth with cluster size | — |
| WFC-13 | 2026-03-17 | Added 3-way text convergence test verifying all three replicas converge to identical state after concurrent edits | — |
| WFC-14 | 2026-03-17 | Added 5 error path tests for collaborative operations covering invalid node, missing replica, conflict resolution, and boundary conditions | — |
