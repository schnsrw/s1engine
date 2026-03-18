# npm / WASM

## Install

```bash
npm install @s1engine/wasm
```

## Quick Start

```javascript
import init, { WasmEngine } from '@s1engine/wasm';

await init();
const engine = new WasmEngine();

// Create a document
const doc = engine.create();
const html = doc.to_html();

// Open a DOCX file
const response = await fetch('/report.docx');
const bytes = new Uint8Array(await response.arrayBuffer());
const doc2 = engine.open(bytes);
console.log(doc2.to_plain_text());

// Export to PDF
const pdfBytes = doc2.to_pdf();
```

## With Vite

```javascript
import init, { WasmEngine } from '@s1engine/wasm';
const engine = await init().then(() => new WasmEngine());
```

## With Webpack 5

```javascript
// webpack.config.js
module.exports = { experiments: { asyncWebAssembly: true } };
```

## Bundle Size

- Release: ~4.1 MB raw, ~1.6 MB gzipped
- Includes: DOCX, ODT, PDF, TXT, MD support + CRDT + layout engine
