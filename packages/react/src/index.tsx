/**
 * @s1engine/react — React adapter for the s1engine editor.
 *
 * @example
 * ```tsx
 * import { S1EditorComponent } from '@s1engine/react';
 *
 * function MyApp() {
 *   return (
 *     <S1EditorComponent
 *       theme="default"
 *       toolbar="standard"
 *       onReady={() => console.log('Ready!')}
 *       onChange={(e) => console.log('Changed:', e)}
 *     />
 *   );
 * }
 * ```
 */

import React, { useEffect, useRef, useImperativeHandle, forwardRef } from 'react';
import { S1Editor, Toolbars } from '@s1engine/editor';
import type { EditorOptions, Format, ToolbarConfig } from '@s1engine/editor';

export interface S1EditorProps extends Omit<EditorOptions, 'toolbar'> {
  /** Toolbar preset name or custom config. */
  toolbar?: keyof typeof Toolbars | ToolbarConfig | false;
  /** CSS class for the container. */
  className?: string;
  /** Inline style for the container. */
  style?: React.CSSProperties;
}

export interface S1EditorRef {
  /** The underlying S1Editor instance. */
  editor: S1Editor | null;
  /** Open a document from bytes. */
  open: (data: ArrayBuffer) => void;
  /** Open a document from URL. */
  openUrl: (url: string) => Promise<void>;
  /** Create a new document. */
  createNew: () => void;
  /** Export the document. */
  exportDocument: (format: Format) => Promise<Blob>;
}

export const S1EditorComponent = forwardRef<S1EditorRef, S1EditorProps>(
  function S1EditorComponent(props, ref) {
    const containerRef = useRef<HTMLDivElement>(null);
    const editorRef = useRef<S1Editor | null>(null);

    const {
      toolbar = 'standard',
      className,
      style,
      ...editorOptions
    } = props;

    useImperativeHandle(ref, () => ({
      get editor() { return editorRef.current; },
      open: (data: ArrayBuffer) => editorRef.current?.open(data),
      openUrl: (url: string) => editorRef.current?.openUrl(url) ?? Promise.resolve(),
      createNew: () => editorRef.current?.createNew(),
      exportDocument: (format: Format) =>
        editorRef.current?.exportDocument(format) ?? Promise.reject(new Error('Editor not ready')),
    }));

    useEffect(() => {
      if (!containerRef.current) return;

      const resolvedToolbar = toolbar === false
        ? false
        : typeof toolbar === 'string'
          ? Toolbars[toolbar]
          : toolbar;

      let mounted = true;
      let editor: S1Editor | null = null;

      S1Editor.create(containerRef.current, {
        ...editorOptions,
        toolbar: resolvedToolbar || undefined,
      }).then((e) => {
        if (!mounted) { e.destroy(); return; }
        editor = e;
        editorRef.current = e;
      });

      return () => {
        mounted = false;
        editor?.destroy();
        editorRef.current = null;
      };
    }, []); // eslint-disable-line react-hooks/exhaustive-deps

    return (
      <div
        ref={containerRef}
        className={className}
        style={{ width: '100%', height: '100%', ...style }}
      />
    );
  }
);

export { S1Editor, Toolbars } from '@s1engine/editor';
export type { EditorOptions, Format, ToolbarConfig, ToolbarItem, Theme } from '@s1engine/editor';
