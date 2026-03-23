/**
 * Font loading for Rudra Office.
 *
 * S4-23: Loads font files from the local /fonts/ directory to remove
 * dependencies on external CDNs (Google Fonts).
 */

import { state } from './state.js';

// Local fonts directory (configured in Vite/server to serve from editor/fonts)
const LOCAL_FONTS_DIR = '/fonts/';

// Metric-compatible alternatives for Microsoft Office fonts.
const FONT_ALTERNATIVES = {
  'Calibri': 'Carlito',
  'Cambria': 'Caladea',
  'Times New Roman': 'Tinos',
  'Arial': 'Arimo',
  'Courier New': 'Cousine',
};

// Map font family names to local file paths
const FONT_FILE_MAP = {
  'Carlito': 'Carlito-Regular.ttf',
  'Caladea': 'Caladea-Regular.ttf',
  'Tinos': 'Tinos-Regular.ttf',
  'Arimo': 'Arimo-Regular.ttf',
  'Cousine': 'Cousine-Regular.ttf',
  'Roboto': 'Roboto-Regular.ttf',
  'Noto Sans': 'NotoSans-Regular.ttf',
};

// Preload these fonts on startup
const PRELOAD_FONTS = [
  'Carlito',
  'Caladea',
  'Tinos',
  'Arimo',
  'Noto Sans',
];

const loadedFonts = new Set();
const pendingLoads = new Map();
let fontDb = null;

/**
 * Initialize the font system and preload common fonts.
 */
export async function initFonts(wasm) {
  fontDb = new wasm.WasmFontDatabase();
  state.fontDb = fontDb;

  const promises = PRELOAD_FONTS.map(family => loadLocalFont(family));
  await Promise.allSettled(promises);

  console.log(`[fonts] Preloaded ${fontDb.font_count()} self-hosted font faces`);
}

export function getFontDb() { return fontDb; }

/**
 * Ensure all fonts used by a document are loaded.
 */
export async function ensureDocumentFonts(doc) {
  if (!fontDb || !doc) return 0;
  let families;
  try {
    families = JSON.parse(doc.get_used_fonts());
  } catch { return 0; }

  let loaded = 0;
  const promises = [];
  for (const family of families) {
    if (loadedFonts.has(family)) continue;
    const altFamily = FONT_ALTERNATIVES[family] || family;
    if (!loadedFonts.has(altFamily)) {
      promises.push(loadLocalFont(altFamily).then(ok => { if (ok) loaded++; }));
    }
  }
  if (promises.length > 0) await Promise.allSettled(promises);
  return loaded;
}

/**
 * Load a font from the local /fonts/ directory.
 */
export async function loadLocalFont(family) {
  if (loadedFonts.has(family)) return true;
  if (pendingLoads.has(family)) return pendingLoads.get(family);

  const promise = _doLoadLocalFont(family);
  pendingLoads.set(family, promise);
  try {
    return await promise;
  } finally {
    pendingLoads.delete(family);
  }
}

async function _doLoadLocalFont(family) {
  if (!fontDb) return false;
  const fileName = FONT_FILE_MAP[family] || family.replace(/\s+/g, '-') + '-Regular.ttf';
  const url = LOCAL_FONTS_DIR + fileName;

  try {
    const resp = await fetch(url);
    if (!resp.ok) return false;
    const buffer = await resp.arrayBuffer();
    fontDb.load_font(new Uint8Array(buffer));
    loadedFonts.add(family);
    return true;
  } catch (err) {
    console.warn(`[fonts] Failed to load self-hosted "${family}":`, err.message);
    return false;
  }
}
