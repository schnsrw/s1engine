// Multi-page rendering — per-page contenteditable containers.
// WASM get_page_map_json() is the single source of truth for pagination.
import { state, $ } from './state.js';
import { updateStatusBar as _updateStatus } from './file.js';

const PT_TO_PX = 96 / 72;

let _repaginateTimer = null;

/**
 * Build or update per-page DOM containers inside #pageContainer.
 * Each page gets: .page-header (non-editable), .page-content (contenteditable), .page-footer (non-editable).
 */
export function repaginate() {
  const container = $('pageContainer');
  const { doc } = state;
  if (!container || !doc) return;

  // Sync text to WASM before querying page map
  syncAllTextInline();

  // Get authoritative page map from WASM layout engine
  let pageMapJson = null;
  try { pageMapJson = doc.get_page_map_json(); } catch (_) {}

  let pageMap = null;
  if (pageMapJson) {
    try { pageMap = JSON.parse(pageMapJson); } catch (_) {}
  }

  // Fast-path: if the page map hasn't changed and DOM pages exist, skip reconciliation
  if (pageMap && state._lastPageMapHash === pageMapJson && state.pageElements.length > 0) {
    return;
  }
  state._lastPageMapHash = pageMapJson;

  if (!pageMap || !pageMap.pages || pageMap.pages.length === 0) {
    // Fallback: single page with all content
    state._lastPageMapHash = null;
    ensureSinglePage(container);
    state.pageMap = null;
    _updateStatus();
    return;
  }

  const pages = pageMap.pages;
  const numPages = pages.length;
  const defaultHeaderHtml = state.docHeaderHtml || '';
  const defaultFooterHtml = state.docFooterHtml || '';
  const firstPageHeaderHtml = state.docFirstPageHeaderHtml || '';
  const firstPageFooterHtml = state.docFirstPageFooterHtml || '';
  const hasDifferentFirst = state.hasDifferentFirstPage || false;

  // Get page dimensions
  const dims = state.pageDims || { marginTopPt: 72, marginBottomPt: 72, marginLeftPt: 72, marginRightPt: 72 };

  // Build nodeToPage map
  const newNodeToPage = new Map();
  for (const pg of pages) {
    for (const nid of pg.nodeIds) {
      newNodeToPage.set(nid, pg.pageNum);
    }
  }

  // Ensure correct number of .doc-page elements
  const existingPages = container.querySelectorAll('.doc-page');
  const existingCount = existingPages.length;

  // Create missing pages
  for (let i = existingCount; i < numPages; i++) {
    const pageNum = i + 1;
    const isFirstPage = pageNum === 1;
    const hdr = (isFirstPage && hasDifferentFirst) ? firstPageHeaderHtml : defaultHeaderHtml;
    const ftr = (isFirstPage && hasDifferentFirst) ? firstPageFooterHtml : defaultFooterHtml;
    const pageEl = createPageElement(pageNum, pages[i], dims, hdr, ftr, numPages);
    container.appendChild(pageEl);
  }

  // Remove excess pages
  for (let i = numPages; i < existingCount; i++) {
    existingPages[i].remove();
  }

  // Update state.pageElements
  state.pageElements = Array.from(container.querySelectorAll('.doc-page'));

  // Reconcile nodes into correct pages
  for (let i = 0; i < numPages; i++) {
    const pg = pages[i];
    const pageEl = state.pageElements[i];
    const contentEl = pageEl.querySelector('.page-content');
    if (!contentEl) continue;

    // Update page dimensions
    applyPageStyle(pageEl, pg, dims);

    // Update header/footer — first page uses first-page variants if available
    const isFirstPage = pg.pageNum === 1;
    const hdr = (isFirstPage && hasDifferentFirst) ? firstPageHeaderHtml : defaultHeaderHtml;
    const ftr = (isFirstPage && hasDifferentFirst) ? firstPageFooterHtml : defaultFooterHtml;
    updatePageHeaderFooter(pageEl, pg.pageNum, numPages, hdr, ftr);

    // Set of nodeIds that belong on this page
    const pageNodeIds = new Set(pg.nodeIds);

    // Track which nodes are already correctly placed
    const existingNodeIds = new Set();
    contentEl.querySelectorAll(':scope > [data-node-id]').forEach(el => {
      const nid = el.dataset.nodeId;
      if (pageNodeIds.has(nid)) {
        existingNodeIds.add(nid);
      }
    });

    // Add missing nodes in order
    let lastInserted = null;
    for (const nid of pg.nodeIds) {
      if (existingNodeIds.has(nid)) {
        // Already on this page — find it for ordering reference
        lastInserted = contentEl.querySelector(`[data-node-id="${nid}"]`);
        continue;
      }

      // Check if node exists on another page (move it)
      let existingEl = null;
      for (const otherPage of state.pageElements) {
        if (otherPage === pageEl) continue;
        const otherContent = otherPage.querySelector('.page-content');
        if (!otherContent) continue;
        existingEl = otherContent.querySelector(`[data-node-id="${nid}"]`);
        if (existingEl) break;
      }

      if (existingEl) {
        // Move from another page
        if (lastInserted && lastInserted.nextSibling) {
          contentEl.insertBefore(existingEl, lastInserted.nextSibling);
        } else if (!lastInserted) {
          contentEl.prepend(existingEl);
        } else {
          contentEl.appendChild(existingEl);
        }
        lastInserted = existingEl;
      } else {
        // Render from WASM
        try {
          const html = doc.render_node_html(nid);
          const temp = document.createElement('div');
          temp.innerHTML = html;
          const newEl = temp.firstElementChild;
          if (newEl) {
            if (!newEl.innerHTML.trim()) newEl.innerHTML = '<br>';
            if (lastInserted && lastInserted.nextSibling) {
              contentEl.insertBefore(newEl, lastInserted.nextSibling);
            } else if (!lastInserted) {
              contentEl.prepend(newEl);
            } else {
              contentEl.appendChild(newEl);
            }
            lastInserted = newEl;
            // Update cache
            state.nodeIdToElement.set(nid, newEl);
            state.syncedTextCache.set(nid, newEl.textContent || '');
          }
        } catch (_) {}
      }
    }

    // Remove nodes that don't belong on this page anymore
    contentEl.querySelectorAll(':scope > [data-node-id]').forEach(el => {
      if (!pageNodeIds.has(el.dataset.nodeId)) {
        // Will be picked up by the correct page's loop
        // Only remove if it's already been placed elsewhere, or store temporarily
        const correctPage = newNodeToPage.get(el.dataset.nodeId);
        if (correctPage && correctPage !== pg.pageNum) {
          // Will be moved by the correct page's iteration — leave it for now
          // But if we've already processed that page, we need to move it
        }
      }
    });

    // Ensure correct ordering within the page
    reorderChildren(contentEl, pg.nodeIds);
  }

  // Second pass: remove any orphaned nodes (nodes in DOM not in any page)
  for (const pageEl of state.pageElements) {
    const contentEl = pageEl.querySelector('.page-content');
    if (!contentEl) continue;
    contentEl.querySelectorAll(':scope > [data-node-id]').forEach(el => {
      if (!newNodeToPage.has(el.dataset.nodeId)) {
        el.remove();
      }
    });
  }

  // Update state
  state.pageMap = pageMap;
  state.nodeToPage = newNodeToPage;

  // Rebuild nodeIdToElement map
  state.nodeIdToElement.clear();
  for (const pageEl of state.pageElements) {
    const contentEl = pageEl.querySelector('.page-content');
    if (!contentEl) continue;
    contentEl.querySelectorAll('[data-node-id]').forEach(el => {
      state.nodeIdToElement.set(el.dataset.nodeId, el);
    });
  }

  _updateStatus();
}

/**
 * Schedule a debounced repagination (300ms).
 */
export function scheduleRepaginate() {
  clearTimeout(_repaginateTimer);
  _repaginateTimer = setTimeout(() => repaginate(), 300);
}

/**
 * Legacy compatibility — called by existing code that used updatePageBreaks()
 */
export function updatePageBreaks() {
  repaginate();
}

// ─── Internal helpers ──────────────────────────────────

function createPageElement(pageNum, pgData, dims, headerHtml, footerHtml, totalPages) {
  const pageEl = document.createElement('div');
  pageEl.className = 'doc-page';
  pageEl.dataset.page = String(pageNum);

  applyPageStyle(pageEl, pgData, dims);

  // Header
  const header = document.createElement('div');
  header.className = 'page-header';
  header.contentEditable = 'false';
  if (headerHtml) {
    header.innerHTML = headerHtml;
    substitutePageNumbers(header, pageNum, totalPages);
  }
  pageEl.appendChild(header);

  // Content (editable)
  const content = document.createElement('div');
  content.className = 'page-content';
  content.contentEditable = 'true';
  content.spellcheck = true;
  content.lang = 'en';
  content.setAttribute('role', 'textbox');
  content.setAttribute('aria-multiline', 'true');
  content.setAttribute('aria-label', `Page ${pageNum} content`);
  pageEl.appendChild(content);

  // Footer
  const footer = document.createElement('div');
  footer.className = 'page-footer';
  footer.contentEditable = 'false';
  if (footerHtml) {
    footer.innerHTML = footerHtml;
    substitutePageNumbers(footer, pageNum, totalPages);
  }
  pageEl.appendChild(footer);

  return pageEl;
}

function applyPageStyle(pageEl, pgData, dims) {
  const w = (pgData?.width || 612) * PT_TO_PX;
  const h = (pgData?.height || 792) * PT_TO_PX;
  pageEl.style.width = Math.round(w) + 'px';
  pageEl.style.minHeight = Math.round(h) + 'px';

  // Content area gets the margin padding
  const contentEl = pageEl.querySelector('.page-content');
  if (contentEl) {
    contentEl.style.paddingTop = Math.round(dims.marginTopPt * PT_TO_PX) + 'px';
    contentEl.style.paddingBottom = Math.round(dims.marginBottomPt * PT_TO_PX) + 'px';
    contentEl.style.paddingLeft = Math.round(dims.marginLeftPt * PT_TO_PX) + 'px';
    contentEl.style.paddingRight = Math.round(dims.marginRightPt * PT_TO_PX) + 'px';
  }
}

function updatePageHeaderFooter(pageEl, pageNum, totalPages, headerHtml, footerHtml) {
  const header = pageEl.querySelector('.page-header');
  if (header) {
    if (headerHtml) {
      header.innerHTML = headerHtml;
      substitutePageNumbers(header, pageNum, totalPages);
    } else {
      header.innerHTML = '';
    }
  }

  const footer = pageEl.querySelector('.page-footer');
  if (footer) {
    if (footerHtml) {
      footer.innerHTML = footerHtml;
      substitutePageNumbers(footer, pageNum, totalPages);
    } else {
      footer.innerHTML = '';
    }
  }
}

/**
 * Reorder children of contentEl to match the order in nodeIds array.
 */
function reorderChildren(contentEl, nodeIds) {
  const childMap = new Map();
  contentEl.querySelectorAll(':scope > [data-node-id]').forEach(el => {
    childMap.set(el.dataset.nodeId, el);
  });

  let prev = null;
  for (const nid of nodeIds) {
    const el = childMap.get(nid);
    if (!el) continue;
    if (prev) {
      if (el.previousElementSibling !== prev) {
        prev.after(el);
      }
    } else {
      if (el !== contentEl.firstElementChild) {
        contentEl.prepend(el);
      }
    }
    prev = el;
  }
}

/**
 * Fallback: ensure a single page exists when no page map available.
 */
function ensureSinglePage(container) {
  if (container.querySelector('.doc-page')) return;
  const dims = state.pageDims || { marginTopPt: 72, marginBottomPt: 72, marginLeftPt: 72, marginRightPt: 72 };
  const pageEl = createPageElement(1, null, dims, '', '', 1);
  container.appendChild(pageEl);
  state.pageElements = [pageEl];
}

/**
 * Sync all text content to WASM (inline to avoid circular import with render.js).
 */
function syncAllTextInline() {
  const { doc } = state;
  if (!doc) return;
  for (const pageEl of state.pageElements) {
    const contentEl = pageEl?.querySelector('.page-content');
    if (!contentEl) continue;
    contentEl.querySelectorAll('[data-node-id]').forEach(el => {
      const tag = el.tagName.toLowerCase();
      if ((tag === 'p' || /^h[1-6]$/.test(tag)) && el.dataset.nodeId) {
        const nodeId = el.dataset.nodeId;
        const newText = el.textContent || '';
        if (state.syncedTextCache.get(nodeId) !== newText) {
          try {
            doc.set_paragraph_text(nodeId, newText);
            state.syncedTextCache.set(nodeId, newText);
          } catch (_) {}
        }
      }
    });
  }
}

/**
 * Replace page number / page count field placeholders in header/footer HTML.
 */
function substitutePageNumbers(container, pageNum, totalPages) {
  container.querySelectorAll('[data-field]').forEach(el => {
    const field = el.dataset.field;
    if (field === 'PageNumber' || field === 'PAGE') {
      el.textContent = String(pageNum);
    } else if (field === 'PageCount' || field === 'NUMPAGES') {
      el.textContent = String(totalPages);
    }
  });
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null);
  let node;
  while ((node = walker.nextNode())) {
    const t = node.textContent;
    if (t.includes('PAGE') || t.includes('NUMPAGES')) {
      node.textContent = t
        .replace(/\bNUMPAGES\b/g, String(totalPages))
        .replace(/\bPAGE\b/g, String(pageNum));
    }
  }
}

/**
 * Get the active .page-content element (the one with focus or the first one).
 */
export function getActivePage() {
  // Check which page has focus
  const active = document.activeElement;
  if (active && active.classList?.contains('page-content')) return active;
  // Walk up from active element
  let n = active;
  while (n) {
    if (n.classList?.contains('page-content')) return n;
    n = n.parentElement;
  }
  // Fallback to first page
  const first = $('pageContainer')?.querySelector('.page-content');
  return first;
}

/**
 * Get the .page-content element for a given node ID.
 */
export function getPageForNode(nodeId) {
  const pageNum = state.nodeToPage.get(nodeId);
  if (pageNum && state.pageElements[pageNum - 1]) {
    return state.pageElements[pageNum - 1].querySelector('.page-content');
  }
  // Fallback: search all pages
  const container = $('pageContainer');
  if (!container) return null;
  const el = container.querySelector(`[data-node-id="${nodeId}"]`);
  return el?.closest('.page-content') || null;
}
