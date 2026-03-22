/**
 * @rudra/editor — Embeddable document editor component.
 *
 * @example
 * ```ts
 * import { S1Editor } from '@rudra/editor';
 * import '@rudra/editor/style.css';
 *
 * const editor = await S1Editor.create(document.getElementById('editor'), {
 *   theme: 'default',
 *   toolbar: 'standard',
 *   onReady: () => console.log('Editor ready'),
 * });
 *
 * // Open a document
 * const response = await fetch('/report.docx');
 * editor.open(await response.arrayBuffer());
 * ```
 *
 * @packageDocumentation
 */

import type { EditorOptions, Format, ToolbarConfig, Theme } from '@rudra/sdk';

/** Toolbar presets. */
export const Toolbars = {
  full: {
    items: [
      'undo', 'redo', '|',
      'style-gallery', '|',
      'font-family', 'font-size', '|',
      'bold', 'italic', 'underline', 'strikethrough', '|',
      'text-color', 'highlight-color', '|',
      'align-left', 'align-center', 'align-right', 'align-justify', '|',
      'bullet-list', 'numbered-list', 'indent', 'outdent', '|',
      'insert-table', 'insert-image', 'insert-link', '|',
      'line-spacing', '|',
      'format-painter', '|',
      'find-replace',
    ],
  } satisfies ToolbarConfig,

  standard: {
    items: [
      'undo', 'redo', '|',
      'bold', 'italic', 'underline', '|',
      'font-family', 'font-size', '|',
      'text-color', '|',
      'align-left', 'align-center', 'align-right', '|',
      'bullet-list', 'numbered-list', '|',
      'insert-table', 'insert-image', 'insert-link',
    ],
  } satisfies ToolbarConfig,

  minimal: {
    items: [
      'bold', 'italic', 'underline', '|',
      'bullet-list', 'numbered-list',
    ],
  } satisfies ToolbarConfig,

  none: {
    items: [],
  } satisfies ToolbarConfig,
};

/** Default initialization timeout in milliseconds. */
const INIT_TIMEOUT_MS = 10_000;

/** Default export timeout in milliseconds. */
const EXPORT_TIMEOUT_MS = 30_000;

/**
 * Embeddable document editor.
 *
 * Create an editor instance by calling `S1Editor.create(container, options)`.
 * The editor will render inside the given container element.
 */
export class S1Editor {
  private container: HTMLElement;
  private options: EditorOptions;
  private iframe: HTMLIFrameElement | null = null;
  private _ready = false;
  private _dirty = false;
  private _iframeOrigin: string = '';
  private _messageHandler: ((e: MessageEvent) => void) | null = null;
  private _pendingExportCleanups: Array<() => void> = [];

  private constructor(container: HTMLElement, options: EditorOptions) {
    this.container = container;
    this.options = options;
  }

  /**
   * Create a new editor instance.
   *
   * @param container - The HTML element to render the editor into
   * @param options - Editor configuration options
   * @returns A promise that resolves when the editor is ready
   */
  static async create(
    container: HTMLElement,
    options: EditorOptions = {}
  ): Promise<S1Editor> {
    const editor = new S1Editor(container, options);
    await editor._init();
    return editor;
  }

  /**
   * Open a document from raw bytes.
   */
  open(data: ArrayBuffer): void {
    this._postMessage({ type: 'open', data: Array.from(new Uint8Array(data)) });
  }

  /**
   * Open a document from a URL.
   */
  async openUrl(url: string): Promise<void> {
    const response = await fetch(url);
    if (!response.ok) {
      const message = `Failed to fetch document: ${response.status} ${response.statusText}`;
      this._emitError('DOCUMENT_NOT_FOUND', message);
      throw new Error(message);
    }
    const buffer = await response.arrayBuffer();
    this.open(buffer);
  }

  /**
   * Create a new empty document.
   */
  createNew(): void {
    this._postMessage({ type: 'new' });
  }

  /**
   * Export the document to the specified format.
   */
  async exportDocument(format: Format): Promise<Blob> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        cleanup();
        const message = `Export timed out after ${EXPORT_TIMEOUT_MS}ms`;
        this._emitError('EXPORT_FAILED', message);
        reject(new Error(message));
      }, EXPORT_TIMEOUT_MS);

      const handler = (e: MessageEvent) => {
        if (!this._isValidMessage(e)) return;
        if (e.data?.type === 'exported' && e.data.format === format) {
          cleanup();
          resolve(new Blob([new Uint8Array(e.data.bytes)]));
        }
        if (e.data?.type === 'exportError') {
          cleanup();
          reject(new Error(e.data.message));
        }
      };

      const cleanup = () => {
        clearTimeout(timeout);
        window.removeEventListener('message', handler);
        this._pendingExportCleanups = this._pendingExportCleanups.filter(fn => fn !== cleanup);
      };

      this._pendingExportCleanups.push(cleanup);
      window.addEventListener('message', handler);
      this._postMessage({ type: 'export', format });
    });
  }

  /** Whether the document has unsaved changes. */
  get isDirty(): boolean {
    return this._dirty;
  }

  /** Whether the editor is in read-only mode. */
  get isReadOnly(): boolean {
    return this.options.readOnly ?? false;
  }

  /** Set read-only mode. */
  set readOnly(value: boolean) {
    this.options.readOnly = value;
    this._postMessage({ type: 'setReadOnly', value });
  }

  /** Set the toolbar configuration. */
  setToolbar(config: ToolbarConfig | keyof typeof Toolbars): void {
    const resolved = typeof config === 'string' ? Toolbars[config] : config;
    this._postMessage({ type: 'setToolbar', config: resolved });
  }

  /** Set the editor theme. */
  setTheme(theme: Theme | string): void {
    this._postMessage({ type: 'setTheme', theme });
  }

  /** Destroy the editor and clean up resources. */
  destroy(): void {
    // Clean up pending export listeners
    for (const cleanup of this._pendingExportCleanups) {
      cleanup();
    }
    this._pendingExportCleanups = [];

    // Remove global message handler
    if (this._messageHandler) {
      window.removeEventListener('message', this._messageHandler);
      this._messageHandler = null;
    }

    if (this.iframe) {
      this.iframe.remove();
      this.iframe = null;
    }
    this.container.innerHTML = '';
    this._ready = false;
    this._dirty = false;
  }

  // ─── Private ──────────────────────────────────────

  private async _init(): Promise<void> {
    this.container.style.position = 'relative';
    this.container.style.overflow = 'hidden';

    this.iframe = document.createElement('iframe');
    this.iframe.style.width = '100%';
    this.iframe.style.height = '100%';
    this.iframe.style.border = 'none';
    this.iframe.setAttribute('title', 's1 Document Editor');
    this.iframe.setAttribute('allow', 'clipboard-read; clipboard-write');

    // Configurable editor URL — supports non-root deployments, CDN hosting,
    // and custom editor builds. Defaults to '/editor/index.html'.
    const editorUrl = this.options.editorUrl ?? '/editor/index.html';
    this._iframeOrigin = this._resolveOrigin(editorUrl);

    this.iframe.src = editorUrl;
    this.container.appendChild(this.iframe);

    // Set up persistent message handler for change/save/error events
    this._messageHandler = (e: MessageEvent) => this._handleMessage(e);
    window.addEventListener('message', this._messageHandler);

    // Wait for editor to signal ready
    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        if (!this._ready) {
          const message =
            `Editor failed to initialize within ${INIT_TIMEOUT_MS}ms. ` +
            `Ensure the editor is hosted at: ${editorUrl}`;
          this._emitError('INIT_FAILED', message);
          reject(new Error(message));
        }
      }, INIT_TIMEOUT_MS);

      const readyHandler = (e: MessageEvent) => {
        if (!this._isValidMessage(e)) return;
        if (e.data?.type === 's1-editor-ready') {
          window.removeEventListener('message', readyHandler);
          clearTimeout(timeout);
          this._ready = true;
          this._applyOptions();
          this.options.onReady?.();
          resolve();
        }
      };
      window.addEventListener('message', readyHandler);
    });
  }

  /** Handle incoming messages from the editor iframe. */
  private _handleMessage(e: MessageEvent): void {
    if (!this._isValidMessage(e)) return;

    switch (e.data?.type) {
      case 's1-document-changed':
        this._dirty = true;
        this.options.onChange?.(e.data.event ?? { type: 'structure', timestamp: Date.now() });
        break;
      case 's1-document-saved':
        this._dirty = false;
        this.options.onSave?.(e.data.document);
        break;
      case 's1-editor-error':
        this._emitError(e.data.code ?? 'WASM_ERROR', e.data.message ?? 'Unknown editor error');
        break;
    }
  }

  /** Validate that a message event comes from our editor iframe. */
  private _isValidMessage(e: MessageEvent): boolean {
    // Verify the message comes from the expected origin
    if (this._iframeOrigin && e.origin !== this._iframeOrigin) return false;
    // Verify the message comes from our iframe's window
    if (this.iframe && e.source !== this.iframe.contentWindow) return false;
    return true;
  }

  /** Resolve the origin from an editor URL (handles relative and absolute URLs). */
  private _resolveOrigin(url: string): string {
    try {
      return new URL(url, window.location.href).origin;
    } catch {
      return window.location.origin;
    }
  }

  /** Emit an error through the onError callback. */
  private _emitError(code: string, message: string): void {
    this.options.onError?.({ name: 'S1Error', message, code } as any);
  }

  private _applyOptions(): void {
    if (this.options.readOnly) {
      this._postMessage({ type: 'setReadOnly', value: true });
    }
    if (this.options.theme) {
      this._postMessage({ type: 'setTheme', theme: this.options.theme });
    }
    if (this.options.toolbar === false) {
      this._postMessage({ type: 'setToolbar', config: Toolbars.none });
    } else if (this.options.toolbar) {
      this._postMessage({ type: 'setToolbar', config: this.options.toolbar });
    }
    // Forward additional supported options to the iframe editor
    if (this.options.statusBar !== undefined) {
      this._postMessage({ type: 'setStatusBar', value: this.options.statusBar });
    }
    if (this.options.ruler !== undefined) {
      this._postMessage({ type: 'setRuler', value: this.options.ruler });
    }
    if (this.options.pageView !== undefined) {
      this._postMessage({ type: 'setPageView', value: this.options.pageView });
    }
    if (this.options.spellcheck !== undefined) {
      this._postMessage({ type: 'setSpellcheck', value: this.options.spellcheck });
    }
  }

  private _postMessage(message: Record<string, unknown>): void {
    if (this.iframe?.contentWindow) {
      this.iframe.contentWindow.postMessage(message, this._iframeOrigin || '*');
    }
  }
}

// Re-export types
export type { EditorOptions, Format, ToolbarConfig, ToolbarItem, Theme } from '@rudra/sdk';
