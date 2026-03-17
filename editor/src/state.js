// Central editor state — single source of truth
export const state = {
  engine: null,
  doc: null,
  currentView: 'editor',
  currentFormat: '',
  pagesRendered: false,
  ignoreInput: false,
  selectedImg: null,
  resizing: null,
  syncTimer: null,
  lastSelInfo: null,
  _selectAll: false,
  syncedTextCache: new Map(),
  // Table context menu
  ctxTable: null,
  ctxCell: null,
  ctxRow: 0,
  ctxCol: 0,
  // Find
  findMatches: [],
  findIndex: -1,
  // Internal clipboard for rich paste
  internalClipboard: null,
  // Zoom
  zoomLevel: 100,
  // Autosave
  autosaveTimer: null,
  dirty: false,
  tabId: Date.now() + '-' + Math.random().toString(36).slice(2),
  lastSaveTimestamp: 0,
  // Version history
  versionTimer: null,
  // Virtual scrolling
  virtualScroll: null,
  // Slash command menu
  slashMenuOpen: false,
  slashMenuIndex: 0,
  slashQuery: '',
  // Comment threading replies (in-memory)
  commentReplies: [],
  // Resolved comments (in-memory Set of comment IDs)
  resolvedComments: new Set(),
  // Pending formats for collapsed-cursor formatting (E-01 fix)
  pendingFormats: {},
  // Collaboration
  collabDoc: null,
  collabStatus: 'disconnected', // 'disconnected' | 'connecting' | 'connected' | 'offline'
  collabPeers: new Map(), // peerId -> { userName, userColor }
  // Header/footer HTML extracted from WASM to_html()
  docHeaderHtml: '',
  docFooterHtml: '',
  docFirstPageHeaderHtml: '',
  docFirstPageFooterHtml: '',
  hasDifferentFirstPage: false,
  // Page dimensions from WASM sections (in points)
  pageDims: null,
  // Multi-page rendering state
  pageMap: null,           // parsed get_page_map_json() result
  pageElements: [],        // array of .doc-page DOM elements
  nodeToPage: new Map(),   // nodeId string → page number
  activePageNum: 1,        // currently focused page
  // E1.5: Callback for refreshing find highlights after text changes
  _findRefreshTimer: null,
  _onTextChanged: null,
  // E3.1: Typing batch undo — tracks continuous typing in same paragraph
  _typingBatch: null, // { nodeId, count, timer }
  // E3.2: Undo history log (JS-side, for display only)
  undoHistory: [], // [{ label, timestamp }]  — most recent first
  undoHistoryPos: 0, // current position in undo history
  // E8.2: O(1) DOM lookup map — nodeId string → DOM element
  // Populated on full render and updated on per-node render.
  // Cleared on full re-render so stale references don't linger.
  nodeIdToElement: new Map(),
  // E8.1: Virtual scroll RAF throttle handle
  _vsRAF: null,
  // E8.1: Last scroll position for rapid scroll detection
  _vsLastScrollTop: 0,
  // E8.3: Layout cache — stores paginated HTML to avoid redundant WASM calls
  _layoutCache: null,
  // E8.3: Layout dirty flag — set true on structural changes, cleared after re-layout
  _layoutDirty: true,
  // E8.3: Layout debounce timer
  _layoutDebounceTimer: null,
  // E8.3: Lazy page observers (IntersectionObserver for pages view)
  _lazyPageObserver: null,
  // E8.4: Image data release — maps nodeId to original src for off-screen images
  _offscreenImageSrcs: new Map(),
  // E8.4: Performance warning shown flag (avoid repeated warnings)
  _perfWarningShown: false,
  // E6.3: IME composition in progress — blocks WASM sync until compositionend
  _composing: false,
  // E5.4: Editing mode — 'editing' | 'suggesting' | 'viewing'
  editingMode: 'editing',
  // FS-11: Read-only / Viewer mode — blocks all editing when true
  readOnlyMode: false,
  // UXP-14: Format Painter
  formatPainterMode: null, // null | 'once' | 'sticky'
  copiedFormat: null,       // { bold, italic, underline, strikethrough, ... } or null
  // UXP-02: Header/Footer editing mode — null | 'header' | 'footer'
  hfEditingMode: null,
  // UXP-02: Which page (1-based) is being edited for header/footer
  hfEditingPage: null,
  // FS-24: Smart quotes toggle
  smartQuotesEnabled: true,
  // FS-36: Auto-capitalize at sentence start toggle
  autoCapitalizeEnabled: true,
  // E9.5: TOC style — 'default' | 'dotted' | 'dashed' | 'no-page-numbers'
  tocStyle: 'default',
  // PDF viewer state
  pdfViewer: null,
  pdfBytes: null,
  pdfCurrentPage: 1,
  pdfZoom: 1.0,
  pdfTool: 'select',
  pdfAnnotations: [],
  pdfModified: false,
  pdfTextEdits: [],
  pdfFormFields: [],
};

export const $ = (id) => document.getElementById(id);
