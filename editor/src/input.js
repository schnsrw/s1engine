// Keyboard, input, paste, clipboard handling
import { state, $ } from './state.js';
import {
  getSelectionInfo, getActiveElement, getCursorOffset,
  setCursorAtOffset, setCursorAtStart, isCursorAtStart, isCursorAtEnd,
} from './selection.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText, debouncedSync } from './render.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo } from './toolbar.js';
import { deleteSelectedImage, setupImages } from './images.js';
import { updatePageBreaks } from './pagination.js';
import { markDirty } from './file.js';

export function initInput() {
  const page = $('docPage');

  // ─── Regular input (typing) ─────────────────────
  page.addEventListener('input', () => {
    if (state.ignoreInput) return;
    const el = getActiveElement();
    if (el) debouncedSync(el);
  });

  // ─── Copy — write both plain text and HTML to clipboard ───
  page.addEventListener('copy', e => {
    if (!state.doc) return;
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) return;

    e.preventDefault();
    const text = sel.toString();
    const html = getSelectionHtml();

    // Store internal clipboard for rich paste within editor
    syncAllText();
    storeInternalClipboard();

    e.clipboardData.setData('text/plain', text);
    e.clipboardData.setData('text/html', html);
  });

  // ─── Keydown ────────────────────────────────────
  page.addEventListener('keydown', e => {
    if (!state.doc) return;
    const doc = state.doc;

    // Delete selected image
    if (state.selectedImg && (e.key === 'Delete' || e.key === 'Backspace')) {
      e.preventDefault(); deleteSelectedImage(); return;
    }

    const info = getSelectionInfo();

    // ── Ctrl/Cmd shortcuts ──
    if (e.ctrlKey || e.metaKey) {
      switch (e.key.toLowerCase()) {
        case 'b': e.preventDefault(); toggleFormat('bold'); return;
        case 'i': e.preventDefault(); toggleFormat('italic'); return;
        case 'u': e.preventDefault(); toggleFormat('underline'); return;
        case 'z': e.preventDefault(); e.shiftKey ? doRedo() : doUndo(); return;
        case 'y': e.preventDefault(); doRedo(); return;
        case 'x': e.preventDefault(); doCut(e); return;
        case 'c': /* handled by copy event above */ return;
        case 'v': /* handled by paste event */ return;
        case 'a': /* let browser handle select all */ return;
        case 's': e.preventDefault(); saveToLocal(); return;
        case 'f': e.preventDefault(); $('findBar').classList.add('show'); $('findInput').focus(); return;
        case 'h': e.preventDefault(); $('findBar').classList.add('show'); $('replaceInput')?.focus(); return;
        case 'p': e.preventDefault(); window.print(); return;
      }
    }

    // ── Delete/Backspace with selection ──
    if ((e.key === 'Delete' || e.key === 'Backspace') && info && !info.collapsed) {
      e.preventDefault();
      clearTimeout(state.syncTimer);
      syncAllText();
      try {
        doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
        renderDocument();
        const el = page.querySelector(`[data-node-id="${info.startNodeId}"]`);
        if (el) setCursorAtOffset(el, info.startOffset);
        else {
          const first = page.querySelector('[data-node-id]');
          if (first) setCursorAtStart(first);
          else {
            // Document is empty — create a new paragraph to keep it editable
            try { doc.append_paragraph(''); } catch (_) {}
            renderDocument();
            const n = page.querySelector('[data-node-id]');
            if (n) setCursorAtStart(n);
          }
        }
        updateUndoRedo();
      } catch (err) { console.error('delete selection:', err); }
      return;
    }

    const el = getActiveElement();

    // ── Tab — table navigation ──
    if (e.key === 'Tab') {
      const cell = el?.closest?.('td, th');
      if (cell) {
        e.preventDefault();
        const row = cell.parentElement;
        const table = row?.closest('table');
        if (!table) return;
        const cells = Array.from(table.querySelectorAll('td, th'));
        const idx = cells.indexOf(cell);
        const next = e.shiftKey ? cells[idx - 1] : cells[idx + 1];
        if (next) {
          const textNode = next.querySelector('[data-node-id]');
          if (textNode) { setCursorAtStart(textNode); }
          else { next.focus(); }
        }
        return;
      }
    }

    // ── Shift+Enter — insert line break ──
    if (e.key === 'Enter' && e.shiftKey) {
      e.preventDefault();
      if (!el) return;
      const nodeId = el.dataset.nodeId;
      const offset = getCursorOffset(el);
      clearTimeout(state.syncTimer); syncParagraphText(el);
      try {
        doc.insert_line_break(nodeId, offset);
        const updated = renderNodeById(nodeId);
        if (updated) setCursorAtOffset(updated, offset + 1);
        state.pagesRendered = false; updatePageBreaks(); updateUndoRedo();
      } catch (_) {
        // Fallback: insert newline character directly
        document.execCommand('insertLineBreak');
      }
      return;
    }

    // ── Enter — split paragraph ──
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!el) return;
      const nodeId = el.dataset.nodeId;
      const offset = getCursorOffset(el);
      clearTimeout(state.syncTimer); syncParagraphText(el);
      try {
        const newId = doc.split_paragraph(nodeId, offset);
        renderNodeById(nodeId);
        const newHtml = doc.render_node_html(newId);
        const tmp = document.createElement('div'); tmp.innerHTML = newHtml;
        const newEl = tmp.firstElementChild;
        if (newEl) {
          if (!newEl.innerHTML.trim()) newEl.innerHTML = '<br>';
          const orig = page.querySelector(`[data-node-id="${nodeId}"]`);
          if (orig) orig.after(newEl);
          setupImages(newEl);
          setCursorAtStart(newEl);
        }
        state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
      } catch (err) { console.error('split:', err); }
      return;
    }

    // ── Backspace at start — merge prev ──
    if (e.key === 'Backspace' && el && isCursorAtStart(el)) {
      let prev = el.previousElementSibling;
      while (prev && (prev.classList.contains('page-break') || prev.classList.contains('editor-footer'))) prev = prev.previousElementSibling;
      if (prev?.dataset?.nodeId) {
        e.preventDefault();
        clearTimeout(state.syncTimer); syncParagraphText(el); syncParagraphText(prev);
        const cursorPos = Array.from(prev.textContent || '').length;
        try {
          doc.merge_paragraphs(prev.dataset.nodeId, el.dataset.nodeId);
          const updated = renderNodeById(prev.dataset.nodeId);
          el.remove();
          if (updated) setCursorAtOffset(updated, cursorPos);
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
        } catch (err) { console.error('merge:', err); }
      }
      return;
    }

    // ── Delete at end — merge next ──
    if (e.key === 'Delete' && el && isCursorAtEnd(el)) {
      let next = el.nextElementSibling;
      while (next && (next.classList.contains('page-break') || next.classList.contains('editor-footer'))) next = next.nextElementSibling;
      if (next?.dataset?.nodeId) {
        e.preventDefault();
        clearTimeout(state.syncTimer); syncParagraphText(el); syncParagraphText(next);
        const cursorPos = Array.from(el.textContent || '').length;
        try {
          doc.merge_paragraphs(el.dataset.nodeId, next.dataset.nodeId);
          const updated = renderNodeById(el.dataset.nodeId);
          next.remove();
          if (updated) setCursorAtOffset(updated, cursorPos);
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
        } catch (err) { console.error('merge:', err); }
      }
      return;
    }
  });

  // ─── Paste ──────────────────────────────────────
  page.addEventListener('paste', e => {
    e.preventDefault();
    if (!state.doc) return;

    // Try internal rich clipboard first (replaces entire document — preserves all formatting)
    if (state.internalClipboard) {
      try {
        restoreFromInternalClipboard();
        return;
      } catch (err) {
        console.error('internal paste failed, falling back:', err);
      }
    }

    const info = getSelectionInfo();
    if (!info) return;

    // Delete selection first if not collapsed
    if (!info.collapsed) {
      syncAllText();
      try {
        state.doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
      } catch (_) {}
    } else {
      syncParagraphText(info.startEl);
    }

    // Fall back to plain text paste
    const text = e.clipboardData.getData('text/plain');
    if (!text) return;

    if (text.includes('\n')) {
      try {
        state.doc.paste_plain_text(info.startNodeId, info.startOffset, text);
        renderDocument();
        updateUndoRedo();
      } catch (_) {
        insertTextAtCursor(text.replace(/\n/g, ' '));
      }
    } else {
      try {
        state.doc.insert_text_in_paragraph(info.startNodeId, info.startOffset, text);
        const updated = renderNodeById(info.startNodeId);
        if (updated) setCursorAtOffset(updated, info.startOffset + Array.from(text).length);
        updateUndoRedo();
      } catch (_) {
        insertTextAtCursor(text);
      }
    }
  });

  // ─── Selection change ──────────────────────────
  document.addEventListener('selectionchange', updateToolbarState);

  // ─── Prevent toolbar from stealing focus ───────
  $('toolbar').addEventListener('mousedown', e => {
    const tag = e.target.tagName.toLowerCase();
    if (tag !== 'select' && tag !== 'input') e.preventDefault();
  });

  // ─── Global Escape handler — close modals/menus ──
  document.addEventListener('keydown', e => {
    if (e.key !== 'Escape') return;
    // Close find bar
    if ($('findBar').classList.contains('show')) {
      $('findBar').classList.remove('show');
      const docPage = $('docPage');
      if (docPage) docPage.focus();
      return;
    }
    // Close table modal
    if ($('tableModal').classList.contains('show')) {
      $('tableModal').classList.remove('show');
      return;
    }
    // Close menus
    $('exportMenu').classList.remove('show');
    $('insertMenu').classList.remove('show');
    $('tableContextMenu').style.display = 'none';
    // Close comments panel
    if ($('commentsPanel').classList.contains('show')) {
      $('commentsPanel').classList.remove('show');
      return;
    }
  });
}

// ─── Internal Clipboard System ─────────────────────
// Stores the full document state before cut, so paste restores everything

function storeInternalClipboard() {
  // Internal clipboard disabled — use standard paste flow
  // The old approach replaced the entire document on paste, causing data loss
  state.internalClipboard = null;
}

function restoreFromInternalClipboard() {
  // Disabled — no-op. Standard paste flow handles all cases.
  state.internalClipboard = null;
  throw new Error('Internal clipboard disabled');
}

function getSelectionHtml() {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return '';
  const range = sel.getRangeAt(0);
  const div = document.createElement('div');
  div.appendChild(range.cloneContents());
  return div.innerHTML;
}

function insertTextAtCursor(text) {
  const sel = window.getSelection();
  if (!sel?.rangeCount) return;
  const range = sel.getRangeAt(0);
  range.deleteContents();
  range.insertNode(document.createTextNode(text));
  range.collapse(false);
  sel.removeAllRanges(); sel.addRange(range);
  const el = getActiveElement();
  if (el) debouncedSync(el);
}

function doUndo() {
  if (!state.doc) return;
  clearTimeout(state.syncTimer);
  syncAllText();
  try { state.doc.undo(); renderDocument(); updateToolbarState(); }
  catch (e) { console.error('undo:', e); }
}

function doRedo() {
  if (!state.doc) return;
  try { state.doc.redo(); renderDocument(); updateToolbarState(); }
  catch (e) { console.error('redo:', e); }
}

function doCut() {
  const info = getSelectionInfo();
  if (!info || info.collapsed || !state.doc) return;

  // Store document state for rich paste
  syncAllText();
  storeInternalClipboard();

  // Copy HTML + plain text to system clipboard
  const sel = window.getSelection();
  if (sel) {
    const text = sel.toString();
    const html = getSelectionHtml();
    // Use clipboard API with both formats
    try {
      const blob = new Blob([html], { type: 'text/html' });
      const textBlob = new Blob([text], { type: 'text/plain' });
      navigator.clipboard.write([
        new ClipboardItem({ 'text/html': blob, 'text/plain': textBlob })
      ]).catch(() => {
        navigator.clipboard.writeText(text).catch(() => {});
      });
    } catch (_) {
      navigator.clipboard.writeText(text).catch(() => {});
    }
  }

  // Delete the selection
  try {
    state.doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
    renderDocument();
    const el = $('docPage').querySelector(`[data-node-id="${info.startNodeId}"]`);
    if (el) setCursorAtOffset(el, info.startOffset);
    else {
      const first = $('docPage').querySelector('[data-node-id]');
      if (first) setCursorAtStart(first);
      else { state.doc.append_paragraph(''); renderDocument(); }
    }
    updateUndoRedo();
  } catch (e) { console.error('cut:', e); }
}

function saveToLocal() {
  if (!state.doc) return;
  try {
    syncAllText();
    const bytes = state.doc.export('docx');
    const name = $('docName').value || 'Untitled Document';
    const req = indexedDB.open('FolioAutosave', 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('documents')) {
        db.createObjectStore('documents', { keyPath: 'id' });
      }
    };
    req.onsuccess = () => {
      const db = req.result;
      const tx = db.transaction('documents', 'readwrite');
      tx.objectStore('documents').put({ id: 'current', name, bytes, timestamp: Date.now() });
      state.dirty = false;
      const info = $('statusInfo');
      const prev = info.textContent;
      info.textContent = 'Saved';
      setTimeout(() => { info.textContent = prev; }, 1500);
    };
  } catch (e) { console.error('save:', e); }
}

// Expose for toolbar buttons
export { doUndo, doRedo };
