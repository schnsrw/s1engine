// Document rendering — WASM → DOM
import { state, $ } from './state.js';
import { setupImages } from './images.js';
import { updatePageBreaks } from './pagination.js';
import { updateUndoRedo } from './toolbar.js';
import { markDirty } from './file.js';

export function renderDocument() {
  const { doc } = state;
  if (!doc) return;
  try {
    const html = doc.to_html();
    state.ignoreInput = true;
    const page = $('docPage');
    page.innerHTML = html;
    fixEmptyBlocks();
    setupImages();
    cacheAllText();
    state.ignoreInput = false;
    state.pagesRendered = false;
    updatePageBreaks();
    updateUndoRedo();
  } catch (e) { console.error('Render error:', e); }
}

export function renderNodeById(nodeIdStr) {
  const { doc } = state;
  if (!doc) return null;
  try {
    const html = doc.render_node_html(nodeIdStr);
    const el = $('docPage').querySelector(`[data-node-id="${nodeIdStr}"]`);
    if (!el) return null;
    const temp = document.createElement('div');
    temp.innerHTML = html;
    const newEl = temp.firstElementChild;
    if (!newEl) return null;
    if (!newEl.innerHTML.trim()) newEl.innerHTML = '<br>';
    el.replaceWith(newEl);
    setupImages(newEl);
    state.syncedTextCache.set(nodeIdStr, newEl.textContent || '');
    return newEl;
  } catch (e) { console.error('renderNode error:', e); }
  return null;
}

export function fixEmptyBlocks() {
  $('docPage').querySelectorAll('p:empty, h1:empty, h2:empty, h3:empty, h4:empty, h5:empty, h6:empty')
    .forEach(el => { el.innerHTML = '<br>'; });
}

export function cacheAllText() {
  state.syncedTextCache.clear();
  $('docPage').querySelectorAll('[data-node-id]').forEach(el => {
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
  } catch (e) { console.error('sync error:', e); }
}

export function syncAllText() {
  if (!state.doc) return;
  $('docPage').querySelectorAll('[data-node-id]').forEach(el => {
    const tag = el.tagName.toLowerCase();
    if (tag === 'p' || /^h[1-6]$/.test(tag)) syncParagraphText(el);
  });
}

export function debouncedSync(el) {
  clearTimeout(state.syncTimer);
  state.syncTimer = setTimeout(() => {
    syncParagraphText(el);
    state.pagesRendered = false;
    updatePageBreaks();
    updateUndoRedo();
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
