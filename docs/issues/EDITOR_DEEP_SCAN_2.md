# Editor Deep Scan #2 — Functional, UI & UX Issues

> Created: 2026-03-18 | Last updated: 2026-03-18
> 35 issues found, 31 fixed, 1 already-correct (100% resolved)

## Critical

| ID | Issue | File | Status |
|----|-------|------|--------|
| ED2-01 | Collab offline buffer drops ops silently after 10K — data loss risk | collab.js:175-186 | FIXED |

## High

| ID | Issue | File | Status |
|----|-------|------|--------|
| ED2-02 | Event listener accumulation in image delegation — memory leak | images.js:24-40 | FIXED |
| ED2-03 | Uncleared timers on document close (layout/find debounce) | file.js:354-365 | FIXED |
| ED2-04 | Virtual scroll race condition with concurrent renders | render.js:1249-1260 | FIXED |
| ED2-05 | Touch handle drag listeners accumulate on reinit | touch.js:103-147 | FIXED |
| ED2-06 | Pending formats cleared on blur to toolbar — format loss | input.js:20-23 | FIXED |
| ED2-07 | Null check missing in collab message handler | collab.js:286 | FIXED |
| ED2-08 | Find module event listeners duplicated on reinit | find.js:15-30 | FIXED |
| ED2-09 | Track changes popup setTimeout race condition | render.js:348 | FIXED |
| ED2-10 | Properties panel selection listener never removed | properties-panel.js:57 | FIXED |
| ED2-11 | Touch context menu cleanup ineffective | touch.js:404-405 | FIXED |
| ED2-12 | Inline styles break dark mode — images, touch UI | images.js:73 | FIXED |
| ED2-13 | Paste/cut missing line breaks, paragraph structure, DOCX fidelity | input.js:851-994 | FIXED |

## Medium

| ID | Issue | File | Status |
|----|-------|------|--------|
| ED2-14 | Image resize listeners leak on tab switch | images.js:244-247 | FIXED |
| ED2-15 | Missing tooltips on many interactive elements | various | FIXED |
| ED2-16 | Synchronous querySelectorAll in hot render path | render.js:404-406 | FIXED |
| ED2-17 | Missing try-catch on many WASM calls | toolbar-handlers.js:3165+ | FIXED |
| ED2-18 | Modal selection restore fails after re-render | toolbar-handlers.js:32-42 | FIXED |
| ED2-19 | Paste handler may access null clipboard data | input.js:1322-1360 | FIXED |
| ED2-20 | PDF viewer scroll handler never removed | pdf-viewer.js:169 | FIXED |
| ED2-21 | Format painter state not reset on undo | toolbar-handlers.js | FIXED |
| ED2-31 | Tab key inserts \t text instead of Tab node — renders as collapsed whitespace | input.js:644-662 | FIXED |

## Low

| ID | Issue | File | Status |
|----|-------|------|--------|
| ED2-22 | Find debounce fires after bar closed | find.js:227 | FIXED |
| ED2-23 | Slash menu state not cleared on Escape | input.js:375 | OK |
| ED2-24 | Virtual scroll placeholders not cleaned up | render.js | FIXED |
| ED2-25 | Autosave failures silently swallowed | file.js:141 | FIXED |
| ED2-26 | Properties panel debounce fires when hidden | properties-panel.js:51-68 | FIXED |
| ED2-27 | No large document warning shown to user | render.js:22-24 | FIXED |
| ED2-28 | No undo for image drag & drop | images.js:99-110 | FIXED |
| ED2-29 | Find bar not closed on document close | find.js | FIXED |
| ED2-30 | Missing copy feedback in share dialog | collab.js:1040-1042 | FIXED |

---

## Resolution Log

| ID | Date | Fix Description |
|----|------|-----------------|
| ED2-01 | 2026-03-18 | Added warning toast at 8K ops, error toast at 10K limit; flags reset on reconnect |
| ED2-02 | 2026-03-18 | Refactored setupImages() with early guard; delegation runs once, subsequent calls only mark draggable |
| ED2-03 | 2026-03-18 | Added clearTimeout for _typingBatch.timer, _findRefreshTimer, autosaveTimer, versionTimer in newDocument() |
| ED2-04 | 2026-03-18 | Added _rendering flag; virtual scroll skips during active render |
| ED2-06 | 2026-03-18 | Blur handler checks e.relatedTarget — skips clear if target is toolbar/modal/find-bar/props-panel |
| ED2-07 | 2026-03-18 | Added null check for pageContainer in collab setText handler |
| ED2-08 | 2026-03-18 | Added _findInitialized guard flag to prevent duplicate listeners |
| ED2-09 | 2026-03-18 | Replaced setTimeout(0) with requestAnimationFrame for dismiss listener |
| ED2-10 | 2026-03-18 | hidePropertiesPanel removes selectionchange listener; showPropertiesPanel re-adds it |
| ED2-11 | 2026-03-18 | Explicit removeEventListener before adding new outside-click handlers; rAF instead of setTimeout |
| ED2-12 | 2026-03-18 | Replaced inline style.cssText on drop indicator with .img-drop-indicator CSS class + dark mode override |
| ED2-13 | 2026-03-18 | Normalized \r\n→\n; <br> at block level now creates paragraph separator; line breaks preserved |
| ED2-14 | 2026-03-18 | removeEventListener before addEventHandler in startResize; visibilitychange cleanup on tab hide |
| ED2-17 | 2026-03-18 | Wrapped applyParagraphStyle, createFromTemplate, loadCustomTemplate WASM calls in try-catch |
| ED2-18 | 2026-03-18 | Added isConnected check on saved range; fallback to first paragraph cursor placement |
| ED2-19 | 2026-03-18 | Added null checks for clipboard items, types, and blob in context menu paste |
| ED2-21 | 2026-03-18 | exitFormatPainter() called in doUndo() and doRedo() when format painter active |
| ED2-22 | 2026-03-18 | Created closeFindBar() that clears _findRefreshTimer; all close paths use it |
| ED2-23 | 2026-03-18 | Already correct: closeSlashMenu() already sets state.slashMenuOpen = false |
| ED2-25 | 2026-03-18 | Replaced .catch(() => {}) with console.warn + status bar "Autosave failed" message |
| ED2-29 | 2026-03-18 | Added closeFindBar() calls in newDocument() and openFile() |
| ED2-31 | 2026-03-18 | Added insert_tab WASM API (creates Tab node like insert_line_break); editor calls insert_tab instead of inserting \t text |
| ED2-05 | 2026-03-18 | Added _handleDragSetup guard flag in touch.js; setupHandleDrag() runs once preventing listener accumulation |
| ED2-15 | 2026-03-18 | Added title attributes to 18 elements in index.html: doc name, File/Insert menu items, find bar controls, history tabs |
| ED2-16 | 2026-03-18 | Added _nodeMapDirty flag; populateNodeIdMap() skips querySelectorAll when map is still valid (incremental renders) |
| ED2-20 | 2026-03-18 | Added pdfViewer.destroy() call in switchView() when leaving PDF view; removes scroll handler and resize observer |
| ED2-24 | 2026-03-18 | teardownVirtualScroll() now removes remaining .vs-placeholder elements from page container |
| ED2-26 | 2026-03-18 | Debounce callbacks in properties-panel.js check panel visibility before updating; skip if hidden |
| ED2-27 | 2026-03-18 | Added one-time toast "Large document — some features may be slower" when document exceeds 500 paragraph threshold |
| ED2-28 | 2026-03-18 | Added recordUndoAction('Move image') after drag-and-drop move operation in images.js |
| ED2-30 | 2026-03-18 | Enhanced copyShareUrl(): shows green checkmark + "Copied!" text with green styling, reverts after 1.5s |
