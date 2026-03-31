// Hidden textarea input bridge for canvas-mode editing.
//
// Canvas elements can't receive text input natively. This module creates
// an invisible <textarea> positioned near the caret that captures all
// keyboard / IME input and forwards it to the WASM editing methods.

import { state } from '../state.js';
import { isCanvasMode } from '../render.js';
import {
  getPosition, getPositionJson, getRangeJson, isCollapsed,
  setFromEditResult,
} from '../selection/model-selection.js';
import { handleCanvasKeydown } from './keyboard.js';
import {
  handleCompositionStart, handleCompositionUpdate, handleCompositionEnd,
} from './composition.js';
import { repaintDirtyPages, repaintCaret } from '../canvas-render.js';

let _textarea = null;
let _container = null;

/**
 * Create the hidden textarea and wire input events.
 * Call once after WASM init.
 * @param {HTMLElement} container - The page container element
 */
export function initCanvasBridge(container) {
  _container = container;

  if (_textarea) return; // already initialised
  _textarea = document.createElement('textarea');
  _textarea.id = 's1-canvas-input';
  _textarea.setAttribute('autocomplete', 'off');
  _textarea.setAttribute('autocorrect', 'off');
  _textarea.setAttribute('autocapitalize', 'off');
  _textarea.setAttribute('spellcheck', 'false');
  _textarea.setAttribute('aria-label', 'Canvas text input');
  // The textarea must be focusable (no pointer-events:none) but visually hidden.
  // We use clip and opacity instead of display:none so it can still receive focus.
  _textarea.style.cssText =
    'position:fixed;top:-9999px;left:-9999px;width:1px;height:1px;' +
    'opacity:0.01;padding:0;border:0;outline:none;overflow:hidden;' +
    'resize:none;z-index:9999;';
  document.body.appendChild(_textarea);

  // --- input event: captures typed characters ---
  _textarea.addEventListener('input', (e) => {
    if (!isCanvasMode() || !state.doc) return;
    // During composition, handled by composition events
    if (e.isComposing || state._composing) return;

    const text = _textarea.value;
    if (!text) return;
    _textarea.value = '';

    const posJson = getPositionJson();
    const rangeJson = getRangeJson();
    if (!posJson) return;

    try {
      let resultStr;
      if (!isCollapsed() && rangeJson) {
        resultStr = state.doc.canvas_replace_range(rangeJson, text);
      } else {
        resultStr = state.doc.canvas_insert_text(posJson, text);
      }
      const result = JSON.parse(resultStr);
      setFromEditResult(result.selection);
      repaintDirtyPages(result.dirty_pages);
      repaintCaret();
    } catch (err) {
      console.error('[canvas-bridge] insert text failed:', err);
    }
  });

  // --- keydown: arrows, enter, backspace, shortcuts ---
  _textarea.addEventListener('keydown', (e) => {
    if (!isCanvasMode() || !state.doc) return;
    handleCanvasKeydown(e);
  });

  // --- IME composition ---
  _textarea.addEventListener('compositionstart', (e) => {
    if (!isCanvasMode()) return;
    state._composing = true;
    handleCompositionStart(e);
  });
  _textarea.addEventListener('compositionupdate', (e) => {
    if (!isCanvasMode()) return;
    handleCompositionUpdate(e);
  });
  _textarea.addEventListener('compositionend', (e) => {
    if (!isCanvasMode()) return;
    handleCompositionEnd(e);
    _textarea.value = '';
    setTimeout(() => { state._composing = false; }, 10);
  });

  // --- clipboard events (capture from the textarea) ---
  _textarea.addEventListener('copy', (e) => {
    if (!isCanvasMode() || !state.doc) return;
    import('./clipboard.js').then(clip => clip.handleCanvasCopy(e));
  });
  _textarea.addEventListener('cut', (e) => {
    if (!isCanvasMode() || !state.doc) return;
    import('./clipboard.js').then(clip => clip.handleCanvasCut(e));
  });
  _textarea.addEventListener('paste', (e) => {
    if (!isCanvasMode() || !state.doc) return;
    import('./clipboard.js').then(clip => clip.handleCanvasPaste(e));
  });
}

/**
 * Focus the hidden textarea so it captures keyboard input.
 * Call after every canvas click and after toolbar actions.
 */
export function focusBridge() {
  if (_textarea && isCanvasMode()) {
    _textarea.focus({ preventScroll: true });
  }
}

/**
 * Reposition the hidden textarea near the caret so IME candidate
 * windows appear in the right place.
 */
export function repositionBridge() {
  if (!_textarea || !state.doc) return;
  const posJson = getPositionJson();
  if (!posJson) return;

  try {
    const rectStr = state.doc.caret_rect(posJson);
    const rect = JSON.parse(rectStr);
    const ptToPx = 96 / 72;
    // Convert page-local pt to screen px (approximate — assumes single-column centered)
    const container = _container;
    if (!container) return;
    const cr = container.getBoundingClientRect();
    const pageGap = 20;

    // Find page top
    let pageTop = pageGap;
    const pages = container.querySelectorAll('.s1-canvas-page');
    for (let i = 0; i < rect.page_index && i < pages.length; i++) {
      pageTop += parseFloat(pages[i].style.height) + pageGap;
    }
    const currentPage = pages[rect.page_index];
    const pageW = currentPage ? parseFloat(currentPage.style.width) : 612 * ptToPx;
    const pageLeft = Math.max(pageGap, (container.clientWidth - pageW) / 2);

    const screenX = cr.left + pageLeft + rect.x * ptToPx - container.scrollLeft;
    const screenY = cr.top + pageTop + rect.y * ptToPx - container.scrollTop;

    _textarea.style.left = screenX + 'px';
    _textarea.style.top = screenY + 'px';
  } catch (_) { /* layout may not be ready */ }
}

/** @returns {HTMLTextAreaElement|null} The hidden textarea element */
export function getBridgeElement() {
  return _textarea;
}
