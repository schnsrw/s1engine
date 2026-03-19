# White-Labeling Strategy

## Overview

White-labeling allows consumers to rebrand s1engine's editor as their own product. The consumer's users should never see "s1engine" — the editor appears as a native part of the consumer's application.

## White-Label Layers

```
Layer 1: Visual Theming           CSS custom properties, colors, fonts
Layer 2: Branding                 Logo, product name, favicon, loading screen
Layer 3: Toolbar Customization    Button layout, custom items, feature toggles
Layer 4: Locale / i18n            UI strings, translations, date formats
Layer 5: Domain & Hosting         Custom domain, custom CDN, email templates
Layer 6: Feature Gating           Enable/disable features per tenant
```

---

## Layer 1: Visual Theming

### CSS Custom Properties

Every visual aspect is controlled via CSS custom properties. Consumers override these to match their brand.

```css
/* Default theme — override any of these */
:root, .s1-editor {
  /* ── Brand Colors ──────────────────────────── */
  --s1-primary:              #1a73e8;
  --s1-primary-hover:        #1557b0;
  --s1-primary-light:        #e8f0fe;
  --s1-on-primary:           #ffffff;

  /* ── Surface Colors ────────────────────────── */
  --s1-background:           #f8f9fa;
  --s1-surface:              #ffffff;
  --s1-surface-variant:      #f1f3f4;
  --s1-on-surface:           #202124;
  --s1-on-surface-muted:     #5f6368;

  /* ── Border & Dividers ─────────────────────── */
  --s1-border:               #dadce0;
  --s1-border-light:         #e8eaed;
  --s1-divider:              #e0e0e0;

  /* ── State Colors ──────────────────────────── */
  --s1-hover:                rgba(0, 0, 0, 0.04);
  --s1-active:               rgba(0, 0, 0, 0.08);
  --s1-selected:             #e8f0fe;
  --s1-focus-ring:           rgba(26, 115, 232, 0.4);
  --s1-error:                #d93025;
  --s1-success:              #1e8e3e;
  --s1-warning:              #f9ab00;

  /* ── Toolbar ───────────────────────────────── */
  --s1-toolbar-bg:           #ffffff;
  --s1-toolbar-border:       #dadce0;
  --s1-toolbar-height:       40px;
  --s1-toolbar-padding:      4px 8px;
  --s1-toolbar-button-size:  32px;
  --s1-toolbar-button-radius: 4px;
  --s1-toolbar-button-hover: rgba(0, 0, 0, 0.06);
  --s1-toolbar-button-active: rgba(0, 0, 0, 0.1);
  --s1-toolbar-separator:    #dadce0;
  --s1-toolbar-dropdown-bg:  #ffffff;
  --s1-toolbar-dropdown-shadow: 0 2px 6px rgba(0,0,0,0.15);

  /* ── Editor Area ───────────────────────────── */
  --s1-editor-bg:            #f8f9fa;
  --s1-page-bg:              #ffffff;
  --s1-page-shadow:          0 1px 3px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.06);
  --s1-page-gap:             20px;
  --s1-page-border-radius:   2px;
  --s1-selection-bg:         rgba(26, 115, 232, 0.2);
  --s1-cursor-color:         #000000;

  /* ── Status Bar ────────────────────────────── */
  --s1-statusbar-bg:         #f1f3f4;
  --s1-statusbar-border:     #dadce0;
  --s1-statusbar-text:       #5f6368;
  --s1-statusbar-height:     24px;

  /* ── Collaboration ─────────────────────────── */
  --s1-collab-cursor-width:  2px;
  --s1-collab-label-radius:  3px;
  --s1-collab-label-font:    11px;

  /* ── Typography (UI only, not document) ────── */
  --s1-font-family:          -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --s1-font-size:            13px;
  --s1-font-size-small:      11px;
  --s1-font-weight-normal:   400;
  --s1-font-weight-medium:   500;

  /* ── Spacing ───────────────────────────────── */
  --s1-spacing-xs:           4px;
  --s1-spacing-sm:           8px;
  --s1-spacing-md:           12px;
  --s1-spacing-lg:           16px;
  --s1-spacing-xl:           24px;

  /* ── Transitions ───────────────────────────── */
  --s1-transition-fast:      100ms ease;
  --s1-transition-normal:    200ms ease;

  /* ── Z-Index Scale ─────────────────────────── */
  --s1-z-toolbar:            100;
  --s1-z-dropdown:           200;
  --s1-z-modal:              300;
  --s1-z-tooltip:            400;
}
```

### Pre-Built Themes

```typescript
// Built-in themes
S1Editor.themes = {
  default: { /* light theme, Google Docs-like */ },
  dark: {
    '--s1-background': '#1e1e1e',
    '--s1-surface': '#2d2d2d',
    '--s1-on-surface': '#e0e0e0',
    '--s1-toolbar-bg': '#2d2d2d',
    '--s1-editor-bg': '#1e1e1e',
    '--s1-page-bg': '#2d2d2d',
    // ...
  },
  minimal: {
    '--s1-toolbar-bg': 'transparent',
    '--s1-toolbar-border': 'transparent',
    '--s1-page-shadow': 'none',
    '--s1-page-border-radius': '0',
    // ...
  },
  'high-contrast': {
    '--s1-on-surface': '#000000',
    '--s1-border': '#000000',
    '--s1-primary': '#0000ff',
    '--s1-focus-ring': '0 0 0 3px #0000ff',
    // ...
  }
}
```

### Custom Theme Application

```typescript
// Option 1: Theme object
S1Editor.create(container, {
  theme: {
    name: 'acme-brand',
    primaryColor: '#ff6b35',
    backgroundColor: '#fafafa',
    toolbarBackground: '#ffffff',
    // ... override any Theme property
  }
})

// Option 2: CSS overrides
S1Editor.create(container, {
  theme: 'default',  // start from base theme
  style: {
    '--s1-primary': '#ff6b35',
    '--s1-font-family': '"Acme Sans", sans-serif'
  }
})

// Option 3: External CSS
// In your stylesheet:
.s1-editor {
  --s1-primary: #ff6b35;
  --s1-font-family: "Acme Sans", sans-serif;
}
```

---

## Layer 2: Branding

### Configuration

```typescript
S1Editor.create(container, {
  branding: {
    // Logo
    logo: {
      src: '/assets/acme-logo.svg',
      alt: 'Acme Editor',
      width: 120,
      height: 28,
      position: 'toolbar-left'    // 'toolbar-left', 'toolbar-center', 'none'
    },

    // Product name (replaces "s1engine" in all UI text)
    productName: 'Acme Docs',

    // Favicon (for standalone deployment)
    favicon: '/assets/acme-favicon.ico',

    // Loading screen
    loadingScreen: {
      logo: '/assets/acme-logo-large.svg',
      message: 'Loading Acme Docs...',
      backgroundColor: '#ffffff'
    },

    // Attribution
    poweredBy: false,             // Remove "Powered by s1engine" (default: true)

    // Links
    helpUrl: 'https://help.acme.com/docs-editor',
    feedbackUrl: 'https://feedback.acme.com',
    privacyUrl: 'https://acme.com/privacy',
    termsUrl: 'https://acme.com/terms'
  }
})
```

### Branding Scopes

| Element | Customizable | How |
|---------|-------------|-----|
| Toolbar logo | Yes | `branding.logo` |
| Product name in UI | Yes | `branding.productName` |
| Loading screen | Yes | `branding.loadingScreen` |
| Error messages | Yes | `branding.productName` replaces "s1engine" |
| Empty state | Yes | Custom empty state component |
| "Powered by" footer | Yes | `branding.poweredBy = false` |
| Page title (standalone) | Yes | `branding.productName` |
| Favicon (standalone) | Yes | `branding.favicon` |
| Help/support links | Yes | `branding.helpUrl` |
| Keyboard shortcut help | Yes | Product name in help dialog |
| About dialog | Yes | Hidden or customized |
| Export file metadata | Yes | `branding.productName` in document properties |

### Zero-Branding Mode

For complete white-labeling, set:

```typescript
{
  branding: {
    productName: 'Your Product',
    logo: { src: '/your-logo.svg' },
    poweredBy: false,
    helpUrl: 'https://your-docs.com',
  }
}
```

This removes **all** s1engine references from the UI.

---

## Layer 3: Toolbar Customization

See [EDITOR_SDK.md](EDITOR_SDK.md) for full toolbar API.

### Feature Toggles

```typescript
S1Editor.create(container, {
  features: {
    // Editing features
    tables: true,                 // Table insert/edit
    images: true,                 // Image insert
    links: true,                  // Hyperlink insert
    pageBreaks: true,             // Page break insert
    headers: true,                // Header/footer editing
    trackChanges: false,          // Track changes (hide if disabled)
    comments: false,              // Comments (hide if disabled)

    // Format features
    exportPdf: true,              // PDF export button
    exportDocx: true,             // DOCX export
    exportOdt: false,             // ODT export (hide)
    print: true,                  // Print button

    // UI features
    findReplace: true,            // Find & replace dialog
    wordCount: true,              // Word count in status bar
    pageNumbers: true,            // Page numbers in status bar
    zoom: true,                   // Zoom controls
    fullscreen: true,             // Fullscreen button
    collaboration: true,          // Collaboration features
    versionHistory: false,        // Version history panel
  }
})
```

---

## Layer 4: Locale / i18n

### Translation Files

```json
// en.json (default)
{
  "toolbar": {
    "bold": "Bold",
    "italic": "Italic",
    "underline": "Underline",
    "heading": "Heading",
    "heading_1": "Heading 1",
    "heading_2": "Heading 2",
    "heading_3": "Heading 3",
    "normal": "Normal text",
    "font_family": "Font",
    "font_size": "Font size",
    "align_left": "Align left",
    "align_center": "Center",
    "align_right": "Align right",
    "align_justify": "Justify",
    "bullet_list": "Bulleted list",
    "numbered_list": "Numbered list",
    "insert_table": "Insert table",
    "insert_image": "Insert image",
    "insert_link": "Insert link",
    "export_pdf": "Export as PDF",
    "undo": "Undo",
    "redo": "Redo",
    "find_replace": "Find and replace"
  },
  "status": {
    "page": "Page {current} of {total}",
    "words": "{count} words",
    "saved": "All changes saved",
    "saving": "Saving...",
    "unsaved": "Unsaved changes",
    "read_only": "Read only"
  },
  "collab": {
    "connected": "Connected",
    "disconnected": "Disconnected",
    "reconnecting": "Reconnecting...",
    "peers": "{count} editors",
    "you": "You"
  },
  "dialog": {
    "open_file": "Open file",
    "save_as": "Save as",
    "cancel": "Cancel",
    "ok": "OK",
    "close": "Close",
    "confirm": "Confirm",
    "delete_confirm": "Are you sure you want to delete this?"
  },
  "error": {
    "file_too_large": "File is too large. Maximum size is {maxSize}.",
    "unsupported_format": "Unsupported file format.",
    "conversion_failed": "Failed to convert document.",
    "connection_lost": "Connection lost. Reconnecting..."
  }
}
```

### Using Translations

```typescript
// Set locale on creation
S1Editor.create(container, { locale: 'es' })

// Change locale at runtime
editor.setLocale('fr')

// Provide custom translations
S1Editor.create(container, {
  locale: 'en',
  translations: {
    'toolbar.export_pdf': 'Download PDF',
    'status.saved': 'Up to date',
    'collab.peers': '{count} people editing'
  }
})

// Register a complete new locale
S1Editor.registerLocale('ja', {
  toolbar: {
    bold: '太字',
    italic: '斜体',
    // ...
  }
})
```

### Built-in Locales

| Code | Language | Status |
|------|----------|--------|
| `en` | English | Complete |
| `es` | Spanish | Phase 6 |
| `fr` | French | Phase 6 |
| `de` | German | Phase 6 |
| `pt` | Portuguese | Phase 6 |
| `zh` | Chinese (Simplified) | Phase 6 |
| `ja` | Japanese | Phase 6 |
| `ar` | Arabic (RTL) | Phase 6 |
| `he` | Hebrew (RTL) | Phase 6 |

---

## Layer 5: Domain & Hosting (Server-Side)

For consumers running their own s1-server instance:

### Custom Domain

```toml
# s1-server.toml

[white_label]
enabled = true

[[white_label.tenants]]
tenant_id = "acme"
domain = "docs.acme.com"
branding = {
  product_name = "Acme Docs",
  logo_url = "https://cdn.acme.com/logo.svg",
  favicon_url = "https://cdn.acme.com/favicon.ico",
  theme = "acme-theme",
  powered_by = false
}
```

### CDN / Asset Hosting

```toml
[white_label.assets]
# Serve editor assets from consumer's CDN
cdn_url = "https://cdn.acme.com/editor"

# Or use s1-server as asset server
serve_assets = true
asset_path = "/editor"
```

### Custom Email Templates (for sharing/notifications)

```toml
[white_label.email]
from = "noreply@acme.com"
templates_path = "/etc/s1/email-templates/acme/"
```

---

## Layer 6: Feature Gating (Per-Tenant)

For multi-tenant deployments, features can be enabled/disabled per tenant:

```toml
# s1-server.toml

[[tenants]]
id = "free-tier"
features = {
  max_documents = 10,
  max_file_size_mb = 5,
  collaboration = false,
  export_pdf = false,
  export_docx = true,
  track_changes = false,
  comments = false,
  version_history = false,
  custom_fonts = false,
  api_access = false,
}

[[tenants]]
id = "pro-tier"
features = {
  max_documents = -1,           # unlimited
  max_file_size_mb = 50,
  collaboration = true,
  export_pdf = true,
  export_docx = true,
  track_changes = true,
  comments = true,
  version_history = true,
  custom_fonts = true,
  api_access = true,
}
```

The server communicates feature flags to the client during initialization:

```json
// GET /api/v1/config (returns tenant-specific config)
{
  "features": {
    "collaboration": true,
    "export_pdf": true,
    "comments": false,
    // ...
  },
  "branding": {
    "product_name": "Acme Docs",
    "logo_url": "...",
    // ...
  },
  "limits": {
    "max_file_size_bytes": 52428800
  }
}
```

The editor SDK reads this config and adjusts the UI accordingly (hiding disabled toolbar items, enforcing limits).

---

## White-Label Checklist

Before a consumer ships their white-labeled editor, verify:

- [ ] All "s1engine" text replaced with consumer's product name
- [ ] Custom logo displays correctly in toolbar
- [ ] Custom favicon set (for standalone deployment)
- [ ] Loading screen shows consumer's branding
- [ ] "Powered by" footer hidden or customized
- [ ] Color theme matches consumer's brand guidelines
- [ ] UI font matches consumer's design system
- [ ] Error messages reference consumer's support channels
- [ ] Help/support URLs point to consumer's documentation
- [ ] Keyboard shortcut help shows consumer's product name
- [ ] Export file metadata uses consumer's product name
- [ ] Email notifications (if applicable) use consumer's branding
- [ ] Custom domain configured (if applicable)
- [ ] Browser page title shows consumer's product name
- [ ] No s1engine references visible in browser DevTools (network requests excluded)

---

## OEM Licensing Considerations

For consumers who want to redistribute s1engine as part of their product:

### AGPL-3.0 License Requirements

Under AGPL-3.0, consumers can:
- Use s1engine freely for internal or open-source projects
- Modify the source code (must share modifications under AGPL)
- Self-host and redistribute (must provide source to all network users)

**Key requirement**: Any service that uses s1engine over a network (SaaS, web app) must make the complete source code available to all users under AGPL-3.0.

### Commercial Dual-License

For consumers who need to use s1engine in proprietary/closed-source products without AGPL obligations:
- Proprietary SaaS embedding (no source disclosure)
- Warranty and liability coverage
- SLA for support and priority bug fixes
- Custom feature development
- Indemnification

A commercial license removes all AGPL obligations. This is the same model used by OnlyOffice, MongoDB, and Qt.

### Licensing Tiers

| Component | License |
|-----------|---------|
| Core engine | AGPL-3.0 (free for open-source) |
| Editor SDK | AGPL-3.0 (free for open-source) |
| Self-hosted server | AGPL-3.0 (free for open-source) |
| All of the above for proprietary use | Commercial license (paid) |
| Enterprise auth (SSO, SCIM) | Commercial only |
| Audit logging & compliance | Commercial only |
| Managed cloud service | SaaS (commercial) |
