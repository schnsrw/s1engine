/**
 * @s1engine/web-component — Custom element for the s1engine editor.
 *
 * @example
 * ```html
 * <script type="module" src="@s1engine/web-component"></script>
 * <s1-editor
 *   theme="default"
 *   toolbar="standard"
 * ></s1-editor>
 * ```
 */

import { S1Editor, Toolbars } from '@s1engine/editor';
import type { Format } from '@s1engine/editor';

export class S1EditorElement extends HTMLElement {
  private editor: S1Editor | null = null;
  private _shadow: ShadowRoot;

  static get observedAttributes() {
    return ['theme', 'toolbar', 'read-only', 'src'];
  }

  constructor() {
    super();
    this._shadow = this.attachShadow({ mode: 'open' });
    this._shadow.innerHTML = `
      <style>
        :host { display: block; width: 100%; height: 100%; }
        .s1-container { width: 100%; height: 100%; }
      </style>
      <div class="s1-container"></div>
    `;
  }

  async connectedCallback() {
    const container = this._shadow.querySelector('.s1-container') as HTMLElement;
    if (!container) return;

    const theme = this.getAttribute('theme') || 'default';
    const toolbarName = this.getAttribute('toolbar') || 'standard';
    const readOnly = this.hasAttribute('read-only');

    const toolbar = (Toolbars as any)[toolbarName] || Toolbars.standard;

    this.editor = await S1Editor.create(container, {
      theme: theme as any,
      toolbar,
      readOnly,
      onReady: () => this.dispatchEvent(new CustomEvent('ready')),
      onChange: (e) => this.dispatchEvent(new CustomEvent('change', { detail: e })),
    });

    // Auto-load src if set
    const src = this.getAttribute('src');
    if (src) {
      await this.editor.openUrl(src);
    }
  }

  disconnectedCallback() {
    this.editor?.destroy();
    this.editor = null;
  }

  attributeChangedCallback(name: string, _old: string, value: string) {
    if (!this.editor) return;
    if (name === 'theme') this.editor.setTheme(value);
    if (name === 'read-only') this.editor.readOnly = value !== null;
    if (name === 'toolbar') {
      const tb = (Toolbars as any)[value] || Toolbars.standard;
      this.editor.setToolbar(tb);
    }
  }

  // ─── Public API ─────────────────────────────────

  /** Open a document from bytes. */
  open(data: ArrayBuffer) { this.editor?.open(data); }

  /** Open a document from URL. */
  async openUrl(url: string) { await this.editor?.openUrl(url); }

  /** Create a new document. */
  createNew() { this.editor?.createNew(); }

  /** Export the document. */
  async exportDocument(format: Format): Promise<Blob | undefined> {
    return this.editor?.exportDocument(format);
  }
}

// Register the custom element
if (!customElements.get('s1-editor')) {
  customElements.define('s1-editor', S1EditorElement);
}

export { Toolbars } from '@s1engine/editor';
