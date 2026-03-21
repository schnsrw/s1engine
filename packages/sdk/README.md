# @rudra/sdk

Headless JavaScript/TypeScript SDK for [Rudra Code](https://github.com/schnsrw/s1engine). Manipulate documents (DOCX, ODT, PDF, TXT, Markdown) in the browser without any UI.

## Installation

```bash
npm install @rudra/sdk @rudra/wasm
```

## Quick Start

```typescript
import { S1Engine } from '@rudra/sdk';

// Initialize
const engine = await S1Engine.init();

// Create a new document
const doc = engine.create();
doc.title = 'My Report';
console.log(doc.toPlainText());

// Open an existing DOCX
const response = await fetch('/documents/report.docx');
const doc2 = engine.open(await response.arrayBuffer());
console.log(`Words: ${doc2.wordCount}`);

// Export to PDF
const pdfBlob = doc2.exportBlob('pdf');

// Edit programmatically
doc2.insertText('0:5', 0, 'Hello ');
doc2.formatSelection(
  { start: { nodeId: '0:5', offset: 0 }, end: { nodeId: '0:5', offset: 5 } },
  'bold', 'true'
);

// Listen to changes
doc2.on('change', (event) => {
  console.log(`Document changed: ${event.type}`);
});

// Clean up
doc2.destroy();
engine.destroy();
```

## API

### S1Engine

| Method | Description |
|--------|-------------|
| `S1Engine.init(wasmUrl?)` | Initialize WASM engine |
| `engine.create()` | Create empty document |
| `engine.open(data)` | Open document from bytes |
| `engine.openUrl(url)` | Fetch and open document |
| `engine.detectFormat(data)` | Detect format from bytes |
| `engine.destroy()` | Release memory |

### S1Document

| Method | Description |
|--------|-------------|
| `doc.toHTML()` | Get HTML rendering |
| `doc.toPlainText()` | Get plain text |
| `doc.export(format)` | Export as ArrayBuffer |
| `doc.exportBlob(format)` | Export as Blob |
| `doc.insertText(nodeId, offset, text)` | Insert text |
| `doc.deleteText(nodeId, offset, length)` | Delete text |
| `doc.formatSelection(range, key, value)` | Apply formatting |
| `doc.undo()` / `doc.redo()` | Undo/redo |
| `doc.title` | Get/set title |
| `doc.wordCount` | Get word count |
| `doc.stats` | Get full statistics |
| `doc.on(event, callback)` | Subscribe to events |
| `doc.destroy()` | Release memory |

## License

AGPL-3.0-or-later
