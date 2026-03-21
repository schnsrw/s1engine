# Performance Specification v1.0

> Covers: performance targets, memory budgets, WASM bundle size, benchmarks, profiling strategy.
> Last updated: 2026-03-21

## 1. Performance Targets

### 1.1 Editor Responsiveness

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Keystroke to visible character | <16ms | Browser Performance API (`input` event to next `requestAnimationFrame`) |
| Single paragraph re-render (incremental) | <50ms | `performance.now()` around `renderSingleParagraphIfPossible()` |
| Full document render (10 pages) | <200ms | `performance.now()` around `renderDocument()` |
| Full document render (100 pages) | <2000ms | Same, with virtual scrolling active |
| CRDT text sync to peer | <20ms | Timestamp in op message vs receipt |
| CRDT structural sync to peer | <500ms | fullSync round-trip |
| Undo/redo response | <50ms | Keystroke to visible change |

### 1.2 WASM Engine

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| `to_html()` (10-page doc) | <50ms | `performance.now()` around WASM call |
| `to_html()` (100-page doc) | <500ms | Same |
| `render_node_html(nodeId)` | <5ms | Same |
| `get_page_map_json()` | <10ms | Same |
| `open()` DOCX (10 pages) | <10ms | Same |
| `open()` DOCX (100 pages) | <100ms | Same |
| `export("docx")` | <50ms | Same |
| `to_pdf()` (10 pages) | <200ms | Same |
| `to_pdf()` (100 pages) | <2000ms | Same |
| `insert_text_in_paragraph()` | <2ms | Same |
| `apply_local_insert_text()` (CRDT) | <5ms | Same |
| `apply_remote_ops()` (CRDT) | <10ms | Same |

### 1.3 Server

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| File upload (10-page DOCX) | <100ms | Request duration |
| Format conversion (DOCX to PDF) | <500ms | Request duration |
| WebSocket message relay | <5ms | Server-side processing time |
| Room broadcast (10 peers) | <10ms | Time from receive to all sends |
| Session list API | <10ms | Request duration |

## 2. Memory Budgets

### 2.1 Browser (WASM + JS)

| Component | Budget | Notes |
|-----------|--------|-------|
| WASM linear memory (baseline) | <5MB | Empty document |
| WASM linear memory (10-page doc) | <10MB | Typical document |
| WASM linear memory (100-page doc) | <20MB | Large document |
| DOM (10 pages rendered) | <5MB | Estimated from Chrome DevTools |
| DOM (100 pages, virtual scroll) | <10MB | Only ~6 pages in DOM at a time |
| CRDT state (1000 ops) | <2MB | Fugue tree + metadata |
| CRDT state (10000 ops) | <10MB | With compression |
| Font database (4 fonts loaded) | <5MB | TTF/OTF in memory |
| Offline buffer (10000 ops) | <5MB | JSON strings |
| **Total (10-page doc, active collab)** | **<30MB** | |

### 2.2 Server

| Component | Budget | Notes |
|-----------|--------|-------|
| Base process | <20MB | Axum + Tokio runtime |
| Per file session | <2MB | Document bytes + metadata |
| Per collab room | <5MB | ops_log (10k entries) + broadcast channel |
| Per WebSocket connection | <100KB | Buffers + peer state |
| 10 concurrent sessions | <70MB | Base + 10 sessions + 10 rooms |

### 2.3 WASM Memory Warnings

The editor monitors WASM memory usage and shows warnings:

| Threshold | Action |
|-----------|--------|
| 50MB | Console warning, "Large document" indicator in status bar |
| 100MB | User-facing warning toast |
| 200MB | Recommendation to save and reload |

## 3. WASM Bundle Size

### 3.1 Size Budget

| Component | Approximate Size (release, gzip) |
|-----------|----------------------------------|
| Core model + ops | ~100KB |
| DOCX reader/writer | ~150KB |
| ODT reader/writer | ~120KB |
| TXT + Markdown | ~40KB |
| Layout engine | ~80KB |
| Text shaping (rustybuzz) | ~400KB |
| PDF export | ~60KB |
| CRDT (Fugue + awareness) | ~80KB |
| **Total (gzip)** | **~350-500KB** |

### 3.2 Optimization Techniques

| Technique | Applied | Impact |
|-----------|---------|--------|
| `wasm-opt -Oz` | Yes (in release build) | ~20% reduction |
| LTO (link-time optimization) | Yes (`lto = true` in release profile) | ~10% reduction |
| `codegen-units = 1` | Yes (in release profile) | Better optimization |
| gzip/brotli compression | Yes (served by reverse proxy) | ~60% reduction |
| Feature flags (exclude unused formats) | Available | Can exclude PDF, ODT, etc. |

### 3.3 Lazy Loading Strategy

```
Initial load:
  s1engine_wasm_bg.wasm  (~500KB gzip)   ← Loaded during boot
  s1engine_wasm.js        (~20KB)          ← JS glue

Deferred (after first render):
  Font files              (~100KB-5MB each) ← Loaded async
  PDF export             (included in WASM) ← Only used on export
```

## 4. Document Open Performance

### 4.1 DOCX Open Pipeline

```
Bytes → ZIP decompress → XML parse → Model build → Ready

Step 1: ZIP decompression
  - Read ZIP central directory
  - Decompress document.xml, styles.xml, numbering.xml, etc.
  - Target: <5ms for 100KB DOCX

Step 2: XML parsing (quick-xml)
  - Stream parse XML events
  - Build document model nodes
  - Target: <5ms for 10-page doc

Step 3: Model construction
  - Create NodeId assignments
  - Resolve style chains
  - Index media store
  - Target: <2ms

Total target: <10ms for 10-page DOCX, <100ms for 100-page DOCX
```

### 4.2 First Paint Optimization

```
Boot sequence:
  T=0ms:    WASM module loaded
  T=50ms:   Engine instantiated
  T=60ms:   Document opened (WASM)
  T=110ms:  to_html() complete
  T=150ms:  First page painted (repaginate)
  T=200ms:  All pages painted
  T=500ms:  Fonts loaded (async)
  T=600ms:  Re-layout with real font metrics (async, optional)
```

**First meaningful paint target: <200ms** from WASM ready to visible content.

## 5. Critical Path Analysis

### 5.1 Typing (Fastest Path)

```
User types 'a'
  → beforeinput event                    [0ms]
  → Browser inserts character natively   [<1ms]
  → debouncedSync queued (150ms)         [0ms]
  → User sees character immediately      [<16ms total]

After 150ms debounce:
  → Sync text to WASM model              [<2ms]
  → CRDT local insert                    [<5ms]
  → Broadcast CRDT op                    [<1ms]
  → No DOM re-render needed              [0ms]
```

**Key insight:** Typing does NOT trigger re-render. The browser's native contentEditable handles display. WASM sync is debounced.

### 5.2 Formatting (Medium Path)

```
User clicks Bold
  → Apply format via WASM                [<5ms]
  → renderSingleParagraphIfPossible()    [<50ms]
    → render_node_html(nodeId)           [<5ms]
    → DOM patch (innerHTML diff)         [<20ms]
    → Cursor restore                     [<5ms]
  → Broadcast formatting op             [<1ms]
```

### 5.3 Enter Key (Slow Path)

```
User presses Enter
  → split_paragraph() via WASM           [<10ms]
  → renderDocument() (full re-render)    [<200ms for 10 pages]
    → to_html()                          [<50ms]
    → DOM clear + rebuild                [<100ms]
    → repaginate()                       [<30ms]
    → Post-render fixups                 [<20ms]
  → Broadcast fullSync (debounced)       [async]
```

## 6. Benchmarks

### 6.1 Criterion Benchmarks (Rust)

Located in `crates/s1engine/benches/engine_bench.rs`:

| Benchmark | What it measures |
|-----------|-----------------|
| `open_docx_small` | Open a small DOCX (<5 pages) |
| `open_docx_medium` | Open a medium DOCX (~20 pages) |
| `open_docx_large` | Open a large DOCX (~100 pages) |
| `export_docx` | Export to DOCX format |
| `export_odt` | Export to ODT format |
| `export_pdf` | Export to PDF format |
| `to_plain_text` | Extract plain text |
| `to_html` | Generate HTML from model |
| `insert_text` | Insert text operation |
| `crdt_insert` | CRDT local insert |
| `crdt_remote_apply` | Apply remote CRDT ops |

**Running benchmarks:**
```bash
cargo bench -p s1engine
```

Results are stored in `target/criterion/` with HTML reports.

### 6.2 Browser Benchmarks (Manual)

| Test | Steps | Target |
|------|-------|--------|
| Open 10-page DOCX | Upload file, measure to first paint | <300ms |
| Open 100-page DOCX | Upload file, measure to first paint | <2000ms |
| Type 100 characters | Measure input lag per character | <16ms each |
| Bold a paragraph | Select all, click Bold, measure re-render | <100ms |
| Enter 50 times | Create 50 new paragraphs, measure total | <10s |
| Paste 1000 lines | Paste large text block, measure render | <3s |
| Collab: 3 peers typing | All typing simultaneously, measure convergence | <2s |

### 6.3 Benchmark Fixtures

Test documents for benchmarks:
```
tests/fixtures/
  bench_small.docx     (~5 pages, basic formatting)
  bench_medium.docx    (~20 pages, tables, images)
  bench_large.docx     (~100 pages, complex formatting)
  bench_tables.docx    (document with many tables)
  bench_images.docx    (document with many images)
```

## 7. Profiling Strategy

### 7.1 Rust Profiling

| Tool | Use Case | Command |
|------|----------|---------|
| `cargo flamegraph` | CPU hotspots | `cargo flamegraph --bench engine_bench` |
| `cargo bench` (criterion) | Regression detection | `cargo bench -p s1engine` |
| `DHAT` (heap profiler) | Memory allocation | Feature-flag `dhat` in tests |
| `perf` (Linux) | System-level profiling | `perf record cargo test` |

### 7.2 WASM Profiling

| Tool | Use Case |
|------|----------|
| Chrome DevTools > Performance | Overall JS + WASM timing |
| Chrome DevTools > Memory | Heap snapshots, WASM memory |
| `console.time()` / `console.timeEnd()` | Specific operation timing |
| `performance.mark()` / `performance.measure()` | User Timing API |
| `wasm-opt --instrument` | WASM-level instrumentation |

### 7.3 When to Profile

- Before every release
- When a benchmark regresses by >10%
- When a user reports "slow" behavior
- After adding a new format or feature

## 8. Optimization Rules

### 8.1 Principles

1. **Measure first.** Never optimize without a benchmark showing the bottleneck.
2. **Profile the real workload.** Synthetic benchmarks can mislead.
3. **Optimize the algorithm first.** A better algorithm beats micro-optimization every time.
4. **Avoid premature allocation.** Use `&str`, `&[u8]`, iterators. Allocate only when needed.
5. **Cache computed results.** Layout cache, page map hash, debounced sync.
6. **Do less work.** Incremental render > full render. Virtual scroll > render all.

### 8.2 Known Hot Paths

| Hot Path | Current Optimization |
|----------|---------------------|
| Text input (typing) | Browser handles natively; WASM sync debounced 150ms |
| Single paragraph update | `renderSingleParagraphIfPossible()` avoids full re-render |
| Pagination check | Page map hash cache skips DOM reconciliation |
| Virtual scrolling | Only ~6 pages in DOM; IntersectionObserver for visibility |
| Image loading | Lazy loading with placeholder for off-screen images |
| CRDT ops | Compressed: consecutive single-char inserts merged |
| Layout engine | Incremental via `LayoutCache` (per-block content hash) |

### 8.3 Known Bottlenecks (Not Yet Optimized)

| Bottleneck | Impact | Potential Fix |
|------------|--------|---------------|
| `to_html()` for large docs | O(n) in document size | Incremental HTML generation |
| `repaginate()` full rebuild | O(n) in page count | Incremental repagination |
| fullSync base64 encoding | ~33% bandwidth overhead | Binary WebSocket messages |
| Font loading on first visit | 500ms+ delay | Service worker precache |
| PDF export for large docs | Linear in page count | Streaming PDF generation |

## 9. Regression Detection

### 9.1 CI Benchmark Baseline

Criterion benchmarks should be run in CI and compared against baselines:

```bash
# Save baseline
cargo bench -p s1engine -- --save-baseline main

# Compare against baseline
cargo bench -p s1engine -- --baseline main
```

### 9.2 Regression Thresholds

| Metric | Threshold for Warning | Threshold for Failure |
|--------|----------------------|----------------------|
| Open DOCX | +20% | +50% |
| Export DOCX | +20% | +50% |
| to_html() | +15% | +40% |
| CRDT insert | +10% | +30% |
| Memory usage | +25% | +100% |
| WASM bundle size | +5% | +15% |

## 10. Implementation Status

| Feature | Status |
|---------|--------|
| Criterion benchmark framework | DONE |
| Engine benchmarks (open, export, text) | DONE |
| Debounced text sync (150ms) | DONE |
| Incremental single-paragraph render | DONE |
| Page map hash caching | DONE |
| Virtual scrolling for large docs | DONE |
| Image lazy loading | DONE |
| Layout cache (per-block hash) | DONE |
| CRDT operation compression | DONE |
| WASM release build with wasm-opt | DONE |
| WASM memory monitoring | DONE |
| Browser performance instrumentation | PARTIAL |
| CI benchmark regression tracking | SPECIFIED (Section 11) |
| Streaming PDF generation | SPECIFIED (Section 12) |
| Incremental HTML generation | SPECIFIED (Section 13) |
| Incremental repagination | SPECIFIED (Section 14) |
| Binary WebSocket for fullSync | SPECIFIED (Section 15) |
| Service worker font precache | SPECIFIED (Section 16) |

## 11. CI Benchmark Regression Tracking

### 11.1 Overview

Criterion benchmarks run in CI on every pull request and merge to `main`. Results are compared against a stored baseline to detect performance regressions before they reach production.

### 11.2 Approach

A two-phase CI pipeline:

1. **Baseline phase** (runs on `main` branch merges only): Execute `cargo bench` and store results as the canonical baseline.
2. **Comparison phase** (runs on every PR): Execute `cargo bench` and compare against the stored baseline. Flag regressions that exceed thresholds.

### 11.3 CI Pipeline Configuration

```yaml
# .github/workflows/benchmarks.yml
name: Benchmarks
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest  # Use a consistent runner type for reproducible results
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Restore benchmark baseline
        uses: actions/cache@v4
        with:
          path: target/criterion
          key: criterion-baseline-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}
          restore-keys: criterion-baseline-${{ runner.os }}-

      - name: Run benchmarks
        run: cargo bench -p s1engine -- --output-format=bencher 2>&1 | tee bench_output.txt

      - name: Save baseline (main only)
        if: github.ref == 'refs/heads/main'
        run: cargo bench -p s1engine -- --save-baseline main

      - name: Compare against baseline (PR only)
        if: github.event_name == 'pull_request'
        run: |
          cargo bench -p s1engine -- --baseline main --output-format=bencher 2>&1 | tee comparison.txt
          # Parse and check for regressions
          python3 scripts/check_bench_regression.py comparison.txt

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: criterion-report
          path: target/criterion/
```

### 11.4 Regression Check Script

```python
# scripts/check_bench_regression.py
"""Parse criterion benchmark output and fail if regressions exceed thresholds."""

import sys
import re

THRESHOLDS = {
    "open_docx":     {"warn": 0.20, "fail": 0.50},
    "export_docx":   {"warn": 0.20, "fail": 0.50},
    "export_pdf":    {"warn": 0.20, "fail": 0.50},
    "to_html":       {"warn": 0.15, "fail": 0.40},
    "crdt_insert":   {"warn": 0.10, "fail": 0.30},
    "crdt_remote":   {"warn": 0.10, "fail": 0.30},
    "insert_text":   {"warn": 0.10, "fail": 0.30},
}

DEFAULT_THRESHOLD = {"warn": 0.20, "fail": 0.50}

def check(filepath):
    regressions = []
    warnings = []
    with open(filepath) as f:
        for line in f:
            # Match criterion output: "bench_name  time: [low est high]  change: [low est high] (p = X < 0.05)"
            m = re.search(r'change:\s+\[.*?([-+]?\d+\.\d+)%', line)
            if m:
                change_pct = float(m.group(1)) / 100.0
                bench_name = line.split()[0] if line.split() else "unknown"
                thresholds = next(
                    (v for k, v in THRESHOLDS.items() if k in bench_name),
                    DEFAULT_THRESHOLD
                )
                if change_pct > thresholds["fail"]:
                    regressions.append(f"FAIL: {bench_name} regressed {change_pct:.1%}")
                elif change_pct > thresholds["warn"]:
                    warnings.append(f"WARN: {bench_name} regressed {change_pct:.1%}")

    for w in warnings:
        print(w)
    for r in regressions:
        print(r)

    if regressions:
        print(f"\n{len(regressions)} benchmark(s) exceeded failure threshold.")
        sys.exit(1)
    elif warnings:
        print(f"\n{len(warnings)} benchmark(s) exceeded warning threshold (not blocking).")
    else:
        print("All benchmarks within acceptable range.")

if __name__ == "__main__":
    check(sys.argv[1])
```

### 11.5 Data Structures

| Artifact | Location | Retention |
|----------|----------|-----------|
| Criterion HTML reports | `target/criterion/report/` | Uploaded as CI artifact (90 days) |
| Baseline data | `target/criterion/*/base/` | Cached in CI (keyed on `Cargo.lock` hash) |
| Regression output | `comparison.txt` | Uploaded as CI artifact |
| Historical trends | GitHub Actions artifact archive | 90 days rolling |

### 11.6 Performance Impact

- CI benchmark job adds approximately 3-5 minutes to PR pipeline.
- Cached baseline avoids re-running `main` benchmarks on every PR.
- Only the `s1engine` crate benchmarks run (not individual sub-crate unit tests).

### 11.7 Implementation Priority

**MEDIUM** -- Important for preventing silent regressions. Should be implemented once the benchmark suite is stable and representative of real workloads.

## 12. Streaming PDF Generation

### 12.1 Overview

Current PDF export builds the entire PDF in memory, then returns it as a `Vec<u8>`. For large documents (100+ pages, many images), this requires significant memory. Streaming PDF generation writes pages incrementally to an output sink, keeping memory usage proportional to a single page rather than the entire document.

### 12.2 Approach

PDF 1.7 requires a cross-reference table (xref) at the end of the file, which references byte offsets of all objects. True streaming (where you never revisit earlier bytes) requires writing all objects in order, tracking offsets, and appending the xref at the end. This is achievable because `pdf-writer` already writes objects sequentially.

```
Streaming Pipeline:

  Page 1 layout → write page objects → flush to sink → free page memory
  Page 2 layout → write page objects → flush to sink → free page memory
  ...
  Page N layout → write page objects → flush to sink → free page memory
  Write shared resources (fonts, images referenced by ID)
  Write catalog, page tree, xref table
  Write trailer (with byte offset to xref)
```

### 12.3 API

```rust
/// Streaming PDF writer that outputs pages incrementally.
pub struct StreamingPdfWriter<W: Write> {
    sink: BufWriter<W>,
    object_offsets: Vec<(ObjRef, u64)>,  // Track byte offset of each object
    page_refs: Vec<ObjRef>,              // Collect page object references for page tree
    font_refs: HashMap<FontId, ObjRef>,  // Shared font objects (written at end)
    image_refs: HashMap<MediaId, ObjRef>, // Shared image objects (written at end)
    current_offset: u64,
}

impl<W: Write> StreamingPdfWriter<W> {
    /// Create a new streaming PDF writer.
    pub fn new(sink: W) -> Self { ... }

    /// Write the PDF header (%PDF-1.7).
    pub fn write_header(&mut self) -> Result<(), Error> { ... }

    /// Layout and write a single page. After this call, the page's
    /// layout data can be freed from memory.
    pub fn write_page(&mut self, page: &LayoutPage, doc: &Document) -> Result<(), Error> {
        // 1. Allocate object references for this page
        // 2. Build content stream (text positioning, images, borders)
        // 3. Write page object, content stream object to sink
        // 4. Record byte offsets in object_offsets
        // 5. Record font/image references for shared resource writing
        Ok(())
    }

    /// Write all shared resources (fonts, images) and finalize the PDF.
    /// Must be called after all pages are written.
    pub fn finalize(mut self) -> Result<(), Error> {
        // 1. Write font objects (subset + embed)
        // 2. Write image objects
        // 3. Write page tree (references all page_refs)
        // 4. Write catalog
        // 5. Write xref table (using object_offsets)
        // 6. Write trailer
        Ok(())
    }
}
```

### 12.4 Memory Profile

| Document | Current (All-in-Memory) | Streaming | Savings |
|----------|------------------------|-----------|---------|
| 10 pages, no images | ~2MB peak | ~500KB peak | 4x |
| 100 pages, no images | ~15MB peak | ~500KB peak | 30x |
| 100 pages, 50 images (2MB each) | ~115MB peak | ~5MB peak* | 23x |

*Images are still loaded one-at-a-time for embedding; only one image needs to be in memory at a time.

### 12.5 Limitations

- Font subsetting still requires collecting all glyphs used across ALL pages before writing. Approach: two-pass — first pass collects glyph usage, second pass writes pages + subsetted fonts.
- The xref table is written at the end, so the output cannot be consumed by a PDF reader until `finalize()` completes.
- Not compatible with PDF/A linearization (which requires a specific object ordering for progressive display).

### 12.6 Implementation Priority

**LOW** -- Current in-memory approach handles 100-page documents within memory budgets. Streaming becomes important for server-side batch conversion of very large documents (500+ pages) or memory-constrained environments.

## 13. Incremental HTML Generation

### 13.1 Overview

`to_html()` currently generates HTML for the entire document. For single-paragraph edits, only the edited paragraph's HTML needs to change. Incremental HTML generation builds HTML for only the changed portions.

### 13.2 Approach

The WASM engine already exposes `render_node_html(nodeId)` for single-node rendering. Incremental HTML generation extends this with:

1. A **dirty tracking system** (see RENDERING_SPEC Section 13) that identifies which nodes changed.
2. A **partial HTML generation** API that returns HTML for a set of nodes.
3. A **node HTML cache** that stores the last-generated HTML for each paragraph, avoiding redundant generation.

### 13.3 API

```rust
// In WasmDocument:

/// Returns HTML for a set of nodes (by ID).
/// More efficient than calling render_node_html() in a loop because
/// it shares style resolution and attribute computation across nodes.
#[wasm_bindgen]
pub fn render_nodes_html(&self, node_ids_json: &str) -> String {
    // Parse node IDs
    let ids: Vec<String> = serde_json::from_str(node_ids_json).unwrap_or_default();
    let mut html_parts: Vec<String> = Vec::with_capacity(ids.len());
    for id_str in &ids {
        if let Ok(id) = NodeId::parse(id_str) {
            if let Some(html) = self.render_single_node_html(id) {
                html_parts.push(html);
            }
        }
    }
    serde_json::to_string(&html_parts).unwrap_or_else(|_| "[]".to_string())
}
```

### 13.4 Node HTML Cache (Rust Side)

```rust
struct HtmlCache {
    entries: HashMap<NodeId, CacheEntry>,
}

struct CacheEntry {
    html: String,
    content_hash: u64,  // Hash of the node's text content + attributes
}

impl HtmlCache {
    fn get_or_generate(&mut self, node_id: NodeId, doc: &Document) -> &str {
        let current_hash = doc.node_content_hash(node_id);
        if let Some(entry) = self.entries.get(&node_id) {
            if entry.content_hash == current_hash {
                return &entry.html;  // Cache hit
            }
        }
        // Cache miss: generate HTML
        let html = generate_node_html(doc, node_id);
        self.entries.insert(node_id, CacheEntry { html, content_hash: current_hash });
        &self.entries[&node_id].html
    }

    fn invalidate(&mut self, node_id: NodeId) {
        self.entries.remove(&node_id);
    }

    fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}
```

### 13.5 JavaScript Usage

```javascript
// After a formatting change on a single paragraph:
function incrementalHtmlUpdate(nodeId) {
  const html = state.doc.render_node_html(nodeId);
  if (html === null) return false; // Node not found

  const existing = document.querySelector(`[data-node-id="${nodeId}"]`);
  if (!existing) return false;

  if (existing.outerHTML !== html) {
    // Replace only this element
    const temp = document.createElement("div");
    temp.innerHTML = html;
    const newElement = temp.firstElementChild;
    existing.replaceWith(newElement);
    // Re-register in nodeIdToElement map
    state.nodeIdToElement.set(nodeId, newElement);
  }
  return true;
}

// For batch updates (multiple dirty nodes):
function incrementalBatchUpdate(dirtyNodeIds) {
  const htmlJson = state.doc.render_nodes_html(JSON.stringify(dirtyNodeIds));
  const htmlArray = JSON.parse(htmlJson);

  for (let i = 0; i < dirtyNodeIds.length; i++) {
    const nodeId = dirtyNodeIds[i];
    const html = htmlArray[i];
    const existing = document.querySelector(`[data-node-id="${nodeId}"]`);
    if (existing && html) {
      const temp = document.createElement("div");
      temp.innerHTML = html;
      existing.replaceWith(temp.firstElementChild);
    }
  }
}
```

### 13.6 Performance Impact

| Scenario | Full `to_html()` | Incremental (1 node) | Incremental (5 nodes) | Improvement |
|----------|-------------------|----------------------|-----------------------|-------------|
| 10-page doc | 50ms | 3ms | 10ms | 5-17x |
| 50-page doc | 200ms | 3ms | 10ms | 20-67x |
| 100-page doc | 500ms | 3ms | 10ms | 50-167x |

The improvement scales with document size because the cost of incremental generation is constant (proportional to the number of changed nodes, not total document size).

### 13.7 Cache Invalidation Triggers

| Event | Invalidation Scope |
|-------|-------------------|
| Text edit in paragraph | Single node |
| Format change on paragraph | Single node |
| Style definition change | All nodes using that style |
| Font change | All nodes (layout may differ) |
| Document open/load | Entire cache |
| fullSync received | Entire cache |

### 13.8 Implementation Priority

**HIGH** -- This is one of the top remaining performance optimizations. It directly reduces the cost of the most common operation (single-paragraph edits) in proportion to document size. Combined with dirty flags (RENDERING_SPEC Section 13) and incremental repagination (Section 14), this enables near-constant-time editing regardless of document length.

## 14. Incremental Repagination

### 14.1 Overview

Current `repaginate()` rebuilds all page DOM containers on every invocation. For a single-paragraph edit, only pages at or after the edit point can change. Incremental repagination skips pages before the edit.

### 14.2 Approach

This optimization is fully specified in RENDERING_SPEC.md Section 12. Here we add the performance-specific details.

### 14.3 Data Structures

```javascript
// Module-level state additions:
let _previousPageMap = null;     // Last page map JSON (parsed)
let _pageMapHash = null;         // Hash of last page map string (for fast equality check)
let _pageElements = new Map();   // Map<pageNumber, HTMLElement> — DOM references
```

### 14.4 Algorithm Summary

1. Get new page map from WASM.
2. Compare hash with cached hash. If identical, skip entirely (existing optimization).
3. If different, find the first diverging page index.
4. Verify pages before that index exist in DOM.
5. Rebuild only pages from the diverging index onward.
6. Update cache.

### 14.5 Performance Impact

| Document | Full Repagination | Incremental (last page edit) | Incremental (middle edit) |
|----------|-------------------|-----------------------------|--------------------------|
| 10 pages | 30ms | 5ms | 15ms |
| 50 pages | 120ms | 8ms | 60ms |
| 100 pages | 250ms | 10ms | 125ms |
| 500 pages | 1200ms | 15ms | 600ms |

Average case: editing is most common in the middle or end of a document, yielding 2-4x improvement for medium documents and 10-80x for large documents.

### 14.6 Implementation Priority

**HIGH** -- Directly improves Enter/Backspace responsiveness in long documents. Combined with incremental HTML generation (Section 13), the full render path for a structural edit drops from O(n) to O(n - editPosition).

## 15. Binary WebSocket for fullSync

### 15.1 Overview

The current fullSync implementation encodes document bytes as base64 within a JSON text message, adding 33% bandwidth overhead. Binary WebSocket frames transmit raw bytes directly.

### 15.2 Approach

This optimization is fully specified in COLLABORATION_PROTOCOL.md Section 10. Here we add the performance-specific analysis.

### 15.3 API

```javascript
// Client-side:
function sendFullSync() {
  if (state.binaryWsSupported) {
    sendBinaryFullSync();  // See COLLABORATION_PROTOCOL Section 10.5
  } else {
    sendTextFullSync();    // Existing base64 implementation
  }
}
```

```rust
// Server-side (collab.rs):
// Handle Message::Binary in the WebSocket handler alongside Message::Text.
// See COLLABORATION_PROTOCOL Section 10.4 for the full server handling spec.
```

### 15.4 Performance Impact

| Metric | Text (Base64) | Binary | Improvement |
|--------|---------------|--------|-------------|
| Encode time (1MB doc) | ~15ms (base64 encode) | ~0ms (zero-copy) | 15ms saved |
| Decode time (1MB doc) | ~10ms (base64 decode) | ~0ms (zero-copy) | 10ms saved |
| Bandwidth (1MB doc) | 1.37MB | 1.00MB | 27% savings |
| Bandwidth (5MB doc) | 6.83MB | 5.00MB | 27% savings |
| GC pressure | High (large string allocation) | Low (ArrayBuffer, no GC) | Reduced GC pauses |

### 15.5 Implementation Priority

**HIGH** -- Low implementation effort with significant bandwidth and latency savings. Especially impactful for users on slow networks or editing large documents with images.

## 16. Service Worker Font Precache

### 16.1 Overview

Font files (TTF/OTF/WOFF2) are loaded asynchronously after the document renders. On first visit, this creates a 500ms+ delay where text renders with fallback fonts, then shifts when the correct fonts load. A service worker precaches font files so subsequent visits have instant font availability.

### 16.2 Approach

Register a service worker that:
1. On install, precaches a set of core fonts.
2. On fetch, serves cached fonts instantly (cache-first strategy).
3. In the background, updates cached fonts when new versions are available.

### 16.3 Service Worker Script

```javascript
// editor/sw-fonts.js

const CACHE_NAME = "s1engine-fonts-v1";

// Core fonts to precache on install
const CORE_FONTS = [
  "/fonts/NotoSans-Regular.woff2",
  "/fonts/NotoSans-Bold.woff2",
  "/fonts/NotoSans-Italic.woff2",
  "/fonts/NotoSans-BoldItalic.woff2",
  "/fonts/NotoSerif-Regular.woff2",
  "/fonts/NotoSerif-Bold.woff2",
  "/fonts/NotoSerif-Italic.woff2",
  "/fonts/NotoSerif-BoldItalic.woff2",
  "/fonts/NotoMono-Regular.woff2",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(CORE_FONTS);
    })
  );
  self.skipWaiting(); // Activate immediately
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    // Clean up old caches
    caches.keys().then((keys) => {
      return Promise.all(
        keys
          .filter((key) => key.startsWith("s1engine-fonts-") && key !== CACHE_NAME)
          .map((key) => caches.delete(key))
      );
    })
  );
  self.clients.claim(); // Take control of all pages
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);

  // Only intercept font requests
  if (!url.pathname.startsWith("/fonts/")) return;

  event.respondWith(
    caches.match(event.request).then((cached) => {
      if (cached) {
        // Cache hit: serve immediately, update in background
        fetch(event.request).then((response) => {
          if (response.ok) {
            caches.open(CACHE_NAME).then((cache) => {
              cache.put(event.request, response);
            });
          }
        }).catch(() => {}); // Ignore network errors for background update
        return cached;
      }

      // Cache miss: fetch from network, cache for next time
      return fetch(event.request).then((response) => {
        if (response.ok) {
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(event.request, clone);
          });
        }
        return response;
      });
    })
  );
});
```

### 16.4 Registration

```javascript
// In editor/src/main.js (during initialization):
if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw-fonts.js").then((reg) => {
    console.info("Font service worker registered:", reg.scope);
  }).catch((err) => {
    console.warn("Font service worker registration failed:", err);
    // Non-fatal: fonts will load normally via network
  });
}
```

### 16.5 Cache Strategy

| Request Type | Strategy | Rationale |
|-------------|----------|-----------|
| Core fonts (in CORE_FONTS list) | Precache on install | Essential for first paint |
| Other font requests (`/fonts/*`) | Cache on first use (stale-while-revalidate) | Non-core fonts cached after first load |
| Non-font requests | Pass through (no interception) | Service worker is font-specific |

### 16.6 Storage Budget

| Component | Size |
|-----------|------|
| 9 core fonts (WOFF2) | ~1.5MB total |
| Additional cached fonts (up to 20) | ~5MB |
| **Total cache budget** | **~6.5MB** |

Browsers typically allow 5-50MB of Cache Storage per origin. The font cache stays well within this limit.

### 16.7 Performance Impact

| Metric | Without Service Worker | With Service Worker (second visit) | Improvement |
|--------|----------------------|-------------------------------------|-------------|
| Font load time (first visit) | 500-2000ms (network) | 500-2000ms (same, caching in background) | None (first visit) |
| Font load time (second visit) | 100-500ms (HTTP cache, conditional) | <5ms (Cache API, no network) | 20-100x |
| Font load time (offline) | Fails | <5ms (fully cached) | Enables offline |
| FOUT (flash of unstyled text) | Visible on every visit | Only on first visit | Eliminates FOUT |

### 16.8 Offline Support

With the service worker active, the editor can render documents with correct fonts even when offline. The WASM engine and document data are separate concerns (not cached by this service worker), but font availability is guaranteed.

### 16.9 Cache Invalidation

| Trigger | Action |
|---------|--------|
| New font version deployed | Update `CACHE_NAME` version (e.g., `s1engine-fonts-v2`) |
| Service worker update | `activate` event deletes old caches |
| User clears browser storage | Cache lost; repopulated on next visit |
| Cache storage pressure | Browser may evict; non-critical (falls back to network) |

### 16.10 Implementation Priority

**LOW** -- Improves second-visit load time but does not affect core editing performance. Most valuable for users who frequently return to the editor (daily use). Can be implemented independently of other optimizations.
