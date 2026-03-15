// Keyboard, input, paste, clipboard handling
import { state, $ } from './state.js';
import {
  getSelectionInfo, getActiveElement, getCursorOffset,
  setCursorAtOffset, setCursorAtStart, isCursorAtStart, isCursorAtEnd,
} from './selection.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText, debouncedSync } from './render.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo, recordUndoAction, renderUndoHistory } from './toolbar.js';
import { deleteSelectedImage, setupImages } from './images.js';
import { updatePageBreaks } from './pagination.js';
import { markDirty, saveVersion, updateDirtyIndicator, updateStatusBar } from './file.js';
import { broadcastOp } from './collab.js';
import { setZoomLevel } from './toolbar-handlers.js';

export function initInput() {
  const page = $('pageContainer');

  // ─── E-01 fix: Capture cursor offset before text insertion for pending formats ───
  page.addEventListener('beforeinput', (e) => {
    if (state.ignoreInput) return;
    // E1.6 fix: Prevent browser from deleting page-break / HR elements via
    // native deletion (deleteContentBackward/Forward). Check if any target
    // range includes a non-editable element.
    if (e.inputType && e.inputType.startsWith('delete') && e.getTargetRanges) {
      const ranges = e.getTargetRanges();
      for (const r of ranges) {
        // Walk through the range to see if it spans a page-break or HR
        const container = r.commonAncestorContainer;
        const parent = container.nodeType === 1 ? container : container.parentElement;
        if (parent) {
          const breaks = parent.querySelectorAll ? parent.querySelectorAll('.page-break, hr.page-break, .editor-header, .editor-footer') : [];
          for (const b of breaks) {
            if (r.intersectsNode?.(b)) {
              e.preventDefault();
              return;
            }
          }
        }
      }
    }
    if (e.inputType === 'insertText' && e.data) {
      const pending = state.pendingFormats;
      if (pending && Object.keys(pending).length > 0) {
        const el = getActiveElement();
        if (el) {
          state._pendingFormatInsert = {
            nodeId: el.dataset.nodeId,
            offset: getCursorOffset(el),
            charCount: Array.from(e.data).length,
          };
        }
      }
    }
  });

  // ─── Regular input (typing) ─────────────────────
  page.addEventListener('input', (e) => {
    if (state.ignoreInput) return;
    const el = getActiveElement();
    if (el) debouncedSync(el);

    // ── E-01 fix: Apply pending formats to newly inserted character(s) ──
    if (state._pendingFormatInsert && e.inputType === 'insertText') {
      const pfi = state._pendingFormatInsert;
      state._pendingFormatInsert = null;
      const pending = state.pendingFormats;
      if (pending && Object.keys(pending).length > 0 && state.doc) {
        // Sync the paragraph text immediately so the WASM model has the new character
        if (el) {
          clearTimeout(state.syncTimer);
          syncParagraphText(el);
        }
        try {
          const startOff = pfi.offset;
          const endOff = startOff + pfi.charCount;
          const nodeId = pfi.nodeId;
          for (const [key, value] of Object.entries(pending)) {
            state.doc.format_selection(nodeId, startOff, nodeId, endOff, key, value);
            broadcastOp({ action: 'formatSelection', startNode: nodeId, startOffset: startOff, endNode: nodeId, endOffset: endOff, key, value });
          }
          // Re-render the node to show the formatting, then restore cursor
          const updated = renderNodeById(nodeId);
          if (updated) setCursorAtOffset(updated, endOff);
          // Update the tracked cursor position so selectionchange doesn't
          // clear pending formats due to the cursor advancing by one character
          state._pendingFormatCursorPos = { nodeId, offset: endOff };
          updateUndoRedo();
        } catch (err) { console.error('pending format apply:', err); }
        // Keep pending formats active so subsequent characters also get formatted
        // They will be cleared when the user clicks somewhere else or changes selection
      }
    }

    // ── Slash menu: detect "/" or update filter ──
    if (state.slashMenuOpen) {
      const text = el?.textContent || '';
      const offset = getCursorOffset(el);
      // Find the "/" that triggered the menu
      // offset is in codepoints, so convert text to codepoint array for slicing
      const codepoints = [...text];
      const before = codepoints.slice(0, offset).join('');
      const slashPos = before.lastIndexOf('/');
      if (slashPos >= 0) {
        const query = before.substring(slashPos + 1);
        updateSlashFilter(query);
      } else {
        closeSlashMenu();
      }
    } else if (e.inputType === 'insertText' && e.data === '/') {
      // Open menu if "/" is at start of paragraph or after whitespace
      if (el) {
        const offset = getCursorOffset(el);
        const text = el.textContent || '';
        // offset is in codepoints, so index into codepoint array
        const codepoints = [...text];
        const charBefore = offset >= 2 ? codepoints[offset - 2] : null;
        if (offset === 1 || (charBefore && /\s/.test(charBefore))) {
          openSlashMenu();
        }
      }
    }
  });

  // ─── Copy — write both plain text and HTML to clipboard via WASM ───
  page.addEventListener('copy', e => {
    if (!state.doc) return;
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) return;

    e.preventDefault();
    const text = sel.toString();

    // E2.1: Generate clean semantic HTML from WASM model (no data attributes, no node IDs)
    syncAllText();
    const info = getSelectionInfo();
    let html = '';
    if (info && info.startNodeId && info.endNodeId) {
      try {
        html = state.doc.export_selection_html(
          info.startNodeId, info.startOffset,
          info.endNodeId, info.endOffset
        );
      } catch (err) {
        console.warn('WASM export_selection_html failed, falling back to DOM:', err);
        html = getSelectionHtml();
      }
    } else {
      html = getSelectionHtml();
    }

    e.clipboardData.setData('text/plain', text);
    e.clipboardData.setData('text/html', html);
  });

  // ─── Keydown ────────────────────────────────────
  page.addEventListener('keydown', e => {
    if (!state.doc) return;
    const doc = state.doc;

    // ── Slash menu navigation ──
    if (state.slashMenuOpen) {
      const commands = filterSlashCommands(state.slashQuery);
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        state.slashMenuIndex = Math.min(state.slashMenuIndex + 1, commands.length - 1);
        renderSlashMenu(commands);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        state.slashMenuIndex = Math.max(state.slashMenuIndex - 1, 0);
        renderSlashMenu(commands);
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        if (commands.length > 0 && state.slashMenuIndex < commands.length) {
          executeSlashCommand(commands[state.slashMenuIndex].id);
        } else {
          closeSlashMenu();
        }
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        closeSlashMenu();
        return;
      }
      if (e.key === 'Backspace' || e.key === 'Delete') {
        // If query is empty, the "/" itself will be deleted, so close menu
        if (state.slashQuery.length === 0) {
          closeSlashMenu();
          // Let backspace/delete proceed to remove the "/"
        } else {
          // Let it proceed normally; after the DOM updates, verify the slash
          // trigger character is still present. Use setTimeout(0) so the check
          // runs after the browser applies the deletion to the DOM.
          setTimeout(() => {
            if (!state.slashMenuOpen) return;
            const activeEl = getActiveElement();
            const text = activeEl?.textContent || '';
            const cursorOff = activeEl ? getCursorOffset(activeEl) : 0;
            // cursorOff is in codepoints, so slice codepoint array
            const before = [...text].slice(0, cursorOff).join('');
            if (before.lastIndexOf('/') < 0) {
              closeSlashMenu();
            }
          }, 0);
        }
      }
    }

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
        case '=':
        case '+': e.preventDefault(); adjustEditorZoom(10); return;
        case '-': e.preventDefault(); adjustEditorZoom(-10); return;
        case '0': e.preventDefault(); adjustEditorZoom(0); return;
        case '/': e.preventDefault(); { const m = document.getElementById('shortcutsModal'); if (m) m.classList.add('show'); } return;
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
        // Try to place cursor at the start of the deletion point
        let el = page.querySelector(`[data-node-id="${info.startNodeId}"]`);
        if (el) {
          setCursorAtOffset(el, info.startOffset);
        } else {
          // The start node was deleted — find any remaining paragraph
          el = page.querySelector('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id]');
          if (el) {
            setCursorAtStart(el);
          } else {
            // Document is completely empty — create a new paragraph
            try {
              doc.append_paragraph('');
              broadcastOp({ action: 'insertParagraph', afterNodeId: null, text: '' });
            } catch (_) {}
            renderDocument();
            const n = page.querySelector('[data-node-id]');
            if (n) setCursorAtStart(n);
          }
        }
        recordUndoAction('Delete selection');
        updateUndoRedo();
        broadcastOp({ action: 'deleteSelection', startNode: info.startNodeId, startOffset: info.startOffset, endNode: info.endNodeId, endOffset: info.endOffset });
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
        } else if (!e.shiftKey && idx === cells.length - 1) {
          // E4.1: Tab in last cell — insert new row
          const tableNodeId = table.dataset?.nodeId;
          if (tableNodeId && doc) {
            try {
              syncAllText();
              const dims = JSON.parse(doc.get_table_dimensions(tableNodeId));
              const rowIdx = dims.rows; // Insert at end
              doc.insert_table_row(tableNodeId, rowIdx);
              broadcastOp({ action: 'insertTableRow', tableNodeId, rowIndex: rowIdx });
              renderDocument();
              // Focus first cell of new row
              const updatedTable = page.querySelector(`[data-node-id="${tableNodeId}"]`);
              if (updatedTable) {
                const newCells = updatedTable.querySelectorAll('td, th');
                const firstNewCell = newCells[newCells.length - dims.cols];
                if (firstNewCell) {
                  const tn = firstNewCell.querySelector('[data-node-id]');
                  if (tn) setCursorAtStart(tn);
                }
              }
              updateUndoRedo(); markDirty();
            } catch (err) { console.error('Tab add row:', err); }
          }
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
        broadcastOp({ action: 'insertLineBreak', nodeId, offset });
        const updated = renderNodeById(nodeId);
        if (updated) setCursorAtOffset(updated, offset + 1);
        recordUndoAction('Insert line break');
        state.pagesRendered = false; updatePageBreaks(); updateUndoRedo();
      } catch (err) {
        console.error('insert line break:', err);
      }
      return;
    }

    // ── Enter — split paragraph ──
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      state._typingBatch = null; // E3.1: End typing session on Enter
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
        recordUndoAction('Split paragraph');
        state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
        broadcastOp({ action: 'splitParagraph', nodeId, offset });
      } catch (err) { console.error('split:', err); }
      return;
    }

    // ── E1.6: Prevent deletion of page-break / HR divs ──
    if ((e.key === 'Delete' || e.key === 'Backspace') && !el) {
      // Cursor might be on a non-editable element (page-break, HR)
      const sel = window.getSelection();
      if (sel && sel.anchorNode) {
        const anchor = sel.anchorNode.nodeType === 1 ? sel.anchorNode : sel.anchorNode.parentElement;
        if (anchor && (anchor.classList?.contains('page-break') || anchor.tagName === 'HR' ||
            anchor.closest?.('.page-break') || anchor.closest?.('.editor-header') || anchor.closest?.('.editor-footer'))) {
          e.preventDefault();
          return;
        }
      }
    }

    // ── Backspace at start — merge prev ──
    if (e.key === 'Backspace' && el && isCursorAtStart(el)) {
      let prev = el.previousElementSibling;
      // Skip CSS pagination divs (editor-header/footer, page-break DIVs) but
      // NOT model-level page break HRs — those should block merging.
      while (prev && prev.tagName !== 'HR' && (prev.classList.contains('page-break') || prev.classList.contains('editor-footer') || prev.classList.contains('editor-header'))) prev = prev.previousElementSibling;
      // If we hit a model-level page break HR, don't merge across it
      if (prev && prev.tagName === 'HR' && prev.classList.contains('page-break')) {
        e.preventDefault();
        return;
      }
      if (prev?.dataset?.nodeId) {
        e.preventDefault();
        clearTimeout(state.syncTimer); syncParagraphText(el); syncParagraphText(prev);
        const cursorPos = Array.from(prev.textContent || '').length;
        const nodeId1 = prev.dataset.nodeId;
        const nodeId2 = el.dataset.nodeId;
        try {
          doc.merge_paragraphs(nodeId1, nodeId2);
          const updated = renderNodeById(nodeId1);
          el.remove();
          if (updated) setCursorAtOffset(updated, cursorPos);
          recordUndoAction('Merge paragraphs');
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
          broadcastOp({ action: 'mergeParagraphs', nodeId1, nodeId2 });
        } catch (err) { console.error('merge:', err); }
      }
      return;
    }

    // ── Delete at end — merge next ──
    if (e.key === 'Delete' && el && isCursorAtEnd(el)) {
      let next = el.nextElementSibling;
      // Skip CSS pagination divs but NOT model-level page break HRs
      while (next && next.tagName !== 'HR' && (next.classList.contains('page-break') || next.classList.contains('editor-footer') || next.classList.contains('editor-header'))) next = next.nextElementSibling;
      // If we hit a model-level page break HR, don't merge across it
      if (next && next.tagName === 'HR' && next.classList.contains('page-break')) {
        e.preventDefault();
        return;
      }
      if (next?.dataset?.nodeId) {
        e.preventDefault();
        clearTimeout(state.syncTimer); syncParagraphText(el); syncParagraphText(next);
        const cursorPos = Array.from(el.textContent || '').length;
        const nodeId1 = el.dataset.nodeId;
        const nodeId2 = next.dataset.nodeId;
        try {
          doc.merge_paragraphs(nodeId1, nodeId2);
          const updated = renderNodeById(nodeId1);
          next.remove();
          if (updated) setCursorAtOffset(updated, cursorPos);
          recordUndoAction('Merge paragraphs');
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
          broadcastOp({ action: 'mergeParagraphs', nodeId1, nodeId2 });
        } catch (err) { console.error('merge:', err); }
      }
      return;
    }
  });

  // ─── Paste ──────────────────────────────────────
  page.addEventListener('paste', e => {
    e.preventDefault();
    if (!state.doc) return;
    const doc = state.doc;

    let info = getSelectionInfo();

    // Delete selection first if not collapsed
    if (info && !info.collapsed) {
      syncAllText();
      try {
        doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
        renderDocument();
      } catch (_) {}
      // After delete + re-render, DOM is rebuilt. Clear stale selection info.
      state.lastSelInfo = null;
      info = null;
    } else if (info) {
      syncParagraphText(info.startEl);
    }

    // Ensure we have a valid target paragraph
    const ensureTarget = () => {
      // Try to get fresh selection after re-render
      let firstEl = page.querySelector('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]');
      if (!firstEl) {
        // Document is completely empty — create a paragraph
        try {
          doc.append_paragraph('');
          broadcastOp({ action: 'insertParagraph', afterNodeId: null, text: '' });
          renderDocument();
        } catch (_) {}
        firstEl = page.querySelector('[data-node-id]');
      }
      if (firstEl) {
        setCursorAtStart(firstEl);
        return { startNodeId: firstEl.dataset.nodeId, startOffset: 0, startEl: firstEl };
      }
      return null;
    };

    if (!info || !info.startNodeId) {
      info = ensureTarget();
      if (!info) return;
    } else {
      // Verify the node still exists in the DOM (might have been deleted)
      const existing = page.querySelector(`[data-node-id="${info.startNodeId}"]`);
      if (!existing) {
        info = ensureTarget();
        if (!info) return;
      }
    }

    const text = e.clipboardData.getData('text/plain');
    const html = e.clipboardData.getData('text/html');

    // E2.2: Try rich paste (HTML → formatted runs via WASM) first
    if (html && text) {
      const parsed = parseClipboardHtml(html);
      if (parsed && parsed.paragraphs.length > 0) {
        try {
          const runsJson = JSON.stringify(parsed);
          doc.paste_formatted_runs_json(info.startNodeId, info.startOffset, runsJson);
          broadcastOp({ action: 'pasteFormattedRuns', nodeId: info.startNodeId, offset: info.startOffset, runsJson });
          renderDocument();
          placeCursorAfterPaste(page, text);
          recordUndoAction('Paste formatted text');
          updateUndoRedo();
          markDirty();
          return;
        } catch (err) {
          console.warn('Rich paste failed, falling back to plain text:', err);
        }
      }
    }

    if (!text) return;

    if (text.includes('\n')) {
      try {
        doc.paste_plain_text(info.startNodeId, info.startOffset, text);
        broadcastOp({ action: 'pasteText', nodeId: info.startNodeId, offset: info.startOffset, text });
        renderDocument();
        placeCursorAfterPaste(page, text);
        recordUndoAction('Paste text');
        updateUndoRedo();
        markDirty();
      } catch (err) {
        console.error('paste multi-line:', err);
        // Fallback: insert as single line via WASM
        try {
          const flatText = text.replace(/\n/g, ' ');
          doc.insert_text_in_paragraph(info.startNodeId, info.startOffset, flatText);
          broadcastOp({ action: 'insertText', nodeId: info.startNodeId, offset: info.startOffset, text: flatText });
          renderDocument();
          updateUndoRedo();
        } catch (e2) { console.error('paste fallback:', e2); }
      }
    } else {
      try {
        doc.insert_text_in_paragraph(info.startNodeId, info.startOffset, text);
        broadcastOp({ action: 'insertText', nodeId: info.startNodeId, offset: info.startOffset, text });
        const updated = renderNodeById(info.startNodeId);
        if (updated) setCursorAtOffset(updated, info.startOffset + Array.from(text).length);
        recordUndoAction('Paste text');
        updateUndoRedo();
        markDirty();
      } catch (err) {
        console.error('paste single-line:', err);
      }
    }
  });

  // ─── Selection change ──────────────────────────
  // E-01 fix: Clear pending formats when cursor moves or selection changes
  document.addEventListener('selectionchange', () => {
    if (state.pendingFormats && Object.keys(state.pendingFormats).length > 0) {
      // Only clear if the selection actually moved to a different position
      // (toolbar mousedown prevention means formatting buttons won't trigger this)
      const sel = window.getSelection();
      if (sel && sel.rangeCount > 0) {
        const info = getSelectionInfo();
        const prev = state._pendingFormatCursorPos;
        if (info && prev) {
          const moved = info.startNodeId !== prev.nodeId || info.startOffset !== prev.offset;
          // If the cursor moved and we're not in the middle of a pending format insert,
          // clear the pending formats
          if (moved && !state._pendingFormatInsert) {
            state.pendingFormats = {};
          }
        }
        // Track current cursor position for comparison
        if (info) {
          state._pendingFormatCursorPos = { nodeId: info.startNodeId, offset: info.startOffset };
        }
      }
    }
    updateToolbarState();
  });

  // ─── E2.4: Text Drag & Drop within editor ─────
  let dragSourceInfo = null;

  page.addEventListener('dragstart', e => {
    if (state.selectedImg) return; // Image drag handled by images.js
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) return;
    const info = getSelectionInfo();
    if (!info || info.collapsed) return;
    syncAllText();
    dragSourceInfo = { ...info };
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', sel.toString());
    // Generate rich HTML for the drag
    try {
      const html = state.doc.export_selection_html(
        info.startNodeId, info.startOffset, info.endNodeId, info.endOffset
      );
      e.dataTransfer.setData('text/html', html);
    } catch (_) {}
  });

  page.addEventListener('dragover', e => {
    if (!dragSourceInfo) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  });

  page.addEventListener('drop', e => {
    if (!dragSourceInfo || !state.doc) return;
    e.preventDefault();
    const doc = state.doc;
    const text = e.dataTransfer.getData('text/plain');
    if (!text) { dragSourceInfo = null; return; }

    // Find drop target paragraph and offset from caret position
    let dropNode = null, dropOffset = 0;
    if (document.caretPositionFromPoint) {
      const pos = document.caretPositionFromPoint(e.clientX, e.clientY);
      if (pos) {
        const el = pos.offsetNode?.nodeType === 1 ? pos.offsetNode : pos.offsetNode?.parentElement;
        const block = el?.closest('[data-node-id]');
        if (block) { dropNode = block; dropOffset = getCursorOffset(block); }
      }
    } else if (document.caretRangeFromPoint) {
      const range = document.caretRangeFromPoint(e.clientX, e.clientY);
      if (range) {
        const el = range.startContainer?.nodeType === 1 ? range.startContainer : range.startContainer?.parentElement;
        const block = el?.closest('[data-node-id]');
        if (block) {
          // Approximate char offset from range
          const sel = window.getSelection();
          sel.removeAllRanges();
          sel.addRange(range);
          dropNode = block;
          dropOffset = getCursorOffset(block);
        }
      }
    }

    if (!dropNode || !dropNode.dataset.nodeId) { dragSourceInfo = null; return; }

    const dropNodeId = dropNode.dataset.nodeId;
    const src = dragSourceInfo;
    dragSourceInfo = null;

    try {
      // Delete source text first, then insert at drop position
      // Adjust drop offset if drop is in the same paragraph and after the deleted range
      doc.delete_selection(src.startNodeId, src.startOffset, src.endNodeId, src.endOffset);
      broadcastOp({ action: 'deleteSelection', startNode: src.startNodeId, startOffset: src.startOffset, endNode: src.endNodeId, endOffset: src.endOffset });

      // After deletion, the drop node might have been removed or offset changed
      // Re-render and find the drop target again
      renderDocument();
      const newDrop = page.querySelector(`[data-node-id="${dropNodeId}"]`);
      if (newDrop) {
        // Adjust offset if deleting from same node shifted the position
        let adjustedOffset = dropOffset;
        if (src.startNodeId === dropNodeId && src.endNodeId === dropNodeId) {
          if (dropOffset > src.endOffset) {
            adjustedOffset -= (src.endOffset - src.startOffset);
          } else if (dropOffset > src.startOffset) {
            adjustedOffset = src.startOffset;
          }
        }
        const maxLen = [...(newDrop.textContent || '')].length;
        adjustedOffset = Math.min(adjustedOffset, maxLen);

        doc.insert_text_in_paragraph(dropNodeId, adjustedOffset, text);
        broadcastOp({ action: 'insertText', nodeId: dropNodeId, offset: adjustedOffset, text });
        renderDocument();
        const updated = page.querySelector(`[data-node-id="${dropNodeId}"]`);
        if (updated) setCursorAtOffset(updated, adjustedOffset + [...text].length);
      } else {
        // Drop node was deleted — insert into first available paragraph
        const firstEl = page.querySelector('[data-node-id]');
        if (firstEl) {
          doc.insert_text_in_paragraph(firstEl.dataset.nodeId, 0, text);
          broadcastOp({ action: 'insertText', nodeId: firstEl.dataset.nodeId, offset: 0, text });
          renderDocument();
        }
      }
      updateUndoRedo();
      markDirty();
    } catch (err) { console.error('text drop:', err); }
  });

  page.addEventListener('dragend', () => { dragSourceInfo = null; });

  // ─── Prevent toolbar from stealing focus ───────
  $('toolbar').addEventListener('mousedown', e => {
    const tag = e.target.tagName.toLowerCase();
    if (tag !== 'select' && tag !== 'input') e.preventDefault();
  });

  // ─── Global Escape handler — close modals/menus ──
  document.addEventListener('keydown', e => {
    if (e.key !== 'Escape') return;
    // Close slash menu
    if (state.slashMenuOpen) {
      closeSlashMenu();
      return;
    }
    // Close find bar
    if ($('findBar').classList.contains('show')) {
      $('findBar').classList.remove('show');
      const activePage = $('pageContainer')?.querySelector('.page-content');
      if (activePage) activePage.focus();
      return;
    }
    // Close table modal
    if ($('tableModal').classList.contains('show')) {
      $('tableModal').classList.remove('show');
      return;
    }
    // Close comment modal
    if ($('commentModal').classList.contains('show')) {
      $('commentModal').classList.remove('show');
      return;
    }
    // Close link modal
    if ($('linkModal').classList.contains('show')) {
      $('linkModal').classList.remove('show');
      return;
    }
    // Close alt text modal
    if ($('altTextModal').classList.contains('show')) {
      $('altTextModal').classList.remove('show');
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
    // Close history panel
    if ($('historyPanel').classList.contains('show')) {
      $('historyPanel').classList.remove('show');
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

// insertTextAtCursor removed — all text insertion must go through WASM to maintain model consistency

// ─── E2.2: Parse clipboard HTML into structured runs for WASM ─────
// Converts HTML from clipboard (Google Docs, Word, LibreOffice, etc.) into
// the JSON format expected by paste_formatted_runs_json:
// { paragraphs: [{ runs: [{ text, bold, italic, ... }] }] }
function parseClipboardHtml(html) {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, 'text/html');
    const body = doc.body;
    if (!body || !body.childNodes.length) return null;

    const paragraphs = [];
    // Walk top-level block elements
    const blocks = body.querySelectorAll('p, h1, h2, h3, h4, h5, h6, div, li, tr');
    if (blocks.length === 0) {
      // No block elements — treat entire body as one paragraph
      const runs = extractRunsFromElement(body);
      if (runs.length > 0) paragraphs.push({ runs });
    } else {
      for (const block of blocks) {
        // Skip nested blocks (e.g., div inside p)
        if (block.closest('li') && block.tagName !== 'LI') continue;
        if (block.closest('td') && block.tagName !== 'TD' && block.tagName !== 'TR') continue;
        const runs = extractRunsFromElement(block);
        if (runs.length > 0) paragraphs.push({ runs });
      }
    }

    if (paragraphs.length === 0) {
      // Last resort: extract all text
      const text = body.textContent || '';
      if (text) return { paragraphs: [{ runs: [{ text }] }] };
      return null;
    }

    return { paragraphs };
  } catch (_) {
    return null;
  }
}

function extractRunsFromElement(el) {
  const runs = [];
  walkInline(el, {}, runs);
  // Merge adjacent runs with identical formatting
  const merged = [];
  for (const run of runs) {
    if (merged.length > 0) {
      const prev = merged[merged.length - 1];
      if (sameFormatting(prev, run)) {
        prev.text += run.text;
        continue;
      }
    }
    merged.push({ ...run });
  }
  return merged;
}

function walkInline(node, inherited, runs) {
  if (node.nodeType === 3) { // Text node
    const text = node.textContent;
    if (text) {
      runs.push({ text, ...inherited });
    }
    return;
  }
  if (node.nodeType !== 1) return;

  // Skip non-inline block elements nested inside (we handle blocks at top level)
  const tag = node.tagName.toLowerCase();

  // Build formatting from this element
  const fmt = { ...inherited };
  if (tag === 'b' || tag === 'strong') fmt.bold = true;
  if (tag === 'i' || tag === 'em') fmt.italic = true;
  if (tag === 'u') fmt.underline = true;
  if (tag === 's' || tag === 'strike' || tag === 'del') fmt.strikethrough = true;
  if (tag === 'sup') fmt.superscript = true;
  if (tag === 'sub') fmt.subscript = true;
  if (tag === 'br') {
    // Line breaks within a block become newline text
    runs.push({ text: '\n', ...inherited });
    return;
  }

  // Parse inline styles
  const style = node.style;
  if (style) {
    if (style.fontWeight === 'bold' || parseInt(style.fontWeight) >= 700) fmt.bold = true;
    if (style.fontStyle === 'italic') fmt.italic = true;
    if (style.textDecoration?.includes('underline')) fmt.underline = true;
    if (style.textDecoration?.includes('line-through')) fmt.strikethrough = true;
    if (style.fontSize) {
      const size = parseFloat(style.fontSize);
      if (size > 0) {
        // Convert px to pt (rough: 1pt ≈ 1.333px)
        if (style.fontSize.endsWith('px')) fmt.fontSize = Math.round(size * 0.75 * 10) / 10;
        else if (style.fontSize.endsWith('pt')) fmt.fontSize = size;
      }
    }
    if (style.fontFamily) {
      const ff = style.fontFamily.replace(/['"]/g, '').split(',')[0].trim();
      if (ff) fmt.fontFamily = ff;
    }
    if (style.color) {
      const hex = colorToHex(style.color);
      if (hex) fmt.color = hex;
    }
  }

  for (const child of node.childNodes) {
    walkInline(child, fmt, runs);
  }
}

function sameFormatting(a, b) {
  return !!a.bold === !!b.bold &&
    !!a.italic === !!b.italic &&
    !!a.underline === !!b.underline &&
    !!a.strikethrough === !!b.strikethrough &&
    (a.fontSize || null) === (b.fontSize || null) &&
    (a.fontFamily || null) === (b.fontFamily || null) &&
    (a.color || null) === (b.color || null);
}

function colorToHex(cssColor) {
  if (!cssColor) return null;
  // Already hex
  if (cssColor.startsWith('#')) {
    let hex = cssColor.slice(1);
    if (hex.length === 3) hex = hex[0]+hex[0]+hex[1]+hex[1]+hex[2]+hex[2];
    return hex.toUpperCase();
  }
  // rgb(r, g, b)
  const m = cssColor.match(/rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/);
  if (m) {
    const r = parseInt(m[1]).toString(16).padStart(2, '0');
    const g = parseInt(m[2]).toString(16).padStart(2, '0');
    const b = parseInt(m[3]).toString(16).padStart(2, '0');
    return (r + g + b).toUpperCase();
  }
  return null;
}

// Place cursor at end of pasted content
function placeCursorAfterPaste(page, text) {
  const lines = text.split('\n');
  const lastLine = lines[lines.length - 1];
  const allEls = Array.from(page.querySelectorAll('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]'));
  let targetEl = null;
  for (let i = allEls.length - 1; i >= 0; i--) {
    if ((allEls[i].textContent || '').endsWith(lastLine)) {
      targetEl = allEls[i];
      break;
    }
  }
  if (!targetEl && allEls.length > 0) targetEl = allEls[allEls.length - 1];
  if (targetEl) setCursorAtOffset(targetEl, [...(targetEl.textContent || '')].length);
}

function doUndo() {
  if (!state.doc) return;
  clearTimeout(state.syncTimer);
  syncAllText();
  try {
    // E3.1: Batch undo — if we're in a typing session, undo all typing steps at once
    const batch = state._typingBatch;
    if (batch && batch.count > 1) {
      const steps = batch.count;
      state._typingBatch = null;
      for (let i = 0; i < steps; i++) {
        if (!state.doc.can_undo()) break;
        state.doc.undo();
      }
    } else {
      state._typingBatch = null;
      state.doc.undo();
    }
    // E3.2: Advance undo history position
    state.undoHistoryPos = Math.min(state.undoHistoryPos + 1, state.undoHistory.length);
    renderDocument();
    updateToolbarState();
    renderUndoHistory();
    broadcastOp({ action: 'fullDocSync' });
  } catch (e) { console.error('undo:', e); }
}

function doRedo() {
  if (!state.doc) return;
  try {
    state.doc.redo();
    // E3.2: Move undo history position back
    state.undoHistoryPos = Math.max(state.undoHistoryPos - 1, 0);
    renderDocument();
    updateToolbarState();
    renderUndoHistory();
    broadcastOp({ action: 'fullDocSync' });
  } catch (e) { console.error('redo:', e); }
}

function doCut() {
  const info = getSelectionInfo();
  if (!info || info.collapsed || !state.doc) return;

  // E2.3: Copy via WASM then delete
  syncAllText();

  // Generate clean HTML from WASM model
  const sel = window.getSelection();
  if (sel) {
    const text = sel.toString();
    let html = '';
    try {
      html = state.doc.export_selection_html(
        info.startNodeId, info.startOffset,
        info.endNodeId, info.endOffset
      );
    } catch (err) {
      console.warn('WASM export_selection_html failed in cut, falling back:', err);
      html = getSelectionHtml();
    }
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
    broadcastOp({ action: 'deleteSelection', startNode: info.startNodeId, startOffset: info.startOffset, endNode: info.endNodeId, endOffset: info.endOffset });
    renderDocument();
    const el = $('pageContainer')?.querySelector(`[data-node-id="${info.startNodeId}"]`);
    if (el) setCursorAtOffset(el, info.startOffset);
    else {
      const first = $('pageContainer')?.querySelector('[data-node-id]');
      if (first) setCursorAtStart(first);
      else { state.doc.append_paragraph(''); renderDocument(); }
    }
    recordUndoAction('Cut text');
    updateUndoRedo();
  } catch (e) { console.error('cut:', e); }
}

function saveToLocal() {
  if (!state.doc) return;
  try {
    syncAllText();
    const bytes = state.doc.export('docx');
    const name = $('docName').value || 'Untitled Document';
    const req = indexedDB.open('FolioAutosave', 2);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('documents')) {
        db.createObjectStore('documents', { keyPath: 'id' });
      }
      if (!db.objectStoreNames.contains('versions')) {
        db.createObjectStore('versions', { keyPath: 'id', autoIncrement: true });
      }
    };
    req.onsuccess = () => {
      const db = req.result;
      const tx = db.transaction('documents', 'readwrite');
      tx.objectStore('documents').put({ id: 'current', name, bytes, timestamp: Date.now() });
      state.dirty = false;
      updateDirtyIndicator();
      const info = $('statusInfo');
      info._userMsg = true;
      info.textContent = 'Saved';
      setTimeout(() => { info._userMsg = false; updateStatusBar(); }, 1500);
    };
    // Also save a version snapshot on manual save
    saveVersion('Manual save');
  } catch (e) { console.error('save:', e); }
}

// ─── Slash Command Menu ─────────────────────────────
const SLASH_COMMANDS = [
  { id: 'heading1',   label: 'Heading 1',       icon: 'H1', keywords: 'heading h1 title' },
  { id: 'heading2',   label: 'Heading 2',       icon: 'H2', keywords: 'heading h2' },
  { id: 'heading3',   label: 'Heading 3',       icon: 'H3', keywords: 'heading h3' },
  { id: 'bullet',     label: 'Bullet List',     icon: '\u2022',  keywords: 'bullet list unordered ul' },
  { id: 'numbered',   label: 'Numbered List',   icon: '1.',  keywords: 'numbered list ordered ol' },
  { id: 'table',      label: 'Table',           icon: '\u2637',  keywords: 'table grid' },
  { id: 'image',      label: 'Image',           icon: '\uD83D\uDDBC',  keywords: 'image picture photo' },
  { id: 'hr',         label: 'Horizontal Rule', icon: '\u2014',  keywords: 'horizontal rule divider line separator hr' },
  { id: 'pagebreak',  label: 'Page Break',      icon: '\u23CE',  keywords: 'page break new page' },
  { id: 'quote',      label: 'Quote',           icon: '\u201C',  keywords: 'quote blockquote' },
  { id: 'code',       label: 'Code Block',      icon: '</>',keywords: 'code block monospace' },
];

function filterSlashCommands(query) {
  if (!query) return SLASH_COMMANDS;
  const q = query.toLowerCase();
  return SLASH_COMMANDS.filter(cmd =>
    cmd.label.toLowerCase().includes(q) || cmd.keywords.includes(q)
  );
}

function renderSlashMenu(commands) {
  const menu = $('slashMenu');
  if (commands.length === 0) {
    menu.style.display = 'none';
    state.slashMenuOpen = false;
    return;
  }
  menu.innerHTML = commands.map((cmd, i) =>
    `<div class="slash-menu-item${i === state.slashMenuIndex ? ' active' : ''}" data-cmd="${cmd.id}" role="option" aria-selected="${i === state.slashMenuIndex}">` +
      `<span class="slash-menu-icon">${cmd.icon}</span>` +
      `<span class="slash-menu-label">${cmd.label}</span>` +
    `</div>`
  ).join('');
  menu.style.display = 'block';

  // Scroll active item into view
  const activeItem = menu.querySelector('.slash-menu-item.active');
  if (activeItem) activeItem.scrollIntoView({ block: 'nearest' });

  // Click handler for each item
  menu.querySelectorAll('.slash-menu-item').forEach(item => {
    item.addEventListener('mousedown', e => {
      e.preventDefault();
      executeSlashCommand(item.dataset.cmd);
    });
  });
}

function positionSlashMenu() {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;
  const range = sel.getRangeAt(0);
  const rect = range.getBoundingClientRect();
  const menu = $('slashMenu');
  const canvas = $('editorCanvas');
  const canvasRect = canvas.getBoundingClientRect();

  let top = rect.bottom - canvasRect.top + canvas.scrollTop + 4;
  let left = rect.left - canvasRect.left;

  // Clamp within canvas bounds
  const menuW = 240;
  if (left + menuW > canvasRect.width) left = canvasRect.width - menuW - 8;
  if (left < 8) left = 8;

  menu.style.top = top + 'px';
  menu.style.left = left + 'px';
}

function openSlashMenu() {
  state.slashMenuOpen = true;
  state.slashMenuIndex = 0;
  state.slashQuery = '';
  const commands = filterSlashCommands('');
  renderSlashMenu(commands);
  positionSlashMenu();
}

function closeSlashMenu() {
  state.slashMenuOpen = false;
  state.slashQuery = '';
  state.slashMenuIndex = 0;
  $('slashMenu').style.display = 'none';
}

function updateSlashFilter(query) {
  state.slashQuery = query;
  state.slashMenuIndex = 0;
  const commands = filterSlashCommands(query);
  renderSlashMenu(commands);
  if (commands.length === 0) closeSlashMenu();
}

function deleteSlashText() {
  // Delete the "/" and any typed query text from the paragraph
  const el = getActiveElement();
  if (!el) return;
  const offset = getCursorOffset(el);
  const slashLen = 1 + state.slashQuery.length; // "/" + query
  const deleteFrom = Math.max(0, offset - slashLen);

  // Remove text by manipulating textContent
  const text = el.textContent || '';
  const chars = Array.from(text);
  chars.splice(deleteFrom, slashLen);
  el.textContent = chars.join('') || '';
  if (!el.textContent) el.innerHTML = '<br>';

  // Sync and restore cursor
  syncParagraphText(el);
  if (el.textContent && deleteFrom > 0) setCursorAtOffset(el, deleteFrom);
  else setCursorAtStart(el);
}

function executeSlashCommand(cmdId) {
  const doc = state.doc;
  if (!doc) { closeSlashMenu(); return; }

  const el = getActiveElement();
  const nodeId = el?.dataset?.nodeId;
  if (!nodeId) { closeSlashMenu(); return; }

  // Delete the slash text first
  deleteSlashText();
  closeSlashMenu();

  syncAllText();

  try {
    const bcastFmt = (key, value, len) => {
      doc.format_selection(nodeId, 0, nodeId, len, key, value);
      broadcastOp({ action: 'formatSelection', startNode: nodeId, startOffset: 0, endNode: nodeId, endOffset: len, key, value });
    };
    switch (cmdId) {
      case 'heading1': doc.set_heading_level(nodeId, 1); broadcastOp({ action: 'setHeading', nodeId, level: 1 }); renderDocument(); break;
      case 'heading2': doc.set_heading_level(nodeId, 2); broadcastOp({ action: 'setHeading', nodeId, level: 2 }); renderDocument(); break;
      case 'heading3': doc.set_heading_level(nodeId, 3); broadcastOp({ action: 'setHeading', nodeId, level: 3 }); renderDocument(); break;
      case 'bullet':   doc.set_list_format(nodeId, 'bullet', 0); broadcastOp({ action: 'setListFormat', nodeId, format: 'bullet', level: 0 }); renderDocument(); break;
      case 'numbered': doc.set_list_format(nodeId, 'decimal', 0); broadcastOp({ action: 'setListFormat', nodeId, format: 'decimal', level: 0 }); renderDocument(); break;
      case 'table':
        doc.insert_table(nodeId, 3, 3);
        broadcastOp({ action: 'insertTable', afterNodeId: nodeId, rows: 3, cols: 3 });
        renderDocument();
        break;
      case 'image':
        $('imageInput').click();
        break;
      case 'hr':
        doc.insert_horizontal_rule(nodeId);
        broadcastOp({ action: 'insertHR', afterNodeId: nodeId });
        renderDocument();
        break;
      case 'pagebreak':
        doc.insert_page_break(nodeId);
        broadcastOp({ action: 'insertPageBreak', afterNodeId: nodeId });
        renderDocument();
        break;
      case 'quote': {
        doc.set_heading_level(nodeId, 0);
        broadcastOp({ action: 'setHeading', nodeId, level: 0 });
        const textLen = el ? Array.from(el.textContent || '').length : 0;
        if (textLen > 0) {
          bcastFmt('italic', 'true', textLen);
          bcastFmt('color', '666666', textLen);
        }
        renderDocument();
        break;
      }
      case 'code': {
        doc.set_heading_level(nodeId, 0);
        broadcastOp({ action: 'setHeading', nodeId, level: 0 });
        const codeLen = el ? Array.from(el.textContent || '').length : 0;
        if (codeLen > 0) {
          bcastFmt('fontFamily', 'Courier New', codeLen);
          bcastFmt('fontSize', '11', codeLen);
        }
        renderDocument();
        break;
      }
    }
    // E3.4: Record slash command in undo history
    const labels = { heading1: 'Set Heading 1', heading2: 'Set Heading 2', heading3: 'Set Heading 3',
      bullet: 'Insert bullet list', numbered: 'Insert numbered list', table: 'Insert table',
      hr: 'Insert horizontal rule', pagebreak: 'Insert page break', quote: 'Apply quote style', code: 'Apply code style' };
    if (labels[cmdId]) recordUndoAction(labels[cmdId]);
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.error('slash command:', e); }
}

export { closeSlashMenu };

// Expose for toolbar buttons
export { doUndo, doRedo };

// E10.2: Zoom via keyboard (Ctrl+=/Ctrl+-/Ctrl+0)
function adjustEditorZoom(delta) {
  if (delta === 0) {
    setZoomLevel(100);
  } else {
    setZoomLevel((state.zoomLevel || 100) + delta);
  }
}
