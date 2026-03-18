# @s1engine/wasm

WebAssembly bindings for [s1engine](https://github.com/schnsrw/s1engine) — a modular document engine built in Rust.

## Features

- Read and write DOCX, ODT, PDF, TXT, and Markdown
- Full document model with undo/redo
- CRDT-based real-time collaboration
- Page layout with text shaping and pagination
- Format conversion (DOCX to ODT, PDF export, etc.)

## Installation

```bash
npm install @s1engine/wasm
```

## Quick Start

```javascript
import init, { WasmEngine } from '@s1engine/wasm';

// Initialize WASM module
await init();

// Create engine and document
const engine = new WasmEngine();
const doc = engine.create();

// Get document as HTML
const html = doc.to_html();

// Export to DOCX
const docxBytes = doc.to_docx();

// Open an existing DOCX file
const fileBytes = new Uint8Array(await file.arrayBuffer());
const doc2 = engine.open(fileBytes);
console.log(doc2.to_plain_text());
```

## Usage with Bundlers

### Vite

```javascript
import init, { WasmEngine } from '@s1engine/wasm';

const engine = await init().then(() => new WasmEngine());
```

### Webpack 5

Add to `webpack.config.js`:
```javascript
experiments: { asyncWebAssembly: true }
```

## API Reference

See the [full documentation](https://github.com/schnsrw/s1engine) for the complete API.

## License

MIT OR Apache-2.0
