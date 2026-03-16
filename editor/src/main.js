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
import { initFonts, ensureDocumentFonts } from './fonts.js';

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

    // Initialize font database and preload common fonts
    initFonts(wasm).catch(e => console.warn('[fonts] Preload failed:', e));

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
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
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
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

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

boot();
