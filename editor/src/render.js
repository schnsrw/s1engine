// Document rendering — WASM → DOM (multi-page)
import { state, $ } from './state.js';
import { setupImages } from './images.js';
import { repaginate } from './pagination.js';
import { updateUndoRedo } from './toolbar.js';
import { markDirty, updateTrackChanges, updateStatusBar } from './file.js';
import { broadcastTextSync, broadcastOp } from './collab.js';

// ═══════════════════════════════════════════════════
// E8.4: Large-document warning threshold
// ═══════════════════════════════════════════════════
const LARGE_DOC_PARAGRAPH_THRESHOLD = 500;

/**
 * Helper: query all page-content elements.
 */
function getAllPageContents() {
  const container = $('pageContainer');
  if (!container) return [];
  return Array.from(container.querySelectorAll('.page-content'));
}

/**
 * Helper: query across all page-content elements or pageContainer.
 */
function queryAllNodes(selector) {
  const container = $('pageContainer');
  if (!container) return [];
  return Array.from(container.querySelectorAll(selector));
}

export function renderDocument() {
  const { doc } = state;
  if (!doc) return;
  try {
    // Tear down any existing virtual scroll before re-rendering
    teardownVirtualScroll();

    const html = doc.to_html();
    state.ignoreInput = true;

    // Parse the HTML to extract content and header/footer
    const temp = document.createElement('div');
    temp.innerHTML = html;

    // Extract header/footer HTML from WASM output
    const hdrEl = temp.querySelector(':scope > header');
    const ftrEl = temp.querySelector(':scope > footer');
    state.docHeaderHtml = hdrEl ? hdrEl.innerHTML : '';
    state.docFooterHtml = ftrEl ? ftrEl.innerHTML : '';
    if (hdrEl) hdrEl.remove();
    if (ftrEl) ftrEl.remove();

    // Clear nodeIdToElement map (DOM is rebuilt)
    state.nodeIdToElement.clear();

    // Apply page dimensions from WASM
    applyPageDimensions();

    // Ensure pageContainer exists and is empty for fresh render
    const container = $('pageContainer');
    if (container) {
      container.innerHTML = '';
      state.pageElements = [];
      state.nodeToPage.clear();
    }

    // Run repaginate which builds per-page containers and places nodes
    repaginate();

    // If repagination didn't create pages (no page map), create a single page with all content
    if (state.pageElements.length === 0 && container) {
      const pageEl = document.createElement('div');
      pageEl.className = 'doc-page';
      pageEl.dataset.page = '1';
      const dims = state.pageDims || { marginTopPt: 72, marginBottomPt: 72, marginLeftPt: 72, marginRightPt: 72 };
      const ptToPx = 96 / 72;
      pageEl.style.width = Math.round((dims.widthPt || 612) * ptToPx) + 'px';
      pageEl.style.minHeight = Math.round((dims.heightPt || 792) * ptToPx) + 'px';

      const header = document.createElement('div');
      header.className = 'page-header';
      header.contentEditable = 'false';
      pageEl.appendChild(header);

      const content = document.createElement('div');
      content.className = 'page-content';
      content.contentEditable = 'true';
      content.spellcheck = true;
      content.lang = 'en';
      content.setAttribute('role', 'textbox');
      content.setAttribute('aria-multiline', 'true');
      content.setAttribute('aria-label', 'Page 1 content');
      // Put all content from temp into the single page
      content.innerHTML = temp.innerHTML;
      pageEl.appendChild(content);

      const footer = document.createElement('div');
      footer.className = 'page-footer';
      footer.contentEditable = 'false';
      footer.innerHTML = '<span style="display:block;text-align:center;color:#5f6368;font-size:9pt">1</span>';
      pageEl.appendChild(footer);

      container.appendChild(pageEl);
      state.pageElements = [pageEl];
    }

    // Post-render fixups across all pages
    fixEmptyBlocks();
    setupImages();
    cacheAllText();
    populateNodeIdMap();
    setupTrackChangeHandlers();

    state.ignoreInput = false;
    state.pagesRendered = false;

    updateUndoRedo();
    updateTrackChanges();
    updateStatusBar();
    checkLargeDocumentWarning();

    // Re-apply zoom level after full re-render
    applyZoom();

    // Activate virtual scrolling for large documents
    maybeInitVirtualScroll();
    // Refresh find highlights after full re-render
    state._onTextChanged?.();
  } catch (e) { console.error('Render error:', e); }
}

// ─── Per-change Track Changes popup ─────────────────
function setupTrackChangeHandlers() {
  const tcElements = queryAllNodes('[data-tc-node-id]');
  if (tcElements.length === 0) return;

  tcElements.forEach(el => {
    el.style.cursor = 'pointer';
    el.addEventListener('click', e => {
      e.stopPropagation();
      showTcPopup(el);
    });
  });
}

function dismissTcPopup() {
  const existing = document.getElementById('tcPopup');
  if (existing) existing.remove();
}

function showTcPopup(el) {
  dismissTcPopup();

  const nodeId = el.dataset.tcNodeId;
  const tcType = el.dataset.tcType;
  if (!nodeId || !state.doc) return;

  const popup = document.createElement('div');
  popup.id = 'tcPopup';
  popup.className = 'tc-popup';

  const label = document.createElement('span');
  label.className = 'tc-popup-label';
  label.textContent = tcType === 'insert' ? 'Insertion' : tcType === 'delete' ? 'Deletion' : 'Format change';
  popup.appendChild(label);

  const acceptBtn = document.createElement('button');
  acceptBtn.className = 'tc-popup-btn tc-popup-accept';
  acceptBtn.innerHTML = '&#10003; Accept';
  acceptBtn.title = 'Accept this change';
  acceptBtn.addEventListener('click', e => {
    e.stopPropagation();
    dismissTcPopup();
    try {
      state.doc.accept_change(nodeId);
      broadcastOp({ action: 'acceptChange', nodeId });
      renderDocument();
    } catch (err) { console.error('accept change:', err); }
  });
  popup.appendChild(acceptBtn);

  const rejectBtn = document.createElement('button');
  rejectBtn.className = 'tc-popup-btn tc-popup-reject';
  rejectBtn.innerHTML = '&#10007; Reject';
  rejectBtn.title = 'Reject this change';
  rejectBtn.addEventListener('click', e => {
    e.stopPropagation();
    dismissTcPopup();
    try {
      state.doc.reject_change(nodeId);
      broadcastOp({ action: 'rejectChange', nodeId });
      renderDocument();
    } catch (err) { console.error('reject change:', err); }
  });
  popup.appendChild(rejectBtn);

  document.body.appendChild(popup);

  const rect = el.getBoundingClientRect();
  const popupW = 200;
  let left = rect.left + (rect.width / 2) - (popupW / 2);
  let top = rect.bottom + 6;
  if (left < 8) left = 8;
  if (left + popupW > window.innerWidth - 8) left = window.innerWidth - popupW - 8;
  if (top + 40 > window.innerHeight) top = rect.top - 44;
  popup.style.left = left + 'px';
  popup.style.top = top + 'px';

  const dismiss = (e) => {
    if (!popup.contains(e.target)) {
      dismissTcPopup();
      document.removeEventListener('click', dismiss, true);
    }
  };
  setTimeout(() => document.addEventListener('click', dismiss, true), 0);
}

/**
 * Read page dimensions from WASM sections and store in state.
 */
export function applyPageDimensions() {
  if (!state.doc) return;
  const ptToPx = 96 / 72;
  try {
    const json = state.doc.get_sections_json();
    const sections = JSON.parse(json);
    if (sections.length > 0) {
      const sec = sections[0];
      state.pageDims = {
        widthPt: sec.pageWidth || 612,
        heightPt: sec.pageHeight || 792,
        marginTopPt: sec.marginTop || 72,
        marginBottomPt: sec.marginBottom || 72,
        marginLeftPt: sec.marginLeft || 72,
        marginRightPt: sec.marginRight || 72,
      };
    }
  } catch (_) {
    // Defaults apply
  }
}

/**
 * Apply zoom level to all page elements.
 */
function applyZoom() {
  if (!state.zoomLevel || state.zoomLevel === 100) return;
  const scale = state.zoomLevel / 100;
  for (const pageEl of state.pageElements) {
    pageEl.style.transform = `scale(${scale})`;
    pageEl.style.transformOrigin = 'top center';
  }
}

// ═══════════════════════════════════════════════════
// E8.2: nodeIdToElement map — O(1) DOM lookups
// ═══════════════════════════════════════════════════

function populateNodeIdMap() {
  const map = state.nodeIdToElement;
  map.clear();
  queryAllNodes('[data-node-id]').forEach(el => {
    map.set(el.dataset.nodeId, el);
  });
}

export function lookupNodeElement(nodeIdStr) {
  const cached = state.nodeIdToElement.get(nodeIdStr);
  if (cached && cached.isConnected) return cached;
  // Fallback: search across all pages
  const container = $('pageContainer');
  if (!container) return null;
  const el = container.querySelector(`[data-node-id="${nodeIdStr}"]`);
  if (el) state.nodeIdToElement.set(nodeIdStr, el);
  else state.nodeIdToElement.delete(nodeIdStr);
  return el;
}

// ═══════════════════════════════════════════════════
// Incremental DOM patching
// ═══════════════════════════════════════════════════

export function renderNodeById(nodeIdStr) {
  const { doc } = state;
  if (!doc) return null;
  try {
    const html = doc.render_node_html(nodeIdStr);
    const el = lookupNodeElement(nodeIdStr);
    if (!el) return null;

    const temp = document.createElement('div');
    temp.innerHTML = html;
    const newEl = temp.firstElementChild;
    if (!newEl) return null;
    if (!newEl.innerHTML.trim()) newEl.innerHTML = '<br>';

    if (el.outerHTML === newEl.outerHTML) {
      state.syncedTextCache.set(nodeIdStr, el.textContent || '');
      return el;
    }

    el.replaceWith(newEl);
    state.nodeIdToElement.set(nodeIdStr, newEl);
    newEl.querySelectorAll('[data-node-id]').forEach(child => {
      state.nodeIdToElement.set(child.dataset.nodeId, child);
    });
    setupImages(newEl);
    state.syncedTextCache.set(nodeIdStr, newEl.textContent || '');
    return newEl;
  } catch (e) { console.error('renderNode error:', e); }
  return null;
}

export function renderNodesById(nodeIds) {
  const results = new Map();
  for (const id of nodeIds) {
    results.set(id, renderNodeById(id));
  }
  return results;
}

export function fixEmptyBlocks() {
  queryAllNodes('p:empty, h1:empty, h2:empty, h3:empty, h4:empty, h5:empty, h6:empty')
    .forEach(el => { el.innerHTML = '<br>'; });
}

export function cacheAllText() {
  state.syncedTextCache.clear();
  queryAllNodes('[data-node-id]').forEach(el => {
    if (el.classList.contains('vs-placeholder')) return;
    const tag = el.tagName.toLowerCase();
    if (tag === 'p' || /^h[1-6]$/.test(tag)) {
      state.syncedTextCache.set(el.dataset.nodeId, el.textContent || '');
    }
  });
}

export function syncParagraphText(el) {
  const { doc, syncedTextCache } = state;
  if (!doc || state.ignoreInput || !el) return;
  const nodeId = el.dataset?.nodeId;
  if (!nodeId) return;
  const newText = el.textContent || '';
  if (syncedTextCache.get(nodeId) === newText) return;
  try {
    doc.set_paragraph_text(nodeId, newText);
    syncedTextCache.set(nodeId, newText);
    markDirty();
    clearFindHighlights();
    broadcastTextSync(nodeId, newText);

    const batch = state._typingBatch;
    if (batch && batch.nodeId === nodeId) {
      batch.count++;
      clearTimeout(batch.timer);
      batch.timer = setTimeout(() => { state._typingBatch = null; }, 500);
    } else {
      if (batch) clearTimeout(batch.timer);
      state._typingBatch = {
        nodeId,
        count: 1,
        timer: setTimeout(() => { state._typingBatch = null; }, 500),
      };
    }
  } catch (e) { console.error('sync error:', e); }
}

export function syncAllText() {
  if (!state.doc) return;
  queryAllNodes('[data-node-id]').forEach(el => {
    if (el.classList.contains('vs-placeholder')) return;
    const tag = el.tagName.toLowerCase();
    if (tag === 'p' || /^h[1-6]$/.test(tag)) syncParagraphText(el);
  });
}

function clearFindHighlights() {
  const container = $('pageContainer');
  if (!container) return;
  const marks = container.querySelectorAll('mark.find-highlight');
  if (marks.length === 0) return;
  marks.forEach(m => {
    const parent = m.parentNode;
    while (m.firstChild) parent.insertBefore(m.firstChild, m);
    m.remove();
    parent.normalize();
  });
}

export function debouncedSync(el) {
  clearTimeout(state.syncTimer);
  state.syncTimer = setTimeout(() => {
    syncParagraphText(el);
    state.pagesRendered = false;
    repaginate();
    updateUndoRedo();
    updateStatusBar();
    state._onTextChanged?.();
  }, 200);
}

export function renderPages() {
  const { doc } = state;
  if (!doc) return;
  try {
    const html = doc.to_paginated_html();
    const container = $('pagesView');
    const pageCount = (html.match(/class="s1-page"/g) || []).length;
    container.innerHTML =
      '<div class="pages-nav">' +
        '<button id="prevPage">&#9664; Prev</button>' +
        '<span>' + pageCount + ' page' + (pageCount !== 1 ? 's' : '') + '</span>' +
        '<button id="nextPage">Next &#9654;</button>' +
      '</div>' + html;
    state.pagesRendered = true;
    const pages = container.querySelectorAll('.s1-page');
    let cur = 0;
    const prev = container.querySelector('#prevPage');
    const next = container.querySelector('#nextPage');
    if (prev) prev.onclick = () => { if (cur > 0) pages[--cur].scrollIntoView({ behavior: 'smooth', block: 'start' }); };
    if (next) next.onclick = () => { if (cur < pages.length - 1) pages[++cur].scrollIntoView({ behavior: 'smooth', block: 'start' }); };
  } catch (e) {
    $('pagesView').innerHTML = '<div style="padding:32px;color:#ff6b6b">Layout error: ' + e.message + '</div>';
  }
}

export function renderText() {
  const { doc } = state;
  if (!doc) return;
  try { $('textContent').textContent = doc.to_plain_text(); }
  catch (e) { $('textContent').textContent = 'Error: ' + e.message; }
}

// ═══════════════════════════════════════════════════
// E8.4: Large-document warning
// ═══════════════════════════════════════════════════

function checkLargeDocumentWarning() {
  const paraCount = queryAllNodes(
    'p[data-node-id], h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]'
  ).length;
  const info = $('statusInfo');
  if (paraCount > LARGE_DOC_PARAGRAPH_THRESHOLD && info) {
    info._userMsg = true;
    info.textContent = `Large document: ${paraCount} paragraphs. Performance may be affected.`;
    setTimeout(() => { info._userMsg = false; updateStatusBar(); }, 5000);
  }
}

// ═══════════════════════════════════════════════════
// VIRTUAL SCROLLING — for documents with 100+ blocks
// ═══════════════════════════════════════════════════

const VS_THRESHOLD = 100;
const VS_BUFFER = 20;

function getBlockElements() {
  const results = [];
  for (const pageEl of state.pageElements) {
    const contentEl = pageEl.querySelector('.page-content');
    if (!contentEl) continue;
    for (const el of contentEl.children) {
      const tag = el.tagName.toLowerCase();
      if (tag === 'p' || /^h[1-6]$/.test(tag) || tag === 'table' ||
          tag === 'ul' || tag === 'ol' || tag === 'hr' ||
          el.dataset.nodeId || el.classList.contains('vs-placeholder')) {
        results.push(el);
      }
    }
  }
  return results;
}

function isVirtualScrollSuppressed() {
  const findBar = $('findBar');
  if (findBar && findBar.classList.contains('show')) return true;
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0 && !sel.isCollapsed) {
    const range = sel.getRangeAt(0);
    const container = $('pageContainer');
    if (container) {
      const startBlock = findAncestorBlock(range.startContainer, container);
      const endBlock = findAncestorBlock(range.endContainer, container);
      if (startBlock && endBlock && startBlock !== endBlock) return true;
    }
  }
  return false;
}

function findAncestorBlock(node, root) {
  let n = node;
  while (n && n !== root) {
    if (n.nodeType === 1 && n.dataset?.nodeId) {
      const tag = n.tagName.toLowerCase();
      if (tag === 'p' || /^h[1-6]$/.test(tag) || tag === 'table') return n;
    }
    n = n.parentNode;
  }
  return null;
}

function maybeInitVirtualScroll() {
  const blocks = getBlockElements();
  if (blocks.length < VS_THRESHOLD) return;
  if (isVirtualScrollSuppressed()) return;
  initVirtualScroll(blocks);
}

function initVirtualScroll(blocks) {
  const canvas = $('editorCanvas');
  if (!canvas) return;

  const entries = blocks.map(el => ({
    el,
    nodeId: el.dataset?.nodeId || null,
    height: Math.max(el.getBoundingClientRect().height, 20),
    html: el.outerHTML,
    visible: true,
  }));

  const bufferPx = VS_BUFFER * 30;
  const observer = new IntersectionObserver((ioEntries) => {
    if (!state.virtualScroll) return;
    if (isVirtualScrollSuppressed()) return;
    const vs = state.virtualScroll;

    for (const ioe of ioEntries) {
      const el = ioe.target;
      const idx = vs.indexMap.get(el);
      if (idx === undefined) continue;
      const entry = vs.entries[idx];
      if (!entry) continue;

      if (ioe.isIntersecting && !entry.visible) {
        restoreBlock(entry, idx);
      } else if (!ioe.isIntersecting && entry.visible) {
        collapseBlock(entry, idx, vs);
      }
    }
  }, {
    root: canvas,
    rootMargin: `${bufferPx}px 0px ${bufferPx}px 0px`,
    threshold: 0,
  });

  const indexMap = new WeakMap();
  entries.forEach((entry, i) => {
    indexMap.set(entry.el, i);
    observer.observe(entry.el);
  });

  state.virtualScroll = { entries, observer, indexMap };
}

function collapseBlock(entry, idx, vs) {
  if (!entry.visible || !entry.el || !entry.el.parentNode) return;
  if (isVirtualScrollSuppressed()) return;
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    if (entry.el.contains(range.startContainer) || entry.el.contains(range.endContainer)) return;
  }

  if (entry.nodeId) {
    const tag = entry.el.tagName?.toLowerCase() || '';
    if (tag === 'p' || /^h[1-6]$/.test(tag)) syncParagraphText(entry.el);
  }

  entry.html = entry.el.outerHTML;
  entry.height = Math.max(entry.el.getBoundingClientRect().height, 20);

  const placeholder = document.createElement('div');
  placeholder.className = 'vs-placeholder';
  placeholder.style.height = entry.height + 'px';
  if (entry.nodeId) placeholder.dataset.nodeId = entry.nodeId;
  placeholder.dataset.vsIndex = String(idx);

  entry.el.replaceWith(placeholder);
  if (entry.nodeId) state.nodeIdToElement.set(entry.nodeId, placeholder);
  vs.observer.unobserve(entry.el);
  entry.el = placeholder;
  vs.indexMap.set(placeholder, idx);
  vs.observer.observe(placeholder);
  entry.visible = false;
}

function restoreBlock(entry, idx) {
  if (entry.visible || !entry.el || !entry.el.parentNode) return;
  const vs = state.virtualScroll;
  let newEl = null;

  if (entry.nodeId && state.doc) {
    try {
      const html = state.doc.render_node_html(entry.nodeId);
      const temp = document.createElement('div');
      temp.innerHTML = html;
      newEl = temp.firstElementChild;
    } catch (_) {}
  }

  if (!newEl) {
    if (!entry.html) return;
    const temp = document.createElement('div');
    temp.innerHTML = entry.html;
    newEl = temp.firstElementChild;
    if (!newEl) return;
  }

  vs.observer.unobserve(entry.el);
  entry.el.replaceWith(newEl);
  entry.el = newEl;
  vs.indexMap.set(newEl, idx);
  vs.observer.observe(newEl);
  entry.visible = true;

  if (entry.nodeId) {
    state.nodeIdToElement.set(entry.nodeId, newEl);
    newEl.querySelectorAll('[data-node-id]').forEach(child => {
      state.nodeIdToElement.set(child.dataset.nodeId, child);
    });
  }

  if (!newEl.innerHTML.trim()) newEl.innerHTML = '<br>';
  setupImages(newEl);
  if (entry.nodeId) state.syncedTextCache.set(entry.nodeId, newEl.textContent || '');
}

function teardownVirtualScroll() {
  if (!state.virtualScroll) return;
  state.virtualScroll.observer.disconnect();
  state.virtualScroll = null;
}
