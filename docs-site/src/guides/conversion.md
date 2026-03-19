# Format Conversion

## Rust API

```rust
use s1engine::{Engine, Format};

let engine = Engine::new();
let doc = engine.open(&std::fs::read("input.docx")?)?;

// Convert to any supported format
std::fs::write("output.odt", doc.export(Format::Odt)?)?;
std::fs::write("output.pdf", doc.export(Format::Pdf)?)?;
std::fs::write("output.txt", doc.export(Format::Txt)?)?;
std::fs::write("output.md", doc.export(Format::Md)?)?;
```

## Server API

```bash
curl -X POST http://localhost:8080/api/v1/convert \
  -F file=@input.docx \
  -F format=pdf \
  -o output.pdf
```

## Supported Conversions

| From | DOCX | ODT | PDF | TXT | MD |
|------|------|-----|-----|-----|-----|
| DOCX | - | Yes | Yes | Yes | Yes |
| ODT | Yes | - | Yes | Yes | Yes |
| TXT | Yes | Yes | Yes | - | Yes |
| MD | Yes | Yes | Yes | Yes | - |
| DOC | Yes | Yes | Yes | Yes | Yes |

## Fidelity Notes

- DOCX ↔ ODT: Full formatting round-trip
- Any → PDF: Layout engine with font embedding
- Any → TXT: Text extraction (formatting lost)
- DOC → Any: Basic text extraction only
