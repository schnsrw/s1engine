// Properties Panel — contextual right sidebar for element formatting
// Shows paragraph, image, table, or section properties based on selection context
import { state, $ } from './state.js';
import { renderDocument, renderSmart, syncAllText } from './render.js';
import { updateUndoRedo } from './toolbar.js';
import { getActiveNodeId } from './selection.js';
import { broadcastOp } from './collab.js';

// ─── Panel State ─────────────────────────────────
let _currentContext = 'none'; // 'paragraph' | 'image' | 'table' | 'section' | 'none'
let _debounceTimer = null;
let _selectionListener = null;
let _forcedTab = null; // null = auto, 'section' = force section view

// ─── Initialization ──────────────────────────────
export function initPropertiesPanel() {
  // Toggle from menubar
  const toggleBtn = $('btnPropsPanel');
  if (toggleBtn) {
    toggleBtn.addEventListener('click', () => {
      // Route to spreadsheet properties panel when in sheet mode
      if (state.currentView === 'spreadsheet' && state.spreadsheetView) {
        state.spreadsheetView.showPropertiesPanel();
        return;
      }
      togglePropertiesPanel();
    });
  }

  // Toggle from View menu
  const menuToggle = $('menuShowPropsPanel');
  if (menuToggle) {
    menuToggle.addEventListener('click', () => {
      // Close the app menu dropdown
      document.querySelectorAll('.app-menu-item').forEach(m => {
        m.classList.remove('open');
        const btn = m.querySelector('.app-menu-btn');
        if (btn) btn.setAttribute('aria-expanded', 'false');
      });
      togglePropertiesPanel();
    });
  }

  // Close button
  const closeBtn = $('propsPanelClose');
  if (closeBtn) {
    closeBtn.addEventListener('click', () => {
      hidePropertiesPanel();
    });
  }

  // Listen to selection changes when panel is open
  _selectionListener = () => {
    const panel = $('propsPanel');
    if (!panel || !panel.classList.contains('show')) return;
    clearTimeout(_debounceTimer);
    _debounceTimer = setTimeout(() => {
      // ED2-26: Re-check visibility before updating — panel may have been
      // hidden between the debounce scheduling and the callback firing.
      const p = $('propsPanel');
      if (!p || !p.classList.contains('show')) return;
      detectContextAndUpdate();
    }, 150);
  };

  document.addEventListener('selectionchange', _selectionListener);

  // Listen for image selection/deselection custom event
  document.addEventListener('s1-selection-context-change', () => {
    const panel = $('propsPanel');
    if (!panel || !panel.classList.contains('show')) return;
    // Small delay to let state settle
    clearTimeout(_debounceTimer);
    _debounceTimer = setTimeout(() => {
      // ED2-26: Re-check visibility before updating
      const p = $('propsPanel');
      if (!p || !p.classList.contains('show')) return;
      detectContextAndUpdate();
    }, 50);
  });

  // Tab switching
  const tabContainer = $('propsTabs');
  if (tabContainer) {
    tabContainer.querySelectorAll('.props-tab').forEach(tab => {
      tab.addEventListener('click', () => {
        tabContainer.querySelectorAll('.props-tab').forEach(t => t.classList.remove('active'));
        tab.classList.add('active');
        const tabName = tab.dataset.propsTab;
        if (tabName === 'section') {
          // Force section view
          _forcedTab = 'section';
          showSectionContext();
        } else {
          // Auto mode
          _forcedTab = null;
          detectContextAndUpdate();
        }
      });
    });
  }

  // Wire up paragraph property controls
  initParagraphControls();
  initImageControls();
  initTableControls();
  initSectionControls();
}

export function togglePropertiesPanel() {
  const panel = $('propsPanel');
  if (!panel) return;
  panel.classList.toggle('show');
  if (panel.classList.contains('show')) {
    // ED2-10: Re-add selectionchange listener when panel is shown
    if (_selectionListener) {
      document.removeEventListener('selectionchange', _selectionListener);
      document.addEventListener('selectionchange', _selectionListener);
    }
    detectContextAndUpdate();
  } else {
    // ED2-10: Remove selectionchange listener when panel is hidden
    if (_selectionListener) {
      document.removeEventListener('selectionchange', _selectionListener);
    }
  }
}

export function showPropertiesPanel() {
  const panel = $('propsPanel');
  if (!panel) return;
  panel.classList.add('show');
  // ED2-10: Re-add selectionchange listener when panel is shown
  if (_selectionListener) {
    document.removeEventListener('selectionchange', _selectionListener); // prevent duplicates
    document.addEventListener('selectionchange', _selectionListener);
  }
  detectContextAndUpdate();
}

export function hidePropertiesPanel() {
  const panel = $('propsPanel');
  if (!panel) return;
  panel.classList.remove('show');
  // ED2-10: Remove selectionchange listener when panel is hidden to prevent leak
  if (_selectionListener) {
    document.removeEventListener('selectionchange', _selectionListener);
  }
}

function showSectionContext() {
  const sections = ['propsParagraph', 'propsImage', 'propsTable', 'propsSection'];
  sections.forEach(id => {
    const el = $(id);
    if (el) el.style.display = 'none';
  });
  if ($('propsSection')) $('propsSection').style.display = '';
  const contextLabel = $('propsContextLabel');
  if (contextLabel) contextLabel.textContent = 'Page Layout';
  populateSectionProps();
}

// ─── Context Detection ───────────────────────────
export function detectContextAndUpdate() {
  if (!state.doc) return;
  const panel = $('propsPanel');
  if (!panel || !panel.classList.contains('show')) return;

  // If user forced the "Page" tab, always show section properties
  if (_forcedTab === 'section') {
    showSectionContext();
    return;
  }

  const newContext = detectContext();
  _currentContext = newContext;

  // Show/hide sections
  const sections = ['propsParagraph', 'propsImage', 'propsTable', 'propsSection'];
  sections.forEach(id => {
    const el = $(id);
    if (el) el.style.display = 'none';
  });

  // P4 Step 5: Always populate document info (macros, etc.) when panel is open
  populateDocInfo();

  const contextLabel = $('propsContextLabel');

  switch (newContext) {
    case 'image':
      if ($('propsImage')) $('propsImage').style.display = '';
      if (contextLabel) contextLabel.textContent = 'Image Properties';
      populateImageProps();
      break;
    case 'table':
      if ($('propsTable')) $('propsTable').style.display = '';
      if (contextLabel) contextLabel.textContent = 'Table Properties';
      populateTableProps();
      break;
    case 'section':
      if ($('propsSection')) $('propsSection').style.display = '';
      if (contextLabel) contextLabel.textContent = 'Section Properties';
      populateSectionProps();
      break;
    case 'paragraph':
    default:
      if ($('propsParagraph')) $('propsParagraph').style.display = '';
      if (contextLabel) contextLabel.textContent = 'Paragraph Properties';
      populateParagraphProps();
      break;
  }
}

function detectContext() {
  // Check if an image is selected
  if (state.selectedImg) {
    return 'image';
  }

  // Check selection for table or paragraph context
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return 'paragraph';

  let node = sel.anchorNode;
  while (node && node !== document.body) {
    if (node.nodeType === 1) {
      // Check table context
      if (node.tagName === 'TD' || node.tagName === 'TH') {
        return 'table';
      }
      if (node.tagName === 'TABLE') {
        return 'table';
      }
    }
    node = node.parentNode;
  }

  return 'paragraph';
}

// ─── Paragraph Properties ────────────────────────
function initParagraphControls() {
  // Alignment buttons
  ['Left', 'Center', 'Right', 'Justify'].forEach(align => {
    const btn = $('ppAlign' + align);
    if (btn) {
      btn.addEventListener('click', () => {
        applyParagraphAlignment(align.toLowerCase());
      });
    }
  });

  // Indentation inputs
  ['ppIndentLeft', 'ppIndentRight', 'ppIndentFirstLine'].forEach(id => {
    const input = $(id);
    if (input) {
      input.addEventListener('change', () => {
        applyParagraphIndent(id, parseFloat(input.value) || 0);
      });
    }
  });

  // Spacing inputs
  ['ppSpaceBefore', 'ppSpaceAfter'].forEach(id => {
    const input = $(id);
    if (input) {
      input.addEventListener('change', () => {
        applyParagraphSpacing(id, parseFloat(input.value) || 0);
      });
    }
  });

  // Line spacing
  const lsSelect = $('ppLineSpacing');
  if (lsSelect) {
    lsSelect.addEventListener('change', () => {
      applyLineSpacing(lsSelect.value);
    });
  }

  // Keep options
  const kwn = $('ppKeepWithNext');
  if (kwn) {
    kwn.addEventListener('change', () => {
      applyParagraphKeep('keepWithNext', kwn.checked);
    });
  }

  const klt = $('ppKeepTogether');
  if (klt) {
    klt.addEventListener('change', () => {
      applyParagraphKeep('keepLinesTogether', klt.checked);
    });
  }
}

function populateParagraphProps() {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;

  try {
    const fmt = JSON.parse(state.doc.get_formatting_json(nodeId));

    // Alignment
    const align = fmt.alignment || 'left';
    ['Left', 'Center', 'Right', 'Justify'].forEach(a => {
      const btn = $('ppAlign' + a);
      if (btn) btn.classList.toggle('active', align === a.toLowerCase());
    });

    // Indentation — read from WASM formatting JSON (in points, convert to inches)
    const indentLeftPt = parseFloat(fmt.indentLeft) || 0;
    const indentRightPt = parseFloat(fmt.indentRight) || 0;
    const indentFirstLinePt = parseFloat(fmt.indentFirstLine) || 0;

    const ppIL = $('ppIndentLeft');
    const ppIR = $('ppIndentRight');
    const ppIFL = $('ppIndentFirstLine');
    if (ppIL) ppIL.value = (indentLeftPt / 72).toFixed(2);
    if (ppIR) ppIR.value = (indentRightPt / 72).toFixed(2);
    if (ppIFL) ppIFL.value = (indentFirstLinePt / 72).toFixed(2);

    // Spacing — read from WASM formatting JSON (already in points)
    const spaceBefore = parseFloat(fmt.spacingBefore) || 0;
    const spaceAfter = parseFloat(fmt.spacingAfter) || 0;
    const ppSB = $('ppSpaceBefore');
    const ppSA = $('ppSpaceAfter');
    if (ppSB) ppSB.value = Math.round(spaceBefore);
    if (ppSA) ppSA.value = Math.round(spaceAfter);

    // Line spacing — read from WASM formatting JSON
    let lineSpacing = '1.15';
    if (fmt.lineSpacing) {
      const ls = fmt.lineSpacing;
      // Match exact values to dropdown options
      if (ls === '1.0' || ls === '1.00') lineSpacing = '1.0';
      else if (ls === '1.15') lineSpacing = '1.15';
      else if (ls === '1.5' || ls === '1.50') lineSpacing = '1.5';
      else if (ls === '2.0' || ls === '2.00') lineSpacing = '2.0';
      else if (ls === '2.5' || ls === '2.50') lineSpacing = '2.5';
      else if (ls === '3.0' || ls === '3.00') lineSpacing = '3.0';
      else {
        // Try numeric match
        const numVal = parseFloat(ls);
        if (!isNaN(numVal)) {
          if (numVal <= 1.05) lineSpacing = '1.0';
          else if (numVal <= 1.2) lineSpacing = '1.15';
          else if (numVal <= 1.55) lineSpacing = '1.5';
          else if (numVal <= 2.05) lineSpacing = '2.0';
          else if (numVal <= 2.55) lineSpacing = '2.5';
          else lineSpacing = '3.0';
        }
      }
    }
    const ppLS = $('ppLineSpacing');
    if (ppLS) ppLS.value = lineSpacing;

  } catch (e) {
    // Silently fail — don't break the UI
    console.debug('Properties panel: paragraph populate error', e);
  }
}

function applyParagraphAlignment(align) {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  syncAllText();
  try {
    state.doc.set_alignment(nodeId, align);
    broadcastOp({ action: 'setAlignment', nodeId, alignment: align });
    renderSmart(nodeId);
    updateUndoRedo();
    populateParagraphProps();
  } catch (e) { console.error('props panel: alignment', e); }
}

function applyParagraphIndent(inputId, value) {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  syncAllText();

  // Convert inches to points (1in = 72pt)
  const pts = value * 72;

  let indentType;
  if (inputId === 'ppIndentLeft') indentType = 'left';
  else if (inputId === 'ppIndentRight') indentType = 'right';
  else indentType = 'firstLine';

  try {
    state.doc.set_indent(nodeId, indentType, pts);
    broadcastOp({ action: 'setIndent', nodeId, side: indentType, value: pts });
    renderSmart(nodeId);
    updateUndoRedo();
  } catch (e) { console.error('props panel: indent', e); }
}

function applyParagraphSpacing(inputId, value) {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  syncAllText();

  // value is in points
  const spacingType = inputId === 'ppSpaceBefore' ? 'before' : 'after';

  try {
    state.doc.set_paragraph_spacing(nodeId, spacingType, value);
    broadcastOp({ action: 'setParagraphSpacing', nodeId, spacingType, value });
    renderSmart(nodeId);
    updateUndoRedo();
  } catch (e) {
    console.error('props panel: spacing', e);
  }
}

function applyLineSpacing(value) {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  syncAllText();

  try {
    state.doc.set_line_spacing(nodeId, value);
    broadcastOp({ action: 'setLineSpacing', nodeId, value });
    renderSmart(nodeId);
    updateUndoRedo();
    populateParagraphProps();
  } catch (e) { console.error('props panel: line spacing', e); }
}

function applyParagraphKeep(keepType, enabled) {
  if (!state.doc) return;
  const nodeId = getActiveNodeId();
  if (!nodeId) return;
  syncAllText();
  try {
    state.doc.set_paragraph_keep(nodeId, keepType, enabled);
    broadcastOp({ action: 'setParagraphKeep', nodeId, keepType, enabled });
    renderSmart(nodeId);
    updateUndoRedo();
  } catch (e) {
    console.debug('props panel: keep option error', e);
  }
}

// ─── Image Properties ────────────────────────────
function initImageControls() {
  // Width / Height
  const ipW = $('ipWidth');
  const ipH = $('ipHeight');
  const ipLock = $('ipLockAspect');

  if (ipW) {
    ipW.addEventListener('change', () => {
      applyImageSize('width', parseFloat(ipW.value) || 100);
    });
  }
  if (ipH) {
    ipH.addEventListener('change', () => {
      applyImageSize('height', parseFloat(ipH.value) || 100);
    });
  }

  // Alt text
  const ipAlt = $('ipAltText');
  if (ipAlt) {
    ipAlt.addEventListener('change', () => {
      applyImageAltText(ipAlt.value);
    });
  }

  // Wrap mode
  const ipWrap = $('ipWrapMode');
  if (ipWrap) {
    ipWrap.addEventListener('change', () => {
      applyImageWrapMode(ipWrap.value);
    });
  }
}

function populateImageProps() {
  if (!state.doc || !state.selectedImg) return;

  const img = state.selectedImg;
  const nodeEl = img.closest('[data-node-id]');
  if (!nodeEl) return;

  const nodeId = nodeEl.dataset.nodeId;

  // Dimensions in pixels
  const ipW = $('ipWidth');
  const ipH = $('ipHeight');
  if (ipW) ipW.value = Math.round(img.offsetWidth || img.naturalWidth || 0);
  if (ipH) ipH.value = Math.round(img.offsetHeight || img.naturalHeight || 0);

  // Alt text
  const ipAlt = $('ipAltText');
  if (ipAlt) ipAlt.value = img.getAttribute('alt') || '';

  // Wrap mode
  const imgNodeId = findImageNodeId(nodeId);
  if (imgNodeId) {
    try {
      const mode = state.doc.get_image_wrap_mode(imgNodeId) || 'inline';
      const ipWrap = $('ipWrapMode');
      if (ipWrap) ipWrap.value = mode;
    } catch (_) {}
  }
}

function applyImageSize(dimension, value) {
  if (!state.doc || !state.selectedImg) return;
  const img = state.selectedImg;
  const nodeEl = img.closest('[data-node-id]');
  if (!nodeEl) return;

  const imgNodeId = nodeEl.dataset.nodeId;
  const lock = $('ipLockAspect')?.checked !== false; // default locked

  const pxToPt = 0.75; // 1px = 0.75pt (96dpi -> 72pt)
  let w, h;

  if (dimension === 'width') {
    w = value;
    if (lock) {
      const ratio = (img.naturalHeight || img.offsetHeight) / (img.naturalWidth || img.offsetWidth);
      h = Math.round(value * ratio);
    } else {
      h = parseFloat($('ipHeight')?.value) || img.offsetHeight;
    }
  } else {
    h = value;
    if (lock) {
      const ratio = (img.naturalWidth || img.offsetWidth) / (img.naturalHeight || img.offsetHeight);
      w = Math.round(value * ratio);
    } else {
      w = parseFloat($('ipWidth')?.value) || img.offsetWidth;
    }
  }

  // Update the DOM
  img.style.width = w + 'px';
  img.style.height = h + 'px';

  // Update input fields
  if ($('ipWidth')) $('ipWidth').value = Math.round(w);
  if ($('ipHeight')) $('ipHeight').value = Math.round(h);

  // Persist to WASM
  const actualImgNodeId = findImageNodeId(imgNodeId);
  if (actualImgNodeId) {
    try {
      state.doc.resize_image(actualImgNodeId, w * pxToPt, h * pxToPt);
      broadcastOp({ action: 'resizeImage', nodeId: actualImgNodeId, width: w * pxToPt, height: h * pxToPt });
      updateUndoRedo();
    } catch (e) { console.error('props panel: resize', e); }
  }
}

function applyImageAltText(alt) {
  if (!state.doc || !state.selectedImg) return;
  const img = state.selectedImg;
  const nodeEl = img.closest('[data-node-id]');
  if (!nodeEl) return;

  const imgNodeId = findImageNodeId(nodeEl.dataset.nodeId);
  if (!imgNodeId) return;

  try {
    state.doc.set_image_alt_text(imgNodeId, alt);
    broadcastOp({ action: 'setImageAltText', nodeId: imgNodeId, alt });
    img.setAttribute('alt', alt);
    updateUndoRedo();
  } catch (e) { console.error('props panel: alt text', e); }
}

function applyImageWrapMode(mode) {
  if (!state.doc || !state.selectedImg) return;
  const img = state.selectedImg;
  const nodeEl = img.closest('[data-node-id]');
  if (!nodeEl) return;

  const imgNodeId = findImageNodeId(nodeEl.dataset.nodeId);
  if (!imgNodeId) return;

  try {
    state.doc.set_image_wrap_mode(imgNodeId, mode);
    broadcastOp({ action: 'setImageWrapMode', nodeId: imgNodeId, mode });
    renderDocument();
    updateUndoRedo();
  } catch (e) { console.error('props panel: wrap mode', e); }
}

/**
 * Walk from a paragraph-level node ID to find the Image node ID beneath it.
 */
function findImageNodeId(paraNodeIdStr) {
  if (!state.doc) return null;
  try {
    const nodeJson = state.doc.node_info_json(paraNodeIdStr);
    if (nodeJson) {
      const info = JSON.parse(nodeJson);
      if (info.type === 'Image') return paraNodeIdStr;
      if (info.children && info.children.length) {
        for (const childId of info.children) {
          const childJson = state.doc.node_info_json(childId);
          if (childJson) {
            const childInfo = JSON.parse(childJson);
            if (childInfo.type === 'Image') return childId;
            if (childInfo.children) {
              for (const gcId of childInfo.children) {
                const gcJson = state.doc.node_info_json(gcId);
                if (gcJson) {
                  const gcInfo = JSON.parse(gcJson);
                  if (gcInfo.type === 'Image') return gcId;
                }
              }
            }
          }
        }
      }
    }
  } catch (_) {}
  return null;
}

// ─── Table Properties (Stub) ─────────────────────
function initTableControls() {
  // Table width mode
  const propsWidthMode = $('propsWidthMode');
  if (propsWidthMode) {
    propsWidthMode.addEventListener('change', () => {
      // Stub: table width mode not yet wired to WASM
      console.debug('Table width mode:', propsWidthMode.value);
    });
  }

  // Cell background
  const tpCellBg = $('tpCellBg');
  if (tpCellBg) {
    tpCellBg.addEventListener('input', () => {
      applyTableCellBackground(tpCellBg.value);
    });
  }
}

function populateTableProps() {
  if (!state.doc) return;

  // Try to read table dimensions from the current context
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;

  let cellEl = sel.anchorNode;
  while (cellEl && cellEl.tagName !== 'TD' && cellEl.tagName !== 'TH') {
    cellEl = cellEl.parentNode;
  }
  if (!cellEl) return;

  let tableEl = cellEl.closest('table');
  if (!tableEl) return;

  const tableNodeEl = tableEl.closest('[data-node-id]');
  if (!tableNodeEl) return;

  try {
    const dims = JSON.parse(state.doc.get_table_dimensions(tableNodeEl.dataset.nodeId));
    const tpInfo = $('tpTableInfo');
    if (tpInfo) {
      tpInfo.textContent = `${dims.rows} rows x ${dims.cols} columns`;
    }
  } catch (e) {
    const tpInfo = $('tpTableInfo');
    if (tpInfo) tpInfo.textContent = '';
  }
}

function applyTableCellBackground(hex) {
  if (!state.doc) return;

  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;

  let cellEl = sel.anchorNode;
  while (cellEl && cellEl.tagName !== 'TD' && cellEl.tagName !== 'TH') {
    cellEl = cellEl.parentNode;
  }
  if (!cellEl) return;

  const cellNodeEl = cellEl.closest('[data-node-id]');
  if (!cellNodeEl) return;

  try {
    const cellId = cellNodeEl.dataset.nodeId;
    state.doc.set_cell_background(cellId, hex.replace('#', ''));
    broadcastOp({ action: 'setCellBackground', nodeId: cellId, hex: hex.replace('#', '') });
    cellEl.style.backgroundColor = hex;
    updateUndoRedo();
  } catch (e) { console.error('props panel: cell bg', e); }
}

// ─── Section Properties (Stub) ───────────────────
function initSectionControls() {
  // Page size preset
  const spPageSize = $('spPageSize');
  if (spPageSize) {
    spPageSize.addEventListener('change', () => {
      applySectionPageSize(spPageSize.value);
    });
  }

  // Orientation
  const spOrientation = $('spOrientation');
  if (spOrientation) {
    spOrientation.addEventListener('change', () => {
      applySectionOrientation(spOrientation.value);
    });
  }

  // Margins
  ['spMarginTop', 'spMarginBottom', 'spMarginLeft', 'spMarginRight'].forEach(id => {
    const input = $(id);
    if (input) {
      input.addEventListener('change', () => {
        applySectionMargins();
      });
    }
  });

  // Columns
  const spColumns = $('spColumns');
  if (spColumns) {
    spColumns.addEventListener('change', () => {
      applySectionColumns(parseInt(spColumns.value) || 1);
    });
  }
}

function populateSectionProps() {
  if (!state.doc) return;

  try {
    const setup = JSON.parse(state.doc.get_page_setup_json());

    // Page size — detect common sizes
    const spPageSize = $('spPageSize');
    if (spPageSize) {
      const w = setup.pageWidth;
      const h = setup.pageHeight;
      if (Math.abs(w - 612) < 1 && Math.abs(h - 792) < 1) spPageSize.value = 'letter';
      else if (Math.abs(w - 612) < 1 && Math.abs(h - 1008) < 1) spPageSize.value = 'legal';
      else if (Math.abs(w - 595.28) < 2 && Math.abs(h - 841.89) < 2) spPageSize.value = 'a4';
      else if (Math.abs(w - 419.53) < 2 && Math.abs(h - 595.28) < 2) spPageSize.value = 'a5';
      else spPageSize.value = 'custom';
    }

    // Orientation
    const spOrient = $('spOrientation');
    if (spOrient) spOrient.value = setup.orientation || 'portrait';

    // Margins (pt -> in)
    const setMargin = (id, val) => {
      const el = $(id);
      if (el) el.value = (val / 72).toFixed(2);
    };
    setMargin('spMarginTop', setup.marginTop || 72);
    setMargin('spMarginBottom', setup.marginBottom || 72);
    setMargin('spMarginLeft', setup.marginLeft || 72);
    setMargin('spMarginRight', setup.marginRight || 72);

    // Columns — read from first section
    try {
      const sections = JSON.parse(state.doc.get_sections_json());
      if (sections.length > 0) {
        const spCols = $('spColumns');
        if (spCols) spCols.value = String(sections[0].columns || 1);
      }
    } catch (_) {}

  } catch (e) {
    console.debug('props panel: section populate error', e);
  }
}

function applySectionPageSize(preset) {
  if (!state.doc) return;

  const sizes = {
    letter: { pageWidth: 612, pageHeight: 792 },
    legal: { pageWidth: 612, pageHeight: 1008 },
    a4: { pageWidth: 595.28, pageHeight: 841.89 },
    a5: { pageWidth: 419.53, pageHeight: 595.28 },
  };

  const size = sizes[preset];
  if (!size) return;

  try {
    const current = JSON.parse(state.doc.get_page_setup_json());
    Object.assign(current, size);
    state.doc.set_page_setup(JSON.stringify(current));
    broadcastOp({ action: 'setPageSetup', setup: current });
    renderDocument();
    updateUndoRedo();
    populateSectionProps();
  } catch (e) { console.error('props panel: page size', e); }
}

function applySectionOrientation(orient) {
  if (!state.doc) return;

  try {
    const current = JSON.parse(state.doc.get_page_setup_json());
    // Swap width/height if changing orientation
    if (orient !== current.orientation) {
      const w = current.pageWidth;
      current.pageWidth = current.pageHeight;
      current.pageHeight = w;
      current.orientation = orient;
    }
    state.doc.set_page_setup(JSON.stringify(current));
    broadcastOp({ action: 'setPageSetup', setup: current });
    renderDocument();
    updateUndoRedo();
    populateSectionProps();
  } catch (e) { console.error('props panel: orientation', e); }
}

function applySectionMargins() {
  if (!state.doc) return;

  try {
    const current = JSON.parse(state.doc.get_page_setup_json());
    const getInches = (id) => parseFloat($(id)?.value || '1') * 72;
    current.marginTop = getInches('spMarginTop');
    current.marginBottom = getInches('spMarginBottom');
    current.marginLeft = getInches('spMarginLeft');
    current.marginRight = getInches('spMarginRight');
    state.doc.set_page_setup(JSON.stringify(current));
    broadcastOp({ action: 'setPageSetup', setup: current });
    renderDocument();
    updateUndoRedo();
  } catch (e) { console.error('props panel: margins', e); }
}

function applySectionColumns(cols) {
  if (!state.doc) return;

  try {
    state.doc.set_section_columns(0, cols, 36);
    broadcastOp({ action: 'setSectionColumns', sectionIndex: 0, columns: cols, spacing: 36 });
    renderDocument();
    updateUndoRedo();
  } catch (e) { console.error('props panel: columns', e); }
}

// ─── Document Info (macros, signatures) ──────────
// P4 Step 5: Show VBA macro names in properties panel
export function populateDocInfo() {
  if (!state.doc) return;

  const infoSection = $('propsDocInfo');
  const macrosGroup = $('propsDocMacros');
  const macroList = $('propsMacroList');
  if (!infoSection || !macrosGroup || !macroList) return;

  try {
    const meta = state.doc.metadata_json ? JSON.parse(state.doc.metadata_json()) : {};
    const props = meta.custom_properties || {};

    if (props.hasMacros === 'true') {
      infoSection.style.display = '';
      macrosGroup.style.display = '';

      const names = props.macroNames || '';
      if (names) {
        const modules = names.split(',').map(n => n.trim()).filter(Boolean);
        macroList.innerHTML = modules
          .map(n => `<div style="padding:2px 0;border-bottom:1px solid #f0f0f0">${n}</div>`)
          .join('');
      } else {
        macroList.textContent = 'Macros detected (names unavailable)';
      }
    } else {
      infoSection.style.display = 'none';
      macrosGroup.style.display = 'none';
    }
  } catch (e) {
    console.debug('props panel: doc info error', e);
  }
}
