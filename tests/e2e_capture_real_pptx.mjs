import puppeteer from 'puppeteer';
import fs from 'fs';

async function main() {
  const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 3000));

  // Step 1: Insert a real image into sdkjs using its own API
  const result = await page.evaluate(async () => {
    const api = window._api || window.editor;
    const ld = api.WordControl.m_oLogicDocument;
    if (!ld) return { error: 'no logicDoc' };

    // Create a small red square PNG as data URL  
    var c = document.createElement('canvas');
    c.width = 50; c.height = 50;
    var ctx = c.getContext('2d');
    ctx.fillStyle = '#ff0000';
    ctx.fillRect(0, 0, 50, 50);
    ctx.fillStyle = '#ffffff';
    ctx.font = '20px Arial';
    ctx.fillText('IMG', 5, 35);
    var dataUrl = c.toDataURL('image/png');

    // Use sdkjs API to insert inline image
    try {
      // Move to start
      ld.MoveCursorToStartPos(false);
      
      // Create drawing via the proper sdkjs way
      AscCommon.History.TurnOff();
      AscCommon.g_oIdCounter.Set_Load(true);
      
      var oImagePr = new Asc.asc_CImgProperty();
      oImagePr.put_Width(25.4); // 1 inch in mm
      oImagePr.put_Height(25.4);
      oImagePr.put_WrappingStyle(0); // inline
      
      // Insert using the API
      api.asc_addImage(dataUrl, oImagePr);
      
      AscCommon.g_oIdCounter.Set_Load(false);
      AscCommon.History.TurnOn();
      
      await new Promise(r => setTimeout(r, 1000));
      
      // Now serialize to DOCY binary via sdkjs's own writer
      var memory = new AscCommon.CMemory();
      memory.Init(1024 * 1024);
      
      var openParams = {};
      var writer = new AscCommonWord.BinaryFileWriter(ld, openParams);
      writer.Write(false, true);  // Write to memory
      
      // Get the binary
      var binData = writer.memory.GetData();
      var binLen = writer.memory.GetCurPosition();
      
      // Find pptxDrawing in the binary (look for type 12 in run content)
      // Actually, let's just get the full DOCY string
      var docyStr = writer.GetResult();
      
      return { 
        success: true, 
        docyLen: docyStr ? docyStr.length : 0,
        docy: docyStr ? docyStr.substring(0, 100) : null,
      };
    } catch(e) {
      return { error: e.message, stack: e.stack ? e.stack.split('\n').slice(0,3).join('\n') : '' };
    }
  });

  console.log(JSON.stringify(result, null, 2));
  await browser.close();
}

main().catch(e => { console.error(e); process.exit(1); });
