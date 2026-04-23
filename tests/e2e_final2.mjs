import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });
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
// Wait for deferred recalculate to complete
await new Promise(r => setTimeout(r, 5000));
// Move cursor to a paragraph with content
await page.evaluate(() => {
  var api = window._api || window.editor;
  var ld = api.WordControl.m_oLogicDocument;
  if (ld) {
    ld.MoveCursorToStartPos(false);
    // Move down to paragraph 9 (which has an image)
    for (var i = 0; i < 12; i++) ld.MoveCursorDown(false);
  }
});
await new Promise(r => setTimeout(r, 1000));
await page.screenshot({ path: '/tmp/docy_final_with_recalc.png' });
console.log('Done');
await browser.close();
