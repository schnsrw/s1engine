// Toolbar event handler wiring
import { state, $ } from './state.js';
import { toggleFormat, applyFormat, updateToolbarState, updateUndoRedo } from './toolbar.js';
import { doUndo, doRedo, closeSlashMenu } from './input.js';
import { renderDocument, renderNodeById, syncParagraphText, syncAllText, applyPageDimensions, isCanvasMode, setCanvasMode, initCanvasRenderer } from './render.js';
import { getSelectionInfo, setCursorAtOffset, setSelectionRange, getActiveNodeId, saveSelection } from './selection.js';
import { insertImage } from './images.js';
import { updatePageBreaks } from './pagination.js';
import { renderRuler } from './ruler.js';
import { getVersions, restoreVersion, saveVersion, openAutosaveDB, newDocument, updateDirtyIndicator, updateStatusBar } from './file.js';
import { showShareDialog, broadcastOp } from './collab.js';
import { trackEvent, getStats, clearStats, getSessionDuration } from './analytics.js';
import { getLastError, clearErrors } from './error-tracking.js';

// E7.2: Screen reader announcement — briefly sets the aria-live region text
let _announceTimer = 0;
export function announce(msg) {
  const el = $('a11yLive');
  if (!el) return;
  clearTimeout(_announceTimer);
  el.textContent = msg;
  _announceTimer = setTimeout(() => { el.textContent = ''; }, 1000);
}

function restoreSelectionForPickers() {
  const info = state.lastSelInfo;
  if (!info || info.collapsed) return false;
  const page = $('pageContainer');
  if (!page) return false;
  const startEl = page.querySelector(`[data-node-id="${info.startNodeId}"]`);
  const endEl = page.querySelector(`[data-node-id="${info.endNodeId}"]`);
  if (!startEl || !endEl) return false;
  setSelectionRange(startEl, info.startOffset, endEl, info.endOffset);
  return true;
}

// ── Toast notification system ──────────────────────
// Replaces alert() calls with non-blocking toast messages.
// Types: 'info' (default, dark), 'error' (red), 'success' (green)
export function showToast(message, type = 'info', duration = 4000) {
  const container = $('toastContainer');
  if (!container) { console.warn('toast:', message); return; }
  const toast = document.createElement('div');
  toast.className = 'toast' + (type === 'error' ? ' toast-error' : type === 'success' ? ' toast-success' : '');
  toast.textContent = message;
  container.appendChild(toast);
  const remove = () => {
    toast.style.transition = 'opacity 0.2s ease, transform 0.2s ease';
    toast.style.opacity = '0';
    toast.style.transform = 'translateY(-8px)';
    setTimeout(() => { toast.remove(); }, 220);
  };
  toast.addEventListener('click', remove);
  if (duration > 0) setTimeout(remove, duration);
}

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

  // Text color
  const colorPicker = $('colorPicker');
  if (colorPicker) {
    colorPicker.addEventListener('pointerdown', () => saveSelection());
    colorPicker.addEventListener('input', e => {
      const hex = e.target.value.replace('#', '');
      $('colorSwatch').style.background = '#' + hex;
      restoreSelectionForPickers();
      applyFormat('color', hex);
    });
  }

  // Highlight color
  const highlightPicker = $('highlightPicker');
  if (highlightPicker) {
    highlightPicker.addEventListener('pointerdown', () => saveSelection());
    highlightPicker.addEventListener('input', e => {
      const hex = e.target.value.replace('#', '');
      restoreSelectionForPickers();
      applyFormat('highlightColor', hex);
    });
  }

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
    const nodeId = getActiveNodeId();
    if (!nodeId) return;
    syncAllText();
    try {
      state.doc.insert_table(nodeId, rows, cols);
      broadcastOp({ action: 'insertTable', afterNodeId: nodeId, rows, cols });
      renderDocument();
      updateUndoRedo();
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

  // Insert comment — modal
  $('miComment').addEventListener('click', () => {
    $('insertMenu').classList.remove('show');
    trackEvent('insert', 'comment');
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
  $('miHeaderFooter').addEventListener('click', () => {
    closeAllMenus();
    openHeaderFooterModal();
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
      state.docHeaderHtml = '<span style="display:block;text-align:center;color:#5f6368;font-size:9pt">' + escapeHtml(headerText) + '</span>';
    } else {
      state.docHeaderHtml = '';
    }

    // Build footer HTML
    let footerParts = [];
    if (footerText) {
      footerParts.push(escapeHtml(footerText));
    }
    if (showPageNum) {
      footerParts.push('<span data-field="PageNumber"></span>');
    }
    if (footerParts.length > 0) {
      state.docFooterHtml = '<span style="display:block;text-align:center;color:#5f6368;font-size:9pt">' + footerParts.join(' \u2014 ') + '</span>';
    } else {
      state.docFooterHtml = '';
    }

    state.hasDifferentFirstPage = differentFirst;
    if (differentFirst) {
      // First page gets no header/footer when "different first page" is checked
      state.docFirstPageHeaderHtml = '';
      state.docFirstPageFooterHtml = '';
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

  // View → Pages Panel toggle
  if ($('menuShowPages')) $('menuShowPages').addEventListener('click', () => {
    closeAllMenus();
    togglePagesPanel();
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
    showToast('s1engine Editor v1.0 — WASM-powered document editor. MIT License.', 'info', 6000);
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
        else if (action === 'headerfooter') $('miHeaderFooter').click();
        else if (action === 'drawing') $('miDrawing')?.click();
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

  // Touch selection support (double-tap word select, long-press context menu)
  initTouchSelection();

  // E9.3: Equation editor
  initEquationModal();

  // E9.1: Custom dictionary & auto-correct
  initDictModal();
  initAutoCorrectModal();

  // E5.4: Editing mode selector
  initEditingMode();

  // E9.2: Save as template
  initSaveAsTemplate();

  // E5.4: @mention in comments
  initCommentMentions();

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
export function setZoomLevel(level) {
  level = Math.max(50, Math.min(200, Math.round(level)));
  state.zoomLevel = level;
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
  // Update active state in zoom dropdown
  const dd = $('zoomDropdown');
  if (dd) {
    dd.querySelectorAll('.zoom-preset').forEach(btn => {
      const v = btn.dataset.zoom;
      btn.classList.toggle('active', v === String(level));
    });
  }
  try { localStorage.setItem('s1-zoom', String(level)); } catch (_) {}
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
    const saved = localStorage.getItem('s1-zoom');
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
  localStorage.setItem('s1-theme', next);
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
        if (state.resolvedComments.has(id)) {
          state.resolvedComments.delete(id);
        } else {
          state.resolvedComments.add(id);
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

function renderCommentCard(c) {
  const cid = c.id || '';
  const isResolved = state.resolvedComments && state.resolvedComments.has(cid);
  const resolvedClass = isResolved ? ' comment-resolved' : '';
  const resolveLabel = isResolved ? 'Unresolve' : 'Resolve';
  const startNodeId = c.start_node_id || c.startNodeId || '';
  return `
    <div class="comment-card${resolvedClass}" data-comment-id="${escapeAttr(cid)}" data-start-node-id="${escapeAttr(startNodeId)}" style="cursor:pointer">
      <div class="comment-author">${escapeHtml(c.author || 'Unknown')}</div>
      ${c.date ? `<div class="comment-date">${escapeHtml(c.date)}</div>` : ''}
      <div class="comment-text">${escapeHtml(c.text || c.body || '')}</div>
      <div class="comment-actions">
        <button class="comment-reply-btn" data-parent-id="${escapeAttr(cid)}" title="Reply to this comment">Reply</button>
        <button class="comment-resolve-btn" data-id="${escapeAttr(cid)}" title="${resolveLabel} this comment">${resolveLabel}</button>
        <button class="comment-delete" data-id="${escapeAttr(cid)}" title="Delete this comment">Delete</button>
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

  $('headerFooterModal').classList.add('show');
  $('headerText').focus();
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
      const diffStr = diff > 0 ? `<span style="color:#34a853">+${diff}</span>` : diff < 0 ? `<span style="color:#ea4335">${diff}</span>` : '';
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
      btn.addEventListener('click', () => {
        const id = parseInt(btn.dataset.id);
        if (!id || !state.engine) return;
        if (!confirm('Restore this version? Current unsaved changes will be lost.')) return;
        restoreVersion(id).then(() => {
          refreshHistory();
        }).catch(e => {
          showToast('Failed to restore version: ' + e.message, 'error');
          console.error('restore version:', e);
        });
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
          <span style="color:#34a853">+${added} added</span>,
          <span style="color:#ea4335">-${removed} removed</span>,
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
  const saved = localStorage.getItem('s1-theme');
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
const MODAL_IDS = ['tableModal', 'commentModal', 'linkModal', 'altTextModal', 'tablePropsModal', 'headerFooterModal', 'templateModal', 'pageSetupModal', 'equationModal', 'dictModal', 'autoCorrectModal'];
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
      menu.style.top = (rect.bottom + 4) + 'px';
      // Align right edge of menu to right edge of button
      menu.style.left = Math.max(0, rect.right - 220) + 'px';
      menu.classList.add('show');
      btn.setAttribute('aria-expanded', 'true');
    }
  });

  // Wire up color pickers inside the More menu
  const moreColorPicker = $('moreColorPicker');
  if (moreColorPicker) {
    moreColorPicker.addEventListener('input', e => {
      const hex = e.target.value.replace('#', '');
      $('colorSwatch').style.background = '#' + hex;
      applyFormat('color', hex);
    });
    moreColorPicker.addEventListener('change', () => {
      closeMoreMenu();
    });
  }
  const moreHighlightPicker = $('moreHighlightPicker');
  if (moreHighlightPicker) {
    moreHighlightPicker.addEventListener('input', e => {
      const hex = e.target.value.replace('#', '');
      applyFormat('highlightColor', hex);
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
  // Populate with current values from state.pageDims or defaults
  const dims = state.pageDims || {
    widthPt: 612, heightPt: 792,
    marginTopPt: 72, marginBottomPt: 72, marginLeftPt: 72, marginRightPt: 72,
  };

  const wIn = dims.widthPt / 72;
  const hIn = dims.heightPt / 72;

  // Determine orientation
  const isLandscape = wIn > hIn;
  document.querySelectorAll('input[name="psOrientation"]').forEach(r => {
    r.checked = (r.value === (isLandscape ? 'landscape' : 'portrait'));
  });

  // Detect page size from dimensions (compare short/long sides to presets)
  const shortSide = Math.min(wIn, hIn);
  const longSide = Math.max(wIn, hIn);
  let detectedSize = 'letter';
  for (const [key, sz] of Object.entries(PAGE_SIZES)) {
    const preShort = Math.min(sz.w, sz.h);
    const preLong = Math.max(sz.w, sz.h);
    if (Math.abs(preShort - shortSide) < 0.1 && Math.abs(preLong - longSide) < 0.1) {
      detectedSize = key;
      break;
    }
  }
  $('psPageSize').value = detectedSize;

  // Margins
  $('psMarginTop').value = (dims.marginTopPt / 72).toFixed(2);
  $('psMarginBottom').value = (dims.marginBottomPt / 72).toFixed(2);
  $('psMarginLeft').value = (dims.marginLeftPt / 72).toFixed(2);
  $('psMarginRight').value = (dims.marginRightPt / 72).toFixed(2);

  $('pageSetupModal').classList.add('show');
}

function applyPageSetup() {
  const sizeKey = $('psPageSize').value || 'letter';
  const size = PAGE_SIZES[sizeKey] || PAGE_SIZES.letter;
  const orientation = document.querySelector('input[name="psOrientation"]:checked')?.value || 'portrait';

  let pageW = size.w;
  let pageH = size.h;
  if (orientation === 'landscape') {
    pageW = size.h;
    pageH = size.w;
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

  // Re-render with new dimensions
  renderDocument();
  renderRuler();
  announce('Page setup applied');
}

// ═══════════════════════════════════════════════════
// E9.3: Equation Editor
// ═══════════════════════════════════════════════════

let _eqPreviewTimer = 0;

export function openEquationModal(prefillLatex) {
  closeAllMenus();
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

export function getAutoCorrectMap() {
  try {
    const raw = localStorage.getItem(AC_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return { ...DEFAULT_AUTOCORRECT };
}

function saveAutoCorrectMap(map) {
  try { localStorage.setItem(AC_STORAGE_KEY, JSON.stringify(map)); } catch (_) {}
}

export function isAutoCorrectEnabled() {
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
// E5.4 — Editing Mode (Editing / Suggesting / Viewing)
// ═══════════════════════════════════════════════════════

function initEditingMode() {
  const sel = $('editingModeSelect');
  if (!sel) return;

  sel.addEventListener('change', () => {
    const mode = sel.value;
    state.editingMode = mode;
    applyEditingMode(mode);
    announce('Mode: ' + mode.charAt(0).toUpperCase() + mode.slice(1));
  });
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

  btn.addEventListener('click', () => {
    closeAllMenus();
    if (!state.doc) {
      showToast('No document open to save as template.', 'error');
      return;
    }

    // Prompt for template name
    const name = prompt('Template name:');
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
    // Render a tiny preview from saved HTML
    const previewInner = document.createElement('div');
    previewInner.className = 'template-preview-content';
    previewInner.innerHTML = tpl.html || '';
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
  if (tpl.html && typeof state.doc.import_html === 'function') {
    try {
      state.doc.import_html(tpl.html);
    } catch (_) {
      const tmp = document.createElement('div');
      tmp.innerHTML = tpl.html;
      state.doc.append_paragraph(tmp.textContent || '');
    }
  } else {
    const tmp = document.createElement('div');
    tmp.innerHTML = tpl.html || '';
    state.doc.append_paragraph(tmp.textContent || '');
  }

  state.doc.clear_history();

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

// ── Pages Panel — Thumbnail sidebar ─────────────────────────────
function togglePagesPanel() {
  const panel = $('pagesPanel');
  if (!panel) return;
  panel.classList.toggle('show');
  if (panel.classList.contains('show')) renderPageThumbnails();
}

/** Render mini page thumbnails in the pages sidebar. */
function renderPageThumbnails() {
  const list = $('pagesList');
  const pageContainer = $('pageContainer');
  if (!list || !pageContainer) return;

  const pages = pageContainer.querySelectorAll('.s1-page');
  if (!pages.length) {
    list.innerHTML = '<div style="text-align:center;padding:20px;color:var(--text-muted);font-size:12px">No pages to display</div>';
    return;
  }

  list.innerHTML = '';
  const THUMB_WIDTH = 148; // px width for thumbnails

  pages.forEach((page, i) => {
    const thumb = document.createElement('div');
    thumb.className = 'page-thumb';
    thumb.title = `Page ${i + 1}`;
    thumb.dataset.pageIndex = i;

    // Create a scaled-down canvas snapshot of the page
    const canvas = document.createElement('canvas');
    canvas.className = 'page-thumb-canvas';

    // Get page dimensions from the s1-page style
    const pageW = parseFloat(page.style.width) || 612;
    const pageH = parseFloat(page.style.height) || 792;
    const scale = THUMB_WIDTH / pageW;
    const thumbH = pageH * scale;

    canvas.width = THUMB_WIDTH * 2; // retina
    canvas.height = thumbH * 2;
    canvas.style.width = THUMB_WIDTH + 'px';
    canvas.style.height = Math.round(thumbH) + 'px';

    // Draw a white background then rasterize the page via html2canvas-lite approach
    const ctx = canvas.getContext('2d');
    ctx.scale(2, 2);
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, THUMB_WIDTH, thumbH);

    // Simple rasterization: draw text content as gray lines (fast, no external lib)
    drawMiniPage(ctx, page, THUMB_WIDTH, thumbH, scale);

    thumb.appendChild(canvas);

    // Page number label
    const label = document.createElement('div');
    label.className = 'page-thumb-label';
    label.textContent = i + 1;
    thumb.appendChild(label);

    // Click to scroll to page
    thumb.addEventListener('click', () => {
      page.scrollIntoView({ behavior: 'smooth', block: 'start' });
      // Highlight active
      list.querySelectorAll('.page-thumb').forEach(t => t.classList.remove('active'));
      thumb.classList.add('active');
    });

    list.appendChild(thumb);
  });

  // Mark first page as active
  const first = list.querySelector('.page-thumb');
  if (first) first.classList.add('active');

  // Update active thumb on scroll
  setupPageScrollTracking(pageContainer, list);
}

/** Draw a simplified mini representation of a page for the thumbnail. */
function drawMiniPage(ctx, page, thumbW, thumbH, scale) {
  // Draw blocks as simplified gray rectangles / text lines
  const blocks = page.querySelectorAll('.s1-block');
  ctx.fillStyle = '#666';

  blocks.forEach(block => {
    const style = block.style;
    const bx = (parseFloat(style.left) || 0) * scale;
    const by = (parseFloat(style.top) || 0) * scale;
    const bw = (parseFloat(style.width) || 100) * scale;

    // Draw each line of text as a thin gray bar
    const text = block.textContent || '';
    if (!text.trim()) return;

    const lines = block.querySelectorAll('span, div');
    if (lines.length > 0) {
      lines.forEach(line => {
        const lt = line.textContent || '';
        if (!lt.trim()) return;
        const lStyle = line.style;
        const ly = (parseFloat(lStyle.top) || 0) * scale;
        const lw = Math.min(lt.length * 1.2 * scale, bw);
        const lh = Math.max(2, 3 * scale);
        ctx.globalAlpha = 0.35;
        ctx.fillRect(bx, by + ly, lw, lh);
      });
    } else {
      // No child spans — draw a single bar for the block text
      const lh = Math.max(2, 3 * scale);
      const lw = Math.min(text.length * 1.2 * scale, bw);
      ctx.globalAlpha = 0.35;
      ctx.fillRect(bx, by, lw, lh);
    }
  });

  ctx.globalAlpha = 1.0;

  // Draw images as light blue rectangles
  const images = page.querySelectorAll('img');
  images.forEach(img => {
    const parent = img.closest('.s1-block');
    if (!parent) return;
    const ps = parent.style;
    const ix = (parseFloat(ps.left) || 0) * scale;
    const iy = (parseFloat(ps.top) || 0) * scale;
    const iw = (parseFloat(ps.width) || 50) * scale;
    const ih = (parseFloat(img.style.height || img.height) || 50) * scale;
    ctx.fillStyle = '#c8ddf0';
    ctx.fillRect(ix, iy, iw, ih);
    ctx.strokeStyle = '#8ab4d6';
    ctx.lineWidth = 0.5;
    ctx.strokeRect(ix, iy, iw, ih);
    ctx.fillStyle = '#666';
  });

  // Draw tables as grid outlines
  const tables = page.querySelectorAll('.s1-table');
  tables.forEach(table => {
    const ts = table.style;
    const tx = (parseFloat(ts.left) || 0) * scale;
    const ty = (parseFloat(ts.top) || 0) * scale;
    const tw = (parseFloat(ts.width) || 200) * scale;
    const th = (parseFloat(ts.height) || 50) * scale;
    ctx.strokeStyle = '#aaa';
    ctx.lineWidth = 0.5;
    ctx.strokeRect(tx, ty, tw, th);
  });
}

/** Track scroll position to highlight the active page thumbnail. */
function setupPageScrollTracking(pageContainer, thumbList) {
  const canvas = $('editorCanvas');
  if (!canvas) return;

  let ticking = false;
  canvas.addEventListener('scroll', () => {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      ticking = false;
      const pages = pageContainer.querySelectorAll('.s1-page');
      const scrollTop = canvas.scrollTop;
      const viewMid = scrollTop + canvas.clientHeight / 3;

      let activeIdx = 0;
      pages.forEach((page, i) => {
        if (page.offsetTop <= viewMid) activeIdx = i;
      });

      thumbList.querySelectorAll('.page-thumb').forEach((t, i) => {
        t.classList.toggle('active', i === activeIdx);
        if (i === activeIdx) {
          // Ensure active thumb is visible in sidebar
          t.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        }
      });
    });
  });
}

/** Re-render page thumbnails after document changes (called from render.js). */
export function refreshPageThumbnails() {
  const panel = $('pagesPanel');
  if (panel && panel.classList.contains('show')) {
    renderPageThumbnails();
  }
}
