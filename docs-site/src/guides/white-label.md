# White-Labeling Rudra Office

This guide covers how to rebrand, restyle, and configure Rudra Office for your own product. Whether you are embedding the editor in a SaaS platform or shipping it as a standalone product under your own name, every visible element can be customized without modifying the editor source code.

---

## Configuration Overview

All runtime configuration is provided through the `window.S1_CONFIG` object in your `index.html`. The editor reads this object on startup and adjusts its behavior accordingly. You can either set the object before the editor script loads, or merge your overrides into the default:

```html
<script>
  window.S1_CONFIG = window.S1_CONFIG || {
    apiUrl: '',
    relayUrl: '',
    mode: 'standalone',
    autoCollab: true,
    productName: 'Rudra Office',
    enableCollab: true,
    enablePdfViewer: true,
    enableTemplates: true,
    aiUrl: '',
    enableAI: true,
  };
</script>
```

Place this block **before** the editor stylesheet and script tags. Each option is documented in the sections below.

---

## 1. Branding

### Product Name

Set `productName` to replace all instances of "Rudra Office" in the UI (title bar, about dialogs, share links):

```html
<script>
  window.S1_CONFIG = {
    ...window.S1_CONFIG,
    productName: 'Acme Docs',
  };
</script>
```

Update the HTML `<title>` tag and the `.logo-text` element to match:

```html
<title>Acme Docs</title>
```

```html
<div class="logo">
  <img src="/assets/my-logo.svg" alt="Acme" width="28" height="28">
  <span class="logo-text">Acme Docs</span>
</div>
```

### Logo

Replace the logo image at `/assets/logo.svg` with your own SVG or PNG. The default logo slot is 28x28 pixels. If your logo requires different dimensions, adjust the `width` and `height` attributes on the `<img>` element inside the `.logo` container.

> **Dark mode note:** The editor applies `filter: brightness(0) invert(1)` to the logo in dark mode so that a dark-on-transparent SVG becomes light. If your logo already handles both themes (e.g., uses `currentColor`), override this filter:
>
> ```css
> [data-theme="dark"] .logo img {
>   filter: none;
> }
> ```

### Accent Color

The primary brand color is controlled by the `--accent` CSS custom property. Override it in a `<style>` block after the editor stylesheet:

```html
<style>
  :root {
    --accent: #6200ea;
    --accent-light: #e8d5ff;
    --accent-bg: #d1b3ff;
  }
</style>
```

See the [Theme System](#2-theme-system) section for the complete list of properties.

### Favicon and Manifest

Replace these files in your deployment:

- `/icon-192.svg` -- favicon / PWA icon
- `/manifest.json` -- PWA manifest (update `name`, `short_name`, `icons`, `theme_color`)
- Update `<meta name="theme-color" content="#...">` in `index.html`

---

## 2. Theme System

The editor uses CSS custom properties (variables) defined on `:root` for all visual elements. Override any of these to change the editor's appearance without touching the source CSS.

### Light Theme Properties (defaults)

| Property | Default | Purpose |
|---|---|---|
| `--bg-app` | `#f8f9fa` | Application background (behind the document canvas) |
| `--bg-white` | `#fff` | Surface background (panels, cards) |
| `--bg-toolbar` | `#edf2fa` | Toolbar and menu bar background |
| `--bg-toolbar-hover` | `#d3e3fd` | Toolbar button hover state |
| `--bg-active` | `#c2dbff` | Active/pressed toolbar button background |
| `--bg-hover` | `#f1f3f4` | General hover background for list items |
| `--accent-bg` | `#c2dbff` | Accent background (selection highlights, badges) |
| `--border` | `#c4c7c5` | Primary border color |
| `--border-light` | `#dadce0` | Subtle border color (dividers, separators) |
| `--text-primary` | `#202124` | Primary text color |
| `--text-secondary` | `#5f6368` | Secondary text color (labels, descriptions) |
| `--text-muted` | `#80868b` | Muted text (placeholders, disabled items) |
| `--accent` | `#1a73e8` | Primary accent color (buttons, links, active states) |
| `--accent-light` | `#d2e3fc` | Light accent (selected item backgrounds) |
| `--danger` | `#d93025` | Error / destructive action color |
| `--success` | `#188038` | Success / confirmation color |

### Layout Properties

| Property | Default | Purpose |
|---|---|---|
| `--radius-sm` | `4px` | Small border radius (buttons, inputs) |
| `--radius-md` | `8px` | Medium border radius (cards, panels, modals) |
| `--font-ui` | `'Google Sans', 'Segoe UI', Roboto, ...` | UI font stack |

### Elevation (Shadows)

| Property | Default | Purpose |
|---|---|---|
| `--shadow-sm` | `0 1px 2px rgba(60,64,67,.3), ...` | Toolbar, small dropdowns |
| `--shadow-md` | `0 1px 3px rgba(60,64,67,.3), ...` | Panels, menus |
| `--shadow-lg` | `0 1px 3px rgba(60,64,67,.3), ...` | Modals, dialogs |

### Z-Index Hierarchy

| Property | Default | Purpose |
|---|---|---|
| `--z-toolbar` | `100` | Toolbar and menu bar |
| `--z-dropdown` | `200` | Dropdown menus |
| `--z-sidebar` | `300` | Side panels (pages, properties, comments) |
| `--z-modal-backdrop` | `400` | Modal backdrop overlay |
| `--z-modal` | `500` | Modal dialogs |
| `--z-context-menu` | `600` | Right-click context menus |
| `--z-toast` | `700` | Toast notifications |

### Example: Complete Brand Override

```css
:root {
  --accent: #0052cc;
  --accent-light: #deebff;
  --accent-bg: #b3d4ff;
  --bg-toolbar: #f4f5f7;
  --bg-toolbar-hover: #ebecf0;
  --bg-active: #b3d4ff;
  --radius-sm: 3px;
  --radius-md: 6px;
  --font-ui: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
}
```

---

## 3. Dark Mode

The editor supports three dark mode behaviors:

1. **Automatic** -- Follows the operating system preference via `prefers-color-scheme: dark`.
2. **Manual toggle** -- Users click the dark mode button in the status bar, which sets `data-theme="dark"` on the `<html>` element.
3. **Explicit override** -- You can force a theme on page load.

### How It Works

- The `[data-theme="dark"]` selector overrides all CSS custom properties with dark values.
- The `@media (prefers-color-scheme: dark)` block applies the same overrides, gated by `:root:not([data-theme="light"])` so that an explicit light preference wins.
- The user's choice is persisted in `localStorage` under the key `s1-theme`.

### Dark Theme Properties

| Property | Dark Value | Light Value |
|---|---|---|
| `--bg-app` | `#202124` | `#f8f9fa` |
| `--bg-white` | `#292a2d` | `#fff` |
| `--bg-toolbar` | `#35363a` | `#edf2fa` |
| `--bg-toolbar-hover` | `#44454a` | `#d3e3fd` |
| `--bg-active` | `#1a3a5c` | `#c2dbff` |
| `--border` | `#5f6368` | `#c4c7c5` |
| `--border-light` | `#3c4043` | `#dadce0` |
| `--text-primary` | `#e8eaed` | `#202124` |
| `--text-secondary` | `#bdc1c6` | `#5f6368` |
| `--text-muted` | `#9aa0a6` | `#80868b` |
| `--accent` | `#8ab4f8` | `#1a73e8` |
| `--accent-light` | `#1a3a5c` | `#d2e3fc` |
| `--danger` | `#f28b82` | `#d93025` |
| `--success` | `#81c995` | `#188038` |

### Forcing a Theme Programmatically

```javascript
// Force dark mode
document.documentElement.setAttribute('data-theme', 'dark');
localStorage.setItem('s1-theme', 'dark');

// Force light mode
document.documentElement.setAttribute('data-theme', 'light');
localStorage.setItem('s1-theme', 'light');

// Reset to OS preference
document.documentElement.removeAttribute('data-theme');
localStorage.removeItem('s1-theme');
```

### Providing Custom Dark Theme Colors

Override the `[data-theme="dark"]` selector with your own palette:

```css
[data-theme="dark"] {
  --bg-app: #1a1a2e;
  --bg-white: #16213e;
  --bg-toolbar: #1a1a2e;
  --accent: #e94560;
  --accent-light: #3a1528;
}
```

---

## 4. Toolbar Customization

The toolbar is defined in `index.html` as a series of `<button>` and `<select>` elements with specific IDs. You can show or hide individual toolbar items using CSS, or remove them from the HTML entirely.

### Toolbar Button Reference

| ID | Function | Collapsible Class |
|---|---|---|
| `fontFamily` | Font family dropdown | `tb-collapsible-768` |
| `fontSize` | Font size input | `tb-collapsible-768` |
| `btnBold` | Bold | -- |
| `btnItalic` | Italic | -- |
| `btnUnderline` | Underline | `tb-collapsible-480` |
| `btnStrike` | Strikethrough | `tb-collapsible-1024` |
| `btnFormatPainter` | Format Painter | `tb-collapsible-1024` |
| `btnSuperscript` | Superscript | `tb-collapsible-1024` |
| `btnSubscript` | Subscript | `tb-collapsible-1024` |
| `colorPicker` | Text color | `tb-collapsible-1024` |
| `highlightPicker` | Highlight color | `tb-collapsible-1024` |
| `btnClearFormat` | Clear formatting | `tb-collapsible-1024` |
| `btnAlignL` | Align left | `tb-collapsible-480` |
| `btnAlignC` | Align center | `tb-collapsible-480` |
| `btnAlignR` | Align right | `tb-collapsible-480` |
| `btnAlignJ` | Justify | `tb-collapsible-480` |
| `lineSpacing` | Line spacing dropdown | `tb-collapsible-1024` |
| `btnOutdent` | Decrease indent | `tb-collapsible-1024` |
| `btnIndent` | Increase indent | `tb-collapsible-1024` |
| `btnBulletList` | Bulleted list | `tb-collapsible-480` |
| `btnNumberList` | Numbered list | `tb-collapsible-480` |
| `btnInsertMenu` | Insert dropdown menu | -- |

### Hiding Toolbar Items via CSS

```css
/* Hide the strikethrough button */
#btnStrike { display: none !important; }

/* Hide the format painter */
#btnFormatPainter { display: none !important; }

/* Hide the entire font selection area */
#fontFamily, #fontSize { display: none !important; }
```

### Responsive Collapsing

Toolbar items use `tb-collapsible-*` classes to automatically hide at specific viewport widths:

- `tb-collapsible-1024` -- Hidden below 1024px
- `tb-collapsible-768` -- Hidden below 768px
- `tb-collapsible-480` -- Hidden below 480px

You can adjust these breakpoints or assign different classes to control which items collapse first.

### Menu Bar Items

The menu bar (File, Edit, View, Insert, Format, Review, Tools) can be customized the same way:

```css
/* Hide the Review menu entirely */
[data-menu="reviewMenu"] { display: none !important; }

/* Hide the Tools menu */
[data-menu="toolsMenu"] { display: none !important; }
```

---

## 5. Feature Flags

Feature flags in `S1_CONFIG` control the availability of major features. When a flag is set to `false`, the associated UI elements are hidden and the underlying functionality is not initialized.

### Available Flags

| Flag | Default | Effect |
|---|---|---|
| `enableCollab` | `true` | Enables the Share button, collaboration status indicator, and real-time editing. |
| `enablePdfViewer` | `true` | Enables the built-in PDF viewer tab for opening PDF files. |
| `enableTemplates` | `true` | Enables "New from Template" and "Save as Template" in the File menu. |
| `enableAI` | `true` | Enables the AI assistant panel. Requires `aiUrl` to point to a running LLM endpoint. |

### Disabling Features

```html
<script>
  window.S1_CONFIG = {
    ...window.S1_CONFIG,
    enableCollab: false,
    enableAI: false,
    enableTemplates: false,
  };
</script>
```

### AI Configuration

The AI assistant connects to a local LLM sidecar (e.g., llama.cpp). Set `aiUrl` to the base URL of the sidecar and `enableAI` to `true`:

```html
<script>
  window.S1_CONFIG = {
    ...window.S1_CONFIG,
    enableAI: true,
    aiUrl: 'http://localhost:8081',
  };
</script>
```

When `enableAI` is `false` or `aiUrl` is empty, the AI panel and its toolbar entry are not rendered.

---

## 6. Custom Fonts

The font family dropdown is defined in `index.html` as a `<select>` element with id `fontFamily`. You can modify the available fonts by editing the `<option>` list directly.

### Default Font List

```
Arial, Times New Roman, Georgia, Courier New, Verdana,
Trebuchet MS, Garamond, Palatino, Tahoma, Comic Sans MS
```

### Adding Custom Fonts

1. Load the font via a `<link>` tag or `@font-face` rule:

```html
<link href="https://fonts.googleapis.com/css2?family=Lora:wght@400;700&display=swap" rel="stylesheet">
```

2. Add the option to the `fontFamily` select in `index.html`:

```html
<select id="fontFamily" ...>
  <!-- existing options -->
  <option value="Lora" style="font-family:Lora">Lora</option>
</select>
```

### Replacing the Entire Font List

Remove all existing `<option>` elements and add your own. Keep the first option as the default:

```html
<select id="fontFamily" ...>
  <option value="">Default</option>
  <option value="Inter" style="font-family:Inter">Inter</option>
  <option value="Source Serif Pro" style="font-family:'Source Serif Pro'">Source Serif Pro</option>
  <option value="Fira Code" style="font-family:'Fira Code'">Fira Code</option>
</select>
```

### Changing the UI Font

The editor UI font is controlled by the `--font-ui` custom property:

```css
:root {
  --font-ui: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
}
```

This affects all toolbar labels, menus, dialogs, and status bar text. It does not affect the document content font.

---

## 7. Locale / i18n

Internationalization support is planned for a future release. The current architecture is prepared for it:

- All user-facing strings in the toolbar, menus, and dialogs are defined in the HTML and can be replaced.
- Tooltip text is set via `title` attributes on every interactive element.
- Right-to-left (RTL) layout support is available through the BiDi text processing in the engine.

### Preparing for i18n Today

If you need to localize the editor before the official i18n module ships, you can:

1. **Replace HTML strings** -- Edit `index.html` to translate menu labels, button text, and tooltips.
2. **Override at runtime** -- Use JavaScript to walk the DOM and replace text content after the editor loads:

```javascript
document.addEventListener('DOMContentLoaded', () => {
  document.querySelector('#btnBold').title = 'Gras (Ctrl+B)';
  document.querySelector('[data-menu="fileMenu"] .app-menu-btn').textContent = 'Fichier';
  // ... other translations
});
```

3. **Use a translation map** -- Maintain a JSON object mapping element IDs/selectors to translated strings, and apply them in a loop.

---

## 8. CSS Override Examples

All customizations below should be placed in a `<style>` block after the editor stylesheet link, or in a separate CSS file loaded after `styles.css`.

### Change the Accent Color to Purple

```css
:root {
  --accent: #7c3aed;
  --accent-light: #ede9fe;
  --accent-bg: #c4b5fd;
}
[data-theme="dark"] {
  --accent: #a78bfa;
  --accent-light: #2e1a5e;
}
```

### Rounded Toolbar Buttons

```css
.tb-btn {
  border-radius: 8px !important;
}
```

### Hide the Status Bar Word Count

```css
.status-bar .word-count {
  display: none;
}
```

### Custom Scrollbar Styling

```css
/* Webkit browsers */
.canvas::-webkit-scrollbar {
  width: 8px;
}
.canvas::-webkit-scrollbar-track {
  background: var(--bg-app);
}
.canvas::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}
.canvas::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
}
```

### Compact Toolbar (Reduced Height)

```css
.toolbar {
  padding: 2px 8px !important;
  min-height: 32px !important;
}
.tb-btn {
  width: 28px !important;
  height: 28px !important;
}
.tb-btn .msi {
  font-size: 18px !important;
}
```

### Hide the Ruler

```css
.ruler {
  display: none !important;
}
```

### Full-Width Document (No Page Margins)

```css
.doc-page {
  max-width: 100% !important;
  box-shadow: none !important;
  border: none !important;
}
```

### Custom Modal Styling

```css
.modal {
  border-radius: 12px;
  box-shadow: var(--shadow-lg);
}
.modal-header {
  border-bottom: 1px solid var(--border-light);
}
```

### Hide Specific Menu Items

```css
/* Hide "Export as ODT" from the File menu */
[data-fmt="odt"] { display: none !important; }

/* Hide the Templates menu entries */
#btnTemplate, #btnSaveTemplate { display: none !important; }

/* Hide the collaboration Share button */
#btnShare { display: none !important; }
```

---

## 9. Integration Mode

The `mode` configuration option controls how the editor behaves within a host application.

### Standalone Mode (Default)

```javascript
window.S1_CONFIG = {
  ...window.S1_CONFIG,
  mode: 'standalone',
};
```

In standalone mode, the editor runs as a full-page application with its own file management, title bar, and menu system. This is the default for deployments where the editor is the primary application.

### Integrated Mode

```javascript
window.S1_CONFIG = {
  ...window.S1_CONFIG,
  mode: 'integrated',
  apiUrl: 'https://your-app.com/api/v1',
  relayUrl: 'wss://your-app.com/ws/collab',
};
```

In integrated mode, the editor is designed to be embedded within a larger application. Use this when:

- The editor is loaded inside an `<iframe>` within your application.
- File open/save operations are handled by your host application via the API.
- Collaboration is managed through your own WebSocket relay.

### Embedding in an iframe

```html
<iframe
  src="https://editor.your-app.com?room=doc-123"
  style="width: 100%; height: 100vh; border: none;"
  allow="clipboard-read; clipboard-write"
></iframe>
```

### Configuring the API Endpoint

When `apiUrl` is set, the editor uses it for document CRUD operations and format conversion:

```javascript
window.S1_CONFIG = {
  ...window.S1_CONFIG,
  apiUrl: 'https://api.your-app.com/v1',
};
```

### Configuring the Collaboration Relay

When `relayUrl` is set along with `autoCollab: true`, the editor automatically connects to the WebSocket relay when a `room` parameter is present in the URL:

```javascript
window.S1_CONFIG = {
  ...window.S1_CONFIG,
  relayUrl: 'wss://collab.your-app.com/ws',
  autoCollab: true,
};
```

---

## 10. Removing Branding

For commercial licensees with a white-label agreement, you can fully remove Rudra branding.

### Step 1: Replace Visual Assets

| File | Purpose |
|---|---|
| `/assets/logo.svg` | Title bar logo |
| `/icon-192.svg` | Favicon and PWA icon |
| `/manifest.json` | PWA name, icons, theme color |

### Step 2: Update HTML

In `index.html`, update these elements:

```html
<!-- Page title -->
<title>Your Product Name</title>

<!-- Theme color -->
<meta name="theme-color" content="#your-brand-color">

<!-- Logo container -->
<div class="logo">
  <img src="/assets/your-logo.svg" alt="Your Brand" width="28" height="28">
  <span class="logo-text">Your Product Name</span>
</div>
```

### Step 3: Set the Config

```html
<script>
  window.S1_CONFIG = {
    ...window.S1_CONFIG,
    productName: 'Your Product Name',
  };
</script>
```

### Step 4: Override Brand Colors

```css
:root {
  --accent: #your-brand-color;
  --accent-light: #your-brand-light;
  --accent-bg: #your-brand-bg;
}
```

### Step 5: Remove "Powered By" (if present)

If any "Powered by Rudra" or attribution text is displayed, commercial licensees may remove it per the terms of their license agreement. Search the HTML for any such references and remove or replace them.

### Licensing Note

Rudra Office is licensed under AGPL-3.0. White-label redistribution without Rudra branding requires a separate commercial license. Contact the maintainers for licensing terms.
