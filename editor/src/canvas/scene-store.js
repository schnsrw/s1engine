// Page scene cache for canvas-mode rendering.
//
// Caches WASM page_scene() results keyed by (pageIndex, revision)
// so that unchanged pages are not re-fetched after every edit.

import { state } from '../state.js';

const _cache = new Map(); // Map<pageIndex, { revision: number, scene: object }>

/**
 * Get or fetch the scene for a page.
 *
 * @param {number} pageIndex
 * @param {number} revision - document/layout revision to check staleness
 * @returns {object|null} Parsed scene JSON, or null on error
 */
export function getOrFetch(pageIndex, revision) {
  const cached = _cache.get(pageIndex);
  if (cached && cached.revision === revision) {
    return cached.scene;
  }

  const doc = state.doc;
  if (!doc || typeof doc.page_scene !== 'function') return null;

  try {
    const jsonStr = doc.page_scene(pageIndex);
    const scene = JSON.parse(jsonStr);
    if (scene.error) return null;
    _cache.set(pageIndex, { revision, scene });
    return scene;
  } catch (e) {
    console.error(`[scene-store] Failed to fetch page ${pageIndex}:`, e);
    return null;
  }
}

/**
 * Invalidate cached scenes for a range of pages.
 * Call after an edit that changed pages [startPage .. endPage).
 *
 * @param {number} startPage
 * @param {number} endPage
 */
export function invalidate(startPage, endPage) {
  for (let i = startPage; i < endPage; i++) {
    _cache.delete(i);
  }
}

/**
 * Invalidate all cached scenes.
 */
export function invalidateAll() {
  _cache.clear();
}

/**
 * Get the number of cached scenes (for diagnostics).
 * @returns {number}
 */
export function cacheSize() {
  return _cache.size;
}
