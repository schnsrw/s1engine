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
await new Promise(r => setTimeout(r, 5000));
// Print adapter logs
for (const l of logs.filter(l => l.includes('adapter') || l.includes('Loading'))) console.log(l);
await page.screenshot({ path: '/tmp/docy_chat_FINAL.png' });
console.log('Done');
await browser.close();
