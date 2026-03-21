// File Tab Bar — tracks open files and allows switching between them.
// Phase 6: Multi-file editing for Rudra Office multi-app launcher.

import { state, $ } from './state.js';

// NOTE: We use dynamic import() for './file.js' to avoid circular dependency,
// since file.js also imports from tabs.js.

// ── Icon map for file types ───────────────────────
const TYPE_ICONS = {
  document: 'description',
  spreadsheet: 'table_chart',
  pdf: 'picture_as_pdf',
  presentation: 'slideshow',
  unknown: 'insert_drive_file',
};

/**
 * Determine the file type category from a filename extension.
 * Returns 'document', 'spreadsheet', 'pdf', 'presentation', or 'unknown'.
 */
export function detectFileTypeFromName(filename) {
  const ext = (filename || '').split('.').pop()?.toLowerCase() || '';
  switch (ext) {
    case 'docx': case 'doc': case 'odt': case 'txt': case 'md':
      return 'document';
    case 'xlsx': case 'xls': case 'ods': case 'csv':
      return 'spreadsheet';
    case 'pdf':
      return 'pdf';
    case 'pptx': case 'ppt': case 'odp':
      return 'presentation';
    default:
      return 'unknown';
  }
}

/**
 * Map file type category to the editor view name used by switchView().
 */
function typeToView(type) {
  switch (type) {
    case 'document': return 'editor';
    case 'spreadsheet': return 'spreadsheet';
    case 'pdf': return 'pdf';
    default: return 'editor';
  }
}

/**
 * Generate a unique file tab ID.
 */
function nextFileId() {
  state._fileIdCounter += 1;
  return 'tab-' + state._fileIdCounter;
}

/**
 * Add a new open file to the tab bar and make it active.
 *
 * @param {string} name - Display name (filename without path)
 * @param {string} type - File type category: 'document' | 'spreadsheet' | 'pdf'
 * @param {Uint8Array|null} data - Raw file bytes (for restore on tab switch)
 * @returns {string} The new tab's ID
 */
export function addFileTab(name, type, data) {
  // Save state of the currently active tab before switching
  saveActiveTabState();

  const id = nextFileId();
  const entry = {
    id,
    name: name || 'Untitled',
    type: type || 'document',
    data: data ? data.slice(0) : null, // defensive copy
    scrollTop: 0,
    selectionInfo: null,
    format: state.currentFormat || '',
    // These will be populated when the file is actually opened:
    doc: null,
    spreadsheetViewState: null,
    pdfBytes: null,
    pdfAnnotations: null,
    pdfTextEdits: null,
    pdfCurrentPage: 1,
  };

  state.openFiles.push(entry);
  state.activeFileId = id;

  renderTabBar();
  return id;
}

/**
 * Save the current editor state into the active tab entry.
 * Called before switching away from a tab.
 */
function saveActiveTabState() {
  if (!state.activeFileId) return;
  const tab = state.openFiles.find(f => f.id === state.activeFileId);
  if (!tab) return;

  tab.format = state.currentFormat || '';
  tab.name = $('docName')?.value || tab.name;

  // Save scroll position
  const editorWrapper = $('editorWrapper');
  const pdfView = $('pdfView');
  const ssView = $('spreadsheetView');

  if (state.currentView === 'editor' && editorWrapper) {
    const scrollable = editorWrapper.querySelector('.editor-canvas') || editorWrapper;
    tab.scrollTop = scrollable.scrollTop || 0;
    // Save cursor/selection info
    const sel = window.getSelection();
    if (sel && sel.rangeCount) {
      let n = sel.anchorNode;
      while (n && !n.dataset?.nodeId) n = n.parentNode;
      if (n?.dataset?.nodeId) {
        tab.selectionInfo = {
          nodeId: n.dataset.nodeId,
          offset: state.lastSelInfo?.startOffset || 0
        };
      }
    }
    // Save WASM doc reference
    tab.doc = state.doc;
  } else if (state.currentView === 'pdf') {
    tab.pdfBytes = state.pdfBytes;
    tab.pdfAnnotations = state.pdfAnnotations ? [...state.pdfAnnotations] : [];
    tab.pdfTextEdits = state.pdfTextEdits ? [...state.pdfTextEdits] : [];
    tab.pdfCurrentPage = state.pdfCurrentPage || 1;
    if (pdfView) tab.scrollTop = pdfView.scrollTop || 0;
  } else if (state.currentView === 'spreadsheet') {
    // Save spreadsheet internal state (the view instance handles its own state)
    tab.spreadsheetViewState = state.spreadsheetView;
    if (ssView) tab.scrollTop = ssView.scrollTop || 0;
  }
}

/**
 * Update the active tab's name (called when user renames the document).
 */
export function updateActiveTabName(name) {
  if (!state.activeFileId) return;
  const tab = state.openFiles.find(f => f.id === state.activeFileId);
  if (tab) {
    tab.name = name;
    renderTabBar();
  }
}

/**
 * Switch to a specific open file tab.
 */
let _switching = false;
export async function switchToTab(tabId) {
  if (_switching) return;
  if (tabId === state.activeFileId) return;
  const target = state.openFiles.find(f => f.id === tabId);
  if (!target) return;
  _switching = true;
  try {

  // Save current tab state
  saveActiveTabState();

  // Detach current spreadsheet view without destroying (we saved reference)
  if (state.currentView === 'spreadsheet' && state.spreadsheetView) {
    // Don't destroy — we saved the reference in the tab entry
    state.spreadsheetView = null;
  }
  // Detach PDF viewer
  if (state.currentView === 'pdf' && state.pdfViewer) {
    state.pdfViewer.destroy();
    state.pdfViewer = null;
  }

  state.activeFileId = tabId;

  // Dynamic import to avoid circular dependency
  const { switchView } = await import('./file.js');

  // Restore target tab state
  const viewType = typeToView(target.type);

  // Set the document name
  const docName = $('docName');
  if (docName) docName.value = target.name;

  state.currentFormat = target.format || '';

  if (viewType === 'editor') {
    state.doc = target.doc;
    switchView('editor');
    // Re-render document if we have a doc reference
    if (state.doc) {
      const { renderDocument } = await import('./render.js');
      renderDocument();
      // Restore scroll position
      requestAnimationFrame(() => {
        const editorWrapper = $('editorWrapper');
        const scrollable = editorWrapper?.querySelector('.editor-canvas') || editorWrapper;
        if (scrollable && target.scrollTop) scrollable.scrollTop = target.scrollTop;
      });
    }
  } else if (viewType === 'pdf') {
    state.pdfBytes = target.pdfBytes;
    state.pdfAnnotations = target.pdfAnnotations || [];
    state.pdfTextEdits = target.pdfTextEdits || [];
    state.pdfCurrentPage = target.pdfCurrentPage || 1;
    switchView('pdf');
    // Re-open PDF
    if (target.pdfBytes) {
      const { PdfViewer } = await import('./pdf-viewer.js');
      const container = $('pdfCanvasContainer');
      state.pdfViewer = new PdfViewer(container);
      if (container) container.dataset.tool = state.pdfTool || 'select';
      await state.pdfViewer.open(target.pdfBytes.slice(0));
      const { updatePdfStatusBar } = await import('./file.js');
      updatePdfStatusBar();
    }
  } else if (viewType === 'spreadsheet') {
    switchView('spreadsheet');
    // Restore spreadsheet view
    if (target.spreadsheetViewState) {
      state.spreadsheetView = target.spreadsheetViewState;
      // Re-attach to DOM container
      const container = $('spreadsheetContainer');
      if (container && state.spreadsheetView.reattach) {
        state.spreadsheetView.reattach(container);
      } else if (state.spreadsheetView.render) {
        state.spreadsheetView.render();
      }
    }
  }

  // Show editor chrome (toolbar, status bar) — they may have been hidden
  $('welcomeScreen').style.display = 'none';
  $('statusbar').classList.add('show');

  renderTabBar();

  } finally {
    _switching = false;
  }
}

/**
 * Close a file tab. If it's the active tab, switch to an adjacent one.
 */
export async function closeFileTab(tabId) {
  const idx = state.openFiles.findIndex(f => f.id === tabId);
  if (idx === -1) return;

  const tab = state.openFiles[idx];

  // Clean up resources for the tab being closed
  if (tab.spreadsheetViewState && tab.spreadsheetViewState.destroy) {
    tab.spreadsheetViewState.destroy();
  }
  // Release doc reference (WASM objects get GC'd)
  tab.doc = null;
  tab.pdfBytes = null;
  tab.data = null;

  state.openFiles.splice(idx, 1);

  if (state.openFiles.length === 0) {
    // No more tabs open — show welcome screen
    state.activeFileId = null;
    state.doc = null;
    if (state.pdfViewer) { state.pdfViewer.destroy(); state.pdfViewer = null; }
    if (state.spreadsheetView) { state.spreadsheetView.destroy(); state.spreadsheetView = null; }

    const { switchView } = await import('./file.js');
    switchView('editor');
    $('welcomeScreen').style.display = '';
    $('toolbar').classList.remove('show');
    const menubar = $('appMenubar');
    if (menubar) menubar.classList.remove('show');
    $('statusbar').classList.remove('show');
    $('docName').value = 'Untitled Document';
    renderTabBar();
    return;
  }

  // If the closed tab was active, switch to an adjacent one
  if (tabId === state.activeFileId) {
    const newIdx = Math.min(idx, state.openFiles.length - 1);
    state.activeFileId = null; // clear so switchToTab doesn't skip
    await switchToTab(state.openFiles[newIdx].id);
  }

  renderTabBar();
}

/**
 * Render the tab bar DOM based on state.openFiles.
 */
export function renderTabBar() {
  const bar = $('fileTabBar');
  if (!bar) return;

  // Only show tab bar when there are open files
  if (state.openFiles.length === 0) {
    bar.classList.remove('show');
    bar.innerHTML = '';
    return;
  }

  bar.classList.add('show');
  bar.innerHTML = '';

  for (const file of state.openFiles) {
    const tabEl = document.createElement('div');
    tabEl.className = 'file-tab' + (file.id === state.activeFileId ? ' active' : '');
    tabEl.dataset.tabId = file.id;
    tabEl.setAttribute('role', 'tab');
    tabEl.setAttribute('aria-selected', file.id === state.activeFileId ? 'true' : 'false');
    tabEl.setAttribute('title', file.name);
    tabEl.tabIndex = 0;

    // Icon
    const icon = document.createElement('span');
    icon.className = 'file-tab-icon msi';
    icon.textContent = TYPE_ICONS[file.type] || TYPE_ICONS.unknown;
    tabEl.appendChild(icon);

    // Name
    const nameSpan = document.createElement('span');
    nameSpan.className = 'file-tab-name';
    nameSpan.textContent = file.name;
    tabEl.appendChild(nameSpan);

    // Dirty indicator (shown when file has unsaved changes)
    const dirty = document.createElement('span');
    dirty.className = 'file-tab-dirty';
    tabEl.appendChild(dirty);

    // Close button
    const closeBtn = document.createElement('button');
    closeBtn.className = 'file-tab-close';
    closeBtn.setAttribute('title', 'Close tab');
    closeBtn.setAttribute('aria-label', `Close ${file.name}`);
    closeBtn.textContent = '\u00D7'; // multiplication sign (x)
    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      closeFileTab(file.id);
    });
    tabEl.appendChild(closeBtn);

    // Click to switch
    tabEl.addEventListener('click', () => switchToTab(file.id));

    // Middle-click to close
    tabEl.addEventListener('auxclick', (e) => {
      if (e.button === 1) {
        e.preventDefault();
        closeFileTab(file.id);
      }
    });

    // Keyboard: Enter/Space to switch, Delete to close
    tabEl.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        switchToTab(file.id);
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        e.preventDefault();
        closeFileTab(file.id);
      }
    });

    bar.appendChild(tabEl);
  }

  // Scroll active tab into view
  requestAnimationFrame(() => {
    const active = bar.querySelector('.file-tab.active');
    if (active) active.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
  });
}

/**
 * Initialize the tab bar module.
 * Wire up the docName input to sync with the active tab name.
 */
export function initTabs() {
  const docName = $('docName');
  if (docName) {
    docName.addEventListener('input', () => {
      updateActiveTabName(docName.value);
    });
  }
}
