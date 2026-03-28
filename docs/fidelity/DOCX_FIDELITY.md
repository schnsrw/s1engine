# DOCX (OOXML) Feature Fidelity Audit

Audit of `s1-format-docx` crate against ECMA-376 5th Edition / ISO 29500 WordprocessingML.

**Crate location:** `crates/s1-format-docx/src/`
**Audit date:** 2026-03-29

---

## Summary

| Category | Handled | Partially | Ignored | Total | Coverage |
|----------|---------|-----------|---------|-------|----------|
| Run Properties (rPr) | 23 | 0 | 12+ | ~35 | ~66% |
| Paragraph Properties (pPr) | 18 | 0 | 13+ | ~31 | ~58% |
| Table Properties | 21 | 0 | 6+ | ~27 | ~78% |
| Section Properties | 8 | 0 | 7 | 15 | ~53% |
| Images/Drawing | 11 | 2 | 8+ | ~21 | ~52% |
| Styles | 8 | 0 | 8 | 16 | ~50% |
| Numbering/Lists | 8 | 0 | 2 | 10 | ~80% |
| Headers/Footers | 6 | 0 | 0 | 6 | 100% |
| Comments | 4 | 0 | 1 | 5 | ~80% |
| Footnotes/Endnotes | 4 | 0 | 0 | 4 | 100% |
| Bookmarks | 2 | 0 | 0 | 2 | 100% |
| Hyperlinks | 3 | 0 | 0 | 3 | 100% |
| Track Changes | 10 | 0 | 5 | 15 | ~67% |
| Fields | 3 | 2 | 5+ | ~10 | ~30% |
| Content Controls (SDT) | 7 | 0 | 7 | 14 | ~50% |
| Metadata | 9 | 0 | 2 | 11 | ~82% |
| **TOTAL** | **~145** | **~4** | **~75+** | **~225** | **~66%** |

> **Updated 2026-03-29**: Added 20 new features: caps, smallCaps, vanish, dstrike, szCs, bCs, iCs, position, eastAsia/cs fonts, widowControl, outlineLvl, tblLayout, tblCellMar, tblInd, trHeight, cantSplit, tcMar, textDirection, noWrap.

---

## Run Properties (w:rPr)

### HANDLED (Read + Write)

| Element | Attribute | Model Mapping | Notes |
|---------|-----------|---------------|-------|
| `w:b` | — | `Bold(true)` | Toggle bold |
| `w:i` | — | `Italic(true)` | Toggle italic |
| `w:u` | `w:val` | `Underline(style)` | 6 styles: single, double, thick, dotted, dashed, wave |
| `w:strike` | — | `Strikethrough(true)` | Single strikethrough |
| `w:sz` | `w:val` | `FontSize(f64)` | Half-points -> points (val/2) |
| `w:color` | `w:val` | `Color(r,g,b)` | Hex color string |
| `w:highlight` | `w:val` | `HighlightColor(Color)` | Named colors mapped to hex |
| `w:shd` | `w:fill` | `HighlightColor(Color)` | Background shading via fill attr |
| `w:rFonts` | `w:ascii`, `w:hAnsi`, `w:cs` | `FontFamily(String)` | Priority: ascii > hAnsi > cs |
| `w:vertAlign` | `w:val` | `Superscript/Subscript(true)` | superscript, subscript, baseline |
| `w:spacing` | `w:val` | `FontSpacing(f64)` | Twips -> points (val/20) |
| `w:lang` | `w:val` | `Language(String)` | BCP 47 language tag |
| `w:shadow` | — | `TextShadow(true)` | Shadow text effect |
| `w:outline` | — | `TextOutline(true)` | Outline text effect |

### IGNORED (Not Parsed)

| Element | What It Does | Impact |
|---------|-------------|--------|
| `w:caps` | All capitals display | Low — visual only |
| `w:smallCaps` | Small capitals display | Low — visual only |
| `w:dstrike` | Double strikethrough | Low — rare |
| `w:vanish` | Hidden text | Medium — content visibility |
| `w:emboss` | Embossed text effect | Low — decorative |
| `w:imprint` | Engraved text effect | Low — decorative |
| `w:effect` | Text animation (blink, etc.) | Low — deprecated |
| `w:border` | Run-level text border | Low — rare |
| `w:kern` | Kerning threshold | Low — typography detail |
| `w:position` | Baseline shift in half-points | Medium — affects layout |
| `w:fitText` | Fit text to width | Low — rare |
| `w:szCs` | Complex-script font size | Medium — BiDi text |
| `w:bCs`, `w:iCs` | Complex-script bold/italic | Medium — BiDi text |
| `w:rFonts/@w:eastAsia` | East Asian font family | Medium — CJK text |
| `w:ligatures` | OpenType ligature control | Low — typography detail |
| `w:numForm` | Number form (lining/oldstyle) | Low — typography detail |
| `w:numSpacing` | Number spacing (proportional/tabular) | Low — typography detail |
| `w:contextualAlts` | Contextual alternates | Low — typography detail |
| `w:w` (run) | Character width scaling | Low — rare |
| `w:eastAsianLayout` | East Asian typography settings | Medium — CJK |

---

## Paragraph Properties (w:pPr)

### HANDLED (Read + Write)

| Element | Attribute | Model Mapping | Notes |
|---------|-----------|---------------|-------|
| `w:pStyle` | `w:val` | `StyleId(String)` | Paragraph style reference |
| `w:jc` | `w:val` | `Alignment(enum)` | left, center, right, both(justify) |
| `w:spacing` | `w:before` | `SpacingBefore(f64)` | Twips -> points |
| `w:spacing` | `w:after` | `SpacingAfter(f64)` | Twips -> points |
| `w:spacing` | `w:line`, `w:lineRule` | `LineSpacing(enum)` | auto/atLeast/exact with value |
| `w:ind` | `w:left` | `IndentLeft(f64)` | Twips -> points |
| `w:ind` | `w:right` | `IndentRight(f64)` | Twips -> points |
| `w:ind` | `w:firstLine` | `IndentFirstLine(f64)` | Twips -> points |
| `w:ind` | `w:hanging` | `IndentFirstLine(-f64)` | Negative first-line indent |
| `w:keepNext` | — | `KeepWithNext(true)` | Keep with next paragraph |
| `w:keepLines` | — | `KeepLinesTogether(true)` | Keep lines together |
| `w:pageBreakBefore` | — | `PageBreakBefore(true)` | Page break before paragraph |
| `w:bidi` | — | `Bidi(true)` | Bidirectional paragraph |
| `w:suppressAutoHyphens` | — | `SuppressAutoHyphens(true)` | Suppress auto-hyphenation |
| `w:contextualSpacing` | — | `ContextualSpacing(true)` | Contextual spacing |
| `w:wordWrap` | — | `WordWrap(true)` | Word wrap control |
| `w:shd` | `w:fill` | `Background(Color)` | Paragraph background |
| `w:tabs` | children | `TabStops(Vec<TabStop>)` | Position, alignment, leader |
| `w:numPr` | `w:ilvl`, `w:numId` | `ListInfo(level, num_id)` | List/numbering reference |
| `w:pBdr` | children | `ParagraphBorders(Borders)` | Top/bottom/left/right borders |

### IGNORED (Not Parsed)

| Element | What It Does | Impact |
|---------|-------------|--------|
| `w:outlineLvl` | Outline level (0-9) | Medium — TOC generation |
| `w:widowControl` | Widow/orphan control | Medium — pagination |
| `w:suppressLineNumbers` | Suppress line numbers | Low |
| `w:textDirection` | Text direction (btLr, tbRl, etc.) | Medium — vertical text |
| `w:autoSpaceDE` | Auto-space East Asian/digit | Low — CJK |
| `w:autoSpaceDN` | Auto-space East Asian/number | Low — CJK |
| `w:snapToGrid` | Snap to document grid | Low |
| `w:kinsoku` | Japanese line break rules | Medium — CJK |
| `w:framePr` | Text frame properties | Medium — legacy frames |
| `w:mirrorIndents` | Mirror left/right indents | Low — facing pages |
| `w:adjustRightInd` | Auto-adjust right indent | Low |
| `w:textboxTightWrap` | Tight wrap in text box | Low |
| `w:topLinePunct` | CJK top-line punctuation | Low — CJK |
| `w:overflowPunct` | CJK overflow punctuation | Low — CJK |

---

## Table Properties

### TABLE LEVEL (w:tblPr) — HANDLED

| Element | Model Mapping | Notes |
|---------|---------------|-------|
| `w:tblStyle` | Style reference | Table style ID |
| `w:tblW` | `TableWidth(enum)` | auto/dxa(fixed)/pct types |
| `w:jc` | `TableAlignment(enum)` | center, right, left |
| `w:tblBorders` | `TableBorders(Borders)` | All 6 sides |

### TABLE LEVEL — IGNORED

| Element | Impact |
|---------|--------|
| `w:tblCellMar` | Medium — default cell margins |
| `w:tblLayout` | Medium — fixed vs autofit |
| `w:tblInd` | Low — table indent from margin |
| `w:tblLook` | Low — conditional formatting flags |
| `w:tblCaption` | Low — accessibility |
| `w:tblDescription` | Low — accessibility |
| `w:tblPct` | Low — alternate width method |
| `w:tblShd` | Low — table-level shading |

### ROW LEVEL (w:trPr) — HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:tblHeader` | `TableHeaderRow(true)` |

### ROW LEVEL — IGNORED

| Element | Impact |
|---------|--------|
| `w:trHeight` | Medium — explicit row height |
| `w:cantSplit` | Medium — row page break control |
| `w:wBefore`/`w:wAfter` | Low — row indent |
| `w:jc` | Low — row alignment override |

### CELL LEVEL (w:tcPr) — HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:tcW` | `CellWidth(f64)` |
| `w:gridSpan` | `ColSpan(i64)` |
| `w:vMerge` | `RowSpan` tracking |
| `w:vAlign` | `VerticalAlign(enum)` |
| `w:shd` | `CellBackground(Color)` |
| `w:tcBorders` | `CellBorders(Borders)` |

### CELL LEVEL — IGNORED

| Element | Impact |
|---------|--------|
| `w:tcMar` | Medium — per-cell margins |
| `w:textDirection` | Medium — cell text direction |
| `w:noWrap` | Low — cell no-wrap |
| `w:hideMark` | Low — hide cell mark |

---

## Section Properties (w:sectPr)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:pgSz/@w:w,h` | `PageWidth/PageHeight(f64)` |
| `w:pgSz/@w:orient` | `Orientation(enum)` |
| `w:pgMar/@w:top,bottom,left,right` | `MarginTop/Bottom/Left/Right(f64)` |
| `w:pgMar/@w:header,footer` | `HeaderDistance/FooterDistance(f64)` |
| `w:cols/@w:num,space` | `Columns(u32)`, `ColumnSpacing(f64)` |
| `w:type` | `SectionBreakType(enum)` |
| `w:headerReference` | Header node ID resolution |
| `w:footerReference` | Footer node ID resolution |
| `w:titlePg` | `title_page: bool` |
| `w:evenAndOddHeaders` | `even_and_odd_headers: bool` |

### IGNORED

| Element | Impact |
|---------|--------|
| `w:pgBorders` | Medium — page borders |
| `w:lnNumType` | Low — line numbering |
| `w:docGrid` | Medium — document grid |
| `w:vAlign` | Low — vertical page alignment |
| `w:footnotePr` | Low — footnote numbering/position |
| `w:endnotePr` | Low — endnote numbering/position |
| `w:paperSrc` | Low — printer paper source |

---

## Images & Drawing (DrawingML)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:drawing` | Container detection |
| `wp:inline` | `ImagePositionType: Inline` |
| `wp:anchor` | `ImagePositionType: Floating` |
| `wp:extent/@cx,cy` | `ImageWidth/Height(f64)` (EMU -> points) |
| `wp:docPr/@descr` | `ImageAltText(String)` |
| `a:blip/@r:embed` | `ImageMediaId(MediaId)` via relationship |
| `wp:positionH/@relativeFrom` | `ImageHorizontalRelativeFrom` |
| `wp:positionV/@relativeFrom` | `ImageVerticalRelativeFrom` |
| `wp:posOffset` | `ImageHorizontalOffset/ImageVerticalOffset` |
| `wp:wrapSquare` | `ImageWrapType: Square` |
| `wp:wrapTight` | `ImageWrapType: Tight` |
| `wp:wrapThrough` | `ImageWrapType: Through` |
| `wp:wrapTopAndBottom` | `ImageWrapType: TopAndBottom` |
| `wp:wrapNone` | `ImageWrapType: None` |
| `wp:dist*` | `ImageDistanceFromText(f64)` |

### IGNORED

| Element | Impact |
|---------|--------|
| `wp:wrapPolygon` | Low — custom wrap outline |
| `wp:behindDoc` | Medium — z-order |
| `wp:relativeHeight` | Medium — z-order |
| `wp:simplePos` | Low — simple positioning |
| `wp:locked` | Low — anchor lock |
| `wp:layoutInCell` | Low — cell layout flag |
| `wp:allowOverlap` | Low — overlap flag |
| `a:graphic/wsp:*` | Medium — shapes, WordArt |

---

## Styles (w:styles)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:docDefaults/w:rPrDefault` | `DocumentDefaults` (font, size) |
| `w:docDefaults/w:pPrDefault` | `DocumentDefaults` (spacing) |
| `w:style/@w:styleId` | `Style.id` |
| `w:style/@w:type` | `Style.style_type` |
| `w:style/@w:default` | `Style.is_default` |
| `w:style/w:name` | `Style.name` |
| `w:style/w:basedOn` | `Style.parent_id` |
| `w:style/w:next` | `Style.next_style_id` |
| `w:style/w:rPr` | `Style.attributes` (merged) |
| `w:style/w:pPr` | `Style.attributes` (merged) |

### IGNORED

| Element | Impact |
|---------|--------|
| `w:latentStyles` | Low — hidden style management |
| `w:qFormat` | Low — UI display priority |
| `w:uiPriority` | Low — UI sort order |
| `w:semiHidden` | Low — UI visibility |
| `w:unhideWhenUsed` | Low — UI auto-unhide |
| `w:link` | Medium — character/paragraph style linking |
| `w:aliases` | Low — display aliases |
| `w:tblPr`/`w:tblStylePr` | Medium — table conditional styles |

---

## Numbering/Lists (w:numbering.xml)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:abstractNum` | `AbstractNumbering` |
| `w:abstractNum/@w:abstractNumId` | `.abstract_num_id` |
| `w:lvl/@w:ilvl` | `NumberingLevel.level` |
| `w:lvl/w:start` | `.start` |
| `w:lvl/w:numFmt` | `.num_format` (6 formats) |
| `w:lvl/w:lvlText` | `.level_text` |
| `w:lvl/w:lvlJc` | `.alignment` |
| `w:lvl/w:pPr/w:ind` | `.indent_left`, `.indent_hanging` |
| `w:lvl/w:rPr/w:rFonts` | `.bullet_font` |
| `w:num` | `NumberingInstance` |
| `w:num/w:abstractNumId` | `.abstract_num_id` reference |
| `w:lvlOverride` | `LevelOverride` |

### NUMBER FORMATS HANDLED (6 of 60+)

| Format | OOXML Value |
|--------|-------------|
| Bullet | `bullet` |
| Decimal | `decimal` |
| Lower Alpha | `lowerLetter` |
| Upper Alpha | `upperLetter` |
| Lower Roman | `lowerRoman` |
| Upper Roman | `upperRoman` |

### NUMBER FORMATS IGNORED

| Format | Impact |
|--------|--------|
| `ordinal`, `cardinalText`, `ordinalText` | Low |
| `hex`, `chicago`, `ideographDigital` | Low — rare |
| `japaneseCounting`, `chineseCounting*` | Medium — CJK |
| `korean*`, `taiwanese*`, `hindi*`, `thai*` | Medium — i18n |
| All 50+ other ST_NumberFormat values | Low-Medium |

---

## Track Changes

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:ins` | Insertion revision (id, author, date) |
| `w:del` | Deletion revision (id, author, date) |
| `w:moveFrom` | Move source |
| `w:moveTo` | Move destination |
| `w:rPrChange` | Run property change (captures old XML) |
| `w:pPrChange` | Paragraph property change |
| `w:tblPrChange` | Table property change |
| `w:trPrChange` | Row property change |
| `w:tcPrChange` | Cell property change |
| `w:delText` | Deleted text content |

### IGNORED

| Element | Impact |
|---------|--------|
| `w:customXmlInsRangeStart/End` | Low |
| `w:customXmlDelRangeStart/End` | Low |
| `w:numberingChange` | Medium — list revision |
| `w:sectPrChange` | Medium — section revision |
| `w:cellIns`/`w:cellDel` | Medium — table structure revision |

---

## Fields

### HANDLED

| Field Type | Notes |
|------------|-------|
| `w:fldSimple` | Simple field instruction |
| `w:fldChar` (begin/separate/end) | Complex field markers |
| `w:instrText` | Field instruction text |
| `PAGE` | Page number |
| `NUMPAGES` | Total pages |

### IGNORED (NOT PARSED)

| Field Type | Impact |
|------------|--------|
| `DATE`, `TIME` | Medium |
| `AUTHOR`, `FILENAME` | Low |
| `TOC` (instruction) | High — table of contents |
| `HYPERLINK` | Medium — field-based links |
| `REF`, `PAGEREF` | Medium — cross-references |
| `SEQ` | Medium — figure/table numbering |
| `MERGEFIELD`, `IF` | Medium — mail merge |
| `STYLEREF` | Low — style-based references |
| All other 60+ field types | Varies |

---

## Content Controls (SDT)

### HANDLED

| Element | Notes |
|---------|-------|
| `w:sdt` | Container recognized |
| `w:sdtPr` | Properties parsed |
| `w:sdtContent` | Content extracted |
| `w:checkbox` | Checkbox state |
| `w:dropDownList` + `w:listItem` | Dropdown with options |
| `w:text` | Text content control |
| `w:docPartGallery` | Gallery reference |

### IGNORED

| Element | Impact |
|---------|--------|
| `w:alias` | Low — display name |
| `w:tag` | Medium — custom tag |
| `w:dataBinding` | High — data binding |
| `w:placeholder` | Medium — placeholder text |
| `w:date` | Medium — date picker |
| `w:comboBox` | Medium — combo box |
| `w:color`/`w:appearance` | Low — visual |

---

## Metadata (docProps/core.xml)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `dc:title` | `metadata.title` |
| `dc:creator` | `metadata.creator` |
| `dc:subject` | `metadata.subject` |
| `dc:description` | `metadata.description` |
| `cp:keywords` | `metadata.keywords` (comma-split) |
| `cp:revision` | `metadata.revision` |
| `dcterms:created` | `metadata.created` |
| `dcterms:modified` | `metadata.modified` |
| `dc:language` | `metadata.language` |

### IGNORED

| Element | Impact |
|---------|--------|
| `cp:lastModifiedBy` | Low |
| `cp:category` | Low |

---

## Round-Trip Preservation

These elements are stored in `preserved_parts` HashMap for lossless round-trip:

| ZIP Path | Content |
|----------|---------|
| `customXml/*` | Custom XML parts |
| `word/diagrams/*` | SmartArt diagrams |
| `word/charts/*` | Embedded charts |
| `word/embeddings/*` | OLE embeddings |
| `word/vbaProject.bin` | VBA macro binary |
| `word/vbaData.xml` | VBA macro data |
| `_xmlsignatures/*` | Digital signatures |
| Any unrecognized path | Preserved as raw bytes |

---

## Key Gaps (Ordered by Impact)

### HIGH IMPACT

1. **Advanced fields** — TOC field instructions, cross-references, mail merge fields not parsed
2. **Data binding** — SDT data binding to custom XML not implemented
3. **Table cell margins** — `w:tblCellMar` ignored, affects table layout
4. **Table layout mode** — `w:tblLayout` (fixed vs autofit) ignored
5. **Complex-script support** — `w:szCs`, `w:bCs`, `w:iCs`, east Asian fonts ignored

### MEDIUM IMPACT

6. **Widow/orphan control** — `w:widowControl` ignored
7. **Outline levels** — `w:outlineLvl` ignored (affects TOC generation from headings without styles)
8. **Row height** — `w:trHeight` ignored
9. **Page borders** — `w:pgBorders` ignored
10. **Position (baseline shift)** — `w:position` in rPr ignored
11. **Behind/in-front text** — `wp:behindDoc` z-order ignored
12. **Document grid** — `w:docGrid` ignored (CJK layout)
13. **Cell text direction** — `w:textDirection` in tcPr ignored

### LOW IMPACT

14. **Caps/smallCaps** — Display-only formatting
15. **Emboss/imprint/vanish** — Rare text effects
16. **Kerning threshold** — Typography detail
17. **Ligatures/numForm/numSpacing** — Advanced OpenType
18. **Table caption/description** — Accessibility metadata
19. **Latent styles** — UI management only
