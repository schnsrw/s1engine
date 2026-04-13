/**
 * adapter.js — s1engine WASM ↔ OnlyOffice sdkjs bridge
 *
 * Architecture: s1engine-first (see docs/ADR-001-editing-authority.md)
 * s1engine owns the document model. OnlyOffice is the rendering facade.
 *
 * M1: Text-only open/save (working)
 * M3: Structural import — paragraphs, runs, formatting (this version)
 * M4: Structural export (future)
 */

import init, { WasmEngine } from './pkg/s1engine_wasm.js';

let wasmEngine = null;
let wasmReady = false;

export async function initWasm() {
  if (wasmReady) return;
  await init();
  wasmEngine = new WasmEngine();
  wasmReady = true;
  console.log('[adapter] WASM ready');
}

// ──────────────────────────────────────────────────────────────────────────
// OPEN: DOCX → s1engine → structured OnlyOffice document
// ──────────────────────────────────────────────────────────────────────────

export async function openDocx(docxBytes, api) {
  if (!wasmReady) await initWasm();

  var doc = wasmEngine.open(docxBytes);
  var bodyChildrenJson = doc.body_children_json();
  var bodyChildren = JSON.parse(bodyChildrenJson);

  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  // Disable recalc + history during bulk construction
  logicDoc.TurnOff_Recalculate();
  logicDoc.TurnOff_InterfaceEvents();

  // Remove all existing content
  var existingCount = logicDoc.Content.length;
  for (var r = existingCount - 1; r >= 0; r--) {
    logicDoc.Internal_Content_Remove(r, 1);
  }

  // Build structured content from s1engine model
  var paraCount = 0;
  for (var i = 0; i < bodyChildren.length; i++) {
    if (bodyChildren[i].type !== 'Paragraph') continue;

    var paraInfo = JSON.parse(doc.node_info_json(bodyChildren[i].id));
    var para = buildParagraph(logicDoc, doc, paraInfo);
    logicDoc.Internal_Content_Add(paraCount, para);
    paraCount++;
  }

  // Ensure at least one paragraph exists
  if (paraCount === 0) {
    var emptyPara = new AscWord.Paragraph(logicDoc, false);
    logicDoc.Internal_Content_Add(0, emptyPara);
    paraCount = 1;
  }

  // Re-enable and render
  logicDoc.TurnOn_InterfaceEvents(false);
  logicDoc.TurnOn_Recalculate(false);

  // Rebuild section info so page layout (margins, size) works correctly
  if (logicDoc.UpdateAllSectionsInfo) logicDoc.UpdateAllSectionsInfo();

  logicDoc.MoveCursorToStartPos(false);
  logicDoc.Recalculate();
  api.Resize();

  console.log('[adapter] open: ' + paraCount + ' paragraphs loaded');
}

/**
 * Build a single OnlyOffice Paragraph from s1engine node info.
 */
function buildParagraph(logicDoc, wasmDoc, paraInfo) {
  var para = new AscWord.Paragraph(logicDoc, false);

  // Apply paragraph properties — set explicit defaults first to override template
  // sdkjs alignment: 0=Right, 1=Left, 2=Center, 3=Justify (commonDefines.js:734)
  para.Pr.Jc = 1; // default: left-aligned
  if (paraInfo.alignment) {
    var alignMap = { 'left': 1, 'center': 2, 'right': 0, 'justify': 3, 'both': 3 };
    if (alignMap[paraInfo.alignment] !== undefined) {
      para.Pr.Jc = alignMap[paraInfo.alignment];
    }
  }
  if (paraInfo.styleId) {
    para.Pr.PStyle = paraInfo.styleId;
  }

  // Paragraph layout properties — set explicit defaults to override template
  // Default: allow paragraphs to split across pages, enable widow/orphan control
  para.Pr.KeepLines = paraInfo.keepLinesTogether === true;
  para.Pr.KeepNext = paraInfo.keepWithNext === true;
  para.Pr.PageBreakBefore = paraInfo.pageBreakBefore === true;
  para.Pr.WidowControl = paraInfo.widowControl !== false; // default true

  // Build runs from children (must call Correct_Content after all runs added)
  if (paraInfo.children && paraInfo.children.length > 0) {
    for (var i = 0; i < paraInfo.children.length; i++) {
      try {
        var childInfo = JSON.parse(wasmDoc.node_info_json(paraInfo.children[i]));
        if (childInfo.type === 'Run') {
          var run = buildRun(para, wasmDoc, childInfo);
          // Insert BEFORE the paragraph mark run (always last element)
          var insertPos = Math.max(para.Content.length - 1, 0);
          para.Internal_Content_Add(insertPos, run);
        }
      } catch (e) {
        // Skip unreadable nodes
      }
    }
  }

  // Finalize paragraph structure — required by sdkjs layout engine.
  // Ensures proper Run boundaries and paragraph mark placement.
  // BinaryFileReader (Serialize2.js:11391) does this for every paragraph.
  if (para.Correct_Content) para.Correct_Content();
  if (para.MoveCursorToStartPos) para.MoveCursorToStartPos(false);

  // Mark text as needing HarfBuzz shaping before line breaking.
  // Without this, the layout engine treats text as individual characters
  // and breaks lines at character boundaries instead of word boundaries.
  // ShapeText() is called from Recalculate_Page(0) at Paragraph_Recalculate.js:432
  if (para.RecalcInfo) para.RecalcInfo.NeedShapeText();

  return para;
}

/**
 * Build a single OnlyOffice ParaRun from s1engine node info.
 * Applies formatting: bold, italic, underline, strikethrough, font, size.
 */
function buildRun(para, wasmDoc, runInfo) {
  var run = new AscWord.ParaRun(para, false);

  // Apply run formatting — explicit defaults to override document template
  run.Pr.Bold = runInfo.bold === true;
  run.Pr.Italic = runInfo.italic === true;
  run.Pr.Underline = runInfo.underline === true;
  run.Pr.Strikeout = runInfo.strikethrough === true;

  if (runInfo.fontSize !== undefined && runInfo.fontSize !== null) {
    run.Pr.FontSize = runInfo.fontSize;
  }

  if (runInfo.fontFamily) {
    run.Pr.RFonts.Ascii = { Name: runInfo.fontFamily, Index: -1 };
    run.Pr.RFonts.HAnsi = { Name: runInfo.fontFamily, Index: -1 };
    run.Pr.RFonts.CS = { Name: runInfo.fontFamily, Index: -1 };
    run.Pr.RFonts.EastAsia = { Name: runInfo.fontFamily, Index: -1 };
  }

  // Color — node_info_json returns "#rrggbb" format
  if (runInfo.color && runInfo.color !== '#000000') {
    var hex = runInfo.color.substring(1);
    var r = parseInt(hex.substring(0, 2), 16);
    var g = parseInt(hex.substring(2, 4), 16);
    var b = parseInt(hex.substring(4, 6), 16);
    run.Pr.Color = new AscCommonWord.CDocumentColor(r, g, b, false);
  }

  // Add text content from run's children
  if (runInfo.children && runInfo.children.length > 0) {
    for (var i = 0; i < runInfo.children.length; i++) {
      try {
        var textInfo = JSON.parse(wasmDoc.node_info_json(runInfo.children[i]));
        if (textInfo.text) {
          for (var j = 0; j < textInfo.text.length; j++) {
            var c = textInfo.text.charCodeAt(j);
            if (c === 0x0A || c === 0x0D) continue;
            if (c === 0x09) {
              run.Add_ToContent(-1, new AscWord.CRunTab(), false);
            } else if (c === 0x20) {
              // CRITICAL: spaces must be CRunSpace, not CRunText(32).
              // CRunSpace.IsSpace()=true triggers FlushWord in TextShaper,
              // which marks word boundaries for line breaking.
              // CRunText(32).IsText()=true stays part of the "word" —
              // entire paragraph becomes one unbreakable word.
              run.Add_ToContent(-1, new AscWord.CRunSpace(), false);
            } else {
              run.Add_ToContent(-1, new AscWord.CRunText(c), false);
            }
          }
        }
      } catch (e) {
        // Skip unreadable text nodes
      }
    }
  }

  return run;
}

// ──────────────────────────────────────────────────────────────────────────
// SAVE: OnlyOffice → extract structured text → s1engine → DOCX
// ──────────────────────────────────────────────────────────────────────────

export function saveDocx(api) {
  if (!wasmReady) throw new Error('WASM not initialized');

  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  // Extract paragraph text using sdkjs Paragraph.GetText API
  var paragraphs = [];
  for (var i = 0; i < logicDoc.Content.length; i++) {
    var el = logicDoc.Content[i];
    if (el && el.IsParagraph && el.IsParagraph()) {
      paragraphs.push(el.GetText({ ParaSeparator: '' }) || '');
    }
  }

  console.log('[adapter] save: ' + paragraphs.length + ' paragraphs');

  // Build new s1engine document and export
  var newDoc = wasmEngine.create();
  for (var i = 0; i < paragraphs.length; i++) {
    newDoc.append_paragraph(paragraphs[i]);
  }
  var bytes = newDoc.export('docx');
  console.log('[adapter] save: ' + bytes.length + ' bytes');
  return bytes;
}

export function downloadFile(data, filename) {
  var blob = new Blob([data], {
    type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
  });
  var a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = filename;
  a.click();
  URL.revokeObjectURL(a.href);
}
