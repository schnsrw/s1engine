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
  await new Promise(r => setTimeout(r, 4000));
  const ld = api.WordControl.m_oLogicDocument;
  let results = [];
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para || !para.Content) continue;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run || !run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        const item = run.Content[k];
        if (item instanceof AscCommonWord.ParaDrawing) {
          results.push({
            para: i,
            drawingType: item.DrawingType, // 1=inline
            extentW: item.Extent ? item.Extent.W : null,
            extentH: item.Extent ? item.Extent.H : null,
            posX: item.X, posY: item.Y,
            pageNum: item.PageNum,
            goExtX: item.GraphicObj && item.GraphicObj.spPr && item.GraphicObj.spPr.xfrm ? item.GraphicObj.spPr.xfrm.extX : null,
            goExtY: item.GraphicObj && item.GraphicObj.spPr && item.GraphicObj.spPr.xfrm ? item.GraphicObj.spPr.xfrm.extY : null,
            wrappingType: item.wrappingType,
          });
        }
      }
    }
  }
  return results;
}, b64);
for (const x of r) console.log(JSON.stringify(x));
await browser.close();
