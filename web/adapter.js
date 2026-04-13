/**
 * adapter.js — Text-only DOCX bridge (M1/M2)
 *
 * Open: DOCX → s1engine WASM → plain text → sdkjs editor
 * Save: sdkjs editor → plain text → s1engine WASM → DOCX
 *
 * Formatting fidelity is not preserved in this version.
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

export async function openDocx(docxBytes, api) {
  if (!wasmReady) await initWasm();

  // Parse DOCX and get ALL text at once
  var doc = wasmEngine.open(docxBytes);
  var fullText = doc.to_plain_text();
  console.log('[adapter] Extracted ' + fullText.length + ' chars');

  if (!fullText || fullText.length === 0) {
    throw new Error('Document has no text content');
  }

  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  // Clear editor
  logicDoc.SelectAll();
  logicDoc.Remove(1, true, false, true);
  logicDoc.RemoveSelection();
  try { api.put_TextPrUnderline(false); } catch(e) {}

  // Split into paragraphs and insert
  var paragraphs = fullText.split('\n');
  var isFirst = true;
  for (var i = 0; i < paragraphs.length; i++) {
    var text = paragraphs[i];
    if (!isFirst) logicDoc.AddNewParagraph(false);
    isFirst = false;
    if (text) {
      for (var j = 0; j < text.length; j++) {
        var c = text.charCodeAt(j);
        if (c === 13) continue;
        if (c === 9) logicDoc.AddToParagraph(new AscWord.CRunTab());
        else logicDoc.AddToParagraph(new AscWord.CRunText(c));
      }
    }
  }

  logicDoc.MoveCursorToStartPos(false);
  logicDoc.Recalculate();
  api.Resize();
  console.log('[adapter] Loaded ' + paragraphs.length + ' paragraphs');
}

export function saveDocx(api) {
  if (!wasmReady) throw new Error('WASM not initialized');

  var logicDoc = api.WordControl.m_oLogicDocument;
  if (!logicDoc) throw new Error('No logic document');

  // Extract all text at once using GetSelectedText with no selection
  var allText = logicDoc.GetSelectedText(false, {
    ParaSeparator: '\n',
    Numbering: false,
    Math: false,
    TabSymbol: '\t',
    NewLineSeparator: '\n'
  }) || '';

  // Fallback: manual extraction if GetSelectedText returns empty
  if (!allText) {
    for (var i = 0; i < logicDoc.Content.length; i++) {
      var el = logicDoc.Content[i];
      if (!el || !el.Content) continue;
      var paraText = '';
      for (var j = 0; j < el.Content.length; j++) {
        var run = el.Content[j];
        if (!run || !run.Content) continue;
        for (var k = 0; k < run.Content.length; k++) {
          var item = run.Content[k];
          if (item && item.Value !== undefined && item.Value !== null) {
            paraText += String.fromCharCode(item.Value);
          }
        }
      }
      allText += paraText + '\n';
    }
  }

  var paragraphs = allText.split('\n');
  // Remove trailing empty paragraph if present
  while (paragraphs.length > 1 && paragraphs[paragraphs.length - 1] === '') {
    paragraphs.pop();
  }

  console.log('[adapter] Saving ' + paragraphs.length + ' paragraphs, ' + allText.length + ' chars');

  // Build new document and export
  var newDoc = wasmEngine.create();
  for (var i = 0; i < paragraphs.length; i++) {
    newDoc.append_paragraph(paragraphs[i]);
  }
  var bytes = newDoc.export('docx');
  console.log('[adapter] Exported ' + bytes.length + ' bytes');
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
