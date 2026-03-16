// Keyboard, input, paste, clipboard handling
import { state, $ } from './state.js';
import {
  getSelectionInfo, getActiveElement, getCursorOffset,
  setCursorAtOffset, setCursorAtStart, isCursorAtStart, isCursorAtEnd,
  getEditableText,
} from './selection.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText, debouncedSync, markLayoutDirty } from './render.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo, recordUndoAction, renderUndoHistory } from './toolbar.js';
import { deleteSelectedImage, setupImages } from './images.js';
import { deleteSelectedShape, hasSelectedShape } from './shapes.js';
import { updatePageBreaks } from './pagination.js';
import { markDirty, saveVersion, updateDirtyIndicator, updateStatusBar, openAutosaveDB } from './file.js';
import { broadcastOp } from './collab.js';
import { setZoomLevel, getAutoCorrectMap, isAutoCorrectEnabled, exitFormatPainter, applyFormatPainter, enterHeaderFooterEditMode, exitHeaderFooterEditMode } from './toolbar-handlers.js';

export function initInput() {
  const page = $('pageContainer');

  // ─── Clear pending formats on editor blur ───
  page.addEventListener('blur', () => {
    state.pendingFormats = {};
  }, true);

  // ─── UXP-02: Double-click to enter header/footer edit mode ───
  page.addEventListener('dblclick', (e) => {
    const hfEl = e.target.closest('.page-header, .page-footer');
    if (!hfEl) return;
    // Already in edit mode — let normal double-click (word select) work
    if (hfEl.classList.contains('hf-editing')) return;
    e.preventDefault();
    e.stopPropagation();
    const pageEl = hfEl.closest('.doc-page');
    if (!pageEl) return;
    const kind = hfEl.dataset.hfKind || (hfEl.classList.contains('page-header') ? 'header' : 'footer');
    enterHeaderFooterEditMode(kind, pageEl);
  });

  // ─── UXP-02: Click outside header/footer exits edit mode ───
  document.addEventListener('mousedown', (e) => {
    if (!state.hfEditingMode) return;
    const hfEl = e.target.closest('.hf-editing');
    if (hfEl) return; // Click is inside the editing header/footer
    // Click on toolbar/modal elements should not exit
    if (e.target.closest('.hf-toolbar, .hf-close-btn, .modal-overlay, .modal')) return;
    exitHeaderFooterEditMode();
  });

  // ─── UXP-02: Escape key exits header/footer edit mode ───
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && state.hfEditingMode) {
      e.preventDefault();
      exitHeaderFooterEditMode();
    }
  }, true);

  // ─── E-01 fix: Capture cursor offset before text insertion for pending formats ───
  page.addEventListener('beforeinput', (e) => {
    if (state.ignoreInput) return;
    // Block browser-native drag-drop text insertion (we handle moves via WASM)
    if (e.inputType === 'insertFromDrop') {
      e.preventDefault();
      return;
    }
    // Prevent deletion into non-editable elements (page headers/footers)
    // UXP-02: Allow deletion when header/footer is in editing mode
    if (e.inputType && e.inputType.startsWith('delete') && e.getTargetRanges) {
      const ranges = e.getTargetRanges();
      for (const r of ranges) {
        // StaticRange has startContainer/endContainer, not commonAncestorContainer
        const container = r.startContainer;
        if (!container) continue;
        const parent = container.nodeType === 1 ? container : container.parentElement;
        if (parent && parent.closest?.('.page-header, .page-footer')) {
          // Allow deletion if the header/footer is in editing mode
          const hfEl = parent.closest('.page-header, .page-footer');
          if (hfEl && hfEl.classList.contains('hf-editing')) continue;
          e.preventDefault();
          return;
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
      const text = el ? getEditableText(el) : '';
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
        const text = getEditableText(el);
        // offset is in codepoints, so index into codepoint array
        const codepoints = [...text];
        const charBefore = offset >= 2 ? codepoints[offset - 2] : null;
        if (offset === 1 || (charBefore && /\s/.test(charBefore))) {
          openSlashMenu();
        }
      }
    }

    // ── E9.1: Auto-correct after space or punctuation ──
    if (e.inputType === 'insertText' && el && /^[\s.,;:!?\-)\]}>]$/.test(e.data || '')) {
      tryAutoCorrect(el, e.data);
    }
  });

  // ─── Copy — write both plain text and HTML to clipboard via WASM ───
  page.addEventListener('copy', e => {
    if (!state.doc) return;

    // Use select-all info if active, otherwise check native selection
    const info = state._selectAll ? state.lastSelInfo : getSelectionInfo();
    const sel = window.getSelection();
    const isSyntheticSelection = state._selectAll || (info && !info.collapsed && info.startNodeId !== info.endNodeId &&
      info.startEl?.closest?.('.page-content') !== info.endEl?.closest?.('.page-content'));
    if (!isSyntheticSelection && (!sel || sel.isCollapsed)) return;

    e.preventDefault();
    syncAllText();

    // E2.1: Generate clean semantic HTML from WASM model (no data attributes, no node IDs)
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

    // Get plain text: for cross-page/select-all selections, extract from HTML
    // since window.getSelection().toString() only covers one contentEditable
    let text = '';
    if (state._selectAll) {
      try { text = state.doc.to_plain_text(); } catch (_) { text = sel ? sel.toString() : ''; }
    } else if (isSyntheticSelection && html) {
      text = htmlToPlainText(html);
    } else {
      text = sel ? sel.toString() : '';
    }

    e.clipboardData.setData('text/plain', text);
    e.clipboardData.setData('text/html', html);
  });

  // ─── Clear select-all on click, and handle shift-click cross-page selection ───
  page.addEventListener('mousedown', (e) => {
    if (e.shiftKey && state.lastSelInfo && !state._selectAll) {
      // Shift-click: extend selection from current cursor to click target
      const clickTarget = e.target.closest('[data-node-id]');
      if (clickTarget) {
        const targetPageContent = clickTarget.closest('.page-content');
        const currentPageContent = state.lastSelInfo.startEl?.closest('.page-content');
        if (targetPageContent && currentPageContent && targetPageContent !== currentPageContent) {
          // Cross-page shift-click — create synthetic selection
          e.preventDefault();
          const targetNodeId = clickTarget.dataset.nodeId;
          const targetOffset = Array.from(getEditableText(clickTarget)).length; // click at end

          // Determine order: which comes first in the document?
          const allNodes = [];
          for (const pageEl of state.pageElements) {
            const content = pageEl.querySelector('.page-content');
            if (!content) continue;
            content.querySelectorAll(':scope > [data-node-id]').forEach(el => allNodes.push(el));
          }
          const startIdx = allNodes.findIndex(el => el.dataset.nodeId === state.lastSelInfo.startNodeId);
          const endIdx = allNodes.findIndex(el => el.dataset.nodeId === targetNodeId);

          if (startIdx >= 0 && endIdx >= 0) {
            const isForward = endIdx >= startIdx;
            const startNodeId = isForward ? state.lastSelInfo.startNodeId : targetNodeId;
            const startOffset = isForward ? state.lastSelInfo.startOffset : 0;
            const endNodeId = isForward ? targetNodeId : state.lastSelInfo.startNodeId;
            const endOffset = isForward ? targetOffset : state.lastSelInfo.startOffset;
            const startEl = allNodes[isForward ? startIdx : endIdx];
            const endEl = allNodes[isForward ? endIdx : startIdx];

            state.lastSelInfo = {
              startNodeId, startOffset, endNodeId, endOffset,
              collapsed: false, startEl, endEl,
            };

            // Clear previous highlights, then apply new ones
            page.querySelectorAll('.select-all-highlight').forEach(el => el.classList.remove('select-all-highlight'));
            for (const pageEl of state.pageElements) {
              const content = pageEl.querySelector('.page-content') || pageEl;
              let inRange = false;
              for (const el of content.children) {
                if (!el.dataset?.nodeId) continue;
                const nid = el.dataset.nodeId;
                if (nid === startNodeId) inRange = true;
                if (inRange) el.classList.add('select-all-highlight');
                if (nid === endNodeId) { inRange = false; break; }
              }
            }
          }
          return;
        }
      }
    }
    clearSelectAll();
  });

  // ─── UXP-14: Format Painter — apply on mouseup ──
  // When format painter mode is active, apply the copied format to whatever
  // text the user just selected via click-drag.
  document.addEventListener('mouseup', () => {
    if (!state.formatPainterMode) return;
    // Use a short delay to let the browser finalize the selection range
    setTimeout(() => {
      applyFormatPainter();
    }, 10);
  });

  // ─── Keydown ────────────────────────────────────
  page.addEventListener('keydown', e => {
    if (!state.doc) return;
    const doc = state.doc;

    // Clear select-all on any non-modifier key (except Cmd+X/C/A/Z/Y which use it)
    if (state._selectAll && !(e.ctrlKey || e.metaKey)) {
      // If the user types a printable character, delete the entire selection first
      // so typing replaces the selected content (like in any standard editor)
      if (e.key.length === 1 && !e.altKey) {
        e.preventDefault();
        const selectInfo = state.lastSelInfo;
        clearSelectAll();
        if (selectInfo && doc) {
          clearTimeout(state.syncTimer);
          syncAllText();
          try {
            doc.delete_selection(selectInfo.startNodeId, selectInfo.startOffset, selectInfo.endNodeId, selectInfo.endOffset);
            renderDocument();
            // Insert the typed character at the start of where the selection was
            const el = $('pageContainer')?.querySelector(`[data-node-id="${selectInfo.startNodeId}"]`);
            if (el) {
              doc.insert_text_in_paragraph(selectInfo.startNodeId, selectInfo.startOffset, e.key);
              broadcastOp({ action: 'insertText', nodeId: selectInfo.startNodeId, offset: selectInfo.startOffset, text: e.key });
              const updated = renderNodeById(selectInfo.startNodeId);
              if (updated) setCursorAtOffset(updated, selectInfo.startOffset + Array.from(e.key).length);
            } else {
              // Start node was deleted, find first available
              const first = $('pageContainer')?.querySelector('[data-node-id]');
              if (first) {
                doc.insert_text_in_paragraph(first.dataset.nodeId, 0, e.key);
                broadcastOp({ action: 'insertText', nodeId: first.dataset.nodeId, offset: 0, text: e.key });
                const updated = renderNodeById(first.dataset.nodeId);
                if (updated) setCursorAtOffset(updated, Array.from(e.key).length);
              } else {
                doc.append_paragraph(e.key);
                renderDocument();
              }
            }
            recordUndoAction('Replace selection');
            updateUndoRedo();
            markDirty();
          } catch (err) { console.error('select-all replace:', err); }
        }
        return;
      }
      // For non-printable keys (arrows, etc.), just clear the highlight
      clearSelectAll();
    }

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
            const text = activeEl ? getEditableText(activeEl) : '';
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

    // Delete selected shape(s) — E9.4
    if (hasSelectedShape() && (e.key === 'Delete' || e.key === 'Backspace')) {
      // Don't intercept if editing text inside a shape textbox
      if (!e.target.closest('.shape-textbox-edit')) {
        e.preventDefault(); deleteSelectedShape(); return;
      }
    }

    // ── Cross-page arrow key navigation ──
    if ((e.key === 'ArrowDown' || e.key === 'ArrowRight') && !e.ctrlKey && !e.metaKey) {
      const el = getActiveElement();
      if (el && isCursorAtEnd(el)) {
        let next = el.nextElementSibling;
        while (next && !next.dataset?.nodeId) next = next.nextElementSibling;
        if (!next) {
          // At end of page — jump to next page's first node
          const pageContent = el.closest('.page-content');
          const pageEl = pageContent?.closest('.doc-page');
          if (pageEl) {
            const pageIdx = state.pageElements.indexOf(pageEl);
            if (pageIdx >= 0 && pageIdx < state.pageElements.length - 1) {
              const nextPageContent = state.pageElements[pageIdx + 1]?.querySelector('.page-content');
              const firstNode = nextPageContent?.querySelector(':scope > [data-node-id]');
              if (firstNode) {
                e.preventDefault();
                nextPageContent.focus();
                setCursorAtStart(firstNode);
                return;
              }
            }
          }
        }
      }
    }
    if ((e.key === 'ArrowUp' || e.key === 'ArrowLeft') && !e.ctrlKey && !e.metaKey) {
      const el = getActiveElement();
      if (el && isCursorAtStart(el)) {
        let prev = el.previousElementSibling;
        while (prev && !prev.dataset?.nodeId) prev = prev.previousElementSibling;
        if (!prev) {
          // At start of page — jump to previous page's last node
          const pageContent = el.closest('.page-content');
          const pageEl = pageContent?.closest('.doc-page');
          if (pageEl) {
            const pageIdx = state.pageElements.indexOf(pageEl);
            if (pageIdx > 0) {
              const prevPageContent = state.pageElements[pageIdx - 1]?.querySelector('.page-content');
              const prevNodes = prevPageContent?.querySelectorAll(':scope > [data-node-id]');
              const lastNode = prevNodes?.length > 0 ? prevNodes[prevNodes.length - 1] : null;
              if (lastNode) {
                e.preventDefault();
                prevPageContent.focus();
                setCursorAtOffset(lastNode, Array.from(getEditableText(lastNode)).length);
                return;
              }
            }
          }
        }
      }
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
        case 'x': e.preventDefault(); doCut(); return;
        case 'c': /* handled by copy event above */ return;
        case 'v': /* handled by paste event */ return;
        case 'a': {
          e.preventDefault();
          selectAll();
          return;
        }
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

      // Ctrl+Alt+M — insert comment at current selection
      if (e.altKey && e.key.toLowerCase() === 'm') {
        e.preventDefault();
        const commentBtn = document.getElementById('miComment');
        if (commentBtn) commentBtn.click();
        return;
      }

      // Ctrl+Shift+E — insert equation
      if (e.shiftKey && e.key.toLowerCase() === 'e') {
        e.preventDefault();
        import('./toolbar-handlers.js').then(mod => {
          if (typeof mod.openEquationModal === 'function') mod.openEquationModal('');
        });
        return;
      }

      // Ctrl+Shift+O — toggle document outline panel
      if (e.shiftKey && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        import('./toolbar-handlers.js').then(mod => {
          if (typeof mod.toggleOutlinePanel === 'function') mod.toggleOutlinePanel();
        });
        return;
      }

      // Ctrl+Alt+0 — Normal style; Ctrl+Alt+1-6 — Heading 1-6
      if (e.altKey && e.key >= '0' && e.key <= '6') {
        e.preventDefault();
        const level = parseInt(e.key);
        const styleMap = ['normal', 'heading1', 'heading2', 'heading3', 'heading4', 'heading5', 'heading6'];
        import('./toolbar-handlers.js').then(mod => {
          if (typeof mod.applyParagraphStyle === 'function') {
            mod.applyParagraphStyle(styleMap[level]);
          }
        });
        return;
      }

      // Ctrl+Alt+F — insert footnote
      if (e.altKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        insertFootnoteAtCursor();
        return;
      }

      // Ctrl+Alt+D — insert endnote
      if (e.altKey && e.key.toLowerCase() === 'd') {
        e.preventDefault();
        insertEndnoteAtCursor();
        return;
      }
    }

    // ── Delete/Backspace with selection ──
    const deleteInfo = state._selectAll ? state.lastSelInfo : info;
    if ((e.key === 'Delete' || e.key === 'Backspace') && deleteInfo && !deleteInfo.collapsed) {
      e.preventDefault();
      clearSelectAll();
      clearTimeout(state.syncTimer);
      syncAllText();
      // Clear pending formats after deletion — new text should use document defaults
      state.pendingFormats = {};
      try {
        doc.delete_selection(deleteInfo.startNodeId, deleteInfo.startOffset, deleteInfo.endNodeId, deleteInfo.endOffset);
        renderDocument();
        // Try to place cursor at the start of the deletion point (search all pages)
        let el = null;
        for (const pg of (state.pageElements.length > 0 ? state.pageElements : [page])) {
          const content = pg.querySelector?.('.page-content') || pg;
          el = content.querySelector(`[data-node-id="${deleteInfo.startNodeId}"]`);
          if (el) break;
        }
        if (el) {
          const content = el.closest('.page-content');
          if (content) content.focus();
          setCursorAtOffset(el, deleteInfo.startOffset);
        } else {
          // The start node was deleted — find any remaining paragraph across all pages
          for (const pg of (state.pageElements.length > 0 ? state.pageElements : [page])) {
            const content = pg.querySelector?.('.page-content') || pg;
            el = content.querySelector('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id]');
            if (el) break;
          }
          if (el) {
            const content = el.closest('.page-content');
            if (content) content.focus();
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
        broadcastOp({ action: 'deleteSelection', startNode: deleteInfo.startNodeId, startOffset: deleteInfo.startOffset, endNode: deleteInfo.endNodeId, endOffset: deleteInfo.endOffset });
      } catch (err) { console.error('delete selection:', err); }
      return;
    }

    const el = getActiveElement();

    // ── Tab — list indent or table navigation ──
    if (e.key === 'Tab') {
      // List indent/outdent: Tab increases level, Shift+Tab decreases
      if (el && !el.closest?.('td, th')) {
        const isListItem = el.querySelector('.list-marker') !== null || !!el.dataset.listType;
        if (isListItem) {
          e.preventDefault();
          const nodeId = el.dataset.nodeId;
          const currentLevel = parseInt(el.dataset.listLevel || '0', 10);
          const listType = el.dataset.listType || 'bullet';
          const newLevel = e.shiftKey ? Math.max(0, currentLevel - 1) : Math.min(8, currentLevel + 1);
          try {
            doc.set_list_format(nodeId, listType, newLevel);
            broadcastOp({ action: 'setListFormat', nodeId, format: listType, level: newLevel });
            const updated = renderNodeById(nodeId);
            if (updated) {
              const content = updated.closest('.page-content');
              if (content) content.focus();
              setCursorAtStart(updated);
            }
            recordUndoAction(e.shiftKey ? 'Outdent list item' : 'Indent list item');
            state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
          } catch (err) { console.error('list indent:', err); }
          return;
        }
      }
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

    // ── Enter — split paragraph (with list handling) ──
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      state._typingBatch = null; // E3.1: End typing session on Enter
      if (!el) return;
      const nodeId = el.dataset.nodeId;
      const offset = getCursorOffset(el);
      clearTimeout(state.syncTimer); syncParagraphText(el);

      // Check if this is a list item (has a list marker element or data attribute)
      const isListItem = el.querySelector('.list-marker') !== null || !!el.dataset.listType;
      // For empty check, exclude the list marker span text
      const contentText = getEditableText(el);
      const isEmpty = contentText.replace(/[\u200B\s]/g, '').length === 0;

      // Enter on empty list item → exit the list (remove list formatting)
      if (isListItem && isEmpty) {
        try {
          doc.set_list_format(nodeId, 'none', 0);
          broadcastOp({ action: 'setListFormat', nodeId, format: 'none', level: 0 });
          const updated = renderNodeById(nodeId);
          if (updated) {
            const content = updated.closest('.page-content');
            if (content) content.focus();
            setCursorAtStart(updated);
          }
          recordUndoAction('Exit list');
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
        } catch (err) { console.error('exit list:', err); }
        return;
      }

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
          // Register new element in nodeIdToElement map
          state.nodeIdToElement.set(newId, newEl);
          newEl.querySelectorAll('[data-node-id]').forEach(child => {
            state.nodeIdToElement.set(child.dataset.nodeId, child);
          });
          setupImages(newEl);
          // Ensure the contenteditable parent has focus before setting cursor
          const content = newEl.closest('.page-content');
          if (content) content.focus();
          setCursorAtStart(newEl);
        }
        recordUndoAction('Split paragraph');
        state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
        broadcastOp({ action: 'splitParagraph', nodeId, offset });
      } catch (err) { console.error('split:', err); }
      return;
    }

    // Prevent deletion when cursor is on non-editable elements
    if ((e.key === 'Delete' || e.key === 'Backspace') && !el) {
      const sel = window.getSelection();
      if (sel && sel.anchorNode) {
        const anchor = sel.anchorNode.nodeType === 1 ? sel.anchorNode : sel.anchorNode.parentElement;
        if (anchor && anchor.closest?.('.page-header, .page-footer')) {
          e.preventDefault();
          return;
        }
      }
    }

    // ── Backspace at start of list item — remove list formatting first ──
    if (e.key === 'Backspace' && el && isCursorAtStart(el)) {
      const isListItem = el.querySelector('.list-marker') !== null || el.dataset.listType;
      if (isListItem) {
        e.preventDefault();
        const nodeId = el.dataset.nodeId;
        try {
          doc.set_list_format(nodeId, 'none', 0);
          broadcastOp({ action: 'setListFormat', nodeId, format: 'none', level: 0 });
          const updated = renderNodeById(nodeId);
          if (updated) {
            const content = updated.closest('.page-content');
            if (content) content.focus();
            setCursorAtStart(updated);
          }
          recordUndoAction('Remove list formatting');
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
        } catch (err) { console.error('remove list:', err); }
        return;
      }

      let prev = el.previousElementSibling;
      // Skip non-model elements
      while (prev && !prev.dataset?.nodeId) prev = prev.previousElementSibling;

      // Cross-page: if no prev sibling within this page, look at previous page's last paragraph
      if (!prev) {
        const pageContent = el.closest('.page-content');
        const pageEl = pageContent?.closest('.doc-page');
        if (pageEl) {
          const pageIdx = state.pageElements.indexOf(pageEl);
          if (pageIdx > 0) {
            const prevPageContent = state.pageElements[pageIdx - 1]?.querySelector('.page-content');
            if (prevPageContent) {
              const prevPageNodes = prevPageContent.querySelectorAll(':scope > [data-node-id]');
              prev = prevPageNodes.length > 0 ? prevPageNodes[prevPageNodes.length - 1] : null;
            }
          }
        }
      }

      if (prev?.dataset?.nodeId) {
        e.preventDefault();
        clearTimeout(state.syncTimer); syncParagraphText(el); syncParagraphText(prev);
        const cursorPos = Array.from(getEditableText(prev)).length;
        const nodeId1 = prev.dataset.nodeId;
        const nodeId2 = el.dataset.nodeId;
        try {
          doc.merge_paragraphs(nodeId1, nodeId2);
          const updated = renderNodeById(nodeId1);
          // Only remove source element after confirming render succeeded
          if (updated) {
            el.remove();
            setCursorAtOffset(updated, cursorPos);
          }
          recordUndoAction('Merge paragraphs');
          state.pagesRendered = false; updatePageBreaks(); updateUndoRedo(); markDirty();
          broadcastOp({ action: 'mergeParagraphs', nodeId1, nodeId2 });
        } catch (err) { console.error('merge:', err); }
      }
      return;
    }

    // ── Delete at end — merge next (including cross-page) ──
    if (e.key === 'Delete' && el && isCursorAtEnd(el)) {
      let next = el.nextElementSibling;
      // Skip non-model elements
      while (next && !next.dataset?.nodeId) next = next.nextElementSibling;

      // Cross-page: if no next sibling within this page, look at next page's first paragraph
      if (!next) {
        const pageContent = el.closest('.page-content');
        const pageEl = pageContent?.closest('.doc-page');
        if (pageEl) {
          const pageIdx = state.pageElements.indexOf(pageEl);
          if (pageIdx >= 0 && pageIdx < state.pageElements.length - 1) {
            const nextPageContent = state.pageElements[pageIdx + 1]?.querySelector('.page-content');
            if (nextPageContent) {
              next = nextPageContent.querySelector(':scope > [data-node-id]');
            }
          }
        }
      }

      if (next?.dataset?.nodeId) {
        e.preventDefault();
        clearTimeout(state.syncTimer); syncParagraphText(el); syncParagraphText(next);
        const cursorPos = Array.from(getEditableText(el)).length;
        const nodeId1 = el.dataset.nodeId;
        const nodeId2 = next.dataset.nodeId;
        try {
          doc.merge_paragraphs(nodeId1, nodeId2);
          const updated = renderNodeById(nodeId1);
          // Only remove source element after confirming render succeeded
          if (updated) {
            next.remove();
            setCursorAtOffset(updated, cursorPos);
          }
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
      const origStartNodeId = info.startNodeId;
      const origStartOffset = info.startOffset;
      try {
        doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
        renderDocument();
      } catch (_) {}
      // After delete + re-render, DOM is rebuilt. Clear stale selection info.
      state.lastSelInfo = null;
      // Try to restore cursor at the original start position
      const restoredEl = page.querySelector(`[data-node-id="${origStartNodeId}"]`);
      if (restoredEl) {
        info = { startNodeId: origStartNodeId, startOffset: origStartOffset, startEl: restoredEl };
      } else {
        // Start node was deleted — find first available paragraph
        info = null;
      }
    } else if (info) {
      // Sync text before paste to ensure WASM model matches DOM
      if (info.startEl && info.startEl.isConnected) {
        syncParagraphText(info.startEl);
      }
    }

    // Ensure we have a valid target paragraph
    const ensureTarget = () => {
      // First try WASM paragraph list (authoritative)
      try {
        const allIds = JSON.parse(doc.paragraph_ids_json());
        if (allIds.length > 0) {
          const firstId = allIds[0];
          const el = page.querySelector(`[data-node-id="${firstId}"]`);
          if (el) {
            return { startNodeId: firstId, startOffset: 0, startEl: el };
          }
          // Element not in DOM yet — return WASM-based info anyway
          return { startNodeId: firstId, startOffset: 0, startEl: null };
        }
      } catch (_) {}

      // DOM fallback: search across all pages
      let firstEl = page.querySelector('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]');
      if (!firstEl) {
        // Document is completely empty — create a paragraph
        // NOTE: Don't renderDocument() here — the main paste handler will do it
        try {
          doc.append_paragraph('');
          broadcastOp({ action: 'insertParagraph', afterNodeId: null, text: '' });
        } catch (_) {}
        // Re-check WASM model for the newly created paragraph
        try {
          const ids = JSON.parse(doc.paragraph_ids_json());
          if (ids.length > 0) {
            return { startNodeId: ids[0], startOffset: 0, startEl: null };
          }
        } catch (_) {}
        firstEl = page.querySelector('[data-node-id]');
      }
      if (firstEl) {
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

    // E2.2: Try rich paste (HTML → structured content via WASM)
    if (html) {
      const parsed = parseClipboardHtml(html);
      if (parsed && parsed.elements.length > 0) {
        const ok = pasteStructuredContent(doc, info, parsed, page);
        if (ok) {
          renderDocument();
          placeCursorAfterPaste(page, text || '', info.startNodeId, info.startOffset);
          recordUndoAction('Paste formatted content');
          updateUndoRedo();
          markDirty();
          return;
        }
        // Rich paste failed entirely — fall through to plain text
        console.warn('Rich paste returned no content, falling back to plain text');
      }
    }

    if (!text) return;

    if (text.includes('\n')) {
      try {
        doc.paste_plain_text(info.startNodeId, info.startOffset, text);
        broadcastOp({ action: 'pasteText', nodeId: info.startNodeId, offset: info.startOffset, text });
        renderDocument();
        placeCursorAfterPaste(page, text, info.startNodeId, info.startOffset);
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
        renderDocument();
        const page = $('pageContainer');
        const updated = page?.querySelector(`[data-node-id="${info.startNodeId}"]`);
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
        const maxLen = Array.from(getEditableText(newDrop)).length;
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
    // UXP-14: Exit format painter mode
    if (state.formatPainterMode) {
      exitFormatPainter();
      return;
    }
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
    // Close equation modal
    if ($('equationModal')?.classList.contains('show')) {
      $('equationModal').classList.remove('show');
      return;
    }
    // Close custom dictionary modal
    if ($('dictModal')?.classList.contains('show')) {
      $('dictModal').classList.remove('show');
      return;
    }
    // Close auto-correct modal
    if ($('autoCorrectModal')?.classList.contains('show')) {
      $('autoCorrectModal').classList.remove('show');
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

  // ─── Right-Click Context Menu ────────────────────
  page.addEventListener('contextmenu', e => {
    // Only intercept in editor view on content areas
    if (state.currentView !== 'editor') return;
    const target = e.target.closest('.page-content');
    if (!target) return;
    e.preventDefault();

    // Remove any existing context menu
    const existing = document.querySelector('.context-menu');
    if (existing) existing.remove();

    const sel = window.getSelection();
    const info = getSelectionInfo();
    const hasSelection = (sel && !sel.isCollapsed) || (info && !info.collapsed);

    // Check if right-clicked on a table cell
    const cell = e.target.closest('td, th');
    const table = cell ? cell.closest('table') : null;

    // Check if right-clicked on an image (img may be inside a paragraph with data-node-id)
    const img = e.target.closest('img');

    const menu = document.createElement('div');
    menu.className = 'context-menu';
    menu.setAttribute('role', 'menu');

    const addItem = (label, shortcut, action, disabled) => {
      const item = document.createElement('button');
      item.className = 'ctx-item';
      item.setAttribute('role', 'menuitem');
      if (disabled) item.disabled = true;
      item.innerHTML = `<span>${label}</span>${shortcut ? `<span class="ctx-shortcut">${shortcut}</span>` : ''}`;
      item.addEventListener('click', () => { menu.remove(); action(); });
      menu.appendChild(item);
    };
    const addSep = () => {
      const sep = document.createElement('div');
      sep.className = 'ctx-sep';
      menu.appendChild(sep);
    };

    // Standard edit operations — use WASM-backed operations (not deprecated execCommand)
    addItem('Cut', '\u2318X', () => {
      doCut();
    }, !hasSelection);
    addItem('Copy', '\u2318C', () => {
      // Trigger the programmatic copy path — only copy selected text, not full document
      if (info && state.doc) {
        try {
          syncAllText();
          let html = '';
          try {
            html = state.doc.export_selection_html(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
          } catch (_) {}
          // Get plain text: extract from HTML for cross-page, else from selection
          const isCrossPage = info.startEl?.closest?.('.page-content') !== info.endEl?.closest?.('.page-content');
          const selText = (isCrossPage && html) ? htmlToPlainText(html) : (window.getSelection()?.toString() || '');
          if (html) {
            const htmlBlob = new Blob([html], { type: 'text/html' });
            const textBlob = new Blob([selText], { type: 'text/plain' });
            navigator.clipboard.write([new ClipboardItem({ 'text/html': htmlBlob, 'text/plain': textBlob })]).catch(() => {
              navigator.clipboard.writeText(selText).catch(() => {});
            });
          } else {
            navigator.clipboard.writeText(selText).catch(() => {});
          }
        } catch (_) {}
      }
    }, !hasSelection);
    addItem('Paste', '\u2318V', async () => {
      try {
        const items = await navigator.clipboard.read();
        for (const item of items) {
          if (item.types.includes('text/html')) {
            const blob = await item.getType('text/html');
            const html = await blob.text();
            const parsed = parseClipboardHtml(html);
            if (parsed && parsed.elements.length > 0) {
              const freshInfo = getSelectionInfo();
              if (freshInfo && state.doc) {
                pasteStructuredContent(state.doc, freshInfo, parsed, page);
                renderDocument();
                recordUndoAction('Paste');
                updateUndoRedo();
                markDirty();
              }
              return;
            }
          }
          if (item.types.includes('text/plain')) {
            const blob = await item.getType('text/plain');
            const text = await blob.text();
            const freshInfo = getSelectionInfo();
            if (text && freshInfo && state.doc) {
              state.doc.paste_plain_text(freshInfo.startNodeId, freshInfo.startOffset, text);
              renderDocument();
              recordUndoAction('Paste');
              updateUndoRedo();
              markDirty();
            }
            return;
          }
        }
      } catch (_) {
        // Fallback: read plain text
        try {
          const text = await navigator.clipboard.readText();
          const freshInfo = getSelectionInfo();
          if (text && freshInfo && state.doc) {
            state.doc.paste_plain_text(freshInfo.startNodeId, freshInfo.startOffset, text);
            renderDocument();
            recordUndoAction('Paste');
            updateUndoRedo();
            markDirty();
          }
        } catch (_2) {}
      }
    });

    addSep();

    // Formatting shortcuts
    if (hasSelection) {
      addItem('Bold', '\u2318B', () => toggleFormat('bold'));
      addItem('Italic', '\u2318I', () => toggleFormat('italic'));
      addItem('Underline', '\u2318U', () => toggleFormat('underline'));
      addSep();
    }

    // Table operations
    if (table && table.dataset.nodeId) {
      const tableId = table.dataset.nodeId;
      addItem('Insert row above', '', () => {
        const rowIdx = cell ? Array.from(cell.closest('tr').parentNode.children).indexOf(cell.closest('tr')) : 0;
        try { state.doc.insert_table_row(tableId, rowIdx); broadcastOp({ action: 'insertTableRow', tableNodeId: tableId, rowIndex: rowIdx }); renderDocument(); recordUndoAction('Insert row'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
      });
      addItem('Insert row below', '', () => {
        const rowIdx = cell ? Array.from(cell.closest('tr').parentNode.children).indexOf(cell.closest('tr')) + 1 : 1;
        try { state.doc.insert_table_row(tableId, rowIdx); broadcastOp({ action: 'insertTableRow', tableNodeId: tableId, rowIndex: rowIdx }); renderDocument(); recordUndoAction('Insert row'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
      });
      addItem('Delete row', '', () => {
        const rowIdx = cell ? Array.from(cell.closest('tr').parentNode.children).indexOf(cell.closest('tr')) : 0;
        try { state.doc.delete_table_row(tableId, rowIdx); broadcastOp({ action: 'deleteTableRow', tableNodeId: tableId, rowIndex: rowIdx }); renderDocument(); recordUndoAction('Delete row'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
      });
      addSep();
      addItem('Insert column left', '', () => {
        const colIdx = cell ? Array.from(cell.parentNode.children).indexOf(cell) : 0;
        try { state.doc.insert_table_column(tableId, colIdx); broadcastOp({ action: 'insertTableColumn', tableNodeId: tableId, colIndex: colIdx }); renderDocument(); recordUndoAction('Insert column'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
      });
      addItem('Insert column right', '', () => {
        const colIdx = cell ? Array.from(cell.parentNode.children).indexOf(cell) + 1 : 1;
        try { state.doc.insert_table_column(tableId, colIdx); broadcastOp({ action: 'insertTableColumn', tableNodeId: tableId, colIndex: colIdx }); renderDocument(); recordUndoAction('Insert column'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
      });
      addItem('Delete column', '', () => {
        const colIdx = cell ? Array.from(cell.parentNode.children).indexOf(cell) : 0;
        try { state.doc.delete_table_column(tableId, colIdx); broadcastOp({ action: 'deleteTableColumn', tableNodeId: tableId, colIndex: colIdx }); renderDocument(); recordUndoAction('Delete column'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
      });
      addSep();
    }

    // Image operations
    if (img) {
      // Image node ID could be on the img itself or on its parent paragraph
      const imgNodeEl = img.closest('[data-node-id]');
      const imgNodeId = imgNodeEl?.dataset?.nodeId;
      if (imgNodeId) {
        addItem('Delete image', '', () => {
          try { state.doc.delete_image(imgNodeId); broadcastOp({ action: 'deleteNode', nodeId: imgNodeId }); renderDocument(); recordUndoAction('Delete image'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
        });
        addItem('Set alt text', '', () => {
          const alt = prompt('Alt text:', img.alt || '');
          if (alt !== null) {
            try { state.doc.set_image_alt_text(imgNodeId, alt); broadcastOp({ action: 'setImageAltText', nodeId: imgNodeId, alt }); recordUndoAction('Set alt text'); updateUndoRedo(); markDirty(); } catch (e) { console.error(e); }
          }
        });
        addSep();
      }
    }

    // Select All
    addItem('Select All', '\u2318A', () => selectAll());

    // Position menu
    let left = e.clientX;
    let top = e.clientY;
    document.body.appendChild(menu);
    const rect = menu.getBoundingClientRect();
    if (left + rect.width > window.innerWidth - 8) left = window.innerWidth - rect.width - 8;
    if (top + rect.height > window.innerHeight - 8) top = window.innerHeight - rect.height - 8;
    menu.style.left = left + 'px';
    menu.style.top = top + 'px';

    // Close on click outside or Escape
    const closeMenu = (ev) => {
      if (!menu.contains(ev.target)) {
        menu.remove();
        document.removeEventListener('mousedown', closeMenu);
        document.removeEventListener('keydown', escClose);
      }
    };
    const escClose = (ev) => {
      if (ev.key === 'Escape') {
        menu.remove();
        document.removeEventListener('mousedown', closeMenu);
        document.removeEventListener('keydown', escClose);
      }
    };
    setTimeout(() => {
      document.addEventListener('mousedown', closeMenu);
      document.addEventListener('keydown', escClose);
    }, 0);
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

/** Extract plain text from HTML (for cross-page copy where sel.toString() fails) */
function htmlToPlainText(html) {
  const div = document.createElement('div');
  div.innerHTML = html;
  // Replace block-level elements with newlines
  div.querySelectorAll('p, h1, h2, h3, h4, h5, h6, div, tr, li').forEach(el => {
    el.insertAdjacentText('afterend', '\n');
  });
  div.querySelectorAll('br').forEach(el => {
    el.replaceWith('\n');
  });
  div.querySelectorAll('td, th').forEach((el, i) => {
    if (el.nextElementSibling) el.insertAdjacentText('afterend', '\t');
  });
  return div.textContent?.replace(/\n{3,}/g, '\n\n').trim() || '';
}

// insertTextAtCursor removed — all text insertion must go through WASM to maintain model consistency

// ─── E9.1: Auto-Correct ────────────────────────────
// After space/punctuation, check if the previous word matches an auto-correct rule.
// If so, replace the word via WASM and re-render the node.
function tryAutoCorrect(el, triggerChar) {
  if (!isAutoCorrectEnabled()) return;
  if (!state.doc || !el?.dataset?.nodeId) return;

  const text = getEditableText(el);
  const cursorOff = getCursorOffset(el);
  // cursorOff includes the trigger char, so the word ends at cursorOff - 1
  const codepoints = [...text];
  const endIdx = cursorOff - 1; // index of trigger char
  if (endIdx < 1) return;

  // Walk backward to find the start of the previous word
  let startIdx = endIdx - 1;
  while (startIdx >= 0 && /\S/.test(codepoints[startIdx])) startIdx--;
  startIdx++; // startIdx is now the first char of the word

  if (startIdx >= endIdx) return;
  const word = codepoints.slice(startIdx, endIdx).join('').toLowerCase();
  if (!word) return;

  const acMap = getAutoCorrectMap();
  const replacement = acMap[word];
  if (!replacement) return;

  const nodeId = el.dataset.nodeId;

  // Sync the current paragraph text to WASM first
  clearTimeout(state.syncTimer);
  syncParagraphText(el);

  try {
    // Delete the misspelled word (startIdx to endIdx), then insert the correction
    // Using WASM for model consistency
    if (typeof state.doc.replace_text_range === 'function') {
      state.doc.replace_text_range(nodeId, startIdx, endIdx, replacement);
    } else {
      // Fallback: reconstruct the paragraph text
      const fullText = getEditableText(el);
      const cp = [...fullText];
      const newCp = [...cp.slice(0, startIdx), ...replacement, ...cp.slice(endIdx)];
      state.doc.set_paragraph_text(nodeId, newCp.join(''));
    }
    // Calculate new cursor offset: startIdx + replacement length + 1 (for the trigger char)
    const newCursorOff = startIdx + [...replacement].length + 1;
    const updated = renderNodeById(nodeId);
    if (updated) setCursorAtOffset(updated, newCursorOff);
    state.syncedTextCache.set(nodeId, getEditableText(updated || el));
  } catch (e) {
    console.warn('auto-correct failed:', e);
  }
}

// ─── E2.2: Parse clipboard HTML into structured content for WASM ─────
// Returns { elements: [ {type:'paragraph', runs:[...], ...}, {type:'image', src, width, height}, {type:'table', rows:[...]} ] }
function parseClipboardHtml(html) {
  try {
    // Strip MS Office conditional comments (<!--[if ...]>...<![endif]-->)
    // and XML processing instructions that can confuse parsing
    let cleaned = html
      .replace(/<!--\[if[\s\S]*?<!\[endif\]-->/gi, '')
      .replace(/<!\[if[\s\S]*?<!\[endif\]>/gi, '')
      .replace(/<\?xml[\s\S]*?\?>/gi, '')
      .replace(/<o:p>[\s\S]*?<\/o:p>/gi, '')
      // Remove Word's <w:Sdt> and other Office XML tags that DOMParser may mangle
      .replace(/<\/?w:[^>]+>/gi, '')
      // Remove VML namespace declarations that break DOMParser
      .replace(/<\/?v:[^>]+>/gi, '');

    const parser = new DOMParser();
    const doc = parser.parseFromString(cleaned, 'text/html');
    const body = doc.body;
    if (!body || !body.childNodes.length) return null;

    // Google Docs wraps pasted content in a <b> with id="docs-internal-guid-..."
    // Unwrap it so the actual paragraphs inside get processed correctly
    const gdocsWrapper = body.querySelector('b[id^="docs-internal-guid-"]');
    const walkRoot = gdocsWrapper || body;

    // Remove style tags (MS Word injects <style> blocks into clipboard HTML)
    walkRoot.querySelectorAll('style').forEach(s => s.remove());

    const elements = [];

    // Google Docs: if the wrapper contains block elements (<p>, <h*>), walk normally.
    // If it only has inline children (<span>, text, etc.), treat the whole thing as one paragraph.
    // Also check for <br> separators — Google Docs often uses <br> between spans for multi-paragraph content.
    const hasBlockChildren = walkRoot.querySelector('p, h1, h2, h3, h4, h5, h6, div, table, ul, ol, li, blockquote, pre, hr');
    if (gdocsWrapper && !hasBlockChildren) {
      // Check if there are <br> tags indicating multiple paragraphs
      const hasBrSeparators = walkRoot.querySelector('br');
      if (hasBrSeparators) {
        // Split content at <br> boundaries into separate paragraphs
        let currentRuns = [];
        for (const child of walkRoot.childNodes) {
          if (child.nodeType === 1 && child.tagName.toLowerCase() === 'br') {
            // Flush current runs as a paragraph
            if (currentRuns.length > 0) {
              const para = { type: 'paragraph', runs: currentRuns };
              extractParagraphFormat(walkRoot, para);
              elements.push(para);
              currentRuns = [];
            }
            continue;
          }
          // Extract runs from this child node
          const childRuns = (child.nodeType === 3)
            ? (child.textContent ? [{ text: child.textContent }] : [])
            : extractRunsFromElement(child);
          currentRuns.push(...childRuns);
        }
        // Flush remaining runs
        if (currentRuns.length > 0) {
          const para = { type: 'paragraph', runs: currentRuns };
          extractParagraphFormat(walkRoot, para);
          elements.push(para);
        }
      } else {
        // All-inline Google Docs content: extract as a single paragraph with formatted runs
        const runs = extractRunsFromElement(walkRoot);
        if (runs.length > 0) {
          const para = { type: 'paragraph', runs };
          extractParagraphFormat(walkRoot, para);
          elements.push(para);
        }
      }
    } else {
      // Walk all top-level children
      walkBlockElements(walkRoot, elements);
    }

    if (elements.length === 0) {
      // Fallback: treat body as a single paragraph with formatted inline content
      const runs = extractRunsFromElement(walkRoot);
      if (runs.length > 0) {
        return { elements: [{ type: 'paragraph', runs }] };
      }
      // Last resort: extract all text as plain
      const text = body.textContent || '';
      if (text) return { elements: [{ type: 'paragraph', runs: [{ text }] }] };
      return null;
    }

    // Clean up internal _fromInline marker
    for (const el of elements) {
      delete el._fromInline;
    }

    return { elements };
  } catch (_) {
    return null;
  }
}

/** Walk block-level children and produce structured elements */
function walkBlockElements(container, elements) {
  for (const child of container.childNodes) {
    if (child.nodeType === 3) {
      // Text node at top level — merge into previous inline paragraph if applicable
      const text = child.textContent;
      if (text && text.trim()) {
        const prev = elements[elements.length - 1];
        if (prev && prev.type === 'paragraph' && prev._fromInline) {
          prev.runs.push({ text });
        } else {
          elements.push({ type: 'paragraph', runs: [{ text }], _fromInline: true });
        }
      }
      continue;
    }
    if (child.nodeType !== 1) continue;
    const tag = child.tagName.toLowerCase();

    // Skip MS Office namespace elements, style/meta/link tags
    if (tag.includes(':') || tag === 'style' || tag === 'meta' || tag === 'link' || tag === 'script') continue;

    // Inline-only elements at block level (span, b, i, a, etc.)
    // Merge consecutive inline elements into a single paragraph instead
    // of creating one paragraph per inline element.
    if (/^(span|b|strong|i|em|u|a|font|mark|small|big|code|kbd|abbr|cite|q|var|samp)$/.test(tag)) {
      const runs = extractRunsFromElement(child);
      if (runs.length > 0) {
        // Merge with the previous paragraph if it was also from inline content
        const prev = elements[elements.length - 1];
        if (prev && prev.type === 'paragraph' && prev._fromInline) {
          prev.runs.push(...runs);
        } else {
          const para = { type: 'paragraph', runs, _fromInline: true };
          elements.push(para);
        }
      }
      continue;
    }

    // Images
    if (tag === 'img') {
      const imgEl = extractImageElement(child);
      if (imgEl) elements.push(imgEl);
      continue;
    }

    // Tables
    if (tag === 'table') {
      const tbl = extractTableElement(child);
      if (tbl) elements.push(tbl);
      continue;
    }

    // Block elements that contain paragraphs (divs, sections)
    if (tag === 'div' || tag === 'section' || tag === 'article' || tag === 'main') {
      // Check if it contains block children or is just a wrapper for inline content
      const hasBlocks = child.querySelector('p, h1, h2, h3, h4, h5, h6, div, table, img, ul, ol, li');
      if (hasBlocks) {
        walkBlockElements(child, elements);
      } else {
        const runs = extractRunsFromElement(child);
        if (runs.length > 0) {
          const para = { type: 'paragraph', runs };
          extractParagraphFormat(child, para);
          elements.push(para);
        }
      }
      continue;
    }

    // Lists
    if (tag === 'ul' || tag === 'ol') {
      const listType = tag === 'ul' ? 'bullet' : 'decimal';
      const items = child.querySelectorAll(':scope > li');
      items.forEach((li, idx) => {
        // Check for images inside list items
        const imgs = li.querySelectorAll('img');
        // Check for nested lists inside the li
        const nestedLists = li.querySelectorAll(':scope > ul, :scope > ol');
        const runs = extractRunsFromElement(li);
        if (runs.length > 0) {
          const para = { type: 'paragraph', runs, listType, listLevel: 0 };
          extractParagraphFormat(li, para);
          elements.push(para);
        }
        imgs.forEach(img => {
          const imgEl = extractImageElement(img);
          if (imgEl) elements.push(imgEl);
        });
        // Process nested lists recursively with actual depth detection
        nestedLists.forEach(nested => {
          const processNestedList = (listEl) => {
            const lType = listEl.tagName.toLowerCase() === 'ul' ? 'bullet' : 'decimal';
            const lis = listEl.querySelectorAll(':scope > li');
            lis.forEach(nli => {
              // Walk up from this <li> counting ancestor <ul>/<ol> elements
              // to determine the actual nesting depth (0-based)
              let depth = 0;
              let ancestor = nli.parentElement;
              while (ancestor && ancestor !== child) {
                if (ancestor.tagName && /^(ul|ol)$/i.test(ancestor.tagName)) {
                  depth++;
                }
                ancestor = ancestor.parentElement;
              }
              const nRuns = extractRunsFromElement(nli);
              if (nRuns.length > 0) {
                elements.push({ type: 'paragraph', runs: nRuns, listType: lType, listLevel: depth });
              }
              // Recurse into nested lists inside this <li>
              const subLists = nli.querySelectorAll(':scope > ul, :scope > ol');
              subLists.forEach(sub => processNestedList(sub));
            });
          };
          processNestedList(nested);
        });
      });
      continue;
    }

    // Paragraphs and headings
    if (/^(p|h[1-6])$/.test(tag)) {
      // Check for images inside the paragraph
      const imgs = child.querySelectorAll('img');
      const runs = extractRunsFromElement(child);
      if (runs.length > 0) {
        const para = { type: 'paragraph', runs };
        extractParagraphFormat(child, para);

        // MS Word list paragraph detection (class="MsoListParagraph" or mso-list style)
        const cls = typeof child.className === 'string' ? child.className : '';
        const cssText = child.style?.cssText || '';
        if (cls.includes('MsoListParagraph') || cssText.includes('mso-list')) {
          // Word list markers are typically the first run's leading text
          const firstText = runs[0]?.text || '';
          const bulletMatch = firstText.match(/^[\u00B7\u2022\u25CF\u25CB\uF0B7·•]\s*/);
          const numMatch = firstText.match(/^(\d+[.)]\s*|[a-zA-Z][.)]\s*)/);
          if (bulletMatch) {
            runs[0].text = firstText.slice(bulletMatch[0].length);
            para.listType = 'bullet';
          } else if (numMatch) {
            runs[0].text = firstText.slice(numMatch[0].length);
            para.listType = 'decimal';
          }
          // Extract indent level from mso-list style
          const levelMatch = cssText.match(/level(\d+)/);
          if (levelMatch) para.listLevel = parseInt(levelMatch[1]) - 1;
          // Clean up empty first run after marker removal
          if (runs[0] && !runs[0].text) runs.shift();
        }

        // s1engine data-list-type/data-list-level attributes (from our own copy/cut)
        if (!para.listType && child.dataset?.listType) {
          para.listType = child.dataset.listType;
          para.listLevel = parseInt(child.dataset.listLevel || '0');
        }

        // MS Word heading detection (class="MsoTitle", "MsoHeading1", etc.)
        if (tag === 'p') {
          const hClassMatch = cls.match(/MsoHeading(\d)/i);
          if (hClassMatch) para.headingLevel = parseInt(hClassMatch[1]);
          if (cls.includes('MsoTitle')) para.headingLevel = 1;
          if (cls.includes('MsoSubtitle')) para.headingLevel = 2;
        }

        if (runs.length > 0) elements.push(para);
      }
      // Add any images found inside the paragraph as separate elements
      imgs.forEach(img => {
        const imgEl = extractImageElement(img);
        if (imgEl) elements.push(imgEl);
      });
      continue;
    }

    // Horizontal rules
    if (tag === 'hr') {
      elements.push({ type: 'hr' });
      continue;
    }

    // Fallback: treat as paragraph
    const runs = extractRunsFromElement(child);
    if (runs.length > 0) {
      const para = { type: 'paragraph', runs };
      extractParagraphFormat(child, para);
      elements.push(para);
    }
  }
}

/** Extract image data from an <img> element */
function extractImageElement(img) {
  // Prefer getAttribute to avoid URL resolution by DOMParser
  const src = img.getAttribute('src') || img.src;
  if (!src) return null;
  // Only accept data URLs and blob URLs (file:/// and http:// won't work in paste)
  if (!src.startsWith('data:') && !src.startsWith('blob:')) return null;
  const style = img.style || {};
  // Parse width/height from inline styles, attributes, or defaults
  let width = parseFloat(style.width) || parseFloat(img.getAttribute('width')) || img.naturalWidth || img.width || 200;
  let height = parseFloat(style.height) || parseFloat(img.getAttribute('height')) || img.naturalHeight || img.height || 200;
  // Convert px to pt if needed
  if (style.width && style.width.endsWith('px')) width = width * 0.75;
  else if (style.width && style.width.endsWith('pt')) { /* already pt */ }
  if (style.height && style.height.endsWith('px')) height = height * 0.75;
  else if (style.height && style.height.endsWith('pt')) { /* already pt */ }
  const alt = img.alt || '';
  return { type: 'image', src, width, height, alt };
}

/** Extract table structure from a <table> element */
function extractTableElement(tableEl) {
  const rows = [];
  const trs = tableEl.querySelectorAll('tr');
  for (const tr of trs) {
    const cells = [];
    for (const td of tr.querySelectorAll('td, th')) {
      const runs = extractRunsFromElement(td);
      const text = runs.map(r => r.text).join('');
      cells.push({ text: text || '', runs });
    }
    if (cells.length > 0) rows.push(cells);
  }
  if (rows.length === 0) return null;
  return { type: 'table', rows };
}

/** Paste structured content (paragraphs, images, tables) into the document.
 *  Returns true if any content was successfully pasted, false otherwise. */
function pasteStructuredContent(doc, info, parsed, page) {
  const elements = parsed.elements;
  if (!elements || elements.length === 0) return false;

  let lastNodeId = info.startNodeId;
  let startOffset = info.startOffset;
  let firstParaHandled = false;
  let anyPasted = false;

  // Validate that a node exists in the WASM model
  const nodeExists = (nodeId) => {
    try { doc.get_formatting_json(nodeId); return true; } catch (_) { return false; }
  };

  // Find a valid target node from WASM paragraph list
  const findValidTarget = () => {
    try {
      const allIds = JSON.parse(doc.paragraph_ids_json());
      if (allIds.length > 0) return { nodeId: allIds[allIds.length - 1], offset: 0 };
    } catch (_) {}
    // Last resort: create a paragraph
    try {
      const newId = doc.append_paragraph('');
      return { nodeId: newId, offset: 0 };
    } catch (_) {}
    return null;
  };

  // Ensure our target node is valid; if not, find a new one
  if (!nodeExists(lastNodeId)) {
    const target = findValidTarget();
    if (!target) return false;
    lastNodeId = target.nodeId;
    startOffset = target.offset;
    info = { startNodeId: lastNodeId, startOffset };
  }

  // Helper: find a body-level parent for insert_image/insert_table/insert_hr
  const getBodyNodeId = (nodeId) => {
    // Search across all page elements
    const container = $('pageContainer');
    if (!container) return nodeId;
    try {
      const el = container.querySelector(`[data-node-id="${nodeId}"]`);
      if (!el) return nodeId;
      let n = el;
      while (n && n.parentElement) {
        if (n.parentElement.classList?.contains('page-content') ||
            n.parentElement.id === 'pageContainer') {
          return n.dataset?.nodeId || nodeId;
        }
        n = n.parentElement;
      }
    } catch (_) {}
    return nodeId;
  };

  // Helper: get last paragraph ID from WASM (not DOM)
  const getLastParaId = () => {
    try {
      const allIds = JSON.parse(doc.paragraph_ids_json());
      if (allIds.length > 0) return allIds[allIds.length - 1];
    } catch (_) {}
    return lastNodeId;
  };

  for (let i = 0; i < elements.length; i++) {
    const el = elements[i];

    if (el.type === 'paragraph') {
      if (!firstParaHandled) {
        firstParaHandled = true;
        // Collect consecutive paragraphs for batch paste
        const textParas = [];
        let j = i;
        while (j < elements.length && elements[j].type === 'paragraph') {
          textParas.push(elements[j]);
          j++;
        }
        if (textParas.length > 0) {
          // Filter runs: drop empty text runs, sanitize data
          const cleanedParas = textParas.map(p => {
            const runs = (p.runs || []).filter(r => r.text && r.text.length > 0);
            return { ...p, runs };
          }).filter(p => p.runs.length > 0);

          if (cleanedParas.length > 0) {
            let richPasteOk = false;

            // Sanitize runs for JSON serialization
            const sanitizedParas = cleanedParas.map(p => {
              const runs = (p.runs || []).map(r => {
                const sr = { text: r.text || '' };
                if (r.bold === true) sr.bold = true;
                if (r.italic === true) sr.italic = true;
                if (r.underline === true) sr.underline = true;
                if (r.strikethrough === true) sr.strikethrough = true;
                if (r.superscript === true) sr.superscript = true;
                if (r.subscript === true) sr.subscript = true;
                if (typeof r.fontSize === 'number' && r.fontSize > 0) sr.fontSize = r.fontSize;
                if (typeof r.fontFamily === 'string' && r.fontFamily) sr.fontFamily = r.fontFamily;
                if (typeof r.color === 'string' && r.color) sr.color = r.color;
                if (typeof r.highlightColor === 'string' && r.highlightColor) sr.highlightColor = r.highlightColor;
                if (typeof r.hyperlinkUrl === 'string' && r.hyperlinkUrl) sr.hyperlinkUrl = r.hyperlinkUrl;
                return sr;
              });
              return { ...p, runs };
            });

            // Clamp startOffset to actual paragraph text length
            let safeOffset = startOffset;
            try {
              const paraText = doc.get_paragraph_text(info.startNodeId);
              const maxLen = paraText ? Array.from(paraText).length : 0;
              if (safeOffset > maxLen) safeOffset = maxLen;
            } catch (_) { safeOffset = 0; }

            // Strategy 1: batch paste via paste_formatted_runs_json
            try {
              const pasteJson = JSON.stringify({
                paragraphs: sanitizedParas.map(p => ({
                  runs: p.runs,
                  ...extractParaFmtForJson(p)
                }))
              });
              doc.paste_formatted_runs_json(info.startNodeId, safeOffset, pasteJson);
              broadcastOp({ action: 'pasteFormattedRuns', nodeId: info.startNodeId, offset: safeOffset, runsJson: pasteJson });
              richPasteOk = true;
              anyPasted = true;
            } catch (e) {
              console.error('[paste] paste_formatted_runs_json failed:', e.message || e);
              import('./toolbar-handlers.js').then(({ showToast: st }) => {
                st('Large paste \u2014 formatting may be simplified', 'info');
              });
            }

            // Strategy 2: plain text + per-run formatting
            if (!richPasteOk) {
              richPasteOk = pasteWithManualFormatting(doc, info.startNodeId, safeOffset, sanitizedParas);
              if (richPasteOk) anyPasted = true;
            }

            // Strategy 3: plain text only (no formatting)
            if (!richPasteOk) {
              try {
                const plainText = sanitizedParas.map(p => p.runs.map(r => r.text).join('')).join('\n');
                if (plainText) {
                  doc.paste_plain_text(info.startNodeId, safeOffset, plainText);
                  anyPasted = true;
                }
              } catch (e2) {
                try {
                  const flatText = sanitizedParas.map(p => p.runs.map(r => r.text).join('')).join(' ');
                  if (flatText) {
                    doc.insert_text_in_paragraph(info.startNodeId, safeOffset, flatText);
                    anyPasted = true;
                  }
                } catch (_) {}
              }
            }

            // Apply list formatting to pasted paragraphs
            if (anyPasted) {
              const hasLists = cleanedParas.some(p => p.listType);
              if (hasLists) {
                try {
                  const allIds = JSON.parse(doc.paragraph_ids_json());
                  const startIdx = allIds.indexOf(info.startNodeId);
                  if (startIdx >= 0) {
                    for (let pi = 0; pi < cleanedParas.length; pi++) {
                      const p = cleanedParas[pi];
                      if (p.listType) {
                        const paraIdx = startIdx + pi;
                        if (paraIdx < allIds.length) {
                          try { doc.set_list_format(allIds[paraIdx], p.listType, p.listLevel || 0); } catch (_) {}
                        }
                      }
                    }
                  }
                } catch (_) {}
              }
            }
          }

          i = j - 1; // skip pasted paragraphs
        }
        // Get fresh last paragraph ID from WASM
        lastNodeId = getLastParaId();
      } else {
        // Subsequent paragraphs after non-paragraph elements (images, tables, etc.)
        try {
          // Use WASM paragraph list instead of DOM to get text length
          const paraText = doc.get_paragraph_text(lastNodeId);
          const lastLen = paraText ? Array.from(paraText).length : 0;
          const newId = doc.split_paragraph(lastNodeId, lastLen);
          const text = el.runs ? el.runs.map(r => r.text).join('') : '';
          if (text) doc.insert_text_in_paragraph(newId, 0, text);
          lastNodeId = newId;
          anyPasted = true;
        } catch (e) {
          // Fallback: append paragraph
          try {
            const text = el.runs ? el.runs.map(r => r.text).join('') : '';
            const newId = doc.append_paragraph(text);
            lastNodeId = newId;
            anyPasted = true;
          } catch (e2) { console.warn('paste paragraph:', e2); }
        }
      }
      continue;
    }

    if (!firstParaHandled) {
      firstParaHandled = true;
    }

    // For non-paragraph elements, ensure lastNodeId is still valid
    if (!nodeExists(lastNodeId)) {
      lastNodeId = getLastParaId();
    }

    if (el.type === 'image') {
      try {
        const imgData = dataUrlToBytes(el.src);
        if (imgData) {
          const bodyId = getBodyNodeId(lastNodeId);
          const newId = doc.insert_image(bodyId, imgData.bytes, imgData.contentType, el.width || 200, el.height || 200);
          if (el.alt) {
            try { doc.set_image_alt_text(newId, el.alt); } catch (_) {}
          }
          lastNodeId = newId;
          anyPasted = true;
        }
      } catch (e) { console.warn('paste image:', e); }
      continue;
    }

    if (el.type === 'table') {
      try {
        const bodyId = getBodyNodeId(lastNodeId);
        const rows = el.rows.length;
        const cols = el.rows[0] ? el.rows[0].length : 1;
        const tableId = doc.insert_table(bodyId, rows, cols);
        const dims = JSON.parse(doc.get_table_dimensions(tableId));
        for (let r = 0; r < el.rows.length && r < dims.rows; r++) {
          for (let c = 0; c < el.rows[r].length && c < dims.cols; c++) {
            try {
              const cellId = doc.get_cell_id(tableId, r, c);
              const cell = el.rows[r][c];
              if (!cellId || !cell) continue;
              // cell is { text, runs } — set plain text, then apply run formatting
              const cellText = typeof cell === 'string' ? cell : (cell.text || '');
              if (cellText) {
                doc.set_cell_text(cellId, cellText);
              }
              // Apply inline formatting from runs if available
              const cellRuns = (typeof cell === 'object' && cell.runs) ? cell.runs : null;
              if (cellRuns && cellRuns.length > 1 && cellText) {
                // Try to apply per-run formatting via format_selection on the cell paragraph
                try {
                  // Find the paragraph inside this cell from the full paragraph list
                  // Cell paragraphs appear in paragraph_ids_json after table insertion
                  let runOffset = 0;
                  for (const run of cellRuns) {
                    const runEnd = runOffset + (run.text ? run.text.length : 0);
                    if (runEnd > runOffset) {
                      // Apply each formatting attribute using format_selection with cellId as paragraph
                      // Note: format_selection expects paragraph IDs; cell paragraphs use cellId as parent
                      if (run.bold) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'bold', 'true'); } catch(_){}
                      if (run.italic) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'italic', 'true'); } catch(_){}
                      if (run.underline) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'underline', 'true'); } catch(_){}
                      if (run.strikethrough) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'strikethrough', 'true'); } catch(_){}
                      if (run.fontSize) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'fontSize', String(run.fontSize)); } catch(_){}
                      if (run.fontFamily) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'fontFamily', run.fontFamily); } catch(_){}
                      if (run.color) try { doc.format_selection(cellId, runOffset, cellId, runEnd, 'color', run.color); } catch(_){}
                    }
                    runOffset = runEnd;
                  }
                } catch (_) {
                  // Formatting failed silently — plain text already set above
                }
              }
            } catch (_) {}
          }
        }
        lastNodeId = tableId;
        anyPasted = true;
      } catch (e) { console.warn('paste table:', e); }
      continue;
    }

    if (el.type === 'hr') {
      try {
        const bodyId = getBodyNodeId(lastNodeId);
        const newId = doc.insert_horizontal_rule(bodyId);
        lastNodeId = newId;
        anyPasted = true;
      } catch (e) { console.warn('paste hr:', e); }
      continue;
    }
  }

  return anyPasted;
}

/**
 * Fallback paste strategy: insert plain text, then apply formatting per-run
 * using format_selection. Works when paste_formatted_runs_json fails.
 */
function pasteWithManualFormatting(doc, targetNodeId, offset, sanitizedParas) {
  try {
    // For single-paragraph paste: insert text then format each run
    if (sanitizedParas.length === 1) {
      const runs = sanitizedParas[0].runs;
      const fullText = runs.map(r => r.text).join('');
      if (!fullText) return false;

      doc.insert_text_in_paragraph(targetNodeId, offset, fullText);

      // Now format each run's character range
      let runStart = offset;
      for (const run of runs) {
        const runLen = Array.from(run.text).length;
        if (runLen === 0) continue;
        const runEnd = runStart + runLen;
        const hasFormatting = run.bold || run.italic || run.underline ||
          run.strikethrough || run.fontSize || run.fontFamily || run.color;
        if (hasFormatting) {
          // Apply each formatting attribute individually
          if (run.bold) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'bold', 'true'); } catch(_){}
          if (run.italic) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'italic', 'true'); } catch(_){}
          if (run.underline) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'underline', 'true'); } catch(_){}
          if (run.strikethrough) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'strikethrough', 'true'); } catch(_){}
          if (run.fontSize) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'fontSize', String(run.fontSize)); } catch(_){}
          if (run.fontFamily) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'fontFamily', run.fontFamily); } catch(_){}
          if (run.color) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'color', run.color); } catch(_){}
          if (run.highlightColor) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'highlightColor', run.highlightColor); } catch(_){}
          if (run.superscript) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'superscript', 'true'); } catch(_){}
          if (run.subscript) try { doc.format_selection(targetNodeId, runStart, targetNodeId, runEnd, 'subscript', 'true'); } catch(_){}
        }
        runStart = runEnd;
      }
      return true;
    }

    // Multi-paragraph: insert as plain text with newlines, then format
    const plainText = sanitizedParas.map(p => p.runs.map(r => r.text).join('')).join('\n');
    if (!plainText) return false;

    doc.paste_plain_text(targetNodeId, offset, plainText);

    // Now apply formatting paragraph by paragraph
    const allIds = JSON.parse(doc.paragraph_ids_json());
    const startIdx = allIds.indexOf(targetNodeId);
    if (startIdx < 0) return true; // text was pasted, just no formatting

    let paraIdx = startIdx;
    for (const para of sanitizedParas) {
      if (paraIdx >= allIds.length) break;
      const currentParaId = allIds[paraIdx];

      // Determine run offset within this paragraph
      // For first para: starts at 'offset', for subsequent: starts at 0
      let runStart = (paraIdx === startIdx) ? offset : 0;

      for (const run of para.runs) {
        const runLen = Array.from(run.text).length;
        if (runLen === 0) continue;
        const runEnd = runStart + runLen;
        const hasFormatting = run.bold || run.italic || run.underline ||
          run.strikethrough || run.fontSize || run.fontFamily || run.color;
        if (hasFormatting) {
          if (run.bold) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'bold', 'true'); } catch(_){}
          if (run.italic) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'italic', 'true'); } catch(_){}
          if (run.underline) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'underline', 'true'); } catch(_){}
          if (run.strikethrough) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'strikethrough', 'true'); } catch(_){}
          if (run.fontSize) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'fontSize', String(run.fontSize)); } catch(_){}
          if (run.fontFamily) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'fontFamily', run.fontFamily); } catch(_){}
          if (run.color) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'color', run.color); } catch(_){}
          if (run.highlightColor) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'highlightColor', run.highlightColor); } catch(_){}
          if (run.superscript) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'superscript', 'true'); } catch(_){}
          if (run.subscript) try { doc.format_selection(currentParaId, runStart, currentParaId, runEnd, 'subscript', 'true'); } catch(_){}
        }
        runStart = runEnd;
      }
      paraIdx++;
    }
    return true;
  } catch (e) {
    console.error('[paste] Manual formatting fallback failed:', e);
    return false;
  }
}

/** Extract paragraph format fields into a flat object for JSON */
function extractParaFmtForJson(para) {
  const fmt = {};
  if (para.alignment) fmt.alignment = para.alignment;
  if (para.spacingBefore) fmt.spacingBefore = para.spacingBefore;
  if (para.spacingAfter) fmt.spacingAfter = para.spacingAfter;
  if (para.lineSpacing) fmt.lineSpacing = para.lineSpacing;
  if (para.indentLeft) fmt.indentLeft = para.indentLeft;
  if (para.indentRight) fmt.indentRight = para.indentRight;
  if (para.indentFirstLine) fmt.indentFirstLine = para.indentFirstLine;
  if (para.headingLevel) fmt.headingLevel = para.headingLevel;
  if (para.listType) fmt.listType = para.listType;
  if (para.listLevel != null) fmt.listLevel = para.listLevel;
  return fmt;
}

/** Convert a data URL to { bytes: Uint8Array, contentType: string } */
function dataUrlToBytes(dataUrl) {
  if (!dataUrl) return null;
  // Handle data URLs
  const match = dataUrl.match(/^data:([^;]+);base64,(.+)$/);
  if (match) {
    const contentType = match[1];
    const b64 = match[2];
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return { bytes, contentType };
  }
  return null;
}

/** Find the last paragraph node ID in the editor */
function findLastParagraphId(page) {
  const pages = state.pageElements.length > 0 ? state.pageElements : [page];
  for (let i = pages.length - 1; i >= 0; i--) {
    const content = pages[i].querySelector('.page-content') || pages[i];
    const nodes = content.querySelectorAll(':scope > [data-node-id]');
    if (nodes.length > 0) return nodes[nodes.length - 1].dataset.nodeId;
  }
  return null;
}

/** Extract paragraph-level formatting from a block element. */
function extractParagraphFormat(block, para) {
  const tag = block.tagName.toLowerCase();
  // Heading level
  const hMatch = tag.match(/^h([1-6])$/);
  if (hMatch) para.headingLevel = parseInt(hMatch[1]);

  const style = block.style;
  if (!style) return;

  // Text alignment
  if (style.textAlign) {
    const align = style.textAlign.toLowerCase();
    if (['left', 'center', 'right', 'justify'].includes(align)) {
      para.alignment = align;
    }
  }

  // Spacing (margins → paragraph spacing in pt)
  if (style.marginTop) {
    const v = parseFloat(style.marginTop);
    if (v > 0) para.spacingBefore = style.marginTop.endsWith('px') ? v * 0.75 : v;
  }
  if (style.marginBottom) {
    const v = parseFloat(style.marginBottom);
    if (v > 0) para.spacingAfter = style.marginBottom.endsWith('px') ? v * 0.75 : v;
  }

  // Line spacing
  if (style.lineHeight) {
    const lh = style.lineHeight;
    if (lh === '1.5' || lh === '150%') para.lineSpacing = '1.5';
    else if (lh === '2' || lh === '200%') para.lineSpacing = '2';
  }

  // Indentation
  if (style.paddingLeft || style.marginLeft) {
    const raw = style.paddingLeft || style.marginLeft;
    const v = parseFloat(raw);
    if (v > 0) para.indentLeft = raw.endsWith('px') ? v * 0.75 : v;
  }
  if (style.paddingRight || style.marginRight) {
    const raw = style.paddingRight || style.marginRight;
    const v = parseFloat(raw);
    if (v > 0) para.indentRight = raw.endsWith('px') ? v * 0.75 : v;
  }
  if (style.textIndent) {
    const v = parseFloat(style.textIndent);
    if (v !== 0) para.indentFirstLine = style.textIndent.endsWith('px') ? v * 0.75 : v;
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

  // Skip MS Office namespace elements and empty marker elements
  if (tag.includes(':') || tag === 'meta' || tag === 'link' || tag === 'style') return;

  // Build formatting from this element
  const fmt = { ...inherited };
  if (tag === 'b' || tag === 'strong') fmt.bold = true;
  if (tag === 'i' || tag === 'em') fmt.italic = true;
  if (tag === 'u' || tag === 'ins') fmt.underline = true;
  if (tag === 's' || tag === 'strike' || tag === 'del') fmt.strikethrough = true;
  if (tag === 'sup') fmt.superscript = true;
  if (tag === 'sub') fmt.subscript = true;
  if (tag === 'a') {
    const href = node.getAttribute('href');
    if (href && !href.startsWith('javascript:') && !href.startsWith('#_mso')) {
      fmt.hyperlinkUrl = href;
    }
  }
  if (tag === 'br') {
    // Line breaks within a block become newline text
    runs.push({ text: '\n', ...inherited });
    return;
  }

  // Parse inline styles
  const style = node.style;
  if (style) {
    const fw = style.fontWeight;
    if (fw === 'bold' || fw === 'bolder' || (parseInt(fw) >= 700 && !isNaN(parseInt(fw)))) fmt.bold = true;
    // Allow style to override tag-based bold (e.g. Google Docs <b> with font-weight:normal)
    if (fw === 'normal' || fw === '400') fmt.bold = inherited.bold || false;
    if (style.fontStyle === 'italic') fmt.italic = true;
    if (style.fontStyle === 'normal' && (tag === 'i' || tag === 'em')) fmt.italic = false;
    const td = style.textDecoration || style.textDecorationLine || '';
    if (td.includes('underline')) fmt.underline = true;
    if (td.includes('line-through')) fmt.strikethrough = true;
    if (style.fontSize) {
      const size = parseFloat(style.fontSize);
      if (size > 0) {
        // Convert px to pt (1pt = 1.333px)
        if (style.fontSize.endsWith('px')) fmt.fontSize = Math.round(size * 0.75 * 10) / 10;
        else if (style.fontSize.endsWith('pt')) fmt.fontSize = size;
        else if (style.fontSize.endsWith('em') || style.fontSize.endsWith('rem')) fmt.fontSize = Math.round(size * 12);
      }
    }
    if (style.fontFamily) {
      const ff = style.fontFamily.replace(/['"]/g, '').split(',')[0].trim();
      if (ff) fmt.fontFamily = ff;
    }
    if (style.color) {
      const hex = colorToHex(style.color);
      if (hex && hex !== '000000') fmt.color = hex;
    }
    if (style.backgroundColor) {
      const hex = colorToHex(style.backgroundColor);
      if (hex && hex !== 'FFFFFF' && hex !== 'TRANSPARENT') fmt.highlightColor = hex;
    }
    if (style.verticalAlign === 'super') fmt.superscript = true;
    if (style.verticalAlign === 'sub') fmt.subscript = true;
  }

  // MS Word specific: mso-bidi-* styles in cssText
  if (style && style.cssText) {
    if (/mso-bidi-font-weight\s*:\s*bold/i.test(style.cssText)) fmt.bold = true;
    if (/mso-bidi-font-style\s*:\s*italic/i.test(style.cssText)) fmt.italic = true;
  }

  // Check for class-based formatting (some editors use classes)
  const cls = node.className || '';
  if (typeof cls === 'string') {
    if (cls.includes('bold') || cls.includes('font-weight-bold')) fmt.bold = true;
    if (cls.includes('italic')) fmt.italic = true;
    if (cls.includes('underline')) fmt.underline = true;
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
    !!a.superscript === !!b.superscript &&
    !!a.subscript === !!b.subscript &&
    (a.fontSize || null) === (b.fontSize || null) &&
    (a.fontFamily || null) === (b.fontFamily || null) &&
    (a.color || null) === (b.color || null) &&
    (a.highlightColor || null) === (b.highlightColor || null) &&
    (a.hyperlinkUrl || null) === (b.hyperlinkUrl || null);
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
// pasteNodeId + pasteOffset: where paste started
function placeCursorAfterPaste(page, text, pasteNodeId, pasteOffset) {
  // Helper to find element by node ID across all pages
  const findNodeEl = (nodeId) => {
    // Search across all pages, not just one
    for (const pageEl of (state.pageElements.length > 0 ? state.pageElements : [page])) {
      const content = pageEl.querySelector?.('.page-content') || pageEl;
      const el = content.querySelector(`[data-node-id="${nodeId}"]`);
      if (el) return el;
    }
    return page.querySelector(`[data-node-id="${nodeId}"]`);
  };

  // Try to find where pasted content ends by using WASM paragraph list
  if (pasteNodeId && state.doc) {
    try {
      const allIds = JSON.parse(state.doc.paragraph_ids_json());
      const startIdx = allIds.indexOf(pasteNodeId);
      if (startIdx >= 0) {
        const newlineCount = text ? (text.match(/\n/g) || []).length : 0;
        const targetIdx = Math.min(startIdx + newlineCount, allIds.length - 1);
        const targetId = allIds[targetIdx];
        const el = findNodeEl(targetId);
        if (el) {
          const content = el.closest('.page-content');
          if (content) content.focus();
          const len = Array.from(getEditableText(el)).length;
          setCursorAtOffset(el, len);
          return;
        }
      }
    } catch (_) {}
  }
  // Fallback: place at end of last paragraph across all pages
  const pages = state.pageElements.length > 0 ? state.pageElements : [page];
  for (let i = pages.length - 1; i >= 0; i--) {
    const content = pages[i].querySelector?.('.page-content') || pages[i];
    const nodes = content.querySelectorAll(':scope > [data-node-id]');
    if (nodes.length > 0) {
      const lastEl = nodes[nodes.length - 1];
      const pc = lastEl.closest('.page-content');
      if (pc) pc.focus();
      const len = Array.from(getEditableText(lastEl)).length;
      setCursorAtOffset(lastEl, len);
      return;
    }
  }
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
  clearTimeout(state.syncTimer);
  syncAllText();
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
  // Use select-all info if active, otherwise resolve from DOM
  const info = state._selectAll ? state.lastSelInfo : getSelectionInfo();
  if (!info || info.collapsed || !state.doc) return;

  const wasSelectAll = state._selectAll;
  const isCrossPage = !wasSelectAll && info.startNodeId !== info.endNodeId &&
    info.startEl?.closest?.('.page-content') !== info.endEl?.closest?.('.page-content');
  clearSelectAll();

  // E2.3: Copy via WASM then delete
  syncAllText();

  // Generate clean HTML from WASM model
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

  // Get text: from WASM if was select-all, extract from HTML for cross-page, else from selection
  let text = '';
  if (wasSelectAll) {
    try { text = state.doc.to_plain_text(); } catch (_) { text = window.getSelection()?.toString() || ''; }
  } else if (isCrossPage && html) {
    text = htmlToPlainText(html);
  } else {
    text = window.getSelection()?.toString() || '';
  }

  // Write to clipboard
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

  // Delete the selection
  try {
    state.doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
    broadcastOp({ action: 'deleteSelection', startNode: info.startNodeId, startOffset: info.startOffset, endNode: info.endNodeId, endOffset: info.endOffset });
    renderDocument();
    const el = $('pageContainer')?.querySelector(`[data-node-id="${info.startNodeId}"]`);
    if (el) {
      const content = el.closest('.page-content');
      if (content) content.focus();
      setCursorAtOffset(el, info.startOffset);
    } else {
      const first = $('pageContainer')?.querySelector('[data-node-id]');
      if (first) {
        const content = first.closest('.page-content');
        if (content) content.focus();
        setCursorAtStart(first);
      } else {
        state.doc.append_paragraph('');
        renderDocument();
        const n = $('pageContainer')?.querySelector('[data-node-id]');
        if (n) {
          const content = n.closest('.page-content');
          if (content) content.focus();
          setCursorAtStart(n);
        }
      }
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
    const now = Date.now();
    // CRC32 checksum for integrity verification
    const checksum = _crc32Local(bytes);
    const commentReplies = state.commentReplies && state.commentReplies.length > 0
      ? JSON.stringify(state.commentReplies) : null;
    openAutosaveDB().then(db => {
      const tx = db.transaction('documents', 'readwrite');
      const store = tx.objectStore('documents');
      store.put({ id: 'current', name, bytes, timestamp: now, tabId: state.tabId, commentReplies, checksum });
      state.lastSaveTimestamp = now;
      state.dirty = false;
      updateDirtyIndicator();
      const info = $('statusInfo');
      info._userMsg = true;
      info.textContent = 'Saved';
      setTimeout(() => { info._userMsg = false; updateStatusBar(); }, 1500);
    }).catch(e => console.error('save:', e));
    // Also save a version snapshot on manual save
    saveVersion('Manual save');
  } catch (e) { console.error('save:', e); }
}

// Lightweight CRC32 for manual save (same algorithm as file.js)
const _crc32LocalTable = (() => {
  const t = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let j = 0; j < 8; j++) c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
    t[i] = c;
  }
  return t;
})();
function _crc32Local(bytes) {
  const data = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  let crc = 0xFFFFFFFF;
  for (let i = 0; i < data.length; i++) crc = _crc32LocalTable[(crc ^ data[i]) & 0xFF] ^ (crc >>> 8);
  return (crc ^ 0xFFFFFFFF) >>> 0;
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
  { id: 'sectionbreak', label: 'Section Break', icon: '\u2500',  keywords: 'section break next page continuous' },
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
  const menu = $('slashMenu');
  if (menu) {
    menu.setAttribute('role', 'listbox');
    menu.setAttribute('aria-label', 'Insert commands');
  }
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
      case 'heading1': doc.set_heading_level(nodeId, 1); broadcastOp({ action: 'setHeading', nodeId, level: 1 }); renderDocument(); restoreCursorAfterRender(nodeId); break;
      case 'heading2': doc.set_heading_level(nodeId, 2); broadcastOp({ action: 'setHeading', nodeId, level: 2 }); renderDocument(); restoreCursorAfterRender(nodeId); break;
      case 'heading3': doc.set_heading_level(nodeId, 3); broadcastOp({ action: 'setHeading', nodeId, level: 3 }); renderDocument(); restoreCursorAfterRender(nodeId); break;
      case 'bullet':   doc.set_list_format(nodeId, 'bullet', 0); broadcastOp({ action: 'setListFormat', nodeId, format: 'bullet', level: 0 }); renderDocument(); restoreCursorAfterRender(nodeId); break;
      case 'numbered': doc.set_list_format(nodeId, 'decimal', 0); broadcastOp({ action: 'setListFormat', nodeId, format: 'decimal', level: 0 }); renderDocument(); restoreCursorAfterRender(nodeId); break;
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
      case 'sectionbreak':
        doc.insert_section_break(nodeId, 'nextPage');
        broadcastOp({ action: 'insertSectionBreak', afterNodeId: nodeId, breakType: 'nextPage' });
        renderDocument();
        break;
      case 'quote': {
        doc.set_heading_level(nodeId, 0);
        broadcastOp({ action: 'setHeading', nodeId, level: 0 });
        const textLen = el ? Array.from(getEditableText(el)).length : 0;
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
        const codeLen = el ? Array.from(getEditableText(el)).length : 0;
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
      hr: 'Insert horizontal rule', pagebreak: 'Insert page break', sectionbreak: 'Insert section break', quote: 'Apply quote style', code: 'Apply code style' };
    if (labels[cmdId]) recordUndoAction(labels[cmdId]);
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.error('slash command:', e); }
}

export { closeSlashMenu };

// Expose for toolbar buttons
export { doUndo, doRedo };

// ─── Restore cursor after re-render ────────────────
function restoreCursorAfterRender(nodeId) {
  const page = $('pageContainer');
  if (!page) return;
  const el = page.querySelector(`[data-node-id="${nodeId}"]`);
  if (el) setCursorAtStart(el);
}

// ─── Select All (cross-page) ────────────────────────
function selectAll() {
  const page = $('pageContainer');
  if (!page || !state.doc) return;

  // Collect all paragraph-level elements across all pages
  const allNodes = [];
  const pages = state.pageElements.length > 0 ? state.pageElements : [page];
  for (const pageEl of pages) {
    const content = pageEl.querySelector('.page-content') || pageEl;
    content.querySelectorAll(':scope > [data-node-id]').forEach(el => allNodes.push(el));
  }
  if (allNodes.length === 0) return;

  const firstEl = allNodes[0];
  const lastEl = allNodes[allNodes.length - 1];
  const lastLen = Array.from(getEditableText(lastEl)).length;

  // Store synthetic full-document selection
  state.lastSelInfo = {
    startNodeId: firstEl.dataset.nodeId,
    startOffset: 0,
    endNodeId: lastEl.dataset.nodeId,
    endOffset: lastLen,
    collapsed: false,
    startEl: firstEl,
    endEl: lastEl,
  };
  state._selectAll = true;

  // Visual: highlight ALL page-content areas and their child nodes across all pages
  for (const pageEl of pages) {
    const content = pageEl.querySelector('.page-content') || pageEl;
    content.classList.add('select-all-highlight');
    content.querySelectorAll('[data-node-id]').forEach(el => el.classList.add('select-all-highlight'));
  }

  // Set native selection on the first page for keyboard events to work
  const firstContent = firstEl.closest('.page-content');
  if (firstContent) {
    try {
      firstContent.focus();
      const range = document.createRange();
      range.selectNodeContents(firstContent);
      const sel = window.getSelection();
      sel.removeAllRanges(); sel.addRange(range);
    } catch (_) {}
  }
}

function clearSelectAll() {
  state._selectAll = false;
  const page = $('pageContainer');
  if (page) {
    page.querySelectorAll('.select-all-highlight').forEach(el => el.classList.remove('select-all-highlight'));
  }
}

// E10.2: Zoom via keyboard (Ctrl+=/Ctrl+-/Ctrl+0)
function adjustEditorZoom(delta) {
  if (delta === 0) {
    setZoomLevel(100);
  } else {
    setZoomLevel((state.zoomLevel || 100) + delta);
  }
}

// ═══════════════════════════════════════════════════
// E9.6: Insert Footnote / Endnote at Cursor
// ═══════════════════════════════════════════════════

export function insertFootnoteAtCursor() {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  const nodeId = info.startNodeId;
  if (!nodeId) return;
  import('./render.js').then(({ syncAllText: syncAll, renderDocument: renderDoc }) => {
    syncAll();
    try {
      if (typeof state.doc.insert_footnote === 'function') {
        state.doc.insert_footnote(nodeId, '');
        import('./collab.js').then(({ broadcastOp: bcast }) => {
          bcast({ action: 'insertFootnote', nodeId });
        });
        renderDoc();
        import('./toolbar.js').then(({ updateUndoRedo: uur, recordUndoAction: rua }) => {
          rua('Insert footnote');
          uur();
        });
        import('./toolbar-handlers.js').then(({ announce: ann }) => { ann('Footnote inserted'); });
      } else {
        import('./toolbar-handlers.js').then(({ showToast: st }) => {
          st('Footnote insertion not available in this build.', 'info');
        });
      }
    } catch (e) { console.error('insert footnote:', e); }
  });
}

export function insertEndnoteAtCursor() {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  const nodeId = info.startNodeId;
  if (!nodeId) return;
  import('./render.js').then(({ syncAllText: syncAll, renderDocument: renderDoc }) => {
    syncAll();
    try {
      if (typeof state.doc.insert_endnote === 'function') {
        state.doc.insert_endnote(nodeId, '');
        import('./collab.js').then(({ broadcastOp: bcast }) => {
          bcast({ action: 'insertEndnote', nodeId });
        });
        renderDoc();
        import('./toolbar.js').then(({ updateUndoRedo: uur, recordUndoAction: rua }) => {
          rua('Insert endnote');
          uur();
        });
        import('./toolbar-handlers.js').then(({ announce: ann }) => { ann('Endnote inserted'); });
      } else {
        import('./toolbar-handlers.js').then(({ showToast: st }) => {
          st('Endnote insertion not available in this build.', 'info');
        });
      }
    } catch (e) { console.error('insert endnote:', e); }
  });
}

// ═══════════════════════════════════════════════════
// E6.2: Pinch-to-Zoom (trackpad ctrl+wheel)
// ═══════════════════════════════════════════════════

export function initPinchToZoom() {
  const canvas = $('editorCanvas');
  if (!canvas) return;

  canvas.addEventListener('wheel', e => {
    if (!e.ctrlKey) return;
    // Trackpad pinch fires as ctrl+wheel
    e.preventDefault();
    const delta = e.deltaY > 0 ? -5 : 5;
    setZoomLevel((state.zoomLevel || 100) + delta);
  }, { passive: false });
}

// ═══════════════════════════════════════════════════
// E7.2: Table Cell Navigation Announcements
// ═══════════════════════════════════════════════════

export function initTableCellAnnouncements() {
  let _lastAnnouncedCell = null;
  document.addEventListener('selectionchange', () => {
    const sel = window.getSelection();
    if (!sel || !sel.rangeCount) return;
    const anchor = sel.anchorNode;
    if (!anchor) return;
    const el = anchor.nodeType === 1 ? anchor : anchor.parentElement;
    if (!el) return;
    const cell = el.closest('td, th');
    if (!cell) {
      _lastAnnouncedCell = null;
      return;
    }
    if (cell === _lastAnnouncedCell) return;
    _lastAnnouncedCell = cell;

    const row = cell.parentElement;
    const table = row?.closest('table');
    if (!row || !table) return;

    const rowIdx = Array.from(row.parentElement?.children || []).indexOf(row) + 1;
    const colIdx = Array.from(row.children).indexOf(cell) + 1;
    const isHeader = cell.tagName === 'TH' || row.parentElement?.tagName === 'THEAD';
    const label = `Row ${rowIdx}, Column ${colIdx}${isHeader ? ', header row' : ''}`;

    import('./toolbar-handlers.js').then(({ announce: ann }) => { ann(label); });
  });
}
