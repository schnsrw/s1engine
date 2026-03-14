// Image selection, resize, drag, alignment
import { state, $ } from './state.js';
import { renderDocument } from './render.js';
import { updateUndoRedo } from './toolbar.js';
import { getActiveNodeId } from './selection.js';

export function setupImages(scope) {
  const root = scope || $('docPage');
  const imgs = root.tagName === 'IMG' ? [root] : root.querySelectorAll('img');
  imgs.forEach(img => {
    img.addEventListener('click', e => { e.preventDefault(); e.stopPropagation(); selectImage(img); });
    img.setAttribute('draggable', 'false');
  });
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
      try { state.doc.resize_image(imgNodeId, wPt, hPt); } catch (_) {}
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
}

function stopResize() {
  document.removeEventListener('mousemove', doResize);
  document.removeEventListener('mouseup', stopResize);
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
      try { doc.insert_image(nodeId, bytes, type, w, h); renderDocument(); updateUndoRedo(); }
      catch (e) { console.error('insert image:', e); }
      URL.revokeObjectURL(url);
    };
    img.onerror = () => {
      try { doc.insert_image(nodeId, bytes, type, 300, 200); renderDocument(); updateUndoRedo(); }
      catch (e) { console.error('insert image:', e); }
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
      state.doc.delete_image(imgEl.dataset.nodeId);
      deselectImage();
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('delete image:', e); }
  }
}

// Global click to deselect
document.addEventListener('mousedown', e => {
  if (state.selectedImg && !e.target.closest('.img-wrap') && e.target !== state.selectedImg) {
    deselectImage();
  }
});
