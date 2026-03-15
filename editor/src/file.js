// File operations: new, open, export, drag-drop
import { state, $ } from './state.js';
import { renderDocument, syncAllText, renderPages, renderText, applyPageDimensions } from './render.js';
import { insertImage } from './images.js';
import { renderRuler } from './ruler.js';
import { broadcastOp } from './collab.js';

let detect_format_fn = null;

export function setDetectFormat(fn) { detect_format_fn = fn; }

// ─── Autosave ─────────────────────────────────────
const AUTOSAVE_INTERVAL = 30000; // 30 seconds
const DB_NAME = 'FolioAutosave';
const DB_VERSION = 2;

function openAutosaveDB() {
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

function doAutosave() {
  if (!state.doc) return;
  try {
    syncAllText();
    const bytes = state.doc.export('docx');
    const name = $('docName').value || 'Untitled Document';
    openAutosaveDB().then(db => {
      // Multi-tab locking: read current stored document first
      const readTx = db.transaction('documents', 'readonly');
      const getReq = readTx.objectStore('documents').get('current');
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
        // Safe to save — write with our tabId and timestamp
        const now = Date.now();
        const writeTx = db.transaction('documents', 'readwrite');
        writeTx.objectStore('documents').put({
          id: 'current', name, bytes, timestamp: now, tabId: state.tabId
        });
        state.lastSaveTimestamp = now;
        state.dirty = false;
        updateDirtyIndicator();
        const info = $('statusInfo');
        info._userMsg = true;
        info.textContent = 'Auto-saved';
        setTimeout(() => { info._userMsg = false; updateStatusBar(); }, 1500);
      };
      getReq.onerror = () => {
        // If read fails, still attempt to save
        const now = Date.now();
        const writeTx = db.transaction('documents', 'readwrite');
        writeTx.objectStore('documents').put({
          id: 'current', name, bytes, timestamp: now, tabId: state.tabId
        });
        state.lastSaveTimestamp = now;
        state.dirty = false;
        updateDirtyIndicator();
      };
    }).catch(() => {});
  } catch (_) {}
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
      req.onsuccess = () => resolve(req.result || null);
      req.onerror = () => resolve(null);
    });
  }).catch(() => null);
}

export function newDocument() {
  if (!state.engine) return;
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
}

export function openFile(bytes, name) {
  if (!state.engine) return;
  try {
    let fmt = 'txt';
    try { if (detect_format_fn) fmt = detect_format_fn(bytes); } catch (_) {}
    state.doc = state.engine.open(bytes);
    state.currentFormat = fmt.toUpperCase();
    activateEditor();
    renderDocument();
    renderRuler(); // Update ruler with actual document page dimensions
    if (name) $('docName').value = name.replace(/\.[^.]+$/, '');
    updateTrackChanges();
    state.dirty = false;
    updateDirtyIndicator();
    startAutosave();
    startVersionTimer();
  } catch (e) {
    alert('Failed to open: ' + e.message);
    console.error(e);
  }
}

export function exportDoc(format) {
  const { doc } = state;
  if (!doc) return;
  try {
    syncAllText();
    if (format === 'pdf') {
      const url = doc.to_pdf_data_url();
      const a = document.createElement('a');
      a.href = url; a.download = ($('docName').value || 'document') + '.pdf'; a.click();
      return;
    }
    const bytes = doc.export(format);
    const blob = new Blob([bytes]);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = ($('docName').value || 'document') + '.' + format; a.click();
    URL.revokeObjectURL(url);
  } catch (e) { alert('Export failed: ' + e.message); }
}

function activateEditor() {
  $('welcomeScreen').style.display = 'none';
  $('toolbar').classList.add('show');
  const menubar = $('appMenubar');
  if (menubar) menubar.classList.add('show');
  $('statusbar').classList.add('show');
  switchView('editor');
}

export function switchView(view) {
  state.currentView = view;
  $('editorCanvas').classList.toggle('show', view === 'editor');
  $('pagesView').classList.toggle('show', view === 'pages');
  $('textView').classList.toggle('show', view === 'text');
  $('toolbar').classList.toggle('show', view === 'editor');
  // Update legacy tab bar (hidden) and new status bar view buttons
  document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.view === view));
  document.querySelectorAll('.status-view-btn').forEach(b => b.classList.toggle('active', b.dataset.view === view));
  // Update view menu entries
  document.querySelectorAll('.tab-menu-entry').forEach(e => e.classList.toggle('active', e.dataset.view === view));
  if (view === 'pages') { syncAllText(); renderPages(); }
  if (view === 'text') { syncAllText(); renderText(); }
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
    if (state.dirty && state.doc) {
      e.preventDefault();
      e.returnValue = '';
    }
  });

  $('btnNew').addEventListener('click', newDocument);
  $('welcomeNew').addEventListener('click', newDocument);
  $('btnOpen').addEventListener('click', () => $('fileInput').click());
  $('welcomeOpen').addEventListener('click', () => $('fileInput').click());

  $('fileInput').addEventListener('change', e => {
    const f = e.target.files[0]; if (!f) return;
    const r = new FileReader();
    r.onload = () => openFile(new Uint8Array(r.result), f.name);
    r.readAsArrayBuffer(f); e.target.value = '';
  });

  // Drag & drop
  [$('dropZone'), $('editorCanvas')].forEach(t => {
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
