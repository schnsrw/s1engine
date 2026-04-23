import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 3000));
const b64 = fs.readFileSync('/Users/sachin/Downloads/Chat Reaction.docx').toString('base64');
const r = await page.evaluate(async (d) => {
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  await openDocx(a, api);
  await new Promise(r => setTimeout(r, 2000));
  
  const ld = api.WordControl.m_oLogicDocument;
  // Find first image and check URL resolution
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para.Content) continue;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run || !run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        const item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing && item.GraphicObj && item.GraphicObj.blipFill) {
          var src = item.GraphicObj.blipFill.RasterImageId;
          var resolved = AscCommon.g_oDocumentUrls.getImageUrl(src);
          var img = api.ImageLoader.map_image_index[src];
          var imgLoaded = img && img.Image && img.Image.complete;
          var imgW = img && img.Image ? img.Image.naturalWidth : 0;
          return {
            para: i,
            srcPrefix: src.substring(0, 40),
            resolvedPrefix: resolved ? resolved.substring(0, 40) : null,
            resolvedIsDataUrl: resolved && resolved.indexOf('data:') === 0,
            imgLoaded, imgW,
            hasSpPr: !!item.GraphicObj.spPr,
            hasXfrm: item.GraphicObj.spPr && item.GraphicObj.spPr.xfrm ? true : false,
            extCx: item.GraphicObj.spPr && item.GraphicObj.spPr.xfrm ? item.GraphicObj.spPr.xfrm.extX : null,
          };
        }
      }
    }
  }
  return { error: 'no image found' };
}, b64);
console.log(JSON.stringify(r, null, 2));
await browser.close();
