# Phase 2: Embeddable Editor SDK

> Goal: Transform the editor into an embeddable component with a clean JS/TS API.
> Created: 2026-03-18 | Depends on: Phase 1 (complete)

## Milestone 2.1 — Core SDK (@s1engine/sdk)

| ID | Task | Status |
|----|------|--------|
| P2-01 | Create packages/sdk/ with package.json and tsconfig.json | DONE |
| P2-02 | Implement S1Engine class (init, create, open, openUrl, detectFormat) | DONE |
| P2-03 | Implement S1Document class (toHTML, export, editing, metadata, stats) | DONE |
| P2-04 | Implement EventEmitter with typed events | DONE |
| P2-05 | Implement typed error classes (S1Error with codes) | DONE |
| P2-06 | Full TypeScript definitions for all public APIs | DONE |

## Milestone 2.2 — Editor Component (@s1engine/editor)

| ID | Task | Status |
|----|------|--------|
| P2-07 | Create packages/editor/ with package.json and build config | DONE |
| P2-08 | Implement S1Editor class (create, open, destroy, setToolbar, setTheme) | DONE |
| P2-09 | Implement EditorOptions (theme, toolbar, collab, callbacks) | DONE |
| P2-10 | Implement ToolbarConfig (presets: full, standard, minimal, none) | DONE |
| P2-11 | Extract existing editor JS into the package structure | DONE |
| P2-12 | Vite library mode build (UMD + ESM output) | DONE |

## Milestone 2.3 — Framework Adapters

| ID | Task | Status |
|----|------|--------|
| P2-13 | React adapter (@s1engine/react) with useRef + forwardRef | DONE |
| P2-14 | Vue adapter (@s1engine/vue) with defineComponent + expose | DONE |
| P2-15 | Web Component (@s1engine/web-component) with custom element | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P2-01 | 2026-03-18 | packages/sdk/ created with package.json (@s1engine/sdk), tsconfig.json (ES2020, strict) |
| P2-02 | 2026-03-18 | S1Engine: init() with dynamic WASM import, create(), open(), openUrl(), detectFormat(), version |
| P2-03 | 2026-03-18 | S1Document: toHTML/toPlainText/toPaginatedHTML, export/exportBlob/exportDataUrl, insertText/deleteText/formatSelection/splitParagraph/insertTable/insertImage, undo/redo, metadata/title/stats/wordCount/pageCount, isDirty, destroy() |
| P2-04 | 2026-03-18 | EventEmitter<T>: typed on/once/off/removeAllListeners/emit with error isolation |
| P2-05 | 2026-03-18 | S1Error class with code + details, ErrorCodes enum (INIT_FAILED, WASM_ERROR, etc.) |
| P2-06 | 2026-03-18 | Full types: Format, Position, SelectionRange, EditorOptions, Theme, ToolbarConfig, CollabConfig, DocumentStats, BrandingConfig |
| P2-07 | 2026-03-18 | packages/editor/ created with Vite build config, ESM output |
| P2-08 | 2026-03-18 | S1Editor: create(), open(), openUrl(), createNew(), exportDocument(), readOnly, setToolbar(), setTheme(), destroy() |
| P2-09 | 2026-03-18 | EditorOptions: theme, locale, toolbar, statusBar, ruler, pageView, readOnly, autosave, spellcheck, collab, branding, callbacks |
| P2-10 | 2026-03-18 | Toolbars object: full (28 items), standard (16 items), minimal (5 items), none (0 items) |
| P2-13 | 2026-03-18 | @s1engine/react: S1EditorComponent with forwardRef, useImperativeHandle, lifecycle cleanup |
| P2-14 | 2026-03-18 | @s1engine/vue: S1EditorVue with defineComponent, props, emits, expose, lifecycle |
| P2-15 | 2026-03-18 | @s1engine/web-component: <s1-editor> custom element with shadow DOM, observed attributes, public API |
