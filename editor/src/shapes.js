// E9.4 — Drawing/Shape Tools
// SVG-based shapes: rectangle, oval, line, arrow, text box, callout
// Supports: insert via menu, click-drag drawing, select, move, resize, delete,
//           properties panel, context menu, z-ordering, grouping, persistence.

import { state, $ } from './state.js';
import { markDirty } from './file.js';

// ─── Shape Type Definitions ─────────────────────────────────────
const SHAPE_TYPES = {
  rectangle: { label: 'Rectangle', icon: 'rectangle', cursor: 'crosshair' },
  oval:      { label: 'Oval',      icon: 'circle',    cursor: 'crosshair' },
  line:      { label: 'Line',      icon: 'pen_size_1', cursor: 'crosshair' },
  arrow:     { label: 'Arrow',     icon: 'arrow_right_alt', cursor: 'crosshair' },
  textbox:   { label: 'Text Box',  icon: 'text_fields', cursor: 'crosshair' },
  callout:   { label: 'Callout',   icon: 'chat_bubble', cursor: 'crosshair' },
};

// ─── State ──────────────────────────────────────────────────────
const shapes = new Map();          // shapeId -> shapeData
let selectedShapeIds = new Set();   // multi-select via Ctrl+click
let drawingMode = null;             // null | 'rectangle' | 'oval' | ...
let drawOverlay = null;             // transparent overlay DOM element
let drawStart = null;               // { x, y, pageContent }
let dragState = null;               // { shapeId, startX, startY, origX, origY }
let resizeState = null;             // { shapeId, handle, startX, startY, origBounds }
let nextZIndex = 1;
let shapeIdCounter = 0;
let groupIdCounter = 0;

// ─── Public Init ────────────────────────────────────────────────
export function initShapes() {
  setupInsertMenuEntries();
  setupGlobalListeners();
  setupContextMenu();
  setupPropertiesPanel();
}

// ─── Generate unique shape ID ───────────────────────────────────
function genShapeId() {
  return 's_' + Date.now().toString(36) + '_' + (shapeIdCounter++);
}

// ─── Insert Menu Wiring ─────────────────────────────────────────
function setupInsertMenuEntries() {
  // App menu bar "Drawing..." entry
  const miDrawing = $('miDrawing');
  if (miDrawing) {
    miDrawing.addEventListener('click', () => {
      // Close the Insert menu dropdown
      const insertMenuBar = $('insertMenuBar');
      if (insertMenuBar) {
        const parent = insertMenuBar.closest('.app-menu-item');
        if (parent) parent.classList.remove('open');
      }
      openShapePicker();
    });
  }

  // Toolbar Insert dropdown entry
  const insertMenu = $('insertMenu');
  if (insertMenu) {
    const drawBtn = insertMenu.querySelector('[data-action="drawing"]');
    if (drawBtn) {
      drawBtn.addEventListener('click', () => {
        insertMenu.classList.remove('show');
        $('btnInsertMenu')?.setAttribute('aria-expanded', 'false');
        openShapePicker();
      });
    }
  }

  // Shape picker panel buttons
  const picker = $('shapePicker');
  if (picker) {
    picker.querySelectorAll('[data-shape]').forEach(btn => {
      btn.addEventListener('click', () => {
        const type = btn.dataset.shape;
        closeShapePicker();
        startDrawing(type);
      });
    });
    // Close picker on backdrop click
    picker.addEventListener('click', e => {
      if (e.target === picker) closeShapePicker();
    });
    // Close button
    const closeBtn = picker.querySelector('.shape-picker-close');
    if (closeBtn) closeBtn.addEventListener('click', closeShapePicker);
  }
}

// ─── Shape Picker Modal ─────────────────────────────────────────
function openShapePicker() {
  const picker = $('shapePicker');
  if (picker) picker.classList.add('show');
}

function closeShapePicker() {
  const picker = $('shapePicker');
  if (picker) picker.classList.remove('show');
}

// ─── Drawing Mode ───────────────────────────────────────────────
function startDrawing(shapeType) {
  if (!SHAPE_TYPES[shapeType]) return;
  drawingMode = shapeType;
  document.body.classList.add('drawing-mode');
  // Show instruction toast
  showShapeToast('Click and drag on the page to draw a ' + SHAPE_TYPES[shapeType].label);
}

function stopDrawing() {
  drawingMode = null;
  document.body.classList.remove('drawing-mode');
  if (drawOverlay) {
    drawOverlay.remove();
    drawOverlay = null;
  }
  drawStart = null;
  hideShapeToast();
}

// ─── Toast (brief instruction) ──────────────────────────────────
function showShapeToast(msg) {
  let toast = $('shapeToast');
  if (!toast) return;
  toast.textContent = msg;
  toast.classList.add('show');
}

function hideShapeToast() {
  let toast = $('shapeToast');
  if (toast) toast.classList.remove('show');
}

// ─── Global Listeners ───────────────────────────────────────────
function setupGlobalListeners() {
  const page = $('pageContainer');
  if (!page) return;

  // Mousedown — start drawing on page-content, or select/move shapes
  page.addEventListener('mousedown', onMouseDown);
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);

  // Keyboard
  document.addEventListener('keydown', onKeyDown);

  // Click outside shape to deselect
  document.addEventListener('mousedown', e => {
    if (selectedShapeIds.size === 0) return;
    // If clicking on a shape or shape handle, don't deselect
    if (e.target.closest('.editor-shape') || e.target.closest('.shape-handle') ||
        e.target.closest('#shapeContextMenu') || e.target.closest('#shapePropsPanel') ||
        e.target.closest('#shapePicker')) return;
    deselectAllShapes();
  }, true);
}

function onMouseDown(e) {
  // Drawing mode — create overlay and begin drawing
  if (drawingMode) {
    const pageContent = e.target.closest('.page-content');
    if (!pageContent) return;
    e.preventDefault();
    e.stopPropagation();

    const rect = pageContent.getBoundingClientRect();
    drawStart = {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
      pageContent,
    };

    // Create transparent overlay on this page
    if (drawOverlay) drawOverlay.remove();
    drawOverlay = document.createElement('div');
    drawOverlay.className = 'shape-draw-overlay';
    drawOverlay.style.cssText = `position:absolute;inset:0;z-index:100;cursor:crosshair;`;
    pageContent.style.position = 'relative';
    pageContent.appendChild(drawOverlay);

    // Preview element
    const preview = document.createElement('div');
    preview.className = 'shape-draw-preview';
    preview.style.cssText = `position:absolute;border:2px dashed var(--accent, #1a73e8);pointer-events:none;`;
    preview.style.left = drawStart.x + 'px';
    preview.style.top = drawStart.y + 'px';
    preview.style.width = '0px';
    preview.style.height = '0px';
    drawOverlay.appendChild(preview);
    return;
  }

  // Check if clicking on a resize handle
  const handleEl = e.target.closest('.shape-handle');
  if (handleEl) {
    e.preventDefault();
    e.stopPropagation();
    const shapeEl = handleEl.closest('.editor-shape');
    if (!shapeEl) return;
    const shapeId = shapeEl.dataset.shapeId;
    resizeState = {
      shapeId,
      handle: handleEl.dataset.handle,
      startX: e.clientX,
      startY: e.clientY,
      origBounds: {
        x: parseFloat(shapeEl.style.left),
        y: parseFloat(shapeEl.style.top),
        width: parseFloat(shapeEl.style.width),
        height: parseFloat(shapeEl.style.height),
      },
    };
    return;
  }

  // Check if clicking on a shape
  const shapeEl = e.target.closest('.editor-shape');
  if (shapeEl) {
    e.preventDefault();
    e.stopPropagation();
    const shapeId = shapeEl.dataset.shapeId;

    // Ctrl/Meta+click for multi-select
    if (e.ctrlKey || e.metaKey) {
      if (selectedShapeIds.has(shapeId)) {
        selectedShapeIds.delete(shapeId);
        updateShapeSelection(shapeId, false);
      } else {
        selectedShapeIds.add(shapeId);
        updateShapeSelection(shapeId, true);
      }
    } else {
      if (!selectedShapeIds.has(shapeId)) {
        deselectAllShapes();
        selectedShapeIds.add(shapeId);
        updateShapeSelection(shapeId, true);
      }
      // Start drag
      dragState = {
        shapeId,
        startX: e.clientX,
        startY: e.clientY,
        origX: parseFloat(shapeEl.style.left),
        origY: parseFloat(shapeEl.style.top),
      };
    }
    showPropsPanel();
    return;
  }
}

function onMouseMove(e) {
  // Drawing preview
  if (drawingMode && drawStart && drawOverlay) {
    const rect = drawStart.pageContent.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const preview = drawOverlay.querySelector('.shape-draw-preview');
    if (preview) {
      const left = Math.min(drawStart.x, x);
      const top = Math.min(drawStart.y, y);
      const w = Math.abs(x - drawStart.x);
      const h = Math.abs(y - drawStart.y);
      preview.style.left = left + 'px';
      preview.style.top = top + 'px';
      preview.style.width = w + 'px';
      preview.style.height = h + 'px';
    }
    return;
  }

  // Dragging
  if (dragState) {
    const dx = e.clientX - dragState.startX;
    const dy = e.clientY - dragState.startY;
    // Move all selected shapes
    for (const sid of selectedShapeIds) {
      const data = shapes.get(sid);
      if (!data) continue;
      const el = data._el;
      if (!el) continue;
      if (sid === dragState.shapeId) {
        el.style.left = (dragState.origX + dx) + 'px';
        el.style.top = (dragState.origY + dy) + 'px';
      }
    }
    return;
  }

  // Resizing
  if (resizeState) {
    const dx = e.clientX - resizeState.startX;
    const dy = e.clientY - resizeState.startY;
    const ob = resizeState.origBounds;
    const data = shapes.get(resizeState.shapeId);
    if (!data) return;
    const el = data._el;
    if (!el) return;

    let newX = ob.x, newY = ob.y, newW = ob.width, newH = ob.height;
    const handle = resizeState.handle;

    if (handle.includes('r')) { newW = Math.max(20, ob.width + dx); }
    if (handle.includes('l')) { newW = Math.max(20, ob.width - dx); newX = ob.x + dx; }
    if (handle.includes('b')) { newH = Math.max(20, ob.height + dy); }
    if (handle.includes('t') && handle !== 'textbox') { newH = Math.max(20, ob.height - dy); newY = ob.y + dy; }

    // Corner handles: preserve aspect ratio for ovals
    if ((handle === 'tl' || handle === 'tr' || handle === 'bl' || handle === 'br') && data.type === 'oval') {
      const ratio = ob.width / ob.height;
      if (Math.abs(dx) > Math.abs(dy)) {
        newH = newW / ratio;
      } else {
        newW = newH * ratio;
      }
    }

    el.style.left = newX + 'px';
    el.style.top = newY + 'px';
    el.style.width = newW + 'px';
    el.style.height = newH + 'px';

    updateSVGInner(el, data.type, newW, newH);
    updateHandles(el);
    return;
  }
}

function onMouseUp(e) {
  // Finish drawing
  if (drawingMode && drawStart && drawOverlay) {
    const rect = drawStart.pageContent.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const left = Math.min(drawStart.x, x);
    const top = Math.min(drawStart.y, y);
    const w = Math.abs(x - drawStart.x);
    const h = Math.abs(y - drawStart.y);

    // Require minimum size (at least 10px in each dimension for rects/ovals, 10px length for lines)
    const type = drawingMode;
    const isLine = (type === 'line' || type === 'arrow');
    const minSize = isLine ? 10 : 20;
    if (Math.max(w, h) >= minSize) {
      const shapeData = {
        type,
        x: left,
        y: top,
        width: isLine ? w : Math.max(w, 30),
        height: isLine ? h : Math.max(h, 30),
        fill: type === 'line' || type === 'arrow' ? 'none' : '#cfe2f3',
        stroke: '#1a73e8',
        strokeWidth: 2,
        opacity: 100,
        text: '',
        zIndex: nextZIndex++,
        lineStartX: isLine ? drawStart.x : undefined,
        lineStartY: isLine ? drawStart.y : undefined,
        lineEndX: isLine ? x : undefined,
        lineEndY: isLine ? y : undefined,
      };
      createShapeElement(shapeData, drawStart.pageContent);
    }
    stopDrawing();
    return;
  }

  // Finish dragging
  if (dragState) {
    const data = shapes.get(dragState.shapeId);
    if (data && data._el) {
      data.x = parseFloat(data._el.style.left);
      data.y = parseFloat(data._el.style.top);
      markDirty();
    }
    dragState = null;
    return;
  }

  // Finish resizing
  if (resizeState) {
    const data = shapes.get(resizeState.shapeId);
    if (data && data._el) {
      data.x = parseFloat(data._el.style.left);
      data.y = parseFloat(data._el.style.top);
      data.width = parseFloat(data._el.style.width);
      data.height = parseFloat(data._el.style.height);
      markDirty();
    }
    resizeState = null;
    return;
  }
}

// ─── Keyboard ───────────────────────────────────────────────────
function onKeyDown(e) {
  // Escape cancels drawing mode
  if (e.key === 'Escape' && drawingMode) {
    e.preventDefault();
    stopDrawing();
    return;
  }

  // Escape deselects shapes
  if (e.key === 'Escape' && selectedShapeIds.size > 0) {
    e.preventDefault();
    deselectAllShapes();
    hidePropsPanel();
    return;
  }

  // Delete/Backspace removes selected shapes
  if ((e.key === 'Delete' || e.key === 'Backspace') && selectedShapeIds.size > 0) {
    // Don't intercept if user is editing a textbox inside a shape
    if (e.target.closest('.shape-textbox-edit')) return;
    e.preventDefault();
    deleteSelectedShapes();
    return;
  }
}

// ─── Shape Creation ─────────────────────────────────────────────
function createShapeElement(data, pageContent) {
  const id = genShapeId();
  data.id = id;
  shapes.set(id, data);

  const wrapper = document.createElement('div');
  wrapper.className = 'editor-shape';
  wrapper.dataset.shapeId = id;
  wrapper.dataset.shapeType = data.type;
  wrapper.setAttribute('tabindex', '-1');
  wrapper.setAttribute('role', 'img');
  wrapper.setAttribute('aria-label', SHAPE_TYPES[data.type]?.label || 'Shape');
  wrapper.style.cssText = `
    position:absolute;
    left:${data.x}px;
    top:${data.y}px;
    width:${data.width}px;
    height:${data.height}px;
    z-index:${data.zIndex};
    cursor:move;
  `;

  // Create SVG inside
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('width', '100%');
  svg.setAttribute('height', '100%');
  svg.setAttribute('viewBox', `0 0 ${data.width} ${data.height}`);
  svg.style.cssText = 'display:block;pointer-events:none;overflow:visible;';

  // Add defs for arrow marker
  if (data.type === 'arrow') {
    const defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
    const marker = document.createElementNS('http://www.w3.org/2000/svg', 'marker');
    marker.setAttribute('id', 'arrowhead-' + id);
    marker.setAttribute('markerWidth', '10');
    marker.setAttribute('markerHeight', '7');
    marker.setAttribute('refX', '10');
    marker.setAttribute('refY', '3.5');
    marker.setAttribute('orient', 'auto');
    const polygon = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
    polygon.setAttribute('points', '0 0, 10 3.5, 0 7');
    polygon.setAttribute('fill', data.stroke);
    polygon.classList.add('arrow-marker-fill');
    marker.appendChild(polygon);
    defs.appendChild(marker);
    svg.appendChild(defs);
  }

  appendSVGInner(svg, data);
  wrapper.appendChild(svg);

  // For textbox type, add editable foreignObject
  if (data.type === 'textbox') {
    const fo = document.createElement('div');
    fo.className = 'shape-textbox-edit';
    fo.contentEditable = 'true';
    fo.spellcheck = false;
    fo.style.cssText = `
      position:absolute;inset:4px;
      font-family:var(--font-ui);font-size:13px;
      color:var(--text-primary);
      outline:none;overflow:auto;
      line-height:1.4;padding:4px;
      cursor:text;
    `;
    fo.setAttribute('title', 'Type text inside this text box');
    fo.addEventListener('input', () => {
      data.text = fo.textContent;
      markDirty();
    });
    // Prevent drag when editing text
    fo.addEventListener('mousedown', e => e.stopPropagation());
    wrapper.appendChild(fo);
  }

  // Ensure page-content has relative positioning
  pageContent.style.position = 'relative';
  pageContent.appendChild(wrapper);

  data._el = wrapper;
  data._pageContent = pageContent;

  // Select the new shape
  deselectAllShapes();
  selectedShapeIds.add(id);
  updateShapeSelection(id, true);
  showPropsPanel();
  markDirty();

  return id;
}

function appendSVGInner(svg, data) {
  const w = data.width;
  const h = data.height;
  const fill = data.fill || 'none';
  const stroke = data.stroke || '#1a73e8';
  const sw = data.strokeWidth || 2;
  const opacity = (data.opacity ?? 100) / 100;

  let el;
  switch (data.type) {
    case 'rectangle':
      el = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      el.setAttribute('x', sw / 2);
      el.setAttribute('y', sw / 2);
      el.setAttribute('width', Math.max(0, w - sw));
      el.setAttribute('height', Math.max(0, h - sw));
      el.setAttribute('rx', '2');
      el.setAttribute('fill', fill);
      el.setAttribute('stroke', stroke);
      el.setAttribute('stroke-width', sw);
      el.setAttribute('opacity', opacity);
      break;

    case 'oval':
      el = document.createElementNS('http://www.w3.org/2000/svg', 'ellipse');
      el.setAttribute('cx', w / 2);
      el.setAttribute('cy', h / 2);
      el.setAttribute('rx', Math.max(0, w / 2 - sw / 2));
      el.setAttribute('ry', Math.max(0, h / 2 - sw / 2));
      el.setAttribute('fill', fill);
      el.setAttribute('stroke', stroke);
      el.setAttribute('stroke-width', sw);
      el.setAttribute('opacity', opacity);
      break;

    case 'line':
      el = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      el.setAttribute('x1', 0);
      el.setAttribute('y1', h);
      el.setAttribute('x2', w);
      el.setAttribute('y2', 0);
      el.setAttribute('stroke', stroke);
      el.setAttribute('stroke-width', sw);
      el.setAttribute('opacity', opacity);
      el.setAttribute('stroke-linecap', 'round');
      break;

    case 'arrow':
      el = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      el.setAttribute('x1', 0);
      el.setAttribute('y1', h);
      el.setAttribute('x2', w);
      el.setAttribute('y2', 0);
      el.setAttribute('stroke', stroke);
      el.setAttribute('stroke-width', sw);
      el.setAttribute('opacity', opacity);
      el.setAttribute('stroke-linecap', 'round');
      el.setAttribute('marker-end', `url(#arrowhead-${data.id})`);
      break;

    case 'textbox':
      el = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      el.setAttribute('x', sw / 2);
      el.setAttribute('y', sw / 2);
      el.setAttribute('width', Math.max(0, w - sw));
      el.setAttribute('height', Math.max(0, h - sw));
      el.setAttribute('rx', '2');
      el.setAttribute('fill', fill === 'none' ? '#ffffff' : fill);
      el.setAttribute('stroke', stroke);
      el.setAttribute('stroke-width', sw);
      el.setAttribute('opacity', opacity);
      break;

    case 'callout': {
      el = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
      // Rounded rectangle with a speech pointer at bottom-left
      const pw = 20; // pointer width
      const ph = 15; // pointer height
      const bodyH = Math.max(20, h - ph);
      const points = [
        `${sw},${sw}`,
        `${w - sw},${sw}`,
        `${w - sw},${bodyH}`,
        `${pw + 20},${bodyH}`,
        `${pw},${h - sw}`,
        `${pw},${bodyH}`,
        `${sw},${bodyH}`,
      ].join(' ');
      el.setAttribute('points', points);
      el.setAttribute('fill', fill);
      el.setAttribute('stroke', stroke);
      el.setAttribute('stroke-width', sw);
      el.setAttribute('opacity', opacity);
      el.setAttribute('stroke-linejoin', 'round');
      break;
    }
  }

  if (el) {
    el.classList.add('shape-inner');
    svg.appendChild(el);
  }
}

function updateSVGInner(wrapper, type, w, h) {
  const svg = wrapper.querySelector('svg');
  if (!svg) return;
  svg.setAttribute('viewBox', `0 0 ${w} ${h}`);

  const inner = svg.querySelector('.shape-inner');
  if (!inner) return;

  const data = shapes.get(wrapper.dataset.shapeId);
  const sw = data?.strokeWidth || 2;

  switch (type) {
    case 'rectangle':
    case 'textbox':
      inner.setAttribute('x', sw / 2);
      inner.setAttribute('y', sw / 2);
      inner.setAttribute('width', Math.max(0, w - sw));
      inner.setAttribute('height', Math.max(0, h - sw));
      break;
    case 'oval':
      inner.setAttribute('cx', w / 2);
      inner.setAttribute('cy', h / 2);
      inner.setAttribute('rx', Math.max(0, w / 2 - sw / 2));
      inner.setAttribute('ry', Math.max(0, h / 2 - sw / 2));
      break;
    case 'line':
    case 'arrow':
      inner.setAttribute('x1', 0);
      inner.setAttribute('y1', h);
      inner.setAttribute('x2', w);
      inner.setAttribute('y2', 0);
      break;
    case 'callout': {
      const pw = 20, ph = 15;
      const bodyH = Math.max(20, h - ph);
      const points = [
        `${sw},${sw}`, `${w - sw},${sw}`, `${w - sw},${bodyH}`,
        `${pw + 20},${bodyH}`, `${pw},${h - sw}`, `${pw},${bodyH}`, `${sw},${bodyH}`,
      ].join(' ');
      inner.setAttribute('points', points);
      break;
    }
  }
}

// ─── Selection Handling ─────────────────────────────────────────
function updateShapeSelection(shapeId, selected) {
  const data = shapes.get(shapeId);
  if (!data || !data._el) return;
  const el = data._el;

  if (selected) {
    el.classList.add('shape-selected');
    addHandles(el);
  } else {
    el.classList.remove('shape-selected');
    removeHandles(el);
  }
}

function deselectAllShapes() {
  for (const sid of selectedShapeIds) {
    updateShapeSelection(sid, false);
  }
  selectedShapeIds.clear();
  hidePropsPanel();
  hideShapeContextMenu();
}

function addHandles(el) {
  removeHandles(el);
  const positions = ['tl', 'tc', 'tr', 'ml', 'mr', 'bl', 'bc', 'br'];
  positions.forEach(pos => {
    const h = document.createElement('div');
    h.className = 'shape-handle ' + pos;
    h.dataset.handle = pos;
    h.setAttribute('title', 'Resize shape');
    el.appendChild(h);
  });
}

function removeHandles(el) {
  el.querySelectorAll('.shape-handle').forEach(h => h.remove());
}

function updateHandles(el) {
  // Handles position themselves via CSS — no JS update needed
}

// ─── Deletion ───────────────────────────────────────────────────
function deleteSelectedShapes() {
  for (const sid of selectedShapeIds) {
    deleteShape(sid);
  }
  selectedShapeIds.clear();
  hidePropsPanel();
  markDirty();
}

function deleteShape(shapeId) {
  const data = shapes.get(shapeId);
  if (data && data._el) {
    data._el.remove();
  }
  shapes.delete(shapeId);
}

export function deleteSelectedShape() {
  if (selectedShapeIds.size > 0) {
    deleteSelectedShapes();
    return true;
  }
  return false;
}

export function hasSelectedShape() {
  return selectedShapeIds.size > 0;
}

// ─── Properties Panel ───────────────────────────────────────────
function setupPropertiesPanel() {
  const panel = $('shapePropsPanel');
  if (!panel) return;

  // Fill color
  const fillInput = panel.querySelector('#shapeFillColor');
  if (fillInput) {
    fillInput.addEventListener('input', () => applyPropToSelected('fill', fillInput.value));
  }

  // Stroke color
  const strokeInput = panel.querySelector('#shapeStrokeColor');
  if (strokeInput) {
    strokeInput.addEventListener('input', () => applyPropToSelected('stroke', strokeInput.value));
  }

  // Stroke width
  const swSelect = panel.querySelector('#shapeStrokeWidth');
  if (swSelect) {
    swSelect.addEventListener('change', () => applyPropToSelected('strokeWidth', parseInt(swSelect.value)));
  }

  // Opacity
  const opRange = panel.querySelector('#shapeOpacity');
  const opLabel = panel.querySelector('#shapeOpacityVal');
  if (opRange) {
    opRange.addEventListener('input', () => {
      if (opLabel) opLabel.textContent = opRange.value + '%';
      applyPropToSelected('opacity', parseInt(opRange.value));
    });
  }

  // Close button
  const closeBtn = panel.querySelector('.shape-props-close');
  if (closeBtn) {
    closeBtn.addEventListener('click', hidePropsPanel);
  }
}

function applyPropToSelected(prop, value) {
  for (const sid of selectedShapeIds) {
    const data = shapes.get(sid);
    if (!data) continue;
    data[prop] = value;
    refreshShapeAppearance(data);
  }
  markDirty();
}

function refreshShapeAppearance(data) {
  if (!data._el) return;
  const svg = data._el.querySelector('svg');
  if (!svg) return;
  const inner = svg.querySelector('.shape-inner');
  if (!inner) return;

  const opacity = (data.opacity ?? 100) / 100;
  inner.setAttribute('opacity', opacity);

  if (data.type !== 'line' && data.type !== 'arrow') {
    inner.setAttribute('fill', data.fill || 'none');
  }
  inner.setAttribute('stroke', data.stroke || '#1a73e8');
  inner.setAttribute('stroke-width', data.strokeWidth || 2);

  // Update arrow marker color
  if (data.type === 'arrow') {
    const markerFill = svg.querySelector('.arrow-marker-fill');
    if (markerFill) markerFill.setAttribute('fill', data.stroke || '#1a73e8');
  }
}

function showPropsPanel() {
  const panel = $('shapePropsPanel');
  if (!panel) return;
  if (selectedShapeIds.size === 0) { hidePropsPanel(); return; }

  // Populate from first selected shape
  const firstId = selectedShapeIds.values().next().value;
  const data = shapes.get(firstId);
  if (!data) return;

  const fillInput = panel.querySelector('#shapeFillColor');
  const strokeInput = panel.querySelector('#shapeStrokeColor');
  const swSelect = panel.querySelector('#shapeStrokeWidth');
  const opRange = panel.querySelector('#shapeOpacity');
  const opLabel = panel.querySelector('#shapeOpacityVal');

  if (fillInput) fillInput.value = data.fill && data.fill !== 'none' ? data.fill : '#cfe2f3';
  if (strokeInput) strokeInput.value = data.stroke || '#1a73e8';
  if (swSelect) swSelect.value = data.strokeWidth || 2;
  if (opRange) opRange.value = data.opacity ?? 100;
  if (opLabel) opLabel.textContent = (data.opacity ?? 100) + '%';

  // Position panel near the shape
  const el = data._el;
  if (el) {
    const rect = el.getBoundingClientRect();
    panel.style.top = Math.max(8, rect.top - 44) + 'px';
    panel.style.left = Math.max(8, rect.left) + 'px';
  }

  panel.classList.add('show');
}

function hidePropsPanel() {
  const panel = $('shapePropsPanel');
  if (panel) panel.classList.remove('show');
}

// ─── Context Menu ───────────────────────────────────────────────
function setupContextMenu() {
  const page = $('pageContainer');
  if (!page) return;

  page.addEventListener('contextmenu', e => {
    const shapeEl = e.target.closest('.editor-shape');
    if (!shapeEl) return;
    e.preventDefault();
    e.stopPropagation();

    const shapeId = shapeEl.dataset.shapeId;
    if (!selectedShapeIds.has(shapeId)) {
      deselectAllShapes();
      selectedShapeIds.add(shapeId);
      updateShapeSelection(shapeId, true);
    }

    showShapeContextMenu(e.clientX, e.clientY);
  });

  // Context menu actions
  const ctx = $('shapeContextMenu');
  if (!ctx) return;

  ctx.querySelector('#scmProperties')?.addEventListener('click', () => {
    hideShapeContextMenu();
    showPropsPanel();
  });

  ctx.querySelector('#scmBringFront')?.addEventListener('click', () => {
    hideShapeContextMenu();
    for (const sid of selectedShapeIds) {
      const data = shapes.get(sid);
      if (data) {
        data.zIndex = nextZIndex++;
        if (data._el) data._el.style.zIndex = data.zIndex;
      }
    }
    markDirty();
  });

  ctx.querySelector('#scmSendBack')?.addEventListener('click', () => {
    hideShapeContextMenu();
    for (const sid of selectedShapeIds) {
      const data = shapes.get(sid);
      if (data) {
        data.zIndex = 0;
        if (data._el) data._el.style.zIndex = 0;
      }
    }
    markDirty();
  });

  ctx.querySelector('#scmDuplicate')?.addEventListener('click', () => {
    hideShapeContextMenu();
    duplicateSelected();
  });

  ctx.querySelector('#scmDelete')?.addEventListener('click', () => {
    hideShapeContextMenu();
    deleteSelectedShapes();
  });

  ctx.querySelector('#scmGroup')?.addEventListener('click', () => {
    hideShapeContextMenu();
    groupSelected();
  });

  ctx.querySelector('#scmUngroup')?.addEventListener('click', () => {
    hideShapeContextMenu();
    ungroupSelected();
  });

  // Close context menu on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('#shapeContextMenu')) {
      hideShapeContextMenu();
    }
  });
}

function showShapeContextMenu(x, y) {
  const ctx = $('shapeContextMenu');
  if (!ctx) return;
  ctx.style.left = x + 'px';
  ctx.style.top = y + 'px';
  ctx.style.display = 'block';

  // Show/hide group/ungroup based on selection
  const groupBtn = ctx.querySelector('#scmGroup');
  const ungroupBtn = ctx.querySelector('#scmUngroup');
  if (groupBtn) groupBtn.style.display = selectedShapeIds.size > 1 ? '' : 'none';
  if (ungroupBtn) {
    // Show ungroup if any selected shape is a group
    let hasGroup = false;
    for (const sid of selectedShapeIds) {
      const data = shapes.get(sid);
      if (data?.groupId) { hasGroup = true; break; }
    }
    ungroupBtn.style.display = hasGroup ? '' : 'none';
  }
}

function hideShapeContextMenu() {
  const ctx = $('shapeContextMenu');
  if (ctx) ctx.style.display = 'none';
}

// ─── Duplicate ──────────────────────────────────────────────────
function duplicateSelected() {
  const newIds = [];
  for (const sid of selectedShapeIds) {
    const data = shapes.get(sid);
    if (!data || !data._pageContent) continue;
    const clone = {
      ...data,
      _el: null,
      _pageContent: data._pageContent,
      x: data.x + 20,
      y: data.y + 20,
      zIndex: nextZIndex++,
      groupId: undefined,
    };
    const newId = createShapeElement(clone, data._pageContent);
    newIds.push(newId);
  }
  // Select the new shapes
  deselectAllShapes();
  for (const nid of newIds) {
    selectedShapeIds.add(nid);
    updateShapeSelection(nid, true);
  }
  markDirty();
}

// ─── Grouping ───────────────────────────────────────────────────
function groupSelected() {
  if (selectedShapeIds.size < 2) return;
  const gid = 'g_' + (groupIdCounter++);
  for (const sid of selectedShapeIds) {
    const data = shapes.get(sid);
    if (data) data.groupId = gid;
  }
  markDirty();
}

function ungroupSelected() {
  for (const sid of selectedShapeIds) {
    const data = shapes.get(sid);
    if (data) data.groupId = undefined;
  }
  markDirty();
}

// ─── Persistence helpers ────────────────────────────────────────
export function getShapesData() {
  const result = [];
  for (const [id, data] of shapes) {
    result.push({
      id, type: data.type,
      x: data.x, y: data.y,
      width: data.width, height: data.height,
      fill: data.fill, stroke: data.stroke,
      strokeWidth: data.strokeWidth,
      opacity: data.opacity,
      text: data.text,
      zIndex: data.zIndex,
      groupId: data.groupId,
    });
  }
  return result;
}

export function loadShapesData(dataArray, pageContent) {
  clearAllShapes();
  if (!Array.isArray(dataArray) || !pageContent) return;
  for (const item of dataArray) {
    const data = { ...item };
    createShapeElement(data, pageContent);
  }
  deselectAllShapes();
}

export function clearAllShapes() {
  for (const [id, data] of shapes) {
    if (data._el) data._el.remove();
  }
  shapes.clear();
  selectedShapeIds.clear();
  nextZIndex = 1;
}
