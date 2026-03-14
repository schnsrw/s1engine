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
};

export const $ = (id) => document.getElementById(id);
