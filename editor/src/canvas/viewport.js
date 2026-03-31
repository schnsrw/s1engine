// Viewport manager for canvas-mode rendering.
//
// Tracks scroll position, zoom level, and which pages are visible.
// Provides coordinate transforms between world (page pt) and screen (CSS px).

import { state } from '../state.js';

const PT_TO_PX = 96 / 72;
const PAGE_GAP = 20; // px between pages

let _zoom = 1.0;
let _scrollTop = 0;
let _scrollLeft = 0;
let _containerWidth = 0;
let _containerHeight = 0;
let _pageSizes = []; // [{ widthPx, heightPx }]

/**
 * Update the viewport state from the scroll container.
 * Call on scroll, resize, and after rendering.
 *
 * @param {HTMLElement} container
 */
export function updateFromContainer(container) {
  if (!container) return;
  _scrollTop = container.scrollTop;
  _scrollLeft = container.scrollLeft;
  _containerWidth = container.clientWidth;
  _containerHeight = container.clientHeight;
}

/**
 * Set the page sizes (call after layout or re-render).
 *
 * @param {Array<{ width: number, height: number }>} sizes - Sizes in pt
 */
export function setPageSizes(sizes) {
  _pageSizes = sizes.map(s => ({
    widthPx: s.width * PT_TO_PX * _zoom,
    heightPx: s.height * PT_TO_PX * _zoom,
  }));
}

/**
 * Set the zoom level.
 * @param {number} zoom - 1.0 = 100%
 */
export function setZoom(zoom) {
  _zoom = Math.max(0.25, Math.min(4.0, zoom));
}

/** @returns {number} Current zoom level */
export function getZoom() { return _zoom; }

/**
 * Compute the range of pages currently visible in the viewport.
 * Returns { start, end } (end is exclusive).
 *
 * @returns {{ start: number, end: number }}
 */
export function getVisiblePageRange() {
  if (_pageSizes.length === 0) return { start: 0, end: 0 };

  const buffer = 1; // render one extra page above/below
  let pageTop = PAGE_GAP;
  let start = -1;
  let end = 0;

  for (let i = 0; i < _pageSizes.length; i++) {
    const pageBottom = pageTop + _pageSizes[i].heightPx;

    // Page is visible if it overlaps the viewport
    if (pageBottom >= _scrollTop && pageTop <= _scrollTop + _containerHeight) {
      if (start < 0) start = i;
      end = i + 1;
    }

    pageTop = pageBottom + PAGE_GAP;
  }

  // Apply buffer
  start = Math.max(0, (start < 0 ? 0 : start) - buffer);
  end = Math.min(_pageSizes.length, end + buffer);

  return { start, end };
}

/**
 * Convert a point in page coordinate space (pt) to screen CSS px.
 *
 * @param {number} pageIndex
 * @param {number} xPt - X in points, page-local
 * @param {number} yPt - Y in points, page-local
 * @returns {{ x: number, y: number }} Screen coordinates in CSS px
 */
export function worldToScreen(pageIndex, xPt, yPt) {
  let pageTop = PAGE_GAP;
  for (let i = 0; i < pageIndex && i < _pageSizes.length; i++) {
    pageTop += _pageSizes[i].heightPx + PAGE_GAP;
  }

  const pageW = _pageSizes[pageIndex]?.widthPx || 612 * PT_TO_PX * _zoom;
  const pageLeft = Math.max(PAGE_GAP, (_containerWidth - pageW) / 2);

  return {
    x: pageLeft + xPt * PT_TO_PX * _zoom - _scrollLeft,
    y: pageTop + yPt * PT_TO_PX * _zoom - _scrollTop,
  };
}

/**
 * Convert a screen CSS px point to page coordinate space (pt).
 *
 * @param {number} screenX
 * @param {number} screenY
 * @returns {{ pageIndex: number, xPt: number, yPt: number } | null}
 */
export function screenToWorld(screenX, screenY) {
  // Convert screen to scroll-adjusted container coords
  const cx = screenX + _scrollLeft;
  const cy = screenY + _scrollTop;

  let pageTop = PAGE_GAP;
  for (let i = 0; i < _pageSizes.length; i++) {
    const ps = _pageSizes[i];
    const pageLeft = Math.max(PAGE_GAP, (_containerWidth - ps.widthPx) / 2);
    const pageBottom = pageTop + ps.heightPx;
    const pageRight = pageLeft + ps.widthPx;

    if (cy >= pageTop && cy < pageBottom && cx >= pageLeft && cx < pageRight) {
      return {
        pageIndex: i,
        xPt: (cx - pageLeft) / (PT_TO_PX * _zoom),
        yPt: (cy - pageTop) / (PT_TO_PX * _zoom),
      };
    }

    pageTop = pageBottom + PAGE_GAP;
  }

  return null;
}

/**
 * Get the Y offset (in CSS px, scroll-adjusted) of a page's top edge.
 * Useful for scrolling a page into view.
 *
 * @param {number} pageIndex
 * @returns {number}
 */
export function pageTopPx(pageIndex) {
  let top = PAGE_GAP;
  for (let i = 0; i < pageIndex && i < _pageSizes.length; i++) {
    top += _pageSizes[i].heightPx + PAGE_GAP;
  }
  return top;
}
