#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const vm = require('vm');

const repoRoot = path.resolve(__dirname, '..');
const webRoot = path.join(repoRoot, 'web');
const distRoot = path.join(webRoot, 'dist');
const appOut = path.join(distRoot, 'assets', 'app');
const runtimeOut = path.join(distRoot, 'assets', 'runtime');

const ONLYOFFICE_RUNTIME_ASSETS = [
  ['onlyoffice-sdkjs/common/Images', 'onlyoffice-sdkjs/common/Images'],
  ['onlyoffice-sdkjs/common/SmartArts', 'onlyoffice-sdkjs/common/SmartArts'],
  ['onlyoffice-sdkjs/common/libfont/engine', 'onlyoffice-sdkjs/common/libfont/engine'],
  ['onlyoffice-sdkjs/common/spell/spell', 'onlyoffice-sdkjs/common/spell/spell'],
];

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function rmrf(target) {
  fs.rmSync(target, { recursive: true, force: true });
}

function copyRecursive(src, dest) {
  const stat = fs.statSync(src);
  if (stat.isDirectory()) {
    ensureDir(dest);
    for (const entry of fs.readdirSync(src)) {
      copyRecursive(path.join(src, entry), path.join(dest, entry));
    }
    return;
  }
  ensureDir(path.dirname(dest));
  fs.copyFileSync(src, dest);
}

function copySelectedTree(pairs) {
  for (const [fromRelative, toRelative] of pairs) {
    copyRecursive(path.join(webRoot, fromRelative), path.join(distRoot, toRelative));
  }
}

function hashContent(content) {
  return crypto.createHash('sha256').update(content).digest('hex').slice(0, 12);
}

function writeHashedAsset(outDir, baseName, ext, content) {
  const hash = hashContent(content);
  const fileName = `${baseName}.${hash}.${ext}`;
  const outPath = path.join(outDir, fileName);
  ensureDir(outDir);
  fs.writeFileSync(outPath, content);
  return fileName;
}

function read(filePath) {
  return fs.readFileSync(filePath, 'utf8');
}

function buildOnlyOfficeBundle() {
  const scriptsJsPath = path.join(webRoot, 'onlyoffice-sdkjs', 'develop', 'sdkjs', 'word', 'scripts.js');
  const sandbox = { sdk_scripts: [] };
  vm.runInNewContext(read(scriptsJsPath), sandbox, { filename: scriptsJsPath });

  const sources = [
    path.join(webRoot, 'onlyoffice-sdkjs', 'vendor', 'polyfill.js'),
    path.join(webRoot, 'onlyoffice-web-apps', 'vendor', 'jquery', 'jquery.min.js'),
    path.join(webRoot, 'onlyoffice-web-apps', 'vendor', 'xregexp', 'xregexp-all-min.js'),
    ...sandbox.sdk_scripts.map((src) =>
      path.join(webRoot, src.replace('../../../../sdkjs/', 'onlyoffice-sdkjs/'))
    ),
    path.join(webRoot, 'onlyoffice-sdkjs', 'word', 'document', 'editor.js'),
  ];

  const missing = sources.filter((src) => !fs.existsSync(src));
  if (missing.length > 0) {
    throw new Error(`Missing OnlyOffice runtime sources:\n${missing.join('\n')}`);
  }

  const bundle = sources
    .map((src) => `\n/* ${path.relative(repoRoot, src)} */\n${read(src)}\n`)
    .join('\n');

  return writeHashedAsset(runtimeOut, 'onlyoffice-word-runtime', 'js', bundle);
}

function buildAppAssets() {
  const adapterSource = read(path.join(webRoot, 'adapter.js')).replace(
    "from './pkg/s1engine_wasm.js'",
    "from '../../pkg/s1engine_wasm.js'"
  );

  return {
    styles: writeHashedAsset(appOut, 'styles', 'css', read(path.join(webRoot, 'styles.css'))),
    adapter: writeHashedAsset(appOut, 'adapter', 'js', adapterSource),
    menubar: writeHashedAsset(appOut, 'menubar', 'js', read(path.join(webRoot, 'menubar.js'))),
    toolbar: writeHashedAsset(appOut, 'toolbar', 'js', read(path.join(webRoot, 'toolbar.js'))),
    collab: writeHashedAsset(appOut, 'collab', 'js', read(path.join(webRoot, 'collab.js'))),
  };
}

function rewriteIndexHtml(assetMap, runtimeBundleName) {
  let html = read(path.join(webRoot, 'index.html'));

  html = html.replace('href="styles.css"', `href="assets/app/${assetMap.styles}"`);

  html = html.replaceAll("import('./menubar.js')", `import('./assets/app/${assetMap.menubar}')`);
  html = html.replaceAll("import('./adapter.js')", `import('./assets/app/${assetMap.adapter}')`);
  html = html.replaceAll("import('./toolbar.js')", `import('./assets/app/${assetMap.toolbar}')`);
  html = html.replaceAll("import('./collab.js')", `import('./assets/app/${assetMap.collab}')`);

  const runtimeBlock = [
    '  <script src="onlyoffice-sdkjs/vendor/polyfill.js"></script>',
    '  <script src="onlyoffice-web-apps/vendor/jquery/jquery.min.js"></script>',
    '  <script src="onlyoffice-web-apps/vendor/xregexp/xregexp-all-min.js"></script>',
    '',
    '  <script>',
    '    var sdk_scripts = [];',
    '  </script>',
    '  <script src="onlyoffice-sdkjs/develop/sdkjs/word/scripts.js"></script>',
    '  <script>',
    '    var fixedScripts = sdk_scripts.map(function(s) {',
    "      return s.replace('../../../../sdkjs/', 'onlyoffice-sdkjs/');",
    '    });',
    '    fixedScripts.forEach(function(src) {',
    "      document.write('<script src=\"' + src + '\"><\\/script>');",
    '    });',
    '    document.write(\'<script src="onlyoffice-sdkjs/word/document/editor.js"><\\/script>\');',
    '  </script>',
  ].join('\n');

  html = html.replace(runtimeBlock, `  <script src="assets/runtime/${runtimeBundleName}"></script>`);

  fs.writeFileSync(path.join(distRoot, 'index.html'), html);
}

function main() {
  rmrf(distRoot);
  ensureDir(appOut);
  ensureDir(runtimeOut);

  const assetMap = buildAppAssets();
  const runtimeBundleName = buildOnlyOfficeBundle();

  copyRecursive(path.join(webRoot, 'fonts'), path.join(distRoot, 'fonts'));
  copyRecursive(path.join(webRoot, 'pkg'), path.join(distRoot, 'pkg'));
  copySelectedTree(ONLYOFFICE_RUNTIME_ASSETS);

  rewriteIndexHtml(assetMap, runtimeBundleName);

  console.log('Built web dist at web/dist');
  console.log(`Runtime bundle: assets/runtime/${runtimeBundleName}`);
  console.log(`App assets: assets/app/${assetMap.styles}, ${assetMap.adapter}, ${assetMap.menubar}`);
}

main();
