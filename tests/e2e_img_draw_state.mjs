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
  await new Promise(r => setTimeout(r, 3000));
  const ld = api.WordControl.m_oLogicDocument;
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
          return {
            para: i,
            hasTransform: !!go.transform,
            hasCalcGeom: !!go.calcGeometry,
            hasBrush: !!go.brush,
            hasPen: !!go.pen,
            hasBlipFill: !!go.blipFill,
            bWordShape: go.bWordShape,
            objType: go.getObjectType(),
            extX: go.spPr && go.spPr.xfrm ? go.spPr.xfrm.extX : null,
            extY: go.spPr && go.spPr.xfrm ? go.spPr.xfrm.extY : null,
            recalcNeeded: go.recalcInfo ? go.recalcInfo.recalculate : null,
          };
        }
      }
    }
  }
  return { error: 'no drawing' };
}, b64);
console.log(JSON.stringify(r, null, 2));
await browser.close();
