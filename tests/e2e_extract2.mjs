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

    // Now write just the pptxDrawing using the writer directly
    var memory = new AscCommon.CMemory();
    memory.Init(65536);
    
    var bs = new AscCommon.BinaryCommonWriter(memory);
    var dtw = new AscCommonWord.Binary_DocumentTableWriter(memory, ld, {}, {}, null, null, null);
    
    // Write the image drawing
    memory.Seek(0);
    dtw.WriteImage(drawing);
    
    var bytes = [];
    for (var i = 0; i < memory.pos; i++) bytes.push(memory.data[i]);
    return { len: bytes.length, bytes: bytes.slice(0, 500) };
  });

  if (result.error) {
    console.log('Error:', result.error);
  } else {
    console.log('Image drawing:', result.len, 'bytes');
    const buf = Buffer.from(result.bytes);
    fs.writeFileSync('/tmp/pptx_drawing_real.bin', buf);
    for (let i = 0; i < Math.min(buf.length, 200); i++) {
      if (i % 20 === 0) process.stdout.write(`\n${i.toString().padStart(4)}: `);
      process.stdout.write(buf[i].toString(16).padStart(2, '0') + ' ');
    }
    console.log();
  }
  await browser.close();
}
main().catch(e => { console.error(e); process.exit(1); });
