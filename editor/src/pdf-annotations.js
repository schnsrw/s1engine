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
  const page = getPageFromEvent(e);
  if (!page) return;

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
  else if (state.pdfTool === 'highlight') createHighlightFromSelection();
  else if (state.pdfTool === 'redact' && _redactStart) endRedact(e);
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

  const annotation = new PdfAnnotation('highlight', pageNum, {
    quads,
    selectedText: sel.toString(),
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

  // Close on outside click
  const onOutsideClick = (ev) => {
    if (!inputWrap.contains(ev.target)) { cleanup(); document.removeEventListener('mousedown', onOutsideClick, true); }
  };
  setTimeout(() => document.addEventListener('mousedown', onOutsideClick, true), 0);

  cancelBtn.addEventListener('click', () => { cleanup(); document.removeEventListener('mousedown', onOutsideClick, true); });
  addBtn.addEventListener('click', () => {
    const content = input.value.trim();
    document.removeEventListener('mousedown', onOutsideClick, true);
    if (!content) { cleanup(); return; }
    const annotation = new PdfAnnotation('comment', page.pageNum, {
      x: page.x, y: page.y, content, replies: [],
    });
    annotation.color = '#1a73e8';
    state.pdfAnnotations.push(annotation);
    state.pdfModified = true;
    cleanup();
    renderAnnotationsForPage(page.pageNum);
    refreshAnnotationsPanel();
    $('pdfAnnotationsPanel')?.classList.add('show');
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
    const annotation = new PdfAnnotation('redact', _redactStart.pageNum, {
      rects: [{ x: left, y: top, width, height }],
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
  for (const quad of ann.props.quads) {
    const div = document.createElement('div');
    div.className = 'pdf-highlight-overlay';
    div.style.left = quad.x + 'px';
    div.style.top = quad.y + 'px';
    div.style.width = quad.width + 'px';
    div.style.height = quad.height + 'px';
    div.title = ann.props.selectedText || '';
    div.dataset.annotId = ann.id;
    div.addEventListener('click', (e) => { e.stopPropagation(); selectAnnotation(ann.id); });
    container.appendChild(div);
  }
}

function renderCommentMarker(ann, container) {
  const marker = document.createElement('div');
  marker.className = 'pdf-comment-marker';
  marker.style.left = (ann.props.x - 12) + 'px';
  marker.style.top = (ann.props.y - 12) + 'px';
  marker.innerHTML = '<span class="msi">comment</span>';
  marker.title = ann.props.content;
  marker.dataset.annotId = ann.id;
  marker.addEventListener('click', (e) => {
    e.stopPropagation();
    selectAnnotation(ann.id);
    $('pdfAnnotationsPanel')?.classList.add('show');
  });
  container.appendChild(marker);
}

function renderRedactOverlay(ann, container) {
  for (const rect of ann.props.rects) {
    const div = document.createElement('div');
    div.className = 'pdf-redact-overlay';
    div.style.left = rect.x + 'px';
    div.style.top = rect.y + 'px';
    div.style.width = rect.width + 'px';
    div.style.height = rect.height + 'px';
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
  const ann = state.pdfAnnotations[idx];
  state.pdfAnnotations.splice(idx, 1);
  state.pdfModified = true;
  renderAnnotationsForPage(ann.pageNum);
  refreshAnnotationsPanel();
}

// ─── Save ────────────────────────────────────────────

export function savePdfWithAnnotations() {
  if (!state.pdfBytes) return;
  const blob = new Blob([state.pdfBytes], { type: 'application/pdf' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = ($('docName').value || 'document') + '.pdf';
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  state.pdfModified = false;
  showToast('PDF downloaded');
}

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
