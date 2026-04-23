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
await new Promise(r => setTimeout(r, 5000));

// Take 3 screenshots at different scroll positions
for (let page_num = 0; page_num < 3; page_num++) {
  await page.evaluate((pn) => {
    var api = window._api || window.editor;
    if (api && api.WordControl) {
      var ld = api.WordControl.m_oLogicDocument;
      if (ld && ld.Pages && pn < ld.Pages.length) {
        api.WordControl.m_oScrollVerApi.scrollToY(ld.Pages[pn].Pos.Y * api.WordControl.m_nZoomValue / 100);
      }
    }
  }, page_num);
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: `/tmp/docy_page${page_num + 1}.png` });
  console.log(`Page ${page_num + 1} saved`);
}
await browser.close();
