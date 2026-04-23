import puppeteer from 'puppeteer';
import fs from 'fs';

async function main() {
  const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 3000));

  const result = await page.evaluate(async () => {
    const api = window._api || window.editor;
    const ld = api.WordControl.m_oLogicDocument;

    // Create test image
    var c = document.createElement('canvas');
    c.width = 10; c.height = 10;
    var ctx = c.getContext('2d');
    ctx.fillStyle = '#ff0000';
    ctx.fillRect(0, 0, 10, 10);
    var dataUrl = c.toDataURL('image/png');

    // Insert image via API
    ld.MoveCursorToStartPos(false);
    
    AscCommon.History.TurnOff();
    AscCommon.g_oIdCounter.Set_Load(true);
    
    // Manually create the image the way sdkjs does it
    var drawing = new AscCommonWord.ParaDrawing(25.4, 25.4, null, ld.DrawingDocument, ld, ld.Content[0]);
    drawing.Set_DrawingType(0x01); // inline
    drawing.setExtent(25.4, 25.4);
    
    var imageObj = AscFormat.DrawingObjectsController.prototype.createImage(dataUrl, 0, 0, 25.4, 25.4);
    drawing.Set_GraphicObject(imageObj);
    imageObj.setParent(drawing);
    
    // Add to first paragraph  
    var run = new AscCommonWord.ParaRun(ld.Content[0], false);
    run.AddToContent(0, drawing, false);
    ld.Content[0].AddToContent(0, run);
    
    AscCommon.g_oIdCounter.Set_Load(false);
    AscCommon.History.TurnOn();
    
    await new Promise(r => setTimeout(r, 500));
    
    // Now serialize using sdkjs BinaryFileWriter
    var writer = new AscCommonWord.BinaryFileWriter(ld, {});
    var docyStr = writer.Write(false, true);
    
    if (!docyStr) return { error: 'Write returned null' };
    
    // Decode the DOCY binary
    var parts = docyStr.split(';');
    var b64 = parts[3];
    var binStr = atob(b64);
    var bin = new Uint8Array(binStr.length);
    for (var i = 0; i < binStr.length; i++) bin[i] = binStr.charCodeAt(i);
    
    // Find Document table
    var count = bin[0];
    var docOff = -1;
    for (var i = 0; i < count; i++) {
      var p = 1 + i * 5;
      if (bin[p] === 6) { // Document
        docOff = bin[p+1] | (bin[p+2]<<8) | (bin[p+3]<<16) | (bin[p+4]<<24);
      }
    }
    if (docOff < 0) return { error: 'no doc table' };
    
    // Walk document to find pptxDrawing (type 12 in run content)
    var docLen = bin[docOff] | (bin[docOff+1]<<8) | (bin[docOff+2]<<16) | (bin[docOff+3]<<24);
    
    // Scan for byte sequence that indicates pptxDrawing: look for 0x0C (12) followed by reasonable length
    var pptxStart = -1;
    var pptxLen = -1;
    // Walk Read1 in document
    var cur = docOff + 4;
    var end = docOff + 4 + docLen;
    while (cur + 5 <= end) {
      var t = bin[cur];
      var l = bin[cur+1] | (bin[cur+2]<<8) | (bin[cur+3]<<16) | (bin[cur+4]<<24);
      if (t === 0 && l > 0) { // Paragraph
        // Walk paragraph Read1
        var pcur = cur + 5;
        var pend = cur + 5 + l;
        while (pcur + 5 <= pend) {
          var pt = bin[pcur];
          var pl = bin[pcur+1] | (bin[pcur+2]<<8) | (bin[pcur+3]<<16) | (bin[pcur+4]<<24);
          if (pt === 2 && pl > 0) { // Content
            // Walk content Read1
            var ccur = pcur + 5;
            var cend = pcur + 5 + pl;
            while (ccur + 5 <= cend) {
              var ct = bin[ccur];
              var cl = bin[ccur+1] | (bin[ccur+2]<<8) | (bin[ccur+3]<<16) | (bin[ccur+4]<<24);
              if (ct === 5 && cl > 0) { // Run
                // Walk run Read1
                var rcur = ccur + 5;
                var rend = ccur + 5 + cl;
                while (rcur + 5 <= rend) {
                  var rt = bin[rcur];
                  var rl = bin[rcur+1] | (bin[rcur+2]<<8) | (bin[rcur+3]<<16) | (bin[rcur+4]<<24);
                  if (rt === 8 && rl > 0) { // Content inside run
                    // Scan for pptxDrawing (Read2 type 12)
                    var r2cur = rcur + 5;
                    var r2end = rcur + 5 + rl;
                    while (r2cur + 2 <= r2end) {
                      var r2t = bin[r2cur];
                      var r2lt = bin[r2cur+1];
                      if (r2t === 12 && r2lt === 6) { // pptxDrawing Variable
                        var vlen = bin[r2cur+2] | (bin[r2cur+3]<<8) | (bin[r2cur+4]<<16) | (bin[r2cur+5]<<24);
                        pptxStart = r2cur + 6; // Start of pptx data
                        pptxLen = vlen;
                        break;
                      }
                      // Skip this Read2 prop
                      var skip = 2;
                      if (r2lt === 1) skip = 3;
                      else if (r2lt === 4) skip = 6;
                      else if (r2lt === 6) {
                        var vl = bin[r2cur+2] | (bin[r2cur+3]<<8) | (bin[r2cur+4]<<16) | (bin[r2cur+5]<<24);
                        skip = 6 + vl;
                      }
                      r2cur += skip;
                    }
                  }
                  if (pptxStart >= 0) break;
                  rcur += 5 + rl;
                }
              }
              if (pptxStart >= 0) break;
              ccur += 5 + cl;
            }
          }
          if (pptxStart >= 0) break;
          pcur += 5 + pl;
        }
      }
      if (pptxStart >= 0) break;
      cur += 5 + l;
    }
    
    if (pptxStart < 0) return { error: 'pptxDrawing not found in document' };
    
    // Also extract the outer Read2 properties (Type, Extent) before pptxData
    // Go back to find the run content Read2 start
    // The pptxDrawing Read2 is inside run Content. Let me find all Read2 props in that content.
    
    // Extract pptx bytes
    var pptxBytes = [];
    for (var i = pptxStart; i < pptxStart + pptxLen && i < bin.length; i++) {
      pptxBytes.push(bin[i]);
    }
    
    // Also get the full drawing Read2 block (Type + Extent + PptxData)
    // Back up to find the start of the pptxDrawing Read2 item
    // The pptxDrawing item (type=12) is preceded by Type(0) and Extent(14) items
    // Let me find the run Content block and extract ALL Read2 from it
    
    return {
      success: true,
      pptxStart: pptxStart,
      pptxLen: pptxLen,
      pptxBytes: pptxBytes,
      binLen: bin.length,
    };
  });

  if (result.error) {
    console.log('Error:', result.error);
  } else {
    console.log('Found pptxDrawing at offset', result.pptxStart, 'len', result.pptxLen);
    const buf = Buffer.from(result.pptxBytes);
    fs.writeFileSync('/tmp/pptx_from_sdkjs.bin', buf);
    console.log('Saved', buf.length, 'bytes to /tmp/pptx_from_sdkjs.bin');
    
    // Hex dump first 100 bytes
    for (let i = 0; i < Math.min(buf.length, 100); i++) {
      if (i % 20 === 0) process.stdout.write(`\n${i.toString().padStart(4)}: `);
      process.stdout.write(buf[i].toString(16).padStart(2, '0') + ' ');
    }
    console.log();
  }

  await browser.close();
}

main().catch(e => { console.error(e); process.exit(1); });
