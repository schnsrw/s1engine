# Quick Start

Get up and running with s1engine in 5 minutes.

## Option 1: Rust Library

```bash
cargo add s1engine
```

```rust
use s1engine::{Engine, DocumentBuilder, Format};

// Open a DOCX file
let engine = Engine::new();
let data = std::fs::read("report.docx")?;
let doc = engine.open(&data)?;
println!("{}", doc.to_plain_text());

// Create a document programmatically
let doc = DocumentBuilder::new()
    .title("My Report")
    .heading(1, "Introduction")
    .text("Built with s1engine.")
    .build();

// Export to PDF
let pdf_bytes = doc.export(Format::Pdf)?;
std::fs::write("output.pdf", pdf_bytes)?;
```

## Option 2: npm / WASM (Browser)

```bash
npm install @s1engine/wasm
```

```javascript
import init, { WasmEngine } from '@s1engine/wasm';

await init();
const engine = new WasmEngine();
const doc = engine.create();
const html = doc.to_html();
```

## Option 3: Docker (Full Editor)

```bash
docker run -p 8787:8787 s1engine/editor
```

Open `http://localhost:8787` in your browser.

## Next Steps

- [Installation details](./installation.md)
- [Embed in React](../guides/react.md)
- [Format conversion guide](../guides/conversion.md)
- [Collaboration setup](../guides/collaboration.md)
