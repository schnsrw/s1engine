# White-Labeling

Customize the editor's appearance and branding.

## Theme System

The editor uses CSS custom properties for all visual elements:

```css
:root {
  --bg-app: #f8f9fa;
  --bg-white: #fff;
  --bg-toolbar: #edf2fa;
  --text-primary: #202124;
  --text-secondary: #5f6368;
  --accent: #1a73e8;
  --font-ui: 'Inter', sans-serif;
}
```

## Dark Mode

Toggle via `data-theme="dark"` on the document root:

```javascript
document.documentElement.setAttribute('data-theme', 'dark');
```

## Branding

Via the SDK EditorOptions:

```typescript
S1Editor.create(container, {
  branding: {
    logo: '/my-logo.svg',
    productName: 'MyDocs',
    accentColor: '#6200ea',
  }
});
```

## Toolbar Customization

```typescript
S1Editor.create(container, {
  toolbar: {
    items: ['bold', 'italic', '|', 'heading', '|', 'insert-image']
  }
});
```

## Locale / i18n

```javascript
import { setLocale } from './i18n/index.js';
setLocale('es', { toolbar: { bold: 'Negrita' } });
```
