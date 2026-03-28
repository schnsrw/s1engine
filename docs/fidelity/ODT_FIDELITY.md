# ODT (ODF) Feature Fidelity Audit

Audit of `s1-format-odt` crate against OASIS ODF 1.2 / ISO/IEC 26300.

**Crate location:** `crates/s1-format-odt/src/`
**Audit date:** 2026-03-29

---

## Summary

| Category | Handled | Partially | Ignored | Total | Coverage |
|----------|---------|-----------|---------|-------|----------|
| Text Properties | 11 | 0 | 15+ | ~26 | ~42% |
| Paragraph Properties | 15 | 0 | 20+ | ~35 | ~43% |
| Table Properties | 6 | 2 | 10+ | ~18 | ~33% |
| List Properties | 3 | 1 | 5+ | ~9 | ~33% |
| Page Layout | 6 | 0 | 8+ | ~14 | ~43% |
| Headers/Footers | 4 | 0 | 3 | 7 | ~57% |
| Images/Frames | 4 | 0 | 10+ | ~14 | ~29% |
| Fields | 2 | 1 | 10+ | ~13 | ~15% |
| Comments | 4 | 0 | 1 | 5 | ~80% |
| Footnotes/Endnotes | 4 | 0 | 1 | 5 | ~80% |
| Bookmarks | 3 | 0 | 0 | 3 | 100% |
| Change Tracking | 0 | 1 | 5 | 6 | ~8% |
| Styles | 6 | 0 | 5+ | ~11 | ~55% |
| Metadata | 9 | 0 | 5+ | ~14 | ~64% |
| **TOTAL** | **~77** | **~5** | **~98+** | **~180** | **~43%** |

---

## Text Properties (style:text-properties)

### HANDLED

| Attribute | Model Mapping | Notes |
|-----------|---------------|-------|
| `fo:font-weight="bold"` | `Bold(true)` | |
| `fo:font-style="italic"` | `Italic(true)` | |
| `fo:font-size` | `FontSize(f64)` | Supports pt, cm, mm, in, px |
| `style:font-name` / `fo:font-family` | `FontFamily(String)` | |
| `fo:color` | `Color(r,g,b)` | Hex color |
| `style:text-underline-style` | `Underline(style)` | solid, double, dotted, dash, wave, none |
| `style:text-line-through-style` | `Strikethrough(true)` | solid only |
| `fo:background-color` | `HighlightColor(Color)` | Hex or named color |
| `style:text-position` | `Superscript/Subscript(true)` | "super 58%", "sub 58%" parsed |
| `fo:letter-spacing` | `FontSpacing(f64)` | Length value |
| `fo:language` | `Language(String)` | via paragraph-level style |

### IGNORED

| Attribute | What It Does | Impact |
|-----------|-------------|--------|
| `style:font-name-asian` | CJK font | Medium — CJK |
| `style:font-name-complex` | Complex script font | Medium — BiDi |
| `fo:font-size-asian/complex` | Script-specific size | Medium |
| `style:text-shadow` | Text shadow | Low — decorative |
| `style:text-outline` | Outline text | Low — decorative |
| `fo:text-transform` | uppercase/lowercase/capitalize | Medium — affects display |
| `style:text-scale` | Width scaling | Low |
| `style:font-size-rel` | Relative font size | Low |
| `fo:font-variant` | small-caps | Low |
| `style:text-relief` | Embossed/engraved | Low |
| `style:text-emphasize` | Emphasis marks (CJK) | Medium — CJK |
| `style:use-window-font-color` | Theme-aware color | Low |
| `style:text-rotation-angle` | Rotated text | Low |
| `fo:hyphenate` | Word hyphenation control | Low |
| `style:font-weight-asian/complex` | Script-specific bold | Medium |

---

## Paragraph Properties (style:paragraph-properties)

### HANDLED

| Attribute | Model Mapping | Notes |
|-----------|---------------|-------|
| `fo:text-align` | `Alignment(enum)` | start, center, end, right, justify |
| `fo:margin-top` | `SpacingBefore(f64)` | Length -> points |
| `fo:margin-bottom` | `SpacingAfter(f64)` | Length -> points |
| `fo:margin-left` | `IndentLeft(f64)` | Length -> points |
| `fo:margin-right` | `IndentRight(f64)` | Length -> points |
| `fo:text-indent` | `IndentFirstLine(f64)` | Length -> points |
| `fo:line-height` (percentage) | `LineSpacing::Multiple` | e.g. "150%" -> 1.5 |
| `fo:line-height` (absolute) | `LineSpacing::Exact` | e.g. "12pt" |
| `fo:break-before="page"` | `PageBreakBefore(true)` | |
| `fo:keep-with-next="always"` | `KeepWithNext(true)` | |
| `fo:keep-together="always"` | `KeepLinesTogether(true)` | |
| `fo:background-color` | `Background(Color)` | Paragraph background |
| `fo:border-*` | `ParagraphBorders(Borders)` | top/bottom/left/right |
| `style:tab-stops/style:tab-stop` | `TabStops(Vec<TabStop>)` | position, type, leader |

### IGNORED

| Attribute | What It Does | Impact |
|-----------|-------------|--------|
| `style:line-height-at-least` | Minimum line height | Medium |
| `fo:widows` | Widow control | Medium — pagination |
| `fo:orphans` | Orphan control | Medium — pagination |
| `fo:padding-*` | Paragraph padding | Low |
| `style:auto-text-indent` | Automatic first indent | Low |
| `style:snap-to-grid` | Grid snapping | Low |
| `style:writing-mode` | LTR/RTL/vertical | Medium |
| `style:master-page-name` | Master page reference | Medium |
| `fo:column-count/gap` | Multi-column | Medium |
| `style:register-true` | Register-true printing | Low |
| `style:page-number` | Page number offset | Low |
| `style:text-autospace` | Auto-spacing for CJK | Low |
| `fo:hyphenation-*` | Hyphenation rules | Low |
| `style:vertical-align` | Vertical alignment | Low |
| `fo:border` (shorthand) | Combined border | Low — individual sides handled |
| `style:shadow` | Paragraph shadow | Low |
| `style:join-border` | Join adjacent borders | Low |
| `style:line-spacing` | Additional line spacing | Low |

---

## Table Properties

### HANDLED

| Element/Attribute | Model Mapping |
|-------------------|---------------|
| `table:table` | `NodeType::Table` |
| `table:table-row` | `NodeType::TableRow` |
| `table:table-cell` | `NodeType::TableCell` |
| `table:table-column` | Width extraction via style |
| `table:number-columns-spanned` | Parsed (not fully used) |
| `table:number-rows-spanned` | Parsed (not fully used) |
| `style:vertical-align` (cell) | `VerticalAlign(enum)` |
| `fo:background-color` (cell) | `CellBackground(Color)` |

### IGNORED

| Element/Attribute | Impact |
|-------------------|--------|
| `style:table-properties` (most) | Medium — table-level formatting |
| `style:column-width` | Medium — explicit column widths |
| `table:table-header-rows` | Medium — repeating headers |
| `fo:border-*` (cell) | Medium — cell borders |
| `fo:padding-*` (cell) | Low — cell padding |
| `style:vertical-align` (table) | Low |
| `table:table-source` | Low — external data |
| `style:min-row-height` | Low — row height |
| `style:row-height` | Medium — explicit height |

---

## Lists

### HANDLED

| Element | Model Mapping | Notes |
|---------|---------------|-------|
| `text:list` | Flattened to paragraphs with `ListInfo` | |
| `text:list-item` | Children -> paragraphs at list level | |
| Nested `text:list` | Level incremented | |

### LIMITATIONS (BY DESIGN)

| Issue | Status | Impact |
|-------|--------|--------|
| All lists treated as Bullet | ODT-05 | Medium — no ordered lists |
| Multi-paragraph list items flattened | WONTFIX | Medium |
| `text:list-style` definitions | IGNORED | Medium — custom markers |
| `text:list-level-style-number` | IGNORED | Medium — numbering formats |
| `text:list-level-style-bullet` | IGNORED | Low — custom bullets |

---

## Page Layout

### HANDLED

| Element/Attribute | Model Mapping |
|-------------------|---------------|
| `fo:page-width` | `SectionProperties.page_width` |
| `fo:page-height` | `SectionProperties.page_height` |
| `style:print-orientation` | `SectionProperties.orientation` |
| `fo:margin-top/bottom/left/right` | `SectionProperties.margin_*` |
| `style:master-page` | Linked to page layout |
| `style:header` / `style:footer` | Header/footer content |

### IGNORED

| Element/Attribute | Impact |
|-------------------|--------|
| `fo:margin-inside/outside` | Medium — mirror margins |
| `fo:border-*` (page) | Low — page borders |
| `fo:padding-*` (page) | Low |
| `fo:background-*` (page) | Low — page background |
| `style:footnote-info` | Low — footnote layout |
| `style:page-usage` | Low |
| `style:num-format` (page) | Medium — page number format |
| `style:layout-grid-*` | Low — grid layout |

---

## Images & Frames

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `draw:frame` | Container -> Image node |
| `draw:image` | Image via `xlink:href` |
| `svg:width` / `svg:height` | `ImageWidth/Height` |
| `draw:name` | `ImageAltText` |
| Media extraction from `Pictures/` | `MediaStore` |

### IGNORED (WONTFIX — ODT-11)

| Element | Impact |
|---------|--------|
| `draw:custom-shape` | Medium — shapes |
| `draw:rect`, `draw:circle`, `draw:ellipse` | Low — geometric shapes |
| `draw:line`, `draw:polygon` | Low |
| `draw:text-box` | Medium — text boxes |
| `draw:g` (groups) | Low |
| `draw:connector` | Low |
| `text:anchor-type` (positioning) | Medium — float vs inline |
| `style:wrap` | Medium — text wrapping |
| `draw:z-index` | Low — z-ordering |
| SVG content | Low — vector graphics |

---

## Fields

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `text:page-number` | `FieldType::PageNumber` |
| `text:page-count` | `FieldType::PageCount` |
| `text:database-display` | `FieldType::Custom` (text preserved) |

### IGNORED

| Element | Impact |
|---------|--------|
| `text:date` | Medium |
| `text:time` | Medium |
| `text:file-name` | Low |
| `text:author-name` | Low |
| `text:subject`, `text:title` | Low |
| `text:sender-*` | Low |
| `text:chapter` | Medium |
| `text:variable-set/get` | Medium |
| `text:conditional-text` | Medium |
| `text:sequence` | Medium — figure numbering |

---

## Annotations (Comments)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `office:annotation` | `CommentStart` + `CommentBody` |
| `office:name` | `CommentId` |
| `dc:creator` | `CommentAuthor` |
| `dc:date` | `CommentDate` |
| `office:annotation-end` | `CommentEnd` |

---

## Footnotes & Endnotes

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `text:note` with `text:note-class="footnote"` | `FootnoteRef` + `FootnoteBody` |
| `text:note` with `text:note-class="endnote"` | `EndnoteRef` + `EndnoteBody` |
| `text:note-citation` | Citation number |
| `text:note-body` | Body paragraphs |

---

## Bookmarks

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `text:bookmark-start` | `BookmarkStart` with `BookmarkName` |
| `text:bookmark-end` | `BookmarkEnd` with `BookmarkName` |
| `text:bookmark` (collapsed) | Both Start + End |

---

## Change Tracking

### PARTIALLY HANDLED

| Element | Handling |
|---------|---------|
| `text:tracked-changes` | Raw XML captured and preserved |
| `text:changed-region` | Metadata extracted to `ChangeTrackingInfo` |

### IGNORED (No Semantic Parsing)

| Element | Impact |
|---------|--------|
| `text:insertion` | High — insert revision |
| `text:deletion` | High — delete revision |
| `text:format-change` | Medium — format revision |
| `text:change`, `text:change-start/end` | High — change markers |

---

## Styles

### HANDLED

| Element | Notes |
|---------|-------|
| `style:style` (paragraph family) | Parsed with all text/para properties |
| `style:style` (text family) | Character styles |
| `style:style` (table family) | Basic table styles |
| `style:parent-style-name` | Inheritance chain |
| `style:display-name` | Display name |
| Automatic styles (`office:automatic-styles`) | Merged into node AttributeMap |

### IGNORED

| Element | Impact |
|---------|--------|
| `style:default-style` | Medium — format defaults |
| `text:outline-style` | Low — outline numbering |
| `number:*` styles | Medium — number formatting |
| `draw:*` styles | Low — drawing styles |
| Conditional styles (`style:map`) | Medium — context-dependent styles |

---

## Metadata (meta.xml)

### HANDLED

| Element | Model Mapping |
|---------|---------------|
| `dc:title` | `metadata.title` |
| `dc:subject` | `metadata.subject` |
| `dc:description` | `metadata.description` |
| `dc:creator` / `meta:initial-creator` | `metadata.creator` |
| `meta:keyword` | `metadata.keywords[]` |
| `meta:creation-date` | `metadata.created` |
| `dc:date` | `metadata.modified` |
| `meta:editing-cycles` | `metadata.revision` |
| `dc:language` | `metadata.language` |

### IGNORED

| Element | Impact |
|---------|--------|
| `meta:generator` | Low |
| `meta:print-date` | Low |
| `meta:template` | Low |
| `meta:user-defined` | Medium — custom properties |
| `meta:document-statistic` | Low |

---

## Key Gaps (Ordered by Impact)

### HIGH IMPACT

1. **Change tracking** — Only raw XML preserved, no semantic parsing of insertions/deletions
2. **List numbering** — All lists treated as bullets, no ordered list support
3. **Cell borders** — Not parsed from style:table-cell-properties
4. **Default styles** — `style:default-style` ignored, affects base formatting

### MEDIUM IMPACT

5. **Text wrapping** — Image text wrap styles not parsed
6. **Image positioning** — `text:anchor-type` ignored (all images inline)
7. **Widow/orphan control** — `fo:widows`, `fo:orphans` ignored
8. **Writing mode** — `style:writing-mode` ignored (RTL/vertical layout)
9. **Column widths** — `style:column-width` in table-column styles partially handled
10. **Text transform** — `fo:text-transform` (uppercase/lowercase) ignored
11. **Page number format** — `style:num-format` in page layout ignored
12. **Date/time fields** — Common fields not parsed

### LOW IMPACT

13. **Text shadow/outline** — Decorative effects
14. **Cell padding** — `fo:padding-*` in cells
15. **Mirror margins** — `fo:margin-inside/outside`
16. **CJK-specific** — Asian font variants, emphasis marks, auto-spacing
