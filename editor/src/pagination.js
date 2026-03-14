// Page break visualization using WASM layout engine
import { state, $ } from './state.js';

export function updatePageBreaks() {
  const page = $('docPage');
  if (!page) return;
  // Remove existing indicators
  page.querySelectorAll('.page-break, .editor-footer').forEach(el => el.remove());
  page.querySelectorAll('header, footer').forEach(el => el.remove());

  const { doc } = state;
  if (!doc) return;

  // Get page map from WASM layout engine
  let pageMap = null;
  try { pageMap = JSON.parse(doc.get_page_map_json()); } catch (_) {}

  const numPages = pageMap?.pages?.length || 1;

  if (pageMap && pageMap.pages && numPages > 1) {
    const pages = pageMap.pages;

    for (let i = 0; i < numPages - 1; i++) {
      const nextPage = pages[i + 1];
      if (!nextPage.nodeIds?.length) continue;

      // Find first DOM element of next page
      const firstNextEl = page.querySelector(`[data-node-id="${nextPage.nodeIds[0]}"]`);
      if (!firstNextEl) continue;

      // Sanitize footer/header text (prevent XSS from document content)
      const footerText = escapeHtml(pages[i].footer || '');
      const headerText = escapeHtml(nextPage.header || '');

      const brk = document.createElement('div');
      brk.className = 'page-break';
      brk.contentEditable = 'false';
      brk.innerHTML =
        `<div class="pb-footer">${footerText}</div>` +
        '<div class="pb-gap"></div>' +
        `<div class="pb-header">${headerText}</div>` +
        `<span class="pb-badge">Page ${i + 1} of ${numPages}</span>`;
      firstNextEl.before(brk);
    }

    // Footer on last page
    const lastPage = pages[numPages - 1];
    const ftr = document.createElement('div');
    ftr.className = 'editor-footer';
    ftr.contentEditable = 'false';
    ftr.textContent = lastPage.footer || `Page ${numPages} of ${numPages}`;
    page.appendChild(ftr);
  } else if (numPages === 1) {
    // Single page — still show page footer
    const ftr = document.createElement('div');
    ftr.className = 'editor-footer';
    ftr.contentEditable = 'false';
    ftr.textContent = 'Page 1 of 1';
    page.appendChild(ftr);
  }

  updateStatusBar(numPages);
}

let _statusTimer = 0;
function updateStatusBar(numPages) {
  const { doc, currentFormat } = state;
  if (!doc) return;
  // Debounce — to_plain_text() is expensive, avoid calling on every keystroke
  clearTimeout(_statusTimer);
  _statusTimer = setTimeout(() => {
    try {
      const pCount = doc.paragraph_count();
      const text = doc.to_plain_text();
      const words = text.trim() ? text.trim().split(/\s+/).length : 0;
      const chars = text.length;
      $('statusInfo').textContent =
        `${words.toLocaleString()} words \u00b7 ${chars.toLocaleString()} characters \u00b7 ${pCount} paragraphs \u00b7 ${numPages} page${numPages !== 1 ? 's' : ''}`;
      $('statusFormat').textContent = currentFormat;
    } catch (_) {}
  }, 300);
}

function escapeHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}
