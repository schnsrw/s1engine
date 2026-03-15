// s1 editor — Entry Point
// Wires all modules together and initializes the WASM engine.

import './styles.css';
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

  try {
    // Import WASM bindings from wasm-pkg directory
    const wasm = await import('../wasm-pkg/s1engine_wasm.js');
    await wasm.default();  // init wasm module

    state.engine = new wasm.WasmEngine();
    setDetectFormat(wasm.detect_format);

    dot.classList.add('ok');
    label.textContent = 's1engine ready';
    // Re-enable toolbar
    if (toolbar) toolbar.style.pointerEvents = '';

    // Wire up all handlers
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
    renderRuler();
    initPdfToolbar();

    // Expose state for testing
    window.__s1_state = state;

    // Check for collaboration auto-join (?room=... URL param)
    checkAutoJoin();

    // Check for auto-recovered document
    try {
      const saved = await checkAutoRecover();
      if (saved && saved.bytes) {
        const age = Date.now() - (saved.timestamp || 0);
        // Only offer recovery for documents saved within the last 24 hours
        if (age < 86400000) {
          const name = saved.name || 'Untitled Document';
          const mins = Math.round(age / 60000);
          const timeStr = mins < 1 ? 'just now' : mins < 60 ? `${mins}m ago` : `${Math.round(mins / 60)}h ago`;
          // Check checksum integrity
          const integrityOk = saved._checksumValid !== false;
          const warning = integrityOk ? '' : '\n\nWarning: checksum mismatch detected — this file may be corrupted.';
          if (confirm(`Recover unsaved document "${name}" (saved ${timeStr})?${warning}`)) {
            openFile(new Uint8Array(saved.bytes), name + '.docx');
            // Restore comment thread replies if they were persisted
            if (saved.commentReplies) {
              try { state.commentReplies = JSON.parse(saved.commentReplies); } catch (_) {}
            }
          }
        }
      }
    } catch (_) {}

  } catch (e) {
    console.error('WASM init failed:', e);
    dot.classList.add('err');
    label.textContent = 'WASM failed: ' + e.message;
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
    if (state.pdfViewer) state.pdfViewer.setZoom(e.target.value);
  });

  // Tool selection
  document.querySelectorAll('.pdf-tool-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.pdf-tool-btn').forEach(b => {
        b.classList.remove('active');
        b.setAttribute('aria-pressed', 'false');
      });
      btn.classList.add('active');
      btn.setAttribute('aria-pressed', 'true');
      state.pdfTool = btn.dataset.tool;

      // Update cursor style on canvas container
      const container = $('pdfCanvasContainer');
      if (container) {
        container.dataset.tool = btn.dataset.tool;
      }
    });
  });

  // Download PDF button
  $('pdfSave')?.addEventListener('click', () => {
    if (!state.pdfBytes) return;
    const blob = new Blob([state.pdfBytes], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = ($('docName').value || 'document') + '.pdf';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
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

boot();
