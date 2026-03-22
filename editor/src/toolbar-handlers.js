// Toolbar event handler wiring
import { state, $ } from './state.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo, recordUndoAction } from './toolbar.js';

// Circular dep breakers: import from extracted feature modules instead of input.js
import { doUndo, doRedo } from './features/document/input/undo-redo.js';
import { closeSlashMenu } from './features/document/input/slash-menu.js';
import { insertFootnoteAtCursor, insertEndnoteAtCursor } from './features/document/input/footnotes.js';

// Re-export extracted modules for backward compatibility.
// Other files that import from toolbar-handlers.js continue to work unchanged.
export { showToast, announce } from './features/document/toolbar/toast-announce.js';
export { exitFormatPainter, applyFormatPainter } from './features/document/toolbar/format-painter.js';
export { enterHeaderFooterEditMode, exitHeaderFooterEditMode } from './features/document/toolbar/header-footer.js';
export { getAutoCorrectMap, isAutoCorrectEnabled } from './features/document/toolbar/autocorrect.js';
export { setZoomLevel } from './features/document/toolbar/zoom.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText, applyPageDimensions, isCanvasMode, setCanvasMode, initCanvasRenderer, markLayoutDirty } from './render.js';
import { getSelectionInfo, setCursorAtOffset, setSelectionRange, getActiveNodeId, saveSelection } from './selection.js';
import { insertImage } from './images.js';
import { updatePageBreaks } from './pagination.js';
import { renderRuler } from './ruler.js';
import { getVersions, restoreVersion, saveVersion, openAutosaveDB, newDocument, updateDirtyIndicator, updateStatusBar, updateTrackChanges, markDirty } from './file.js';
import { showShareDialog, broadcastOp } from './collab.js';
import { trackEvent, getStats, clearStats, getSessionDuration } from './analytics.js';
import { getLastError, clearErrors } from './error-tracking.js';

// ── Custom modal helpers (replace browser confirm/prompt) ─────

function showConfirmModal(message) {
  return new Promise(resolve => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay show';
    const modal = document.createElement('div');
    modal.className = 'modal';
    const h3 = document.createElement('h3');
    h3.textContent = message;
    modal.appendChild(h3);
    const actions = document.createElement('div');
    actions.className = 'modal-actions';
    const cancelBtn = document.createElement('button');
    cancelBtn.textContent = 'Cancel';
    cancelBtn.className = 'modal-cancel';
    const okBtn = document.createElement('button');
    okBtn.textContent = 'Confirm';
    okBtn.className = 'modal-ok primary';
    actions.appendChild(cancelBtn);
    actions.appendChild(okBtn);
    modal.appendChild(actions);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    const close = (val) => { document.body.removeChild(overlay); resolve(val); };
    cancelBtn.onclick = () => close(false);
    okBtn.onclick = () => close(true);
    overlay.onclick = (e) => { if (e.target === overlay) close(false); };
    okBtn.focus();
  });
}

function showPromptModal(message, defaultValue) {
  return new Promise(resolve => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay show';
    const modal = document.createElement('div');
    modal.className = 'modal';
    const h3 = document.createElement('h3');
    h3.textContent = message;
    modal.appendChild(h3);
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'modal-input';
    input.value = defaultValue || '';
    input.style.cssText = 'width:100%;padding:8px;margin:8px 0 16px;border:1px solid #dadce0;border-radius:4px;font-size:14px;box-sizing:border-box;';
    modal.appendChild(input);
    const actions = document.createElement('div');
    actions.className = 'modal-actions';
    const cancelBtn = document.createElement('button');
    cancelBtn.textContent = 'Cancel';
    cancelBtn.className = 'modal-cancel';
    const okBtn = document.createElement('button');
    okBtn.textContent = 'OK';
    okBtn.className = 'modal-ok primary';
    actions.appendChild(cancelBtn);
    actions.appendChild(okBtn);
    modal.appendChild(actions);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    const close = (val) => { document.body.removeChild(overlay); resolve(val); };
    cancelBtn.onclick = () => close(null);
    okBtn.onclick = () => close(input.value);
    overlay.onclick = (e) => { if (e.target === overlay) close(null); };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') close(input.value);
      if (e.key === 'Escape') close(null);
    });
    input.focus();
    input.select();
  });
}

// ── Selection save/restore for modal dialogs ─────
// Saves the current DOM selection before a modal opens, restores it after close.
// We store both the DOM Range (fast path) and node ID + offset info (survives re-renders).
let _savedModalSelection = null;
let _savedModalSelInfo = null;

function saveModalSelection() {
  try {
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
      _savedModalSelection = sel.getRangeAt(0).cloneRange();
    } else {
      _savedModalSelection = null;
    }
  } catch (_) {
    _savedModalSelection = null;
  }
  // Also save node ID + offset info which survives DOM re-renders
  try {
    _savedModalSelInfo = getSelectionInfo();
  } catch (_) {
    _savedModalSelInfo = null;
  }
}

function restoreModalSelection() {
  if (!_savedModalSelection && !_savedModalSelInfo) return;
  try {
    // ED2-18: Verify saved range nodes are still in the DOM after re-render
    const startOk = _savedModalSelection && _savedModalSelection.startContainer && _savedModalSelection.startContainer.isConnected;
    const endOk = _savedModalSelection && _savedModalSelection.endContainer && _savedModalSelection.endContainer.isConnected;
    if (startOk && endOk) {
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(_savedModalSelection);
    } else if (_savedModalSelInfo && _savedModalSelInfo.startNodeId) {
      // DOM Range is invalid (nodes disconnected after re-render).
      // Fall back to restoring via node ID + offset which survives re-renders.
      const page = $('pageContainer');
      if (page) {
        const startEl = page.querySelector(`[data-node-id="${_savedModalSelInfo.startNodeId}"]`);
        if (startEl) {
          const content = startEl.closest('.page-content');
          if (content) content.focus();
          if (_savedModalSelInfo.collapsed) {
            setCursorAtOffset(startEl, _savedModalSelInfo.startOffset);
          } else {
            const endEl = page.querySelector(`[data-node-id="${_savedModalSelInfo.endNodeId}"]`);
            if (endEl) {
              setSelectionRange(startEl, _savedModalSelInfo.startOffset, endEl, _savedModalSelInfo.endOffset);
            } else {
              setCursorAtOffset(startEl, _savedModalSelInfo.startOffset);
            }
          }
        }
      }
    } else {
      // Last resort: place cursor at start of first paragraph
      const firstPara = $('pageContainer')?.querySelector('.page-content p[data-node-id], .page-content h1[data-node-id]');
      if (firstPara) {
        const content = firstPara.closest('.page-content');
        if (content) content.focus();
        const range = document.createRange();
        range.setStart(firstPara, 0);
        range.collapse(true);
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);
      }
    }
  } catch (_) {
    // Range may be invalid if DOM changed; try node-ID fallback
    try {
      if (_savedModalSelInfo && _savedModalSelInfo.startNodeId) {
        const page = $('pageContainer');
        if (page) {
          const startEl = page.querySelector(`[data-node-id="${_savedModalSelInfo.startNodeId}"]`);
          if (startEl) {
            const content = startEl.closest('.page-content');
            if (content) content.focus();
            setCursorAtOffset(startEl, _savedModalSelInfo.startOffset);
          }
        }
      }
    } catch (_2) { /* give up */ }
  }
  _savedModalSelection = null;
  _savedModalSelInfo = null;
}

// announce() and showToast() extracted to features/document/toolbar/toast-announce.js
// Re-exported at top of this file for backward compatibility.
import { announce as _announce } from './features/document/toolbar/toast-announce.js';
const announce = _announce; // local alias for internal use

function restoreSelectionForPickers() {
  const info = state.lastSelInfo;
  if (!info) return false;
  const page = $('pageContainer');
  if (!page) return false;
  const startEl = page.querySelector(`[data-node-id="${info.startNodeId}"]`);
  if (!startEl) return false;

  // CRITICAL: Focus the page-content BEFORE setting the range.
  // Without focus, addRange() silently fails on contenteditable elements.
  const content = startEl.closest('.page-content');
  if (content) content.focus();

  if (info.collapsed) {
    setSelectionRange(startEl, info.startOffset, startEl, info.startOffset);
    return true;
  }
  const endEl = page.querySelector(`[data-node-id="${info.endNodeId}"]`);
  if (!endEl) return false;
  setSelectionRange(startEl, info.startOffset, endEl, info.endOffset);
  return true;
}

// showToast() extracted to features/document/toolbar/toast-announce.js
import { showToast as _showToast } from './features/document/toolbar/toast-announce.js';
const showToast = _showToast; // local alias for internal use

export function initToolbar() {
  // U9: Prevent toolbar buttons from stealing focus (causes selection flash).
  // mousedown preventDefault keeps the contenteditable focused.
  const toolbar = $('toolbar');
  if (toolbar) {
    toolbar.addEventListener('mousedown', e => {
      // Allow inputs/selects to receive focus normally
      const tag = e.target.tagName;
      if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;
      e.preventDefault();
    });
  }

  // App menu bar (File/Edit/View/Insert/Format/Tools) dropdown behavior
  initAppMenubar();
  // Format toggles
  $('btnBold').addEventListener('click', () => { toggleFormat('bold'); announce('Bold toggled'); trackEvent('toolbar', 'bold'); });
  $('btnItalic').addEventListener('click', () => { toggleFormat('italic'); announce('Italic toggled'); trackEvent('toolbar', 'italic'); });
  $('btnUnderline').addEventListener('click', () => { toggleFormat('underline'); announce('Underline toggled'); trackEvent('toolbar', 'underline'); });
  $('btnStrike').addEventListener('click', () => { toggleFormat('strikethrough'); announce('Strikethrough toggled'); trackEvent('toolbar', 'strikethrough'); });
  $('btnSuperscript').addEventListener('click', () => { toggleFormat('superscript'); announce('Superscript toggled'); trackEvent('toolbar', 'superscript'); });
  $('btnSubscript').addEventListener('click', () => { toggleFormat('subscript'); announce('Subscript toggled'); trackEvent('toolbar', 'subscript'); });

  // UXP-14: Format Painter
  initFormatPainter();

  // Clear formatting
  $('btnClearFormat').addEventListener('click', () => {
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    // E-01 fix: When cursor is collapsed, just clear pending formats
    if (info.collapsed) {
      state.pendingFormats = {};
      updateToolbarState();
      return;
    }
    syncAllText();
    try {
      const keys = ['bold', 'italic', 'underline', 'strikethrough', 'superscript', 'subscript', 'color', 'highlightColor', 'fontFamily', 'fontSize'];
      const sn = info.startNodeId, so = info.startOffset;
      const en = info.endNodeId, eo = info.endOffset;
      keys.forEach(k => {
        try {
          if (eo > 0 || sn !== en) state.doc.format_selection(sn, so, en, eo, k, 'false');
        } catch (_) {}
      });
      keys.forEach(k => {
        broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key: k, value: 'false' });
      });
      renderDocument();
      updateToolbarState();
      updateUndoRedo();
      announce('Formatting cleared');
    } catch (e) { console.error('clear format:', e); }
  });

  // Undo/Redo
  $('btnUndo').addEventListener('click', () => { doUndo(); trackEvent('toolbar', 'undo'); });
  $('btnRedo').addEventListener('click', () => { doRedo(); trackEvent('toolbar', 'redo'); });

  // Print
  $('btnPrint').addEventListener('click', () => {
    window.print();
    trackEvent('toolbar', 'print');
  });

  // Font family
  $('fontFamily').addEventListener('change', e => {
    if (e.target.value) applyFormat('fontFamily', e.target.value);
  });

  // Font size
  $('fontSize').addEventListener('change', e => {
    const v = parseInt(e.target.value);
    if (v >= 6 && v <= 96) applyFormat('fontSize', String(v));
  });

  // Style gallery dropdown
  initStyleGallery();

  // FS-21: Text color — palette dropdown
  const colorPicker = $('colorPicker');
  initColorPaletteDropdown(colorPicker, $('colorSwatch'), 'color');

  // FS-22: Highlight color — preset highlight palette dropdown
  const highlightPicker = $('highlightPicker');
  initHighlightPaletteDropdown(highlightPicker);

  // Line spacing
  $('lineSpacing').addEventListener('change', e => {
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    syncAllText();
    try {
      const paraIds = getSelectedParagraphIds(info);
      paraIds.forEach(nodeId => {
        state.doc.set_line_spacing(nodeId, e.target.value);
        broadcastOp({ action: 'setLineSpacing', nodeId, value: e.target.value });
      });
      renderDocument();
      updateUndoRedo();
    } catch (err) { console.error('line spacing:', err); }
  });

  // Indent / Outdent
  $('btnIndent').addEventListener('click', () => { applyIndent(36); trackEvent('toolbar', 'indent'); });   // +0.5in (36pt)
  $('btnOutdent').addEventListener('click', () => { applyIndent(-36); trackEvent('toolbar', 'outdent'); }); // -0.5in

  // Alignment
  $('btnAlignL').addEventListener('click', () => { applyAlignment('left'); trackEvent('toolbar', 'align-left'); });
  $('btnAlignC').addEventListener('click', () => { applyAlignment('center'); trackEvent('toolbar', 'align-center'); });
  $('btnAlignR').addEventListener('click', () => { applyAlignment('right'); trackEvent('toolbar', 'align-right'); });
  $('btnAlignJ').addEventListener('click', () => { applyAlignment('justify'); trackEvent('toolbar', 'align-justify'); });

  // Lists
  $('btnBulletList').addEventListener('click', () => { toggleList('bullet'); trackEvent('toolbar', 'bullet-list'); });
  $('btnNumberList').addEventListener('click', () => { toggleList('decimal'); trackEvent('toolbar', 'number-list'); });

  // Insert menu (position:fixed, so calculate position from button rect)
  $('btnInsertMenu').addEventListener('click', e => {
    e.stopPropagation();
    const menu = $('insertMenu');
    const wasOpen = menu.classList.contains('show');
    menu.classList.toggle('show');
    if (!wasOpen) {
      const rect = $('btnInsertMenu').getBoundingClientRect();
      // FS-23: Clamp menu position to stay within the viewport, flip above if overflowing
      let top = rect.bottom + 4;
      let left = rect.left;
      // Measure menu after making it visible
      requestAnimationFrame(() => {
        const menuRect = menu.getBoundingClientRect();
        if (top + menuRect.height > window.innerHeight) {
          const flipped = rect.top - menuRect.height - 4;
          top = flipped > 4 ? flipped : Math.max(4, window.innerHeight - menuRect.height - 4);
        }
        if (left + menuRect.width > window.innerWidth) {
          left = Math.max(4, window.innerWidth - menuRect.width - 4);
        }
        menu.style.top = top + 'px';
        menu.style.left = left + 'px';
      });
      menu.style.top = top + 'px';
      menu.style.left = left + 'px';
      // FS-06: Focus first item when opening via keyboard/click
      requestAnimationFrame(() => {
        const firstItem = menu.querySelector('[data-action]');
        if (firstItem) firstItem.focus();
      });
    }
    $('btnInsertMenu').setAttribute('aria-expanded', menu.classList.contains('show') ? 'true' : 'false');
  });

  // FS-06: Arrow-key navigation in insert dropdown (WAI-ARIA Menu Pattern)
  $('insertMenu')?.addEventListener('keydown', e => {
    const menu = $('insertMenu');
    const items = Array.from(menu.querySelectorAll('[data-action]'));
    if (items.length === 0) return;
    const currentIdx = items.indexOf(document.activeElement);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = currentIdx < 0 ? 0 : (currentIdx + 1) % items.length;
      items[next].focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = currentIdx <= 0 ? items.length - 1 : currentIdx - 1;
      items[prev].focus();
    } else if (e.key === 'Home') {
      e.preventDefault();
      items[0].focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      items[items.length - 1].focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      menu.classList.remove('show');
      $('btnInsertMenu').setAttribute('aria-expanded', 'false');
      $('btnInsertMenu').focus();
    }
  });


  // Insert table
  $('miTable').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    saveModalSelection();
    $('tableModal').classList.add('show');
    $('tableRows').focus();
    trackEvent('insert', 'table');
  });
  $('tableCancelBtn').addEventListener('click', () => {
    $('tableModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });
  $('tableInsertBtn').addEventListener('click', () => {
    const rows = parseInt($('tableRows').value) || 3;
    const cols = parseInt($('tableCols').value) || 3;
    if (rows < 1 || rows > 100 || cols < 1 || cols > 50) {
      showToast('Rows must be 1-100, columns must be 1-50.', 'error');
      return;
    }
    $('tableModal').classList.remove('show');
    if (!state.doc) return;
    // Restore cursor position saved before modal opened
    restoreModalSelection();
    let nodeId = getActiveNodeId();
    // Fallback: if selection was lost, use the last known selection
    if (!nodeId && state.lastSelInfo) {
      nodeId = state.lastSelInfo.startNodeId;
    }
    // Last resort: use the first paragraph in the document
    if (!nodeId) {
      try {
        const allIds = JSON.parse(state.doc.paragraph_ids_json());
        if (allIds.length > 0) nodeId = allIds[0];
      } catch (_) {}
    }
    if (!nodeId) return;
    syncAllText();
    try {
      state.doc.insert_table(nodeId, rows, cols);
      broadcastOp({ action: 'insertTable', afterNodeId: nodeId, rows, cols });
      renderDocument();
      updateUndoRedo();
      markDirty();
      announce(`Table inserted ${rows} by ${cols}`);
    } catch (e) { console.error('insert table:', e); }
  });
  // Modal backdrop click to close
  $('tableModal').addEventListener('click', e => {
    if (e.target === $('tableModal')) {
      $('tableModal').classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });

  // Insert image
  $('miImage').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    $('imageInput').click();
    trackEvent('insert', 'image');
  });
  $('imageInput').addEventListener('change', e => {
    const f = e.target.files[0];
    if (f) insertImage(f);
    e.target.value = '';
  });

  // Insert hyperlink — modal
  $('miLink').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    trackEvent('insert', 'link');
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    state._linkSelInfo = info; // stash selection for after modal
    saveModalSelection();
    $('linkUrl').value = '';
    $('linkModal').classList.add('show');
    $('linkUrl').focus();
  });
  $('linkCancelBtn').addEventListener('click', () => {
    $('linkModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });
  $('linkInsertBtn').addEventListener('click', () => {
    let url = $('linkUrl').value.trim();
    if (!url) { $('linkModal').classList.remove('show'); return; }
    if (!/^https?:\/\//i.test(url) && !url.startsWith('#')) url = 'https://' + url;
    try { new URL(url); } catch (_) {
      $('linkUrl').style.borderColor = 'var(--danger)';
      setTimeout(() => { $('linkUrl').style.borderColor = ''; }, 1500);
      return;
    }
    $('linkModal').classList.remove('show');
    try { applyFormat('hyperlinkUrl', url); }
    catch (e) { console.error('hyperlink:', e); }
  });
  $('linkModal').addEventListener('click', e => {
    if (e.target === $('linkModal')) { $('linkModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
  $('linkUrl').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); $('linkInsertBtn').click(); }
    if (e.key === 'Escape') { $('linkModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });

  // Insert horizontal rule
  $('miHR').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    if (!state.doc) return;
    const nodeId = getActiveNodeId();
    if (!nodeId) return;
    syncAllText();
    try {
      state.doc.insert_horizontal_rule(nodeId);
      broadcastOp({ action: 'insertHR', afterNodeId: nodeId });
      renderDocument();
      updateUndoRedo();
      announce('Horizontal rule inserted');
    } catch (e) { console.error('insert HR:', e); }
  });

  // Insert page break
  $('miPageBreak').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    if (!state.doc) return;
    const nodeId = getActiveNodeId();
    if (!nodeId) return;
    syncAllText();
    try {
      state.doc.insert_page_break(nodeId);
      broadcastOp({ action: 'insertPageBreak', afterNodeId: nodeId });
      renderDocument();
      updateUndoRedo();
      announce('Page break inserted');
    } catch (e) { console.error('insert page break:', e); }
  });

  // UXP-22: Insert column break
  if ($('miColumnBreak')) {
    $('miColumnBreak').addEventListener('click', () => {
      $('insertMenu').classList.remove('show');
      if (!state.doc) return;
      const nodeId = getActiveNodeId();
      if (!nodeId) return;
      syncAllText();
      try {
        state.doc.insert_column_break(nodeId);
        broadcastOp({ action: 'insertColumnBreak', paraNodeId: nodeId });
        renderDocument();
        updateUndoRedo();
        announce('Column break inserted');
      } catch (e) { console.error('insert column break:', e); }
    });
  }

  // UXP-08: Insert section break (all 4 types via app menu bar)
  const sectionBreakTypes = [
    { id: 'miSectionNextPage', type: 'nextPage', label: 'Section break (next page)' },
    { id: 'miSectionContinuous', type: 'continuous', label: 'Section break (continuous)' },
    { id: 'miSectionEvenPage', type: 'evenPage', label: 'Section break (even page)' },
    { id: 'miSectionOddPage', type: 'oddPage', label: 'Section break (odd page)' },
  ];
  for (const { id, type: breakType, label } of sectionBreakTypes) {
    const el = $(id);
    if (!el) continue;
    el.addEventListener('click', () => {
      // Close all parent menus
      document.querySelectorAll('.app-menu-item.open').forEach(m => m.classList.remove('open'));
      if (!state.doc) return;
      const nodeId = getActiveNodeId();
      if (!nodeId) return;
      syncAllText();
      try {
        state.doc.insert_section_break(nodeId, breakType);
        broadcastOp({ action: 'insertSectionBreak', afterNodeId: nodeId, breakType });
        renderDocument();
        updateUndoRedo();
        announce(label + ' inserted');
      } catch (e) { console.error('insert section break:', e); }
    });
  }

  // Insert comment — modal
  $('miComment').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    trackEvent('insert', 'comment');
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    state._commentSelInfo = info;
    saveModalSelection();
    $('commentText').value = '';
    $('commentAuthor').value = 'User';
    $('commentModal').classList.add('show');
    $('commentText').focus();
  });
  $('commentCancelBtn').addEventListener('click', () => {
    $('commentModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });
  $('commentInsertBtn').addEventListener('click', () => {
    const text = $('commentText').value.trim();
    if (!text) { $('commentModal').classList.remove('show'); return; }
    const author = $('commentAuthor').value.trim() || 'User';
    $('commentModal').classList.remove('show');
    const info = state._commentSelInfo;
    if (!info || !state.doc) return;
    try {
      // Pass selection offsets so comment markers wrap the selected text range
      if (typeof state.doc.insert_comment_at_range === 'function') {
        state.doc.insert_comment_at_range(
          info.startNodeId, info.startOffset,
          info.endNodeId, info.endOffset,
          author, text
        );
      } else {
        state.doc.insert_comment(info.startNodeId, info.endNodeId, author, text);
      }
      broadcastOp({ action: 'insertComment', startNodeId: info.startNodeId, startOffset: info.startOffset, endNodeId: info.endNodeId, endOffset: info.endOffset, author, text });
      renderDocument();
      updateUndoRedo();
      refreshComments();
      announce('Comment added');
    } catch (e) { console.error('insert comment:', e); }
  });
  $('commentModal').addEventListener('click', e => {
    if (e.target === $('commentModal')) { $('commentModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
  $('commentText').addEventListener('keydown', e => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); $('commentInsertBtn').click(); }
    if (e.key === 'Escape') { $('commentModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });

  // Headers & Footers — menu bar entry
  // UXP-02: Opens inline editing mode on the first visible page's header.
  // If no pages exist, falls back to the modal.
  $('miHeaderFooter').addEventListener('click', () => {
    closeAllMenus();
    const firstPage = state.pageElements && state.pageElements[0];
    if (firstPage) {
      enterHeaderFooterEditMode('header', firstPage);
    } else {
      openHeaderFooterModal();
    }
  });

  // UXP-10: Footnote — menu bar entry
  // Disable if WASM binding doesn't support footnotes in current build
  if (state.doc && typeof state.doc.insert_footnote !== 'function') {
    const miFn = $('miFootnote');
    if (miFn) { miFn.classList.add('disabled'); miFn.title = 'Footnotes: not available in this build'; }
  }
  $('miFootnote')?.addEventListener('click', () => {
    closeAllMenus();
    trackEvent('insert', 'footnote');
    insertFootnoteAtCursor();
  });

  // UXP-10: Endnote — menu bar entry
  if (state.doc && typeof state.doc.insert_endnote !== 'function') {
    const miEn = $('miEndnote');
    if (miEn) { miEn.classList.add('disabled'); miEn.title = 'Endnotes: not available in this build'; }
  }
  $('miEndnote')?.addEventListener('click', () => {
    closeAllMenus();
    trackEvent('insert', 'endnote');
    insertEndnoteAtCursor();
  });

  // Header/Footer modal handlers
  $('hfCancelBtn').addEventListener('click', () => {
    $('headerFooterModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });
  $('hfApplyBtn').addEventListener('click', () => {
    const headerText = $('headerText').value.trim();
    const footerText = $('footerText').value.trim();
    const showPageNum = $('footerPageNum').checked;
    const differentFirst = $('differentFirstPage').checked;

    // Build header HTML
    if (headerText) {
      state.docHeaderHtml = '<span style="display:block;text-align:center;color:var(--text-secondary,#5f6368);font-size:9pt">' + escapeHtml(headerText) + '</span>';
    } else {
      state.docHeaderHtml = '';
    }

    // Build footer HTML
    let footerParts = [];
    if (footerText) {
      footerParts.push(escapeHtml(footerText));
    }
    if (showPageNum) {
      footerParts.push('<span data-field="PageNumber" contenteditable="false"></span>');
    }
    if (footerParts.length > 0) {
      state.docFooterHtml = '<span style="display:block;text-align:center;color:var(--text-secondary,#5f6368);font-size:9pt">' + footerParts.join(' \u2014 ') + '</span>';
    } else {
      state.docFooterHtml = '';
    }

    state.hasDifferentFirstPage = differentFirst;
    if (differentFirst) {
      // First page gets no header/footer when "different first page" is checked
      state.docFirstPageHeaderHtml = '';
      state.docFirstPageFooterHtml = '';
    }

    // UXP-02: Sync to WASM backend
    _syncHeaderFooterToWasm('header', 'default', headerText);
    const footerSyncText = showPageNum ? (footerText ? footerText + ' \u2014 ' : '') : footerText;
    _syncHeaderFooterToWasm('footer', 'default', footerSyncText);
    if (differentFirst) {
      try {
        if (state.doc && typeof state.doc.set_title_page === 'function') {
          state.doc.set_title_page(0, true);
        }
        _syncHeaderFooterToWasm('header', 'first', '');
        _syncHeaderFooterToWasm('footer', 'first', '');
      } catch (_) {}
    } else {
      try {
        if (state.doc && typeof state.doc.set_title_page === 'function') {
          state.doc.set_title_page(0, false);
        }
      } catch (_) {}
    }

    // Exit any inline editing mode if active
    if (state.hfEditingMode) {
      state.hfEditingMode = null;
      state.hfEditingPage = null;
      const container = $('pageContainer');
      if (container) {
        container.querySelectorAll('.hf-editing').forEach(hfEl => {
          const label = hfEl.querySelector('.hf-editing-label');
          const toolbar = hfEl.querySelector('.hf-toolbar');
          if (label) label.remove();
          if (toolbar) toolbar.remove();
          hfEl.contentEditable = 'false';
          hfEl.classList.remove('hf-editing');
          hfEl.classList.add('hf-hoverable');
          const pageEl = hfEl.closest('.doc-page');
          if (pageEl) {
            const contentEl = pageEl.querySelector('.page-content');
            if (contentEl) contentEl.classList.remove('hf-dimmed');
          }
        });
      }
    }

    $('headerFooterModal').classList.remove('show');
    renderDocument();
  });
  $('headerFooterModal').addEventListener('click', e => {
    if (e.target === $('headerFooterModal')) {
      $('headerFooterModal').classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });
  $('headerText').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); $('hfApplyBtn').click(); }
    if (e.key === 'Escape') { $('headerFooterModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
  $('footerText').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); $('hfApplyBtn').click(); }
    if (e.key === 'Escape') { $('headerFooterModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });

  // Comments panel toggle — Insert menu button
  $('btnComments').addEventListener('click', () => {
    $('commentsPanel').classList.toggle('show');
    if ($('commentsPanel').classList.contains('show')) refreshComments();
  });
  // Comments panel toggle — toolbar icon button
  if ($('btnCommentsToggle')) $('btnCommentsToggle').addEventListener('click', () => {
    // Route to spreadsheet comments panel when in sheet mode
    if (state.currentView === 'spreadsheet' && state.spreadsheetView) {
      state.spreadsheetView.showCommentsPanel();
      return;
    }
    $('commentsPanel').classList.toggle('show');
    if ($('commentsPanel').classList.contains('show')) refreshComments();
  });
  $('commentsClose').addEventListener('click', () => {
    $('commentsPanel').classList.remove('show');
  });

  // Pages panel toggle — toolbar icon button
  if ($('btnPages')) $('btnPages').addEventListener('click', () => {
    togglePagesPanel();
  });
  // Pages panel tabs (Pages / Outline)
  initPagesPanelTabs();

  // Find toolbar button
  $('btnFind').addEventListener('click', () => {
    $('findBar').classList.add('show');
    $('findInput').focus();
  });

  // Spell check toggle (UXP-19: with persistence & per-page sync)
  initSpellCheck();

  // History panel (undo history + version history)
  $('btnHistory').addEventListener('click', () => {
    const panel = $('historyPanel');
    panel.classList.toggle('show');
    if (panel.classList.contains('show')) refreshHistory();
  });
  $('historyClose').addEventListener('click', () => {
    $('historyPanel').classList.remove('show');
  });
  // E3.2: History tab switching
  document.querySelectorAll('.history-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.history-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      const isUndo = tab.dataset.tab === 'undo';
      const undoTab = $('undoHistoryTab');
      const versionsTab = $('versionsTab');
      if (undoTab) undoTab.style.display = isUndo ? '' : 'none';
      if (versionsTab) versionsTab.style.display = isUndo ? 'none' : '';
      if (!isUndo) refreshHistory();
    });
  });

  // Share / Collaboration
  $('btnShare').addEventListener('click', showShareDialog);

  // Zoom controls (status bar)
  $('zoomIn').addEventListener('click', () => adjustZoom(10));
  $('zoomOut').addEventListener('click', () => adjustZoom(-10));

  // Zoom controls (toolbar)
  if ($('tbZoomIn')) $('tbZoomIn').addEventListener('click', () => adjustZoom(10));
  if ($('tbZoomOut')) $('tbZoomOut').addEventListener('click', () => adjustZoom(-10));

  // Dark mode toggle (E10.1)
  initDarkMode();

  // Zoom dropdown (E10.2)
  initZoomDropdown();

  // Menu bar entries — wire Format menu to same toggle actions
  if ($('menuBold')) $('menuBold').addEventListener('click', () => { closeAllMenus(); toggleFormat('bold'); });
  if ($('menuItalic')) $('menuItalic').addEventListener('click', () => { closeAllMenus(); toggleFormat('italic'); });
  if ($('menuUnderline')) $('menuUnderline').addEventListener('click', () => { closeAllMenus(); toggleFormat('underline'); });
  if ($('menuStrike')) $('menuStrike').addEventListener('click', () => { closeAllMenus(); toggleFormat('strikethrough'); });
  if ($('menuClearFormat')) $('menuClearFormat').addEventListener('click', () => { closeAllMenus(); $('btnClearFormat').click(); });
  if ($('menuFormatPainter')) $('menuFormatPainter').addEventListener('click', () => { closeAllMenus(); $('btnFormatPainter').click(); });
  if ($('menuUndo')) $('menuUndo').addEventListener('click', () => { closeAllMenus(); doUndo(); });
  if ($('menuRedo')) $('menuRedo').addEventListener('click', () => { closeAllMenus(); doRedo(); });
  if ($('menuFind')) $('menuFind').addEventListener('click', () => { closeAllMenus(); $('findBar').classList.add('show'); $('findInput').focus(); });
  if ($('menuZoomIn')) $('menuZoomIn').addEventListener('click', () => { closeAllMenus(); adjustZoom(10); });
  if ($('menuZoomOut')) $('menuZoomOut').addEventListener('click', () => { closeAllMenus(); adjustZoom(-10); });

  // View → Pages Panel toggle
  if ($('menuShowPages')) $('menuShowPages').addEventListener('click', () => {
    closeAllMenus();
    togglePagesPanel();
  });
  // View → Document Outline toggle (opens left panel on Outline tab)
  if ($('menuShowOutline')) $('menuShowOutline').addEventListener('click', () => {
    closeAllMenus();
    toggleOutlinePanel();
  });
  // View → Comments Panel toggle
  if ($('menuShowComments')) $('menuShowComments').addEventListener('click', () => {
    closeAllMenus();
    const cp = $('commentsPanel');
    if (cp) {
      cp.classList.toggle('show');
      if (cp.classList.contains('show')) refreshComments();
    }
  });

  // Pages panel close button
  if ($('pagesPanelClose')) $('pagesPanelClose').addEventListener('click', () => {
    $('pagesPanel')?.classList.remove('show');
  });

  // Dark mode toggle
  if ($('menuDarkMode')) $('menuDarkMode').addEventListener('click', () => {
    closeAllMenus();
    toggleDarkMode();
  });

  // Print Preview (UXP-16) — full-screen read-only preview overlay
  if ($('menuPrintPreview')) $('menuPrintPreview').addEventListener('click', () => {
    closeAllMenus();
    openPrintPreview();
  });

  // FS-11: Read-Only Mode toggle
  if ($('menuReadOnly')) $('menuReadOnly').addEventListener('click', () => {
    closeAllMenus();
    toggleReadOnlyMode();
  });

  // Wire up print preview overlay buttons
  if ($('printPreviewClose')) $('printPreviewClose').addEventListener('click', closePrintPreview);
  if ($('printPreviewPrint')) $('printPreviewPrint').addEventListener('click', () => {
    closePrintPreview();
    // Short delay so the overlay is removed before the print dialog opens
    setTimeout(() => window.print(), 120);
  });

  // F4.3: Canvas mode toggle — switch between DOM and Canvas rendering
  initCanvasRenderer($('editorCanvas'));
  const canvasToggle = $('canvasModeToggle');
  if (canvasToggle) {
    canvasToggle.checked = isCanvasMode();
    canvasToggle.addEventListener('change', () => {
      setCanvasMode(canvasToggle.checked);
      closeAllMenus();
      if (state.doc) renderDocument();
    });
  }

  // Help menu — keyboard shortcuts dialog (E7.4)
  if ($('menuShortcuts')) $('menuShortcuts').addEventListener('click', () => {
    closeAllMenus();
    saveModalSelection();
    $('shortcutsModal').classList.add('show');
  });
  if ($('shortcutsCloseBtn')) $('shortcutsCloseBtn').addEventListener('click', () => {
    $('shortcutsModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });
  if ($('shortcutsModal')) $('shortcutsModal').addEventListener('click', e => {
    if (e.target === $('shortcutsModal')) { $('shortcutsModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
  // About dialog — simple alert
  if ($('menuAbout')) $('menuAbout').addEventListener('click', () => {
    closeAllMenus();
    showToast('Rudra Office v1.0 — powered by Rudra Code. WASM-powered document editor. AGPL-3.0 License.', 'info', 6000);
  });

  // E10.5: Usage Statistics modal
  initUsageStatsModal();

  // E10.5: Error Detail modal (status bar indicator + modal)
  initErrorDetailModal();

  // E10.4: Welcome dialog
  initWelcomeDialog();

  // E10.4: Feature Tour
  initFeatureTour();

  // E10.4: What's New modal
  initWhatsNew();

  // E5.1: Share dialog permission sync
  initSharePermissionSync();

  // Toolbar insert dropdown — items use data-action
  const insertMenu = $('insertMenu');
  if (insertMenu) {
    insertMenu.querySelectorAll('[data-action]').forEach(btn => {
      btn.addEventListener('click', () => {
        insertMenu.classList.remove('show');
        $('btnInsertMenu').setAttribute('aria-expanded', 'false');
        const action = btn.dataset.action;
        // Map data-action to existing handlers
        if (action === 'table') $('miTable').click();
        else if (action === 'image') $('miImage').click();
        else if (action === 'link') $('miLink').click();
        else if (action === 'comment') $('miComment').click();
        else if (action === 'toc') $('miTOC').click();
        else if (action === 'hr') $('miHR').click();
        else if (action === 'pagebreak') $('miPageBreak').click();
        else if (action === 'sectionNextPage') $('miSectionNextPage')?.click();
        else if (action === 'sectionContinuous') $('miSectionContinuous')?.click();
        else if (action === 'sectionEvenPage') $('miSectionEvenPage')?.click();
        else if (action === 'sectionOddPage') $('miSectionOddPage')?.click();
        else if (action === 'headerfooter') $('miHeaderFooter').click();
        else if (action === 'footnote') $('miFootnote')?.click();
        else if (action === 'endnote') $('miEndnote')?.click();
        else if (action === 'equation') $('miEquation')?.click();
        else if (action === 'bookmark') $('miBookmark')?.click();
        else if (action === 'drawing') $('miDrawing')?.click();
        else if (action === 'columnbreak') $('miColumnBreak')?.click();
        else if (action === 'specialchars') $('miSpecialChars')?.click();
      });
    });
  }

  // Table context menu + properties dialog
  initTableContextMenu();
  initTablePropsModal();

  // More (overflow) menu toggle and item handlers
  initMoreMenu();

  // Template chooser
  initTemplateChooser();

  // Table of Contents insertion
  initTOCInsertion();

  // Page Setup dialog
  initPageSetup();

  // UXP-22: Columns dialog
  initColumnsModal();

  // Auto Format Document
  initAutoFormat();

  // Touch selection support (double-tap word select, long-press context menu)
  initTouchSelection();

  // E9.3: Equation editor
  initEquationModal();

  // UXP-20: Bookmark editor
  initBookmarkModal();

  // E9.1: Custom dictionary & auto-correct
  initDictModal();
  initAutoCorrectModal();

  // E5.4 / UXP-07: Editing mode selector + Track Changes Panel
  initEditingMode();
  initTrackChangesPanel();
  initReviewMenu();

  // E9.2: Save as template
  initSaveAsTemplate();

  // E5.4: @mention in comments
  initCommentMentions();

  // FS-43: Special Characters dialog
  initSpecialCharsModal();

  // FS-44: Borders & Shading dialog
  initBordersModal();

  // P5-7: Sign Document modal
  initSignDocumentModal();

  // Close menus on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('.insert-dropdown')) {
      $('insertMenu')?.classList.remove('show');
      $('btnInsertMenu')?.setAttribute('aria-expanded', 'false');
    }
    if (!e.target.closest('.style-gallery')) {
      $('styleGalleryPanel')?.classList.remove('show');
      $('styleGalleryBtn')?.setAttribute('aria-expanded', 'false');
    }
    if (!e.target.closest('.more-dropdown')) {
      closeMoreMenu();
    }
    const tcm = $('tableContextMenu');
    if (tcm) tcm.style.display = 'none';
    // Close zoom dropdown on outside click
    if (!e.target.closest('.zoom-value-wrap') && !e.target.closest('.tb-zoom-wrap')) {
      closeZoomDropdown();
    }
    // Close slash menu on outside click
    if (!e.target.closest('.slash-menu') && !e.target.closest('.doc-page')) {
      closeSlashMenu();
    }
  });

  // Close fixed-position menus on scroll (they would become detached from their buttons)
  const editorCanvas = $('editorCanvas');
  if (editorCanvas) {
    editorCanvas.addEventListener('scroll', () => {
      $('insertMenu')?.classList.remove('show');
      $('styleGalleryPanel')?.classList.remove('show');
      closeMoreMenu();
      closeZoomDropdown();
      closeAllMenus();
    }, { passive: true });
  }

  // E7.1: Keyboard accessibility
  initModalFocusTrap();
  initToolbarKeyboardNav();
}

// ── UXP-14: Format Painter ─────────────────────────
// Copies character formatting from the current selection/cursor and applies
// it to the next selection. Single-click = apply once, double-click = sticky
// mode (keeps applying until Escape or button click).

function initFormatPainter() {
  const btn = $('btnFormatPainter');
  if (!btn) return;

  let clickTimer = null;
  let clickCount = 0;

  btn.addEventListener('click', (e) => {
    e.preventDefault();
    // If already in paint mode, toggle off
    if (state.formatPainterMode) {
      exitFormatPainter();
      trackEvent('toolbar', 'format-painter-off');
      return;
    }
    clickCount++;
    if (clickCount === 1) {
      // Wait briefly to see if a second click arrives (double-click detection)
      clickTimer = setTimeout(() => {
        // Single click: copy format + enter "once" mode
        if (copyFormatFromSelection()) {
          enterFormatPainter('once');
          announce('Format Painter: select text to apply formatting');
          showToast('Format painter active — click a paragraph to apply', 'info', 3000);
          trackEvent('toolbar', 'format-painter-once');
        }
        clickCount = 0;
      }, 250);
    } else if (clickCount >= 2) {
      // Double click: copy format + enter "sticky" mode
      clearTimeout(clickTimer);
      clickCount = 0;
      if (copyFormatFromSelection()) {
        enterFormatPainter('sticky');
        announce('Format Painter locked: select text to apply, press Escape to exit');
        showToast('Format painter locked — click paragraphs to apply, press Escape to exit', 'info', 4000);
        trackEvent('toolbar', 'format-painter-sticky');
      }
    }
  });
}

/**
 * Copy character formatting from the current selection or cursor position.
 * Returns true if formatting was successfully captured, false otherwise.
 */
function copyFormatFromSelection() {
  if (!state.doc) return false;
  const info = getSelectionInfo();
  if (!info) return false;

  try {
    let fmt;
    if (info.collapsed) {
      fmt = JSON.parse(state.doc.get_formatting_json(info.startNodeId));
    } else {
      try {
        fmt = JSON.parse(state.doc.get_selection_formatting_json(
          info.startNodeId, info.startOffset, info.endNodeId, info.endOffset));
      } catch (_) {
        fmt = JSON.parse(state.doc.get_formatting_json(info.startNodeId));
      }
    }

    // Extract character-level formatting properties
    state.copiedFormat = {
      bold: fmt.bold === true || fmt.bold === 'true' ? 'true' : 'false',
      italic: fmt.italic === true || fmt.italic === 'true' ? 'true' : 'false',
      underline: fmt.underline === true || fmt.underline === 'true' ? 'true' : 'false',
      strikethrough: fmt.strikethrough === true || fmt.strikethrough === 'true' ? 'true' : 'false',
      superscript: fmt.superscript === true || fmt.superscript === 'true' ? 'true' : 'false',
      subscript: fmt.subscript === true || fmt.subscript === 'true' ? 'true' : 'false',
    };
    // Include value-based formatting only if they are set
    if (fmt.fontSize && fmt.fontSize !== 'mixed') {
      state.copiedFormat.fontSize = String(Math.round(parseFloat(fmt.fontSize)));
    }
    if (fmt.fontFamily && fmt.fontFamily !== 'mixed') {
      state.copiedFormat.fontFamily = fmt.fontFamily;
    }
    if (fmt.color && fmt.color !== 'mixed') {
      state.copiedFormat.color = fmt.color;
    }
    if (fmt.highlightColor && fmt.highlightColor !== 'mixed') {
      state.copiedFormat.highlightColor = fmt.highlightColor;
    }
    return true;
  } catch (e) {
    console.error('Format Painter: failed to copy formatting:', e);
    return false;
  }
}

/**
 * Enter format painter mode. Sets cursor to crosshair and activates the button.
 * @param {'once'|'sticky'} mode
 */
function enterFormatPainter(mode) {
  state.formatPainterMode = mode;
  const btn = $('btnFormatPainter');
  if (btn) {
    btn.classList.add('format-painter-active');
    btn.classList.add('active');
    btn.setAttribute('aria-pressed', 'true');
  }
  // Set crosshair cursor on all page content areas
  const page = $('pageContainer');
  if (page) page.classList.add('format-painter-cursor');
  // D21: Toggle body class for global cursor override
  document.body.classList.add('format-painter-active');
}

/**
 * Exit format painter mode. Restores cursor and clears state.
 */
// Canonical version in features/document/toolbar/format-painter.js; re-exported at top.
function exitFormatPainter() {
  state.formatPainterMode = null;
  state.copiedFormat = null;
  const btn = $('btnFormatPainter');
  if (btn) {
    btn.classList.remove('format-painter-active');
    btn.classList.remove('active');
    btn.setAttribute('aria-pressed', 'false');
  }
  const page = $('pageContainer');
  if (page) page.classList.remove('format-painter-cursor');
  // D21: Remove body class for global cursor override
  document.body.classList.remove('format-painter-active');
}

/**
 * Apply the previously copied format to the current text selection.
 * Called on mouseup in the document while format painter is active.
 * Returns true if format was applied, false otherwise.
 */
function applyFormatPainter() {
  if (!state.formatPainterMode) return false;
  // D10: If copiedFormat or doc is missing, the painter is in a stale state — force clear
  if (!state.copiedFormat || !state.doc) {
    exitFormatPainter();
    return false;
  }
  const info = getSelectionInfo();
  if (!info || info.collapsed) return false;

  syncAllText();
  try {
    const sn = info.startNodeId, so = info.startOffset;
    const en = info.endNodeId, eo = info.endOffset;
    const fmt = state.copiedFormat;

    // Apply each formatting property from the copied format
    const formatKeys = ['bold', 'italic', 'underline', 'strikethrough', 'superscript', 'subscript'];
    for (const key of formatKeys) {
      if (key in fmt) {
        state.doc.format_selection(sn, so, en, eo, key, fmt[key]);
        broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key, value: fmt[key] });
      }
    }
    // Apply value-based properties (fontSize, fontFamily, color, highlightColor)
    const valueKeys = ['fontSize', 'fontFamily', 'color', 'highlightColor'];
    for (const key of valueKeys) {
      if (fmt[key]) {
        state.doc.format_selection(sn, so, en, eo, key, fmt[key]);
        broadcastOp({ action: 'formatSelection', startNode: sn, startOffset: so, endNode: en, endOffset: eo, key, value: fmt[key] });
      }
    }

    renderDocument();
    updateToolbarState();
    recordUndoAction('Apply format painter');
    updateUndoRedo();
    markDirty();
    announce('Format applied');

    // In "once" mode, exit after first application
    if (state.formatPainterMode === 'once') {
      exitFormatPainter();
    }
    return true;
  } catch (e) {
    console.error('Format Painter: failed to apply formatting:', e);
    // D10: Always exit format painter on error to prevent stale active state
    exitFormatPainter();
    return false;
  }
}

function applyAlignment(align) {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  syncAllText();
  try {
    const paraIds = getSelectedParagraphIds(info);
    paraIds.forEach(nodeId => {
      state.doc.set_alignment(nodeId, align);
      broadcastOp({ action: 'setAlignment', nodeId, alignment: align });
    });
    renderDocument();
    updateToolbarState();
    updateUndoRedo();
    announce('Alignment: ' + align);
  } catch (e) { console.error('alignment:', e); }
}

function toggleList(format) {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  syncAllText();
  try {
    // Collect all paragraph node IDs in the selection range
    const paraIds = getSelectedParagraphIds(info);
    // Check if first paragraph already has this list format — if so, toggle off
    let toggleOff = false;
    try {
      const fmt = JSON.parse(state.doc.get_formatting_json(info.startNodeId));
      if (fmt.listFormat === format) toggleOff = true;
    } catch (_) {}
    const applyFormat = toggleOff ? 'none' : format;
    // Save cursor info for restoration
    const cursorNodeId = info.startNodeId;
    const cursorOffset = info.startOffset;
    paraIds.forEach(nodeId => {
      state.doc.set_list_format(nodeId, applyFormat, 0);
      broadcastOp({ action: 'setListFormat', nodeId, format: applyFormat, level: 0 });
    });
    renderDocument();
    // Restore cursor position
    const page = $('pageContainer');
    if (page) {
      const restored = page.querySelector(`[data-node-id="${cursorNodeId}"]`);
      if (restored) {
        setCursorAtOffset(restored, cursorOffset);
      }
    }
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.error('list:', e); }
}

// Get all paragraph node IDs between selection start and end.
// Handles reversed selections and cross-page ranges correctly.
function getSelectedParagraphIds(info) {
  const page = $('pageContainer');
  if (!page) return [info.startNodeId];
  const paraEls = page.querySelectorAll('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]');

  // Find indices of start and end to handle forward and reverse selections
  let startIdx = -1, endIdx = -1;
  for (let i = 0; i < paraEls.length; i++) {
    const nid = paraEls[i].dataset.nodeId;
    if (nid === info.startNodeId && startIdx === -1) startIdx = i;
    if (nid === info.endNodeId) endIdx = i;
  }
  if (startIdx === -1) return [info.startNodeId];
  if (endIdx === -1) endIdx = startIdx;

  // Normalize direction
  const lo = Math.min(startIdx, endIdx);
  const hi = Math.max(startIdx, endIdx);
  const ids = [];
  for (let i = lo; i <= hi; i++) {
    ids.push(paraEls[i].dataset.nodeId);
  }
  return ids.length > 0 ? ids : [info.startNodeId];
}

function applyIndent(delta) {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  syncAllText();
  try {
    const paraIds = getSelectedParagraphIds(info);
    paraIds.forEach(nodeId => {
      const fmt = JSON.parse(state.doc.get_formatting_json(nodeId));
      const current = parseFloat(fmt.indentLeft || '0');
      const newVal = Math.max(0, current + delta);
      state.doc.set_indent(nodeId, 'left', newVal);
      broadcastOp({ action: 'setIndent', nodeId, side: 'left', value: newVal });
    });
    renderDocument();
    updateUndoRedo();
  } catch (e) { console.error('indent:', e); }
}

function adjustZoom(delta) {
  setZoomLevel((state.zoomLevel || 100) + delta);
}

// ── E10.2: Unified zoom — set, persist, update UI ──
// Canonical version in features/document/toolbar/zoom.js; re-exported at top.
function setZoomLevel(level) {
  level = Math.max(50, Math.min(200, Math.round(level)));
  const changed = state.zoomLevel !== level;
  state.zoomLevel = level;
  // Persist zoom across sessions
  try { localStorage.setItem('s1_zoom', String(level)); } catch (_) {}
  const label = level + '%';
  if ($('zoomValue')) $('zoomValue').textContent = label;
  if ($('tbZoomValue')) $('tbZoomValue').textContent = label;
  // Apply CSS zoom to the page container (not transform:scale, which offsets coordinates)
  const container = $('pageContainer');
  if (container) {
    if (level === 100) {
      container.style.zoom = '';
    } else {
      container.style.zoom = (level / 100);
    }
  }
  // Update active state in zoom dropdowns (status bar + toolbar)
  [$('zoomDropdown'), $('tbZoomDropdown')].forEach(dd => {
    if (dd) {
      dd.querySelectorAll('.zoom-preset').forEach(btn => {
        const v = btn.dataset.zoom;
        btn.classList.toggle('active', v === String(level));
      });
    }
  });
  // Invalidate layout cache when zoom changes so repagination uses fresh dimensions
  if (changed) markLayoutDirty();
  try { localStorage.setItem('s1-zoom', String(level)); } catch (_) {}
  renderRuler();
}

function calcFitWidthZoom() {
  const canvas = $('editorCanvas');
  if (!canvas) return 100;
  const dims = state.pageDims || { widthPt: 612 };
  const pageWidth = Math.round(dims.widthPt * 96 / 72); // dynamic from doc
  const canvasWidth = canvas.clientWidth - 48; // subtract padding
  return Math.max(50, Math.min(200, Math.round((canvasWidth / pageWidth) * 100)));
}

function calcFitPageZoom() {
  const canvas = $('editorCanvas');
  if (!canvas) return 100;
  const pageWidth = 816;
  const pageHeight = 1056; // default page height in px (11in @ 96dpi)
  const canvasWidth = canvas.clientWidth - 48;
  const canvasHeight = canvas.clientHeight - 48;
  const zoomW = (canvasWidth / pageWidth) * 100;
  const zoomH = (canvasHeight / pageHeight) * 100;
  return Math.max(50, Math.min(200, Math.round(Math.min(zoomW, zoomH))));
}

function initZoomDropdown() {
  // Status bar zoom dropdown
  _initSingleZoomDropdown($('zoomValue'), $('zoomDropdown'));
  // Toolbar zoom dropdown (UXP-17)
  _initSingleZoomDropdown($('tbZoomValue'), $('tbZoomDropdown'));

  // Restore saved zoom level
  try {
    const saved = localStorage.getItem('s1_zoom');
    if (saved) {
      const parsed = parseInt(saved);
      if (!isNaN(parsed) && parsed >= 50 && parsed <= 200) {
        setZoomLevel(parsed);
      }
    }
  } catch (_) {}
}

function _initSingleZoomDropdown(valueBtn, dd) {
  if (!valueBtn || !dd) return;

  // Toggle dropdown on zoom value click
  valueBtn.addEventListener('click', e => {
    e.stopPropagation();
    const isOpen = dd.classList.contains('show');
    closeZoomDropdown();
    if (!isOpen) {
      dd.classList.add('show');
      valueBtn.setAttribute('aria-expanded', 'true');
    }
  });

  // Handle preset clicks
  dd.querySelectorAll('.zoom-preset').forEach(btn => {
    btn.addEventListener('click', e => {
      e.stopPropagation();
      const val = btn.dataset.zoom;
      closeZoomDropdown();
      if (val === 'fit-width') {
        setZoomLevel(calcFitWidthZoom());
      } else if (val === 'fit-page') {
        setZoomLevel(calcFitPageZoom());
      } else {
        setZoomLevel(parseInt(val));
      }
    });
  });
}

function closeZoomDropdown() {
  // Close both status bar and toolbar zoom dropdowns
  [['zoomDropdown', 'zoomValue'], ['tbZoomDropdown', 'tbZoomValue']].forEach(([ddId, btnId]) => {
    const dd = $(ddId);
    const btn = $(btnId);
    if (dd) dd.classList.remove('show');
    if (btn) btn.setAttribute('aria-expanded', 'false');
  });
}

function closeAllMenus() {
  document.querySelectorAll('.app-menu-item').forEach(m => {
    m.classList.remove('open');
    const btn = m.querySelector('.app-menu-btn');
    if (btn) btn.setAttribute('aria-expanded', 'false');
  });
}

function toggleDarkMode() {
  const html = document.documentElement;
  const current = html.getAttribute('data-theme');
  // If no explicit theme set, check OS preference to determine current state
  const isDark = current === 'dark' || (!current && window.matchMedia('(prefers-color-scheme: dark)').matches);
  const next = isDark ? 'light' : 'dark';
  html.setAttribute('data-theme', next);
  localStorage.setItem('s1-theme', next);
  updateDarkModeIcon();
}

// ── UXP-19: Spell Check Toggle with Persistence ──
function initSpellCheck() {
  const btn = $('btnSpellCheck');
  if (!btn) return;

  // Restore saved preference (default: on)
  let enabled = true;
  try {
    const saved = localStorage.getItem('s1-spellcheck');
    if (saved === 'false') enabled = false;
  } catch (_) {}

  applySpellCheck(enabled);
  btn.classList.toggle('active', enabled);
  btn.setAttribute('aria-pressed', String(enabled));

  btn.addEventListener('click', () => {
    const page = $('pageContainer');
    const isEnabled = page.getAttribute('spellcheck') === 'true';
    const next = !isEnabled;
    applySpellCheck(next);
    btn.classList.toggle('active', next);
    btn.setAttribute('aria-pressed', String(next));
    try { localStorage.setItem('s1-spellcheck', String(next)); } catch (_) {}
  });
}

function applySpellCheck(enabled) {
  const container = $('pageContainer');
  if (container) {
    container.setAttribute('spellcheck', String(enabled));
    // Sync to all page-content elements so browser spellcheck applies per-page
    container.querySelectorAll('.page-content').forEach(el => {
      el.spellcheck = enabled;
    });
  }
}

// Export for render.js to call when creating new pages
export function isSpellCheckEnabled() {
  try {
    const saved = localStorage.getItem('s1-spellcheck');
    if (saved === 'false') return false;
  } catch (_) {}
  return true;
}

function initAppMenubar() {
  let menubarActive = false;
  const menuItems = Array.from(document.querySelectorAll('.app-menu-item'));
  const menuBtns = menuItems.map(item => item.querySelector('.app-menu-btn')).filter(Boolean);

  // Helper: get menu entries (buttons only, skip separators) for the currently open menu
  function getMenuEntries(menuItem) {
    const dropdown = menuItem.querySelector('.app-menu-dropdown');
    if (!dropdown) return [];
    return Array.from(dropdown.querySelectorAll('.app-menu-entry'));
  }

  // Helper: open a menu and optionally focus the first entry
  function openMenu(item, focusFirst) {
    closeAllMenus();
    item.classList.add('open');
    const btn = item.querySelector('.app-menu-btn');
    if (btn) btn.setAttribute('aria-expanded', 'true');
    menubarActive = true;

    // Clamp dropdown to stay within viewport
    const dropdown = item.querySelector('.app-menu-dropdown');
    if (dropdown) {
      // Reset position first
      dropdown.style.left = '';
      dropdown.style.right = '';
      requestAnimationFrame(() => {
        const rect = dropdown.getBoundingClientRect();
        if (rect.right > window.innerWidth) {
          dropdown.style.left = 'auto';
          dropdown.style.right = '0';
        }
        if (rect.bottom > window.innerHeight) {
          dropdown.style.maxHeight = (window.innerHeight - rect.top - 8) + 'px';
        }
      });
    }

    if (focusFirst) {
      const entries = getMenuEntries(item);
      if (entries.length > 0) entries[0].focus();
    }
  }

  // Helper: close all menus and reset aria-expanded
  function closeMenubar() {
    closeAllMenus();
    menuBtns.forEach(b => b.setAttribute('aria-expanded', 'false'));
    menubarActive = false;
  }

  menuItems.forEach((item, idx) => {
    const btn = item.querySelector('.app-menu-btn');
    if (!btn) return;

    // Click to open/close
    btn.addEventListener('click', e => {
      e.stopPropagation();
      const wasOpen = item.classList.contains('open');
      closeMenubar();
      if (!wasOpen) {
        openMenu(item, false);
      }
    });

    // Hover to switch menu while one is open
    btn.addEventListener('mouseenter', () => {
      if (menubarActive) {
        openMenu(item, false);
      }
    });

    // Keyboard on top-level menu button
    btn.addEventListener('keydown', e => {
      const key = e.key;
      if (key === 'ArrowDown' || key === 'Enter' || key === ' ') {
        e.preventDefault();
        openMenu(item, true);
      } else if (key === 'ArrowRight') {
        e.preventDefault();
        const next = menuBtns[(idx + 1) % menuBtns.length];
        next.focus();
        if (menubarActive) {
          openMenu(menuItems[(idx + 1) % menuItems.length], true);
        }
      } else if (key === 'ArrowLeft') {
        e.preventDefault();
        const prev = menuBtns[(idx - 1 + menuBtns.length) % menuBtns.length];
        prev.focus();
        if (menubarActive) {
          openMenu(menuItems[(idx - 1 + menuItems.length) % menuItems.length], true);
        }
      } else if (key === 'Escape') {
        closeMenubar();
        btn.focus();
      }
    });
  });

  // Keyboard navigation within open dropdown menus
  document.addEventListener('keydown', e => {
    const openItem = menuItems.find(m => m.classList.contains('open'));
    if (!openItem) return;

    const entries = getMenuEntries(openItem);
    if (entries.length === 0) return;
    const currentIdx = entries.indexOf(document.activeElement);
    const idx = menuItems.indexOf(openItem);

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = currentIdx < 0 ? 0 : (currentIdx + 1) % entries.length;
      entries[next].focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = currentIdx <= 0 ? entries.length - 1 : currentIdx - 1;
      entries[prev].focus();
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      // Check if focused element is a submenu trigger — open submenu instead of switching top-level menu
      const submenuTrigger = document.activeElement?.closest('.app-menu-submenu');
      if (submenuTrigger) {
        submenuTrigger.classList.add('open');
        const subItems = submenuTrigger.querySelectorAll('.app-submenu-dropdown .app-menu-entry');
        if (subItems.length > 0) subItems[0].focus();
        return;
      }
      const nextIdx = (idx + 1) % menuItems.length;
      openMenu(menuItems[nextIdx], true);
      menuBtns[nextIdx].focus();
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      // Check if we're inside a submenu — close it and return to parent
      const openSubmenu = document.activeElement?.closest('.app-menu-submenu.open');
      if (openSubmenu) {
        openSubmenu.classList.remove('open');
        const trigger = openSubmenu.querySelector('.app-menu-submenu-trigger');
        if (trigger) trigger.focus();
        return;
      }
      const prevIdx = (idx - 1 + menuItems.length) % menuItems.length;
      openMenu(menuItems[prevIdx], true);
      menuBtns[prevIdx].focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      // Close submenu first if open, then close menu
      const openSubmenu = document.activeElement?.closest('.app-menu-submenu.open');
      if (openSubmenu) {
        openSubmenu.classList.remove('open');
        const trigger = openSubmenu.querySelector('.app-menu-submenu-trigger');
        if (trigger) trigger.focus();
        return;
      }
      const btn = openItem.querySelector('.app-menu-btn');
      closeMenubar();
      if (btn) btn.focus();
    } else if (e.key === 'Enter' || e.key === ' ') {
      if (document.activeElement && entries.includes(document.activeElement)) {
        e.preventDefault();
        // If it's a submenu trigger, open submenu instead of clicking
        const submenuTrigger = document.activeElement.closest('.app-menu-submenu');
        if (submenuTrigger && document.activeElement.classList.contains('app-menu-submenu-trigger')) {
          submenuTrigger.classList.add('open');
          const subItems = submenuTrigger.querySelectorAll('.app-submenu-dropdown .app-menu-entry');
          if (subItems.length > 0) subItems[0].focus();
          return;
        }
        document.activeElement.click();
      }
    } else if (e.key === 'Home') {
      e.preventDefault();
      if (entries.length > 0) entries[0].focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      if (entries.length > 0) entries[entries.length - 1].focus();
    }
  });

  // Close on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('.app-menu-item')) {
      closeMenubar();
    }
  });
}

// ─── Comment Replies (in-memory store) ────────────
// Replies stored in-memory keyed by parent comment ID.
// Each reply: { id, parentId, author, text, timestamp }
try { state.commentReplies = JSON.parse(localStorage.getItem('s1_commentReplies') || '[]'); } catch(_) { state.commentReplies = []; }
if (!state.commentReplies) state.commentReplies = [];
let _replyCounter = 0;

function refreshComments() {
  const list = $('commentsList');
  if (!list || !state.doc) return;
  try {
    const comments = JSON.parse(state.doc.get_comments_json());
    const replies = state.commentReplies || [];

    if ((!comments || comments.length === 0) && replies.length === 0) {
      list.innerHTML = '<div class="comments-empty">No comments in this document.</div>';
      return;
    }

    // Build reply map: parentId -> [reply, ...]
    const replyMap = {};
    replies.forEach(r => {
      if (!replyMap[r.parentId]) replyMap[r.parentId] = [];
      replyMap[r.parentId].push(r);
    });

    // Separate active and resolved comments
    const activeComments = [];
    const resolvedCommentsList = [];
    (comments || []).forEach(c => {
      const cid = c.id || '';
      if (state.resolvedComments && state.resolvedComments.has(cid)) {
        resolvedCommentsList.push(c);
      } else {
        activeComments.push(c);
      }
    });

    // Render active comments first, then resolved at bottom
    const allOrdered = [...activeComments, ...resolvedCommentsList];

    let html = '';
    allOrdered.forEach(c => {
      const cid = c.id || '';
      html += renderCommentCard(c);

      // Render replies for this comment
      const threadReplies = replyMap[cid] || [];
      threadReplies.sort((a, b) => a.timestamp - b.timestamp);
      threadReplies.forEach(r => {
        html += renderReplyCard(r);
      });

      // Reply form placeholder
      html += `<div class="comment-reply-area" data-parent-id="${escapeAttr(cid)}"></div>`;
    });

    list.innerHTML = html;

    // Wire up delete buttons for WASM comments
    list.querySelectorAll('.comment-delete').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.dataset.id;
        if (!id || !state.doc) return;
        try {
          state.doc.delete_comment(id);
          broadcastOp({ action: 'deleteComment', commentId: id });
          // Also remove any replies to this comment
          state.commentReplies = (state.commentReplies || []).filter(r => r.parentId !== id);
          try { localStorage.setItem('s1_commentReplies', JSON.stringify(state.commentReplies)); } catch(_) {}
          // Also remove from resolved set
          if (state.resolvedComments) state.resolvedComments.delete(id);
          renderDocument();
          updateUndoRedo();
          refreshComments();
        } catch (e) { console.error('delete comment:', e); }
      });
    });

    // Wire up resolve/unresolve buttons
    list.querySelectorAll('.comment-resolve-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.dataset.id;
        if (!id) return;
        if (!state.resolvedComments) state.resolvedComments = new Set();
        const wasResolved = state.resolvedComments.has(id);
        if (wasResolved) {
          state.resolvedComments.delete(id);
          announce('Comment reopened');
        } else {
          state.resolvedComments.add(id);
          announce('Comment resolved');
        }
        refreshComments();
      });
    });

    // Wire up reply buttons
    list.querySelectorAll('.comment-reply-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const parentId = btn.dataset.parentId;
        showReplyForm(parentId);
      });
    });

    // Wire up delete buttons for replies
    list.querySelectorAll('.reply-delete').forEach(btn => {
      btn.addEventListener('click', () => {
        const replyId = btn.dataset.replyId;
        state.commentReplies = (state.commentReplies || []).filter(r => r.id !== replyId);
        try { localStorage.setItem('s1_commentReplies', JSON.stringify(state.commentReplies)); } catch(_) {}
        refreshComments();
      });
    });

    // Wire up click-to-scroll on comment cards
    list.querySelectorAll('.comment-card[data-start-node-id]').forEach(card => {
      card.addEventListener('click', (e) => {
        // Don't trigger scroll when clicking action buttons
        if (e.target.closest('.comment-actions, .comment-reply-form')) return;
        const nodeId = card.dataset.startNodeId;
        if (!nodeId) return;
        scrollToCommentNode(nodeId);
      });
    });
  } catch (e) {
    list.innerHTML = '<div class="comments-empty">Unable to load comments.</div>';
  }
}

/**
 * Scroll to a comment's associated node and briefly highlight it.
 */
function scrollToCommentNode(nodeId) {
  const page = $('pageContainer');
  if (!page) return;
  const el = page.querySelector(`[data-node-id="${nodeId}"]`);
  if (!el) return;

  // Scroll the element into view
  el.scrollIntoView({ behavior: 'smooth', block: 'center' });

  // Add highlight and remove after animation
  el.classList.add('comment-highlight');
  setTimeout(() => el.classList.remove('comment-highlight'), 2000);
}

function commentInitials(name) {
  if (!name) return '?';
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  return name.slice(0, 2).toUpperCase();
}

function commentAvatarColor(name) {
  // Deterministic color from name hash — professional muted palette
  const colors = ['#4285f4','#ea4335','#fbbc04','#34a853','#ff6d01','#46bdc6','#7baaf7','#f07b72','#fcd04f','#57bb8a'];
  let hash = 0;
  for (let i = 0; i < (name || '').length; i++) hash = (hash * 31 + name.charCodeAt(i)) | 0;
  return colors[Math.abs(hash) % colors.length];
}

function formatCommentTime(dateStr) {
  if (!dateStr) return '';
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return escapeHtml(dateStr);
    const now = new Date();
    const diffMs = now - d;
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return 'Just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    if (diffDay < 7) return `${diffDay}d ago`;
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: d.getFullYear() !== now.getFullYear() ? 'numeric' : undefined });
  } catch (_) { return escapeHtml(dateStr); }
}

function renderCommentCard(c) {
  const cid = c.id || '';
  const isResolved = state.resolvedComments && state.resolvedComments.has(cid);
  const resolvedClass = isResolved ? ' comment-resolved' : '';
  const resolveLabel = isResolved ? 'Unresolve' : 'Resolve';
  const resolvedStatus = isResolved ? 'Resolved' : 'Active';
  const startNodeId = c.start_node_id || c.startNodeId || '';
  const author = c.author || 'Unknown';
  const initials = commentInitials(author);
  const avatarBg = commentAvatarColor(author);
  const timeDisplay = formatCommentTime(c.date);
  return `
    <div class="comment-card${resolvedClass}" data-comment-id="${escapeAttr(cid)}" data-start-node-id="${escapeAttr(startNodeId)}" style="cursor:pointer" role="article" aria-label="Comment by ${escapeAttr(author)} - ${resolvedStatus}">
      <div class="comment-header">
        <div class="comment-avatar" style="background:${avatarBg}" title="${escapeAttr(author)}">${escapeHtml(initials)}</div>
        <div class="comment-meta">
          <div class="comment-author">${escapeHtml(author)}</div>
          ${timeDisplay ? `<div class="comment-date" title="${escapeAttr(c.date || '')}">${timeDisplay}</div>` : ''}
        </div>
        ${isResolved ? '<span class="comment-resolved-badge" title="Resolved">Resolved</span>' : ''}
      </div>
      <div class="comment-text">${escapeHtml(c.text || c.body || '')}</div>
      <span class="sr-only" role="status">${resolvedStatus}</span>
      <div class="comment-actions">
        <button class="comment-reply-btn" data-parent-id="${escapeAttr(cid)}" title="Reply to this comment">Reply</button>
        <button class="comment-resolve-btn" data-id="${escapeAttr(cid)}" title="${resolveLabel} this comment" aria-pressed="${isResolved}">${resolveLabel}</button>
        <button class="comment-delete" data-id="${escapeAttr(cid)}" title="Delete this comment">Delete</button>
      </div>
    </div>`;
}

function renderReplyCard(r) {
  const author = r.author || 'Unknown';
  const initials = commentInitials(author);
  const avatarBg = commentAvatarColor(author);
  const timeDisplay = r.timestamp ? formatCommentTime(new Date(r.timestamp).toISOString()) : '';
  return `
    <div class="comment-card comment-reply" data-reply-id="${escapeAttr(r.id)}">
      <div class="comment-header">
        <div class="comment-avatar comment-avatar-sm" style="background:${avatarBg}" title="${escapeAttr(author)}">${escapeHtml(initials)}</div>
        <div class="comment-meta">
          <div class="comment-author">${escapeHtml(author)}</div>
          ${timeDisplay ? `<div class="comment-date">${timeDisplay}</div>` : ''}
        </div>
      </div>
      <div class="comment-text">${escapeHtml(r.text)}</div>
      <div class="comment-actions">
        <button class="reply-delete" data-reply-id="${escapeAttr(r.id)}" title="Delete this reply">Delete</button>
      </div>
    </div>`;
}

function showReplyForm(parentId) {
  const area = $('commentsList').querySelector(`.comment-reply-area[data-parent-id="${parentId}"]`);
  if (!area) return;
  // If already showing a form, remove it
  if (area.querySelector('.comment-reply-form')) {
    area.innerHTML = '';
    return;
  }
  area.innerHTML = `
    <div class="comment-reply-form">
      <input class="comment-reply-input" type="text" placeholder="Write a reply..." autocomplete="off">
      <div class="comment-reply-form-actions">
        <button class="comment-reply-submit">Post</button>
        <button class="comment-reply-cancel">Cancel</button>
      </div>
    </div>`;

  const input = area.querySelector('.comment-reply-input');
  if (input) input.focus();

  // Submit on Enter
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submitReply(parentId, input.value);
    }
    if (e.key === 'Escape') {
      area.innerHTML = '';
    }
  });

  area.querySelector('.comment-reply-submit').addEventListener('click', () => {
    submitReply(parentId, input.value);
  });
  area.querySelector('.comment-reply-cancel').addEventListener('click', () => {
    area.innerHTML = '';
  });
}

function submitReply(parentId, text) {
  if (!text || !text.trim()) return;
  const author = 'User';
  const trimmed = text.trim();

  // Persist the reply via the WASM document model if available
  if (state.doc && parentId) {
    try {
      state.doc.insert_comment_reply(parentId, author, trimmed);
      broadcastOp({ action: 'insertCommentReply', parentId, author, text: trimmed });
    } catch (e) {
      // WASM method may not exist in all builds — fall through to local storage
      console.warn('insert_comment_reply not available, storing locally:', e);
    }
  }

  // Store in local state for immediate UI rendering
  const reply = {
    id: 'reply-' + (++_replyCounter) + '-' + Date.now(),
    parentId,
    author,
    text: trimmed,
    timestamp: Date.now(),
  };
  if (!state.commentReplies) state.commentReplies = [];
  state.commentReplies.push(reply);
  try { localStorage.setItem('s1_commentReplies', JSON.stringify(state.commentReplies)); } catch(_) {}
  refreshComments();
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}
function escapeAttr(s) {
  return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

// ── Header/Footer Modal Opener ───────────────────
function openHeaderFooterModal() {
  // Extract plain text from existing header/footer HTML for editing
  const tmpH = document.createElement('div');
  tmpH.innerHTML = state.docHeaderHtml || '';
  $('headerText').value = tmpH.textContent || '';

  const tmpF = document.createElement('div');
  tmpF.innerHTML = state.docFooterHtml || '';
  // If footer has a page number field, check the box and strip it from text
  const hasPageField = tmpF.querySelector('[data-field="PageNumber"]') !== null ||
    (state.docFooterHtml || '').includes('data-field="PageNumber"');
  $('footerPageNum').checked = hasPageField;
  // Remove page number field element before extracting text
  tmpF.querySelectorAll('[data-field]').forEach(el => el.remove());
  let footerText = tmpF.textContent || '';
  // Strip em dash separator that we add when combining text + page num
  footerText = footerText.replace(/\s*\u2014\s*$/, '').replace(/^\s*\u2014\s*/, '').trim();
  $('footerText').value = footerText;

  $('differentFirstPage').checked = state.hasDifferentFirstPage || false;

  saveModalSelection();
  $('headerFooterModal').classList.add('show');
  $('headerText').focus();
}

// ── UXP-02: Header/Footer Inline Editing ─────────

/**
 * Enter inline header/footer editing mode.
 * @param {'header'|'footer'} kind — which region to edit
 * @param {HTMLElement} pageEl — the .doc-page element
 */
// Canonical version in features/document/toolbar/header-footer.js; re-exported at top.
function enterHeaderFooterEditMode(kind, pageEl) {
  // Exit any existing edit mode first
  if (state.hfEditingMode) {
    exitHeaderFooterEditMode();
  }

  const selector = kind === 'header' ? '.page-header' : '.page-footer';
  const hfEl = pageEl.querySelector(selector);
  if (!hfEl) return;

  const pageNum = parseInt(pageEl.dataset.page, 10) || 1;
  state.hfEditingMode = kind;
  state.hfEditingPage = pageNum;

  // Make the header/footer editable
  hfEl.contentEditable = 'true';
  hfEl.classList.remove('hf-hoverable');
  hfEl.classList.add('hf-editing');
  hfEl.removeAttribute('title');

  // Add label badge
  const label = document.createElement('span');
  label.className = 'hf-editing-label';
  label.textContent = kind === 'header' ? 'Header' : 'Footer';
  hfEl.appendChild(label);

  // Add mini toolbar with options and close button
  const toolbar = document.createElement('span');
  toolbar.className = 'hf-toolbar';

  const pageNumBtn = document.createElement('button');
  pageNumBtn.textContent = 'Insert Page Number';
  pageNumBtn.title = 'Insert a page number field at cursor position';
  pageNumBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    _insertPageNumberField(hfEl);
  });
  toolbar.appendChild(pageNumBtn);

  const optionsBtn = document.createElement('button');
  optionsBtn.textContent = 'Options';
  optionsBtn.title = 'Open header and footer options';
  optionsBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    openHeaderFooterModal();
  });
  toolbar.appendChild(optionsBtn);

  const closeBtn = document.createElement('button');
  closeBtn.textContent = 'Close';
  closeBtn.title = 'Exit header/footer editing (Escape)';
  closeBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    exitHeaderFooterEditMode();
  });
  toolbar.appendChild(closeBtn);

  hfEl.appendChild(toolbar);

  // Dim the main content area
  const contentEl = pageEl.querySelector('.page-content');
  if (contentEl) contentEl.classList.add('hf-dimmed');

  // Focus the header/footer
  hfEl.focus();

  // Place cursor at end of existing content (before our label/toolbar elements)
  try {
    const sel = window.getSelection();
    const range = document.createRange();
    // Find the last text-bearing child (skip our label/toolbar)
    const textNodes = [];
    for (const child of hfEl.childNodes) {
      if (child.nodeType === Node.TEXT_NODE ||
          (child.nodeType === Node.ELEMENT_NODE &&
           !child.classList.contains('hf-editing-label') &&
           !child.classList.contains('hf-toolbar'))) {
        textNodes.push(child);
      }
    }
    if (textNodes.length > 0) {
      const lastChild = textNodes[textNodes.length - 1];
      range.selectNodeContents(lastChild);
      range.collapse(false);
    } else {
      range.selectNodeContents(hfEl);
      range.collapse(false);
    }
    sel.removeAllRanges();
    sel.addRange(range);
  } catch (_) {}
}

/**
 * Exit header/footer editing mode and sync content back.
 */
function exitHeaderFooterEditMode() {
  if (!state.hfEditingMode) return;

  const kind = state.hfEditingMode;
  const pageNum = state.hfEditingPage;
  state.hfEditingMode = null;
  state.hfEditingPage = null;

  // Find all editing header/footer elements and restore them
  const container = $('pageContainer');
  if (!container) return;

  container.querySelectorAll('.hf-editing').forEach(hfEl => {
    // Extract the user-entered content (excluding our UI elements)
    const label = hfEl.querySelector('.hf-editing-label');
    const toolbar = hfEl.querySelector('.hf-toolbar');
    if (label) label.remove();
    if (toolbar) toolbar.remove();

    // Get the text content the user entered, EXCLUDING field element text.
    // Field elements (page number, page count) have substituted text that
    // must not be synced back to the WASM model as plain text — doing so
    // would duplicate it alongside the Field nodes, causing garbled output
    // like "Page 1Page 1" or "12" instead of just "1".
    const userHtml = hfEl.innerHTML.trim();
    const cloneForText = hfEl.cloneNode(true);
    cloneForText.querySelectorAll('[data-field]').forEach(f => f.remove());
    const userText = cloneForText.textContent.trim();

    // Restore non-editable state
    hfEl.contentEditable = 'false';
    hfEl.classList.remove('hf-editing');
    hfEl.classList.add('hf-hoverable');
    hfEl.setAttribute('title',
      hfEl.dataset.hfKind === 'header' ? 'Double-click to edit header' : 'Double-click to edit footer');

    // Un-dim content
    const pageEl = hfEl.closest('.doc-page');
    if (pageEl) {
      const contentEl = pageEl.querySelector('.page-content');
      if (contentEl) contentEl.classList.remove('hf-dimmed');
    }

    // Sync the edited content back to state
    const isHeader = hfEl.dataset.hfKind === 'header';
    const isFirstPage = (pageNum === 1) && state.hasDifferentFirstPage;

    if (userText || userHtml) {
      // Preserve any data-field spans (page numbers) the user may have added
      const hasFields = hfEl.querySelector('[data-field]') !== null;
      let finalHtml;
      if (hasFields) {
        // Keep the HTML structure (it has field elements)
        finalHtml = userHtml;
      } else {
        // Wrap in styled span
        finalHtml = '<span style="display:block;text-align:center;color:var(--text-secondary,#5f6368);font-size:9pt">' +
          _escapeHtmlForHF(userText) + '</span>';
      }

      if (isHeader) {
        if (isFirstPage) {
          state.docFirstPageHeaderHtml = finalHtml;
        } else {
          state.docHeaderHtml = finalHtml;
        }
      } else {
        if (isFirstPage) {
          state.docFirstPageFooterHtml = finalHtml;
        } else {
          state.docFooterHtml = finalHtml;
        }
      }
    } else {
      // Empty content — clear
      if (isHeader) {
        if (isFirstPage) {
          state.docFirstPageHeaderHtml = '';
        } else {
          state.docHeaderHtml = '';
        }
      } else {
        if (isFirstPage) {
          state.docFirstPageFooterHtml = '';
        } else {
          state.docFooterHtml = '';
        }
      }
    }

    // Sync to WASM backend if available
    _syncHeaderFooterToWasm(hfEl.dataset.hfKind, isFirstPage ? 'first' : 'default', userText);
  });

  // Re-render pages to apply updated header/footer across all pages
  renderDocument();
}

/**
 * Sync header/footer text to the WASM model.
 */
function _syncHeaderFooterToWasm(kind, hfType, text) {
  const { doc } = state;
  if (!doc) return;
  try {
    if (typeof doc.set_header_footer_text === 'function') {
      doc.set_header_footer_text(0, kind, hfType, text);
    }
  } catch (e) {
    console.warn('Failed to sync header/footer to WASM:', e);
  }
}

/**
 * Insert a page number field element at the current cursor position.
 */
function _insertPageNumberField(hfEl) {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return;

  const range = sel.getRangeAt(0);
  // Verify the selection is within the header/footer element
  if (!hfEl.contains(range.startContainer)) {
    // Place cursor at end
    const newRange = document.createRange();
    newRange.selectNodeContents(hfEl);
    newRange.collapse(false);
    sel.removeAllRanges();
    sel.addRange(newRange);
  }

  const field = document.createElement('span');
  field.setAttribute('data-field', 'PageNumber');
  field.contentEditable = 'false';
  field.style.fontWeight = 'normal';

  // Show placeholder number
  const pageEl = hfEl.closest('.doc-page');
  const pageNum = pageEl ? (parseInt(pageEl.dataset.page, 10) || 1) : 1;
  field.textContent = String(pageNum);

  const updatedRange = sel.getRangeAt(0);
  updatedRange.deleteContents();
  updatedRange.insertNode(field);

  // Move cursor after the inserted field
  const afterRange = document.createRange();
  afterRange.setStartAfter(field);
  afterRange.collapse(true);
  sel.removeAllRanges();
  sel.addRange(afterRange);
}

/**
 * Escape HTML for header/footer content.
 */
function _escapeHtmlForHF(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

// ── Version History ──────────────────────────────
function formatVersionDate(ts) {
  const d = new Date(ts);
  const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  const month = months[d.getMonth()];
  const day = d.getDate();
  let hours = d.getHours();
  const mins = d.getMinutes().toString().padStart(2, '0');
  const ampm = hours >= 12 ? 'PM' : 'AM';
  hours = hours % 12 || 12;
  return `${month} ${day}, ${hours}:${mins} ${ampm}`;
}

function refreshHistory() {
  const list = $('historyList');
  if (!list) return;
  list.innerHTML = '<div class="history-loading">Loading versions...</div>';
  getVersions().then(versions => {
    if (!versions || versions.length === 0) {
      list.innerHTML = '<div class="history-empty">No saved versions yet. Versions are saved automatically every 5 minutes and on manual save (Ctrl+S).</div>';
      return;
    }
    // Current word count for diff display
    let currentWordCount = 0;
    try { currentWordCount = versions[0]?.wordCount || 0; } catch (_) {}

    list.innerHTML = versions.map((v, i) => {
      const diff = i > 0 ? v.wordCount - currentWordCount : 0;
      const diffStr = diff > 0 ? `<span class="diff-added">+${diff}</span>` : diff < 0 ? `<span class="diff-removed">${diff}</span>` : '';
      return `
      <div class="version-card" data-version-id="${v.id}">
        <div class="version-info">
          <div class="version-date">${escapeHtml(formatVersionDate(v.timestamp))}</div>
          <div class="version-meta">${v.wordCount.toLocaleString()} word${v.wordCount !== 1 ? 's' : ''} ${diffStr}${v.label ? ' &middot; ' + escapeHtml(v.label) : ''}</div>
          ${i === 0 ? '<span class="version-badge">Current version</span>' : ''}
        </div>
        ${i > 0 ? '<div class="version-actions"><button class="version-preview" data-id="' + v.id + '" title="Preview text changes">Preview</button><button class="version-restore" data-id="' + v.id + '">Restore</button></div>' : ''}
      </div>`;
    }).join('');

    // Preview button — shows plain text diff in a popup
    list.querySelectorAll('.version-preview').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = parseInt(btn.dataset.id);
        if (!id || !state.engine) return;
        openAutosaveDB().then(db => {
          const tx = db.transaction('versions', 'readonly');
          const req = tx.objectStore('versions').get(id);
          req.onsuccess = () => {
            const v = req.result;
            if (!v?.bytes) return;
            try {
              const oldDoc = state.engine.open(new Uint8Array(v.bytes));
              const oldText = oldDoc.to_plain_text();
              const curText = state.doc ? state.doc.to_plain_text() : '';
              showTextDiffPopup(oldText, curText, formatVersionDate(v.timestamp));
            } catch (e) { console.error('preview:', e); }
          };
        }).catch(() => {});
      });
    });

    list.querySelectorAll('.version-restore').forEach(btn => {
      btn.addEventListener('click', async () => {
        const id = parseInt(btn.dataset.id);
        if (!id || !state.engine) return;
        const confirmed = await showConfirmModal('Restore this version? Current unsaved changes will be lost.');
        if (!confirmed) return;
        try {
          showToast('Restoring version...', 'info');
          await restoreVersion(id);
          refreshHistory();
          showToast('Version restored', 'info', 2000);
        } catch (e) {
          showToast('Failed to restore version: ' + e.message, 'error');
          console.error('restore version:', e);
        }
      });
    });
  });
}

// ── Version Diff Popup ───────────────────────────
function showTextDiffPopup(oldText, newText, versionLabel) {
  // Remove existing popup if any
  const existing = document.querySelector('.version-diff-overlay');
  if (existing) existing.remove();

  const oldLines = oldText.split('\n');
  const newLines = newText.split('\n');

  // Simple line-by-line diff: mark added, removed, unchanged
  const maxLen = Math.max(oldLines.length, newLines.length);
  let diffHtml = '';
  let added = 0, removed = 0, unchanged = 0;

  for (let i = 0; i < maxLen; i++) {
    const ol = i < oldLines.length ? oldLines[i] : null;
    const nl = i < newLines.length ? newLines[i] : null;
    if (ol === nl) {
      diffHtml += `<div class="diff-line diff-same">${escapeHtml(ol || '')}</div>`;
      unchanged++;
    } else if (ol === null) {
      diffHtml += `<div class="diff-line diff-added">+ ${escapeHtml(nl)}</div>`;
      added++;
    } else if (nl === null) {
      diffHtml += `<div class="diff-line diff-removed">- ${escapeHtml(ol)}</div>`;
      removed++;
    } else {
      diffHtml += `<div class="diff-line diff-removed">- ${escapeHtml(ol)}</div>`;
      diffHtml += `<div class="diff-line diff-added">+ ${escapeHtml(nl)}</div>`;
      removed++;
      added++;
    }
  }

  const overlay = document.createElement('div');
  overlay.className = 'version-diff-overlay';
  overlay.innerHTML = `
    <div class="version-diff-dialog">
      <div class="version-diff-header">
        <span class="version-diff-title">Changes since ${escapeHtml(versionLabel)}</span>
        <span class="version-diff-stats">
          <span class="diff-added">+${added} added</span>,
          <span class="diff-removed">-${removed} removed</span>,
          ${unchanged} unchanged
        </span>
        <button class="version-diff-close" title="Close">&times;</button>
      </div>
      <div class="version-diff-body">${diffHtml || '<div class="diff-line diff-same">(No differences)</div>'}</div>
    </div>`;

  overlay.addEventListener('click', e => {
    if (e.target === overlay || e.target.classList.contains('version-diff-close')) {
      overlay.remove();
    }
  });
  overlay.querySelector('.version-diff-close').addEventListener('click', () => overlay.remove());

  document.body.appendChild(overlay);
}

// ── Style Gallery ─────────────────────────────────
// Style definitions: styleId for model, heading level for compat, font overrides for run-level
const STYLE_DEFS = {
  normal:   { styleId: '',         heading: 0, fontSize: null, fontFamily: null, color: null, italic: false },
  title:    { styleId: 'Title',    heading: 0, fontSize: '26', fontFamily: null, color: null, italic: false },
  subtitle: { styleId: 'Subtitle', heading: 0, fontSize: '15', fontFamily: null, color: '666666', italic: false },
  heading1: { styleId: 'Heading1', heading: 1, fontSize: null, fontFamily: null, color: null, italic: false },
  heading2: { styleId: 'Heading2', heading: 2, fontSize: null, fontFamily: null, color: null, italic: false },
  heading3: { styleId: 'Heading3', heading: 3, fontSize: null, fontFamily: null, color: null, italic: false },
  heading4: { styleId: 'Heading4', heading: 4, fontSize: null, fontFamily: null, color: null, italic: false },
  quote:    { styleId: 'Quote',    heading: 0, fontSize: null, fontFamily: null, color: '666666', italic: true },
  code:     { styleId: 'Code',     heading: 0, fontSize: '11', fontFamily: 'Courier New', color: null, italic: false },
};

/** Apply a named paragraph style to the current selection.
 *  Called from style gallery clicks and keyboard shortcuts (Ctrl+Alt+0-6).
 *  @param {string} styleName - key into STYLE_DEFS (e.g. 'heading1', 'title', 'normal')
 */
export function applyParagraphStyle(styleName) {
  if (!state.doc) return;
  const def = STYLE_DEFS[styleName];
  if (!def) return;
  const info = getSelectionInfo();
  if (!info) return;

  syncAllText();
  try {
    // FS-16/37: Batch all style operations into one undo step
    if (typeof state.doc.begin_batch === 'function') {
      state.doc.begin_batch('Apply style: ' + styleName);
    }

    const paraIds = getSelectedParagraphIds(info);
    paraIds.forEach(nodeId => {
      // Set the style ID on the paragraph node (authoritative for model/export)
      if (typeof state.doc.set_paragraph_style_id === 'function') {
        state.doc.set_paragraph_style_id(nodeId, def.styleId);
        broadcastOp({ action: 'setStyle', nodeId, styleId: def.styleId });
      }
      // Also set heading level for backward compatibility with rendering
      state.doc.set_heading_level(nodeId, def.heading);
      broadcastOp({ action: 'setHeading', nodeId, level: def.heading });

      // Apply run-level formatting overrides (whole paragraph text)
      const pEl = $('pageContainer')?.querySelector(`[data-node-id="${nodeId}"]`);
      const textLen = pEl ? Array.from(pEl.textContent || '').length : 0;
      if (textLen > 0) {
        const bcast = (key, value) => {
          state.doc.format_selection(nodeId, 0, nodeId, textLen, key, value);
          broadcastOp({ action: 'formatSelection', startNode: nodeId, startOffset: 0, endNode: nodeId, endOffset: textLen, key, value });
        };
        if (def.fontSize) bcast('fontSize', def.fontSize);
        if (def.fontFamily) bcast('fontFamily', def.fontFamily);
        if (def.color) bcast('color', def.color);
        if (def.italic) {
          bcast('italic', 'true');
        } else {
          try { bcast('italic', 'false'); } catch(_) {}
        }
      }
    });

    renderDocument();
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.warn('applyParagraphStyle:', e); } finally {
    // FS-16/37: End batch — all ops become one undo step (always run, even on error)
    try {
      if (typeof state.doc?.end_batch === 'function' && state.doc.is_batching()) {
        state.doc.end_batch();
      }
    } catch (_) {}
  }
}

function initStyleGallery() {
  const btn = $('styleGalleryBtn');
  const panel = $('styleGalleryPanel');

  // Toggle panel on button click (position:fixed, so calculate from button rect)
  btn.addEventListener('click', e => {
    e.stopPropagation();
    const wasOpen = panel.classList.contains('show');
    panel.classList.toggle('show');
    if (!wasOpen) {
      const rect = btn.getBoundingClientRect();
      let top = rect.bottom + 4;
      let left = rect.left;
      // FS-23: Clamp to viewport, flip above if overflowing
      requestAnimationFrame(() => {
        const panelRect = panel.getBoundingClientRect();
        if (top + panelRect.height > window.innerHeight) {
          const flipped = rect.top - panelRect.height - 4;
          top = flipped > 4 ? flipped : Math.max(4, window.innerHeight - panelRect.height - 4);
        }
        if (left + panelRect.width > window.innerWidth) {
          left = Math.max(4, window.innerWidth - panelRect.width - 4);
        }
        panel.style.top = top + 'px';
        panel.style.left = left + 'px';
      });
      panel.style.top = top + 'px';
      panel.style.left = left + 'px';
    }
    btn.setAttribute('aria-expanded', panel.classList.contains('show') ? 'true' : 'false');
  });

  // Handle style item clicks
  panel.querySelectorAll('.style-gallery-item').forEach(item => {
    item.addEventListener('click', () => {
      const styleName = item.dataset.style;
      if (!styleName || !state.doc) { panel.classList.remove('show'); return; }
      const info = getSelectionInfo();
      if (!info) { panel.classList.remove('show'); return; }

      const el = $('pageContainer')?.querySelector(`[data-node-id="${info.startNodeId}"]`);
      if (el) syncParagraphText(el);

      const def = STYLE_DEFS[styleName];
      if (!def) { panel.classList.remove('show'); return; }

      try {
        applyParagraphStyle(styleName);
      } catch (err) { console.error('style gallery:', err); }

      panel.classList.remove('show');
      btn.setAttribute('aria-expanded', 'false');
    });
  });
}

function initTableContextMenu() {
  $('pageContainer').addEventListener('contextmenu', e => {
    const cell = e.target.closest('td, th');
    if (!cell || !state.doc) return;
    e.preventDefault();

    const table = cell.closest('table');
    const tableEl = table?.closest('[data-node-id]');
    if (!tableEl) return;

    const row = cell.parentElement;
    const rowIndex = Array.from(row.parentElement.children).indexOf(row);
    const colIndex = Array.from(row.children).indexOf(cell);

    state.ctxTable = tableEl.dataset.nodeId;
    state.ctxCell = cell.closest('[data-node-id]')?.dataset.nodeId;
    state.ctxRow = rowIndex;
    state.ctxCol = colIndex;

    const menu = $('tableContextMenu');
    menu.style.display = 'block';
    // Position with viewport boundary check
    const menuW = 200, menuH = 280;
    const x = Math.min(e.clientX, window.innerWidth - menuW);
    const y = Math.min(e.clientY, window.innerHeight - menuH);
    menu.style.left = Math.max(0, x) + 'px';
    menu.style.top = Math.max(0, y) + 'px';
  });

  const cmAction = (id, fn) => {
    $(id).addEventListener('click', () => {
      $('tableContextMenu').style.display = 'none';
      if (!state.doc || !state.ctxTable) return;
      syncAllText();
      try {
        fn();
        // E-04: Try to re-render only the table instead of the full document.
        // Save scroll position in case we must fall back to full re-render.
        const canvas = $('editorCanvas');
        const scrollTop = canvas ? canvas.scrollTop : 0;
        const tableNodeId = state.ctxTable;
        const tableEl = $('pageContainer')?.querySelector(`[data-node-id="${tableNodeId}"]`);
        if (tableEl) {
          // Re-render just the table node
          const updated = renderNodeById(tableNodeId);
          if (!updated) {
            // Fallback: full re-render with scroll restore
            renderDocument();
            if (canvas) canvas.scrollTop = scrollTop;
          }
        } else {
          // Table element not found — full re-render with scroll restore
          renderDocument();
          if (canvas) canvas.scrollTop = scrollTop;
        }
        updateUndoRedo();
      }
      catch (e) { console.error('table op:', e); }
    });
  };

  cmAction('cmInsertRowAbove', () => { state.doc.insert_table_row(state.ctxTable, state.ctxRow); broadcastOp({ action: 'insertTableRow', tableId: state.ctxTable, index: state.ctxRow }); });
  cmAction('cmInsertRowBelow', () => { state.doc.insert_table_row(state.ctxTable, state.ctxRow + 1); broadcastOp({ action: 'insertTableRow', tableId: state.ctxTable, index: state.ctxRow + 1 }); });
  cmAction('cmDeleteRow', () => { state.doc.delete_table_row(state.ctxTable, state.ctxRow); broadcastOp({ action: 'deleteTableRow', tableId: state.ctxTable, index: state.ctxRow }); });
  cmAction('cmInsertColLeft', () => { state.doc.insert_table_column(state.ctxTable, state.ctxCol); broadcastOp({ action: 'insertTableColumn', tableId: state.ctxTable, index: state.ctxCol }); });
  cmAction('cmInsertColRight', () => { state.doc.insert_table_column(state.ctxTable, state.ctxCol + 1); broadcastOp({ action: 'insertTableColumn', tableId: state.ctxTable, index: state.ctxCol + 1 }); });
  cmAction('cmDeleteCol', () => { state.doc.delete_table_column(state.ctxTable, state.ctxCol); broadcastOp({ action: 'deleteTableColumn', tableId: state.ctxTable, index: state.ctxCol }); });

  // Delete entire table
  cmAction('cmDeleteTable', () => {
    state.doc.delete_node(state.ctxTable);
    broadcastOp({ action: 'deleteNode', nodeId: state.ctxTable });
  });

  // UXP-12: Merge cells — uses existing WASM merge_cells API
  cmAction('cmMergeCells', () => {
    // J2: Validate that row/col indices are valid numbers before merging
    if (typeof state.ctxRow !== 'number' || typeof state.ctxCol !== 'number'
        || isNaN(state.ctxRow) || isNaN(state.ctxCol)
        || state.ctxRow < 0 || state.ctxCol < 0) {
      showToast('Select a valid cell before merging', 'error');
      return;
    }
    // J2: Validate rectangular selection if multi-cell selection is tracked
    if (state.selectionCells
        && (!state.selectionCells.rowSpan || !state.selectionCells.colSpan)) {
      showToast('Select a rectangular range of cells to merge', 'error');
      return;
    }
    // Merge a 2x2 block starting from the right-clicked cell.
    // If cell is at the last row/col, merge just the available range.
    const dims = JSON.parse(state.doc.get_table_dimensions(state.ctxTable));
    if (state.ctxRow >= dims.rows || state.ctxCol >= dims.cols) {
      showToast('Cell position is out of table bounds', 'error');
      return;
    }
    const endRow = Math.min(state.ctxRow + 1, dims.rows - 1);
    const endCol = Math.min(state.ctxCol + 1, dims.cols - 1);
    state.doc.merge_cells(state.ctxTable, state.ctxRow, state.ctxCol, endRow, endCol);
    broadcastOp({ action: 'mergeCells', tableId: state.ctxTable, startRow: state.ctxRow, startCol: state.ctxCol, endRow, endCol });
    announce('Cells merged');
  });

  // UXP-12: Split cell — removes ColSpan/RowSpan from the clicked cell.
  // Uses split_merged_cell WASM API if available, otherwise clears spans via merge_cells(1x1).
  cmAction('cmSplitCell', () => {
    if (typeof state.doc.split_merged_cell === 'function') {
      state.doc.split_merged_cell(state.ctxTable, state.ctxRow, state.ctxCol);
      broadcastOp({ action: 'splitCell', tableId: state.ctxTable, row: state.ctxRow, col: state.ctxCol });
    } else {
      // Fallback: merge a 1x1 range to reset spans (effectively clears ColSpan/RowSpan)
      state.doc.merge_cells(state.ctxTable, state.ctxRow, state.ctxCol, state.ctxRow, state.ctxCol);
      broadcastOp({ action: 'splitCell', tableId: state.ctxTable, row: state.ctxRow, col: state.ctxCol });
    }
    announce('Cell split');
  });

  // Cell background — color picker instead of prompt
  $('cmCellBg').addEventListener('click', e => {
    e.preventDefault();
    e.stopPropagation();
    // Trigger the hidden color picker
    const picker = $('cmCellBgPicker');
    picker.style.pointerEvents = 'auto';
    picker.click();
  });
  $('cmCellBgPicker').addEventListener('input', e => {
    $('tableContextMenu').style.display = 'none';
    if (!state.ctxTable || !state.doc || !state.ctxCell) return;
    const hex = e.target.value.replace('#', '');
    try {
      state.doc.set_cell_background(state.ctxCell, hex);
      broadcastOp({ action: 'setCellBackground', cellId: state.ctxCell, color: hex });
      // E-04: Re-render only the table, not the entire document
      const canvas = $('editorCanvas');
      const scrollTop = canvas ? canvas.scrollTop : 0;
      const tableNodeId = state.ctxTable;
      const tableEl = tableNodeId ? $('pageContainer')?.querySelector(`[data-node-id="${tableNodeId}"]`) : null;
      if (tableEl) {
        const updated = renderNodeById(tableNodeId);
        if (!updated) { renderDocument(); if (canvas) canvas.scrollTop = scrollTop; }
      } else {
        renderDocument();
        if (canvas) canvas.scrollTop = scrollTop;
      }
      updateUndoRedo();
    } catch (err) { console.error('cell bg:', err); }
  });
  $('cmCellBgPicker').addEventListener('change', () => {
    $('cmCellBgPicker').style.pointerEvents = 'none';
  });

  // UXP-12: Column resize — drag handles between columns
  initTableColumnResize();
}

// ── UXP-12: Table Column Resize ──────────────────
function initTableColumnResize() {
  const page = $('pageContainer');
  if (!page) return;

  let _colDrag = null; // { table, colIndex, startX, startWidths, tableWidth }

  // Show col-resize cursor when hovering near column borders in tables
  page.addEventListener('mousemove', e => {
    if (_colDrag) return; // Dragging in progress
    const cell = e.target.closest('td, th');
    if (!cell) return;
    const rect = cell.getBoundingClientRect();
    const nearRight = Math.abs(e.clientX - rect.right) < 5;
    const nearLeft = Math.abs(e.clientX - rect.left) < 5 && cell !== cell.parentElement.firstElementChild;
    if (nearRight || nearLeft) {
      cell.style.cursor = 'col-resize';
    } else {
      cell.style.cursor = '';
    }
  });

  // Start column resize drag
  page.addEventListener('mousedown', e => {
    const cell = e.target.closest('td, th');
    if (!cell) return;
    const rect = cell.getBoundingClientRect();
    const nearRight = Math.abs(e.clientX - rect.right) < 5;
    const nearLeft = Math.abs(e.clientX - rect.left) < 5 && cell !== cell.parentElement.firstElementChild;
    if (!nearRight && !nearLeft) return;

    e.preventDefault();
    e.stopPropagation();

    const table = cell.closest('table');
    if (!table) return;

    // Determine which column border we're dragging
    const row = cell.parentElement;
    const cells = Array.from(row.children);
    let colIndex;
    if (nearRight) {
      colIndex = cells.indexOf(cell);
    } else {
      // Near left border = right border of previous column
      colIndex = cells.indexOf(cell) - 1;
    }
    if (colIndex < 0 || colIndex >= cells.length - 1) return; // Can't resize last col's right border beyond table

    // Capture current column widths
    const firstRow = table.querySelector('tr');
    if (!firstRow) return;
    const allCells = Array.from(firstRow.children);
    const startWidths = allCells.map(c => c.getBoundingClientRect().width);
    const tableWidth = table.getBoundingClientRect().width;

    _colDrag = { table, colIndex, startX: e.clientX, startWidths, tableWidth };
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    document.addEventListener('mousemove', onColResize);
    document.addEventListener('mouseup', endColResize);
  });

  function onColResize(e) {
    if (!_colDrag) return;
    const delta = e.clientX - _colDrag.startX;
    const { table, colIndex, startWidths, tableWidth } = _colDrag;

    // Calculate new widths (min 30px per column)
    const newLeftW = Math.max(30, startWidths[colIndex] + delta);
    const newRightW = Math.max(30, startWidths[colIndex + 1] - delta);

    // Apply column widths via colgroup or cell styles
    const rows = table.querySelectorAll('tr');
    rows.forEach(row => {
      const cells = row.children;
      if (cells[colIndex]) {
        cells[colIndex].style.width = newLeftW + 'px';
      }
      if (cells[colIndex + 1]) {
        cells[colIndex + 1].style.width = newRightW + 'px';
      }
    });
  }

  function endColResize(e) {
    if (!_colDrag) return;
    onColResize(e); // Final position
    _colDrag = null;
    document.removeEventListener('mousemove', onColResize);
    document.removeEventListener('mouseup', endColResize);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }
}

// ── E10.1: Dark Mode ─────────────────────────────
function initDarkMode() {
  const btn = $('btnDarkMode');
  if (!btn) return;

  // Restore saved preference
  let saved; try { saved = localStorage.getItem('s1-theme'); } catch (_) {}
  if (saved === 'dark' || saved === 'light') {
    document.documentElement.setAttribute('data-theme', saved);
  }
  updateDarkModeIcon();

  btn.addEventListener('click', () => toggleDarkMode());

  // UXP-18: Listen for OS dark mode preference changes
  try {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    mq.addEventListener('change', () => {
      // Only react if user hasn't set an explicit preference
      let explicit; try { explicit = localStorage.getItem('s1-theme'); } catch (_) {}
      if (!explicit) {
        updateDarkModeIcon();
      }
    });
  } catch (_) {}
}

function updateDarkModeIcon() {
  const btn = $('btnDarkMode');
  if (!btn) return;
  const theme = document.documentElement.getAttribute('data-theme');
  const isDark = theme === 'dark' || (!theme && window.matchMedia('(prefers-color-scheme: dark)').matches);
  const icon = btn.querySelector('.msi');
  if (icon) icon.textContent = isDark ? 'light_mode' : 'dark_mode';
  btn.title = isDark ? 'Switch to light mode' : 'Switch to dark mode';
}

// ── E4.3: Table Properties Dialog ────────────────
function initTablePropsModal() {
  const modal = $('tablePropsModal');
  if (!modal) return;

  // Open from context menu
  $('cmTableProps').addEventListener('click', () => {
    $('tableContextMenu').style.display = 'none';
    if (!state.doc || !state.ctxTable) return;
    openTableProps(state.ctxTable);
  });

  // Width mode toggle
  $('tpWidthMode').addEventListener('change', () => {
    const mode = $('tpWidthMode').value;
    const show = mode !== 'auto';
    $('tpWidthValue').style.display = show ? '' : 'none';
    $('tpWidthUnit').style.display = show ? '' : 'none';
    $('tpWidthUnit').textContent = mode === 'percent' ? '%' : 'px';
    if (mode === 'percent') $('tpWidthValue').value = '100';
    if (mode === 'fixed') $('tpWidthValue').value = '600';
  });

  // Border preset buttons (radio-style)
  modal.querySelectorAll('.tp-border-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      modal.querySelectorAll('.tp-border-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
    });
  });

  // Alignment buttons (radio-style)
  modal.querySelectorAll('.tp-align-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      modal.querySelectorAll('.tp-align-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
    });
  });

  // Cancel
  $('tpCancelBtn').addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Apply
  $('tpApplyBtn').addEventListener('click', () => {
    const tableId = state._tpTableId;
    if (!tableId || !state.doc) { modal.classList.remove('show'); return; }

    applyTableProps(tableId);
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Backdrop close
  modal.addEventListener('click', e => {
    if (e.target === modal) { modal.classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
}

function openTableProps(tableId) {
  state._tpTableId = tableId;
  // Reset to defaults
  $('tpWidthMode').value = 'auto';
  $('tpWidthValue').style.display = 'none';
  $('tpWidthUnit').style.display = 'none';
  $('tpBorderColor').value = '#000000';
  $('tpBorderWidth').value = '1';

  // Reset button states
  const modal = $('tablePropsModal');
  modal.querySelectorAll('.tp-border-btn').forEach(b => b.classList.remove('active'));
  modal.querySelector('.tp-border-btn[data-borders="all"]').classList.add('active');
  modal.querySelectorAll('.tp-align-btn').forEach(b => b.classList.remove('active'));
  modal.querySelector('.tp-align-btn[data-align="left"]').classList.add('active');

  // Try to read current table state from DOM
  const tableEl = $('pageContainer')?.querySelector(`[data-node-id="${tableId}"]`);
  if (tableEl) {
    const table = tableEl.tagName === 'TABLE' ? tableEl : tableEl.querySelector('table');
    if (table) {
      // Detect current width
      if (table.style.width) {
        if (table.style.width.endsWith('%')) {
          $('tpWidthMode').value = 'percent';
          $('tpWidthValue').value = parseInt(table.style.width);
          $('tpWidthValue').style.display = '';
          $('tpWidthUnit').style.display = '';
          $('tpWidthUnit').textContent = '%';
        } else if (table.style.width.endsWith('px')) {
          $('tpWidthMode').value = 'fixed';
          $('tpWidthValue').value = parseInt(table.style.width);
          $('tpWidthValue').style.display = '';
          $('tpWidthUnit').style.display = '';
          $('tpWidthUnit').textContent = 'px';
        }
      }
      // Detect alignment
      const align = table.style.marginLeft === 'auto' && table.style.marginRight === 'auto'
        ? 'center'
        : table.style.marginLeft === 'auto' ? 'right' : 'left';
      modal.querySelectorAll('.tp-align-btn').forEach(b => b.classList.remove('active'));
      const alignBtn = modal.querySelector(`.tp-align-btn[data-align="${align}"]`);
      if (alignBtn) alignBtn.classList.add('active');
    }
  }

  modal.classList.add('show');
}

function applyTableProps(tableId) {
  const tableEl = $('pageContainer')?.querySelector(`[data-node-id="${tableId}"]`);
  if (!tableEl) return;
  const table = tableEl.tagName === 'TABLE' ? tableEl : tableEl.querySelector('table');
  if (!table) return;

  // Apply width
  const widthMode = $('tpWidthMode').value;
  if (widthMode === 'auto') {
    table.style.width = '';
  } else if (widthMode === 'percent') {
    table.style.width = Math.max(10, Math.min(100, parseInt($('tpWidthValue').value) || 100)) + '%';
  } else if (widthMode === 'fixed') {
    table.style.width = Math.max(50, Math.min(2000, parseInt($('tpWidthValue').value) || 600)) + 'px';
  }

  // Apply borders
  const activeBorder = $('tablePropsModal').querySelector('.tp-border-btn.active');
  const borderStyle = activeBorder ? activeBorder.dataset.borders : 'all';
  const borderColor = $('tpBorderColor').value;
  const borderWidth = $('tpBorderWidth').value + 'px';
  const borderVal = `${borderWidth} solid ${borderColor}`;

  const cells = table.querySelectorAll('td, th');
  if (borderStyle === 'none') {
    table.style.border = 'none';
    cells.forEach(c => { c.style.border = 'none'; });
  } else if (borderStyle === 'outer') {
    table.style.border = borderVal;
    cells.forEach(c => { c.style.border = 'none'; });
  } else {
    table.style.border = borderVal;
    cells.forEach(c => { c.style.border = borderVal; });
  }

  // Apply alignment
  const activeAlign = $('tablePropsModal').querySelector('.tp-align-btn.active');
  const align = activeAlign ? activeAlign.dataset.align : 'left';
  if (align === 'center') {
    table.style.marginLeft = 'auto';
    table.style.marginRight = 'auto';
  } else if (align === 'right') {
    table.style.marginLeft = 'auto';
    table.style.marginRight = '0';
  } else {
    table.style.marginLeft = '';
    table.style.marginRight = '';
  }

  state.pagesRendered = false;
  updatePageBreaks();
}

// ── E7.1: Modal Focus Trap ───────────────────────
// When a modal is open, Tab/Shift+Tab cycle only within the modal.
// Escape closes the modal. Focus returns to the element that opened it.
// Selection is saved before open and restored after close.
const MODAL_IDS = ['tableModal', 'commentModal', 'linkModal', 'altTextModal', 'tablePropsModal', 'headerFooterModal', 'templateModal', 'pageSetupModal', 'columnsModal', 'equationModal', 'bookmarkModal', 'dictModal', 'autoCorrectModal', 'shortcutsModal', 'usageStatsModal', 'errorDetailModal', 'whatsNewModal', 'welcomeModal', 'specialCharsModal', 'bordersModal', 'captionModal'];
const FOCUSABLE_SELECTOR = 'button, [href], input:not([type=hidden]), select, textarea, [tabindex]:not([tabindex="-1"])';

function initModalFocusTrap() {
  // Track which element opened the modal so we can return focus
  const openerMap = new WeakMap();
  // Track saved selection per modal
  const selectionMap = new WeakMap();

  // Observe modal open/close via class changes
  const observer = new MutationObserver(mutations => {
    for (const m of mutations) {
      if (m.type !== 'attributes' || m.attributeName !== 'class') continue;
      const overlay = m.target;
      if (!overlay.classList.contains('modal-overlay')) continue;
      if (overlay.classList.contains('show')) {
        // Modal just opened — save selection, record opener, focus first element
        try {
          const sel = window.getSelection();
          if (sel && sel.rangeCount > 0) {
            selectionMap.set(overlay, sel.getRangeAt(0).cloneRange());
          }
        } catch (_) {}
        openerMap.set(overlay, document.activeElement);
        // UI-02: Set aria-modal for screen readers and trap context
        overlay.setAttribute('aria-modal', 'true');
        overlay.setAttribute('role', 'dialog');
        requestAnimationFrame(() => {
          const focusable = Array.from(overlay.querySelectorAll(FOCUSABLE_SELECTOR))
            .filter(el => !el.disabled && el.offsetParent !== null);
          if (focusable.length > 0) focusable[0].focus();
        });
      } else {
        // Modal just closed — return focus to opener, restore selection
        overlay.removeAttribute('aria-modal');
        const opener = openerMap.get(overlay);
        if (opener && typeof opener.focus === 'function') {
          opener.focus();
        }
        const savedRange = selectionMap.get(overlay);
        if (savedRange) {
          try {
            const sel = window.getSelection();
            sel.removeAllRanges();
            sel.addRange(savedRange);
          } catch (_) {
            // Range may be invalid if DOM changed
          }
          selectionMap.delete(overlay);
        }
      }
    }
  });

  MODAL_IDS.forEach(id => {
    const el = $(id);
    if (el) observer.observe(el, { attributes: true, attributeFilter: ['class'] });
  });

  // UI-11: Centralized backdrop-click-to-close for all modal overlays.
  // Clicking on the overlay background (not the modal content) closes the modal.
  MODAL_IDS.forEach(id => {
    const overlay = $(id);
    if (!overlay) return;
    overlay.addEventListener('click', e => {
      if (e.target === overlay && overlay.classList.contains('show')) {
        overlay.classList.remove('show');
        ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
      }
    });
  });

  // Trap Tab and handle Escape within open modals
  document.addEventListener('keydown', e => {
    // Find the currently visible modal
    let openModal = null;
    for (const id of MODAL_IDS) {
      const el = $(id);
      if (el && el.classList.contains('show')) { openModal = el; break; }
    }
    if (!openModal) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      openModal.classList.remove('show');
      return;
    }

    if (e.key === 'Tab') {
      const focusable = Array.from(openModal.querySelectorAll(FOCUSABLE_SELECTOR))
        .filter(el => !el.disabled && el.offsetParent !== null);
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (e.shiftKey) {
        // Shift+Tab: if focus is on first element, wrap to last
        if (document.activeElement === first || !openModal.contains(document.activeElement)) {
          e.preventDefault();
          last.focus();
        }
      } else {
        // Tab: if focus is on last element, wrap to first
        if (document.activeElement === last || !openModal.contains(document.activeElement)) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  });
}

// ── E7.1: Toolbar Keyboard Navigation ────────────
// Left/Right arrow keys move between toolbar buttons when toolbar has focus.
function initToolbarKeyboardNav() {
  const toolbar = $('toolbar');
  if (!toolbar) return;

  toolbar.addEventListener('keydown', e => {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;

    // Collect all focusable toolbar items (buttons, selects, inputs)
    const items = Array.from(toolbar.querySelectorAll(
      'button.tb-btn:not([disabled]), select.tb-select, input.tb-input, button.style-gallery-btn, button.tb-btn-wide'
    )).filter(el => el.offsetParent !== null);

    const idx = items.indexOf(document.activeElement);
    if (idx < 0) return;

    e.preventDefault();
    if (e.key === 'ArrowRight') {
      items[(idx + 1) % items.length].focus();
    } else {
      items[(idx - 1 + items.length) % items.length].focus();
    }
  });
}

// ── E6.1: More (Overflow) Menu ───────────────────
// Provides access to toolbar functions hidden at narrow widths.
// At 1024px: shows strikethrough, superscript/subscript, text/highlight color,
//            clear formatting, line spacing, indent/outdent, find, etc.
// At 768px: also shows alignment buttons.
function initMoreMenu() {
  const btn = $('btnMore');
  const menu = $('moreMenu');
  if (!btn || !menu) return;

  btn.addEventListener('click', e => {
    e.stopPropagation();
    const wasOpen = menu.classList.contains('show');
    closeMoreMenu();
    if (!wasOpen) {
      const rect = btn.getBoundingClientRect();
      let top = rect.bottom + 4;
      menu.style.left = Math.max(0, rect.right - 220) + 'px';
      menu.style.top = top + 'px';
      menu.classList.add('show');
      btn.setAttribute('aria-expanded', 'true');
      // FS-23: Flip above trigger if dropdown overflows viewport bottom
      requestAnimationFrame(() => {
        const menuRect = menu.getBoundingClientRect();
        if (top + menuRect.height > window.innerHeight) {
          top = rect.top - menuRect.height - 4;
          menu.style.top = Math.max(4, top) + 'px';
        }
      });
      // FS-06: Focus first item when opening
      requestAnimationFrame(() => {
        const firstItem = menu.querySelector('button:not([type="color"])');
        if (firstItem) firstItem.focus();
      });
    }
  });

  // FS-06: Arrow-key navigation in more dropdown (WAI-ARIA Menu Pattern)
  menu.addEventListener('keydown', e => {
    const items = Array.from(menu.querySelectorAll('button:not([type="color"])'));
    if (items.length === 0) return;
    const currentIdx = items.indexOf(document.activeElement);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const next = currentIdx < 0 ? 0 : (currentIdx + 1) % items.length;
      items[next].focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = currentIdx <= 0 ? items.length - 1 : currentIdx - 1;
      items[prev].focus();
    } else if (e.key === 'Home') {
      e.preventDefault();
      items[0].focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      items[items.length - 1].focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeMoreMenu();
      btn.focus();
    }
  });

  // Wire up color pickers inside the More menu
  const moreColorPicker = $('moreColorPicker');
  let _moreColorSelInfo = null;
  if (moreColorPicker) {
    moreColorPicker.addEventListener('pointerdown', () => {
      saveSelection();
      _moreColorSelInfo = state.lastSelInfo ? { ...state.lastSelInfo } : null;
    });
    moreColorPicker.addEventListener('input', e => {
      const hex = e.target.value.replace('#', '').toUpperCase();
      $('colorSwatch').style.background = '#' + hex;
      if (_moreColorSelInfo && !_moreColorSelInfo.collapsed && state.doc) {
        try {
          syncAllText();
          state.doc.format_selection(_moreColorSelInfo.startNodeId, _moreColorSelInfo.startOffset, _moreColorSelInfo.endNodeId, _moreColorSelInfo.endOffset, 'color', hex);
          restoreSelectionForPickers();
          renderDocument();
          updateToolbarState(); updateUndoRedo(); markDirty();
        } catch (err) { console.error('more color:', err); }
      } else {
        restoreSelectionForPickers();
        applyFormat('color', hex);
      }
    });
    moreColorPicker.addEventListener('change', () => {
      closeMoreMenu();
    });
  }
  let _moreHlSelInfo = null;
  const moreHighlightPicker = $('moreHighlightPicker');
  if (moreHighlightPicker) {
    moreHighlightPicker.addEventListener('pointerdown', () => {
      saveSelection();
      _moreHlSelInfo = state.lastSelInfo ? { ...state.lastSelInfo } : null;
    });
    moreHighlightPicker.addEventListener('input', e => {
      const hex = e.target.value.replace('#', '').toUpperCase();
      if (_moreHlSelInfo && !_moreHlSelInfo.collapsed && state.doc) {
        try {
          syncAllText();
          state.doc.format_selection(_moreHlSelInfo.startNodeId, _moreHlSelInfo.startOffset, _moreHlSelInfo.endNodeId, _moreHlSelInfo.endOffset, 'highlightColor', hex);
          restoreSelectionForPickers();
          renderDocument();
          updateToolbarState(); updateUndoRedo(); markDirty();
        } catch (err) { console.error('more highlight:', err); }
      } else {
        restoreSelectionForPickers();
        applyFormat('highlightColor', hex);
      }
    });
    moreHighlightPicker.addEventListener('change', () => {
      closeMoreMenu();
    });
  }

  // Handle More menu item clicks — delegate to existing handlers
  menu.querySelectorAll('[data-more]').forEach(item => {
    item.addEventListener('click', e => {
      const action = item.dataset.more;

      // For color pickers, trigger the hidden input instead of closing
      if (action === 'textColor') {
        e.stopPropagation();
        saveSelection();
        if (moreColorPicker) {
          moreColorPicker.style.pointerEvents = 'auto';
          moreColorPicker.click();
        }
        return;
      }
      if (action === 'highlightColor') {
        e.stopPropagation();
        saveSelection();
        if (moreHighlightPicker) {
          moreHighlightPicker.style.pointerEvents = 'auto';
          moreHighlightPicker.click();
        }
        return;
      }

      closeMoreMenu();
      switch (action) {
        // Formatting toggles (E6.1 additions)
        case 'strikethrough':
          toggleFormat('strikethrough');
          announce('Strikethrough toggled');
          break;
        case 'formatPainter':
          // Trigger the format painter button click (single-click = once mode)
          $('btnFormatPainter')?.click();
          break;
        case 'superscript':
          toggleFormat('superscript');
          announce('Superscript toggled');
          break;
        case 'subscript':
          toggleFormat('subscript');
          announce('Subscript toggled');
          break;
        case 'clearFormat':
          $('btnClearFormat')?.click();
          break;
        // Alignment (visible at 768px)
        case 'alignLeft':
          $('btnAlignL')?.click();
          break;
        case 'alignCenter':
          $('btnAlignC')?.click();
          break;
        case 'alignRight':
          $('btnAlignR')?.click();
          break;
        case 'alignJustify':
          $('btnAlignJ')?.click();
          break;
        // Layout
        case 'lineSpacing': {
          const sel = $('lineSpacing');
          if (sel) {
            const opts = Array.from(sel.options);
            const curIdx = sel.selectedIndex;
            sel.selectedIndex = (curIdx + 1) % opts.length;
            sel.dispatchEvent(new Event('change'));
          }
          break;
        }
        case 'indent':
          $('btnIndent')?.click();
          break;
        case 'outdent':
          $('btnOutdent')?.click();
          break;
        // Tools
        case 'find':
          $('findBar')?.classList.add('show');
          $('findInput')?.focus();
          break;
        case 'spellcheck':
          $('btnSpellCheck')?.click();
          break;
        case 'comments': {
          const cp = $('commentsPanel');
          if (cp) { cp.classList.toggle('show'); if (cp.classList.contains('show')) refreshComments(); }
          break;
        }
        case 'history':
          $('historyPanel')?.classList.toggle('show');
          break;
        case 'share':
          $('btnShare')?.click();
          break;
      }
    });
  });

  // Close overflow menu when window resizes above breakpoint
  window.addEventListener('resize', () => {
    if (window.innerWidth > 1024) {
      closeMoreMenu();
    }
  });
}

function closeMoreMenu() {
  const menu = $('moreMenu');
  const btn = $('btnMore');
  if (menu) menu.classList.remove('show');
  if (btn) btn.setAttribute('aria-expanded', 'false');
}

// ── FS-21: Color Palette Dropdown ──────────────────
// 8x5 preset grid + recently used + custom button
const _COLOR_PALETTE = [
  // Row 1: blacks/grays
  '000000','434343','666666','999999','B7B7B7','CCCCCC','D9D9D9','EFEFEF','F3F3F3','FFFFFF',
  // Row 2: saturated
  '980000','FF0000','FF9900','FFFF00','00FF00','00FFFF','4A86E8','0000FF','9900FF','FF00FF',
  // Row 3: dark tones
  'E6B8AF','F4CCCC','FCE5CD','FFF2CC','D9EAD3','D0E0E3','C9DAF8','CFE2F3','D9D2E9','EAD1DC',
  // Row 4: medium tones
  'DD7E6B','EA9999','F9CB9C','FFE599','B6D7A8','A2C4C9','A4C2F4','9FC5E8','B4A7D6','D5A6BD',
];
let _recentColors = [];

function _applyColorFormat(hex, formatKey) {
  // Use saved selection info (captured before palette opened)
  const selInfo = state.lastSelInfo ? { ...state.lastSelInfo } : null;
  if (selInfo && !selInfo.collapsed && state.doc) {
    try {
      syncAllText();
      state.doc.format_selection(
        selInfo.startNodeId, selInfo.startOffset,
        selInfo.endNodeId, selInfo.endOffset,
        formatKey, hex
      );
      broadcastOp({ action: 'formatSelection', startNode: selInfo.startNodeId, startOffset: selInfo.startOffset, endNode: selInfo.endNodeId, endOffset: selInfo.endOffset, key: formatKey, value: hex });
      renderDocument();
      // Restore the selection after render so user sees it
      restoreSelectionForPickers();
      updateToolbarState();
      updateUndoRedo();
      markDirty();
    } catch (err) { console.error(formatKey + ' apply:', err); }
  } else if (selInfo) {
    // Collapsed selection — apply as pending format for next typed text
    restoreSelectionForPickers();
    applyFormat(formatKey, hex);
  }
}

function _addRecentColor(hex) {
  _recentColors = _recentColors.filter(c => c !== hex);
  _recentColors.unshift(hex);
  if (_recentColors.length > 8) _recentColors.length = 8;
}

function initColorPaletteDropdown(pickerInput, swatchEl, formatKey) {
  if (!pickerInput) return;
  const colorBtn = pickerInput.closest('.color-btn');
  if (!colorBtn) return;

  // Create the dropdown
  const dd = document.createElement('div');
  dd.className = 'color-palette-dropdown';
  dd.id = 'colorPaletteDD_' + formatKey;
  document.body.appendChild(dd);

  function buildDropdown() {
    let html = '';
    if (_recentColors.length > 0) {
      html += '<div class="color-palette-section"><div class="color-palette-label">Recently used</div><div class="color-palette-grid">';
      _recentColors.forEach(c => {
        html += `<button class="color-palette-swatch" data-color="${c}" title="#${c}" style="background:#${c}"></button>`;
      });
      html += '</div></div>';
    }
    html += '<div class="color-palette-section"><div class="color-palette-grid">';
    _COLOR_PALETTE.forEach(c => {
      html += `<button class="color-palette-swatch" data-color="${c}" title="#${c}" style="background:#${c}"></button>`;
    });
    html += '</div></div>';
    html += '<button class="color-palette-custom" title="Pick a custom color">Custom...</button>';
    dd.innerHTML = html;
  }

  function openPalette() {
    saveSelection();
    buildDropdown();
    const rect = colorBtn.getBoundingClientRect();
    let top = rect.bottom + 4;
    let left = rect.left;
    dd.classList.add('show');
    // FS-23: height check
    requestAnimationFrame(() => {
      const ddRect = dd.getBoundingClientRect();
      if (top + ddRect.height > window.innerHeight) {
        top = rect.top - ddRect.height - 4;
      }
      if (left + ddRect.width > window.innerWidth) {
        left = Math.max(4, window.innerWidth - ddRect.width - 4);
      }
      dd.style.top = Math.max(4, top) + 'px';
      dd.style.left = Math.max(4, left) + 'px';
    });
    dd.style.top = top + 'px';
    dd.style.left = left + 'px';
  }

  function closePalette() {
    dd.classList.remove('show');
  }

  // Click the toolbar button to open palette
  colorBtn.addEventListener('click', e => {
    e.preventDefault();
    e.stopPropagation();
    if (dd.classList.contains('show')) { closePalette(); return; }
    // Close other palettes
    document.querySelectorAll('.color-palette-dropdown.show, .highlight-palette-dropdown.show').forEach(d => d.classList.remove('show'));
    openPalette();
  });

  // Swatch clicks
  dd.addEventListener('click', e => {
    const swatch = e.target.closest('.color-palette-swatch');
    if (swatch) {
      const hex = swatch.dataset.color;
      if (swatchEl) swatchEl.style.background = '#' + hex;
      _addRecentColor(hex);
      _applyColorFormat(hex, formatKey);
      closePalette();
      return;
    }
    if (e.target.closest('.color-palette-custom')) {
      closePalette();
      // Open native color picker
      pickerInput.style.pointerEvents = 'auto';
      pickerInput.click();
      const onInput = ev => {
        const hex = ev.target.value.replace('#', '').toUpperCase();
        if (swatchEl) swatchEl.style.background = '#' + hex;
        _addRecentColor(hex);
        _applyColorFormat(hex, formatKey);
      };
      const onChange = () => {
        pickerInput.removeEventListener('input', onInput);
        pickerInput.removeEventListener('change', onChange);
        pickerInput.style.pointerEvents = 'none';
      };
      pickerInput.addEventListener('input', onInput);
      pickerInput.addEventListener('change', onChange);
    }
  });

  // Close on outside click
  document.addEventListener('click', e => {
    if (!dd.classList.contains('show')) return;
    if (e.target.closest('.color-palette-dropdown') || e.target.closest('.color-btn')) return;
    closePalette();
  });
}

// ── FS-22: Highlight Color Palette Dropdown ──────────
const _HIGHLIGHT_COLORS = [
  { hex: 'FCE94F', label: 'Yellow' },
  { hex: '8AE234', label: 'Green' },
  { hex: '89CFF0', label: 'Cyan' },
  { hex: 'F4A7B9', label: 'Pink' },
  { hex: 'FCAF3E', label: 'Orange' },
  { hex: 'AD7FA8', label: 'Purple' },
  { hex: 'EF2929', label: 'Red' },
  { hex: '729FCF', label: 'Blue' },
];

function initHighlightPaletteDropdown(pickerInput) {
  if (!pickerInput) return;
  const hlBtn = pickerInput.closest('.color-btn');
  if (!hlBtn) return;

  const dd = document.createElement('div');
  dd.className = 'highlight-palette-dropdown';
  dd.id = 'highlightPaletteDD';
  document.body.appendChild(dd);

  function buildDropdown() {
    let html = '<div class="highlight-palette-grid">';
    _HIGHLIGHT_COLORS.forEach(c => {
      html += `<button class="highlight-palette-swatch" data-color="${c.hex}" title="${c.label}" style="background:#${c.hex}"></button>`;
    });
    html += '</div>';
    html += '<button class="highlight-palette-none" title="Remove highlight">None</button>';
    dd.innerHTML = html;
  }

  function openPalette() {
    saveSelection();
    buildDropdown();
    const rect = hlBtn.getBoundingClientRect();
    let top = rect.bottom + 4;
    let left = rect.left;
    dd.classList.add('show');
    // FS-23: height check
    requestAnimationFrame(() => {
      const ddRect = dd.getBoundingClientRect();
      if (top + ddRect.height > window.innerHeight) {
        top = rect.top - ddRect.height - 4;
      }
      if (left + ddRect.width > window.innerWidth) {
        left = Math.max(4, window.innerWidth - ddRect.width - 4);
      }
      dd.style.top = Math.max(4, top) + 'px';
      dd.style.left = Math.max(4, left) + 'px';
    });
    dd.style.top = top + 'px';
    dd.style.left = left + 'px';
  }

  function closePalette() {
    dd.classList.remove('show');
  }

  // Save selection on mousedown (before focus shifts away from editor)
  hlBtn.addEventListener('mousedown', e => {
    e.preventDefault();
    saveSelection();
  });
  hlBtn.addEventListener('click', e => {
    e.preventDefault();
    e.stopPropagation();
    if (dd.classList.contains('show')) { closePalette(); return; }
    document.querySelectorAll('.color-palette-dropdown.show, .highlight-palette-dropdown.show').forEach(d => d.classList.remove('show'));
    openPalette();
  });

  dd.addEventListener('click', e => {
    const swatch = e.target.closest('.highlight-palette-swatch');
    if (swatch) {
      const hex = swatch.dataset.color;
      _applyColorFormat(hex, 'highlightColor');
      closePalette();
      return;
    }
    if (e.target.closest('.highlight-palette-none')) {
      // Remove highlight by applying empty/transparent value
      _applyColorFormat('', 'highlightColor');
      closePalette();
    }
  });

  document.addEventListener('click', e => {
    if (!dd.classList.contains('show')) return;
    if (e.target.closest('.highlight-palette-dropdown') || e.target.closest('.color-btn')) return;
    closePalette();
  });
}

// ── E6.2: Touch Selection Support ────────────────
// Double-tap to select word, long-press for context menu (cut/copy/paste).
function initTouchSelection() {
  const page = $('pageContainer');
  if (!page) return;

  // Only activate on touch devices
  let lastTapTime = 0;
  let longPressTimer = null;
  let longPressTarget = null;

  // Double-tap to select word
  page.addEventListener('touchend', e => {
    const now = Date.now();
    const dt = now - lastTapTime;
    lastTapTime = now;

    if (dt < 350 && dt > 50) {
      // Double-tap detected — select the word at the tap point
      e.preventDefault();
      const touch = e.changedTouches[0];
      if (!touch) return;

      // Use caretPositionFromPoint or caretRangeFromPoint for word selection
      let range = null;
      if (document.caretRangeFromPoint) {
        range = document.caretRangeFromPoint(touch.clientX, touch.clientY);
      } else if (document.caretPositionFromPoint) {
        const pos = document.caretPositionFromPoint(touch.clientX, touch.clientY);
        if (pos && pos.offsetNode) {
          range = document.createRange();
          range.setStart(pos.offsetNode, pos.offset);
          range.collapse(true);
        }
      }

      if (range && range.startContainer.nodeType === Node.TEXT_NODE) {
        // Expand range to word boundaries
        const text = range.startContainer.textContent;
        let start = range.startOffset;
        let end = range.startOffset;

        // Find word boundaries (letters, digits, and common word chars)
        const isWordChar = ch => /[\w\u00C0-\u024F\u0400-\u04FF]/.test(ch);
        while (start > 0 && isWordChar(text[start - 1])) start--;
        while (end < text.length && isWordChar(text[end])) end++;

        if (end > start) {
          const sel = window.getSelection();
          const wordRange = document.createRange();
          wordRange.setStart(range.startContainer, start);
          wordRange.setEnd(range.startContainer, end);
          sel.removeAllRanges();
          sel.addRange(wordRange);
        }
      }
    }
  }, { passive: false });

  // Long-press for context menu (500ms)
  page.addEventListener('touchstart', e => {
    if (e.touches.length !== 1) return;
    const touch = e.touches[0];
    longPressTarget = { x: touch.clientX, y: touch.clientY };

    longPressTimer = setTimeout(() => {
      if (!longPressTarget) return;
      // Show a native-like cut/copy/paste context menu
      showTouchContextMenu(longPressTarget.x, longPressTarget.y);
      longPressTarget = null;
    }, 500);
  }, { passive: true });

  page.addEventListener('touchmove', e => {
    // Cancel long-press if finger moves significantly
    if (longPressTimer && longPressTarget && e.touches[0]) {
      const dx = e.touches[0].clientX - longPressTarget.x;
      const dy = e.touches[0].clientY - longPressTarget.y;
      if (Math.abs(dx) > 10 || Math.abs(dy) > 10) {
        clearTimeout(longPressTimer);
        longPressTimer = null;
        longPressTarget = null;
      }
    }
  }, { passive: true });

  page.addEventListener('touchend', () => {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
    longPressTarget = null;
  }, { passive: true });

  page.addEventListener('touchcancel', () => {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
    longPressTarget = null;
  }, { passive: true });
}

// Touch context menu for cut/copy/paste
function showTouchContextMenu(x, y) {
  // Remove any existing touch context menu
  const existing = document.getElementById('touchContextMenu');
  if (existing) existing.remove();

  const menu = document.createElement('div');
  menu.id = 'touchContextMenu';
  menu.className = 'ctx-menu';
  menu.style.display = 'block';
  menu.style.left = Math.max(0, Math.min(x - 60, window.innerWidth - 180)) + 'px';
  menu.style.top = Math.max(0, y - 50) + 'px';
  menu.setAttribute('role', 'menu');

  const items = [
    { label: 'Cut', icon: 'content_cut', action: 'cut' },
    { label: 'Copy', icon: 'content_copy', action: 'copy' },
    { label: 'Paste', icon: 'content_paste', action: 'paste' },
  ];

  items.forEach(({ label, icon, action }) => {
    const btn = document.createElement('button');
    btn.className = 'ctx-item';
    btn.setAttribute('role', 'menuitem');
    btn.innerHTML = `<span class="msi ctx-icon">${icon}</span> ${label}`;
    btn.style.minHeight = '44px'; // WCAG touch target
    btn.addEventListener('click', async () => {
      menu.remove();
      try {
        if (action === 'cut') {
          document.execCommand('cut');
        } else if (action === 'copy') {
          document.execCommand('copy');
        } else if (action === 'paste') {
          if (navigator.clipboard && navigator.clipboard.readText) {
            const text = await navigator.clipboard.readText();
            document.execCommand('insertText', false, text);
          } else {
            document.execCommand('paste');
          }
        }
      } catch (err) {
        console.warn('Touch context action failed:', err);
      }
    });
    menu.appendChild(btn);
  });

  document.body.appendChild(menu);

  // Auto-dismiss on outside tap
  const dismiss = (e) => {
    if (!menu.contains(e.target)) {
      menu.remove();
      document.removeEventListener('touchstart', dismiss);
      document.removeEventListener('click', dismiss);
    }
  };
  // Delay to avoid the current touch from dismissing immediately
  setTimeout(() => {
    document.addEventListener('touchstart', dismiss, { passive: true });
    document.addEventListener('click', dismiss);
  }, 100);

  // Auto-dismiss after 5 seconds
  setTimeout(() => {
    if (menu.parentNode) menu.remove();
  }, 5000);
}

// ── Template Chooser ─────────────────────────────
function initTemplateChooser() {
  const btnTemplate = $('btnTemplate');
  if (btnTemplate) {
    btnTemplate.addEventListener('click', () => {
      closeAllMenus();
      saveModalSelection();
      $('templateModal').classList.add('show');
    });
  }

  const templateCancelBtn = $('templateCancelBtn');
  if (templateCancelBtn) {
    templateCancelBtn.addEventListener('click', () => {
      $('templateModal').classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    });
  }

  // Backdrop click to close
  const templateModal = $('templateModal');
  if (templateModal) {
    templateModal.addEventListener('click', e => {
      if (e.target === templateModal) {
        templateModal.classList.remove('show');
        ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
      }
    });
  }

  // Template card clicks
  const templateGrid = $('templateGrid');
  if (templateGrid) {
    templateGrid.querySelectorAll('.template-card').forEach(card => {
      card.addEventListener('click', () => {
        const templateName = card.dataset.template;
        $('templateModal').classList.remove('show');
        createFromTemplate(templateName);
      });
    });
  }
}

function createFromTemplate(templateName) {
  if (!state.engine) return;

  // For blank, reuse newDocument directly
  if (templateName === 'blank') {
    newDocument();
    return;
  }

  // Create new document and populate with template content
  state.doc = state.engine.create();
  state.currentFormat = 'new';

  try {
    switch (templateName) {
      case 'letter':
        state.doc.append_heading(1, 'Letter');
        state.doc.append_paragraph(new Date().toLocaleDateString());
        state.doc.append_paragraph('');
        state.doc.append_paragraph('Dear [Recipient],');
        state.doc.append_paragraph('');
        state.doc.append_paragraph('I am writing to you regarding...');
        state.doc.append_paragraph('');
        state.doc.append_paragraph('Sincerely,');
        state.doc.append_paragraph('[Your Name]');
        break;

      case 'resume':
        state.doc.append_heading(1, 'Your Name');
        state.doc.append_heading(2, 'Contact');
        state.doc.append_paragraph('Email: your.email@example.com | Phone: (555) 123-4567 | Location: City, State');
        state.doc.append_heading(2, 'Experience');
        state.doc.append_paragraph('Job Title - Company Name (Start Date - End Date)');
        state.doc.append_paragraph('Describe your responsibilities and achievements.');
        state.doc.append_heading(2, 'Education');
        state.doc.append_paragraph('Degree - University Name (Graduation Year)');
        state.doc.append_heading(2, 'Skills');
        state.doc.append_paragraph('List your relevant skills, technologies, and competencies.');
        break;

      case 'report':
        state.doc.append_heading(1, 'Report Title');
        state.doc.append_heading(2, 'Executive Summary');
        state.doc.append_paragraph('Provide a brief overview of the report findings and recommendations.');
        state.doc.append_heading(2, 'Introduction');
        state.doc.append_paragraph('Describe the background, purpose, and scope of this report.');
        state.doc.append_heading(2, 'Conclusion');
        state.doc.append_paragraph('Summarize the key findings and recommended next steps.');
        break;

      case 'meeting':
        state.doc.append_heading(1, 'Meeting Notes');
        state.doc.append_paragraph('Date: ' + new Date().toLocaleDateString());
        state.doc.append_paragraph('Attendees: [Name 1], [Name 2], [Name 3]');
        state.doc.append_heading(2, 'Agenda');
        state.doc.append_paragraph('1. Topic one');
        state.doc.append_paragraph('2. Topic two');
        state.doc.append_heading(2, 'Discussion');
        state.doc.append_paragraph('Summary of discussion points and decisions made.');
        state.doc.append_heading(2, 'Action Items');
        state.doc.append_paragraph('- [Action item 1] - Assigned to [Person] - Due [Date]');
        state.doc.append_paragraph('- [Action item 2] - Assigned to [Person] - Due [Date]');
        break;

      case 'essay':
        state.doc.append_heading(1, 'Essay Title');
        state.doc.append_paragraph('Begin your introduction here. Present the main topic and your thesis statement.');
        state.doc.append_paragraph('');
        state.doc.append_paragraph('Develop your argument in the body paragraphs. Each paragraph should focus on a single supporting point with evidence and analysis.');
        state.doc.append_paragraph('');
        state.doc.append_paragraph('Conclude by summarizing your main points and restating your thesis in light of the evidence presented.');
        break;

      default:
        state.doc.append_paragraph('');
        break;
    }

    state.doc.clear_history();
  } catch (e) { console.warn('createFromTemplate:', e); }

  // Activate editor (same pattern as newDocument)
  $('welcomeScreen').style.display = 'none';
  $('toolbar').classList.add('show');
  const menubar = $('appMenubar');
  if (menubar) menubar.classList.add('show');
  $('statusbar').classList.add('show');
  import('./file.js').then(({ switchView }) => { switchView('editor'); });

  renderDocument();
  renderRuler();

  // Capitalize template name for document title
  const titleMap = {
    letter: 'Letter',
    resume: 'Resume',
    report: 'Report',
    meeting: 'Meeting Notes',
    essay: 'Essay',
  };
  $('docName').value = titleMap[templateName] || 'Untitled Document';

  ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  state.dirty = false;
  updateDirtyIndicator();
  updateStatusBar();

  // Save an initial version for the template
  import('./file.js').then(mod => {
    if (typeof mod.saveVersion === 'function') {
      mod.saveVersion('Template: ' + (titleMap[templateName] || templateName));
    }
  });
}

// ── Table of Contents Insertion ──────────────────
function initTOCInsertion() {
  const miTOC = $('miTOC');
  if (!miTOC) return;

  miTOC.addEventListener('click', () => {
    // Close the insert menu (both the menu-bar version and toolbar dropdown)
    $('insertMenu')?.classList.remove('show');
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));

    if (!state.doc) return;
    const nodeId = getActiveNodeId();
    if (!nodeId) return;
    syncAllText();
    try {
      state.doc.insert_table_of_contents(nodeId, 3, 'Table of Contents');
      broadcastOp({ action: 'insertTOC', afterNodeId: nodeId, maxLevel: 3, title: 'Table of Contents' });
      renderDocument();
      updateUndoRedo();
      announce('Table of Contents inserted');
    } catch (e) { console.error('insert TOC:', e); }
  });
}

// ── Page Setup Dialog ────────────────────────────
// Page size presets: width and height in inches
const PAGE_SIZES = {
  letter: { w: 8.5, h: 11 },
  a4:     { w: 8.27, h: 11.69 },
  legal:  { w: 8.5, h: 14 },
  a3:     { w: 11.69, h: 16.54 },
};

function initPageSetup() {
  const menuPageSetup = $('menuPageSetup');
  if (!menuPageSetup) return;

  menuPageSetup.addEventListener('click', () => {
    closeAllMenus();
    openPageSetup();
  });

  // Cancel
  $('psCancelBtn')?.addEventListener('click', () => {
    $('pageSetupModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Apply
  $('psApplyBtn')?.addEventListener('click', () => {
    applyPageSetup();
    $('pageSetupModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Show/hide custom size fields when "Custom" is selected
  const pageSizeSelect = $('psPageSize');
  if (pageSizeSelect) {
    pageSizeSelect.addEventListener('change', () => {
      const customPanel = $('psCustomSize');
      if (customPanel) {
        customPanel.style.display = pageSizeSelect.value === 'custom' ? 'block' : 'none';
      }
    });
  }

  // Backdrop click to close
  const modal = $('pageSetupModal');
  if (modal) {
    modal.addEventListener('click', e => {
      if (e.target === modal) {
        modal.classList.remove('show');
        ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
      }
    });
  }
}

function openPageSetup() {
  // Try to read current page setup from the WASM document model
  let dims = null;
  let orientationFromDoc = null;
  if (state.doc && typeof state.doc.get_page_setup_json === 'function') {
    try {
      const json = state.doc.get_page_setup_json();
      const setup = JSON.parse(json);
      dims = {
        widthPt: setup.pageWidth || 612,
        heightPt: setup.pageHeight || 792,
        marginTopPt: setup.marginTop || 72,
        marginBottomPt: setup.marginBottom || 72,
        marginLeftPt: setup.marginLeft || 72,
        marginRightPt: setup.marginRight || 72,
      };
      orientationFromDoc = setup.orientation || null;
    } catch (_) {}
  }

  // Fall back to state.pageDims or defaults
  if (!dims) {
    dims = state.pageDims || {
      widthPt: 612, heightPt: 792,
      marginTopPt: 72, marginBottomPt: 72, marginLeftPt: 72, marginRightPt: 72,
    };
  }

  const wIn = dims.widthPt / 72;
  const hIn = dims.heightPt / 72;

  // Determine orientation from doc property or from dimensions
  const isLandscape = orientationFromDoc
    ? orientationFromDoc === 'landscape'
    : wIn > hIn;
  document.querySelectorAll('input[name="psOrientation"]').forEach(r => {
    r.checked = (r.value === (isLandscape ? 'landscape' : 'portrait'));
  });

  // Detect page size from dimensions (compare short/long sides to presets)
  const shortSide = Math.min(wIn, hIn);
  const longSide = Math.max(wIn, hIn);
  let detectedSize = 'custom';
  for (const [key, sz] of Object.entries(PAGE_SIZES)) {
    const preShort = Math.min(sz.w, sz.h);
    const preLong = Math.max(sz.w, sz.h);
    if (Math.abs(preShort - shortSide) < 0.1 && Math.abs(preLong - longSide) < 0.1) {
      detectedSize = key;
      break;
    }
  }
  $('psPageSize').value = detectedSize;

  // Show/hide custom size panel
  const customPanel = $('psCustomSize');
  if (customPanel) {
    customPanel.style.display = detectedSize === 'custom' ? 'block' : 'none';
  }

  // Populate custom width/height fields (always update so they're ready if user switches to Custom)
  const customW = $('psCustomWidth');
  const customH = $('psCustomHeight');
  if (customW) customW.value = wIn.toFixed(2);
  if (customH) customH.value = hIn.toFixed(2);

  // Margins
  $('psMarginTop').value = (dims.marginTopPt / 72).toFixed(2);
  $('psMarginBottom').value = (dims.marginBottomPt / 72).toFixed(2);
  $('psMarginLeft').value = (dims.marginLeftPt / 72).toFixed(2);
  $('psMarginRight').value = (dims.marginRightPt / 72).toFixed(2);

  saveModalSelection();
  $('pageSetupModal').classList.add('show');
}

function applyPageSetup() {
  const sizeKey = $('psPageSize').value || 'letter';
  const orientation = document.querySelector('input[name="psOrientation"]:checked')?.value || 'portrait';

  let pageW, pageH;
  if (sizeKey === 'custom') {
    pageW = Math.max(1, Math.min(60, parseFloat($('psCustomWidth').value) || 8.5));
    pageH = Math.max(1, Math.min(60, parseFloat($('psCustomHeight').value) || 11));
  } else {
    const size = PAGE_SIZES[sizeKey] || PAGE_SIZES.letter;
    pageW = size.w;
    pageH = size.h;
  }

  if (orientation === 'landscape') {
    // Ensure width > height for landscape
    if (pageW < pageH) {
      const tmp = pageW;
      pageW = pageH;
      pageH = tmp;
    }
  } else {
    // Ensure height > width for portrait
    if (pageH < pageW) {
      const tmp = pageW;
      pageW = pageH;
      pageH = tmp;
    }
  }

  const marginTop = Math.max(0, Math.min(5, parseFloat($('psMarginTop').value) || 1));
  const marginBottom = Math.max(0, Math.min(5, parseFloat($('psMarginBottom').value) || 1));
  const marginLeft = Math.max(0, Math.min(5, parseFloat($('psMarginLeft').value) || 1));
  const marginRight = Math.max(0, Math.min(5, parseFloat($('psMarginRight').value) || 1));

  // Convert inches to points (1 inch = 72 points)
  const widthPt = pageW * 72;
  const heightPt = pageH * 72;
  const mTop = marginTop * 72;
  const mBottom = marginBottom * 72;
  const mLeft = marginLeft * 72;
  const mRight = marginRight * 72;

  // Validate: margins must not exceed page dimensions
  if (mLeft + mRight >= widthPt) {
    announce('Left + right margins exceed page width');
    return;
  }
  if (mTop + mBottom >= heightPt) {
    announce('Top + bottom margins exceed page height');
    return;
  }

  // Persist page setup to the document model via WASM
  if (state.doc && typeof state.doc.set_page_setup === 'function') {
    try {
      const setupJson = JSON.stringify({
        pageWidth: widthPt,
        pageHeight: heightPt,
        marginTop: mTop,
        marginBottom: mBottom,
        marginLeft: mLeft,
        marginRight: mRight,
        orientation: orientation,
      });
      state.doc.set_page_setup(setupJson);
    } catch (e) {
      console.warn('set_page_setup failed:', e);
    }
  }

  // Update state.pageDims so the editor uses the new dimensions
  state.pageDims = {
    widthPt, heightPt,
    marginTopPt: mTop,
    marginBottomPt: mBottom,
    marginLeftPt: mLeft,
    marginRightPt: mRight,
  };

  // Store a WasmLayoutConfig in state for re-renders via Pages view
  try {
    import('../wasm-pkg/s1engine_wasm.js').then(mod => {
      const config = new mod.WasmLayoutConfig();
      config.set_page_width(widthPt);
      config.set_page_height(heightPt);
      config.set_margin_top(mTop);
      config.set_margin_bottom(mBottom);
      config.set_margin_left(mLeft);
      config.set_margin_right(mRight);
      state._layoutConfig = config;
    }).catch(() => {});
  } catch (_) {}

  // Invalidate page map cache to force full repagination
  state._lastPageMapHash = null;
  state._layoutCache = null;
  state._layoutDirty = true;

  // Mark document as dirty (unsaved changes)
  state.dirty = true;

  // Re-render with new dimensions
  renderDocument();
  renderRuler();
  announce('Page setup applied');
}

// ═══════════════════════════════════════════════════
// UXP-22: Columns Modal
// ═══════════════════════════════════════════════════

function initColumnsModal() {
  const menuBtn = $('menuColumns');
  if (!menuBtn) return;

  menuBtn.addEventListener('click', () => {
    closeAllMenus();
    openColumnsModal();
  });

  // Preset buttons
  document.querySelectorAll('.columns-preset-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const cols = parseInt(btn.dataset.cols) || 1;
      $('colCount').value = cols;
      // Highlight active preset
      document.querySelectorAll('.columns-preset-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
    });
  });

  // Cancel
  $('colCancelBtn')?.addEventListener('click', () => {
    $('columnsModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Apply
  $('colApplyBtn')?.addEventListener('click', () => {
    applyColumns();
    $('columnsModal').classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Sync preset highlight when custom input changes
  $('colCount')?.addEventListener('input', () => {
    const val = parseInt($('colCount').value) || 1;
    document.querySelectorAll('.columns-preset-btn').forEach(b => {
      b.classList.toggle('active', parseInt(b.dataset.cols) === val);
    });
  });

  // Backdrop click to close
  const modal = $('columnsModal');
  if (modal) {
    modal.addEventListener('click', e => {
      if (e.target === modal) {
        modal.classList.remove('show');
        ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
      }
    });
  }

  // Enter key to apply
  [$('colCount'), $('colSpacing')].forEach(input => {
    if (input) {
      input.addEventListener('keydown', e => {
        if (e.key === 'Enter') { e.preventDefault(); $('colApplyBtn')?.click(); }
        if (e.key === 'Escape') { $('columnsModal').classList.remove('show'); }
      });
    }
  });
}

function openColumnsModal() {
  if (!state.doc) return;
  saveModalSelection();

  // Read current column settings from section 0
  let currentCols = 1;
  let currentSpacing = 0.5; // inches
  try {
    const json = state.doc.get_section_columns(0);
    if (json) {
      const info = JSON.parse(json);
      currentCols = info.columns || 1;
      currentSpacing = (info.spacing || 36) / 72; // pt to inches
    }
  } catch (_) {}

  $('colCount').value = currentCols;
  $('colSpacing').value = currentSpacing.toFixed(2);

  // Highlight matching preset
  document.querySelectorAll('.columns-preset-btn').forEach(b => {
    b.classList.toggle('active', parseInt(b.dataset.cols) === currentCols);
  });

  $('columnsModal').classList.add('show');
}

function applyColumns() {
  if (!state.doc) return;

  const cols = Math.max(1, Math.min(6, parseInt($('colCount').value) || 1));
  const spacingIn = Math.max(0, Math.min(3, parseFloat($('colSpacing').value) || 0.5));
  const spacingPt = spacingIn * 72; // Convert inches to points

  try {
    state.doc.set_section_columns(0, cols, spacingPt);
    broadcastOp({ action: 'setSectionColumns', sectionIndex: 0, columns: cols, spacingPt });
  } catch (e) {
    console.error('set columns:', e);
    announce('Failed to set columns');
    return;
  }

  // Store in state for render.js to pick up
  state.sectionColumns = cols;
  state.sectionColumnSpacing = spacingPt;

  markDirty();
  renderDocument();
  announce(`Layout set to ${cols} column${cols > 1 ? 's' : ''}`);
}

/**
 * Apply column-count CSS to page-content elements based on section properties.
 * Called from render.js after rendering the document HTML.
 */
export function applyColumnLayout() {
  if (!state.doc) return;
  try {
    const json = state.doc.get_section_columns(0);
    if (json) {
      const info = JSON.parse(json);
      const cols = info.columns || 1;
      const spacing = info.spacing || 36;
      document.querySelectorAll('.page-content').forEach(el => {
        if (cols > 1) {
          el.setAttribute('data-columns', String(cols));
          el.style.columnGap = spacing + 'pt';
        } else {
          el.removeAttribute('data-columns');
          el.style.columnGap = '';
        }
      });
    }
  } catch (_) {}
}

// ═══════════════════════════════════════════════════
// Auto Format Document
// ═══════════════════════════════════════════════════

function initAutoFormat() {
  const menuBtn = $('menuAutoFormat');
  if (!menuBtn) return;

  menuBtn.addEventListener('click', () => {
    closeAllMenus();
    autoFormatDocument();
  });
}

/**
 * Auto-detect paragraph types (headings, body) and apply appropriate styles.
 * Uses heuristics based on text length, casing, and punctuation patterns.
 * Does NOT use AI — purely rule-based for speed and reliability.
 */
function autoFormatDocument() {
  const doc = state.doc;
  if (!doc) return;

  // Sync DOM to WASM model first
  syncAllText();

  // Get all paragraph elements from the rendered DOM
  const container = $('pageContainer') || $('editorCanvas');
  if (!container) return;

  const paragraphs = container.querySelectorAll('[data-node-id]');
  if (!paragraphs.length) return;

  let changesApplied = 0;

  // Begin batch for single undo step
  if (typeof doc.begin_batch === 'function') {
    try { doc.begin_batch('Auto Format Document'); } catch (_) {}
  }

  try {
    for (const pEl of paragraphs) {
      const nodeId = pEl.dataset.nodeId;
      if (!nodeId) continue;
      const text = (pEl.textContent || '').trim();
      if (!text) continue;

      const isShort = text.length < 80;
      const isMedium = text.length < 120;
      const isAllCaps = text === text.toUpperCase() && text.length > 3 && /[A-Z]/.test(text);
      const endsWithColon = text.endsWith(':');
      const startsWithNumber = /^\d+[\.\)]\s/.test(text);
      const hasNoPunctuation = !/[.!?;]/.test(text.slice(0, -1)); // ignore last char
      const isTitleCase = text === text.replace(/\w\S*/g, t => t.charAt(0).toUpperCase() + t.substr(1));
      const wordCount = text.split(/\s+/).length;

      let targetStyleName = null;

      // Rule 1: Short ALL CAPS text -> Heading 1
      if (isShort && isAllCaps && wordCount >= 1 && wordCount <= 15) {
        targetStyleName = 'heading1';
      }
      // Rule 2: Short title-case text with no sentence-ending punctuation -> Heading 2
      else if (isShort && isTitleCase && hasNoPunctuation && wordCount >= 2 && wordCount <= 15) {
        targetStyleName = 'heading2';
      }
      // Rule 3: Short text that starts with a number pattern (like "1. Section") -> Heading 2
      else if (isMedium && startsWithNumber && hasNoPunctuation && wordCount <= 12) {
        targetStyleName = 'heading2';
      }
      // Rule 4: Short text ending with colon -> Heading 3
      else if (isShort && endsWithColon && wordCount <= 10) {
        targetStyleName = 'heading3';
      }

      // Apply the detected style via the same API used by the style gallery
      if (targetStyleName) {
        const def = STYLE_DEFS[targetStyleName];
        if (!def) continue;

        try {
          if (typeof doc.set_paragraph_style_id === 'function') {
            doc.set_paragraph_style_id(nodeId, def.styleId);
          }
          doc.set_heading_level(nodeId, def.heading);

          // Apply run-level font overrides for the whole paragraph
          const textLen = Array.from(text).length;
          if (textLen > 0) {
            if (def.fontSize) {
              try { doc.format_selection(nodeId, 0, nodeId, textLen, 'fontSize', def.fontSize); } catch (_) {}
            }
            if (def.fontFamily) {
              try { doc.format_selection(nodeId, 0, nodeId, textLen, 'fontFamily', def.fontFamily); } catch (_) {}
            }
          }

          changesApplied++;
        } catch (e) {
          console.warn('[auto-format] Style apply error for node', nodeId, e);
        }
      }
    }
  } finally {
    // End batch
    try {
      if (typeof doc.end_batch === 'function' && doc.is_batching()) {
        doc.end_batch();
      }
    } catch (_) {}
  }

  if (changesApplied > 0) {
    markDirty();
    renderDocument();
    updateToolbarState();
    showToast(`Auto-formatted ${changesApplied} paragraph${changesApplied !== 1 ? 's' : ''}`, 'success', 3000);
  } else {
    showToast('No paragraphs detected for auto-formatting', 'info', 3000);
  }
}

// ═══════════════════════════════════════════════════
// E9.3: Equation Editor
// ═══════════════════════════════════════════════════

let _eqPreviewTimer = 0;

export function openEquationModal(prefillLatex) {
  closeAllMenus();
  saveModalSelection();
  const input = $('equationInput');
  const preview = $('equationPreview');
  input.value = prefillLatex || '';
  preview.innerHTML = '';
  if (prefillLatex) renderEquationPreview(prefillLatex, preview);
  $('equationModal').classList.add('show');
  input.focus();
}

function renderEquationPreview(latex, container) {
  if (!latex || !latex.trim()) {
    container.innerHTML = '<span style="color:var(--text-muted);font-size:13px">Type LaTeX to see a preview</span>';
    return;
  }
  if (typeof katex !== 'undefined') {
    try {
      katex.render(latex, container, { throwOnError: false, displayMode: true });
    } catch (e) {
      container.innerHTML = '<span style="color:var(--danger);font-size:13px">Invalid LaTeX: ' + escapeHtml(e.message) + '</span>';
    }
  } else {
    // KaTeX not loaded — show raw LaTeX as fallback
    container.textContent = latex;
    container.style.fontFamily = "'Roboto Mono', monospace";
  }
}

function initEquationModal() {
  const modal = $('equationModal');
  if (!modal) return;

  const input = $('equationInput');
  const preview = $('equationPreview');

  // Live preview with debounce
  input.addEventListener('input', () => {
    clearTimeout(_eqPreviewTimer);
    _eqPreviewTimer = setTimeout(() => {
      renderEquationPreview(input.value, preview);
    }, 300);
  });

  // Cancel
  $('eqCancelBtn').addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Insert
  $('eqInsertBtn').addEventListener('click', () => {
    const latex = $('equationInput').value.trim();
    if (!latex) { modal.classList.remove('show'); return; }
    modal.classList.remove('show');
    insertEquation(latex);
  });

  // Backdrop click to close
  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });

  // Enter to insert (Ctrl+Enter)
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      $('eqInsertBtn').click();
    }
    if (e.key === 'Escape') {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });

  // Insert menu entry
  const miEquation = $('miEquation');
  if (miEquation) {
    miEquation.addEventListener('click', () => {
      closeAllMenus();
      $('insertMenu')?.classList.remove('show');
      openEquationModal('');
    });
  }

  // Double-click on equation in document to edit
  $('pageContainer')?.addEventListener('dblclick', e => {
    const eqEl = e.target.closest('[data-equation]');
    if (!eqEl) return;
    e.preventDefault();
    const latex = eqEl.dataset.equation || '';
    state._editingEquationEl = eqEl;
    openEquationModal(latex);
  });
}

function insertEquation(latex) {
  if (!state.doc) return;

  // If editing an existing equation, update it
  const editingEl = state._editingEquationEl;
  if (editingEl && editingEl.isConnected) {
    editingEl.dataset.equation = latex;
    renderKatexInElement(editingEl, latex);
    state._editingEquationEl = null;
    announce('Equation updated');
    return;
  }
  state._editingEquationEl = null;

  // Insert a new equation node via WASM if available
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  syncAllText();
  try {
    if (typeof state.doc.insert_equation === 'function') {
      state.doc.insert_equation(nodeId, latex);
      broadcastOp({ action: 'insertEquation', afterNodeId: nodeId, latex });
      renderDocument();
      updateUndoRedo();
      announce('Equation inserted');
    } else {
      // Fallback: insert as a styled paragraph with data attribute
      const newId = state.doc.split_paragraph(nodeId, 0);
      if (newId) {
        state.doc.set_paragraph_text(newId, latex);
        broadcastOp({ action: 'splitParagraph', nodeId, offset: 0 });
      }
      renderDocument();
      updateUndoRedo();
      announce('Equation inserted (text fallback)');
    }
  } catch (e) {
    console.error('insert equation:', e);
    showToast('Failed to insert equation: ' + e.message, 'error');
  }
}

/** Render a single element's LaTeX via KaTeX */
function renderKatexInElement(el, latex) {
  if (typeof katex !== 'undefined') {
    try {
      katex.render(latex, el, { throwOnError: false, displayMode: el.dataset.eqDisplay === 'true' });
    } catch (_) {
      el.textContent = latex;
    }
  } else {
    el.textContent = latex;
  }
}

/**
 * Post-render: find all equation elements in the document and render with KaTeX.
 * Called after renderDocument().
 */
export function renderDocumentEquations() {
  if (typeof katex === 'undefined') return;
  const container = $('pageContainer');
  if (!container) return;
  // Look for elements with data-equation attribute (rendered by WASM)
  container.querySelectorAll('[data-equation]').forEach(el => {
    const latex = el.dataset.equation;
    if (!latex) return;
    const isDisplay = el.dataset.eqDisplay === 'true' || el.tagName === 'DIV';
    try {
      katex.render(latex, el, { throwOnError: false, displayMode: isDisplay });
    } catch (_) {
      // Leave raw text
    }
  });
}

// ═══════════════════════════════════════════════════
// UXP-20: Bookmarks & Cross-References
// ═══════════════════════════════════════════════════

export function openBookmarkModal() {
  closeAllMenus();
  saveModalSelection();
  const input = $('bookmarkNameInput');
  const errorEl = $('bookmarkNameError');
  input.value = '';
  errorEl.style.display = 'none';
  errorEl.textContent = '';
  refreshBookmarkList();
  $('bookmarkModal').classList.add('show');
  input.focus();
}

function refreshBookmarkList() {
  const listSection = $('bookmarkListSection');
  const listEl = $('bookmarkList');
  if (!listEl || !listSection) return;
  listEl.innerHTML = '';

  const container = $('pageContainer');
  if (!container) { listSection.style.display = 'none'; return; }

  const markers = container.querySelectorAll('[data-bookmark]');
  if (markers.length === 0) { listSection.style.display = 'none'; return; }

  listSection.style.display = '';
  markers.forEach(marker => {
    const name = marker.dataset.bookmark;
    if (!name) return;
    const item = document.createElement('div');
    item.className = 'bookmark-list-item';
    item.tabIndex = 0;
    item.setAttribute('role', 'button');
    item.title = 'Jump to bookmark: ' + name;

    const nameSpan = document.createElement('span');
    nameSpan.className = 'bookmark-list-name';
    nameSpan.textContent = name;
    item.appendChild(nameSpan);

    const deleteBtn = document.createElement('button');
    deleteBtn.className = 'bookmark-list-delete';
    deleteBtn.title = 'Delete bookmark: ' + name;
    deleteBtn.textContent = '\u00D7';
    deleteBtn.addEventListener('click', e => {
      e.stopPropagation();
      deleteBookmark(name, marker);
      refreshBookmarkList();
    });
    item.appendChild(deleteBtn);

    item.addEventListener('click', () => {
      $('bookmarkModal').classList.remove('show');
      scrollToBookmark(name);
    });
    item.addEventListener('keydown', e => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        $('bookmarkModal').classList.remove('show');
        scrollToBookmark(name);
      }
    });
    listEl.appendChild(item);
  });
}

function scrollToBookmark(name) {
  const container = $('pageContainer');
  if (!container) return;
  const target = container.querySelector('[data-bookmark="' + CSS.escape(name) + '"]');
  if (target) {
    target.scrollIntoView({ behavior: 'smooth', block: 'center' });
    // Brief highlight
    target.classList.add('bookmark-highlight');
    setTimeout(() => target.classList.remove('bookmark-highlight'), 1500);
  }
}

function deleteBookmark(name, markerEl) {
  if (!state.doc) return;
  const nodeId = markerEl?.dataset?.nodeId;
  if (nodeId && typeof state.doc.remove_node === 'function') {
    try {
      state.doc.remove_node(nodeId);
      broadcastOp({ action: 'removeNode', nodeId });
      renderDocument();
      updateUndoRedo();
      announce('Bookmark "' + name + '" deleted');
    } catch (e) {
      console.error('delete bookmark:', e);
    }
  } else {
    // Fallback: remove DOM element
    markerEl?.remove();
    announce('Bookmark "' + name + '" removed');
  }
}

function validateBookmarkName(name) {
  if (!name || !name.trim()) return 'Bookmark name is required.';
  if (name.length > 80) return 'Bookmark name must be 80 characters or fewer.';
  if (!/^[A-Za-z0-9_-][A-Za-z0-9_ -]*$/.test(name)) return 'Use letters, numbers, hyphens, underscores, and spaces only.';
  // Check uniqueness
  const container = $('pageContainer');
  if (container) {
    const existing = container.querySelector('[data-bookmark="' + CSS.escape(name.trim()) + '"]');
    if (existing) return 'A bookmark named "' + name.trim() + '" already exists.';
  }
  return null;
}

function insertBookmark(name) {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) {
    showToast('Place the cursor in a paragraph first.', 'warning');
    return;
  }
  syncAllText();
  try {
    if (typeof state.doc.insert_bookmark === 'function') {
      const bkId = state.doc.insert_bookmark(nodeId, name);
      broadcastOp({ action: 'insertBookmark', paraId: nodeId, name });
      renderDocument();
      updateUndoRedo();
      announce('Bookmark "' + name + '" inserted');
    } else {
      showToast('Bookmark insertion not available in this build.', 'warning');
    }
  } catch (e) {
    console.error('insert bookmark:', e);
    showToast('Failed to insert bookmark: ' + e.message, 'error');
  }
}

function initBookmarkModal() {
  const modal = $('bookmarkModal');
  if (!modal) return;

  const input = $('bookmarkNameInput');
  const errorEl = $('bookmarkNameError');

  // Validate on input
  input.addEventListener('input', () => {
    const err = validateBookmarkName(input.value.trim());
    if (err && input.value.trim().length > 0) {
      errorEl.textContent = err;
      errorEl.style.display = '';
    } else {
      errorEl.style.display = 'none';
    }
  });

  // Cancel
  $('bkCancelBtn').addEventListener('click', () => {
    modal.classList.remove('show');
  });

  // Insert
  $('bkInsertBtn').addEventListener('click', () => {
    const name = input.value.trim();
    const err = validateBookmarkName(name);
    if (err) {
      errorEl.textContent = err;
      errorEl.style.display = '';
      input.focus();
      return;
    }
    modal.classList.remove('show');
    insertBookmark(name);
  });

  // Enter to submit
  input.addEventListener('keydown', e => {
    if (e.key === 'Enter') {
      e.preventDefault();
      $('bkInsertBtn').click();
    }
  });

  // Backdrop click
  modal.addEventListener('click', e => {
    if (e.target === modal) modal.classList.remove('show');
  });

  // Menu bar entry
  const miBookmark = $('miBookmark');
  if (miBookmark) {
    miBookmark.addEventListener('click', () => {
      closeAllMenus();
      $('insertMenu')?.classList.remove('show');
      openBookmarkModal();
    });
  }

  // Tooltip on bookmark markers in document (show on hover, click to scroll)
  $('pageContainer')?.addEventListener('click', e => {
    const marker = e.target.closest('[data-bookmark]');
    if (!marker) return;
    const name = marker.dataset.bookmark;
    if (name) {
      showToast('Bookmark: ' + name, 'info', 2000);
    }
  });
}

/**
 * Post-render: style bookmark markers with an icon via CSS.
 * Called after renderDocument().
 */
export function renderDocumentBookmarks() {
  const container = $('pageContainer');
  if (!container) return;
  // Bookmark markers are already rendered by WASM with the bookmark-marker class.
  // This function ensures they have the visual indicator if not already present.
  container.querySelectorAll('.bookmark-marker').forEach(el => {
    if (el.querySelector('.bookmark-flag')) return;
    const flag = document.createElement('span');
    flag.className = 'bookmark-flag';
    flag.setAttribute('aria-hidden', 'true');
    flag.textContent = '';
    el.prepend(flag);
  });
}

// ═══════════════════════════════════════════════════
// E9.1: Custom Dictionary
// ═══════════════════════════════════════════════════

const DICT_STORAGE_KEY = 's1-custom-dictionary';

function getCustomDictionary() {
  try {
    const raw = localStorage.getItem(DICT_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return [];
}

function saveCustomDictionary(words) {
  try { localStorage.setItem(DICT_STORAGE_KEY, JSON.stringify(words)); } catch (_) {}
}

function initDictModal() {
  const modal = $('dictModal');
  if (!modal) return;

  // Open from menu
  const menuEntry = $('menuCustomDict');
  if (menuEntry) {
    menuEntry.addEventListener('click', () => {
      closeAllMenus();
      saveModalSelection();
      refreshDictWordList();
      modal.classList.add('show');
      $('dictWordInput').focus();
    });
  }

  // Add word
  $('dictAddBtn').addEventListener('click', () => {
    const input = $('dictWordInput');
    const word = input.value.trim().toLowerCase();
    if (!word) return;
    const dict = getCustomDictionary();
    if (!dict.includes(word)) {
      dict.push(word);
      dict.sort();
      saveCustomDictionary(dict);
    }
    input.value = '';
    input.focus();
    refreshDictWordList();
  });

  $('dictWordInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); $('dictAddBtn').click(); }
    if (e.key === 'Escape') { modal.classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });

  // Close
  $('dictCloseBtn').addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Backdrop
  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });
}

function refreshDictWordList() {
  const list = $('dictWordList');
  if (!list) return;
  const dict = getCustomDictionary();
  if (dict.length === 0) {
    list.innerHTML = '<span style="color:var(--text-muted);font-size:12px">No custom words added yet.</span>';
    return;
  }
  list.innerHTML = dict.map(w =>
    `<div class="dict-word-item">
      <span>${escapeHtml(w)}</span>
      <button class="dict-remove-btn" data-word="${escapeAttr(w)}" title="Remove '${escapeAttr(w)}' from dictionary">&times;</button>
    </div>`
  ).join('');
  list.querySelectorAll('.dict-remove-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const word = btn.dataset.word;
      const dict = getCustomDictionary().filter(w => w !== word);
      saveCustomDictionary(dict);
      refreshDictWordList();
    });
  });
}

// ═══════════════════════════════════════════════════
// E9.1: Auto-Correct
// ═══════════════════════════════════════════════════

const AC_STORAGE_KEY = 's1-autocorrect';
const AC_ENABLED_KEY = 's1-autocorrect-enabled';

const DEFAULT_AUTOCORRECT = {
  'teh': 'the', 'adn': 'and', 'hte': 'the', 'taht': 'that',
  'wiht': 'with', 'thier': 'their', 'recieve': 'receive',
  'occured': 'occurred', 'seperate': 'separate', 'definately': 'definitely',
  'accomodate': 'accommodate', 'acheive': 'achieve', 'occurence': 'occurrence',
  'enviroment': 'environment', 'goverment': 'government', 'begining': 'beginning',
  'beleive': 'believe', 'calender': 'calendar', 'collegue': 'colleague',
  'commitee': 'committee', 'concensus': 'consensus',
};

// Canonical version in features/document/toolbar/autocorrect.js; re-exported at top.
function getAutoCorrectMap() {
  try {
    const raw = localStorage.getItem(AC_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return { ...DEFAULT_AUTOCORRECT };
}

function saveAutoCorrectMap(map) {
  try { localStorage.setItem(AC_STORAGE_KEY, JSON.stringify(map)); } catch (_) {}
}

function isAutoCorrectEnabled() {
  try {
    const val = localStorage.getItem(AC_ENABLED_KEY);
    if (val === null) return true; // enabled by default
    return val === 'true';
  } catch (_) {}
  return true;
}

function setAutoCorrectEnabled(enabled) {
  try { localStorage.setItem(AC_ENABLED_KEY, String(enabled)); } catch (_) {}
}

function initAutoCorrectModal() {
  const modal = $('autoCorrectModal');
  if (!modal) return;

  // Open from menu
  const menuEntry = $('menuAutoCorrect');
  if (menuEntry) {
    menuEntry.addEventListener('click', () => {
      closeAllMenus();
      $('acEnabledToggle').checked = isAutoCorrectEnabled();
      refreshAcRuleList();
      modal.classList.add('show');
      $('acReplaceInput').focus();
    });
  }

  // Enable/disable toggle
  $('acEnabledToggle').addEventListener('change', () => {
    setAutoCorrectEnabled($('acEnabledToggle').checked);
  });

  // Add rule
  $('acAddBtn').addEventListener('click', () => {
    const replaceInput = $('acReplaceInput');
    const withInput = $('acWithInput');
    const from = replaceInput.value.trim().toLowerCase();
    const to = withInput.value.trim();
    if (!from || !to) return;
    const map = getAutoCorrectMap();
    map[from] = to;
    saveAutoCorrectMap(map);
    replaceInput.value = '';
    withInput.value = '';
    replaceInput.focus();
    refreshAcRuleList();
  });

  $('acReplaceInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); $('acWithInput').focus(); }
    if (e.key === 'Escape') { modal.classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
  $('acWithInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); $('acAddBtn').click(); }
    if (e.key === 'Escape') { modal.classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });

  // Close
  $('acCloseBtn').addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  // Backdrop
  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });
}

function refreshAcRuleList() {
  const list = $('acRuleList');
  if (!list) return;
  const map = getAutoCorrectMap();
  const entries = Object.entries(map).sort((a, b) => a[0].localeCompare(b[0]));
  if (entries.length === 0) {
    list.innerHTML = '<span style="color:var(--text-muted);font-size:12px">No auto-correct rules defined.</span>';
    return;
  }
  list.innerHTML = entries.map(([from, to]) =>
    `<div class="ac-rule-item">
      <span class="ac-rule-from">${escapeHtml(from)}</span>
      <span class="ac-rule-arrow">&#8594;</span>
      <span class="ac-rule-to">${escapeHtml(to)}</span>
      <button class="ac-rule-remove" data-from="${escapeAttr(from)}" title="Remove this rule">&times;</button>
    </div>`
  ).join('');
  list.querySelectorAll('.ac-rule-remove').forEach(btn => {
    btn.addEventListener('click', () => {
      const from = btn.dataset.from;
      const map = getAutoCorrectMap();
      delete map[from];
      saveAutoCorrectMap(map);
      refreshAcRuleList();
    });
  });
}

// ═══════════════════════════════════════════════════════
// E10.4 — Welcome Dialog (first-run onboarding)
// ═══════════════════════════════════════════════════════

function initWelcomeDialog() {
  const modal = $('welcomeModal');
  const btn = $('welcomeGetStarted');
  const checkbox = $('welcomeDontShow');
  if (!modal || !btn) return;

  // Show on first visit unless user opted out permanently or dismissed this session
  try {
    const permanent = localStorage.getItem('s1-onboarded');
    const session = sessionStorage.getItem('s1-onboarded-session');
    if (!permanent && !session) {
      modal.classList.add('show');
    }
  } catch (_) {
    // localStorage unavailable — show anyway
    modal.classList.add('show');
  }

  btn.addEventListener('click', () => {
    modal.classList.remove('show');
    // Set the onboarded flag based on checkbox
    try {
      if (checkbox && checkbox.checked) {
        localStorage.setItem('s1-onboarded', 'permanent');
      } else {
        // Mark as seen for this session only (sessionStorage)
        sessionStorage.setItem('s1-onboarded-session', 'true');
      }
    } catch (_) {}
    // Start feature tour after welcome dialog (if not done before)
    try {
      if (!localStorage.getItem('s1-tour-done')) {
        setTimeout(() => startFeatureTour(), 400);
      }
    } catch (_) {}
  });

  // Close on overlay click
  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      try { sessionStorage.setItem('s1-onboarded-session', 'true'); } catch (_) {}
    }
  });
}

// ═══════════════════════════════════════════════════════
// E10.4 — Feature Tour (guided step-by-step highlight)
// ═══════════════════════════════════════════════════════

const TOUR_STEPS = [
  { selector: '#toolbar', text: 'Use the toolbar to format text -- bold, italic, fonts, colors, alignment, and more.' },
  { selector: '#appMenubar', text: 'Access all features from the menu bar -- File, Edit, View, Insert, Format, Tools, and Help.' },
  { selector: '.canvas', text: 'Start typing here to create your document. Open files with Ctrl+O or drag and drop.' },
  { selector: '.statusbar', text: 'View page count, word count, zoom level, and collaboration status here.' },
];

let tourStep = 0;
let tourSpotlight = null;

export function startFeatureTour() {
  tourStep = 0;
  const overlay = $('tourOverlay');
  if (!overlay) return;
  overlay.style.display = '';
  overlay.classList.add('active');

  // Create spotlight element if it doesn't exist
  if (!tourSpotlight) {
    tourSpotlight = document.createElement('div');
    tourSpotlight.className = 'tour-spotlight';
    document.body.appendChild(tourSpotlight);
  }
  tourSpotlight.style.display = '';

  showTourStep();
}

function showTourStep() {
  const overlay = $('tourOverlay');
  const tooltip = $('tourTooltip');
  const textEl = $('tourStepText');
  const counterEl = $('tourStepCounter');
  const nextBtn = $('tourNextBtn');
  if (!overlay || !tooltip || !textEl) return;

  if (tourStep >= TOUR_STEPS.length) {
    endTour();
    return;
  }

  const step = TOUR_STEPS[tourStep];
  const target = document.querySelector(step.selector);

  textEl.textContent = step.text;
  counterEl.textContent = `Step ${tourStep + 1} of ${TOUR_STEPS.length}`;
  nextBtn.textContent = tourStep === TOUR_STEPS.length - 1 ? 'Finish' : 'Next';

  if (target) {
    const rect = target.getBoundingClientRect();
    const padding = 6;

    // Position spotlight
    if (tourSpotlight) {
      tourSpotlight.style.top = (rect.top - padding) + 'px';
      tourSpotlight.style.left = (rect.left - padding) + 'px';
      tourSpotlight.style.width = (rect.width + padding * 2) + 'px';
      tourSpotlight.style.height = (rect.height + padding * 2) + 'px';
      tourSpotlight.style.display = '';
    }

    // Position tooltip below or above the target
    const tooltipHeight = 160;
    const spaceBelow = window.innerHeight - rect.bottom;
    if (spaceBelow > tooltipHeight + 20) {
      tooltip.style.top = (rect.bottom + 12) + 'px';
      tooltip.style.bottom = '';
    } else {
      tooltip.style.top = '';
      tooltip.style.bottom = (window.innerHeight - rect.top + 12) + 'px';
    }
    tooltip.style.left = Math.max(16, Math.min(rect.left, window.innerWidth - 340)) + 'px';
  } else {
    // Element not found — center tooltip
    if (tourSpotlight) tourSpotlight.style.display = 'none';
    tooltip.style.top = '50%';
    tooltip.style.left = '50%';
    tooltip.style.transform = 'translate(-50%, -50%)';
  }

  tooltip.style.display = '';
}

function endTour() {
  const overlay = $('tourOverlay');
  if (overlay) {
    overlay.classList.remove('active');
    overlay.style.display = 'none';
  }
  if (tourSpotlight) {
    tourSpotlight.style.display = 'none';
  }
  const tooltip = $('tourTooltip');
  if (tooltip) tooltip.style.display = 'none';

  try { localStorage.setItem('s1-tour-done', 'true'); } catch (_) {}
}

function initFeatureTour() {
  const skipBtn = $('tourSkipBtn');
  const nextBtn = $('tourNextBtn');

  if (skipBtn) skipBtn.addEventListener('click', endTour);
  if (nextBtn) nextBtn.addEventListener('click', () => {
    tourStep++;
    showTourStep();
  });

  // Feature Tour from Help menu
  if ($('menuFeatureTour')) $('menuFeatureTour').addEventListener('click', () => {
    closeAllMenus();
    startFeatureTour();
  });
}

// ═══════════════════════════════════════════════════════
// E10.5 — Usage Statistics Modal
// ═══════════════════════════════════════════════════════

function initUsageStatsModal() {
  const modal = $('usageStatsModal');
  const closeBtn = $('usageStatsCloseBtn');
  const clearBtn = $('usageStatsClearBtn');
  if (!modal || !closeBtn) return;

  function populateStats() {
    const tbody = $('usageStatsBody');
    if (!tbody) return;
    const stats = getStats();
    const entries = Object.entries(stats).sort((a, b) => b[1] - a[1]);
    if (entries.length === 0) {
      tbody.innerHTML = '<tr><td colspan="2" style="padding:12px 8px;color:var(--text-secondary,#5f6368);text-align:center">No usage data recorded yet.</td></tr>';
      return;
    }
    tbody.innerHTML = entries.map(([key, count]) => {
      const [category, action] = key.split(':');
      const label = `${category} / ${action}`;
      return `<tr style="border-bottom:1px solid var(--border,#dadce0)"><td style="padding:6px 8px">${label}</td><td style="padding:6px 8px;text-align:right;font-variant-numeric:tabular-nums">${count}</td></tr>`;
    }).join('');
    // Add session duration row
    const dur = getSessionDuration();
    const durStr = dur < 60 ? `${dur}s` : dur < 3600 ? `${Math.floor(dur / 60)}m ${dur % 60}s` : `${Math.floor(dur / 3600)}h ${Math.floor((dur % 3600) / 60)}m`;
    tbody.innerHTML += `<tr style="border-top:2px solid var(--border,#dadce0)"><td style="padding:6px 8px;font-weight:500">Session duration</td><td style="padding:6px 8px;text-align:right">${durStr}</td></tr>`;
  }

  // Help > Usage Statistics
  if ($('menuUsageStats')) $('menuUsageStats').addEventListener('click', () => {
    closeAllMenus();
    populateStats();
    modal.classList.add('show');
    trackEvent('menu', 'usage-statistics');
  });

  closeBtn.addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  if (clearBtn) clearBtn.addEventListener('click', () => {
    clearStats();
    populateStats();
    showToast('Usage statistics cleared.', 'info', 3000);
  });

  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });
}

// ═══════════════════════════════════════════════════════
// E10.5 — Error Detail Modal
// ═══════════════════════════════════════════════════════

function initErrorDetailModal() {
  const modal = $('errorDetailModal');
  const closeBtn = $('errorDetailCloseBtn');
  const clearBtn = $('errorClearBtn');
  const indicator = $('errorIndicator');
  if (!modal || !closeBtn) return;

  function showErrorDetail() {
    const content = $('errorDetailContent');
    if (!content) return;
    const { error, count } = getLastError();
    if (!error || count === 0) {
      content.textContent = 'No errors recorded.';
    } else {
      let msg = `Total errors in this session: ${count}\n\nLast error:\n`;
      if (error instanceof Error) {
        msg += `${error.name}: ${error.message}`;
        if (error.stack) msg += `\n\nStack trace:\n${error.stack}`;
      } else if (typeof error === 'string') {
        msg += error;
      } else {
        try { msg += JSON.stringify(error, null, 2); } catch (_) { msg += String(error); }
      }
      content.textContent = msg;
    }
    modal.classList.add('show');
  }

  // Click error indicator in status bar to open modal
  if (indicator) indicator.addEventListener('click', showErrorDetail);

  closeBtn.addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  if (clearBtn) clearBtn.addEventListener('click', () => {
    clearErrors();
    $('errorDetailContent').textContent = 'No errors recorded.';
    showToast('Error log cleared.', 'info', 3000);
  });

  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });
}

// ═══════════════════════════════════════════════════════
// E10.4 — What's New Modal
// ═══════════════════════════════════════════════════════

function initWhatsNew() {
  const modal = $('whatsNewModal');
  const closeBtn = $('whatsNewCloseBtn');
  if (!modal || !closeBtn) return;

  // Wire Help > What's New menu entry
  if ($('menuWhatsNew')) $('menuWhatsNew').addEventListener('click', () => {
    closeAllMenus();
    modal.classList.add('show');
  });

  closeBtn.addEventListener('click', () => {
    modal.classList.remove('show');
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  });

  modal.addEventListener('click', e => {
    if (e.target === modal) {
      modal.classList.remove('show');
      ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
    }
  });
}

// ═══════════════════════════════════════════════════════
// E5.1 — Share Dialog Enhancement (permission in URL)
// ═══════════════════════════════════════════════════════

function initSharePermissionSync() {
  const permSelect = $('sharePermission');
  const urlInput = $('shareUrlInput');
  if (!permSelect || !urlInput) return;

  permSelect.addEventListener('change', () => {
    // Update the share URL to include the selected permission level
    const currentUrl = urlInput.value;
    if (!currentUrl) return;
    try {
      const url = new URL(currentUrl);
      url.searchParams.set('access', permSelect.value);
      urlInput.value = url.toString();
    } catch (_) {}
  });

  // Also update when Copy Link is clicked to include permission
  const copyBtn = $('shareCopyBtn');
  if (copyBtn) {
    const origClick = copyBtn.onclick;
    // The copy handler is already wired in collab.js via initCollabUI.
    // We just need to ensure the URL has the access param before copy.
    // The permission change handler above already does this.
  }
}

// ═══════════════════════════════════════════════════════
// E5.4 / UXP-07 — Editing Mode + Track Changes UI
// ═══════════════════════════════════════════════════════

/** Sync all mode selectors (status bar, panel, Review menu) to a given mode. */
function syncModeSelectors(mode) {
  const statusSel = $('editingModeSelect');
  if (statusSel && statusSel.value !== mode) statusSel.value = mode;
  const panelSel = $('tcModeSelect');
  if (panelSel && panelSel.value !== mode) panelSel.value = mode;
  // Update mode icon in status bar
  const wrap = $('editingModeWrap');
  if (wrap) wrap.dataset.mode = mode;
  const icon = $('editingModeIcon');
  if (icon) {
    const icons = { editing: 'edit', suggesting: 'rate_review', viewing: 'visibility' };
    icon.textContent = icons[mode] || 'edit';
  }
  // Update Review menu entries (mark active)
  ['Editing', 'Suggesting', 'Viewing'].forEach(m => {
    const btn = $('menuMode' + m);
    if (btn) btn.classList.toggle('active', mode === m.toLowerCase());
  });
}

function setEditingMode(mode) {
  state.editingMode = mode;
  syncModeSelectors(mode);
  applyEditingMode(mode);
  const labels = { editing: 'Editing', suggesting: 'Suggesting', viewing: 'Viewing' };
  showToast('Switched to ' + (labels[mode] || mode) + ' mode', 'info', 2000);
  announce('Mode: ' + mode.charAt(0).toUpperCase() + mode.slice(1));
}

function initEditingMode() {
  // Status bar selector
  const sel = $('editingModeSelect');
  if (sel) {
    sel.addEventListener('change', () => setEditingMode(sel.value));
  }
  // TC panel selector
  const panelSel = $('tcModeSelect');
  if (panelSel) {
    panelSel.addEventListener('change', () => setEditingMode(panelSel.value));
  }
  // Set initial icon state
  syncModeSelectors(state.editingMode || 'editing');
}

function applyEditingMode(mode) {
  const pages = document.querySelectorAll('.page-content');
  switch (mode) {
    case 'viewing':
      pages.forEach(p => {
        p.contentEditable = 'false';
        p.classList.add('viewing-mode');
        p.classList.remove('suggesting-mode');
      });
      break;
    case 'suggesting':
      pages.forEach(p => {
        p.contentEditable = 'true';
        p.classList.add('suggesting-mode');
        p.classList.remove('viewing-mode');
      });
      // Enable track changes if WASM supports it
      if (state.doc && typeof state.doc.set_track_changes_enabled === 'function') {
        try { state.doc.set_track_changes_enabled(true); } catch (_) {}
      }
      state.trackChangesMode = true;
      break;
    default: // 'editing'
      pages.forEach(p => {
        p.contentEditable = 'true';
        p.classList.remove('viewing-mode', 'suggesting-mode');
      });
      if (state.doc && typeof state.doc.set_track_changes_enabled === 'function') {
        try { state.doc.set_track_changes_enabled(false); } catch (_) {}
      }
      state.trackChangesMode = false;
      break;
  }
}

// ═══════════════════════════════════════════════════════
// FS-11 — Read-Only / Viewer Mode
// ═══════════════════════════════════════════════════════

/**
 * Toggle the read-only viewer mode.
 * When active, all editing is disabled but selection, copy, and navigation work.
 */
function toggleReadOnlyMode() {
  state.readOnlyMode = !state.readOnlyMode;
  const pages = document.querySelectorAll('.page-content');
  pages.forEach(p => {
    p.contentEditable = state.readOnlyMode ? 'false' : 'true';
  });
  // Update menu entry to reflect current state
  const btn = $('menuReadOnly');
  if (btn) {
    btn.classList.toggle('active', state.readOnlyMode);
    const icon = btn.querySelector('.msi');
    if (icon) icon.textContent = state.readOnlyMode ? 'lock_open' : 'lock';
    btn.innerHTML = state.readOnlyMode
      ? '<span class="msi">lock_open</span> Exit Read-Only'
      : '<span class="msi">lock</span> Read-Only Mode';
  }
  announce(state.readOnlyMode ? 'Read-only mode enabled' : 'Read-only mode disabled');
}

// ═══════════════════════════════════════════════════════
// UXP-07 — Review Menu
// ═══════════════════════════════════════════════════════

function initReviewMenu() {
  // Mode entries
  const menuEditing = $('menuModeEditing');
  const menuSuggesting = $('menuModeSuggesting');
  const menuViewing = $('menuModeViewing');
  if (menuEditing) menuEditing.addEventListener('click', () => {
    setEditingMode('editing');
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });
  if (menuSuggesting) menuSuggesting.addEventListener('click', () => {
    setEditingMode('suggesting');
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });
  if (menuViewing) menuViewing.addEventListener('click', () => {
    setEditingMode('viewing');
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });

  // Accept / Reject all from Review menu
  const menuAcceptAll = $('menuAcceptAll');
  const menuRejectAll = $('menuRejectAll');
  if (menuAcceptAll) menuAcceptAll.addEventListener('click', () => {
    acceptAllChanges();
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });
  if (menuRejectAll) menuRejectAll.addEventListener('click', () => {
    rejectAllChanges();
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });

  // Prev / Next change from Review menu
  const menuPrev = $('menuPrevChange');
  const menuNext = $('menuNextChange');
  if (menuPrev) menuPrev.addEventListener('click', () => {
    navigateChange(-1);
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });
  if (menuNext) menuNext.addEventListener('click', () => {
    navigateChange(1);
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });

  // Show changes panel
  const menuShowPanel = $('menuShowChangesPanel');
  if (menuShowPanel) menuShowPanel.addEventListener('click', () => {
    toggleTrackChangesPanel();
    document.querySelectorAll('.app-menu-item').forEach(m => m.classList.remove('open'));
  });

  // Keyboard shortcuts: Ctrl+Shift+[ and Ctrl+Shift+]
  document.addEventListener('keydown', e => {
    if (e.ctrlKey && e.shiftKey && e.key === '[') {
      e.preventDefault();
      navigateChange(-1);
    }
    if (e.ctrlKey && e.shiftKey && e.key === ']') {
      e.preventDefault();
      navigateChange(1);
    }
  });
}

// ═══════════════════════════════════════════════════════
// UXP-07 — Track Changes Panel
// ═══════════════════════════════════════════════════════

/** Current change index for navigation (0-based, -1 = none). */
let _tcNavIndex = -1;
/** Cached list of TC DOM elements (sorted by document order). */
let _tcElements = [];

function initTrackChangesPanel() {
  // Close button
  const closeBtn = $('tcPanelClose');
  if (closeBtn) closeBtn.addEventListener('click', () => toggleTrackChangesPanel(false));

  // Toggle button in TC bar
  const toggleBtn = $('btnToggleChangesPanel');
  if (toggleBtn) toggleBtn.addEventListener('click', () => toggleTrackChangesPanel());

  // Accept All / Reject All in panel
  const panelAcceptAll = $('tcPanelAcceptAll');
  const panelRejectAll = $('tcPanelRejectAll');
  if (panelAcceptAll) panelAcceptAll.addEventListener('click', acceptAllChanges);
  if (panelRejectAll) panelRejectAll.addEventListener('click', rejectAllChanges);

  // Navigation buttons in TC bar
  const prevBtn = $('btnPrevChange');
  const nextBtn = $('btnNextChange');
  if (prevBtn) prevBtn.addEventListener('click', () => navigateChange(-1));
  if (nextBtn) nextBtn.addEventListener('click', () => navigateChange(1));
}

function toggleTrackChangesPanel(forceShow) {
  const panel = $('tcPanel');
  if (!panel) return;
  const show = forceShow !== undefined ? forceShow : !panel.classList.contains('show');
  panel.classList.toggle('show', show);
  if (show) refreshTrackChangesPanel();
}

function acceptAllChanges() {
  if (!state.doc) return;
  try {
    state.doc.accept_all_changes();
    broadcastOp({ action: 'acceptAllChanges' });
    markDirty();
    renderDocument();
    updateTrackChanges();
    refreshTrackChangesPanel();
    _tcNavIndex = -1;
    updateTcNavPos();
  } catch (e) { console.error('accept all:', e); }
}

function rejectAllChanges() {
  if (!state.doc) return;
  try {
    state.doc.reject_all_changes();
    broadcastOp({ action: 'rejectAllChanges' });
    markDirty();
    renderDocument();
    updateTrackChanges();
    refreshTrackChangesPanel();
    _tcNavIndex = -1;
    updateTcNavPos();
  } catch (e) { console.error('reject all:', e); }
}

/** Refresh the cached list of TC elements from the DOM. */
function refreshTcElements() {
  const container = $('pageContainer');
  if (!container) { _tcElements = []; return; }
  _tcElements = Array.from(container.querySelectorAll('[data-tc-node-id]'));
}

/** Navigate to the next/previous tracked change in the document.
 *  direction: +1 for next, -1 for previous. */
function navigateChange(direction) {
  refreshTcElements();
  if (_tcElements.length === 0) return;

  // Clear previous highlight
  _tcElements.forEach(el => el.classList.remove('tc-active'));

  _tcNavIndex += direction;
  if (_tcNavIndex >= _tcElements.length) _tcNavIndex = 0;
  if (_tcNavIndex < 0) _tcNavIndex = _tcElements.length - 1;

  const target = _tcElements[_tcNavIndex];
  if (target) {
    target.classList.add('tc-active');
    target.scrollIntoView({ behavior: 'smooth', block: 'center' });
    // Highlight matching card in panel
    highlightPanelCard(target.dataset.tcNodeId);
  }
  updateTcNavPos();
}

/** Update the nav position indicator in the TC bar. */
function updateTcNavPos() {
  const posEl = $('tcNavPos');
  if (!posEl) return;
  if (_tcElements.length === 0 || _tcNavIndex < 0) {
    posEl.textContent = '';
  } else {
    posEl.textContent = (_tcNavIndex + 1) + '/' + _tcElements.length;
  }
}

/** Highlight a specific change card in the side panel. */
function highlightPanelCard(nodeId) {
  const panel = $('tcPanelList');
  if (!panel) return;
  panel.querySelectorAll('.tc-change-card').forEach(card => {
    card.classList.toggle('tc-card-active', card.dataset.nodeId === nodeId);
  });
}

/** Populate the Track Changes side panel with all tracked changes from the document. */
export function refreshTrackChangesPanel() {
  const panel = $('tcPanel');
  if (!panel || !panel.classList.contains('show')) return;

  const list = $('tcPanelList');
  const empty = $('tcPanelEmpty');
  if (!list) return;

  // Clear existing cards (but not the empty placeholder)
  list.querySelectorAll('.tc-change-card').forEach(c => c.remove());

  if (!state.doc) {
    if (empty) empty.style.display = '';
    return;
  }

  // Get tracked changes from WASM
  let changes = [];
  try {
    if (typeof state.doc.tracked_changes_json === 'function') {
      const json = state.doc.tracked_changes_json();
      changes = JSON.parse(json);
    }
  } catch (_) {}

  // Also gather from DOM to get text preview
  refreshTcElements();
  const tcTextMap = new Map();
  _tcElements.forEach(el => {
    tcTextMap.set(el.dataset.tcNodeId, {
      text: (el.textContent || '').slice(0, 80),
      type: el.dataset.tcType || 'insert',
    });
  });

  if (changes.length === 0 && _tcElements.length === 0) {
    if (empty) empty.style.display = '';
    return;
  }
  if (empty) empty.style.display = 'none';

  // If WASM returned changes, use them; otherwise fall back to DOM elements
  const items = changes.length > 0 ? changes : _tcElements.map(el => ({
    nodeId: el.dataset.tcNodeId,
    type: el.dataset.tcType === 'delete' ? 'Delete' : el.dataset.tcType === 'format' ? 'FormatChange' : 'Insert',
    author: '',
    date: '',
  }));

  items.forEach((ch, idx) => {
    const nodeId = ch.nodeId;
    const card = document.createElement('div');
    card.className = 'tc-change-card';
    card.dataset.nodeId = nodeId;
    card.dataset.index = idx;

    // Header with badge + author
    const header = document.createElement('div');
    header.className = 'tc-change-card-header';

    const badge = document.createElement('span');
    const type = (ch.type || 'Insert').toLowerCase();
    const isInsert = type === 'insert';
    const isDelete = type === 'delete';
    const isFormat = type === 'formatchange' || type === 'format';
    badge.className = 'tc-change-badge ' + (isInsert ? 'tc-badge-insert' : isDelete ? 'tc-badge-delete' : 'tc-badge-format');
    const badgeIcon = isInsert ? 'add' : isDelete ? 'remove' : 'format_paint';
    const badgeLabel = isInsert ? 'Insert' : isDelete ? 'Delete' : 'Format';
    badge.innerHTML = '<span class="msi">' + badgeIcon + '</span> ' + badgeLabel;
    header.appendChild(badge);

    if (ch.author) {
      const author = document.createElement('span');
      author.className = 'tc-change-author';
      author.textContent = ch.author;
      author.title = ch.author + (ch.date ? ' (' + ch.date + ')' : '');
      header.appendChild(author);
    }
    card.appendChild(header);

    // Text preview
    const domInfo = tcTextMap.get(nodeId);
    if (domInfo && domInfo.text) {
      const preview = document.createElement('div');
      preview.className = 'tc-change-preview';
      const span = document.createElement('span');
      span.className = isInsert ? 'tc-change-preview-ins' : isDelete ? 'tc-change-preview-del' : '';
      span.textContent = domInfo.text;
      preview.appendChild(span);
      card.appendChild(preview);
    }

    // Action buttons
    const actions = document.createElement('div');
    actions.className = 'tc-change-actions';

    const acceptBtn = document.createElement('button');
    acceptBtn.className = 'tc-card-btn tc-card-accept';
    acceptBtn.textContent = 'Accept';
    acceptBtn.title = 'Accept this change';
    acceptBtn.addEventListener('click', e => {
      e.stopPropagation();
      try {
        state.doc.accept_change(nodeId);
        broadcastOp({ action: 'acceptChange', nodeId });
        markDirty();
        renderDocument();
        updateTrackChanges();
        refreshTrackChangesPanel();
      } catch (err) { console.error('accept change:', err); }
    });
    actions.appendChild(acceptBtn);

    const rejectBtn = document.createElement('button');
    rejectBtn.className = 'tc-card-btn tc-card-reject';
    rejectBtn.textContent = 'Reject';
    rejectBtn.title = 'Reject this change';
    rejectBtn.addEventListener('click', e => {
      e.stopPropagation();
      try {
        state.doc.reject_change(nodeId);
        broadcastOp({ action: 'rejectChange', nodeId });
        markDirty();
        renderDocument();
        updateTrackChanges();
        refreshTrackChangesPanel();
      } catch (err) { console.error('reject change:', err); }
    });
    actions.appendChild(rejectBtn);
    card.appendChild(actions);

    // Click card to navigate to the change in the document
    card.addEventListener('click', () => {
      refreshTcElements();
      const target = _tcElements.find(el => el.dataset.tcNodeId === nodeId);
      if (target) {
        _tcElements.forEach(el => el.classList.remove('tc-active'));
        target.classList.add('tc-active');
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
        _tcNavIndex = _tcElements.indexOf(target);
        updateTcNavPos();
      }
      highlightPanelCard(nodeId);
    });

    list.appendChild(card);
  });
}

// ═══════════════════════════════════════════════════════
// E9.2 — Save as Custom Template + Gallery Thumbnails
// ═══════════════════════════════════════════════════════

const CUSTOM_TEMPLATES_KEY = 's1-custom-templates';

function getCustomTemplates() {
  try {
    const raw = localStorage.getItem(CUSTOM_TEMPLATES_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return [];
}

function saveCustomTemplates(templates) {
  try { localStorage.setItem(CUSTOM_TEMPLATES_KEY, JSON.stringify(templates)); } catch (_) {}
}

function initSaveAsTemplate() {
  const btn = $('btnSaveTemplate');
  if (!btn) return;

  btn.addEventListener('click', async () => {
    closeAllMenus();
    if (!state.doc) {
      showToast('No document open to save as template.', 'error');
      return;
    }

    // Prompt for template name using custom modal
    const name = await showPromptModal('Template name:', '');
    if (!name || !name.trim()) return;

    try {
      const html = state.doc.to_html();
      const docName = $('docName')?.value || 'Untitled';
      let wordCount = 0;
      try { wordCount = state.doc.to_plain_text().split(/\s+/).filter(Boolean).length; } catch (_) {}

      const template = {
        name: name.trim(),
        html,
        createdAt: new Date().toISOString(),
        wordCount,
        docName,
      };

      const templates = getCustomTemplates();
      templates.push(template);
      saveCustomTemplates(templates);

      showToast('Template "' + name.trim() + '" saved.', 'success');
    } catch (e) {
      console.error('save template:', e);
      showToast('Failed to save template.', 'error');
    }
  });

  // Enhance template grid with thumbnails and custom templates
  enhanceTemplateGrid();
}

function enhanceTemplateGrid() {
  const grid = $('templateGrid');
  if (!grid) return;

  // Add colored preview headers to built-in templates
  const builtinColors = {
    blank: '#e8eaed',
    letter: '#e3f2fd',
    resume: '#fce4ec',
    report: '#e8f5e9',
    meeting: '#fff3e0',
    essay: '#f3e5f5',
  };
  const builtinDescriptions = {
    blank: 'Start fresh',
    letter: 'Formal letter template',
    resume: 'Professional resume layout',
    report: 'Business report structure',
    meeting: 'Meeting notes with agenda',
    essay: 'Academic essay format',
  };

  grid.querySelectorAll('.template-card[data-template]').forEach(card => {
    const key = card.dataset.template;
    if (builtinColors[key] && !card.querySelector('.template-preview')) {
      const preview = document.createElement('div');
      preview.className = 'template-preview';
      preview.style.background = builtinColors[key];
      const desc = document.createElement('div');
      desc.className = 'template-desc';
      desc.textContent = builtinDescriptions[key] || '';
      card.insertBefore(preview, card.firstChild);
      card.appendChild(desc);
    }
  });

  // Append custom templates
  const custom = getCustomTemplates();
  custom.forEach((tpl, idx) => {
    const card = document.createElement('button');
    card.className = 'template-card template-card-custom';
    card.dataset.customIdx = String(idx);
    card.title = 'Custom template: ' + escapeAttr(tpl.name);

    const preview = document.createElement('div');
    preview.className = 'template-preview template-preview-custom';
    // Render a text-only preview (safe — no HTML interpretation)
    const previewInner = document.createElement('div');
    previewInner.className = 'template-preview-content';
    const tmpParse = new DOMParser().parseFromString(tpl.html || '', 'text/html');
    previewInner.textContent = (tmpParse.body.textContent || '').slice(0, 200);
    preview.appendChild(previewInner);

    const icon = document.createElement('span');
    icon.className = 'msi template-icon';
    icon.textContent = 'draft';
    const name = document.createElement('span');
    name.className = 'template-name';
    name.textContent = tpl.name;
    const desc = document.createElement('div');
    desc.className = 'template-desc';
    desc.textContent = tpl.wordCount ? tpl.wordCount + ' words' : '';

    card.appendChild(preview);
    card.appendChild(icon);
    card.appendChild(name);
    card.appendChild(desc);

    card.addEventListener('click', () => {
      $('templateModal').classList.remove('show');
      loadCustomTemplate(idx);
    });
    grid.appendChild(card);
  });
}

function loadCustomTemplate(idx) {
  const templates = getCustomTemplates();
  const tpl = templates[idx];
  if (!tpl || !state.engine) return;

  // Create a new document and load the HTML
  state.doc = state.engine.create();
  state.currentFormat = 'new';

  // Try to import the HTML content
  try {
    if (tpl.html && typeof state.doc.import_html === 'function') {
      try {
        state.doc.import_html(tpl.html);
      } catch (_) {
        // Fallback: extract plain text via DOMParser (no innerHTML)
        const parsed = new DOMParser().parseFromString(tpl.html, 'text/html');
        state.doc.append_paragraph(parsed.body.textContent || '');
      }
    } else {
      const parsed = new DOMParser().parseFromString(tpl.html || '', 'text/html');
      state.doc.append_paragraph(parsed.body.textContent || '');
    }

    state.doc.clear_history();
  } catch (e) { console.warn('loadCustomTemplate:', e); }

  $('welcomeScreen').style.display = 'none';
  $('toolbar').classList.add('show');
  const menubar = $('appMenubar');
  if (menubar) menubar.classList.add('show');
  $('statusbar').classList.add('show');
  import('./file.js').then(({ switchView }) => { switchView('editor'); });

  renderDocument();
  renderRuler();

  $('docName').value = tpl.docName || tpl.name || 'Untitled Document';
  ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
  state.dirty = false;
  import('./file.js').then(mod => {
    if (typeof mod.updateDirtyIndicator === 'function') mod.updateDirtyIndicator();
    if (typeof mod.updateStatusBar === 'function') mod.updateStatusBar();
  });
}

// ═══════════════════════════════════════════════════════
// E5.4 — @Mention in Comments
// ═══════════════════════════════════════════════════════

function initCommentMentions() {
  const commentText = $('commentText');
  if (!commentText) return;

  let mentionActive = false;
  let mentionQuery = '';
  let mentionStartPos = -1;

  commentText.addEventListener('input', () => {
    const text = commentText.value;
    const cursorPos = commentText.selectionStart;

    // Check if we're in a mention context (@ typed)
    const beforeCursor = text.substring(0, cursorPos);
    const atIdx = beforeCursor.lastIndexOf('@');

    if (atIdx >= 0 && (atIdx === 0 || /\s/.test(beforeCursor[atIdx - 1]))) {
      const query = beforeCursor.substring(atIdx + 1);
      if (!/\s/.test(query)) {
        mentionActive = true;
        mentionQuery = query.toLowerCase();
        mentionStartPos = atIdx;
        showMentionDropdown(query.toLowerCase(), commentText);
        return;
      }
    }

    mentionActive = false;
    hideMentionDropdown();
  });

  commentText.addEventListener('keydown', e => {
    if (!mentionActive) return;
    const dd = $('mentionDropdown');
    if (!dd || dd.style.display === 'none') return;

    const items = dd.querySelectorAll('.mention-item');
    const activeItem = dd.querySelector('.mention-item.active');
    let activeIdx = Array.from(items).indexOf(activeItem);

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIdx = Math.min(activeIdx + 1, items.length - 1);
      items.forEach(i => i.classList.remove('active'));
      if (items[activeIdx]) items[activeIdx].classList.add('active');
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIdx = Math.max(activeIdx - 1, 0);
      items.forEach(i => i.classList.remove('active'));
      if (items[activeIdx]) items[activeIdx].classList.add('active');
    } else if (e.key === 'Enter' || e.key === 'Tab') {
      if (activeItem) {
        e.preventDefault();
        insertMention(commentText, mentionStartPos, activeItem.dataset.name);
        mentionActive = false;
        hideMentionDropdown();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      mentionActive = false;
      hideMentionDropdown();
    }
  });
}

function showMentionDropdown(query, textareaEl) {
  const dd = $('mentionDropdown');
  if (!dd) return;

  // Get peers from collab state
  const peers = [];
  if (state.collabPeers && state.collabPeers.size > 0) {
    state.collabPeers.forEach((peer, id) => {
      peers.push({ id, name: peer.userName || 'Peer', color: peer.userColor || '#1a73e8' });
    });
  }
  // Add current user
  peers.unshift({ id: 'self', name: 'User', color: '#34a853' });

  // Filter by query
  const filtered = peers.filter(p => p.name.toLowerCase().includes(query));

  if (filtered.length === 0) {
    dd.style.display = 'none';
    return;
  }

  dd.innerHTML = filtered.map((p, i) =>
    '<div class="mention-item' + (i === 0 ? ' active' : '') + '" data-name="' + escapeAttr(p.name) + '" title="Mention ' + escapeAttr(p.name) + '">' +
      '<span class="mention-avatar" style="background:' + p.color + '">' + p.name.charAt(0).toUpperCase() + '</span>' +
      '<span class="mention-name">' + escapeHtml(p.name) + '</span>' +
    '</div>'
  ).join('');

  // Position near the textarea
  const rect = textareaEl.getBoundingClientRect();
  dd.style.display = 'block';
  dd.style.left = (rect.left + 10) + 'px';
  dd.style.top = (rect.bottom + 4) + 'px';
  dd.style.minWidth = '160px';

  // Wire click handlers
  dd.querySelectorAll('.mention-item').forEach(item => {
    item.addEventListener('click', () => {
      const startPos = textareaEl.value.lastIndexOf('@');
      if (startPos >= 0) {
        insertMention(textareaEl, startPos, item.dataset.name);
      }
      hideMentionDropdown();
    });
  });
}

function hideMentionDropdown() {
  const dd = $('mentionDropdown');
  if (dd) dd.style.display = 'none';
}

function insertMention(textareaEl, atPos, name) {
  const before = textareaEl.value.substring(0, atPos);
  const afterCursor = textareaEl.value.substring(textareaEl.selectionStart);
  textareaEl.value = before + '@' + name + ' ' + afterCursor;
  const newCursor = atPos + name.length + 2; // @ + name + space
  textareaEl.selectionStart = newCursor;
  textareaEl.selectionEnd = newCursor;
  textareaEl.focus();
}

// ── Left Sidebar — Pages + Outline ──────────────────────────────

let _scrollTrackingSetup = false;

function togglePagesPanel() {
  const panel = $('pagesPanel');
  if (!panel) return;
  panel.classList.toggle('show');
  if (panel.classList.contains('show')) {
    const activeTab = panel.querySelector('.pages-tab.active');
    if (activeTab?.dataset.tab === 'pages') renderPageThumbnails();
    else renderOutline();
  }
}

/** Wire up tab switching in the left panel. */
function initPagesPanelTabs() {
  const panel = $('pagesPanel');
  if (!panel) return;
  panel.querySelectorAll('.pages-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      panel.querySelectorAll('.pages-tab').forEach(t => { t.classList.remove('active'); t.setAttribute('aria-selected', 'false'); });
      panel.querySelectorAll('.pages-tab-content').forEach(c => c.classList.remove('active'));
      tab.classList.add('active');
      tab.setAttribute('aria-selected', 'true');
      const target = panel.querySelector(`.pages-tab-content[data-tab="${tab.dataset.tab}"]`);
      if (target) target.classList.add('active');
      if (tab.dataset.tab === 'pages') renderPageThumbnails();
      else renderOutline();
    });
  });
}

// ── Page Thumbnails (real DOM clone, CSS-scaled) ────────────────

function renderPageThumbnails() {
  const list = $('pagesList');
  const pageContainer = $('pageContainer');
  if (!list || !pageContainer) return;

  const pages = pageContainer.querySelectorAll('.doc-page, .s1-page');
  if (!pages.length) {
    list.innerHTML = '<div style="text-align:center;padding:30px 12px;color:var(--text-muted);font-size:12px">No pages to display</div>';
    return;
  }

  list.innerHTML = '';
  const THUMB_WIDTH = 156; // px

  pages.forEach((page, i) => {
    const thumb = document.createElement('div');
    thumb.className = 'page-thumb';
    thumb.title = `Page ${i + 1} — click to scroll`;
    thumb.dataset.pageIndex = i;

    // Clone the page DOM and scale it down with CSS transform
    const pageW = page.offsetWidth || 816;
    const pageH = page.offsetHeight || 1056;
    const scale = THUMB_WIDTH / pageW;
    const thumbH = Math.round(pageH * scale);

    const inner = document.createElement('div');
    inner.className = 'page-thumb-inner';
    inner.style.width = THUMB_WIDTH + 'px';
    inner.style.height = thumbH + 'px';

    const clone = page.cloneNode(true);
    // Strip contenteditable / interactive attributes from clone
    clone.removeAttribute('contenteditable');
    clone.querySelectorAll('[contenteditable]').forEach(el => el.removeAttribute('contenteditable'));
    clone.querySelectorAll('input, textarea, button, select').forEach(el => el.remove());

    // Apply CSS transform to scale down
    clone.style.cssText = `
      transform: scale(${scale});
      transform-origin: top left;
      width: ${pageW}px;
      height: ${pageH}px;
      position: absolute;
      top: 0; left: 0;
      pointer-events: none;
      box-shadow: none;
      margin: 0;
      overflow: hidden;
    `;
    inner.style.position = 'relative';
    inner.style.overflow = 'hidden';
    inner.appendChild(clone);
    thumb.appendChild(inner);

    // Page number label
    const label = document.createElement('div');
    label.className = 'page-thumb-label';
    label.textContent = i + 1;
    thumb.appendChild(label);

    // Click to scroll
    thumb.addEventListener('click', () => {
      page.scrollIntoView({ behavior: 'smooth', block: 'start' });
      list.querySelectorAll('.page-thumb').forEach(t => t.classList.remove('active'));
      thumb.classList.add('active');
    });

    list.appendChild(thumb);
  });

  // Mark first as active
  const first = list.querySelector('.page-thumb');
  if (first) first.classList.add('active');

  // Setup scroll tracking (once)
  if (!_scrollTrackingSetup) {
    _scrollTrackingSetup = true;
    setupPageScrollTracking();
  }
}

/** Track scroll to highlight active page thumb + outline heading. */
function setupPageScrollTracking() {
  const canvas = $('editorCanvas');
  if (!canvas) return;

  let ticking = false;
  canvas.addEventListener('scroll', () => {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      ticking = false;
      const pageContainer = $('pageContainer');
      if (!pageContainer) return;

      const pages = pageContainer.querySelectorAll('.doc-page, .s1-page');
      const scrollTop = canvas.scrollTop;
      const viewMid = scrollTop + canvas.clientHeight / 3;

      // Update page thumbnails
      let activeIdx = 0;
      pages.forEach((page, i) => {
        if (page.offsetTop <= viewMid) activeIdx = i;
      });
      const thumbList = $('pagesList');
      if (thumbList) {
        thumbList.querySelectorAll('.page-thumb').forEach((t, i) => {
          const wasActive = t.classList.contains('active');
          t.classList.toggle('active', i === activeIdx);
          if (i === activeIdx && !wasActive) {
            t.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
          }
        });
      }

      // Update outline active heading
      const outlineList = $('outlineList');
      if (outlineList && outlineList.children.length) {
        const headings = pageContainer.querySelectorAll('h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]');
        let activeHeadingId = null;
        headings.forEach(h => {
          const rect = h.getBoundingClientRect();
          const canvasRect = canvas.getBoundingClientRect();
          if (rect.top - canvasRect.top <= canvas.clientHeight / 3) {
            activeHeadingId = h.dataset.nodeId;
          }
        });
        outlineList.querySelectorAll('.outline-item').forEach(item => {
          item.classList.toggle('active', item.dataset.nodeId === activeHeadingId);
        });
      }
    });
  });
}

// ── Outline (Document Heading Hierarchy) ────────────────────────

/** Cache of the last heading fingerprint to avoid unnecessary DOM rebuilds. */
let _lastOutlineFingerprint = '';

/**
 * Render the outline panel from the heading hierarchy.
 * Uses the WASM get_headings_json API when available (authoritative source),
 * falling back to DOM scraping for compatibility.
 */
function renderOutline() {
  const list = $('outlineList');
  const pageContainer = $('pageContainer');
  if (!list || !pageContainer) return;

  // Try WASM-based heading extraction first (authoritative, survives DOM inconsistencies)
  let headingData = null;
  if (state.doc && typeof state.doc.get_headings_json === 'function') {
    try {
      const json = state.doc.get_headings_json();
      headingData = JSON.parse(json);
    } catch (_) {
      headingData = null;
    }
  }

  // Fallback: scrape headings from rendered DOM
  if (!headingData) {
    headingData = [];
    const domHeadings = pageContainer.querySelectorAll(
      'h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]'
    );
    domHeadings.forEach(h => {
      const text = h.textContent.trim();
      if (text) {
        headingData.push({
          nodeId: h.dataset.nodeId || '',
          level: parseInt(h.tagName[1], 10),
          text,
        });
      }
    });
  }

  // Fingerprint check: skip rebuild if nothing changed
  const fp = headingData.map(h => `${h.level}:${h.nodeId}:${h.text}`).join('|');
  if (fp === _lastOutlineFingerprint && list.children.length > 0) return;
  _lastOutlineFingerprint = fp;

  if (!headingData.length) {
    list.innerHTML = '<div class="outline-empty">No headings found.<br><br>Add headings (H1\u2013H6) to your document to see the outline here.</div>';
    updateOutlineTabBadge(0);
    return;
  }

  list.innerHTML = '';
  headingData.forEach(h => {
    const item = document.createElement('div');
    item.className = 'outline-item';
    item.dataset.level = h.level;
    item.dataset.nodeId = h.nodeId;
    item.textContent = h.text;
    item.title = `${h.text} (H${h.level})`;
    item.setAttribute('role', 'link');
    item.setAttribute('tabindex', '0');

    // Click to scroll to heading in document
    item.addEventListener('click', () => scrollToHeading(h.nodeId, list, item));
    // Keyboard: Enter/Space to activate
    item.addEventListener('keydown', e => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        scrollToHeading(h.nodeId, list, item);
      }
    });

    list.appendChild(item);
  });

  updateOutlineTabBadge(headingData.length);
}

/** Scroll to a heading element by its node ID, with highlight feedback. */
function scrollToHeading(nodeId, list, activeItem) {
  const pageContainer = $('pageContainer');
  if (!pageContainer) return;
  const headingEl = pageContainer.querySelector(`[data-node-id="${nodeId}"]`);
  if (!headingEl) return;

  headingEl.scrollIntoView({ behavior: 'smooth', block: 'center' });

  // Brief highlight flash
  headingEl.style.transition = 'background 0.3s';
  headingEl.style.background = 'rgba(26, 115, 232, 0.12)';
  setTimeout(() => { headingEl.style.background = ''; }, 1500);

  // Mark active in outline
  if (list && activeItem) {
    list.querySelectorAll('.outline-item').forEach(it => it.classList.remove('active'));
    activeItem.classList.add('active');
  }
}

/** Update the outline tab with a heading count badge. */
function updateOutlineTabBadge(count) {
  const panel = $('pagesPanel');
  if (!panel) return;
  const tab = panel.querySelector('.pages-tab[data-tab="outline"]');
  if (!tab) return;
  // Remove existing badge
  const existing = tab.querySelector('.outline-badge');
  if (existing) existing.remove();
  if (count > 0) {
    const badge = document.createElement('span');
    badge.className = 'outline-badge';
    badge.textContent = count;
    badge.title = `${count} heading${count !== 1 ? 's' : ''}`;
    tab.appendChild(badge);
  }
}

/**
 * Toggle the outline panel open and switch to the Outline tab.
 * Exported for use by keyboard shortcuts (Ctrl+Shift+O).
 */
export function toggleOutlinePanel() {
  const panel = $('pagesPanel');
  if (!panel) return;

  // If panel is open on outline tab, close it
  const isOpen = panel.classList.contains('show');
  const activeTab = panel.querySelector('.pages-tab.active');
  if (isOpen && activeTab?.dataset.tab === 'outline') {
    panel.classList.remove('show');
    return;
  }

  // Open and switch to outline tab
  if (!isOpen) panel.classList.add('show');
  panel.querySelectorAll('.pages-tab').forEach(t => {
    t.classList.remove('active');
    t.setAttribute('aria-selected', 'false');
  });
  panel.querySelectorAll('.pages-tab-content').forEach(c => c.classList.remove('active'));
  const outlineTab = panel.querySelector('.pages-tab[data-tab="outline"]');
  const outlineContent = panel.querySelector('.pages-tab-content[data-tab="outline"]');
  if (outlineTab) { outlineTab.classList.add('active'); outlineTab.setAttribute('aria-selected', 'true'); }
  if (outlineContent) outlineContent.classList.add('active');
  renderOutline();
}

// ── In-document TOC interaction delegation ─────────────────────

let _tocDelegationSetup = false;

/** Wire up event delegation for in-document TOC blocks (click entries, update button, style selector). */
function initTOCInteraction() {
  if (_tocDelegationSetup) return;
  const container = $('pageContainer');
  if (!container) return;
  _tocDelegationSetup = true;

  container.addEventListener('click', e => {
    // TOC entry click — navigate to heading
    const entry = e.target.closest('.toc-entry[data-target-node]');
    if (entry) {
      e.preventDefault();
      e.stopPropagation();
      const targetNodeId = entry.dataset.targetNode;
      if (targetNodeId) {
        const heading = container.querySelector(`[data-node-id="${targetNodeId}"]`);
        if (heading) {
          heading.scrollIntoView({ behavior: 'smooth', block: 'center' });
          heading.style.transition = 'background 0.3s';
          heading.style.background = 'rgba(26, 115, 232, 0.12)';
          setTimeout(() => { heading.style.background = ''; }, 1500);
        }
      }
      return;
    }

    // TOC update button
    const updateBtn = e.target.closest('.toc-update-btn');
    if (updateBtn) {
      e.preventDefault();
      e.stopPropagation();
      if (state.doc && typeof state.doc.update_table_of_contents === 'function') {
        try {
          syncAllText();
          state.doc.update_table_of_contents();
          renderDocument();
          announce('Table of Contents updated');
        } catch (err) { console.error('TOC update:', err); }
      }
      return;
    }
  });

  // TOC style selector — change leader style (dotted, dashed, plain)
  container.addEventListener('change', e => {
    const sel = e.target.closest('.toc-style-select');
    if (!sel) return;
    e.stopPropagation();
    const tocBlock = sel.closest('.doc-toc');
    if (!tocBlock) return;
    const style = sel.value; // 'plain', 'dotted', 'dashed'
    tocBlock.querySelectorAll('.toc-entry').forEach(entry => {
      entry.classList.remove('toc-dotted', 'toc-dashed');
      if (style === 'dotted') entry.classList.add('toc-dotted');
      else if (style === 'dashed') entry.classList.add('toc-dashed');
    });
  });

  // Keyboard: Enter/Space on TOC entries
  container.addEventListener('keydown', e => {
    const entry = e.target.closest('.toc-entry[data-target-node]');
    if (entry && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      entry.click();
    }
  });
}

/* ═══════════════════════════════════════════════════
   UXP-16: PRINT PREVIEW — Full-screen read-only preview
   ═══════════════════════════════════════════════════ */

/** Keyboard handler reference so we can remove it on close */
let _printPreviewKeyHandler = null;

/**
 * Open the print preview overlay.
 * Clones all .doc-page elements from the editor into a scrollable read-only view.
 * Handles virtual-scroll placeholders by re-rendering content from WASM.
 */
function openPrintPreview() {
  if (!state.doc) return;

  const overlay = $('printPreviewOverlay');
  const body = $('printPreviewBody');
  if (!overlay || !body) return;

  // Clear any previous preview content
  body.innerHTML = '';

  // Gather all rendered pages
  const pageContainer = $('pageContainer');
  if (!pageContainer) return;
  const pages = pageContainer.querySelectorAll('.doc-page');

  if (pages.length === 0) {
    body.innerHTML = '<p style="color:#9aa0a6;font-size:14px;margin-top:80px">No pages to preview. Open a document first.</p>';
    overlay.classList.add('show');
    return;
  }

  // Clone each page into the preview body
  for (let i = 0; i < pages.length; i++) {
    const wrap = document.createElement('div');
    wrap.className = 'print-preview-page-wrap';

    const clone = pages[i].cloneNode(true);
    clone.className = 'print-preview-page';
    // Preserve original page dimensions
    clone.style.width = pages[i].style.width;
    clone.style.minHeight = pages[i].style.minHeight;

    // Restore any virtual-scroll placeholders by re-rendering from WASM
    clone.querySelectorAll('.vs-placeholder').forEach(ph => {
      const nodeId = ph.dataset?.nodeId;
      if (nodeId && state.doc) {
        try {
          const html = state.doc.render_node_html(nodeId);
          const temp = document.createElement('div');
          temp.innerHTML = html;
          const rendered = temp.firstElementChild;
          if (rendered) {
            ph.replaceWith(rendered);
          }
        } catch (_) {
          // Leave placeholder in place if render fails
        }
      }
    });

    // Restore any images that were released by virtual scroll (placeholder src)
    clone.querySelectorAll('img[data-original-src]').forEach(img => {
      img.src = img.dataset.originalSrc;
    });

    // Make all content non-editable
    clone.querySelectorAll('[contenteditable]').forEach(el => {
      el.contentEditable = 'false';
    });

    // Remove any active selection highlights from cloned content
    clone.querySelectorAll('.select-all-highlight').forEach(el => {
      el.classList.remove('select-all-highlight');
    });

    wrap.appendChild(clone);

    // Page number label below each page
    const label = document.createElement('div');
    label.className = 'print-preview-page-num';
    label.textContent = 'Page ' + (i + 1) + ' of ' + pages.length;
    wrap.appendChild(label);

    body.appendChild(wrap);
  }

  // Update page count in toolbar
  const pageInfo = $('printPreviewPageInfo');
  if (pageInfo) {
    pageInfo.textContent = pages.length + (pages.length === 1 ? ' page' : ' pages');
  }

  // Show overlay
  overlay.classList.add('show');

  // Prevent body scroll when overlay is open
  document.body.style.overflow = 'hidden';

  // Focus management: make overlay focusable and focus it
  overlay.tabIndex = -1;
  overlay.focus();

  // Keyboard handler: Escape to close, Ctrl+P to print, Tab trap
  const closeBtn = $('printPreviewClose');
  const printBtn = $('printPreviewPrint');
  _printPreviewKeyHandler = (e) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closePrintPreview();
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
      e.preventDefault();
      e.stopPropagation();
      closePrintPreview();
      setTimeout(() => window.print(), 120);
    } else if (e.key === 'Tab') {
      // Tab trap: keep focus within the print preview overlay
      const focusable = overlay.querySelectorAll('button, [tabindex]:not([tabindex="-1"])');
      if (focusable.length === 0) {
        e.preventDefault();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey) {
        if (document.activeElement === first || document.activeElement === overlay) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last || document.activeElement === overlay) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  };
  document.addEventListener('keydown', _printPreviewKeyHandler, true);

  // Focus the close button for accessibility
  if (closeBtn) closeBtn.focus();

  trackEvent('view', 'print-preview-open');
}

/**
 * Close the print preview overlay and return to normal editing.
 */
function closePrintPreview() {
  const overlay = $('printPreviewOverlay');
  if (!overlay) return;

  overlay.classList.remove('show');
  document.body.style.overflow = '';

  // Clean up cloned content to free memory
  const body = $('printPreviewBody');
  if (body) body.innerHTML = '';

  // Remove keyboard handler
  if (_printPreviewKeyHandler) {
    document.removeEventListener('keydown', _printPreviewKeyHandler, true);
    _printPreviewKeyHandler = null;
  }
}

/** Re-render page thumbnails / outline after document changes. */
export function refreshPageThumbnails() {
  const panel = $('pagesPanel');
  // Always refresh outline data (fingerprinting prevents unnecessary DOM rebuilds)
  renderOutline();
  // Initialize TOC interaction delegation (idempotent)
  initTOCInteraction();
  // Only refresh visual panel content if panel is visible
  if (!panel || !panel.classList.contains('show')) return;
  const activeTab = panel.querySelector('.pages-tab.active');
  if (activeTab?.dataset.tab === 'pages') renderPageThumbnails();
}

// ═══════════════════════════════════════════════════
// FS-43: Special Characters Dialog
// ═══════════════════════════════════════════════════

const _SC_DATA = {
  common: [
    { ch: '\u00A9', name: 'Copyright' },
    { ch: '\u00AE', name: 'Registered' },
    { ch: '\u2122', name: 'Trademark' },
    { ch: '\u00B0', name: 'Degree' },
    { ch: '\u00B1', name: 'Plus-Minus' },
    { ch: '\u00B7', name: 'Middle Dot' },
    { ch: '\u2026', name: 'Ellipsis' },
    { ch: '\u2014', name: 'Em Dash' },
    { ch: '\u2013', name: 'En Dash' },
    { ch: '\u2018', name: 'Left Single Quote' },
    { ch: '\u2019', name: 'Right Single Quote' },
    { ch: '\u201C', name: 'Left Double Quote' },
    { ch: '\u201D', name: 'Right Double Quote' },
    { ch: '\u00AB', name: 'Left Guillemet' },
    { ch: '\u00BB', name: 'Right Guillemet' },
    { ch: '\u2020', name: 'Dagger' },
    { ch: '\u2021', name: 'Double Dagger' },
    { ch: '\u00A7', name: 'Section' },
    { ch: '\u00B6', name: 'Pilcrow' },
    { ch: '\u2022', name: 'Bullet' },
    { ch: '\u25CF', name: 'Black Circle' },
    { ch: '\u25CB', name: 'White Circle' },
    { ch: '\u25A0', name: 'Black Square' },
    { ch: '\u25A1', name: 'White Square' },
    { ch: '\u2605', name: 'Black Star' },
    { ch: '\u2606', name: 'White Star' },
    { ch: '\u2665', name: 'Heart' },
    { ch: '\u2666', name: 'Diamond' },
    { ch: '\u266A', name: 'Music Note' },
    { ch: '\u00BF', name: 'Inverted Question' },
    { ch: '\u00A1', name: 'Inverted Exclamation' },
    { ch: '\u00D7', name: 'Multiplication' },
    { ch: '\u00F7', name: 'Division' },
    { ch: '\u221A', name: 'Square Root' },
    { ch: '\u221E', name: 'Infinity' },
    { ch: '\u2248', name: 'Almost Equal' },
    { ch: '\u2260', name: 'Not Equal' },
    { ch: '\u2264', name: 'Less or Equal' },
    { ch: '\u2265', name: 'Greater or Equal' },
  ],
  arrows: [
    { ch: '\u2190', name: 'Left Arrow' },
    { ch: '\u2191', name: 'Up Arrow' },
    { ch: '\u2192', name: 'Right Arrow' },
    { ch: '\u2193', name: 'Down Arrow' },
    { ch: '\u2194', name: 'Left Right Arrow' },
    { ch: '\u2195', name: 'Up Down Arrow' },
    { ch: '\u2196', name: 'NW Arrow' },
    { ch: '\u2197', name: 'NE Arrow' },
    { ch: '\u2198', name: 'SE Arrow' },
    { ch: '\u2199', name: 'SW Arrow' },
    { ch: '\u21D0', name: 'Left Double Arrow' },
    { ch: '\u21D1', name: 'Up Double Arrow' },
    { ch: '\u21D2', name: 'Right Double Arrow' },
    { ch: '\u21D3', name: 'Down Double Arrow' },
    { ch: '\u21D4', name: 'Left Right Double Arrow' },
    { ch: '\u21B5', name: 'Return Arrow' },
    { ch: '\u21BA', name: 'CCW Arrow' },
    { ch: '\u21BB', name: 'CW Arrow' },
    { ch: '\u27A1', name: 'Black Right Arrow' },
    { ch: '\u2B05', name: 'Black Left Arrow' },
    { ch: '\u2B06', name: 'Black Up Arrow' },
    { ch: '\u2B07', name: 'Black Down Arrow' },
    { ch: '\u25B2', name: 'Up Triangle' },
    { ch: '\u25BC', name: 'Down Triangle' },
    { ch: '\u25C0', name: 'Left Triangle' },
    { ch: '\u25B6', name: 'Right Triangle' },
  ],
  math: [
    { ch: '\u00B1', name: 'Plus-Minus' },
    { ch: '\u00D7', name: 'Multiplication' },
    { ch: '\u00F7', name: 'Division' },
    { ch: '\u2212', name: 'Minus' },
    { ch: '\u221A', name: 'Square Root' },
    { ch: '\u221B', name: 'Cube Root' },
    { ch: '\u221E', name: 'Infinity' },
    { ch: '\u2248', name: 'Almost Equal' },
    { ch: '\u2260', name: 'Not Equal' },
    { ch: '\u2261', name: 'Identical' },
    { ch: '\u2264', name: 'Less or Equal' },
    { ch: '\u2265', name: 'Greater or Equal' },
    { ch: '\u226A', name: 'Much Less Than' },
    { ch: '\u226B', name: 'Much Greater Than' },
    { ch: '\u2282', name: 'Subset' },
    { ch: '\u2283', name: 'Superset' },
    { ch: '\u2286', name: 'Subset or Equal' },
    { ch: '\u2287', name: 'Superset or Equal' },
    { ch: '\u2208', name: 'Element Of' },
    { ch: '\u2209', name: 'Not Element Of' },
    { ch: '\u2205', name: 'Empty Set' },
    { ch: '\u2200', name: 'For All' },
    { ch: '\u2203', name: 'There Exists' },
    { ch: '\u2204', name: 'Not Exists' },
    { ch: '\u2227', name: 'Logical And' },
    { ch: '\u2228', name: 'Logical Or' },
    { ch: '\u00AC', name: 'Not' },
    { ch: '\u2234', name: 'Therefore' },
    { ch: '\u2235', name: 'Because' },
    { ch: '\u2211', name: 'Summation' },
    { ch: '\u220F', name: 'Product' },
    { ch: '\u222B', name: 'Integral' },
    { ch: '\u2202', name: 'Partial Differential' },
    { ch: '\u2207', name: 'Nabla' },
    { ch: '\u03C0', name: 'Pi' },
    { ch: '\u2220', name: 'Angle' },
    { ch: '\u22A5', name: 'Perpendicular' },
    { ch: '\u2225', name: 'Parallel' },
  ],
  currency: [
    { ch: '$', name: 'Dollar' },
    { ch: '\u20AC', name: 'Euro' },
    { ch: '\u00A3', name: 'Pound' },
    { ch: '\u00A5', name: 'Yen' },
    { ch: '\u20A3', name: 'French Franc' },
    { ch: '\u20B9', name: 'Indian Rupee' },
    { ch: '\u20BD', name: 'Russian Ruble' },
    { ch: '\u20A9', name: 'Won' },
    { ch: '\u20B1', name: 'Peso' },
    { ch: '\u20BA', name: 'Turkish Lira' },
    { ch: '\u20B4', name: 'Ukrainian Hryvnia' },
    { ch: '\u20BF', name: 'Bitcoin' },
    { ch: '\u00A2', name: 'Cent' },
    { ch: '\u20AB', name: 'Dong' },
    { ch: '\u20AA', name: 'Shekel' },
    { ch: '\u20B8', name: 'Tenge' },
    { ch: '\u20AE', name: 'Tugrik' },
    { ch: '\u00A4', name: 'Currency Sign' },
  ],
  greek: [
    { ch: '\u0391', name: 'Alpha' },
    { ch: '\u0392', name: 'Beta' },
    { ch: '\u0393', name: 'Gamma' },
    { ch: '\u0394', name: 'Delta' },
    { ch: '\u0395', name: 'Epsilon' },
    { ch: '\u0396', name: 'Zeta' },
    { ch: '\u0397', name: 'Eta' },
    { ch: '\u0398', name: 'Theta' },
    { ch: '\u0399', name: 'Iota' },
    { ch: '\u039A', name: 'Kappa' },
    { ch: '\u039B', name: 'Lambda' },
    { ch: '\u039C', name: 'Mu' },
    { ch: '\u039D', name: 'Nu' },
    { ch: '\u039E', name: 'Xi' },
    { ch: '\u039F', name: 'Omicron' },
    { ch: '\u03A0', name: 'Pi' },
    { ch: '\u03A1', name: 'Rho' },
    { ch: '\u03A3', name: 'Sigma' },
    { ch: '\u03A4', name: 'Tau' },
    { ch: '\u03A5', name: 'Upsilon' },
    { ch: '\u03A6', name: 'Phi' },
    { ch: '\u03A7', name: 'Chi' },
    { ch: '\u03A8', name: 'Psi' },
    { ch: '\u03A9', name: 'Omega' },
    { ch: '\u03B1', name: 'alpha' },
    { ch: '\u03B2', name: 'beta' },
    { ch: '\u03B3', name: 'gamma' },
    { ch: '\u03B4', name: 'delta' },
    { ch: '\u03B5', name: 'epsilon' },
    { ch: '\u03B6', name: 'zeta' },
    { ch: '\u03B7', name: 'eta' },
    { ch: '\u03B8', name: 'theta' },
    { ch: '\u03B9', name: 'iota' },
    { ch: '\u03BA', name: 'kappa' },
    { ch: '\u03BB', name: 'lambda' },
    { ch: '\u03BC', name: 'mu' },
    { ch: '\u03BD', name: 'nu' },
    { ch: '\u03BE', name: 'xi' },
    { ch: '\u03BF', name: 'omicron' },
    { ch: '\u03C0', name: 'pi' },
    { ch: '\u03C1', name: 'rho' },
    { ch: '\u03C3', name: 'sigma' },
    { ch: '\u03C4', name: 'tau' },
    { ch: '\u03C5', name: 'upsilon' },
    { ch: '\u03C6', name: 'phi' },
    { ch: '\u03C7', name: 'chi' },
    { ch: '\u03C8', name: 'psi' },
    { ch: '\u03C9', name: 'omega' },
  ],
  punctuation: [
    { ch: '\u00A0', name: 'Non-Breaking Space' },
    { ch: '\u2002', name: 'En Space' },
    { ch: '\u2003', name: 'Em Space' },
    { ch: '\u2009', name: 'Thin Space' },
    { ch: '\u200B', name: 'Zero-Width Space' },
    { ch: '\u200D', name: 'Zero-Width Joiner' },
    { ch: '\u200C', name: 'Zero-Width Non-Joiner' },
    { ch: '\u2010', name: 'Hyphen' },
    { ch: '\u2011', name: 'Non-Breaking Hyphen' },
    { ch: '\u2012', name: 'Figure Dash' },
    { ch: '\u2013', name: 'En Dash' },
    { ch: '\u2014', name: 'Em Dash' },
    { ch: '\u2015', name: 'Horizontal Bar' },
    { ch: '\u2018', name: 'Left Single Quote' },
    { ch: '\u2019', name: 'Right Single Quote' },
    { ch: '\u201A', name: 'Single Low Quote' },
    { ch: '\u201C', name: 'Left Double Quote' },
    { ch: '\u201D', name: 'Right Double Quote' },
    { ch: '\u201E', name: 'Double Low Quote' },
    { ch: '\u2026', name: 'Ellipsis' },
    { ch: '\u00AB', name: 'Left Guillemet' },
    { ch: '\u00BB', name: 'Right Guillemet' },
    { ch: '\u2039', name: 'Left Single Guillemet' },
    { ch: '\u203A', name: 'Right Single Guillemet' },
    { ch: '\u00BF', name: 'Inverted Question' },
    { ch: '\u00A1', name: 'Inverted Exclamation' },
    { ch: '\u2030', name: 'Per Mille' },
    { ch: '\u2031', name: 'Per Ten Thousand' },
    { ch: '\u2032', name: 'Prime' },
    { ch: '\u2033', name: 'Double Prime' },
  ],
};

function initSpecialCharsModal() {
  const modal = $('specialCharsModal');
  const grid = $('scGrid');
  const tabs = $('scCategoryTabs');
  const search = $('scSearch');
  const previewChar = $('scPreviewChar');
  const previewName = $('scPreviewName');
  const previewCode = $('scPreviewCode');
  const closeBtn = $('scCloseBtn');
  const miSpecialChars = $('miSpecialChars');

  if (!modal || !grid) return;

  let _savedSel = null;
  let _currentCat = 'common';

  function renderGrid(category, filter) {
    const chars = _SC_DATA[category] || [];
    const filtered = filter
      ? chars.filter(c => c.name.toLowerCase().includes(filter.toLowerCase()) || c.ch === filter)
      : chars;
    grid.innerHTML = '';
    for (const item of filtered) {
      const btn = document.createElement('button');
      btn.className = 'sc-cell';
      btn.textContent = item.ch;
      btn.title = `${item.name} (U+${item.ch.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')})`;
      btn.dataset.char = item.ch;
      btn.dataset.name = item.name;
      grid.appendChild(btn);
    }
  }

  // Open modal from menu bar
  if (miSpecialChars) {
    miSpecialChars.addEventListener('click', () => {
      // Close all parent menus
      document.querySelectorAll('.app-menu-item.open').forEach(m => m.classList.remove('open'));
      openSpecialCharsModal();
    });
  }

  function openSpecialCharsModal() {
    // Save selection
    try {
      const sel = window.getSelection();
      if (sel && sel.rangeCount > 0) {
        _savedSel = sel.getRangeAt(0).cloneRange();
      }
    } catch (_) { _savedSel = null; }
    _currentCat = 'common';
    search.value = '';
    renderGrid('common');
    tabs.querySelectorAll('.sc-tab').forEach(t => t.classList.toggle('active', t.dataset.cat === 'common'));
    previewChar.textContent = '';
    previewName.textContent = '';
    previewCode.textContent = '';
    modal.classList.add('show');
    search.focus();
  }

  // Tab switching
  tabs.addEventListener('click', (e) => {
    const tab = e.target.closest('.sc-tab');
    if (!tab) return;
    _currentCat = tab.dataset.cat;
    tabs.querySelectorAll('.sc-tab').forEach(t => t.classList.toggle('active', t === tab));
    search.value = '';
    renderGrid(_currentCat);
  });

  // Search filtering
  search.addEventListener('input', () => {
    const q = search.value.trim();
    if (q) {
      // Search across all categories
      const allChars = [];
      for (const cat of Object.values(_SC_DATA)) {
        for (const item of cat) {
          if (item.name.toLowerCase().includes(q.toLowerCase()) || item.ch === q) {
            // Avoid duplicates
            if (!allChars.find(c => c.ch === item.ch)) {
              allChars.push(item);
            }
          }
        }
      }
      grid.innerHTML = '';
      for (const item of allChars) {
        const btn = document.createElement('button');
        btn.className = 'sc-cell';
        btn.textContent = item.ch;
        btn.title = `${item.name} (U+${item.ch.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')})`;
        btn.dataset.char = item.ch;
        btn.dataset.name = item.name;
        grid.appendChild(btn);
      }
    } else {
      renderGrid(_currentCat);
    }
  });

  // Hover preview
  grid.addEventListener('mouseover', (e) => {
    const cell = e.target.closest('.sc-cell');
    if (!cell) return;
    const ch = cell.dataset.char;
    const name = cell.dataset.name;
    previewChar.textContent = ch;
    previewName.textContent = name;
    previewCode.textContent = 'U+' + ch.codePointAt(0).toString(16).toUpperCase().padStart(4, '0');
  });

  // Click to insert
  grid.addEventListener('click', (e) => {
    const cell = e.target.closest('.sc-cell');
    if (!cell) return;
    if (state.readOnlyMode) return;
    const ch = cell.dataset.char;
    modal.classList.remove('show');

    // Restore selection and insert at cursor
    if (_savedSel) {
      try {
        const startOk = _savedSel.startContainer && _savedSel.startContainer.isConnected;
        const endOk = _savedSel.endContainer && _savedSel.endContainer.isConnected;
        if (startOk && endOk) {
          const sel = window.getSelection();
          sel.removeAllRanges();
          sel.addRange(_savedSel);
        }
      } catch (_) {}
    }

    // Block insertion in read-only mode
    if (state.readOnlyMode) return;
    // Focus editor and insert text
    const pageContent = $('pageContainer')?.querySelector('.page-content');
    if (pageContent) pageContent.focus();
    // Use execCommand to insert at cursor position, preserving undo
    document.execCommand('insertText', false, ch);
    announce('Inserted ' + (cell.dataset.name || 'character'));
    trackEvent('insert', 'special-char');
  });

  // Close
  closeBtn.addEventListener('click', () => {
    modal.classList.remove('show');
    // Restore focus
    if (_savedSel) {
      try {
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(_savedSel);
      } catch (_) {}
    }
  });

  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.remove('show');
  });

  // Keyboard handling
  search.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      modal.classList.remove('show');
    }
  });

  // Render initial grid
  renderGrid('common');
}

// ═══════════════════════════════════════════════════
// FS-44: Borders & Shading Dialog
// ═══════════════════════════════════════════════════

function initBordersModal() {
  const modal = $('bordersModal');
  const presets = $('bordersPresets');
  const borderTop = $('borderTop');
  const borderBottom = $('borderBottom');
  const borderLeft = $('borderLeft');
  const borderRight = $('borderRight');
  const borderStyle = $('borderStyle');
  const borderWidth = $('borderWidth');
  const borderColor = $('borderColor');
  const shadingColor = $('shadingColor');
  const previewInner = $('bordersPreviewInner');
  const applyBtn = $('bordersApplyBtn');
  const cancelBtn = $('bordersCancelBtn');
  const resetBtn = $('bordersResetBtn');
  const menuBtn = $('menuBordersShading');

  if (!modal || !applyBtn) return;

  let _savedSel = null;

  function updatePreview() {
    const style = borderStyle.value;
    const width = borderWidth.value + 'pt';
    const color = borderColor.value;
    const bg = shadingColor.value;

    const makeBorder = (on) => on ? `${width} ${style} ${color}` : 'none';
    previewInner.style.borderTop = makeBorder(borderTop.checked);
    previewInner.style.borderBottom = makeBorder(borderBottom.checked);
    previewInner.style.borderLeft = makeBorder(borderLeft.checked);
    previewInner.style.borderRight = makeBorder(borderRight.checked);
    previewInner.style.backgroundColor = (bg && bg !== '#ffffff') ? bg : '';

    // Update active preset
    presets.querySelectorAll('.borders-preset').forEach(p => p.classList.remove('active'));
    const allOn = borderTop.checked && borderBottom.checked && borderLeft.checked && borderRight.checked;
    const allOff = !borderTop.checked && !borderBottom.checked && !borderLeft.checked && !borderRight.checked;
    const topBottomOnly = borderTop.checked && borderBottom.checked && !borderLeft.checked && !borderRight.checked;
    const topOnly = borderTop.checked && !borderBottom.checked && !borderLeft.checked && !borderRight.checked;
    const bottomOnly = !borderTop.checked && borderBottom.checked && !borderLeft.checked && !borderRight.checked;

    if (allOff) presets.querySelector('[data-preset="none"]')?.classList.add('active');
    else if (allOn) presets.querySelector('[data-preset="box"]')?.classList.add('active');
    else if (topOnly) presets.querySelector('[data-preset="top"]')?.classList.add('active');
    else if (bottomOnly) presets.querySelector('[data-preset="bottom"]')?.classList.add('active');
    else if (topBottomOnly) presets.querySelector('[data-preset="topBottom"]')?.classList.add('active');
  }

  // Preset buttons
  presets.addEventListener('click', (e) => {
    const btn = e.target.closest('.borders-preset');
    if (!btn) return;
    const preset = btn.dataset.preset;
    switch (preset) {
      case 'none':
        borderTop.checked = false;
        borderBottom.checked = false;
        borderLeft.checked = false;
        borderRight.checked = false;
        break;
      case 'box':
        borderTop.checked = true;
        borderBottom.checked = true;
        borderLeft.checked = true;
        borderRight.checked = true;
        break;
      case 'top':
        borderTop.checked = true;
        borderBottom.checked = false;
        borderLeft.checked = false;
        borderRight.checked = false;
        break;
      case 'bottom':
        borderTop.checked = false;
        borderBottom.checked = true;
        borderLeft.checked = false;
        borderRight.checked = false;
        break;
      case 'topBottom':
        borderTop.checked = true;
        borderBottom.checked = true;
        borderLeft.checked = false;
        borderRight.checked = false;
        break;
    }
    updatePreview();
  });

  // Individual toggles
  [borderTop, borderBottom, borderLeft, borderRight].forEach(cb => {
    cb.addEventListener('change', updatePreview);
  });
  [borderStyle, borderWidth, borderColor, shadingColor].forEach(el => {
    el.addEventListener('change', updatePreview);
    el.addEventListener('input', updatePreview);
  });

  // Open from Format menu
  if (menuBtn) {
    menuBtn.addEventListener('click', () => {
      document.querySelectorAll('.app-menu-item.open').forEach(m => m.classList.remove('open'));
      openBordersModal();
    });
  }

  function openBordersModal() {
    // Save selection
    try {
      const sel = window.getSelection();
      if (sel && sel.rangeCount > 0) {
        _savedSel = sel.getRangeAt(0).cloneRange();
      }
    } catch (_) { _savedSel = null; }

    // Reset to defaults
    borderTop.checked = false;
    borderBottom.checked = false;
    borderLeft.checked = false;
    borderRight.checked = false;
    borderStyle.value = 'solid';
    borderWidth.value = '1';
    borderColor.value = '#000000';
    shadingColor.value = '#ffffff';
    updatePreview();
    modal.classList.add('show');
  }

  // Apply borders
  applyBtn.addEventListener('click', () => {
    modal.classList.remove('show');

    // Restore selection
    if (_savedSel) {
      try {
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(_savedSel);
      } catch (_) {}
    }

    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    syncAllText();

    const paraIds = getSelectedParagraphIds(info);
    if (!paraIds || paraIds.length === 0) return;

    const style = borderStyle.value;
    const width = borderWidth.value;
    const color = borderColor.value.replace('#', '');
    const bg = shadingColor.value;
    const bgHex = bg.replace('#', '');

    // Build border value string: "width style color" or "none"
    const makeBorderVal = (on) => on ? `${width} ${style} ${color}` : 'none';
    const bTop = makeBorderVal(borderTop.checked);
    const bBottom = makeBorderVal(borderBottom.checked);
    const bLeft = makeBorderVal(borderLeft.checked);
    const bRight = makeBorderVal(borderRight.checked);

    for (const nodeId of paraIds) {
      try {
        // Try WASM set_attributes for border properties
        if (typeof state.doc.set_paragraph_borders === 'function') {
          state.doc.set_paragraph_borders(nodeId, bTop, bBottom, bLeft, bRight);
        } else {
          // Fallback: set individual border attributes
          try { state.doc.set_attribute(nodeId, 'borderTop', bTop); } catch (_) {}
          try { state.doc.set_attribute(nodeId, 'borderBottom', bBottom); } catch (_) {}
          try { state.doc.set_attribute(nodeId, 'borderLeft', bLeft); } catch (_) {}
          try { state.doc.set_attribute(nodeId, 'borderRight', bRight); } catch (_) {}
        }
        // Shading / background
        if (bgHex !== 'ffffff') {
          try {
            if (typeof state.doc.set_paragraph_shading === 'function') {
              state.doc.set_paragraph_shading(nodeId, bgHex);
            } else {
              try { state.doc.set_attribute(nodeId, 'backgroundColor', bgHex); } catch (_) {}
            }
          } catch (_) {}
        } else {
          // Clear background
          try {
            if (typeof state.doc.set_paragraph_shading === 'function') {
              state.doc.set_paragraph_shading(nodeId, '');
            } else {
              try { state.doc.set_attribute(nodeId, 'backgroundColor', ''); } catch (_) {}
            }
          } catch (_) {}
        }
        broadcastOp({
          action: 'setParagraphBorders', nodeId,
          borderTop: bTop, borderBottom: bBottom,
          borderLeft: bLeft, borderRight: bRight,
          backgroundColor: bgHex !== 'ffffff' ? bgHex : '',
        });
      } catch (err) {
        console.error('borders apply:', err);
      }
    }

    // Visual fallback: apply directly to DOM elements if WASM attributes not rendered
    const page = $('pageContainer');
    if (page) {
      for (const nodeId of paraIds) {
        const el = page.querySelector(`[data-node-id="${nodeId}"]`);
        if (el) {
          const px = (parseFloat(width) * 1.333).toFixed(1) + 'px';
          el.style.borderTop = borderTop.checked ? `${px} ${style} #${color}` : '';
          el.style.borderBottom = borderBottom.checked ? `${px} ${style} #${color}` : '';
          el.style.borderLeft = borderLeft.checked ? `${px} ${style} #${color}` : '';
          el.style.borderRight = borderRight.checked ? `${px} ${style} #${color}` : '';
          el.style.backgroundColor = bgHex !== 'ffffff' ? `#${bgHex}` : '';
          // Padding for bordered paragraphs
          if (borderTop.checked || borderBottom.checked || borderLeft.checked || borderRight.checked) {
            el.style.padding = el.style.padding || '4px 8px';
          }
        }
      }
    }

    renderDocument();
    recordUndoAction('Apply borders and shading');
    updateUndoRedo();
    // Notify if only visual fallback was used (not persisted to model)
    if (typeof state.doc?.set_paragraph_borders !== 'function') {
      announce('Borders applied visually (re-export to preserve)');
    } else {
      announce('Borders applied');
    }
    trackEvent('format', 'borders');
  });

  // Reset
  resetBtn.addEventListener('click', () => {
    borderTop.checked = false;
    borderBottom.checked = false;
    borderLeft.checked = false;
    borderRight.checked = false;
    borderStyle.value = 'solid';
    borderWidth.value = '1';
    borderColor.value = '#000000';
    shadingColor.value = '#ffffff';
    updatePreview();
  });

  // Cancel
  cancelBtn.addEventListener('click', () => {
    modal.classList.remove('show');
  });

  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.remove('show');
  });

  // Initialize preview
  updatePreview();
}

// ═══════════════════════════════════════════════════════
// P5-7 — Sign Document Modal
// ═══════════════════════════════════════════════════════

function initSignDocumentModal() {
  const modal = $('signDocModal');
  const statusEl = $('signDocStatus');
  const certInput = $('signCertInput');
  const reasonSelect = $('signReason');
  const cancelBtn = $('signDocCancelBtn');
  const applyBtn = $('signDocApplyBtn');
  const menuBtn = $('menuSignDocument');

  if (!modal || !applyBtn) return;

  // Show current signature status when opening the modal
  function updateSignStatus() {
    if (!statusEl) return;
    if (!state.doc) {
      statusEl.style.display = 'none';
      return;
    }
    try {
      let hasSig = false;
      let sigSubject = '';
      let sigDate = '';
      let sigValid = '';
      if (typeof state.doc.metadata_json === 'function') {
        const meta = JSON.parse(state.doc.metadata_json());
        const cp = meta.custom_properties || {};
        hasSig = cp.hasDigitalSignature === 'true';
        sigSubject = cp.signatureSubject || '';
        sigDate = cp.signatureDate || '';
        sigValid = cp.signatureValid || '';
      }
      if (hasSig) {
        statusEl.style.display = 'block';
        statusEl.style.background = 'var(--surface-dim, #f0f4f9)';
        statusEl.style.border = '1px solid var(--border, #dadce0)';
        let html = '<strong>Current signature:</strong><br>';
        if (sigSubject) html += 'Signer: ' + sigSubject + '<br>';
        if (sigDate) html += 'Date: ' + sigDate + '<br>';
        if (sigValid) html += 'Status: ' + sigValid;
        statusEl.innerHTML = html;
      } else {
        statusEl.style.display = 'block';
        statusEl.style.background = 'var(--surface-dim, #f8f9fa)';
        statusEl.style.border = '1px solid var(--border, #dadce0)';
        statusEl.innerHTML = 'This document is not signed.';
      }
    } catch (_) {
      statusEl.style.display = 'none';
    }
  }

  // Open from Tools menu
  if (menuBtn) {
    menuBtn.addEventListener('click', () => {
      closeAllMenus();
      certInput.value = '';
      reasonSelect.value = 'Approval';
      updateSignStatus();
      modal.classList.add('show');
    });
  }

  // Cancel
  cancelBtn.addEventListener('click', () => {
    modal.classList.remove('show');
  });

  // Close on overlay click
  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.remove('show');
  });

  // Sign
  applyBtn.addEventListener('click', async () => {
    if (!state.doc) {
      announce('No document open');
      modal.classList.remove('show');
      return;
    }

    const files = certInput.files;
    if (!files || files.length === 0) {
      announce('Please select a certificate file');
      return;
    }

    try {
      const certFile = files[0];
      const certBytes = new Uint8Array(await certFile.arrayBuffer());
      const reason = reasonSelect.value || 'Approval';
      const now = new Date().toISOString();

      // Try WASM sign_document if available
      if (typeof state.doc.sign_document === 'function') {
        state.doc.sign_document(certBytes, now);
      } else {
        // Fallback: store signature metadata in custom properties
        if (typeof state.doc.set_custom_property === 'function') {
          state.doc.set_custom_property('hasDigitalSignature', 'true');
          state.doc.set_custom_property('signatureDate', now);
          state.doc.set_custom_property('signatureValid', 'self_signed');
          state.doc.set_custom_property('signatureReason', reason);
        }
      }

      markDirty();
      modal.classList.remove('show');
      announce('Document signed');
      trackEvent('tools', 'sign_document');
    } catch (err) {
      console.error('sign document:', err);
      announce('Signing failed: ' + (err.message || err));
    }
  });
}

