// File operations: new, open, export, drag-drop
import { state, $ } from './state.js';
import { renderDocument, syncAllText, renderPages, renderText } from './render.js';
import { insertImage } from './images.js';

let detect_format_fn = null;

export function setDetectFormat(fn) { detect_format_fn = fn; }

// ─── Autosave ─────────────────────────────────────
const AUTOSAVE_INTERVAL = 30000; // 30 seconds
const DB_NAME = 'FolioAutosave';
const DB_VERSION = 1;

function openAutosaveDB() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('documents')) {
        db.createObjectStore('documents', { keyPath: 'id' });
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
      const tx = db.transaction('documents', 'readwrite');
      tx.objectStore('documents').put({
        id: 'current', name, bytes, timestamp: Date.now()
      });
      state.dirty = false;
      const info = $('statusInfo');
      const prev = info.textContent;
      info.textContent = 'Auto-saved';
      setTimeout(() => { info.textContent = prev; }, 1500);
    }).catch(() => {});
  } catch (_) {}
}

export function markDirty() {
  state.dirty = true;
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
  $('docName').value = 'Untitled Document';
  $('docPage').focus();
  state.dirty = false;
  startAutosave();
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
    if (name) $('docName').value = name.replace(/\.[^.]+$/, '');
    updateTrackChanges();
    state.dirty = false;
    startAutosave();
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
  $('tabbar').classList.add('show');
  $('statusbar').classList.add('show');
  switchView('editor');
}

export function switchView(view) {
  state.currentView = view;
  $('editorCanvas').classList.toggle('show', view === 'editor');
  $('pagesView').classList.toggle('show', view === 'pages');
  $('textView').classList.toggle('show', view === 'text');
  $('toolbar').classList.toggle('show', view === 'editor');
  document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.view === view));
  if (view === 'pages') { syncAllText(); renderPages(); }
  if (view === 'text') { syncAllText(); renderText(); }
}

function updateTrackChanges() {
  const { doc } = state;
  if (!doc) return;
  try {
    const html = doc.to_html();
    const hasTC = html.includes('<ins') || html.includes('<del');
    if (hasTC) {
      const ins = (html.match(/<ins/g) || []).length;
      const del = (html.match(/<del/g) || []).length;
      $('tcCount').textContent = (ins + del) + ' tracked changes';
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

  // Export
  $('btnExport').addEventListener('click', e => { e.stopPropagation(); $('exportMenu').classList.toggle('show'); });
  document.addEventListener('click', e => {
    if (!e.target.closest('.dropdown')) $('exportMenu').classList.remove('show');
    if (!e.target.closest('.insert-dropdown')) $('insertMenu').classList.remove('show');
  });
  $('exportMenu').querySelectorAll('.dropdown-item').forEach(btn => {
    btn.addEventListener('click', e => { e.stopPropagation(); exportDoc(btn.dataset.fmt); $('exportMenu').classList.remove('show'); });
  });

  // Tabs
  document.querySelectorAll('.tab').forEach(tab => {
    tab.addEventListener('click', () => switchView(tab.dataset.view));
  });

  // Track changes
  $('btnAcceptAll').addEventListener('click', () => {
    if (!state.doc) return;
    try { state.doc.accept_all_changes(); renderDocument(); updateTrackChanges(); }
    catch (e) { console.error('accept:', e); }
  });
  $('btnRejectAll').addEventListener('click', () => {
    if (!state.doc) return;
    try { state.doc.reject_all_changes(); renderDocument(); updateTrackChanges(); }
    catch (e) { console.error('reject:', e); }
  });
}
