/**
 * Font loading for Rudra Office.
 *
 * Loads font files from /fonts/ directory into both:
 * 1. The WASM FontDatabase (for Rust layout engine - text shaping & line breaking)
 * 2. The browser (via @font-face CSS - for canvas rendering)
 *
 * This ensures the SAME font metrics are used for layout computation
 * and visual rendering, keeping line breaks and page breaks in sync.
 */

import { state } from './state.js';

const LOCAL_FONTS_DIR = '/fonts/';

// ── Metric-compatible alternatives for Microsoft Office fonts ────
const FONT_ALTERNATIVES = {
  'Calibri': 'Carlito',
  'Cambria': 'Caladea',
  'Times New Roman': 'Tinos',
  'Arial': 'Arimo',
  'Courier New': 'Cousine',
  'Georgia': 'EB Garamond',
  'Verdana': 'Arimo',
  'Tahoma': 'Arimo',
  'Comic Sans MS': 'Carlito',
  'Impact': 'Arimo',
  'Trebuchet MS': 'Arimo',
  'Palatino Linotype': 'EB Garamond',
  'Book Antiqua': 'EB Garamond',
  'Garamond': 'EB Garamond',
  'Century Gothic': 'Montserrat',
  'Franklin Gothic': 'Source Sans 3',
  'Liberation Sans': 'Arimo',
  'Liberation Serif': 'Tinos',
  'Liberation Mono': 'Cousine',
};

// ── Font file registry: family → { regular, bold, italic, boldItalic } ──
const FONT_REGISTRY = {
  // MS Office metric-compatible (Croscore)
  'Carlito':    { regular: 'Carlito-Regular.ttf', bold: 'Carlito-Bold.ttf', italic: 'Carlito-Italic.ttf', boldItalic: 'Carlito-BoldItalic.ttf' },
  'Caladea':    { regular: 'Caladea-Regular.ttf', bold: 'Caladea-Bold.ttf', italic: 'Caladea-Italic.ttf', boldItalic: 'Caladea-BoldItalic.ttf' },
  'Tinos':      { regular: 'Tinos-Regular.ttf', bold: 'Tinos-Bold.ttf', italic: 'Tinos-Italic.ttf', boldItalic: 'Tinos-BoldItalic.ttf' },
  'Arimo':      { regular: 'Arimo-Regular.ttf', italic: 'Arimo-Italic.ttf' },
  'Cousine':    { regular: 'Cousine-Regular.ttf', bold: 'Cousine-Bold.ttf', italic: 'Cousine-Italic.ttf', boldItalic: 'Cousine-BoldItalic.ttf' },

  // Liberation (direct MS Office replacements)
  'Liberation Sans':  { regular: 'LiberationSans-Regular.ttf', bold: 'LiberationSans-Bold.ttf', italic: 'LiberationSans-Italic.ttf', boldItalic: 'LiberationSans-BoldItalic.ttf' },
  'Liberation Serif': { regular: 'LiberationSerif-Regular.ttf', bold: 'LiberationSerif-Bold.ttf', italic: 'LiberationSerif-Italic.ttf', boldItalic: 'LiberationSerif-BoldItalic.ttf' },
  'Liberation Mono':  { regular: 'LiberationMono-Regular.ttf', bold: 'LiberationMono-Bold.ttf', italic: 'LiberationMono-Italic.ttf', boldItalic: 'LiberationMono-BoldItalic.ttf' },

  // Common document fonts
  'Roboto':     { regular: 'Roboto-Regular.ttf', italic: 'Roboto-Italic.ttf' },
  'Noto Sans':  { regular: 'NotoSans-Regular.ttf', italic: 'NotoSans-Italic.ttf' },
  'Noto Serif': { regular: 'NotoSerif-Regular.ttf', italic: 'NotoSerif-Italic.ttf' },
  'Open Sans':  { regular: 'OpenSans-Regular.ttf', italic: 'OpenSans-Italic.ttf' },
  'Lato':       { regular: 'Lato-Regular.ttf', bold: 'Lato-Bold.ttf', italic: 'Lato-Italic.ttf', boldItalic: 'Lato-BoldItalic.ttf' },
  'Source Sans 3': { regular: 'SourceSans3-Regular.ttf', italic: 'SourceSans3-Italic.ttf' },
  'Merriweather': { regular: 'Merriweather-Regular.ttf', bold: 'Merriweather-Bold.ttf', italic: 'Merriweather-Italic.ttf' },
  'PT Sans':    { regular: 'PTSans-Regular.ttf', bold: 'PTSans-Bold.ttf', italic: 'PTSans-Italic.ttf' },
  'PT Serif':   { regular: 'PTSerif-Regular.ttf', bold: 'PTSerif-Bold.ttf', italic: 'PTSerif-Italic.ttf' },
  'EB Garamond': { regular: 'EBGaramond-Regular.ttf', italic: 'EBGaramond-Italic.ttf' },
  'Inter':      { regular: 'Inter-Regular.ttf', italic: 'Inter-Italic.ttf' },
  'Montserrat': { regular: 'Montserrat-Regular.ttf', italic: 'Montserrat-Italic.ttf' },
  'Playfair Display': { regular: 'PlayfairDisplay-Regular.ttf', italic: 'PlayfairDisplay-Italic.ttf' },

  // Internationalization
  'Noto Sans JP': { regular: 'NotoSansJP-Regular.ttf' },
  'Noto Sans SC': { regular: 'NotoSansSC-Regular.ttf' },
  'Noto Sans KR': { regular: 'NotoSansKR-Regular.ttf' },
  'Noto Sans Arabic': { regular: 'NotoSansArabic-Regular.ttf' },
  'Noto Sans Hebrew': { regular: 'NotoSansHebrew-Regular.ttf' },
  'Noto Sans Devanagari': { regular: 'NotoSansDevanagari-Regular.ttf' },
};

// Fonts to preload on startup (most common in DOCX documents)
const PRELOAD_FONTS = [
  'Carlito',     // Calibri replacement
  'Caladea',     // Cambria replacement
  'Tinos',       // Times New Roman replacement
  'Arimo',       // Arial replacement
  'Cousine',     // Courier New replacement
  'Noto Sans',   // Universal fallback
  'Noto Serif',  // Serif fallback
  'Roboto',      // Common modern font
  'Open Sans',   // Common web font
  'Lato',        // Common document font
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

  // Preload essential fonts in parallel (both WASM and browser)
  const promises = PRELOAD_FONTS.map(family => loadFontFamily(family));
  await Promise.allSettled(promises);

  console.log(`[fonts] Preloaded ${fontDb.font_count()} font faces for ${PRELOAD_FONTS.length} families`);
}

export function getFontDb() { return fontDb; }

/**
 * Ensure all fonts used by a document are loaded into both WASM and browser.
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
    // Try the exact family first, then alternatives
    const target = loadedFonts.has(family) ? null : (FONT_ALTERNATIVES[family] || family);
    if (target && !loadedFonts.has(target)) {
      promises.push(loadFontFamily(target).then(ok => { if (ok) loaded++; }));
    }
  }
  if (promises.length > 0) await Promise.allSettled(promises);
  return loaded;
}

/**
 * Load all variants (regular, bold, italic, boldItalic) of a font family
 * into both the WASM FontDatabase and the browser via @font-face.
 */
export async function loadFontFamily(family) {
  if (loadedFonts.has(family)) return true;
  if (pendingLoads.has(family)) return pendingLoads.get(family);

  const promise = _doLoadFamily(family);
  pendingLoads.set(family, promise);
  try {
    return await promise;
  } finally {
    pendingLoads.delete(family);
  }
}

async function _doLoadFamily(family) {
  if (!fontDb) return false;
  const entry = FONT_REGISTRY[family];
  if (!entry) {
    // Try loading as a single file: Family-Regular.ttf
    return _loadSingleFont(family, family.replace(/\s+/g, '') + '-Regular.ttf');
  }

  let anyLoaded = false;
  const variants = [
    { file: entry.regular, weight: '400', style: 'normal' },
    { file: entry.bold, weight: '700', style: 'normal' },
    { file: entry.italic, weight: '400', style: 'italic' },
    { file: entry.boldItalic, weight: '700', style: 'italic' },
  ];

  const promises = variants
    .filter(v => v.file)
    .map(v => _loadAndRegister(family, v.file, v.weight, v.style));

  const results = await Promise.allSettled(promises);
  for (const r of results) {
    if (r.status === 'fulfilled' && r.value) anyLoaded = true;
  }

  if (anyLoaded) loadedFonts.add(family);
  return anyLoaded;
}

async function _loadSingleFont(family, fileName) {
  const ok = await _loadAndRegister(family, fileName, '400', 'normal');
  if (ok) loadedFonts.add(family);
  return ok;
}

/**
 * Fetch a font file and register it in both:
 * 1. WASM FontDatabase (for Rust layout engine)
 * 2. Browser (via FontFace API for canvas rendering)
 */
async function _loadAndRegister(family, fileName, weight, style) {
  const url = LOCAL_FONTS_DIR + fileName;
  try {
    const resp = await fetch(url);
    if (!resp.ok) return false;
    const buffer = await resp.arrayBuffer();
    const bytes = new Uint8Array(buffer);

    // Register in WASM FontDatabase (for text shaping / line breaking)
    fontDb.load_font(bytes);

    // Register in browser (for canvas rendering) via FontFace API
    try {
      const face = new FontFace(family, buffer, { weight, style });
      await face.load();
      document.fonts.add(face);
    } catch (e) {
      // FontFace API might not be available; CSS @font-face fallback
      _addFontFaceCSS(family, url, weight, style);
    }

    return true;
  } catch (err) {
    return false;
  }
}

/** CSS @font-face fallback for browsers without FontFace API. */
function _addFontFaceCSS(family, url, weight, style) {
  const css = `@font-face { font-family: '${family}'; src: url('${url}') format('truetype'); font-weight: ${weight}; font-style: ${style}; font-display: swap; }`;
  const styleEl = document.createElement('style');
  styleEl.textContent = css;
  document.head.appendChild(styleEl);
}

// Legacy export for backward compatibility
export const loadLocalFont = loadFontFamily;
