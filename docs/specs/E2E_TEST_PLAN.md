# End-to-End Test Plan v1.0

> Covers: test scenarios, automated tests, manual test scripts, cross-browser, mobile, collab, format round-trip, regression.
> Last updated: 2026-03-21

## 1. Test Strategy

### 1.1 Test Pyramid

```
        ┌──────────────┐
        │  E2E (Manual) │   ~50 scenarios
        │  Browser-based │
        ├──────────────┤
        │  Integration   │   ~100 tests
        │  Node.js + WS  │
        ├──────────────┤
        │  Unit Tests    │   ~1,172 tests
        │  Rust (cargo)  │
        └──────────────┘
```

### 1.2 Test Environments

| Environment | Purpose |
|-------------|---------|
| Local development | `cargo test`, `make demo` |
| Docker (`docker compose up`) | Full server + editor stack |
| CI (GitHub Actions) | Automated Rust tests + WASM tests |
| Browser matrix | Manual cross-browser testing |

## 2. Automated Tests

### 2.1 Rust Unit Tests (cargo test)

| Crate | Test Count | Key Coverage |
|-------|-----------|-------------|
| s1-model | 72 | Node types, tree operations, attributes, styles |
| s1-ops | 48 | Operations, transactions, undo/redo, cursor |
| s1-format-txt | 41 | Text read/write, encoding detection |
| s1-format-docx | 201 | DOCX read/write, round-trip, styles, tables, images |
| s1-format-odt | 119 | ODT read/write, round-trip, styles |
| s1-format-md | 32 | Markdown read/write |
| s1-format-pdf | 23 | PDF export, layout to PDF |
| s1-convert | 90 | Format conversion, DOC extraction |
| s1-layout | 62 | Layout engine, pagination, line breaking |
| s1-text | 39 | Text shaping, font loading, BiDi, hyphenation |
| s1engine | 143 | Facade API, builder, format detection |
| s1-crdt | 182 | Fugue text CRDT, tree CRDT, LWW attrs, sync |
| s1engine-wasm | 105 | WASM bindings (wasm-pack test) |
| s1engine-c | 10 | C FFI bindings |
| Property tests | 5 | Model + ops invariants (proptest) |

**Run all:**
```bash
cargo test --workspace
```

### 2.2 WASM Tests (wasm-pack)

```bash
cd ffi/wasm && wasm-pack test --headless --chrome
```

Covers: `WasmEngine`, `WasmDocument`, `WasmDocumentBuilder`, `WasmFontDatabase`, `WasmLayoutConfig`, `detect_format`, `WasmCollabDocument`.

### 2.3 Automated WebSocket Tests (Node.js)

Scripts for testing the collab server:

```bash
node tests/ws/test_connect.js      # Basic WS connect/disconnect
node tests/ws/test_crdt_sync.js    # Two peers, CRDT text sync
node tests/ws/test_fullsync.js     # Full document sync
node tests/ws/test_reconnect.js    # Disconnect and reconnect
node tests/ws/test_version_gap.js  # Version gap detection + recovery
node tests/ws/test_concurrent.js   # 3 peers editing simultaneously
```

| Test | What It Validates |
|------|-------------------|
| `test_connect` | WS handshake, `joined` message, `peer-join`/`peer-leave` |
| `test_crdt_sync` | Peer A types text, peer B receives CRDT op, both converge |
| `test_fullsync` | Structural change triggers fullSync, peer receives and applies |
| `test_reconnect` | Peer disconnects, reconnects, gets catch-up ops from ops_log |
| `test_version_gap` | Drop a message, detect version gap, request and receive fullSync |
| `test_concurrent` | 3 peers type simultaneously in different paragraphs, all converge |

## 3. Manual Test Scenarios

### 3.1 Document Lifecycle

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| M-01 | Create new document | Open editor with no file | Empty document with one paragraph, cursor blinking |
| M-02 | Open DOCX | Upload a Word-created DOCX | Document rendered with formatting preserved |
| M-03 | Open ODT | Upload a LibreOffice ODT | Document rendered with formatting preserved |
| M-04 | Open legacy DOC | Upload a .doc file | Converted to DOCX internally, content displayed |
| M-05 | Export to DOCX | Edit document, click "Download DOCX" | Valid DOCX downloaded, opens in Word |
| M-06 | Export to ODT | Edit document, click "Download ODT" | Valid ODT downloaded, opens in LibreOffice |
| M-07 | Export to PDF | Click "Download PDF" | Valid PDF with correct layout |
| M-08 | Export to TXT | Click "Download TXT" | Plain text, all formatting stripped |
| M-09 | Auto-recovery | Edit document, close tab, reopen | Recovery prompt appears, content restored |
| M-10 | Open very large document (100+ pages) | Upload large DOCX | Document opens (may take 2-3s), virtual scrolling active |

### 3.2 Text Editing

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| M-11 | Type text | Click in paragraph, type | Characters appear at cursor position |
| M-12 | Delete with Backspace | Place cursor, press Backspace | Previous character deleted |
| M-13 | Delete with Delete key | Place cursor, press Delete | Next character deleted |
| M-14 | Enter (split paragraph) | Press Enter mid-paragraph | Two paragraphs, cursor at start of second |
| M-15 | Backspace at paragraph start | Press Backspace at start | Paragraphs merge, cursor at join point |
| M-16 | Select all + delete | Ctrl+A, then Delete | All content removed, single empty paragraph |
| M-17 | Undo typing | Type text, Ctrl+Z | Text removed, cursor restored |
| M-18 | Redo typing | Undo, then Ctrl+Y | Text reappears |
| M-19 | Type in empty paragraph | Click empty line, type | br removed, text appears, no flicker |
| M-20 | Tab key | Press Tab | Tab character or indent applied |

### 3.3 Formatting

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| M-21 | Bold | Select text, click Bold or Ctrl+B | Text becomes bold, selection preserved |
| M-22 | Italic | Select text, click Italic or Ctrl+I | Text becomes italic |
| M-23 | Underline | Select text, click Underline or Ctrl+U | Text becomes underlined |
| M-24 | Heading | Place cursor, select Heading 1 | Paragraph becomes heading |
| M-25 | Alignment | Select paragraph, click Center | Paragraph centered |
| M-26 | Font size | Select text, change font size | Text size changes |
| M-27 | Font family | Select text, change font | Font changes |
| M-28 | Text color | Select text, pick color | Text color changes |
| M-29 | Highlight | Select text, pick highlight | Background color changes |
| M-30 | Bullet list | Click bullet list button | List markers appear |
| M-31 | Numbered list | Click numbered list button | Numbers appear |
| M-32 | Clear formatting | Select text, click clear format | Revert to default style |

### 3.4 Clipboard

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| M-33 | Copy/paste plain text | Copy text, paste | Text pasted at cursor |
| M-34 | Copy/paste formatted text | Copy bold text, paste | Formatting preserved |
| M-35 | Paste from Word | Copy from MS Word, paste | Formatting mostly preserved |
| M-36 | Paste from Google Docs | Copy from GDocs, paste | Basic formatting preserved |
| M-37 | Paste multi-line | Copy multiple paragraphs, paste | Multiple paragraphs created |
| M-38 | Cut all content | Ctrl+A, Ctrl+X | All removed, empty paragraph, clipboard has content |
| M-39 | Paste into empty doc | Ctrl+A, Ctrl+X, Ctrl+V | Content restored |
| M-40 | Paste image from clipboard | Copy image, paste | Image inserted inline |

### 3.5 Tables

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| M-41 | Insert table | Click Insert Table, select 3x3 | 3x3 table appears |
| M-42 | Type in cells | Click cell, type | Text appears in cell |
| M-43 | Tab between cells | Press Tab in cell | Cursor moves to next cell |
| M-44 | Delete table | Select table, press Delete | Table removed |
| M-45 | Table spanning pages | Insert large table | Table splits across pages by row |

### 3.6 Images

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| M-46 | Insert image | Click Insert Image, select file | Image appears inline |
| M-47 | Align image center | Right-click image, Align Center | Image centered in paragraph |
| M-48 | Resize image | Right-click, select Medium | Image resized |
| M-49 | Delete image | Right-click, Delete | Image removed |
| M-50 | Image from DOCX | Open DOCX with images | Images displayed correctly |

## 4. Collaboration Test Scenarios

### 4.1 Basic Collaboration

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| C-01 | Two peers join | Open same document in two tabs | Both see each other's cursor |
| C-02 | Type in different paragraphs | Both type in separate paragraphs | Both see each other's text appear |
| C-03 | Type in same paragraph | Both type in same paragraph | Text interleaves correctly via CRDT |
| C-04 | Format while peer types | A formats, B types | Both see format and text changes |
| C-05 | Peer cursor display | A moves cursor | B sees A's cursor with name label and color |

### 4.2 Structural Changes

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| C-06 | Enter key (split paragraph) | A presses Enter | B sees paragraph split after fullSync |
| C-07 | Insert table | A inserts table | B sees table appear after fullSync |
| C-08 | Paste multi-line | A pastes multiple paragraphs | B sees new paragraphs after fullSync |
| C-09 | Delete paragraph | A deletes a paragraph | B sees paragraph removed after fullSync |

### 4.3 Disconnect/Reconnect

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| C-10 | Peer disconnects | A closes tab | B sees A's cursor disappear |
| C-11 | Peer reconnects | A closes tab, reopens | A gets catch-up ops, both converge |
| C-12 | Offline editing | A disconnects, types, reconnects | A's offline edits sent to B |
| C-13 | Both offline, then reconnect | Both disconnect, both type, both reconnect | CRDT merges both edits |
| C-14 | Version gap recovery | Drop a WebSocket message | Gap detected, fullSync requested, peers converge |

### 4.4 Edge Cases

| # | Scenario | Steps | Expected Result |
|---|----------|-------|-----------------|
| C-15 | 3+ concurrent peers | Open in 3 tabs, all type | All 3 converge |
| C-16 | View-only peer | One peer with `access=view` | Can see edits but cannot type |
| C-17 | Cut all while peer types | A selects all and cuts | B's subsequent edits apply to A's empty doc |
| C-18 | Rapid undo/redo | A types, rapidly Ctrl+Z multiple times | Undo is local, B sees inverse ops |
| C-19 | Large document sync | 100-page doc, fullSync | fullSync completes within 5s |
| C-20 | Tab duplication | Duplicate tab | Both tabs are independent peers |

## 5. Cross-Browser Matrix

### 5.1 Supported Browsers

| Browser | Version | Priority | Notes |
|---------|---------|----------|-------|
| Chrome | Latest | P0 (primary) | Development browser |
| Firefox | Latest | P0 | Second priority |
| Safari | Latest | P1 | macOS/iOS |
| Edge | Latest | P1 | Chromium-based |
| Chrome Android | Latest | P2 | Mobile |
| Safari iOS | Latest | P2 | Mobile |

### 5.2 Browser-Specific Test Focus

| Browser | Focus Areas |
|---------|-------------|
| Chrome | Baseline for all tests |
| Firefox | contentEditable quirks, `beforeinput` event differences |
| Safari | WebSocket behavior, clipboard API limitations, IME handling |
| Edge | Should match Chrome (Chromium-based), verify toolbar rendering |
| Mobile Chrome | Touch events, virtual keyboard, viewport resize |
| Mobile Safari | Touch selection, zoom behavior, iOS keyboard |

### 5.3 Critical Cross-Browser Tests

| # | Test | Why |
|---|------|-----|
| XB-01 | Type text | `beforeinput` event handling varies |
| XB-02 | Paste HTML | Clipboard API differs (HTTPS requirement, format support) |
| XB-03 | Undo/redo | Some browsers handle Ctrl+Z natively vs intercepted |
| XB-04 | Select all | Triple-click behavior varies |
| XB-05 | IME input (CJK) | Composition events handled differently |
| XB-06 | WebSocket reconnect | Connection handling varies |
| XB-07 | WASM loading | WASM compilation and execution |
| XB-08 | Font rendering | Text metrics may differ slightly |
| XB-09 | PDF download | Blob URL handling varies |
| XB-10 | Scroll performance | Virtual scroll IntersectionObserver support |

## 6. Mobile Testing

### 6.1 Touch Interaction Tests

| # | Test | Expected Behavior |
|---|------|-------------------|
| T-01 | Tap to place cursor | Cursor appears at tap position |
| T-02 | Long press to select | Word selected with handles |
| T-03 | Drag selection handles | Selection extends/shrinks |
| T-04 | Pinch to zoom | Document zooms in/out |
| T-05 | Scroll document | Smooth scrolling through pages |
| T-06 | Virtual keyboard appears | Document scrolls to keep cursor visible |
| T-07 | Toolbar buttons | All buttons tappable (44x44px minimum) |

### 6.2 Responsive Layout Tests

| # | Test | Expected Behavior |
|---|------|-------------------|
| R-01 | Portrait phone (<400px) | Toolbar wraps, pages scale to fit |
| R-02 | Landscape phone (~700px) | Full toolbar visible |
| R-03 | Tablet portrait (~768px) | Full layout, pages at readable size |
| R-04 | Tablet landscape (~1024px) | Desktop-like experience |
| R-05 | Orientation change | Layout adjusts smoothly |

## 7. Format Round-Trip Tests

### 7.1 DOCX Round-Trip

| # | Test | Steps | Pass Criteria |
|---|------|-------|---------------|
| RT-01 | Basic text | Open DOCX with paragraphs -> export DOCX -> reopen | Text identical |
| RT-02 | Formatting | DOCX with bold/italic/underline -> export -> reopen | Formatting preserved |
| RT-03 | Tables | DOCX with tables -> export -> reopen | Table structure identical |
| RT-04 | Images | DOCX with images -> export -> reopen | Images present, same dimensions |
| RT-05 | Styles | DOCX with heading styles -> export -> reopen | Heading levels preserved |
| RT-06 | Lists | DOCX with bullet/numbered lists -> export -> reopen | List types and nesting preserved |
| RT-07 | Headers/footers | DOCX with header/footer -> export -> reopen | Header/footer content preserved |
| RT-08 | Page breaks | DOCX with page breaks -> export -> reopen | Page breaks at same positions |
| RT-09 | Track changes | DOCX with tracked changes -> export -> reopen | Changes preserved |
| RT-10 | Comments | DOCX with comments -> export -> reopen | Comments preserved |
| RT-11 | Edit then export | Open DOCX, edit text, export -> open in Word | Edits present, no corruption |
| RT-12 | Word compatibility | Export DOCX, open in MS Word | No warnings, renders correctly |

### 7.2 ODT Round-Trip

| # | Test | Steps | Pass Criteria |
|---|------|-------|---------------|
| RT-13 | Basic text | Open ODT -> export ODT -> reopen | Text identical |
| RT-14 | Formatting | ODT with formatting -> export -> reopen | Formatting preserved |
| RT-15 | Tables | ODT with tables -> export -> reopen | Table structure preserved |
| RT-16 | Images | ODT with images -> export -> reopen | Images preserved |
| RT-17 | Styles | ODT with styles -> export -> reopen | Styles preserved |
| RT-18 | LibreOffice compatibility | Export ODT, open in LibreOffice | No warnings, renders correctly |

### 7.3 Cross-Format Conversion

| # | Test | Steps | Pass Criteria |
|---|------|-------|---------------|
| RT-19 | DOCX to ODT | Open DOCX, export as ODT | Content and basic formatting preserved |
| RT-20 | ODT to DOCX | Open ODT, export as DOCX | Content and basic formatting preserved |
| RT-21 | DOCX to PDF | Open DOCX, export as PDF | Layout resembles original |
| RT-22 | DOCX to TXT | Open DOCX, export as TXT | All text content extracted |
| RT-23 | TXT to DOCX | Open TXT, export as DOCX | Text in paragraphs, no formatting loss |

### 7.4 Automated Round-Trip (Rust)

The Rust test suite includes round-trip tests for each format crate:

```rust
#[test]
fn docx_round_trip() {
    let engine = Engine::new();
    let original = std::fs::read("tests/fixtures/sample.docx").unwrap();
    let doc = engine.open(&original).unwrap();
    let exported = doc.export(Format::Docx).unwrap();
    let doc2 = engine.open(&exported).unwrap();
    assert_eq!(doc.to_plain_text(), doc2.to_plain_text());
    assert_eq!(doc.paragraph_count(), doc2.paragraph_count());
}
```

## 8. Regression Test List

Every bug fix MUST add a corresponding test. Format: `[date] [area] description`.

| # | Date | Area | Bug Description | Test Location |
|---|------|------|-----------------|---------------|
| REG-01 | 2026-03-10 | Cursor | Cursor jumps to top-left after bold | `editor/tests/cursor_test.js` |
| REG-02 | 2026-03-10 | Clipboard | Cut all produces no empty paragraph | `editor/tests/clipboard_test.js` |
| REG-03 | 2026-03-11 | Collab | fullSync with stale version overwrites newer content | `tests/ws/test_version_gap.js` |
| REG-04 | 2026-03-12 | Render | Empty paragraph has zero height (no br) | `editor/tests/render_test.js` |
| REG-05 | 2026-03-12 | DOCX | Missing styles.xml crashes parser | `crates/s1-format-docx/tests/` |
| REG-06 | 2026-03-13 | Images | Image alignment sets wrong property | `editor/tests/image_test.js` |
| REG-07 | 2026-03-13 | Collab | Reconnect sends duplicate offline ops | `tests/ws/test_reconnect.js` |
| REG-08 | 2026-03-14 | Tables | Table cell with no paragraphs crashes | `crates/s1-format-docx/tests/` |
| REG-09 | 2026-03-14 | Selection | Selection lost after toolbar click | `editor/tests/selection_test.js` |
| REG-10 | 2026-03-15 | Undo | Undo after Enter produces orphaned node | `crates/s1-ops/tests/` |
| REG-11 | 2026-03-15 | Pagination | Page map not updated after delete | `editor/tests/pagination_test.js` |
| REG-12 | 2026-03-15 | Admin | Admin session not cleared on logout | `server/tests/admin_test.rs` |

**Policy:** When adding a regression test, add the entry to this table with the date, area, and description.

## 9. Test Data Management

### 9.1 Fixture Files

```
tests/fixtures/
  basic.docx                Simple paragraphs
  formatted.docx            Bold, italic, underline, colors
  tables.docx               Various table configurations
  images.docx               Inline images (PNG, JPEG)
  headers_footers.docx      Headers, footers, page numbers
  lists.docx                Bullet and numbered lists
  styles.docx               Custom styles, heading levels
  track_changes.docx        Insertions and deletions
  comments.docx             Margin comments
  large_100pages.docx       Performance test fixture
  nested_tables.docx        Tables within tables
  cjk_text.docx             Chinese/Japanese/Korean content
  rtl_text.docx             Arabic/Hebrew right-to-left text
  empty.docx                Valid but empty document
  corrupt.docx              Intentionally malformed

  basic.odt                 Simple ODT
  formatted.odt             Formatted ODT
  tables.odt                ODT with tables
  images.odt                ODT with images

  plain.txt                 UTF-8 plain text
  utf16.txt                 UTF-16 encoded text
  markdown.md               Markdown with formatting

  legacy.doc                Legacy Word document
```

### 9.2 Test Data Generation

For performance benchmarks, generate test documents programmatically:

```rust
fn generate_large_docx(pages: usize) -> Vec<u8> {
    let mut builder = DocumentBuilder::new();
    for i in 0..pages * 5 {
        builder = builder.paragraph(|p| {
            p.text(&format!("Paragraph {} with enough text to fill a line. ", i))
        });
    }
    builder.build().export(Format::Docx).unwrap()
}
```

## 10. Test Execution Schedule

| Test Type | When | Duration | Owner |
|-----------|------|----------|-------|
| Rust unit tests | Every commit (CI) | ~60s | Automated |
| WASM tests | Every commit (CI) | ~30s | Automated |
| Clippy + fmt | Every commit (CI) | ~20s | Automated |
| WebSocket integration tests | Pre-release | ~2min | Automated |
| Cross-browser manual tests | Pre-release | ~2hr | Manual |
| Mobile manual tests | Pre-release | ~1hr | Manual |
| Performance benchmarks | Weekly / pre-release | ~5min | Semi-automated |
| Full regression suite | Pre-release | ~30min | Manual + automated |

## 11. Implementation Status

| Test Category | Status |
|---------------|--------|
| Rust unit tests (1,172) | DONE |
| WASM tests (105) | DONE |
| Property-based tests (proptest) | DONE (5 tests) |
| Format round-trip tests (Rust) | DONE |
| Criterion benchmarks | DONE |
| WebSocket integration tests (Node.js) | DONE (32 e2e + 7 CRDT + 56 collab tests) |
| Cross-browser manual test scripts | SPECIFIED BELOW |
| Mobile test scripts | SPECIFIED BELOW |
| Automated E2E (Playwright/Cypress) | SPECIFIED BELOW |
| CI benchmark regression tracking | SPECIFIED BELOW |
| Performance dashboard | SPECIFIED BELOW |

## Cross-Browser Manual Test Script

Execute in Chrome, Firefox, Safari, Edge (latest):

```
1. Open http://localhost:8080
2. Click "New" → empty document appears, cursor blinking at start ✓/✗
3. Type "Hello World" → text appears, no delay ✓/✗
4. Select "Hello" → highlight visible ✓/✗
5. Click Bold → text bolds, selection preserved ✓/✗
6. Ctrl+Z → bold removed ✓/✗
7. Ctrl+A → all text selected ✓/✗
8. Ctrl+X → text removed, clipboard has content ✓/✗
9. Ctrl+V → text pasted back ✓/✗
10. Press Enter → paragraph splits, cursor on new line ✓/✗
11. Click empty line → cursor visible at correct position ✓/✗
12. Upload test.docx via Open → renders with formatting ✓/✗
13. Export as DOCX → download works ✓/✗
14. Export as PDF → download works ✓/✗
15. Click Share → modal opens, URL generated ✓/✗
16. Open share URL in new tab → document loads ✓/✗
17. Type in tab 1 → appears in tab 2 within 2s ✓/✗
18. Type in tab 2 → appears in tab 1 within 2s ✓/✗
```

**Browser-specific checks:**
- Safari: Clipboard API may require user gesture — test cut/copy/paste via keyboard AND context menu
- Firefox: WebSocket binary frames — verify fullSync works
- Edge: IME input (Chinese/Japanese) — verify composition events

## Mobile Test Script

Execute on iOS Safari + Android Chrome:

```
1. Open editor URL → responsive layout, toolbar scrollable ✓/✗
2. Tap in document → keyboard opens, cursor visible ✓/✗
3. Type text → appears, no lag ✓/✗
4. Long press → context menu (cut/copy/paste) ✓/✗
5. Pinch to zoom → editor zoom changes (not browser zoom) ✓/✗
6. Rotate device → layout adapts ✓/✗
7. Find bar → positioned above keyboard, not behind it ✓/✗
8. Upload file → file picker works ✓/✗
```

## Automated E2E (Playwright)

```javascript
// playwright.config.ts
export default defineConfig({
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'mobile-chrome', use: { ...devices['Pixel 5'] } },
    { name: 'mobile-safari', use: { ...devices['iPhone 13'] } },
  ],
  webServer: { command: 'docker compose up -d', url: 'http://localhost:8080' },
});

// tests/editor.spec.ts
test('new document has cursor', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.page-content')).toBeFocused();
});

test('typing inserts text', async ({ page }) => {
  await page.goto('/');
  await page.locator('.page-content').type('Hello World');
  await expect(page.locator('.page-content')).toContainText('Hello World');
});

test('cut and paste', async ({ page }) => {
  await page.goto('/');
  await page.locator('.page-content').type('Test Content');
  await page.keyboard.press('Control+a');
  await page.keyboard.press('Control+x');
  await page.keyboard.press('Control+v');
  await expect(page.locator('.page-content')).toContainText('Test Content');
});

test('collaboration sync', async ({ browser }) => {
  const ctx1 = await browser.newContext();
  const ctx2 = await browser.newContext();
  // Upload doc, get share URL, open in both contexts
  // Type in ctx1, verify appears in ctx2
});
```

## CI Benchmark Regression

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench --output-format=bencher | tee bench-output.txt
- name: Compare with baseline
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: bench-output.txt
    alert-threshold: '110%'  # Alert if >10% regression
    fail-on-alert: true
```

## Performance Dashboard

Server endpoint `GET /admin/api/perf` returns:
```json
{
  "uptime_secs": 3600,
  "active_rooms": 5,
  "total_editors": 12,
  "ops_per_second": 42,
  "avg_sync_latency_ms": 15,
  "memory_mb": 128,
  "wasm_bundle_kb": 450
}
```
Admin dashboard renders as sparkline charts (10-minute rolling window).
