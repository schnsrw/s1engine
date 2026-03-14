// Toolbar event handler wiring
import { state, $ } from './state.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo } from './toolbar.js';
import { doUndo, doRedo } from './input.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText } from './render.js';
import { getSelectionInfo, setCursorAtOffset, setSelectionRange, getActiveNodeId } from './selection.js';
import { insertImage } from './images.js';
import { updatePageBreaks } from './pagination.js';

export function initToolbar() {
  // Format toggles
  $('btnBold').addEventListener('click', () => toggleFormat('bold'));
  $('btnItalic').addEventListener('click', () => toggleFormat('italic'));
  $('btnUnderline').addEventListener('click', () => toggleFormat('underline'));
  $('btnStrike').addEventListener('click', () => toggleFormat('strikethrough'));
  $('btnSuperscript').addEventListener('click', () => toggleFormat('superscript'));
  $('btnSubscript').addEventListener('click', () => toggleFormat('subscript'));

  // Clear formatting
  $('btnClearFormat').addEventListener('click', () => {
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    syncAllText();
    try {
      const keys = ['bold', 'italic', 'underline', 'strikethrough', 'superscript', 'subscript', 'color', 'highlight', 'fontFamily', 'fontSize'];
      keys.forEach(k => {
        try {
          if (info.collapsed) {
            const el = $('docPage').querySelector(`[data-node-id="${info.startNodeId}"]`);
            const textLen = el ? Array.from(el.textContent || '').length : 0;
            if (textLen > 0) state.doc.format_selection(info.startNodeId, 0, info.startNodeId, textLen, k, 'false');
          } else {
            state.doc.format_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset, k, 'false');
          }
        } catch (_) {}
      });
      renderDocument();
      updateToolbarState();
      updateUndoRedo();
    } catch (e) { console.error('clear format:', e); }
  });

  // Undo/Redo
  $('btnUndo').addEventListener('click', doUndo);
  $('btnRedo').addEventListener('click', doRedo);

  // Print
  $('btnPrint').addEventListener('click', () => {
    window.print();
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

  // Block type (heading level)
  $('blockType').addEventListener('change', e => {
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    const el = $('docPage').querySelector(`[data-node-id="${info.startNodeId}"]`);
    if (el) syncParagraphText(el);
    try {
      state.doc.set_heading_level(info.startNodeId, parseInt(e.target.value));
      renderDocument();
      updateToolbarState();
      updateUndoRedo();
    } catch (err) { console.error('heading:', err); }
  });

  // Text color
  $('colorPicker').addEventListener('input', e => {
    const hex = e.target.value.replace('#', '');
    $('colorSwatch').style.background = '#' + hex;
    applyFormat('color', hex);
  });

  // Highlight color
  $('highlightPicker').addEventListener('input', e => {
    const hex = e.target.value.replace('#', '');
    applyFormat('highlight', hex);
  });

  // Line spacing
  $('lineSpacing').addEventListener('change', e => {
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    syncAllText();
    try {
      state.doc.set_line_spacing(info.startNodeId, e.target.value);
      renderNodeById(info.startNodeId);
      state.pagesRendered = false;
      updatePageBreaks();
      updateUndoRedo();
    } catch (err) { console.error('line spacing:', err); }
  });

  // Indent / Outdent
  $('btnIndent').addEventListener('click', () => applyIndent(36));   // +0.5in (36pt)
  $('btnOutdent').addEventListener('click', () => applyIndent(-36)); // -0.5in

  // Alignment
  $('btnAlignL').addEventListener('click', () => applyAlignment('left'));
  $('btnAlignC').addEventListener('click', () => applyAlignment('center'));
  $('btnAlignR').addEventListener('click', () => applyAlignment('right'));
  $('btnAlignJ').addEventListener('click', () => applyAlignment('justify'));

  // Lists
  $('btnBulletList').addEventListener('click', () => toggleList('bullet'));
  $('btnNumberList').addEventListener('click', () => toggleList('decimal'));

  // Insert menu
  $('btnInsertMenu').addEventListener('click', e => {
    e.stopPropagation();
    const menu = $('insertMenu');
    menu.classList.toggle('show');
    $('btnInsertMenu').setAttribute('aria-expanded', menu.classList.contains('show') ? 'true' : 'false');
  });

  // Insert table
  $('miTable').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    $('tableModal').classList.add('show');
    $('tableRows').focus();
  });
  $('tableCancelBtn').addEventListener('click', () => {
    $('tableModal').classList.remove('show');
    $('docPage').focus();
  });
  $('tableInsertBtn').addEventListener('click', () => {
    const rows = parseInt($('tableRows').value) || 3;
    const cols = parseInt($('tableCols').value) || 3;
    if (rows < 1 || rows > 100 || cols < 1 || cols > 50) {
      alert('Rows must be 1-100, columns must be 1-50.');
      return;
    }
    $('tableModal').classList.remove('show');
    if (!state.doc) return;
    const nodeId = getActiveNodeId();
    if (!nodeId) return;
    syncAllText();
    try {
      state.doc.insert_table(nodeId, rows, cols);
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('insert table:', e); }
  });
  // Modal backdrop click to close
  $('tableModal').addEventListener('click', e => {
    if (e.target === $('tableModal')) {
      $('tableModal').classList.remove('show');
      $('docPage').focus();
    }
  });

  // Insert image
  $('miImage').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    $('imageInput').click();
  });
  $('imageInput').addEventListener('change', e => {
    const f = e.target.files[0];
    if (f) insertImage(f);
    e.target.value = '';
  });

  // Insert hyperlink
  $('miLink').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    const url = prompt('Enter URL (e.g. https://example.com):');
    if (!url || !state.doc) return;
    // Validate URL format
    let validUrl = url;
    if (!/^https?:\/\//i.test(url) && !url.startsWith('#')) {
      validUrl = 'https://' + url;
    }
    try { new URL(validUrl); } catch (_) {
      alert('Invalid URL: ' + url);
      return;
    }
    const info = getSelectionInfo();
    if (!info) return;
    try {
      applyFormat('hyperlinkUrl', validUrl);
    } catch (e) { console.error('hyperlink:', e); }
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
      renderDocument();
      updateUndoRedo();
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
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('insert page break:', e); }
  });

  // Insert comment
  $('miComment').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    const text = prompt('Enter comment:');
    if (!text) return;
    const author = prompt('Author:', 'User') || 'User';
    try {
      state.doc.insert_comment(info.startNodeId, info.endNodeId, author, text);
      renderDocument();
      updateUndoRedo();
      refreshComments();
    } catch (e) { console.error('insert comment:', e); }
  });

  // Comments panel toggle
  $('btnComments').addEventListener('click', () => {
    $('commentsPanel').classList.toggle('show');
    if ($('commentsPanel').classList.contains('show')) refreshComments();
  });
  $('commentsClose').addEventListener('click', () => {
    $('commentsPanel').classList.remove('show');
  });

  // Zoom controls
  $('zoomIn').addEventListener('click', () => adjustZoom(10));
  $('zoomOut').addEventListener('click', () => adjustZoom(-10));

  // Table context menu
  initTableContextMenu();

  // Close menus on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('.insert-dropdown')) {
      $('insertMenu').classList.remove('show');
      $('btnInsertMenu').setAttribute('aria-expanded', 'false');
    }
    $('tableContextMenu').style.display = 'none';
  });
}

function applyAlignment(align) {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  const el = $('docPage').querySelector(`[data-node-id="${info.startNodeId}"]`);
  if (el) syncParagraphText(el);
  try {
    state.doc.set_alignment(info.startNodeId, align);
    const updated = renderNodeById(info.startNodeId);
    if (updated) setCursorAtOffset(updated, info.startOffset);
    state.pagesRendered = false;
    updatePageBreaks();
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.error('alignment:', e); }
}

function toggleList(format) {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  syncAllText();
  try {
    state.doc.set_list_format(info.startNodeId, format, 0);
    renderDocument();
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.error('list:', e); }
}

function applyIndent(delta) {
  if (!state.doc) return;
  const info = getSelectionInfo();
  if (!info) return;
  syncAllText();
  try {
    // Get current indent, add delta, clamp to 0
    const fmt = JSON.parse(state.doc.get_formatting_json(info.startNodeId));
    const current = parseFloat(fmt.indentLeft || '0');
    const newVal = Math.max(0, current + delta);
    state.doc.set_indent(info.startNodeId, newVal);
    renderNodeById(info.startNodeId);
    state.pagesRendered = false;
    updatePageBreaks();
    updateUndoRedo();
  } catch (e) { console.error('indent:', e); }
}

function adjustZoom(delta) {
  state.zoomLevel = (state.zoomLevel || 100) + delta;
  state.zoomLevel = Math.max(50, Math.min(200, state.zoomLevel));
  $('zoomValue').textContent = state.zoomLevel + '%';
  const page = $('docPage');
  if (page) {
    page.style.transform = `scale(${state.zoomLevel / 100})`;
    page.style.transformOrigin = 'top center';
  }
}

function refreshComments() {
  const list = $('commentsList');
  if (!list || !state.doc) return;
  try {
    const comments = JSON.parse(state.doc.get_comments_json());
    if (!comments || comments.length === 0) {
      list.innerHTML = '<div class="comments-empty">No comments in this document.</div>';
      return;
    }
    list.innerHTML = comments.map(c => `
      <div class="comment-card" data-comment-id="${escapeAttr(c.id || '')}">
        <div class="comment-author">${escapeHtml(c.author || 'Unknown')}</div>
        ${c.date ? `<div class="comment-date">${escapeHtml(c.date)}</div>` : ''}
        <div class="comment-text">${escapeHtml(c.text || c.body || '')}</div>
        <div class="comment-actions">
          <button class="comment-delete" data-id="${escapeAttr(c.id || '')}">Delete</button>
        </div>
      </div>
    `).join('');
    list.querySelectorAll('.comment-delete').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.dataset.id;
        if (!id || !state.doc) return;
        try {
          state.doc.delete_comment(id);
          renderDocument();
          updateUndoRedo();
          refreshComments();
        } catch (e) { console.error('delete comment:', e); }
      });
    });
  } catch (e) {
    list.innerHTML = '<div class="comments-empty">Unable to load comments.</div>';
  }
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}
function escapeAttr(s) {
  return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

function initTableContextMenu() {
  $('docPage').addEventListener('contextmenu', e => {
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
      try { fn(); renderDocument(); updateUndoRedo(); }
      catch (e) { console.error('table op:', e); }
    });
  };

  cmAction('cmInsertRowAbove', () => state.doc.insert_table_row(state.ctxTable, state.ctxRow));
  cmAction('cmInsertRowBelow', () => state.doc.insert_table_row(state.ctxTable, state.ctxRow + 1));
  cmAction('cmDeleteRow', () => state.doc.delete_table_row(state.ctxTable, state.ctxRow));
  cmAction('cmInsertColLeft', () => state.doc.insert_table_column(state.ctxTable, state.ctxCol));
  cmAction('cmInsertColRight', () => state.doc.insert_table_column(state.ctxTable, state.ctxCol + 1));
  cmAction('cmDeleteCol', () => state.doc.delete_table_column(state.ctxTable, state.ctxCol));

  $('cmCellBg').addEventListener('click', () => {
    $('tableContextMenu').style.display = 'none';
    if (!state.doc || !state.ctxCell) return;
    const color = prompt('Enter hex color (e.g. FF0000):');
    if (!color) return;
    try {
      state.doc.set_cell_background(state.ctxCell, color);
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('cell bg:', e); }
  });
}
