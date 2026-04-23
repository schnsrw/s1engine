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
  await new Promise(r => setTimeout(r, 3000));
  
  // Check ImageLoader type and methods
  var il = api.ImageLoader;
  return {
    hasImageLoader: !!il,
    type: il ? il.constructor.name : null,
    methods: il ? Object.getOwnPropertyNames(Object.getPrototypeOf(il)).slice(0, 20) : [],
    hasLoadImage: il ? typeof il.LoadImage : null,
    hasMap: il ? !!il.map_image_index : false,
    mapKeys: il && il.map_image_index ? Object.keys(il.map_image_index).length : 0,
    // Check if the image is in the map with a data URL key
    firstDataUrlKey: il && il.map_image_index ? 
      Object.keys(il.map_image_index).find(k => k.indexOf('data:') === 0) : null,
  };
}, b64);
console.log(JSON.stringify(r, null, 2));
await browser.close();
