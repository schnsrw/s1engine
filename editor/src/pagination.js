// Multi-page rendering — per-page contenteditable containers.
// WASM get_page_map_json() is the single source of truth for pagination.
import { state, $ } from './state.js';
import { updateStatusBar as _updateStatus } from './file.js';
import { getEditableText, isInsideNonEditable } from './selection.js';
import { isSpellCheckEnabled } from './toolbar-handlers.js';

const PT_TO_PX = 96 / 72;

let _repaginateTimer = null;

/**
 * Build or update per-page DOM containers inside #pageContainer.
 * Each page gets: .page-header (non-editable), .page-content (contenteditable), .page-footer (non-editable).
 *
 * E8.3: Integrates layout caching — when _layoutDirty is false and page map
 * hasn't changed, skips the expensive DOM reconciliation.
 */
export function repaginate() {
  const container = $('pageContainer');
  const { doc } = state;
  if (!container || !doc) return;

  // Preserve synthetic selection state (select-all or cross-page shift-click)
  const wasSelectAll = state._selectAll;
  const savedSyntheticSel = (wasSelectAll || (state.lastSelInfo && !state.lastSelInfo.collapsed))
    ? { ...state.lastSelInfo } : null;

  // Save native selection before DOM reconciliation (moving nodes can invalidate selection)
  const sel = window.getSelection();
  let savedNodeId = null, savedOffset = 0;
  if (!wasSelectAll && sel && sel.rangeCount) {
    let n = sel.anchorNode;
    while (n && !n.dataset?.nodeId) n = n.parentNode;
    if (n?.dataset?.nodeId) {
      savedNodeId = n.dataset.nodeId;
      try {
        const range = sel.getRangeAt(0);
        const walker = document.createTreeWalker(n, NodeFilter.SHOW_TEXT, null);
        let count = 0, tw;
        while ((tw = walker.nextNode())) {
          // Skip text inside non-editable elements (list markers)
          if (isInsideNonEditable(tw, n)) continue;
          if (tw === range.startContainer) { savedOffset = count + range.startOffset; break; }
          count += tw.textContent.length;
        }
      } catch (_) {}
    }
  }

  // Sync text to WASM before querying page map
  syncAllTextInline();

  // Get authoritative page map from WASM layout engine
  let pageMapJson = null;
  try { pageMapJson = doc.get_page_map_json(); } catch (_) {}

  let pageMap = null;
  if (pageMapJson) {
    try { pageMap = JSON.parse(pageMapJson); } catch (_) {}
  }

  // L3: If layout is dirty (e.g. font change), force cache invalidation
  if (state._layoutDirty) {
    state._lastPageMapHash = null; // Force re-render
    state._layoutDirty = false;
  }

  // Fast-path: if the page map hasn't changed and DOM pages exist, skip reconciliation.
  // E8.3: Also skip when layout is not dirty (text-only edits within paragraphs).
  if (pageMap && state._lastPageMapHash === pageMapJson && state.pageElements.length > 0) {
    return;
  }
  state._lastPageMapHash = pageMapJson;

  // E8.3: Invalidate layout cache when pagination actually changes
  state._layoutCache = null;

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

  // Build nodeToPage map and table chunk map
  const newNodeToPage = new Map();
  // tableChunkMap: Map<pageNum, Map<tableId, {rowIds, isContinuation, chunkId}>>
  const tableChunkMap = new Map();
  for (const pg of pages) {
    // Process table chunks for this page
    if (pg.tableChunks && pg.tableChunks.length > 0) {
      const pageChunks = new Map();
      for (const chunk of pg.tableChunks) {
        const chunkId = chunk.isContinuation
          ? `${chunk.tableId}-p${pg.pageNum}`
          : chunk.tableId;
        pageChunks.set(chunk.tableId, {
          rowIds: chunk.rowIds,
          isContinuation: chunk.isContinuation,
          chunkId,
        });
      }
      tableChunkMap.set(pg.pageNum, pageChunks);
    }

    for (const nid of pg.nodeIds) {
      // For tables with chunks, map the chunk ID (not the raw table ID)
      const pageChunks = tableChunkMap.get(pg.pageNum);
      const chunk = pageChunks?.get(nid);
      if (chunk) {
        newNodeToPage.set(chunk.chunkId, pg.pageNum);
        // Don't overwrite the first page's mapping for the base table ID
        if (!chunk.isContinuation) {
          newNodeToPage.set(nid, pg.pageNum);
        }
      } else {
        newNodeToPage.set(nid, pg.pageNum);
      }
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

    // Update header/footer — use sequential index (i+1), NOT pg.pageNum from WASM
    const pageNum = i + 1;
    const isFirstPage = pageNum === 1;
    const hdr = (isFirstPage && hasDifferentFirst) ? firstPageHeaderHtml : defaultHeaderHtml;
    const ftr = (isFirstPage && hasDifferentFirst) ? firstPageFooterHtml : defaultFooterHtml;
    updatePageHeaderFooter(pageEl, pageNum, numPages, hdr, ftr);

    // Build the effective node ID list for this page (replacing table IDs with chunk IDs)
    const pageChunks = tableChunkMap.get(pg.pageNum);
    const effectiveNodeIds = pg.nodeIds.map(nid => {
      const chunk = pageChunks?.get(nid);
      return chunk ? chunk.chunkId : nid;
    });

    // Set of nodeIds that belong on this page
    const pageNodeIds = new Set(effectiveNodeIds);

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
    for (let idx = 0; idx < effectiveNodeIds.length; idx++) {
      const effectiveId = effectiveNodeIds[idx];
      const originalId = pg.nodeIds[idx];
      const chunk = pageChunks?.get(originalId);

      if (existingNodeIds.has(effectiveId)) {
        // Already on this page — find it for ordering reference
        lastInserted = contentEl.querySelector(`[data-node-id="${CSS.escape(effectiveId)}"]`);
        continue;
      }

      if (chunk) {
        // This is a table chunk — render using render_table_chunk
        try {
          const rowIdsJson = JSON.stringify(chunk.rowIds);
          const html = doc.render_table_chunk(
            originalId,
            rowIdsJson,
            chunk.chunkId,
            chunk.isContinuation
          );
          const temp = document.createElement('div');
          temp.innerHTML = html;
          const newEl = temp.firstElementChild;
          if (newEl) {
            if (lastInserted && lastInserted.nextSibling) {
              contentEl.insertBefore(newEl, lastInserted.nextSibling);
            } else if (!lastInserted) {
              contentEl.prepend(newEl);
            } else {
              contentEl.appendChild(newEl);
            }
            lastInserted = newEl;
            state.nodeIdToElement.set(effectiveId, newEl);
          }
        } catch (e) {
          console.warn('render_table_chunk failed:', e);
        }
        continue;
      }

      // Non-table node: check if it exists on another page (move it)
      let existingEl = null;
      for (const otherPage of state.pageElements) {
        if (otherPage === pageEl) continue;
        const otherContent = otherPage.querySelector('.page-content');
        if (!otherContent) continue;
        existingEl = otherContent.querySelector(`[data-node-id="${CSS.escape(effectiveId)}"]`);
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
          const html = doc.render_node_html(effectiveId);
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
            state.nodeIdToElement.set(effectiveId, newEl);
            state.syncedTextCache.set(effectiveId, getEditableText(newEl));
          }
        } catch (_) {}
      }
    }

    // Remove nodes that don't belong on this page anymore
    contentEl.querySelectorAll(':scope > [data-node-id]').forEach(el => {
      const nid = el.dataset.nodeId;
      if (!pageNodeIds.has(nid)) {
        // Check if it's a table chunk that belongs elsewhere
        const correctPage = newNodeToPage.get(nid);
        if (correctPage !== undefined && correctPage !== pg.pageNum) {
          // Move node to the correct page immediately
          const targetIdx = correctPage - 1;
          if (targetIdx >= 0 && targetIdx < state.pageElements.length) {
            const targetContent = state.pageElements[targetIdx].querySelector('.page-content');
            if (targetContent) {
              targetContent.appendChild(el);
            }
          }
        } else if (el.dataset.tableSource) {
          // Stale table chunk — remove it
          el.remove();
        }
      }
    });

    // Ensure correct ordering within the page
    reorderChildren(contentEl, effectiveNodeIds);
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

  // Restore selection after DOM reconciliation
  if (wasSelectAll && savedSyntheticSel) {
    // Re-apply select-all: restore state and re-highlight
    state._selectAll = true;
    // Update element references since DOM may have moved
    const newStartEl = container.querySelector(`[data-node-id="${savedSyntheticSel.startNodeId}"]`);
    const newEndEl = container.querySelector(`[data-node-id="${savedSyntheticSel.endNodeId}"]`);
    state.lastSelInfo = {
      ...savedSyntheticSel,
      startEl: newStartEl || savedSyntheticSel.startEl,
      endEl: newEndEl || savedSyntheticSel.endEl,
    };
    // Re-apply visual highlights
    for (const pageEl of state.pageElements) {
      const content = pageEl.querySelector('.page-content') || pageEl;
      content.classList.add('select-all-highlight');
      content.querySelectorAll('[data-node-id]').forEach(el => el.classList.add('select-all-highlight'));
    }
  } else if (savedSyntheticSel && !savedSyntheticSel.collapsed) {
    // Restore cross-page synthetic selection
    const newStartEl = container.querySelector(`[data-node-id="${savedSyntheticSel.startNodeId}"]`);
    const newEndEl = container.querySelector(`[data-node-id="${savedSyntheticSel.endNodeId}"]`);
    if (newStartEl && newEndEl) {
      state.lastSelInfo = {
        ...savedSyntheticSel,
        startEl: newStartEl,
        endEl: newEndEl,
      };
    }
  } else if (savedNodeId) {
    const restoredEl = container.querySelector(`[data-node-id="${savedNodeId}"]`);
    if (restoredEl) {
      try {
        const walker = document.createTreeWalker(restoredEl, NodeFilter.SHOW_TEXT, null);
        let counted = 0, tw;
        while ((tw = walker.nextNode())) {
          // Skip text inside non-editable elements (list markers)
          if (isInsideNonEditable(tw, restoredEl)) continue;
          if (counted + tw.textContent.length >= savedOffset) {
            const range = document.createRange();
            range.setStart(tw, savedOffset - counted);
            range.collapse(true);
            const s = window.getSelection();
            s.removeAllRanges(); s.addRange(range);
            break;
          }
          counted += tw.textContent.length;
        }
      } catch (_) {}
    }
  }

  _updateStatus();
}

/**
 * Schedule a debounced repagination (300ms).
 * E8.3: Also marks layout as dirty since this is called after structural changes.
 */
export function scheduleRepaginate() {
  state._layoutDirty = true;
  state._layoutCache = null;
  clearTimeout(_repaginateTimer);
  _repaginateTimer = setTimeout(() => repaginate(), 300);
}

/**
 * Legacy compatibility — called by existing code that used updatePageBreaks().
 * Uses debounced repagination to avoid redundant DOM reconciliation.
 * E8.3: Marks layout dirty since this indicates a structural change.
 */
export function updatePageBreaks() {
  scheduleRepaginate();
}

// ─── Internal helpers ──────────────────────────────────

function createPageElement(pageNum, pgData, dims, headerHtml, footerHtml, totalPages) {
  const pageEl = document.createElement('div');
  pageEl.className = 'doc-page';
  pageEl.dataset.page = String(pageNum);

  applyPageStyle(pageEl, pgData, dims);

  // Header
  const header = document.createElement('div');
  header.className = 'page-header hf-hoverable';
  header.contentEditable = 'false';
  header.setAttribute('data-hf-kind', 'header');
  header.setAttribute('title', 'Double-click to edit header');
  if (headerHtml) {
    header.innerHTML = headerHtml;
    substitutePageNumbers(header, pageNum, totalPages);
  }
  pageEl.appendChild(header);

  // Content (editable)
  const content = document.createElement('div');
  content.className = 'page-content';
  // FS-11: Respect read-only mode
  content.contentEditable = state.readOnlyMode ? 'false' : 'true';
  content.spellcheck = isSpellCheckEnabled();
  content.lang = 'en';
  content.setAttribute('role', 'textbox');
  content.setAttribute('aria-multiline', 'true');
  content.setAttribute('aria-label', `Page ${pageNum} content`);
  pageEl.appendChild(content);

  // Footer
  const footer = document.createElement('div');
  footer.className = 'page-footer hf-hoverable';
  footer.contentEditable = 'false';
  footer.setAttribute('data-hf-kind', 'footer');
  footer.setAttribute('title', 'Double-click to edit footer');
  if (footerHtml) {
    footer.innerHTML = footerHtml;
    substitutePageNumbers(footer, pageNum, totalPages);
  }
  pageEl.appendChild(footer);

  return pageEl;
}

function applyPageStyle(pageEl, pgData, defaultDims) {
  const w = (pgData?.width || 612) * PT_TO_PX;
  const h = (pgData?.height || 792) * PT_TO_PX;
  pageEl.style.width = Math.round(w) + 'px';
  pageEl.style.minHeight = Math.round(h) + 'px';

  // Content area gets per-page margins (from layout engine) or fallback to section defaults
  const contentEl = pageEl.querySelector('.page-content');
  if (contentEl) {
    const mt = pgData?.marginTop ?? defaultDims.marginTopPt ?? 72;
    const mb = pgData?.marginBottom ?? defaultDims.marginBottomPt ?? 72;
    const ml = pgData?.marginLeft ?? defaultDims.marginLeftPt ?? 72;
    const mr = pgData?.marginRight ?? defaultDims.marginRightPt ?? 72;
    contentEl.style.paddingTop = Math.round(mt * PT_TO_PX) + 'px';
    contentEl.style.paddingBottom = Math.round(mb * PT_TO_PX) + 'px';
    contentEl.style.paddingLeft = Math.round(ml * PT_TO_PX) + 'px';
    contentEl.style.paddingRight = Math.round(mr * PT_TO_PX) + 'px';
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
        const newText = getEditableText(el);
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
  // Only substitute in data-field elements (not in document content text)
  container.querySelectorAll('[data-field]').forEach(el => {
    const field = (el.dataset.field || '').toUpperCase();
    if (field === 'PAGENUMBER' || field === 'PAGE') {
      el.textContent = String(pageNum);
    } else if (field === 'PAGECOUNT' || field === 'NUMPAGES') {
      el.textContent = String(totalPages);
    }
  });
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
