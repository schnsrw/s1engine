# DOCX Format Fidelity Specification

## Reference Standard

**ECMA-376: Office Open XML File Formats** (5th Edition, 2021)
- Part 1: Fundamentals and Markup Language Reference
- Part 4: Transitional Migration Features

s1engine targets ECMA-376 Transitional conformance, which is the most widely used OOXML profile.

## Implementation Overview

The DOCX format crate (`s1-format-docx`) provides full read/write round-trip capability for the OOXML word-processing document format. The implementation uses `quick_xml` for XML parsing and `zip` for archive handling. All parsing is lenient (unknown elements are skipped with debug warnings), while writing produces strict, valid output.

### ZIP Structure

| Entry | Read | Write | Notes |
|-------|------|-------|-------|
| `[Content_Types].xml` | Full | Full | Dynamic based on document content |
| `_rels/.rels` | Full | Full | |
| `word/document.xml` | Full | Full | Main document body |
| `word/styles.xml` | Full | Full | Style definitions |
| `word/numbering.xml` | Full | Full | List/numbering definitions |
| `word/comments.xml` | Full | Full | Comment bodies |
| `word/footnotes.xml` | Full | Full | Footnote bodies |
| `word/endnotes.xml` | Full | Full | Endnote bodies |
| `word/header*.xml` | Full | Full | Per-section headers |
| `word/footer*.xml` | Full | Full | Per-section footers |
| `docProps/core.xml` | Full | Full | Dublin Core metadata |
| `word/_rels/document.xml.rels` | Full | Full | Relationship resolution |
| `word/media/*` | Full | Full | Embedded images |
| `_xmlsignatures/*` | Preserved | Preserved | Digital signatures (round-trip) |
| `customXml/*` | Preserved | Preserved | Custom XML parts (round-trip) |
| `word/diagrams/*` | Preserved | Preserved | SmartArt diagrams (round-trip) |
| `word/charts/*` | Preserved | Preserved | Chart objects (round-trip) |
| `word/embeddings/*` | Preserved | Preserved | OLE embeddings (round-trip) |
| `word/vbaProject.bin` | Preserved | Preserved | VBA macro storage (round-trip) |
| `word/vbaData.xml` | Preserved | Preserved | VBA metadata (round-trip) |
| `word/settings.xml` | Not read | Not written | Document-level settings |
| `word/fontTable.xml` | Not read | Not written | Font table |
| `word/webSettings.xml` | Not read | Not written | Web view settings |
| `word/theme/theme1.xml` | Not read | Not written | Theme definitions |
| `docProps/app.xml` | Not read | Not written | Application metadata |

---

## Feature Matrix

### Legend

| Symbol | Meaning |
|--------|---------|
| Full | Fully supported with semantic model representation |
| Partial | Supported with limitations (see notes) |
| Preserved | Round-tripped as raw XML/binary, no semantic access |
| None | Not supported; silently dropped |

---

### 1. Paragraphs

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Basic paragraphs (`w:p`) | Full | Full | Full | Full | |
| Paragraph properties (`w:pPr`) | Full | Full | Full | Full | |
| Style reference (`w:pStyle`) | Full | Full | Full | Full | StyleId attribute |
| Alignment (`w:jc`) | Full | Full | Full | Full | Left, Center, Right, Justify |
| Indentation (`w:ind`) | Full | Full | Full | Full | Left, Right, FirstLine, Hanging |
| Spacing before/after (`w:spacing`) | Full | Full | Full | Full | In twips, converted to points |
| Line spacing | Full | Full | Full | Full | Single, 1.5, Double, Exact, AtLeast, Multiple |
| Keep with next (`w:keepNext`) | Full | Full | Full | Full | |
| Keep lines together (`w:keepLines`) | Full | Full | Full | Full | |
| Page break before (`w:pageBreakBefore`) | Full | Full | Full | Full | |
| Paragraph borders (`w:pBdr`) | Full | Full | Full | Full | Top, Bottom, Left, Right |
| Background/shading (`w:shd`) | Full | Full | Full | Full | |
| Tab stops (`w:tabs`) | Full | Full | Full | Full | Left, Center, Right, Decimal; Dot, Dash, Underscore leaders |
| Contextual spacing | Full | Full | Full | Full | |
| Word wrap (East Asian) | Full | Full | Full | Full | |
| Suppress auto hyphens | Full | Full | Full | Full | |
| BiDi direction | Full | Full | Full | Full | |
| Outline level (`w:outlineLvl`) | None | None | None | None | Heading level inferred from style |
| Paragraph numbering override | Full | Full | Full | Full | Via numPr/numId/ilvl |
| Widow/Orphan control | None | None | None | None | Not modeled |

### 2. Runs (Character Formatting)

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Basic runs (`w:r`) | Full | Full | Full | Full | |
| Run properties (`w:rPr`) | Full | Full | Full | Full | |
| Character style (`w:rStyle`) | Full | Full | Full | Full | |
| Bold (`w:b`) | Full | Full | Full | Full | Toggle property |
| Italic (`w:i`) | Full | Full | Full | Full | Toggle property |
| Underline (`w:u`) | Full | Full | Full | Full | Single, Double, Thick, Dotted, Dashed, Wave |
| Strikethrough (`w:strike`) | Full | Full | Full | Full | |
| Double strikethrough (`w:dstrike`) | None | None | None | None | |
| Font family (`w:rFonts`) | Full | Full | Full | Full | ascii, hAnsi, cs priority |
| Font size (`w:sz`) | Full | Full | Full | Full | Half-points to points conversion |
| Font color (`w:color`) | Full | Full | Full | Full | Hex color |
| Highlight color (`w:highlight`) | Full | Full | Full | Full | Named colors mapped to hex |
| Superscript (`w:vertAlign val="superscript"`) | Full | Full | Full | Full | |
| Subscript (`w:vertAlign val="subscript"`) | Full | Full | Full | Full | |
| Character spacing (`w:spacing`) | Full | Full | Full | Full | |
| Language (`w:lang`) | Full | Full | Full | Full | |
| Text shadow (`w14:shadow`) | Partial | Partial | Preserved | Partial | Stored as string attribute |
| Text outline (`w14:textOutline`) | Partial | Partial | Preserved | Partial | Stored as string attribute |
| Text glow (`w14:glow`) | Partial | Partial | Preserved | Partial | Stored as string attribute |
| Text reflection (`w14:reflection`) | Partial | Partial | Preserved | Partial | Stored as string attribute |
| Small caps (`w:smallCaps`) | None | None | None | None | |
| All caps (`w:caps`) | None | None | None | None | |
| Emboss/Imprint | None | None | None | None | |
| Text effects (w14) | Preserved | Preserved | Preserved | Preserved | Raw XML round-trip |

### 3. Text Content

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Plain text (`w:t`) | Full | Full | Full | Full | Whitespace preservation handled |
| Line break (`w:br`) | Full | Full | Full | Full | |
| Page break (`w:br w:type="page"`) | Full | Full | Full | Full | |
| Column break (`w:br w:type="column"`) | Full | Full | Full | Full | |
| Tab character (`w:tab`) | Full | Full | Full | Full | |
| Soft hyphen | None | None | None | None | |
| Non-breaking space | Full | Full | Full | Full | Via `w:t` with preserved space |
| Carriage return | Full | Full | Full | Full | |

### 4. Lists / Numbering

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Abstract numbering definitions | Full | Full | Full | Full | `word/numbering.xml` |
| Numbering instances (`w:num`) | Full | Full | Full | Full | |
| Multi-level lists (0-8) | Full | Full | Full | Full | |
| Bullet lists | Full | Full | Full | Full | |
| Decimal numbering | Full | Full | Full | Full | |
| Lower/Upper alpha | Full | Full | Full | Full | |
| Lower/Upper roman | Full | Full | Full | Full | |
| List level override | Full | Full | Full | Full | numId + ilvl on paragraph |
| Start number override | Full | Full | Full | Full | |
| Custom bullet characters | Partial | Partial | Partial | Partial | Format string not fully parsed; type inferred |
| List style reference | Full | Full | Full | Full | |
| Restart numbering | Full | Full | Full | Full | |
| Number format string (`w:lvlText`) | Partial | Partial | Partial | Partial | Stored but not fully interpreted |

### 5. Tables

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Basic tables (`w:tbl`) | Full | Full | Full | Full | |
| Table properties (`w:tblPr`) | Full | Full | Full | Full | |
| Table width | Full | Full | Full | Full | Auto, Fixed, Percent |
| Table alignment | Full | Full | Full | Full | |
| Table borders | Full | Full | Full | Full | All border styles |
| Table rows (`w:tr`) | Full | Full | Full | Full | |
| Header rows (`w:tblHeader`) | Full | Full | Full | Full | Repeating on page breaks |
| Row height | Partial | Partial | Partial | Partial | Parsed but not always written back |
| Table cells (`w:tc`) | Full | Full | Full | Full | |
| Cell width | Full | Full | Full | Full | |
| Cell vertical alignment | Full | Full | Full | Full | Top, Center, Bottom |
| Cell borders | Full | Full | Full | Full | |
| Cell background/shading | Full | Full | Full | Full | |
| Column span (`w:gridSpan`) | Full | Full | Full | Full | |
| Row span (`w:vMerge`) | Full | Full | Full | Full | |
| Column widths (`w:tblGrid`) | Full | Full | Full | Full | Via TableColumnWidths attribute |
| Nested tables | Full | Full | Full | Full | Tables inside cells |
| Cell margins | Full | Full | Full | Full | |
| Table indentation | Partial | Partial | Partial | Partial | |
| Table conditional formatting | None | None | None | None | Band/first/last row/col styles |
| Table look (`w:tblLook`) | None | None | None | None | |

### 6. Images

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Inline images (`wp:inline`) | Full | Full | Full | Full | DrawingML |
| Floating images (`wp:anchor`) | Full | Full | Full | Full | DrawingML |
| Image dimensions | Full | Full | Full | Full | EMU to points |
| Alt text | Full | Full | Full | Full | |
| Image positioning (inline vs anchor) | Full | Full | Full | Full | |
| Text wrapping (square, tight, etc.) | Full | Full | Full | Full | 6 wrap styles |
| Horizontal/vertical offset | Full | Full | Full | Full | EMU precision |
| Relative positioning (column, page, etc.) | Full | Full | Full | Full | |
| Distance from text | Full | Full | Full | Full | |
| Image formats (PNG, JPEG, GIF, BMP, TIFF, SVG, WMF, EMF) | Full | Full | Full | Full | Via content type mapping |
| Linked (external) images | None | None | None | None | Only embedded |
| Image effects (crop, rotate, etc.) | None | None | None | None | |
| VML images (`v:imagedata`) | Partial | None | None | Partial | Read via AlternateContent fallback |

### 7. Headers and Footers

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Default header/footer | Full | Full | Full | Full | |
| First page header/footer | Full | Full | Full | Full | `w:titlePg` |
| Even page header/footer | Full | Full | Full | Full | `w:evenAndOddHeaders` |
| Per-section headers/footers | Full | Full | Full | Full | |
| Header/footer with paragraphs | Full | Full | Full | Full | |
| Header/footer with images | Full | Full | Full | Full | |
| Header/footer with tables | Full | Full | Full | Full | |
| Page number fields in headers/footers | Full | Full | Full | Full | Both simple and complex fields |
| Header/footer distance | Full | Full | Full | Full | |
| Images in headers/footers | Full | Full | Full | Full | With own relationship files |

### 8. Sections

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Section properties (`w:sectPr`) | Full | Full | Full | Full | |
| Page size (`w:pgSz`) | Full | Full | Full | Full | Width, height in twips |
| Page orientation | Full | Full | Full | Full | Portrait/Landscape |
| Page margins (`w:pgMar`) | Full | Full | Full | Full | All six margins + header/footer distance |
| Columns (`w:cols`) | Full | Full | Full | Full | Count, spacing, equal width |
| Section break types | Full | Full | Full | Full | NextPage, Continuous, EvenPage, OddPage |
| Title page flag | Full | Full | Full | Full | For first-page header/footer |
| Even/odd headers flag | Full | Full | Full | Full | |
| Multiple sections per document | Full | Full | Full | Full | Inline sectPr in last paragraph of each section |
| Page borders (`w:pgBorders`) | None | None | None | None | Skipped with debug note |
| Line numbering (`w:lnNumType`) | None | None | None | None | Skipped with debug note |
| Document grid (`w:docGrid`) | None | None | None | None | Skipped with debug note |
| Vertical alignment (`w:vAlign`) | None | None | None | None | Skipped with debug note |

### 9. Comments

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Comment definitions (`w:comments`) | Full | Full | Full | Full | |
| Comment author | Full | Full | Full | Full | |
| Comment date | Full | Full | Full | Full | ISO 8601 |
| Comment text (multi-paragraph) | Full | Full | Full | Full | |
| Comment range markers | Full | Full | Full | Full | `commentRangeStart`/`commentRangeEnd` |
| Comment reference (`commentReference`) | Full | Full | Full | Full | |
| Threaded replies (`w:paraId`-based) | Partial | Partial | Partial | Partial | CommentParentId attribute available but not fully resolved |
| Resolved comments | None | None | None | None | w15:commentEx |

### 10. Footnotes and Endnotes

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Footnote definitions | Full | Full | Full | Full | |
| Footnote references (inline) | Full | Full | Full | Full | FootnoteRef node type |
| Footnote numbering | Full | Full | Full | Full | Auto-assigned |
| Multi-paragraph footnotes | Full | Full | Full | Full | |
| Separator/continuation footnotes | Full | Full | Full | Full | id=0 and id=-1 skipped correctly |
| Endnote definitions | Full | Full | Full | Full | |
| Endnote references (inline) | Full | Full | Full | Full | EndnoteRef node type |
| Custom footnote marks | None | None | None | None | |
| Footnote/endnote properties | None | None | None | None | Numbering format, position |

### 11. Bookmarks

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Bookmark start (`w:bookmarkStart`) | Full | Full | Full | Full | Both self-closing and paired |
| Bookmark end (`w:bookmarkEnd`) | Full | Full | Full | Full | |
| Bookmark name | Full | Full | Full | Full | |
| Bookmark ID | Partial | Partial | Partial | Partial | ID read but name is primary identifier |
| Cross-references to bookmarks | None | None | None | None | Complex field instructions |

### 12. Track Changes

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Insertion tracking (`w:ins`) | Full | Full | Full | Full | Block and inline level |
| Deletion tracking (`w:del`) | Full | Full | Full | Full | Block and inline level |
| Move tracking (`w:moveTo`/`w:moveFrom`) | Full | Full | Full | Full | Block and inline level |
| Revision author | Full | Full | Full | Full | |
| Revision date | Full | Full | Full | Full | ISO 8601 |
| Revision ID | Full | Full | Full | Full | |
| Format change tracking (`w:rPrChange`) | Partial | Partial | Preserved | Partial | RevisionOriginalFormatting as string |
| Table structure changes | None | None | None | None | `w:tblPrChange`, `w:trPrChange`, etc. |
| Section property changes | None | None | None | None | `w:sectPrChange` |
| Accept/reject operations | None | None | None | None | Consumer responsibility |

### 13. Equations

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Inline equations (`m:oMath`) | Full | Full | Full | Full | Raw OMML XML preserved |
| Display equations (`m:oMathPara`) | Full | Full | Full | Full | Raw OMML XML preserved |
| Equation source preservation | Full | Full | Full | Full | EquationSource attribute |
| LaTeX conversion | None | None | N/A | None | Raw OMML only |
| Equation editing | None | None | N/A | None | Opaque XML blob |

### 14. Shapes and Drawings

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Shape type (rect, ellipse, etc.) | Full | Full | Full | Full | ShapeType attribute |
| Shape dimensions | Full | Full | Full | Full | ShapeWidth, ShapeHeight |
| Shape fill color | Full | Full | Full | Full | |
| Shape stroke color/width | Full | Full | Full | Full | |
| Raw VML/DrawingML preservation | Full | Full | Full | Full | ShapeRawXml attribute |
| mc:AlternateContent handling | Full | Full | Full | Full | Fallback to VML when DrawingML unavailable |
| SmartArt | Preserved | Preserved | Preserved | Preserved | `word/diagrams/*` round-tripped |
| Charts | Preserved | Preserved | Preserved | Preserved | `word/charts/*` round-tripped |
| WordArt | None | None | None | None | |
| 3D effects | None | None | None | None | |
| Group shapes | None | None | None | None | |

### 15. Form Controls

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Structured document tags (SDT) | Partial | Partial | Partial | Partial | TOC SDTs parsed; others via RawXml |
| Checkbox (`w14:checkbox`) | Preserved | Preserved | Preserved | Preserved | Via FormType/FormChecked attributes |
| Dropdown (`w:dropDownList`) | Preserved | Preserved | Preserved | Preserved | Via FormType/FormOptions attributes |
| Text input | Preserved | Preserved | Preserved | Preserved | Via FormType attribute |
| Content control properties | None | None | None | None | |
| Legacy form fields | None | None | None | None | `w:fldChar`-based |

### 16. Fields

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Simple fields (`w:fldSimple`) | Full | Full | Full | Full | |
| Complex fields (`w:fldChar`) | Full | Full | Full | Full | begin/separate/end sequence |
| PAGE field | Full | Full | Full | Full | |
| NUMPAGES field | Full | Full | Full | Full | |
| DATE field | Full | Full | Full | Full | |
| TIME field | Full | Full | Full | Full | |
| FILENAME field | Full | Full | Full | Full | |
| AUTHOR field | Full | Full | Full | Full | |
| TOC field | Full | Full | Full | Full | Via SDT + TOC node type |
| Hyperlinks (`w:hyperlink`) | Full | Full | Full | Full | External URLs + bookmark refs |
| MERGEFIELD | None | None | None | None | |
| IF field | None | None | None | None | |
| Other complex fields | None | None | None | None | |

### 17. Styles

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Paragraph styles | Full | Full | Full | Full | |
| Character styles | Full | Full | Full | Full | |
| Table styles | Full | Full | Full | Full | Style type recognized |
| List/numbering styles | Full | Full | Full | Full | |
| Style inheritance (`w:basedOn`) | Full | Full | Full | Full | |
| Next style (`w:next`) | Full | Full | Full | Full | |
| Default style flag | Full | Full | Full | Full | |
| Document defaults (`w:docDefaults`) | Full | Full | Full | Full | Default font size, family, spacing |
| Latent styles | None | None | None | None | |
| Style sets | None | None | None | None | |

### 18. Metadata

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Title (`dc:title`) | Full | Full | Full | Full | |
| Creator (`dc:creator`) | Full | Full | Full | Full | |
| Description (`dc:description`) | Full | Full | Full | Full | |
| Subject (`dc:subject`) | Full | Full | Full | Full | |
| Keywords (`cp:keywords`) | Full | Full | Full | Full | |
| Created date (`dcterms:created`) | Full | Full | Full | Full | |
| Modified date (`dcterms:modified`) | Full | Full | Full | Full | |
| Last modified by (`cp:lastModifiedBy`) | Full | Full | Full | Full | |
| Revision count (`cp:revision`) | Full | Full | Full | Full | |
| Custom properties | Full | Full | Full | Full | Via `custom_properties` map |

### 19. Digital Signatures

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Signature detection | Full | N/A | Preserved | Full | `hasDigitalSignature` metadata flag |
| Signature count | Full | N/A | Preserved | Full | `signatureCount` metadata |
| Signer subject (X.509) | Full | N/A | Preserved | Full | Extracted from certificate |
| Signing time | Full | N/A | Preserved | Full | ISO 8601 |
| Signature validation status | Full | N/A | Preserved | Full | `signatureValid` metadata |
| Raw signature XML preservation | Full | Full | Full | Full | `_xmlsignatures/*` round-tripped |
| Signature creation | None | None | N/A | None | Out of scope |

### 20. VBA Macros

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| Macro detection | Full | N/A | Preserved | Full | `hasMacros` metadata flag |
| vbaProject.bin preservation | Full | Full | Full | Full | Binary round-trip |
| vbaData.xml preservation | Full | Full | Full | Full | XML round-trip |
| Macro execution | None | None | N/A | None | Out of scope; security boundary |

### 21. Table of Contents

| Feature | Read | Write | Round-Trip | Status | Notes |
|---------|------|-------|------------|--------|-------|
| TOC via SDT | Full | Full | Full | Full | `parse_sdt_toc` |
| TOC max level | Full | Full | Full | Full | TocMaxLevel attribute |
| TOC title | Full | Full | Full | Full | TocTitle attribute |
| Cached TOC entries | Full | Full | Full | Full | Paragraph children of TOC node |
| TOC field code | Partial | Partial | Partial | Partial | Parsed for heading levels |
| TOC update/regeneration | None | None | N/A | None | Consumer responsibility |

---

## Extension Namespace Handling

Elements in extension namespaces (`w14:`, `w15:`, `w16:`, `wp14:`) are not semantically parsed. They are:
- Skipped during reading with a debug-mode warning
- Not present in the document model
- NOT round-tripped (lost on write)

This affects Office 2016+ specific features like:
- `w14:checkbox` (modern checkboxes)
- `w15:webExtension` (add-in references)
- `w14:shadow`, `w14:textOutline` (text effects)

**`mc:AlternateContent`** is handled: the `Choice` branch (usually DrawingML) is preferred, with `Fallback` (usually VML) used as an alternative.

---

## Security Constraints

| Constraint | Implementation |
|------------|---------------|
| Max ZIP entry size (XML) | 256 MB |
| Max ZIP entry size (media) | 64 MB |
| ZIP bomb protection | Size checks before extraction |
| Billion laughs (XML) | `quick_xml` does not expand entities |
| External entity resolution | Disabled (no XXE) |

---

## Known Limitations

1. **No settings.xml support**: Document-level settings (auto-hyphenation, proofing state, etc.) are not read or written.
2. **No theme support**: Theme colors and fonts (`theme/theme1.xml`) are not resolved. Colors are always stored as explicit hex values.
3. **No font table**: Font substitution information (`fontTable.xml`) is not used. The consumer must handle font fallback.
4. **No embedded OLE editing**: OLE objects are preserved as binary blobs but cannot be inspected or modified.
5. **No revision accept/reject**: Track changes are modeled but not actionable. The consumer must implement accept/reject logic.
6. **Extension namespaces lost on write**: `w14:`, `w15:`, `w16:` elements are dropped unless captured in `RawXml` or `ShapeRawXml` attributes.
7. **Complex field instructions**: Only common fields (PAGE, NUMPAGES, DATE, TIME, FILENAME, AUTHOR, TOC, HYPERLINK) are recognized. Other field types (MERGEFIELD, IF, etc.) are not semantically parsed.
8. **No conditional table formatting**: Table-level conditional styles (band rows, first/last column emphasis) are not modeled.

---

## Test Strategy

### Unit Tests (201 tests in `s1-format-docx`)

- **Reader tests**: Minimal DOCX construction via in-memory ZIP, parse, and verify model structure
- **Writer tests**: Build model programmatically, write to DOCX, verify ZIP structure and XML content
- **Property parser tests**: Isolated parsing of `w:rPr`, `w:pPr`, table/cell properties
- **Style parser tests**: Style definitions, inheritance, default styles, character styles
- **Section parser tests**: Page size, margins, columns, break types, header/footer references
- **Comment parser tests**: Single/multiple/threaded comments, multi-paragraph content
- **Footnote/Endnote parser tests**: Single/multiple, separator skipping, multi-paragraph
- **Header/footer parser tests**: Paragraphs, complex fields, page number fields
- **Numbering parser tests**: Abstract definitions, instances, multi-level lists
- **Signature parser tests**: Detection, subject extraction, validation status

### Round-Trip Tests

Round-trip testing validates: `DOCX bytes -> read() -> DocumentModel -> write() -> DOCX bytes -> read() -> DocumentModel`, then compare the two models.

Current round-trip coverage:
- Basic text content
- Paragraph formatting (alignment, indentation, spacing)
- Character formatting (bold, italic, underline, color, font)
- Tables with formatting
- Images (inline and floating)
- Styles (paragraph, character, table)
- Lists (bullet and numbered, multi-level)
- Headers and footers (default, first, even)
- Footnotes and endnotes
- Comments with range markers
- Sections (page layout, margins, orientation)
- Metadata (Dublin Core properties)
- Digital signatures (preserved)
- VBA macros (preserved)
- Custom XML parts (preserved)

### Recommended Additional Tests

- Real-world DOCX files from various office applications
- Large documents (1000+ paragraphs, 100+ images)
- Complex nested tables (3+ levels)
- Documents with every supported feature combined
- Corrupted/malformed DOCX files (error handling)
- Password-protected DOCX files (encryption detection)

---

## Feature Coverage Summary

| Area | s1engine Status |
|------|----------------|
| Core text/paragraphs | Full |
| Tables | Full (minus conditional formatting) |
| Images | Full (minus effects) |
| Track changes | Read/preserve |
| Comments | Full |
| Equations | Preserve only |
| Shapes | Basic properties + raw preservation |
| SmartArt | Preserve only |
| Charts | Preserve only |
| Form controls | Preserve only |
| Macros | Preserve only (no execution) |
| Complex fields | Partial |
| Themes | Not supported |
| Settings | Not supported |

---

## File Size and Performance

| Document Type | Typical Read Time | Typical Write Time |
|---------------|-------------------|-------------------|
| Simple (1-10 pages, text only) | < 5ms | < 5ms |
| Medium (50 pages, images) | 10-50ms | 10-50ms |
| Large (500 pages, complex) | 50-200ms | 50-200ms |

*Benchmarks should be run using `criterion` on representative documents.*

---

## Live Editing Fidelity

The feature matrix above covers **round-trip fidelity** (import → export preservation).
Live editing fidelity — how faithfully features render while the user is actively editing
— has additional limitations:

### Round-trip safe vs. live-edit visible

| Feature | Round-trip | Live editor | Notes |
|---------|:----------:|:-----------:|-------|
| Widow/orphan control | Preserved | Not enforced | Layout engine pagination does not yet split at widow/orphan boundaries |
| Outline level | Preserved | Not rendered | Stored in paragraph properties but not reflected in editor view |
| Small caps / all caps | Preserved | Not rendered | Attribute preserved but CSS rendering not applied in editor |
| Text effects (shadow, emboss, etc.) | Preserved | Not rendered | Stored as raw attribute, no visual representation |
| Section columns | Preserved | Rendered | Fully supported in layout engine |
| Headers/footers | Preserved | Rendered | Fully supported in paginated view |
| Page breaks | Preserved | Rendered | Rendered as page boundaries |
| Conditional table formatting | Preserved | Not applied | Table band/first-row styles not resolved during editing |

### Collaborative editing fidelity

During real-time collaboration, additional fidelity constraints apply:

- **Text convergence** is CRDT-native when the CRDT module is available, providing
  character-level consistency with sub-second convergence.
- **Structural operations** (paragraph split/merge, table edits, image changes)
  converge via periodic document snapshots, which may cause temporary divergence
  (typically 1-5 seconds) between peers' visible page layouts.
- **Pagination** is recomputed locally by each peer after receiving changes, so page
  breaks may briefly differ between peers during rapid editing.
- **Undo/redo** broadcasts the resulting document state inline to peers, avoiding an
  extra round-trip but still requiring a full re-render on the receiving side.

### Recommendation for consumers

Distinguish between these two fidelity guarantees in release notes and user-facing
documentation. "Document preserved" means the data survives a round-trip. "Visually
accurate" means the live editor renders it faithfully while editing.

---

## Version History

| Date | Change |
|------|--------|
| 2026-03-21 | Initial specification |
