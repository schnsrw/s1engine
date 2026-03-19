// Image selection, resize, drag, alignment
import { state, $ } from './state.js';
import { renderDocument } from './render.js';
import { updateUndoRedo, recordUndoAction } from './toolbar.js';
import { getActiveNodeId } from './selection.js';
import { broadcastOp } from './collab.js';

let _imgDelegationSetup = false;
export function setupImages(scope) {
  // Set up event delegation ONCE on pageContainer — check flag BEFORE any listener work
  if (!_imgDelegationSetup) {
    const page = $('pageContainer');
    if (page) {
      _imgDelegationSetup = true;
      _setupImageDelegation(page);
    }
  }

  // Mark all images as draggable (no per-element event listeners)
  const root = scope || $('pageContainer');
  if (!root) return;
  const imgs = root.tagName === 'IMG' ? [root] : root.querySelectorAll('img');
  imgs.forEach(img => {
    img.setAttribute('draggable', 'true');
  });
}

function _setupImageDelegation(page) {

  // Delegated click for image selection
  page.addEventListener('click', e => {
    const img = e.target.closest('img');
    if (img && !e.target.closest('.img-handle')) {
      e.preventDefault();
      e.stopPropagation();
      selectImage(img);
    }
  });

  // Delegated dragstart for image drag
  page.addEventListener('dragstart', e => {
    const img = e.target.closest('img');
    if (img) onImageDragStart(e);
  });

  page.addEventListener('dragover', onDragOver);
  page.addEventListener('drop', onDrop);
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
  // Use a custom type so plain text isn't polluted with node IDs
  e.dataTransfer.setData('application/x-s1-image', _draggedImgNodeId);
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
      _dropIndicator.className = 'img-drop-indicator';
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
        // G2: Record undo BEFORE render so it's saved even if renderDocument() throws
        recordUndoAction('Move image');
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
  // Consider block elements across all pages (not just direct children)
  const els = page.querySelectorAll('.page-content > [data-node-id], :scope > [data-node-id]');
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
    h.setAttribute('aria-hidden', 'true');
    h.setAttribute('role', 'presentation');
    h.addEventListener('mousedown', startResize);
    wrap.appendChild(h);
  });
  // Notify properties panel of image selection change
  document.dispatchEvent(new CustomEvent('s1-selection-context-change'));
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
  // Notify properties panel of image deselection
  document.dispatchEvent(new CustomEvent('s1-selection-context-change'));
}

function startResize(e) {
  e.preventDefault(); e.stopPropagation();
  const handle = e.target.dataset.handle;
  const wrap = e.target.closest('.img-wrap');
  if (!wrap) return;
  const img = wrap.querySelector('img');
  if (!img) return;
  // ED2-14: Remove any stale listeners before adding new ones (prevents leaks on tab switch)
  document.removeEventListener('mousemove', doResize);
  document.removeEventListener('mouseup', stopResize);
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
  // offsetWidth/Height are already in CSS pixels (pre-zoom), convert to pt
  const wPt = img.offsetWidth * 0.75;
  const hPt = img.offsetHeight * 0.75;
  try {
    state.doc.resize_image(imgNodeId, wPt, hPt);
  } catch (_) {}
}

function doResize(e) {
  const r = state.resizing;
  if (!r) return;
  // Account for CSS zoom — mouse deltas are in screen pixels, but element
  // dimensions are in CSS pixels which are scaled by zoom
  const zoom = (state.zoomLevel || 100) / 100;
  const dx = (e.clientX - r.startX) / zoom;
  const dy = (e.clientY - r.startY) / zoom;
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

// ED2-14: Clean up resize listeners when tab loses visibility (prevents leaks on tab switch)
document.addEventListener('visibilitychange', () => {
  if (document.hidden && state.resizing) {
    // Persist the current resize state before stopping
    const img = state.resizing;
    if (img) persistResizeDuringDrag(img);
    stopResize();
  }
});

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
    // Position off-screen first to measure, then clamp to viewport
    menu.style.left = '-9999px';
    menu.style.top = '-9999px';
    menu.style.display = 'block';
    const menuRect = menu.getBoundingClientRect();
    const maxX = window.innerWidth - menuRect.width;
    const maxY = window.innerHeight - menuRect.height;
    menu.style.left = Math.min(Math.max(0, e.clientX), maxX) + 'px';
    menu.style.top = Math.min(Math.max(0, e.clientY), maxY) + 'px';
  });

  // Close image context menu on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('#imageContextMenu')) {
      document.getElementById('imageContextMenu').style.display = 'none';
    }
  });

  // Image alignment helper — sets alignment on the PARENT PARAGRAPH, not the image
  function alignImage(alignment) {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    try {
      // Find the parent paragraph of this image
      const imgEl = document.querySelector(`[data-node-id="${state._ctxImageNodeId}"]`);
      const paraEl = imgEl?.closest('[data-node-id]')?.parentElement?.closest('[data-node-id]');
      const paraNodeId = paraEl?.dataset?.nodeId || state._ctxImageNodeId;

      // Try to get parent via WASM if available
      let targetId = paraNodeId;
      try {
        if (typeof state.doc.get_parent_id === 'function') {
          targetId = state.doc.get_parent_id(state._ctxImageNodeId) || paraNodeId;
        }
      } catch (_) {}

      state.doc.set_alignment(targetId, alignment);
      broadcastOp({ action: 'setAlignment', nodeId: targetId, alignment });
      renderDocument();
      updateUndoRedo();
    } catch (e) { console.error('image align:', e); }
  }

  document.getElementById('imAlignLeft').addEventListener('click', () => alignImage('left'));
  document.getElementById('imAlignCenter').addEventListener('click', () => alignImage('center'));
  document.getElementById('imAlignRight').addEventListener('click', () => alignImage('right'));

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
      // J5: Sanitize alt text — strip HTML tags before passing to WASM
      const sanitized = alt.replace(/<[^>]*>/g, '');
      state.doc.set_image_alt_text(state._ctxImageNodeId, sanitized);
      broadcastOp({ action: 'setImageAltText', nodeId: state._ctxImageNodeId, alt: sanitized });
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

  // Replace image from context menu
  document.getElementById('imReplace').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    // Reuse the hidden file input to pick a new image
    const input = document.getElementById('imageInput');
    const nodeId = state._ctxImageNodeId;
    const onReplace = (e) => {
      input.removeEventListener('change', onReplace);
      const file = e.target.files[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        const bytes = new Uint8Array(reader.result);
        const type = file.type || 'image/png';
        const img = new Image();
        const url = URL.createObjectURL(file);
        img.onload = () => {
          const pxToPt = 72 / 96;
          let w = img.naturalWidth * pxToPt, h = img.naturalHeight * pxToPt;
          if (w > 468) { h *= 468 / w; w = 468; }
          try {
            state.doc.replace_image(nodeId, bytes, type, w, h);
            broadcastOp({ action: 'replaceImage', nodeId });
            deselectImage();
            renderDocument();
            updateUndoRedo();
          } catch (err) {
            // Fallback: delete old, insert new after previous sibling
            console.warn('replace_image not available, using delete+insert:', err);
            try {
              state.doc.delete_image(nodeId);
              state.doc.insert_image(nodeId, bytes, type, w, h);
              deselectImage();
              renderDocument();
              updateUndoRedo();
            } catch (e2) { console.error('replace image fallback:', e2); }
          }
          URL.revokeObjectURL(url);
        };
        img.onerror = () => {
          URL.revokeObjectURL(url);
          console.error('Failed to load replacement image');
        };
        img.src = url;
      };
      reader.readAsArrayBuffer(file);
      input.value = '';
    };
    input.addEventListener('change', onReplace);
    input.click();
  });

  // FS-10: Crop image from context menu
  document.getElementById('imCrop').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state.selectedImg) return;
    startCrop(state.selectedImg);
  });

  // FS-20: Caption image from context menu
  document.getElementById('imCaption').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    if (!state._ctxImageNodeId || !state.doc) return;
    // Pre-fill current caption from data attribute
    const img = state.selectedImg;
    const currentCaption = img ? (img.dataset.caption || '') : '';
    const input = document.getElementById('captionInput');
    input.value = currentCaption;
    document.getElementById('captionModal').classList.add('show');
    input.focus();
  });

  // FS-20: Caption modal handlers
  document.getElementById('captionCancelBtn').addEventListener('click', () => {
    document.getElementById('captionModal').classList.remove('show');
  });
  document.getElementById('captionSaveBtn').addEventListener('click', () => {
    const captionText = document.getElementById('captionInput').value.trim();
    document.getElementById('captionModal').classList.remove('show');
    if (!state._ctxImageNodeId || !state.doc) return;
    _applyCaptionToImage(state._ctxImageNodeId, captionText);
  });
  document.getElementById('captionModal').addEventListener('click', e => {
    if (e.target.id === 'captionModal') document.getElementById('captionModal').classList.remove('show');
  });
  document.getElementById('captionInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); document.getElementById('captionSaveBtn').click(); }
    if (e.key === 'Escape') document.getElementById('captionModal').classList.remove('show');
  });

  // ─── FS-42: Image Transparency & Filter Controls ───────────────────
  _initImageFilterControls();

  // Delete image from context menu
  document.getElementById('imDelete').addEventListener('click', () => {
    document.getElementById('imageContextMenu').style.display = 'none';
    deleteSelectedImage();
  });

  // UXP-21: Image text wrapping submenu
  const wrapSubmenu = document.getElementById('imWrapSubmenu');
  if (wrapSubmenu) {
    wrapSubmenu.querySelectorAll('[data-wrap]').forEach(btn => {
      btn.addEventListener('click', () => {
        document.getElementById('imageContextMenu').style.display = 'none';
        if (!state._ctxImageNodeId || !state.doc) return;
        const mode = btn.dataset.wrap;
        try {
          // Find the actual image node (not the paragraph).
          // _ctxImageNodeId is the containing paragraph; walk to find Image child.
          const imgNodeId = findImageNodeId(state._ctxImageNodeId);
          if (!imgNodeId) {
            console.warn('Could not find image node for wrap mode');
            return;
          }
          state.doc.set_image_wrap_mode(imgNodeId, mode);
          broadcastOp({ action: 'setImageWrapMode', nodeId: imgNodeId, mode });
          renderDocument();
          updateUndoRedo();
        } catch (e) { console.error('set wrap mode:', e); }
      });
    });

    // Highlight current wrap mode when opening submenu
    const wrapWrapper = wrapSubmenu.closest('.ctx-submenu-wrapper');
    if (wrapWrapper) {
      wrapWrapper.addEventListener('mouseenter', () => {
        if (!state._ctxImageNodeId || !state.doc) return;
        const imgNodeId = findImageNodeId(state._ctxImageNodeId);
        let currentMode = 'inline';
        if (imgNodeId) {
          try { currentMode = state.doc.get_image_wrap_mode(imgNodeId) || 'inline'; } catch (_) {}
        }
        wrapSubmenu.querySelectorAll('[data-wrap]').forEach(b => {
          b.classList.toggle('ctx-item-active', b.dataset.wrap === currentMode);
        });
      });
    }
  }
}

// ─── FS-10: Image Crop Tool ───────────────────────
let _cropState = null;

function startCrop(img) {
  cancelCrop();
  const wrap = img.closest('.img-wrap');
  if (!wrap) return;

  const overlay = document.createElement('div');
  overlay.className = 'crop-overlay';
  overlay.contentEditable = 'false';
  overlay.style.pointerEvents = 'all';
  wrap.appendChild(overlay);

  const cropBox = document.createElement('div');
  cropBox.className = 'crop-box';
  wrap.appendChild(cropBox);

  // 4 corners + 4 edges
  const positions = ['tl', 'tr', 'bl', 'br', 'tm', 'bm', 'ml', 'mr'];
  positions.forEach(pos => {
    const h = document.createElement('span');
    h.className = 'crop-handle ' + pos;
    h.dataset.cropHandle = pos;
    cropBox.appendChild(h);
  });

  const imgW = img.offsetWidth;
  const imgH = img.offsetHeight;

  // Start with full image selected (no crop)
  _cropState = {
    img, wrap, overlay, cropBox,
    top: 0, left: 0, right: 0, bottom: 0, // percentages
    imgW, imgH, dragging: null, startX: 0, startY: 0,
    startCrop: null,
  };

  _updateCropBox();

  // Attach events
  cropBox.addEventListener('mousedown', _onCropHandleDown);
  cropBox.addEventListener('touchstart', _onCropHandleDown, { passive: false });
  document.addEventListener('keydown', _onCropKeydown);
  wrap.addEventListener('dblclick', _onCropConfirm);
}

function _updateCropBox() {
  if (!_cropState) return;
  const { cropBox, overlay, imgW, imgH, top, left, right, bottom } = _cropState;
  const x = left / 100 * imgW;
  const y = top / 100 * imgH;
  const w = imgW - (left / 100 * imgW) - (right / 100 * imgW);
  const h = imgH - (top / 100 * imgH) - (bottom / 100 * imgH);
  cropBox.style.left = x + 'px';
  cropBox.style.top = y + 'px';
  cropBox.style.width = Math.max(10, w) + 'px';
  cropBox.style.height = Math.max(10, h) + 'px';
  overlay.style.clipPath = `polygon(
    0% 0%, 100% 0%, 100% 100%, 0% 100%, 0% 0%,
    ${left}% ${top}%, ${left}% ${100 - bottom}%, ${100 - right}% ${100 - bottom}%, ${100 - right}% ${top}%, ${left}% ${top}%
  )`;
}

function _onCropHandleDown(e) {
  const handle = e.target.dataset?.cropHandle;
  if (!handle || !_cropState) return;
  e.preventDefault();
  e.stopPropagation();
  const zoom = (state.zoomLevel || 100) / 100;
  const clientX = e.touches ? e.touches[0].clientX : e.clientX;
  const clientY = e.touches ? e.touches[0].clientY : e.clientY;
  _cropState.dragging = handle;
  _cropState.startX = clientX / zoom;
  _cropState.startY = clientY / zoom;
  _cropState.startCrop = {
    top: _cropState.top, left: _cropState.left,
    right: _cropState.right, bottom: _cropState.bottom,
  };
  document.addEventListener('mousemove', _onCropHandleMove);
  document.addEventListener('mouseup', _onCropHandleUp);
  document.addEventListener('touchmove', _onCropHandleMove, { passive: false });
  document.addEventListener('touchend', _onCropHandleUp);
}

function _onCropHandleMove(e) {
  if (!_cropState || !_cropState.dragging) return;
  if (e.cancelable) e.preventDefault();
  const zoom = (state.zoomLevel || 100) / 100;
  const clientX = e.touches ? e.touches[0].clientX : e.clientX;
  const clientY = e.touches ? e.touches[0].clientY : e.clientY;
  const dx = (clientX / zoom) - _cropState.startX;
  const dy = (clientY / zoom) - _cropState.startY;
  const { imgW, imgH, startCrop, dragging } = _cropState;
  const dxPct = (dx / imgW) * 100;
  const dyPct = (dy / imgH) * 100;

  let { top, left, right, bottom } = startCrop;

  if (dragging.includes('t')) top = Math.max(0, Math.min(100 - bottom - 5, startCrop.top + dyPct));
  if (dragging.includes('b')) bottom = Math.max(0, Math.min(100 - top - 5, startCrop.bottom - dyPct));
  if (dragging.includes('l')) left = Math.max(0, Math.min(100 - right - 5, startCrop.left + dxPct));
  if (dragging.includes('r')) right = Math.max(0, Math.min(100 - left - 5, startCrop.right - dxPct));

  // Edge-only handles
  if (dragging === 'tm') { left = startCrop.left; right = startCrop.right; }
  if (dragging === 'bm') { left = startCrop.left; right = startCrop.right; }
  if (dragging === 'ml') { top = startCrop.top; bottom = startCrop.bottom; }
  if (dragging === 'mr') { top = startCrop.top; bottom = startCrop.bottom; }

  _cropState.top = top;
  _cropState.left = left;
  _cropState.right = right;
  _cropState.bottom = bottom;
  _updateCropBox();
}

function _onCropHandleUp() {
  if (_cropState) _cropState.dragging = null;
  document.removeEventListener('mousemove', _onCropHandleMove);
  document.removeEventListener('mouseup', _onCropHandleUp);
  document.removeEventListener('touchmove', _onCropHandleMove);
  document.removeEventListener('touchend', _onCropHandleUp);
}

function _onCropKeydown(e) {
  if (!_cropState) return;
  if (e.key === 'Enter') {
    e.preventDefault();
    _applyCrop();
  } else if (e.key === 'Escape') {
    e.preventDefault();
    cancelCrop();
  }
}

function _onCropConfirm(e) {
  if (!_cropState) return;
  e.preventDefault();
  e.stopPropagation();
  _applyCrop();
}

function _applyCrop() {
  if (!_cropState) return;
  const { img, top, left, right, bottom } = _cropState;
  // Apply clip-path inset to visually crop
  img.style.clipPath = `inset(${top.toFixed(1)}% ${right.toFixed(1)}% ${bottom.toFixed(1)}% ${left.toFixed(1)}%)`;
  // Store crop data as data attributes
  img.dataset.cropTop = top.toFixed(1);
  img.dataset.cropRight = right.toFixed(1);
  img.dataset.cropBottom = bottom.toFixed(1);
  img.dataset.cropLeft = left.toFixed(1);

  // Persist crop data to WASM if possible
  const nodeEl = img.closest('[data-node-id]');
  if (nodeEl && state.doc) {
    const nodeId = nodeEl.dataset.nodeId;
    try {
      if (typeof state.doc.set_image_crop === 'function') {
        state.doc.set_image_crop(nodeId, top, bottom, left, right);
        broadcastOp({ action: 'setImageCrop', nodeId, top, bottom, left, right });
      }
    } catch (_) {}
    recordUndoAction('Crop image');
    updateUndoRedo();
  }
  cancelCrop();
}

function cancelCrop() {
  if (!_cropState) return;
  const { overlay, cropBox } = _cropState;
  overlay?.remove();
  cropBox?.remove();
  document.removeEventListener('keydown', _onCropKeydown);
  document.removeEventListener('mousemove', _onCropHandleMove);
  document.removeEventListener('mouseup', _onCropHandleUp);
  document.removeEventListener('touchmove', _onCropHandleMove);
  document.removeEventListener('touchend', _onCropHandleUp);
  _cropState = null;
}

/**
 * FS-20: Apply a caption to an image.
 * Inserts a caption paragraph after the image node via WASM if possible,
 * otherwise adds a data-caption attribute and renders a figcaption element.
 */
function _applyCaptionToImage(imageNodeId, captionText) {
  try {
    // Try to insert a caption paragraph via WASM
    if (typeof state.doc.insert_image_caption === 'function') {
      state.doc.insert_image_caption(imageNodeId, captionText);
      broadcastOp({ action: 'insertImageCaption', nodeId: imageNodeId, caption: captionText });
    } else {
      // Fallback: insert a new paragraph after the image node with the caption text
      try {
        const newParaId = state.doc.insert_paragraph_after(imageNodeId, captionText);
        if (newParaId) {
          // Style it as a caption — center-aligned, small italic
          try { state.doc.set_alignment(newParaId, 'center'); } catch (_) {}
          try { state.doc.format_selection(newParaId, 0, newParaId, Array.from(captionText).length, 'italic', 'true'); } catch (_) {}
          try { state.doc.format_selection(newParaId, 0, newParaId, Array.from(captionText).length, 'fontSize', '10'); } catch (_) {}
          broadcastOp({ action: 'insertCaptionParagraph', afterNodeId: imageNodeId, text: captionText });
        }
      } catch (_) {
        // Last resort: store caption as data attribute on the image element
        if (state.selectedImg) {
          state.selectedImg.dataset.caption = captionText;
          // Add or update visual figcaption
          const imgContainer = state.selectedImg.closest('[data-node-id]');
          if (imgContainer) {
            let figcap = imgContainer.querySelector('.img-caption');
            if (captionText) {
              if (!figcap) {
                figcap = document.createElement('div');
                figcap.className = 'img-caption';
                figcap.contentEditable = 'false';
                imgContainer.appendChild(figcap);
              }
              figcap.textContent = captionText;
            } else if (figcap) {
              figcap.remove();
            }
          }
        }
      }
    }
    renderDocument();
    recordUndoAction('Set image caption');
    updateUndoRedo();
  } catch (err) {
    console.error('set image caption:', err);
  }
}

// ─── FS-42: Image Transparency & Filter Controls ────────────────────────

/**
 * Initializes the image filter controls section in the context menu.
 * Adds opacity slider, filter sliders, and filter presets.
 */
function _initImageFilterControls() {
  const menu = document.getElementById('imageContextMenu');
  if (!menu) return;

  // Check if already initialized
  if (menu.querySelector('.img-filter-controls')) return;

  // Create the filter controls section
  const filterSection = document.createElement('div');
  filterSection.className = 'img-filter-controls';

  // Opacity slider
  const opacityRow = _createFilterRow('Opacity', 'imgOpacitySlider', 0, 100, 100, '%');
  filterSection.appendChild(opacityRow);

  // Brightness slider
  const brightnessRow = _createFilterRow('Brightness', 'imgBrightnessSlider', 0, 200, 100, '%');
  filterSection.appendChild(brightnessRow);

  // Blur slider
  const blurRow = _createFilterRow('Blur', 'imgBlurSlider', 0, 10, 0, 'px');
  filterSection.appendChild(blurRow);

  // Insert before the delete button separator
  const deleteSep = menu.querySelector('#imDelete')?.previousElementSibling;
  if (deleteSep) {
    menu.insertBefore(filterSection, deleteSep);
  } else {
    menu.appendChild(filterSection);
  }

  // Presets row
  const presetRow = document.createElement('div');
  presetRow.className = 'img-filter-preset-row';
  const presets = [
    { label: 'Original', filter: 'none', opacity: 100 },
    { label: 'Grayscale', filter: 'grayscale(100%)', opacity: 100 },
    { label: 'Sepia', filter: 'sepia(80%)', opacity: 100 },
    { label: 'Brighten', filter: 'brightness(130%)', opacity: 100 },
    { label: 'Darken', filter: 'brightness(70%)', opacity: 100 },
  ];
  for (const preset of presets) {
    const btn = document.createElement('button');
    btn.className = 'img-filter-preset';
    btn.textContent = preset.label;
    btn.title = `Apply "${preset.label}" filter preset`;
    btn.dataset.preset = preset.label;
    btn.addEventListener('click', () => {
      if (!state.selectedImg) return;
      _applyImageFilter(state.selectedImg, preset.filter, preset.opacity);
      _syncFilterSlidersToImage(state.selectedImg);
      // Highlight active preset
      presetRow.querySelectorAll('.img-filter-preset').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
    });
    presetRow.appendChild(btn);
  }
  if (deleteSep) {
    menu.insertBefore(presetRow, deleteSep);
  } else {
    menu.appendChild(presetRow);
  }

  // Wire up slider events
  const opacitySlider = document.getElementById('imgOpacitySlider');
  const brightnessSlider = document.getElementById('imgBrightnessSlider');
  const blurSlider = document.getElementById('imgBlurSlider');

  const updateFilter = () => {
    if (!state.selectedImg) return;
    const opacity = parseInt(opacitySlider.value);
    const brightness = parseInt(brightnessSlider.value);
    const blur = parseInt(blurSlider.value);
    const filterParts = [];
    if (brightness !== 100) filterParts.push(`brightness(${brightness}%)`);
    if (blur > 0) filterParts.push(`blur(${blur}px)`);
    const filterStr = filterParts.length > 0 ? filterParts.join(' ') : 'none';
    _applyImageFilter(state.selectedImg, filterStr, opacity);
    // Clear active preset since user is manually adjusting
    presetRow.querySelectorAll('.img-filter-preset').forEach(b => b.classList.remove('active'));
  };

  opacitySlider.addEventListener('input', () => {
    opacitySlider.nextElementSibling.textContent = opacitySlider.value + '%';
    updateFilter();
  });
  brightnessSlider.addEventListener('input', () => {
    brightnessSlider.nextElementSibling.textContent = brightnessSlider.value + '%';
    updateFilter();
  });
  blurSlider.addEventListener('input', () => {
    blurSlider.nextElementSibling.textContent = blurSlider.value + 'px';
    updateFilter();
  });

  // Sync sliders when context menu opens for an image
  const menuEl = document.getElementById('imageContextMenu');
  const observer = new MutationObserver(() => {
    if (menuEl.style.display !== 'none' && state.selectedImg) {
      _syncFilterSlidersToImage(state.selectedImg);
    }
  });
  observer.observe(menuEl, { attributes: true, attributeFilter: ['style'] });
}

/**
 * Create a slider row for the filter controls section.
 */
function _createFilterRow(label, id, min, max, defaultVal, unit) {
  const row = document.createElement('div');
  row.className = 'img-filter-row';
  const lbl = document.createElement('label');
  lbl.textContent = label;
  lbl.setAttribute('for', id);
  const slider = document.createElement('input');
  slider.type = 'range';
  slider.id = id;
  slider.min = min;
  slider.max = max;
  slider.value = defaultVal;
  slider.title = `Adjust image ${label.toLowerCase()}`;
  const val = document.createElement('span');
  val.className = 'filter-value';
  val.textContent = defaultVal + unit;
  row.appendChild(lbl);
  row.appendChild(slider);
  row.appendChild(val);
  return row;
}

/**
 * Apply CSS filter and opacity to an image element, and store as data attributes.
 */
function _applyImageFilter(img, filterStr, opacity) {
  // Apply CSS properties
  img.style.opacity = (opacity / 100).toFixed(2);
  img.style.filter = filterStr === 'none' ? '' : filterStr;

  // Store as data attributes for persistence
  img.dataset.filterOpacity = opacity;
  img.dataset.filter = filterStr;

  // Persist to WASM model if available
  const nodeEl = img.closest('[data-node-id]');
  if (nodeEl && state.doc) {
    const nodeId = nodeEl.dataset.nodeId;
    try {
      if (typeof state.doc.set_image_attribute === 'function') {
        state.doc.set_image_attribute(nodeId, 'filter', filterStr);
        state.doc.set_image_attribute(nodeId, 'opacity', String(opacity));
      }
    } catch (_) {}
    broadcastOp({ action: 'setImageFilter', nodeId, filter: filterStr, opacity });
  }

  recordUndoAction('Adjust image filter');
  updateUndoRedo();
}

/**
 * Sync the filter control sliders to match the currently selected image's properties.
 */
function _syncFilterSlidersToImage(img) {
  const opacitySlider = document.getElementById('imgOpacitySlider');
  const brightnessSlider = document.getElementById('imgBrightnessSlider');
  const blurSlider = document.getElementById('imgBlurSlider');
  if (!opacitySlider || !brightnessSlider || !blurSlider) return;

  // Read from data attributes or CSS
  const opacity = img.dataset.filterOpacity ? parseInt(img.dataset.filterOpacity) : Math.round(parseFloat(img.style.opacity || 1) * 100);
  const filterStr = img.dataset.filter || img.style.filter || 'none';

  opacitySlider.value = opacity;
  opacitySlider.nextElementSibling.textContent = opacity + '%';

  // Parse brightness from filter string
  const brightnessMatch = filterStr.match(/brightness\((\d+)%?\)/);
  const brightness = brightnessMatch ? parseInt(brightnessMatch[1]) : 100;
  brightnessSlider.value = brightness;
  brightnessSlider.nextElementSibling.textContent = brightness + '%';

  // Parse blur from filter string
  const blurMatch = filterStr.match(/blur\((\d+)px?\)/);
  const blur = blurMatch ? parseInt(blurMatch[1]) : 0;
  blurSlider.value = blur;
  blurSlider.nextElementSibling.textContent = blur + 'px';

  // Highlight matching preset
  const presetRow = document.querySelector('.img-filter-preset-row');
  if (presetRow) {
    presetRow.querySelectorAll('.img-filter-preset').forEach(btn => {
      btn.classList.remove('active');
      if (filterStr === 'none' && opacity === 100 && btn.dataset.preset === 'Original') {
        btn.classList.add('active');
      } else if (filterStr === 'grayscale(100%)' && btn.dataset.preset === 'Grayscale') {
        btn.classList.add('active');
      } else if (filterStr === 'sepia(80%)' && btn.dataset.preset === 'Sepia') {
        btn.classList.add('active');
      } else if (filterStr === 'brightness(130%)' && btn.dataset.preset === 'Brighten') {
        btn.classList.add('active');
      } else if (filterStr === 'brightness(70%)' && btn.dataset.preset === 'Darken') {
        btn.classList.add('active');
      }
    });
  }
}

/**
 * Walk from a paragraph-level node ID to find the Image node ID beneath it.
 * Context menu stores the paragraph node ID; we need the image child's ID
 * for attribute operations.
 */
function findImageNodeId(paraNodeIdStr) {
  if (!state.doc) return null;
  // The context menu may have stored either the paragraph or the image itself.
  // Try to get the node type; if it is already Image, return it.
  try {
    const nodeJson = state.doc.node_info_json(paraNodeIdStr);
    if (nodeJson) {
      const info = JSON.parse(nodeJson);
      if (info.type === 'Image') return paraNodeIdStr;
      // Walk children to find the first Image
      if (info.children && info.children.length) {
        for (const childId of info.children) {
          const childJson = state.doc.node_info_json(childId);
          if (childJson) {
            const childInfo = JSON.parse(childJson);
            if (childInfo.type === 'Image') return childId;
            // Check grandchildren (Paragraph -> Run -> Image)
            if (childInfo.children) {
              for (const gcId of childInfo.children) {
                const gcJson = state.doc.node_info_json(gcId);
                if (gcJson) {
                  const gcInfo = JSON.parse(gcJson);
                  if (gcInfo.type === 'Image') return gcId;
                }
              }
            }
          }
        }
      }
    }
  } catch (_) {}
  return null;
}
