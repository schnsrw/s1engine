// Selection mapping: DOM ↔ WASM node IDs + character offsets
import { state, $ } from './state.js';

// Returns the nearest page-content container or the pageContainer itself
const PAGE = () => {
  // Try to find which page-content contains the current selection
  const active = document.activeElement;
  if (active && active.classList?.contains('page-content')) return active;
  let n = active;
  while (n) {
    if (n.classList?.contains('page-content')) return n;
    n = n.parentElement;
  }
  // Fallback: return pageContainer so walks still stop at the boundary
  return $('pageContainer');
};

export function findParagraphEl(node, offset) {
  let n = node;
  const page = PAGE();

  // When the node IS the page-content (or pageContainer) and offset points to a
  // child element, resolve to that child first.  This happens when the browser
  // represents a cross-paragraph selection end as (contenteditable, childIndex).
  if (n && (n === page || n === $('pageContainer')) && n.nodeType === 1 && typeof offset === 'number') {
    const children = n.childNodes;
    // offset points AFTER the child, so the last selected element is offset-1
    const targetIdx = Math.max(0, Math.min(offset - 1, children.length - 1));
    const child = children[targetIdx];
    if (child) {
      // If the child itself is a paragraph, return it
      if (child.nodeType === 1 && child.dataset?.nodeId) {
        const tag = child.tagName.toLowerCase();
        if (tag === 'p' || /^h[1-6]$/.test(tag)) return child;
      }
      // Otherwise descend into the child to find a paragraph
      n = child;
    }
  }

  while (n && n !== page && n !== $('pageContainer')) {
    if (n.nodeType === 1 && n.dataset?.nodeId) {
      const tag = n.tagName.toLowerCase();
      if (tag === 'p' || /^h[1-6]$/.test(tag)) return n;
    }
    n = n.parentNode;
  }
  return null;
}

export function findNodeEl(node, offset) {
  let n = node;
  const page = PAGE();

  // Same container-level resolution as findParagraphEl
  if (n && (n === page || n === $('pageContainer')) && n.nodeType === 1 && typeof offset === 'number') {
    const children = n.childNodes;
    const targetIdx = Math.max(0, Math.min(offset - 1, children.length - 1));
    const child = children[targetIdx];
    if (child) {
      if (child.nodeType === 1 && child.dataset?.nodeId) return child;
      n = child;
    }
  }

  while (n && n !== page && n !== $('pageContainer')) {
    if (n.nodeType === 1 && n.dataset?.nodeId) return n;
    n = n.parentNode;
  }
  return null;
}

// Check if a node is inside a non-editable element (like list markers)
export function isInsideNonEditable(node, container) {
  let n = node.nodeType === 1 ? node : node.parentElement;
  while (n && n !== container) {
    if (n.getAttribute?.('contenteditable') === 'false') return true;
    n = n.parentElement;
  }
  return false;
}

export function countCharsToPoint(container, targetNode, targetOffset) {
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
  let count = 0, node;
  while ((node = walker.nextNode())) {
    // Skip text inside non-editable elements (list markers, etc.)
    if (isInsideNonEditable(node, container)) continue;
    if (node === targetNode) {
      return count + Array.from(node.textContent.substring(0, targetOffset)).length;
    }
    count += Array.from(node.textContent).length;
  }
  if (targetNode.nodeType === 1) {
    const kids = targetNode.childNodes;
    const w2 = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
    let c = 0, n2;
    while ((n2 = w2.nextNode())) {
      if (isInsideNonEditable(n2, container)) continue;
      for (let i = 0; i < targetOffset && i < kids.length; i++) {
        if (kids[i].contains(n2) || kids[i] === n2) { c += Array.from(n2.textContent).length; break; }
      }
    }
    return c;
  }
  return count;
}

export function getSelectionInfo() {
  // When select-all is active, return the synthetic full-document selection
  if (state._selectAll && state.lastSelInfo) return state.lastSelInfo;

  // When a synthetic cross-page selection exists (from shift-click or setSelectionRange),
  // return it if the native selection hasn't meaningfully changed
  if (state.lastSelInfo && !state.lastSelInfo.collapsed) {
    const startContent = state.lastSelInfo.startEl?.closest?.('.page-content');
    const endContent = state.lastSelInfo.endEl?.closest?.('.page-content');
    if (startContent && endContent && startContent !== endContent) {
      // Cross-page synthetic selection — check if native selection is still anchored
      // in the start page (user hasn't clicked elsewhere)
      const sel = window.getSelection();
      if (sel && sel.rangeCount) {
        const anchorEl = findParagraphEl(sel.anchorNode, sel.anchorOffset);
        if (anchorEl) {
          const anchorContent = anchorEl.closest?.('.page-content');
          if (anchorContent === startContent) {
            return state.lastSelInfo; // Still valid cross-page selection
          }
        }
      }
    }
  }

  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return state.lastSelInfo;
  const range = sel.getRangeAt(0);
  const startEl = findParagraphEl(range.startContainer, range.startOffset);
  const endEl = findParagraphEl(range.endContainer, range.endOffset);
  if (!startEl) return state.lastSelInfo;

  const startOff = countCharsToPoint(startEl, range.startContainer, range.startOffset);
  const endOff = endEl ? countCharsToPoint(endEl, range.endContainer, range.endOffset) : startOff;

  const info = {
    startNodeId: startEl.dataset.nodeId,
    startOffset: startOff,
    endNodeId: endEl ? endEl.dataset.nodeId : startEl.dataset.nodeId,
    endOffset: endOff,
    collapsed: range.collapsed,
    startEl,
    endEl: endEl || startEl,
  };
  state.lastSelInfo = info;
  return info;
}

export function saveSelection() {
  // Don't overwrite synthetic select-all with native (single-page) selection
  if (state._selectAll) return;
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;
  const range = sel.getRangeAt(0);
  const startEl = findParagraphEl(range.startContainer, range.startOffset);
  if (!startEl) return;
  const endEl = findParagraphEl(range.endContainer, range.endOffset);
  const startOff = countCharsToPoint(startEl, range.startContainer, range.startOffset);
  const endOff = endEl ? countCharsToPoint(endEl, range.endContainer, range.endOffset) : startOff;
  state.lastSelInfo = {
    startNodeId: startEl.dataset.nodeId, startOffset: startOff,
    endNodeId: endEl ? endEl.dataset.nodeId : startEl.dataset.nodeId, endOffset: endOff,
    collapsed: range.collapsed, startEl, endEl: endEl || startEl,
  };
}

export function getActiveNodeId() {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return null;
  let n = sel.anchorNode;
  const page = PAGE();
  while (n && n !== page) {
    if (n.nodeType === 1 && n.dataset?.nodeId) return n.dataset.nodeId;
    n = n.parentNode;
  }
  return null;
}

export function getActiveElement() {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return null;
  let n = sel.anchorNode;
  const page = PAGE();
  while (n && n !== page) {
    if (n.nodeType === 1 && n.dataset?.nodeId) return n;
    n = n.parentNode;
  }
  return null;
}

export function getCursorOffset(el) {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return 0;
  return countCharsToPoint(el, sel.getRangeAt(0).startContainer, sel.getRangeAt(0).startOffset);
}

export function setCursorAtOffset(el, charOffset) {
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
  let counted = 0, node;
  while ((node = walker.nextNode())) {
    // Skip text inside non-editable elements (list markers, etc.)
    if (isInsideNonEditable(node, el)) continue;
    const chars = Array.from(node.textContent);
    if (counted + chars.length >= charOffset) {
      const range = document.createRange();
      let strOff = 0;
      for (let i = 0; i < charOffset - counted && i < chars.length; i++) strOff += chars[i].length;
      range.setStart(node, strOff);
      range.collapse(true);
      const sel = window.getSelection();
      sel.removeAllRanges(); sel.addRange(range);
      return;
    }
    counted += chars.length;
  }
  const range = document.createRange();
  range.selectNodeContents(el); range.collapse(false);
  const sel = window.getSelection();
  sel.removeAllRanges(); sel.addRange(range);
}

export function setCursorAtStart(el) {
  // Skip past any non-editable elements (list markers) at the start
  const marker = el.querySelector('[contenteditable="false"]');
  if (marker && el.firstElementChild === marker) {
    // Place cursor right after the marker
    const r = document.createRange();
    const afterMarker = marker.nextSibling;
    if (afterMarker) {
      if (afterMarker.nodeType === 3) {
        r.setStart(afterMarker, 0);
      } else {
        r.setStartAfter(marker);
      }
    } else {
      r.setStartAfter(marker);
    }
    r.collapse(true);
    const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
    return;
  }
  const r = document.createRange(); r.selectNodeContents(el); r.collapse(true);
  const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
}

export function setSelectionRange(startEl, startOff, endEl, endOff) {
  const resolve = (el, off) => {
    const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
    let counted = 0, node;
    while ((node = w.nextNode())) {
      if (isInsideNonEditable(node, el)) continue;
      const chars = Array.from(node.textContent);
      if (counted + chars.length >= off) {
        let strOff = 0;
        for (let i = 0; i < off - counted && i < chars.length; i++) strOff += chars[i].length;
        return { node, offset: strOff };
      }
      counted += chars.length;
    }
    return null;
  };

  // Check if start and end are in different contentEditable containers (cross-page)
  const startContent = startEl.closest?.('.page-content');
  const endContent = endEl.closest?.('.page-content');

  if (startContent && endContent && startContent !== endContent) {
    // Cross-page selection: native Range can't span contentEditable boundaries.
    // Set native selection on the start page, store synthetic selection for operations.
    const start = resolve(startEl, startOff);
    if (start) {
      startContent.focus();
      const range = document.createRange();
      range.setStart(start.node, start.offset);
      // Extend to end of start page's content
      range.setEndAfter(startContent.lastChild || startContent);
      const sel = window.getSelection();
      sel.removeAllRanges(); sel.addRange(range);
    }
    // Store synthetic selection for WASM operations
    state.lastSelInfo = {
      startNodeId: startEl.dataset.nodeId,
      startOffset: startOff,
      endNodeId: endEl.dataset.nodeId,
      endOffset: endOff,
      collapsed: false,
      startEl,
      endEl,
    };
    // Apply visual highlight to all elements in range
    const container = $('pageContainer');
    if (container) {
      let inRange = false;
      for (const pageEl of (state.pageElements || [])) {
        const content = pageEl.querySelector('.page-content') || pageEl;
        for (const el of content.children) {
          if (!el.dataset?.nodeId) continue;
          if (el.dataset.nodeId === startEl.dataset.nodeId) inRange = true;
          if (inRange) el.classList.add('select-all-highlight');
          if (el.dataset.nodeId === endEl.dataset.nodeId) { inRange = false; break; }
        }
      }
    }
    return;
  }

  const start = resolve(startEl, startOff);
  const end = resolve(endEl, endOff);
  if (!start) return;
  const range = document.createRange();
  range.setStart(start.node, start.offset);
  if (end) range.setEnd(end.node, end.offset); else range.collapse(true);
  const sel = window.getSelection();
  sel.removeAllRanges(); sel.addRange(range);
}

// Get only the editable text content (excludes list markers and other non-editable elements)
export function getEditableText(el) {
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
  let text = '';
  let node;
  while ((node = walker.nextNode())) {
    if (isInsideNonEditable(node, el)) continue;
    text += node.textContent;
  }
  return text;
}

export function isCursorAtStart(el) { return getCursorOffset(el) === 0; }
export function isCursorAtEnd(el) { return getCursorOffset(el) >= Array.from(getEditableText(el)).length; }
