# Dependencies

All external dependencies used by s1engine, organized by crate.

## Dependency Policy

1. **Minimize dependencies** -- every dependency is a maintenance burden and attack surface
2. **Prefer well-maintained crates** -- active maintainers, recent releases, good issue response
3. **Pure Rust only** -- no C/C++ FFI anywhere in the dependency tree
4. **Pin major versions** -- use `>=x.y, <x+1` ranges in Cargo.toml
5. **Audit security** -- run `cargo audit` in CI

---

## Core Crates

### s1-model
**Zero external dependencies.** Pure Rust data structures.

This is intentional and must never change. The document model is the foundation -- it must be portable, compilable anywhere, and have zero supply-chain risk.

### s1-ops
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |

No external dependencies. Pure logic operating on `s1-model` types.

### s1-crdt
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `s1-ops` | workspace | Operation types | -- |

No external dependencies. Fugue-based text CRDT, tree CRDT with Kleppmann moves, LWW attribute CRDT, binary serialization, operation compression. All pure Rust.

### s1-format-txt
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `encoding_rs` | ^0.8 | Detect/convert text encodings (UTF-8, UTF-16, Latin-1, etc.) | MIT/Apache-2.0 |

### s1-format-docx
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `quick-xml` | ^0.37 | Fast, streaming XML parser and writer | MIT |
| `zip` | ^2.0 | ZIP archive reading and writing (DOCX files are ZIP archives) | MIT |
| `base64` | ^0.22 | Base64 encoding for embedded content | MIT/Apache-2.0 |

**Why quick-xml?**
- Fastest Rust XML parser (streaming, zero-copy where possible)
- No C dependencies
- Well-maintained, widely used in Rust ecosystem
- Supports both reading and writing

**Why zip (crate)?**
- Pure Rust ZIP implementation
- Supports deflate compression (required by OOXML/ODF)
- Read and write support

### s1-format-odt
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `quick-xml` | ^0.37 | XML parser/writer (shared with DOCX) | MIT |
| `zip` | ^2.0 | ZIP archive handling (shared with DOCX) | MIT |

---

## Layout & Export Crates

### s1-text (Text Processing -- Pure Rust)
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `thiserror` | ^2.0 | Error type derivation | MIT/Apache-2.0 |
| `rustybuzz` | ^0.20 | Text shaping (pure Rust port of HarfBuzz) | MIT |
| `ttf-parser` | ^0.25 | Font parsing (pure Rust, read-only) | MIT/Apache-2.0 |
| `fontdb` | ^0.23 | System font discovery and indexing | MIT |
| `unicode-bidi` | ^0.3 | Unicode Bidirectional Algorithm (UAX #9) | MIT/Apache-2.0 |
| `unicode-linebreak` | ^0.1 | Unicode Line Breaking Algorithm (UAX #14) | MIT |

**Why rustybuzz (not harfbuzz-rs)?**
- Pure Rust -- no C/C++ build dependencies, works on WASM
- Port of HarfBuzz with good complex script support
- Handles Arabic, Devanagari, Thai, CJK
- OpenType feature support (ligatures, kerning, contextual alternates)

**Why ttf-parser (not freetype-rs)?**
- Pure Rust -- no C/C++ build dependencies
- Read-only font parsing (TrueType, OpenType)
- Lightweight, zero-copy parsing
- Sufficient for metrics and glyph data (we don't need rasterization)

**Why fontdb?**
- Pure Rust system font discovery
- Indexes fonts by family, style, weight
- Cross-platform (macOS, Linux, Windows)

### s1-layout
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `s1-text` | workspace | Text shaping and font metrics | -- |
| `thiserror` | ^2.0 | Error type derivation | MIT/Apache-2.0 |
| `fontdb` | ^0.23 | Font database (shared with s1-text) | MIT |

Layout algorithms (Knuth-Plass line breaking, pagination, table layout) are implemented in pure Rust.

### s1-format-pdf
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `s1-layout` | workspace | Layout tree (PDF renders from layout) | -- |
| `s1-text` | workspace | Font access for subsetting | -- |
| `thiserror` | ^2.0 | Error type derivation | MIT/Apache-2.0 |
| `pdf-writer` | ^0.14 | Low-level PDF file generation | MIT/Apache-2.0 |
| `subsetter` | ^0.2 | Font subsetting (embed only used glyphs) | MIT/Apache-2.0 |
| `miniz_oxide` | ^0.8 | Deflate compression for PDF streams | MIT/Apache-2.0 |
| `image` | ^0.25 | Image decoding (PNG, JPEG) | MIT/Apache-2.0 |

**Why pdf-writer?**
- Pure Rust, no C dependencies
- Low-level control over PDF output
- Small, focused crate (not a monolithic PDF toolkit)
- Same authors as Typst -- proven in production

**Why subsetter?**
- Companion to `pdf-writer` -- same authors
- Subsets TrueType/OpenType fonts to include only used glyphs
- Critical for reasonable PDF file sizes

### s1-convert
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `s1-format-docx` | workspace | DOCX codec | -- |
| `s1-format-odt` | workspace | ODT codec | -- |
| `thiserror` | ^2.0 | Error type derivation | MIT/Apache-2.0 |
| `cfb` | ^0.10 | OLE2 Compound File Binary reader (for DOC files) | MIT |

**Why cfb?**
- Reads Microsoft's OLE2 container format (used by .doc, .xls, .ppt)
- Pure Rust
- Required as first step to reading legacy DOC files

---

## Facade Crate

### s1engine
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1-model` | workspace | Core types | -- |
| `s1-ops` | workspace | Operations | -- |
| `thiserror` | ^2.0 | Error type derivation | MIT/Apache-2.0 |
| `s1-format-docx` | workspace | DOCX support (feature-gated) | -- |
| `s1-format-odt` | workspace | ODT support (feature-gated) | -- |
| `s1-format-pdf` | workspace | PDF export (feature-gated) | -- |
| `s1-format-txt` | workspace | TXT support (feature-gated) | -- |
| `s1-convert` | workspace | Conversion (feature-gated) | -- |
| `s1-crdt` | workspace | CRDT (feature-gated) | -- |
| `s1-layout` | workspace | Layout (feature-gated) | -- |
| `s1-text` | workspace | Text processing (feature-gated) | -- |

---

## FFI Crates

### ffi/wasm
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1engine` | workspace | Engine facade | -- |
| `s1-text` | workspace | Font database | -- |
| `wasm-bindgen` | ^0.2 | Rust <-> JavaScript FFI for WASM | MIT/Apache-2.0 |
| `js-sys` | ^0.3 | JavaScript standard library bindings | MIT/Apache-2.0 |

### ffi/c
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `s1engine` | workspace | Engine facade | -- |

Opaque handle pattern with `extern "C"` functions. No additional dependencies.

---

## Dev Dependencies (shared)
| Dependency | Version | Purpose | License |
|---|---|---|---|
| `proptest` | ^1.0 | Property-based testing | MIT/Apache-2.0 |
| `pretty_assertions` | ^1.0 | Better test failure diffs | MIT/Apache-2.0 |

---

## Dependency Graph

```
s1engine (facade)
|-- s1-model              (0 external deps)
|-- s1-ops                (0 external deps)
|-- s1-crdt               (0 external deps)
|-- s1-format-docx        (quick-xml, zip, base64)
|-- s1-format-odt         (quick-xml, zip)
|-- s1-format-txt         (encoding_rs)
|-- s1-format-pdf         (pdf-writer, subsetter, miniz_oxide, image)
|-- s1-layout             (fontdb)
|-- s1-convert            (cfb)
|-- s1-text               (rustybuzz, ttf-parser, fontdb, unicode-bidi, unicode-linebreak)
|-- thiserror
+-- [dev] proptest, pretty_assertions
```

**Total external runtime dependencies**: ~16 crates (not counting transitive deps)
**C/C++ libraries**: 0 -- entire stack is pure Rust

---

## License Compatibility

All dependencies use MIT, Apache-2.0, or MIT/Apache-2.0 dual license, which are compatible with s1engine's MIT/Apache-2.0 dual license.

---

## Alternatives Considered

| Chosen | Alternative | Why We Chose This |
|---|---|---|
| `quick-xml` | `roxmltree`, `xmltree` | `quick-xml` supports both read and write; streaming API for large files |
| `zip` | `rc-zip` | `zip` is more mature and widely used |
| `pdf-writer` | `printpdf`, `lopdf` | `pdf-writer` is lower-level, smaller, proven by Typst |
| `rustybuzz` | `harfbuzz-rs` | `rustybuzz` is pure Rust, works on WASM, no C build deps |
| `ttf-parser` | `freetype-rs` | `ttf-parser` is pure Rust, zero-copy; FreeType requires C build |
| `unicode-bidi` | ICU / `icu4x` | Smaller, focused; full ICU is overkill for just BiDi |
| `fontdb` | `font-kit` | `fontdb` is lighter, no C deps, does what we need |
