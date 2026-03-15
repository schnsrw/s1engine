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
  // Pending formats for collapsed-cursor formatting (E-01 fix)
  pendingFormats: {},
  // Collaboration
  collabDoc: null,
  collabStatus: 'disconnected', // 'disconnected' | 'connecting' | 'connected' | 'offline'
  collabPeers: new Map(), // peerId -> { userName, userColor }
  // Header/footer HTML extracted from WASM to_html()
  docHeaderHtml: '',
  docFooterHtml: '',
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
};

export const $ = (id) => document.getElementById(id);
