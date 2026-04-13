/**
 * adapter.js — Text-only DOCX bridge (M1/M2)
 *
 * Open: DOCX → s1engine WASM → to_plain_text() → split by \n → AddToParagraph per char
 * Save: sdkjs Content[] → Paragraph.GetText() per paragraph → s1engine → export DOCX
 *
 * Formatting is not preserved. This is explicitly a text-only bridge.
 * See docs/INTEGRATION_MILESTONES.md M3/M4 for structural fidelity.
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

/**
 * Open DOCX: parse with s1engine, extract plain text, insert into sdkjs editor.
 */
export async function openDocx(docxBytes, api) {
  if (!wasmReady) await initWasm();

  // Step 1: Parse DOCX and extract all text
  var doc = wasmEngine.open(docxBytes);
  var fullText = doc.to_plain_text();

  if (!fullText || fullText.length === 0) {
    throw new Error('Document contains no text');
  }

  console.log('[adapter] open: ' + fullText.length + ' chars extracted');

  // Step 2: Access sdkjs document model
  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  // Step 3: Clear existing editor content
  logicDoc.SelectAll();
  logicDoc.Remove(1, true, false, true);
  logicDoc.RemoveSelection();
  try { api.put_TextPrUnderline(false); } catch(e) {}

  // Step 4: Disable recalculation during bulk insert for speed
  logicDoc.TurnOff_Recalculate();

  // Step 5: Split text into paragraphs, insert via AddToParagraph
  var paragraphs = fullText.split('\n');
  var isFirst = true;
  for (var i = 0; i < paragraphs.length; i++) {
    var text = paragraphs[i];
    if (!isFirst) logicDoc.AddNewParagraph(false);
    isFirst = false;
    if (text) {
      for (var j = 0; j < text.length; j++) {
        var c = text.charCodeAt(j);
        if (c === 13) continue; // skip \r
        if (c === 9) logicDoc.AddToParagraph(new AscWord.CRunTab());
        else logicDoc.AddToParagraph(new AscWord.CRunText(c));
      }
    }
  }

  // Step 6: Re-enable recalculation and render
  logicDoc.TurnOn_Recalculate(false);
  logicDoc.MoveCursorToStartPos(false);
  logicDoc.Recalculate();
  api.Resize();

  console.log('[adapter] open: loaded ' + paragraphs.length + ' paragraphs');
}

/**
 * Save DOCX: extract text from sdkjs paragraphs, build new doc, export.
 *
 * Uses Paragraph.GetText() per paragraph — the sdkjs API for paragraph text.
 * See Paragraph.js:16730 for implementation.
 */
export function saveDocx(api) {
  if (!wasmReady) throw new Error('WASM not initialized');

  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  // Extract text from each sdkjs paragraph using GetText API
  var paragraphs = [];
  for (var i = 0; i < logicDoc.Content.length; i++) {
    var el = logicDoc.Content[i];
    if (el && el.IsParagraph && el.IsParagraph()) {
      var text = el.GetText({ ParaSeparator: '' }) || '';
      paragraphs.push(text);
    }
  }

  console.log('[adapter] save: ' + paragraphs.length + ' paragraphs, first="' + (paragraphs[0] || '').substring(0, 60) + '"');

  // Build new s1engine document
  var newDoc = wasmEngine.create();
  for (var i = 0; i < paragraphs.length; i++) {
    newDoc.append_paragraph(paragraphs[i]);
  }

  // Export as DOCX
  var bytes = newDoc.export('docx');
  console.log('[adapter] save: exported ' + bytes.length + ' bytes');
  return bytes;
}

/**
 * Trigger browser download of bytes as a file.
 */
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
