import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: false, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 900 });
const logs = [];
page.on('console', msg => logs.push('[' + msg.type() + '] ' + msg.text()));
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
// Wait for deferred timeout to fire
await new Promise(r => setTimeout(r, 5000));
// Print all adapter logs
for (const l of logs.filter(l => l.includes('adapter') || l.includes('Loading') || l.includes('error'))) {
  console.log(l);
}
await page.screenshot({ path: '/tmp/docy_browser_sim.png' });
console.log('Screenshot saved');
await browser.close();
