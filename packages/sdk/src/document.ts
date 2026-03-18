import { EventEmitter } from './events.js';
import type {
  Format, SelectionRange, DocumentMetadata, DocumentStats,
  LayoutConfig, ChangeEvent, S1Error, CollabConfig
} from './types.js';
import { ErrorCodes } from './types.js';

/** Event map for S1Document. */
interface DocumentEvents {
  change: [ChangeEvent];
  save: [ArrayBuffer];
  export: [Format, ArrayBuffer];
}

/**
 * Represents an open document. Wraps the WASM WasmDocument.
 *
 * @example
 * ```ts
 * const doc = engine.create();
 * console.log(doc.toPlainText());
 * const pdf = doc.export('pdf');
 * ```
 */
export class S1Document extends EventEmitter<DocumentEvents> {
  /** @internal */
  _wasm: any;
  private _dirty = false;

  /** @internal */
  constructor(wasmDoc: any) {
    super();
    this._wasm = wasmDoc;
  }

  // ─── Content ──────────────────────────────────────

  /** Get the document rendered as HTML. */
  toHTML(): string {
    return this._wasm.to_html();
  }

  /** Get the document as paginated HTML with layout. */
  toPaginatedHTML(config?: LayoutConfig): string {
    if (config) {
      // Apply layout config if provided
      const wasmConfig = this._wasm.get_layout_config?.();
      if (wasmConfig && config.pageWidth) wasmConfig.set_page_width(config.pageWidth);
      if (wasmConfig && config.pageHeight) wasmConfig.set_page_height(config.pageHeight);
    }
    return this._wasm.to_paginated_html?.() ?? this._wasm.to_html();
  }

  /** Get the document as plain text. */
  toPlainText(): string {
    return this._wasm.to_plain_text();
  }

  // ─── Export ───────────────────────────────────────

  /** Export the document to the specified format. Returns raw bytes. */
  export(format: Format): ArrayBuffer {
    const bytes = this._wasm.export(format);
    const buffer = bytes instanceof ArrayBuffer ? bytes : bytes.buffer;
    this.emit('export', format, buffer);
    return buffer;
  }

  /** Export the document as a Blob. */
  exportBlob(format: Format): Blob {
    const buffer = this.export(format);
    const mimeTypes: Record<string, string> = {
      docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      odt: 'application/vnd.oasis.opendocument.text',
      pdf: 'application/pdf',
      txt: 'text/plain',
      md: 'text/markdown',
      html: 'text/html',
    };
    return new Blob([buffer], { type: mimeTypes[format] || 'application/octet-stream' });
  }

  /** Export the document as a data URL string. */
  exportDataUrl(format: Format): string {
    const blob = this.exportBlob(format);
    // Note: For large documents, use exportBlob() and URL.createObjectURL() instead.
    return URL.createObjectURL(blob);
  }

  // ─── Editing ──────────────────────────────────────

  /** Insert text at a position within a paragraph. */
  insertText(nodeId: string, offset: number, text: string): void {
    this._wasm.insert_text_in_paragraph(nodeId, offset, text);
    this._markDirty('insert');
  }

  /** Delete text from a paragraph. */
  deleteText(nodeId: string, offset: number, length: number): void {
    this._wasm.delete_text_in_paragraph(nodeId, offset, length);
    this._markDirty('delete');
  }

  /** Apply formatting to a selection range. */
  formatSelection(range: SelectionRange, key: string, value: string): void {
    this._wasm.format_selection(
      range.start.nodeId, range.start.offset,
      range.end.nodeId, range.end.offset,
      key, value
    );
    this._markDirty('format');
  }

  /** Insert a table after a node. */
  insertTable(afterNodeId: string, rows: number, cols: number): string {
    const id = this._wasm.insert_table(afterNodeId, rows, cols);
    this._markDirty('structure');
    return id;
  }

  /** Insert an image after a node. */
  insertImage(afterNodeId: string, data: ArrayBuffer, mimeType: string): string {
    const bytes = new Uint8Array(data);
    const id = this._wasm.insert_image(afterNodeId, bytes, mimeType);
    this._markDirty('structure');
    return id;
  }

  /** Split a paragraph at the given offset (like pressing Enter). */
  splitParagraph(nodeId: string, offset: number): string {
    const newId = this._wasm.split_paragraph(nodeId, offset);
    this._markDirty('structure');
    return newId;
  }

  /** Undo the last operation. Returns true if something was undone. */
  undo(): boolean {
    return this._wasm.undo();
  }

  /** Redo the last undone operation. Returns true if something was redone. */
  redo(): boolean {
    return this._wasm.redo();
  }

  /** Check if undo is available. */
  get canUndo(): boolean {
    return this._wasm.can_undo();
  }

  /** Check if redo is available. */
  get canRedo(): boolean {
    return this._wasm.can_redo();
  }

  // ─── Metadata ─────────────────────────────────────

  /** Get document metadata. */
  get metadata(): DocumentMetadata {
    const m = this._wasm.metadata?.() ?? {};
    return {
      title: m.title,
      author: m.creator || m.author,
      subject: m.subject,
      description: m.description,
    };
  }

  /** Get/set document title. */
  get title(): string {
    return this._wasm.metadata?.()?.title ?? '';
  }

  set title(value: string) {
    this._wasm.set_title(value);
  }

  /** Get document statistics. */
  get stats(): DocumentStats {
    try {
      const json = this._wasm.get_document_stats_json();
      return JSON.parse(json);
    } catch {
      const text = this.toPlainText();
      const words = text.split(/\s+/).filter(Boolean).length;
      return {
        words,
        characters: text.length,
        charactersNoSpaces: text.replace(/\s/g, '').length,
        paragraphs: 0,
        pages: 0,
      };
    }
  }

  /** Get page count. */
  get pageCount(): number {
    return this.stats.pages || 1;
  }

  /** Get word count. */
  get wordCount(): number {
    return this.stats.words;
  }

  // ─── State ────────────────────────────────────────

  /** Whether the document has unsaved changes. */
  get isDirty(): boolean {
    return this._dirty;
  }

  /** Mark the document as saved (clears dirty flag). */
  markClean(): void {
    this._dirty = false;
  }

  // ─── Lifecycle ────────────────────────────────────

  /** Release WASM memory. The document cannot be used after this. */
  destroy(): void {
    this._wasm.close?.();
    this._wasm = null;
    this.removeAllListeners();
  }

  // ─── Internal ─────────────────────────────────────

  private _markDirty(type: ChangeEvent['type']): void {
    this._dirty = true;
    this.emit('change', { type, timestamp: Date.now() });
  }
}
