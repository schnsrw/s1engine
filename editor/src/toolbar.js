// Toolbar state & formatting handlers
import { state, $ } from './state.js';
import { getSelectionInfo, saveSelection, setCursorAtOffset, setSelectionRange } from './selection.js';
import { renderNodeById, renderNodesById, syncParagraphText } from './render.js';
import { updatePageBreaks } from './pagination.js';
import { broadcastOp } from './collab.js';

/**
 * Collect all paragraph-level node IDs between startNodeId and endNodeId (inclusive).
 * Walks only direct children of .page-content across pages (O(pages * blocks_per_page)),
 * skipping deeply nested elements for better performance on large documents.
 */
function collectNodeIdsBetween(container, startNodeId, endNodeId) {
  if (startNodeId === endNodeId) return [startNodeId];
  const ids = [];
  let inRange = false;
  // Walk page-content direct children only — paragraphs are always direct children
  for (const pageEl of state.pageElements) {
    const contentEl = pageEl.querySelector('.page-content');
    if (!contentEl) continue;
    for (const el of contentEl.children) {
      if (!el.dataset?.nodeId) continue;
      const tag = el.tagName.toLowerCase();
      if (tag !== 'p' && !/^h[1-6]$/.test(tag)) continue;
      const nid = el.dataset.nodeId;
      if (nid === startNodeId || nid === endNodeId) {
        if (!inRange) { inRange = true; ids.push(nid); if (nid === endNodeId && ids.length > 0) return ids; continue; }
        else { ids.push(nid); return ids; }
      }
      if (inRange) ids.push(nid);
    }
  }
  // Fallback: if we didn't find the range (e.g. reversed selection), return at least start+end
  if (ids.length === 0) {
    ids.push(startNodeId);
    if (endNodeId !== startNodeId) ids.push(endNodeId);
  }
  return ids;
}

// Detect which style best matches the current paragraph formatting.
// Prefers explicit styleId from document model; falls back to heuristics.
function detectCurrentStyle(fmt) {
  // 1. Prefer explicit styleId from the model (set by set_paragraph_style_id)
  if (fmt.styleId) {
    const sid = fmt.styleId.toLowerCase();
    if (sid === 'title') return 'title';
    if (sid === 'subtitle') return 'subtitle';
    if (sid === 'quote') return 'quote';
    if (sid === 'code') return 'code';
    if (sid.startsWith('heading')) {
      const n = parseInt(sid.replace(/\D/g, '')) || 0;
      if (n >= 1 && n <= 4) return 'heading' + n;
    }
    // Known non-empty styleId but not one we display → treat as normal
    return 'normal';
  }
  // 2. Fall back to heading level (backward compatibility)
  const level = parseInt(fmt.headingLevel || '0') || 0;
  if (level >= 1 && level <= 4) return 'heading' + level;
  // 3. Heuristic detection for documents without styleId
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
    // List button active state
    setToggle('btnBulletList', paraFmt.listFormat === 'bullet');
    setToggle('btnNumberList', paraFmt.listFormat === 'decimal');

    // E7.2: Announce page change when cursor moves to a different page
    if (info.startEl) {
      const pageEl = info.startEl.closest?.('.doc-page');
      if (pageEl) {
        const pageNum = parseInt(pageEl.dataset.page) || 1;
        if (pageNum !== state.activePageNum) {
          state.activePageNum = pageNum;
          const totalPages = state.pageElements?.length || 1;
          const a11y = $('a11yLive');
          if (a11y) {
            a11y.textContent = `Page ${pageNum} of ${totalPages}`;
            setTimeout(() => { if (a11y.textContent === `Page ${pageNum} of ${totalPages}`) a11y.textContent = ''; }, 1500);
          }
        }
      }
    }
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
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
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
  // Collect ALL paragraph-level elements in the selection range and sync them
  clearTimeout(state.syncTimer);
  const allNodeIds = collectNodeIdsBetween(page, info.startNodeId, info.endNodeId);
  for (const nid of allNodeIds) {
    const el = page.querySelector(`[data-node-id="${nid}"]`);
    if (el) syncParagraphText(el);
  }

  try {
    let sn, so, en, eo;
    doc.format_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset, key, value);
    sn = info.startNodeId; so = info.startOffset; en = info.endNodeId; eo = info.endOffset;

    // FS-37: Batch undo for rapid formatting operations (within 500ms)
    // Multiple format operations (e.g., bold then italic) within the batch window
    // are grouped into a single undo step.
    const now = Date.now();
    const BATCH_WINDOW_MS = 500;
    const batch = state._formatBatch;
    if (batch && (now - batch.lastTime) < BATCH_WINDOW_MS) {
      // Continue the batch — increment count, reset timer
      batch.count++;
      batch.lastTime = now;
      clearTimeout(batch.timer);
      batch.timer = setTimeout(() => { state._formatBatch = null; }, BATCH_WINDOW_MS);
    } else {
      // Start a new batch
      if (batch) clearTimeout(batch.timer);
      state._formatBatch = {
        count: 1,
        lastTime: now,
        timer: setTimeout(() => { state._formatBatch = null; }, BATCH_WINDOW_MS),
      };
    }

    // E3.4: Record formatting action
    const friendlyKey = key.charAt(0).toUpperCase() + key.slice(1);
    recordUndoAction(`${value === 'false' ? 'Remove' : 'Apply'} ${friendlyKey}`);

    // Re-render ALL affected nodes (not just start + end)
    const rendered = renderNodesById(allNodeIds);
    const newStartEl = rendered.get(info.startNodeId);
    const newEndEl = info.endNodeId !== info.startNodeId ? rendered.get(info.endNodeId) : null;

    // Restore selection — for cross-page select-all, re-apply highlight
    if (state._selectAll) {
      // Update lastSelInfo with fresh DOM elements
      if (newStartEl) state.lastSelInfo = { ...info, startEl: newStartEl, endEl: newEndEl || newStartEl };
      // Re-apply visual highlight across all pages
      for (const pageEl of state.pageElements) {
        const content = pageEl.querySelector('.page-content') || pageEl;
        content.classList.add('select-all-highlight');
        content.querySelectorAll('[data-node-id]').forEach(el => el.classList.add('select-all-highlight'));
      }
    } else {
      // Focus the page-content that contains the start element
      const targetContent = (newStartEl || startEl)?.closest?.('.page-content');
      if (targetContent) targetContent.focus();
      else page.focus();
      if (newStartEl) setSelectionRange(newStartEl, info.startOffset, newEndEl || newStartEl, info.endOffset);
      if (newStartEl) state.lastSelInfo = { ...info, startEl: newStartEl, endEl: newEndEl || newStartEl };
    }
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
      // E-14: Flush pending sync timer before querying formatting — sync ALL paragraphs in range
      clearTimeout(state.syncTimer);
      const toggleNodeIds = collectNodeIdsBetween(page, info.startNodeId, info.endNodeId);
      for (const nid of toggleNodeIds) {
        const el = page.querySelector(`[data-node-id="${nid}"]`);
        if (el) syncParagraphText(el);
      }
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
  // Sync ALL paragraphs in the selection range
  clearTimeout(state.syncTimer);
  const allNodeIds = collectNodeIdsBetween(page, info.startNodeId, info.endNodeId);
  for (const nid of allNodeIds) {
    const el = page.querySelector(`[data-node-id="${nid}"]`);
    if (el) syncParagraphText(el);
  }

  try {
    const sn = info.startNodeId, so = info.startOffset;
    const en = info.endNodeId, eo = info.endOffset;

    // Apply both format operations before re-rendering
    doc.format_selection(sn, so, en, eo, clearKey, clearVal);
    doc.format_selection(sn, so, en, eo, setKey, setVal);

    // E3.4: Record formatting action
    const friendlyKey = setKey.charAt(0).toUpperCase() + setKey.slice(1);
    recordUndoAction(`Apply ${friendlyKey}`);

    // Re-render ALL affected nodes (not just start + end)
    const rendered = renderNodesById(allNodeIds);
    const newStartEl = rendered.get(info.startNodeId);
    let newEndEl = info.endNodeId !== info.startNodeId ? rendered.get(info.endNodeId) : null;

    // Restore selection — for cross-page select-all, re-apply highlight
    if (state._selectAll) {
      if (newStartEl) state.lastSelInfo = { ...info, startEl: newStartEl, endEl: newEndEl || newStartEl };
      for (const pageEl of state.pageElements) {
        const content = pageEl.querySelector('.page-content') || pageEl;
        content.classList.add('select-all-highlight');
        content.querySelectorAll('[data-node-id]').forEach(el => el.classList.add('select-all-highlight'));
      }
    } else {
      page.focus();
      if (newStartEl) setSelectionRange(newStartEl, so, newEndEl || newStartEl, eo);
      if (newStartEl) state.lastSelInfo = { ...info, startEl: newStartEl, endEl: newEndEl || newStartEl };
    }

    state.pagesRendered = false;
    updatePageBreaks();
    updateToolbarState();
    updateUndoRedo();
    broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key: clearKey, value: clearVal });
    broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key: setKey, value: setVal });
  } catch (e) { console.error('format pair error:', e); }
}

// ─── Floating Selection Toolbar ──────────────────────────
// Shows a compact formatting bar above the selection, similar to Google Docs.

let _floatingEl = null;
let _floatingHideTimer = null;

function ensureFloatingToolbar() {
  if (_floatingEl) return _floatingEl;
  const el = document.createElement('div');
  el.className = 'floating-toolbar';
  el.setAttribute('role', 'toolbar');
  el.setAttribute('aria-label', 'Selection formatting');
  el.innerHTML = `
    <button class="ft-btn" data-fmt="bold" title="Bold (Ctrl+B)"><b>B</b></button>
    <button class="ft-btn" data-fmt="italic" title="Italic (Ctrl+I)"><i>I</i></button>
    <button class="ft-btn" data-fmt="underline" title="Underline (Ctrl+U)"><u>U</u></button>
    <button class="ft-btn" data-fmt="strikethrough" title="Strikethrough"><s>S</s></button>
    <span class="ft-sep"></span>
    <button class="ft-btn" data-fmt="superscript" title="Superscript"><span style="font-size:10px;vertical-align:super">A</span><sup style="font-size:8px">2</sup></button>
    <button class="ft-btn" data-fmt="subscript" title="Subscript"><span style="font-size:10px">A</span><sub style="font-size:8px">2</sub></button>
    <span class="ft-sep"></span>
    <button class="ft-btn ft-color" data-action="color" title="Text color">
      <span style="font-weight:600">A</span>
      <span class="ft-color-bar" id="ftColorBar"></span>
    </button>
    <button class="ft-btn ft-color" data-action="highlight" title="Highlight color">
      <span style="font-weight:600;background:#ffeb3b;padding:0 3px;border-radius:2px">A</span>
    </button>
    <span class="ft-sep"></span>
    <button class="ft-btn" data-action="link" title="Insert link">
      <span style="text-decoration:underline;color:var(--accent)">Link</span>
    </button>
    <button class="ft-btn" data-action="comment" title="Add comment">
      <span style="font-size:11px">Comment</span>
    </button>
  `;
  // Prevent toolbar clicks from stealing focus/selection
  el.addEventListener('mousedown', e => {
    e.preventDefault();
    e.stopPropagation();
  });
  // Handle button clicks
  el.addEventListener('click', e => {
    const btn = e.target.closest('.ft-btn');
    if (!btn) return;
    const fmt = btn.dataset.fmt;
    const action = btn.dataset.action;
    if (fmt) {
      // Toggle formatting
      const info = getSelectionInfo();
      if (!info || info.collapsed) return;
      toggleFormat(fmt);
    } else if (action === 'color') {
      const picker = document.createElement('input');
      picker.type = 'color';
      picker.value = '#' + ($('colorSwatch')?.style.background ? '000000' : '000000');
      picker.style.position = 'absolute';
      picker.style.opacity = '0';
      picker.style.pointerEvents = 'none';
      document.body.appendChild(picker);
      picker.addEventListener('input', () => {
        const hex = picker.value.replace('#', '').toUpperCase();
        applyFormat('color', hex);
      });
      picker.addEventListener('change', () => {
        if (picker.parentNode) picker.remove();
      });
      // Clean up on blur too (user clicked away without selecting)
      picker.addEventListener('blur', () => {
        setTimeout(() => { if (picker.parentNode) picker.remove(); }, 100);
      });
      picker.click();
    } else if (action === 'highlight') {
      applyFormat('highlightColor', 'FFFF00');
    } else if (action === 'link') {
      const url = prompt('Enter URL:');
      if (url && url.trim()) {
        try {
          new URL(url.trim().startsWith('http') ? url.trim() : 'https://' + url.trim());
          applyFormat('hyperlinkUrl', url.trim());
        } catch (_) { alert('Invalid URL'); }
      }
    } else if (action === 'comment') {
      const text = prompt('Add comment:');
      if (text && text.trim()) {
        const info = getSelectionInfo();
        if (info && !info.collapsed && state.doc) {
          try {
            state.doc.insert_comment(info.startNodeId, info.endNodeId, 'User', text.trim());
            import('./render.js').then(m => m.renderDocument());
          } catch (e) { console.error('insert comment:', e); }
        }
      }
    }
  });
  document.body.appendChild(el);
  _floatingEl = el;
  return el;
}

export function updateFloatingToolbar() {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || state.currentView !== 'editor' || !state.doc) {
    hideFloatingToolbar();
    return;
  }
  // Ensure selection is within editor
  const range = sel.getRangeAt(0);
  const container = $('pageContainer');
  if (!container || !container.contains(range.commonAncestorContainer)) {
    hideFloatingToolbar();
    return;
  }
  // Don't show if selection is very small (less than 2 chars)
  const text = sel.toString();
  if (text.length < 2) {
    hideFloatingToolbar();
    return;
  }

  clearTimeout(_floatingHideTimer);
  const ft = ensureFloatingToolbar();

  // Position above selection
  const rect = range.getBoundingClientRect();
  const ftWidth = 420;
  let left = rect.left + (rect.width / 2) - (ftWidth / 2);
  let top = rect.top - 44;

  // Keep within viewport
  if (left < 8) left = 8;
  if (left + ftWidth > window.innerWidth - 8) left = window.innerWidth - ftWidth - 8;
  if (top < 8) top = rect.bottom + 8;

  ft.style.left = left + 'px';
  ft.style.top = top + 'px';
  ft.classList.add('visible');

  // Update active states on floating toolbar buttons
  const info = state.lastSelInfo;
  if (info && !info.collapsed) {
    try {
      const fmt = JSON.parse(state.doc.get_selection_formatting_json(
        info.startNodeId, info.startOffset, info.endNodeId, info.endOffset));
      ft.querySelectorAll('.ft-btn[data-fmt]').forEach(btn => {
        const key = btn.dataset.fmt;
        btn.classList.toggle('active', fmt[key] === true);
      });
    } catch (_) {}
  }
}

function hideFloatingToolbar() {
  if (_floatingEl) {
    _floatingEl.classList.remove('visible');
  }
}

// Show floating toolbar after a small delay on mouseup (like Google Docs)
let _floatingMouseUpTimer = null;
document.addEventListener('mouseup', () => {
  clearTimeout(_floatingMouseUpTimer);
  _floatingMouseUpTimer = setTimeout(() => {
    updateFloatingToolbar();
  }, 200);
});
document.addEventListener('keyup', (e) => {
  // Show on shift+arrow key selection
  if (e.shiftKey && (e.key.startsWith('Arrow') || e.key === 'Home' || e.key === 'End')) {
    clearTimeout(_floatingMouseUpTimer);
    _floatingMouseUpTimer = setTimeout(() => {
      updateFloatingToolbar();
    }, 300);
  }
});
// Hide on scroll/click elsewhere
document.addEventListener('mousedown', (e) => {
  if (_floatingEl && !_floatingEl.contains(e.target)) {
    hideFloatingToolbar();
  }
});
document.addEventListener('scroll', () => {
  hideFloatingToolbar();
}, true);
// Hide when typing
document.addEventListener('keydown', (e) => {
  if (!e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey && e.key.length === 1) {
    hideFloatingToolbar();
  }
});
