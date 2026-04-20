import puppeteer from 'puppeteer';
import fs from 'fs';

async function main() {
  const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 3000));

  const result = await page.evaluate(() => {
    try {
      // Create a minimal pptx binary using pptx_content_writer directly
      var memory = new AscCommon.CMemory();
      memory.Init(65536);
      
      var pw = AscCommon.pptx_content_writer;
      if (!pw) return { error: 'no pptx_content_writer' };
      
      // Create a minimal CImageShape  
      var imgShape = new AscFormat.CImageShape();
      imgShape.setBDeleted(false);
      
      // Set up spPr with transform
      var spPr = new AscFormat.CSpPr();
      var xfrm = new AscFormat.CXfrm();
      xfrm.setOffX(0);
      xfrm.setOffY(0);
      xfrm.setExtX(50); // 50mm
      xfrm.setExtY(50); // 50mm
      spPr.setXfrm(xfrm);
      imgShape.setSpPr(spPr);
      
      // Create a 10x10 red PNG
      var canvas = document.createElement('canvas');
      canvas.width = 10; canvas.height = 10;
      var ctx = canvas.getContext('2d');
      ctx.fillStyle = 'red';
      ctx.fillRect(0, 0, 10, 10);
      var dataUrl = canvas.toDataURL('image/png');
      
      // Set blipFill
      var blipFill = new AscFormat.CBlipFill();
      blipFill.setRasterImageId(dataUrl);
      imgShape.setBlipFill(blipFill);
      
      // Serialize using pptx_content_writer
      memory.Seek(0);
      pw.WriteDrawing(memory, imgShape, null, null, null, null, null);
      
      var bytes = [];
      for (var i = 0; i < memory.pos; i++) {
        bytes.push(memory.data[i]);
      }
      return { success: true, len: bytes.length, bytes: bytes.slice(0, 500) };
    } catch(e) {
      return { error: e.message, stack: e.stack.split('\n').slice(0, 5).join('\n') };
    }
  });

  if (result.error) {
    console.log('Error:', result.error);
    console.log(result.stack);
  } else if (result.success) {
    console.log('Captured', result.len, 'bytes of pptxDrawing');
    const buf = Buffer.from(result.bytes);
    fs.writeFileSync('/tmp/pptx_sample.bin', buf);
    // Hex dump
    for (let i = 0; i < Math.min(buf.length, 300); i++) {
      if (i % 20 === 0) process.stdout.write(`\n${i.toString().padStart(4)}: `);
      process.stdout.write(buf[i].toString(16).padStart(2, '0') + ' ');
    }
    console.log();
  }

  await browser.close();
}
main().catch(e => { console.error(e); process.exit(1); });
