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

  // Parse DOCX with s1engine and convert to DOCY binary format.
  // DOCY is sdkjs's native format — BinaryFileReader handles everything:
  // headers, footers, images, tables, TOC, comments, footnotes, etc.
  var doc = wasmEngine.open(docxBytes);

  // DOCY path: convert s1engine model → DOCY binary → sdkjs native rendering
  try {
    var docy = doc.to_docy();
    if (docy && docy.length > 20) {
      console.log('[adapter] DOCY path (' + docy.length + ' chars)');
      // OpenDocumentFromBin calls BeforeOpenDocument → InitEditor → BinaryFileReader.Read
      api.OpenDocumentFromBin('', docy);
      console.log('[adapter] Document opened via DOCY');
      return doc;
    }
  } catch(e) {
    console.warn('[adapter] DOCY failed:', e.message, '— using manual path');
  }

  // Manual paragraph-by-paragraph construction (fallback)
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
  var contentIndex = 0;
  for (var i = 0; i < bodyChildren.length; i++) {
    var child = bodyChildren[i];
    try {
      if (child.type === 'Paragraph') {
        var paraInfo = JSON.parse(doc.node_info_json(child.id));
        var para = buildParagraph(logicDoc, doc, paraInfo);
        logicDoc.Internal_Content_Add(contentIndex, para);
        contentIndex++;
      } else if (child.type === 'Table') {
        var table = buildTable(logicDoc, doc, child.id);
        if (table) {
          logicDoc.Internal_Content_Add(contentIndex, table);
          contentIndex++;
        }
      }
      // Images inside paragraphs are handled at paragraph level
      // Top-level images are not standard DOCX structure — skip
    } catch (e) {
      // Skip problematic elements
    }
  }

  // Ensure at least one paragraph exists
  if (contentIndex === 0) {
    var emptyPara = new AscWord.Paragraph(logicDoc, false);
    logicDoc.Internal_Content_Add(0, emptyPara);
    contentIndex = 1;
  }

  // Re-enable and render
  logicDoc.TurnOn_InterfaceEvents(false);
  logicDoc.TurnOn_Recalculate(false);

  // Rebuild section info so page layout (margins, size) works correctly
  if (logicDoc.UpdateAllSectionsInfo) logicDoc.UpdateAllSectionsInfo();

  logicDoc.MoveCursorToStartPos(false);
  logicDoc.Recalculate();

  // Recalculate page drawing positions on canvas.
  // Recalculate() updates the document model (mm coordinates).
  // CalculateDocumentSize + OnCalculatePagesPlace update where pages
  // are drawn on the canvas (pixel coordinates).
  // Without both, click coordinates and rendered page positions disagree.
  var wc = api.WordControl;
  if (wc) {
    if (wc.CalculateDocumentSize) wc.CalculateDocumentSize();
    if (wc.OnCalculatePagesPlace) wc.OnCalculatePagesPlace();
    if (wc.checkBodyOffset) wc.checkBodyOffset();
    if (wc.OnResize) wc.OnResize(true);
    else api.Resize();
  } else {
    api.Resize();
  }

  // Sync caret/selection overlays after layout and offset refresh.
  logicDoc.Document_UpdateSelectionState();
  logicDoc.Document_UpdateInterfaceState();
  if (api.WordControl && api.WordControl.m_oDrawingDocument) {
    api.WordControl.m_oDrawingDocument.UpdateTargetFromPaint = true;
    if (api.WordControl.m_oDrawingDocument.UpdateTargetNoAttack) {
      api.WordControl.m_oDrawingDocument.UpdateTargetNoAttack();
    }
  }

  // Recompute the logical caret position from the rebuilt document model.
  // Without this, the visual target can keep a stale Y from the previous doc state.
  if (logicDoc.RecalculateCurPos) {
    logicDoc.RecalculateCurPos();
  }

  console.log('[adapter] open: ' + contentIndex + ' elements loaded');
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

  // List format — apply numbering/bullets if present
  if (paraInfo.listFormat) {
    var listType = paraInfo.listFormat === 'bullet' ? 0 : 1; // 0=bullet, 1=numbered
    var listLevel = paraInfo.listLevel || 0;
    // sdkjs list API: put_ListType(type, subtype)
    // Apply via paragraph NumPr property
    if (para.Pr) {
      para.Pr.NumPr = { NumId: listType + 1, Lvl: listLevel };
    }
  }

  // NOTE: Indentation and spacing mapping REMOVED — was using wrong unit
  // conversion (x20) causing 4-page docs to become 238 pages.
  // TODO: Investigate correct units (s1engine pt vs sdkjs twips/mm) before re-adding.

  // Paragraph layout properties — set explicit defaults to override template
  // Default: allow paragraphs to split across pages, enable widow/orphan control
  para.Pr.KeepLines = paraInfo.keepLinesTogether === true;
  para.Pr.KeepNext = paraInfo.keepWithNext === true;
  para.Pr.PageBreakBefore = paraInfo.pageBreakBefore === true;
  para.Pr.WidowControl = paraInfo.widowControl !== false; // default true

  // Build runs from children (must call Correct_Content after all runs added)
  // s1engine model has Run, LineBreak, Tab as paragraph-level children
  if (paraInfo.children && paraInfo.children.length > 0) {
    var currentRun = null; // accumulate inline content into runs

    for (var i = 0; i < paraInfo.children.length; i++) {
      try {
        var childInfo = JSON.parse(wasmDoc.node_info_json(paraInfo.children[i]));
        var insertPos;

        if (childInfo.type === 'Run') {
          var run = buildRun(para, wasmDoc, childInfo);
          insertPos = Math.max(para.Content.length - 1, 0);
          para.Internal_Content_Add(insertPos, run);
          currentRun = run;

        } else if (childInfo.type === 'LineBreak') {
          // Line break → CRunBreak(break_Line) inside a run
          if (!currentRun) {
            currentRun = new AscWord.ParaRun(para, false);
            insertPos = Math.max(para.Content.length - 1, 0);
            para.Internal_Content_Add(insertPos, currentRun);
          }
          currentRun.Add_ToContent(-1, new AscWord.CRunBreak(AscWord.break_Line), false);

        } else if (childInfo.type === 'Tab') {
          // Tab → CRunTab inside a run
          if (!currentRun) {
            currentRun = new AscWord.ParaRun(para, false);
            insertPos = Math.max(para.Content.length - 1, 0);
            para.Internal_Content_Add(insertPos, currentRun);
          }
          currentRun.Add_ToContent(-1, new AscWord.CRunTab(), false);

        } else if (childInfo.type === 'Image') {
          // Inline image — get data URL and dimensions from s1engine
          try {
            var imgDataUrl = wasmDoc.get_image_data_url(paraInfo.children[i]);
            var imgW = childInfo.imageWidth || 100; // pt
            var imgH = childInfo.imageHeight || 100;
            // Convert pt to mm for sdkjs (1pt = 0.3528mm)
            var wMm = imgW * 0.3528;
            var hMm = imgH * 0.3528;
            // Use sdkjs API to add inline image via a drawing
            var drawDoc = logicDoc.DrawingDocument || logicDoc.Get_DrawingDocument();
            if (drawDoc && api) {
              // Create image via ParaDrawing
              var drawing = new AscCommonWord.ParaDrawing(wMm, hMm, null, drawDoc, logicDoc, para);
              var imageObj = AscFormat.CreateImage(imgDataUrl, 0, 0, wMm, hMm);
              drawing.Set_GraphicObject(imageObj);
              drawing.Set_DrawingType(1); // inline
              insertPos = Math.max(para.Content.length - 1, 0);
              para.Internal_Content_Add(insertPos, drawing);
              currentRun = null;
            }
          } catch(imgErr) {
            // Image loading failed — skip silently
          }

        } else if (childInfo.type === 'PageBreak' || childInfo.type === 'ColumnBreak') {
          // Page/column break → CRunBreak inside a run
          if (!currentRun) {
            currentRun = new AscWord.ParaRun(para, false);
            insertPos = Math.max(para.Content.length - 1, 0);
            para.Internal_Content_Add(insertPos, currentRun);
          }
          var breakType = childInfo.type === 'PageBreak' ? AscWord.break_Page : AscWord.break_Column;
          currentRun.Add_ToContent(-1, new AscWord.CRunBreak(breakType), false);
        }
        // Other node types (Table, Image, etc.) — skip for M3
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

  // Hyperlink — mark as blue underlined text (full hyperlink objects are M5+ scope)
  if (runInfo.hyperlinkUrl) {
    run.Pr.Underline = true;
    run.Pr.Color = new AscCommonWord.CDocumentColor(0, 0, 255, false);
  }

  // Color — node_info_json returns "#rrggbb" format
  if (runInfo.color && runInfo.color !== '#000000' && !runInfo.hyperlinkUrl) {
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

/**
 * Build an OnlyOffice CTable from s1engine table node.
 * Structure: Table → TableRow[] → TableCell[] → Paragraph[]
 */
function buildTable(logicDoc, wasmDoc, tableId) {
  var tableInfo = JSON.parse(wasmDoc.node_info_json(tableId));
  if (!tableInfo.children || tableInfo.children.length === 0) return null;

  // Count rows and max columns
  var rowInfos = [];
  var maxCols = 0;
  for (var r = 0; r < tableInfo.children.length; r++) {
    var rowInfo = JSON.parse(wasmDoc.node_info_json(tableInfo.children[r]));
    if (rowInfo.type !== 'TableRow') continue;
    rowInfos.push(rowInfo);
    if (rowInfo.children) maxCols = Math.max(maxCols, rowInfo.children.length);
  }

  if (rowInfos.length === 0 || maxCols === 0) return null;

  // Create table with CTable constructor
  var drawDoc = logicDoc.DrawingDocument || (logicDoc.Get_DrawingDocument ? logicDoc.Get_DrawingDocument() : null);
  var table = new AscWord.CTable(drawDoc, logicDoc, true, rowInfos.length, maxCols, [], false);

  // Populate each cell
  for (var r = 0; r < rowInfos.length; r++) {
    var rowInfo = rowInfos[r];
    if (!rowInfo.children) continue;

    var tableRow = table.Content[r];
    if (!tableRow) continue;

    for (var c = 0; c < rowInfo.children.length; c++) {
      var cellInfo = JSON.parse(wasmDoc.node_info_json(rowInfo.children[c]));
      if (cellInfo.type !== 'TableCell') continue;

      var tableCell = tableRow.Content[c];
      if (!tableCell || !tableCell.Content) continue;

      // Build cell content (paragraphs)
      if (cellInfo.children && cellInfo.children.length > 0) {
        // Remove default empty paragraph
        while (tableCell.Content.Content.length > 0) {
          tableCell.Content.Internal_Content_Remove(0, 1);
        }

        for (var p = 0; p < cellInfo.children.length; p++) {
          try {
            var cellChildInfo = JSON.parse(wasmDoc.node_info_json(cellInfo.children[p]));
            if (cellChildInfo.type === 'Paragraph') {
              var para = buildParagraph(tableCell.Content, wasmDoc, cellChildInfo);
              tableCell.Content.Internal_Content_Add(p, para);
            }
          } catch (e) {}
        }
      }
    }
  }

  return table;
}

// ──────────────────────────────────────────────────────────────────────────
// SAVE: OnlyOffice → extract structured text → s1engine → DOCX
// ──────────────────────────────────────────────────────────────────────────

export function saveDocx(api) {
  if (!wasmReady) throw new Error('WASM not initialized');

  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  var newDoc = wasmEngine.create();
  var paraCount = 0;

  for (var i = 0; i < logicDoc.Content.length; i++) {
    var el = logicDoc.Content[i];

    // Handle tables
    if (el && el.IsTable && el.IsTable()) {
      try { exportTable(newDoc, el); } catch(e) {}
      paraCount++;
      continue;
    }

    if (!el || !el.IsParagraph || !el.IsParagraph()) continue;

    // Extract paragraph text and structure
    var paraData = extractParagraph(el);
    paraCount++;

    // Create paragraph in s1engine
    var paraId;
    if (paraData.headingLevel > 0 && paraData.headingLevel <= 6) {
      paraId = newDoc.append_heading(paraData.headingLevel, paraData.text);
    } else {
      paraId = newDoc.append_paragraph(paraData.text);
    }

    // Apply paragraph alignment
    if (paraData.alignment && paraData.alignment !== 'left') {
      try { newDoc.set_alignment(paraId, paraData.alignment); } catch(e) {}
    }

    // Apply run-level formatting via range APIs
    if (paraData.runs) {
      for (var r = 0; r < paraData.runs.length; r++) {
        var rd = paraData.runs[r];
        try {
          if (rd.bold) newDoc.set_bold_range(paraId, rd.start, paraId, rd.end, true);
          if (rd.italic) newDoc.set_italic_range(paraId, rd.start, paraId, rd.end, true);
          if (rd.underline) newDoc.set_underline_range(paraId, rd.start, paraId, rd.end, true);
          if (rd.fontSize) newDoc.set_font_size_range(paraId, rd.start, paraId, rd.end, rd.fontSize);
          if (rd.fontName) newDoc.set_font_family_range(paraId, rd.start, paraId, rd.end, rd.fontName);
          if (rd.color && rd.color !== '#000000') newDoc.set_color_range(paraId, rd.start, paraId, rd.end, rd.color);
        } catch(e) {}
      }
    }

    // Insert line breaks and tabs at correct positions (reverse order for stable offsets)
    for (var b = paraData.breaks.length - 1; b >= 0; b--) {
      var brk = paraData.breaks[b];
      try {
        if (brk.type === 'line') newDoc.insert_line_break(paraId, brk.offset);
        else if (brk.type === 'tab') newDoc.insert_tab(paraId, brk.offset);
      } catch(e) {}
    }
  }

  var bytes = newDoc.export('docx');
  console.log('[adapter] save: ' + paraCount + ' paragraphs, ' + bytes.length + ' bytes');
  return bytes;
}

/**
 * Extract structured data from an OnlyOffice Paragraph.
 * Returns: { text, alignment, headingLevel, breaks, runs }
 * runs: [{start, end, bold, italic, underline, strikeout, fontName, fontSize, color}]
 */
function extractParagraph(para) {
  var text = '';
  var breaks = [];
  var runs = []; // run formatting spans with char offsets
  var alignment = 'left';
  var headingLevel = 0;

  // Paragraph properties
  var compiledPr = para.Get_CompiledPr2 ? para.Get_CompiledPr2(false) : null;
  var paraPr = compiledPr ? compiledPr.ParaPr : para.Pr;

  if (paraPr && paraPr.Jc !== undefined) {
    var alignMap = { 0: 'right', 1: 'left', 2: 'center', 3: 'justify' };
    alignment = alignMap[paraPr.Jc] || 'left';
  }

  if (paraPr && paraPr.OutlineLvl !== undefined && paraPr.OutlineLvl >= 0) {
    headingLevel = paraPr.OutlineLvl + 1;
  } else if (paraPr && paraPr.PStyle) {
    var match = paraPr.PStyle.match(/[Hh]eading\s*(\d)/);
    if (match) headingLevel = parseInt(match[1]);
  }

  // Extract runs with formatting
  for (var j = 0; j < para.Content.length; j++) {
    var run = para.Content[j];
    if (!run || !run.Content) continue;

    var runStart = text.length;
    var rPr = run.Pr || {};

    for (var k = 0; k < run.Content.length; k++) {
      var item = run.Content[k];
      if (!item) continue;

      if (item.Value !== undefined && item.Value !== null) {
        text += String.fromCharCode(item.Value);
      } else if (item.IsSpace && item.IsSpace()) {
        text += ' ';
      } else if (item.IsTab && item.IsTab()) {
        breaks.push({ type: 'tab', offset: text.length });
        text += '\t';
      } else if (item.IsBreak && item.IsBreak()) {
        if (item.BreakType === 1) breaks.push({ type: 'line', offset: text.length });
        else if (item.BreakType === 2) breaks.push({ type: 'page', offset: text.length });
      }
    }

    var runEnd = text.length;
    if (runEnd > runStart) {
      var runData = { start: runStart, end: runEnd };
      if (rPr.Bold === true) runData.bold = true;
      if (rPr.Italic === true) runData.italic = true;
      if (rPr.Underline === true) runData.underline = true;
      if (rPr.Strikeout === true) runData.strikeout = true;
      if (rPr.FontSize !== undefined) runData.fontSize = rPr.FontSize;
      if (rPr.RFonts && rPr.RFonts.Ascii && rPr.RFonts.Ascii.Name) {
        runData.fontName = rPr.RFonts.Ascii.Name;
      }
      if (rPr.Color && rPr.Color.r !== undefined) {
        var c = rPr.Color;
        runData.color = '#' + ((1<<24)+(c.r<<16)+(c.g<<8)+c.b).toString(16).slice(1);
      }
      runs.push(runData);
    }
  }

  return { text: text, alignment: alignment, headingLevel: headingLevel, breaks: breaks, runs: runs };
}

/**
 * Export a table from OnlyOffice to s1engine.
 * Creates table via insert_table then populates cells.
 */
function exportTable(newDoc, tableEl) {
  if (!tableEl.Content || tableEl.Content.length === 0) return;

  var rows = tableEl.Content.length;
  var cols = 0;
  for (var r = 0; r < rows; r++) {
    var row = tableEl.Content[r];
    if (row && row.Content) cols = Math.max(cols, row.Content.length);
  }
  if (rows === 0 || cols === 0) return;

  // Create table in s1engine
  try {
    var tableId = newDoc.insert_table(rows, cols);

    // Populate each cell with its paragraph text
    for (var r = 0; r < rows; r++) {
      var row = tableEl.Content[r];
      if (!row || !row.Content) continue;

      for (var c = 0; c < row.Content.length; c++) {
        var cell = row.Content[c];
        if (!cell || !cell.Content || !cell.Content.Content) continue;

        // Get text from all paragraphs in the cell
        var cellText = '';
        for (var p = 0; p < cell.Content.Content.length; p++) {
          var para = cell.Content.Content[p];
          if (para && para.IsParagraph && para.IsParagraph()) {
            if (cellText) cellText += '\n';
            cellText += para.GetText({ ParaSeparator: '' }) || '';
          }
        }

        if (cellText) {
          try {
            newDoc.set_table_cell_text(tableId, r, c, cellText);
          } catch(e) {
            // set_table_cell_text might not exist — try alternative
          }
        }
      }
    }
  } catch(e) {
    // Table export failed — insert paragraphs with cell text as fallback
    for (var r = 0; r < rows; r++) {
      var row = tableEl.Content[r];
      if (!row || !row.Content) continue;
      for (var c = 0; c < row.Content.length; c++) {
        var cell = row.Content[c];
        if (!cell || !cell.Content || !cell.Content.Content) continue;
        for (var p = 0; p < cell.Content.Content.length; p++) {
          var para = cell.Content.Content[p];
          if (para && para.IsParagraph && para.IsParagraph()) {
            var text = para.GetText({ ParaSeparator: '' }) || '';
            if (text) newDoc.append_paragraph(text);
          }
        }
      }
    }
  }
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
