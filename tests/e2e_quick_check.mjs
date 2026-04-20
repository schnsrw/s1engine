import puppeteer from 'puppeteer';
import fs from 'fs';
const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
const errors = [];
page.on('console', msg => { if (msg.type() === 'error' || msg.text().includes('error') || msg.text().includes('Error')) errors.push(msg.text()); });
page.on('pageerror', err => errors.push('PAGE: ' + err.message));
await page.goto('http://localhost:8080', { waitUntil: 'networkidle0', timeout: 30000 });
await new Promise(r => setTimeout(r, 3000));
// Use text-only.docx - no images
const b64 = fs.readFileSync(process.env.CARGO_MANIFEST_DIR ? process.env.CARGO_MANIFEST_DIR + '/../s1engine/tests/fixtures/text-only.docx' : 'crates/s1engine/tests/fixtures/text-only.docx').toString('base64');
await page.evaluate(async (d) => {
  const b = atob(d), a = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
  const { openDocx } = await import('./adapter.js');
  const api = window._api || window.editor;
  await openDocx(a, api);
  console.log('ELEMENTS: ' + (api.WordControl.m_oLogicDocument ? api.WordControl.m_oLogicDocument.Content.length : 0));
}, b64);
await new Promise(r => setTimeout(r, 1000));
console.log('Errors:', errors.length);
for (const e of errors.slice(0, 5)) console.log(' ', e);
await browser.close();
