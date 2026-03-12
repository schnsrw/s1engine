# Repository Remarks and Priorities

## Re-evaluation Summary

This is a materially stronger repository than it was in the previous pass.

What changed:

- I re-ran `cargo test --workspace`
- the workspace passed cleanly
- current run shows `739` passing tests across the workspace test binaries, plus passing doc-tests
- the earlier high-risk correctness issues appear to be fixed in code and covered by regression tests

The repo now feels like a credible pre-release codebase rather than an impressive-but-risky prototype.

The main remaining work is no longer "core tree integrity". It is now:

- public contract cleanup
- documentation synchronization
- example verification
- API semantics polish

---

## Priority Legend

- `P0`: blocking correctness or integrity issue
- `P1`: contract or API issue that still hurts trust
- `P2`: hardening and maturity work
- `P3`: strategic improvement

---

## Closed Since Previous Review

### Closed P0 Items

1. Subtree undo now appears fixed
   - Evidence:
     - `crates/s1-ops/src/operation.rs:207`
     - `crates/s1-ops/src/operation.rs:278`
     - tests: `subtree_undo_restores_entire_tree`, `subtree_undo_redo_roundtrip`, `subtree_undo_deep_table`
   - Assessment:
     - previous loss-of-subtree undo concern is no longer an active issue

2. Mixed attribute undo now appears fixed
   - Evidence:
     - `crates/s1-ops/src/operation.rs:356`
     - tests: `attribute_undo_mixed_add_and_overwrite`, `attribute_undo_byte_exact_equality`
   - Assessment:
     - the prior lossy inverse concern looks resolved

3. Unicode-safe text offsets now appear fixed
   - Evidence:
     - `crates/s1-model/src/tree.rs:413`
     - character-aware offset conversion and new Unicode-focused tests
   - Assessment:
     - this was one of the most important fixes; good improvement

4. Cycle prevention on moves now appears fixed
   - Evidence:
     - `crates/s1-model/src/tree.rs:357`
     - tests: `move_node_rejects_self_parent`, `move_node_rejects_descendant_as_parent`, `move_node_rejects_deep_descendant`
   - Assessment:
     - tree integrity is notably stronger now

5. Repo truthfulness improved in several important places
   - Evidence:
     - `README.md`
     - `docs/API_DESIGN.md`
     - `docs/ARCHITECTURE.md`
     - `docs/DEPENDENCIES.md`
     - added `ffi/c`
     - added `crates/s1engine/examples`
     - added `crates/s1engine/tests/invariants.rs`
     - added `crates/s1-crdt/tests/proptests.rs`
   - Assessment:
     - this is real progress, not cosmetic progress

### Current P0 Status

- No confirmed `P0` issues remain from this review pass.
- If you are entering a hardening phase, that is the correct move.

---

## Current Priority Queue

### P1: Contract and API Truthfulness

1. `docs/SPECIFICATION.md` is still significantly out of sync with the actual codebase
   - Location:
     - `docs/SPECIFICATION.md:1097`
     - `docs/SPECIFICATION.md:1125`
     - `docs/SPECIFICATION.md:1140`
     - `docs/SPECIFICATION.md:1143`
     - `docs/SPECIFICATION.md:1146`
     - `docs/SPECIFICATION.md:1160`
     - `docs/SPECIFICATION.md:1204`
     - `docs/SPECIFICATION.md:1230`
   - Problem:
     - it still describes an older, more aspirational API surface:
       - `EngineConfig`
       - `create_document()`
       - `save()`
       - `paragraphs()`
       - `find_replace()`
       - `layout()`
       - outdated C/WASM APIs
   - Why it matters:
     - this is likely the document a serious reader will treat as authoritative
   - Suggestion:
     - either rewrite Sections 10-12 to match the real API, or explicitly label them as future design, not current behavior

2. `docs/ROADMAP.md` still contains outdated deliverable snippets
   - Location:
     - `docs/ROADMAP.md:298`
     - `docs/ROADMAP.md:302`
     - `docs/ROADMAP.md:305`
   - Problem:
     - the roadmap still shows `doc.layout()`, `doc.save()`, and `engine.open_file("legacy.doc")` style usage that does not match the current facade
   - Why it matters:
     - roadmap examples should reinforce the public contract, not contradict it
   - Suggestion:
     - replace old deliverable snippets with examples that actually compile against the current crate

3. `docs/API_DESIGN.md` is much better, but still not fully verified
   - Location:
     - `docs/API_DESIGN.md:131`
   - Problem:
     - the example uses `s1engine::Transaction::new("Update metadata")`, but the current API uses `Transaction::new()` or `Transaction::with_label(...)`
   - Why it matters:
     - this is exactly the kind of drift that keeps reappearing unless examples are enforced by tests
   - Suggestion:
     - make markdown examples compile in CI, not just Rust doc comments inside source files

4. `paragraph_ids()` and `paragraph_count()` are underdefined and likely misleading
   - Location:
     - `crates/s1engine/src/document.rs:100`
   - Problem:
     - the method says "paragraph node IDs in document order", but the implementation only filters direct children of the body node
   - Why it matters:
     - users will reasonably expect "paragraph count" to mean all paragraphs, not only top-level body paragraphs
   - Suggestion:
     - choose one of:
       - rename them to `body_paragraph_ids()` / `body_paragraph_count()`, or
       - implement full traversal semantics and document that clearly

### P2: Hardening and Maintenance

5. Status numbers in docs are hand-maintained and already drifting
   - Location:
     - `README.md:11`
   - Problem:
     - `README.md` says `732 tests passing`, but the current workspace run is `739`
   - Why it matters:
     - hand-maintained numbers go stale quickly and create small trust cuts over time
   - Suggestion:
     - either remove exact counts from human-written docs or generate them automatically

6. Markdown docs are still not part of the verification pipeline
   - Current state:
     - source doc-tests pass
     - source examples and integration tests exist
     - markdown docs can still drift
   - Suggestion:
     - validate `README.md`, `docs/API_DESIGN.md`, and any public-facing snippets in CI
     - if full doctesting is too much, mirror each published example under `examples/` or `tests/`

7. The repository is now ready for stronger contract testing
   - Good news:
     - you already added invariant tests and a CRDT property test
   - Next suggestion:
     - add more public-contract tests around:
       - paragraph query semantics
       - support matrix claims
       - FFI parity with Rust facade
       - cross-format text preservation

8. A support matrix would now have real value
   - Why now:
     - the implementation breadth is wide enough that unsupported edges matter
   - Suggestion:
     - document support as:
       - supported
       - partially supported
       - lossy
       - unsupported
     - do this per format and per feature family

### P3: Strategic Maturity Work

9. Decide whether `s1engine` is a low-level engine facade or a polished SDK
   - Current state:
     - internals are stronger than the high-level API
   - Suggestion:
     - either keep the facade intentionally thin and document it that way, or
     - invest in a higher-level query/edit API that matches the ambition of the docs

10. Add benchmark baselines
   - Suggestion:
     - now that correctness is stronger, start tracking parse/export/layout costs on representative documents

11. Add fuzzing if you want to harden format readers seriously
   - Suggestion:
     - prioritize parser robustness and panic resistance for DOCX/ODT container and XML inputs

---

## Suggested Fix Order

### Phase A: Contract Cleanup

- update `docs/SPECIFICATION.md`
- update stale snippets in `docs/ROADMAP.md`
- fix `docs/API_DESIGN.md` transaction example
- decide and document paragraph query semantics

### Phase B: Verification

- add CI coverage for markdown examples
- stop hand-maintaining exact test counts in docs
- add tests for paragraph query semantics and any published support claims

### Phase C: Public Maturity

- publish a support matrix
- decide SDK-vs-engine positioning more explicitly
- add benchmark baselines
- add fuzzing targets

---

## Mentor View

- You fixed the scary stuff. That matters.
- The repo now needs consistency more than rescue.
- This is the phase where many projects lose discipline because things "mostly work". Avoid that.
- Do not let the long-form docs lag behind again. That is now the main risk to credibility.

### Mentor Advice

- Treat `docs/SPECIFICATION.md` as a product artifact, not a note.
- Stop using unverified examples in public docs.
- Tighten names and semantics in the facade before expanding the surface area further.
- Keep shipping hardening work; it is visibly paying off.

---

## Critic View

- The code improved faster than the long-form docs.
- The repo is close enough to seriousness that stale specification material now stands out sharply.
- The remaining weak point is not implementation volume; it is contract precision.

---

## Appraiser View

- Architecture: `8.5/10`
- Implementation breadth: `8.5/10`
- Test breadth: `9/10`
- API maturity: `6/10`
- Documentation accuracy: `6/10`
- Release readiness: `6/10`

### Positive Appraisal

- This is now clearly more than an architecture-first repo.
- The addition of invariant tests, property tests, examples, and C FFI materially improved confidence.
- The repo is on a better trajectory now because the recent work reduced real technical risk, not just appearances.

---

## What Would Raise This Repo Fastest From Here

- rewrite `docs/SPECIFICATION.md` to the current API
- fix the remaining stale roadmap/API snippets
- define `paragraph_count()` semantics precisely
- validate markdown examples in CI
- publish a support matrix

---

## Final Remark

This repository has crossed an important threshold.

In the previous pass, the main question was: "is the core safe enough to trust?"

In this pass, the main question is: "is the public contract accurate enough to trust?"

That is a much better problem to have.
