// Document rendering — WASM → DOM (multi-page) + optional Canvas mode
import { state, $ } from './state.js';
import { setupImages } from './images.js';
import { repaginate } from './pagination.js';
import { updateUndoRedo } from './toolbar.js';
import { markDirty, updateTrackChanges, updateStatusBar } from './file.js';
import { broadcastTextSync, broadcastOp } from './collab.js';
import { getEditableText } from './selection.js';
import { getFontDb } from './fonts.js';
import { renderDocumentEquations, refreshPageThumbnails, isSpellCheckEnabled, refreshTrackChangesPanel } from './toolbar-handlers.js';
import {
  isCanvasMode,
  setCanvasMode,
  initCanvasRenderer,
  renderDocumentCanvas,
  destroyCanvasRenderer,
} from './canvas-render.js';

// ═══════════════════════════════════════════════════
// E8.4: Large-document warning thresholds
// ═══════════════════════════════════════════════════
const LARGE_DOC_PARAGRAPH_THRESHOLD = 500;
const LARGE_DOC_PAGE_THRESHOLD = 500;
const WASM_MEMORY_WARNING_BYTES = 50 * 1024 * 1024; // 50MB

// ═══════════════════════════════════════════════════
// E8.1: Virtual scroll buffer zone constants
// ═══════════════════════════════════════════════════
const BUFFER_PAGES = 2;        // Render 2 pages of content above/below viewport
const DEFAULT_PAGE_HEIGHT_PX = 1056;   // Letter page height in px (792pt * 96/72)

/** Get the current page height in px from the document or fall back to Letter. */
function getPageHeightPx() {
  if (state.pageDims && state.pageDims.height) {
    return Math.round(state.pageDims.height * 96 / 72);
  }
  return DEFAULT_PAGE_HEIGHT_PX;
}

// E8.4: Transparent 1x1 pixel placeholder for off-screen images
const PLACEHOLDER_IMG_SRC = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';

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

  // F4.3: Canvas rendering mode — render via HTML5 Canvas instead of DOM
  if (isCanvasMode()) {
    try {
      const container = $('pageContainer');
      if (container) {
        // Canvas mode replaces DOM content entirely
        const ok = renderDocumentCanvas(container);
        if (ok) {
          updateUndoRedo();
          updateStatusBar();
          return;
        }
      }
      // If canvas rendering fails, fall through to DOM rendering
      console.warn('Canvas render failed, falling back to DOM rendering');
    } catch (e) {
      console.error('Canvas render error, falling back to DOM:', e);
    }
  }

  try {
    // Tear down any existing virtual scroll before re-rendering
    teardownVirtualScroll();

    // E8.3: Full re-render always marks layout dirty and invalidates cache
    state._layoutDirty = true;
    state._layoutCache = null;

    const html = doc.to_html();
    state.ignoreInput = true;

    // Parse the HTML to extract content and header/footer
    const temp = document.createElement('div');
    temp.innerHTML = html;

    // Extract header/footer HTML from WASM output
    // Supports "different first page" — first-page headers/footers are separate
    const headers = temp.querySelectorAll(':scope > header');
    const footers = temp.querySelectorAll(':scope > footer');
    state.docHeaderHtml = '';
    state.docFooterHtml = '';
    state.docFirstPageHeaderHtml = '';
    state.docFirstPageFooterHtml = '';
    state.hasDifferentFirstPage = false;
    headers.forEach(h => {
      if (h.dataset.headerType === 'first') {
        state.docFirstPageHeaderHtml = h.innerHTML;
        state.hasDifferentFirstPage = true;
      } else {
        state.docHeaderHtml = h.innerHTML;
      }
      h.remove();
    });
    footers.forEach(f => {
      if (f.dataset.footerType === 'first') {
        state.docFirstPageFooterHtml = f.innerHTML;
        state.hasDifferentFirstPage = true;
      } else {
        state.docFooterHtml = f.innerHTML;
      }
      f.remove();
    });

    // UXP-10: Extract footnotes/endnotes sections from WASM output and store for placement
    const footnotesSection = temp.querySelector(':scope > .footnotes-section');
    const endnotesSection = temp.querySelector(':scope > .endnotes-section');
    state._footnotesHtml = footnotesSection ? footnotesSection.outerHTML : '';
    state._endnotesHtml = endnotesSection ? endnotesSection.outerHTML : '';
    if (footnotesSection) footnotesSection.remove();
    if (endnotesSection) endnotesSection.remove();

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
      header.className = 'page-header hf-hoverable';
      header.contentEditable = 'false';
      header.setAttribute('data-hf-kind', 'header');
      header.setAttribute('title', 'Double-click to edit header');
      if (state.docHeaderHtml) {
        header.innerHTML = state.docHeaderHtml;
        // Substitute page number fields for page 1 of 1
        header.querySelectorAll('[data-field]').forEach(el => {
          if (el.dataset.field === 'PageNumber' || el.dataset.field === 'PAGE') el.textContent = '1';
          else if (el.dataset.field === 'PageCount' || el.dataset.field === 'NUMPAGES') el.textContent = '1';
        });
      }
      pageEl.appendChild(header);

      const content = document.createElement('div');
      content.className = 'page-content';
      content.contentEditable = 'true';
      content.spellcheck = isSpellCheckEnabled();
      content.lang = 'en';
      content.setAttribute('role', 'textbox');
      content.setAttribute('aria-multiline', 'true');
      content.setAttribute('aria-label', 'Page 1 content');
      // Put all content from temp into the single page
      content.innerHTML = temp.innerHTML;
      pageEl.appendChild(content);

      const footer = document.createElement('div');
      footer.className = 'page-footer hf-hoverable';
      footer.contentEditable = 'false';
      footer.setAttribute('data-hf-kind', 'footer');
      footer.setAttribute('title', 'Double-click to edit footer');
      if (state.docFooterHtml) {
        footer.innerHTML = state.docFooterHtml;
        // Substitute page number fields for page 1 of 1
        footer.querySelectorAll('[data-field]').forEach(el => {
          if (el.dataset.field === 'PageNumber' || el.dataset.field === 'PAGE') el.textContent = '1';
          else if (el.dataset.field === 'PageCount' || el.dataset.field === 'NUMPAGES') el.textContent = '1';
        });
      }
      pageEl.appendChild(footer);

      container.appendChild(pageEl);
      state.pageElements = [pageEl];
    }

    // UXP-10: Place footnotes/endnotes sections on the last page
    placeFootnoteSections();

    // Post-render fixups across all pages
    fixEmptyBlocks();
    setupImages();
    cacheAllText();
    populateNodeIdMap();
    setupTrackChangeHandlers();
    setupTOCHandlers();
    setupFootnoteHandlers();

    // E9.3: Render equations via KaTeX
    renderDocumentEquations();

    // E9.6: Auto-number footnotes/endnotes
    autoNumberFootnotes();

    // E9.5: Apply TOC styling
    applyTocStyling();

    state.ignoreInput = false;
    state.pagesRendered = false;

    updateUndoRedo();
    updateTrackChanges();
    refreshTrackChangesPanel();
    updateStatusBar();
    checkLargeDocumentWarning();

    // Re-apply zoom level after full re-render
    applyZoom();

    // Activate virtual scrolling for large documents
    maybeInitVirtualScroll();
    // Refresh find highlights after full re-render
    state._onTextChanged?.();
    // Update page thumbnails in sidebar
    refreshPageThumbnails();
  } catch (e) { console.error('Render error:', e); }
}

// ─── Per-change Track Changes popup (event delegation) ─────────────────
let _tcDelegationSetup = false;
function setupTrackChangeHandlers() {
  // Style all tc elements with pointer cursor
  queryAllNodes('[data-tc-node-id]').forEach(el => {
    el.style.cursor = 'pointer';
  });

  // Only set up delegation once — no listener accumulation
  if (_tcDelegationSetup) return;
  const container = $('pageContainer');
  if (!container) return;
  _tcDelegationSetup = true;
  container.addEventListener('click', e => {
    const tcEl = e.target.closest?.('[data-tc-node-id]');
    if (tcEl) {
      e.stopPropagation();
      showTcPopup(tcEl);
    }
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
  // Apply page dimensions as CSS custom properties so .doc-page sizes adapt
  const dims = state.pageDims || { widthPt: 612, heightPt: 792 };
  const widthPx = Math.round(dims.widthPt * ptToPx);
  const heightPx = Math.round(dims.heightPt * ptToPx);
  document.documentElement.style.setProperty('--page-width', widthPx + 'px');
  document.documentElement.style.setProperty('--page-height', heightPx + 'px');
}

/**
 * Apply zoom level to all page elements.
 */
function applyZoom() {
  const container = $('pageContainer');
  if (!container) return;
  if (!state.zoomLevel || state.zoomLevel === 100) {
    container.style.zoom = '';
    return;
  }
  // Use CSS zoom instead of transform:scale — zoom adjusts layout coordinates
  // so click handlers, selection, and resize all work correctly at any zoom level.
  container.style.zoom = (state.zoomLevel / 100);
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

    // Skip DOM replacement if innerHTML matches (cheaper than outerHTML —
    // avoids serializing the wrapper element's tag and attributes).
    const newInner = newEl.innerHTML;
    if (el.innerHTML === newInner) {
      state.syncedTextCache.set(nodeIdStr, getEditableText(el));
      return el;
    }

    // E8.2: Try attribute-only patch — if the element structure is the same
    // and only attributes/classes differ, patch in-place instead of full replace.
    if (el.tagName === newEl.tagName && tryPatchAttributes(el, newEl)) {
      state.syncedTextCache.set(nodeIdStr, getEditableText(el));
      return el;
    }

    el.replaceWith(newEl);
    state.nodeIdToElement.set(nodeIdStr, newEl);
    newEl.querySelectorAll('[data-node-id]').forEach(child => {
      state.nodeIdToElement.set(child.dataset.nodeId, child);
    });
    setupImages(newEl);
    state.syncedTextCache.set(nodeIdStr, getEditableText(newEl));
    return newEl;
  } catch (e) { console.error('renderNode error:', e); }
  return null;
}

/**
 * E8.2: Try to patch only attributes/styles on existing element instead of
 * replacing the entire DOM subtree. Returns true if patch was sufficient.
 * Only applies when child structure matches (same number of children, same tags).
 */
function tryPatchAttributes(existing, incoming) {
  // Only attempt patch on simple paragraph/heading elements
  const tag = existing.tagName.toLowerCase();
  if (!(tag === 'p' || /^h[1-6]$/.test(tag))) return false;

  // Check if child element count matches
  if (existing.children.length !== incoming.children.length) return false;

  // Check if text content matches (if text differs, need full replace for runs)
  if (existing.textContent !== incoming.textContent) return false;

  // Patch wrapper element attributes
  patchElementAttributes(existing, incoming);

  // Patch each child span/run element's attributes and styles
  for (let i = 0; i < existing.children.length; i++) {
    const existChild = existing.children[i];
    const incomChild = incoming.children[i];
    if (existChild.tagName !== incomChild.tagName) return false;
    patchElementAttributes(existChild, incomChild);
  }

  return true;
}

/**
 * E8.2: Patch attributes from source element onto target element.
 * Updates style, class, and data attributes without replacing the node.
 */
function patchElementAttributes(target, source) {
  // Sync className
  if (target.className !== source.className) {
    target.className = source.className;
  }
  // Sync inline style
  if (target.style.cssText !== source.style.cssText) {
    target.style.cssText = source.style.cssText;
  }
  // Sync data attributes
  const targetAttrs = target.attributes;
  const sourceAttrs = source.attributes;
  // Remove attributes not in source
  const toRemove = [];
  for (let i = 0; i < targetAttrs.length; i++) {
    const name = targetAttrs[i].name;
    if (name === 'class' || name === 'style') continue;
    if (!source.hasAttribute(name)) toRemove.push(name);
  }
  for (const name of toRemove) target.removeAttribute(name);
  // Add/update attributes from source
  for (let i = 0; i < sourceAttrs.length; i++) {
    const { name, value } = sourceAttrs[i];
    if (name === 'class' || name === 'style') continue;
    if (target.getAttribute(name) !== value) target.setAttribute(name, value);
  }
}

export function renderNodesById(nodeIds) {
  const results = new Map();
  for (const id of nodeIds) {
    results.set(id, renderNodeById(id));
  }
  return results;
}

export function fixEmptyBlocks() {
  queryAllNodes('p, h1, h2, h3, h4, h5, h6').forEach(el => {
    // Match truly empty blocks and whitespace-only blocks that need a <br>
    // for contenteditable to maintain the block height
    if (!el.textContent.trim() && !el.querySelector('img, br, table')) {
      el.innerHTML = '<br>';
    }
  });
}

export function cacheAllText() {
  state.syncedTextCache.clear();
  // Use pageElements for faster traversal (skip non-page DOM)
  for (const pageEl of state.pageElements) {
    const contentEl = pageEl.querySelector('.page-content');
    if (!contentEl) continue;
    const children = contentEl.children;
    for (let i = 0; i < children.length; i++) {
      const el = children[i];
      if (!el.dataset?.nodeId || el.classList.contains('vs-placeholder')) continue;
      const tag = el.tagName.toLowerCase();
      if (tag === 'p' || /^h[1-6]$/.test(tag)) {
        state.syncedTextCache.set(el.dataset.nodeId, getEditableText(el));
      }
    }
  }
}

export function syncParagraphText(el) {
  const { doc, syncedTextCache } = state;
  if (!doc || state.ignoreInput || !el) return;
  const nodeId = el.dataset?.nodeId;
  if (!nodeId) return;
  // Use getEditableText to exclude list marker text from sync
  const newText = getEditableText(el);
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
    // E6.3: Don't sync during IME composition — wait for compositionend
    if (state._composing) return;
    syncParagraphText(el);
    state.pagesRendered = false;
    // E8.3: Debounce layout — repagination is deferred and batched.
    // Text-only edits within a single paragraph use the debounced path
    // so rapid typing doesn't trigger layout on every keystroke.
    debouncedRepaginate();
    updateUndoRedo();
    updateStatusBar();
    state._onTextChanged?.();
  }, 200);
}

/**
 * E8.3: Debounced repagination — batches rapid edits so layout doesn't
 * re-run on every keystroke. Uses a 300ms debounce window.
 */
function debouncedRepaginate() {
  clearTimeout(state._layoutDebounceTimer);
  state._layoutDebounceTimer = setTimeout(() => {
    repaginate();
  }, 300);
}

/**
 * E8.3: Mark layout as dirty — called on structural changes (insert/delete
 * node, paste, etc.) to ensure next render re-computes pagination.
 * Text-only edits within a paragraph should NOT call this.
 */
export function markLayoutDirty() {
  state._layoutDirty = true;
  state._layoutCache = null;
}

export function renderPages() {
  const { doc } = state;
  if (!doc) return;
  try {
    // E8.3: Use cached paginated HTML if layout is not dirty
    let html;
    if (!state._layoutDirty && state._layoutCache) {
      html = state._layoutCache;
    } else {
      const fontDb = getFontDb();
      if (fontDb && fontDb.font_count() > 0) {
        html = doc.to_paginated_html_with_fonts(fontDb);
      } else {
        html = doc.to_paginated_html();
      }
      state._layoutCache = html;
      state._layoutDirty = false;
    }

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

    // E8.3: Lazy page rendering — use IntersectionObserver to defer rendering
    // of off-screen pages. Pages outside viewport are shown as empty page-shaped divs.
    setupLazyPageRendering(container, pages);
  } catch (e) {
    $('pagesView').innerHTML = '<div style="padding:32px;color:#ff6b6b">Layout error: ' + e.message + '</div>';
  }
}

/**
 * E8.3: Set up IntersectionObserver for lazy page rendering in Pages view.
 * Off-screen pages have their content hidden until they enter the viewport.
 */
function setupLazyPageRendering(container, pages) {
  // Clean up previous observer
  if (state._lazyPageObserver) {
    state._lazyPageObserver.disconnect();
    state._lazyPageObserver = null;
  }

  // Only activate lazy rendering for documents with many pages
  if (pages.length < 10) return;

  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      const page = entry.target;
      if (entry.isIntersecting) {
        // Restore page content
        if (page.dataset.lazyHidden === 'true') {
          page.style.visibility = '';
          page.style.contentVisibility = '';
          page.dataset.lazyHidden = 'false';
        }
      } else {
        // Hide off-screen page content to reduce paint cost
        if (page.dataset.lazyHidden !== 'true') {
          page.style.contentVisibility = 'hidden';
          page.dataset.lazyHidden = 'true';
        }
      }
    }
  }, {
    root: container.closest('.canvas') || null,
    rootMargin: `${getPageHeightPx() * BUFFER_PAGES}px 0px ${getPageHeightPx() * BUFFER_PAGES}px 0px`,
    threshold: 0,
  });

  pages.forEach(page => observer.observe(page));
  state._lazyPageObserver = observer;
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
  const pageCount = state.pageElements?.length || 1;
  const info = $('statusInfo');

  // E8.4: Check WASM memory usage if available
  let wasmMemoryBytes = 0;
  try {
    if (typeof WebAssembly !== 'undefined' && WebAssembly.Memory) {
      // Try to get WASM memory from the module instance
      const mem = state.engine?.memory?.buffer || state.doc?.memory?.buffer;
      if (mem) wasmMemoryBytes = mem.byteLength;
    }
  } catch (_) {}

  const warnings = [];
  if (paraCount > LARGE_DOC_PARAGRAPH_THRESHOLD) {
    warnings.push(`${paraCount} paragraphs`);
  }
  if (pageCount > LARGE_DOC_PAGE_THRESHOLD) {
    warnings.push(`${pageCount} pages`);
  }
  if (wasmMemoryBytes > WASM_MEMORY_WARNING_BYTES) {
    warnings.push(`${Math.round(wasmMemoryBytes / (1024 * 1024))}MB memory`);
  }

  if (warnings.length > 0 && info && !state._perfWarningShown) {
    state._perfWarningShown = true;
    info._userMsg = true;
    info.textContent = `Large document (${warnings.join(', ')}). Performance may be affected.`;
    info.classList.add('perf-warning');
    setTimeout(() => {
      info._userMsg = false;
      info.classList.remove('perf-warning');
      updateStatusBar();
    }, 6000);
  }
}

// ═══════════════════════════════════════════════════
// E9.5: TOC Click-to-Scroll & Update Button
// ═══════════════════════════════════════════════════

let _tocDelegationSetup = false;

function setupTOCHandlers() {
  if (_tocDelegationSetup) return;
  const container = $('pageContainer');
  if (!container) return;
  _tocDelegationSetup = true;

  // Event delegation for TOC entry clicks — scroll to the matching heading
  container.addEventListener('click', e => {
    const tocEntry = e.target.closest?.('.toc-entry');
    if (!tocEntry) return;
    e.preventDefault();
    e.stopPropagation();

    const entryText = tocEntry.textContent.trim();
    if (!entryText) return;

    // Find the heading that matches the TOC entry text
    const headings = container.querySelectorAll('h1[data-node-id], h2[data-node-id], h3[data-node-id], h4[data-node-id], h5[data-node-id], h6[data-node-id]');
    for (const heading of headings) {
      if (heading.textContent.trim() === entryText) {
        heading.scrollIntoView({ behavior: 'smooth', block: 'center' });
        // Briefly flash the heading to indicate where cursor landed
        heading.style.transition = 'background 0.3s';
        heading.style.background = 'rgba(26, 115, 232, 0.12)';
        setTimeout(() => { heading.style.background = ''; }, 1200);
        return;
      }
    }
  });

  // "Update Table of Contents" button inside the TOC container
  container.addEventListener('click', e => {
    const updateBtn = e.target.closest?.('.toc-update-btn');
    if (!updateBtn) return;
    e.preventDefault();
    e.stopPropagation();

    const tocEl = updateBtn.closest('.doc-toc');
    if (!tocEl || !state.doc) return;
    const tocNodeId = tocEl.dataset?.nodeId;
    if (!tocNodeId) return;

    try {
      state.doc.update_table_of_contents(tocNodeId);
      renderDocument();
    } catch (err) {
      console.error('update TOC:', err);
    }
  });

  // Inject "Update" button into existing TOC elements
  injectTOCUpdateButtons(container);
}

function injectTOCUpdateButtons(container) {
  const tocElements = container.querySelectorAll('.doc-toc');
  tocElements.forEach(tocEl => {
    // Check if update button already exists
    if (tocEl.querySelector('.toc-update-btn')) return;
    const titleEl = tocEl.querySelector('.doc-toc-title');
    if (titleEl) {
      const btn = document.createElement('button');
      btn.className = 'toc-update-btn';
      btn.textContent = 'Update';
      btn.title = 'Update Table of Contents';
      btn.type = 'button';
      titleEl.appendChild(btn);
    }
  });
}

// ═══════════════════════════════════════════════════
// UXP-10: Place Footnote/Endnote Sections on Pages
// ═══════════════════════════════════════════════════

/**
 * Place footnotes section at the bottom of the last page's content area,
 * and endnotes at the end of the last page.
 *
 * Footnotes appear with a thin separator line above them at the bottom of
 * the page content. Endnotes appear after all body content on the last page.
 */
function placeFootnoteSections() {
  const footnotesHtml = state._footnotesHtml || '';
  const endnotesHtml = state._endnotesHtml || '';
  if (!footnotesHtml && !endnotesHtml) return;

  // Remove any existing footnote/endnote sections from all pages
  const container = $('pageContainer');
  if (!container) return;
  container.querySelectorAll('.footnotes-section, .endnotes-section').forEach(el => el.remove());

  // Find the last page with content
  const pages = state.pageElements;
  if (!pages || pages.length === 0) return;
  const lastPage = pages[pages.length - 1];
  const contentEl = lastPage.querySelector('.page-content');
  if (!contentEl) return;

  // Insert footnotes at the end of the last page's content
  if (footnotesHtml) {
    const temp = document.createElement('div');
    temp.innerHTML = footnotesHtml;
    const fnSection = temp.firstElementChild;
    if (fnSection) {
      contentEl.appendChild(fnSection);
    }
  }

  // Insert endnotes after footnotes
  if (endnotesHtml) {
    const temp = document.createElement('div');
    temp.innerHTML = endnotesHtml;
    const enSection = temp.firstElementChild;
    if (enSection) {
      contentEl.appendChild(enSection);
    }
  }
}

// ═══════════════════════════════════════════════════
// E9.6: Footnote/Endnote Click Navigation
// ═══════════════════════════════════════════════════

let _footnoteDelegationSetup = false;

function setupFootnoteHandlers() {
  if (_footnoteDelegationSetup) return;
  const container = $('pageContainer');
  if (!container) return;
  _footnoteDelegationSetup = true;

  container.addEventListener('click', e => {
    // Click footnote reference marker in text -> scroll to footnote body
    const fnRef = e.target.closest?.('.footnote-ref, [data-footnote-ref]');
    if (fnRef) {
      e.preventDefault();
      e.stopPropagation();
      const fnId = fnRef.dataset.footnoteRef || fnRef.dataset.footnoteId || fnRef.textContent.trim();
      if (fnId) {
        // Look for matching footnote body
        const fnBody = container.querySelector(
          `.footnote-body[data-footnote-id="${fnId}"], ` +
          `.endnote-body[data-endnote-id="${fnId}"], ` +
          `[data-footnote-body="${fnId}"]`
        );
        if (fnBody) {
          fnBody.scrollIntoView({ behavior: 'smooth', block: 'center' });
          fnBody.style.transition = 'background 0.3s';
          fnBody.style.background = 'rgba(26, 115, 232, 0.12)';
          setTimeout(() => { fnBody.style.background = ''; }, 1200);
        }
      }
      return;
    }

    // Click footnote body number -> scroll back to reference in text
    const fnBody = e.target.closest?.('.footnote-body, .endnote-body, [data-footnote-body]');
    if (fnBody) {
      const fnId = fnBody.dataset.footnoteId || fnBody.dataset.endnoteId || fnBody.dataset.footnoteBody;
      if (fnId) {
        const fnRef2 = container.querySelector(
          `.footnote-ref[data-footnote-ref="${fnId}"], ` +
          `.footnote-ref[data-footnote-id="${fnId}"], ` +
          `[data-footnote-ref="${fnId}"]`
        );
        if (fnRef2) {
          e.preventDefault();
          e.stopPropagation();
          fnRef2.scrollIntoView({ behavior: 'smooth', block: 'center' });
          fnRef2.style.transition = 'background 0.3s';
          fnRef2.style.background = 'rgba(26, 115, 232, 0.12)';
          setTimeout(() => { fnRef2.style.background = ''; }, 1200);
        }
      }
    }
  });
}

// ═══════════════════════════════════════════════════
// E9.6: Footnote/Endnote Auto-Numbering
// ═══════════════════════════════════════════════════

function autoNumberFootnotes() {
  const container = $('pageContainer');
  if (!container) return;

  // Number footnote references sequentially
  const fnRefs = container.querySelectorAll('.footnote-ref, [data-footnote-ref]');
  fnRefs.forEach((ref, i) => {
    ref.textContent = String(i + 1);
    ref.title = `Footnote ${i + 1}`;
  });

  // Number footnote bodies to match
  const fnBodies = container.querySelectorAll('.footnote-body, [data-footnote-body]');
  fnBodies.forEach((body, i) => {
    const numEl = body.querySelector('.footnote-number');
    if (numEl) {
      numEl.textContent = String(i + 1) + '.';
    } else {
      // Prepend a number if none exists
      const num = document.createElement('span');
      num.className = 'footnote-number';
      num.textContent = String(i + 1) + '.';
      num.style.fontWeight = '600';
      num.style.marginRight = '4px';
      body.insertBefore(num, body.firstChild);
    }
  });

  // Number endnote references
  const enRefs = container.querySelectorAll('.endnote-ref, [data-endnote-ref]');
  enRefs.forEach((ref, i) => {
    ref.textContent = String(i + 1);
    ref.title = `Endnote ${i + 1}`;
  });

  const enBodies = container.querySelectorAll('.endnote-body, [data-endnote-body]');
  enBodies.forEach((body, i) => {
    const numEl = body.querySelector('.endnote-number');
    if (numEl) {
      numEl.textContent = String(i + 1) + '.';
    } else {
      const num = document.createElement('span');
      num.className = 'endnote-number';
      num.textContent = String(i + 1) + '.';
      num.style.fontWeight = '600';
      num.style.marginRight = '4px';
      body.insertBefore(num, body.firstChild);
    }
  });
}

// ═══════════════════════════════════════════════════
// E9.5: TOC Styling
// ═══════════════════════════════════════════════════

export function applyTocStyling() {
  const container = $('pageContainer');
  if (!container) return;
  const style = state.tocStyle || 'default';

  container.querySelectorAll('.toc-entry').forEach(entry => {
    // Remove any previous leader styling
    entry.classList.remove('toc-dotted', 'toc-dashed', 'toc-no-page-numbers');

    // Get the page number element (if any)
    const pageNumEl = entry.querySelector('.toc-page-num');

    switch (style) {
      case 'dotted':
        entry.classList.add('toc-dotted');
        break;
      case 'dashed':
        entry.classList.add('toc-dashed');
        break;
      case 'no-page-numbers':
        entry.classList.add('toc-no-page-numbers');
        if (pageNumEl) pageNumEl.style.display = 'none';
        break;
      default:
        // Default style — show page numbers, no leaders
        if (pageNumEl) pageNumEl.style.display = '';
        break;
    }
  });

  // Inject TOC style dropdown alongside Update button
  container.querySelectorAll('.doc-toc').forEach(tocEl => {
    if (tocEl.querySelector('.toc-style-select')) return;
    const titleEl = tocEl.querySelector('.doc-toc-title');
    if (!titleEl) return;

    const sel = document.createElement('select');
    sel.className = 'toc-style-select';
    sel.title = 'TOC display style';
    sel.innerHTML = `
      <option value="default"${style === 'default' ? ' selected' : ''}>Default</option>
      <option value="dotted"${style === 'dotted' ? ' selected' : ''}>Dotted Leaders</option>
      <option value="dashed"${style === 'dashed' ? ' selected' : ''}>Dashed Leaders</option>
      <option value="no-page-numbers"${style === 'no-page-numbers' ? ' selected' : ''}>No Page Numbers</option>
    `;
    sel.addEventListener('change', () => {
      state.tocStyle = sel.value;
      applyTocStyling();
    });
    sel.addEventListener('mousedown', e => e.stopPropagation());
    sel.addEventListener('click', e => e.stopPropagation());
    titleEl.appendChild(sel);
  });
}

// ═══════════════════════════════════════════════════
// E8.3: Background/Idle Layout Scheduling
// ═══════════════════════════════════════════════════

/**
 * Schedule a non-urgent layout recomputation during idle time.
 * Used for text-only edits where immediate repagination is not needed.
 */
export function scheduleIdleLayout() {
  if (window.requestIdleCallback) {
    requestIdleCallback(() => renderPages(), { timeout: 1000 });
  } else {
    setTimeout(() => renderPages(), 100);
  }
}

// ═══════════════════════════════════════════════════
// F4.3: Canvas mode — re-export for external use
// ═══════════════════════════════════════════════════

export { canvasHitTest } from './canvas-render.js';

// Re-export already-imported canvas mode functions so consumers of render.js
// have a single import point for all rendering functionality.
export {
  isCanvasMode,
  setCanvasMode,
  initCanvasRenderer,
  destroyCanvasRenderer,
};

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

  // E8.1: Buffer zone — render BUFFER_PAGES pages of content above/below viewport
  const bufferPx = BUFFER_PAGES * getPageHeightPx();
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

  // E8.1: Rapid scroll handling — detect fast scrolls and skip intermediate renders
  canvas.removeEventListener('scroll', _handleVirtualScroll);
  canvas.addEventListener('scroll', _handleVirtualScroll, { passive: true });
  state._vsLastScrollTop = canvas.scrollTop;
}

/**
 * E8.1: Throttled scroll handler using requestAnimationFrame.
 * On rapid scroll (delta > viewport height), skips intermediate rendering
 * and jumps directly to target position.
 */
function _handleVirtualScroll() {
  // E8.1e: Throttle with requestAnimationFrame
  if (state._vsRAF) return;
  state._vsRAF = requestAnimationFrame(() => {
    state._vsRAF = null;
    const canvas = $('editorCanvas');
    if (!canvas || !state.virtualScroll) return;

    const currentScroll = canvas.scrollTop;
    const lastScroll = state._vsLastScrollTop;
    const viewportHeight = canvas.clientHeight;
    const scrollDelta = Math.abs(currentScroll - lastScroll);

    state._vsLastScrollTop = currentScroll;

    // E8.1c: Rapid scroll detection — if scroll delta > viewport height,
    // the IntersectionObserver will naturally handle it, but we ensure
    // scroll position is preserved after any pending restores complete.
    if (scrollDelta > viewportHeight) {
      // Force a synchronous check: the IO callbacks will fire on the next
      // frame anyway; just record that we had a rapid scroll so we can
      // defer image loading for off-screen blocks.
      _releaseOffscreenImages();
    }

    // E8.4: Release image data for off-screen images during scroll
    _releaseOffscreenImages();
  });
}

/**
 * E8.4: Release image src data for images that have scrolled off-screen
 * in virtual scroll mode. Replace with a lightweight placeholder.
 * Restore when scrolled back into view (handled in restoreBlock).
 */
function _releaseOffscreenImages() {
  if (!state.virtualScroll) return;
  const vs = state.virtualScroll;
  const canvas = $('editorCanvas');
  if (!canvas) return;

  const viewTop = canvas.scrollTop;
  const viewBottom = viewTop + canvas.clientHeight;
  const bufferPx = BUFFER_PAGES * getPageHeightPx();

  for (const entry of vs.entries) {
    if (!entry.visible || !entry.el || !entry.el.parentNode) continue;

    const rect = entry.el.getBoundingClientRect();
    const canvasRect = canvas.getBoundingClientRect();
    const elTop = rect.top - canvasRect.top + viewTop;
    const elBottom = elTop + rect.height;

    // Well outside the buffer zone — release images
    if (elBottom < viewTop - bufferPx * 2 || elTop > viewBottom + bufferPx * 2) {
      const imgs = entry.el.querySelectorAll('img[src]:not([src^="data:image/gif"])');
      imgs.forEach(img => {
        if (!state._offscreenImageSrcs.has(img)) {
          state._offscreenImageSrcs.set(img, img.src);
          img.dataset.originalSrc = img.src;
          img.src = PLACEHOLDER_IMG_SRC;
        }
      });
    }
  }
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

  // E8.1b: Placeholder preserves measured height to prevent scroll jumps
  const placeholder = document.createElement('div');
  placeholder.className = 'vs-placeholder';
  placeholder.style.height = entry.height + 'px';
  placeholder.style.minHeight = entry.height + 'px';
  if (entry.nodeId) placeholder.dataset.nodeId = entry.nodeId;
  placeholder.dataset.vsIndex = String(idx);

  // E8.4: Release image references before collapsing
  entry.el.querySelectorAll('img').forEach(img => {
    state._offscreenImageSrcs.delete(img);
  });

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

  // E8.1d: Save scroll position before restoring to prevent displacement
  const canvas = $('editorCanvas');
  const scrollBefore = canvas ? canvas.scrollTop : 0;

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
  if (entry.nodeId) state.syncedTextCache.set(entry.nodeId, getEditableText(newEl));

  // E8.1d: Restore scroll position if it was displaced by the DOM change
  if (canvas && Math.abs(canvas.scrollTop - scrollBefore) > 2) {
    canvas.scrollTop = scrollBefore;
  }
}

function teardownVirtualScroll() {
  if (!state.virtualScroll) return;
  const vs = state.virtualScroll;
  // Disconnect the IntersectionObserver to release all observed element references
  vs.observer.disconnect();

  // E8.1: Remove scroll handler
  const canvas = $('editorCanvas');
  if (canvas) canvas.removeEventListener('scroll', _handleVirtualScroll);

  // Cancel any pending RAF
  if (state._vsRAF) {
    cancelAnimationFrame(state._vsRAF);
    state._vsRAF = null;
  }

  // E8.4: Restore any released image sources
  state._offscreenImageSrcs.forEach((originalSrc, img) => {
    if (img.isConnected) img.src = originalSrc;
  });
  state._offscreenImageSrcs.clear();

  // Restore any collapsed blocks so they aren't lost
  for (const entry of vs.entries) {
    if (!entry.visible && entry.html && entry.el?.parentNode) {
      try {
        const temp = document.createElement('div');
        temp.innerHTML = entry.html;
        const restored = temp.firstElementChild;
        if (restored) {
          entry.el.replaceWith(restored);
          if (entry.nodeId) state.nodeIdToElement.set(entry.nodeId, restored);
        }
      } catch (_) {}
    }
  }
  // Clear all references so entries + indexMap can be GC'd
  vs.entries.length = 0;
  state.virtualScroll = null;
}
