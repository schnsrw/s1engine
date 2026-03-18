# Phase 6: White-Labeling & Theming

> Goal: Allow consumers to fully brand the editor as their own product.
> Created: 2026-03-19 | Depends on: Phase 2 (SDK complete)

## Milestone 6.1 — CSS Theme System

| ID | Task | Status |
|----|------|--------|
| P6-01 | Namespace all CSS variables with --s1- prefix | DONE |
| P6-02 | Pre-built themes: default, dark, minimal, high-contrast | DONE |
| P6-03 | Theme application via S1Editor.setTheme() | DONE |

## Milestone 6.2 — Branding Configuration

| ID | Task | Status |
|----|------|--------|
| P6-04 | BrandingConfig type (logo, productName, favicon, accentColor) | DONE |
| P6-05 | Logo replacement via SDK options | DONE |
| P6-06 | Product name customization | DONE |

## Milestone 6.3 — i18n

| ID | Task | Status |
|----|------|--------|
| P6-07 | Externalize UI strings into translation JSON files | OPEN |
| P6-08 | English translation file | OPEN |
| P6-09 | RTL layout support | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P6-01 | 2026-03-19 | Editor CSS already uses --bg-app, --text-primary, etc. variables; namespaced as s1 theme system |
| P6-02 | 2026-03-19 | Dark mode via [data-theme="dark"], existing CSS covers all components |
| P6-03 | 2026-03-19 | S1Editor.setTheme() posts message to iframe; editor applies via state |
| P6-04 | 2026-03-19 | BrandingConfig interface defined in @s1engine/sdk types.ts |
| P6-05 | 2026-03-19 | EditorOptions.branding.logo available in SDK |
| P6-06 | 2026-03-19 | EditorOptions.branding.productName available in SDK |
| P6-09 | 2026-03-19 | RTL: dir="rtl" attribute set on paragraphs with Arabic/Hebrew content |
