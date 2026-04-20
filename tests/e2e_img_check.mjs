import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
const logs = [];
page.on('console', msg => logs.push(msg.text()));
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 3000));
const b64 = fs.readFileSync('/Users/sachin/Downloads/Chat Reaction.docx').toString('base64');
const result = await page.evaluate(async (d) => {
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  const doc = await openDocx(a, api);
  const ld = api.WordControl.m_oLogicDocument;
  
  // Find all drawings
  let drawings = [];
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para.Content) continue;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        const item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing) {
          const go = item.GraphicObj;
          drawings.push({
            para: i,
            type: go ? go.getObjectType() : 'none',
            hasBlipFill: go && go.blipFill ? true : false,
            rasterImageId: go && go.blipFill ? go.blipFill.RasterImageId : null,
            extent: item.Extent ? { w: item.Extent.W, h: item.Extent.H } : null,
          });
        }
      }
    }
  }
  return { count: drawings.length, drawings: drawings.slice(0, 5) };
}, b64);
console.log(JSON.stringify(result, null, 2));
await browser.close();
