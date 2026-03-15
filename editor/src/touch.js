// E6.2 — Touch Selection & E6.3 — Mobile Keyboard
// Touch selection handles, gestures, long-press context menu,
// mobile keyboard detection, and floating formatting bar.

import { state, $ } from './state.js';
import {
  getSelectionInfo, getActiveElement, findParagraphEl,
  getEditableText,
} from './selection.js';
import { toggleFormat, updateToolbarState } from './toolbar.js';
import { renderNodeById, syncAllText, debouncedSync } from './render.js';
import { markDirty } from './file.js';

// ═══════════════════════════════════════════════════
// Detection
// ═══════════════════════════════════════════════════

function isTouchDevice() {
  return 'ontouchstart' in window || navigator.maxTouchPoints > 0;
}

// ═══════════════════════════════════════════════════
// E6.2 — Touch Selection Handles
// ═══════════════════════════════════════════════════

let startHandle = null;
let endHandle = null;
let touchHandlesVisible = false;
let activeDragHandle = null; // 'start' | 'end' | null

function createHandleElements() {
  startHandle = document.createElement('div');
  startHandle.className = 'touch-handle touch-handle-start';
  startHandle.setAttribute('aria-hidden', 'true');

  endHandle = document.createElement('div');
  endHandle.className = 'touch-handle touch-handle-end';
  endHandle.setAttribute('aria-hidden', 'true');

  document.body.appendChild(startHandle);
  document.body.appendChild(endHandle);
}

function showHandles() {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || sel.rangeCount === 0) {
    hideHandles();
    return;
  }

  const range = sel.getRangeAt(0);
  // Get bounding rects for start and end of range
  const startRange = document.createRange();
  startRange.setStart(range.startContainer, range.startOffset);
  startRange.collapse(true);
  const startRect = startRange.getBoundingClientRect();

  const endRange = document.createRange();
  endRange.setStart(range.endContainer, range.endOffset);
  endRange.collapse(true);
  const endRect = endRange.getBoundingClientRect();

  // Only show if rects are valid (non-zero dimensions indicate visible position)
  if ((startRect.width === 0 && startRect.height === 0 && startRect.x === 0) ||
      (endRect.width === 0 && endRect.height === 0 && endRect.x === 0)) {
    hideHandles();
    return;
  }

  const lineHeight = endRect.height || 18;

  // Position start handle below left edge of selection
  startHandle.style.left = `${startRect.left - 10}px`;
  startHandle.style.top = `${startRect.bottom + 2}px`;
  startHandle.style.display = 'block';

  // Position end handle below right edge of selection
  endHandle.style.left = `${endRect.left - 10}px`;
  endHandle.style.top = `${endRect.bottom + 2}px`;
  endHandle.style.display = 'block';

  touchHandlesVisible = true;
}

function hideHandles() {
  if (startHandle) startHandle.style.display = 'none';
  if (endHandle) endHandle.style.display = 'none';
  touchHandlesVisible = false;
}

/** Resolve a screen point to a caret position using the best available API. */
function caretFromPoint(x, y) {
  if (document.caretPositionFromPoint) {
    const pos = document.caretPositionFromPoint(x, y);
    if (pos) return { node: pos.offsetNode, offset: pos.offset };
  } else if (document.caretRangeFromPoint) {
    const range = document.caretRangeFromPoint(x, y);
    if (range) return { node: range.startContainer, offset: range.startOffset };
  }
  return null;
}

function setupHandleDrag() {
  const onTouchStart = (handle, which) => {
    handle.addEventListener('touchstart', (e) => {
      e.preventDefault();
      e.stopPropagation();
      activeDragHandle = which;
      handle.classList.add('dragging');
    }, { passive: false });
  };

  onTouchStart(startHandle, 'start');
  onTouchStart(endHandle, 'end');

  document.addEventListener('touchmove', (e) => {
    if (!activeDragHandle) return;
    e.preventDefault();
    const touch = e.touches[0];
    const caret = caretFromPoint(touch.clientX, touch.clientY);
    if (!caret) return;

    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;

    const currentRange = sel.getRangeAt(0);

    try {
      const newRange = document.createRange();
      if (activeDragHandle === 'start') {
        newRange.setStart(caret.node, caret.offset);
        newRange.setEnd(currentRange.endContainer, currentRange.endOffset);
      } else {
        newRange.setStart(currentRange.startContainer, currentRange.startOffset);
        newRange.setEnd(caret.node, caret.offset);
      }

      // Only apply if range is not collapsed and start is before end
      if (!newRange.collapsed) {
        sel.removeAllRanges();
        sel.addRange(newRange);
        showHandles();
      }
    } catch (_) {
      // Range might be invalid if start > end; ignore
    }
  }, { passive: false });

  document.addEventListener('touchend', () => {
    if (activeDragHandle) {
      const handle = activeDragHandle === 'start' ? startHandle : endHandle;
      handle.classList.remove('dragging');
      activeDragHandle = null;
      // Re-show handles at final position
      setTimeout(showHandles, 10);
    }
  });
}

// ═══════════════════════════════════════════════════
// E6.2 — Double-tap (word) & Triple-tap (paragraph)
// ═══════════════════════════════════════════════════

let lastTapTime = 0;
let lastTapX = 0;
let lastTapY = 0;
let tapCount = 0;
let tapTimer = null;

const TAP_DISTANCE = 30;    // px threshold for "same spot"
const DOUBLE_TAP_MS = 300;
const TRIPLE_TAP_MS = 500;

function setupTapGestures() {
  const page = $('pageContainer');
  if (!page) return;

  page.addEventListener('touchend', (e) => {
    // Skip if a handle is being dragged or context menu is open
    if (activeDragHandle) return;
    if (document.querySelector('.touch-ctx-menu')) return;

    const touch = e.changedTouches[0];
    const now = Date.now();
    const dx = Math.abs(touch.clientX - lastTapX);
    const dy = Math.abs(touch.clientY - lastTapY);
    const sameLoc = dx < TAP_DISTANCE && dy < TAP_DISTANCE;
    const timeSinceLastTap = now - lastTapTime;

    if (sameLoc && timeSinceLastTap < TRIPLE_TAP_MS) {
      tapCount++;
    } else {
      tapCount = 1;
    }

    lastTapTime = now;
    lastTapX = touch.clientX;
    lastTapY = touch.clientY;

    clearTimeout(tapTimer);

    if (tapCount >= 3) {
      // Triple-tap: select paragraph immediately
      selectParagraphAt(touch.clientX, touch.clientY);
      tapCount = 0;
    } else if (tapCount === 2) {
      // Double-tap: defer briefly to see if a third tap follows
      const cx = touch.clientX, cy = touch.clientY;
      tapTimer = setTimeout(() => {
        if (tapCount === 2) {
          selectWordAt(cx, cy);
          tapCount = 0;
        }
      }, DOUBLE_TAP_MS);
    } else {
      // Single tap — update handle visibility after selection settles
      tapTimer = setTimeout(() => {
        if (tapCount === 1) {
          const sel = window.getSelection();
          if (sel && !sel.isCollapsed) {
            showHandles();
          } else {
            hideHandles();
          }
          tapCount = 0;
        }
      }, DOUBLE_TAP_MS + 20);
    }
  }, { passive: true });
}

function selectWordAt(x, y) {
  const caret = caretFromPoint(x, y);
  if (!caret || !caret.node) return;

  const textNode = caret.node;
  if (textNode.nodeType !== 3) return; // must be a text node
  const text = textNode.textContent;
  const offset = caret.offset;

  // Expand to word boundaries
  let start = offset;
  let end = offset;
  const wordChar = /[\w\u00C0-\u024F\u1E00-\u1EFF]/; // letters + accented

  while (start > 0 && wordChar.test(text[start - 1])) start--;
  while (end < text.length && wordChar.test(text[end])) end++;

  if (start === end) return; // clicked on whitespace/punctuation

  const range = document.createRange();
  range.setStart(textNode, start);
  range.setEnd(textNode, end);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
  showHandles();
}

function selectParagraphAt(x, y) {
  const caret = caretFromPoint(x, y);
  if (!caret || !caret.node) return;

  const paraEl = findParagraphEl(caret.node, caret.offset);
  if (!paraEl) return;

  const range = document.createRange();
  range.selectNodeContents(paraEl);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
  showHandles();
}

// ═══════════════════════════════════════════════════
// E6.2 — Long-press Context Menu
// ═══════════════════════════════════════════════════

let longPressTimer = null;
let longPressX = 0;
let longPressY = 0;
const LONG_PRESS_MS = 500;

function setupLongPress() {
  const page = $('pageContainer');
  if (!page) return;

  page.addEventListener('touchstart', (e) => {
    if (e.touches.length !== 1) return;
    const touch = e.touches[0];
    longPressX = touch.clientX;
    longPressY = touch.clientY;

    longPressTimer = setTimeout(() => {
      showTouchContextMenu(longPressX, longPressY);
    }, LONG_PRESS_MS);
  }, { passive: true });

  page.addEventListener('touchmove', (e) => {
    if (!longPressTimer) return;
    const touch = e.touches[0];
    const dx = Math.abs(touch.clientX - longPressX);
    const dy = Math.abs(touch.clientY - longPressY);
    if (dx > 10 || dy > 10) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
  }, { passive: true });

  page.addEventListener('touchend', () => {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }, { passive: true });

  page.addEventListener('touchcancel', () => {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }, { passive: true });

  // Prevent native context menu on touch devices
  page.addEventListener('contextmenu', (e) => {
    if (isTouchDevice() && e.pointerType === 'touch') {
      // Only prevent if our custom menu will show
      // Desktop right-clicks should still work normally
    }
  });
}

function showTouchContextMenu(x, y) {
  // Remove any existing menu
  closeTouchContextMenu();

  const menu = document.createElement('div');
  menu.className = 'touch-ctx-menu';
  menu.setAttribute('role', 'menu');

  const items = [
    { label: 'Cut', action: () => execTouchAction('cut') },
    { label: 'Copy', action: () => execTouchAction('copy') },
    { label: 'Paste', action: () => execTouchAction('paste') },
    { sep: true },
    { label: 'Select All', action: () => execTouchAction('selectAll') },
    { sep: true },
    { label: 'Bold', action: () => { toggleFormat('bold'); updateToolbarState(); } },
    { label: 'Italic', action: () => { toggleFormat('italic'); updateToolbarState(); } },
  ];

  for (const item of items) {
    if (item.sep) {
      const sep = document.createElement('div');
      sep.className = 'touch-ctx-sep';
      menu.appendChild(sep);
      continue;
    }
    const btn = document.createElement('button');
    btn.className = 'touch-ctx-item';
    btn.textContent = item.label;
    btn.setAttribute('role', 'menuitem');
    btn.addEventListener('touchend', (e) => {
      e.preventDefault();
      e.stopPropagation();
      closeTouchContextMenu();
      item.action();
    });
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      closeTouchContextMenu();
      item.action();
    });
    menu.appendChild(btn);
  }

  // Position — center horizontally at the tap point, above it
  document.body.appendChild(menu);
  const menuRect = menu.getBoundingClientRect();
  let left = x - menuRect.width / 2;
  let top = y - menuRect.height - 12;

  // Keep within viewport
  if (left < 8) left = 8;
  if (left + menuRect.width > window.innerWidth - 8) left = window.innerWidth - menuRect.width - 8;
  if (top < 8) top = y + 24; // show below if no room above

  menu.style.left = `${left}px`;
  menu.style.top = `${top}px`;

  // Close on tap outside
  setTimeout(() => {
    document.addEventListener('touchstart', closeTouchContextMenuOnOutside, { passive: true });
    document.addEventListener('mousedown', closeTouchContextMenuOnOutside);
  }, 50);
}

function closeTouchContextMenuOnOutside(e) {
  const menu = document.querySelector('.touch-ctx-menu');
  if (menu && !menu.contains(e.target)) {
    closeTouchContextMenu();
  }
}

function closeTouchContextMenu() {
  const existing = document.querySelector('.touch-ctx-menu');
  if (existing) existing.remove();
  document.removeEventListener('touchstart', closeTouchContextMenuOnOutside);
  document.removeEventListener('mousedown', closeTouchContextMenuOnOutside);
}

async function execTouchAction(action) {
  const sel = window.getSelection();
  const info = getSelectionInfo();

  switch (action) {
    case 'cut':
      if (sel && !sel.isCollapsed) {
        try {
          const text = sel.toString();
          await navigator.clipboard.writeText(text);
          // Delete the selected text via WASM
          if (info && !info.collapsed && state.doc) {
            syncAllText();
            state.doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
            const { renderDocument } = await import('./render.js');
            renderDocument();
            markDirty();
          }
        } catch (_) {}
      }
      break;
    case 'copy':
      if (sel && !sel.isCollapsed) {
        try {
          await navigator.clipboard.writeText(sel.toString());
        } catch (_) {}
      }
      break;
    case 'paste':
      try {
        const text = await navigator.clipboard.readText();
        if (text && state.doc && info) {
          syncAllText();
          if (!info.collapsed) {
            state.doc.delete_selection(info.startNodeId, info.startOffset, info.endNodeId, info.endOffset);
          }
          state.doc.insert_text_in_paragraph(info.startNodeId, info.startOffset, text);
          const { renderDocument } = await import('./render.js');
          renderDocument();
          markDirty();
        }
      } catch (_) {}
      break;
    case 'selectAll':
      if (sel) {
        const page = $('pageContainer');
        if (page) {
          const range = document.createRange();
          range.selectNodeContents(page);
          sel.removeAllRanges();
          sel.addRange(range);
          showHandles();
        }
      }
      break;
  }
  hideHandles();
}

// ═══════════════════════════════════════════════════
// E6.2 — Selection change listener (show/hide handles)
// ═══════════════════════════════════════════════════

function setupSelectionListener() {
  document.addEventListener('selectionchange', () => {
    if (!isTouchDevice()) return;
    // Don't interfere with handle dragging
    if (activeDragHandle) return;

    const sel = window.getSelection();
    if (sel && !sel.isCollapsed) {
      showHandles();
    } else {
      hideHandles();
    }
  });
}

// ═══════════════════════════════════════════════════
// E6.3 — Mobile Keyboard Detection
// ═══════════════════════════════════════════════════

let keyboardBar = null;
let isKeyboardOpen = false;
let initialViewportHeight = 0;
const KEYBOARD_THRESHOLD = 150; // px difference to detect keyboard

function setupKeyboardDetection() {
  if (!window.visualViewport) return;

  initialViewportHeight = window.visualViewport.height;

  window.visualViewport.addEventListener('resize', () => {
    const currentHeight = window.visualViewport.height;
    const diff = initialViewportHeight - currentHeight;

    if (diff > KEYBOARD_THRESHOLD && !isKeyboardOpen) {
      isKeyboardOpen = true;
      document.body.classList.add('keyboard-open');
      showKeyboardBar(currentHeight);
      scrollCursorIntoView();
    } else if (diff <= KEYBOARD_THRESHOLD && isKeyboardOpen) {
      isKeyboardOpen = false;
      document.body.classList.remove('keyboard-open');
      hideKeyboardBar();
    } else if (isKeyboardOpen) {
      // Keyboard height changed — reposition bar
      repositionKeyboardBar(currentHeight);
    }
  });

  // Also listen for scroll to reposition
  window.visualViewport.addEventListener('scroll', () => {
    if (isKeyboardOpen && keyboardBar) {
      repositionKeyboardBar(window.visualViewport.height);
    }
  });
}

// ═══════════════════════════════════════════════════
// E6.3 — Floating Formatting Bar Above Keyboard
// ═══════════════════════════════════════════════════

function createKeyboardBar() {
  keyboardBar = document.createElement('div');
  keyboardBar.className = 'mobile-kb-bar';
  keyboardBar.setAttribute('role', 'toolbar');
  keyboardBar.setAttribute('aria-label', 'Formatting');

  const buttons = [
    { label: 'Bold', icon: 'format_bold', action: () => toggleFormat('bold'), title: 'Bold' },
    { label: 'Italic', icon: 'format_italic', action: () => toggleFormat('italic'), title: 'Italic' },
    { label: 'Underline', icon: 'format_underlined', action: () => toggleFormat('underline'), title: 'Underline' },
    { sep: true },
    { label: 'Indent', icon: 'format_indent_increase', action: () => execIndent('increase'), title: 'Increase indent' },
    { label: 'Outdent', icon: 'format_indent_decrease', action: () => execIndent('decrease'), title: 'Decrease indent' },
    { sep: true },
    { label: 'Done', text: 'Done', action: () => blurEditor(), title: 'Dismiss keyboard' },
  ];

  for (const btn of buttons) {
    if (btn.sep) {
      const sep = document.createElement('div');
      sep.className = 'mobile-kb-sep';
      keyboardBar.appendChild(sep);
      continue;
    }
    const el = document.createElement('button');
    el.className = 'mobile-kb-btn';
    el.title = btn.title;
    if (btn.icon) {
      el.innerHTML = `<span class="msi">${btn.icon}</span>`;
    } else if (btn.text) {
      el.textContent = btn.text;
    }
    el.addEventListener('touchstart', (e) => {
      e.preventDefault(); // Prevent focus loss
    });
    el.addEventListener('click', (e) => {
      e.preventDefault();
      btn.action();
      updateToolbarState();
    });
    keyboardBar.appendChild(el);
  }

  document.body.appendChild(keyboardBar);
}

function showKeyboardBar(viewportHeight) {
  if (!keyboardBar) createKeyboardBar();
  keyboardBar.style.display = 'flex';
  repositionKeyboardBar(viewportHeight);
}

function hideKeyboardBar() {
  if (keyboardBar) {
    keyboardBar.style.display = 'none';
  }
}

function repositionKeyboardBar(viewportHeight) {
  if (!keyboardBar) return;
  const vv = window.visualViewport;
  // Position just above the keyboard (at the bottom of the visual viewport)
  const top = vv.offsetTop + viewportHeight - 44;
  keyboardBar.style.top = `${top}px`;
  keyboardBar.style.left = `${vv.offsetLeft}px`;
  keyboardBar.style.width = `${vv.width}px`;
}

function execIndent(direction) {
  const el = getActiveElement();
  if (!el || !state.doc) return;
  const nodeId = el.dataset.nodeId;
  const isListItem = el.querySelector('.list-marker') !== null || !!el.dataset.listType;
  if (isListItem) {
    const currentLevel = parseInt(el.dataset.listLevel || '0', 10);
    const listType = el.dataset.listType || 'bullet';
    const newLevel = direction === 'increase'
      ? Math.min(8, currentLevel + 1)
      : Math.max(0, currentLevel - 1);
    try {
      state.doc.set_list_format(nodeId, listType, newLevel);
      renderNodeById(nodeId);
      markDirty();
    } catch (_) {}
  }
}

function blurEditor() {
  // Remove focus from contenteditable to dismiss keyboard
  if (document.activeElement) {
    document.activeElement.blur();
  }
}

function scrollCursorIntoView() {
  // Wait a tick for the keyboard to fully appear
  requestAnimationFrame(() => {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    if (rect.height === 0 && rect.width === 0) return;

    const vv = window.visualViewport;
    if (!vv) return;

    const visibleBottom = vv.offsetTop + vv.height - 44; // 44 = keyboard bar height
    const visibleTop = vv.offsetTop;

    if (rect.bottom > visibleBottom || rect.top < visibleTop) {
      // Find the element containing the cursor and scroll it into view
      const activeEl = getActiveElement();
      if (activeEl) {
        activeEl.scrollIntoView({ block: 'center', behavior: 'smooth' });
      }
    }
  });
}

// ═══════════════════════════════════════════════════
// E6.3 — IME Composition Events
// ═══════════════════════════════════════════════════

function setupCompositionEvents() {
  const page = $('pageContainer');
  if (!page) return;

  page.addEventListener('compositionstart', () => {
    state._composing = true;
  });

  page.addEventListener('compositionupdate', () => {
    // During composition, don't sync to WASM — let the browser manage
    // the intermediate states of CJK/IME input
    if (state.syncTimer) {
      clearTimeout(state.syncTimer);
      state.syncTimer = null;
    }
  });

  page.addEventListener('compositionend', () => {
    state._composing = false;
    // Now safe to sync the final composed text to WASM
    const el = getActiveElement();
    if (el) {
      debouncedSync(el);
    }
  });
}

// ═══════════════════════════════════════════════════
// Public: Initialize all touch/mobile features
// ═══════════════════════════════════════════════════

export function initTouch() {
  // Touch selection handles & gestures — only on touch devices
  if (isTouchDevice()) {
    createHandleElements();
    setupHandleDrag();
    setupTapGestures();
    setupLongPress();
    setupSelectionListener();
  }

  // Keyboard detection — useful on any device with visualViewport
  setupKeyboardDetection();

  // IME composition — useful on all platforms (CJK input, etc.)
  setupCompositionEvents();
}
