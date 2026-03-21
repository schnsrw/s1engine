# PDF Export Specification v1.0

## Reference Standard
- PDF 1.7 (ISO 32000-1:2008)
- PDF/A-1b (ISO 19005-1:2005) for archival compliance

## Current Implementation

### What Works
| Feature | Status | Notes |
|---------|--------|-------|
| Paragraphs with text | DONE | Full Unicode support via ToUnicode CMap |
| Bold/italic/underline | DONE | Font variant selection |
| Font embedding | DONE | Subsetting via `subsetter` crate |
| Font size/family | DONE | Multiple fonts per document |
| Text color | DONE | RGB color model |
| Images (JPEG) | DONE | Passthrough (no re-encoding) |
| Images (PNG) | DONE | Decode → re-encode for PDF stream |
| Page layout | DONE | Knuth-Plass line breaking via s1-layout |
| Page numbers | DONE | Field substitution |
| Headers/footers | DONE | Per-section support |
| Page breaks | DONE | Explicit + automatic pagination |
| Margins | DONE | Per-section page margins |
| Tables | DONE | Basic table with borders |
| Lists | DONE | Bullet and numbered |
| Headings | DONE | H1-H6 with appropriate sizing |
| PDF/A-1b metadata | DONE | XMP, OutputIntent, ICC profile, MarkInfo |

### What's Partial
| Feature | Status | Gap |
|---------|--------|-----|
| Table borders | SPECIFIED (Section: Table Border Styles) | Basic borders done; custom styles specified below |
| Cell shading | SPECIFIED (Section: Cell Shading Patterns) | Solid fill done; patterns specified below |
| Hyphenation | SPECIFIED (Section: Hyphenation Languages) | English done; multi-language specified below |
| BiDi text | SPECIFIED (Section: BiDi Text Rendering) | Core algorithm done; edge cases specified below |
| Multi-column | NOT DONE | Single column only |
| Floating images | NOT DONE | All images rendered inline |

### What's Not Supported
| Feature | Notes |
|---------|-------|
| Form fields | PDF forms not generated |
| Annotations | Not in export (PDF viewer has them) |
| Digital signatures | Signature embedding not in export |
| Incremental updates | Always full rewrite |
| Vector graphics | Shapes render as placeholders |
| Transparency | No alpha channel support |
| Gradients | Not supported |

## PDF Generation Pipeline

```
DocumentModel
  → s1-layout::Engine (Knuth-Plass line breaking)
    → Page layout with lines, positions, dimensions
      → pdf-writer (low-level PDF generation)
        → Font subsetting (subsetter crate)
        → Image embedding (JPEG passthrough, PNG decode)
        → ToUnicode CMap (text extraction support)
          → Final PDF bytes
```

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| 10-page document | <100ms | ~50ms |
| 100-page document | <2s | ~500ms |
| Output file size (10 pages, no images) | <100KB | ~50KB |
| Output file size (10 pages, 5 images) | <2MB | Depends on image size |
| Font subsetting | <50ms per font | ~20ms |

## Edge Cases

| # | Scenario | Expected Behavior |
|---|----------|-------------------|
| 1 | Empty document | Single page with header/footer only |
| 2 | Document with only images | Images positioned with text flow |
| 3 | Very long paragraph (no breaks) | Word-level wrapping, hyphenation if available |
| 4 | Table wider than page | Table shrunk to fit page width |
| 5 | Nested tables | Inner table rendered within cell |
| 6 | Missing font | Fallback to default serif font |
| 7 | Non-Latin text (CJK, Arabic, Hindi) | Unicode CMap ensures correct extraction |
| 8 | Color on white background | RGB color preserved |
| 9 | Highlight color | Background rectangle behind text |

## Table Border Styles

### Overview

OOXML defines 30+ border types in `ST_Border`. The PDF export maps each to PDF line-drawing operations (path construction + stroke).

### Border Type Mapping

| OOXML Border Type | PDF Rendering | Line Width | Pattern |
|-------------------|---------------|------------|---------|
| `single` | Solid line | 0.5pt | Continuous |
| `thick` | Solid line | 1.5pt | Continuous |
| `double` | Two parallel lines | 0.5pt each, 1pt gap | Continuous |
| `dotted` | Dotted line | 0.5pt | `[1 2]` dash array |
| `dashed` | Dashed line | 0.5pt | `[4 2]` dash array |
| `dashSmallGap` | Dashed line | 0.5pt | `[4 1]` dash array |
| `dashDotDot` | Dash-dot-dot | 0.5pt | `[4 1 1 1 1 1]` dash array |
| `dashDot` | Dash-dot | 0.5pt | `[4 1 1 1]` dash array |
| `triple` | Three parallel lines | 0.5pt each, 0.5pt gaps | Continuous |
| `thinThickSmallGap` | Thin + thick | 0.25pt + 1pt, 0.5pt gap | Continuous |
| `thickThinSmallGap` | Thick + thin | 1pt + 0.25pt, 0.5pt gap | Continuous |
| `thinThickMediumGap` | Thin + thick | 0.25pt + 1pt, 1pt gap | Continuous |
| `thickThinMediumGap` | Thick + thin | 1pt + 0.25pt, 1pt gap | Continuous |
| `thinThickLargeGap` | Thin + thick | 0.25pt + 1pt, 2pt gap | Continuous |
| `thickThinLargeGap` | Thick + thin | 1pt + 0.25pt, 2pt gap | Continuous |
| `thinThickThinSmallGap` | Thin + thick + thin | 0.25pt + 1pt + 0.25pt | Continuous |
| `thinThickThinMediumGap` | Thin + thick + thin | 0.25pt + 1pt + 0.25pt, 1pt gaps | Continuous |
| `thinThickThinLargeGap` | Thin + thick + thin | 0.25pt + 1pt + 0.25pt, 2pt gaps | Continuous |
| `wave` | Wavy line | 0.5pt | Approximated with Bezier curves |
| `doubleWave` | Double wavy | 0.5pt each, 1pt gap | Two Bezier wave paths |
| `threeDEmboss` | 3D emboss | Varies | Light top/left, dark bottom/right |
| `threeDEngrave` | 3D engrave | Varies | Dark top/left, light bottom/right |
| `outset` | Outset 3D | Varies | Light top/left, dark bottom/right |
| `inset` | Inset 3D | Varies | Dark top/left, light bottom/right |
| `none` | No border | 0 | None |
| `nil` | No border (explicit) | 0 | None |

### PDF Drawing Implementation

```rust
fn draw_border_segment(
    content: &mut Content,
    x1: f32, y1: f32,  // Start point
    x2: f32, y2: f32,  // End point
    border: &BorderStyle,
) {
    let color = border.color.unwrap_or(Color::BLACK);
    content.set_stroke_color_rgb(color.r_f32(), color.g_f32(), color.b_f32());

    match border.style {
        BorderType::Single => {
            content.set_line_width(border.width_pt());
            content.move_to(x1, y1);
            content.line_to(x2, y2);
            content.stroke();
        }
        BorderType::Double => {
            let offset = border.width_pt() + 1.0; // gap between lines
            let (nx, ny) = normal_vector(x1, y1, x2, y2);
            // First line
            content.set_line_width(border.width_pt());
            content.move_to(x1 + nx * offset / 2.0, y1 + ny * offset / 2.0);
            content.line_to(x2 + nx * offset / 2.0, y2 + ny * offset / 2.0);
            content.stroke();
            // Second line
            content.move_to(x1 - nx * offset / 2.0, y1 - ny * offset / 2.0);
            content.line_to(x2 - nx * offset / 2.0, y2 - ny * offset / 2.0);
            content.stroke();
        }
        BorderType::Dotted => {
            content.set_line_width(border.width_pt());
            content.set_dash_pattern(&[1.0, 2.0], 0.0);
            content.move_to(x1, y1);
            content.line_to(x2, y2);
            content.stroke();
            content.set_dash_pattern(&[], 0.0); // reset
        }
        BorderType::Dashed => {
            content.set_line_width(border.width_pt());
            content.set_dash_pattern(&[4.0, 2.0], 0.0);
            content.move_to(x1, y1);
            content.line_to(x2, y2);
            content.stroke();
            content.set_dash_pattern(&[], 0.0);
        }
        // ...other styles follow the same pattern with appropriate
        // dash arrays, line widths, and parallel-line offsets.
    }
}
```

### Border Priority Rules

When adjacent cells have conflicting borders, the following priority applies (matching OOXML rules):

1. `none` / `nil` always loses (any border wins over no border).
2. Wider border wins over narrower border.
3. If same width: darker color wins.
4. If same width and color: style priority: `double` > `solid` > `dashed` > `dotted` > `none`.
5. Cell border overrides row border overrides table border.

## Cell Shading Patterns

### Overview

OOXML cell shading (`<w:shd>`) supports both solid fills and pattern fills. The `w:val` attribute specifies the pattern type, `w:fill` specifies the background color, and `w:color` specifies the foreground (pattern) color.

### Pattern Types

| OOXML Pattern (`w:val`) | Description | PDF Implementation |
|--------------------------|-------------|-------------------|
| `clear` | No pattern (solid fill or transparent) | Rectangle fill with `w:fill` color |
| `solid` | Solid fill | Rectangle fill with `w:color` (foreground) |
| `horzStripe` | Horizontal stripes | Repeating horizontal lines (1pt wide, 4pt spacing) |
| `vertStripe` | Vertical stripes | Repeating vertical lines (1pt wide, 4pt spacing) |
| `reverseDiagStripe` | Diagonal stripes (top-left to bottom-right) | 45-degree repeating lines |
| `diagStripe` | Diagonal stripes (bottom-left to top-right) | 135-degree repeating lines |
| `horzCross` | Horizontal + vertical crosshatch | Grid of perpendicular lines |
| `diagCross` | Diagonal crosshatch | X-pattern of 45-degree and 135-degree lines |
| `thinHorzStripe` | Thin horizontal stripes | 0.5pt lines, 2pt spacing |
| `thinVertStripe` | Thin vertical stripes | 0.5pt lines, 2pt spacing |
| `thinReverseDiagStripe` | Thin diagonal (top-left to bottom-right) | 0.5pt, 45-degree lines |
| `thinDiagStripe` | Thin diagonal (bottom-left to top-right) | 0.5pt, 135-degree lines |
| `thinHorzCross` | Thin crosshatch | 0.5pt grid lines |
| `thinDiagCross` | Thin diagonal crosshatch | 0.5pt X-pattern |
| `pct5` through `pct95` | Percentage fills (5% to 95%) | Solid fill with alpha-blended color |

### PDF Pattern Implementation

PDF patterns are implemented using **Tiling Patterns** (PDF Reference 8.7.3):

```rust
fn create_pattern_fill(
    writer: &mut PdfWriter,
    pattern_type: ShadingPattern,
    fg_color: Color,
    bg_color: Color,
    cell_rect: Rect,
) -> Name {
    // Step 1: Fill background
    draw_filled_rect(cell_rect, bg_color);

    match pattern_type {
        ShadingPattern::Clear => {
            // Background only, no pattern overlay
        }
        ShadingPattern::Solid => {
            draw_filled_rect(cell_rect, fg_color);
        }
        ShadingPattern::HorzStripe => {
            // Create a tiling pattern:
            // BBox: [0, 0, cell_width, 4.0] (4pt repeat)
            // XStep: cell_width, YStep: 4.0
            // Paint: horizontal line at y=2, width=cell_width, height=1pt
            let pattern_id = writer.alloc_ref();
            let mut pattern = writer.tiling_pattern(pattern_id);
            pattern.bbox(Rect::new(0.0, 0.0, cell_rect.width(), 4.0));
            pattern.x_step(cell_rect.width());
            pattern.y_step(4.0);
            let mut content = pattern.content_stream();
            content.set_fill_color_rgb(fg_color.r_f32(), fg_color.g_f32(), fg_color.b_f32());
            content.rect(0.0, 1.5, cell_rect.width(), 1.0);
            content.fill();
        }
        ShadingPattern::VertStripe => {
            // Same as HorzStripe but rotated 90 degrees
            // BBox: [0, 0, 4.0, cell_height], XStep: 4.0, YStep: cell_height
        }
        ShadingPattern::DiagStripe | ShadingPattern::ReverseDiagStripe => {
            // Diagonal lines using move_to/line_to at 45/135 degrees
            // Tiling pattern with square bbox and diagonal line path
        }
        ShadingPattern::HorzCross => {
            // Combine horizontal and vertical stripe patterns
        }
        ShadingPattern::DiagCross => {
            // Combine both diagonal stripe patterns
        }
        ShadingPattern::Pct(n) => {
            // Percentage fill: blend fg and bg colors
            let alpha = n as f32 / 100.0;
            let blended = Color::new(
                bg_color.r as f32 * (1.0 - alpha) + fg_color.r as f32 * alpha,
                bg_color.g as f32 * (1.0 - alpha) + fg_color.g as f32 * alpha,
                bg_color.b as f32 * (1.0 - alpha) + fg_color.b as f32 * alpha,
            );
            draw_filled_rect(cell_rect, blended);
        }
    }
}
```

### Percentage Fill Color Blending

For percentage fills (`pct5` through `pct95`), the resulting color is computed by linear interpolation:

```
result.r = bg.r * (1 - pct/100) + fg.r * (pct/100)
result.g = bg.g * (1 - pct/100) + fg.g * (pct/100)
result.b = bg.b * (1 - pct/100) + fg.b * (pct/100)
```

This produces a solid fill rather than a dithered pattern, which is the standard behavior for modern PDF generators (matching Word's PDF export).

## Hyphenation Language Support

### Overview

The `s1-text` crate provides hyphenation via the Knuth-Liang algorithm with language-specific pattern dictionaries. Currently only English is supported. This section specifies multi-language support.

### Language Detection Priority

When determining which hyphenation dictionary to use for a text run:

```
1. Run-level language attribute (highest priority)
   - OOXML: <w:rPr><w:lang w:val="de-DE"/></w:rPr>
   - ODF: <text:span fo:language="de" fo:country="DE">
   - Model: AttributeKey::Language → AttributeValue::String("de-DE")

2. Paragraph-level language attribute
   - OOXML: <w:pPr><w:rPr><w:lang w:val="fr-FR"/></w:rPr></w:pPr>
   - Model: Inherited from paragraph style

3. Document default language
   - OOXML: <w:docDefaults><w:rPrDefault><w:rPr><w:lang w:val="en-US"/>
   - Model: Document.default_language()

4. Fallback: "en-US" (if no language is specified anywhere)
```

### Supported Languages

| Language | BCP 47 Code | Dictionary Source | Quality |
|----------|-------------|-------------------|---------|
| English (US) | `en-US` | TeX hyphenation patterns (hyph-en-us) | Excellent |
| English (UK) | `en-GB` | TeX hyphenation patterns (hyph-en-gb) | Excellent |
| German | `de-DE` | TeX hyphenation patterns (hyph-de) | Excellent |
| German (Swiss) | `de-CH` | TeX hyphenation patterns (hyph-de-ch) | Excellent |
| French | `fr-FR` | TeX hyphenation patterns (hyph-fr) | Excellent |
| Spanish | `es-ES` | TeX hyphenation patterns (hyph-es) | Excellent |
| Italian | `it-IT` | TeX hyphenation patterns (hyph-it) | Excellent |
| Portuguese | `pt-PT` | TeX hyphenation patterns (hyph-pt) | Excellent |
| Portuguese (Brazil) | `pt-BR` | TeX hyphenation patterns (hyph-pt-br) | Good |
| Dutch | `nl-NL` | TeX hyphenation patterns (hyph-nl) | Excellent |
| Swedish | `sv-SE` | TeX hyphenation patterns (hyph-sv) | Good |
| Norwegian | `nb-NO` | TeX hyphenation patterns (hyph-nb) | Good |
| Danish | `da-DK` | TeX hyphenation patterns (hyph-da) | Good |
| Finnish | `fi-FI` | TeX hyphenation patterns (hyph-fi) | Good |
| Polish | `pl-PL` | TeX hyphenation patterns (hyph-pl) | Good |
| Czech | `cs-CZ` | TeX hyphenation patterns (hyph-cs) | Good |
| Hungarian | `hu-HU` | TeX hyphenation patterns (hyph-hu) | Good |
| Russian | `ru-RU` | TeX hyphenation patterns (hyph-ru) | Good |
| Ukrainian | `uk-UA` | TeX hyphenation patterns (hyph-uk) | Good |
| Turkish | `tr-TR` | TeX hyphenation patterns (hyph-tr) | Good |
| Greek | `el-GR` | TeX hyphenation patterns (hyph-el) | Good |
| Catalan | `ca-ES` | TeX hyphenation patterns (hyph-ca) | Good |
| Croatian | `hr-HR` | TeX hyphenation patterns (hyph-hr) | Good |
| Romanian | `ro-RO` | TeX hyphenation patterns (hyph-ro) | Good |
| Slovak | `sk-SK` | TeX hyphenation patterns (hyph-sk) | Good |

### Dictionary Loading Strategy

Hyphenation dictionaries are loaded lazily and cached:

```rust
use std::collections::HashMap;
use std::sync::RwLock;

static HYPHENATION_CACHE: RwLock<HashMap<String, HyphenationDict>> = RwLock::new(HashMap::new());

fn get_hyphenation_dict(lang: &str) -> Option<&HyphenationDict> {
    // Normalize: "de-DE" → "de-de", "en" → "en-us" (default region)
    let normalized = normalize_lang_code(lang);

    // Check cache
    if let Some(dict) = HYPHENATION_CACHE.read().unwrap().get(&normalized) {
        return Some(dict);
    }

    // Load from embedded patterns (compiled into the binary)
    if let Some(patterns) = load_embedded_patterns(&normalized) {
        let dict = HyphenationDict::from_patterns(patterns);
        HYPHENATION_CACHE.write().unwrap().insert(normalized.clone(), dict);
        return HYPHENATION_CACHE.read().unwrap().get(&normalized);
    }

    // Fallback: try base language ("de-DE" → "de")
    let base = normalized.split('-').next()?;
    if let Some(patterns) = load_embedded_patterns(base) {
        let dict = HyphenationDict::from_patterns(patterns);
        HYPHENATION_CACHE.write().unwrap().insert(normalized.clone(), dict);
        return HYPHENATION_CACHE.read().unwrap().get(&normalized);
    }

    None // No dictionary available; skip hyphenation for this language
}
```

### Hyphenation Rules

- Minimum word length for hyphenation: 5 characters.
- Minimum characters before hyphen: 2.
- Minimum characters after hyphen: 3.
- Proper nouns (capitalized words) are NOT hyphenated unless explicitly allowed.
- Words containing digits are NOT hyphenated.
- Words containing hyphens are only hyphenated at the existing hyphen positions.

### Languages NOT Supported (No Hyphenation)

The following languages do not use hyphenation or have no available pattern dictionaries:

| Language | Reason |
|----------|--------|
| Chinese (zh) | No word boundaries; line breaks at any character |
| Japanese (ja) | Line breaks follow kinsoku rules, not hyphenation |
| Korean (ko) | Line breaks at syllable boundaries, not hyphenation |
| Arabic (ar) | Hyphenation not standard; line breaks at word boundaries |
| Hebrew (he) | Hyphenation not standard; line breaks at word boundaries |
| Thai (th) | No spaces between words; requires word segmentation, not hyphenation |

For these languages, the line-breaking algorithm uses Unicode Line Break Algorithm (UAX #14) instead of hyphenation.

## BiDi Text Rendering in PDF

### Overview

Bidirectional (BiDi) text rendering follows UAX #9 (Unicode Bidirectional Algorithm). The `s1-text` crate implements this using the `unicode-bidi` crate. For PDF export, the resolved visual order must be used when positioning glyphs.

### UAX #9 Algorithm Steps (as applied in PDF export)

```
Input: A paragraph of text with mixed LTR and RTL runs.

Step 1: Determine paragraph direction
  - Check AttributeKey::ParagraphDirection on the paragraph node.
  - If "rtl": paragraph base level = 1 (RTL).
  - If "ltr" or absent: paragraph base level = 0 (LTR).
  - Fallback: examine first strong character (UAX #9 rule P2/P3).

Step 2: Resolve embedding levels
  - Apply the unicode-bidi crate's BidiInfo::new(text, base_level).
  - This resolves explicit embedding controls (LRE, RLE, LRO, RRO, PDF)
    and implicit BiDi types for each character.
  - Each character gets an embedding level (integer 0-125).

Step 3: Reorder for visual display
  - Apply BidiInfo::reorder_line(&resolved_levels) for each line.
  - This produces visual_runs: a list of (start, end, level) ranges
    in visual (display) order.
  - Even levels = LTR display order; odd levels = RTL display order.

Step 4: Shape each visual run independently
  - For each run in visual order:
    a. Determine the script and language (from run attributes or Unicode script detection).
    b. Select the appropriate font for the run's script.
    c. Shape the run using rustybuzz with the correct direction:
       - Even level: rustybuzz::Direction::LeftToRight
       - Odd level: rustybuzz::Direction::RightToLeft
    d. Shaping produces glyph IDs and advance widths.

Step 5: Position glyphs on the PDF page
  - For LTR paragraph (base level 0):
    - Start at left margin.
    - Place visual runs left-to-right.
    - Within each RTL run, glyphs are in right-to-left order (shaper handles this).
  - For RTL paragraph (base level 1):
    - Start at right margin.
    - Place visual runs right-to-left.
    - Within each LTR run, glyphs are in left-to-right order.

Step 6: Handle line breaking
  - Line breaking operates on the LOGICAL text (not visual).
  - After determining line break positions, each line is independently
    reordered using step 3.
  - This ensures that a word broken across lines maintains correct
    directional context on each line.
```

### Mixed LTR/RTL Example

```
Logical text: "Hello مرحبا World"
Base level: 0 (LTR paragraph)

Resolved levels:
  H(0) e(0) l(0) l(0) o(0) (0) م(1) ر(1) ح(1) ب(1) ا(1) (0) W(0) o(0) r(0) l(0) d(0)

Visual runs (after reorder):
  Run 1: "Hello " (level 0, LTR) — glyphs left-to-right
  Run 2: "ابحرم" (level 1, RTL) — glyphs right-to-left (reversed from logical)
  Run 3: " World" (level 0, LTR) — glyphs left-to-right

PDF positioning (left margin = 72pt):
  x=72:  H e l l o [space]
  x=120: ا ب ح ر م          (Arabic glyphs, right-to-left within run)
  x=168: [space] W o r l d
```

### RTL Paragraph Example

```
Logical text: "مرحبا Hello مرحبا"
Base level: 1 (RTL paragraph)

Resolved levels:
  م(1) ر(1) ح(1) ب(1) ا(1) (1) H(2) e(2) l(2) l(2) o(2) (1) م(1) ر(1) ح(1) ب(1) ا(1)

Visual runs (after reorder, RIGHT-TO-LEFT):
  Run 1: "ابحرم" (level 1, RTL) — starts at right margin
  Run 2: " Hello " (level 2, LTR) — embedded LTR
  Run 3: "ابحرم" (level 1, RTL)

PDF positioning (right margin = 540pt, text flows rightward from right):
  x=540: ا ب ح ر م          (right-aligned, RTL)
  x=492: [space] H e l l o [space]
  x=440: ا ب ح ر م
```

### PDF-Specific Considerations

| Aspect | Implementation |
|--------|---------------|
| Text matrix | Each run sets its own `Tm` (text matrix) with the correct x,y position |
| Font switching | Each run may use a different font (Latin vs Arabic vs CJK) |
| ToUnicode CMap | Maps glyph IDs back to logical Unicode order for text extraction |
| Mirroring | Characters like parentheses are mirrored in RTL context (UAX #9 rule L4) |
| Neutral characters | Spaces, punctuation inherit direction from surrounding strong characters |
| Numbers in RTL | European numbers (0-9) remain LTR even in RTL context (embedding level 2) |
| Paragraph alignment | RTL paragraphs default to right-aligned; LTR to left-aligned |

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| Deeply nested embeddings (LTR in RTL in LTR) | Handled by UAX #9 level resolution; max level 125 |
| Bidirectional override characters (LRO, RLO) | Respected; force direction regardless of character BiDi type |
| Weak/neutral characters between same-direction runs | Inherit direction of surrounding runs |
| Number followed by RTL text | Number maintains LTR; may visually appear "inside" RTL text |
| Empty RTL paragraph | Cursor placed at right edge; paragraph direction stored in model |
| Mixed scripts in same run | Each script shaped independently; positions concatenated |
| Line break inside RTL word | Both halves remain RTL; second half starts at right margin of next line |

## Test Strategy

1. **Round-trip**: Open DOCX → export PDF → verify text extraction matches
2. **Visual comparison**: Export PDF → render pages → compare with DOCX rendering
3. **PDF/A validation**: Run through veraPDF validator
4. **Cross-reader**: Open in Adobe Reader, Chrome PDF viewer, Firefox PDF viewer
5. **Accessibility**: Check tagged PDF, text extraction, bookmark structure
