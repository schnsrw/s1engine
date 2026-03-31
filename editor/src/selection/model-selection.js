// Canvas-mode selection state — Rust model is the source of truth.
//
// Stores PositionRef / RangeRef as JSON objects that correspond exactly
// to the WASM hit_test / caret_rect / selection_rects API shapes.

import { state } from '../state.js';

// Internal state
let _position = null;  // PositionRef JSON: { node_id, offset_utf16, affinity }
let _range = null;      // RangeRef JSON: { anchor: PositionRef, focus: PositionRef }
let _collapsed = true;

/**
 * Set the selection from a WASM hit_test result.
 * @param {object} hitResult - { position: PositionRef, ... }
 */
export function setFromHitTest(hitResult) {
  if (!hitResult || !hitResult.position) return;
  _position = hitResult.position;
  _range = { anchor: { ...hitResult.position }, focus: { ...hitResult.position } };
  _collapsed = true;
  _emit();
}

/**
 * Set the selection from an EditResult's selection field.
 * @param {object} sel - PositionRef from EditResult
 */
export function setFromEditResult(sel) {
  if (!sel) return;
  _position = sel;
  _range = { anchor: { ...sel }, focus: { ...sel } };
  _collapsed = true;
  _emit();
}

/**
 * Extend the selection focus to a new position (for shift-click / drag).
 * @param {object} pos - PositionRef
 */
export function extendFocus(pos) {
  if (!_range) return;
  _range = { anchor: { ..._range.anchor }, focus: { ...pos } };
  _position = pos;
  _collapsed = _range.anchor.node_id === _range.focus.node_id
    && _range.anchor.offset_utf16 === _range.focus.offset_utf16;
  _emit();
}

/**
 * Set the full range directly (e.g. from move_range result).
 * @param {object} range - { anchor: PositionRef, focus: PositionRef }
 */
export function setRange(range) {
  if (!range) return;
  _range = range;
  _position = range.focus;
  _collapsed = range.anchor.node_id === range.focus.node_id
    && range.anchor.offset_utf16 === range.focus.offset_utf16;
  _emit();
}

/**
 * Set from a collapsed position (e.g. after move_position).
 * @param {object} pos - PositionRef
 */
export function setCollapsed(pos) {
  if (!pos) return;
  _position = pos;
  _range = { anchor: { ...pos }, focus: { ...pos } };
  _collapsed = true;
  _emit();
}

/** @returns {object|null} Current PositionRef (deep copy to prevent mutation) */
export function getPosition() {
  return _position ? { ..._position } : null;
}

/** @returns {string} Position as JSON string for WASM calls */
export function getPositionJson() {
  return _position ? JSON.stringify(_position) : null;
}

/** @returns {object|null} Current RangeRef (deep copy) */
export function getRange() {
  return _range ? { anchor: { ..._range.anchor }, focus: { ..._range.focus } } : null;
}

/** @returns {string} Range as JSON string for WASM calls */
export function getRangeJson() {
  return _range ? JSON.stringify(_range) : null;
}

/** @returns {boolean} Whether selection is collapsed (caret, no highlight) */
export function isCollapsed() { return _collapsed; }

/** Clear selection entirely. */
export function clear() {
  _position = null;
  _range = null;
  _collapsed = true;
}

function _emit() {
  try {
    document.dispatchEvent(new CustomEvent('editor:selection-changed', {
      detail: { position: _position, range: _range, collapsed: _collapsed },
    }));
  } catch (_) { /* guard against SSR / test environments */ }
}
