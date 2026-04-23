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
  let drawings = [];
  for (let i = 0; i < ld.Content.length; i++) {
    const para = ld.Content[i];
    if (!para.Content) continue;
    let paraDrawings = 0;
    for (let j = 0; j < para.Content.length; j++) {
      const run = para.Content[j];
      if (!run || !run.Content) continue;
      for (let k = 0; k < run.Content.length; k++) {
        if (run.Content[k] instanceof AscCommonWord.ParaDrawing) paraDrawings++;
      }
    }
    if (paraDrawings > 0) drawings.push({ para: i, count: paraDrawings });
  }
  return drawings;
}, b64);
console.log('Paragraphs with drawings:');
for (const d of r) console.log(`  Para ${d.para}: ${d.count} drawing(s)`);
console.log('Total:', r.reduce((s, d) => s + d.count, 0));
await browser.close();
