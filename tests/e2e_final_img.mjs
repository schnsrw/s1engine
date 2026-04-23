import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });
const logs = [];
page.on('console', msg => logs.push(msg.text()));
page.on('pageerror', err => logs.push('PAGEERROR: ' + err.message));
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 4000));
const b64 = fs.readFileSync('/Users/sachin/Downloads/Chat Reaction.docx').toString('base64');
await page.evaluate(async (d) => {
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  await openDocx(a, api);
}, b64);
// Wait for ALL deferred operations
await new Promise(r => setTimeout(r, 8000));

// Check state
const state = await page.evaluate(() => {
  const api = window._api || window.editor;
  const ld = api.WordControl.m_oLogicDocument;
  let imgInfo = [];
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para.Content) continue;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run || !run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        const item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing && item.GraphicObj) {
          const go = item.GraphicObj;
          const src = go.blipFill ? go.blipFill.RasterImageId : null;
          const fullSrc = src ? AscCommon.getFullImageSrc2(src) : null;
          const cached = fullSrc && api.ImageLoader ? api.ImageLoader.map_image_index[fullSrc] : null;
          imgInfo.push({
            para: i,
            hasCalcGeom: !!go.calcGeometry,
            hasBrush: !!go.brush,
            hasTransform: !!go.transform,
            srcIsDataUrl: src ? src.startsWith('data:') : false,
            fullSrcSame: fullSrc === src,
            inCache: !!cached,
            cacheStatus: cached ? cached.Status : null,
            cacheHasImg: cached && cached.Image ? true : false,
          });
        }
      }
    }
  }
  return imgInfo;
});
console.log('Image states:');
for (const s of state) console.log(JSON.stringify(s));

// Log errors
const errs = logs.filter(l => l.includes('error') || l.includes('Error') || l.includes('PAGEERROR'));
if (errs.length) { console.log('\nErrors:'); for (const e of errs.slice(0,5)) console.log(' ', e); }

await page.screenshot({ path: '/tmp/docy_final_test.png' });
console.log('\nScreenshot saved');
await browser.close();
