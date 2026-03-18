# Layout Engine

The layout engine (`s1-layout`) paginates documents for PDF export and editor display.

## Features

- Knuth-Plass line breaking algorithm
- Widow/orphan control
- Text shaping via rustybuzz (pure Rust HarfBuzz)
- Font loading via fontdb with substitution table
- Hyphenation (English) via Knuth-Liang patterns
- BiDi text support via unicode-bidi
- Table layout with column width distribution
- Image placement with dimension constraints
- Margin collapsing (CSS spec-compliant)

## Configuration

```rust
LayoutConfig {
    page_width: 612.0,   // US Letter (points)
    page_height: 792.0,
    margin_top: 72.0,    // 1 inch
    margin_bottom: 72.0,
    margin_left: 72.0,
    margin_right: 72.0,
    min_orphan_lines: 2,
    min_widow_lines: 2,
}
```

## Performance

- Full layout of a 10-page document: ~50ms
- Incremental layout: planned (dirty_from_page field exists)
- Font cache: LRU with 50K entry fallback cache
