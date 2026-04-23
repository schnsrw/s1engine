import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 3000));
const b64 = fs.readFileSync('/Users/sachin/Downloads/Chat Reaction.docx').toString('base64');
await page.evaluate(async (d) => {
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  await openDocx(a, api);
}, b64);
// Wait for render + scroll to top
await new Promise(r => setTimeout(r, 4000));
await page.evaluate(() => {
  var api = window._api || window.editor;
  if (api && api.WordControl && api.WordControl.m_oLogicDocument) {
    api.WordControl.m_oLogicDocument.MoveCursorToStartPos(false);
  }
});
await new Promise(r => setTimeout(r, 1000));
await page.screenshot({ path: '/tmp/docy_chat_top.png' });
// Also scroll to page 2
await page.evaluate(() => {
  var api = window._api || window.editor;
  if (api) api.GoToPage(1);
});
await new Promise(r => setTimeout(r, 1000));
await page.screenshot({ path: '/tmp/docy_chat_page2.png' });
console.log('Screenshots saved');
await browser.close();
