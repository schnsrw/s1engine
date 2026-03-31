// Canvas-based document renderer for pixel-accurate glyph placement.
//
// Provides an alternative to DOM-based rendering by drawing directly onto
// HTML5 Canvas elements using the structured layout JSON from the WASM engine.

import { state, $ } from './state.js';
import * as modelSelection from './selection/model-selection.js';

// -------------------------------------------------------
// Module state
// -------------------------------------------------------

let _canvasMode = false;
let _canvasPages = []; // Array of { canvas, ctx, pageData } per page
let _lastLayoutJson = null; // Cached layout JSON for hit testing
let _lastSceneSummary = null; // Cached scene summary
let _scenePageCache = new Map(); // Map<pageIndex, { revision, scene }>
let _caretState = null; // { pageIndex, x, y, width, height }
let _selectionRects = []; // Array of { pageIndex, x, y, width, height }

// -------------------------------------------------------
// Public API
// -------------------------------------------------------

/**
 * Check whether canvas rendering mode is active.
 * @returns {boolean}
 */
export function isCanvasMode() {
  return _canvasMode;
}

/**
 * Enable or disable canvas rendering mode.
 * When enabled, the next render cycle will use canvas instead of DOM.
 * @param {boolean} enabled
 */
export function setCanvasMode(enabled) {
  _canvasMode = !!enabled;
  try {
    localStorage.setItem('s1-canvas-mode', _canvasMode ? '1' : '0');
  } catch (_) {
    // localStorage may not be available
  }
}

/**
 * Initialize the canvas renderer. Restores the saved preference.
 * @param {HTMLElement} _container - The scroll container (not used yet, reserved)
 */
export function initCanvasRenderer(_container) {
  try {
    _canvasMode = localStorage.getItem('s1-canvas-mode') === '1';
  } catch (_) {
    _canvasMode = false;
  }
  // Update the toggle UI to match restored preference
  const toggle = $('canvasModeToggle');
  if (toggle) toggle.checked = _canvasMode;
}

/**
 * Render the document using canvas elements, replacing the content of the
 * given container. Fetches layout JSON from the WASM engine.
 *
 * @param {HTMLElement} container - The container to render into (e.g. pageContainer)
 * @returns {boolean} true if rendering was performed, false on error
 */
export function renderDocumentCanvas(container) {
  const { doc } = state;
  if (!doc || !container) return false;

  let layoutJson;
  try {
    const jsonStr = doc.to_layout_json();
    layoutJson = JSON.parse(jsonStr);
  } catch (e) {
    console.error('Canvas render: failed to get layout JSON:', e);
    return false;
  }

  _lastLayoutJson = layoutJson;
  renderLayoutToCanvas(layoutJson, container);
  return true;
}

/**
 * Hit-test a point in the canvas coordinate system to find the closest
 * document run and approximate character offset.
 *
 * @param {number} clientX - X position relative to the container
 * @param {number} clientY - Y position relative to the container
 * @param {HTMLElement} container - The scroll container
 * @returns {{ sourceId: string, offset: number, run: object } | null}
 */
export function canvasHitTest(clientX, clientY, container) {
  if (!_lastLayoutJson || !_lastLayoutJson.pages || !container) return null;

  const dpr = window.devicePixelRatio || 1;
  const ptToPx = 96 / 72;
  const PAGE_GAP = 20; // px gap between pages

  // Convert client coords to container-relative coords
  const rect = container.getBoundingClientRect();
  const scrollX = container.scrollLeft;
  const scrollY = container.scrollTop;
  const cx = clientX - rect.left + scrollX;
  const cy = clientY - rect.top + scrollY;

  // Walk through pages to find which one was clicked
  let pageTopPx = PAGE_GAP;
  for (let pi = 0; pi < _lastLayoutJson.pages.length; pi++) {
    const page = _lastLayoutJson.pages[pi];
    const pageWidthPx = page.width * ptToPx;
    const pageHeightPx = page.height * ptToPx;

    // Center the page horizontally in the container
    const containerWidth = container.clientWidth;
    const pageLeftPx = Math.max(PAGE_GAP, (containerWidth - pageWidthPx) / 2);

    if (cy >= pageTopPx && cy < pageTopPx + pageHeightPx &&
        cx >= pageLeftPx && cx < pageLeftPx + pageWidthPx) {
      // Convert to page-local pt coordinates
      const localX = (cx - pageLeftPx) / ptToPx;
      const localY = (cy - pageTopPx) / ptToPx;
      return findClosestRun(page, localX, localY);
    }
    pageTopPx += pageHeightPx + PAGE_GAP;
  }
  return null;
}

// -------------------------------------------------------
// Internal rendering
// -------------------------------------------------------

const PAGE_GAP_PX = 20;

/**
 * Render parsed layout JSON into canvas elements inside the container.
 */
function renderLayoutToCanvas(layoutJson, container) {
  // Remove previous canvas pages
  _canvasPages.forEach(p => {
    if (p.canvas.parentNode) p.canvas.parentNode.removeChild(p.canvas);
  });
  _canvasPages = [];

  if (!layoutJson || !layoutJson.pages) return;

  const dpr = window.devicePixelRatio || 1;
  const ptToPx = 96 / 72;

  // Clear container
  container.innerHTML = '';

  for (const page of layoutJson.pages) {
    const widthPx = page.width * ptToPx;
    const heightPx = page.height * ptToPx;

    const canvas = document.createElement('canvas');
    canvas.className = 's1-canvas-page';
    canvas.style.width = widthPx + 'px';
    canvas.style.height = heightPx + 'px';
    canvas.style.margin = PAGE_GAP_PX + 'px auto';
    canvas.style.display = 'block';
    canvas.style.background = 'white';
    canvas.style.boxShadow = '0 1px 4px rgba(0,0,0,0.15), 0 2px 8px rgba(0,0,0,0.08)';
    canvas.style.borderRadius = '2px';

    // Set actual pixel size for retina displays
    canvas.width = Math.ceil(widthPx * dpr);
    canvas.height = Math.ceil(heightPx * dpr);

    const ctx = canvas.getContext('2d');
    ctx.scale(dpr, dpr);

    // White background
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, widthPx, heightPx);

    // Convert pt positions to px for drawing
    ctx.save();
    ctx.scale(ptToPx, ptToPx);

    // Render header
    if (page.header) {
      renderBlock(ctx, page.header);
    }

    // Render body blocks
    for (const block of page.blocks || []) {
      renderBlock(ctx, block);
    }

    // Render floating images
    for (const img of page.floatingImages || []) {
      renderBlock(ctx, img);
    }

    // Render footnotes
    if (page.footnotes && page.footnotes.length > 0) {
      // Draw a thin separator line above footnotes
      const contentBottom = page.contentArea
        ? page.contentArea.y + page.contentArea.height
        : page.height - 72;
      ctx.strokeStyle = '#999999';
      ctx.lineWidth = 0.5;
      ctx.beginPath();
      ctx.moveTo(page.contentArea ? page.contentArea.x : 72, contentBottom - 12);
      ctx.lineTo((page.contentArea ? page.contentArea.x : 72) + 120, contentBottom - 12);
      ctx.stroke();

      for (const note of page.footnotes) {
        renderBlock(ctx, note);
      }
    }

    // Render footer
    if (page.footer) {
      renderBlock(ctx, page.footer);
    }

    ctx.restore();

    container.appendChild(canvas);
    _canvasPages.push({ canvas, ctx, pageData: page });
  }
}

/**
 * Render a single layout block (paragraph, table, or image) to canvas.
 */
function renderBlock(ctx, block) {
  if (!block || !block.type) return;

  switch (block.type) {
    case 'paragraph':
      renderParagraph(ctx, block);
      break;
    case 'table':
      renderTable(ctx, block);
      break;
    case 'image':
      renderImage(ctx, block);
      break;
    case 'shape':
      renderShape(ctx, block);
      break;
    case 'textBox':
      renderTextBox(ctx, block);
      break;
    default:
      break;
  }
}

/**
 * Render a paragraph block: background, border, list markers, then lines/runs.
 */
function renderParagraph(ctx, block) {
  const bounds = block.bounds;
  if (!bounds) return;

  // Background color
  if (block.backgroundColor) {
    ctx.fillStyle = block.backgroundColor;
    ctx.fillRect(bounds.x, bounds.y, bounds.width, bounds.height);
  }

  // Border
  if (block.border) {
    ctx.strokeStyle = '#000000';
    ctx.lineWidth = 0.5;
    ctx.strokeRect(bounds.x, bounds.y, bounds.width, bounds.height);
  }

  // List marker
  if (block.listMarker) {
    const firstLine = (block.lines && block.lines.length > 0) ? block.lines[0] : null;
    const markerY = firstLine ? (bounds.y + firstLine.baselineY) : (bounds.y + 12);
    const markerX = bounds.x - 18 + (block.listLevel || 0) * 18;
    ctx.fillStyle = '#000000';
    ctx.font = '12pt serif';
    ctx.fillText(block.listMarker, markerX, markerY);
  }

  // Render lines
  for (const line of block.lines || []) {
    for (const run of line.runs || []) {
      renderRun(ctx, run, bounds, line);
    }
  }
}

/**
 * Render a single glyph run on the canvas.
 */
function renderRun(ctx, run, blockBounds, line) {
  if (!run.text && !run.inlineImage) return;

  // Handle inline images
  if (run.inlineImage && run.inlineImage.src) {
    const img = new Image();
    const imgData = run.inlineImage;
    const x = blockBounds.x + run.x;
    const y = blockBounds.y + line.baselineY - imgData.height;
    img.onload = function () {
      ctx.drawImage(img, x, y, imgData.width, imgData.height);
    };
    img.onerror = function () {
      // Draw a placeholder rect for broken images
      ctx.save();
      ctx.strokeStyle = '#ccc';
      ctx.lineWidth = 0.5;
      ctx.strokeRect(x, y, imgData.width, imgData.height);
      ctx.restore();
    };
    img.src = imgData.src;
    return;
  }

  // Build font string
  const parts = [];
  if (run.italic) parts.push('italic');
  if (run.bold) parts.push('bold');
  const fontSize = run.fontSize || 12;
  const family = run.fontFamily || 'serif';
  parts.push(fontSize + 'pt');
  parts.push(family);
  ctx.font = parts.join(' ');

  // Position: run.x is relative to the block's x, baselineY is relative to block's y
  const x = blockBounds.x + run.x;
  const baselineY = blockBounds.y + line.baselineY;

  // Superscript/subscript offset
  let yOffset = 0;
  if (run.superscript) yOffset = -(fontSize * 0.35);
  if (run.subscript) yOffset = (fontSize * 0.2);

  // Highlight background
  if (run.highlightColor) {
    const metrics = ctx.measureText(run.text);
    ctx.fillStyle = run.highlightColor;
    ctx.fillRect(x, baselineY - fontSize * 0.85 + yOffset, metrics.width, fontSize * 1.2);
  }

  // Text color
  ctx.fillStyle = run.color || '#000000';

  // Strikethrough (draw behind text)
  if (run.strikethrough) {
    const metrics = ctx.measureText(run.text);
    const midY = baselineY - fontSize * 0.3 + yOffset;
    ctx.beginPath();
    ctx.strokeStyle = run.color || '#000000';
    ctx.lineWidth = Math.max(0.5, fontSize / 20);
    ctx.moveTo(x, midY);
    ctx.lineTo(x + metrics.width, midY);
    ctx.stroke();
  }

  // Draw text
  if (run.characterSpacing && run.characterSpacing !== 0) {
    // Manual letter-spacing: draw character by character
    let cx = x;
    for (const ch of run.text) {
      ctx.fillText(ch, cx, baselineY + yOffset);
      cx += ctx.measureText(ch).width + run.characterSpacing;
    }
  } else {
    ctx.fillText(run.text, x, baselineY + yOffset);
  }

  // Underline
  if (run.underline) {
    const metrics = ctx.measureText(run.text);
    const underlineY = baselineY + 2 + yOffset;
    ctx.beginPath();
    ctx.strokeStyle = run.color || '#000000';
    ctx.lineWidth = Math.max(0.5, fontSize / 20);
    ctx.moveTo(x, underlineY);
    ctx.lineTo(x + metrics.width, underlineY);
    ctx.stroke();
  }
}

/**
 * Render a table block: borders, cells, cell content.
 */
function renderTable(ctx, block) {
  const bounds = block.bounds;
  if (!bounds) return;

  for (const row of block.rows || []) {
    for (const cell of row.cells || []) {
      const cellX = bounds.x + cell.bounds.x;
      const cellY = bounds.y + cell.bounds.y;
      const cellW = cell.bounds.width;
      const cellH = cell.bounds.height;

      // Cell background
      if (cell.backgroundColor) {
        ctx.fillStyle = cell.backgroundColor;
        ctx.fillRect(cellX, cellY, cellW, cellH);
      }

      // Cell borders — draw simple lines (ignoring CSS border parsing)
      ctx.strokeStyle = '#000000';
      ctx.lineWidth = 0.5;
      if (cell.borderTop) {
        ctx.beginPath();
        ctx.moveTo(cellX, cellY);
        ctx.lineTo(cellX + cellW, cellY);
        ctx.stroke();
      }
      if (cell.borderBottom) {
        ctx.beginPath();
        ctx.moveTo(cellX, cellY + cellH);
        ctx.lineTo(cellX + cellW, cellY + cellH);
        ctx.stroke();
      }
      if (cell.borderLeft) {
        ctx.beginPath();
        ctx.moveTo(cellX, cellY);
        ctx.lineTo(cellX, cellY + cellH);
        ctx.stroke();
      }
      if (cell.borderRight) {
        ctx.beginPath();
        ctx.moveTo(cellX + cellW, cellY);
        ctx.lineTo(cellX + cellW, cellY + cellH);
        ctx.stroke();
      }

      // Render cell content blocks — adjust coordinates relative to cell
      for (const cellBlock of cell.blocks || []) {
        renderBlock(ctx, cellBlock);
      }
    }
  }
}

/**
 * Render an image block.
 */
function renderImage(ctx, block) {
  if (!block.src) {
    // Draw a placeholder box
    const b = block.imageBounds || block.bounds || {};
    ctx.strokeStyle = '#cccccc';
    ctx.lineWidth = 1;
    ctx.strokeRect(b.x || 0, b.y || 0, b.width || 100, b.height || 100);
    ctx.fillStyle = '#f0f0f0';
    ctx.fillRect(b.x || 0, b.y || 0, b.width || 100, b.height || 100);
    ctx.fillStyle = '#999999';
    ctx.font = '10pt sans-serif';
    ctx.fillText('[Image]', (b.x || 0) + 4, (b.y || 0) + 14);
    return;
  }

  const b = block.imageBounds || block.bounds || {};
  const img = new Image();
  img.onload = function () {
    ctx.drawImage(img, b.x || 0, b.y || 0, b.width || img.width, b.height || img.height);
  };
  img.src = block.src;
}

/**
 * Render a shape block (rectangle, ellipse, line, etc.).
 */
function renderShape(ctx, block) {
  const b = block.bounds;
  if (!b) return;

  ctx.save();

  // Apply rotation if specified
  if (block.rotationDeg && block.rotationDeg !== 0) {
    const cx = b.x + b.width / 2;
    const cy = b.y + b.height / 2;
    ctx.translate(cx, cy);
    ctx.rotate((block.rotationDeg * Math.PI) / 180);
    ctx.translate(-cx, -cy);
  }

  // Apply flip transforms
  if (block.flipH || block.flipV) {
    const cx = b.x + b.width / 2;
    const cy = b.y + b.height / 2;
    ctx.translate(cx, cy);
    ctx.scale(block.flipH ? -1 : 1, block.flipV ? -1 : 1);
    ctx.translate(-cx, -cy);
  }

  const shapeType = block.shapeType || 'rect';

  // Draw shape path
  ctx.beginPath();
  if (shapeType === 'ellipse') {
    const cx = b.x + b.width / 2;
    const cy = b.y + b.height / 2;
    ctx.ellipse(cx, cy, b.width / 2, b.height / 2, 0, 0, Math.PI * 2);
  } else if (shapeType === 'roundRect') {
    const r = Math.min(b.width, b.height) * 0.1;
    roundRect(ctx, b.x, b.y, b.width, b.height, r);
  } else if (shapeType === 'line') {
    ctx.moveTo(b.x, b.y + b.height / 2);
    ctx.lineTo(b.x + b.width, b.y + b.height / 2);
  } else if (shapeType === 'triangle') {
    ctx.moveTo(b.x + b.width / 2, b.y);
    ctx.lineTo(b.x + b.width, b.y + b.height);
    ctx.lineTo(b.x, b.y + b.height);
    ctx.closePath();
  } else if (shapeType === 'diamond') {
    ctx.moveTo(b.x + b.width / 2, b.y);
    ctx.lineTo(b.x + b.width, b.y + b.height / 2);
    ctx.lineTo(b.x + b.width / 2, b.y + b.height);
    ctx.lineTo(b.x, b.y + b.height / 2);
    ctx.closePath();
  } else {
    // Default: rectangle
    ctx.rect(b.x, b.y, b.width, b.height);
  }

  // Fill
  if (block.fillColor) {
    ctx.fillStyle = block.fillColor;
    ctx.fill();
  }

  // Stroke
  if (block.strokeColor || shapeType !== 'line') {
    ctx.strokeStyle = block.strokeColor || '#000000';
    ctx.lineWidth = block.strokeWidth || 1;
    ctx.stroke();
  }

  ctx.restore();
}

/**
 * Render a text box block (shape with embedded text content).
 */
function renderTextBox(ctx, block) {
  const b = block.bounds;
  if (!b) return;

  ctx.save();

  // Draw the text box frame
  ctx.beginPath();
  ctx.rect(b.x, b.y, b.width, b.height);

  if (block.fillColor) {
    ctx.fillStyle = block.fillColor;
    ctx.fill();
  }
  if (block.strokeColor) {
    ctx.strokeStyle = block.strokeColor;
    ctx.lineWidth = block.strokeWidth || 0.5;
    ctx.stroke();
  }

  // Render inner content blocks
  for (const inner of block.blocks || []) {
    renderBlock(ctx, inner);
  }

  ctx.restore();
}

/**
 * Helper: draw a rounded rectangle path.
 */
function roundRect(ctx, x, y, w, h, r) {
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.arcTo(x + w, y, x + w, y + r, r);
  ctx.lineTo(x + w, y + h - r);
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r);
  ctx.lineTo(x + r, y + h);
  ctx.arcTo(x, y + h, x, y + h - r, r);
  ctx.lineTo(x, y + r);
  ctx.arcTo(x, y, x + r, y, r);
}

// -------------------------------------------------------
// Scene API integration
// -------------------------------------------------------

/**
 * Render using the new scene API (page_scene) instead of to_layout_json().
 * Falls back to legacy layout JSON if scene API is not available.
 *
 * @param {HTMLElement} container
 * @returns {boolean}
 */
export function renderDocumentScene(container) {
  const { doc, fontDb } = state;
  if (!doc || !container) return false;

  // Check if scene API is available
  if (typeof doc.scene_summary !== 'function') {
    return renderDocumentCanvas(container);
  }

  try {
    // Use font-aware methods when fontDb is loaded (accurate text shaping)
    const summaryStr = (fontDb && typeof doc.scene_summary_with_fonts === 'function')
      ? doc.scene_summary_with_fonts(fontDb, '{}')
      : doc.scene_summary('{}');
    _lastSceneSummary = JSON.parse(summaryStr);
  } catch (e) {
    console.error('Scene render: failed to get scene summary:', e);
    return renderDocumentCanvas(container);
  }

  const summary = _lastSceneSummary;
  if (!summary || !summary.pages) return false;

  const startPage = 0;
  const endPage = summary.page_count;

  try {
    const scenesStr = (fontDb && typeof doc.visible_page_scenes_with_fonts === 'function')
      ? doc.visible_page_scenes_with_fonts(fontDb, startPage, endPage)
      : doc.visible_page_scenes(startPage, endPage);
    const scenesData = JSON.parse(scenesStr);
    renderScenesToCanvas(scenesData.pages || [], container);
    return true;
  } catch (e) {
    console.error('Scene render: failed to get page scenes:', e);
    return renderDocumentCanvas(container);
  }
}

/**
 * Render scene pages to canvas elements.
 */
function renderScenesToCanvas(scenes, container) {
  _canvasPages.forEach(p => {
    if (p.canvas.parentNode) p.canvas.parentNode.removeChild(p.canvas);
  });
  _canvasPages = [];
  container.innerHTML = '';

  const dpr = window.devicePixelRatio || 1;
  const ptToPx = 96 / 72;

  for (const scene of scenes) {
    const bounds = scene.bounds_pt;
    if (!bounds) continue;

    const widthPx = bounds.width * ptToPx;
    const heightPx = bounds.height * ptToPx;

    const canvas = document.createElement('canvas');
    canvas.className = 's1-canvas-page';
    canvas.style.width = widthPx + 'px';
    canvas.style.height = heightPx + 'px';
    canvas.style.margin = PAGE_GAP_PX + 'px auto';
    canvas.style.display = 'block';
    canvas.style.background = 'white';
    canvas.style.boxShadow = '0 1px 4px rgba(0,0,0,0.15), 0 2px 8px rgba(0,0,0,0.08)';
    canvas.style.borderRadius = '2px';
    canvas.dataset.pageIndex = scene.page_index;

    canvas.width = Math.ceil(widthPx * dpr);
    canvas.height = Math.ceil(heightPx * dpr);

    const ctx = canvas.getContext('2d');
    ctx.scale(dpr, dpr);

    // White background
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, widthPx, heightPx);

    // Draw scene items in order
    ctx.save();
    ctx.scale(ptToPx, ptToPx);
    renderSceneItems(ctx, scene.items || []);

    // Draw caret if on this page
    if (_caretState && _caretState.pageIndex === scene.page_index) {
      ctx.fillStyle = '#000000';
      ctx.fillRect(_caretState.x, _caretState.y, _caretState.width, _caretState.height);
    }

    // Draw selection rects for this page
    for (const sel of _selectionRects) {
      if (sel.pageIndex === scene.page_index) {
        ctx.fillStyle = 'rgba(51, 102, 204, 0.25)';
        ctx.fillRect(sel.x, sel.y, sel.width, sel.height);
      }
    }

    ctx.restore();

    container.appendChild(canvas);
    // Save backing buffer for fast caret blink (avoids full redraw)
    const backingBuffer = ctx.getImageData(0, 0, canvas.width, canvas.height);
    _canvasPages.push({ canvas, ctx, pageData: scene, _backingBuffer: backingBuffer });
  }
}

/**
 * Render scene items (text runs, backgrounds, borders, images, shapes, etc.).
 */
function renderSceneItems(ctx, items) {
  for (const item of items) {
    switch (item.kind) {
      case 'paragraph_background':
        ctx.fillStyle = item.color || '#F0F0F0';
        ctx.fillRect(item.bounds_pt.x, item.bounds_pt.y, item.bounds_pt.width, item.bounds_pt.height);
        break;

      case 'paragraph_border':
        ctx.strokeStyle = '#000000';
        ctx.lineWidth = 0.5;
        ctx.strokeRect(item.bounds_pt.x, item.bounds_pt.y, item.bounds_pt.width, item.bounds_pt.height);
        break;

      case 'list_marker':
        ctx.fillStyle = item.color || '#000000';
        ctx.font = (item.font_size_pt || 11) + 'pt serif';
        ctx.fillText(item.marker_text, item.bounds_pt.x, item.bounds_pt.y + (item.font_size_pt || 11));
        break;

      case 'text_run':
        renderSceneTextRun(ctx, item);
        break;

      case 'table_cell_background':
        ctx.fillStyle = item.color || '#E8E8E8';
        ctx.fillRect(item.bounds_pt.x, item.bounds_pt.y, item.bounds_pt.width, item.bounds_pt.height);
        break;

      case 'table_border_segment':
        ctx.strokeStyle = '#000000';
        ctx.lineWidth = 0.5;
        ctx.beginPath();
        ctx.moveTo(item.start_pt.x, item.start_pt.y);
        ctx.lineTo(item.end_pt.x, item.end_pt.y);
        ctx.stroke();
        break;

      case 'image':
        renderSceneImage(ctx, item);
        break;

      case 'shape':
        renderSceneShape(ctx, item);
        break;

      case 'text_box':
        renderSceneTextBox(ctx, item);
        break;

      case 'comment_anchor':
        ctx.fillStyle = item.color || 'rgba(255, 243, 205, 0.5)';
        ctx.fillRect(item.bounds_pt.x, item.bounds_pt.y, item.bounds_pt.width, item.bounds_pt.height);
        break;

      case 'footnote_separator':
        ctx.strokeStyle = '#999999';
        ctx.lineWidth = 0.5;
        ctx.beginPath();
        ctx.moveTo(item.bounds_pt.x, item.bounds_pt.y);
        ctx.lineTo(item.bounds_pt.x + item.bounds_pt.width, item.bounds_pt.y);
        ctx.stroke();
        break;

      default:
        break;
    }
  }
}

/**
 * Render a text_run scene item.
 */
function renderSceneTextRun(ctx, item) {
  const b = item.bounds_pt;
  if (!b) return;

  // Hidden text: skip rendering entirely
  if (item.hidden) return;

  // Resolve display text (caps/smallCaps transforms applied in layout,
  // but we handle small_caps font size reduction here)
  let displayText = item.text || '';
  const fontSize = item.font_size_pt || 11;
  let effectiveFontSize = fontSize;

  // Small caps: render at ~70% size (text already uppercased by layout engine)
  if (item.small_caps) {
    effectiveFontSize = fontSize * 0.7;
  }

  const parts = [];
  if (item.italic) parts.push('italic');
  if (item.bold) parts.push('bold');
  parts.push(effectiveFontSize + 'pt');
  parts.push(item.font_family || 'serif');
  ctx.font = parts.join(' ');

  const baselineY = item.baseline_y || (b.y + fontSize);
  let yOffset = 0;
  if (item.superscript) yOffset = -(fontSize * 0.35);
  if (item.subscript) yOffset = (fontSize * 0.2);
  // Baseline shift (positive = up, in points)
  if (item.baseline_shift) yOffset -= item.baseline_shift;

  // Highlight background
  if (item.highlight_color) {
    ctx.fillStyle = item.highlight_color;
    ctx.fillRect(b.x, b.y, b.width, b.height);
  }

  // Text color
  ctx.fillStyle = item.color || '#000000';

  // Strikethrough (single)
  if (item.strikethrough) {
    const midY = baselineY - fontSize * 0.3 + yOffset;
    ctx.beginPath();
    ctx.strokeStyle = item.color || '#000000';
    ctx.lineWidth = Math.max(0.5, fontSize / 20);
    ctx.moveTo(b.x, midY);
    ctx.lineTo(b.x + b.width, midY);
    ctx.stroke();
  }

  // Double strikethrough
  if (item.double_strikethrough) {
    const midY = baselineY - fontSize * 0.3 + yOffset;
    const gap = Math.max(1.0, fontSize / 12);
    ctx.beginPath();
    ctx.strokeStyle = item.color || '#000000';
    ctx.lineWidth = Math.max(0.5, fontSize / 24);
    ctx.moveTo(b.x, midY - gap / 2);
    ctx.lineTo(b.x + b.width, midY - gap / 2);
    ctx.moveTo(b.x, midY + gap / 2);
    ctx.lineTo(b.x + b.width, midY + gap / 2);
    ctx.stroke();
  }

  // Inline image: render the image instead of text
  if (item.inline_image && item.inline_image.src) {
    const img = new Image();
    const iw = item.inline_image.width || b.width;
    const ih = item.inline_image.height || b.height;
    const ix = b.x;
    const iy = baselineY - ih + yOffset;
    img.onload = () => ctx.drawImage(img, ix, iy, iw, ih);
    img.onerror = () => {
      ctx.strokeStyle = '#ccc';
      ctx.lineWidth = 0.5;
      ctx.strokeRect(ix, iy, iw, ih);
    };
    img.src = item.inline_image.src;
    return;
  }

  // Draw text
  if (item.character_spacing && item.character_spacing !== 0) {
    let cx = b.x;
    for (const ch of displayText) {
      ctx.fillText(ch, cx, baselineY + yOffset);
      cx += ctx.measureText(ch).width + item.character_spacing;
    }
  } else {
    ctx.fillText(displayText, b.x, baselineY + yOffset);
  }

  // Underline (all 6 styles)
  const uStyle = item.underline;
  if (uStyle && uStyle !== 'none') {
    const underlineY = baselineY + 2 + yOffset;
    ctx.beginPath();
    ctx.strokeStyle = item.color || '#000000';

    switch (uStyle) {
      case 'double': {
        const gap = Math.max(1.5, fontSize / 10);
        ctx.lineWidth = Math.max(0.5, fontSize / 24);
        ctx.moveTo(b.x, underlineY - gap / 2);
        ctx.lineTo(b.x + b.width, underlineY - gap / 2);
        ctx.moveTo(b.x, underlineY + gap / 2);
        ctx.lineTo(b.x + b.width, underlineY + gap / 2);
        break;
      }
      case 'thick':
        ctx.lineWidth = Math.max(1.0, fontSize / 10);
        ctx.moveTo(b.x, underlineY);
        ctx.lineTo(b.x + b.width, underlineY);
        break;
      case 'dotted':
        ctx.lineWidth = Math.max(0.5, fontSize / 20);
        ctx.setLineDash([1, 2]);
        ctx.moveTo(b.x, underlineY);
        ctx.lineTo(b.x + b.width, underlineY);
        ctx.setLineDash([]);
        break;
      case 'dashed':
        ctx.lineWidth = Math.max(0.5, fontSize / 20);
        ctx.setLineDash([4, 2]);
        ctx.moveTo(b.x, underlineY);
        ctx.lineTo(b.x + b.width, underlineY);
        ctx.setLineDash([]);
        break;
      case 'wave': {
        ctx.lineWidth = Math.max(0.5, fontSize / 20);
        const amp = Math.max(1, fontSize / 12);
        const period = Math.max(4, fontSize / 4);
        let wx = b.x;
        ctx.moveTo(wx, underlineY);
        while (wx < b.x + b.width) {
          ctx.quadraticCurveTo(wx + period / 4, underlineY - amp, wx + period / 2, underlineY);
          ctx.quadraticCurveTo(wx + 3 * period / 4, underlineY + amp, wx + period, underlineY);
          wx += period;
        }
        break;
      }
      default: // 'single'
        ctx.lineWidth = Math.max(0.5, fontSize / 20);
        ctx.moveTo(b.x, underlineY);
        ctx.lineTo(b.x + b.width, underlineY);
    }
    ctx.stroke();
  }
}

/**
 * Render a scene image item.
 */
function renderSceneImage(ctx, item) {
  const b = item.bounds_pt;
  if (!b) return;

  if (item.src_base64) {
    const img = new Image();
    img.onload = () => ctx.drawImage(img, b.x, b.y, b.width, b.height);
    img.src = item.src_base64;
  } else {
    ctx.strokeStyle = '#cccccc';
    ctx.lineWidth = 1;
    ctx.strokeRect(b.x, b.y, b.width, b.height);
    ctx.fillStyle = '#f0f0f0';
    ctx.fillRect(b.x, b.y, b.width, b.height);
    ctx.fillStyle = '#999999';
    ctx.font = '10pt sans-serif';
    ctx.fillText('[Image]', b.x + 4, b.y + 14);
  }
}

/**
 * Render a scene shape item.
 */
function renderSceneShape(ctx, item) {
  renderShape(ctx, {
    bounds: item.bounds_pt,
    shapeType: item.shape_type,
    fillColor: item.fill_color,
    strokeColor: item.stroke_color,
    strokeWidth: item.stroke_width,
    rotationDeg: item.rotation_deg,
    flipH: item.flip_h,
    flipV: item.flip_v,
  });
}

/**
 * Render a scene text box item.
 */
function renderSceneTextBox(ctx, item) {
  const b = item.bounds_pt;
  if (!b) return;

  ctx.save();
  ctx.beginPath();
  ctx.rect(b.x, b.y, b.width, b.height);
  if (item.fill_color) {
    ctx.fillStyle = item.fill_color;
    ctx.fill();
  }
  if (item.stroke_color) {
    ctx.strokeStyle = item.stroke_color;
    ctx.lineWidth = item.stroke_width || 0.5;
    ctx.stroke();
  }
  ctx.restore();
}

// -------------------------------------------------------
// Caret and selection painting (scene-mode)
// -------------------------------------------------------

/**
 * Update the caret position from a WASM caret_rect result.
 * @param {object} rectPt - { page_index, x, y, width, height }
 */
export function updateCaretFromRect(rectPt) {
  if (!rectPt) {
    _caretState = null;
    return;
  }
  _caretState = {
    pageIndex: rectPt.page_index,
    x: rectPt.x,
    y: rectPt.y,
    width: rectPt.width || 1.0,
    height: rectPt.height,
  };
}

/**
 * Update selection rectangles from WASM selection_rects result.
 * @param {Array} rects - Array of { page_index, x, y, width, height }
 */
export function updateSelectionFromRects(rects) {
  _selectionRects = (rects || []).map(r => ({
    pageIndex: r.page_index,
    x: r.x,
    y: r.y,
    width: r.width,
    height: r.height,
  }));
}

/**
 * Perform a scene-based hit test using the WASM hit_test API.
 * Falls back to client-side hit testing if not available.
 *
 * @param {number} clientX
 * @param {number} clientY
 * @param {HTMLElement} container
 * @returns {object|null} Hit test result from WASM
 */
export function sceneHitTest(clientX, clientY, container) {
  const { doc } = state;
  if (!doc || typeof doc.hit_test !== 'function') {
    return canvasHitTest(clientX, clientY, container);
  }

  const ptToPx = 96 / 72;
  const rect = container.getBoundingClientRect();
  const scrollX = container.scrollLeft;
  const scrollY = container.scrollTop;
  const cx = clientX - rect.left + scrollX;
  const cy = clientY - rect.top + scrollY;

  // Find which page canvas was clicked
  let pageTopPx = PAGE_GAP_PX;
  for (const entry of _canvasPages) {
    const pageIdx = parseInt(entry.canvas.dataset.pageIndex || '0');
    const pageW = parseFloat(entry.canvas.style.width);
    const pageH = parseFloat(entry.canvas.style.height);
    const containerWidth = container.clientWidth;
    const pageLeftPx = Math.max(PAGE_GAP_PX, (containerWidth - pageW) / 2);

    if (cy >= pageTopPx && cy < pageTopPx + pageH &&
        cx >= pageLeftPx && cx < pageLeftPx + pageW) {
      const localXPt = (cx - pageLeftPx) / ptToPx;
      const localYPt = (cy - pageTopPx) / ptToPx;

      try {
        const resultStr = doc.hit_test(pageIdx, localXPt, localYPt);
        return JSON.parse(resultStr);
      } catch (e) {
        console.error('Scene hit test failed:', e);
        return null;
      }
    }
    pageTopPx += pageH + PAGE_GAP_PX;
  }
  return null;
}

// -------------------------------------------------------
// Hit testing (legacy layout JSON)
// -------------------------------------------------------

/**
 * Find the closest glyph run to a point within a page.
 */
function findClosestRun(page, x, y) {
  let closest = null;
  let minDist = Infinity;

  function scanBlocks(blocks) {
    for (const block of blocks) {
      if (block.type === 'paragraph') {
        for (const line of block.lines || []) {
          const lineTop = block.bounds.y + line.baselineY - line.height;
          const lineBottom = block.bounds.y + line.baselineY + 4;
          if (y >= lineTop && y <= lineBottom) {
            for (const run of line.runs || []) {
              const runX = block.bounds.x + run.x;
              const runRight = runX + run.width;
              // Vertical distance is near zero (on the line), so use horizontal
              let dist;
              if (x >= runX && x <= runRight) {
                dist = 0;
              } else {
                dist = Math.min(Math.abs(x - runX), Math.abs(x - runRight));
              }
              if (dist < minDist) {
                minDist = dist;
                // Estimate character offset within run
                const charOffset = estimateCharOffset(run, x - runX);
                closest = {
                  sourceId: run.sourceId,
                  offset: charOffset,
                  run: run,
                  blockSourceId: block.sourceId,
                };
              }
            }
          }
        }
      } else if (block.type === 'table') {
        for (const row of block.rows || []) {
          for (const cell of row.cells || []) {
            scanBlocks(cell.blocks || []);
          }
        }
      }
    }
  }

  scanBlocks(page.blocks || []);
  if (page.header) scanBlocks([page.header]);
  if (page.footer) scanBlocks([page.footer]);

  return closest;
}

/**
 * Estimate the character offset for a click position within a run.
 */
function estimateCharOffset(run, localX) {
  if (!run.text || run.text.length === 0) return 0;
  if (localX <= 0) return 0;

  const avgCharWidth = run.width / run.text.length;
  if (avgCharWidth <= 0) return 0;

  const charIndex = Math.round(localX / avgCharWidth);
  return Math.max(0, Math.min(charIndex, run.text.length));
}

/**
 * Clean up canvas elements and state.
 */
export function destroyCanvasRenderer() {
  _canvasPages.forEach(p => {
    if (p.canvas.parentNode) p.canvas.parentNode.removeChild(p.canvas);
  });
  _canvasPages = [];
  _lastLayoutJson = null;
  _stopCaretBlink();
}

// -------------------------------------------------------
// Canvas mouse event wiring
// -------------------------------------------------------

let _mouseDown = false;
let _caretBlinkTimer = null;
let _caretVisible = true;

/**
 * Wire mouse events on the canvas container for click-to-place-caret,
 * drag-to-select, and double-click-to-select-word.
 *
 * @param {HTMLElement} container - The page scroll container
 */
export function initCanvasMouseEvents(container) {
  if (!container) return;

  // Focus the hidden textarea so it captures keyboard input.
  // Uses direct DOM access to avoid circular import issues.
  function focusCanvasInput() {
    const ta = document.getElementById('s1-canvas-input');
    if (ta) ta.focus({ preventScroll: true });
  }

  container.addEventListener('mousedown', (e) => {
    if (!_canvasMode || !state.doc) return;
    if (e.button !== 0) return;
    const target = e.target;
    if (!target.classList || !target.classList.contains('s1-canvas-page')) return;

    e.preventDefault(); // prevent focus from going to the canvas element
    _mouseDown = true;
    const hit = sceneHitTest(e.clientX, e.clientY, container);
    if (!hit || !hit.position) return;

    if (e.shiftKey) {
      modelSelection.extendFocus(hit.position);
      repaintSelection();
    } else {
      modelSelection.setFromHitTest(hit);
      repaintCaret();
    }

    focusCanvasInput();
  });

  container.addEventListener('mousemove', (e) => {
    if (!_mouseDown || !_canvasMode || !state.doc) return;
    const hit = sceneHitTest(e.clientX, e.clientY, container);
    if (!hit || !hit.position) return;

    modelSelection.extendFocus(hit.position);
    repaintSelection();
  });

  const upHandler = () => { _mouseDown = false; };
  container.addEventListener('mouseup', upHandler);
  document.addEventListener('mouseup', upHandler);

  // Double-click: select word
  container.addEventListener('dblclick', (e) => {
    if (!_canvasMode || !state.doc) return;
    const target = e.target;
    if (!target.classList || !target.classList.contains('s1-canvas-page')) return;

    const hit = sceneHitTest(e.clientX, e.clientY, container);
    if (!hit || !hit.position) return;

    try {
      const rangeStr = state.doc.word_boundary(JSON.stringify(hit.position));
      const range = JSON.parse(rangeStr);
      modelSelection.setRange(range);
      repaintSelection();
    } catch (_) {}

    focusCanvasInput();
  });
}

// -------------------------------------------------------
// Dirty-page repaint
// -------------------------------------------------------

/**
 * Re-render only the pages that changed after an edit.
 *
 * @param {{ start: number, end: number }} dirtyPages
 */
export function repaintDirtyPages(dirtyPages) {
  if (!_canvasMode || !state.doc) return;

  const container = document.getElementById('pageContainer');
  if (!container) return;

  // Full re-render for correctness. Individual page re-render is possible
  // but requires careful handling of page count changes.
  try {
    if (typeof state.doc.scene_summary === 'function') {
      renderDocumentScene(container);
    } else {
      renderDocumentCanvas(container);
    }
    // Re-focus the hidden textarea after repaint so typing continues to work
    const ta = document.getElementById('s1-canvas-input');
    if (ta) ta.focus({ preventScroll: true });
  } catch (e) {
    console.error('[canvas] repaintDirtyPages failed:', e);
  }
}

// -------------------------------------------------------
// Caret rendering
// -------------------------------------------------------

/**
 * Repaint the caret at the current selection position.
 * Fetches the caret rect from WASM and draws it on the appropriate page canvas.
 */
export function repaintCaret() {
  if (!_canvasMode || !state.doc) return;

  const posJson = modelSelection.getPositionJson();
  if (!posJson) { _stopCaretBlink(); return; }

  try {
    const rectStr = state.doc.caret_rect(posJson);
    const rect = JSON.parse(rectStr);
    updateCaretFromRect(rect);
    _startCaretBlink();
    _drawCaret();
  } catch (e) {
    console.error('[canvas] repaintCaret failed:', e);
  }
}

/**
 * Repaint selection highlight rectangles.
 */
export function repaintSelection() {
  if (!_canvasMode || !state.doc) return;

  const rangeJson = modelSelection.getRangeJson();
  if (!rangeJson || modelSelection.isCollapsed()) {
    _selectionRects = [];
    repaintCaret();
    return;
  }

  try {
    const rectsStr = state.doc.selection_rects(rangeJson);
    const rects = JSON.parse(rectsStr);
    updateSelectionFromRects(rects);
    _stopCaretBlink();

    // Redraw pages with selection highlight overlay
    _drawSelectionOverlay();
  } catch (e) {
    console.error('[canvas] repaintSelection failed:', e);
  }
}

function _startCaretBlink() {
  _stopCaretBlink();
  _caretVisible = true;
  _caretBlinkTimer = setInterval(() => {
    _caretVisible = !_caretVisible;
    _drawCaret();
  }, 530);
}

function _stopCaretBlink() {
  if (_caretBlinkTimer) {
    clearInterval(_caretBlinkTimer);
    _caretBlinkTimer = null;
  }
  _caretVisible = false;
}

function _drawCaret() {
  if (!_caretState) return;
  const entry = _canvasPages[_caretState.pageIndex];
  if (!entry) return;

  const ptToPx = 96 / 72;
  const dpr = window.devicePixelRatio || 1;
  const { ctx, canvas } = entry;

  // Use backing buffer: save page pixels once, restore on each blink toggle.
  // This avoids re-fetching the full scene from WASM on every 530ms blink.
  if (!entry._backingBuffer) {
    entry._backingBuffer = ctx.getImageData(0, 0, canvas.width, canvas.height);
  }
  // Restore the clean page (without caret)
  ctx.putImageData(entry._backingBuffer, 0, 0);

  if (_caretVisible) {
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.scale(ptToPx, ptToPx);
    ctx.fillStyle = '#000000';
    ctx.fillRect(
      _caretState.x,
      _caretState.y,
      _caretState.width,
      _caretState.height
    );
    ctx.restore();
  }
}

function _drawSelectionOverlay() {
  // Group rects by page
  const byPage = {};
  for (const r of _selectionRects) {
    if (!byPage[r.pageIndex]) byPage[r.pageIndex] = [];
    byPage[r.pageIndex].push(r);
  }

  const ptToPx = 96 / 72;

  const dpr = window.devicePixelRatio || 1;
  for (const [pi, rects] of Object.entries(byPage)) {
    const entry = _canvasPages[parseInt(pi)];
    if (!entry) continue;

    // Restore clean page from backing buffer (fast, no WASM call)
    if (entry._backingBuffer) {
      entry.ctx.putImageData(entry._backingBuffer, 0, 0);
    }

    // Draw selection rects
    const { ctx } = entry;
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.scale(ptToPx, ptToPx);
    ctx.fillStyle = 'rgba(66, 133, 244, 0.3)';
    for (const r of rects) {
      ctx.fillRect(r.x, r.y, r.width, r.height);
    }
    ctx.restore();
  }
}

/**
 * Re-render a single page canvas from a parsed scene object.
 */
function _redrawPageFromScene(entry, scene) {
  const { ctx, canvas } = entry;
  const dpr = window.devicePixelRatio || 1;
  const ptToPx = 96 / 72;
  const widthPx = parseFloat(canvas.style.width);
  const heightPx = parseFloat(canvas.style.height);

  // Clear
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, widthPx, heightPx);

  // Render scene items
  ctx.save();
  ctx.scale(ptToPx, ptToPx);
  if (scene.items) {
    renderSceneItems(ctx, scene.items);
  }
  ctx.restore();

  // Update backing buffer so caret blink uses the fresh page
  entry._backingBuffer = ctx.getImageData(0, 0, canvas.width, canvas.height);
}
