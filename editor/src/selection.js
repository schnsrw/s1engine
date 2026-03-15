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

export function countCharsToPoint(container, targetNode, targetOffset) {
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
  let count = 0, node;
  while ((node = walker.nextNode())) {
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
      for (let i = 0; i < targetOffset && i < kids.length; i++) {
        if (kids[i].contains(n2) || kids[i] === n2) { c += Array.from(n2.textContent).length; break; }
      }
    }
    return c;
  }
  return count;
}

export function getSelectionInfo() {
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
  const r = document.createRange(); r.selectNodeContents(el); r.collapse(true);
  const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
}

export function setSelectionRange(startEl, startOff, endEl, endOff) {
  const resolve = (el, off) => {
    const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
    let counted = 0, node;
    while ((node = w.nextNode())) {
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
  const start = resolve(startEl, startOff);
  const end = resolve(endEl, endOff);
  if (!start) return;
  const range = document.createRange();
  range.setStart(start.node, start.offset);
  if (end) range.setEnd(end.node, end.offset); else range.collapse(true);
  const sel = window.getSelection();
  sel.removeAllRanges(); sel.addRange(range);
}

export function isCursorAtStart(el) { return getCursorOffset(el) === 0; }
export function isCursorAtEnd(el) { return getCursorOffset(el) >= Array.from(el.textContent || '').length; }
