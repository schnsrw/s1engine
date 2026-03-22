// File operations: new, open, export, drag-drop
import { state, $ } from './state.js';
import { renderDocument, syncAllText, applyPageDimensions } from './render.js';
import { insertImage } from './images.js';
import { renderRuler } from './ruler.js';
import { broadcastOp } from './collab.js';
import { showToast } from './toolbar-handlers.js';
import { trackEvent } from './analytics.js';
import { ensureDocumentFonts, getFontDb } from './fonts.js';
import { closeFindBar } from './find.js';
import { addFileTab, detectFileTypeFromName } from './tabs.js';

let detect_format_fn = null;

export function setDetectFormat(fn) { detect_format_fn = fn; }

// ─── Autosave ─────────────────────────────────────
const AUTOSAVE_INTERVAL = 30000; // 30 seconds
const DB_NAME = 's1Autosave';
const DB_VERSION = 2;

// ─── Document Hash ───────────────────────────────
// SHA-256 hash via Web Crypto API for data integrity verification on save/load.
// SHA-256 provides strong collision resistance, far superior to CRC32.
// Falls back to synchronous FNV-1a if crypto.subtle is unavailable.
async function computeHash(data) {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  try {
    const buf = await crypto.subtle.digest('SHA-256', bytes);
    return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('');
  } catch (_) {
    // Fallback: FNV-1a 32-bit (synchronous, for environments without crypto.subtle)
    return _fnv1aFallback(bytes);
  }
}

function _fnv1aFallback(bytes) {
  let hash = 0x811c9dc5; // FNV offset basis
  for (let i = 0; i < bytes.length; i++) {
    hash ^= bytes[i];
    hash = Math.imul(hash, 0x01000193); // FNV prime
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

// ─── BroadcastChannel Multi-Tab Coordination ─────
let _saveChannel = null;
try {
  _saveChannel = new BroadcastChannel('s1-autosave');
} catch (_) {
  // BroadcastChannel not available (e.g., older browsers, file:// protocol)
}

if (_saveChannel) {
  _saveChannel.onmessage = (e) => {
    if (!e.data) return;
    if (e.data.type === 'save-complete' && e.data.tabId !== state.tabId) {
      // Another tab saved — update our timestamp so we don't overwrite
      state.lastSaveTimestamp = Math.max(state.lastSaveTimestamp, e.data.timestamp);
    }
    if (e.data.type === 'tab-closing' && e.data.tabId !== state.tabId) {
      // Another tab is closing — we may want to take over saving
    }
  };
  // Notify other tabs when this tab closes
  window.addEventListener('beforeunload', () => {
    try {
      _saveChannel.postMessage({ type: 'tab-closing', tabId: state.tabId });
    } catch (_) {}
  });
}

export function openAutosaveDB() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = (e) => {
      const db = req.result;
      if (!db.objectStoreNames.contains('documents')) {
        db.createObjectStore('documents', { keyPath: 'id' });
      }
      if (!db.objectStoreNames.contains('versions')) {
        db.createObjectStore('versions', { keyPath: 'id', autoIncrement: true });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

function startAutosave() {
  clearInterval(state.autosaveTimer);
  state.autosaveTimer = setInterval(() => {
    if (!state.doc || !state.dirty) return;
    doAutosave();
  }, AUTOSAVE_INTERVAL);
}

async function doAutosave() {
  if (!state.doc) return;
  try {
    syncAllText();
    const bytes = state.doc.export('docx');
    const name = $('docName').value || 'Untitled Document';
    const checksum = await computeHash(bytes);
    openAutosaveDB().then(db => {
      // Atomic write: write to temp key first, then swap to 'current'
      const tx = db.transaction('documents', 'readwrite');
      const store = tx.objectStore('documents');
      const getReq = store.get('current');
      getReq.onsuccess = () => {
        const existing = getReq.result;
        // If another tab saved more recently than our last save, skip
        if (existing && existing.timestamp > state.lastSaveTimestamp && existing.tabId && existing.tabId !== state.tabId) {
          const info = $('statusInfo');
          info._userMsg = true;
          info.textContent = 'Skipped save (another tab is active)';
          setTimeout(() => { info._userMsg = false; updateStatusBar(); }, 1500);
          return;
        }
        const now = Date.now();
        const commentReplies = state.commentReplies && state.commentReplies.length > 0
          ? JSON.stringify(state.commentReplies) : null;
        // Write to temp key first (atomic write pattern)
        const tempId = '_temp_' + now;
        store.put({
          id: tempId, name, bytes, timestamp: now, tabId: state.tabId, commentReplies, checksum, byteLength: bytes.length
        });
        // Now write to 'current' (same transaction = atomic)
        store.put({
          id: 'current', name, bytes, timestamp: now, tabId: state.tabId, commentReplies, checksum, byteLength: bytes.length
        });
        // Clean up temp key
        store.delete(tempId);
        state.lastSaveTimestamp = now;
        state.dirty = false;
        updateDirtyIndicator();
        // Broadcast to other tabs
        if (_saveChannel) {
          try { _saveChannel.postMessage({ type: 'save-complete', tabId: state.tabId, timestamp: now }); } catch (_) {}
        }
        const info = $('statusInfo');
        info._userMsg = true;
        info.textContent = 'All changes saved';
        info.style.color = '#1e8e3e';
        setTimeout(() => { info._userMsg = false; info.style.color = ''; updateStatusBar(); }, 3000);
      };
    }).catch(e => {
      // ED2-25: Log autosave failures instead of silently swallowing
      console.warn('Autosave failed:', e);
      const info = $('statusInfo');
      if (info) {
        info._userMsg = true;
        info.textContent = 'Autosave failed';
        setTimeout(() => { info._userMsg = false; updateStatusBar(); }, 3000);
      }
    });
  } catch (e) {
    if (e.name === 'QuotaExceededError' || (e.message && e.message.includes('quota'))) {
      showToast('Storage full — autosave disabled. Please export your work.', 'error', 10000);
      clearInterval(state.autosaveTimer);
      state.autosaveTimer = null;
    } else {
      console.warn('[autosave] Error:', e);
    }
  }
}

/**
 * Verify a saved document entry's integrity using its SHA-256 hash.
 * Returns true if valid or if no checksum exists (backwards compatibility).
 * Supports both legacy CRC32 (number) and new SHA-256 (string) checksums.
 */
export async function verifyChecksum(entry) {
  if (!entry || !entry.bytes) return false;
  if (entry.checksum === undefined || entry.checksum === null) return true; // no checksum = legacy entry
  const computed = await computeHash(entry.bytes);
  return computed === entry.checksum;
}

// ─── Version History ──────────────────────────────
const VERSION_INTERVAL = 300000; // 5 minutes
const MAX_VERSIONS = 20;

function countWords(doc) {
  try {
    const text = doc.to_plain_text();
    const words = text.trim().split(/\s+/).filter(w => w.length > 0);
    return words.length;
  } catch (_) { return 0; }
}

export function saveVersion(label) {
  if (!state.doc) return Promise.resolve();
  try {
    syncAllText();
    const bytes = state.doc.export('docx');
    const name = $('docName').value || 'Untitled Document';
    const wordCount = countWords(state.doc);
    const entry = {
      name,
      bytes,
      wordCount,
      timestamp: Date.now(),
      label: label || null,
    };
    return openAutosaveDB().then(db => {
      const tx = db.transaction('versions', 'readwrite');
      tx.objectStore('versions').add(entry);
      return new Promise((resolve) => {
        tx.oncomplete = () => { pruneVersions(db).then(resolve); };
        tx.onerror = () => resolve();
      });
    }).catch(() => {});
  } catch (_) { return Promise.resolve(); }
}

function pruneVersions(db) {
  return new Promise(resolve => {
    const tx = db.transaction('versions', 'readwrite');
    const store = tx.objectStore('versions');
    const req = store.getAll();
    req.onsuccess = () => {
      const all = req.result || [];
      if (all.length > MAX_VERSIONS) {
        // Sort by timestamp ascending, delete oldest
        all.sort((a, b) => a.timestamp - b.timestamp);
        const toDelete = all.slice(0, all.length - MAX_VERSIONS);
        toDelete.forEach(v => store.delete(v.id));
      }
      resolve();
    };
    req.onerror = () => resolve();
  });
}

export function getVersions() {
  return openAutosaveDB().then(db => {
    return new Promise((resolve) => {
      const tx = db.transaction('versions', 'readonly');
      const req = tx.objectStore('versions').getAll();
      req.onsuccess = () => {
        const all = req.result || [];
        all.sort((a, b) => b.timestamp - a.timestamp);
        resolve(all);
      };
      req.onerror = () => resolve([]);
    });
  }).catch(() => []);
}

export function restoreVersion(id) {
  return openAutosaveDB().then(db => {
    return new Promise((resolve, reject) => {
      const tx = db.transaction('versions', 'readonly');
      const req = tx.objectStore('versions').get(id);
      req.onsuccess = () => {
        const v = req.result;
        if (!v || !v.bytes) { reject(new Error('Version not found')); return; }
        try {
          state.doc = state.engine.open(new Uint8Array(v.bytes));
          state.currentFormat = 'DOCX';
          renderDocument();
          if (v.name) $('docName').value = v.name;
          state.dirty = true;
          const info = $('statusInfo');
          info.textContent = 'Restored version';
          setTimeout(() => { info.textContent = 'Ready'; }, 2000);
          resolve();
        } catch (e) { reject(e); }
      };
      req.onerror = () => reject(new Error('DB read error'));
    });
  });
}

function startVersionTimer() {
  clearInterval(state.versionTimer);
  state.versionTimer = setInterval(() => {
    if (!state.doc) return;
    saveVersion();
  }, VERSION_INTERVAL);
}

export function markDirty() {
  state.dirty = true;
  updateDirtyIndicator();
}

export function updateDirtyIndicator() {
  const nameEl = $('docName');
  if (!nameEl) return;
  const name = nameEl.value || '';
  // Show bullet before name when dirty
  if (state.dirty && !name.startsWith('\u2022 ')) {
    nameEl.classList.add('doc-dirty');
  } else {
    nameEl.classList.remove('doc-dirty');
  }
}

// Update word count in status bar
let _statusRAF = 0;
export function updateStatusBar() {
  cancelAnimationFrame(_statusRAF);
  _statusRAF = requestAnimationFrame(_updateStatusBarImpl);
}
function _updateStatusBarImpl() {
  const { doc } = state;
  if (!doc) return;
  try {
    const text = doc.to_plain_text();
    const words = text.trim() ? text.trim().split(/\s+/).filter(w => w.length > 0) : [];
    const wordCount = words.length;
    const charCount = [...text].length;
    const container = $('pageContainer');
    const paraCount = container ? container.querySelectorAll('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]').length : 0;
    // Count pages from pageElements
    const pageCount = state.pageElements?.length || 1;
    const info = $('statusInfo');
    if (info && !info._userMsg) {
      info.textContent = `${wordCount.toLocaleString()} words \u00B7 ${charCount.toLocaleString()} characters \u00B7 ${paraCount} paragraphs \u00B7 ${pageCount} page${pageCount !== 1 ? 's' : ''}`;
    }
    if (state.currentFormat) {
      $('statusFormat').textContent = state.currentFormat;
    }
  } catch (_) {}
}

export function checkAutoRecover() {
  return openAutosaveDB().then(db => {
    return new Promise(resolve => {
      const tx = db.transaction('documents', 'readonly');
      const req = tx.objectStore('documents').get('current');
      req.onsuccess = async () => {
        const entry = req.result || null;
        if (entry) {
          // Verify checksum integrity (async SHA-256)
          entry._checksumValid = await verifyChecksum(entry);
        }
        resolve(entry);
      };
      req.onerror = () => resolve(null);
    });
  }).catch(() => null);
}

/**
 * Clear all editor state before opening a new document.
 * Prevents stale data from previous document leaking.
 */
function resetEditorState() {
  // Clear DOM lookup caches
  state.nodeIdToElement.clear();
  state.syncedTextCache.clear();
  state.nodeToPage.clear();
  state.pageElements = [];
  state.pageMap = null;
  state._lastPageMapHash = null;
  // Clear find state
  state.findMatches = [];
  state.findIndex = -1;
  // Clear header/footer
  state.docHeaderHtml = '';
  state.docFooterHtml = '';
  state.docFirstPageHeaderHtml = '';
  state.docFirstPageFooterHtml = '';
  state.hasDifferentFirstPage = false;
  // Clear page dimensions (ruler will recalculate)
  state.pageDims = null;
  // Clear selection/undo state
  state.lastSelInfo = null;
  state.pendingFormats = {};
  // Clear typing batch timer before nullifying
  if (state._typingBatch && state._typingBatch.timer) {
    clearTimeout(state._typingBatch.timer);
  }
  state._typingBatch = null;
  state.undoHistory = [];
  state.undoHistoryPos = 0;
  // Clear comments state
  state.commentReplies = [];
  if (state.resolvedComments) state.resolvedComments.clear();
  // Clear collaboration
  state.internalClipboard = null;
  state.selectedImg = null;
  state.resizing = null;
  // Clear timers
  clearTimeout(state.syncTimer);
  state.syncTimer = null;
  clearTimeout(state._findRefreshTimer);
  state._findRefreshTimer = null;
  clearInterval(state.autosaveTimer);
  state.autosaveTimer = null;
  clearInterval(state.versionTimer);
  state.versionTimer = null;
  // E8: Clear performance optimization state
  state._layoutCache = null;
  state._layoutDirty = true;
  clearTimeout(state._layoutDebounceTimer);
  state._layoutDebounceTimer = null;
  state._vsRAF = null;
  state._vsLastScrollTop = 0;
  state._offscreenImageSrcs.clear();
  state._perfWarningShown = false;
  if (state._lazyPageObserver) {
    state._lazyPageObserver.disconnect();
    state._lazyPageObserver = null;
  }
  // Clean up spreadsheet viewer
  if (state.spreadsheetView) {
    state.spreadsheetView.destroy();
    state.spreadsheetView = null;
  }
  // Clean up PDF viewer and annotations
  if (state.pdfViewer) {
    state.pdfViewer.destroy();
    state.pdfViewer = null;
  }
  state.pdfBytes = null;
  state.pdfCurrentPage = 1;
  state.pdfZoom = 1.0;
  state.pdfTool = 'select';
  state.pdfAnnotations = [];
  state.pdfTextEdits = [];
  state.pdfModified = false;
  state.pdfFormFields = [];
}

export function newDocument() {
  if (!state.engine) return;
  // ED2-29: Close find bar when creating a new document
  closeFindBar();
  resetEditorState();
  state.doc = state.engine.create();
  state.currentFormat = 'new';
  state.doc.append_paragraph('');
  state.doc.clear_history();
  activateEditor();
  renderDocument();
  renderRuler(); // Update ruler with document page dimensions
  $('docName').value = 'Untitled Document';
  ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  state.dirty = false;
  updateDirtyIndicator();
  updateStatusBar();
  startAutosave();
  startVersionTimer();

  // Phase 6: Add tab for the new document
  addFileTab('Untitled Document', 'document', null);
}

/**
 * Detect if a byte array is a PDF by checking for the %PDF magic header.
 */
function isPdf(bytes) {
  return bytes.length >= 5 &&
    bytes[0] === 0x25 && bytes[1] === 0x50 && bytes[2] === 0x44 && bytes[3] === 0x46;
}

function showLoadingOverlay() {
  const overlay = document.createElement('div');
  overlay.className = 'loading-overlay';
  overlay.innerHTML = '<div class="loading-spinner"></div>';
  document.body.appendChild(overlay);
  return overlay;
}

function removeLoadingOverlay(overlay) {
  if (overlay && overlay.parentNode) overlay.parentNode.removeChild(overlay);
}

export async function openFile(bytes, name) {
  if (!state.engine) return;
  // ED2-29: Close find bar when opening a new file
  closeFindBar();
  const _loadingOverlay = showLoadingOverlay();
  try { await _openFileImpl(bytes, name); } finally { removeLoadingOverlay(_loadingOverlay); }
}

async function _openFileImpl(bytes, name) {

  // Reject files larger than 100MB
  if (bytes && bytes.length > 100 * 1024 * 1024) {
    showToast('File too large (max 100MB)', 'error');
    return;
  }

  // Reject empty / zero-byte files
  if (!bytes || bytes.length === 0) {
    showToast('File is empty or corrupted', 'error');
    return;
  }

  const ext = name?.split('.').pop()?.toLowerCase();

  // PDF detection — open in PDF viewer
  if (ext === 'pdf' || isPdf(bytes)) {
    try {
      resetEditorState();
      // Clear PDF-specific state for new PDF
      state.pdfAnnotations = [];
      state.pdfModified = false;
      state.pdfTextEdits = [];
      state.pdfFormFields = [];
      // Lazy-load PDF viewer module
      const { PdfViewer } = await import('./pdf-viewer.js');
      state.currentFormat = 'PDF';
      // Activate UI — hide welcome, show status bar, switch to PDF view
      $('welcomeScreen').style.display = 'none';
      $('statusbar').classList.add('show');
      switchView('pdf'); // This hides doc toolbar+menubar and shows PDF view
      // Destroy previous viewer if any
      if (state.pdfViewer) { state.pdfViewer.destroy(); }
      state.pdfViewer = new PdfViewer($('pdfCanvasContainer'));
      // Set initial tool cursor
      const container = $('pdfCanvasContainer');
      if (container) container.dataset.tool = 'select';
      // Make independent copies — PDF.js may transfer/detach the buffer it receives
      const pdfData = bytes.slice(0); // copy for PDF.js
      state.pdfBytes = bytes.slice(0); // clean copy for download
      await state.pdfViewer.open(pdfData);
      state.pdfCurrentPage = 1;
      state.pdfModified = false;
      if (name) $('docName').value = name.replace(/\.[^.]+$/, '');
      updatePdfStatusBar();
      $('statusFormat').textContent = 'PDF';
      // Initialize annotation tools, text editing, and thumbnails
      try {
        const [annot, textEdit, pages] = await Promise.all([
          import('./pdf-annotations.js'),
          import('./pdf-text-edit.js'),
          import('./pdf-pages.js'),
        ]);
        annot.initAnnotationTools();
        textEdit.initPdfTextEdit();
        pages.renderThumbnails();
        // Wire up thumbnail sync on scroll
        state.pdfViewer.onPageChange((pageNum) => {
          pages.updateActiveThumbnail(pageNum);
        });
      } catch (err) { console.warn('PDF tools init:', err); }
      // Phase 6: Add tab for the opened PDF
      const displayName = name ? name.replace(/\.[^.]+$/, '') : 'PDF Document';
      addFileTab(displayName, 'pdf', bytes);
    } catch (e) {
      console.error('PDF open error:', e);
      // If PDF.js fails, show a clear error and reset UI
      const msg = e?.message || String(e);
      if (msg.includes('InvalidPDF') || msg.includes('Invalid PDF')) {
        showToast('This file is not a valid PDF or is corrupted', 'error');
      } else {
        showToast('Failed to open PDF: ' + msg, 'error');
      }
      // Reset to welcome screen
      switchView('editor');
      $('welcomeScreen').style.display = '';
    }
    return;
  }

  // CSV / XLSX detection — open in spreadsheet view
  const isCSV = ext === 'csv';
  const isXLSX = ext === 'xlsx' || (bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4B && name?.toLowerCase().endsWith('.xlsx'));
  if (isCSV || isXLSX) {
    try {
      resetEditorState();
      const { SpreadsheetView } = await import('./spreadsheet.js');
      state.currentFormat = isCSV ? 'CSV' : 'XLSX';
      // Activate UI
      $('welcomeScreen').style.display = 'none';
      $('statusbar').classList.add('show');
      switchView('spreadsheet');
      // Create spreadsheet view
      if (state.spreadsheetView) state.spreadsheetView.destroy();
      const container = $('spreadsheetContainer');
      state.spreadsheetView = new SpreadsheetView(container);
      state.spreadsheetView.loadWorkbook(bytes, name);
      if (name) $('docName').value = name.replace(/\.[^.]+$/, '');
      const info = $('statusInfo');
      if (info) {
        const sheet = state.spreadsheetView._sheet();
        const cellCount = sheet ? Object.keys(sheet.cells).length : 0;
        info.textContent = `${cellCount.toLocaleString()} cells`;
      }
      $('statusFormat').textContent = state.currentFormat;
      // Phase 6: Add tab for the opened spreadsheet
      const displayName = name ? name.replace(/\.[^.]+$/, '') : 'Untitled Spreadsheet';
      addFileTab(displayName, 'spreadsheet', bytes);
    } catch (e) {
      console.error('Spreadsheet open error:', e);
      showToast('Failed to open spreadsheet: ' + e.message, 'error');
      switchView('editor');
      $('welcomeScreen').style.display = '';
    }
    return;
  }

  try {
    resetEditorState();
    let fmt = 'txt';
    try { if (detect_format_fn) fmt = detect_format_fn(bytes); } catch (_) {}
    state.doc = state.engine.open(bytes);
    state.currentFormat = fmt.toUpperCase();
    activateEditor();

    // Load fonts used in the document before rendering so glyphs are correct
    try {
      const loaded = await ensureDocumentFonts(state.doc);
      if (loaded > 0) state._layoutDirty = true;
    } catch (_) {
      // Font loading failed — render with fallback fonts
    }

    renderDocument();
    renderRuler(); // Update ruler with actual document page dimensions
    if (name) $('docName').value = name.replace(/\.[^.]+$/, '');
    updateTrackChanges();

    // P4: Show macro warning if document has VBA macros or digital signatures
    try {
      const meta = state.doc.metadata_json ? JSON.parse(state.doc.metadata_json()) : {};
      if (meta.custom_properties?.hasMacros === 'true') {
        showToast('This document contains macros (VBA). Macro execution is not supported.', 'warning', 8000);
      }
      if (meta.custom_properties?.hasDigitalSignature === 'true') {
        const subject = meta.custom_properties?.signatureSubject || 'Unknown signer';
        showToast(`Signed document: ${subject}`, 'info', 6000);
      }
    } catch (_) {}

    state.dirty = false;
    updateDirtyIndicator();
    startAutosave();
    startVersionTimer();

    // Phase 6: Add tab for the opened document
    const displayName = name ? name.replace(/\.[^.]+$/, '') : 'Document';
    const fileType = detectFileTypeFromName(name || 'document.txt');
    addFileTab(displayName, fileType, bytes);
  } catch (e) {
    showToast('Failed to open: ' + e.message, 'error');
    console.error(e);
  }
}

export function updatePdfStatusBar() {
  if (!state.pdfViewer) return;
  const pageCount = state.pdfViewer.getPageCount();
  const info = $('statusInfo');
  if (info) info.textContent = `Page ${state.pdfCurrentPage} of ${pageCount}`;
  const pageInfo = $('pdfPageInfo');
  if (pageInfo) pageInfo.textContent = `${state.pdfCurrentPage} / ${pageCount}`;
}

export function exportDoc(format) {
  const { doc } = state;
  if (!doc) return;
  const overlay = showLoadingOverlay();
  try {
    syncAllText();
    trackEvent('export', format);
    if (format === 'pdf') {
      try {
        const fontDb = getFontDb();
        const url = (fontDb && fontDb.font_count() > 0)
          ? doc.to_pdf_data_url_with_fonts(fontDb)
          : doc.to_pdf_data_url();
        const a = document.createElement('a');
        a.href = url; a.download = ($('docName').value || 'document') + '.pdf'; a.click();
      } catch (e) {
        showToast('PDF export failed: ' + e.message, 'error');
        console.error('PDF export error:', e);
      }
      return;
    }
    const bytes = doc.export(format);
    const blob = new Blob([bytes]);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = ($('docName').value || 'document') + '.' + format; a.click();
    setTimeout(() => { URL.revokeObjectURL(url); }, 200);
  } catch (e) { showToast('Export failed: ' + e.message, 'error'); } finally { removeLoadingOverlay(overlay); }
}

function activateEditor() {
  $('welcomeScreen').style.display = 'none';
  $('toolbar').classList.add('show');
  const menubar = $('appMenubar');
  if (menubar) menubar.classList.add('show');
  $('statusbar').classList.add('show');
  switchView('editor');
}

// Saved cursor position for restoring when switching back to editor view
let _savedCursorForView = null;

export function switchView(view) {
  // Save cursor position when leaving editor view
  if (state.currentView === 'editor' && view !== 'editor') {
    const sel = window.getSelection();
    if (sel && sel.rangeCount) {
      let n = sel.anchorNode;
      while (n && !n.dataset?.nodeId) n = n.parentNode;
      if (n?.dataset?.nodeId) {
        _savedCursorForView = { nodeId: n.dataset.nodeId, offset: state.lastSelInfo?.startOffset || 0 };
      }
    }
  }

  // ED2-20: Destroy PDF viewer (and its scroll handler) when switching away from PDF view
  if (state.currentView === 'pdf' && view !== 'pdf') {
    if (state.pdfViewer) {
      state.pdfViewer.destroy();
      state.pdfViewer = null;
    }
    // I3: Clear PDF annotations/edits/form state when leaving PDF view
    state.pdfAnnotations = [];
    state.pdfTextEdits = [];
    state.pdfFormFields = [];
  }

  // Destroy spreadsheet view when switching away
  if (state.currentView === 'spreadsheet' && view !== 'spreadsheet') {
    if (state.spreadsheetView) {
      state.spreadsheetView.destroy();
      state.spreadsheetView = null;
    }
    // I5: Reset format state and clear active toolbar buttons when leaving spreadsheet view
    // This prevents bold/italic/underline active states from bleeding into the doc toolbar.
    state.currentFormat = '';
    document.querySelectorAll('.ss-toolbar .tb-btn.active').forEach(b => b.classList.remove('active'));
  }

  state.currentView = view;

  // I1: Update AI context indicator when view changes
  if (state.aiPanelOpen) {
    const chip = document.getElementById('aiContextLabel');
    if (chip) {
      if (view === 'spreadsheet') chip.textContent = 'Spreadsheet';
      else if (view === 'pdf') chip.textContent = 'PDF Viewer';
      else chip.textContent = state.currentFormat || 'Document';
    }
  }
  // Show the editor wrapper (which contains pages panel + canvas + comments panel)
  const wrapper = $('editorWrapper');
  if (wrapper) wrapper.classList.toggle('show', view === 'editor');
  $('pdfView').classList.toggle('show', view === 'pdf');
  // Show spreadsheet view
  const ssView = $('spreadsheetView');
  if (ssView) ssView.classList.toggle('show', view === 'spreadsheet');
  // Hide doc editor chrome when in PDF or spreadsheet mode
  $('toolbar').classList.toggle('show', view === 'editor');
  const menubar = $('appMenubar');
  if (menubar) menubar.classList.toggle('show', view === 'editor');
  // Show spreadsheet-specific menu bar and toolbar only in spreadsheet view
  const ssMenubar = $('ssMenubar');
  if (ssMenubar) ssMenubar.style.display = (view === 'spreadsheet') ? 'flex' : 'none';
  const ssToolbar = $('ssToolbar');
  if (ssToolbar) ssToolbar.style.display = (view === 'spreadsheet') ? 'flex' : 'none';
  // Hide ruler in non-editor modes
  const ruler = $('ruler');
  if (ruler) ruler.style.display = (view !== 'editor') ? 'none' : '';
  // Hide doc-only title bar buttons (pages panel, properties, comments, history) in non-editor modes
  document.querySelectorAll('.doc-only-btn').forEach(b => {
    b.style.display = (view === 'editor') ? '' : 'none';
  });
  // Swap logo based on active view
  const logoImg = document.querySelector('.logo img');
  if (logoImg) {
    const logoMap = { editor: '/assets/logo-doc.svg', spreadsheet: '/assets/logo-sheet.svg', pdf: '/assets/logo-doc.svg' };
    logoImg.src = logoMap[view] || '/assets/logo.svg';
  }
  // Update legacy tab bar (hidden) and new status bar view buttons
  document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.view === view));
  document.querySelectorAll('.status-view-btn').forEach(b => b.classList.toggle('active', b.dataset.view === view));
  // Update view menu entries
  document.querySelectorAll('.tab-menu-entry').forEach(e => e.classList.toggle('active', e.dataset.view === view));

  // Restore cursor position when returning to editor view
  if (view === 'editor' && _savedCursorForView) {
    const { nodeId, offset } = _savedCursorForView;
    _savedCursorForView = null;
    requestAnimationFrame(() => {
      const container = $('pageContainer');
      if (!container) return;
      const el = container.querySelector(`[data-node-id="${nodeId}"]`);
      if (el) {
        const pageContent = el.closest('.page-content');
        if (pageContent) pageContent.focus();
        // Restore cursor using char offset
        const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
        let counted = 0, tw;
        while ((tw = walker.nextNode())) {
          const chars = Array.from(tw.textContent);
          if (counted + chars.length >= offset) {
            let strOff = 0;
            for (let i = 0; i < offset - counted && i < chars.length; i++) strOff += chars[i].length;
            try {
              const range = document.createRange();
              range.setStart(tw, strOff);
              range.collapse(true);
              const s = window.getSelection();
              s.removeAllRanges(); s.addRange(range);
            } catch (_) {}
            break;
          }
          counted += chars.length;
        }
      }
    });
  }
}

export function updateTrackChanges() {
  const { doc } = state;
  if (!doc) return;
  try {
    // Count tracked change elements in the rendered DOM
    const container = $('pageContainer');
    const tcElements = container ? container.querySelectorAll('[data-tc-node-id]') : [];
    const count = tcElements.length;
    if (count > 0) {
      $('tcCount').textContent = count + ' tracked change' + (count !== 1 ? 's' : '');
      $('tcBar').classList.add('show');
    } else {
      $('tcBar').classList.remove('show');
    }
  } catch (_) { $('tcBar').classList.remove('show'); }
}

export function initFileHandlers() {
  // Warn before closing with unsaved changes
  window.addEventListener('beforeunload', e => {
    // Clear timers on unload to prevent leaks in SPA contexts
    clearInterval(state.autosaveTimer);
    clearInterval(state.versionTimer);
    if (state.dirty && state.doc) {
      e.preventDefault();
      e.returnValue = '';
    }
  });

  $('btnNew').addEventListener('click', newDocument);
  $('welcomeNew').addEventListener('click', newDocument);
  $('btnOpen').addEventListener('click', () => $('fileInput').click());
  $('welcomeOpen').addEventListener('click', () => $('fileInput').click());

  // Suite launcher buttons (Phase 6)
  const suiteSpreadsheet = $('suiteSpreadsheet');
  if (suiteSpreadsheet) {
    suiteSpreadsheet.addEventListener('click', async () => {
      // Open a blank spreadsheet
      try {
        resetEditorState();
        const { SpreadsheetView } = await import('./spreadsheet.js');
        state.currentFormat = 'CSV';
        $('welcomeScreen').style.display = 'none';
        $('statusbar').classList.add('show');
        switchView('spreadsheet');
        if (state.spreadsheetView) state.spreadsheetView.destroy();
        const container = $('spreadsheetContainer');
        state.spreadsheetView = new SpreadsheetView(container);
        // Create an empty workbook
        state.spreadsheetView.loadWorkbook('', 'Sheet1.csv');
        $('docName').value = 'Untitled Spreadsheet';
        const info = $('statusInfo');
        if (info) info.textContent = '0 cells';
        $('statusFormat').textContent = 'CSV';
        // Phase 6: Add tab for blank spreadsheet
        addFileTab('Untitled Spreadsheet', 'spreadsheet', null);
      } catch (e) {
        console.error('Spreadsheet open error:', e);
        showToast('Failed to open spreadsheet: ' + e.message, 'error');
      }
    });
  }
  const suitePresentation = $('suitePresentation');
  if (suitePresentation) {
    suitePresentation.addEventListener('click', () => {
      showToast('Presentation editor is planned for a future release.', 'info', 5000);
    });
  }
  const suiteCSV = $('suiteCSV');
  if (suiteCSV) {
    suiteCSV.addEventListener('click', () => $('csvInput').click());
  }
  const csvInput = $('csvInput');
  if (csvInput) {
    csvInput.accept = '.csv,.xlsx';
    csvInput.addEventListener('change', e => {
      const f = e.target.files[0]; if (!f) return;
      const r = new FileReader();
      r.onload = () => openFile(new Uint8Array(r.result), f.name);
      r.readAsArrayBuffer(f); e.target.value = '';
    });
  }

  $('fileInput').addEventListener('change', e => {
    const f = e.target.files[0]; if (!f) return;
    const r = new FileReader();
    r.onload = () => openFile(new Uint8Array(r.result), f.name);
    r.readAsArrayBuffer(f); e.target.value = '';
  });

  // Drag & drop
  [$('dropZone'), $('editorCanvas'), $('pdfView')].forEach(t => {
    if (!t) return;
    t.addEventListener('dragover', e => { e.preventDefault(); t.classList.add('drag-over'); });
    t.addEventListener('dragleave', () => t.classList.remove('drag-over'));
    t.addEventListener('drop', e => {
      e.preventDefault(); t.classList.remove('drag-over');
      const f = e.dataTransfer.files[0]; if (!f) return;
      if (f.type.startsWith('image/') && state.doc) {
        insertImage(f);
      } else {
        const r = new FileReader();
        r.onload = () => openFile(new Uint8Array(r.result), f.name);
        r.readAsArrayBuffer(f);
      }
    });
  });

  // Export — File menu entries with data-fmt attribute
  document.querySelectorAll('.app-menu-entry[data-fmt]').forEach(btn => {
    btn.addEventListener('click', () => {
      exportDoc(btn.dataset.fmt);
      // Close the File menu after export
      document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
    });
  });
  // Legacy export menu (hidden) — still wire for backwards compat
  const exportMenu = $('exportMenu');
  if (exportMenu) {
    exportMenu.querySelectorAll('.dropdown-item').forEach(btn => {
      btn.addEventListener('click', e => { e.stopPropagation(); exportDoc(btn.dataset.fmt); exportMenu.classList.remove('show'); });
    });
  }

  // Tabs (legacy hidden tab bar)
  document.querySelectorAll('.tab').forEach(tab => {
    tab.addEventListener('click', () => switchView(tab.dataset.view));
  });

  // Status bar view buttons
  document.querySelectorAll('.status-view-btn').forEach(btn => {
    btn.addEventListener('click', () => switchView(btn.dataset.view));
  });

  // View menu entries
  document.querySelectorAll('.tab-menu-entry').forEach(entry => {
    entry.addEventListener('click', () => {
      switchView(entry.dataset.view);
      document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
    });
  });

  // Track changes
  $('btnAcceptAll').addEventListener('click', () => {
    if (!state.doc) return;
    try {
      state.doc.accept_all_changes();
      broadcastOp({ action: 'acceptAllChanges' });
      renderDocument(); updateTrackChanges();
    } catch (e) { console.error('accept:', e); }
  });
  $('btnRejectAll').addEventListener('click', () => {
    if (!state.doc) return;
    try {
      state.doc.reject_all_changes();
      broadcastOp({ action: 'rejectAllChanges' });
      renderDocument(); updateTrackChanges();
    } catch (e) { console.error('reject:', e); }
  });
}
