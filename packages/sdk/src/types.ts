// ─── Format Types ─────────────────────────────────

/** Supported document formats. */
export type Format = 'docx' | 'odt' | 'pdf' | 'txt' | 'md' | 'html';

/** Source formats that can be opened. */
export type SourceFormat = 'docx' | 'odt' | 'txt' | 'md' | 'doc';

// ─── Selection & Range ────────────────────────────

/** A position within a text node. */
export interface Position {
  nodeId: string;
  offset: number;
}

/** A selection range in the document. */
export interface SelectionRange {
  start: Position;
  end: Position;
}

// ─── Event Types ──────────────────────────────────

export type EditorEvent =
  | 'change'
  | 'save'
  | 'export'
  | 'ready'
  | 'error'
  | 'selectionChange'
  | 'collabConnect'
  | 'collabDisconnect';

export interface ChangeEvent {
  type: 'insert' | 'delete' | 'format' | 'structure';
  timestamp: number;
}

// ─── Collaboration ────────────────────────────────

export interface CollabConfig {
  serverUrl: string;
  roomId: string;
  userName: string;
  userColor?: string;
  token?: string;
}

export interface CollabPeer {
  id: string;
  name: string;
  color: string;
  online: boolean;
}

// ─── Document Metadata ────────────────────────────

export interface DocumentMetadata {
  title?: string;
  author?: string;
  creator?: string;
  subject?: string;
  description?: string;
  created?: string;
  modified?: string;
}

// ─── Document Statistics ──────────────────────────

export interface DocumentStats {
  words: number;
  characters: number;
  charactersNoSpaces: number;
  paragraphs: number;
  pages: number;
}

// ─── Layout Config ────────────────────────────────

export interface LayoutConfig {
  pageWidth?: number;
  pageHeight?: number;
  marginTop?: number;
  marginBottom?: number;
  marginLeft?: number;
  marginRight?: number;
}

// ─── Editor Options ───────────────────────────────

export interface EditorOptions {
  /** Theme: 'default', 'dark', 'minimal', or custom Theme object. */
  theme?: 'default' | 'dark' | 'minimal' | Theme;
  /** Locale for UI strings (e.g., 'en', 'es', 'fr'). */
  locale?: string;
  /** Toolbar configuration. false = hidden. */
  toolbar?: ToolbarConfig | false;
  /** Show status bar. */
  statusBar?: boolean;
  /** Show ruler. */
  ruler?: boolean;
  /** Paginated view vs continuous scroll. */
  pageView?: boolean;
  /** Read-only mode. */
  readOnly?: boolean;
  /** Autosave configuration. false = disabled. */
  autosave?: AutosaveConfig | false;
  /** Enable browser spellcheck. */
  spellcheck?: boolean;
  /** Collaboration configuration. false = disabled. */
  collab?: CollabConfig | false;
  /** Which formats can be opened. */
  acceptFormats?: SourceFormat[];
  /** Which formats can be exported. */
  exportFormats?: Format[];
  /** Maximum file size in bytes. */
  maxFileSize?: number;
  /** Callbacks. */
  onReady?: () => void;
  onChange?: (event: ChangeEvent) => void;
  onSave?: (doc: any) => void | Promise<void>;
  onError?: (error: S1Error) => void;
  /** White-label branding. */
  branding?: BrandingConfig;
}

export interface Theme {
  primaryColor?: string;
  fontFamily?: string;
  toolbarBg?: string;
  toolbarBorder?: string;
  editorBg?: string;
  pageBg?: string;
  pageShadow?: string;
}

export interface ToolbarConfig {
  items: (string | ToolbarItem | '|')[];
}

export interface ToolbarItem {
  id: string;
  label: string;
  icon?: string;
  title?: string;
  onClick?: () => void;
}

export interface AutosaveConfig {
  interval?: number;
  onSave?: (data: ArrayBuffer) => void | Promise<void>;
}

export interface BrandingConfig {
  logo?: string;
  productName?: string;
  favicon?: string;
  accentColor?: string;
}

// ─── Errors ───────────────────────────────────────

export class S1Error extends Error {
  code: string;
  details?: Record<string, unknown>;

  constructor(code: string, message: string, details?: Record<string, unknown>) {
    super(message);
    this.name = 'S1Error';
    this.code = code;
    this.details = details;
  }
}

/** Error codes. */
export const ErrorCodes = {
  INIT_FAILED: 'INIT_FAILED',
  DOCUMENT_NOT_FOUND: 'DOCUMENT_NOT_FOUND',
  FORMAT_NOT_SUPPORTED: 'FORMAT_NOT_SUPPORTED',
  EXPORT_FAILED: 'EXPORT_FAILED',
  WASM_ERROR: 'WASM_ERROR',
  COLLAB_CONNECTION_FAILED: 'COLLAB_CONNECTION_FAILED',
  FILE_TOO_LARGE: 'FILE_TOO_LARGE',
  INVALID_ARGUMENT: 'INVALID_ARGUMENT',
} as const;
