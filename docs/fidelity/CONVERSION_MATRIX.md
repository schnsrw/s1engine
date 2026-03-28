# End-to-End Conversion Matrix

## Supported Formats

| Format | Extension | Standard | Read | Write |
|--------|-----------|----------|------|-------|
| DOCX | `.docx` | ECMA-376 / ISO 29500 (Office Open XML) | YES | YES |
| ODT | `.odt` | OASIS ODF 1.2 (ISO/IEC 26300) | YES | YES |
| PDF | `.pdf` | ISO 32000 | NO | YES (export only) |
| TXT | `.txt` | Plain text (UTF-8/16, Latin-1) | YES | YES |
| Markdown | `.md` | CommonMark + GFM extensions | YES | YES |
| DOC | `.doc` | MS-CFB (legacy binary) | YES (basic) | NO |
| CSV | `.csv` | RFC 4180 | YES | YES |

> **XLSX** (`s1-format-xlsx` crate) exists but is **NOT integrated** into the engine facade.
> It can be cleanly separated — zero coupling with core crates.

---

## Full FROM -> TO Conversion Matrix

All conversions flow through `DocumentModel` (hub-and-spoke architecture):
`SourceFormat -> DocumentModel -> TargetFormat`

### Fidelity Levels

| Level | Symbol | Meaning |
|-------|--------|---------|
| Full | `++++` | All content and formatting preserved. Round-trip safe. |
| High | `+++.` | Most features preserved. Minor losses (rare edge cases). |
| Partial | `++..` | Core content + basic formatting. Some features lost. |
| Basic | `+...` | Text content preserved. Most formatting lost. |
| Lossy | `....` | Significant structural/content loss. |
| N/A | `----` | Conversion not possible. |

### Matrix

```
FROM ╲ TO │  DOCX    ODT     PDF     TXT     MD      CSV
──────────┼──────────────────────────────────────────────────
DOCX      │   —      +++.    ++++    +...    ++..    ....
ODT       │  +++.     —      ++++    +...    ++..    ....
PDF       │  ----    ----     —      ----    ----    ----
TXT       │  ++++    ++++    ++++     —      ++++    ....
MD        │  ++++    ++++    ++++    +...     —      ....
DOC       │  ++..    ++..    ++..    +...    +...    ....
CSV       │  ++++    ++++    ++++    +...    ++..     —
```

---

## Detailed Conversion Path Analysis

### DOCX -> ODT (`+++.` High Fidelity)

**Preserved:**
- All text content, paragraphs, runs
- Run formatting: bold, italic, underline (6 styles), strikethrough, font, size, color, highlight, super/subscript, letter spacing
- Paragraph formatting: alignment, spacing before/after, line spacing, indentation, borders, background, tab stops
- Tables: rows, cells, borders, shading, cell width, vertical alignment, cell merge (gridSpan/vMerge)
- Lists: numbered/bulleted with nesting via ListInfo
- Styles: paragraph and character styles with inheritance
- Images: embedded with dimensions
- Headers/footers: per-section with page number/count fields
- Comments (annotations): author, date, content
- Footnotes/endnotes: content preserved
- Bookmarks: start/end pairs
- Metadata: title, author, subject, keywords, created/modified dates

**Lost in translation (DOCX features not in ODT model):**
- Track changes: DOCX stores structured revisions; ODT round-trip stores raw XML
- Content controls (SDTs): checkbox, dropdown, text fields — not mapped to ODF form controls
- VML shapes: preserved as raw XML in DOCX, dropped in ODT
- SmartArt/Charts: preserved in DOCX ZIP, no ODF equivalent
- Advanced number formats: only 6 of 60+ OOXML number formats mapped
- Theme colors/fonts: DOCX theme references resolved to literals, not re-created in ODF
- Compatibility settings: 57+ DOCX-specific settings lost
- Complex field codes: only PAGE, NUMPAGES mapped; MERGEFIELD, IF, REF, etc. dropped

### ODT -> DOCX (`+++.` High Fidelity)

**Preserved:** Same as DOCX->ODT above (bidirectional through DocumentModel)

**Lost in translation (ODT features not in DOCX model):**
- ODF automatic styles with complex selectors
- ODF draw:custom-shape elements (dropped — ODT-11 WONTFIX)
- ODF text:database-display fields (stored as Custom field)
- ODF ruby annotations (furigana)
- ODF section properties (text:section with column layout)

### DOCX -> PDF (`++++` Full Visual Fidelity)

**Path:** DOCX -> DocumentModel -> s1-layout (pagination) -> s1-format-pdf

**Preserved:**
- Complete visual reproduction via layout engine
- Text positioning with rustybuzz shaping
- Font embedding with subsetting (TrueType)
- Images: PNG, JPEG embedded
- Tables: cell layout with borders
- Page layout: margins, headers/footers, page numbers
- Metadata in PDF Info dictionary
- PDF/A-1b archival compliance (optional)

**Limitations:**
- Interactive elements (hyperlinks) — partially supported
- Form fields — not interactive in PDF
- Comments — not rendered as PDF annotations
- Bookmarks — supported
- Dynamic fields — computed at layout time, not live

### DOCX -> TXT (`+...` Basic)

**Preserved:**
- Plain text content (all paragraphs)
- Paragraph boundaries (newlines)
- List prefix rendering: `- ` for bullets, `1. ` for numbered
- Heading markers: `# `, `## `, etc.
- Table content: tab-separated cells
- Page breaks: `---` thematic break

**Lost:**
- ALL formatting (bold, italic, color, font, size, etc.)
- Images
- Headers/footers
- Comments, footnotes, endnotes
- Styles
- Table structure (flattened to TSV-like text)
- Page layout

### DOCX -> Markdown (`++..` Partial)

**Preserved:**
- Headings (ATX syntax: `#`, `##`, etc.)
- Paragraphs
- Bold (`**text**`), italic (`*text*`)
- Strikethrough (`~~text~~` — GFM)
- Lists: ordered/unordered with nesting
- Hyperlinks: `[text](url)`
- Code blocks (fenced)
- Tables (GFM extension)

**Lost:**
- Font family, size, color, highlight
- Underline (no Markdown equivalent)
- Images (no inline data URI support)
- Headers/footers
- Comments, footnotes, endnotes
- Page layout, sections
- Complex table formatting (borders, merged cells, widths)
- Styles (flattened to inline formatting)

### DOCX -> CSV (`....` Lossy)

**Preserved:**
- Table cell text content
- Row/column structure

**Lost:**
- ALL formatting
- Non-table content (paragraphs, headings, images)
- Multiple tables (all extracted, separated by blank lines)
- Table formatting (borders, widths, merged cells)
- Everything except raw cell text

### DOC -> DOCX (`++..` Partial)

**Path:** DOC (OLE2 binary) -> doc_reader -> DocumentModel -> DOCX writer

**Preserved:**
- Text content via piece table extraction
- Character formatting: bold, italic, font size, color, underline, strikethrough, superscript/subscript
- Paragraph breaks
- Basic table structure (detected from cell marks 0x07)
- Metadata: title, author, subject, keywords, dates (from SummaryInformation stream)

**Lost:**
- Images (OLE2 embedded pictures not extracted)
- Headers/footers
- Complex styles (only inline formatting preserved)
- Complex tables (merging, borders, widths)
- Comments, footnotes, endnotes
- Macros (VBA not preserved)
- OLE objects

### CSV -> DOCX (`++++` Full)

**Path:** CSV -> csv_parser -> DocumentModel (single table) -> DOCX writer

**Preserved:**
- Complete table structure (rows x columns)
- Cell text content
- Auto-detected delimiters (comma, semicolon, tab)
- Quoted field handling (RFC 4180)

### TXT -> Any Format (`++++` Full for text content)

**Path:** TXT -> encoding detection -> DocumentModel (paragraphs) -> target writer

**Encoding detection priority:**
1. UTF-8 BOM (EF BB BF)
2. UTF-16 LE BOM (FF FE)
3. UTF-16 BE BOM (FE FF)
4. Valid UTF-8 (no BOM)
5. Latin-1 fallback (ISO 8859-1)

---

## Feature Support Cross-Reference

Which document features survive each conversion path:

| Feature | DOCX->ODT | ODT->DOCX | DOCX->PDF | DOCX->TXT | DOCX->MD | DOC->DOCX |
|---------|-----------|-----------|-----------|-----------|----------|-----------|
| **Text content** | YES | YES | YES | YES | YES | YES |
| **Bold/Italic** | YES | YES | YES | no | YES | YES |
| **Underline** | YES (6 styles) | YES | YES | no | no | YES |
| **Strikethrough** | YES | YES | YES | no | YES (GFM) | YES |
| **Font family** | YES | YES | YES | no | no | YES |
| **Font size** | YES | YES | YES | no | no | YES |
| **Text color** | YES | YES | YES | no | no | YES |
| **Highlight** | YES | YES | YES | no | no | no |
| **Super/subscript** | YES | YES | YES | no | no | YES |
| **Letter spacing** | YES | YES | YES | no | no | no |
| **Alignment** | YES | YES | YES | no | no | no |
| **Spacing before/after** | YES | YES | YES | no | no | no |
| **Line spacing** | YES | YES | YES | no | no | no |
| **Indentation** | YES | YES | YES | no | no | no |
| **Para borders** | YES | YES | YES | no | no | no |
| **Tab stops** | YES | YES | YES | no | no | no |
| **Lists (basic)** | YES | YES | YES | prefix | YES | no |
| **List numbering** | YES | YES | YES | prefix | YES | no |
| **Tables** | YES | YES | YES | TSV | GFM | basic |
| **Table borders** | YES | YES | YES | no | no | no |
| **Cell merge** | YES | YES | YES | no | no | no |
| **Cell shading** | YES | YES | YES | no | no | no |
| **Images** | YES | YES | YES | no | no | no |
| **Image positioning** | YES | YES | YES | no | no | no |
| **Headers/footers** | YES | YES | YES | no | no | no |
| **Styles** | YES | YES | YES | no | no | no |
| **Hyperlinks** | YES | YES | partial | no | YES | no |
| **Bookmarks** | YES | YES | YES | no | no | no |
| **Comments** | YES | YES | no | no | no | no |
| **Footnotes** | YES | YES | YES | no | no | no |
| **Endnotes** | YES | YES | YES | no | no | no |
| **Track changes** | raw XML | raw XML | no | no | no | no |
| **Fields (page#)** | YES | YES | YES | no | no | no |
| **Fields (advanced)** | no | no | no | no | no | no |
| **Content controls** | no | no | no | no | no | no |
| **Metadata** | YES | YES | YES | no | no | YES |
| **Page layout** | YES | YES | YES | no | no | no |
| **Sections** | YES | YES | YES | no | no | no |
| **Math (OMML)** | no | no | no | no | no | no |
| **VML shapes** | no | no | no | no | no | no |
| **SmartArt** | preserved | no | no | no | no | no |
| **Charts** | preserved | no | no | no | no | no |

---

## Architecture Diagram

```
                    ┌──────────────┐
         ┌────────>│              │────────┐
         │         │  s1-model    │        │
         │    ┌───>│ DocumentModel│<───┐   │
         │    │    │              │    │   │
         │    │    └──────────────┘    │   │
         │    │           │            │   │
         │    │           v            │   │
         │    │    ┌──────────────┐    │   │
         │    │    │   s1-ops     │    │   │
         │    │    │ Operations   │    │   │
         │    │    │ Undo/Redo    │    │   │
         │    │    └──────────────┘    │   │
         │    │                        │   │
    ┌────┴──┐ │  ┌──────┐ ┌────────┐ │  ┌┴───────┐
    │ DOCX  │ │  │ ODT  │ │  TXT   │ │  │  MD    │
    │reader │ │  │reader│ │ reader │ │  │ reader │
    │writer │ │  │writer│ │ writer │ │  │ writer │
    └───────┘ │  └──────┘ └────────┘ │  └────────┘
              │                       │
    ┌─────────┴──┐            ┌──────┴───────┐
    │ DOC reader │            │ CSV parser   │
    │ (s1-convert│            │ (s1-convert) │
    │  legacy)   │            └──────────────┘
    └────────────┘
                       │
                       v
              ┌──────────────┐
              │  s1-layout   │
              │  pagination  │
              │  line break  │
              └──────┬───────┘
                     │
                     v
              ┌──────────────┐
              │ s1-format-pdf│
              │  PDF export  │
              │  PDF/A-1b    │
              └──────────────┘
```

All paths go through DocumentModel. No format-to-format shortcuts exist.
