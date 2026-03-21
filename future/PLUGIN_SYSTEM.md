# Plugin System Architecture

## Overview

The plugin system allows consumers to extend Rudra Code's editor and server without forking. Plugins can add toolbar buttons, side panels, keyboard shortcuts, document processors, and server-side hooks.

## Design Principles

1. **Plugins are optional** — the editor works fully without any plugins
2. **Plugins are isolated** — a broken plugin doesn't crash the editor
3. **Plugins are typed** — full TypeScript support for plugin API
4. **Plugins are composable** — plugins can depend on other plugins
5. **No runtime code execution** — plugins are loaded at build time or as ES modules (no eval/Function)

---

## Client Plugin Architecture

### Plugin Interface

```typescript
interface S1Plugin {
  /** Unique plugin identifier (reverse-domain recommended) */
  name: string

  /** Semver version */
  version: string

  /** Human-readable display name */
  displayName?: string

  /** Plugin description */
  description?: string

  /** Dependencies on other plugins */
  dependencies?: string[]

  /** Editor version compatibility */
  editorVersion?: string           // semver range, e.g., "^1.0.0"

  /** Called when plugin is loaded */
  init(context: PluginContext): void | Promise<void>

  /** Called when plugin is unloaded */
  destroy?(): void
}

interface PluginContext {
  /** The editor instance */
  editor: S1Editor

  /** The document (may change when new doc is opened) */
  getDocument(): S1Document

  /** The engine */
  getEngine(): S1Engine

  // ─── UI Extension Points ─────────────────────────────

  /** Add items to the toolbar */
  toolbar: ToolbarExtension

  /** Add side panels */
  panels: PanelExtension

  /** Add context menu items */
  contextMenu: ContextMenuExtension

  /** Add status bar items */
  statusBar: StatusBarExtension

  /** Add keyboard shortcuts */
  shortcuts: ShortcutExtension

  /** Add modal dialogs */
  modals: ModalExtension

  // ─── Document Extension Points ───────────────────────

  /** Register document event handlers */
  events: EventExtension

  /** Register custom node renderers */
  renderers: RendererExtension

  // ─── Storage & State ─────────────────────────────────

  /** Plugin-local storage (persisted to localStorage) */
  storage: PluginStorage

  /** Plugin-local state (in-memory, lost on reload) */
  state: Map<string, unknown>

  // ─── Utilities ────────────────────────────────────────

  /** i18n helper */
  t(key: string, params?: Record<string, string>): string

  /** Logger (prefixed with plugin name) */
  log: PluginLogger
}
```

### Extension Point APIs

```typescript
// ─── Toolbar Extension ──────────────────────────────────

interface ToolbarExtension {
  /** Add a button to the toolbar */
  addButton(config: {
    id: string
    label: string
    icon?: string               // SVG string
    tooltip?: string
    shortcut?: string           // Display text (e.g., "Ctrl+K")
    position?: 'start' | 'end' | { after: string } | { before: string }
    onClick: () => void
    isActive?: () => boolean    // Toggle state
    isEnabled?: () => boolean   // Disabled state
  }): void

  /** Add a dropdown to the toolbar */
  addDropdown(config: {
    id: string
    label: string
    icon?: string
    tooltip?: string
    position?: 'start' | 'end' | { after: string }
    items: DropdownItem[]
  }): void

  /** Add a separator */
  addSeparator(config: { position: { after: string } }): void

  /** Remove a toolbar item (built-in or plugin) */
  removeItem(id: string): void
}

// ─── Panel Extension ────────────────────────────────────

interface PanelExtension {
  /** Register a side panel */
  register(config: {
    id: string
    title: string
    icon?: string
    position: 'left' | 'right'
    width?: number              // pixels (default: 300)
    render: (container: HTMLElement) => void | (() => void)  // return cleanup fn
  }): void

  /** Open a registered panel */
  open(id: string): void

  /** Close a panel */
  close(id: string): void

  /** Toggle a panel */
  toggle(id: string): void
}

// ─── Context Menu Extension ─────────────────────────────

interface ContextMenuExtension {
  /** Add items to the right-click context menu */
  addItem(config: {
    id: string
    label: string
    icon?: string
    shortcut?: string
    section?: 'edit' | 'format' | 'insert' | 'custom'
    condition?: (target: ContextMenuTarget) => boolean  // Show only when condition met
    onClick: (target: ContextMenuTarget) => void
  }): void
}

interface ContextMenuTarget {
  type: 'text' | 'image' | 'table' | 'link' | 'page'
  nodeId?: string
  selection?: SelectionRange
  element?: HTMLElement
}

// ─── Keyboard Shortcut Extension ────────────────────────

interface ShortcutExtension {
  /** Register a keyboard shortcut */
  register(config: {
    key: string                 // e.g., "ctrl+shift+k", "mod+k" (mod = ctrl/cmd)
    description: string
    handler: () => void | boolean  // return false to allow default
    when?: 'editing' | 'always'    // default: 'editing'
  }): void

  /** Unregister a shortcut */
  unregister(key: string): void
}

// ─── Status Bar Extension ───────────────────────────────

interface StatusBarExtension {
  /** Add an item to the status bar */
  addItem(config: {
    id: string
    position: 'left' | 'center' | 'right'
    render: (container: HTMLElement) => void
    update?: () => void         // Called on document change
  }): void
}

// ─── Modal Extension ────────────────────────────────────

interface ModalExtension {
  /** Show a modal dialog */
  show(config: {
    title: string
    width?: number
    height?: number
    render: (container: HTMLElement) => void | (() => void)
    onClose?: () => void
    buttons?: ModalButton[]
  }): ModalHandle
}

interface ModalHandle {
  close(): void
  setTitle(title: string): void
}

// ─── Event Extension ────────────────────────────────────

interface EventExtension {
  /** Listen to document events */
  on(event: string, handler: Function): () => void

  /** Listen to editor events */
  onEditor(event: string, handler: Function): () => void
}

// ─── Renderer Extension ─────────────────────────────────

interface RendererExtension {
  /** Register a custom renderer for a node type */
  registerNodeRenderer(config: {
    nodeType: string            // e.g., "custom:chart", "custom:embed"
    render: (node: NodeData, container: HTMLElement) => void
    update?: (node: NodeData, container: HTMLElement) => void
  }): void

  /** Register a decorator (overlays on existing nodes) */
  registerDecorator(config: {
    id: string
    match: (node: NodeData) => boolean
    decorate: (node: NodeData, element: HTMLElement) => void
  }): void
}

// ─── Plugin Storage ─────────────────────────────────────

interface PluginStorage {
  get<T>(key: string): T | null
  set(key: string, value: unknown): void
  remove(key: string): void
  clear(): void
}
```

---

## Built-in Plugins

### Comments Plugin

```typescript
// @rudra/plugin-comments

const commentsPlugin: S1Plugin = {
  name: '@rudra/plugin-comments',
  version: '1.0.0',
  displayName: 'Comments',

  init(ctx: PluginContext) {
    // Add toolbar button
    ctx.toolbar.addButton({
      id: 'add-comment',
      label: 'Comment',
      icon: commentIcon,
      tooltip: 'Add comment (Ctrl+Alt+M)',
      position: { after: 'insert-link' },
      onClick: () => showCommentDialog(ctx),
      isEnabled: () => ctx.getDocument().hasSelection()
    })

    // Add keyboard shortcut
    ctx.shortcuts.register({
      key: 'ctrl+alt+m',
      description: 'Add comment',
      handler: () => showCommentDialog(ctx)
    })

    // Add side panel
    ctx.panels.register({
      id: 'comments-panel',
      title: 'Comments',
      icon: commentListIcon,
      position: 'right',
      render: (container) => renderCommentsPanel(ctx, container)
    })

    // Add context menu item
    ctx.contextMenu.addItem({
      id: 'reply-comment',
      label: 'Reply to comment',
      section: 'custom',
      condition: (target) => target.type === 'text' && hasCommentAt(target),
      onClick: (target) => showReplyDialog(ctx, target)
    })

    // Listen for document changes to update comment positions
    ctx.events.on('change', () => updateCommentPositions(ctx))
  }
}
```

### Find & Replace Plugin

```typescript
const findReplacePlugin: S1Plugin = {
  name: '@rudra/plugin-find-replace',
  version: '1.0.0',
  displayName: 'Find & Replace',

  init(ctx: PluginContext) {
    ctx.shortcuts.register({
      key: 'mod+f',
      description: 'Find',
      handler: () => ctx.panels.open('find-replace')
    })

    ctx.shortcuts.register({
      key: 'mod+h',
      description: 'Find and Replace',
      handler: () => {
        ctx.panels.open('find-replace')
        // Focus replace input
      }
    })

    ctx.toolbar.addButton({
      id: 'find-replace',
      label: 'Find',
      icon: searchIcon,
      tooltip: 'Find and Replace (Ctrl+F)',
      onClick: () => ctx.panels.toggle('find-replace')
    })

    ctx.panels.register({
      id: 'find-replace',
      title: 'Find & Replace',
      position: 'right',
      width: 320,
      render: (container) => renderFindReplacePanel(ctx, container)
    })
  }
}
```

### Word Count Plugin

```typescript
const wordCountPlugin: S1Plugin = {
  name: '@rudra/plugin-word-count',
  version: '1.0.0',

  init(ctx: PluginContext) {
    ctx.statusBar.addItem({
      id: 'word-count',
      position: 'right',
      render: (el) => {
        el.textContent = `${ctx.getDocument().wordCount} words`
      },
      update: () => {
        // Called on every document change
        const el = document.getElementById('s1-plugin-word-count')
        if (el) el.textContent = `${ctx.getDocument().wordCount} words`
      }
    })
  }
}
```

### Track Changes Plugin

```typescript
const trackChangesPlugin: S1Plugin = {
  name: '@rudra/plugin-track-changes',
  version: '1.0.0',

  init(ctx: PluginContext) {
    // Toggle track changes mode
    ctx.toolbar.addButton({
      id: 'track-changes',
      label: 'Track Changes',
      tooltip: 'Toggle Track Changes',
      onClick: () => toggleTrackChanges(ctx),
      isActive: () => isTrackChangesEnabled(ctx)
    })

    // Accept/reject buttons (shown when track changes are present)
    ctx.toolbar.addDropdown({
      id: 'review-changes',
      label: 'Review',
      items: [
        { label: 'Accept Change', onClick: () => acceptChange(ctx) },
        { label: 'Reject Change', onClick: () => rejectChange(ctx) },
        { separator: true },
        { label: 'Accept All', onClick: () => acceptAll(ctx) },
        { label: 'Reject All', onClick: () => rejectAll(ctx) },
      ]
    })

    // Render change markers in document
    ctx.renderers.registerDecorator({
      id: 'track-changes-markers',
      match: (node) => node.hasTrackChanges(),
      decorate: (node, el) => {
        if (node.isInsertion()) {
          el.classList.add('s1-track-insert')
        } else if (node.isDeletion()) {
          el.classList.add('s1-track-delete')
        }
      }
    })

    // Side panel for change list
    ctx.panels.register({
      id: 'changes-panel',
      title: 'Changes',
      position: 'right',
      render: (container) => renderChangesPanel(ctx, container)
    })
  }
}
```

### Version History Plugin

```typescript
const versionHistoryPlugin: S1Plugin = {
  name: '@rudra/plugin-version-history',
  version: '1.0.0',

  init(ctx: PluginContext) {
    ctx.toolbar.addButton({
      id: 'version-history',
      label: 'History',
      tooltip: 'Version History',
      onClick: () => ctx.panels.toggle('version-history')
    })

    ctx.panels.register({
      id: 'version-history',
      title: 'Version History',
      position: 'right',
      width: 300,
      render: (container) => {
        // List versions from server API or IndexedDB
        // Show diff between versions
        // Allow restoring previous version
      }
    })
  }
}
```

---

## Server Plugin Architecture

### Server Plugin Interface

```rust
#[async_trait]
trait ServerPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    /// Called when the server starts
    async fn init(&self, ctx: &ServerPluginContext) -> Result<()>;

    /// Called when the server stops
    async fn shutdown(&self) -> Result<()>;
}

struct ServerPluginContext {
    /// Register additional HTTP routes
    pub router: RouterExtension,

    /// Register middleware hooks
    pub hooks: HookRegistry,

    /// Access to storage backend
    pub storage: Arc<dyn StorageBackend>,

    /// Access to configuration
    pub config: Arc<Config>,
}
```

### Server Hook Points

```rust
#[async_trait]
trait DocumentHooks: Send + Sync {
    /// Called before a document is saved
    /// Return Err to reject the save
    async fn before_save(
        &self,
        doc_id: &str,
        content: &[u8],
        metadata: &DocumentMetadata,
    ) -> Result<()> {
        Ok(()) // default: allow
    }

    /// Called after a document is saved
    async fn after_save(
        &self,
        doc_id: &str,
        metadata: &DocumentMetadata,
    ) -> Result<()> {
        Ok(())
    }

    /// Called before a document is exported
    /// Can modify the export (e.g., add watermark)
    async fn before_export(
        &self,
        doc_id: &str,
        format: Format,
        content: &mut Vec<u8>,
    ) -> Result<()> {
        Ok(())
    }

    /// Called when a document is opened
    async fn on_open(
        &self,
        doc_id: &str,
        user_id: &str,
    ) -> Result<()> {
        Ok(())
    }
}
```

### Example Server Plugins

```rust
// Watermark plugin — adds watermark to PDF exports
struct WatermarkPlugin {
    watermark_text: String,
}

#[async_trait]
impl DocumentHooks for WatermarkPlugin {
    async fn before_export(
        &self,
        _doc_id: &str,
        format: Format,
        content: &mut Vec<u8>,
    ) -> Result<()> {
        if format == Format::Pdf {
            add_watermark(content, &self.watermark_text)?;
        }
        Ok(())
    }
}

// Compliance plugin — validates documents before save
struct CompliancePlugin {
    rules: Vec<ComplianceRule>,
}

#[async_trait]
impl DocumentHooks for CompliancePlugin {
    async fn before_save(
        &self,
        doc_id: &str,
        content: &[u8],
        metadata: &DocumentMetadata,
    ) -> Result<()> {
        let doc = Engine::new().open(content)?;
        for rule in &self.rules {
            rule.validate(&doc)?;
        }
        Ok(())
    }
}
```

---

## Plugin Loading

### Client-Side

```typescript
import { S1Editor } from '@rudra/editor'
import { commentsPlugin } from '@rudra/plugin-comments'
import { findReplacePlugin } from '@rudra/plugin-find-replace'
import { myCustomPlugin } from './my-plugin'

const editor = await S1Editor.create(container, {
  plugins: [
    commentsPlugin,
    findReplacePlugin,
    myCustomPlugin({ option1: 'value' })
  ]
})
```

### Server-Side

```toml
# s1-server.toml
[plugins]
enabled = ["watermark", "compliance"]

[plugins.watermark]
text = "CONFIDENTIAL"
opacity = 0.3

[plugins.compliance]
rules = ["no-pii", "max-file-size"]
max_file_size_mb = 100
```

---

## Plugin Development Guide

### Creating a Plugin

```typescript
// my-plugin.ts
import type { S1Plugin, PluginContext } from '@rudra/editor'

export function myPlugin(options: MyPluginOptions = {}): S1Plugin {
  return {
    name: 'com.example.my-plugin',
    version: '1.0.0',
    displayName: 'My Plugin',

    init(ctx: PluginContext) {
      // Use ctx to extend the editor
      ctx.toolbar.addButton({
        id: 'my-action',
        label: options.buttonLabel || 'My Action',
        onClick: () => {
          const doc = ctx.getDocument()
          // Do something with the document
          ctx.log.info('My action executed')
        }
      })
    },

    destroy() {
      // Clean up resources
    }
  }
}

interface MyPluginOptions {
  buttonLabel?: string
}
```

### Plugin Isolation

Plugins run in the same JavaScript context as the editor (no iframe/worker isolation) for performance. However:

- **Error boundaries**: Plugin init/destroy/event handlers are wrapped in try-catch
- **Cleanup**: All registrations (toolbar items, panels, shortcuts) are tracked and cleaned up on destroy
- **Scoped storage**: `ctx.storage` is namespaced by plugin name
- **Scoped logging**: `ctx.log` prefixes messages with plugin name
- **API contract**: Breaking the PluginContext API (e.g., modifying private properties) voids support

### Plugin Compatibility

```typescript
// Plugin declares compatibility
{
  name: 'my-plugin',
  editorVersion: '>=1.0.0 <2.0.0',  // works with editor 1.x
  dependencies: ['@rudra/plugin-comments']  // requires comments plugin
}

// Editor checks on load
S1Editor.create(container, {
  plugins: [myPlugin()]
  // If editorVersion doesn't match: warning in console
  // If dependency missing: error, plugin not loaded
})
```
