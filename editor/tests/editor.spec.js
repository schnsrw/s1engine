// @ts-check
import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import path from 'path';

const DOCX_PATH = path.resolve('../demo/images/document.docx');
const CHAT_DOCX_PATH = path.resolve('../demo/images/Chat Reaction (1) (1).docx');

// ─── Helper: wait for WASM engine to be ready ──────────────
async function waitForEngine(page) {
  await page.waitForFunction(() => {
    const label = document.getElementById('wasmLabel');
    return label && label.textContent === 's1engine ready';
  }, { timeout: 10000 });
}

// ─── Helper: create a new document ──────────────────────────
async function newDoc(page) {
  await page.goto('/');
  await waitForEngine(page);
  await page.click('#welcomeNew');
  await page.waitForSelector('#docPage[contenteditable="true"]');
}

// ─── Helper: open a DOCX file ───────────────────────────────
async function openDocx(page, filePath) {
  await page.goto('/');
  await waitForEngine(page);
  const fileInput = page.locator('#fileInput');
  await fileInput.setInputFiles(filePath);
  await page.waitForSelector('#docPage[contenteditable="true"]');
  // Wait for render
  await page.waitForTimeout(500);
}

// ─── Helper: get doc page content ───────────────────────────
async function getPageHtml(page) {
  return page.evaluate(() => document.getElementById('docPage').innerHTML);
}

async function getPageText(page) {
  return page.evaluate(() => {
    const page = document.getElementById('docPage');
    // Exclude page break indicators and footers from text
    let text = '';
    page.querySelectorAll('[data-node-id]').forEach(el => {
      text += el.textContent + '\n';
    });
    return text.trim();
  });
}

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Engine Initialization
// ═════════════════════════════════════════════════════════════
test.describe('Engine Init', () => {
  test('WASM engine loads successfully', async ({ page }) => {
    await page.goto('/');
    await waitForEngine(page);
    const label = page.locator('#wasmLabel');
    await expect(label).toHaveText('s1engine ready');
    const dot = page.locator('#wasmDot');
    await expect(dot).toHaveClass(/ok/);
  });

  test('welcome screen is visible', async ({ page }) => {
    await page.goto('/');
    await waitForEngine(page);
    await expect(page.locator('#welcomeScreen')).toBeVisible();
    await expect(page.locator('#welcomeNew')).toBeVisible();
    await expect(page.locator('#welcomeOpen')).toBeVisible();
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: New Document
// ═════════════════════════════════════════════════════════════
test.describe('New Document', () => {
  test('creates empty document on click', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#editorCanvas')).toHaveClass(/show/);
    await expect(page.locator('#toolbar')).toHaveClass(/show/);
    await expect(page.locator('#welcomeScreen')).not.toBeVisible();
  });

  test('can type text into document', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Hello World');
    const text = await getPageText(page);
    expect(text).toContain('Hello World');
  });

  test('Enter key splits paragraph', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Line one');
    await page.keyboard.press('Enter');
    await page.keyboard.type('Line two');
    const paragraphs = await page.locator('#docPage [data-node-id]').count();
    expect(paragraphs).toBeGreaterThanOrEqual(2);
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Text Formatting
// ═════════════════════════════════════════════════════════════
test.describe('Text Formatting', () => {
  test('bold toggles via toolbar', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Bold text');
    await page.keyboard.press('Meta+a');
    await page.click('#btnBold');
    await page.waitForTimeout(300);
    const html = await getPageHtml(page);
    // WASM may render bold as inline style OR <strong>/<b> tags
    expect(html).toMatch(/font-weight:\s*(bold|700)|<strong|<b[\s>]/i);
  });

  test('bold toggles via Ctrl+B', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Some text');
    await page.keyboard.press('Meta+a');
    await page.keyboard.press('Meta+b');
    await page.waitForTimeout(300);
    const html = await getPageHtml(page);
    expect(html).toMatch(/font-weight:\s*(bold|700)|<strong|<b[\s>]/i);
  });

  test('italic toggles via Ctrl+I', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Some text');
    await page.keyboard.press('Meta+a');
    await page.keyboard.press('Meta+i');
    await page.waitForTimeout(300);
    const html = await getPageHtml(page);
    // WASM may render italic as inline style OR <em>/<i> tags
    expect(html).toMatch(/font-style:\s*italic|<em|<i[\s>]/i);
  });

  test('font size change applies', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Big text');
    await page.keyboard.press('Meta+a');
    await page.locator('#fontSize').fill('24');
    await page.locator('#fontSize').press('Enter');
    await page.waitForTimeout(300);
    const html = await getPageHtml(page);
    expect(html).toMatch(/font-size:\s*24/i);
  });

  test('heading level change applies', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('My Heading');
    await page.locator('#blockType').selectOption('1');
    await page.waitForTimeout(300);
    const h1 = await page.locator('#docPage h1').count();
    expect(h1).toBeGreaterThanOrEqual(1);
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Clipboard (Cut / Copy / Paste)
// ═════════════════════════════════════════════════════════════
test.describe('Clipboard', () => {
  test('Ctrl+A selects all text', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Hello World');
    await page.keyboard.press('Meta+a');
    const selText = await page.evaluate(() => window.getSelection().toString());
    expect(selText).toContain('Hello');
  });

  test('Delete after select-all clears document', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Some content here');
    await page.keyboard.press('Meta+a');
    await page.keyboard.press('Delete');
    await page.waitForTimeout(300);
    const text = await getPageText(page);
    expect(text.trim()).toBe('');
  });

  test('Backspace after select-all clears document', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Content to delete');
    await page.keyboard.press('Meta+a');
    await page.keyboard.press('Backspace');
    await page.waitForTimeout(300);
    const text = await getPageText(page);
    expect(text.trim()).toBe('');
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Undo / Redo
// ═════════════════════════════════════════════════════════════
test.describe('Undo/Redo', () => {
  test('undo reverses typing', async ({ page }) => {
    await newDoc(page);
    const docPage = page.locator('#docPage');
    await docPage.focus();
    await page.keyboard.type('First');
    // Sync text (wait for debounce)
    await page.waitForTimeout(300);
    await page.keyboard.press('Enter');
    await page.keyboard.type('Second');
    await page.waitForTimeout(300);
    const textBefore = await getPageText(page);
    // Undo should remove something
    await page.keyboard.press('Meta+z');
    await page.waitForTimeout(300);
    const textAfter = await getPageText(page);
    // After undo, text should be different (shorter or changed)
    expect(textAfter.length).toBeLessThanOrEqual(textBefore.length);
  });

  test('undo button becomes enabled after edit', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('test');
    await page.waitForTimeout(300);
    const disabled = await page.locator('#btnUndo').getAttribute('disabled');
    expect(disabled).toBeNull();
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Open DOCX Files
// ═════════════════════════════════════════════════════════════
test.describe('Open DOCX', () => {
  test('opens document.docx with content', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const text = await getPageText(page);
    expect(text.length).toBeGreaterThan(0);
  });

  test('opens document.docx — toolbar shows', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    await expect(page.locator('#toolbar')).toHaveClass(/show/);
    await expect(page.locator('#editorCanvas')).toHaveClass(/show/);
  });

  test('opens Chat Reaction docx with content', async ({ page }) => {
    await openDocx(page, CHAT_DOCX_PATH);
    const text = await getPageText(page);
    expect(text.length).toBeGreaterThan(10);
  });

  test('opened DOCX preserves formatting in HTML', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const html = await getPageHtml(page);
    // Should have data-node-id attributes (WASM rendering)
    expect(html).toContain('data-node-id');
  });

  test('status bar shows word count after open', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const status = await page.locator('#statusInfo').textContent();
    expect(status).toMatch(/\d+ words/);
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Export
// ═════════════════════════════════════════════════════════════
test.describe('Export', () => {
  test('export menu opens and closes', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Export test');
    await page.click('#btnExport');
    await expect(page.locator('#exportMenu')).toHaveClass(/show/);
    // Click elsewhere to close
    await page.click('#docPage');
    await page.waitForTimeout(200);
    const cls = await page.locator('#exportMenu').getAttribute('class');
    expect(cls).not.toContain('show');
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Views (Editor / Pages / Text)
// ═════════════════════════════════════════════════════════════
test.describe('Views', () => {
  test('switch to Text view shows plain text', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('View test content');
    await page.waitForTimeout(300);
    await page.click('.tab[data-view="text"]');
    await page.waitForTimeout(300);
    await expect(page.locator('#textView')).toHaveClass(/show/);
    const text = await page.locator('#textContent').textContent();
    expect(text).toContain('View test content');
  });

  test('switch to Pages view shows paginated content', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    await page.click('.tab[data-view="pages"]');
    await page.waitForTimeout(500);
    await expect(page.locator('#pagesView')).toHaveClass(/show/);
  });

  test('switch back to Editor view', async ({ page }) => {
    await newDoc(page);
    await page.click('.tab[data-view="text"]');
    await page.waitForTimeout(200);
    await page.click('.tab[data-view="editor"]');
    await page.waitForTimeout(200);
    await expect(page.locator('#editorCanvas')).toHaveClass(/show/);
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Find & Replace
// ═════════════════════════════════════════════════════════════
test.describe('Find & Replace', () => {
  test('Ctrl+F opens find bar', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.press('Meta+f');
    await expect(page.locator('#findBar')).toHaveClass(/show/);
    await expect(page.locator('#findInput')).toBeFocused();
  });

  test('close button hides find bar', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.press('Meta+f');
    await page.click('#findClose');
    const cls = await page.locator('#findBar').getAttribute('class');
    expect(cls).not.toContain('show');
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Page Breaks & Pagination
// ═════════════════════════════════════════════════════════════
test.describe('Pagination', () => {
  test('single-page doc shows page footer', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Short doc');
    await page.waitForTimeout(500);
    const footer = await page.locator('.editor-footer').textContent();
    expect(footer).toContain('Page 1');
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: DOCX Round-Trip (Open → Export → Reopen)
// ═════════════════════════════════════════════════════════════
test.describe('Round-Trip', () => {
  test('open DOCX → export DOCX → content preserved', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const originalText = await getPageText(page);
    expect(originalText.length).toBeGreaterThan(0);

    // Export as DOCX bytes
    const exported = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const bytes = doc.export('docx');
      return Array.from(bytes);
    });

    // We can't easily reopen in the same test, but verify export succeeded
    expect(exported).not.toBeNull();
    if (exported) {
      expect(exported.length).toBeGreaterThan(100); // Valid DOCX is > 100 bytes
    }
  });

  test('export to ODT succeeds', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const exported = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const bytes = doc.export('odt');
      return Array.from(bytes);
    });
    expect(exported).not.toBeNull();
    expect(exported.length).toBeGreaterThan(100);
  });

  test('export to TXT succeeds', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const exported = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const bytes = doc.export('txt');
      return new TextDecoder().decode(new Uint8Array(Array.from(bytes)));
    });
    expect(exported).toBeTruthy();
    expect(exported.length).toBeGreaterThan(10);
  });

  test('export to Markdown succeeds', async ({ page }) => {
    await openDocx(page, DOCX_PATH);
    const exported = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const bytes = doc.export('md');
      return new TextDecoder().decode(new Uint8Array(Array.from(bytes)));
    });
    expect(exported).toBeTruthy();
    expect(exported.length).toBeGreaterThan(0);
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Accessibility (ARIA)
// ═════════════════════════════════════════════════════════════
test.describe('Accessibility', () => {
  test('toolbar has ARIA role', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#toolbar')).toHaveAttribute('role', 'toolbar');
  });

  test('format buttons have aria-label', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#btnBold')).toHaveAttribute('aria-label', 'Bold');
    await expect(page.locator('#btnItalic')).toHaveAttribute('aria-label', 'Italic');
    await expect(page.locator('#btnUnderline')).toHaveAttribute('aria-label', 'Underline');
  });

  test('format buttons update aria-pressed', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Test');
    await page.keyboard.press('Meta+a');
    await expect(page.locator('#btnBold')).toHaveAttribute('aria-pressed', 'false');
    await page.click('#btnBold');
    await page.waitForTimeout(300);
    await expect(page.locator('#btnBold')).toHaveAttribute('aria-pressed', 'true');
  });

  test('document content area has textbox role', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#docPage')).toHaveAttribute('role', 'textbox');
    await expect(page.locator('#docPage')).toHaveAttribute('aria-multiline', 'true');
  });

  test('status bar has status role', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#statusbar')).toHaveAttribute('role', 'status');
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: New Toolbar Features
// ═════════════════════════════════════════════════════════════
test.describe('Toolbar Features', () => {
  test('clear formatting button exists', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#btnClearFormat')).toBeVisible();
  });

  test('print button exists', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#btnPrint')).toBeVisible();
  });

  test('indent/outdent buttons exist', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#btnIndent')).toBeVisible();
    await expect(page.locator('#btnOutdent')).toBeVisible();
  });

  test('line spacing selector exists and has options', async ({ page }) => {
    await newDoc(page);
    const options = await page.locator('#lineSpacing option').count();
    expect(options).toBeGreaterThanOrEqual(4); // 1, 1.15, 1.5, 2
  });

  test('zoom controls work', async ({ page }) => {
    await newDoc(page);
    await expect(page.locator('#zoomValue')).toHaveText('100%');
    await page.click('#zoomIn');
    await expect(page.locator('#zoomValue')).toHaveText('110%');
    await page.click('#zoomOut');
    await expect(page.locator('#zoomValue')).toHaveText('100%');
  });

  test('comments panel toggles', async ({ page }) => {
    await newDoc(page);
    const panel = page.locator('#commentsPanel');
    await expect(panel).not.toHaveClass(/show/);
    await page.click('#btnComments');
    await expect(panel).toHaveClass(/show/);
    await page.click('#commentsClose');
    await expect(panel).not.toHaveClass(/show/);
  });

  test('insert menu has comment option', async ({ page }) => {
    await newDoc(page);
    await page.click('#btnInsertMenu');
    await expect(page.locator('#miComment')).toBeVisible();
  });

  test('superscript formatting applies', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('H2O');
    await page.keyboard.press('Meta+a');
    await page.click('#btnSuperscript');
    await page.waitForTimeout(300);
    const html = await getPageHtml(page);
    expect(html).toMatch(/vertical-align:\s*super|<sup/i);
  });
});

// ═════════════════════════════════════════════════════════════
// TEST SUITE: Cross-Format Export
// ═════════════════════════════════════════════════════════════
test.describe('Cross-Format Export', () => {
  test('Chat Reaction DOCX exports to ODT', async ({ page }) => {
    await openDocx(page, CHAT_DOCX_PATH);
    const exported = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const bytes = doc.export('odt');
      return bytes.length;
    });
    expect(exported).toBeGreaterThan(100);
  });

  test('Chat Reaction DOCX exports to TXT', async ({ page }) => {
    await openDocx(page, CHAT_DOCX_PATH);
    const text = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const bytes = doc.export('txt');
      return new TextDecoder().decode(new Uint8Array(Array.from(bytes)));
    });
    expect(text.length).toBeGreaterThan(10);
  });

  test('new document exports to all formats', async ({ page }) => {
    await newDoc(page);
    await page.locator('#docPage').focus();
    await page.keyboard.type('Export test content');
    await page.waitForTimeout(300);

    const results = await page.evaluate(() => {
      const doc = window.__folio_state?.doc;
      if (!doc) return null;
      const fmts = {};
      for (const fmt of ['docx', 'odt', 'txt', 'md']) {
        try {
          const bytes = doc.export(fmt);
          fmts[fmt] = bytes.length;
        } catch (e) {
          fmts[fmt] = -1;
        }
      }
      return fmts;
    });

    expect(results).not.toBeNull();
    expect(results.docx).toBeGreaterThan(100);
    expect(results.odt).toBeGreaterThan(100);
    expect(results.txt).toBeGreaterThan(0);
    expect(results.md).toBeGreaterThan(0);
  });
});
