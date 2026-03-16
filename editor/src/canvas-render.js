// Canvas-based document renderer for pixel-accurate glyph placement.
//
// Provides an alternative to DOM-based rendering by drawing directly onto
// HTML5 Canvas elements using the structured layout JSON from the WASM engine.

import { state, $ } from './state.js';

// -------------------------------------------------------
// Module state
// -------------------------------------------------------

let _canvasMode = false;
let _canvasPages = []; // Array of { canvas, ctx, pageData } per page
let _lastLayoutJson = null; // Cached layout JSON for hit testing

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

// -------------------------------------------------------
// Hit testing
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
}
