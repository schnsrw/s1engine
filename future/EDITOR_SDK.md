# Editor SDK Specification

## Overview

The s1engine Editor SDK is a set of JavaScript/TypeScript packages that allow developers to embed a document editor into any web application. The SDK is framework-agnostic at its core, with thin adapters for React, Vue, and Web Components.

## Package Hierarchy

```
@s1engine/wasm          Base WASM engine (auto-generated from Rust)
    ↑
@s1engine/sdk           Headless SDK (wraps WASM, no UI)
    ↑
@s1engine/editor        Full editor with UI (toolbar, renderer, etc.)
    ↑
@s1engine/react         React adapter
@s1engine/vue           Vue adapter
@s1engine/web-component Web Component wrapper
```

Each package can be used independently:
- **@s1engine/wasm** — Direct WASM access for advanced users
- **@s1engine/sdk** — Headless document processing (conversion, text extraction)
- **@s1engine/editor** — Full WYSIWYG editor
- **@s1engine/react** — React component wrapping @s1engine/editor

---

## @s1engine/sdk — Headless SDK

### Installation

```bash
npm install @s1engine/sdk
```

### Quick Start

```typescript
import { S1Engine } from '@s1engine/sdk'

// Initialize engine (loads WASM)
const engine = await S1Engine.init()

// Create a new document
const doc = engine.create()

// Open an existing document
const response = await fetch('/my-document.docx')
const bytes = await response.arrayBuffer()
const doc = engine.open(bytes)

// Get content
console.log(doc.toPlainText())
console.log(doc.pageCount)
console.log(doc.wordCount)

// Export to PDF
const pdfBlob = doc.exportBlob('pdf')
downloadFile(pdfBlob, 'document.pdf')

// Export to different format
const odtBytes = doc.export('odt')
```

### API Reference

#### `S1Engine`

```typescript
class S1Engine {
  /**
   * Initialize the engine. Loads WASM module.
   * Call once at application startup.
   *
   * @param options.wasmUrl - Custom URL for WASM file (default: bundled)
   * @param options.fontData - Custom font data (ArrayBuffer of .ttf/.otf)
   */
  static async init(options?: {
    wasmUrl?: string
    fontData?: ArrayBuffer[]
  }): Promise<S1Engine>

  /** Create a new empty document */
  create(options?: { format?: Format; title?: string }): S1Document

  /** Open a document from binary data */
  open(data: ArrayBuffer | Uint8Array): S1Document

  /** Open a document from URL */
  async openUrl(url: string, options?: RequestInit): Promise<S1Document>

  /** Detect format of binary data without fully parsing */
  detectFormat(data: ArrayBuffer | Uint8Array): Format

  /** Engine version */
  readonly version: string
}
```

#### `S1Document`

```typescript
class S1Document {
  // ─── Properties ───────────────────────────────────────

  /** Document title (from metadata) */
  title: string

  /** Original format */
  readonly format: Format

  /** Number of pages (after layout) */
  readonly pageCount: number

  /** Word count */
  readonly wordCount: number

  /** Character count */
  readonly charCount: number

  /** Whether the document has been modified since last save/open */
  readonly isDirty: boolean

  // ─── Content Output ───────────────────────────────────

  /** Get paginated HTML (for display) */
  toPaginatedHTML(config?: LayoutConfig): string

  /** Get simple HTML (no pagination) */
  toHTML(): string

  /** Get plain text */
  toPlainText(): string

  /** Get JSON representation of document structure */
  toJSON(): DocumentJSON

  // ─── Export ───────────────────────────────────────────

  /** Export to format, returns raw bytes */
  export(format: Format): Uint8Array

  /** Export to format, returns Blob */
  exportBlob(format: Format): Blob

  /** Export to format, returns data URL (for download links) */
  exportDataUrl(format: Format): string

  // ─── Editing ──────────────────────────────────────────

  /** Insert text at position */
  insertText(nodeId: string, offset: number, text: string): void

  /** Delete text range */
  deleteText(nodeId: string, offset: number, length: number): void

  /** Split a paragraph at offset */
  splitParagraph(nodeId: string, offset: number): string  // returns new node ID

  /** Merge two adjacent paragraphs */
  mergeParagraphs(nodeId1: string, nodeId2: string): void

  /** Format a selection range */
  formatSelection(
    range: SelectionRange,
    attribute: string,
    value: AttributeValue
  ): void

  /** Set paragraph alignment */
  setAlignment(nodeId: string, alignment: 'left' | 'center' | 'right' | 'justify'): void

  /** Set heading level (0 = normal paragraph) */
  setHeadingLevel(nodeId: string, level: 0 | 1 | 2 | 3 | 4 | 5 | 6): void

  /** Insert a table after a node */
  insertTable(afterNodeId: string, rows: number, cols: number): string

  /** Insert an image */
  insertImage(
    afterNodeId: string,
    data: ArrayBuffer,
    mimeType: string,
    width?: number,
    height?: number
  ): string

  /** Insert a page break */
  insertPageBreak(afterNodeId: string): void

  // ─── Undo / Redo ─────────────────────────────────────

  /** Undo last operation */
  undo(): boolean

  /** Redo last undone operation */
  redo(): boolean

  /** Whether undo is available */
  readonly canUndo: boolean

  /** Whether redo is available */
  readonly canRedo: boolean

  // ─── Metadata ─────────────────────────────────────────

  /** Get/set document metadata */
  getMetadata(): DocumentMetadata
  setMetadata(meta: Partial<DocumentMetadata>): void

  // ─── Events ───────────────────────────────────────────

  /** Subscribe to document events */
  on<E extends keyof DocumentEvents>(
    event: E,
    callback: DocumentEvents[E]
  ): () => void  // returns unsubscribe function

  /** Unsubscribe from event */
  off(event: string, callback: Function): void

  // ─── Collaboration ───────────────────────────────────

  /** Enable CRDT collaboration */
  enableCollab(config: CollabConfig): CollabSession

  // ─── Cleanup ──────────────────────────────────────────

  /** Release WASM resources */
  dispose(): void
}
```

#### Types

```typescript
type Format = 'docx' | 'odt' | 'pdf' | 'txt' | 'md' | 'html'

interface LayoutConfig {
  pageWidth?: number    // points (default: 612 = US Letter)
  pageHeight?: number   // points (default: 792 = US Letter)
  marginTop?: number    // points (default: 72 = 1 inch)
  marginBottom?: number
  marginLeft?: number
  marginRight?: number
}

interface SelectionRange {
  startNodeId: string
  startOffset: number
  endNodeId: string
  endOffset: number
}

type AttributeValue = string | number | boolean

interface DocumentMetadata {
  title?: string
  author?: string
  subject?: string
  description?: string
  keywords?: string[]
  created?: Date
  modified?: Date
}

interface DocumentEvents {
  'change': (event: ChangeEvent) => void
  'selection': (event: SelectionEvent) => void
  'page-count': (count: number) => void
  'dirty': (isDirty: boolean) => void
  'error': (error: S1Error) => void
}

interface ChangeEvent {
  type: 'insert' | 'delete' | 'format' | 'structure'
  nodeId: string
  timestamp: number
}

interface CollabConfig {
  serverUrl: string     // WebSocket URL
  roomId: string
  userName: string
  userColor?: string    // hex color
  token?: string        // JWT for auth
  autoReconnect?: boolean
  reconnectInterval?: number
}

interface CollabSession {
  readonly connected: boolean
  readonly peers: CollabPeer[]
  disconnect(): void
  on(event: 'peer-join' | 'peer-leave' | 'connect' | 'disconnect', cb: Function): void
}
```

---

## @s1engine/editor — Embeddable Editor

### Installation

```bash
npm install @s1engine/editor
```

### Quick Start

```typescript
import { S1Editor } from '@s1engine/editor'

const editor = await S1Editor.create(
  document.getElementById('editor-container'),
  {
    theme: 'default',
    toolbar: 'standard',
    onReady: () => console.log('Editor ready!'),
    onSave: async (doc) => {
      const bytes = doc.export('docx')
      await uploadToServer(bytes)
    }
  }
)

// Open a document
const response = await fetch('/api/documents/123/content')
const bytes = await response.arrayBuffer()
editor.open(bytes)
```

### HTML Setup

```html
<!DOCTYPE html>
<html>
<head>
  <style>
    #editor-container {
      width: 100%;
      height: 100vh;
    }
  </style>
</head>
<body>
  <div id="editor-container"></div>
  <script type="module">
    import { S1Editor } from '@s1engine/editor'

    const editor = await S1Editor.create(
      document.getElementById('editor-container')
    )
  </script>
</body>
</html>
```

### API Reference

#### `S1Editor`

```typescript
class S1Editor {
  /**
   * Create an editor instance inside a container element.
   * This is the main entry point.
   */
  static async create(
    container: HTMLElement,
    options?: EditorOptions
  ): Promise<S1Editor>

  // ─── Document Operations ─────────────────────────────

  /** Open a document from binary data */
  open(data: ArrayBuffer | Uint8Array): void

  /** Open a document from URL */
  async openUrl(url: string): Promise<void>

  /** Create a new empty document */
  createNew(): void

  /** Get the underlying S1Document */
  getDocument(): S1Document

  /** Get the underlying S1Engine */
  getEngine(): S1Engine

  // ─── Editor State ────────────────────────────────────

  /** Whether the document has unsaved changes */
  readonly isDirty: boolean

  /** Whether the editor is in read-only mode */
  readOnly: boolean

  /** Current zoom level (1.0 = 100%) */
  zoom: number

  /** Current page being viewed */
  readonly currentPage: number

  /** Total page count */
  readonly pageCount: number

  // ─── Toolbar ─────────────────────────────────────────

  /** Replace the entire toolbar configuration */
  setToolbar(config: ToolbarConfig): void

  /** Add a toolbar item */
  addToolbarItem(item: ToolbarItem, position?: number): void

  /** Remove a toolbar item by ID */
  removeToolbarItem(id: string): void

  /** Show/hide the toolbar */
  showToolbar(visible: boolean): void

  // ─── Theme ───────────────────────────────────────────

  /** Set theme by name or custom theme object */
  setTheme(theme: 'default' | 'dark' | 'minimal' | Theme): void

  /** Set individual CSS variable */
  setStyleVar(name: string, value: string): void

  // ─── Collaboration ───────────────────────────────────

  /** Start collaboration session */
  startCollab(config: CollabConfig): CollabSession

  /** Stop collaboration */
  stopCollab(): void

  // ─── Events ──────────────────────────────────────────

  /** Subscribe to editor events */
  on<E extends keyof EditorEvents>(
    event: E,
    callback: EditorEvents[E]
  ): () => void

  // ─── Lifecycle ───────────────────────────────────────

  /** Destroy the editor and release all resources */
  destroy(): void
}
```

#### `EditorOptions`

```typescript
interface EditorOptions {
  // ─── Appearance ──────────────────────────────────────
  theme?: 'default' | 'dark' | 'minimal' | Theme
  locale?: string                 // BCP-47 locale (default: 'en')
  toolbar?: ToolbarConfig | ToolbarPreset | false
  statusBar?: boolean             // Show status bar (default: true)
  ruler?: boolean                 // Show page ruler (default: false)
  pageView?: boolean              // Paginated view (default: true)
  zoom?: number                   // Initial zoom (default: 1.0)

  // ─── Behavior ────────────────────────────────────────
  readOnly?: boolean              // Read-only mode (default: false)
  autosave?: {
    enabled: boolean
    interval: number              // milliseconds (default: 30000)
    storage: 'indexeddb' | 'localstorage' | 'none'
  } | false
  spellcheck?: boolean            // Browser spellcheck (default: true)
  maxFileSize?: number            // Max file size in bytes

  // ─── File Handling ───────────────────────────────────
  acceptFormats?: Format[]        // Formats that can be opened
  exportFormats?: Format[]        // Formats available in export menu

  // ─── Collaboration ──────────────────────────────────
  collab?: CollabConfig | false

  // ─── White-label / Branding ──────────────────────────
  branding?: {
    logo?: string                 // URL to logo image
    productName?: string          // Custom product name (replaces "s1engine")
    favicon?: string              // URL to favicon
    poweredBy?: boolean           // Show "Powered by s1engine" (default: true)
  }

  // ─── Callbacks ───────────────────────────────────────
  onReady?: () => void
  onChange?: (event: ChangeEvent) => void
  onSave?: (doc: S1Document) => void | Promise<void>
  onError?: (error: S1Error) => void
  onFileOpen?: (file: File) => boolean | Promise<boolean>  // Return false to cancel
  onExport?: (format: Format, blob: Blob) => void | Promise<void>
}
```

#### Toolbar Configuration

```typescript
type ToolbarPreset = 'full' | 'standard' | 'minimal' | 'none'

interface ToolbarConfig {
  preset?: ToolbarPreset          // Start from preset, then customize
  items: ToolbarItemOrSeparator[]
  position?: 'top' | 'bottom'    // Toolbar position (default: 'top')
  sticky?: boolean               // Stick to viewport (default: true)
}

type ToolbarItemOrSeparator = ToolbarItem | '|'

interface ToolbarItem {
  id: string                      // Unique identifier
  type: 'button' | 'dropdown' | 'color-picker' | 'select' | 'custom'
  label?: string                  // Display label
  icon?: string                   // Icon (SVG string or URL)
  tooltip?: string                // Hover tooltip
  shortcut?: string               // Keyboard shortcut display (e.g., "Ctrl+B")
  disabled?: boolean
  active?: boolean                // Toggled state (for bold, italic, etc.)
  onClick?: (editor: S1Editor) => void
  items?: DropdownItem[]          // For dropdown type
  component?: HTMLElement         // For custom type
}

// Built-in toolbar item IDs
type BuiltInToolbarItem =
  | 'undo' | 'redo'
  | 'bold' | 'italic' | 'underline' | 'strikethrough'
  | 'font-family' | 'font-size' | 'font-color' | 'highlight-color'
  | 'heading'
  | 'align-left' | 'align-center' | 'align-right' | 'align-justify'
  | 'bullet-list' | 'numbered-list'
  | 'indent' | 'outdent'
  | 'insert-table' | 'insert-image' | 'insert-link'
  | 'insert-page-break' | 'insert-horizontal-rule'
  | 'line-spacing'
  | 'find-replace'
  | 'export-pdf' | 'export-docx'
  | 'print'
  | 'fullscreen'
```

#### Preset Toolbar Definitions

```typescript
// 'full' — all available items
const FULL_TOOLBAR = [
  'undo', 'redo', '|',
  'font-family', 'font-size', '|',
  'bold', 'italic', 'underline', 'strikethrough', '|',
  'font-color', 'highlight-color', '|',
  'heading', '|',
  'align-left', 'align-center', 'align-right', 'align-justify', '|',
  'bullet-list', 'numbered-list', 'indent', 'outdent', '|',
  'line-spacing', '|',
  'insert-table', 'insert-image', 'insert-link', '|',
  'insert-page-break', 'insert-horizontal-rule', '|',
  'find-replace', '|',
  'export-pdf', 'export-docx', 'print', '|',
  'fullscreen'
]

// 'standard' — most common items
const STANDARD_TOOLBAR = [
  'undo', 'redo', '|',
  'font-family', 'font-size', '|',
  'bold', 'italic', 'underline', '|',
  'font-color', '|',
  'heading', '|',
  'align-left', 'align-center', 'align-right', '|',
  'bullet-list', 'numbered-list', '|',
  'insert-table', 'insert-image', 'insert-link', '|',
  'export-pdf'
]

// 'minimal' — basic formatting only
const MINIMAL_TOOLBAR = [
  'bold', 'italic', 'underline', '|',
  'heading', '|',
  'bullet-list', 'numbered-list'
]
```

#### Theme Configuration

```typescript
interface Theme {
  name: string

  // Colors
  primaryColor: string            // Brand color
  backgroundColor: string         // Editor background
  surfaceColor: string            // Page/card background
  textColor: string               // Default text color
  mutedTextColor: string          // Secondary text
  borderColor: string             // Borders and dividers
  hoverColor: string              // Hover states
  activeColor: string             // Active/selected states
  errorColor: string              // Error states
  successColor: string            // Success states

  // Toolbar
  toolbarBackground: string
  toolbarBorder: string
  toolbarButtonHover: string
  toolbarButtonActive: string
  toolbarDropdownBackground: string

  // Editor
  editorBackground: string        // Area around pages
  pageBackground: string          // Page surface
  pageShadow: string              // Page drop shadow
  selectionColor: string          // Text selection
  cursorColor: string             // Caret color

  // Typography
  uiFontFamily: string            // UI elements (toolbar, menus)
  uiFontSize: string              // Base UI font size
  uiFontWeight: string

  // Spacing
  toolbarHeight: string
  toolbarPadding: string
  pageGap: string                 // Gap between pages

  // Borders
  borderRadius: string
  pageBorderRadius: string
}
```

---

## @s1engine/react — React Adapter

### Installation

```bash
npm install @s1engine/react
```

### Usage

```tsx
import { S1Editor, useS1Engine } from '@s1engine/react'

function MyApp() {
  const editorRef = useRef<S1EditorHandle>(null)

  const handleSave = async (doc: S1Document) => {
    const bytes = doc.export('docx')
    await fetch('/api/save', {
      method: 'POST',
      body: bytes
    })
  }

  return (
    <div style={{ height: '100vh' }}>
      <S1Editor
        ref={editorRef}
        theme="default"
        toolbar="standard"
        readOnly={false}
        collab={{
          serverUrl: 'wss://collab.example.com',
          roomId: 'doc-123',
          userName: 'Alice'
        }}
        onReady={() => {
          // Load initial document
          fetch('/api/documents/123/content')
            .then(r => r.arrayBuffer())
            .then(bytes => editorRef.current?.open(bytes))
        }}
        onSave={handleSave}
        onChange={(e) => console.log('Document changed:', e.type)}
      />
    </div>
  )
}
```

### Hook: `useS1Engine`

```tsx
function DocumentProcessor() {
  const engine = useS1Engine()

  const convertToPdf = async (file: File) => {
    const bytes = await file.arrayBuffer()
    const doc = engine.open(bytes)
    const pdfBlob = doc.exportBlob('pdf')
    downloadFile(pdfBlob, 'converted.pdf')
    doc.dispose()
  }

  return <button onClick={() => /* file picker */}>Convert to PDF</button>
}
```

### Props

```typescript
interface S1EditorProps {
  // All EditorOptions properties are valid props
  theme?: 'default' | 'dark' | 'minimal' | Theme
  toolbar?: ToolbarConfig | ToolbarPreset | false
  readOnly?: boolean
  locale?: string
  collab?: CollabConfig | false
  branding?: BrandingConfig

  // React-specific
  className?: string
  style?: React.CSSProperties

  // Callbacks (same as EditorOptions but as React props)
  onReady?: () => void
  onChange?: (event: ChangeEvent) => void
  onSave?: (doc: S1Document) => void | Promise<void>
  onError?: (error: S1Error) => void

  // Initial document
  initialData?: ArrayBuffer | Uint8Array
  initialUrl?: string
}

interface S1EditorHandle {
  open(data: ArrayBuffer): void
  openUrl(url: string): Promise<void>
  createNew(): void
  getDocument(): S1Document
  getEngine(): S1Engine
  setReadOnly(readOnly: boolean): void
  setZoom(zoom: number): void
  destroy(): void
}
```

---

## @s1engine/vue — Vue Adapter

### Installation

```bash
npm install @s1engine/vue
```

### Usage

```vue
<template>
  <S1Editor
    ref="editorRef"
    theme="default"
    toolbar="standard"
    :collab="collabConfig"
    @ready="onReady"
    @change="onChange"
    @save="onSave"
    style="height: 100vh"
  />
</template>

<script setup>
import { ref } from 'vue'
import { S1Editor } from '@s1engine/vue'

const editorRef = ref(null)

const collabConfig = {
  serverUrl: 'wss://collab.example.com',
  roomId: 'doc-123',
  userName: 'Alice'
}

function onReady() {
  fetch('/api/documents/123/content')
    .then(r => r.arrayBuffer())
    .then(bytes => editorRef.value.open(bytes))
}

function onChange(event) {
  console.log('Changed:', event.type)
}

async function onSave(doc) {
  const bytes = doc.export('docx')
  await fetch('/api/save', { method: 'POST', body: bytes })
}
</script>
```

---

## @s1engine/web-component — Web Component

### Installation

```bash
npm install @s1engine/web-component
```

### Usage

```html
<script type="module">
  import '@s1engine/web-component'
</script>

<s1-editor
  theme="default"
  toolbar="standard"
  collab-url="wss://collab.example.com"
  collab-room="doc-123"
  collab-user="Alice"
  style="height: 100vh; display: block;"
></s1-editor>

<script>
  const editor = document.querySelector('s1-editor')

  editor.addEventListener('s1-ready', () => {
    fetch('/api/documents/123/content')
      .then(r => r.arrayBuffer())
      .then(bytes => editor.open(bytes))
  })

  editor.addEventListener('s1-save', (e) => {
    const doc = e.detail.document
    // save logic
  })
</script>
```

### Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `theme` | string | Theme name |
| `toolbar` | string | Toolbar preset name |
| `read-only` | boolean | Read-only mode |
| `locale` | string | BCP-47 locale |
| `collab-url` | string | Collaboration server URL |
| `collab-room` | string | Room ID |
| `collab-user` | string | User display name |
| `collab-token` | string | JWT for auth |

### Events

| Event | Detail | Description |
|-------|--------|-------------|
| `s1-ready` | `{}` | Editor initialized |
| `s1-change` | `{ type, nodeId }` | Document changed |
| `s1-save` | `{ document }` | Save triggered |
| `s1-error` | `{ error }` | Error occurred |

### Methods

| Method | Description |
|--------|-------------|
| `open(data: ArrayBuffer)` | Open document |
| `openUrl(url: string)` | Open from URL |
| `createNew()` | Create empty document |
| `getDocument()` | Get S1Document |
| `setReadOnly(bool)` | Toggle read-only |
| `destroy()` | Clean up |

---

## Build & Bundle

### Package Builds

Each package is built with its own bundler configuration:

| Package | Build Tool | Output |
|---------|-----------|--------|
| @s1engine/wasm | wasm-pack | ESM + .wasm |
| @s1engine/sdk | Rollup | ESM + CJS + .d.ts |
| @s1engine/editor | Vite (lib mode) | ESM + CJS + CSS + .d.ts |
| @s1engine/react | Rollup | ESM + CJS + .d.ts |
| @s1engine/vue | Vite | ESM + .d.ts |
| @s1engine/web-component | Rollup | ESM (single file) |

### Bundle Sizes (targets)

| Package | Size (gzipped) |
|---------|---------------|
| @s1engine/wasm | ~1.5 MB |
| @s1engine/sdk | ~15 KB |
| @s1engine/editor | ~80 KB (+ CSS ~20 KB) |
| @s1engine/react | ~5 KB |
| @s1engine/vue | ~5 KB |
| @s1engine/web-component | ~3 KB |
| **Total (editor + WASM)** | **~1.6 MB** |

### Tree Shaking

All packages support tree-shaking via ES modules. If a consumer only uses the headless SDK (no editor UI), the editor CSS and toolbar code are not included in their bundle.

### CDN Usage

For quick prototyping without a bundler:

```html
<script type="module">
  import { S1Editor } from 'https://cdn.jsdelivr.net/npm/@s1engine/editor/dist/index.js'

  const editor = await S1Editor.create(document.body)
</script>
```
