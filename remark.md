# Repository Remarks and Priorities

## Latest Rescan Summary

I rescanned the repository from the current state and re-verified the main quality gates.

Verification performed:

- `cargo test --workspace`
- `cargo check --workspace --examples --benches`
- `cargo clippy --workspace -- -D warnings`
- spot review of `README.md`, `docs/API_DESIGN.md`, `docs/ROADMAP.md`, `docs/SPECIFICATION.md`
- spot review of `crates/s1engine`, `ffi/wasm`, and `ffi/c`

Current state:

- workspace tests pass
- examples and benches compile
- clippy is clean
- markdown examples are now validated through `crates/s1engine/tests/doc_examples.rs`
- fuzz targets exist under `fuzz/fuzz_targets/`
- benchmark baselines are published in `CHANGELOG.md`

Overall assessment:

- the earlier core correctness concerns remain closed
- the repo now looks much more mature and better defended
- the main remaining work is contract consistency, especially around bindings, feature flags, and status messaging

---

## Priority Legend

- `P0`: correctness or integrity blocker
- `P1`: contract or API issue that can mislead users
- `P2`: maintenance and hardening improvement
- `P3`: strategic maturity work

---

## Strong Improvements Since Earlier Reviews

### Core Correctness

- subtree undo has regression coverage and appears fixed
- mixed attribute undo has regression coverage and appears fixed
- Unicode text editing has character-aware handling and regression coverage
- move-cycle prevention is implemented and tested

### Hardening Signals

- invariant tests exist in `crates/s1engine/tests/invariants.rs`
- hostile-input tests exist in `crates/s1engine/tests/hostile_inputs.rs`
- markdown example validation exists in `crates/s1engine/tests/doc_examples.rs`
- fuzz targets exist for engine open and format readers under `fuzz/fuzz_targets/`
- benchmark scaffolding exists in `crates/s1engine/benches/engine_bench.rs`
- performance baselines are documented in `CHANGELOG.md`
- the workspace is currently clippy-clean with `-D warnings`

### Documentation Progress

- `README.md` is much closer to reality than in the earlier passes
- `docs/API_DESIGN.md` is materially improved and example-backed
- `docs/SPECIFICATION.md` is much better than before, but still has a few inconsistencies
- `docs/ROADMAP.md` reflects the implementation far better than it did earlier

### Fidelity Appraisal

- text fidelity is a real strength now:
  - DOCX, ODT, and TXT round-trip text preservation has invariant coverage
  - Unicode text round-trip coverage exists
  - cross-format text preservation is tested through DOCX -> ODT -> TXT
- DOCX fidelity is the strongest format story in the repo:
  - there is broad feature-level round-trip coverage for formatting, tables, images, lists, sections, headers/footers, hyperlinks, bookmarks, comments, tab stops, paragraph borders/shading, and character spacing
- ODT fidelity is respectable for core content, metadata, styles, tables, and images, but still narrower than DOCX
- PDF should still be described as export/render fidelity, not round-trip fidelity
- the main fidelity gap is product-level proof:
  - most evidence still comes from synthetic round-trip tests, not a large real-world fixture corpus

---

## Current Priority Queue

### P1: Public Contract Clarity

1. Binding format surface exceeds the features enabled in the binding crates
   - Location:
     - `ffi/wasm/Cargo.toml`
     - `ffi/c/Cargo.toml`
     - `ffi/wasm/src/lib.rs`
     - `ffi/c/src/lib.rs`
     - `crates/s1engine/src/engine.rs`
     - `crates/s1engine/src/document.rs`
   - Problem:
     - the C and WASM bindings depend on `s1engine` with default features only
     - those default features do not enable `pdf` or `doc-legacy`
     - but the binding parsers still accept `"pdf"` and `"doc"`
     - in WASM, `detect_format()` can return `"doc"` even though the default WASM build cannot actually open DOC via the facade
   - Why it matters:
     - consumers can observe or pass a format string that the same binding build cannot actually service
     - this is a real contract mismatch, not just a docs typo
   - Suggestion:
     - either enable `pdf` and `doc-legacy` for the binding crates, or
     - narrow the accepted binding format strings to the features actually compiled in, or
     - expose feature availability explicitly so runtime behavior is unsurprising

2. Status messaging still disagrees across top-level docs
   - Location:
     - `README.md`
     - `docs/ROADMAP.md`
   - Problem:
     - `README.md` says Phase 5 is in progress
     - `docs/ROADMAP.md` says Phase 5 is complete
   - Why it matters:
     - repo maturity is one of the first trust signals readers see
   - Suggestion:
     - choose one current status and make both documents say the same thing

3. The long-form spec still has inconsistent `.doc` and binding documentation
   - Location:
     - `docs/SPECIFICATION.md`
   - Problem:
     - the conversion section documents `Format::Doc`
     - the public `Format` section later omits `Doc`
     - the WASM section says format strings and `detect_format()` only deal with `docx`/`odt`/`pdf`/`txt`
     - the actual WASM implementation can parse `"doc"` and `detect_format()` can emit `"doc"`
   - Why it matters:
     - the spec is now close enough to reality that these remaining inconsistencies stand out more
   - Suggestion:
     - sync the spec to the actual current API and feature-gated behavior

### P2: Maintenance and Hardening

4. README crate test counts are already drifting again
   - Location:
     - `README.md`
   - Problem:
     - the crate table still uses exact test counts
     - at least some counts no longer match the current run
   - Why it matters:
     - exact counts are high-drift documentation and add low long-term value
   - Suggestion:
     - either remove exact counts, or generate them automatically

5. Fidelity claims are now credible, but still mostly synthetic
   - Evidence:
     - the support matrix is much more honest now
     - invariant tests cover round-trip and cross-format text preservation
     - DOCX writer tests cover a broad feature set
   - Remaining gap:
     - there is still no curated real-world corpus with expected fidelity outcomes
     - there is still no golden comparison suite against Word or LibreOffice outputs
   - Suggestion:
     - add a fixture corpus of real documents
     - annotate expected-loss behavior per fixture
     - separate content fidelity from layout/render fidelity in verification

### P3: Strategic Maturity

6. Decide how much API surface belongs in `s1engine` versus the lower-level crates
   - Current state:
     - the internal crates are strong
     - the facade is still intentionally modest
   - Tension:
     - the docs and project ambition sometimes point slightly above the current facade surface
   - Suggestion:
     - choose one posture and state it clearly:
       - thin facade over strong internal crates, or
       - more opinionated SDK with richer convenience APIs

7. The support matrix exists; next step is support confidence
   - Current state:
     - this is a good improvement and should stay
   - Suggestion:
     - tie major support claims to named tests and fixture groups
     - keep content fidelity and render fidelity separate in public messaging

---

## Suggested Fix Order

### Phase A: Contract Cleanup

- fix the binding feature mismatch for `pdf` and `doc`
- align top-level status messaging between `README.md` and `docs/ROADMAP.md`
- sync the remaining `.doc` and binding details in `docs/SPECIFICATION.md`

### Phase B: Maintenance Tightening

- remove or automate exact test counts in `README.md`
- keep the benchmark section current if the benches evolve
- start building a real-world fidelity fixture corpus

### Phase C: Public Maturity

- choose and document the long-term facade posture more explicitly
- tie support-matrix claims to fixtures and named verification
- keep fidelity wording disciplined: content, structure, and render fidelity are not the same thing

---

## Mentor View

- The repo has moved well past "interesting prototype" territory.
- The current work is mostly about making the contract as strong as the implementation.
- That is a good sign.
- The codebase is now strong enough that documentation drift and feature-surface ambiguity matter more than raw implementation volume.
- Fidelity is good enough to discuss seriously, but not broad enough to oversell without a real-world corpus.

### Mentor Advice

- Keep closing trust gaps, not just feature gaps.
- Make the bindings honest about the features they really ship with.
- Stop using high-drift numbers in docs unless they are generated.
- If you talk about fidelity publicly, always separate:
  - text/content fidelity
  - structure/feature fidelity
  - layout/render fidelity

---

## Critic View

- The current `remark.md` before this rescan had become too optimistic for the actual repo state.
- The repo is in good shape, but not at the point where every `P1` and `P2` concern is closed.
- The biggest live issue is no longer core correctness.
- The biggest live issue is contract honesty around features, especially in the bindings and spec.

---

## Appraiser View

- Architecture: `8.5/10`
- Implementation breadth: `8.8/10`
- Test breadth: `9.5/10`
- Fidelity confidence: `7.8/10`
- API maturity: `7.2/10`
- Documentation accuracy: `7.2/10`
- Release readiness: `7.2/10`

### Positive Appraisal

- The repo has real hardening signals now, not just implementation breadth.
- The test surface is strong and diverse.
- DOCX fidelity for supported features is genuinely impressive for a pre-release engine.
- Adding example validation, fuzz targets, and benchmark baselines was the right move.

---

## Final Remark

This repository now looks like a serious pre-release document engine.

It does not need another broad wave of features before it becomes more convincing.

It needs:

- tighter feature-contract honesty in the bindings
- one consistent project status story
- a final pass on spec drift
- real-world fidelity proof to match the increasingly strong synthetic coverage

That is a much better class of problem than the repo had in the earlier reviews.
