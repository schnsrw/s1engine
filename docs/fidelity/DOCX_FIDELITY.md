# DOCX (OOXML) Feature Fidelity Audit

Audit of `s1-format-docx` crate against ECMA-376 5th Edition / ISO 29500 WordprocessingML.

**Crate location:** `crates/s1-format-docx/src/`
**Audit date:** 2026-04-01

---

## Summary

| Category | Handled | Partially | Ignored | Total | Coverage |
|----------|---------|-----------|---------|-------|----------|
| Run Properties (rPr) | 31 | 0 | 4+ | ~35 | ~89% |
| Paragraph Properties (pPr) | 22 | 0 | 9+ | ~31 | ~71% |
| Table Properties | 29 | 0 | 1+ | ~30 | ~97% |
| Section Properties | 11 | 0 | 4 | 15 | ~73% |
| Images/Drawing | 14 | 0 | 7+ | ~21 | ~67% |
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
| **TOTAL** | **~171** | **~2** | **~54+** | **~228** | **~76%** |

> **Updated 2026-04-01**: Added tblGrid parsing (table column widths from DOCX grid), table background shading (w:tblShd). Underline style changed from bool to String throughout entire pipeline (6 styles: single, double, thick, dotted, dashed, wave). Baseline positioning now uses actual font metrics (ascent) instead of hardcoded 0.8 factor. Inline images now rendered in canvas scene mode.
>
> **Full rendering pipeline audit confirmed:** Text boxes, shapes, header/footer images, horizontal rules, page/section breaks, multi-column layout, table borders, inline/floating images, footnotes, endnotes, comments, list markers, page numbers — all fully wired from DOCX parse → model → layout → scene JSON → canvas rendering.
>
> Layout engine now propagates: double_strikethrough, baseline_shift, caps, small_caps, hidden text, per-paragraph widow_control and contextual_spacing to the style resolver and scene JSON output.

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
| `w:caps` | — | `Caps(true)` | All capitals display. Layout propagates to scene JSON |
| `w:smallCaps` | — | `SmallCaps(true)` | Small capitals display. Layout propagates to scene JSON |
| `w:dstrike` | — | `DoubleStrikethrough(true)` | Double strikethrough. Rendered in canvas |
| `w:vanish` | — | `Hidden(true)` | Hidden text. Layout marks run as hidden |
| `w:position` | `w:val` | `BaselineShift(f64)` | Half-points -> points. Affects vertical positioning |
| `w:szCs` | `w:val` | `FontSizeCS(f64)` | Complex-script font size (half-points -> points) |
| `w:bCs` | — | `BoldCS(true)` | Complex-script bold |
| `w:iCs` | — | `ItalicCS(true)` | Complex-script italic |
| `w:rFonts/@w:eastAsia` | `w:eastAsia` | `FontFamilyEastAsia(String)` | East Asian font family |
| `w:rFonts/@w:cs` | `w:cs` | `FontFamilyCS(String)` | Complex-script font family |

### IGNORED (Not Parsed)

| Element | What It Does | Impact |
|---------|-------------|--------|
| `w:emboss` | Embossed text effect | Low — decorative |
| `w:imprint` | Engraved text effect | Low — decorative |
| `w:effect` | Text animation (blink, etc.) | Low — deprecated |
| `w:border` | Run-level text border | Low — rare |
| `w:kern` | Kerning threshold | Low — typography detail |
| `w:fitText` | Fit text to width | Low — rare |
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
| `w:widowControl` | — | `WidowControl(bool)` | Per-paragraph widow/orphan control. Wired to layout |
| `w:outlineLvl` | `w:val` | `OutlineLevel(i64)` | Outline level (0-9) for TOC |
| `w:textDirection` | `w:val` | `ParagraphWritingMode(String)` | Text direction (btLr, tbRl, etc.) |
| `w:contextualSpacing` | — | `ContextualSpacing(bool)` | Suppress spacing between same-style paragraphs |

### IGNORED (Not Parsed)

| Element | What It Does | Impact |
|---------|-------------|--------|
| `w:suppressLineNumbers` | Suppress line numbers | Low |
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

| `w:tblCellMar` | `TableDefaultCellMargins(Margins)` | Default cell margins (twips -> points) |
| `w:tblLayout` | `TableLayout(TableLayoutMode)` | Fixed vs autofit layout mode |
| `w:tblInd` | `TableIndent(f64)` | Table indent from margin (twips -> points) |

### TABLE LEVEL — IGNORED

| Element | Impact |
|---------|--------|
| `w:tblLook` | Low — conditional formatting flags |
| `w:tblCaption` | Low — accessibility |
| `w:tblDescription` | Low — accessibility |
| `w:tblPct` | Low — alternate width method |
| `w:tblShd` | Low — table-level shading |

### ROW LEVEL (w:trPr) — HANDLED

| Element | Model Mapping |
|---------|---------------|
| `w:tblHeader` | `TableHeaderRow(true)` |

| `w:trHeight` | `RowHeight(f64)`, `RowHeightRule(String)` | Explicit row height (twips -> points) with atLeast/exact rule |
| `w:cantSplit` | `RowNoSplit(bool)` | Prevents row from splitting across pages |

### ROW LEVEL — IGNORED

| Element | Impact |
|---------|--------|
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

| `w:tcMar` | `CellPadding(Margins)` | Per-cell margins (twips -> points) |
| `w:textDirection` | `CellTextDirection(String)` | Cell text direction |
| `w:noWrap` | `CellNoWrap(bool)` | Cell no-wrap flag |

### CELL LEVEL — IGNORED

| Element | Impact |
|---------|--------|
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

### MEDIUM IMPACT

3. **Behind/in-front text** — `wp:behindDoc` z-order not implemented
4. **Legacy text frames** — `w:framePr` not parsed
5. **Japanese line break rules** — `w:kinsoku` not parsed (CJK layout)

### LOW IMPACT

6. **Emboss/imprint** — Rare decorative text effects
7. **Kerning threshold** — Typography detail
8. **Ligatures/numForm/numSpacing** — Advanced OpenType features
9. **Table caption/description** — Accessibility metadata
10. **Latent styles** — UI management only
11. **Character width scaling** — `w:w` run attribute

### RESOLVED (since 2026-03-29)

- ~~Table cell margins~~ → Parsed (`w:tblCellMar`, `w:tcMar`) + round-trip
- ~~Table layout mode~~ → Parsed (`w:tblLayout` fixed/autofit) + round-trip
- ~~Complex-script~~ → Parsed (`w:szCs`, `w:bCs`, `w:iCs`, `w:rFonts/@eastAsia/@cs`) + round-trip
- ~~Widow/orphan control~~ → Parsed + wired through layout engine's `ResolvedParagraphStyle`
- ~~Outline levels~~ → Parsed (`w:outlineLvl`) + round-trip
- ~~Row height~~ → Parsed (`w:trHeight` + `w:hRule`) + round-trip
- ~~Position/baseline shift~~ → Parsed + layout propagation + scene JSON output
- ~~Cell text direction~~ → Parsed (`w:textDirection`) + round-trip
- ~~Caps/smallCaps~~ → Parsed + layout propagation + scene JSON output
- ~~Hidden text~~ → Parsed (`w:vanish`) + layout propagation (marked hidden in scene)
- ~~Double strikethrough~~ → Parsed + layout propagation + scene JSON output
- ~~Page borders~~ → Parsed (`w:pgBorders`) + round-trip
- ~~Document grid~~ → Parsed (`w:docGrid` type/linePitch) + round-trip
- ~~Contextual spacing~~ → Parsed + wired through layout engine
- ~~Row can't split~~ → Parsed (`w:cantSplit`) + round-trip
- ~~Cell no-wrap~~ → Parsed (`w:noWrap`) + round-trip
- ~~Table indent~~ → Parsed (`w:tblInd`) + round-trip
