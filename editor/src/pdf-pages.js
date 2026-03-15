// PDF Page Management — thumbnails sidebar, reorder, delete, rotate, merge, split
import { state, $ } from './state.js';
import { showToast } from './toolbar-handlers.js';
import { updatePdfStatusBar } from './file.js';

// ─── Sidebar Thumbnails ─────────────────────────────

/**
 * Render page thumbnails in the sidebar.
 */
export async function renderThumbnails() {
  const viewer = state.pdfViewer;
  if (!viewer) return;
  const pdfDoc = viewer.getPdfDocument();
  if (!pdfDoc) return;

  const sidebarContent = $('pdfSidebarContent');
  if (!sidebarContent) return;
  sidebarContent.innerHTML = '';

  const pageCount = pdfDoc.numPages;
  const thumbScale = 0.2; // small thumbnails

  for (let i = 1; i <= pageCount; i++) {
    const page = await pdfDoc.getPage(i);
    const viewport = page.getViewport({ scale: thumbScale });

    const thumbEl = document.createElement('div');
    thumbEl.className = 'pdf-thumb';
    if (i === state.pdfCurrentPage) thumbEl.classList.add('active');
    thumbEl.dataset.pageNum = i;

    const canvas = document.createElement('canvas');
    canvas.width = Math.floor(viewport.width);
    canvas.height = Math.floor(viewport.height);
    canvas.style.width = viewport.width + 'px';
    canvas.style.height = viewport.height + 'px';

    const ctx = canvas.getContext('2d');
    await page.render({ canvasContext: ctx, viewport }).promise;

    const label = document.createElement('div');
    label.className = 'pdf-thumb-label';
    label.textContent = String(i);

    thumbEl.appendChild(canvas);
    thumbEl.appendChild(label);

    // Click to navigate
    thumbEl.addEventListener('click', () => {
      viewer.goToPage(i);
      state.pdfCurrentPage = i;
      updatePdfStatusBar();
      // Update active state
      sidebarContent.querySelectorAll('.pdf-thumb').forEach(t => t.classList.remove('active'));
      thumbEl.classList.add('active');
    });

    sidebarContent.appendChild(thumbEl);
  }
}

/**
 * Update the active thumbnail highlight.
 */
export function updateActiveThumbnail(pageNum) {
  const sidebarContent = $('pdfSidebarContent');
  if (!sidebarContent) return;
  sidebarContent.querySelectorAll('.pdf-thumb').forEach(t => {
    t.classList.toggle('active', parseInt(t.dataset.pageNum, 10) === pageNum);
  });
}

// ─── Page Management Modal ──────────────────────────

let _selectedPages = new Set();

export function openPageManager() {
  const modal = $('pdfPageModal');
  if (!modal) return;
  modal.classList.add('show');
  _selectedPages.clear();
  renderPageGrid();
  wirePageModalEvents();
}

function closePageManager() {
  const modal = $('pdfPageModal');
  if (modal) modal.classList.remove('show');
}

async function renderPageGrid() {
  const grid = $('pdfPageGrid');
  if (!grid) return;
  grid.innerHTML = '';

  const viewer = state.pdfViewer;
  if (!viewer) return;
  const pdfDoc = viewer.getPdfDocument();
  if (!pdfDoc) return;

  const pageCount = pdfDoc.numPages;
  const thumbScale = 0.15;

  for (let i = 1; i <= pageCount; i++) {
    const page = await pdfDoc.getPage(i);
    const viewport = page.getViewport({ scale: thumbScale });

    const card = document.createElement('div');
    card.className = 'pdf-page-card';
    card.dataset.pageNum = i;
    if (_selectedPages.has(i)) card.classList.add('selected');

    const canvas = document.createElement('canvas');
    canvas.width = Math.floor(viewport.width);
    canvas.height = Math.floor(viewport.height);
    const ctx = canvas.getContext('2d');
    await page.render({ canvasContext: ctx, viewport }).promise;

    const label = document.createElement('div');
    label.className = 'pdf-page-card-label';
    label.textContent = `Page ${i}`;

    // Action buttons (rotate, delete)
    const actions = document.createElement('div');
    actions.className = 'pdf-page-card-actions';
    actions.innerHTML = `
      <button title="Rotate 90" data-action="rotate" data-page="${i}"><span class="msi">rotate_right</span></button>
      <button title="Delete page" data-action="delete" data-page="${i}"><span class="msi">delete</span></button>
    `;

    card.appendChild(canvas);
    card.appendChild(label);
    card.appendChild(actions);

    // Toggle selection
    card.addEventListener('click', (e) => {
      if (e.target.closest('.pdf-page-card-actions')) return;
      if (_selectedPages.has(i)) {
        _selectedPages.delete(i);
        card.classList.remove('selected');
      } else {
        _selectedPages.add(i);
        card.classList.add('selected');
      }
    });

    grid.appendChild(card);
  }

  // Wire action buttons
  grid.querySelectorAll('[data-action]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const action = btn.dataset.action;
      const pageNum = parseInt(btn.dataset.page, 10);
      if (action === 'rotate') rotatePage(pageNum);
      if (action === 'delete') deletePage(pageNum);
    });
  });

  // Drag-and-drop reorder
  enableDragReorder(grid);
}

function wirePageModalEvents() {
  $('pdfPageCancelBtn')?.addEventListener('click', closePageManager, { once: true });

  $('pdfPageExtractBtn')?.addEventListener('click', () => {
    if (_selectedPages.size === 0) {
      showToast('Select pages to extract', 'error');
      return;
    }
    extractPages([..._selectedPages].sort((a, b) => a - b));
  }, { once: true });

  $('pdfPageMergeBtn')?.addEventListener('click', () => {
    $('pdfMergeInput')?.click();
  }, { once: true });

  $('pdfMergeInput')?.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      mergePdf(new Uint8Array(reader.result));
      e.target.value = '';
    };
    reader.readAsArrayBuffer(file);
  }, { once: true });
}

// ─── Page Operations ─────────────────────────────────
// These use the lopdf-based WASM editor when available (Phase 3+).
// For now, they'll be stubbed with informational messages until
// the Rust PDF editor WASM module is built.

async function rotatePage(pageNum) {
  try {
    if (state._wasmPdfEditor) {
      state._wasmPdfEditor.rotate_page(pageNum - 1, 90);
      state.pdfModified = true;
      await reloadPdf();
      renderPageGrid();
      showToast(`Page ${pageNum} rotated`);
    } else {
      showToast('PDF page operations require WASM PDF editor (coming soon)', 'error');
    }
  } catch (e) {
    showToast('Rotate failed: ' + e.message, 'error');
  }
}

async function deletePage(pageNum) {
  const viewer = state.pdfViewer;
  if (!viewer || viewer.getPageCount() <= 1) {
    showToast('Cannot delete the only page', 'error');
    return;
  }
  try {
    if (state._wasmPdfEditor) {
      state._wasmPdfEditor.delete_page(pageNum - 1);
      state.pdfModified = true;
      await reloadPdf();
      renderPageGrid();
      showToast(`Page ${pageNum} deleted`);
    } else {
      showToast('PDF page operations require WASM PDF editor (coming soon)', 'error');
    }
  } catch (e) {
    showToast('Delete failed: ' + e.message, 'error');
  }
}

async function extractPages(pageNums) {
  try {
    if (state._wasmPdfEditor) {
      const extracted = state._wasmPdfEditor.extract_pages(new Uint32Array(pageNums.map(p => p - 1)));
      const blob = new Blob([extracted], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = ($('docName').value || 'document') + '_extracted.pdf';
      a.click();
      URL.revokeObjectURL(url);
      showToast(`Extracted ${pageNums.length} page(s)`);
    } else {
      showToast('PDF page operations require WASM PDF editor (coming soon)', 'error');
    }
  } catch (e) {
    showToast('Extract failed: ' + e.message, 'error');
  }
}

async function mergePdf(otherBytes) {
  try {
    if (state._wasmPdfEditor) {
      state._wasmPdfEditor.merge(otherBytes);
      state.pdfModified = true;
      await reloadPdf();
      renderPageGrid();
      showToast('PDFs merged');
    } else {
      showToast('PDF page operations require WASM PDF editor (coming soon)', 'error');
    }
  } catch (e) {
    showToast('Merge failed: ' + e.message, 'error');
  }
}

/** Reload the PDF in the viewer after a WASM edit operation. */
async function reloadPdf() {
  if (!state._wasmPdfEditor || !state.pdfViewer) return;
  const bytes = state._wasmPdfEditor.save();
  state.pdfBytes = bytes;
  await state.pdfViewer.open(bytes);
  updatePdfStatusBar();
}

// ─── Drag & Drop Reorder ─────────────────────────────

function enableDragReorder(grid) {
  let draggedCard = null;

  grid.querySelectorAll('.pdf-page-card').forEach(card => {
    card.draggable = true;

    card.addEventListener('dragstart', (e) => {
      draggedCard = card;
      card.style.opacity = '0.5';
      e.dataTransfer.effectAllowed = 'move';
    });

    card.addEventListener('dragend', () => {
      card.style.opacity = '';
      draggedCard = null;
      grid.querySelectorAll('.pdf-page-card').forEach(c => c.classList.remove('drag-over'));
    });

    card.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
      card.classList.add('drag-over');
    });

    card.addEventListener('dragleave', () => {
      card.classList.remove('drag-over');
    });

    card.addEventListener('drop', (e) => {
      e.preventDefault();
      card.classList.remove('drag-over');
      if (!draggedCard || draggedCard === card) return;

      const fromPage = parseInt(draggedCard.dataset.pageNum, 10);
      const toPage = parseInt(card.dataset.pageNum, 10);

      if (fromPage !== toPage) {
        movePage(fromPage, toPage);
      }
    });
  });
}

async function movePage(fromPage, toPage) {
  try {
    if (state._wasmPdfEditor) {
      state._wasmPdfEditor.move_page(fromPage - 1, toPage - 1);
      state.pdfModified = true;
      await reloadPdf();
      renderPageGrid();
      showToast(`Page ${fromPage} moved to position ${toPage}`);
    } else {
      showToast('PDF page operations require WASM PDF editor (coming soon)', 'error');
    }
  } catch (e) {
    showToast('Move failed: ' + e.message, 'error');
  }
}
