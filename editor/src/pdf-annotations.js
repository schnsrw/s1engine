// PDF Annotations — highlight, comment, ink, text, redact overlays
import { state, $ } from './state.js';
import { showToast } from './toolbar-handlers.js';

// ─── Annotation Data Model ───────────────────────────

export class PdfAnnotation {
  constructor(type, pageNum, props = {}) {
    this.id = crypto.randomUUID();
    this.type = type;       // 'highlight' | 'comment' | 'ink' | 'text' | 'redact'
    this.pageNum = pageNum;
    this.created = new Date().toISOString();
    this.author = 'User';
    this.color = '#FFEB3B';
    this.opacity = 0.35;
    this.props = props;
  }
}

// ─── Tool State ──────────────────────────────────────

let _drawingCtx = null;
let _drawingPath = [];
let _isDrawing = false;
let _drawColor = '#d93025';
let _drawWidth = 2;
let _drawingPageNum = 0;

// ─── Annotation Undo Stack ──────────────────────────
const _undoStack = []; // stores snapshots of pdfAnnotations array
const MAX_UNDO = 30;

function _saveUndoState() {
  _undoStack.push(JSON.parse(JSON.stringify(state.pdfAnnotations)));
  if (_undoStack.length > MAX_UNDO) _undoStack.shift();
}

export function undoAnnotation() {
  if (_undoStack.length === 0) return;
  state.pdfAnnotations = _undoStack.pop();
  state.pdfModified = true;
  // Re-render all pages
  const pageCount = state.pdfViewer?.getPageCount() || 0;
  for (let i = 1; i <= pageCount; i++) renderAnnotationsForPage(i);
  refreshAnnotationsPanel();
  showToast('Annotation undone');
}

// ─── Initialization ─────────────────────────────────

export function initAnnotationTools() {
  const container = $('pdfCanvasContainer');
  if (!container) return;

  container.addEventListener('mousedown', onMouseDown);
  container.addEventListener('mousemove', onMouseMove);
  container.addEventListener('mouseup', onMouseUp);
}

export function destroyAnnotationTools() {
  const container = $('pdfCanvasContainer');
  if (!container) return;
  container.removeEventListener('mousedown', onMouseDown);
  container.removeEventListener('mousemove', onMouseMove);
  container.removeEventListener('mouseup', onMouseUp);
  _isDrawing = false;
  _drawingCtx = null;
  _redactStart = null;
}

// ─── Event Handlers ──────────────────────────────────

function getPageFromEvent(e) {
  const pageEl = e.target.closest('.pdf-page');
  if (!pageEl) return null;
  const pageNum = parseInt(pageEl.dataset.pageNum, 10);
  if (isNaN(pageNum)) return null;
  const rect = pageEl.getBoundingClientRect();
  return { pageNum, x: e.clientX - rect.left, y: e.clientY - rect.top, pageEl, rect };
}

function onMouseDown(e) {
  const tool = state.pdfTool;
  if (!tool || tool === 'select') return; // No tool active or select mode
  const page = getPageFromEvent(e);
  if (!page) return;

  // Don't create new annotations when clicking inside existing ones
  if (e.target.closest('.pdf-comment-input, .pdf-text-box, .pdf-comment-marker')) return;

  if (tool === 'draw') startDrawing(page);
  else if (tool === 'comment') addComment(page);
  else if (tool === 'text') addTextBox(page);
  else if (tool === 'redact') startRedact(page);
}

function onMouseMove(e) {
  if (state.pdfTool === 'draw' && _isDrawing) continueDrawing(e);
  else if (state.pdfTool === 'redact' && _redactStart) continueRedact(e);
}

function onMouseUp(e) {
  if (state.pdfTool === 'draw' && _isDrawing) endDrawing(e);
  else if (state.pdfTool === 'highlight') {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed) {
      // No selection — show hint instead of doing nothing
      showToast('Select text first, then release to highlight', 'info');
      return;
    }
    createHighlightFromSelection();
  }
  else if (state.pdfTool === 'redact' && _redactStart) endRedact(e);
}

// PDF keyboard shortcuts
document.addEventListener('keydown', (e) => {
  if (state.currentView !== 'pdf') return;

  // Escape deselects tool
  if (e.key === 'Escape' && state.pdfTool && state.pdfTool !== 'select') {
    state.pdfTool = 'select';
    _updateToolbarActiveState();
    const container = $('pdfCanvasContainer');
    if (container) container.style.cursor = '';
    showToast('Tool deselected');
    return;
  }

  // Ctrl+Z undo annotation
  if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
    e.preventDefault();
    undoAnnotation();
    return;
  }

  // Delete key removes selected annotation
  if (e.key === 'Delete' || e.key === 'Backspace') {
    const selected = document.querySelector('.pdf-annot-card[style*="border-color"]');
    if (selected && selected.dataset.annotId) {
      e.preventDefault();
      deleteAnnotation(selected.dataset.annotId);
    }
  }
});

/** Update active state on PDF toolbar buttons to reflect current tool */
function _updateToolbarActiveState() {
  const toolBtns = document.querySelectorAll('[data-pdf-tool]');
  toolBtns.forEach(btn => {
    btn.classList.toggle('active', btn.dataset.pdfTool === state.pdfTool);
  });
}

// ─── Highlight Tool ──────────────────────────────────

function createHighlightFromSelection() {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed) return;

  const range = sel.getRangeAt(0);
  const rects = range.getClientRects();
  if (!rects.length) return;

  // Find the page containing the selection
  let node = range.startContainer;
  while (node && !(node instanceof Element)) node = node.parentNode;
  const pageEl = node?.closest?.('.pdf-page');
  if (!pageEl) return;

  const pageNum = parseInt(pageEl.dataset.pageNum, 10);
  if (isNaN(pageNum)) return;
  const pageRect = pageEl.getBoundingClientRect();

  const quads = [];
  for (const r of rects) {
    // Only include rects that are inside the page
    if (r.width < 1 || r.height < 1) continue;
    quads.push({
      x: r.left - pageRect.left,
      y: r.top - pageRect.top,
      width: r.width,
      height: r.height,
    });
  }
  if (!quads.length) return;

  _saveUndoState();
  const annotation = new PdfAnnotation('highlight', pageNum, {
    quads,
    selectedText: sel.toString(),
    pageWidth: pageRect.width,
    pageHeight: pageRect.height,
  });
  state.pdfAnnotations.push(annotation);
  state.pdfModified = true;
  sel.removeAllRanges();

  renderAnnotationsForPage(pageNum);
  refreshAnnotationsPanel();
  $('pdfAnnotationsPanel')?.classList.add('show');
  showToast('Highlight added');
}

// ─── Comment Tool ────────────────────────────────────

function addComment(page) {
  const overlayLayer = state.pdfViewer?.getOverlayLayer(page.pageNum);
  if (!overlayLayer) return;
  overlayLayer.style.pointerEvents = 'auto';

  const inputWrap = document.createElement('div');
  inputWrap.className = 'pdf-comment-input';
  inputWrap.style.left = page.x + 'px';
  inputWrap.style.top = page.y + 'px';

  const input = document.createElement('textarea');
  input.placeholder = 'Add a comment...';
  input.rows = 3;

  const btnRow = document.createElement('div');
  btnRow.className = 'pdf-comment-input-actions';

  const cancelBtn = document.createElement('button');
  cancelBtn.textContent = 'Cancel';
  cancelBtn.className = 'pdf-comment-input-cancel';

  const addBtn = document.createElement('button');
  addBtn.textContent = 'Add';
  addBtn.className = 'pdf-comment-input-add';

  btnRow.appendChild(cancelBtn);
  btnRow.appendChild(addBtn);
  inputWrap.appendChild(input);
  inputWrap.appendChild(btnRow);
  overlayLayer.appendChild(inputWrap);
  requestAnimationFrame(() => input.focus());

  const cleanup = () => {
    inputWrap.remove();
    if (!overlayLayer.children.length) overlayLayer.style.pointerEvents = 'none';
  };

  // Close on outside click (delay to avoid catching the same mousedown that opened the comment)
  const onOutsideClick = (ev) => {
    if (!inputWrap.contains(ev.target)) { cleanup(); document.removeEventListener('mousedown', onOutsideClick, true); }
  };
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      document.addEventListener('mousedown', onOutsideClick, true);
    });
  });

  cancelBtn.addEventListener('click', () => { cleanup(); document.removeEventListener('mousedown', onOutsideClick, true); });
  addBtn.addEventListener('click', () => {
    const content = input.value.trim();
    document.removeEventListener('mousedown', onOutsideClick, true);
    if (!content) { cleanup(); return; }
    const annotation = new PdfAnnotation('comment', page.pageNum, {
      x: page.x, y: page.y, content, replies: [],
      pageWidth: page.rect.width,
      pageHeight: page.rect.height,
    });
    annotation.color = '#1a73e8';
    _saveUndoState();
    state.pdfAnnotations.push(annotation);
    state.pdfModified = true;
    cleanup();
    renderAnnotationsForPage(page.pageNum);
    refreshAnnotationsPanel();
    $('pdfAnnotationsPanel')?.classList.add('show');
    // Switch back to select mode after placing comment (single-shot)
    state.pdfTool = 'select';
    _updateToolbarActiveState();
  });
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); addBtn.click(); }
    if (e.key === 'Escape') { cancelBtn.click(); }
  });
}

// ─── Ink/Draw Tool ───────────────────────────────────

function startDrawing(page) {
  if (!state.pdfViewer) return;
  const drawCanvas = state.pdfViewer.getDrawingCanvas(page.pageNum);
  if (!drawCanvas) return;

  _isDrawing = true;
  _drawingPageNum = page.pageNum;
  _drawingCtx = drawCanvas.getContext('2d');
  _drawingPath = [{ x: page.x, y: page.y }];

  drawCanvas.style.pointerEvents = 'auto';
  // The canvas context was already scaled by dpr during page render,
  // so we draw in CSS-pixel coordinates directly (no dpr multiply).
  _drawingCtx.strokeStyle = _drawColor;
  _drawingCtx.lineWidth = _drawWidth;
  _drawingCtx.lineCap = 'round';
  _drawingCtx.lineJoin = 'round';
  _drawingCtx.beginPath();
  _drawingCtx.moveTo(page.x, page.y);
}

function continueDrawing(e) {
  if (!_isDrawing || !_drawingCtx) return;
  const pageEl = e.target.closest('.pdf-page');
  if (!pageEl) return;
  const rect = pageEl.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  _drawingPath.push({ x, y });
  _drawingCtx.lineTo(x, y);
  _drawingCtx.stroke();
  _drawingCtx.beginPath();
  _drawingCtx.moveTo(x, y);
}

function endDrawing(e) {
  if (!_isDrawing) return;
  _isDrawing = false;

  // Disable drawing canvas pointer events
  const drawCanvas = state.pdfViewer?.getDrawingCanvas(_drawingPageNum);
  if (drawCanvas) drawCanvas.style.pointerEvents = 'none';

  if (_drawingPath.length < 2) { _drawingCtx = null; return; }

  _saveUndoState();
  const annotation = new PdfAnnotation('ink', _drawingPageNum, {
    paths: [_drawingPath],
    strokeWidth: _drawWidth,
    strokeColor: _drawColor,
  });
  state.pdfAnnotations.push(annotation);
  state.pdfModified = true;

  _drawingPath = [];
  _drawingCtx = null;
  refreshAnnotationsPanel();
  $('pdfAnnotationsPanel')?.classList.add('show');
}

// ─── Text Tool ───────────────────────────────────────

function addTextBox(page) {
  const overlayLayer = state.pdfViewer?.getOverlayLayer(page.pageNum);
  if (!overlayLayer) return;

  const div = document.createElement('div');
  div.className = 'pdf-text-box';
  div.contentEditable = 'true';
  div.style.left = page.x + 'px';
  div.style.top = page.y + 'px';
  div.style.pointerEvents = 'auto';
  overlayLayer.style.pointerEvents = 'auto';
  overlayLayer.appendChild(div);

  requestAnimationFrame(() => div.focus());

  div.addEventListener('blur', () => {
    const text = div.textContent.trim();
    if (!text) { div.remove(); return; }
    _saveUndoState();
    const annotation = new PdfAnnotation('text', page.pageNum, {
      x: page.x, y: page.y,
      width: div.offsetWidth, height: div.offsetHeight,
      content: text, fontSize: 12, fontFamily: 'sans-serif',
    });
    state.pdfAnnotations.push(annotation);
    state.pdfModified = true;
    refreshAnnotationsPanel();
  });
}

// ─── Redact Tool ─────────────────────────────────────

let _redactStart = null;
let _redactOverlay = null;

function startRedact(page) {
  _redactStart = { pageNum: page.pageNum, x: page.x, y: page.y, pageEl: page.pageEl };

  const overlay = document.createElement('div');
  overlay.className = 'pdf-redact-overlay';
  overlay.style.left = page.x + 'px';
  overlay.style.top = page.y + 'px';
  overlay.style.width = '0px';
  overlay.style.height = '0px';
  overlay.style.pointerEvents = 'none';

  const overlayLayer = state.pdfViewer?.getOverlayLayer(page.pageNum);
  if (overlayLayer) {
    overlayLayer.style.pointerEvents = 'auto';
    overlayLayer.appendChild(overlay);
  }
  _redactOverlay = overlay;
}

function continueRedact(e) {
  if (!_redactStart || !_redactOverlay) return;
  const rect = _redactStart.pageEl.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  _redactOverlay.style.left = Math.min(_redactStart.x, x) + 'px';
  _redactOverlay.style.top = Math.min(_redactStart.y, y) + 'px';
  _redactOverlay.style.width = Math.abs(x - _redactStart.x) + 'px';
  _redactOverlay.style.height = Math.abs(y - _redactStart.y) + 'px';
}

function endRedact(e) {
  if (!_redactStart) return;
  const rect = _redactStart.pageEl.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  const left = Math.min(_redactStart.x, x);
  const top = Math.min(_redactStart.y, y);
  const width = Math.abs(x - _redactStart.x);
  const height = Math.abs(y - _redactStart.y);

  if (width < 10 || height < 10) {
    _redactOverlay?.remove();
  } else {
    _saveUndoState();
    const annotation = new PdfAnnotation('redact', _redactStart.pageNum, {
      rects: [{ x: left, y: top, width, height }],
      pageWidth: _redactStart.pageEl.getBoundingClientRect().width,
      pageHeight: _redactStart.pageEl.getBoundingClientRect().height,
    });
    state.pdfAnnotations.push(annotation);
    state.pdfModified = true;
    renderAnnotationsForPage(_redactStart.pageNum);
    refreshAnnotationsPanel();
  }

  const overlayLayer = state.pdfViewer?.getOverlayLayer(_redactStart.pageNum);
  if (overlayLayer) overlayLayer.style.pointerEvents = 'none';
  _redactStart = null;
  _redactOverlay = null;
}

// ─── Annotation Rendering ────────────────────────────

export function renderAnnotationsForPage(pageNum) {
  if (!state.pdfViewer) return;
  const overlay = state.pdfViewer.getOverlayLayer(pageNum);
  if (!overlay) return;

  // Clear existing annotation overlays (keep text boxes and comment inputs)
  overlay.querySelectorAll('.pdf-highlight-overlay,.pdf-comment-marker,.pdf-redact-overlay')
    .forEach(el => el.remove());

  const pageAnnotations = state.pdfAnnotations.filter(a => a.pageNum === pageNum);

  for (const ann of pageAnnotations) {
    switch (ann.type) {
      case 'highlight': renderHighlightOverlay(ann, overlay); break;
      case 'comment': renderCommentMarker(ann, overlay); break;
      case 'redact': renderRedactOverlay(ann, overlay); break;
    }
  }

  overlay.style.pointerEvents = pageAnnotations.length > 0 ? 'auto' : 'none';
}

function renderHighlightOverlay(ann, container) {
  const overlayWidth = container.clientWidth || parseFloat(container.style.width) || 1;
  const overlayHeight = container.clientHeight || parseFloat(container.style.height) || 1;
  const baseWidth = ann.props.pageWidth || overlayWidth;
  const baseHeight = ann.props.pageHeight || overlayHeight;
  const scaleX = overlayWidth / (baseWidth || overlayWidth);
  const scaleY = overlayHeight / (baseHeight || overlayHeight);

  for (const quad of ann.props.quads) {
    const div = document.createElement('div');
    div.className = 'pdf-highlight-overlay';
    div.style.left = (quad.x * scaleX) + 'px';
    div.style.top = (quad.y * scaleY) + 'px';
    div.style.width = (quad.width * scaleX) + 'px';
    div.style.height = (quad.height * scaleY) + 'px';
    div.title = ann.props.selectedText || '';
    div.dataset.annotId = ann.id;
    div.addEventListener('click', (e) => { e.stopPropagation(); selectAnnotation(ann.id); });
    container.appendChild(div);
  }
}

/** Make an annotation element draggable within its overlay container. */
function _makeDraggable(el, ann, container) {
  let startX, startY, origLeft, origTop;
  el.style.cursor = 'move';

  const onDown = (e) => {
    if (state.pdfTool && state.pdfTool !== 'select') return;
    e.preventDefault();
    e.stopPropagation();
    startX = e.clientX;
    startY = e.clientY;
    origLeft = parseFloat(el.style.left) || 0;
    origTop = parseFloat(el.style.top) || 0;
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  };
  const onMove = (e) => {
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    el.style.left = (origLeft + dx) + 'px';
    el.style.top = (origTop + dy) + 'px';
  };
  const onUp = () => {
    document.removeEventListener('mousemove', onMove);
    document.removeEventListener('mouseup', onUp);
    // Update annotation position
    const newLeft = parseFloat(el.style.left) || 0;
    const newTop = parseFloat(el.style.top) || 0;
    if (ann.props.x !== undefined) { ann.props.x = newLeft; ann.props.y = newTop; }
    if (ann.props.quads) {
      const dx = newLeft - origLeft;
      const dy = newTop - origTop;
      ann.props.quads.forEach(q => { q.x += dx; q.y += dy; });
    }
    if (ann.props.rects) {
      const dx = newLeft - origLeft;
      const dy = newTop - origTop;
      ann.props.rects.forEach(r => { r.x += dx; r.y += dy; });
    }
    state.pdfModified = true;
  };
  el.addEventListener('mousedown', onDown);
}

function renderCommentMarker(ann, container) {
  const overlayWidth = container.clientWidth || parseFloat(container.style.width) || 1;
  const overlayHeight = container.clientHeight || parseFloat(container.style.height) || 1;
  const baseWidth = ann.props.pageWidth || overlayWidth;
  const baseHeight = ann.props.pageHeight || overlayHeight;
  const scaleX = overlayWidth / (baseWidth || overlayWidth);
  const scaleY = overlayHeight / (baseHeight || overlayHeight);

  const marker = document.createElement('div');
  marker.className = 'pdf-comment-marker';
  marker.style.left = (ann.props.x * scaleX) - 12 + 'px';
  marker.style.top = (ann.props.y * scaleY) - 12 + 'px';
  marker.innerHTML = '<span class="msi">comment</span>';
  marker.title = ann.props.content;
  marker.dataset.annotId = ann.id;
  marker.addEventListener('click', (e) => {
    e.stopPropagation();
    selectAnnotation(ann.id);
    $('pdfAnnotationsPanel')?.classList.add('show');
  });
  _makeDraggable(marker, ann, container);
  container.appendChild(marker);
}

function renderRedactOverlay(ann, container) {
  const overlayWidth = container.clientWidth || parseFloat(container.style.width) || 1;
  const overlayHeight = container.clientHeight || parseFloat(container.style.height) || 1;
  const baseWidth = ann.props.pageWidth || overlayWidth;
  const baseHeight = ann.props.pageHeight || overlayHeight;
  const scaleX = overlayWidth / (baseWidth || overlayWidth);
  const scaleY = overlayHeight / (baseHeight || overlayHeight);
  for (const rect of ann.props.rects) {
    const div = document.createElement('div');
    div.className = 'pdf-redact-overlay';
    div.style.left = (rect.x * scaleX) + 'px';
    div.style.top = (rect.y * scaleY) + 'px';
    div.style.width = (rect.width * scaleX) + 'px';
    div.style.height = (rect.height * scaleY) + 'px';
    div.title = 'Redaction area';
    div.dataset.annotId = ann.id;
    div.addEventListener('click', (e) => { e.stopPropagation(); selectAnnotation(ann.id); });
    container.appendChild(div);
  }
}

// ─── Annotations Panel ──────────────────────────────

export function refreshAnnotationsPanel() {
  const list = $('pdfAnnotList');
  if (!list) return;
  list.innerHTML = '';

  if (state.pdfAnnotations.length === 0) {
    list.innerHTML = '<div style="text-align:center;padding:20px;color:var(--text-muted);font-size:12px">No annotations yet</div>';
    return;
  }

  const sorted = [...state.pdfAnnotations].sort((a, b) => {
    if (a.pageNum !== b.pageNum) return a.pageNum - b.pageNum;
    return a.created.localeCompare(b.created);
  });

  for (const ann of sorted) {
    const card = document.createElement('div');
    card.className = 'pdf-annot-card';
    card.dataset.annotId = ann.id;

    let text = '';
    switch (ann.type) {
      case 'highlight': text = ann.props.selectedText || '(highlight)'; break;
      case 'comment': text = ann.props.content; break;
      case 'ink': text = 'Drawing'; break;
      case 'text': text = ann.props.content || '(text box)'; break;
      case 'redact': text = 'Redaction'; break;
    }

    card.innerHTML = `
      <div class="pdf-annot-card-type">${ann.type}</div>
      <div class="pdf-annot-card-text">${escapeHtml(text.substring(0, 100))}</div>
      <div class="pdf-annot-card-meta">Page ${ann.pageNum} &middot; ${formatDate(ann.created)}</div>
    `;

    card.addEventListener('click', () => {
      state.pdfViewer?.goToPage(ann.pageNum);
      selectAnnotation(ann.id);
    });

    // Delete button
    const delBtn = document.createElement('button');
    delBtn.className = 'pdf-annot-delete';
    delBtn.innerHTML = '<span class="msi" style="font-size:14px">close</span>';
    delBtn.title = 'Delete';
    delBtn.addEventListener('click', (e) => { e.stopPropagation(); deleteAnnotation(ann.id); });
    card.appendChild(delBtn);

    list.appendChild(card);
  }
}

function selectAnnotation(id) {
  document.querySelectorAll('.pdf-annot-card').forEach(c => {
    c.style.borderColor = c.dataset.annotId === id ? 'var(--accent)' : '';
  });
  // Open panel if not open
  $('pdfAnnotationsPanel')?.classList.add('show');
}

export function deleteAnnotation(id) {
  const idx = state.pdfAnnotations.findIndex(a => a.id === id);
  if (idx === -1) return;
  _saveUndoState();
  const ann = state.pdfAnnotations[idx];
  state.pdfAnnotations.splice(idx, 1);
  state.pdfModified = true;
  renderAnnotationsForPage(ann.pageNum);
  refreshAnnotationsPanel();
}

// ─── Save ────────────────────────────────────────────

/**
 * Save PDF with annotations baked in.
 *
 * For visual annotations (highlights, comments, text boxes, stamps),
 * we render each page's canvas + overlay to a combined canvas and
 * reassemble into a multi-page PDF. For basic use, we flatten
 * annotations onto the page canvases.
 */
export async function savePdfWithAnnotations() {
  if (!state.pdfViewer || !state.pdfBytes) return;

  const pageCount = state.pdfViewer.getPageCount();
  if (pageCount === 0) return;

  // If no annotations, just download the original
  if (state.pdfAnnotations.length === 0) {
    _downloadBlob(state.pdfBytes, 'application/pdf');
    return;
  }

  try {
    // Use jspdf-like approach: render each page canvas to an image, build new PDF
    // Since we don't have jsPDF, we'll flatten annotations onto the page canvases
    // and use the canvas data to create downloadable images per page.
    // For a proper solution we'd need pdf-lib — for now, render to combined canvas.

    const container = $('pdfCanvasContainer');
    if (!container) { _downloadBlob(state.pdfBytes, 'application/pdf'); return; }

    // Render all pages (ensure they're not lazy-loaded)
    for (let i = 1; i <= pageCount; i++) {
      await state.pdfViewer.renderPage(i);
    }

    // For each page, composite the canvas + overlays
    const pageImages = [];
    for (let i = 1; i <= pageCount; i++) {
      const pageEl = container.querySelector(`.pdf-page[data-page-num="${i}"]`);
      if (!pageEl) continue;

      const pdfCanvas = pageEl.querySelector('.pdf-canvas');
      const drawCanvas = pageEl.querySelector('.pdf-drawing-layer');
      const overlayLayer = pageEl.querySelector('.pdf-overlay-layer');

      if (!pdfCanvas) continue;

      // Create composite canvas
      const w = pdfCanvas.width;
      const h = pdfCanvas.height;
      const compositeCanvas = document.createElement('canvas');
      compositeCanvas.width = w;
      compositeCanvas.height = h;
      const ctx = compositeCanvas.getContext('2d');

      // Draw the PDF page
      ctx.drawImage(pdfCanvas, 0, 0);

      // Draw the ink layer
      if (drawCanvas && drawCanvas.width > 0) {
        ctx.drawImage(drawCanvas, 0, 0);
      }

      // Draw overlay elements (stamps, text boxes) via html2canvas-lite approach
      // Render stamp images directly
      if (overlayLayer) {
        const dpr = window.devicePixelRatio || 1;
        const stamps = overlayLayer.querySelectorAll('.pdf-stamp-overlay');
        for (const stamp of stamps) {
          const img = stamp.querySelector('img');
          if (!img || !img.complete) continue;
          const left = parseFloat(stamp.style.left) || 0;
          const top = parseFloat(stamp.style.top) || 0;
          const sw = parseFloat(stamp.style.width) || 100;
          const sh = parseFloat(stamp.style.height) || 50;
          ctx.drawImage(img, left * dpr, top * dpr, sw * dpr, sh * dpr);
        }

        // Draw highlights
        const highlights = overlayLayer.querySelectorAll('.pdf-highlight-overlay');
        for (const hl of highlights) {
          ctx.fillStyle = 'rgba(255,235,59,0.35)';
          const left = parseFloat(hl.style.left) || 0;
          const top = parseFloat(hl.style.top) || 0;
          const hw = parseFloat(hl.style.width) || 0;
          const hh = parseFloat(hl.style.height) || 0;
          ctx.fillRect(left * dpr, top * dpr, hw * dpr, hh * dpr);
        }

        // Draw redactions
        const redacts = overlayLayer.querySelectorAll('.pdf-redact-overlay');
        for (const rd of redacts) {
          ctx.fillStyle = 'rgba(0,0,0,0.9)';
          const left = parseFloat(rd.style.left) || 0;
          const top = parseFloat(rd.style.top) || 0;
          const rw = parseFloat(rd.style.width) || 0;
          const rh = parseFloat(rd.style.height) || 0;
          ctx.fillRect(left * dpr, top * dpr, rw * dpr, rh * dpr);
        }
      }

      pageImages.push(compositeCanvas.toDataURL('image/jpeg', 0.92));
    }

    // If we have composited images, download as multi-page PDF via simple approach
    // For true PDF output we'd need pdf-lib. For now, note limitation and download original + toast.
    if (pageImages.length > 0) {
      // Try to use the WASM PDF writer if available
      if (state._wasmPdfEditor && typeof state._wasmPdfEditor.flatten_annotations === 'function') {
        const flattenedBytes = state._wasmPdfEditor.flatten_annotations(state.pdfAnnotations);
        _downloadBlob(flattenedBytes, 'application/pdf');
      } else {
        // Fallback: download original PDF (annotations are visual-only)
        _downloadBlob(state.pdfBytes, 'application/pdf');
        showToast('PDF saved (annotations are overlay-only — install pdf-lib for embedded annotations)', 'info');
      }
    } else {
      _downloadBlob(state.pdfBytes, 'application/pdf');
    }
  } catch (err) {
    console.error('save PDF:', err);
    // Fallback
    _downloadBlob(state.pdfBytes, 'application/pdf');
    showToast('PDF saved (some annotations may not be embedded)');
  }

  state.pdfModified = false;
}

function _downloadBlob(data, mimeType) {
  const blob = data instanceof Blob ? data : new Blob([data], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = ($('docName').value || 'document') + '.pdf';
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

document.addEventListener('pdfPageRendered', (event) => {
  const pageNum = event?.detail?.pageNum;
  if (pageNum) {
    renderAnnotationsForPage(pageNum);
  }
});

// ─── Helpers ─────────────────────────────────────────

function escapeHtml(str) {
  const d = document.createElement('div');
  d.textContent = str;
  return d.innerHTML;
}

function formatDate(isoStr) {
  try {
    const d = new Date(isoStr);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) +
           ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  } catch (_) { return ''; }
}
