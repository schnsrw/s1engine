// Canvas-mode clipboard handling (copy, cut, paste).
//
// Uses the WASM copy_range_plain_text / copy_range_html / paste_html
// methods and the Clipboard API.

import { state } from '../state.js';
import {
  getPositionJson, getRangeJson, isCollapsed,
  setFromEditResult,
} from '../selection/model-selection.js';
import { repaintDirtyPages, repaintCaret } from '../canvas-render.js';

/**
 * Copy the current canvas selection to the clipboard.
 * @param {ClipboardEvent|KeyboardEvent} e
 */
export function handleCanvasCopy(e) {
  const doc = state.doc;
  if (!doc || isCollapsed()) return;

  const rangeJson = getRangeJson();
  if (!rangeJson) return;

  try {
    const plainText = doc.copy_range_plain_text(rangeJson);
    let htmlText = '';
    try { htmlText = doc.copy_range_html(rangeJson); } catch (_) {}

    if (e.clipboardData) {
      // Triggered from a real copy event
      e.preventDefault();
      e.clipboardData.setData('text/plain', plainText);
      if (htmlText) e.clipboardData.setData('text/html', htmlText);
    } else {
      // Triggered from Ctrl+C keydown — use async clipboard API
      const items = [
        new ClipboardItem({
          'text/plain': new Blob([plainText], { type: 'text/plain' }),
          ...(htmlText ? { 'text/html': new Blob([htmlText], { type: 'text/html' }) } : {}),
        }),
      ];
      navigator.clipboard.write(items).catch(err => {
        // Fallback to writeText
        navigator.clipboard.writeText(plainText).catch(() => {});
      });
    }
  } catch (err) {
    console.error('[canvas-clipboard] copy failed:', err);
  }
}

/**
 * Cut the current canvas selection to the clipboard.
 * @param {ClipboardEvent|KeyboardEvent} e
 */
export function handleCanvasCut(e) {
  const doc = state.doc;
  if (!doc || isCollapsed()) return;

  // Copy first
  handleCanvasCopy(e);

  // Then delete
  const rangeJson = getRangeJson();
  if (!rangeJson) return;

  try {
    const resultStr = doc.canvas_delete_range(rangeJson);
    const result = JSON.parse(resultStr);
    setFromEditResult(result.selection);
    repaintDirtyPages(result.dirty_pages);
    repaintCaret();
  } catch (err) {
    console.error('[canvas-clipboard] cut failed:', err);
  }
}

/**
 * Paste clipboard content into the canvas at the current position.
 * @param {ClipboardEvent|KeyboardEvent} e
 */
export function handleCanvasPaste(e) {
  const doc = state.doc;
  if (!doc) return;

  if (e.preventDefault) e.preventDefault();

  if (e.clipboardData) {
    _pasteFromData(doc, e.clipboardData);
  } else {
    // Triggered from keydown — use async clipboard API
    navigator.clipboard.read().then(items => {
      for (const item of items) {
        // Try HTML first
        if (item.types.includes('text/html')) {
          item.getType('text/html').then(blob => blob.text()).then(html => {
            _pasteHtml(doc, html);
          }).catch(() => {
            // Fallback to plain text
            if (item.types.includes('text/plain')) {
              item.getType('text/plain').then(blob => blob.text()).then(text => {
                _pastePlain(doc, text);
              }).catch(() => {});
            }
          });
          return;
        }
        if (item.types.includes('text/plain')) {
          item.getType('text/plain').then(blob => blob.text()).then(text => {
            _pastePlain(doc, text);
          }).catch(() => {});
          return;
        }
      }
    }).catch(() => {
      // Clipboard API not available — try fallback
      navigator.clipboard.readText().then(text => {
        _pastePlain(doc, text);
      }).catch(() => {});
    });
  }
}

function _pasteFromData(doc, clipboardData) {
  const html = clipboardData.getData('text/html');
  const text = clipboardData.getData('text/plain');
  if (html) {
    _pasteHtml(doc, html);
  } else if (text) {
    _pastePlain(doc, text);
  }
}

function _pasteHtml(doc, html) {
  try {
    const posJson = getPositionJson();
    const rangeJson = getRangeJson();
    if (!posJson) return;

    let resultStr;
    if (!isCollapsed() && rangeJson) {
      // Delete selection first, then paste
      const delResult = JSON.parse(doc.canvas_delete_range(rangeJson));
      setFromEditResult(delResult.selection);
      const newPosJson = JSON.stringify(delResult.selection);
      resultStr = doc.paste_html(newPosJson, html);
    } else {
      resultStr = doc.paste_html(posJson, html);
    }
    const result = JSON.parse(resultStr);
    setFromEditResult(result.selection);
    repaintDirtyPages(result.dirty_pages);
    repaintCaret();
  } catch (err) {
    console.error('[canvas-clipboard] paste HTML failed:', err);
  }
}

function _pastePlain(doc, text) {
  try {
    let resultStr;
    if (!isCollapsed()) {
      const rangeJson = getRangeJson();
      if (!rangeJson) return;
      resultStr = doc.canvas_replace_range(rangeJson, text);
    } else {
      const posJson = getPositionJson();
      if (!posJson) return;
      resultStr = doc.canvas_insert_text(posJson, text);
    }
    const result = JSON.parse(resultStr);
    setFromEditResult(result.selection);
    repaintDirtyPages(result.dirty_pages);
    repaintCaret();
  } catch (err) {
    console.error('[canvas-clipboard] paste plain failed:', err);
  }
}
