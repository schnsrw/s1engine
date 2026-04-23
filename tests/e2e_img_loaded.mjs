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
  
  // Wait for deferred recalculate
  await new Promise(r => setTimeout(r, 2000));
  
  const ld = api.WordControl.m_oLogicDocument;
  // Check image loading state
  var results = [];
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para.Content) continue;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run || !run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        const item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing && item.GraphicObj) {
          var go = item.GraphicObj;
          var bf = go.blipFill;
          var imgSrc = bf ? bf.RasterImageId : null;
          // Check if image is in the global image store
          var imgFromStore = null;
          if (imgSrc && AscCommon.g_oDocumentUrls) {
            imgFromStore = AscCommon.g_oDocumentUrls.getImageUrl(imgSrc);
          }
          // Check if loaded in image loader
          var isLoaded = false;
          if (api.ImageLoader && api.ImageLoader.map_image_index) {
            isLoaded = !!api.ImageLoader.map_image_index[imgSrc];
          }
          // Check the actual Image element
          var imgElement = null;
          if (api.ImageLoader && api.ImageLoader.map_image_index) {
            imgElement = api.ImageLoader.map_image_index[imgSrc];
          }
          results.push({
            para: i,
            srcLen: imgSrc ? imgSrc.length : 0,
            srcPrefix: imgSrc ? imgSrc.substring(0, 30) : null,
            fromStore: imgFromStore ? imgFromStore.substring(0, 30) : null,
            isLoaded,
            imgStatus: imgElement ? (imgElement.Image ? 'has_img' : 'no_img') : 'not_in_loader',
            spPrOk: go.spPr && go.spPr.xfrm ? true : false,
          });
        }
      }
    }
  }
  return results;
}, b64);
for (const x of r) console.log(JSON.stringify(x));
await browser.close();
