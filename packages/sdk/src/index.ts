/**
 * @s1engine/sdk — Headless JavaScript/TypeScript SDK for s1engine.
 *
 * @example
 * ```ts
 * import { S1Engine } from '@s1engine/sdk';
 *
 * const engine = await S1Engine.init();
 * const doc = engine.create();
 * doc.title = 'My Document';
 * const pdf = doc.export('pdf');
 * ```
 *
 * @packageDocumentation
 */

export { S1Engine } from './engine.js';
export { S1Document } from './document.js';
export { EventEmitter } from './events.js';

// Re-export all types
export type {
  Format,
  SourceFormat,
  Position,
  SelectionRange,
  EditorEvent,
  ChangeEvent,
  CollabConfig,
  CollabPeer,
  DocumentMetadata,
  DocumentStats,
  LayoutConfig,
  EditorOptions,
  Theme,
  ToolbarConfig,
  ToolbarItem,
  AutosaveConfig,
  BrandingConfig,
} from './types.js';

export { S1Error, ErrorCodes } from './types.js';
