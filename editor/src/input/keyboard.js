// Canvas-mode keyboard handler.
//
// Translates keydown events into WASM editing / navigation calls.
// Works with the hidden textarea bridge and model-selection.

import { state } from '../state.js';
import {
  getPosition, getPositionJson, getRange, getRangeJson, isCollapsed,
  setCollapsed, setFromEditResult, setRange, extendFocus,
} from '../selection/model-selection.js';
import { repaintDirtyPages, repaintCaret, repaintSelection } from '../canvas-render.js';
import { focusBridge } from './bridge.js';
import { handleCanvasCopy, handleCanvasCut, handleCanvasPaste } from './clipboard.js';

const isMac = navigator.platform.indexOf('Mac') >= 0;
const MOD = isMac ? 'metaKey' : 'ctrlKey';

/**
 * Handle a keydown event in canvas mode.
 * Called from the hidden textarea's keydown listener.
 * @param {KeyboardEvent} e
 */
export function handleCanvasKeydown(e) {
  const doc = state.doc;
  if (!doc) return;

  // ── Modifier shortcuts ──────────────────────────────
  if (e[MOD]) {
    switch (e.key.toLowerCase()) {
      case 'z':
        e.preventDefault();
        if (e.shiftKey) {
          _doRedo(doc);
        } else {
          _doUndo(doc);
        }
        return;
      case 'y':
        e.preventDefault();
        _doRedo(doc);
        return;
      case 'b':
        e.preventDefault();
        _toggleMark(doc, 'bold');
        return;
      case 'i':
        e.preventDefault();
        _toggleMark(doc, 'italic');
        return;
      case 'u':
        e.preventDefault();
        _toggleMark(doc, 'underline');
        return;
      case 'a':
        e.preventDefault();
        _selectAll(doc);
        return;
      case 'c':
        handleCanvasCopy(e);
        return;
      case 'x':
        handleCanvasCut(e);
        return;
      case 'v':
        handleCanvasPaste(e);
        return;
    }
    return; // don't process other Ctrl combos as text input
  }

  // ── Navigation keys ─────────────────────────────────
  switch (e.key) {
    case 'ArrowLeft':
      e.preventDefault();
      _movePosition(doc, 'backward', e.shiftKey ? true : false, 'character');
      return;
    case 'ArrowRight':
      e.preventDefault();
      _movePosition(doc, 'forward', e.shiftKey ? true : false, 'character');
      return;
    case 'ArrowUp':
      e.preventDefault();
      _movePosition(doc, 'backward', e.shiftKey ? true : false, 'line');
      return;
    case 'ArrowDown':
      e.preventDefault();
      _movePosition(doc, 'forward', e.shiftKey ? true : false, 'line');
      return;
    case 'Home':
      e.preventDefault();
      _lineBoundary(doc, 'start', e.shiftKey);
      return;
    case 'End':
      e.preventDefault();
      _lineBoundary(doc, 'end', e.shiftKey);
      return;
  }

  // ── Editing keys ────────────────────────────────────
  switch (e.key) {
    case 'Enter':
      e.preventDefault();
      _insertParagraphBreak(doc);
      return;
    case 'Backspace':
      e.preventDefault();
      _handleBackspace(doc);
      return;
    case 'Delete':
      e.preventDefault();
      _handleDelete(doc);
      return;
    case 'Tab':
      e.preventDefault();
      // Insert tab character
      _insertText(doc, '\t');
      return;
    case 'Escape':
      return; // Let it propagate
  }

  // All other keys: let the textarea's input event handle text insertion
}

// ── Helpers ─────────────────────────────────────────

function _movePosition(doc, direction, extend, granularity) {
  try {
    if (extend) {
      const rangeJson = getRangeJson();
      if (!rangeJson) return;
      const resultStr = doc.move_range(rangeJson, direction, granularity, true);
      const newRange = JSON.parse(resultStr);
      setRange(newRange);
      repaintSelection();
    } else {
      const range = getRange();
      // If selection is expanded and not extending, collapse first
      if (range && !isCollapsed()) {
        // Collapse to the edge in the movement direction
        const collapsePos = direction === 'forward' ? range.focus : range.anchor;
        setCollapsed(collapsePos);
        repaintCaret();
        return;
      }
      const posJson = getPositionJson();
      if (!posJson) return;
      const resultStr = doc.move_position(posJson, direction, granularity);
      const newPos = JSON.parse(resultStr);
      setCollapsed(newPos);
      repaintCaret();
    }
  } catch (err) {
    console.error('[canvas-keyboard] move failed:', err);
  }
}

function _lineBoundary(doc, side, extend) {
  try {
    const posJson = getPositionJson();
    if (!posJson) return;
    const resultStr = doc.line_boundary(posJson, side);
    const newPos = JSON.parse(resultStr);
    if (extend) {
      extendFocus(newPos);
      repaintSelection();
    } else {
      setCollapsed(newPos);
      repaintCaret();
    }
  } catch (err) {
    console.error('[canvas-keyboard] line_boundary failed:', err);
  }
}

function _insertParagraphBreak(doc) {
  try {
    // Delete selection first if expanded
    if (!isCollapsed()) {
      const rangeJson = getRangeJson();
      if (rangeJson) {
        const delResult = JSON.parse(doc.canvas_delete_range(rangeJson));
        setFromEditResult(delResult.selection);
      }
    }
    const posJson = getPositionJson();
    if (!posJson) return;
    const resultStr = doc.canvas_insert_paragraph_break(posJson);
    const result = JSON.parse(resultStr);
    setFromEditResult(result.selection);
    repaintDirtyPages(result.dirty_pages);
    repaintCaret();
  } catch (err) {
    console.error('[canvas-keyboard] paragraph break failed:', err);
  }
}

function _handleBackspace(doc) {
  try {
    if (!isCollapsed()) {
      // Delete the selection range
      const rangeJson = getRangeJson();
      if (!rangeJson) return;
      const resultStr = doc.canvas_delete_range(rangeJson);
      const result = JSON.parse(resultStr);
      setFromEditResult(result.selection);
      repaintDirtyPages(result.dirty_pages);
      repaintCaret();
    } else {
      // Move backward by one character to create a 1-char range, then delete
      const posJson = getPositionJson();
      if (!posJson) return;
      const prevStr = doc.move_position(posJson, 'backward', 'character');
      const prevPos = JSON.parse(prevStr);
      const currentPos = getPosition();
      // Check if we actually moved (not at document start)
      if (prevPos.node_id === currentPos.node_id &&
          prevPos.offset_utf16 === currentPos.offset_utf16) {
        return; // At start of document
      }
      const range = { anchor: prevPos, focus: currentPos };
      const resultStr = doc.canvas_delete_range(JSON.stringify(range));
      const result = JSON.parse(resultStr);
      setFromEditResult(result.selection);
      repaintDirtyPages(result.dirty_pages);
      repaintCaret();
    }
  } catch (err) {
    console.error('[canvas-keyboard] backspace failed:', err);
  }
}

function _handleDelete(doc) {
  try {
    if (!isCollapsed()) {
      const rangeJson = getRangeJson();
      if (!rangeJson) return;
      const resultStr = doc.canvas_delete_range(rangeJson);
      const result = JSON.parse(resultStr);
      setFromEditResult(result.selection);
      repaintDirtyPages(result.dirty_pages);
      repaintCaret();
    } else {
      // Move forward by one character to create a 1-char range, then delete
      const posJson = getPositionJson();
      if (!posJson) return;
      const nextStr = doc.move_position(posJson, 'forward', 'character');
      const nextPos = JSON.parse(nextStr);
      const currentPos = getPosition();
      if (nextPos.node_id === currentPos.node_id &&
          nextPos.offset_utf16 === currentPos.offset_utf16) {
        return; // At end of document
      }
      const range = { anchor: currentPos, focus: nextPos };
      const resultStr = doc.canvas_delete_range(JSON.stringify(range));
      const result = JSON.parse(resultStr);
      setFromEditResult(result.selection);
      repaintDirtyPages(result.dirty_pages);
      repaintCaret();
    }
  } catch (err) {
    console.error('[canvas-keyboard] delete failed:', err);
  }
}

function _insertText(doc, text) {
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
    console.error('[canvas-keyboard] insert text failed:', err);
  }
}

function _toggleMark(doc, mark) {
  try {
    const rangeJson = getRangeJson();
    if (!rangeJson) return;
    const resultStr = doc.canvas_toggle_mark(rangeJson, mark);
    const result = JSON.parse(resultStr);
    repaintDirtyPages(result.dirty_pages);
    // Selection stays the same; toolbar state should update
    document.dispatchEvent(new CustomEvent('editor:selection-changed', {
      detail: { position: getPosition(), range: getRange(), collapsed: isCollapsed() },
    }));
  } catch (err) {
    console.error('[canvas-keyboard] toggle mark failed:', err);
  }
}

function _doUndo(doc) {
  try {
    doc.undo();
    // Full re-render after undo
    repaintDirtyPages({ start: 0, end: 9999 });
    repaintCaret();
  } catch (err) {
    console.error('[canvas-keyboard] undo failed:', err);
  }
}

function _doRedo(doc) {
  try {
    doc.redo();
    repaintDirtyPages({ start: 0, end: 9999 });
    repaintCaret();
  } catch (err) {
    console.error('[canvas-keyboard] redo failed:', err);
  }
}

function _selectAll(doc) {
  try {
    // Find first and last text nodes in the document
    // Use move_position to go to document start and end
    const posJson = getPositionJson();
    if (!posJson) return;

    // Move to very beginning: backward by paragraph many times
    let startPos = posJson;
    for (let i = 0; i < 1000; i++) {
      const prev = doc.move_position(startPos, 'backward', 'character');
      if (prev === startPos) break;
      startPos = prev;
    }

    // Move to very end: forward by paragraph many times
    let endPos = posJson;
    for (let i = 0; i < 1000; i++) {
      const next = doc.move_position(endPos, 'forward', 'character');
      if (next === endPos) break;
      endPos = next;
    }

    const range = {
      anchor: JSON.parse(startPos),
      focus: JSON.parse(endPos),
    };
    setRange(range);
    repaintSelection();
  } catch (err) {
    console.error('[canvas-keyboard] select all failed:', err);
  }
}
