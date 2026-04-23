import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 3000));
const b64 = fs.readFileSync('/Users/sachin/Downloads/Chat Reaction.docx').toString('base64');
await page.evaluate(async (d) => {
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  await openDocx(a, api);
  
  // Trigger image loading for ALL image data URLs
  await new Promise(r => setTimeout(r, 2000));
  var ld = api.WordControl.m_oLogicDocument;
  for (var i = 0; i < ld.Content.length; i++) {
    var para = ld.Content[i];
    if (!para.Content) continue;
    for (var j = 0; j < para.Content.length; j++) {
      var run = para.Content[j];
      if (!run || !run.Content) continue;
      for (var k = 0; k < run.Content.length; k++) {
        var item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing && item.GraphicObj && item.GraphicObj.blipFill) {
          var src = item.GraphicObj.blipFill.RasterImageId;
          if (src && src.indexOf('data:') === 0) {
            api.ImageLoader.LoadImage(src, 1);
          }
        }
      }
    }
  }
  await new Promise(r => setTimeout(r, 2000));
  // Scroll to paragraph 9 area
  ld.MoveCursorToStartPos(false);
  for (var i = 0; i < 9; i++) {
    ld.MoveCursorDown(false);
  }
  ld.Recalculate();
  api.WordControl.OnResize(true);
}, b64);
await new Promise(r => setTimeout(r, 2000));
await page.screenshot({ path: '/tmp/docy_chat_with_images.png' });
console.log('Done');
await browser.close();
