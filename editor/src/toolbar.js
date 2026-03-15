// Toolbar state & formatting handlers
import { state, $ } from './state.js';
import { getSelectionInfo, saveSelection, setCursorAtOffset, setSelectionRange } from './selection.js';
import { renderNodeById, renderNodesById, syncParagraphText } from './render.js';
import { updatePageBreaks } from './pagination.js';
import { broadcastOp } from './collab.js';

// Detect which style best matches the current paragraph formatting
function detectCurrentStyle(fmt) {
  const level = parseInt(fmt.headingLevel || '0') || 0;
  if (level === 1) return 'heading1';
  if (level === 2) return 'heading2';
  if (level === 3) return 'heading3';
  if (level === 4) return 'heading4';
  const fam = fmt.fontFamily || '';
  if (fam.toLowerCase().includes('courier') || fam.toLowerCase().includes('mono')) return 'code';
  const size = parseFloat(fmt.fontSize || '0');
  if (size >= 24) return 'title';
  if (size >= 14 && size <= 16 && fmt.color === '666666') return 'subtitle';
  if ((fmt.italic === true || fmt.italic === 'true') && fmt.color === '666666') return 'quote';
  return 'normal';
}

let _toolbarRAF = 0;
export function updateToolbarState() {
  // Debounce via requestAnimationFrame — selectionchange fires very frequently
  cancelAnimationFrame(_toolbarRAF);
  _toolbarRAF = requestAnimationFrame(_updateToolbarStateImpl);
}
function _updateToolbarStateImpl() {
  const { doc } = state;
  if (!doc || state.currentView !== 'editor') return;
  saveSelection();
  const info = state.lastSelInfo;
  if (!info) return;
  try {
    let fmt;
    if (info.collapsed) {
      fmt = JSON.parse(doc.get_formatting_json(info.startNodeId));
    } else {
      try {
        fmt = JSON.parse(doc.get_selection_formatting_json(
          info.startNodeId, info.startOffset, info.endNodeId, info.endOffset));
      } catch (_) { fmt = JSON.parse(doc.get_formatting_json(info.startNodeId)); }
    }
    // E-01 fix: Merge pending formats into the resolved formatting for toolbar display
    const pending = (info.collapsed && state.pendingFormats) ? state.pendingFormats : {};
    const on = (k) => {
      if (k in pending) return pending[k] === 'true';
      return fmt[k] === true || fmt[k] === 'true';
    };
    const setToggle = (id, active) => {
      const el = $(id);
      el.classList.toggle('active', active);
      el.setAttribute('aria-pressed', String(active));
    };
    setToggle('btnBold', on('bold'));
    setToggle('btnItalic', on('italic'));
    setToggle('btnUnderline', on('underline'));
    setToggle('btnStrike', on('strikethrough'));
    setToggle('btnSuperscript', on('superscript'));
    setToggle('btnSubscript', on('subscript'));
    if (fmt.fontSize && fmt.fontSize !== 'mixed') $('fontSize').value = Math.round(parseFloat(fmt.fontSize));
    if (fmt.fontFamily && fmt.fontFamily !== 'mixed') $('fontFamily').value = fmt.fontFamily;
    else if (!fmt.fontFamily) $('fontFamily').value = '';
    if (fmt.color && fmt.color !== 'mixed') $('colorSwatch').style.background = '#' + fmt.color;
    const paraFmt = info.collapsed ? fmt : JSON.parse(doc.get_formatting_json(info.startNodeId));
    // Update style gallery
    const styleName = detectCurrentStyle(paraFmt);
    const STYLE_LABELS = {
      normal: 'Normal', title: 'Title', subtitle: 'Subtitle',
      heading1: 'Heading 1', heading2: 'Heading 2', heading3: 'Heading 3', heading4: 'Heading 4',
      quote: 'Quote', code: 'Code',
    };
    $('styleGalleryLabel').textContent = STYLE_LABELS[styleName] || 'Normal';
    const panel = $('styleGalleryPanel');
    if (panel) {
      panel.querySelectorAll('.style-gallery-item').forEach(item => {
        const isActive = item.dataset.style === styleName;
        item.classList.toggle('active', isActive);
        item.setAttribute('aria-selected', String(isActive));
      });
    }
    setToggle('btnAlignL', !paraFmt.alignment || paraFmt.alignment === 'left');
    setToggle('btnAlignC', paraFmt.alignment === 'center');
    setToggle('btnAlignR', paraFmt.alignment === 'right');
    setToggle('btnAlignJ', paraFmt.alignment === 'justify');
  } catch (_) {}
}

export function updateUndoRedo() {
  if (!state.doc) return;
  try {
    const canUndo = state.doc.can_undo();
    const canRedo = state.doc.can_redo();
    $('btnUndo').disabled = !canUndo;
    $('btnRedo').disabled = !canRedo;
    // E3.4: Set tooltip with action label
    const undoLabel = state.undoHistory.length > state.undoHistoryPos
      ? state.undoHistory[state.undoHistoryPos]?.label : null;
    $('btnUndo').title = undoLabel ? `Undo: ${undoLabel}` : 'Undo (Ctrl+Z)';
    $('btnRedo').title = canRedo ? 'Redo (Ctrl+Y)' : 'Redo';
  } catch (_) {}
}

// E3.2: Record an action for the undo history viewer
export function recordUndoAction(label) {
  // Truncate any redo entries (new action invalidates redo history)
  if (state.undoHistoryPos > 0) {
    state.undoHistory.splice(0, state.undoHistoryPos);
    state.undoHistoryPos = 0;
  }
  state.undoHistory.unshift({ label, timestamp: Date.now() });
  // Cap at 100 entries
  if (state.undoHistory.length > 100) state.undoHistory.length = 100;
  renderUndoHistory();
}

// E3.2: Render the undo history in the history panel
export function renderUndoHistory() {
  const list = $('undoHistoryList');
  if (!list) return;
  list.innerHTML = '';
  state.undoHistory.forEach((entry, idx) => {
    const item = document.createElement('div');
    item.className = 'history-item' + (idx < state.undoHistoryPos ? ' undone' : '');
    const time = new Date(entry.timestamp);
    const timeStr = time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    item.innerHTML = `<span class="history-label">${escapeHistoryHtml(entry.label)}</span><span class="history-time">${timeStr}</span>`;
    item.addEventListener('click', () => {
      // Jump to this state: undo or redo as needed
      const stepsToUndo = idx - state.undoHistoryPos;
      if (stepsToUndo > 0) {
        for (let i = 0; i < stepsToUndo; i++) { try { state.doc.undo(); } catch (_) { break; } }
        state.undoHistoryPos = idx;
      } else if (stepsToUndo < 0) {
        for (let i = 0; i < -stepsToUndo; i++) { try { state.doc.redo(); } catch (_) { break; } }
        state.undoHistoryPos = idx;
      }
      // Re-import renderDocument dynamically to avoid circular deps
      import('./render.js').then(m => { m.renderDocument(); renderUndoHistory(); updateUndoRedo(); });
    });
    list.appendChild(item);
  });
}

function escapeHistoryHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export function applyFormat(key, value) {
  const { doc } = state;
  if (!doc) return;
  state._typingBatch = null; // E3.1: End typing session on format change
  const info = getSelectionInfo();
  if (!info) return;

  const page = $('pageContainer');
  const startEl = page.querySelector(`[data-node-id="${info.startNodeId}"]`) || info.startEl;
  const endEl = info.endNodeId !== info.startNodeId
    ? (page.querySelector(`[data-node-id="${info.endNodeId}"]`) || info.endEl)
    : startEl;

  // E-01 fix: When cursor is collapsed, store as pending format instead
  // of formatting the entire paragraph.
  if (info.collapsed) {
    if (!state.pendingFormats) state.pendingFormats = {};
    state.pendingFormats[key] = value;
    // Record cursor position so selectionchange can detect real movement
    state._pendingFormatCursorPos = { nodeId: info.startNodeId, offset: info.startOffset };
    // Update toolbar button state to reflect pending formats
    updateToolbarState();
    return;
  }

  // E-14: Flush pending sync timer before formatting to prevent lost edits
  clearTimeout(state.syncTimer);
  syncParagraphText(startEl);
  if (endEl !== startEl) syncParagraphText(endEl);

  try {
    let sn, so, en, eo;
    doc.format_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset, key, value);
    sn = info.startNodeId; so = info.startOffset; en = info.endNodeId; eo = info.endOffset;
    // E3.4: Record formatting action
    const friendlyKey = key.charAt(0).toUpperCase() + key.slice(1);
    recordUndoAction(`${value === 'false' ? 'Remove' : 'Apply'} ${friendlyKey}`);

    // E-05: Batch render all affected nodes to avoid race conditions
    const nodeIds = [info.startNodeId];
    if (info.endNodeId !== info.startNodeId) nodeIds.push(info.endNodeId);
    const rendered = renderNodesById(nodeIds);
    const newStartEl = rendered.get(info.startNodeId);
    const newEndEl = info.endNodeId !== info.startNodeId ? rendered.get(info.endNodeId) : null;

    page.focus();
    if (newStartEl) setSelectionRange(newStartEl, info.startOffset, newEndEl || newStartEl, info.endOffset);

    if (newStartEl) state.lastSelInfo = { ...info, startEl: newStartEl, endEl: newEndEl || newStartEl };
    state.pagesRendered = false;
    updatePageBreaks();
    updateToolbarState();
    updateUndoRedo();
    if (sn) broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key, value });
  } catch (e) { console.error('format error:', e); }
}

export function toggleFormat(key) {
  const { doc } = state;
  if (!doc) return;
  const info = getSelectionInfo();
  if (!info) return;

  const page = $('pageContainer');
  const startEl = page.querySelector(`[data-node-id="${info.startNodeId}"]`) || info.startEl;
  const endEl = info.endNodeId !== info.startNodeId
    ? (page.querySelector(`[data-node-id="${info.endNodeId}"]`) || info.endEl)
    : startEl;

  let isActive = false;
  try {
    if (info.collapsed) {
      // E-01 fix: Check pending formats first, then fall back to document formatting
      const pending = state.pendingFormats || {};
      if (key in pending) {
        isActive = pending[key] === 'true';
      } else {
        isActive = !!JSON.parse(doc.get_formatting_json(info.startNodeId))[key];
      }
    } else {
      // E-14: Flush pending sync timer before querying formatting
      clearTimeout(state.syncTimer);
      syncParagraphText(startEl);
      if (endEl !== startEl) syncParagraphText(endEl);
      try {
        isActive = JSON.parse(doc.get_selection_formatting_json(
          info.startNodeId, info.startOffset, info.endNodeId, info.endOffset))[key] === true;
      } catch (_) { isActive = !!JSON.parse(doc.get_formatting_json(info.startNodeId))[key]; }
    }
  } catch (_) {}

  const newVal = isActive ? 'false' : 'true';

  // Superscript/subscript mutual exclusion: apply both operations in a single
  // sync/render cycle to avoid the opposite format persisting due to intermediate
  // re-renders between separate applyFormat calls.
  if (key === 'superscript' && newVal === 'true') {
    applyFormatPair('subscript', 'false', key, newVal, info, startEl, endEl);
    return;
  }
  if (key === 'subscript' && newVal === 'true') {
    applyFormatPair('superscript', 'false', key, newVal, info, startEl, endEl);
    return;
  }
  applyFormat(key, newVal);
}

// Apply two format operations in a single sync/render cycle.
// Used for superscript/subscript mutual exclusion so the clearing and
// setting happen atomically without intermediate DOM re-renders.
function applyFormatPair(clearKey, clearVal, setKey, setVal, info, startEl, endEl) {
  const { doc } = state;
  if (!doc) return;
  const page = $('pageContainer');

  // For collapsed cursors, just update pending formats for both keys
  if (info.collapsed) {
    if (!state.pendingFormats) state.pendingFormats = {};
    state.pendingFormats[clearKey] = clearVal;
    state.pendingFormats[setKey] = setVal;
    // Record cursor position so selectionchange can detect real movement
    state._pendingFormatCursorPos = { nodeId: info.startNodeId, offset: info.startOffset };
    updateToolbarState();
    return;
  }

  // E-14: Flush pending sync timer before formatting
  clearTimeout(state.syncTimer);
  syncParagraphText(startEl);
  if (endEl !== startEl) syncParagraphText(endEl);

  try {
    const sn = info.startNodeId, so = info.startOffset;
    const en = info.endNodeId, eo = info.endOffset;

    // Apply both format operations before re-rendering
    doc.format_selection(sn, so, en, eo, clearKey, clearVal);
    doc.format_selection(sn, so, en, eo, setKey, setVal);

    // E-05: Single batch re-render after both operations
    const nodeIds = [info.startNodeId];
    if (info.endNodeId !== info.startNodeId) nodeIds.push(info.endNodeId);
    const rendered = renderNodesById(nodeIds);
    const newStartEl = rendered.get(info.startNodeId);
    let newEndEl = info.endNodeId !== info.startNodeId ? rendered.get(info.endNodeId) : null;

    page.focus();
    if (newStartEl) setSelectionRange(newStartEl, so, newEndEl || newStartEl, eo);
    if (newStartEl) state.lastSelInfo = { ...info, startEl: newStartEl, endEl: newEndEl || newStartEl };

    state.pagesRendered = false;
    updatePageBreaks();
    updateToolbarState();
    updateUndoRedo();
    broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key: clearKey, value: clearVal });
    broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key: setKey, value: setVal });
  } catch (e) { console.error('format pair error:', e); }
}
