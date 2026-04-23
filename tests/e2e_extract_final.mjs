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
    ctx.fillStyle = '#ff0000'; ctx.fillRect(0, 0, 10, 10);
    var dataUrl = c.toDataURL('image/png');
    ld.MoveCursorToStartPos(false);
    AscCommon.History.TurnOff(); AscCommon.g_oIdCounter.Set_Load(true);
    var drawing = new AscCommonWord.ParaDrawing(25.4, 25.4, null, ld.DrawingDocument, ld, ld.Content[0]);
    drawing.Set_DrawingType(0x01); drawing.setExtent(25.4, 25.4);
    var imageObj = AscFormat.DrawingObjectsController.prototype.createImage(dataUrl, 0, 0, 25.4, 25.4);
    drawing.Set_GraphicObject(imageObj); imageObj.setParent(drawing);
    var run = new AscCommonWord.ParaRun(ld.Content[0], false);
    run.AddToContent(0, drawing, false);
    ld.Content[0].AddToContent(0, run);
    AscCommon.g_oIdCounter.Set_Load(false); AscCommon.History.TurnOn();
    var writer = new AscCommonWord.BinaryFileWriter(ld, {});
    var docyStr = writer.Write(false, true);
    var parts = docyStr.split(';');
    var binStr = atob(parts[3]);
    var bin = new Uint8Array(binStr.length);
    for (var i = 0; i < binStr.length; i++) bin[i] = binStr.charCodeAt(i);
    // Find doc table
    var cnt = bin[0], docOff = -1;
    for (var i = 0; i < cnt; i++) { var p = 1+i*5; if (bin[p]===6) docOff = bin[p+1]|(bin[p+2]<<8)|(bin[p+3]<<16)|(bin[p+4]<<24); }
    var docLen = bin[docOff]|(bin[docOff+1]<<8)|(bin[docOff+2]<<16)|(bin[docOff+3]<<24);
    // Get paragraph bytes
    var cur = docOff + 4;
    var pLen = bin[cur+1]|(bin[cur+2]<<8)|(bin[cur+3]<<16)|(bin[cur+4]<<24);
    // Extract Run Content bytes (the whole thing from the pptxDrawing Read1 item)
    // Walk: Par(Read1) > Content(Read1 type=2) > Run(Read1 type=5) > RunContent(Read1 type=8)
    var pcur = cur + 5;
    while (pcur + 5 <= cur + 5 + pLen) {
      var pt = bin[pcur], pl = bin[pcur+1]|(bin[pcur+2]<<8)|(bin[pcur+3]<<16)|(bin[pcur+4]<<24);
      if (pt === 2) { // Content
        var ccur = pcur + 5, cend = pcur + 5 + pl;
        while (ccur + 5 <= cend) {
          var ct = bin[ccur], cl = bin[ccur+1]|(bin[ccur+2]<<8)|(bin[ccur+3]<<16)|(bin[ccur+4]<<24);
          if (ct === 5) { // Run
            var rcur = ccur + 5, rend = ccur + 5 + cl;
            while (rcur + 5 <= rend) {
              var rt = bin[rcur], rl = bin[rcur+1]|(bin[rcur+2]<<8)|(bin[rcur+3]<<16)|(bin[rcur+4]<<24);
              if (rt === 8) { // Run Content - this has the pptxDrawing as Read1
                // Inside: first Read1 item should be pptxDrawing(12)
                var drawCur = rcur + 5;
                var drawType = bin[drawCur];
                var drawLen = bin[drawCur+1]|(bin[drawCur+2]<<8)|(bin[drawCur+3]<<16)|(bin[drawCur+4]<<24);
                if (drawType === 12) { // pptxDrawing!
                  // Extract the FULL pptxDrawing content (Read2 props: Type, Extent, PptxData)
                  var drawBytes = [];
                  for (var k = drawCur + 5; k < drawCur + 5 + drawLen && k < bin.length; k++) drawBytes.push(bin[k]);
                  return { drawType, drawLen, drawBytes: drawBytes.slice(0, 800) };
                }
              }
              rcur += 5 + rl;
            }
          }
          ccur += 5 + cl;
        }
      }
      pcur += 5 + pl;
    }
    return { error: 'pptxDrawing not found' };
  });
  if (result.error) { console.log('Error:', result.error); }
  else {
    console.log('pptxDrawing type=' + result.drawType + ' len=' + result.drawLen);
    const buf = Buffer.from(result.drawBytes);
    fs.writeFileSync('/tmp/pptx_draw_content.bin', buf);
    // Decode Read2 props
    let pos = 0;
    while (pos + 2 <= buf.length) {
      const t = buf[pos], lt = buf[pos+1];
      let vl = 0, shift = 2;
      if (lt === 0) { vl = 0; }
      else if (lt === 1) { vl = 1; shift = 3; }
      else if (lt === 4) { vl = 4; shift = 6; }
      else if (lt === 6) { vl = buf[pos+2]|(buf[pos+3]<<8)|(buf[pos+4]<<16)|(buf[pos+5]<<24); shift = 6 + vl; }
      else break;
      console.log(`  @${pos}: type=${t} lenType=${lt} valLen=${vl}`);
      if (t === 1 && lt === 6) { // PptxData
        console.log('  PptxData hex (first 60):');
        for (let k = pos+6; k < pos+6+Math.min(60, vl); k++) {
          if ((k-pos-6) % 20 === 0) process.stdout.write('\n    ');
          process.stdout.write(buf[k].toString(16).padStart(2,'0') + ' ');
        }
        console.log();
        // Save just the PptxData
        const pptx = buf.subarray(pos+6, pos+6+vl);
        fs.writeFileSync('/tmp/pptx_data_only.bin', pptx);
        console.log('  Saved', pptx.length, 'PptxData bytes');
      }
      pos += shift;
    }
  }
  await browser.close();
}
main().catch(e => { console.error(e); process.exit(1); });
