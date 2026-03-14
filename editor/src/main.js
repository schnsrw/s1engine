// Folio Editor — Entry Point
// Wires all modules together and initializes the WASM engine.

import './styles.css';
import { state, $ } from './state.js';
import { initInput } from './input.js';
import { initFileHandlers, newDocument, openFile, setDetectFormat, checkAutoRecover } from './file.js';
import { initToolbar } from './toolbar-handlers.js';
import { initFind } from './find.js';

async function boot() {
  const dot = $('wasmDot');
  const label = $('wasmLabel');

  try {
    // Import WASM bindings from wasm-pkg directory
    const wasm = await import('../wasm-pkg/s1engine_wasm.js');
    await wasm.default();  // init wasm module

    state.engine = new wasm.WasmEngine();
    setDetectFormat(wasm.detect_format);

    dot.classList.add('ok');
    label.textContent = 's1engine ready';

    // Wire up all handlers
    initInput();
    initFileHandlers();
    initToolbar();
    initFind();

    // Expose state for testing
    window.__folio_state = state;

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
          if (confirm(`Recover unsaved document "${name}" (saved ${timeStr})?`)) {
            openFile(new Uint8Array(saved.bytes), name + '.docx');
          }
        }
      }
    } catch (_) {}

  } catch (e) {
    console.error('WASM init failed:', e);
    dot.classList.add('err');
    label.textContent = 'WASM failed: ' + e.message;
  }
}

boot();
