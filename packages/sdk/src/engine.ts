import { S1Document } from './document.js';
import { S1Error, ErrorCodes } from './types.js';
import type { Format, SourceFormat } from './types.js';

/**
 * Main entry point for s1engine SDK.
 * Initializes the WASM module and provides document operations.
 *
 * @example
 * ```ts
 * import { S1Engine } from '@rudra/sdk';
 *
 * const engine = await S1Engine.init();
 * const doc = engine.create();
 * console.log(doc.toPlainText());
 * ```
 */
export class S1Engine {
  /** @internal */
  private _wasm: any;
  /** @internal — WASM module reference for creating layout configs etc. */
  private _wasmModule: any;

  private constructor(wasmEngine: any, wasmModule: any) {
    this._wasm = wasmEngine;
    this._wasmModule = wasmModule;
  }

  /**
   * Initialize the WASM engine. Must be called before any other operation.
   *
   * @param wasmUrl - Optional URL to the WASM binary. If not provided,
   *                  the SDK will try to load from the default location.
   */
  static async init(wasmUrl?: string): Promise<S1Engine> {
    try {
      // Dynamic import of the WASM module
      // Consumers must have @rudra/wasm installed as a peer dependency
      let wasmModule: any;
      try {
        wasmModule = await import('@rudra/wasm');
      } catch {
        throw new S1Error(
          ErrorCodes.INIT_FAILED,
          'Could not load @rudra/wasm. Install it: npm install @rudra/wasm'
        );
      }

      // Initialize WASM (calls the init() function exported by wasm-bindgen)
      if (typeof wasmModule.default === 'function') {
        await wasmModule.default(wasmUrl);
      }

      const wasmEngine = new wasmModule.WasmEngine();
      return new S1Engine(wasmEngine, wasmModule);
    } catch (e) {
      if (e instanceof S1Error) throw e;
      throw new S1Error(
        ErrorCodes.INIT_FAILED,
        `Failed to initialize WASM engine: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  /**
   * Create a new empty document.
   */
  create(): S1Document {
    const wasmDoc = this._wasm.create();
    return new S1Document(wasmDoc, this._wasmModule);
  }

  /**
   * Open a document from raw bytes. Format is auto-detected.
   *
   * @param data - Document bytes (DOCX, ODT, TXT, MD, DOC)
   */
  open(data: ArrayBuffer | Uint8Array): S1Document {
    try {
      const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
      const wasmDoc = this._wasm.open(bytes);
      return new S1Document(wasmDoc, this._wasmModule);
    } catch (e) {
      throw new S1Error(
        ErrorCodes.WASM_ERROR,
        `Failed to open document: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  /**
   * Open a document from a URL. Fetches the bytes and opens them.
   *
   * @param url - URL to fetch the document from
   */
  async openUrl(url: string): Promise<S1Document> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new S1Error(
        ErrorCodes.DOCUMENT_NOT_FOUND,
        `Failed to fetch document: ${response.status} ${response.statusText}`,
        { url }
      );
    }
    const buffer = await response.arrayBuffer();
    return this.open(buffer);
  }

  /**
   * Detect the format of a document from its bytes.
   *
   * @param data - Document bytes
   * @returns The detected format, or null if unknown
   */
  detectFormat(data: ArrayBuffer | Uint8Array): SourceFormat | null {
    try {
      const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
      const result = this._wasm.detect_format?.(bytes);
      return result ?? null;
    } catch {
      return null;
    }
  }

  /**
   * Get the s1engine version string.
   */
  get version(): string {
    return '1.0.1';
  }

  /**
   * Destroy the engine and release WASM memory.
   */
  destroy(): void {
    this._wasm = null;
  }
}
