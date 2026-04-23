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

    var c = document.createElement('canvas');
    c.width = 10; c.height = 10;
    var ctx = c.getContext('2d');
    ctx.fillStyle = '#ff0000';
    ctx.fillRect(0, 0, 10, 10);
    var dataUrl = c.toDataURL('image/png');

    ld.MoveCursorToStartPos(false);
    AscCommon.History.TurnOff();
    AscCommon.g_oIdCounter.Set_Load(true);
    var drawing = new AscCommonWord.ParaDrawing(25.4, 25.4, null, ld.DrawingDocument, ld, ld.Content[0]);
    drawing.Set_DrawingType(0x01);
    drawing.setExtent(25.4, 25.4);
    var imageObj = AscFormat.DrawingObjectsController.prototype.createImage(dataUrl, 0, 0, 25.4, 25.4);
    drawing.Set_GraphicObject(imageObj);
    imageObj.setParent(drawing);
    var run = new AscCommonWord.ParaRun(ld.Content[0], false);
    run.AddToContent(0, drawing, false);
    ld.Content[0].AddToContent(0, run);
    AscCommon.g_oIdCounter.Set_Load(false);
    AscCommon.History.TurnOn();

    // Use BinaryFileWriter to get full DOCY then extract pptxDrawing
    var writer = new AscCommonWord.BinaryFileWriter(ld, {});
    var docyStr = writer.Write(false, true);
    
    // Decode
    var parts = docyStr.split(';');
    var binStr = atob(parts[3]);
    var bin = new Uint8Array(binStr.length);
    for (var i = 0; i < binStr.length; i++) bin[i] = binStr.charCodeAt(i);
    
    // Find doc table
    var cnt = bin[0];
    var docOff = -1;
    for (var i = 0; i < cnt; i++) {
      var p = 1 + i * 5;
      if (bin[p] === 6) docOff = bin[p+1] | (bin[p+2]<<8) | (bin[p+3]<<16) | (bin[p+4]<<24);
    }
    
    var docLen = bin[docOff] | (bin[docOff+1]<<8) | (bin[docOff+2]<<16) | (bin[docOff+3]<<24);
    
    // Dump first paragraph content hex for analysis
    var cur = docOff + 4;
    var parType = bin[cur];
    var parLen = bin[cur+1] | (bin[cur+2]<<8) | (bin[cur+3]<<16) | (bin[cur+4]<<24);
    
    // Get full paragraph bytes
    var parBytes = [];
    for (var i = cur; i < cur + 5 + parLen && i < bin.length; i++) {
      parBytes.push(bin[i]);
    }
    
    return { parLen, parBytes: parBytes.slice(0, 1000), totalBin: bin.length };
  });

  if (result.error) {
    console.log('Error:', result.error);
  } else {
    console.log('First paragraph:', result.parLen, 'bytes');
    const buf = Buffer.from(result.parBytes);
    fs.writeFileSync('/tmp/para_with_image.bin', buf);
    
    // Decode the paragraph structure
    // Para Read1: pPr(1) + Content(2)
    let pos = 5; // skip paragraph header
    while (pos + 5 <= buf.length) {
      const t = buf[pos];
      const l = buf[pos+1] | (buf[pos+2]<<8) | (buf[pos+3]<<16) | (buf[pos+4]<<24);
      console.log(`  @${pos}: type=${t} len=${l}`);
      
      if (t === 2 && l > 0) { // Content
        // Walk content Read1 items
        let ccur = pos + 5;
        const cend = pos + 5 + l;
        while (ccur + 5 <= cend) {
          const ct = buf[ccur];
          const cl = buf[ccur+1] | (buf[ccur+2]<<8) | (buf[ccur+3]<<16) | (buf[ccur+4]<<24);
          console.log(`    @${ccur}: type=${ct} len=${cl}`);
          
          if (ct === 5 && cl > 0) { // Run
            let rcur = ccur + 5;
            const rend = ccur + 5 + cl;
            while (rcur + 5 <= rend) {
              const rt = buf[rcur];
              const rl = buf[rcur+1] | (buf[rcur+2]<<8) | (buf[rcur+3]<<16) | (buf[rcur+4]<<24);
              console.log(`      @${rcur}: run_type=${rt} len=${rl}`);
              
              if (rt === 8 && rl > 0) { // Run Content
                // Scan Read2 props inside
                let r2 = rcur + 5;
                const r2end = rcur + 5 + rl;
                while (r2 + 2 <= r2end) {
                  const r2t = buf[r2];
                  const r2lt = buf[r2+1];
                  let vl = 0;
                  if (r2lt === 1) vl = 1;
                  else if (r2lt === 4) vl = 4;
                  else if (r2lt === 6) {
                    vl = buf[r2+2] | (buf[r2+3]<<8) | (buf[r2+4]<<16) | (buf[r2+5]<<24);
                    console.log(`        @${r2}: READ2 type=${r2t} lenType=${r2lt} varLen=${vl}`);
                    if (r2t === 12) { // pptxDrawing!
                      console.log('        *** FOUND pptxDrawing! ***');
                      // Save pptx bytes
                      const pptxBytes = [];
                      for (let k = r2 + 6; k < r2 + 6 + vl && k < buf.length; k++) pptxBytes.push(buf[k]);
                      require('fs').writeFileSync('/tmp/pptx_real.bin', Buffer.from(pptxBytes));
                      console.log('        Saved', pptxBytes.length, 'bytes to /tmp/pptx_real.bin');
                      // Hex first 60 bytes
                      for (let k = 0; k < Math.min(60, pptxBytes.length); k++) {
                        if (k % 20 === 0) process.stdout.write(`\n        ${k}: `);
                        process.stdout.write(pptxBytes[k].toString(16).padStart(2,'0') + ' ');
                      }
                      console.log();
                    }
                    r2 += 6 + vl;
                    continue;
                  }
                  r2 += 2 + vl;
                }
              }
              rcur += 5 + rl;
            }
          }
          ccur += 5 + cl;
        }
      }
      pos += 5 + l;
    }
  }
  await browser.close();
}
main().catch(e => { console.error(e); process.exit(1); });
