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
  const ld = api.WordControl.m_oLogicDocument;
  // Find ParaDrawing objects
  let drawings = [];
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para.Content) continue;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run || !run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        const item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing) {
          drawings.push({
            para: i, drawType: item.DrawingType,
            hasGO: !!item.GraphicObj,
            goType: item.GraphicObj ? item.GraphicObj.getObjectType() : null,
            hasBF: item.GraphicObj && item.GraphicObj.blipFill ? true : false,
            extent: item.Extent ? {w: item.Extent.W, h: item.Extent.H} : null,
          });
        }
      }
    }
  }
  return { total: drawings.length, drawings };
}, b64);
console.log('ParaDrawings:', r.total);
for (const d of r.drawings) console.log(' ', JSON.stringify(d));
await browser.close();
