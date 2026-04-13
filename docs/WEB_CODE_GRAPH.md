# Web Integration Code Graph

Last updated: 2026-04-14

## Architecture

```
User (browser)
    │
    ▼
web/index.html ─── patches + boot sequence
    │
    ├── OnlyOffice sdkjs (438 scripts)
    │   ├── word/api.js:845          asc_docs_api constructor
    │   ├── common/apiBase.js:599    asc_setDocInfo
    │   ├── common/apiBase.js:1395   asc_LoadDocument
    │   ├── word/Editor/Document.js  CDocument (Content[], AddToParagraph, AddNewParagraph)
    │   ├── word/Editor/Paragraph.js Paragraph (Content[] of ParaRun)
    │   ├── word/Editor/Run.js       ParaRun (Content[] of CRunText/CRunTab)
    │   └── common/AllFonts.js       Font registry stub
    │
    └── web/adapter.js ─── ES module bridge
        │
        └── web/pkg/s1engine_wasm.js ─── WASM bindings
            │
            └── ffi/wasm/src/lib.rs  ─── Rust WASM API
                │
                └── crates/s1engine/  ─── Document engine
```

## s1engine WASM API (ffi/wasm/src/lib.rs)

| Method | Line | Signature | Returns |
|--------|------|-----------|---------|
| WasmEngine::new() | 28 | `() → Self` | Engine instance |
| WasmEngine::create() | 35 | `() → WasmDocument` | Empty document |
| WasmEngine::open(data) | 51 | `(&[u8]) → Result<WasmDocument>` | Parsed document |
| WasmDocument::to_plain_text() | 293 | `() → Result<String>` | All text, \n between paragraphs |
| WasmDocument::to_html() | 1113 | `() → Result<String>` | Full HTML with styles |
| WasmDocument::body_children_json() | 1366 | `() → Result<String>` | `[{"id":"0:5","type":"Paragraph"},...]` |
| WasmDocument::node_info_json(id) | 1388 | `(&str) → Result<String>` | See node_info format below |
| WasmDocument::get_paragraph_text(id) | 1398 | `(&str) → Result<String>` | Paragraph text only |
| WasmDocument::export(fmt) | 302 | `(&str) → Result<Vec<u8>>` | DOCX/ODT/PDF/TXT/MD bytes |
| WasmDocument::append_paragraph(text) | 1409 | `(&str) → Result<String>` | New paragraph node ID |
| WasmDocument::metadata_json() | 322 | `() → Result<String>` | `{"title":"...","author":"..."}` |

### node_info_json format (built at lib.rs:10973-11043)

Fields are TOP-LEVEL, not inside `attributes`:
```json
{
  "id": "0:5",
  "type": "Paragraph",
  "fullText": "Hello world",
  "bold": true,
  "italic": true,
  "underline": true,
  "strikethrough": true,
  "fontSize": 12.0,
  "fontFamily": "Arial",
  "alignment": "center",
  "styleId": "Heading1",
  "children": ["0:6", "0:7"]
}
```

## OnlyOffice sdkjs API

### Document Model Hierarchy

```
api.WordControl.m_oLogicDocument (CDocument)
  └── Content[] (array of Paragraph)
        └── Content[] (array of ParaRun)
              └── Content[] (array of CRunText/CRunTab/CRunBreak)
                    └── Value (charCode, for CRunText)
```

### Key Methods

| Class | Method | File:Line | Purpose |
|-------|--------|-----------|---------|
| CDocument | AddToParagraph(item) | Document.js:6209 | Insert at cursor |
| CDocument | AddNewParagraph(bRecalc) | Document.js:5647 | New paragraph |
| CDocument | GetSelectedText(false, opts) | Document.js:11762 | All text (no selection needed) |
| CDocument | SelectAll() | Document.js | Select everything |
| CDocument | Remove(1, true, false, true) | Document.js | Delete selection |
| CDocument | Recalculate() | Document.js | Re-layout and paint |
| CDocument | TurnOff_Recalculate() | Document.js:2781 | Disable recalc |
| CDocument | TurnOn_Recalculate(bRecalc) | Document.js:2789 | Re-enable recalc |
| Paragraph | GetText({ParaSeparator:''}) | Paragraph.js:16730 | Paragraph text |
| ParaRun | Content[] | Run.js | Array of run content |
| asc_docs_api | put_TextPrBold(v) | api.js:3515 | Toggle bold |
| asc_docs_api | put_TextPrItalic(v) | api.js:3526 | Toggle italic |
| asc_docs_api | put_TextPrFontName(n) | api.js:3480 | Set font |
| asc_docs_api | put_TextPrFontSize(s) | api.js:3503 | Set size |
| asc_docs_api | put_PrAlign(v) | api.js:4299 | Set alignment |
| asc_docs_api | put_Style(name) | api.js:4612 | Set paragraph style |

### Class Exports

| Class | Export | File |
|-------|--------|------|
| Paragraph | `AscWord.Paragraph` | Paragraph.js:21091 |
| ParaRun | `AscWord.ParaRun` | Run.js:12994 |
| CRunText | `AscWord.CRunText` | Text.js:770 |
| CRunTab | `AscWord.CRunTab` | Tab.js:207 |

### Font System Chain

```
AllFonts.js ── sets window.__fonts_files, window.__fonts_infos
    │
    ▼
Externals.js:631 ── checkAllFonts() builds g_font_files, g_font_infos, g_map_font_index
    │
    ▼
Externals.js:79 ── LoadFontArrayBuffer() XHR + ODTTF XOR at lines 130-137
                    GUID: [0xA0, 0x66, 0xD6, 0x20, 0x14, 0x96, 0x47, 0xfa,
                           0x95, 0x69, 0xB8, 0x50, 0xB0, 0x41, 0x49, 0x48]
    │
    ▼
map.js:2925 ── GetFontFileWeb(name) ── PATCHED in index.html:52 to use g_map_font_index
    │
    ▼
GlobalLoaders.js:50 ── CGlobalFontLoader.fontFilesPath = "../../../../fonts/"
                        Resolves to /fonts/ from browser
```

### Boot Sequence (index.html)

1. `AscNotLoadAllScript = true` — skip sdk-all.js loading
2. Load vendor: polyfill.js, jquery, xregexp
3. Load 438 sdkjs scripts via document.write
4. Load editor.js (getEmpty() for blank doc)
5. Patch GetFontFileWeb — bypass empty font selection list
6. Patch openDocument — fix DOCY string signature check
7. Create asc_docs_api({id-view: 'editor_sdk'})
8. Wire CoAuth offline callbacks
9. asc_setDocInfo(offline) + asc_LoadDocument()

## Patches in index.html (lines 50-80)

| Patch | Line | Problem | Fix |
|-------|------|---------|-----|
| Font lookup | 52-68 | g_fonts_selection_bin="" → all fonts resolve to ASCW3 | Override GetFontFileWeb to use g_map_font_index |
| DOCY signature | 72-78 | checkStreamSignature compares char vs charCode for strings | Set bSerFormat=true when data starts with "DOCY" |
| CoAuth callbacks | 87-90 | onFirstLoadChangesEnd undefined in offline mode | Wire to asyncServerIdEndLoaded |
| Underline | 95 | Empty doc template has underline in default style | put_TextPrUnderline(false) on ready |

## Font Files (web/fonts/)

TTF files pre-XOR'd with ODTTF GUID so sdkjs deobfuscation produces correct bytes.
Must run XOR script after copying new font files.
