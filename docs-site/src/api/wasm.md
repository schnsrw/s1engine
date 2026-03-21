# WASM / JavaScript API

## WasmEngine

```javascript
import init, { WasmEngine } from '@rudra/wasm';
await init();

const engine = new WasmEngine();
const doc = engine.create();
const doc2 = engine.open(uint8Array);
```

## WasmDocument

### Content
```javascript
doc.to_html()              // Full HTML rendering
doc.to_plain_text()        // Text extraction
doc.to_paginated_html()    // Paginated HTML with layout
```

### Export
```javascript
doc.to_docx()   // Uint8Array
doc.to_odt()    // Uint8Array
doc.to_pdf()    // Uint8Array
doc.export('md') // Uint8Array
```

### Editing
```javascript
doc.insert_text_in_paragraph(nodeId, offset, "text")
doc.delete_text_in_paragraph(nodeId, offset, length)
doc.split_paragraph(nodeId, offset)
doc.format_selection(startNode, startOff, endNode, endOff, key, value)
doc.set_heading_level(nodeId, level)
doc.insert_table(afterNodeId, rows, cols)
doc.insert_image(afterNodeId, bytes, mimeType)
```

### Undo/Redo
```javascript
doc.undo()       // Returns boolean
doc.redo()       // Returns boolean
doc.can_undo()   // Boolean
doc.can_redo()   // Boolean
```

### Metadata
```javascript
doc.set_title("My Doc")
doc.set_author("Alice")
doc.get_document_stats_json() // { words, characters, paragraphs, pages }
```
