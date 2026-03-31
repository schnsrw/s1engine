// Rudra Office — Entry Point
//
// Thin entrypoint: registers global handlers, then delegates to boot().
// Feature-specific toolbars live in their own modules under features/.

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
import { initCanvasBridge } from './input/bridge.js';
import { initCanvasMouseEvents } from './canvas-render.js';
import { initFonts, ensureDocumentFonts } from './fonts.js';
import { initAIPanel } from './ai-panel.js';
import { initAIInline } from './ai-inline.js';
import { initTabs } from './tabs.js';
import { initCapabilities, gateElement } from './app/capabilities.js';
import { initPdfToolbar } from './features/pdf/toolbar.js';
import { initSpreadsheetToolbar } from './features/spreadsheet/toolbar.js';

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

// ── Boot ─────────────────────────────────────────────

async function boot() {
  const dot = $('wasmDot');
  const label = $('wasmLabel');

  // Disable toolbar until WASM is ready
  const toolbar = $('toolbar');
  if (toolbar) toolbar.style.pointerEvents = 'none';
  label.textContent = 'Loading engine...';

  // Spellcheck off on doc name
  const docName = $('docName');
  if (docName) { docName.spellcheck = false; docName.autocomplete = 'off'; }

  const wasmTimeout = setTimeout(() => {
    if (label && label.textContent.includes('Loading')) {
      label.textContent = 'Still loading... check your connection';
    }
  }, 8000);

  try {
    const wasm = await import(/* @vite-ignore */ '../wasm-pkg/s1engine_wasm.js');
    await wasm.default();

    clearTimeout(wasmTimeout);
    state.engine = new wasm.WasmEngine();
    setDetectFormat(wasm.detect_format);

    try { await initFonts(wasm); }
    catch (e) { console.warn('[fonts] Preload failed, using system fonts:', e); }

    dot.classList.add('ok');
    label.textContent = 's1engine ready';

    // Capability registry — single source of truth for feature gating
    initCapabilities(state.engine, wasm, window.S1_CONFIG || {});

    // Wire handlers
    initInput();
    initFileHandlers();
    initToolbar();
    initFind();
    initImageContextMenu();
    initCollabUI();

    // Gate UI based on capabilities
    gateElement($('btnShare'), 'canCollaborate', 'Collaboration');
    if ($('btnShare')?.classList.contains('disabled')) {
      $('btnShare').style.display = 'none';
    }
    gateElement($('miFootnote'), 'canInsertFootnote', 'Footnotes');
    gateElement($('miEndnote'), 'canInsertEndnote', 'Endnotes');

    // Canvas editing bridge and mouse events
    initCanvasBridge($('pageContainer'));
    initCanvasMouseEvents($('pageContainer'));

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

    // Re-enable toolbar
    if (toolbar) toolbar.style.pointerEvents = '';

    // Expose state for testing
    window.__s1_state = state;

    // Check for collaboration auto-join
    const params = new URLSearchParams(window.location.search);
    const isSharedLink = params.has('file') || params.has('room');
    await checkAutoJoin();

    // Check for auto-recovered document (never on shared links)
    if (!isSharedLink) try {
      const saved = await checkAutoRecover();
      if (saved && saved.bytes) {
        if (!saved.timestamp || saved.timestamp < Date.now() - 86400000 * 7) {
          // Too old — discard
        } else {
          const age = Date.now() - saved.timestamp;
          if (age < 86400000) {
            const name = saved.name || 'Untitled Document';
            const mins = Math.round(age / 60000);
            const timeStr = mins < 1 ? 'just now' : mins < 60 ? `${mins}m ago` : `${Math.round(mins / 60)}h ago`;
            const integrityOk = saved._checksumValid !== false && (!saved.byteLength || saved.byteLength === saved.bytes.byteLength);
            if (!integrityOk) {
              console.warn('Auto-recover skipped: checksum mismatch for', name);
            } else {
              const recover = await showRecoveryModal(`"${name}" (saved ${timeStr})`);
              if (recover) {
                openFile(new Uint8Array(saved.bytes), name + '.docx');
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
      boot();
    };
    if (toolbar) toolbar.style.pointerEvents = 'none';
  }
}

boot();
