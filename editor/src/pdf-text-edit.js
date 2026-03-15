// PDF Text Editing — inline editing of existing PDF text
import { state, $ } from './state.js';

let _activeEdit = null;

export function initPdfTextEdit() {
  const container = $('pdfCanvasContainer');
  if (!container) return;
  container.addEventListener('dblclick', onDoubleClick);
}

export function destroyPdfTextEdit() {
  const container = $('pdfCanvasContainer');
  if (!container) return;
  container.removeEventListener('dblclick', onDoubleClick);
  commitActiveEdit();
}

function onDoubleClick(e) {
  // Only in select tool mode
  if (state.pdfTool !== 'select') return;

  const span = e.target.closest('.pdf-text-layer span');
  if (!span) return;

  const pageEl = span.closest('.pdf-page');
  if (!pageEl) return;
  const pageNum = parseInt(pageEl.dataset.pageNum, 10);
  if (isNaN(pageNum)) return;

  const originalText = span.textContent;
  const spanRect = span.getBoundingClientRect();
  const pageRect = pageEl.getBoundingClientRect();

  const x = spanRect.left - pageRect.left;
  const y = spanRect.top - pageRect.top;
  const width = spanRect.width;
  const height = spanRect.height;
  const fontSize = parseFloat(span.style.fontSize) || 12;

  const overlayLayer = state.pdfViewer?.getOverlayLayer(pageNum);
  if (!overlayLayer) return;

  // Commit any previous active edit
  commitActiveEdit();

  const editDiv = document.createElement('div');
  editDiv.className = 'pdf-text-box pdf-text-edit-active';
  editDiv.contentEditable = 'true';
  editDiv.textContent = originalText;
  editDiv.style.left = x + 'px';
  editDiv.style.top = y + 'px';
  editDiv.style.minWidth = Math.max(width, 40) + 'px';
  editDiv.style.minHeight = height + 'px';
  editDiv.style.fontSize = fontSize + 'px';
  editDiv.style.lineHeight = '1.2';
  editDiv.style.borderStyle = 'solid';
  editDiv.style.background = 'rgba(255,255,255,0.95)';
  editDiv.style.pointerEvents = 'auto';
  editDiv.style.zIndex = '10';

  overlayLayer.style.pointerEvents = 'auto';
  overlayLayer.appendChild(editDiv);

  _activeEdit = {
    element: editDiv,
    pageNum,
    originalText,
    position: { x, y, width, height },
    fontSize,
    fontFamily: span.style.fontFamily || 'sans-serif',
    originalSpan: span,
  };

  // Hide original span
  span.style.visibility = 'hidden';

  // Focus and select all
  requestAnimationFrame(() => {
    editDiv.focus();
    const range = document.createRange();
    range.selectNodeContents(editDiv);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
  });

  editDiv.addEventListener('blur', () => commitActiveEdit());
  editDiv.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter' && !ev.shiftKey) { ev.preventDefault(); editDiv.blur(); }
    if (ev.key === 'Escape') {
      // Cancel — restore original
      if (_activeEdit?.originalSpan) _activeEdit.originalSpan.style.visibility = 'visible';
      editDiv.remove();
      _activeEdit = null;
    }
  });
}

function commitActiveEdit() {
  if (!_activeEdit) return;
  const { element, pageNum, originalText, position, fontSize, fontFamily, originalSpan } = _activeEdit;
  const newText = element.textContent.trim();

  // Restore original span visibility
  if (originalSpan) originalSpan.style.visibility = 'visible';

  if (newText && newText !== originalText) {
    state.pdfTextEdits.push({
      id: crypto.randomUUID(),
      pageNum, originalText, newText, position,
      fontInfo: { size: fontSize, family: fontFamily, color: '#000000' },
    });
    state.pdfModified = true;
    // Update the text span to show new text
    if (originalSpan) originalSpan.textContent = newText;
  }

  element.remove();
  _activeEdit = null;
}
