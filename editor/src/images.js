// Image selection, resize, drag, alignment
import { state, $ } from './state.js';
import { renderDocument } from './render.js';
import { updateUndoRedo } from './toolbar.js';
import { getActiveNodeId } from './selection.js';
import { broadcastOp } from './collab.js';

export function setupImages(scope) {
  const root = scope || $('pageContainer');
  const imgs = root.tagName === 'IMG' ? [root] : root.querySelectorAll('img');
  imgs.forEach(img => {
    img.addEventListener('click', e => { e.preventDefault(); e.stopPropagation(); selectImage(img); });
    // Enable drag to move images between paragraphs
    img.setAttribute('draggable', 'true');
    img.addEventListener('dragstart', onImageDragStart);
  });
  // Set up drop targets on the page
  if (!root._dropSetup) {
    const page = $('pageContainer');
    page.addEventListener('dragover', onDragOver);
    page.addEventListener('drop', onDrop);
    page._dropSetup = true;
  }
}

// ─── Image Drag & Drop ─────────────────────
let _draggedImgNodeId = null;
let _dropIndicator = null;

function onImageDragStart(e) {
  const img = e.target;
  const nodeEl = img.closest('[data-node-id]');
  if (!nodeEl) { e.preventDefault(); return; }
  _draggedImgNodeId = nodeEl.dataset.nodeId;
  e.dataTransfer.effectAllowed = 'move';
  e.dataTransfer.setData('text/plain', _draggedImgNodeId);
  img.style.opacity = '0.5';
  img.addEventListener('dragend', () => {
    img.style.opacity = '';
    _draggedImgNodeId = null;
    if (_dropIndicator) { _dropIndicator.remove(); _dropIndicator = null; }
  }, { once: true });
}

function onDragOver(e) {
  if (!_draggedImgNodeId) return;
  e.preventDefault();
  e.dataTransfer.dropEffect = 'move';

  // Show drop indicator
  const target = findDropTarget(e);
  if (target) {
    if (!_dropIndicator) {
      _dropIndicator = document.createElement('div');
      _dropIndicator.style.cssText = 'height:3px;background:var(--accent,#1a73e8);margin:2px 0;border-radius:2px;pointer-events:none;transition:opacity .1s';
    }
    const rect = target.getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    if (e.clientY < midY) {
      target.before(_dropIndicator);
    } else {
      target.after(_dropIndicator);
    }
  }
}

function onDrop(e) {
  e.preventDefault();
  if (_dropIndicator) _dropIndicator.remove();
  _dropIndicator = null;

  if (!_draggedImgNodeId || !state.doc) {
    _draggedImgNodeId = null;
    return;
  }

  const target = findDropTarget(e);
  if (target && target.dataset.nodeId) {
    const targetNodeId = target.dataset.nodeId;
    if (targetNodeId !== _draggedImgNodeId) {
      try {
        const rect = target.getBoundingClientRect();
        const midY = rect.top + rect.height / 2;
        if (e.clientY < midY) {
          state.doc.move_node_before(_draggedImgNodeId, targetNodeId);
          broadcastOp({ action: 'moveNodeBefore', nodeId: _draggedImgNodeId, beforeId: targetNodeId });
        } else {
          state.doc.move_node_after(_draggedImgNodeId, targetNodeId);
          broadcastOp({ action: 'moveNodeAfter', nodeId: _draggedImgNodeId, afterId: targetNodeId });
        }
        renderDocument();
        updateUndoRedo();
      } catch (err) {
        console.error('move image:', err);
      }
    }
  }
  _draggedImgNodeId = null;
}

function findDropTarget(e) {
  const page = $('pageContainer');
  // Only consider body-level block elements (not header/footer content)
  const els = page.querySelectorAll(':scope > [data-node-id]');
  let closest = null;
  let closestDist = Infinity;
  els.forEach(el => {
    if (el.dataset.nodeId === _draggedImgNodeId) return;
    const rect = el.getBoundingClientRect();
    const dist = Math.abs(e.clientY - (rect.top + rect.height / 2));
    if (dist < closestDist) {
      closestDist = dist;
      closest = el;
    }
  });
  return closest;
}

export function selectImage(img) {
  deselectImage();
  state.selectedImg = img;
  img.classList.add('img-selected');
  const wrap = document.createElement('span');
  wrap.className = 'img-wrap';
  wrap.contentEditable = 'false';
  img.parentNode.insertBefore(wrap, img);
  wrap.appendChild(img);
  ['tl', 'tr', 'bl', 'br', 'ml', 'mr'].forEach(pos => {
    const h = document.createElement('span');
    h.className = 'img-handle ' + pos;
    h.dataset.handle = pos;
    h.addEventListener('mousedown', startResize);
    wrap.appendChild(h);
  });
}

export function deselectImage() {
  if (!state.selectedImg) return;
  const img = state.selectedImg;
  const nodeEl = img.closest('[data-node-id]');
  if (nodeEl && state.doc && img.style.width) {
    const imgNodeId = nodeEl.dataset.nodeId;
    if (imgNodeId) {
      const wPt = img.offsetWidth * 0.75;
      const hPt = img.offsetHeight * 0.75;
      try {
        state.doc.resize_image(imgNodeId, wPt, hPt);
        broadcastOp({ action: 'resizeImage', nodeId: imgNodeId, width: wPt, height: hPt });
      } catch (_) {}
    }
  }
  img.classList.remove('img-selected');
  const wrap = img.closest('.img-wrap');
  if (wrap) { wrap.parentNode.insertBefore(img, wrap); wrap.remove(); }
  state.selectedImg = null;
}

function startResize(e) {
  e.preventDefault(); e.stopPropagation();
  const handle = e.target.dataset.handle;
  const img = e.target.closest('.img-wrap').querySelector('img');
  if (!img) return;
  state.resizing = {
    img, handle,
    startX: e.clientX, startY: e.clientY,
    startW: img.offsetWidth, startH: img.offsetHeight,
    ratio: img.offsetHeight / img.offsetWidth,
  };
  document.addEventListener('mousemove', doResize);
  document.addEventListener('mouseup', stopResize);
}

// E-08: Throttled resize persist — save to WASM every 500ms during drag
let _resizePersistTimer = null;

function persistResizeDuringDrag(img) {
  const nodeEl = img.closest('[data-node-id]');
  if (!nodeEl || !state.doc) return;
  const imgNodeId = nodeEl.dataset.nodeId;
  if (!imgNodeId) return;
  const wPt = img.offsetWidth * 0.75;
  const hPt = img.offsetHeight * 0.75;
  try {
    state.doc.resize_image(imgNodeId, wPt, hPt);
  } catch (_) {}
}

function doResize(e) {
  const r = state.resizing;
  if (!r) return;
  const dx = e.clientX - r.startX;
  const dy = e.clientY - r.startY;
  let newW = r.startW, newH = r.startH;
  if (r.handle.includes('r')) newW = r.startW + dx;
  if (r.handle.includes('l')) newW = r.startW - dx;
  if (r.handle.includes('b')) newH = r.startH + dy;
  if (r.handle.includes('t')) newH = r.startH - dy;
  if (r.handle === 'br' || r.handle === 'tl' || r.handle === 'tr' || r.handle === 'bl') {
    newH = newW * r.ratio;
  }
  newW = Math.max(20, newW); newH = Math.max(20, newH);
  r.img.style.width = newW + 'px';
  r.img.style.height = newH + 'px';
  // E-08: Throttled persist during drag so resize is not lost on unexpected close
  if (!_resizePersistTimer) {
    _resizePersistTimer = setTimeout(() => {
      _resizePersistTimer = null;
      persistResizeDuringDrag(r.img);
    }, 500);
  }
}

function stopResize() {
  document.removeEventListener('mousemove', doResize);
  document.removeEventListener('mouseup', stopResize);
  // E-08: Clear throttle timer on mouseup (deselectImage will do the final persist)
  clearTimeout(_resizePersistTimer);
  _resizePersistTimer = null;
  state.resizing = null;
}

export function insertImage(file) {
  const { doc } = state;
  if (!doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  const reader = new FileReader();
  reader.onload = () => {
    const bytes = new Uint8Array(reader.result);
    const type = file.type || 'image/png';
    const img = new Image();
    const url = URL.createObjectURL(file);
    img.onload = () => {
      // Convert pixels to points (1pt = 1/72in, 1px = 1/96in at standard DPI)
      const pxToPt = 72 / 96;
      let w = img.naturalWidth * pxToPt, h = img.naturalHeight * pxToPt;
      // Cap at 468pt (6.5in) page content width
      if (w > 468) { h *= 468 / w; w = 468; }
      try { doc.insert_image(nodeId, bytes, type, w, h); broadcastOp({ action: 'insertImage', afterNodeId: nodeId }); renderDocument(); updateUndoRedo(); }
      catch (e) { console.error('insert image:', e); }
      URL.revokeObjectURL(url);
    };
    img.onerror = () => {
      try {
        doc.insert_image(nodeId, bytes, type, 300, 200);
        broadcastOp({ action: 'insertImage', afterNodeId: nodeId });
        renderDocument();
        updateUndoRedo();
      } catch (e) { console.error('insert image:', e); }
      URL.revokeObjectURL(url);
    };
    img.src = url;
  };
  reader.readAsArrayBuffer(file);
}

export function deleteSelectedImage() {
  if (!state.selectedImg || !state.doc) return;
  const imgEl = state.selectedImg.closest('[data-node-id]');
  if (imgEl) {
    try {
      const imgNodeId = imgEl.dataset.nodeId;
      state.doc.delete_image(imgNodeId);
      broadcastOp({ action: 'deleteNode', nodeId: imgNodeId });
      deselectImage();
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('delete image:', e); }
  }
}

// Global click to deselect
document.addEventListener('mousedown', e => {
  if (state.selectedImg && !e.target.closest('.img-wrap') && e.target !== state.selectedImg && !e.target.closest('#imageContextMenu')) {
    deselectImage();
  }
});

// ─── Image Context Menu ──────────────────────────
export function initImageContextMenu() {
  const page = $('pageContainer');
  page.addEventListener('contextmenu', e => {
    const img = e.target.closest('img');
    if (!img || !state.doc) return;
    // Only show image context menu, not the table one
    const cell = e.target.closest('td, th');
    if (cell) return; // let table context menu handle cells

    e.preventDefault();
    selectImage(img);

    const nodeEl = img.closest('[data-node-id]');
    if (!nodeEl) return;
    state._ctxImageNodeId = nodeEl.dataset.nodeId;

    const menu = document.getElementById('imageContextMenu');
    menu.style.display = 'block';
    const menuW = 200, menuH = 200;
    const x = Math.min(e.clientX, window.innerWidth - menuW);
    const y = Math.min(e.clientY, window.innerHeight - menuH);
    menu.style.left = Math.max(0, x) + 'px';
    menu.style.top = Math.max(0, y) + 'px';
  });

  // Close image context menu on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('#imageContextMenu')) {
      document.getElementById('imageContextMenu').style.display = 'none';
    }
  });

  // Align left
  document.getElementById('imAlignLeft').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    try {
      state.doc.set_alignment(state._ctxImageNodeId, 'left');
      broadcastOp({ action: 'setAlignment', nodeId: state._ctxImageNodeId, alignment: 'left' });
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('image align:', e); }
  });

  // Align center
  document.getElementById('imAlignCenter').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    try {
      state.doc.set_alignment(state._ctxImageNodeId, 'center');
      broadcastOp({ action: 'setAlignment', nodeId: state._ctxImageNodeId, alignment: 'center' });
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('image align:', e); }
  });

  // Align right
  document.getElementById('imAlignRight').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    try {
      state.doc.set_alignment(state._ctxImageNodeId, 'right');
      broadcastOp({ action: 'setAlignment', nodeId: state._ctxImageNodeId, alignment: 'right' });
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('image align:', e); }
  });

  // Alt text — open modal
  document.getElementById('imAltText').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    // Pre-fill current alt text
    const img = state.selectedImg;
    const currentAlt = img ? (img.getAttribute('alt') || '') : '';
    const input = document.getElementById('altTextInput');
    input.value = currentAlt;
    document.getElementById('altTextModal').classList.add('show');
    input.focus();
  });

  // Alt text modal handlers
  document.getElementById('altTextCancelBtn').addEventListener('click', () => {
    document.getElementById('altTextModal').classList.remove('show');
  });
  document.getElementById('altTextSaveBtn').addEventListener('click', () => {
    const alt = document.getElementById('altTextInput').value.trim();
    document.getElementById('altTextModal').classList.remove('show');
    if (!state._ctxImageNodeId || !state.doc) return;
    try {
      state.doc.set_image_alt_text(state._ctxImageNodeId, alt);
      broadcastOp({ action: 'setImageAltText', nodeId: state._ctxImageNodeId, alt });
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('alt text:', e); }
  });
  document.getElementById('altTextModal').addEventListener('click', e => {
    if (e.target.id === 'altTextModal') document.getElementById('altTextModal').classList.remove('show');
  });
  document.getElementById('altTextInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); document.getElementById('altTextSaveBtn').click(); }
    if (e.key === 'Escape') document.getElementById('altTextModal').classList.remove('show');
  });

  // Delete image from context menu
  document.getElementById('imDelete').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    deleteSelectedImage();
  });
}
