import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
const logs = [];
page.on('console', msg => logs.push(msg.text()));
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 3000));
const b64 = fs.readFileSync('/Users/sachin/Downloads/Chat Reaction.docx').toString('base64');
await page.evaluate(async (d) => {
  // Patch ReadDrawing to log
  var orig = AscCommon.pptx_content_loader.ReadDrawing;
  AscCommon.pptx_content_loader.ReadDrawing = function(oThis, stream, doc, oParaDrawing) {
    var result = orig.call(this, oThis, stream, doc, oParaDrawing);
    console.log('[IMG-DBG] ReadDrawing result:', result ? result.getObjectType() : 'null', 
      'blipFill:', result && result.blipFill ? (result.blipFill.RasterImageId || '').substring(0,60) : 'none');
    return result;
  };
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  await openDocx(a, api);
  AscCommon.pptx_content_loader.ReadDrawing = orig;
}, b64);
await new Promise(r => setTimeout(r, 1000));
const imgLogs = logs.filter(l => l.includes('IMG-DBG'));
console.log('Image ReadDrawing calls:', imgLogs.length);
for (const l of imgLogs.slice(0, 5)) console.log(' ', l);
await browser.close();
