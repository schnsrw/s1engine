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
      const handler = (e: MessageEvent) => {
        if (e.data?.type === 'exported' && e.data.format === format) {
          window.removeEventListener('message', handler);
          resolve(new Blob([new Uint8Array(e.data.bytes)]));
        }
        if (e.data?.type === 'exportError') {
          window.removeEventListener('message', handler);
          reject(new Error(e.data.message));
        }
      };
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
    if (this.iframe) {
      this.iframe.remove();
      this.iframe = null;
    }
    this.container.innerHTML = '';
    this._ready = false;
  }

  // ─── Private ──────────────────────────────────────

  private async _init(): Promise<void> {
    // For now, embed the editor via iframe pointing to the built editor
    // In the future, this will render directly into the container
    this.container.style.position = 'relative';
    this.container.style.overflow = 'hidden';

    this.iframe = document.createElement('iframe');
    this.iframe.style.width = '100%';
    this.iframe.style.height = '100%';
    this.iframe.style.border = 'none';
    this.iframe.setAttribute('title', 's1 Document Editor');
    this.iframe.setAttribute('allow', 'clipboard-read; clipboard-write');

    // The src would point to the built editor
    // For development, this can be localhost
    const editorUrl = this.options.branding?.logo
      ? '/editor/index.html' // customized
      : '/editor/index.html'; // default

    this.iframe.src = editorUrl;
    this.container.appendChild(this.iframe);

    // Wait for editor to signal ready
    await new Promise<void>((resolve) => {
      const handler = (e: MessageEvent) => {
        if (e.data?.type === 's1-editor-ready') {
          window.removeEventListener('message', handler);
          this._ready = true;
          this._applyOptions();
          this.options.onReady?.();
          resolve();
        }
      };
      window.addEventListener('message', handler);

      // Timeout fallback
      setTimeout(() => {
        if (!this._ready) {
          this._ready = true;
          resolve();
        }
      }, 5000);
    });
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
  }

  private _postMessage(message: Record<string, unknown>): void {
    if (this.iframe?.contentWindow) {
      this.iframe.contentWindow.postMessage(message, '*');
    }
  }
}

// Re-export types
export type { EditorOptions, Format, ToolbarConfig, ToolbarItem, Theme } from '@rudra/sdk';
