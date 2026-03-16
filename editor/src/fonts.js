/**
 * Font loading for s1 editor.
 *
 * Downloads font files from Google Fonts CDN and loads them into the
 * WasmFontDatabase for accurate document rendering.
 */

import { state } from './state.js';

// Google Fonts CSS API — returns CSS with @font-face rules pointing to TTF/WOFF2 URLs.
// We use a user-agent that triggers TTF URLs (some UAs get WOFF2 only).
const GOOGLE_FONTS_CSS_API = 'https://fonts.googleapis.com/css2?family=';

// Metric-compatible alternatives for Microsoft Office fonts not on Google Fonts.
// These maintain identical character widths so documents lay out correctly.
const FONT_ALTERNATIVES = {
  'Calibri': 'Carlito',
  'Cambria': 'Caladea',
  'Times New Roman': 'Tinos',
  'Arial': 'Arimo',
  'Courier New': 'Cousine',
};

// Common document fonts available on Google Fonts
const GOOGLE_FONTS_AVAILABLE = new Set([
  'Carlito', 'Caladea', 'Tinos', 'Arimo', 'Cousine',
  'Roboto', 'Open Sans', 'Lato', 'Montserrat', 'Noto Sans', 'Noto Serif',
  'Source Sans 3', 'EB Garamond', 'Merriweather', 'PT Sans', 'PT Serif',
  'Inconsolata', 'Georgia', 'Tahoma', 'Verdana', 'Trebuchet MS',
  'Garamond', 'Palatino', 'Century Gothic',
  'Noto Sans Arabic', 'Noto Naskh Arabic', 'Noto Sans Devanagari',
  'Noto Sans CJK SC', 'Noto Serif CJK SC',
]);

// Preload these fonts on startup — covers most Office documents
const PRELOAD_FONTS = [
  'Carlito',     // Calibri replacement
  'Caladea',     // Cambria replacement
  'Tinos',       // Times New Roman replacement
  'Arimo',       // Arial replacement
  'Noto Sans',   // General fallback
];

// Cache of already-loaded font families to avoid duplicate fetches
const loadedFonts = new Set();
const pendingLoads = new Map();

/** Font database instance (created from WASM) */
let fontDb = null;

/**
 * Initialize the font system.
 * Creates a WasmFontDatabase and preloads common fonts.
 */
export async function initFonts(wasm) {
  fontDb = new wasm.WasmFontDatabase();
  state.fontDb = fontDb;

  // Preload common fonts in parallel
  const promises = PRELOAD_FONTS.map(family => loadGoogleFont(family));
  await Promise.allSettled(promises);

  console.log(`[fonts] Preloaded ${fontDb.font_count()} font faces`);
}

/**
 * Get the font database instance.
 */
export function getFontDb() {
  return fontDb;
}

/**
 * Ensure all fonts used by a document are loaded.
 * Call this after opening a document, before rendering.
 *
 * @param {object} doc - WasmDocument instance
 * @returns {Promise<number>} Number of new fonts loaded
 */
export async function ensureDocumentFonts(doc) {
  if (!fontDb || !doc) return 0;

  let families;
  try {
    const json = doc.get_used_fonts();
    families = JSON.parse(json);
  } catch {
    return 0;
  }

  let loaded = 0;
  const promises = [];

  for (const family of families) {
    if (loadedFonts.has(family)) continue;
    if (fontDb.has_font(family)) {
      loadedFonts.add(family);
      continue;
    }

    // Try the font or its alternative
    const altFamily = FONT_ALTERNATIVES[family] || family;
    if (!loadedFonts.has(altFamily)) {
      promises.push(
        loadGoogleFont(altFamily).then(ok => {
          if (ok) loaded++;
        })
      );
    }
  }

  if (promises.length > 0) {
    await Promise.allSettled(promises);
  }

  return loaded;
}

/**
 * Download and load a font from Google Fonts.
 *
 * @param {string} family - Font family name (e.g. "Carlito")
 * @returns {Promise<boolean>} true if font was loaded successfully
 */
export async function loadGoogleFont(family) {
  if (loadedFonts.has(family)) return true;
  if (pendingLoads.has(family)) return pendingLoads.get(family);

  const promise = _doLoadGoogleFont(family);
  pendingLoads.set(family, promise);

  try {
    const result = await promise;
    return result;
  } finally {
    pendingLoads.delete(family);
  }
}

async function _doLoadGoogleFont(family) {
  if (!fontDb) return false;

  try {
    // Fetch CSS from Google Fonts API
    // Use a Chrome-like user agent to get TTF URLs
    const cssUrl = `${GOOGLE_FONTS_CSS_API}${encodeURIComponent(family)}:wght@100;200;300;400;500;600;700;800;900`;
    const cssResp = await fetch(cssUrl, {
      headers: {
        // Request TTF format (some user agents get WOFF2 which we can't parse)
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
      }
    });

    if (!cssResp.ok) {
      // Font not available on Google Fonts
      return false;
    }

    const css = await cssResp.text();

    // Extract font file URLs from @font-face rules
    const urlRegex = /url\(([^)]+)\)\s*format\(['"]?(truetype|opentype|woff2?)['"]?\)/g;
    const urls = [];
    let match;
    while ((match = urlRegex.exec(css)) !== null) {
      urls.push(match[1].replace(/['"]/g, ''));
    }

    // Also try plain url() without format
    if (urls.length === 0) {
      const plainUrlRegex = /url\(([^)]+\.(?:ttf|otf|woff2?))\)/g;
      while ((match = plainUrlRegex.exec(css)) !== null) {
        urls.push(match[1].replace(/['"]/g, ''));
      }
    }

    if (urls.length === 0) {
      return false;
    }

    // Download font files in parallel (limit to regular + bold for performance)
    const fontUrls = urls.slice(0, 4); // Regular, Bold, Italic, BoldItalic at most
    const fontPromises = fontUrls.map(async (url) => {
      try {
        const resp = await fetch(url);
        if (!resp.ok) return null;
        const buffer = await resp.arrayBuffer();
        return new Uint8Array(buffer);
      } catch {
        return null;
      }
    });

    const fontDatas = await Promise.all(fontPromises);
    let loadCount = 0;

    for (const data of fontDatas) {
      if (data && data.length > 0) {
        fontDb.load_font(data);
        loadCount++;
      }
    }

    if (loadCount > 0) {
      loadedFonts.add(family);
      return true;
    }

    return false;
  } catch (err) {
    console.warn(`[fonts] Failed to load "${family}":`, err.message);
    return false;
  }
}
