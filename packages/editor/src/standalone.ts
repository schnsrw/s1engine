/**
 * Standalone editor entry point.
 *
 * This module re-exports the existing editor as a self-contained bundle.
 * The editor source lives in `editor/src/` (vanilla JS) and is loaded
 * via the S1Editor iframe wrapper in index.ts.
 *
 * For TypeScript migration:
 * - Phase 1 (current): Iframe-based embedding via S1Editor class
 * - Phase 2 (future): Direct DOM rendering via extracted TS modules
 *
 * Module mapping (editor/src/ → packages/editor/src/):
 *   main.js       → bootstrap.ts (initialization)
 *   render.js     → renderer.ts (document rendering)
 *   input.js      → input-handler.ts (keyboard/mouse/paste)
 *   toolbar.js    → toolbar-state.ts (toolbar state management)
 *   toolbar-handlers.js → toolbar-actions.ts (action handlers)
 *   state.js      → state.ts (application state)
 *   pagination.js → pagination.ts (page layout)
 *   collab.js     → collab-ui.ts (collaboration UI)
 *   file.js       → file-handler.ts (file I/O)
 *   find.js       → find-replace.ts (search)
 *   images.js     → image-handler.ts (image operations)
 *   ruler.js      → ruler.ts (ruler + indents + tab stops)
 *   selection.js  → selection.ts (selection management)
 *   properties-panel.js → properties-panel.ts
 *   styles.css    → styles/ (split into component CSS)
 */

// Re-export the S1Editor class for embedding
export { S1Editor, Toolbars } from './index.js';
export type { EditorOptions, Format, ToolbarConfig, ToolbarItem, Theme } from './index.js';
