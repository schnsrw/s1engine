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
  
  // Check DrawingObjects controller for floating drawings
  var dd = ld.DrawingObjects;
  var floatingCount = dd ? dd.getDrawingArray().length : -1;
  
  // Check each drawing's type
  var drawingDetails = [];
  if (dd) {
    var arr = dd.getDrawingArray();
    for (var i = 0; i < arr.length; i++) {
      var d = arr[i];
      drawingDetails.push({
        type: d.DrawingType,
        isInline: d.DrawingType === 1,
        hasGraphic: !!d.GraphicObj,
        parent: d.Parent ? d.Parent.constructor.name : 'none',
      });
    }
  }
  
  return { floatingCount, drawingDetails };
}, b64);
console.log('Drawing objects:', r.floatingCount);
for (const d of r.drawingDetails) {
  console.log('  type=' + d.type + ' inline=' + d.isInline + ' graphic=' + d.hasGraphic + ' parent=' + d.parent);
}
await browser.close();
