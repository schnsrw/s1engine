# OOXML WordprocessingML (DOCX) Complete Feature Checklist

> Comprehensive feature inventory derived from **ECMA-376 5th Edition / ISO 29500**
> (Office Open XML WordprocessingML -- Part 1, Chapters 11 and 17)
>
> Reference sources: ECMA-376 Standard, Microsoft OpenXML SDK documentation,
> c-rex.net OOXML reference, ooxml.info specification browser.

**Status Key**

| Symbol | Meaning |
|--------|---------|
| [ ]    | Not implemented |
| [~]    | Partially implemented |
| [x]    | Fully implemented |

---

## 1. Document Package Structure (OPC)

The DOCX file is a ZIP package conforming to Open Packaging Conventions (OPC).

### 1.1 Required/Common Parts

| Status | Part | Typical Path | Content Type |
|--------|------|-------------|--------------|
| [ ] | Main Document Part | `word/document.xml` | `application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml` |
| [ ] | Style Definitions Part | `word/styles.xml` | `...styles+xml` |
| [ ] | Numbering Definitions Part | `word/numbering.xml` | `...numbering+xml` |
| [ ] | Document Settings Part | `word/settings.xml` | `...settings+xml` |
| [ ] | Font Table Part | `word/fontTable.xml` | `...fontTable+xml` |
| [ ] | Theme Part | `word/theme/theme1.xml` | `...theme+xml` |
| [ ] | Web Settings Part | `word/webSettings.xml` | `...webSettings+xml` |
| [ ] | Comments Part | `word/comments.xml` | `...comments+xml` |
| [ ] | Footnotes Part | `word/footnotes.xml` | `...footnotes+xml` |
| [ ] | Endnotes Part | `word/endnotes.xml` | `...endnotes+xml` |
| [ ] | Header Part(s) | `word/header{N}.xml` | `...header+xml` |
| [ ] | Footer Part(s) | `word/footer{N}.xml` | `...footer+xml` |
| [ ] | Glossary Document Part | `word/glossary/document.xml` | `...document.glossary+xml` |
| [ ] | Alternative Format Import Part | (embedded content) | varies |
| [ ] | Relationships Parts | `word/_rels/document.xml.rels`, `_rels/.rels` | `...relationships+xml` |
| [ ] | Content Types Part | `[Content_Types].xml` | N/A (required by OPC) |
| [ ] | Core Properties Part | `docProps/core.xml` | Dublin Core metadata |
| [ ] | App Properties Part | `docProps/app.xml` | Extended properties |
| [ ] | Custom Properties Part | `docProps/custom.xml` | Custom metadata |
| [ ] | Image Parts | `word/media/image{N}.{ext}` | `image/png`, `image/jpeg`, etc. |
| [ ] | Embedded Object Parts | `word/embeddings/...` | varies |
| [ ] | Chart Parts | `word/charts/chart{N}.xml` | `...chart+xml` |

---

## 2. Document Structure (17.2 Main Document Story)

### 2.1 Top-Level Elements

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Document | `<w:document>` | Root element of document.xml |
| [ ] | Body | `<w:body>` | Container for all block-level content |
| [ ] | Background | `<w:background>` | Document background color/fill |

### 2.2 Block-Level Content (Body Children)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Paragraph | `<w:p>` | Primary text container |
| [ ] | Table | `<w:tbl>` | Table container |
| [ ] | Structured Document Tag (Block) | `<w:sdt>` | Block-level content control |
| [ ] | Custom XML Block | `<w:customXml>` | Custom XML data wrapper |
| [ ] | Section Properties (last) | `<w:sectPr>` | Final section properties (in body) |
| [ ] | Alternate Content | `<mc:AlternateContent>` | Markup compatibility fallback |
| [ ] | Bookmark Start/End | `<w:bookmarkStart>` / `<w:bookmarkEnd>` | Bookmark anchors |
| [ ] | Comment Range Start/End | `<w:commentRangeStart>` / `<w:commentRangeEnd>` | Comment anchors |
| [ ] | Proofing Error Start/End | `<w:proofErr>` | Spelling/grammar error markers |

---

## 3. Paragraphs & Rich Formatting (17.3)

### 3.1 Paragraph Structure

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Paragraph | `<w:p>` | Paragraph container |
| [ ] | Paragraph Properties | `<w:pPr>` | Paragraph formatting properties |
| [ ] | Run | `<w:r>` | Inline text run |
| [ ] | Hyperlink | `<w:hyperlink>` | Hyperlink wrapper |
| [ ] | Simple Field | `<w:fldSimple>` | Simple field code |
| [ ] | Structured Document Tag (Inline) | `<w:sdt>` | Inline content control |
| [ ] | Custom XML (Inline) | `<w:customXml>` | Inline custom XML |
| [ ] | Inserted Run Content | `<w:ins>` | Tracked insertion |
| [ ] | Deleted Run Content | `<w:del>` | Tracked deletion |
| [ ] | Move Source | `<w:moveFrom>` | Move source (track changes) |
| [ ] | Move Destination | `<w:moveTo>` | Move destination (track changes) |
| [ ] | Smart Tag | `<w:smartTag>` | Smart tag wrapper |
| [ ] | Sub-document | `<w:subDoc>` | Sub-document reference |

### 3.2 Paragraph Properties (`<w:pPr>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Paragraph Style | `<w:pStyle>` | Referenced paragraph style ID |
| [ ] | Keep With Next | `<w:keepNext>` | Keep paragraph with following paragraph |
| [ ] | Keep Lines Together | `<w:keepLines>` | Prevent page break within paragraph |
| [ ] | Page Break Before | `<w:pageBreakBefore>` | Start paragraph on new page |
| [ ] | Frame Properties | `<w:framePr>` | Text frame positioning properties |
| [ ] | Widow/Orphan Control | `<w:widowControl>` | Prevent widow/orphan lines |
| [ ] | Numbering Properties | `<w:numPr>` | List/numbering reference (`<w:ilvl>`, `<w:numId>`) |
| [ ] | Suppress Line Numbers | `<w:suppressLineNumbers>` | Exclude from line numbering |
| [ ] | Paragraph Borders | `<w:pBdr>` | Borders (top/bottom/left/right/between/bar) |
| [ ] | Shading | `<w:shd>` | Background shading/fill |
| [ ] | Tab Stops | `<w:tabs>` | Custom tab stop definitions |
| [ ] | Suppress Auto Hyphens | `<w:suppressAutoHyphens>` | Disable automatic hyphenation |
| [ ] | Kinsoku | `<w:kinsoku>` | East Asian line break control |
| [ ] | Word Wrap | `<w:wordWrap>` | Allow line break within word (CJK) |
| [ ] | Overflow Punctuation | `<w:overflowPunct>` | Allow punctuation overflow |
| [ ] | Top Line Punctuation | `<w:topLinePunct>` | Compress punctuation at line start |
| [ ] | Auto Space DE | `<w:autoSpaceDE>` | Auto-space between Latin and East Asian |
| [ ] | Auto Space DN | `<w:autoSpaceDN>` | Auto-space between numbers and East Asian |
| [ ] | BiDi | `<w:bidi>` | Right-to-left paragraph |
| [ ] | Adjust Right Indent | `<w:adjustRightInd>` | Automatically adjust right indent |
| [ ] | Snap To Grid | `<w:snapToGrid>` | Snap to document grid |
| [ ] | Spacing | `<w:spacing>` | Line spacing and before/after spacing |
| [ ] | Indentation | `<w:ind>` | Left, right, first-line, hanging indents |
| [ ] | Contextual Spacing | `<w:contextualSpacing>` | Ignore spacing between same-style paragraphs |
| [ ] | Mirror Indents | `<w:mirrorIndents>` | Swap left/right indents on odd/even pages |
| [ ] | Suppress Overlap | `<w:suppressOverlap>` | Prevent text frame overlap |
| [ ] | Justification | `<w:jc>` | Alignment (left, center, right, both, distribute) |
| [ ] | Text Direction | `<w:textDirection>` | Text flow direction |
| [ ] | Text Alignment | `<w:textAlignment>` | Vertical text alignment within line |
| [ ] | Text Box Tight Wrap | `<w:textboxTightWrap>` | Tight wrap behavior in text boxes |
| [ ] | Outline Level | `<w:outlineLvl>` | Outline/heading level (0-9) |
| [ ] | Div ID | `<w:divId>` | HTML div association |
| [ ] | Conditional Format Style | `<w:cnfStyle>` | Table conditional formatting |
| [ ] | Paragraph Mark Run Properties | `<w:rPr>` | Formatting of the paragraph mark character |
| [ ] | Section Properties | `<w:sectPr>` | Section break (inline in paragraph) |
| [ ] | Paragraph Properties Change | `<w:pPrChange>` | Track changes for paragraph properties |

### 3.3 Spacing Element (`<w:spacing>`) Attributes

| Status | Attribute | XML Attr | Description |
|--------|-----------|----------|-------------|
| [ ] | Before | `w:before` | Space before paragraph (twips) |
| [ ] | Before Auto Spacing | `w:beforeAutospacing` | Automatic before spacing |
| [ ] | After | `w:after` | Space after paragraph (twips) |
| [ ] | After Auto Spacing | `w:afterAutospacing` | Automatic after spacing |
| [ ] | Line | `w:line` | Line spacing value |
| [ ] | Line Rule | `w:lineRule` | Line spacing type: `auto`, `exact`, `atLeast` |

### 3.4 Indentation Element (`<w:ind>`) Attributes

| Status | Attribute | XML Attr | Description |
|--------|-----------|----------|-------------|
| [ ] | Left | `w:left` | Left indent (twips) |
| [ ] | Left Chars | `w:leftChars` | Left indent (character units) |
| [ ] | Right | `w:right` | Right indent (twips) |
| [ ] | Right Chars | `w:rightChars` | Right indent (character units) |
| [ ] | Hanging | `w:hanging` | Hanging indent (twips) |
| [ ] | Hanging Chars | `w:hangingChars` | Hanging indent (character units) |
| [ ] | First Line | `w:firstLine` | First-line indent (twips) |
| [ ] | First Line Chars | `w:firstLineChars` | First-line indent (character units) |

---

## 4. Run Properties (17.3 - `<w:rPr>` Children)

### 4.1 Core Run Properties (Office 2007+)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Run Style | `<w:rStyle>` | Referenced character style ID |
| [ ] | Run Fonts | `<w:rFonts>` | Font family (ascii, hAnsi, eastAsia, cs, theme) |
| [ ] | Bold | `<w:b>` | Bold |
| [ ] | Bold Complex Script | `<w:bCs>` | Bold for complex script text |
| [ ] | Italic | `<w:i>` | Italic |
| [ ] | Italic Complex Script | `<w:iCs>` | Italic for complex script text |
| [ ] | Caps | `<w:caps>` | All capitals display |
| [ ] | Small Caps | `<w:smallCaps>` | Small capitals display |
| [ ] | Strikethrough | `<w:strike>` | Single strikethrough |
| [ ] | Double Strikethrough | `<w:dstrike>` | Double strikethrough |
| [ ] | Outline | `<w:outline>` | Display character outline only |
| [ ] | Shadow | `<w:shadow>` | Shadow effect |
| [ ] | Emboss | `<w:emboss>` | Emboss effect |
| [ ] | Imprint | `<w:imprint>` | Engrave/imprint effect |
| [ ] | No Proofing | `<w:noProof>` | Suppress spell/grammar check |
| [ ] | Snap To Grid | `<w:snapToGrid>` | Use document grid for spacing |
| [ ] | Vanish (Hidden) | `<w:vanish>` | Hidden text |
| [ ] | Web Hidden | `<w:webHidden>` | Hidden in web view |
| [ ] | Color | `<w:color>` | Text color (val, themeColor, themeTint, themeShade) |
| [ ] | Character Spacing | `<w:spacing>` | Character spacing adjustment (twips) |
| [ ] | Character Scale | `<w:w>` | Horizontal text scaling (percentage) |
| [ ] | Kerning | `<w:kern>` | Font kerning threshold |
| [ ] | Position | `<w:position>` | Vertical text offset (raised/lowered) |
| [ ] | Font Size | `<w:sz>` | Font size (half-points) |
| [ ] | Font Size Complex Script | `<w:szCs>` | Font size for complex script |
| [ ] | Highlight | `<w:highlight>` | Text highlighting color |
| [ ] | Underline | `<w:u>` | Underline (see 4.2 for types) |
| [ ] | Text Effect (Animation) | `<w:effect>` | Legacy animated text effect |
| [ ] | Text Border | `<w:bdr>` | Border around text run |
| [ ] | Shading | `<w:shd>` | Background shading for run |
| [ ] | Fit Text | `<w:fitText>` | Compress run to specified width |
| [ ] | Vertical Alignment | `<w:vertAlign>` | Superscript / Subscript / Baseline |
| [ ] | RTL | `<w:rtl>` | Right-to-left text direction |
| [ ] | Complex Script | `<w:cs>` | Use complex script formatting |
| [ ] | Emphasis Mark | `<w:em>` | East Asian emphasis marks (dot, comma, circle, etc.) |
| [ ] | Languages | `<w:lang>` | Proofing languages (val, eastAsia, bidi) |
| [ ] | East Asian Layout | `<w:eastAsianLayout>` | East Asian typography settings |
| [ ] | Spec Vanish | `<w:specVanish>` | Paragraph mark always hidden |
| [ ] | Run Properties Change | `<w:rPrChange>` | Track changes for run properties |

### 4.2 Underline Types (ST_Underline)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `none` | No underline |
| [ ] | `single` | Single line |
| [ ] | `words` | Words only (skip spaces) |
| [ ] | `double` | Double line |
| [ ] | `thick` | Single thick line |
| [ ] | `dotted` | Dotted |
| [ ] | `dottedHeavy` | Thick dotted |
| [ ] | `dash` | Dashed |
| [ ] | `dashedHeavy` | Thick dashed |
| [ ] | `dashLong` | Long dashed |
| [ ] | `dashLongHeavy` | Thick long dashed |
| [ ] | `dotDash` | Dash-dot |
| [ ] | `dashDotHeavy` | Thick dash-dot |
| [ ] | `dotDotDash` | Dash-dot-dot |
| [ ] | `dashDotDotHeavy` | Thick dash-dot-dot |
| [ ] | `wave` | Wavy |
| [ ] | `wavyHeavy` | Thick wavy |
| [ ] | `wavyDouble` | Double wavy |

### 4.3 Office 2010+ Run Properties (w14 namespace)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Glow | `<w14:glow>` | Glow text effect |
| [ ] | Shadow (2010) | `<w14:shadow>` | Shadow text effect (enhanced) |
| [ ] | Reflection | `<w14:reflection>` | Reflection text effect |
| [ ] | Text Outline | `<w14:textOutline>` | Text outline effect |
| [ ] | Text Fill | `<w14:textFill>` | Text fill effect |
| [ ] | Scene 3D | `<w14:scene3d>` | 3D scene properties |
| [ ] | Properties 3D | `<w14:props3d>` | 3D text properties |
| [ ] | Ligatures | `<w14:ligatures>` | OpenType ligatures |
| [ ] | Number Form | `<w14:numForm>` | Number form (lining/oldStyle) |
| [ ] | Number Spacing | `<w14:numSpacing>` | Number spacing (proportional/tabular) |
| [ ] | Stylistic Sets | `<w14:stylisticSets>` | OpenType stylistic sets |
| [ ] | Contextual Alternates | `<w14:cntxtAlts>` | OpenType contextual alternates |

### 4.4 Run Content Elements

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Text | `<w:t>` | Text content |
| [ ] | Deleted Text | `<w:delText>` | Deleted text (track changes) |
| [ ] | Instruction Text | `<w:instrText>` | Field instruction text |
| [ ] | Deleted Instruction Text | `<w:delInstrText>` | Deleted field instruction |
| [ ] | Break | `<w:br>` | Break (page, column, line) |
| [ ] | Tab | `<w:tab>` | Tab character |
| [ ] | Symbol | `<w:sym>` | Symbol character |
| [ ] | Carriage Return | `<w:cr>` | Carriage return |
| [ ] | Soft Hyphen | `<w:softHyphen>` | Soft/optional hyphen |
| [ ] | Non-Breaking Hyphen | `<w:noBreakHyphen>` | Non-breaking hyphen |
| [ ] | Last Rendered Page Break | `<w:lastRenderedPageBreak>` | Cached page break position |
| [ ] | Drawing | `<w:drawing>` | DrawingML container (images, shapes) |
| [ ] | Object | `<w:object>` | OLE/embedded object |
| [ ] | Picture (VML) | `<w:pict>` | VML picture/shape |
| [ ] | Field Char | `<w:fldChar>` | Complex field character (begin/separate/end) |
| [ ] | Ruby | `<w:ruby>` | Ruby (phonetic guide) annotation |
| [ ] | Footnote Reference | `<w:footnoteReference>` | Footnote reference mark |
| [ ] | Endnote Reference | `<w:endnoteReference>` | Endnote reference mark |
| [ ] | Comment Reference | `<w:commentReference>` | Comment reference mark |
| [ ] | Footnote Ref Mark | `<w:footnoteRef>` | Auto-numbered footnote mark |
| [ ] | Endnote Ref Mark | `<w:endnoteRef>` | Auto-numbered endnote mark |
| [ ] | Separator | `<w:separator>` | Footnote/endnote separator |
| [ ] | Continuation Separator | `<w:continuationSeparator>` | Continuation separator |
| [ ] | Day Short | `<w:dayShort>` | Short day field |
| [ ] | Month Short | `<w:monthShort>` | Short month field |
| [ ] | Year Short | `<w:yearShort>` | Short year field |
| [ ] | Day Long | `<w:dayLong>` | Long day field |
| [ ] | Month Long | `<w:monthLong>` | Long month field |
| [ ] | Year Long | `<w:yearLong>` | Long year field |
| [ ] | Annotation Ref | `<w:annotationRef>` | Annotation reference mark |
| [ ] | Page Number | `<w:pgNum>` | Page number field |

---

## 5. Tables (17.4)

### 5.1 Table Structure

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Table | `<w:tbl>` | Table container |
| [ ] | Table Properties | `<w:tblPr>` | Table-level properties |
| [ ] | Table Grid | `<w:tblGrid>` | Column width definitions |
| [ ] | Grid Column | `<w:gridCol>` | Single column width |
| [ ] | Table Row | `<w:tr>` | Table row |
| [ ] | Table Row Properties | `<w:trPr>` | Row-level properties |
| [ ] | Table Cell | `<w:tc>` | Table cell |
| [ ] | Table Cell Properties | `<w:tcPr>` | Cell-level properties |
| [ ] | Nested Tables | (recursive `<w:tbl>`) | Tables within table cells |

### 5.2 Table Properties (`<w:tblPr>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Table Style | `<w:tblStyle>` | Referenced table style ID |
| [ ] | Table Position | `<w:tblpPr>` | Floating table position properties |
| [ ] | Table Overlap | `<w:tblOverlap>` | Allow table overlap |
| [ ] | BiDi Visual | `<w:bidiVisual>` | Right-to-left table |
| [ ] | Table Width | `<w:tblW>` | Preferred table width (type: auto/dxa/pct/nil) |
| [ ] | Table Justification | `<w:jc>` | Table alignment (left/center/right) |
| [ ] | Table Cell Spacing | `<w:tblCellSpacing>` | Cell spacing |
| [ ] | Table Indentation | `<w:tblInd>` | Table indent from leading margin |
| [ ] | Table Borders | `<w:tblBorders>` | Table borders (top/bottom/left/right/insideH/insideV) |
| [ ] | Shading | `<w:shd>` | Table background shading |
| [ ] | Table Layout | `<w:tblLayout>` | Layout algorithm (fixed/autofit) |
| [ ] | Cell Margin Defaults | `<w:tblCellMar>` | Default cell margins (top/bottom/start/end) |
| [ ] | Table Look | `<w:tblLook>` | Banded rows/cols, first/last row/col flags |
| [ ] | Table Caption | `<w:tblCaption>` | Accessibility caption (Office 2010+) |
| [ ] | Table Description | `<w:tblDescription>` | Accessibility description (Office 2010+) |
| [ ] | Table Properties Change | `<w:tblPrChange>` | Track changes for table props |

### 5.3 Table Row Properties (`<w:trPr>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Conditional Format Style | `<w:cnfStyle>` | Conditional formatting flags |
| [ ] | Div ID | `<w:divId>` | HTML div association |
| [ ] | Grid Before | `<w:gridBefore>` | Grid columns before first cell |
| [ ] | Grid After | `<w:gridAfter>` | Grid columns after last cell |
| [ ] | Width Before | `<w:wBefore>` | Preferred width before row |
| [ ] | Width After | `<w:wAfter>` | Preferred width after row |
| [ ] | Row Height | `<w:trHeight>` | Row height (exact/atLeast/auto) |
| [ ] | Hidden | `<w:hidden>` | Hidden row |
| [ ] | Can't Split | `<w:cantSplit>` | Prevent row from splitting across pages |
| [ ] | Table Header | `<w:tblHeader>` | Repeat row as header on each page |
| [ ] | Cell Spacing | `<w:tblCellSpacing>` | Row-level cell spacing override |
| [ ] | Justification | `<w:jc>` | Row alignment override |
| [ ] | Inserted | `<w:ins>` | Track change: row insertion |
| [ ] | Deleted | `<w:del>` | Track change: row deletion |
| [ ] | Row Properties Change | `<w:trPrChange>` | Track changes for row props |

### 5.4 Table Cell Properties (`<w:tcPr>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Conditional Format Style | `<w:cnfStyle>` | Conditional formatting flags |
| [ ] | Cell Width | `<w:tcW>` | Preferred cell width |
| [ ] | Grid Span | `<w:gridSpan>` | Number of grid columns spanned (horizontal merge) |
| [ ] | Horizontal Merge | `<w:hMerge>` | Horizontal merge (restart/continue) |
| [ ] | Vertical Merge | `<w:vMerge>` | Vertical merge (restart/continue) |
| [ ] | Cell Borders | `<w:tcBorders>` | Cell borders (top/bottom/start/end/insideH/insideV/tl2br/tr2bl) |
| [ ] | Shading | `<w:shd>` | Cell background shading |
| [ ] | No Wrap | `<w:noWrap>` | Don't wrap cell content |
| [ ] | Cell Margins | `<w:tcMar>` | Cell margin overrides |
| [ ] | Text Direction | `<w:textDirection>` | Text flow direction in cell |
| [ ] | Fit Text | `<w:tcFitText>` | Shrink text to fit cell |
| [ ] | Vertical Alignment | `<w:vAlign>` | Vertical alignment (top/center/bottom) |
| [ ] | Hide Mark | `<w:hideMark>` | Ignore end-of-cell marker height |
| [ ] | Cell Insertion | `<w:cellIns>` | Track change: cell insertion |
| [ ] | Cell Deletion | `<w:cellDel>` | Track change: cell deletion |
| [ ] | Cell Merge | `<w:cellMerge>` | Track change: cell merge |
| [ ] | Cell Properties Change | `<w:tcPrChange>` | Track changes for cell props |

### 5.5 Table Width Types (ST_TblWidth)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `auto` | Automatically determined |
| [ ] | `dxa` | Fixed width in twentieths of a point |
| [ ] | `nil` | No width (zero) |
| [ ] | `pct` | Width as percentage (in fiftieths of a percent) |

---

## 6. Sections & Page Layout (17.6)

### 6.1 Section Properties (`<w:sectPr>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Header Reference | `<w:headerReference>` | Header relationship (default/first/even) |
| [ ] | Footer Reference | `<w:footerReference>` | Footer relationship (default/first/even) |
| [ ] | Footnote Properties | `<w:footnotePr>` | Section footnote settings |
| [ ] | Endnote Properties | `<w:endnotePr>` | Section endnote settings |
| [ ] | Section Type | `<w:type>` | Section break type |
| [ ] | Page Size | `<w:pgSz>` | Page dimensions and orientation |
| [ ] | Page Margins | `<w:pgMar>` | Page margins (top/bottom/left/right/header/footer/gutter) |
| [ ] | Paper Source | `<w:paperSrc>` | Printer paper source |
| [ ] | Page Borders | `<w:pgBorders>` | Page border definitions |
| [ ] | Line Numbering | `<w:lnNumType>` | Line number settings |
| [ ] | Page Numbering | `<w:pgNumType>` | Page number format and start value |
| [ ] | Columns | `<w:cols>` | Column layout (num, space, individual col defs) |
| [ ] | Form Protection | `<w:formProt>` | Section-level form protection |
| [ ] | Vertical Text Align | `<w:vAlign>` | Vertical alignment on page |
| [ ] | Suppress Endnotes | `<w:noEndnote>` | Suppress endnotes in section |
| [ ] | Title Page | `<w:titlePage>` | Different first page header/footer |
| [ ] | Text Direction | `<w:textDirection>` | Section text flow direction |
| [ ] | BiDi | `<w:bidi>` | Right-to-left section |
| [ ] | Gutter on Right | `<w:rtlGutter>` | Right-side gutter |
| [ ] | Document Grid | `<w:docGrid>` | Document grid settings |
| [ ] | Printer Settings | `<w:printerSettings>` | Printer settings relationship |
| [ ] | Footnote Columns | `<w15:footnoteColumns>` | Footnote column count (Office 2013+) |
| [ ] | Section Properties Change | `<w:sectPrChange>` | Track changes for section props |

### 6.2 Section Break Types (ST_SectionMark)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `nextPage` | New section starts on next page (default) |
| [ ] | `continuous` | Continuous section break (same page) |
| [ ] | `evenPage` | New section starts on next even page |
| [ ] | `oddPage` | New section starts on next odd page |
| [ ] | `nextColumn` | New section starts in next column |

### 6.3 Page Size Attributes

| Status | Attribute | XML Attr | Description |
|--------|-----------|----------|-------------|
| [ ] | Width | `w:w` | Page width (twips) |
| [ ] | Height | `w:h` | Page height (twips) |
| [ ] | Orientation | `w:orient` | `portrait` or `landscape` |
| [ ] | Paper Code | `w:code` | Printer paper size code |

### 6.4 Page Margin Attributes

| Status | Attribute | XML Attr | Description |
|--------|-----------|----------|-------------|
| [ ] | Top | `w:top` | Top margin (twips) |
| [ ] | Bottom | `w:bottom` | Bottom margin (twips) |
| [ ] | Left | `w:left` | Left margin (twips) |
| [ ] | Right | `w:right` | Right margin (twips) |
| [ ] | Header | `w:header` | Header distance from edge (twips) |
| [ ] | Footer | `w:footer` | Footer distance from edge (twips) |
| [ ] | Gutter | `w:gutter` | Gutter width (twips) |

### 6.5 Document Grid Types

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `default` | No document grid |
| [ ] | `lines` | Line grid only |
| [ ] | `linesAndChars` | Character and line grid |
| [ ] | `snapToChars` | Snap to character grid |

---

## 7. Styles (17.7)

### 7.1 Styles Part Structure

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Styles Root | `<w:styles>` | Root element of styles.xml |
| [ ] | Document Defaults | `<w:docDefaults>` | Default paragraph/run properties |
| [ ] | Default Run Properties | `<w:rPrDefault>` | Default run formatting |
| [ ] | Default Paragraph Properties | `<w:pPrDefault>` | Default paragraph formatting |
| [ ] | Latent Styles | `<w:latentStyles>` | Latent style exception definitions |
| [ ] | Latent Style Exception | `<w:lsdException>` | Individual latent style override |
| [ ] | Style Definition | `<w:style>` | Individual style definition |

### 7.2 Style Types (`w:type` attribute)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `paragraph` | Paragraph style (applies to entire paragraph) |
| [ ] | `character` | Character/run style (applies to text runs) |
| [ ] | `table` | Table style (applies to tables) |
| [ ] | `numbering` | Numbering/list style |

### 7.3 Style Definition (`<w:style>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Name | `<w:name>` | Primary style name |
| [ ] | Aliases | `<w:aliases>` | Alternate style names |
| [ ] | Based On | `<w:basedOn>` | Parent style ID (inheritance) |
| [ ] | Next | `<w:next>` | Default style for next paragraph |
| [ ] | Link | `<w:link>` | Linked style reference (paragraph-character pair) |
| [ ] | Auto Redefine | `<w:autoRedefine>` | Auto-update style from manual formatting |
| [ ] | Hidden | `<w:hidden>` | Completely hidden from UI |
| [ ] | Semi-Hidden | `<w:semiHidden>` | Hidden from main UI |
| [ ] | UI Priority | `<w:uiPriority>` | Sort order in style picker |
| [ ] | Unhide When Used | `<w:unhideWhenUsed>` | Show when applied |
| [ ] | Quick Format | `<w:qFormat>` | Show in quick styles gallery |
| [ ] | Locked | `<w:locked>` | Style cannot be applied |
| [ ] | Personal | `<w:personal>` | E-mail message text style |
| [ ] | Personal Compose | `<w:personalCompose>` | E-mail composition style |
| [ ] | Personal Reply | `<w:personalReply>` | E-mail reply style |
| [ ] | Rsid | `<w:rsid>` | Revision save ID |
| [ ] | Paragraph Properties | `<w:pPr>` | Style paragraph formatting |
| [ ] | Run Properties | `<w:rPr>` | Style run formatting |
| [ ] | Table Properties | `<w:tblPr>` | Style table formatting |
| [ ] | Table Row Properties | `<w:trPr>` | Style row formatting |
| [ ] | Table Cell Properties | `<w:tcPr>` | Style cell formatting |
| [ ] | Table Style Properties | `<w:tblStylePr>` | Conditional table formatting |

### 7.4 Conditional Table Formatting Types (tblStylePr)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `firstRow` | First row formatting |
| [ ] | `lastRow` | Last row formatting |
| [ ] | `firstCol` | First column formatting |
| [ ] | `lastCol` | Last column formatting |
| [ ] | `band1Vert` | Odd vertical band |
| [ ] | `band2Vert` | Even vertical band |
| [ ] | `band1Horz` | Odd horizontal band |
| [ ] | `band2Horz` | Even horizontal band |
| [ ] | `neCell` | Top-right cell |
| [ ] | `nwCell` | Top-left cell |
| [ ] | `seCell` | Bottom-right cell |
| [ ] | `swCell` | Bottom-left cell |

---

## 8. Fonts (17.8)

### 8.1 Font Table Elements

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Font Table Root | `<w:fonts>` | Root of fontTable.xml |
| [ ] | Font Definition | `<w:font>` | Single font definition |
| [ ] | Alt Name | `<w:altName>` | Alternative font name |
| [ ] | Panose-1 | `<w:panose1>` | PANOSE classification |
| [ ] | Charset | `<w:charset>` | Character set |
| [ ] | Family | `<w:family>` | Font family classification |
| [ ] | Pitch | `<w:pitch>` | Fixed or variable pitch |
| [ ] | Signature | `<w:sig>` | Font signature |
| [ ] | Embed Regular | `<w:embedRegular>` | Embedded regular font |
| [ ] | Embed Bold | `<w:embedBold>` | Embedded bold font |
| [ ] | Embed Italic | `<w:embedItalic>` | Embedded italic font |
| [ ] | Embed Bold Italic | `<w:embedBoldItalic>` | Embedded bold italic font |

### 8.2 Run Font Selection (`<w:rFonts>` Attributes)

| Status | Attribute | XML Attr | Description |
|--------|-----------|----------|-------------|
| [ ] | ASCII | `w:ascii` | Font for ASCII characters |
| [ ] | High ANSI | `w:hAnsi` | Font for high ANSI characters |
| [ ] | East Asian | `w:eastAsia` | Font for East Asian characters |
| [ ] | Complex Script | `w:cs` | Font for complex script |
| [ ] | ASCII Theme | `w:asciiTheme` | Theme font for ASCII |
| [ ] | High ANSI Theme | `w:hAnsiTheme` | Theme font for high ANSI |
| [ ] | East Asian Theme | `w:eastAsiaTheme` | Theme font for East Asian |
| [ ] | Complex Script Theme | `w:cstheme` | Theme font for complex script |
| [ ] | Hint | `w:hint` | Font choice hint |

---

## 9. Numbering/Lists (17.9)

### 9.1 Numbering Part Structure

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Numbering Root | `<w:numbering>` | Root of numbering.xml |
| [ ] | Abstract Numbering Def | `<w:abstractNum>` | Abstract numbering definition |
| [ ] | Numbering Instance | `<w:num>` | Concrete numbering instance referencing an abstractNum |
| [ ] | Number Level Override | `<w:lvlOverride>` | Level override within num instance |
| [ ] | Start Override | `<w:startOverride>` | Restart numbering value |
| [ ] | Numbering ID Reference | `<w:numIdMacAtCleanup>` | Last numbering ID removed at cleanup |

### 9.2 Abstract Numbering Definition (`<w:abstractNum>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Numbering Style ID | `<w:nsid>` | Unique abstract number ID |
| [ ] | Multi-Level Type | `<w:multiLevelType>` | Type: `singleLevel`, `multilevel`, `hybridMultilevel` |
| [ ] | Template Code | `<w:tmpl>` | Template code for UI |
| [ ] | Name | `<w:name>` | Abstract num name |
| [ ] | Style Link | `<w:styleLink>` | Linked numbering style |
| [ ] | Num Style Link | `<w:numStyleLink>` | Reference to numbering style |
| [ ] | Level Definition | `<w:lvl>` | Level definition (0-8, up to 9 levels) |

### 9.3 Level Definition (`<w:lvl>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Start Value | `<w:start>` | Starting number |
| [ ] | Number Format | `<w:numFmt>` | Number format (see 9.4) |
| [ ] | Level Restart | `<w:lvlRestart>` | Restart level |
| [ ] | Paragraph Style | `<w:pStyle>` | Paragraph style association |
| [ ] | Is Legal Numbering | `<w:isLgl>` | Display as legal numbering |
| [ ] | Suffix | `<w:suff>` | Character after number (tab/space/nothing) |
| [ ] | Level Text | `<w:lvlText>` | Number text template (e.g., `%1.`) |
| [ ] | Level Picture Bullet | `<w:lvlPicBulletId>` | Picture bullet reference |
| [ ] | Legacy | `<w:legacy>` | Legacy numbering properties |
| [ ] | Level Justification | `<w:lvlJc>` | Number justification (left/center/right) |
| [ ] | Paragraph Properties | `<w:pPr>` | Level paragraph formatting (indent, tabs) |
| [ ] | Run Properties | `<w:rPr>` | Level text formatting |

### 9.4 Number Formats (ST_NumberFormat) -- Complete List

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `decimal` | 1, 2, 3, ... |
| [ ] | `upperRoman` | I, II, III, ... |
| [ ] | `lowerRoman` | i, ii, iii, ... |
| [ ] | `upperLetter` | A, B, C, ... |
| [ ] | `lowerLetter` | a, b, c, ... |
| [ ] | `ordinal` | 1st, 2nd, 3rd, ... |
| [ ] | `cardinalText` | One, Two, Three, ... |
| [ ] | `ordinalText` | First, Second, Third, ... |
| [ ] | `hex` | Hexadecimal |
| [ ] | `chicago` | Chicago Manual of Style |
| [ ] | `bullet` | Bullet character |
| [ ] | `none` | No numbering |
| [ ] | `numberInDash` | - 1 -, - 2 -, ... |
| [ ] | `decimalZero` | 01, 02, 03, ... |
| [ ] | `decimalFullWidth` | Full-width decimal |
| [ ] | `decimalFullWidth2` | Full-width decimal variant |
| [ ] | `decimalHalfWidth` | Half-width decimal |
| [ ] | `decimalEnclosedCircle` | Circled decimal |
| [ ] | `decimalEnclosedCircleChinese` | Chinese circled decimal |
| [ ] | `decimalEnclosedFullstop` | Decimal with fullstop |
| [ ] | `decimalEnclosedParen` | Decimal in parentheses |
| [ ] | `ideographDigital` | Ideograph digital |
| [ ] | `ideographTraditional` | Traditional ideograph |
| [ ] | `ideographLegalTraditional` | Legal traditional ideograph |
| [ ] | `ideographZodiac` | Zodiac ideograph |
| [ ] | `ideographZodiacTraditional` | Traditional zodiac ideograph |
| [ ] | `ideographEnclosedCircle` | Enclosed circle ideograph |
| [ ] | `chineseCounting` | Chinese counting |
| [ ] | `chineseCountingThousand` | Chinese counting (thousands) |
| [ ] | `chineseLegalSimplified` | Simplified Chinese legal |
| [ ] | `japaneseCounting` | Japanese counting |
| [ ] | `japaneseDigitalTenThousand` | Japanese digital ten-thousand |
| [ ] | `japaneseLegal` | Japanese legal |
| [ ] | `koreanCounting` | Korean counting |
| [ ] | `koreanDigital` | Korean digital |
| [ ] | `koreanDigital2` | Korean digital variant |
| [ ] | `koreanLegal` | Korean legal |
| [ ] | `taiwaneseCounting` | Taiwanese counting |
| [ ] | `taiwaneseCountingThousand` | Taiwanese counting (thousands) |
| [ ] | `taiwaneseDigital` | Taiwanese digital |
| [ ] | `ganada` | Korean Ganada |
| [ ] | `chosung` | Korean Chosung |
| [ ] | `aiueo` | Japanese Aiueo |
| [ ] | `aiueoFullWidth` | Full-width Aiueo |
| [ ] | `iroha` | Japanese Iroha |
| [ ] | `irohaFullWidth` | Full-width Iroha |
| [ ] | `hebrew1` | Hebrew 1 |
| [ ] | `hebrew2` | Hebrew 2 |
| [ ] | `arabicAlpha` | Arabic alphabetic |
| [ ] | `arabicAbjad` | Arabic Abjad |
| [ ] | `hindiVowels` | Hindi vowels |
| [ ] | `hindiConsonants` | Hindi consonants |
| [ ] | `hindiNumbers` | Hindi numbers |
| [ ] | `hindiCounting` | Hindi counting |
| [ ] | `thaiLetters` | Thai letters |
| [ ] | `thaiNumbers` | Thai numbers |
| [ ] | `thaiCounting` | Thai counting |
| [ ] | `vietnameseCounting` | Vietnamese counting |
| [ ] | `russianLower` | Russian lowercase |
| [ ] | `russianUpper` | Russian uppercase |

---

## 10. Headers & Footers (17.10)

### 10.1 Header/Footer Elements

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Header | `<w:hdr>` | Header root element (in header part) |
| [ ] | Footer | `<w:ftr>` | Footer root element (in footer part) |
| [ ] | Header Reference | `<w:headerReference>` | Links section to header part |
| [ ] | Footer Reference | `<w:footerReference>` | Links section to footer part |

### 10.2 Header/Footer Types

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `default` | Default header/footer |
| [ ] | `first` | First page header/footer (requires titlePage) |
| [ ] | `even` | Even page header/footer (requires evenAndOddHeaders) |

---

## 11. Footnotes & Endnotes (17.11)

### 11.1 Footnote/Endnote Part Elements

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Footnotes Root | `<w:footnotes>` | Root of footnotes.xml |
| [ ] | Endnotes Root | `<w:endnotes>` | Root of endnotes.xml |
| [ ] | Footnote | `<w:footnote>` | Single footnote definition |
| [ ] | Endnote | `<w:endnote>` | Single endnote definition |
| [ ] | Footnote Reference | `<w:footnoteReference>` | Inline reference to footnote |
| [ ] | Endnote Reference | `<w:endnoteReference>` | Inline reference to endnote |
| [ ] | Separator | `<w:separator>` | Footnote/endnote separator line |
| [ ] | Continuation Separator | `<w:continuationSeparator>` | Continuation separator |

### 11.2 Footnote/Endnote Properties

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Position | `<w:pos>` | Placement: `pageBottom`, `beneathText`, `sectEnd`, `docEnd` |
| [ ] | Number Format | `<w:numFmt>` | Numbering format for notes |
| [ ] | Number Start | `<w:numStart>` | Starting number |
| [ ] | Number Restart | `<w:numRestart>` | Restart rule: `continuous`, `eachSect`, `eachPage` |

### 11.3 Special Footnote/Endnote Types

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `normal` | Standard footnote/endnote |
| [ ] | `separator` | Separator line note |
| [ ] | `continuationSeparator` | Continuation separator |
| [ ] | `continuationNotice` | Continuation notice text |

---

## 12. Glossary Document (17.12)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Glossary Document | `<w:glossaryDocument>` | Root of glossary document part |
| [ ] | Document Parts | `<w:docParts>` | Container for building blocks |
| [ ] | Document Part | `<w:docPart>` | Single building block definition |
| [ ] | Doc Part Properties | `<w:docPartPr>` | Building block metadata |
| [ ] | Doc Part Body | `<w:docPartBody>` | Building block content |
| [ ] | Name | `<w:name>` | Building block name |
| [ ] | Category | `<w:category>` | Building block category |
| [ ] | Types | `<w:types>` | Building block type |
| [ ] | Behaviors | `<w:behaviors>` | Insertion behavior |
| [ ] | Description | `<w:description>` | Building block description |
| [ ] | GUID | `<w:guid>` | Unique identifier |

---

## 13. Annotations (17.13)

### 13.1 Comments

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Comments Root | `<w:comments>` | Root of comments.xml |
| [ ] | Comment | `<w:comment>` | Single comment definition |
| [ ] | Comment Range Start | `<w:commentRangeStart>` | Start of commented text |
| [ ] | Comment Range End | `<w:commentRangeEnd>` | End of commented text |
| [ ] | Comment Reference | `<w:commentReference>` | Inline comment marker |
| [ ] | Extended Comments | `<w15:commentsEx>` | Extended comments (replies, done state) |

### 13.2 Bookmarks

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Bookmark Start | `<w:bookmarkStart>` | Start of bookmark range |
| [ ] | Bookmark End | `<w:bookmarkEnd>` | End of bookmark range |

### 13.3 Range Permissions

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Permission Start | `<w:permStart>` | Start of editable range |
| [ ] | Permission End | `<w:permEnd>` | End of editable range |

### 13.4 Spelling/Grammar

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Proofing Error | `<w:proofErr>` | Proofing error marker (spellStart/spellEnd/gramStart/gramEnd) |

---

## 14. Track Changes / Revisions (17.13.5)

### 14.1 Content-Level Revisions

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Insertion | `<w:ins>` | Inserted content |
| [ ] | Deletion | `<w:del>` | Deleted content |
| [ ] | Deleted Text | `<w:delText>` | Text of deleted content |
| [ ] | Deleted Instruction Text | `<w:delInstrText>` | Deleted field instruction |
| [ ] | Move From | `<w:moveFrom>` | Source of moved content |
| [ ] | Move To | `<w:moveTo>` | Destination of moved content |
| [ ] | Move From Range Start | `<w:moveFromRangeStart>` | Start of move source range |
| [ ] | Move From Range End | `<w:moveFromRangeEnd>` | End of move source range |
| [ ] | Move To Range Start | `<w:moveToRangeStart>` | Start of move destination range |
| [ ] | Move To Range End | `<w:moveToRangeEnd>` | End of move destination range |

### 14.2 Property-Level Revisions

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Run Properties Change | `<w:rPrChange>` | Change in run formatting |
| [ ] | Paragraph Properties Change | `<w:pPrChange>` | Change in paragraph formatting |
| [ ] | Section Properties Change | `<w:sectPrChange>` | Change in section formatting |
| [ ] | Table Properties Change | `<w:tblPrChange>` | Change in table properties |
| [ ] | Table Row Properties Change | `<w:trPrChange>` | Change in row properties |
| [ ] | Table Cell Properties Change | `<w:tcPrChange>` | Change in cell properties |
| [ ] | Table Grid Change | `<w:tblGridChange>` | Change in table grid |
| [ ] | Table Properties Exception Change | `<w:tblPrExChange>` | Change in table exception props |
| [ ] | Numbering Change | `<w:numberingChange>` | Change in numbering |

### 14.3 Table-Level Revisions

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Cell Insertion | `<w:cellIns>` | Cell inserted |
| [ ] | Cell Deletion | `<w:cellDel>` | Cell deleted |
| [ ] | Cell Merge | `<w:cellMerge>` | Cell merge change |

### 14.4 Custom XML Revisions

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Custom XML Insert Range Start | `<w:customXmlInsRangeStart>` | Custom XML insertion start |
| [ ] | Custom XML Insert Range End | `<w:customXmlInsRangeEnd>` | Custom XML insertion end |
| [ ] | Custom XML Delete Range Start | `<w:customXmlDelRangeStart>` | Custom XML deletion start |
| [ ] | Custom XML Delete Range End | `<w:customXmlDelRangeEnd>` | Custom XML deletion end |
| [ ] | Custom XML Move From Range Start | `<w:customXmlMoveFromRangeStart>` | Custom XML move source start |
| [ ] | Custom XML Move From Range End | `<w:customXmlMoveFromRangeEnd>` | Custom XML move source end |
| [ ] | Custom XML Move To Range Start | `<w:customXmlMoveToRangeStart>` | Custom XML move dest start |
| [ ] | Custom XML Move To Range End | `<w:customXmlMoveToRangeEnd>` | Custom XML move dest end |

### 14.5 Revision Attributes (Common)

| Status | Attribute | XML Attr | Description |
|--------|-----------|----------|-------------|
| [ ] | Author | `w:author` | Revision author name |
| [ ] | Date | `w:date` | Revision timestamp |
| [ ] | Revision ID | `w:id` | Unique revision identifier |

---

## 15. Mail Merge (17.14)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Mail Merge | `<w:mailMerge>` | Mail merge settings container |
| [ ] | Main Document Type | `<w:mainDocumentType>` | Type: formLetters/emailMessage/envelope/label/fax/catalog |
| [ ] | Link To Query | `<w:linkToQuery>` | Link to external query |
| [ ] | Data Type | `<w:dataType>` | Data source type |
| [ ] | Connect String | `<w:connectString>` | Data source connection string |
| [ ] | Query | `<w:query>` | Data source query |
| [ ] | Data Source | `<w:dataSource>` | Data source reference |
| [ ] | Header Source | `<w:headerSource>` | Header data source |
| [ ] | Do Not Suppress Blank Lines | `<w:doNotSuppressBlankLines>` | Keep blank lines |
| [ ] | Destination | `<w:destination>` | Output destination |
| [ ] | Address Field Name | `<w:addressFieldName>` | Address field mapping |
| [ ] | Mail Subject | `<w:mailSubject>` | E-mail subject |
| [ ] | Mail As Attachment | `<w:mailAsAttachment>` | Send as attachment |
| [ ] | View Merged Data | `<w:viewMergedData>` | Display merged data |
| [ ] | Active Record | `<w:activeRecord>` | Active record number |
| [ ] | Check Errors | `<w:checkErrors>` | Error reporting level |
| [ ] | ODSO | `<w:odso>` | Office Data Source Object settings |
| [ ] | Field Map Data | `<w:fieldMapData>` | Field-to-column mappings |

---

## 16. Fields & Hyperlinks (17.16)

### 16.1 Field Mechanism Elements

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Simple Field | `<w:fldSimple>` | Simple (non-complex) field |
| [ ] | Field Char | `<w:fldChar>` | Complex field delimiter (begin/separate/end) |
| [ ] | Instruction Text | `<w:instrText>` | Field instruction code |
| [ ] | Field Code | `<w:fldData>` | Field private data |
| [ ] | Hyperlink | `<w:hyperlink>` | Hyperlink element |

### 16.2 Field Char Types

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `begin` | Start of complex field |
| [ ] | `separate` | Separator between code and result |
| [ ] | `end` | End of complex field |

### 16.3 All Field Types (17.16.5) -- Complete List (72 Fields)

#### Document Information Fields

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `AUTHOR` | Document author |
| [ ] | `COMMENTS` | Document comments |
| [ ] | `CREATEDATE` | Document creation date |
| [ ] | `DOCPROPERTY` | Custom document property |
| [ ] | `DOCVARIABLE` | Document variable value |
| [ ] | `EDITTIME` | Total editing time |
| [ ] | `FILENAME` | File name |
| [ ] | `FILESIZE` | File size |
| [ ] | `KEYWORDS` | Document keywords |
| [ ] | `LASTSAVEDBY` | Last saved by author |
| [ ] | `NUMCHARS` | Number of characters |
| [ ] | `NUMPAGES` | Number of pages |
| [ ] | `NUMWORDS` | Number of words |
| [ ] | `PRINTDATE` | Last print date |
| [ ] | `REVNUM` | Revision number |
| [ ] | `SAVEDATE` | Last save date |
| [ ] | `SUBJECT` | Document subject |
| [ ] | `TEMPLATE` | Template name |
| [ ] | `TITLE` | Document title |

#### Date & Time Fields

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `DATE` | Current date |
| [ ] | `TIME` | Current time |

#### Numbering & References

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `LISTNUM` | List number |
| [ ] | `PAGE` | Current page number |
| [ ] | `SECTION` | Current section number |
| [ ] | `SECTIONPAGES` | Number of pages in section |
| [ ] | `SEQ` | Sequence number (auto-numbered captions) |

#### Cross-References

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `HYPERLINK` | Hyperlink |
| [ ] | `NOTEREF` | Footnote/endnote number reference |
| [ ] | `PAGEREF` | Page number of bookmark |
| [ ] | `REF` | Bookmark content reference |
| [ ] | `STYLEREF` | Text from styled paragraph |

#### Table of Contents & Indexes

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `INDEX` | Index |
| [ ] | `RD` | Referenced document (for TOC/Index) |
| [ ] | `TA` | Table of Authorities entry |
| [ ] | `TC` | Table of Contents entry |
| [ ] | `TOA` | Table of Authorities |
| [ ] | `TOC` | Table of Contents |
| [ ] | `XE` | Index entry |

#### Mail Merge Fields

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `ADDRESSBLOCK` | Formatted address block |
| [ ] | `ASK` | Prompt user for bookmark value |
| [ ] | `COMPARE` | Compare two values |
| [ ] | `DATABASE` | Database query results |
| [ ] | `FILLIN` | Prompt for user input |
| [ ] | `GREETINGLINE` | Formatted greeting line |
| [ ] | `IF` | Conditional field |
| [ ] | `MERGEFIELD` | Mail merge data field |
| [ ] | `MERGEREC` | Mail merge record number |
| [ ] | `MERGESEQ` | Mail merge sequence number |
| [ ] | `NEXT` | Next mail merge record |
| [ ] | `NEXTIF` | Conditional next record |
| [ ] | `SET` | Set bookmark value |
| [ ] | `SKIPIF` | Skip current record |

#### Form Fields

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `FORMCHECKBOX` | Form checkbox |
| [ ] | `FORMDROPDOWN` | Form dropdown |
| [ ] | `FORMTEXT` | Form text input |

#### Insert/Include Fields

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `INCLUDEPICTURE` | Include picture from file |
| [ ] | `INCLUDETEXT` | Include text from file |
| [ ] | `LINK` | OLE link |
| [ ] | `QUOTE` | Literal text |

#### User Information

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `USERADDRESS` | User address |
| [ ] | `USERINITIALS` | User initials |
| [ ] | `USERNAME` | User name |

#### Other Fields

| Status | Field | Description |
|--------|-------|-------------|
| [ ] | `ADVANCE` | Adjust text position |
| [ ] | `AUTOTEXT` | AutoText entry |
| [ ] | `AUTOTEXTLIST` | AutoText list |
| [ ] | `BIBLIOGRAPHY` | Bibliography |
| [ ] | `CITATION` | Citation |
| [ ] | `GOTOBUTTON` | GoTo button |
| [ ] | `MACROBUTTON` | Macro button |
| [ ] | `PRINT` | Print instruction |
| [ ] | `PRIVATE` | Private data storage |
| [ ] | `SYMBOL` | Symbol character |

---

## 17. Document Settings (17.15)

### 17.1 Settings Elements (`<w:settings>` Children) -- Complete List

#### Display & View Settings

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Write Protection | `<w:writeProtection>` | Document write protection |
| [ ] | Document View | `<w:view>` | View mode (normal/web/print/outline/masterPages) |
| [ ] | Zoom | `<w:zoom>` | Zoom percentage and type |
| [ ] | Remove Personal Info | `<w:removePersonalInformation>` | Strip personal metadata on save |
| [ ] | Remove Date and Time | `<w:removeDateAndTime>` | Strip timestamps from annotations |
| [ ] | Do Not Display Page Boundaries | `<w:doNotDisplayPageBoundaries>` | Hide page boundaries |
| [ ] | Display Background Shape | `<w:displayBackgroundShape>` | Show background objects |

#### Print Settings

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Print PostScript Over Text | `<w:printPostScriptOverText>` | PostScript printing |
| [ ] | Print Fractional Char Width | `<w:printFractionalCharacterWidth>` | Fractional character widths |
| [ ] | Print Forms Data | `<w:printFormsData>` | Print only form data |
| [ ] | Print Two On One | `<w:printTwoOnOne>` | Two pages per sheet |

#### Font Embedding

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Embed TrueType Fonts | `<w:embedTrueTypeFonts>` | Embed fonts in document |
| [ ] | Embed System Fonts | `<w:embedSystemFonts>` | Include system fonts |
| [ ] | Save Subset Fonts | `<w:saveSubsetFonts>` | Subset embedded fonts |

#### Margin & Border Settings

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Mirror Margins | `<w:mirrorMargins>` | Mirror margins for binding |
| [ ] | Align Borders And Edges | `<w:alignBordersAndEdges>` | Align borders with page border |
| [ ] | Borders Do Not Surround Header | `<w:bordersDoNotSurroundHeader>` | Exclude header from page border |
| [ ] | Borders Do Not Surround Footer | `<w:bordersDoNotSurroundFooter>` | Exclude footer from page border |
| [ ] | Gutter At Top | `<w:gutterAtTop>` | Gutter at top of page |

#### Proofing & Grammar

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Hide Spelling Errors | `<w:hideSpellingErrors>` | Suppress spelling marks |
| [ ] | Hide Grammatical Errors | `<w:hideGrammaticalErrors>` | Suppress grammar marks |
| [ ] | Active Writing Style | `<w:activeWritingStyle>` | Grammar checking style |
| [ ] | Proof State | `<w:proofState>` | Spelling/grammar check state |

#### Template & Styles

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Attached Template | `<w:attachedTemplate>` | Associated document template |
| [ ] | Link Styles | `<w:linkStyles>` | Auto-update styles from template |
| [ ] | Style Pane Filter | `<w:stylePaneFormatFilter>` | Style list filtering |
| [ ] | Style Pane Sort | `<w:stylePaneSortMethod>` | Style list sort order |

#### Revision & Tracking

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Track Revisions | `<w:trackRevisions>` | Enable track changes |
| [ ] | Do Not Track Moves | `<w:doNotTrackMoves>` | Disable move tracking |
| [ ] | Do Not Track Formatting | `<w:doNotTrackFormatting>` | Disable format change tracking |
| [ ] | Revision View | `<w:revisionView>` | Visible annotation types |

#### Document Protection

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Document Protection | `<w:documentProtection>` | Editing restrictions |
| [ ] | Auto Format Override | `<w:autoFormatOverride>` | Allow auto-format in protected doc |
| [ ] | Style Lock Theme | `<w:styleLockTheme>` | Prevent theme changes |
| [ ] | Style Lock QF Set | `<w:styleLockQFSet>` | Prevent Quick Style changes |

#### General Document Settings

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Default Tab Stop | `<w:defaultTabStop>` | Default tab stop distance |
| [ ] | Auto Hyphenation | `<w:autoHyphenation>` | Automatic hyphenation |
| [ ] | Consecutive Hyphen Limit | `<w:consecutiveHyphenLimit>` | Max consecutive hyphens |
| [ ] | Hyphenation Zone | `<w:hyphenationZone>` | Hyphenation zone width |
| [ ] | Do Not Hyphenate Caps | `<w:doNotHyphenateCaps>` | Skip all-caps hyphenation |
| [ ] | Even And Odd Headers | `<w:evenAndOddHeaders>` | Enable even/odd headers |
| [ ] | Book Fold Printing | `<w:bookFoldPrinting>` | Booklet printing mode |
| [ ] | Book Fold Sheets | `<w:bookFoldPrintingSheets>` | Pages per booklet |
| [ ] | Save Forms Data | `<w:saveFormsData>` | Save only form data |
| [ ] | Forms Design | `<w:formsDesign>` | Form design mode |
| [ ] | Document Type | `<w:documentType>` | Document classification |
| [ ] | Save Preview Picture | `<w:savePreviewPicture>` | Generate thumbnail |
| [ ] | Update Fields | `<w:updateFields>` | Recalculate fields on open |
| [ ] | Character Spacing Control | `<w:characterSpacingControl>` | Whitespace compression |
| [ ] | Click And Type Style | `<w:clickAndTypeStyle>` | Auto-generated paragraph style |
| [ ] | Default Table Style | `<w:defaultTableStyle>` | Default table style ID |
| [ ] | Decimal Symbol | `<w:decimalSymbol>` | Radix point character |
| [ ] | List Separator | `<w:listSeparator>` | List separator character |

#### Drawing Grid

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Horizontal Grid Spacing | `<w:drawingGridHorizontalSpacing>` | Horizontal grid unit |
| [ ] | Vertical Grid Spacing | `<w:drawingGridVerticalSpacing>` | Vertical grid unit |
| [ ] | Horizontal Grid Lines | `<w:displayHorizontalDrawingGridEvery>` | Horizontal grid interval |
| [ ] | Vertical Grid Lines | `<w:displayVerticalDrawingGridEvery>` | Vertical grid interval |
| [ ] | No Margin Grid Origin | `<w:doNotUseMarginsForDrawingGridOrigin>` | Grid origin setting |
| [ ] | Horizontal Origin | `<w:drawingGridHorizontalOrigin>` | Grid horizontal origin |
| [ ] | Vertical Origin | `<w:drawingGridVerticalOrigin>` | Grid vertical origin |

#### Footnote/Endnote Document-Wide Settings

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Footnote Properties | `<w:footnotePr>` | Document-wide footnote settings |
| [ ] | Endnote Properties | `<w:endnotePr>` | Document-wide endnote settings |

#### Compatibility & Other

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Compatibility | `<w:compat>` | Compatibility settings (see 17.2) |
| [ ] | Document Variables | `<w:docVars>` | Named document variables |
| [ ] | Rsids | `<w:rsids>` | Revision save IDs |
| [ ] | Math Properties | `<w:mathPr>` | Office Math settings |
| [ ] | Theme Font Languages | `<w:themeFontLang>` | Theme font language mappings |
| [ ] | Color Scheme Mapping | `<w:clrSchemeMapping>` | Theme color to style mappings |
| [ ] | Shape Defaults | `<w:shapeDefaults>` | Default VML shape properties |
| [ ] | Hdr Shape Defaults | `<w:hdrShapeDefaults>` | Default shapes in headers |
| [ ] | Captions | `<w:captions>` | Caption settings |
| [ ] | Read Mode Ink Lock Down | `<w:readModeInkLockDown>` | Freeze layout settings |
| [ ] | Smart Tag Type | `<w:smartTagType>` | Smart tag metadata |
| [ ] | Schema Library | `<w:schemaLibrary>` | Custom XML schemas |

### 17.2 Compatibility Settings (`<w:compat>` Children)

Extensive list of boolean settings for compatibility with older Word versions:

| Status | Setting | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | `<w:useSingleBorderforContiguousCells>` | | Single border for adjacent cells |
| [ ] | `<w:wpJustification>` | | WordPerfect justification |
| [ ] | `<w:noTabHangInd>` | | No tab on hanging indent |
| [ ] | `<w:noLeading>` | | No leading between lines |
| [ ] | `<w:spaceForUL>` | | Add space for underline |
| [ ] | `<w:noColumnBalance>` | | Don't balance columns |
| [ ] | `<w:balanceSingleByteDoubleByteWidth>` | | Balance SB/DB widths |
| [ ] | `<w:noExtraLineSpacing>` | | No extra line spacing |
| [ ] | `<w:doNotLeaveBackslashAlone>` | | Backslash handling |
| [ ] | `<w:ulTrailSpace>` | | Underline trailing spaces |
| [ ] | `<w:doNotExpandShiftReturn>` | | Don't expand Shift+Return |
| [ ] | `<w:spacingInWholePoints>` | | Whole-point spacing only |
| [ ] | `<w:lineWrapLikeWord6>` | | Word 6 line wrapping |
| [ ] | `<w:printBodyTextBeforeHeader>` | | Print body before header |
| [ ] | `<w:printColBlack>` | | Print colors as black |
| [ ] | `<w:wpSpaceWidth>` | | WordPerfect space width |
| [ ] | `<w:showBreaksInFrames>` | | Show breaks in frames |
| [ ] | `<w:subFontBySize>` | | Substitute fonts by size |
| [ ] | `<w:suppressBottomSpacing>` | | Suppress bottom spacing |
| [ ] | `<w:suppressTopSpacing>` | | Suppress top spacing |
| [ ] | `<w:suppressSpacingAtTopOfPage>` | | Suppress top-of-page spacing |
| [ ] | `<w:suppressTopSpacingWP>` | | WP top spacing suppression |
| [ ] | `<w:suppressSpBfAfterPgBrk>` | | Suppress space after page break |
| [ ] | `<w:swapBordersFacingPages>` | | Swap borders on facing pages |
| [ ] | `<w:convMailMergeEsc>` | | Convert mail merge escapes |
| [ ] | `<w:truncateFontHeightsLikeWP6>` | | WP6 font height truncation |
| [ ] | `<w:mwSmallCaps>` | | MacWord small caps |
| [ ] | `<w:usePrinterMetrics>` | | Use printer metrics |
| [ ] | `<w:doNotSuppressParagraphBorders>` | | Don't suppress para borders |
| [ ] | `<w:wrapTrailSpaces>` | | Wrap trailing spaces |
| [ ] | `<w:footnoteLayoutLikeWW8>` | | WW8 footnote layout |
| [ ] | `<w:shapeLayoutLikeWW8>` | | WW8 shape layout |
| [ ] | `<w:alignTablesRowByRow>` | | Align tables row-by-row |
| [ ] | `<w:forgetLastTabAlignment>` | | Forget last tab alignment |
| [ ] | `<w:adjustLineHeightInTable>` | | Adjust line height in table |
| [ ] | `<w:autoSpaceLikeWord95>` | | Word 95 auto-spacing |
| [ ] | `<w:noSpaceRaiseLower>` | | No space for raise/lower |
| [ ] | `<w:doNotUseHTMLParagraphAutoSpacing>` | | Disable HTML auto-spacing |
| [ ] | `<w:layoutRawTableWidth>` | | Raw table width layout |
| [ ] | `<w:layoutTableRowsApart>` | | Layout table rows apart |
| [ ] | `<w:useWord97LineBreakRules>` | | Word 97 line break rules |
| [ ] | `<w:doNotBreakWrappedTables>` | | Don't break wrapped tables |
| [ ] | `<w:doNotSnapToGridInCell>` | | No grid snap in cells |
| [ ] | `<w:selectFldWithFirstOrLastChar>` | | Select field with char |
| [ ] | `<w:applyBreakingRules>` | | Apply breaking rules |
| [ ] | `<w:doNotWrapTextWithPunct>` | | Don't wrap with punctuation |
| [ ] | `<w:doNotUseEastAsianBreakRules>` | | No East Asian break rules |
| [ ] | `<w:useWord2002TableStyleRules>` | | Word 2002 table style rules |
| [ ] | `<w:growAutofit>` | | Allow autofit to grow |
| [ ] | `<w:useNormalStyleForList>` | | Normal style for lists |
| [ ] | `<w:doNotUseIndentAsNumberingTabStop>` | | Indent vs numbering tab |
| [ ] | `<w:useAltKinsokuLineBreakRules>` | | Alt Kinsoku rules |
| [ ] | `<w:allowSpaceOfSameStyleInTable>` | | Same-style spacing in table |
| [ ] | `<w:doNotSuppressIndentation>` | | Don't suppress indentation |
| [ ] | `<w:doNotAutofitConstrainedTables>` | | No autofit on constrained |
| [ ] | `<w:autofitToFirstFixedWidthCell>` | | Autofit to first fixed cell |
| [ ] | `<w:underlineTabInNumList>` | | Underline tab in list |
| [ ] | `<w:displayHangulFixedWidth>` | | Fixed-width Hangul display |
| [ ] | `<w:splitPgBreakAndParaMark>` | | Split page break and para mark |
| [ ] | `<w:doNotVertAlignCellWithSp>` | | No vert align with spacing |
| [ ] | `<w:doNotBreakConstrainedForcedTable>` | | Don't break forced tables |
| [ ] | `<w:doNotVertAlignInTxbx>` | | No vert align in textbox |
| [ ] | `<w:useAnsiKerningPairs>` | | ANSI kerning pairs |
| [ ] | `<w:cachedColBalance>` | | Cached column balance |
| [ ] | `<w:compatSetting>` | | Named compatibility setting (Office 2010+) |

---

## 18. Custom Markup (17.5)

### 18.1 Structured Document Tags (Content Controls)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | SDT Block | `<w:sdt>` (block) | Block-level content control |
| [ ] | SDT Inline/Run | `<w:sdt>` (inline) | Inline-level content control |
| [ ] | SDT Row | `<w:sdt>` (row) | Table row-level content control |
| [ ] | SDT Cell | `<w:sdt>` (cell) | Table cell-level content control |
| [ ] | SDT Properties | `<w:sdtPr>` | Content control properties |
| [ ] | SDT Content | `<w:sdtContent>` | Content control body |
| [ ] | SDT End Properties | `<w:sdtEndPr>` | End character formatting |

### 18.2 SDT Properties (`<w:sdtPr>` Children)

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Alias | `<w:alias>` | Friendly display name |
| [ ] | ID | `<w:id>` | Unique identifier |
| [ ] | Tag | `<w:tag>` | Programmatic tag value |
| [ ] | Lock | `<w:lock>` | Locking (sdtLocked/contentLocked/sdtContentLocked/unlocked) |
| [ ] | Placeholder | `<w:placeholder>` | Placeholder text reference |
| [ ] | Showing Placeholder | `<w:showingPlcHdr>` | Currently showing placeholder |
| [ ] | Data Binding | `<w:dataBinding>` | XML data binding (XPath) |
| [ ] | Temporary | `<w:temporary>` | Remove on first edit |
| [ ] | Run Properties | `<w:rPr>` | Content control formatting |

### 18.3 Content Control Types (sdtPr Type Elements)

| Status | Type | XML Tag | Description |
|--------|------|---------|-------------|
| [ ] | Rich Text | (no specific type element) | Default: rich text content |
| [ ] | Plain Text | `<w:text>` | Plain text only |
| [ ] | Combo Box | `<w:comboBox>` | Combo box (free text + list) |
| [ ] | Drop-Down List | `<w:dropDownList>` | Drop-down selection list |
| [ ] | Date Picker | `<w:date>` | Date picker control |
| [ ] | Picture | `<w:picture>` | Picture placeholder |
| [ ] | Document Part Gallery | `<w:docPartObj>` / `<w:docPartList>` | Building block gallery/list |
| [ ] | Group | `<w:group>` | Grouped content controls |
| [ ] | Checkbox | `<w14:checkbox>` | Checkbox (Office 2010+) |
| [ ] | Equation | `<w:equation>` | Equation content |
| [ ] | Citation | `<w:citation>` | Citation |
| [ ] | Bibliography | `<w:bibliography>` | Bibliography |
| [ ] | Repeating Section | `<w15:repeatingSection>` | Repeating section (Office 2013+) |
| [ ] | Repeating Section Item | `<w15:repeatingSectionItem>` | Repeating item (Office 2013+) |

### 18.4 Custom XML

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Custom XML Block | `<w:customXml>` | Block-level custom XML wrapper |
| [ ] | Custom XML Run | `<w:customXml>` (inline) | Inline custom XML wrapper |
| [ ] | Custom XML Properties | `<w:customXmlPr>` | Custom XML element properties |
| [ ] | Custom XML Attribute | `<w:attr>` | Custom XML attribute |

---

## 19. Images & Drawing (DrawingML in WordprocessingML)

### 19.1 Drawing Container

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Drawing | `<w:drawing>` | DrawingML container (in run) |
| [ ] | Inline Drawing | `<wp:inline>` | Inline-positioned drawing |
| [ ] | Anchor Drawing | `<wp:anchor>` | Floating/anchored drawing |

### 19.2 Inline Drawing (`<wp:inline>`) Properties

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Extent | `<wp:extent>` | Size (cx, cy in EMUs) |
| [ ] | Effect Extent | `<wp:effectExtent>` | Extra space for effects |
| [ ] | Doc Properties | `<wp:docPr>` | Drawing object properties (id, name, descr) |
| [ ] | Graphic Frame Props | `<wp:cNvGraphicFramePr>` | Non-visual graphic props |
| [ ] | Graphic | `<a:graphic>` | DrawingML graphic container |

### 19.3 Anchor Drawing (`<wp:anchor>`) Properties

| Status | Property | XML Tag/Attr | Description |
|--------|----------|-------------|-------------|
| [ ] | Simple Position | `simplePos` attr | Use simple positioning |
| [ ] | Relative Height | `relativeHeight` attr | Z-order |
| [ ] | Behind Document | `behindDoc` attr | Place behind text |
| [ ] | Locked | `locked` attr | Lock anchor position |
| [ ] | Layout In Cell | `layoutInCell` attr | Layout relative to cell |
| [ ] | Allow Overlap | `allowOverlap` attr | Allow overlap with others |
| [ ] | Simple Position Point | `<wp:simplePos>` | Fixed position coordinates |
| [ ] | Horizontal Position | `<wp:positionH>` | Horizontal positioning (relative to margin/page/column/character) |
| [ ] | Vertical Position | `<wp:positionV>` | Vertical positioning (relative to margin/page/paragraph/line) |
| [ ] | Extent | `<wp:extent>` | Size (cx, cy in EMUs) |
| [ ] | Effect Extent | `<wp:effectExtent>` | Effect margins |
| [ ] | Doc Properties | `<wp:docPr>` | Object properties |

### 19.4 Text Wrapping Types

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | No Wrapping | `<wp:wrapNone>` | No text wrapping (in front/behind) |
| [ ] | Square Wrapping | `<wp:wrapSquare>` | Wrap around bounding box |
| [ ] | Tight Wrapping | `<wp:wrapTight>` | Wrap tightly to shape contour |
| [ ] | Through Wrapping | `<wp:wrapThrough>` | Wrap through shape interior |
| [ ] | Top and Bottom | `<wp:wrapTopAndBottom>` | Text above and below only |

### 19.5 Drawing Content Types

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Picture | `<pic:pic>` | Picture/image |
| [ ] | Shape | `<wps:wsp>` | Word processing shape |
| [ ] | Group Shape | `<wpg:wGp>` | Group of shapes |
| [ ] | Canvas | `<wpc:wpc>` | Drawing canvas |
| [ ] | Diagram/SmartArt | `<dgm:relIds>` | SmartArt diagram reference |
| [ ] | Chart | `<c:chart>` | Chart reference |

### 19.6 Picture Elements (`<pic:pic>`)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Non-Visual Props | `<pic:nvPicPr>` | Non-visual picture properties |
| [ ] | Blip Fill | `<pic:blipFill>` | Image data and fill |
| [ ] | Shape Properties | `<pic:spPr>` | Shape (size, transform, geometry) |
| [ ] | Blip | `<a:blip>` | Image data reference (r:embed or r:link) |
| [ ] | Source Rectangle | `<a:srcRect>` | Crop rectangle |
| [ ] | Fill Rectangle | `<a:fillRect>` | Fill mode rectangle |
| [ ] | Stretch | `<a:stretch>` | Stretch fill mode |
| [ ] | Tile | `<a:tile>` | Tile fill mode |

### 19.7 Shape Properties (`<pic:spPr>` / `<wps:spPr>`)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Transform 2D | `<a:xfrm>` | Position and size transform |
| [ ] | Preset Geometry | `<a:prstGeom>` | Predefined shape geometry |
| [ ] | Custom Geometry | `<a:custGeom>` | Custom shape paths |
| [ ] | No Fill | `<a:noFill>` | No fill |
| [ ] | Solid Fill | `<a:solidFill>` | Solid color fill |
| [ ] | Gradient Fill | `<a:gradFill>` | Gradient fill |
| [ ] | Pattern Fill | `<a:pattFill>` | Pattern fill |
| [ ] | Blip Fill | `<a:blipFill>` | Image fill |
| [ ] | Line (outline) | `<a:ln>` | Shape outline/border |
| [ ] | Effect List | `<a:effectLst>` | Visual effects |
| [ ] | Effect DAG | `<a:effectDag>` | Effect directed graph |

---

## 20. VML (Legacy Shapes & Drawing)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Picture (VML) | `<w:pict>` | VML picture container |
| [ ] | Shape | `<v:shape>` | VML shape |
| [ ] | Shape Type | `<v:shapetype>` | VML shape type template |
| [ ] | Rectangle | `<v:rect>` | VML rectangle |
| [ ] | Oval | `<v:oval>` | VML oval |
| [ ] | Line | `<v:line>` | VML line |
| [ ] | Polyline | `<v:polyline>` | VML polyline |
| [ ] | Curve | `<v:curve>` | VML curve |
| [ ] | Group | `<v:group>` | VML shape group |
| [ ] | Image | `<v:image>` | VML image |
| [ ] | Text Box | `<v:textbox>` | VML text box |
| [ ] | Image Data | `<v:imagedata>` | VML image reference |
| [ ] | Fill | `<v:fill>` | VML fill |
| [ ] | Stroke | `<v:stroke>` | VML stroke |
| [ ] | Shadow | `<v:shadow>` | VML shadow |
| [ ] | Text Path | `<v:textpath>` | VML WordArt text path |
| [ ] | Wrap | `<w10:wrap>` | VML text wrapping |
| [ ] | Anchor Lock | `<w10:anchorlock>` | VML anchor lock |
| [ ] | OLE Object | `<o:OLEObject>` | Embedded OLE object |

### 20.1 Alternate Content (Markup Compatibility)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Alternate Content | `<mc:AlternateContent>` | Multiple representation container |
| [ ] | Choice | `<mc:Choice>` | Preferred representation |
| [ ] | Fallback | `<mc:Fallback>` | Fallback representation |

---

## 21. Themes (DrawingML)

### 21.1 Theme Structure

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Theme | `<a:theme>` | Root theme element |
| [ ] | Theme Elements | `<a:themeElements>` | Theme content container |
| [ ] | Color Scheme | `<a:clrScheme>` | Named color definitions |
| [ ] | Font Scheme | `<a:fontScheme>` | Theme font definitions |
| [ ] | Format Scheme | `<a:fmtScheme>` | Theme effects/formatting |
| [ ] | Object Defaults | `<a:objectDefaults>` | Default object styles |
| [ ] | Extra Color Scheme List | `<a:extraClrSchemeLst>` | Additional color schemes |

### 21.2 Theme Colors (within `<a:clrScheme>`)

| Status | Color | XML Tag | Description |
|--------|-------|---------|-------------|
| [ ] | Dark 1 | `<a:dk1>` | Main dark color (typically text) |
| [ ] | Light 1 | `<a:lt1>` | Main light color (typically background) |
| [ ] | Dark 2 | `<a:dk2>` | Secondary dark color |
| [ ] | Light 2 | `<a:lt2>` | Secondary light color |
| [ ] | Accent 1 | `<a:accent1>` | Accent color 1 |
| [ ] | Accent 2 | `<a:accent2>` | Accent color 2 |
| [ ] | Accent 3 | `<a:accent3>` | Accent color 3 |
| [ ] | Accent 4 | `<a:accent4>` | Accent color 4 |
| [ ] | Accent 5 | `<a:accent5>` | Accent color 5 |
| [ ] | Accent 6 | `<a:accent6>` | Accent color 6 |
| [ ] | Hyperlink | `<a:hlink>` | Hyperlink color |
| [ ] | Followed Hyperlink | `<a:folHlink>` | Visited hyperlink color |

### 21.3 Theme Fonts (within `<a:fontScheme>`)

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Major Fonts | `<a:majorFont>` | Heading fonts (latin, ea, cs, font list) |
| [ ] | Minor Fonts | `<a:minorFont>` | Body fonts (latin, ea, cs, font list) |

### 21.4 Color Scheme Mapping (`<w:clrSchemeMapping>` in settings.xml)

| Status | Attribute | Description |
|--------|-----------|-------------|
| [ ] | bg1 | Background 1 mapping |
| [ ] | tx1 | Text 1 mapping |
| [ ] | bg2 | Background 2 mapping |
| [ ] | tx2 | Text 2 mapping |
| [ ] | accent1-6 | Accent 1-6 mappings |
| [ ] | hyperlink | Hyperlink mapping |
| [ ] | followedHyperlink | Followed hyperlink mapping |

---

## 22. Office Math -- OMML (Chapter 22.1)

### 22.1 Math Containers

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Math Paragraph | `<m:oMathPara>` | Math paragraph (display mode) |
| [ ] | Math | `<m:oMath>` | Math expression |
| [ ] | Math Run | `<m:r>` | Math text run |
| [ ] | Math Text | `<m:t>` | Math text content |
| [ ] | Math Properties | `<m:oMathParaPr>` | Math paragraph properties |

### 22.2 Math Objects

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Accent | `<m:acc>` | Accent mark above expression |
| [ ] | Bar | `<m:bar>` | Overbar or underbar |
| [ ] | Box | `<m:box>` | Box around expression |
| [ ] | Border Box | `<m:borderBox>` | Bordered expression |
| [ ] | Delimiters | `<m:d>` | Delimited expression (parentheses, brackets) |
| [ ] | Equation Array | `<m:eqArr>` | Array of equations |
| [ ] | Fraction | `<m:f>` | Fraction (num/den) |
| [ ] | Math Function | `<m:func>` | Function application (sin, cos, etc.) |
| [ ] | Group Character | `<m:groupChr>` | Grouping character (brace, bracket) |
| [ ] | Lower Limit | `<m:limLow>` | Lower limit expression |
| [ ] | Upper Limit | `<m:limUpp>` | Upper limit expression |
| [ ] | Matrix | `<m:m>` | Matrix |
| [ ] | Matrix Row | `<m:mr>` | Matrix row |
| [ ] | N-ary Operator | `<m:nary>` | N-ary operator (sum, integral, product) |
| [ ] | Phantom | `<m:phant>` | Phantom (invisible spacing) |
| [ ] | Radical | `<m:rad>` | Radical/root |
| [ ] | Subscript | `<m:sSub>` | Subscript |
| [ ] | Superscript | `<m:sSup>` | Superscript |
| [ ] | Sub-Superscript | `<m:sSubSup>` | Subscript and superscript |
| [ ] | Pre-Sub-Superscript | `<m:sPre>` | Left subscript/superscript |

### 22.3 Math Element Parts

| Status | Element | XML Tag | Description |
|--------|---------|---------|-------------|
| [ ] | Numerator | `<m:num>` | Fraction numerator |
| [ ] | Denominator | `<m:den>` | Fraction denominator |
| [ ] | Base | `<m:e>` | Base expression |
| [ ] | Subscript | `<m:sub>` | Subscript expression |
| [ ] | Superscript | `<m:sup>` | Superscript expression |
| [ ] | Degree | `<m:deg>` | Degree (for radicals) |
| [ ] | Function Name | `<m:fName>` | Function name |
| [ ] | Limit | `<m:lim>` | Limit expression |

### 22.4 Math Run Properties

| Status | Property | XML Tag | Description |
|--------|----------|---------|-------------|
| [ ] | Math Font | `<m:rPr>` | Math run properties |
| [ ] | Literal | `<m:lit>` | Literal (no italic/spacing) |
| [ ] | Normal Text | `<m:nor>` | Normal (non-math) text |
| [ ] | Script | `<m:scr>` | Script type (roman, script, fraktur, etc.) |
| [ ] | Style | `<m:sty>` | Math style (plain, bold, italic, bold-italic) |
| [ ] | Break | `<m:brk>` | Math line break |
| [ ] | Alignment | `<m:aln>` | Alignment point |

---

## 23. Border Styles (ST_Border) -- Complete Line Styles

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `nil` | No border (remove) |
| [ ] | `none` | No border (never had one) |
| [ ] | `single` | Single line |
| [ ] | `thick` | Single thick line |
| [ ] | `double` | Double line |
| [ ] | `dotted` | Dotted line |
| [ ] | `dashed` | Dashed line |
| [ ] | `dotDash` | Alternating dot-dash |
| [ ] | `dotDotDash` | Dot-dot-dash |
| [ ] | `triple` | Triple line |
| [ ] | `thinThickSmallGap` | Thin-thick, small gap |
| [ ] | `thickThinSmallGap` | Thick-thin, small gap |
| [ ] | `thinThickThinSmallGap` | Thin-thick-thin, small gap |
| [ ] | `thinThickMediumGap` | Thin-thick, medium gap |
| [ ] | `thickThinMediumGap` | Thick-thin, medium gap |
| [ ] | `thinThickThinMediumGap` | Thin-thick-thin, medium gap |
| [ ] | `thinThickLargeGap` | Thin-thick, large gap |
| [ ] | `thickThinLargeGap` | Thick-thin, large gap |
| [ ] | `thinThickThinLargeGap` | Thin-thick-thin, large gap |
| [ ] | `wave` | Wavy line |
| [ ] | `doubleWave` | Double wavy line |
| [ ] | `dashSmallGap` | Dashed, small gap |
| [ ] | `dashDotStroked` | Alternating thin-thick strokes |
| [ ] | `threeDEmboss` | 3D embossed effect |
| [ ] | `threeDEngrave` | 3D engraved effect |
| [ ] | `outset` | Outset border |
| [ ] | `inset` | Inset border |
| [ ] | (200+ art borders) | Decorative art borders (apples, birds, etc.) |

---

## 24. Shading Patterns (ST_Shd)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `clear` | No pattern (use fill color only) |
| [ ] | `solid` | Solid (100%) |
| [ ] | `pct5` through `pct95` | Percentage fills (5%-95%) |
| [ ] | `horzStripe` | Horizontal stripe |
| [ ] | `vertStripe` | Vertical stripe |
| [ ] | `reverseDiagStripe` | Reverse diagonal stripe |
| [ ] | `diagStripe` | Diagonal stripe |
| [ ] | `horzCross` | Horizontal cross |
| [ ] | `diagCross` | Diagonal cross |
| [ ] | `thinHorzStripe` | Thin horizontal stripe |
| [ ] | `thinVertStripe` | Thin vertical stripe |
| [ ] | `thinReverseDiagStripe` | Thin reverse diagonal stripe |
| [ ] | `thinDiagStripe` | Thin diagonal stripe |
| [ ] | `thinHorzCross` | Thin horizontal cross |
| [ ] | `thinDiagCross` | Thin diagonal cross |
| [ ] | `nil` | No shading |

---

## 25. Highlight Colors (ST_HighlightColor)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `black` | Black |
| [ ] | `blue` | Blue |
| [ ] | `cyan` | Cyan |
| [ ] | `darkBlue` | Dark Blue |
| [ ] | `darkCyan` | Dark Cyan |
| [ ] | `darkGray` | Dark Gray |
| [ ] | `darkGreen` | Dark Green |
| [ ] | `darkMagenta` | Dark Magenta |
| [ ] | `darkRed` | Dark Red |
| [ ] | `darkYellow` | Dark Yellow |
| [ ] | `green` | Green |
| [ ] | `lightGray` | Light Gray |
| [ ] | `magenta` | Magenta |
| [ ] | `none` | No highlight |
| [ ] | `red` | Red |
| [ ] | `white` | White |
| [ ] | `yellow` | Yellow |

---

## 26. Embedded Objects & OLE

| Status | Feature | Description |
|--------|---------|-------------|
| [ ] | OLE Object | `<w:object>` + `<o:OLEObject>` - embedded OLE objects |
| [ ] | Embedded Package | Embedded files (xlsx, pptx, pdf, etc.) |
| [ ] | Linked Object | Externally linked OLE object |
| [ ] | Equation (OLE) | Legacy equation editor (Equation 3.0) |
| [ ] | ActiveX Controls | `<w:control>` - ActiveX embedded controls |

---

## 27. Miscellaneous Features (17.17)

### 27.1 Breaks

| Status | Type | `w:type` Value | Description |
|--------|------|----------------|-------------|
| [ ] | Page Break | `page` | Page break |
| [ ] | Column Break | `column` | Column break |
| [ ] | Line Break | `textWrapping` | Line break (with clear: none/left/right/all) |

### 27.2 Tab Stop Types

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `left` | Left-aligned tab |
| [ ] | `center` | Center-aligned tab |
| [ ] | `right` | Right-aligned tab |
| [ ] | `decimal` | Decimal-aligned tab |
| [ ] | `bar` | Bar tab (vertical line) |
| [ ] | `clear` | Clear existing tab |
| [ ] | `num` | List tab |

### 27.3 Tab Stop Leaders

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `none` | No leader |
| [ ] | `dot` | Dot leader (.....) |
| [ ] | `hyphen` | Hyphen leader (-----) |
| [ ] | `underscore` | Underscore leader (_____) |
| [ ] | `heavy` | Heavy/thick leader |
| [ ] | `middleDot` | Middle dot leader |

### 27.4 Text Effects (Legacy)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `blinkBackground` | Blinking background |
| [ ] | `lights` | Lights animation |
| [ ] | `antsBlack` | Marching ants (black) |
| [ ] | `antsRed` | Marching ants (red) |
| [ ] | `shimmer` | Shimmer effect |
| [ ] | `sparkle` | Sparkle effect |
| [ ] | `none` | No animation |

### 27.5 Vertical Text Alignment (Run)

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `baseline` | Normal baseline |
| [ ] | `superscript` | Superscript |
| [ ] | `subscript` | Subscript |

### 27.6 Emphasis Mark Types

| Status | Value | Description |
|--------|-------|-------------|
| [ ] | `none` | No emphasis |
| [ ] | `dot` | Filled dot above |
| [ ] | `comma` | Comma above |
| [ ] | `circle` | Open circle above |
| [ ] | `underDot` | Filled dot below |

---

## Summary Statistics

| Category | Approximate Feature Count |
|----------|--------------------------|
| Document Package Parts | 22 |
| Document Structure | 13 |
| Paragraph Properties | 36+ |
| Run Properties | 51+ (core) + 12 (w14) |
| Underline Types | 18 |
| Run Content Elements | 30+ |
| Table Properties | 16 (tblPr) + 15 (trPr) + 17 (tcPr) |
| Section/Page Layout | 22+ properties, 5 break types |
| Styles | 22 style children, 4 types, 12 conditional |
| Fonts | 12 definitions + 9 rFonts attrs |
| Numbering | 7 abstractNum + 12 level props + 60 numFmt |
| Headers/Footers | 4 elements, 3 types |
| Footnotes/Endnotes | 8 elements + properties |
| Glossary/Building Blocks | 10 elements |
| Comments & Bookmarks | 7 elements |
| Track Changes | 28+ revision elements |
| Mail Merge | 17 settings |
| Fields | 72 field types |
| Document Settings | 99+ settings + 57+ compat |
| Content Controls (SDT) | 14 types + 9 properties |
| Images/Drawing | 17+ elements, 5 wrap types |
| VML (Legacy) | 18+ elements |
| Themes | 12 colors + fonts + effects |
| Math (OMML) | 19 objects + 8 parts + 7 run props |
| Border Styles | 26 line + 200 art |
| Shading Patterns | 16 values |
| Highlight Colors | 17 values |
| **TOTAL** | **~800+ distinct features/elements** |

---

## References

- [ECMA-376 Standard](https://ecma-international.org/publications-and-standards/standards/ecma-376/)
- [ISO/IEC 29500 (LOC)](https://www.loc.gov/preservation/digital/formats/fdd/fdd000395.shtml)
- [Microsoft OpenXML SDK - ParagraphProperties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.paragraphproperties?view=openxml-3.0.1)
- [Microsoft OpenXML SDK - RunProperties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.wordprocessing.runproperties?view=openxml-3.0.1)
- [c-rex.net OOXML Reference](https://c-rex.net/samples/ooxml/e1/Part4/)
- [OOXML Info Specification Browser](https://ooxml.info/docs/17/)
- [Structure of a WordprocessingML Document](https://learn.microsoft.com/en-us/office/open-xml/word/structure-of-a-wordprocessingml-document)
- [OMML Math Elements](https://devblogs.microsoft.com/math-in-office/officemath/)
