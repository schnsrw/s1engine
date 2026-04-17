const fs = require('fs');
const http = require('http');
const path = require('path');
const { chromium } = require('playwright');

const ROOT = process.cwd();
const HOST = '127.0.0.1';
let port = Number(process.env.DOCY_SMOKE_PORT || 0);

const FIXTURES = [
  {
    name: 'complex.docx',
    file: 'complex.docx',
    expectedPrefix: '化学品及企业标识',
    minElements: 100,
  },
  {
    name: 'calibre_demo.docx',
    file: 'testdocs/docx/samples/calibre_demo.docx',
    expectedPrefix: 'Demonstration of DOCX support in calibre',
    minElements: 20,
  },
];

const MIME = {
  '.css': 'text/css; charset=utf-8',
  '.docx': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.txt': 'text/plain; charset=utf-8',
  '.wasm': 'application/wasm',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

function send(res, code, body, type = 'text/plain; charset=utf-8') {
  res.writeHead(code, { 'Content-Type': type });
  res.end(body);
}

function createServer() {
  return http.createServer((req, res) => {
    const reqUrl = new URL(req.url, `http://${HOST}:${port}`);
    let relative = decodeURIComponent(reqUrl.pathname);
    if (relative === '/') relative = '/web/index.html';
    if (relative.startsWith('/sdkjs/')) relative = `/web${relative}`;
    if (relative.startsWith('/onlyoffice-sdkjs/')) relative = `/web${relative}`;
    if (relative.startsWith('/onlyoffice-web-apps/')) relative = `/web${relative}`;
    if (relative.startsWith('/pkg/')) relative = `/web${relative}`;
    if (relative.startsWith('/fonts/')) relative = `/web${relative}`;
    const normalized = path.normalize(relative).replace(/^(\.\.[/\\])+/, '');
    const filePath = path.join(ROOT, normalized);

    if (!filePath.startsWith(ROOT)) {
      send(res, 403, 'forbidden');
      return;
    }

    let finalPath = filePath;
    if (fs.existsSync(finalPath) && fs.statSync(finalPath).isDirectory()) {
      finalPath = path.join(finalPath, 'index.html');
    }

    fs.readFile(finalPath, (err, data) => {
      if (err) {
        send(res, 404, 'not found');
        return;
      }
      const ext = path.extname(finalPath).toLowerCase();
      send(res, 200, data, MIME[ext] || 'application/octet-stream');
    });
  });
}

async function waitForDocReady(page) {
  await page.waitForFunction(() => {
    const api = window._api;
    return !!api && !!api.WordControl && !!api.WordControl.m_oLogicDocument;
  }, null, { timeout: 60000 });
}

async function openFixture(page, fixturePath) {
  console.log(`[docy-smoke] goto index for ${fixturePath}`);
  await page.goto(`http://${HOST}:${port}/web/index.html`);
  console.log('[docy-smoke] waiting for editor bootstrap');
  await waitForDocReady(page);
  console.log('[docy-smoke] editor bootstrap ready');
  await page.locator('#file-picker').setInputFiles(path.resolve(ROOT, fixturePath));
  console.log('[docy-smoke] file selected, waiting for loaded content');
  await page.waitForFunction(() => {
    const api = window._api;
    const logicDoc = api && api.WordControl && api.WordControl.m_oLogicDocument;
    return !!logicDoc && Array.isArray(logicDoc.Content) && logicDoc.Content.length > 1;
  }, null, { timeout: 60000 });
  console.log('[docy-smoke] loaded content visible');
  await page.waitForTimeout(1000);
}

async function captureDocState(page) {
  return await page.evaluate(() => {
    const api = window._api;
    const logicDoc = api && api.WordControl && api.WordControl.m_oLogicDocument;
    const text = logicDoc && logicDoc.GetText
      ? logicDoc.GetText({
          ParaSeparator: '\n',
          TableRowSeparator: '\n',
          TableCellSeparator: ' | ',
        })
      : '';

    return {
      elements: logicDoc && Array.isArray(logicDoc.Content) ? logicDoc.Content.length : 0,
      pages: logicDoc && Array.isArray(logicDoc.Pages) ? logicDoc.Pages.length : 0,
      textPrefix: String(text || '').replace(/\s+/g, ' ').trim().slice(0, 240),
    };
  });
}

async function main() {
  console.log(`[docy-smoke] starting on ${HOST}:${port || 'auto'}`);
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, HOST, resolve);
  });
  port = server.address().port;
  console.log(`[docy-smoke] server ready on ${HOST}:${port}`);

  console.log('[docy-smoke] launching browser');
  const browser = await chromium.launch({ headless: true, channel: 'chrome', timeout: 20000 }).catch(() => {
    return chromium.launch({ headless: true, timeout: 20000 });
  });
  console.log('[docy-smoke] browser ready');

  let failed = false;
  try {
    for (const fixture of FIXTURES) {
      console.log(`[docy-smoke] opening ${fixture.name}`);
      const page = await browser.newPage();
      const pageErrors = [];
      page.on('console', (msg) => {
        if (msg.type() === 'log' || msg.type() === 'warning' || msg.type() === 'error') {
          console.log(`[browser:${fixture.name}:${msg.type()}] ${msg.text()}`);
        }
      });
      page.on('response', (resp) => {
        if (resp.status() === 404) {
          console.log(`[browser:${fixture.name}:404] ${resp.url()}`);
        }
      });
      page.on('pageerror', (err) => pageErrors.push(String(err)));
      page.on('console', (msg) => {
        if (msg.type() === 'error') pageErrors.push(msg.text());
      });

      try {
        await openFixture(page, fixture.file);
        const state = await captureDocState(page);

        if (pageErrors.length) {
          throw new Error(`${fixture.name}: browser errors:\n${pageErrors.join('\n')}`);
        }
        if (state.elements <= fixture.minElements) {
          throw new Error(`${fixture.name}: expected > ${fixture.minElements} elements, got ${state.elements}`);
        }
        if (state.pages <= 0) {
          throw new Error(`${fixture.name}: expected non-zero pages, got ${state.pages}`);
        }
        if (!state.textPrefix.includes(fixture.expectedPrefix)) {
          throw new Error(
            `${fixture.name}: expected text prefix to include ${JSON.stringify(fixture.expectedPrefix)}, got ${JSON.stringify(state.textPrefix)}`
          );
        }

        console.log(`[docy-smoke] ${fixture.name}: ok (${state.elements} elements, ${state.pages} pages)`);
      } catch (err) {
        failed = true;
        console.error('[docy-smoke] FAILED:', err.message);
      } finally {
        await page.close().catch(() => {});
      }
    }
  } finally {
    await browser.close().catch(() => {});
    await new Promise((resolve) => server.close(resolve));
  }

  if (failed) process.exit(1);
}

main().catch((err) => {
  console.error('[docy-smoke] FAILED:', err);
  process.exit(1);
});
