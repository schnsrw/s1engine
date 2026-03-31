// Canvas-mode IME composition bridge.
//
// Wires the hidden textarea's compositionstart / compositionupdate /
// compositionend events to the WASM begin_composition / update_composition /
// commit_composition methods.

import { state } from '../state.js';
import { getPositionJson, setFromEditResult } from '../selection/model-selection.js';
import { repaintDirtyPages, repaintCaret } from '../canvas-render.js';

/**
 * Handle compositionstart — tell WASM to begin tracking the composition anchor.
 * @param {CompositionEvent} _e
 */
export function handleCompositionStart(_e) {
  const doc = state.doc;
  if (!doc) return;

  const posJson = getPositionJson();
  if (!posJson) return;

  try {
    doc.begin_composition(posJson);
  } catch (err) {
    console.error('[canvas-composition] begin failed:', err);
  }
}

/**
 * Handle compositionupdate — replace the preview text with the new composition string.
 * @param {CompositionEvent} e
 */
export function handleCompositionUpdate(e) {
  const doc = state.doc;
  if (!doc) return;

  const text = e.data || '';
  try {
    const resultStr = doc.update_composition(text);
    const result = JSON.parse(resultStr);
    setFromEditResult(result.selection);
    repaintDirtyPages(result.dirty_pages);
    repaintCaret();
    // TODO: draw composition underline rects on the canvas
  } catch (err) {
    console.error('[canvas-composition] update failed:', err);
  }
}

/**
 * Handle compositionend — commit the final text and clear composition state.
 * @param {CompositionEvent} e
 */
export function handleCompositionEnd(e) {
  const doc = state.doc;
  if (!doc) return;

  const text = e.data || '';
  try {
    const resultStr = doc.commit_composition(text);
    const result = JSON.parse(resultStr);
    setFromEditResult(result.selection);
    repaintDirtyPages(result.dirty_pages);
    repaintCaret();
  } catch (err) {
    console.error('[canvas-composition] commit failed:', err);
  }
}
