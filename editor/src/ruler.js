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

  _dragging = { type, startX: e.clientX, startPt, nodeId };

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
  _dragging = null;

  document.removeEventListener('mousemove', onDrag);
  document.removeEventListener('mouseup', endDrag);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';

  // Apply to document using the saved nodeId (selection may be lost)
  applyIndent(type, newPt, nodeId);

  // Reset cached indent values so next update picks up the change
  _lastIndentLeftPt = -1;
  _lastIndentRightPt = -1;
  _lastFirstLinePt = -1;
  updateIndentHandles();
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

  // Reset indent cache so they get positioned
  _lastIndentLeftPt = -1;
  _lastIndentRightPt = -1;
  _lastFirstLinePt = -1;
  updateIndentHandles();
}

// Listen for selection changes to update indent handles
document.addEventListener('selectionchange', () => {
  if (state.currentView === 'editor') {
    updateIndentHandles();
  }
});
