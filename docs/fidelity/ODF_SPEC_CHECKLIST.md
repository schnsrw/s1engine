# ODF (OpenDocument Format) Text Document Feature Checklist

Comprehensive feature list derived from the **OASIS ODF 1.2/1.3** specification
(ISO/IEC 26300). Organized by category for implementation tracking in `s1-format-odt`.

**Legend**: Each item can be marked `[ ]` (not started), `[~]` (partial), `[x]` (complete).

---

## 1. Document Structure

### 1.1 Document Roots
- [ ] `office:document` — single-file XML document
- [ ] `office:document-content` — content.xml root
- [ ] `office:document-styles` — styles.xml root
- [ ] `office:document-meta` — meta.xml root
- [ ] `office:document-settings` — settings.xml root

### 1.2 Body & Text Container
- [ ] `office:body` — document body wrapper
- [ ] `office:text` — text document content
- [ ] `text:tracked-changes` — change tracking container (child of office:text)
- [ ] `text:sequence-decls` — sequence/variable declarations
- [ ] `text:variable-decls` — variable declarations
- [ ] `text:user-field-decls` — user field declarations

### 1.3 Sections
- [ ] `text:section` — document section
  - [ ] `text:section-source` — linked section source
  - [ ] Attribute: `text:name` — section name
  - [ ] Attribute: `text:style-name` — section style
  - [ ] Attribute: `text:display` — visibility (true/none/condition)
  - [ ] Attribute: `text:protection-key` — protection
  - [ ] Attribute: `text:condition` — display condition

### 1.4 Soft Page Breaks
- [ ] `text:soft-page-break` — automatic page break marker

---

## 2. Text Content — Headings & Paragraphs

### 2.1 Paragraphs
- [ ] `text:p` — paragraph element
  - [ ] Attribute: `text:style-name` — paragraph style reference
  - [ ] Attribute: `text:class-names` — multiple style classes
  - [ ] Attribute: `text:cond-style-name` — conditional style
  - [ ] Attribute: `text:id` — unique identifier (ODF 1.2+)

### 2.2 Headings
- [ ] `text:h` — heading element
  - [ ] Attribute: `text:outline-level` — heading level (1-10)
  - [ ] Attribute: `text:restart-numbering` — restart numbering
  - [ ] Attribute: `text:start-value` — starting number
  - [ ] Attribute: `text:is-list-header` — suppress numbering
  - [ ] Attribute: `text:style-name` — heading style reference

---

## 3. Paragraph Element Content (Inline)

### 3.1 Basic Text Content
- [ ] Plain text content (PCDATA)
- [ ] `text:s` — multiple space characters
  - [ ] Attribute: `text:c` — space count
- [ ] `text:tab` — tab character
  - [ ] Attribute: `text:tab-ref` — tab stop reference
- [ ] `text:line-break` — forced line break
- [ ] `text:soft-hyphen` — soft hyphen (U+00AD)

### 3.2 Spans
- [ ] `text:span` — character formatting span
  - [ ] Attribute: `text:style-name` — character style reference
  - [ ] Attribute: `text:class-names` — multiple style classes

### 3.3 Hyperlinks
- [ ] `text:a` — hyperlink
  - [ ] Attribute: `xlink:href` — target URL
  - [ ] Attribute: `xlink:type` — link type (simple)
  - [ ] Attribute: `office:name` — link name
  - [ ] Attribute: `office:target-frame-name` — target frame
  - [ ] Attribute: `text:style-name` — link style
  - [ ] Attribute: `text:visited-style-name` — visited link style

### 3.4 Ruby (East Asian Annotation)
- [ ] `text:ruby` — ruby container
  - [ ] `text:ruby-base` — base text
  - [ ] `text:ruby-text` — annotation text
  - [ ] Attribute: `text:style-name` — ruby style

### 3.5 Meta Inline
- [ ] `text:meta` — inline metadata container (ODF 1.2+, RDFa)
- [ ] `text:number` — generated list/heading number

---

## 4. Text Properties (`style:text-properties`)

### 4.1 Font Properties
- [ ] `fo:font-family` — font family name
- [ ] `style:font-name` — named font reference (from font-face-decls)
- [ ] `fo:font-size` — font size (pt, %, etc.)
- [ ] `style:font-size-rel` — relative font size adjustment
- [ ] `fo:font-style` — normal / italic / oblique
- [ ] `fo:font-weight` — normal / bold / 100-900
- [ ] `fo:font-variant` — normal / small-caps
- [ ] `style:font-style-name` — named font style reference
- [ ] `style:font-pitch` — fixed / variable
- [ ] `style:font-charset` — character set
- [ ] `style:font-family-generic` — generic family (roman, swiss, modern, decorative, script, system)

### 4.2 Font Properties — Asian Text
- [ ] `style:font-name-asian` — Asian font name
- [ ] `style:font-family-asian` — Asian font family
- [ ] `style:font-family-generic-asian` — generic Asian family
- [ ] `style:font-pitch-asian` — Asian font pitch
- [ ] `style:font-charset-asian` — Asian character set
- [ ] `style:font-size-asian` — Asian font size
- [ ] `style:font-size-rel-asian` — relative Asian font size
- [ ] `style:font-style-asian` — Asian font style
- [ ] `style:font-style-name-asian` — Asian font style name
- [ ] `style:font-weight-asian` — Asian font weight

### 4.3 Font Properties — Complex Script (CTL)
- [ ] `style:font-name-complex` — complex script font name
- [ ] `style:font-family-complex` — complex script font family
- [ ] `style:font-family-generic-complex` — generic complex family
- [ ] `style:font-pitch-complex` — complex font pitch
- [ ] `style:font-charset-complex` — complex character set
- [ ] `style:font-size-complex` — complex script font size
- [ ] `style:font-size-rel-complex` — relative complex font size
- [ ] `style:font-style-complex` — complex font style
- [ ] `style:font-style-name-complex` — complex font style name
- [ ] `style:font-weight-complex` — complex font weight

### 4.4 Color
- [ ] `fo:color` — text foreground color
- [ ] `fo:background-color` — text background/highlight color
- [ ] `style:use-window-font-color` — use system window font color

### 4.5 Underline
- [ ] `style:text-underline-style` — none / solid / dotted / dash / long-dash / dot-dash / dot-dot-dash / wave
- [ ] `style:text-underline-type` — none / single / double
- [ ] `style:text-underline-width` — auto / normal / bold / thin / medium / thick / length
- [ ] `style:text-underline-color` — font-color / color value
- [ ] `style:text-underline-mode` — continuous / skip-white-space

### 4.6 Overline
- [ ] `style:text-overline-style` — (same values as underline-style)
- [ ] `style:text-overline-type` — none / single / double
- [ ] `style:text-overline-width` — (same values as underline-width)
- [ ] `style:text-overline-color` — font-color / color value
- [ ] `style:text-overline-mode` — continuous / skip-white-space

### 4.7 Strikethrough (Line-Through)
- [ ] `style:text-line-through-style` — none / solid / dotted / dash / long-dash / dot-dash / dot-dot-dash / wave
- [ ] `style:text-line-through-type` — none / single / double
- [ ] `style:text-line-through-width` — (same values as underline-width)
- [ ] `style:text-line-through-color` — font-color / color value
- [ ] `style:text-line-through-mode` — continuous / skip-white-space
- [ ] `style:text-line-through-text` — replacement character (e.g., "/")
- [ ] `style:text-line-through-text-style` — style for replacement text

### 4.8 Text Position (Superscript / Subscript)
- [ ] `style:text-position` — "super" / "sub" / percentage + optional size percentage

### 4.9 Text Transform & Effects
- [ ] `fo:text-transform` — none / lowercase / uppercase / capitalize
- [ ] `fo:text-shadow` — shadow offset and color
- [ ] `style:text-outline` — outlined/hollow text (boolean)
- [ ] `fo:letter-spacing` — letter spacing (length)
- [ ] `style:letter-kerning` — enable/disable kerning (boolean)
- [ ] `style:text-blinking` — blinking text (boolean)
- [ ] `style:text-emphasize` — emphasis mark (none / accent / dot / circle / disc + above/below)
- [ ] `style:font-relief` — none / embossed / engraved
- [ ] `style:text-scale` — horizontal text scaling percentage
- [ ] `style:text-rotation-angle` — text rotation (0, 90, 270)
- [ ] `style:text-rotation-scale` — fixed / line-height

### 4.10 Text Combine (Warichu / Tate-chu-yoko)
- [ ] `style:text-combine` — none / letters / lines
- [ ] `style:text-combine-start-char` — bracket start char
- [ ] `style:text-combine-end-char` — bracket end char

### 4.11 Language & Locale
- [ ] `fo:language` — language code (e.g., "en")
- [ ] `fo:country` — country code (e.g., "US")
- [ ] `fo:script` — ISO 15924 script code
- [ ] `style:rfc-language-tag` — BCP 47 language tag
- [ ] `style:language-asian` — Asian language code
- [ ] `style:country-asian` — Asian country code
- [ ] `style:script-asian` — Asian script code
- [ ] `style:rfc-language-tag-asian` — Asian BCP 47 tag
- [ ] `style:language-complex` — complex script language code
- [ ] `style:country-complex` — complex script country code
- [ ] `style:script-complex` — complex script code
- [ ] `style:rfc-language-tag-complex` — complex BCP 47 tag

### 4.12 Hyphenation (Text-Level)
- [ ] `fo:hyphenate` — enable hyphenation (boolean)
- [ ] `fo:hyphenation-push-char-count` — minimum chars after hyphen
- [ ] `fo:hyphenation-remain-char-count` — minimum chars before hyphen

### 4.13 Conditional Display
- [ ] `text:condition` — conditional display expression
- [ ] `text:display` — true / none / condition

---

## 5. Paragraph Properties (`style:paragraph-properties`)

### 5.1 Alignment
- [ ] `fo:text-align` — start / end / left / center / right / justify
- [ ] `fo:text-align-last` — start / center / justify (last line of justified text)
- [ ] `style:justify-single-word` — justify single-word lines (boolean)

### 5.2 Indentation
- [ ] `fo:margin-left` — left indentation
- [ ] `fo:margin-right` — right indentation
- [ ] `fo:text-indent` — first line indent (positive or negative/hanging)
- [ ] `style:auto-text-indent` — automatic first-line indent (boolean)

### 5.3 Margins (Spacing Before/After)
- [ ] `fo:margin-top` — space before paragraph
- [ ] `fo:margin-bottom` — space after paragraph
- [ ] `fo:margin` — shorthand for all margins

### 5.4 Line Spacing
- [ ] `fo:line-height` — line height (length, percentage, or "normal")
- [ ] `style:line-height-at-least` — minimum line height
- [ ] `style:line-spacing` — additional line spacing
- [ ] `style:font-independent-line-spacing` — ignore font metrics for line height (boolean)

### 5.5 Pagination Control
- [ ] `fo:break-before` — auto / column / page / even-page / odd-page
- [ ] `fo:break-after` — auto / column / page / even-page / odd-page
- [ ] `fo:keep-together` — auto / always (prevent paragraph split)
- [ ] `fo:keep-with-next` — auto / always (keep with next paragraph)
- [ ] `fo:orphans` — minimum lines at bottom of page (integer)
- [ ] `fo:widows` — minimum lines at top of page (integer)
- [ ] `style:page-number` — starting page number (integer or "auto")

### 5.6 Borders
- [ ] `fo:border` — shorthand for all borders
- [ ] `fo:border-top` — top border (width style color)
- [ ] `fo:border-bottom` — bottom border
- [ ] `fo:border-left` — left border
- [ ] `fo:border-right` — right border
- [ ] `style:border-line-width` — double border line widths (all sides)
- [ ] `style:border-line-width-top` — double border top widths
- [ ] `style:border-line-width-bottom` — double border bottom widths
- [ ] `style:border-line-width-left` — double border left widths
- [ ] `style:border-line-width-right` — double border right widths
- [ ] `style:join-border` — merge adjacent paragraph borders (boolean)

### 5.7 Padding
- [ ] `fo:padding` — shorthand for all padding
- [ ] `fo:padding-top` — top padding
- [ ] `fo:padding-bottom` — bottom padding
- [ ] `fo:padding-left` — left padding
- [ ] `fo:padding-right` — right padding

### 5.8 Background & Shadow
- [ ] `fo:background-color` — paragraph background color
- [ ] `style:background-transparency` — background transparency percentage
- [ ] `style:shadow` — paragraph shadow (offset + color)

### 5.9 Tab Stops
- [ ] `style:tab-stops` — container for tab stop definitions
  - [ ] `style:tab-stop` — individual tab stop
    - [ ] Attribute: `style:position` — tab position (length)
    - [ ] Attribute: `style:type` — left / center / right / char
    - [ ] Attribute: `style:char` — decimal/alignment character
    - [ ] Attribute: `style:leader-style` — none / solid / dotted / dash / long-dash / dot-dash / dot-dot-dash
    - [ ] Attribute: `style:leader-type` — none / single / double
    - [ ] Attribute: `style:leader-width` — leader line width
    - [ ] Attribute: `style:leader-color` — leader line color
    - [ ] Attribute: `style:leader-text` — leader fill character
    - [ ] Attribute: `style:leader-text-style` — style for leader text
- [ ] `style:tab-stop-distance` — default tab stop interval

### 5.10 Drop Caps
- [ ] `style:drop-cap` — drop cap element
  - [ ] Attribute: `style:lines` — number of lines to drop
  - [ ] Attribute: `style:length` — number of characters or "word"
  - [ ] Attribute: `style:distance` — gap between drop cap and text
  - [ ] Attribute: `style:style-name` — style for drop cap text

### 5.11 Hyphenation (Paragraph-Level)
- [ ] `fo:hyphenation-keep` — auto / page (prevent hyphenation across pages)
- [ ] `fo:hyphenation-ladder-count` — max consecutive hyphenated lines

### 5.12 Writing Mode & BiDi
- [ ] `style:writing-mode` — lr-tb / rl-tb / tb-rl / tb-lr / page / lr / rl / tb
- [ ] `style:writing-mode-automatic` — automatic writing mode detection (boolean)

### 5.13 Line Numbering
- [ ] `text:number-lines` — include in line numbering (boolean)
- [ ] `text:line-number` — starting line number (integer)

### 5.14 Other Paragraph Properties
- [ ] `style:register-true` — snap to baseline grid (boolean)
- [ ] `style:snap-to-layout-grid` — snap to layout grid (boolean)
- [ ] `style:vertical-align` — top / middle / bottom / auto / baseline
- [ ] `style:text-autospace` — automatic spacing between ideographs/Latin
- [ ] `style:punctuation-wrap` — simple / hanging (CJK punctuation wrap)
- [ ] `style:line-break` — normal / strict (CJK line break rules)

### 5.15 Background Image (Paragraph)
- [ ] `style:background-image` — paragraph background image
  - [ ] Attribute: `xlink:href` — image URL
  - [ ] Attribute: `style:repeat` — no-repeat / repeat / stretch
  - [ ] Attribute: `style:position` — image position
  - [ ] Attribute: `style:filter-name` — image filter
  - [ ] Attribute: `draw:opacity` — image opacity

---

## 6. Lists

### 6.1 List Elements
- [ ] `text:list` — list container
  - [ ] Attribute: `text:style-name` — list style reference
  - [ ] Attribute: `text:continue-numbering` — continue previous list
  - [ ] Attribute: `text:continue-list` — continue specific list (by xml:id)
  - [ ] Attribute: `xml:id` — list identifier
- [ ] `text:list-item` — list item
  - [ ] Attribute: `text:start-value` — override numbering start
  - [ ] Attribute: `text:style-override` — override item style
- [ ] `text:list-header` — unnumbered list header item

### 6.2 Numbered Paragraphs (Outside Lists)
- [ ] `text:numbered-paragraph` — standalone numbered paragraph
  - [ ] Attribute: `text:list-id` — associated list
  - [ ] Attribute: `text:level` — list level
  - [ ] Attribute: `text:style-name` — list style
  - [ ] Attribute: `text:start-value` — start number

### 6.3 List Styles
- [ ] `text:list-style` — list style definition
  - [ ] Attribute: `style:name` — style name
  - [ ] Attribute: `style:display-name` — display name
  - [ ] Attribute: `text:consecutive-numbering` — consecutive numbering

### 6.4 List Level Style — Bullet
- [ ] `text:list-level-style-bullet` — bullet list level
  - [ ] Attribute: `text:level` — list level (1-10)
  - [ ] Attribute: `text:bullet-char` — bullet character
  - [ ] Attribute: `text:bullet-relative-size` — bullet relative size
  - [ ] Attribute: `text:style-name` — text style for bullet
  - [ ] Attribute: `style:num-prefix` — prefix before bullet
  - [ ] Attribute: `style:num-suffix` — suffix after bullet

### 6.5 List Level Style — Number
- [ ] `text:list-level-style-number` — numbered list level
  - [ ] Attribute: `text:level` — list level (1-10)
  - [ ] Attribute: `style:num-format` — 1 / a / A / i / I / empty
  - [ ] Attribute: `style:num-prefix` — prefix
  - [ ] Attribute: `style:num-suffix` — suffix (e.g., ".", ")")
  - [ ] Attribute: `text:start-value` — start number
  - [ ] Attribute: `text:display-levels` — number of displayed parent levels
  - [ ] Attribute: `text:style-name` — text style for number

### 6.6 List Level Style — Image
- [ ] `text:list-level-style-image` — image bullet list level
  - [ ] Attribute: `text:level` — list level
  - [ ] Attribute: `xlink:href` — image URL

### 6.7 List Level Properties
- [ ] `style:list-level-properties` — level formatting
  - [ ] Attribute: `text:space-before` — space before label
  - [ ] Attribute: `text:min-label-width` — minimum label width
  - [ ] Attribute: `text:min-label-distance` — min distance label-to-text
  - [ ] Attribute: `fo:text-align` — label alignment
  - [ ] Attribute: `text:list-level-position-and-space-mode` — label-alignment / label-width-and-position
- [ ] `style:list-level-label-alignment` — label alignment details
  - [ ] Attribute: `text:label-followed-by` — listtab / space / nothing
  - [ ] Attribute: `text:list-tab-stop-position` — tab position after label
  - [ ] Attribute: `fo:margin-left` — paragraph indent
  - [ ] Attribute: `fo:text-indent` — first line / label indent

### 6.8 Outline Style
- [ ] `text:outline-style` — outline numbering for headings
  - [ ] `text:outline-level-style` — per-level outline style
    - [ ] (same attributes as `text:list-level-style-number`)

---

## 7. Tables

### 7.1 Table Structure Elements
- [ ] `table:table` — table element
  - [ ] Attribute: `table:name` — table name
  - [ ] Attribute: `table:style-name` — table style
  - [ ] Attribute: `table:template-name` — table template
  - [ ] Attribute: `table:protected` — protection flag
  - [ ] Attribute: `table:protection-key` — protection key
  - [ ] Attribute: `table:print` — printable flag
  - [ ] Attribute: `xml:id` — unique identifier
- [ ] `table:table-column` — column definition
  - [ ] Attribute: `table:style-name` — column style
  - [ ] Attribute: `table:number-columns-repeated` — repeated columns
  - [ ] Attribute: `table:default-cell-style-name` — default cell style
  - [ ] Attribute: `table:visibility` — visible / collapse / filter
- [ ] `table:table-row` — table row
  - [ ] Attribute: `table:style-name` — row style
  - [ ] Attribute: `table:number-rows-repeated` — repeated rows
  - [ ] Attribute: `table:default-cell-style-name` — default cell style
  - [ ] Attribute: `table:visibility` — visible / collapse / filter
- [ ] `table:table-cell` — table cell
  - [ ] Attribute: `table:style-name` — cell style
  - [ ] Attribute: `table:number-columns-spanned` — column span
  - [ ] Attribute: `table:number-rows-spanned` — row span
  - [ ] Attribute: `table:content-validation-name` — validation
  - [ ] Attribute: `office:value-type` — cell value type
  - [ ] Attribute: `table:formula` — cell formula
  - [ ] Attribute: `table:protect` — cell protection
- [ ] `table:covered-table-cell` — spanned/covered cell

### 7.2 Table Grouping
- [ ] `table:table-header-rows` — repeating header rows
- [ ] `table:table-rows` — row container
- [ ] `table:table-row-group` — collapsible row group
- [ ] `table:table-header-columns` — repeating header columns
- [ ] `table:table-columns` — column container
- [ ] `table:table-column-group` — collapsible column group

### 7.3 Table Metadata
- [ ] `table:title` — table title (accessibility)
- [ ] `table:desc` — table description (accessibility)

### 7.4 Table Properties (`style:table-properties`)
- [ ] `style:width` — table width
- [ ] `style:rel-width` — relative table width (percentage)
- [ ] `table:align` — left / center / right / margins
- [ ] `table:border-model` — collapsing / separating
- [ ] `table:display` — table visibility (boolean)
- [ ] `fo:margin` / `fo:margin-top` / `fo:margin-bottom` / `fo:margin-left` / `fo:margin-right` — table margins
- [ ] `fo:background-color` — table background
- [ ] `fo:break-before` / `fo:break-after` — page/column breaks
- [ ] `fo:keep-with-next` — keep with next
- [ ] `style:may-break-between-rows` — allow row breaks (boolean)
- [ ] `style:page-number` — starting page number
- [ ] `style:shadow` — table shadow
- [ ] `style:writing-mode` — table writing direction

### 7.5 Table Column Properties (`style:table-column-properties`)
- [ ] `style:column-width` — column width
- [ ] `style:rel-column-width` — relative column width
- [ ] `style:use-optimal-column-width` — auto-fit width (boolean)
- [ ] `fo:break-before` / `fo:break-after` — column breaks

### 7.6 Table Row Properties (`style:table-row-properties`)
- [ ] `style:row-height` — row height
- [ ] `style:min-row-height` — minimum row height
- [ ] `style:use-optimal-row-height` — auto-fit height (boolean)
- [ ] `fo:background-color` — row background color
- [ ] `fo:break-before` / `fo:break-after` — row breaks
- [ ] `fo:keep-together` — keep row together

### 7.7 Table Cell Properties (`style:table-cell-properties`)
- [ ] `fo:background-color` — cell background color
- [ ] `fo:border` / `fo:border-top` / `fo:border-bottom` / `fo:border-left` / `fo:border-right` — cell borders
- [ ] `style:border-line-width` / `-top` / `-bottom` / `-left` / `-right` — double border widths
- [ ] `fo:padding` / `fo:padding-top` / `fo:padding-bottom` / `fo:padding-left` / `fo:padding-right` — cell padding
- [ ] `style:vertical-align` — top / middle / bottom / automatic
- [ ] `fo:wrap-option` — no-wrap / wrap
- [ ] `style:writing-mode` — cell writing mode
- [ ] `style:direction` — ltr / ttb
- [ ] `style:rotation-angle` — cell text rotation (degrees)
- [ ] `style:rotation-align` — none / bottom / top / center
- [ ] `style:cell-protect` — none / hidden-and-protected / protected / formula-hidden
- [ ] `style:print-content` — print cell content (boolean)
- [ ] `style:repeat-content` — repeat content to fill cell (boolean)
- [ ] `style:shrink-to-fit` — shrink text to fit (boolean)
- [ ] `style:text-align-source` — fix / value-type
- [ ] `style:diagonal-tl-br` — top-left to bottom-right diagonal border
- [ ] `style:diagonal-tl-br-widths` — diagonal line widths
- [ ] `style:diagonal-bl-tr` — bottom-left to top-right diagonal border
- [ ] `style:diagonal-bl-tr-widths` — diagonal line widths
- [ ] `style:glyph-orientation-vertical` — vertical glyph orientation
- [ ] `style:decimal-places` — displayed decimal places
- [ ] `style:shadow` — cell shadow

---

## 8. Styles

### 8.1 Style Containers
- [ ] `office:styles` — common/named styles (styles.xml)
- [ ] `office:automatic-styles` — automatic styles (content.xml / styles.xml)
- [ ] `office:master-styles` — master page styles (styles.xml)

### 8.2 Style Element
- [ ] `style:style` — style definition
  - [ ] Attribute: `style:name` — internal style name
  - [ ] Attribute: `style:display-name` — user-visible name
  - [ ] Attribute: `style:family` — text / paragraph / section / table / table-column / table-row / table-cell / graphic / presentation / drawing-page / chart / ruby
  - [ ] Attribute: `style:parent-style-name` — parent/base style
  - [ ] Attribute: `style:next-style-name` — next paragraph style
  - [ ] Attribute: `style:list-style-name` — associated list style
  - [ ] Attribute: `style:master-page-name` — associated master page
  - [ ] Attribute: `style:data-style-name` — number format
  - [ ] Attribute: `style:class` — style category
  - [ ] Attribute: `style:default-outline-level` — default heading level
  - [ ] Attribute: `style:auto-update` — auto-update from formatting (boolean)
  - [ ] Child: `style:text-properties`
  - [ ] Child: `style:paragraph-properties`
  - [ ] Child: `style:table-properties`
  - [ ] Child: `style:table-column-properties`
  - [ ] Child: `style:table-row-properties`
  - [ ] Child: `style:table-cell-properties`
  - [ ] Child: `style:section-properties`
  - [ ] Child: `style:graphic-properties`
  - [ ] Child: `style:ruby-properties`

### 8.3 Default Style
- [ ] `style:default-style` — default style for a family
  - [ ] Supported families: paragraph, text, section, table, table-column, table-row, table-cell, graphic, presentation, drawing-page, chart, ruby, control

### 8.4 Conditional Style Mapping
- [ ] `style:map` — conditional style application
  - [ ] Attribute: `style:condition` — condition expression
  - [ ] Attribute: `style:apply-style-name` — applied style when condition met

### 8.5 Font Face Declarations
- [ ] `style:font-face` — font face definition
  - [ ] Attribute: `style:name` — font reference name
  - [ ] Attribute: `svg:font-family` — font family
  - [ ] Attribute: `style:font-family-generic` — generic family
  - [ ] Attribute: `style:font-pitch` — fixed / variable
  - [ ] Attribute: `style:font-charset` — character set
  - [ ] Attribute: `svg:font-style` — font style
  - [ ] Attribute: `svg:font-variant` — font variant
  - [ ] Attribute: `svg:font-weight` — font weight
  - [ ] Attribute: `svg:font-size` — font size
  - [ ] Attribute: `svg:panose-1` — PANOSE classification

### 8.6 Master Pages
- [ ] `style:master-page` — master page definition
  - [ ] Attribute: `style:name` — master page name
  - [ ] Attribute: `style:display-name` — display name
  - [ ] Attribute: `style:page-layout-name` — associated page layout
  - [ ] Attribute: `style:next-style-name` — next master page
  - [ ] Attribute: `draw:style-name` — drawing page style
  - [ ] Child: `style:header` — header content
  - [ ] Child: `style:header-left` — left-page header
  - [ ] Child: `style:header-first` — first-page header (ODF 1.2+)
  - [ ] Child: `style:footer` — footer content
  - [ ] Child: `style:footer-left` — left-page footer
  - [ ] Child: `style:footer-first` — first-page footer (ODF 1.2+)

### 8.7 Header/Footer Styles
- [ ] `style:header-style` — header formatting container
- [ ] `style:footer-style` — footer formatting container
  - [ ] Child: `style:header-footer-properties`

### 8.8 Header/Footer Properties (`style:header-footer-properties`)
- [ ] `fo:background-color` — header/footer background
- [ ] `fo:border` / `fo:border-top` / `fo:border-bottom` / `fo:border-left` / `fo:border-right` — borders
- [ ] `style:border-line-width` / `-top` / `-bottom` / `-left` / `-right` — double border widths
- [ ] `fo:margin` / `fo:margin-top` / `fo:margin-bottom` / `fo:margin-left` / `fo:margin-right` — margins
- [ ] `fo:padding` / `fo:padding-top` / `fo:padding-bottom` / `fo:padding-left` / `fo:padding-right` — padding
- [ ] `fo:min-height` — minimum header/footer height
- [ ] `svg:height` — fixed header/footer height
- [ ] `style:dynamic-spacing` — dynamic spacing between header/footer and body (boolean)
- [ ] `style:shadow` — shadow effect

### 8.9 Data/Number Styles
- [ ] `number:number-style` — number format
- [ ] `number:currency-style` — currency format
- [ ] `number:percentage-style` — percentage format
- [ ] `number:date-style` — date format
- [ ] `number:time-style` — time format
- [ ] `number:boolean-style` — boolean format
- [ ] `number:text-style` — text format
  - [ ] Common child elements:
    - [ ] `number:number` — number placeholder
    - [ ] `number:text` — literal text
    - [ ] `number:day` / `number:month` / `number:year` — date parts
    - [ ] `number:hours` / `number:minutes` / `number:seconds` — time parts
    - [ ] `number:am-pm` — AM/PM indicator
    - [ ] `number:currency-symbol` — currency symbol
    - [ ] `number:fraction` — fraction formatting
    - [ ] `number:scientific-number` — scientific notation
    - [ ] `style:map` — conditional number format

---

## 9. Page Layout

### 9.1 Page Layout Element
- [ ] `style:page-layout` — page layout definition
  - [ ] Attribute: `style:name` — layout name
  - [ ] Attribute: `style:page-usage` — all / left / right / mirrored
  - [ ] Child: `style:page-layout-properties`
  - [ ] Child: `style:header-style`
  - [ ] Child: `style:footer-style`

### 9.2 Page Layout Properties (`style:page-layout-properties`)

#### Page Dimensions
- [ ] `fo:page-width` — page width
- [ ] `fo:page-height` — page height
- [ ] `style:print-orientation` — portrait / landscape

#### Page Margins
- [ ] `fo:margin` — shorthand for all margins
- [ ] `fo:margin-top` — top margin
- [ ] `fo:margin-bottom` — bottom margin
- [ ] `fo:margin-left` — left margin
- [ ] `fo:margin-right` — right margin

#### Page Borders
- [ ] `fo:border` / `fo:border-top` / `fo:border-bottom` / `fo:border-left` / `fo:border-right`
- [ ] `style:border-line-width` / `-top` / `-bottom` / `-left` / `-right`

#### Page Padding
- [ ] `fo:padding` / `fo:padding-top` / `fo:padding-bottom` / `fo:padding-left` / `fo:padding-right`

#### Page Background
- [ ] `fo:background-color` — page background color
- [ ] `style:background-image` child element — page background image

#### Page Numbering
- [ ] `style:num-format` — page number format (1, i, I, a, A)
- [ ] `style:num-prefix` — page number prefix
- [ ] `style:num-suffix` — page number suffix
- [ ] `style:num-letter-sync` — synchronize letters (boolean)
- [ ] `style:first-page-number` — starting page number

#### Columns
- [ ] `style:columns` — column container
  - [ ] Attribute: `fo:column-count` — number of columns
  - [ ] Attribute: `fo:column-gap` — gap between columns
  - [ ] Child: `style:column` — individual column definition
    - [ ] Attribute: `style:rel-width` — relative column width
    - [ ] Attribute: `fo:start-indent` — left spacing
    - [ ] Attribute: `fo:end-indent` — right spacing
  - [ ] Child: `style:column-sep` — column separator line
    - [ ] Attribute: `style:style` — none / solid / dotted / dashed / dot-dashed
    - [ ] Attribute: `style:width` — separator width
    - [ ] Attribute: `style:color` — separator color
    - [ ] Attribute: `style:height` — separator height (percentage)
    - [ ] Attribute: `style:vertical-align` — top / middle / bottom

#### Footnote Separator
- [ ] `style:footnote-sep` — footnote separator line
  - [ ] Attribute: `style:width` — line width
  - [ ] Attribute: `style:color` — line color
  - [ ] Attribute: `style:distance-before-sep` — space above
  - [ ] Attribute: `style:distance-after-sep` — space below
  - [ ] Attribute: `style:rel-width` — separator width (percentage)
  - [ ] Attribute: `style:adjustment` — left / center / right
  - [ ] Attribute: `style:line-style` — none / solid / dotted / dash / long-dash / dot-dash / dot-dot-dash

#### Layout Grid (CJK)
- [ ] `style:layout-grid-mode` — none / line / both
- [ ] `style:layout-grid-base-height` — grid base height
- [ ] `style:layout-grid-base-width` — grid base width
- [ ] `style:layout-grid-ruby-height` — ruby area height
- [ ] `style:layout-grid-ruby-below` — ruby below text (boolean)
- [ ] `style:layout-grid-lines` — lines per page
- [ ] `style:layout-grid-color` — grid color
- [ ] `style:layout-grid-display` — display grid (boolean)
- [ ] `style:layout-grid-print` — print grid (boolean)
- [ ] `style:layout-grid-snap-to` — snap to grid (boolean)
- [ ] `style:layout-grid-standard-mode` — standard grid mode (boolean)

#### Other Page Properties
- [ ] `style:footnote-max-height` — maximum footnote area height
- [ ] `style:writing-mode` — page writing direction
- [ ] `style:register-truth-ref-style-name` — baseline grid reference style
- [ ] `style:print` — print settings
- [ ] `style:print-page-order` — ltr / ttb
- [ ] `style:scale-to` — scaling percentage
- [ ] `style:scale-to-pages` — scale to fit N pages
- [ ] `style:paper-tray-name` — printer paper tray
- [ ] `style:table-centering` — table centering (none / horizontal / vertical / both)
- [ ] `style:shadow` — page shadow

---

## 10. Images, Frames & Drawing Objects

### 10.1 Frames
- [ ] `draw:frame` — frame container
  - [ ] Attribute: `draw:style-name` — graphic style
  - [ ] Attribute: `draw:name` — frame name
  - [ ] Attribute: `draw:z-index` — stacking order
  - [ ] Attribute: `draw:id` — unique identifier
  - [ ] Attribute: `draw:layer` — drawing layer
  - [ ] Attribute: `draw:transform` — transformation matrix
  - [ ] Attribute: `draw:text-style-name` — text style for frame text
  - [ ] Attribute: `draw:class-names` — style classes
  - [ ] Attribute: `draw:copy-of` — copy of another frame
  - [ ] Attribute: `draw:caption-id` — associated caption
  - [ ] Attribute: `svg:x` — horizontal position
  - [ ] Attribute: `svg:y` — vertical position
  - [ ] Attribute: `svg:width` — frame width
  - [ ] Attribute: `svg:height` — frame height
  - [ ] Attribute: `style:rel-width` — relative width
  - [ ] Attribute: `style:rel-height` — relative height
  - [ ] Attribute: `text:anchor-type` — as-char / char / paragraph / page / frame
  - [ ] Attribute: `text:anchor-page-number` — anchor page number
  - [ ] Attribute: `table:end-cell-address` — end cell (spreadsheet)
  - [ ] Attribute: `table:end-x` / `table:end-y` — end position
  - [ ] Attribute: `presentation:class` — presentation placeholder type
  - [ ] Attribute: `presentation:placeholder` — placeholder flag
  - [ ] Attribute: `presentation:user-transformed` — user-modified flag
  - [ ] Attribute: `presentation:style-name` — presentation style

### 10.2 Frame Children
- [ ] `draw:image` — image content
  - [ ] Attribute: `xlink:href` — image URL (in package or external)
  - [ ] Attribute: `xlink:type` — simple
  - [ ] Attribute: `xlink:show` — embed
  - [ ] Attribute: `xlink:actuate` — onLoad
  - [ ] Attribute: `draw:filter-name` — import filter
  - [ ] Alternative: embedded Base64 content as `office:binary-data`
- [ ] `draw:text-box` — text frame
  - [ ] Attribute: `draw:chain-next-name` — linked text box chain
  - [ ] Attribute: `fo:min-height` — minimum height
  - [ ] Attribute: `fo:max-height` — maximum height
  - [ ] Attribute: `fo:min-width` — minimum width
  - [ ] Attribute: `fo:max-width` — maximum width
- [ ] `draw:object` — embedded ODF object
  - [ ] Attribute: `xlink:href` — object URL
  - [ ] Attribute: `draw:notify-on-update-of-ranges` — cell range updates
- [ ] `draw:object-ole` — embedded OLE object
- [ ] `draw:applet` — Java applet (legacy)
- [ ] `draw:floating-frame` — floating frame / iframe
- [ ] `draw:plugin` — plugin content

### 10.3 Frame Metadata
- [ ] `svg:title` — title (accessibility alt text)
- [ ] `svg:desc` — description (accessibility)
- [ ] `draw:image-map` — clickable image map
  - [ ] `draw:area-rectangle` — rectangular area
  - [ ] `draw:area-circle` — circular area
  - [ ] `draw:area-polygon` — polygonal area
- [ ] `draw:glue-point` — connector glue point
- [ ] `draw:contour-polygon` — wrap contour (polygon)
- [ ] `draw:contour-path` — wrap contour (path)

### 10.4 Drawing Shapes (in text documents)
- [ ] `draw:rect` — rectangle
- [ ] `draw:line` — line
- [ ] `draw:polyline` — polyline
- [ ] `draw:polygon` — polygon
- [ ] `draw:regular-polygon` — regular polygon
- [ ] `draw:path` — SVG path
- [ ] `draw:circle` — circle
- [ ] `draw:ellipse` — ellipse
- [ ] `draw:connector` — connector line
- [ ] `draw:caption` — caption shape
- [ ] `draw:measure` — measurement line
- [ ] `draw:custom-shape` — custom/preset shape
  - [ ] `draw:enhanced-geometry` — shape geometry definition

### 10.5 Graphic Properties (`style:graphic-properties`)

#### Positioning & Anchoring
- [ ] `style:horizontal-pos` — from-left / left / center / right / from-inside / inside / outside
- [ ] `style:horizontal-rel` — page / page-content / page-start-margin / frame / paragraph / paragraph-content / char / page-end-margin
- [ ] `style:vertical-pos` — from-top / top / middle / bottom / below / from-inside / inside / outside
- [ ] `style:vertical-rel` — page / page-content / frame / paragraph / paragraph-content / char / line / baseline / text
- [ ] `text:anchor-type` — as-char / char / paragraph / page / frame
- [ ] `text:anchor-page-number` — page number

#### Wrapping
- [ ] `style:wrap` — none / left / right / parallel / dynamic / run-through
- [ ] `style:wrap-contour` — wrap to shape contour (boolean)
- [ ] `style:wrap-contour-mode` — full / outside
- [ ] `style:wrap-dynamic-threshold` — threshold for dynamic wrap
- [ ] `style:number-wrapped-paragraphs` — number of wrapped paragraphs
- [ ] `style:run-through` — foreground / background

#### Size Constraints
- [ ] `svg:width` / `svg:height` — shape dimensions
- [ ] `svg:x` / `svg:y` — shape position
- [ ] `style:rel-width` / `style:rel-height` — relative size
- [ ] `fo:min-width` / `fo:min-height` — minimum dimensions
- [ ] `fo:max-width` / `fo:max-height` — maximum dimensions
- [ ] `fo:clip` — clipping region

#### Margins & Padding
- [ ] `fo:margin` / `fo:margin-top` / `fo:margin-bottom` / `fo:margin-left` / `fo:margin-right`
- [ ] `fo:padding` / `fo:padding-top` / `fo:padding-bottom` / `fo:padding-left` / `fo:padding-right`

#### Borders
- [ ] `fo:border` / `fo:border-top` / `fo:border-bottom` / `fo:border-left` / `fo:border-right`
- [ ] `style:border-line-width` / `-top` / `-bottom` / `-left` / `-right`

#### Background & Shadow
- [ ] `fo:background-color`
- [ ] `style:background-transparency`
- [ ] `style:shadow`
- [ ] `draw:shadow` — visible / hidden
- [ ] `draw:shadow-offset-x` / `draw:shadow-offset-y`
- [ ] `draw:shadow-color`
- [ ] `draw:shadow-opacity`

#### Fill
- [ ] `draw:fill` — none / solid / gradient / bitmap / hatch
- [ ] `draw:fill-color` — solid fill color
- [ ] `draw:fill-gradient-name` — gradient reference
- [ ] `draw:fill-hatch-name` — hatch pattern reference
- [ ] `draw:fill-hatch-solid` — solid hatch fill
- [ ] `draw:fill-image-name` — image fill reference
- [ ] `draw:fill-image-width` / `draw:fill-image-height` — image fill dimensions
- [ ] `draw:fill-image-ref-point` — tile reference point
- [ ] `draw:fill-image-ref-point-x` / `draw:fill-image-ref-point-y`
- [ ] `draw:gradient-step-count` — gradient steps
- [ ] `draw:opacity` — overall opacity
- [ ] `draw:opacity-name` — named opacity reference
- [ ] `draw:secondary-fill-color` — secondary fill color
- [ ] `draw:tile-repeat-offset` — tile repetition offset

#### Stroke / Line
- [ ] `draw:stroke` — none / solid / dash
- [ ] `draw:stroke-dash` — dash pattern name
- [ ] `draw:stroke-dash-names` — dash pattern names
- [ ] `draw:stroke-linejoin` — round / bevel / miter / middle / none
- [ ] `svg:stroke-color` — stroke color
- [ ] `svg:stroke-width` — stroke width
- [ ] `svg:stroke-opacity` — stroke opacity
- [ ] `svg:stroke-linecap` — butt / round / square
- [ ] `draw:marker-start` / `draw:marker-end` — arrowhead names
- [ ] `draw:marker-start-width` / `draw:marker-end-width` — arrowhead sizes
- [ ] `draw:marker-start-center` / `draw:marker-end-center` — center marker

#### Image-Specific
- [ ] `draw:color-mode` — standard / greyscale / mono / watermark
- [ ] `draw:color-inversion` — invert colors (boolean)
- [ ] `draw:luminance` — brightness adjustment
- [ ] `draw:contrast` — contrast adjustment
- [ ] `draw:gamma` — gamma correction
- [ ] `draw:red` / `draw:green` / `draw:blue` — color channel adjustment
- [ ] `draw:image-opacity` — image opacity

#### Text in Shapes
- [ ] `draw:auto-grow-height` / `draw:auto-grow-width` — auto-grow to fit text
- [ ] `draw:fit-to-size` — fit text to shape
- [ ] `draw:fit-to-contour` — fit text to contour
- [ ] `draw:textarea-horizontal-align` — text area horizontal alignment
- [ ] `draw:textarea-vertical-align` — text area vertical alignment
- [ ] `fo:wrap-option` — text wrapping in shape

#### Miscellaneous Graphic Properties
- [ ] `style:mirror` — none / vertical / horizontal / both
- [ ] `style:print-content` — print content (boolean)
- [ ] `style:protect` — none / content / position / size (combinable)
- [ ] `style:editable` — editable (boolean)
- [ ] `style:flow-with-text` — flow with text (boolean)
- [ ] `style:overflow-behavior` — visible / hidden / auto-create-new-frame
- [ ] `style:shrink-to-fit` — shrink to fit (boolean)
- [ ] `style:writing-mode` — writing direction
- [ ] `style:repeat` — no-repeat / repeat / stretch
- [ ] `draw:wrap-influence-on-position` — wrapping influence mode
- [ ] `draw:visible-area-left` / `draw:visible-area-top` / `draw:visible-area-width` / `draw:visible-area-height` — OLE visible area

---

## 11. Text Fields

### 11.1 Document Fields
- [ ] `text:date` — current date field
  - [ ] Attribute: `text:date-value` — fixed date value
  - [ ] Attribute: `style:data-style-name` — date format
  - [ ] Attribute: `text:fixed` — fixed value (boolean)
- [ ] `text:time` — current time field
  - [ ] Attribute: `text:time-value` — fixed time value
  - [ ] Attribute: `style:data-style-name` — time format
  - [ ] Attribute: `text:fixed` — fixed value (boolean)
- [ ] `text:page-number` — current page number
  - [ ] Attribute: `style:num-format` — number format (1, i, I, a, A)
  - [ ] Attribute: `text:select-page` — previous / current / next
  - [ ] Attribute: `text:page-adjust` — page number offset
- [ ] `text:page-continuation` — page continuation marker ("continued...")
  - [ ] Attribute: `text:select-page` — previous / next
- [ ] `text:chapter` — chapter name/number
  - [ ] Attribute: `text:display` — name / number / number-and-name / plain-number / plain-number-and-name
  - [ ] Attribute: `text:outline-level` — outline level
- [ ] `text:file-name` — document file name
  - [ ] Attribute: `text:display` — full / path / name / name-and-extension
- [ ] `text:template-name` — template name
  - [ ] Attribute: `text:display` — full / path / name / name-and-extension
- [ ] `text:sheet-name` — spreadsheet sheet name

### 11.2 Sender Fields
- [ ] `text:sender-firstname`
- [ ] `text:sender-lastname`
- [ ] `text:sender-initials`
- [ ] `text:sender-title`
- [ ] `text:sender-position`
- [ ] `text:sender-email`
- [ ] `text:sender-phone-private`
- [ ] `text:sender-fax`
- [ ] `text:sender-company`
- [ ] `text:sender-phone-work`
- [ ] `text:sender-street`
- [ ] `text:sender-city`
- [ ] `text:sender-postal-code`
- [ ] `text:sender-country`
- [ ] `text:sender-state-or-province`

### 11.3 Author Fields
- [ ] `text:author-name` — document author name
- [ ] `text:author-initials` — document author initials

### 11.4 Variable Fields
- [ ] `text:variable-decls` — variable declarations container
- [ ] `text:variable-decl` — variable declaration
- [ ] `text:variable-set` — set variable value
- [ ] `text:variable-get` — display variable value
- [ ] `text:variable-input` — variable input field
- [ ] `text:user-field-decls` — user field declarations container
- [ ] `text:user-field-decl` — user field declaration
- [ ] `text:user-field-get` — display user field
- [ ] `text:user-field-input` — user field input
- [ ] `text:sequence-decls` — sequence declarations container
- [ ] `text:sequence-decl` — sequence declaration
- [ ] `text:sequence` — sequence/auto-number field
- [ ] `text:expression` — expression field
- [ ] `text:text-input` — text input field

### 11.5 Metadata Fields
- [ ] `text:initial-creator` — document creator
- [ ] `text:creation-date` — creation date
- [ ] `text:creation-time` — creation time
- [ ] `text:description` — document description
- [ ] `text:user-defined` — custom metadata field
- [ ] `text:print-time` — last print time
- [ ] `text:print-date` — last print date
- [ ] `text:printed-by` — printed by
- [ ] `text:title` — document title
- [ ] `text:subject` — document subject
- [ ] `text:keywords` — document keywords
- [ ] `text:editing-cycles` — editing cycles count
- [ ] `text:editing-duration` — total editing duration
- [ ] `text:modification-time` — last modification time
- [ ] `text:modification-date` — last modification date
- [ ] `text:creator` — last modifier

### 11.6 Document Statistics Fields
- [ ] `text:page-count` — total page count
- [ ] `text:paragraph-count` — paragraph count
- [ ] `text:word-count` — word count
- [ ] `text:character-count` — character count
- [ ] `text:table-count` — table count
- [ ] `text:image-count` — image count
- [ ] `text:object-count` — object count

### 11.7 Database Fields
- [ ] `form:connection-resource` — database connection
- [ ] `text:database-display` — display database field
- [ ] `text:database-next` — next record
- [ ] `text:database-row-select` — select row
- [ ] `text:database-row-number` — row number
- [ ] `text:database-name` — database name

### 11.8 Page Variable Fields
- [ ] `text:page-variable-set` — set page variable
- [ ] `text:page-variable-get` — get page variable

### 11.9 Other Fields
- [ ] `text:placeholder` — placeholder field
- [ ] `text:conditional-text` — conditional text display
- [ ] `text:hidden-text` — conditionally hidden text
- [ ] `text:hidden-paragraph` — conditionally hidden paragraph
- [ ] `text:reference-ref` — reference field
- [ ] `text:bookmark-ref` — bookmark reference
- [ ] `text:note-ref` — note reference
- [ ] `text:sequence-ref` — sequence reference
- [ ] `text:script` — script field
- [ ] `text:execute-macro` — macro execution
- [ ] `text:dde-connection` — DDE connection field
- [ ] `text:measure` — measurement field
- [ ] `text:table-formula` — table formula (deprecated)
- [ ] `text:meta-field` — RDF metadata field (ODF 1.2+)

---

## 12. Annotations & Notes

### 12.1 Annotations (Comments)
- [ ] `office:annotation` — annotation/comment
  - [ ] Attribute: `office:name` — annotation name (ODF 1.2+)
  - [ ] Child: `dc:creator` — comment author
  - [ ] Child: `dc:date` — comment date
  - [ ] Child: `text:p` — comment text content (one or more)
- [ ] `office:annotation-end` — end marker for range annotation (ODF 1.2+)
  - [ ] Attribute: `office:name` — matching annotation name

### 12.2 Footnotes & Endnotes
- [ ] `text:note` — footnote or endnote
  - [ ] Attribute: `text:id` — unique note identifier
  - [ ] Attribute: `text:note-class` — footnote / endnote
  - [ ] Child: `text:note-citation` — note reference mark
    - [ ] Attribute: `text:label` — custom label
  - [ ] Child: `text:note-body` — note content

### 12.3 Note Configuration (in `text:notes-configuration`)
- [ ] `text:notes-configuration` — footnote/endnote settings
  - [ ] Attribute: `text:note-class` — footnote / endnote
  - [ ] Attribute: `text:citation-style-name` — citation style
  - [ ] Attribute: `text:citation-body-style-name` — body citation style
  - [ ] Attribute: `text:default-style-name` — default note paragraph style
  - [ ] Attribute: `text:master-page-name` — endnote master page
  - [ ] Attribute: `text:start-value` — starting number
  - [ ] Attribute: `text:start-numbering-at` — document / chapter / page
  - [ ] Attribute: `text:footnotes-position` — page / document / section / end-of-section
  - [ ] Attribute: `style:num-format` — number format
  - [ ] Attribute: `style:num-prefix` / `style:num-suffix` — prefix/suffix

---

## 13. Bookmarks & References

### 13.1 Bookmarks
- [ ] `text:bookmark` — point bookmark (empty element)
  - [ ] Attribute: `text:name` — bookmark name
- [ ] `text:bookmark-start` — range bookmark start
  - [ ] Attribute: `text:name` — bookmark name
  - [ ] Attribute: `xml:id` — unique identifier (ODF 1.2+)
- [ ] `text:bookmark-end` — range bookmark end
  - [ ] Attribute: `text:name` — bookmark name

### 13.2 Reference Marks
- [ ] `text:reference-mark` — point reference mark
  - [ ] Attribute: `text:name` — reference name
- [ ] `text:reference-mark-start` — range reference mark start
  - [ ] Attribute: `text:name` — reference name
- [ ] `text:reference-mark-end` — range reference mark end
  - [ ] Attribute: `text:name` — reference name

---

## 14. Change Tracking

### 14.1 Change Tracking Container
- [ ] `text:tracked-changes` — container for all changes
  - [ ] Attribute: `text:track-changes` — tracking enabled (boolean)

### 14.2 Changed Regions
- [ ] `text:changed-region` — a single tracked change
  - [ ] Attribute: `xml:id` / `text:id` — unique change identifier

### 14.3 Change Types
- [ ] `text:insertion` — content insertion
- [ ] `text:deletion` — content deletion (contains deleted content)
- [ ] `text:format-change` — formatting change

### 14.4 Change Metadata
- [ ] `office:change-info` — change metadata container
  - [ ] Child: `dc:creator` — change author
  - [ ] Child: `dc:date` — change timestamp

### 14.5 Change Marks (Inline)
- [ ] `text:change` — point change mark
  - [ ] Attribute: `text:change-id` — reference to changed-region
- [ ] `text:change-start` — range change start
  - [ ] Attribute: `text:change-id` — reference to changed-region
- [ ] `text:change-end` — range change end
  - [ ] Attribute: `text:change-id` — reference to changed-region

---

## 15. Text Indexes (Table of Contents, etc.)

### 15.1 Index Marks
- [ ] `text:toc-mark` — point TOC mark
- [ ] `text:toc-mark-start` / `text:toc-mark-end` — range TOC mark
- [ ] `text:user-index-mark` — point user index mark
- [ ] `text:user-index-mark-start` / `text:user-index-mark-end` — range user index mark
- [ ] `text:alphabetical-index-mark` — point alphabetical index mark
- [ ] `text:alphabetical-index-mark-start` / `text:alphabetical-index-mark-end` — range alphabetical mark
- [ ] `text:bibliography-mark` — bibliography entry mark

### 15.2 Index Types
- [ ] `text:table-of-content` — table of contents
  - [ ] `text:table-of-content-source` — TOC configuration
  - [ ] `text:table-of-content-entry-template` — entry template
- [ ] `text:illustration-index` — illustration index
- [ ] `text:table-index` — table index
- [ ] `text:object-index` — object index
- [ ] `text:user-index` — user-defined index
- [ ] `text:alphabetical-index` — alphabetical index
- [ ] `text:bibliography` — bibliography

### 15.3 Index Structure
- [ ] `text:index-body` — generated index content
- [ ] `text:index-title` — index title
- [ ] `text:index-source-styles` — source style references
- [ ] `text:index-source-style` — individual source style
- [ ] `text:index-title-template` — title template

### 15.4 Index Entry Templates
- [ ] `text:index-entry-chapter` — chapter number entry
- [ ] `text:index-entry-text` — entry text
- [ ] `text:index-entry-page-number` — page number entry
- [ ] `text:index-entry-span` — text span entry
- [ ] `text:index-entry-bibliography` — bibliography data entry
- [ ] `text:index-entry-tab-stop` — tab stop entry
- [ ] `text:index-entry-link-start` — hyperlink start
- [ ] `text:index-entry-link-end` — hyperlink end

---

## 16. Section Properties (`style:section-properties`)

- [ ] `fo:background-color` — section background
- [ ] `fo:margin-left` — section left margin
- [ ] `fo:margin-right` — section right margin
- [ ] `style:editable` — section editability (boolean)
- [ ] `style:protect` — section protection
- [ ] `style:writing-mode` — section writing direction
- [ ] `text:dont-balance-text-columns` — disable column balancing (boolean)
- [ ] Child: `style:columns` — multi-column layout (same as page columns)
- [ ] Child: `style:background-image` — section background image

---

## 17. Ruby Properties (`style:ruby-properties`)

- [ ] `style:ruby-position` — above / below
- [ ] `style:ruby-align` — left / center / right / distribute-letter / distribute-space

---

## 18. Metadata

### 18.1 Pre-Defined Metadata Elements (in `meta.xml`)
- [ ] `meta:generator` — application that generated the document
- [ ] `dc:title` — document title
- [ ] `dc:description` — document description
- [ ] `dc:subject` — document subject
- [ ] `meta:keyword` — keywords (multiple allowed)
- [ ] `meta:initial-creator` — original author
- [ ] `dc:creator` — last modifier
- [ ] `meta:printed-by` — last printed by
- [ ] `meta:creation-date` — creation date (ISO 8601)
- [ ] `dc:date` — last modification date
- [ ] `meta:print-date` — last print date
- [ ] `meta:template` — template reference
  - [ ] Attribute: `xlink:href` — template URL
  - [ ] Attribute: `meta:date` — template date
- [ ] `meta:auto-reload` — auto-reload settings
- [ ] `meta:hyperlink-behaviour` — hyperlink behavior
- [ ] `dc:language` — document language
- [ ] `meta:editing-cycles` — number of editing sessions
- [ ] `meta:editing-duration` — cumulative editing time
- [ ] `meta:document-statistic` — document statistics
  - [ ] Attribute: `meta:page-count`
  - [ ] Attribute: `meta:paragraph-count`
  - [ ] Attribute: `meta:word-count`
  - [ ] Attribute: `meta:character-count`
  - [ ] Attribute: `meta:non-whitespace-character-count`
  - [ ] Attribute: `meta:table-count`
  - [ ] Attribute: `meta:image-count`
  - [ ] Attribute: `meta:object-count`
  - [ ] Attribute: `meta:frame-count`
  - [ ] Attribute: `meta:sentence-count`
  - [ ] Attribute: `meta:syllable-count`
  - [ ] Attribute: `meta:row-count`
  - [ ] Attribute: `meta:cell-count`
  - [ ] Attribute: `meta:ole-object-count`

### 18.2 User-Defined Metadata
- [ ] `meta:user-defined` — custom metadata field
  - [ ] Attribute: `meta:name` — field name
  - [ ] Attribute: `meta:value-type` — float / date / time / boolean / string

### 18.3 RDF Metadata (ODF 1.2+)
- [ ] `manifest.rdf` — RDF metadata file
- [ ] In-content RDFa on elements with `xhtml:about`, `xhtml:property`, `xhtml:content`, `xhtml:datatype`

---

## 19. Mathematical Content

- [ ] `math:math` — MathML content (embedded via `draw:object` in `draw:frame`)
  - [ ] Full MathML 2.0 / MathML 3.0 support
  - [ ] Alternative: embedded as separate file in ODF package

---

## 20. Form Controls

### 20.1 Form Container
- [ ] `form:form` — form definition
  - [ ] Attribute: `form:name` — form name
  - [ ] Attribute: `xlink:href` — submission URL
  - [ ] Attribute: `form:method` — get / post
  - [ ] Attribute: `form:enctype` — encoding type
  - [ ] Attribute: `form:command-type` — table / query / command
  - [ ] Attribute: `form:datasource` — data source name
  - [ ] Attribute: `form:apply-filter` — apply filter (boolean)
  - [ ] Attribute: `form:control-implementation` — implementation namespace

### 20.2 Form Control Elements
- [ ] `form:text` — text input
- [ ] `form:textarea` — multi-line text input
- [ ] `form:formatted-text` — formatted text input
- [ ] `form:password` — password input
- [ ] `form:file` — file upload
- [ ] `form:number` — number input
- [ ] `form:date` — date picker
- [ ] `form:time` — time picker
- [ ] `form:fixed-text` — static label
- [ ] `form:checkbox` — checkbox
- [ ] `form:radio` — radio button
- [ ] `form:button` — push button
- [ ] `form:listbox` — dropdown/list selection
  - [ ] Child: `form:option` — list option
- [ ] `form:combobox` — combo box (editable dropdown)
  - [ ] Child: `form:item` — combo item
- [ ] `form:image` — image button
- [ ] `form:image-frame` — image display control
- [ ] `form:value-range` — slider / scrollbar / spin button
- [ ] `form:hidden` — hidden field
- [ ] `form:grid` — data grid/table control
  - [ ] Child: `form:column` — grid column
- [ ] `form:generic-control` — generic/custom control

### 20.3 Common Form Attributes
- [ ] `form:id` — control identifier
- [ ] `form:name` — control name
- [ ] `form:control-implementation` — implementation
- [ ] `form:label` — control label text
- [ ] `form:value` / `form:current-value` — control value
- [ ] `form:disabled` — disabled state
- [ ] `form:printable` — printable flag
- [ ] `form:readonly` — read-only flag
- [ ] `form:tab-index` — tab order
- [ ] `form:tab-stop` — tab stop participation
- [ ] `form:title` — tooltip text
- [ ] `form:linked-cell` — linked spreadsheet cell
- [ ] `form:data-field` — bound database field
- [ ] `form:convert-empty-to-null` — empty-to-null conversion

### 20.4 Form Properties & Events
- [ ] `form:properties` — custom properties container
  - [ ] `form:property` — individual property
  - [ ] `form:list-property` — list-valued property
    - [ ] `form:list-value` — list property value
- [ ] `office:event-listeners` — event handler container

---

## 21. ODF Package Structure

### 21.1 Required Files
- [ ] `mimetype` — MIME type (uncompressed, first entry)
- [ ] `content.xml` — document content
- [ ] `META-INF/manifest.xml` — package manifest

### 21.2 Optional Files
- [ ] `styles.xml` — style definitions
- [ ] `meta.xml` — metadata
- [ ] `settings.xml` — application settings
- [ ] `Thumbnails/thumbnail.png` — document thumbnail
- [ ] `manifest.rdf` — RDF metadata (ODF 1.2+)
- [ ] `Pictures/` — embedded images directory
- [ ] `Object N/` — embedded objects (charts, formulas, etc.)

### 21.3 Manifest
- [ ] `manifest:manifest` — manifest root
  - [ ] `manifest:file-entry` — file entry
    - [ ] Attribute: `manifest:full-path` — file path
    - [ ] Attribute: `manifest:media-type` — MIME type
    - [ ] Attribute: `manifest:size` — uncompressed size
    - [ ] Attribute: `manifest:version` — ODF version

### 21.4 Encryption
- [ ] `manifest:encryption-data` — encryption metadata
  - [ ] `manifest:algorithm` — encryption algorithm
  - [ ] `manifest:key-derivation` — key derivation function
  - [ ] `manifest:start-key-generation` — start key generation

---

## 22. Application Settings (`office:settings`)

- [ ] `config:config-item-set` — settings group
- [ ] `config:config-item` — individual setting
  - [ ] Attribute: `config:name` — setting name
  - [ ] Attribute: `config:type` — boolean / short / int / long / double / string / datetime / base64Binary
- [ ] `config:config-item-map-indexed` — indexed map
- [ ] `config:config-item-map-named` — named map
- [ ] `config:config-item-map-entry` — map entry

---

## 23. Event Listeners

- [ ] `office:event-listeners` — event listener table
  - [ ] `script:event-listener` — event handler
    - [ ] Attribute: `script:event-name` — event name (e.g., "dom:load")
    - [ ] Attribute: `script:language` — script language
    - [ ] Attribute: `xlink:href` — script URI
    - [ ] Attribute: `script:macro-name` — macro name

---

## 24. DDE Connections

- [ ] `text:dde-connection-decls` — DDE declarations container
  - [ ] `text:dde-connection-decl` — DDE connection declaration
    - [ ] Attribute: `office:name` — connection name
    - [ ] Attribute: `office:dde-application` — application
    - [ ] Attribute: `office:dde-topic` — topic
    - [ ] Attribute: `office:dde-item` — item
    - [ ] Attribute: `office:automatic-update` — auto-update (boolean)

---

## Summary Statistics

| Category | Feature Count |
|---|---|
| Document Structure | ~25 |
| Text Content (Headings, Paragraphs, Inline) | ~35 |
| Text Properties | ~84 |
| Paragraph Properties | ~70 |
| Lists | ~45 |
| Tables | ~65 |
| Styles | ~60 |
| Page Layout | ~55 |
| Images, Frames & Drawing | ~120 |
| Text Fields | ~65 |
| Annotations & Notes | ~20 |
| Bookmarks & References | ~10 |
| Change Tracking | ~12 |
| Text Indexes | ~30 |
| Section Properties | ~10 |
| Ruby Properties | ~3 |
| Metadata | ~35 |
| Math | ~2 |
| Form Controls | ~40 |
| Package Structure | ~15 |
| Settings | ~5 |
| Events & DDE | ~8 |
| **TOTAL** | **~810+** |

---

## References

- [OASIS ODF 1.2 Part 1: OpenDocument Schema](https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-part1.html)
- [OASIS ODF 1.3 Part 3: OpenDocument Schema](https://docs.oasis-open.org/office/OpenDocument/v1.3/OpenDocument-v1.3-part3-schema.html)
- [OASIS ODF 1.2 Specification Overview](https://docs.oasis-open.org/office/v1.2/OpenDocument-v1.2.html)
- [ODFDOM API (StyleTextPropertiesElement)](https://odftoolkit.org/api/odfdom/org/odftoolkit/odfdom/dom/element/style/StyleTextPropertiesElement.html)
- [ODFDOM API (StyleParagraphPropertiesElement)](https://odftoolkit.org/api/odfdom/org/odftoolkit/odfdom/dom/element/style/StyleParagraphPropertiesElement.html)
- [ODFDOM API (StylePageLayoutPropertiesElement)](https://odftoolkit.org/api/odfdom/org/odftoolkit/odfdom/dom/element/style/StylePageLayoutPropertiesElement.html)
- [ODFDOM API (StyleGraphicPropertiesElement)](https://odftoolkit.org/api/odfdom/org/odftoolkit/odfdom/dom/element/style/StyleGraphicPropertiesElement.html)
- [ODFDOM Operations Formatting Attributes](https://odftoolkit.org/odfdom/operations/operations-formatting-attributes.html)
- [ODF Form Schema (datypic.com)](http://www.datypic.com/sc/odf/s-form.xsd.html)
- [ODF 1.0 Ed2 Specification (open-std.org)](https://www.open-std.org/keld/iso26300-odf/is26300/OpenDocument-v1.0ed2-cs1.html)
- [OpenDocument Draw Frame Element (datypic.com)](https://www.datypic.com/sc/odf/e-draw_frame.html)
