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
await new Promise(r => setTimeout(r, 3000));
// Scroll down to where images should be
await page.evaluate(() => {
  var el = document.getElementById('editor_sdk');
  if (el) el.scrollTop = 2000;
});
await new Promise(r => setTimeout(r, 1000));
await page.screenshot({ path: '/tmp/docy_chat_scrolled.png' });
console.log('Scrolled screenshot saved');
await browser.close();
