// Toolbar event handler wiring
import { state, $ } from './state.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo } from './toolbar.js';
import { doUndo, doRedo, closeSlashMenu } from './input.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText } from './render.js';
import { getSelectionInfo, setCursorAtOffset, setSelectionRange, getActiveNodeId } from './selection.js';
import { insertImage } from './images.js';
import { updatePageBreaks } from './pagination.js';
import { renderRuler } from './ruler.js';
import { getVersions, restoreVersion, saveVersion } from './file.js';
import { showShareDialog, broadcastOp } from './collab.js';

// E7.2: Screen reader announcement — briefly sets the aria-live region text
let _announceTimer = 0;
export function announce(msg) {
  const el = $('a11yLive');
  if (!el) return;
  clearTimeout(_announceTimer);
  el.textContent = msg;
  _announceTimer = setTimeout(() => { el.textContent = ''; }, 1000);
}

export function initToolbar() {
  // App menu bar (File/Edit/View/Insert/Format/Tools) dropdown behavior
  initAppMenubar();
  // Format toggles
  $('btnBold').addEventListener('click', () => { toggleFormat('bold'); announce('Bold toggled'); });
  $('btnItalic').addEventListener('click', () => { toggleFormat('italic'); announce('Italic toggled'); });
  $('btnUnderline').addEventListener('click', () => { toggleFormat('underline'); announce('Underline toggled'); });
  $('btnStrike').addEventListener('click', () => { toggleFormat('strikethrough'); announce('Strikethrough toggled'); });
  $('btnSuperscript').addEventListener('click', () => { toggleFormat('superscript'); announce('Superscript toggled'); });
  $('btnSubscript').addEventListener('click', () => { toggleFormat('subscript'); announce('Subscript toggled'); });

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

  // Style gallery dropdown
  initStyleGallery();

  // Text color
  $('colorPicker').addEventListener('input', e => {
    const hex = e.target.value.replace('#', '');
    $('colorSwatch').style.background = '#' + hex;
    applyFormat('color', hex);
  });

  // Highlight color
  $('highlightPicker').addEventListener('input', e => {
    const hex = e.target.value.replace('#', '');
    applyFormat('highlightColor', hex);
  });

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

  // Insert menu (position:fixed, so calculate position from button rect)
  $('btnInsertMenu').addEventListener('click', e => {
    e.stopPropagation();
    const menu = $('insertMenu');
    const wasOpen = menu.classList.contains('show');
    menu.classList.toggle('show');
    if (!wasOpen) {
      const rect = $('btnInsertMenu').getBoundingClientRect();
      menu.style.top = (rect.bottom + 4) + 'px';
      menu.style.left = rect.left + 'px';
    }
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
    ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus();
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
      broadcastOp({ action: 'insertTable', afterNodeId: nodeId, rows, cols });
      renderDocument();
      updateUndoRedo();
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
  });
  $('imageInput').addEventListener('change', e => {
    const f = e.target.files[0];
    if (f) insertImage(f);
    e.target.value = '';
  });

  // Insert hyperlink — modal
  $('miLink').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    state._linkSelInfo = info; // stash selection for after modal
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
    } catch (e) { console.error('insert page break:', e); }
  });

  // Insert comment — modal
  $('miComment').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    if (!state.doc) return;
    const info = getSelectionInfo();
    if (!info) return;
    state._commentSelInfo = info;
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
      state.doc.insert_comment(info.startNodeId, info.endNodeId, author, text);
      broadcastOp({ action: 'insertComment', startNodeId: info.startNodeId, endNodeId: info.endNodeId, author, text });
      renderDocument();
      updateUndoRedo();
      refreshComments();
    } catch (e) { console.error('insert comment:', e); }
  });
  $('commentModal').addEventListener('click', e => {
    if (e.target === $('commentModal')) { $('commentModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });
  $('commentText').addEventListener('keydown', e => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); $('commentInsertBtn').click(); }
    if (e.key === 'Escape') { $('commentModal').classList.remove('show'); ($('pageContainer')?.querySelector('.page-content') || $('pageContainer'))?.focus(); }
  });

  // Comments panel toggle
  $('btnComments').addEventListener('click', () => {
    $('commentsPanel').classList.toggle('show');
    if ($('commentsPanel').classList.contains('show')) refreshComments();
  });
  $('commentsClose').addEventListener('click', () => {
    $('commentsPanel').classList.remove('show');
  });

  // Find toolbar button
  $('btnFind').addEventListener('click', () => {
    $('findBar').classList.add('show');
    $('findInput').focus();
  });

  // Spell check toggle
  $('btnSpellCheck').addEventListener('click', () => {
    const page = $('pageContainer');
    const enabled = page.getAttribute('spellcheck') === 'true';
    page.setAttribute('spellcheck', enabled ? 'false' : 'true');
    const btn = $('btnSpellCheck');
    btn.classList.toggle('active', !enabled);
    btn.setAttribute('aria-pressed', String(!enabled));
  });

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
  if ($('menuUndo')) $('menuUndo').addEventListener('click', () => { closeAllMenus(); doUndo(); });
  if ($('menuRedo')) $('menuRedo').addEventListener('click', () => { closeAllMenus(); doRedo(); });
  if ($('menuFind')) $('menuFind').addEventListener('click', () => { closeAllMenus(); $('findBar').classList.add('show'); $('findInput').focus(); });
  if ($('menuZoomIn')) $('menuZoomIn').addEventListener('click', () => { closeAllMenus(); adjustZoom(10); });
  if ($('menuZoomOut')) $('menuZoomOut').addEventListener('click', () => { closeAllMenus(); adjustZoom(-10); });

  // Dark mode toggle
  if ($('menuDarkMode')) $('menuDarkMode').addEventListener('click', () => {
    closeAllMenus();
    toggleDarkMode();
  });

  // Print Preview (E10.3) — switch to Pages view then print
  if ($('menuPrintPreview')) $('menuPrintPreview').addEventListener('click', () => {
    closeAllMenus();
    if (!state.doc) return;
    // Dynamic import to avoid circular dependency
    import('./file.js').then(({ switchView }) => {
      switchView('pages');
      // Short delay to let pages render before opening print dialog
      setTimeout(() => window.print(), 300);
    });
  });

  // Help menu — keyboard shortcuts dialog (E7.4)
  if ($('menuShortcuts')) $('menuShortcuts').addEventListener('click', () => {
    closeAllMenus();
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
    alert('s1engine Editor v1.0\n\nA WASM-powered document editor built on the s1engine SDK.\nMIT License\n\nhttps://github.com/nicholasgasior/s1engine');
  });

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
      });
    });
  }

  // Table context menu + properties dialog
  initTableContextMenu();
  initTablePropsModal();

  // More (overflow) menu toggle and item handlers
  initMoreMenu();

  // Touch selection support (double-tap word select, long-press context menu)
  initTouchSelection();

  // Close menus on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('.insert-dropdown')) {
      $('insertMenu').classList.remove('show');
      $('btnInsertMenu').setAttribute('aria-expanded', 'false');
    }
    if (!e.target.closest('.style-gallery')) {
      $('styleGalleryPanel').classList.remove('show');
      $('styleGalleryBtn').setAttribute('aria-expanded', 'false');
    }
    if (!e.target.closest('.more-dropdown')) {
      closeMoreMenu();
    }
    $('tableContextMenu').style.display = 'none';
    // Close zoom dropdown on outside click
    if (!e.target.closest('.zoom-value-wrap')) {
      closeZoomDropdown();
    }
    // Close slash menu on outside click
    if (!e.target.closest('.slash-menu') && !e.target.closest('.doc-page')) {
      closeSlashMenu();
    }
  });

  // E7.1: Keyboard accessibility
  initModalFocusTrap();
  initToolbarKeyboardNav();
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
    paraIds.forEach(nodeId => {
      state.doc.set_list_format(nodeId, applyFormat, 0);
      broadcastOp({ action: 'setListFormat', nodeId, format: applyFormat, level: 0 });
    });
    renderDocument();
    updateToolbarState();
    updateUndoRedo();
  } catch (e) { console.error('list:', e); }
}

// Get all paragraph node IDs between selection start and end
function getSelectedParagraphIds(info) {
  const page = $('pageContainer');
  if (!page) return [info.startNodeId];
  const paraEls = page.querySelectorAll('p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]');
  const ids = [];
  let inRange = false;
  for (const el of paraEls) {
    const nid = el.dataset.nodeId;
    if (nid === info.startNodeId) inRange = true;
    if (inRange) ids.push(nid);
    if (nid === info.endNodeId) break;
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
export function setZoomLevel(level) {
  level = Math.max(50, Math.min(200, Math.round(level)));
  state.zoomLevel = level;
  const label = level + '%';
  if ($('zoomValue')) $('zoomValue').textContent = label;
  if ($('tbZoomValue')) $('tbZoomValue').textContent = label;
  // Apply zoom to each individual page for better scroll behavior
  const container = $('pageContainer');
  if (container) {
    container.querySelectorAll('.doc-page').forEach(pg => {
      pg.style.transform = `scale(${level / 100})`;
      pg.style.transformOrigin = 'top center';
    });
  }
  // Update active state in zoom dropdown
  const dd = $('zoomDropdown');
  if (dd) {
    dd.querySelectorAll('.zoom-preset').forEach(btn => {
      const v = btn.dataset.zoom;
      btn.classList.toggle('active', v === String(level));
    });
  }
  try { localStorage.setItem('folio-zoom', String(level)); } catch (_) {}
  renderRuler();
}

function calcFitWidthZoom() {
  const canvas = $('editorCanvas');
  if (!canvas) return 100;
  const pageWidth = 816; // default page width in px (8.5in @ 96dpi)
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
  const valueBtn = $('zoomValue');
  const dd = $('zoomDropdown');
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

  // Restore saved zoom level
  try {
    const saved = localStorage.getItem('folio-zoom');
    if (saved) {
      const parsed = parseInt(saved);
      if (!isNaN(parsed) && parsed >= 50 && parsed <= 200) {
        setZoomLevel(parsed);
      }
    }
  } catch (_) {}
}

function closeZoomDropdown() {
  const dd = $('zoomDropdown');
  const valueBtn = $('zoomValue');
  if (dd) dd.classList.remove('show');
  if (valueBtn) valueBtn.setAttribute('aria-expanded', 'false');
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
  localStorage.setItem('folio-theme', next);
  updateDarkModeIcon();
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
      const nextIdx = (idx + 1) % menuItems.length;
      openMenu(menuItems[nextIdx], true);
      menuBtns[nextIdx].focus();
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      const prevIdx = (idx - 1 + menuItems.length) % menuItems.length;
      openMenu(menuItems[prevIdx], true);
      menuBtns[prevIdx].focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      const btn = openItem.querySelector('.app-menu-btn');
      closeMenubar();
      if (btn) btn.focus();
    } else if (e.key === 'Enter' || e.key === ' ') {
      if (document.activeElement && entries.includes(document.activeElement)) {
        e.preventDefault();
        document.activeElement.click();
      }
    } else if (e.key === 'Home') {
      e.preventDefault();
      entries[0].focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      entries[entries.length - 1].focus();
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

    let html = '';
    (comments || []).forEach(c => {
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
          renderDocument();
          updateUndoRedo();
          refreshComments();
        } catch (e) { console.error('delete comment:', e); }
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
        refreshComments();
      });
    });
  } catch (e) {
    list.innerHTML = '<div class="comments-empty">Unable to load comments.</div>';
  }
}

function renderCommentCard(c) {
  const cid = c.id || '';
  return `
    <div class="comment-card" data-comment-id="${escapeAttr(cid)}">
      <div class="comment-author">${escapeHtml(c.author || 'Unknown')}</div>
      ${c.date ? `<div class="comment-date">${escapeHtml(c.date)}</div>` : ''}
      <div class="comment-text">${escapeHtml(c.text || c.body || '')}</div>
      <div class="comment-actions">
        <button class="comment-reply-btn" data-parent-id="${escapeAttr(cid)}">Reply</button>
        <button class="comment-delete" data-id="${escapeAttr(cid)}">Delete</button>
      </div>
    </div>`;
}

function renderReplyCard(r) {
  return `
    <div class="comment-card comment-reply" data-reply-id="${escapeAttr(r.id)}">
      <div class="comment-author">${escapeHtml(r.author || 'Unknown')}</div>
      <div class="comment-text">${escapeHtml(r.text)}</div>
      <div class="comment-actions">
        <button class="reply-delete" data-reply-id="${escapeAttr(r.id)}">Delete</button>
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
  input.focus();

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
  const reply = {
    id: 'reply-' + (++_replyCounter) + '-' + Date.now(),
    parentId,
    author,
    text: text.trim(),
    timestamp: Date.now(),
  };
  if (!state.commentReplies) state.commentReplies = [];
  state.commentReplies.push(reply);
  refreshComments();
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}
function escapeAttr(s) {
  return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
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
    list.innerHTML = versions.map((v, i) => `
      <div class="version-card" data-version-id="${v.id}">
        <div class="version-info">
          <div class="version-date">${escapeHtml(formatVersionDate(v.timestamp))}</div>
          <div class="version-meta">${v.wordCount.toLocaleString()} word${v.wordCount !== 1 ? 's' : ''}${v.label ? ' &middot; ' + escapeHtml(v.label) : ''}</div>
          ${i === 0 ? '<span class="version-badge">Current version</span>' : ''}
        </div>
        ${i > 0 ? '<div class="version-actions"><button class="version-restore" data-id="' + v.id + '">Restore</button></div>' : ''}
      </div>
    `).join('');
    list.querySelectorAll('.version-restore').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = parseInt(btn.dataset.id);
        if (!id || !state.engine) return;
        if (!confirm('Restore this version? Current unsaved changes will be lost.')) return;
        restoreVersion(id).then(() => {
          refreshHistory();
        }).catch(e => {
          alert('Failed to restore version: ' + e.message);
          console.error('restore version:', e);
        });
      });
    });
  });
}

// ── Style Gallery ─────────────────────────────────
// Style definitions: heading level + font/size/color for each style
const STYLE_DEFS = {
  normal:   { heading: 0, fontSize: null, fontFamily: null, color: null, italic: false },
  title:    { heading: 0, fontSize: '26', fontFamily: null, color: null, italic: false },
  subtitle: { heading: 0, fontSize: '15', fontFamily: null, color: '666666', italic: false },
  heading1: { heading: 1, fontSize: null, fontFamily: null, color: null, italic: false },
  heading2: { heading: 2, fontSize: null, fontFamily: null, color: null, italic: false },
  heading3: { heading: 3, fontSize: null, fontFamily: null, color: null, italic: false },
  heading4: { heading: 4, fontSize: null, fontFamily: null, color: null, italic: false },
  quote:    { heading: 0, fontSize: null, fontFamily: null, color: '666666', italic: true },
  code:     { heading: 0, fontSize: '11', fontFamily: 'Courier New', color: null, italic: false },
};

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
      panel.style.top = (rect.bottom + 4) + 'px';
      panel.style.left = rect.left + 'px';
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
        syncAllText();
        const paraIds = getSelectedParagraphIds(info);
        paraIds.forEach(nodeId => {
          // Set heading level on each paragraph
          state.doc.set_heading_level(nodeId, def.heading);
          broadcastOp({ action: 'setHeading', nodeId, level: def.heading });

          // Apply font formatting (whole paragraph)
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
    if (!state.doc || !state.ctxCell) return;
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
}

// ── E10.1: Dark Mode ─────────────────────────────
function initDarkMode() {
  const btn = $('btnDarkMode');
  if (!btn) return;

  // Restore saved preference
  const saved = localStorage.getItem('folio-theme');
  if (saved === 'dark' || saved === 'light') {
    document.documentElement.setAttribute('data-theme', saved);
  }
  updateDarkModeIcon();

  btn.addEventListener('click', () => toggleDarkMode());
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
const MODAL_IDS = ['tableModal', 'commentModal', 'linkModal', 'altTextModal', 'tablePropsModal'];
const FOCUSABLE_SELECTOR = 'button, [href], input:not([type=hidden]), select, textarea, [tabindex]:not([tabindex="-1"])';

function initModalFocusTrap() {
  // Track which element opened the modal so we can return focus
  const openerMap = new WeakMap();

  // Observe modal open/close via class changes
  const observer = new MutationObserver(mutations => {
    for (const m of mutations) {
      if (m.type !== 'attributes' || m.attributeName !== 'class') continue;
      const overlay = m.target;
      if (!overlay.classList.contains('modal-overlay')) continue;
      if (overlay.classList.contains('show')) {
        // Modal just opened — record opener and focus first element
        openerMap.set(overlay, document.activeElement);
        requestAnimationFrame(() => {
          const focusable = overlay.querySelectorAll(FOCUSABLE_SELECTOR);
          if (focusable.length > 0) focusable[0].focus();
        });
      } else {
        // Modal just closed — return focus to opener
        const opener = openerMap.get(overlay);
        if (opener && typeof opener.focus === 'function') {
          opener.focus();
        }
      }
    }
  });

  MODAL_IDS.forEach(id => {
    const el = $(id);
    if (el) observer.observe(el, { attributes: true, attributeFilter: ['class'] });
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
      menu.style.top = (rect.bottom + 4) + 'px';
      // Align right edge of menu to right edge of button
      menu.style.left = Math.max(0, rect.right - 220) + 'px';
      menu.classList.add('show');
      btn.setAttribute('aria-expanded', 'true');
    }
  });

  // Handle More menu item clicks — delegate to existing handlers
  menu.querySelectorAll('[data-more]').forEach(item => {
    item.addEventListener('click', () => {
      closeMoreMenu();
      const action = item.dataset.more;
      switch (action) {
        case 'lineSpacing': {
          // Cycle through common line spacings
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
        case 'find':
          $('findBar')?.classList.add('show');
          $('findInput')?.focus();
          break;
        case 'spellcheck':
          $('btnSpellCheck')?.click();
          break;
        case 'comments':
          $('commentsPanel')?.classList.toggle('show');
          break;
        case 'history':
          $('historyPanel')?.classList.toggle('show');
          break;
        case 'share':
          $('btnShare')?.click();
          break;
      }
    });
  });
}

function closeMoreMenu() {
  const menu = $('moreMenu');
  const btn = $('btnMore');
  if (menu) menu.classList.remove('show');
  if (btn) btn.setAttribute('aria-expanded', 'false');
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
