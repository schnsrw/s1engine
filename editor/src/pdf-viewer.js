// PDF Viewer — wraps PDF.js for rendering, navigation, zoom, and text selection
import * as pdfjsLib from 'pdfjs-dist';
import { state, $ } from './state.js';

// Configure PDF.js worker
try {
  const workerUrl = new URL('pdfjs-dist/build/pdf.worker.mjs', import.meta.url);
  pdfjsLib.GlobalWorkerOptions.workerSrc = workerUrl.toString();
} catch (_) {
  pdfjsLib.GlobalWorkerOptions.workerSrc =
    `https://cdnjs.cloudflare.com/ajax/libs/pdf.js/${pdfjsLib.version}/pdf.worker.min.mjs`;
}

// Standard font data URL for PDF.js (fixes "Ensure standardFontDataUrl is provided" warning)
const STANDARD_FONT_DATA_URL = `https://cdnjs.cloudflare.com/ajax/libs/pdf.js/${pdfjsLib.version}/standard_fonts/`;

/**
 * PdfViewer renders PDF pages into a scrollable container using PDF.js.
 * Each page gets: canvas (rendered content) + text layer (selectable text) + overlay layer (annotations).
 */
export class PdfViewer {
  /** @param {HTMLElement} container — the element to render pages into */
  constructor(container) {
    this._container = container;
    this._pdfDoc = null;
    this._pages = [];
    this._scale = 1.0;
    this._currentPage = 1;
    this._rendering = false;
    this._pendingRender = null;
    this._resizeObserver = null;
    this._onScrollCallback = null; // external callback for scroll tracking
  }

  /** Load a PDF from raw bytes and render all pages in continuous scroll mode. */
  async open(pdfBytes) {
    this.destroy();
    const data = pdfBytes instanceof Uint8Array ? pdfBytes : new Uint8Array(pdfBytes);
    const loadingTask = pdfjsLib.getDocument({
      data,
      standardFontDataUrl: STANDARD_FONT_DATA_URL,
      verbosity: 0, // suppress warnings
    });
    this._pdfDoc = await loadingTask.promise;
    this._currentPage = 1;
    await this.renderAllPages();
    this._setupScrollTracking();
    this._setupResizeObserver();
  }

  /** Register a callback for page change events (e.g. sidebar thumbnail sync). */
  onPageChange(callback) {
    this._onScrollCallback = callback;
  }

  /** Render all pages in continuous scroll layout. */
  async renderAllPages() {
    if (!this._pdfDoc) return;
    this._container.innerHTML = '';
    this._pages = [];

    const pageCount = this._pdfDoc.numPages;
    for (let i = 1; i <= pageCount; i++) {
      const wrapper = this._createPageWrapper(i);
      this._container.appendChild(wrapper.element);
      this._pages.push(wrapper);
    }

    await this._renderVisiblePages();
  }

  /** Render a single page by number (1-indexed). */
  async renderPage(pageNum) {
    if (!this._pdfDoc || pageNum < 1 || pageNum > this._pdfDoc.numPages) return;
    const pageInfo = this._pages[pageNum - 1];
    if (!pageInfo || pageInfo.rendered) return;
    await this._renderPageInternal(pageNum, pageInfo);
  }

  /** Set zoom scale and re-render. */
  async setZoom(scaleOrMode) {
    if (!this._pdfDoc) return;
    let newScale;
    if (scaleOrMode === 'fit-page') {
      const page = await this._pdfDoc.getPage(1);
      const viewport = page.getViewport({ scale: 1.0 });
      const containerHeight = this._container.clientHeight - 40;
      const containerWidth = this._container.clientWidth - 40;
      newScale = Math.min(containerWidth / viewport.width, containerHeight / viewport.height);
    } else if (scaleOrMode === 'fit-width') {
      const page = await this._pdfDoc.getPage(1);
      const viewport = page.getViewport({ scale: 1.0 });
      const containerWidth = this._container.clientWidth - 40;
      newScale = containerWidth / viewport.width;
    } else {
      newScale = parseFloat(scaleOrMode);
      if (isNaN(newScale) || newScale <= 0) return;
    }
    this._scale = newScale;
    state.pdfZoom = newScale;

    // Mark all as unrendered, then only render visible ones
    for (const p of this._pages) {
      p.rendered = false;
      // Pre-size wrappers so scroll position is preserved
      if (p.element.style.width) {
        const oldW = parseFloat(p.element.style.width);
        const oldH = parseFloat(p.element.style.height);
        if (oldW > 0 && oldH > 0) {
          const ratio = oldH / oldW;
          // Estimate new size (will be corrected on render)
          p.element.style.height = (parseFloat(p.element.style.width) * ratio) + 'px';
        }
      }
    }
    await this._renderVisiblePages();
  }

  nextPage() {
    if (!this._pdfDoc) return;
    this._scrollToPage(Math.min(this._currentPage + 1, this._pdfDoc.numPages));
  }

  prevPage() {
    if (!this._pdfDoc) return;
    this._scrollToPage(Math.max(this._currentPage - 1, 1));
  }

  goToPage(pageNum) {
    if (!this._pdfDoc) return;
    this._scrollToPage(Math.max(1, Math.min(pageNum, this._pdfDoc.numPages)));
  }

  getPageCount() {
    return this._pdfDoc ? this._pdfDoc.numPages : 0;
  }

  getCurrentPage() {
    return this._currentPage;
  }

  getPdfDocument() {
    return this._pdfDoc;
  }

  getOverlayLayer(pageNum) {
    const p = this._pages[pageNum - 1];
    return p ? p.overlayLayer : null;
  }

  getDrawingCanvas(pageNum) {
    const p = this._pages[pageNum - 1];
    return p ? p.drawingCanvas : null;
  }

  async getPageDimensions(pageNum) {
    if (!this._pdfDoc) return null;
    const page = await this._pdfDoc.getPage(pageNum);
    const viewport = page.getViewport({ scale: this._scale });
    return { width: viewport.width, height: viewport.height };
  }

  destroy() {
    if (this._resizeObserver) {
      this._resizeObserver.disconnect();
      this._resizeObserver = null;
    }
    if (this._scrollHandler) {
      this._container.removeEventListener('scroll', this._scrollHandler);
      this._scrollHandler = null;
    }
    if (this._pdfDoc) {
      this._pdfDoc.destroy();
      this._pdfDoc = null;
    }
    this._container.innerHTML = '';
    this._pages = [];
    this._onScrollCallback = null;
  }

  // ─── Private Methods ──────────────────────────────

  _createPageWrapper(pageNum) {
    const el = document.createElement('div');
    el.className = 'pdf-page pdf-page-loading';
    el.dataset.pageNum = pageNum;
    // Estimated size (will be corrected on render). US Letter at current scale.
    el.style.width = Math.floor(612 * this._scale) + 'px';
    el.style.height = Math.floor(792 * this._scale) + 'px';

    const canvas = document.createElement('canvas');
    canvas.className = 'pdf-canvas';

    const textLayer = document.createElement('div');
    textLayer.className = 'pdf-text-layer';

    const annotationLayer = document.createElement('div');
    annotationLayer.className = 'pdf-annotation-layer';

    const drawingCanvas = document.createElement('canvas');
    drawingCanvas.className = 'pdf-drawing-layer';

    const overlayLayer = document.createElement('div');
    overlayLayer.className = 'pdf-overlay-layer';

    el.appendChild(canvas);
    el.appendChild(textLayer);
    el.appendChild(annotationLayer);
    el.appendChild(drawingCanvas);
    el.appendChild(overlayLayer);

    return {
      element: el,
      canvas,
      textLayer,
      annotationLayer,
      drawingCanvas,
      overlayLayer,
      rendered: false,
    };
  }

  async _renderPageInternal(pageNum, pageInfo) {
    if (!this._pdfDoc) return;
    const page = await this._pdfDoc.getPage(pageNum);
    const viewport = page.getViewport({ scale: this._scale });

    // Size the wrapper
    pageInfo.element.style.width = viewport.width + 'px';
    pageInfo.element.style.height = viewport.height + 'px';

    // Render to canvas (hi-DPI)
    const canvas = pageInfo.canvas;
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.floor(viewport.width * dpr);
    canvas.height = Math.floor(viewport.height * dpr);
    canvas.style.width = viewport.width + 'px';
    canvas.style.height = viewport.height + 'px';
    ctx.scale(dpr, dpr);

    await page.render({ canvasContext: ctx, viewport }).promise;

    // Setup drawing canvas (same size)
    const drawCanvas = pageInfo.drawingCanvas;
    drawCanvas.width = Math.floor(viewport.width * dpr);
    drawCanvas.height = Math.floor(viewport.height * dpr);
    drawCanvas.style.width = viewport.width + 'px';
    drawCanvas.style.height = viewport.height + 'px';
    const drawCtx = drawCanvas.getContext('2d');
    drawCtx.scale(dpr, dpr);

    // Render text layer for selectable text
    const textContent = await page.getTextContent();
    pageInfo.textLayer.innerHTML = '';
    pageInfo.textLayer.style.width = viewport.width + 'px';
    pageInfo.textLayer.style.height = viewport.height + 'px';

    for (const item of textContent.items) {
      if (!item.str) continue;
      const tx = pdfjsLib.Util.transform(viewport.transform, item.transform);
      const fontHeight = Math.sqrt(tx[0] * tx[0] + tx[1] * tx[1]);
      const span = document.createElement('span');
      span.textContent = item.str;
      span.style.position = 'absolute';
      span.style.left = tx[4] + 'px';
      span.style.top = (viewport.height - tx[5]) + 'px';
      span.style.fontSize = fontHeight + 'px';
      // Sanitize font name (strip non-safe characters)
      const safeFontName = (item.fontName || '').replace(/[^a-zA-Z0-9\s\-_,]/g, '');
      span.style.fontFamily = safeFontName || 'sans-serif';
      span.style.transformOrigin = '0 0';
      // item.width is in PDF units — scale by viewport scale factor
      if (item.width) {
        span.style.width = (item.width * viewport.scale) + 'px';
      }
      span.style.height = fontHeight + 'px';
      pageInfo.textLayer.appendChild(span);
    }

    // Size overlay layer
    pageInfo.overlayLayer.style.width = viewport.width + 'px';
    pageInfo.overlayLayer.style.height = viewport.height + 'px';

    pageInfo.rendered = true;
    pageInfo.element.classList.remove('pdf-page-loading');
    const event = new CustomEvent('pdfPageRendered', { detail: { pageNum, width: viewport.width, height: viewport.height } });
    document.dispatchEvent(event);
  }

  async _rerenderAllPages() {
    if (this._rendering) {
      this._pendingRender = true;
      return;
    }
    this._rendering = true;

    for (let i = 0; i < this._pages.length; i++) {
      const p = this._pages[i];
      if (!p.rendered) {
        await this._renderPageInternal(i + 1, p);
      }
    }

    this._rendering = false;
    if (this._pendingRender) {
      this._pendingRender = false;
      await this._rerenderAllPages();
    }
  }

  async _renderVisiblePages() {
    if (!this._pdfDoc || this._rendering) return;
    this._rendering = true;

    const containerRect = this._container.getBoundingClientRect();
    const buffer = 300;

    for (let i = 0; i < this._pages.length; i++) {
      const p = this._pages[i];
      const rect = p.element.getBoundingClientRect();
      const visible = rect.bottom >= containerRect.top - buffer &&
                      rect.top <= containerRect.bottom + buffer;
      if (visible && !p.rendered) {
        await this._renderPageInternal(i + 1, p);
      }
    }

    this._rendering = false;
  }

  _setupScrollTracking() {
    let ticking = false;
    this._scrollHandler = () => {
      if (ticking) return;
      ticking = true;
      requestAnimationFrame(() => {
        this._updateCurrentPage();
        this._renderVisiblePages();
        ticking = false;
      });
    };
    this._container.addEventListener('scroll', this._scrollHandler, { passive: true });
  }

  _updateCurrentPage() {
    const containerRect = this._container.getBoundingClientRect();
    const containerMid = containerRect.top + containerRect.height / 2;
    let closest = 1;
    let closestDist = Infinity;

    for (let i = 0; i < this._pages.length; i++) {
      const rect = this._pages[i].element.getBoundingClientRect();
      const pageMid = rect.top + rect.height / 2;
      const dist = Math.abs(pageMid - containerMid);
      if (dist < closestDist) {
        closestDist = dist;
        closest = i + 1;
      }
    }

    if (this._currentPage !== closest) {
      this._currentPage = closest;
      state.pdfCurrentPage = closest;
      // Update page info display
      const pageInfo = $('pdfPageInfo');
      if (pageInfo) {
        pageInfo.textContent = `${closest} / ${this._pdfDoc.numPages}`;
      }
      const statusInfo = $('statusInfo');
      if (statusInfo && state.currentView === 'pdf') {
        statusInfo.textContent = `Page ${closest} of ${this._pdfDoc.numPages}`;
      }
      // Notify external listeners (e.g. sidebar thumbnails)
      if (this._onScrollCallback) {
        this._onScrollCallback(closest);
      }
    }
  }

  _scrollToPage(pageNum) {
    const p = this._pages[pageNum - 1];
    if (!p) return;
    p.element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    this._currentPage = pageNum;
    state.pdfCurrentPage = pageNum;
    const pageInfo = $('pdfPageInfo');
    if (pageInfo) {
      pageInfo.textContent = `${pageNum} / ${this._pdfDoc.numPages}`;
    }
    if (this._onScrollCallback) {
      this._onScrollCallback(pageNum);
    }
  }

  _setupResizeObserver() {
    if (typeof ResizeObserver === 'undefined') return;
    this._resizeObserver = new ResizeObserver(() => {
      // Auto re-render for fit modes would go here
    });
    this._resizeObserver.observe(this._container);
  }
}
