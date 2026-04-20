import puppeteer from 'puppeteer';
import fs from 'fs';

// Find a test file with images
const FILES = [
  '/Users/sachin/Downloads/SDS_ANTI-T..._ZH.docx', // has Drawing nodes
];

async function main() {
  const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 3000));

  // Instead of capturing from sdkjs, let me create a minimal test:
  // Make sdkjs create a simple image and then serialize it to see the binary format
  const result = await page.evaluate(() => {
    try {
      const api = window._api || window.editor;
      const ld = api.WordControl.m_oLogicDocument;
      
      // Create a 1x1 red pixel PNG as data URL
      var canvas = document.createElement('canvas');
      canvas.width = 100;
      canvas.height = 100;
      var ctx = canvas.getContext('2d');
      ctx.fillStyle = 'red';
      ctx.fillRect(0, 0, 100, 100);
      var dataUrl = canvas.toDataURL('image/png');
      
      // Try to insert image into sdkjs
      // First create a ParaDrawing with an image
      var drawing = new AscCommonWord.ParaDrawing(50, 50, null, ld.DrawingDocument, ld, null);
      var imageObj = AscFormat.DrawingObjectsController.prototype.createImage(dataUrl, 0, 0, 50, 50);
      if (imageObj) {
        drawing.Set_GraphicObject(imageObj);
        drawing.setExtent(50, 50);
        drawing.Set_DrawingType(drawing_Inline);
        
        // Now serialize it
        var memory = new AscCommon.CMemory();
        var bs = new AscCommon.BinaryCommonWriter(memory);
        var writer = new AscCommonWord.BinaryDocumentTableWriter(memory, ld, {}, {}, null, null, null);
        
        // Capture the pptxDrawing binary
        memory.Seek(0);
        writer.WriteGraphicObj(imageObj);
        
        var bytes = [];
        for (var i = 0; i < memory.pos; i++) {
          bytes.push(memory.data[i]);
        }
        return { success: true, bytes: bytes, len: bytes.length };
      }
      return { success: false, error: 'Could not create image object' };
    } catch(e) {
      return { success: false, error: e.message, stack: e.stack };
    }
  });

  console.log('Result:', JSON.stringify(result).slice(0, 500));
  
  if (result.success && result.bytes) {
    // Save the binary for analysis
    const buf = Buffer.from(result.bytes);
    fs.writeFileSync('/tmp/pptx_drawing_sample.bin', buf);
    console.log('Saved', buf.length, 'bytes to /tmp/pptx_drawing_sample.bin');
    // Hex dump first 200 bytes
    console.log('Hex dump:');
    for (let i = 0; i < Math.min(buf.length, 200); i++) {
      if (i % 20 === 0) process.stdout.write(`\n  ${i.toString().padStart(4)}: `);
      process.stdout.write(buf[i].toString(16).padStart(2, '0') + ' ');
    }
    console.log();
  }

  await browser.close();
}
main().catch(e => { console.error(e); process.exit(1); });
