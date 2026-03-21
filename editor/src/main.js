// Rudra Office — Entry Point
// Wires all modules together and initializes the WASM engine.

import './styles.css';
import './spreadsheet.css';
import { state, $ } from './state.js';
import { initInput, initPinchToZoom, initTableCellAnnouncements } from './input.js';
import { initFileHandlers, newDocument, openFile, setDetectFormat, checkAutoRecover, updatePdfStatusBar } from './file.js';
import { initToolbar } from './toolbar-handlers.js';
import { initFind } from './find.js';
import { renderRuler } from './ruler.js';
import { checkAutoJoin, initCollabUI } from './collab.js';
import { initImageContextMenu } from './images.js';
import { initTouch } from './touch.js';
import { trackEvent } from './analytics.js';
import { recordError } from './error-tracking.js';
import { initShapes } from './shapes.js';
import { initPropertiesPanel } from './properties-panel.js';
import { initFonts, ensureDocumentFonts } from './fonts.js';
import { initAIPanel } from './ai-panel.js';
import { initAIInline } from './ai-inline.js';
import { initTabs } from './tabs.js';

// ── Service Worker Registration ──────────────────────
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {});
}

// ── Global Error Handlers ────────────────────────────
window.addEventListener('error', (e) => {
  console.error('[s1-editor] Uncaught error:', e.error);
  trackEvent('error', 'uncaught');
  recordError(e.error || e.message);
});

window.addEventListener('unhandledrejection', (e) => {
  console.error('[s1-editor] Unhandled promise rejection:', e.reason);
  trackEvent('error', 'unhandled-rejection');
  recordError(e.reason);
});

/**
 * Show a custom modal for auto-recovery instead of browser confirm().
 * Returns a Promise that resolves to true (recover) or false (discard).
 */
function showRecoveryModal(description) {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay show';
    const modal = document.createElement('div');
    modal.className = 'modal';
    modal.innerHTML = '<h3>Recover Unsaved Document?</h3>' +
      '<p style="color:#5f6368;font-size:13px;margin:8px 0 16px;">' + description + '</p>' +
      '<div class="modal-actions">' +
      '<button class="modal-cancel">Discard</button>' +
      '<button class="modal-ok primary">Recover</button></div>';
    overlay.appendChild(modal);
    document.body.appendChild(overlay);
    modal.querySelector('.modal-cancel').onclick = () => { document.body.removeChild(overlay); resolve(false); };
    modal.querySelector('.modal-ok').onclick = () => { document.body.removeChild(overlay); resolve(true); };
    modal.querySelector('.modal-ok').focus();
  });
}

async function boot() {
  const dot = $('wasmDot');
  const label = $('wasmLabel');

  // U1: Disable toolbar buttons until WASM is ready
  const toolbar = $('toolbar');
  if (toolbar) toolbar.style.pointerEvents = 'none';
  label.textContent = 'Loading engine...';

  // U6: Disable spellcheck on document name input
  const docName = $('docName');
  if (docName) {
    docName.spellcheck = false;
    docName.autocomplete = 'off';
  }

  // Timeout to update loading message if WASM takes too long
  const wasmTimeout = setTimeout(() => {
    if (label && label.textContent.includes('Loading')) {
      label.textContent = 'Still loading... check your connection';
    }
  }, 8000);

  try {
    // Import WASM bindings from wasm-pkg directory
    const wasm = await import('../wasm-pkg/s1engine_wasm.js');
    await wasm.default();  // init wasm module

    clearTimeout(wasmTimeout);
    state.engine = new wasm.WasmEngine();
    setDetectFormat(wasm.detect_format);

    // Initialize font database and preload common fonts
    try {
      await initFonts(wasm);
    } catch (e) {
      console.warn('[fonts] Preload failed, using system fonts:', e);
    }

    dot.classList.add('ok');
    label.textContent = 's1engine ready';

    // Wire up all handlers FIRST
    initInput();
    initFileHandlers();
    initToolbar();
    initFind();
    initImageContextMenu();
    initCollabUI();
    initTouch();
    initPinchToZoom();
    initTableCellAnnouncements();
    initShapes();
    initPropertiesPanel();
    initAIPanel();
    initAIInline();
    initTabs();
    renderRuler();
    initPdfToolbar();
    initSpreadsheetToolbar();

    // THEN re-enable toolbar (after handlers are ready)
    if (toolbar) toolbar.style.pointerEvents = '';

    // Expose state for testing
    window.__s1_state = state;

    // Check for collaboration auto-join (?file=... or ?room=... URL param)
    // If a shared file is opened, skip auto-recovery
    const params = new URLSearchParams(window.location.search);
    const isSharedLink = params.has('file') || params.has('room');
    await checkAutoJoin();

    // Check for auto-recovered document (NEVER on shared links)
    if (!isSharedLink) try {
      const saved = await checkAutoRecover();
      if (saved && saved.bytes) {
        // N2: Skip recovery for docs with no timestamp or older than 7 days
        if (!saved.timestamp || saved.timestamp < Date.now() - 86400000 * 7) {
          // No valid timestamp or too old — discard
        } else {
          const age = Date.now() - saved.timestamp;
          // Only offer recovery for documents saved within the last 24 hours
          if (age < 86400000) {
            const name = saved.name || 'Untitled Document';
            const mins = Math.round(age / 60000);
            const timeStr = mins < 1 ? 'just now' : mins < 60 ? `${mins}m ago` : `${Math.round(mins / 60)}h ago`;
            // Check checksum integrity + byte length consistency
            const integrityOk = saved._checksumValid !== false && (!saved.byteLength || saved.byteLength === saved.bytes.byteLength);
            if (!integrityOk) {
              console.warn('Auto-recover skipped: checksum mismatch for', name);
              // Don't offer corrupted files
            } else {
              const recover = await showRecoveryModal(`"${name}" (saved ${timeStr})`);
              if (recover) {
                openFile(new Uint8Array(saved.bytes), name + '.docx');
                // Restore comment thread replies if they were persisted
                if (saved.commentReplies) {
                  try { state.commentReplies = JSON.parse(saved.commentReplies); } catch (_) {}
                }
              }
            }
          }
        }
      }
    } catch (_) {}

  } catch (e) {
    clearTimeout(wasmTimeout);
    console.error('WASM init failed:', e);
    dot.classList.add('err');
    label.textContent = 'Engine failed — click to retry';
    label.style.cursor = 'pointer';
    label.title = e.message;
    label.onclick = () => {
      label.onclick = null;
      label.style.cursor = '';
      dot.classList.remove('err');
      label.textContent = 'Retrying...';
      boot(); // Retry the entire boot sequence
    };
    // Keep toolbar disabled on failure
    if (toolbar) toolbar.style.pointerEvents = 'none';
  }
}

function initPdfToolbar() {
  // Page navigation
  $('pdfPrevPage')?.addEventListener('click', () => {
    if (state.pdfViewer) { state.pdfViewer.prevPage(); updatePdfStatusBar(); }
  });
  $('pdfNextPage')?.addEventListener('click', () => {
    if (state.pdfViewer) { state.pdfViewer.nextPage(); updatePdfStatusBar(); }
  });

  // Zoom controls
  $('pdfZoomOut')?.addEventListener('click', () => {
    if (!state.pdfViewer) return;
    const sel = $('pdfZoomSelect');
    const scales = [0.5, 0.75, 1, 1.25, 1.5, 2];
    const current = state.pdfZoom;
    for (let i = scales.length - 1; i >= 0; i--) {
      if (scales[i] < current - 0.01) {
        state.pdfViewer.setZoom(scales[i]);
        sel.value = String(scales[i]);
        return;
      }
    }
  });
  $('pdfZoomIn')?.addEventListener('click', () => {
    if (!state.pdfViewer) return;
    const sel = $('pdfZoomSelect');
    const scales = [0.5, 0.75, 1, 1.25, 1.5, 2];
    const current = state.pdfZoom;
    for (const s of scales) {
      if (s > current + 0.01) {
        state.pdfViewer.setZoom(s);
        sel.value = String(s);
        return;
      }
    }
  });
  $('pdfZoomSelect')?.addEventListener('change', (e) => {
    if (!state.pdfViewer) return;
    const val = parseFloat(e.target.value);
    if (!isNaN(val) && val >= 0.25 && val <= 4.0) {
      state.pdfViewer.setZoom(val);
    } else {
      e.target.value = String(state.pdfZoom);
    }
  });

  // Tool selection
  const validPdfTools = ['select', 'highlight', 'comment', 'draw', 'text', 'redact'];
  document.querySelectorAll('.pdf-tool-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const toolName = btn.dataset.tool;
      if (!toolName) return;
      if (!validPdfTools.includes(toolName)) return;
      document.querySelectorAll('.pdf-tool-btn').forEach(b => {
        b.classList.remove('active');
        b.setAttribute('aria-pressed', 'false');
      });
      btn.classList.add('active');
      btn.setAttribute('aria-pressed', 'true');
      state.pdfTool = toolName;

      // Update cursor style on canvas container
      const container = $('pdfCanvasContainer');
      if (container) {
        container.dataset.tool = btn.dataset.tool;
      }
    });
  });

  // Download PDF button — bake annotations into PDF via WASM editor, then download
  $('pdfSave')?.addEventListener('click', async () => {
    if (!state.pdfBytes) return;
    try {
      let outputBytes = state.pdfBytes;

      // If there are annotations or text edits, bake them into the PDF
      if (state.pdfAnnotations.length > 0 || state.pdfTextEdits.length > 0) {
        const wasm = await import('../wasm-pkg/s1engine_wasm.js');
        const editor = wasm.WasmPdfEditor.open(state.pdfBytes);

        // Apply annotations
        for (const ann of state.pdfAnnotations) {
          const page = ann.pageNum - 1; // 0-indexed for WASM
          try {
            switch (ann.type) {
              case 'highlight':
                if (ann.props.quads?.length) {
                  // Flatten quads to [x1,y1,x2,y2,...] for WASM
                  const quads = [];
                  for (const q of ann.props.quads) {
                    quads.push(q.x, q.y, q.x + q.width, q.y, q.x, q.y + q.height, q.x + q.width, q.y + q.height);
                  }
                  editor.add_highlight_annotation(page, new Float64Array(quads), 1.0, 0.92, 0.23, ann.author || 'User', ann.props.selectedText || '');
                }
                break;
              case 'comment':
                editor.add_text_annotation(page, ann.props.x, ann.props.y, ann.author || 'User', ann.props.content || '');
                break;
              case 'ink':
                if (ann.props.paths?.[0]?.length) {
                  const pts = [];
                  for (const p of ann.props.paths[0]) { pts.push(p.x, p.y); }
                  editor.add_ink_annotation(page, new Float64Array(pts), 0.85, 0.07, 0.14, ann.props.strokeWidth || 2);
                }
                break;
              case 'text':
                editor.add_freetext_annotation(page, ann.props.x, ann.props.y, ann.props.width || 100, ann.props.height || 20, ann.props.content || '', ann.props.fontSize || 12);
                break;
              case 'redact':
                for (const r of (ann.props.rects || [])) {
                  editor.add_redaction(page, r.x, r.y, r.width, r.height);
                }
                break;
              case 'stamp':
                // Signatures/stamps — add as freetext annotation with "[Signature]" marker
                editor.add_freetext_annotation(page, ann.props.x, ann.props.y, ann.props.width || 150, ann.props.height || 60, '[Signature]', 12);
                break;
            }
          } catch (err) {
            console.warn(`Failed to write ${ann.type} annotation:`, err);
          }
        }

        // Apply text edits (overlay approach)
        for (const edit of state.pdfTextEdits) {
          try {
            const page = edit.pageNum - 1;
            const p = edit.position;
            editor.add_white_rect(page, p.x, p.y, p.width, p.height);
            editor.add_text_overlay(page, p.x, p.y, p.width, p.height, edit.newText, edit.fontInfo?.size || 12);
          } catch (err) {
            console.warn('Failed to write text edit:', err);
          }
        }

        outputBytes = editor.save();
        editor.free();
      }

      // Download
      const blob = new Blob([outputBytes], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = ($('docName').value || 'document') + '.pdf';
      document.body.appendChild(a);
      a.click();
      setTimeout(() => { try { document.body.removeChild(a); } catch(_) {} URL.revokeObjectURL(url); }, 200);
      state.pdfModified = false;

      const { showToast } = await import('./toolbar-handlers.js');
      showToast('PDF saved with annotations');
    } catch (err) {
      console.error('PDF save error:', err);
      // Fallback: download original without annotations
      const blob = new Blob([state.pdfBytes], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = ($('docName').value || 'document') + '.pdf';
      document.body.appendChild(a);
      a.click();
      setTimeout(() => { try { document.body.removeChild(a); } catch(_) {} URL.revokeObjectURL(url); }, 200);

      const { showToast } = await import('./toolbar-handlers.js');
      showToast('Saved without annotations (WASM editor unavailable)', 'error');
    }
  });

  // Signature button — opens signature creation modal
  $('pdfToolSignature')?.addEventListener('click', async () => {
    try {
      const { openSignatureModal } = await import('./pdf-signatures.js');
      openSignatureModal();
    } catch (err) {
      console.error('Signature module error:', err);
    }
  });

  // Annotations panel close
  $('pdfAnnotClose')?.addEventListener('click', () => {
    $('pdfAnnotationsPanel')?.classList.remove('show');
  });

  // Keyboard shortcuts for PDF tools (only active in PDF view)
  document.addEventListener('keydown', (e) => {
    if (state.currentView !== 'pdf') return;
    // Don't intercept if user is typing in an input/textarea
    const tag = e.target.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || e.target.isContentEditable) return;

    const toolMap = { v: 'select', h: 'highlight', c: 'comment', d: 'draw', t: 'text', r: 'redact' };
    const key = e.key.toLowerCase();

    if (toolMap[key] && !e.ctrlKey && !e.metaKey && !e.altKey) {
      const toolName = toolMap[key];
      const btn = document.querySelector(`.pdf-tool-btn[data-tool="${toolName}"]`);
      if (btn) btn.click();
      return;
    }

    // Ctrl+S / Cmd+S = download PDF
    if ((e.ctrlKey || e.metaKey) && key === 's') {
      e.preventDefault();
      $('pdfSave')?.click();
    }
  });
}

/**
 * Custom modal prompt for spreadsheet comments (replaces window.prompt).
 */
function _ssCommentPrompt(message, defaultValue) {
  return new Promise(resolve => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay show';
    const modal = document.createElement('div');
    modal.className = 'modal';
    const h3 = document.createElement('h3');
    h3.textContent = message;
    modal.appendChild(h3);
    const input = document.createElement('textarea');
    input.value = defaultValue || '';
    input.rows = 3;
    input.style.cssText = 'width:100%;padding:8px;margin:8px 0 16px;border:1px solid #dadce0;border-radius:4px;font-size:14px;box-sizing:border-box;resize:vertical;';
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
      if (e.key === 'Escape') close(null);
    });
    input.focus();
  });
}

function initSpreadsheetToolbar() {
  $('ssUndo')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.undo();
  });
  $('ssRedo')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.redo();
  });
  $('ssCut')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.cutCells();
  });
  $('ssCopy')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.copyCells();
  });
  $('ssPaste')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.pasteCells();
  });
  $('ssSortAsc')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.sort(state.spreadsheetView.selectedCell.col, true);
  });
  $('ssSortDesc')?.addEventListener('click', () => {
    if (state.spreadsheetView) state.spreadsheetView.sort(state.spreadsheetView.selectedCell.col, false);
  });
  $('ssFilter')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    const col = state.spreadsheetView.selectedCell.col;
    if (state.spreadsheetView.filterState[col]) {
      state.spreadsheetView.removeFilter(col);
    } else {
      state.spreadsheetView.addFilter(col);
    }
  });
  $('ssFreeze')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    const { col, row } = state.spreadsheetView.selectedCell;
    // Toggle: if already frozen at this position, unfreeze
    if (state.spreadsheetView.frozenCols === col && state.spreadsheetView.frozenRows === row) {
      state.spreadsheetView.freezePanes(0, 0);
    } else {
      state.spreadsheetView.freezePanes(col, row);
    }
  });
  $('ssExportCSV')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    const filename = $('docName')?.value || 'spreadsheet';
    state.spreadsheetView.downloadCSV(filename);
  });

  // ── Formatting buttons ──
  $('ssBold')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.toggleFormat('bold');
    updateSSToolbarState();
  });
  $('ssItalic')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.toggleFormat('italic');
    updateSSToolbarState();
  });
  $('ssUnderline')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.toggleFormat('underline');
    updateSSToolbarState();
  });
  $('ssStrikethrough')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.toggleFormat('strikethrough');
    updateSSToolbarState();
  });

  // Font color
  $('ssFontColor')?.addEventListener('input', (e) => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.setFormat('color', e.target.value);
    const bar = $('ssFontColorBar');
    if (bar) bar.style.background = e.target.value;
  });

  // Fill color
  $('ssFillColor')?.addEventListener('input', (e) => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.setFormat('fill', e.target.value);
    const bar = $('ssFillColorBar');
    if (bar) bar.style.background = e.target.value;
  });

  // Alignment
  $('ssAlignLeft')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.setFormat('align', 'left');
    updateSSToolbarState();
  });
  $('ssAlignCenter')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.setFormat('align', 'center');
    updateSSToolbarState();
  });
  $('ssAlignRight')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.setFormat('align', 'right');
    updateSSToolbarState();
  });

  // Font family
  $('ssFontFamily')?.addEventListener('change', (e) => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.setFormat('fontFamily', e.target.value);
  });

  // Font size
  $('ssFontSize')?.addEventListener('change', (e) => {
    if (!state.spreadsheetView) return;
    const size = parseInt(e.target.value, 10);
    if (!isNaN(size) && size > 0) {
      state.spreadsheetView.setFormat('fontSize', size);
    }
  });

  // Number format
  $('ssNumberFormat')?.addEventListener('change', (e) => {
    if (!state.spreadsheetView) return;
    const fmt = e.target.value;
    state.spreadsheetView.setFormat('numberFormat', fmt === 'general' ? null : fmt);
  });

  // Find & Replace button
  $('ssFindReplace')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.openFindBar(false);
  });

  // Menu bar Find entry
  $('ssMenuFind')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.openFindBar(true);
  });

  // Merge cells button (S1.7)
  $('ssMergeCells')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.mergeCells();
  });

  // Text wrap toggle
  $('ssTextWrap')?.addEventListener('click', () => {
    if (!state.spreadsheetView) return;
    state.spreadsheetView.toggleFormat('wrap');
    updateSSToolbarState();
  });

  // ── Menu bar handlers (S1.1) ──

  // Helper: close all spreadsheet menu items
  function closeSsMenus() {
    document.querySelectorAll('#ssMenubar .app-menu-item').forEach(m => {
      m.classList.remove('open');
      const btn = m.querySelector('.app-menu-btn');
      if (btn) btn.setAttribute('aria-expanded', 'false');
    });
  }

  // Wire spreadsheet menu bar open/close/hover (same pattern as doc menu bar)
  let ssMenubarActive = false;
  const ssMenuItems = Array.from(document.querySelectorAll('#ssMenubar .app-menu-item'));
  ssMenuItems.forEach((item) => {
    const btn = item.querySelector('.app-menu-btn');
    if (!btn) return;
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const wasOpen = item.classList.contains('open');
      closeSsMenus();
      ssMenubarActive = false;
      if (!wasOpen) {
        item.classList.add('open');
        btn.setAttribute('aria-expanded', 'true');
        ssMenubarActive = true;
      }
    });
    btn.addEventListener('mouseenter', () => {
      if (ssMenubarActive) {
        closeSsMenus();
        item.classList.add('open');
        btn.setAttribute('aria-expanded', 'true');
      }
    });
  });
  // Close spreadsheet menus when clicking outside
  document.addEventListener('click', (e) => {
    if (!e.target.closest('#ssMenubar')) {
      closeSsMenus();
      ssMenubarActive = false;
    }
  });

  // File menu
  $('ssMenuNewSheet')?.addEventListener('click', async () => {
    closeSsMenus();
    try {
      const { switchView } = await import('./file.js');
      const { SpreadsheetView } = await import('./spreadsheet.js');
      const { addFileTab } = await import('./tabs.js');
      if (state.spreadsheetView) state.spreadsheetView.destroy();
      state.currentFormat = 'CSV';
      const container = $('spreadsheetContainer');
      state.spreadsheetView = new SpreadsheetView(container);
      state.spreadsheetView.loadWorkbook('', 'Sheet1.csv');
      $('docName').value = 'Untitled Spreadsheet';
      const info = $('statusInfo');
      if (info) info.textContent = '0 cells';
      $('statusFormat').textContent = 'CSV';
      addFileTab('Untitled Spreadsheet', 'spreadsheet', null);
    } catch (e) { console.error('New sheet error:', e); }
  });

  $('ssMenuOpen')?.addEventListener('click', () => {
    closeSsMenus();
    const input = $('csvInput') || $('fileInput');
    if (input) input.click();
  });

  $('ssMenuSaveXLSX')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    try {
      const bytes = state.spreadsheetView.exportXLSX();
      const blob = new Blob([bytes], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = ($('docName')?.value || 'spreadsheet') + '.xlsx';
      a.click();
      setTimeout(() => { URL.revokeObjectURL(url); }, 200);
    } catch (e) { console.error('XLSX export error:', e); }
  });

  $('ssMenuSaveCSV')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    const filename = $('docName')?.value || 'spreadsheet';
    state.spreadsheetView.downloadCSV(filename);
  });

  // Print (S1.6)
  $('ssMenuPrint')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.printToPDF();
  });

  $('ssMenuClose')?.addEventListener('click', async () => {
    closeSsMenus();
    if (state.spreadsheetView) { state.spreadsheetView.destroy(); state.spreadsheetView = null; }
    const { switchView } = await import('./file.js');
    const { closeFileTab } = await import('./tabs.js');
    // Close the active tab if there is one
    if (state.activeFileId) {
      closeFileTab(state.activeFileId);
    } else {
      switchView('editor');
      $('welcomeScreen').style.display = '';
      $('statusbar').classList.remove('show');
    }
  });

  // Edit menu
  $('ssMenuUndo')?.addEventListener('click', () => { closeSsMenus(); if (state.spreadsheetView) state.spreadsheetView.undo(); });
  $('ssMenuRedo')?.addEventListener('click', () => { closeSsMenus(); if (state.spreadsheetView) state.spreadsheetView.redo(); });
  $('ssMenuCut')?.addEventListener('click', () => { closeSsMenus(); if (state.spreadsheetView) state.spreadsheetView.cutCells(); });
  $('ssMenuCopy')?.addEventListener('click', () => { closeSsMenus(); if (state.spreadsheetView) state.spreadsheetView.copyCells(); });
  $('ssMenuPaste')?.addEventListener('click', () => { closeSsMenus(); if (state.spreadsheetView) state.spreadsheetView.pasteCells(); });
  $('ssMenuPasteSpecial')?.addEventListener('click', () => { closeSsMenus(); if (state.spreadsheetView) state.spreadsheetView.showPasteSpecialDialog(); });
  $('ssMenuSelectAll')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    const sheet = state.spreadsheetView._sheet();
    if (sheet) {
      state.spreadsheetView.selectionRange = { startCol: 0, startRow: 0, endCol: sheet.maxCol, endRow: sheet.maxRow };
      state.spreadsheetView.render();
    }
  });

  // View menu
  $('ssMenuFreezePanes')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    const { col, row } = state.spreadsheetView.selectedCell;
    if (state.spreadsheetView.frozenCols === col && state.spreadsheetView.frozenRows === row) {
      state.spreadsheetView.freezePanes(0, 0);
    } else {
      state.spreadsheetView.freezePanes(col, row);
    }
  });
  $('ssMenuGridlines')?.addEventListener('click', () => {
    closeSsMenus();
    // Toggle gridlines (re-render with/without)
    if (state.spreadsheetView) {
      state.spreadsheetView._showGridlines = state.spreadsheetView._showGridlines !== false ? false : true;
      state.spreadsheetView.render();
    }
  });
  $('ssMenuFullScreen')?.addEventListener('click', () => {
    closeSsMenus();
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      document.documentElement.requestFullscreen().catch(() => {});
    }
  });

  // Insert menu
  $('ssMenuFunction')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    const { col, row } = state.spreadsheetView.selectedCell;
    state.spreadsheetView.startEdit(col, row, '=');
  });

  // Insert Image (S3.5)
  $('ssMenuInsertImage')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.insertImage();
  });

  // Insert Shape (S5.6)
  $('ssMenuInsertShape')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.showInsertShapeDialog();
  });

  // Insert Chart (S3)
  $('ssMenuInsertChart')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    _openChartTypeModal();
  });

  // Chart type modal logic
  function _openChartTypeModal() {
    const modal = $('chartTypeModal');
    if (!modal) return;
    modal.style.display = '';
    modal.classList.add('show');
    // Reset selection
    modal.querySelectorAll('.chart-type-grid button').forEach(b => b.classList.remove('selected'));
    const insertBtn = $('chartInsertBtn');
    if (insertBtn) insertBtn.disabled = true;
    state._pendingChartType = null;
  }

  function _closeChartTypeModal() {
    const modal = $('chartTypeModal');
    if (!modal) return;
    modal.style.display = 'none';
    modal.classList.remove('show');
    state._pendingChartType = null;
  }

  // Chart type selection buttons
  const chartGrid = document.querySelector('#chartTypeModal .chart-type-grid');
  if (chartGrid) {
    chartGrid.addEventListener('click', (e) => {
      const btn = e.target.closest('button[data-chart-type]');
      if (!btn) return;
      chartGrid.querySelectorAll('button').forEach(b => b.classList.remove('selected'));
      btn.classList.add('selected');
      state._pendingChartType = btn.dataset.chartType;
      const insertBtn = $('chartInsertBtn');
      if (insertBtn) insertBtn.disabled = false;
    });
    // Double-click to quick-insert
    chartGrid.addEventListener('dblclick', (e) => {
      const btn = e.target.closest('button[data-chart-type]');
      if (!btn) return;
      state._pendingChartType = btn.dataset.chartType;
      _insertChartFromModal();
    });
  }

  $('chartInsertBtn')?.addEventListener('click', () => _insertChartFromModal());
  $('chartCancelBtn')?.addEventListener('click', () => _closeChartTypeModal());

  // Close on overlay click
  $('chartTypeModal')?.addEventListener('click', (e) => {
    if (e.target === $('chartTypeModal')) _closeChartTypeModal();
  });

  function _insertChartFromModal() {
    const type = state._pendingChartType;
    if (!type || !state.spreadsheetView) return;
    state.spreadsheetView.insertChart(type);
    _closeChartTypeModal();
  }

  // Format menu
  $('ssMenuNumberFormat')?.addEventListener('click', () => {
    closeSsMenus();
    const sel = $('ssNumberFormat');
    if (sel) sel.focus();
  });
  $('ssMenuCellStyle')?.addEventListener('click', () => {
    closeSsMenus();
    // Toggle bold on selection as a quick style shortcut
    if (state.spreadsheetView) { state.spreadsheetView.toggleFormat('bold'); updateSSToolbarState(); }
  });
  $('ssMenuMergeCellsMenu')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.mergeCells();
  });

  // Conditional Format (S2.1)
  $('ssMenuConditionalFormat')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.showConditionalFormatDialog();
  });

  // Data menu
  $('ssMenuSortAZ')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.sort(state.spreadsheetView.selectedCell.col, true);
  });
  $('ssMenuSortZA')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.sort(state.spreadsheetView.selectedCell.col, false);
  });
  // Multi-level Sort dialog (S2.4)
  $('ssMenuSortDialog')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.showSortDialog();
  });
  $('ssMenuFilter')?.addEventListener('click', () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    const col = state.spreadsheetView.selectedCell.col;
    if (state.spreadsheetView.filterState[col]) state.spreadsheetView.removeFilter(col);
    else state.spreadsheetView.addFilter(col);
  });
  // Remove Duplicates (S2.6)
  $('ssMenuRemoveDuplicates')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.showRemoveDuplicatesDialog();
  });
  // Data Validation (S2.2)
  $('ssMenuDataValidation')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.showDataValidationDialog();
  });

  // Insert > Comment (S2.3)
  $('ssMenuInsertComment')?.addEventListener('click', async () => {
    closeSsMenus();
    if (!state.spreadsheetView) return;
    const { col, row } = state.spreadsheetView.selectedCell;
    const sheet = state.spreadsheetView._sheet();
    const cell = sheet ? sheet.getCell(col, row) : null;
    const existing = cell?.comment?.text || '';
    // Custom modal prompt instead of browser prompt()
    const text = await _ssCommentPrompt(existing ? 'Edit comment:' : 'Add comment:', existing);
    if (text === null) return;
    const trimmed = text.trim();
    if (trimmed.length > 10000) {
      const { showToast } = await import('./toolbar-handlers.js');
      showToast('Comment too long (max 10,000 characters)', 'error');
      return;
    }
    state.spreadsheetView.setCellComment(col, row, text);
  });
  // Comments panel
  $('ssMenuShowComments')?.addEventListener('click', () => {
    closeSsMenus();
    if (state.spreadsheetView) state.spreadsheetView.showCommentsPanel();
  });

  // Update toolbar active states when selection changes
  // We hook into the canvas mouseup & keyup to refresh state
  const ssContainer = $('spreadsheetContainer');
  if (ssContainer) {
    ssContainer.addEventListener('mouseup', () => setTimeout(updateSSToolbarState, 50));
    ssContainer.addEventListener('keyup', () => setTimeout(updateSSToolbarState, 50));
  }
}

function updateSSToolbarState() {
  if (!state.spreadsheetView) return;
  const style = state.spreadsheetView.getActiveStyle();

  // Toggle button active states
  const toggleBtn = (id, prop) => {
    const btn = $(id);
    if (btn) {
      if (style[prop]) {
        btn.classList.add('active');
      } else {
        btn.classList.remove('active');
      }
    }
  };
  toggleBtn('ssBold', 'bold');
  toggleBtn('ssItalic', 'italic');
  toggleBtn('ssUnderline', 'underline');
  toggleBtn('ssStrikethrough', 'strikethrough');

  // Alignment
  ['ssAlignLeft', 'ssAlignCenter', 'ssAlignRight'].forEach(id => {
    const btn = $(id);
    if (btn) btn.classList.remove('active');
  });
  if (style.align === 'left') $('ssAlignLeft')?.classList.add('active');
  else if (style.align === 'center') $('ssAlignCenter')?.classList.add('active');
  else if (style.align === 'right') $('ssAlignRight')?.classList.add('active');

  // Number format dropdown
  const nfSelect = $('ssNumberFormat');
  if (nfSelect) nfSelect.value = style.numberFormat || 'general';

  // Font family dropdown
  const ffSelect = $('ssFontFamily');
  if (ffSelect && style.fontFamily) {
    ffSelect.value = style.fontFamily;
  } else if (ffSelect) {
    ffSelect.value = 'Arial, sans-serif';
  }

  // Font size dropdown
  const fsSelect = $('ssFontSize');
  if (fsSelect) fsSelect.value = String(style.fontSize || 13);

  // Color indicators
  const fontBar = $('ssFontColorBar');
  if (fontBar) fontBar.style.background = style.color || '#000000';
  const fillBar = $('ssFillColorBar');
  if (fillBar) fillBar.style.background = style.fill || '#ffffff';
}

boot();
