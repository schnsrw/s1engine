// Page ruler — horizontal inch ruler above the document page
// Reads actual page dimensions/margins from the WASM document model.
// Provides draggable indent handles for left indent, first-line indent, and right indent.
import { state } from './state.js';
import { getSelectionInfo } from './selection.js';
import { renderNodeById, syncParagraphText } from './render.js';
import { broadcastOp } from './collab.js';

const PX_PER_INCH = 96;
const PT_PER_INCH = 72;

// Defaults (US Letter, 1" margins) — used when no document is loaded
const DEFAULT_PAGE_WIDTH_PT = 612; // 8.5 * 72
const DEFAULT_MARGIN_LEFT_PT = 72;
const DEFAULT_MARGIN_RIGHT_PT = 72;

// Track last-built dimensions so we rebuild only when they change
let _lastWidthPt = 0;
let _lastMarginLeftPt = 0;
let _lastMarginRightPt = 0;
let _lastIndentLeftPt = -1;
let _lastIndentRightPt = -1;
let _lastFirstLinePt = -1;

// Current page dimensions (set during render, used by drag handlers)
let _pageDims = { widthPt: DEFAULT_PAGE_WIDTH_PT, marginLeftPt: DEFAULT_MARGIN_LEFT_PT, marginRightPt: DEFAULT_MARGIN_RIGHT_PT };

// Drag state
let _dragging = null; // { type: 'left'|'right'|'firstLine', startX, startPt, nodeId }

// UXP-15: Tab stops state
// Each tab stop: { positionPt: number, type: 'left'|'center'|'right'|'decimal' }
let _tabStops = [];
let _draggingTab = null; // { index, startX, startPt }
const TAB_TYPES = ['left', 'center', 'right', 'decimal'];

/**
 * Read page dimensions from the current WASM document's first section.
 */
function getPageDimensions() {
  let widthPt = DEFAULT_PAGE_WIDTH_PT;
  let marginLeftPt = DEFAULT_MARGIN_LEFT_PT;
  let marginRightPt = DEFAULT_MARGIN_RIGHT_PT;

  if (state.doc) {
    try {
      const json = state.doc.get_sections_json();
      const sections = JSON.parse(json);
      if (sections.length > 0) {
        const sec = sections[0];
        widthPt = sec.pageWidth || widthPt;
        marginLeftPt = sec.marginLeft || marginLeftPt;
        marginRightPt = sec.marginRight || marginRightPt;
      }
    } catch (_) { /* defaults */ }
  }

  return { widthPt, marginLeftPt, marginRightPt };
}

/**
 * Read paragraph indent values from the active paragraph.
 * Uses current selection or falls back to lastSelInfo.
 */
function getActiveIndents(overrideNodeId) {
  let indentLeftPt = 0;
  let indentRightPt = 0;
  let firstLinePt = 0;

  if (state.doc) {
    try {
      const nodeId = overrideNodeId || getSelectionInfo()?.startNodeId || state.lastSelInfo?.startNodeId;
      if (nodeId) {
        const fmtJson = state.doc.get_formatting_json(nodeId);
        const fmt = JSON.parse(fmtJson);
        indentLeftPt = parseFloat(fmt.indentLeft || '0');
        indentRightPt = parseFloat(fmt.indentRight || '0');
        firstLinePt = parseFloat(fmt.indentFirstLine || '0');
      }
    } catch (_) { /* defaults */ }
  }

  return { indentLeftPt, indentRightPt, firstLinePt };
}

function ptToPx(pt) {
  return (pt / PT_PER_INCH) * PX_PER_INCH;
}

function pxToPt(px) {
  return (px / PX_PER_INCH) * PT_PER_INCH;
}

function formatInches(pt) {
  const inches = pt / PT_PER_INCH;
  return inches % 1 === 0 ? `${inches} in` : `${inches.toFixed(2)} in`;
}

function clamp(val, min, max) {
  return Math.max(min, Math.min(max, val));
}

/**
 * Apply indent value to a specific paragraph via WASM.
 * Uses nodeId directly rather than querying selection (which may be lost after drag).
 */
function applyIndent(type, valuePt, nodeId) {
  if (!state.doc || !nodeId) return;
  try {
    // Sync text before modifying attributes
    const container = document.getElementById('pageContainer');
    const el = container && container.querySelector(`[data-node-id="${nodeId}"]`);
    if (el) syncParagraphText(el);

    if (type === 'left') {
      state.doc.set_indent(nodeId, 'left', valuePt);
      broadcastOp({ action: 'setIndent', nodeId, side: 'left', value: valuePt });
    } else if (type === 'right') {
      state.doc.set_indent(nodeId, 'right', valuePt);
      broadcastOp({ action: 'setIndent', nodeId, side: 'right', value: valuePt });
    } else if (type === 'firstLine') {
      state.doc.set_indent(nodeId, 'firstLine', valuePt);
      broadcastOp({ action: 'setIndent', nodeId, side: 'firstLine', value: valuePt });
    }

    renderNodeById(nodeId);
  } catch (e) {
    console.error('ruler indent:', e);
  }
}

/**
 * Update indent handle positions without rebuilding the whole ruler.
 */
function updateIndentHandles() {
  const ruler = document.getElementById('ruler');
  if (!ruler) return;

  const { indentLeftPt, indentRightPt, firstLinePt } = getActiveIndents();

  // Skip update if nothing changed
  if (indentLeftPt === _lastIndentLeftPt && indentRightPt === _lastIndentRightPt && firstLinePt === _lastFirstLinePt) {
    return;
  }
  _lastIndentLeftPt = indentLeftPt;
  _lastIndentRightPt = indentRightPt;
  _lastFirstLinePt = firstLinePt;

  const marginLeftPx = ptToPx(_pageDims.marginLeftPt);
  const totalPx = ptToPx(_pageDims.widthPt);
  const marginRightPx = ptToPx(_pageDims.marginRightPt);
  const contentRight = totalPx - marginRightPx;

  // Left indent handle
  const leftHandle = ruler.querySelector('.ruler-handle-left');
  if (leftHandle) {
    const x = marginLeftPx + ptToPx(indentLeftPt);
    leftHandle.style.left = x + 'px';
    leftHandle.title = `Left indent (${formatInches(indentLeftPt)})`;
  }

  // First-line indent handle
  const firstLineHandle = ruler.querySelector('.ruler-handle-firstline');
  if (firstLineHandle) {
    const x = marginLeftPx + ptToPx(indentLeftPt + firstLinePt);
    firstLineHandle.style.left = x + 'px';
    firstLineHandle.title = `First line indent (${formatInches(firstLinePt)})`;
  }

  // Right indent handle
  const rightHandle = ruler.querySelector('.ruler-handle-right');
  if (rightHandle) {
    const x = contentRight - ptToPx(indentRightPt);
    rightHandle.style.left = x + 'px';
    rightHandle.title = `Right indent (${formatInches(indentRightPt)})`;
  }
}

/**
 * Start dragging an indent handle.
 * IMPORTANT: We save the active paragraph's nodeId NOW, because the
 * DOM selection may be lost during the drag (focus leaves docPage).
 */
function startDrag(type, e) {
  e.preventDefault();
  e.stopPropagation();

  // Capture the active paragraph BEFORE focus might shift
  const info = getSelectionInfo();
  const nodeId = info?.startNodeId || state.lastSelInfo?.startNodeId || null;
  if (!nodeId) return; // No paragraph selected — nothing to indent

  const { indentLeftPt, indentRightPt, firstLinePt } = getActiveIndents();

  let startPt;
  if (type === 'left') startPt = indentLeftPt;
  else if (type === 'right') startPt = indentRightPt;
  else if (type === 'firstLine') startPt = firstLinePt;

  // Save current selection so we can restore after drag
  const savedSel = window.getSelection();
  const savedRange = savedSel && savedSel.rangeCount > 0 ? savedSel.getRangeAt(0).cloneRange() : null;

  _dragging = { type, startX: e.clientX, startPt, nodeId, savedRange };

  document.addEventListener('mousemove', onDrag);
  document.addEventListener('mouseup', endDrag);
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function onDrag(e) {
  if (!_dragging) return;

  const zoom = (state.zoomLevel || 100) / 100;
  const deltaPx = (e.clientX - _dragging.startX) / zoom;
  const deltaPt = pxToPt(deltaPx);

  const contentWidthPt = _pageDims.widthPt - _pageDims.marginLeftPt - _pageDims.marginRightPt;

  let newPt;
  if (_dragging.type === 'left') {
    newPt = clamp(_dragging.startPt + deltaPt, 0, contentWidthPt - 36); // min 0.5in remaining
  } else if (_dragging.type === 'right') {
    // Right indent: dragging left increases, dragging right decreases
    newPt = clamp(_dragging.startPt - deltaPt, 0, contentWidthPt - 36);
  } else if (_dragging.type === 'firstLine') {
    newPt = clamp(_dragging.startPt + deltaPt, -144, contentWidthPt); // allow negative (hanging indent)
  }

  // Snap to quarter-inch increments (18pt)
  newPt = Math.round(newPt / 18) * 18;

  // Update handle position visually (live preview)
  const ruler = document.getElementById('ruler');
  if (!ruler) return;

  const marginLeftPx = ptToPx(_pageDims.marginLeftPt);
  const contentRight = ptToPx(_pageDims.widthPt) - ptToPx(_pageDims.marginRightPt);

  if (_dragging.type === 'left') {
    const handle = ruler.querySelector('.ruler-handle-left');
    if (handle) handle.style.left = (marginLeftPx + ptToPx(newPt)) + 'px';
    // Also move first-line handle with left indent
    const flHandle = ruler.querySelector('.ruler-handle-firstline');
    if (flHandle) {
      const { firstLinePt } = getActiveIndents(_dragging.nodeId);
      flHandle.style.left = (marginLeftPx + ptToPx(newPt + firstLinePt)) + 'px';
    }
  } else if (_dragging.type === 'right') {
    const handle = ruler.querySelector('.ruler-handle-right');
    if (handle) handle.style.left = (contentRight - ptToPx(newPt)) + 'px';
  } else if (_dragging.type === 'firstLine') {
    const handle = ruler.querySelector('.ruler-handle-firstline');
    const { indentLeftPt } = getActiveIndents(_dragging.nodeId);
    if (handle) handle.style.left = (marginLeftPx + ptToPx(indentLeftPt + newPt)) + 'px';
  }
}

function endDrag(e) {
  if (!_dragging) return;

  const zoom = (state.zoomLevel || 100) / 100;
  const deltaPx = (e.clientX - _dragging.startX) / zoom;
  const deltaPt = pxToPt(deltaPx);

  const contentWidthPt = _pageDims.widthPt - _pageDims.marginLeftPt - _pageDims.marginRightPt;

  let newPt;
  if (_dragging.type === 'left') {
    newPt = clamp(_dragging.startPt + deltaPt, 0, contentWidthPt - 36);
  } else if (_dragging.type === 'right') {
    newPt = clamp(_dragging.startPt - deltaPt, 0, contentWidthPt - 36);
  } else if (_dragging.type === 'firstLine') {
    newPt = clamp(_dragging.startPt + deltaPt, -144, contentWidthPt);
  }

  // Snap to quarter-inch (18pt)
  newPt = Math.round(newPt / 18) * 18;

  const type = _dragging.type;
  const nodeId = _dragging.nodeId; // Use the nodeId saved at drag start
  const savedRange = _dragging.savedRange;
  _dragging = null;

  document.removeEventListener('mousemove', onDrag);
  document.removeEventListener('mouseup', endDrag);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';

  // Apply to document using the saved nodeId (selection may be lost)
  applyIndent(type, newPt, nodeId);

  // Restore the selection/cursor position that was active before the drag
  if (savedRange) {
    try {
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(savedRange);
    } catch (_) { /* range may be invalid after re-render */ }
  }

  // Reset cached indent values so next update picks up the change
  _lastIndentLeftPt = -1;
  _lastIndentRightPt = -1;
  _lastFirstLinePt = -1;
  updateIndentHandles();
}

// ── UXP-15: Tab Stop Helpers ─────────────────────

/**
 * Add a tab stop marker to the ruler DOM.
 */
function renderTabStopMarker(ruler, tab, index) {
  const marginLeftPx = ptToPx(_pageDims.marginLeftPt);
  const x = marginLeftPx + ptToPx(tab.positionPt);
  const typeLabel = tab.type.charAt(0).toUpperCase() + tab.type.slice(1);
  const inches = (tab.positionPt / PT_PER_INCH).toFixed(2);

  const el = document.createElement('div');
  el.className = `ruler-tab-stop tab-${tab.type}`;
  el.style.left = x + 'px';
  el.title = `${typeLabel} tab stop (${inches} in) — Drag to move, double-click to change type`;
  el.dataset.tabIndex = index;

  // Drag to move tab stop
  el.addEventListener('mousedown', (e) => {
    e.preventDefault();
    e.stopPropagation();
    _draggingTab = { index, startX: e.clientX, startPt: tab.positionPt };
    document.addEventListener('mousemove', onTabDrag);
    document.addEventListener('mouseup', endTabDrag);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  });

  // Double-click to cycle tab type
  el.addEventListener('dblclick', (e) => {
    e.preventDefault();
    e.stopPropagation();
    const currentIdx = TAB_TYPES.indexOf(tab.type);
    const nextIdx = (currentIdx + 1) % TAB_TYPES.length;
    _tabStops[index].type = TAB_TYPES[nextIdx];
    applyTabStopsToDocument();
    refreshTabStopMarkers();
  });

  ruler.appendChild(el);
}

function onTabDrag(e) {
  if (!_draggingTab) return;
  const zoom = (state.zoomLevel || 100) / 100;
  const deltaPx = (e.clientX - _draggingTab.startX) / zoom;
  const deltaPt = pxToPt(deltaPx);
  const contentWidthPt = _pageDims.widthPt - _pageDims.marginLeftPt - _pageDims.marginRightPt;
  const newPt = clamp(_draggingTab.startPt + deltaPt, 0, contentWidthPt);

  // Snap to eighth-inch (9pt)
  const snapped = Math.round(newPt / 9) * 9;

  // Update marker position live
  const ruler = document.getElementById('ruler');
  if (!ruler) return;
  const marker = ruler.querySelector(`.ruler-tab-stop[data-tab-index="${_draggingTab.index}"]`);
  if (marker) {
    const marginLeftPx = ptToPx(_pageDims.marginLeftPt);
    marker.style.left = (marginLeftPx + ptToPx(snapped)) + 'px';
  }
}

function endTabDrag(e) {
  if (!_draggingTab) return;
  const zoom = (state.zoomLevel || 100) / 100;
  const deltaPx = (e.clientX - _draggingTab.startX) / zoom;
  const deltaPt = pxToPt(deltaPx);
  const contentWidthPt = _pageDims.widthPt - _pageDims.marginLeftPt - _pageDims.marginRightPt;
  let newPt = clamp(_draggingTab.startPt + deltaPt, 0, contentWidthPt);
  newPt = Math.round(newPt / 9) * 9;

  const idx = _draggingTab.index;
  _draggingTab = null;
  document.removeEventListener('mousemove', onTabDrag);
  document.removeEventListener('mouseup', endTabDrag);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';

  // Check if dragged off ruler (below ruler = remove)
  const ruler = document.getElementById('ruler');
  if (ruler) {
    const rulerRect = ruler.getBoundingClientRect();
    if (e.clientY > rulerRect.bottom + 20) {
      // Remove the tab stop (dragged off ruler)
      _tabStops.splice(idx, 1);
      applyTabStopsToDocument();
      refreshTabStopMarkers();
      return;
    }
  }

  // Update position
  if (_tabStops[idx]) {
    _tabStops[idx].positionPt = newPt;
    // Sort tab stops by position
    _tabStops.sort((a, b) => a.positionPt - b.positionPt);
    applyTabStopsToDocument();
    refreshTabStopMarkers();
  }
}

/**
 * Refresh tab stop markers on the ruler without full rebuild.
 */
function refreshTabStopMarkers() {
  const ruler = document.getElementById('ruler');
  if (!ruler) return;

  // Remove existing tab stop markers
  ruler.querySelectorAll('.ruler-tab-stop').forEach(el => el.remove());

  // Re-render all tab stops
  _tabStops.forEach((tab, i) => {
    renderTabStopMarker(ruler, tab, i);
  });
}

/**
 * Apply tab stops to the current paragraph via WASM (if API exists).
 * Falls back to storing in state for UI display.
 */
function applyTabStopsToDocument() {
  if (!state.doc) return;
  const nodeId = getSelectionInfo()?.startNodeId || state.lastSelInfo?.startNodeId;
  if (!nodeId) return;

  // Try WASM API if available
  try {
    if (typeof state.doc.set_tab_stops === 'function') {
      const tabJson = JSON.stringify(_tabStops.map(t => ({
        position: t.positionPt,
        alignment: t.type,
      })));
      state.doc.set_tab_stops(nodeId, tabJson);
      broadcastOp({ action: 'setTabStops', nodeId, tabs: tabJson });
    }
  } catch (e) {
    // WASM API may not exist — tab stops stored in JS state for UI display
    console.warn('set_tab_stops not available:', e);
  }
}

/**
 * Load tab stops from the current paragraph (if WASM API exists).
 */
function loadTabStopsFromDocument() {
  if (!state.doc) return;
  const nodeId = getSelectionInfo()?.startNodeId || state.lastSelInfo?.startNodeId;
  if (!nodeId) return;

  try {
    if (typeof state.doc.get_tab_stops === 'function') {
      const json = state.doc.get_tab_stops(nodeId);
      const tabs = JSON.parse(json);
      _tabStops = tabs.map(t => ({
        positionPt: t.position || 0,
        type: t.alignment || 'left',
      }));
    }
  } catch (_) {
    // API may not exist — keep current JS-side tab stops
  }
}

/**
 * Handle click on the ruler content area to add a new tab stop.
 */
function handleRulerClick(e) {
  const ruler = document.getElementById('ruler');
  if (!ruler) return;

  // Don't add tab stops when clicking on handles or existing tab stops
  if (e.target.closest('.ruler-handle, .ruler-tab-stop')) return;

  const rulerRect = ruler.getBoundingClientRect();
  const zoom = (state.zoomLevel || 100) / 100;
  const clickX = (e.clientX - rulerRect.left) / zoom;
  const marginLeftPx = ptToPx(_pageDims.marginLeftPt);
  const marginRightPx = ptToPx(_pageDims.marginRightPt);
  const totalPx = ptToPx(_pageDims.widthPt);

  // Only allow tab stops in the content area (between margins)
  if (clickX < marginLeftPx || clickX > totalPx - marginRightPx) return;

  const positionPt = pxToPt(clickX - marginLeftPx);
  // Snap to eighth-inch (9pt)
  const snapped = Math.round(positionPt / 9) * 9;

  // Don't add duplicate tab stops (within 5pt of existing)
  if (_tabStops.some(t => Math.abs(t.positionPt - snapped) < 5)) return;

  _tabStops.push({ positionPt: snapped, type: 'left' });
  _tabStops.sort((a, b) => a.positionPt - b.positionPt);

  applyTabStopsToDocument();
  refreshTabStopMarkers();
}

export function renderRuler() {
  const ruler = document.getElementById('ruler');
  if (!ruler) return;

  // Apply zoom
  const zoom = (state.zoomLevel || 100) / 100;
  ruler.style.transform = `scaleX(${zoom})`;
  ruler.style.transformOrigin = 'top center';

  // Read actual dimensions from document
  const dims = getPageDimensions();
  _pageDims = dims;
  const { widthPt, marginLeftPt, marginRightPt } = dims;

  // Force rebuild if dimensions changed
  const dimsChanged = (widthPt !== _lastWidthPt || marginLeftPt !== _lastMarginLeftPt || marginRightPt !== _lastMarginRightPt);

  if (!dimsChanged) {
    // Just update indent handles
    updateIndentHandles();
    return;
  }

  _lastWidthPt = widthPt;
  _lastMarginLeftPt = marginLeftPt;
  _lastMarginRightPt = marginRightPt;

  const totalPx = ptToPx(widthPt);
  const marginLeftPx = ptToPx(marginLeftPt);
  const marginRightPx = ptToPx(marginRightPt);
  const pageWidthIn = widthPt / PT_PER_INCH;
  const contentRight = totalPx - marginRightPx;

  ruler.style.width = totalPx + 'px';

  let html = '';

  // Left margin region (gray)
  html += `<div class="ruler-margin ruler-margin-left" style="width:${marginLeftPx}px"></div>`;

  // Right margin region (gray)
  html += `<div class="ruler-margin ruler-margin-right" style="width:${marginRightPx}px"></div>`;

  // Tick marks every 0.25 inch
  const steps = Math.round(pageWidthIn * 4);
  for (let i = 0; i <= steps; i++) {
    const inches = i / 4;
    const x = inches * PX_PER_INCH;
    const isFullInch = i % 4 === 0;
    const isHalfInch = i % 4 === 2;

    let tickClass = 'ruler-tick-quarter';
    if (isFullInch) tickClass = 'ruler-tick-full';
    else if (isHalfInch) tickClass = 'ruler-tick-half';

    html += `<div class="ruler-tick ${tickClass}" style="left:${x}px"></div>`;

    if (isFullInch && inches >= 1 && inches < pageWidthIn) {
      html += `<span class="ruler-number" style="left:${x}px">${inches}</span>`;
    }
  }

  // Margin boundary indicators (non-draggable)
  html += `<div class="ruler-margin-mark ruler-margin-mark-left" style="left:${marginLeftPx}px" title="Left margin (${formatInches(marginLeftPt)})"></div>`;
  html += `<div class="ruler-margin-mark ruler-margin-mark-right" style="left:${contentRight}px" title="Right margin (${formatInches(marginRightPt)})"></div>`;

  // Draggable indent handles
  // First-line indent (inverted triangle, sits above the left indent)
  html += `<div class="ruler-handle ruler-handle-firstline" style="left:${marginLeftPx}px" title="First line indent (0 in)"></div>`;
  // Left indent (triangle pointing down)
  html += `<div class="ruler-handle ruler-handle-left" style="left:${marginLeftPx}px" title="Left indent (0 in)"></div>`;
  // Right indent (triangle pointing down, right side)
  html += `<div class="ruler-handle ruler-handle-right" style="left:${contentRight}px" title="Right indent (0 in)"></div>`;

  ruler.innerHTML = html;

  // Attach drag handlers
  const leftHandle = ruler.querySelector('.ruler-handle-left');
  const firstLineHandle = ruler.querySelector('.ruler-handle-firstline');
  const rightHandle = ruler.querySelector('.ruler-handle-right');

  if (leftHandle) leftHandle.addEventListener('mousedown', (e) => startDrag('left', e));
  if (firstLineHandle) firstLineHandle.addEventListener('mousedown', (e) => startDrag('firstLine', e));
  if (rightHandle) rightHandle.addEventListener('mousedown', (e) => startDrag('right', e));

  // UXP-15: Click on ruler to add tab stops
  ruler.addEventListener('click', handleRulerClick);

  // Reset indent cache so they get positioned
  _lastIndentLeftPt = -1;
  _lastIndentRightPt = -1;
  _lastFirstLinePt = -1;
  updateIndentHandles();

  // UXP-15: Load and render tab stops from the current paragraph
  loadTabStopsFromDocument();
  refreshTabStopMarkers();
}

// Listen for selection changes to update indent handles and tab stops
document.addEventListener('selectionchange', () => {
  if (state.currentView === 'editor') {
    updateIndentHandles();
    // UXP-15: Reload tab stops for the newly selected paragraph
    loadTabStopsFromDocument();
    refreshTabStopMarkers();
  }
});
