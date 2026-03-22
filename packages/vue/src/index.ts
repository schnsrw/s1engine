/**
 * @rudra/vue — Vue 3 adapter for the Rudra Office editor.
 *
 * @example
 * ```vue
 * <template>
 *   <S1EditorVue
 *     theme="default"
 *     toolbar="standard"
 *     @ready="onReady"
 *     @change="onChange"
 *   />
 * </template>
 *
 * <script setup>
 * import { S1EditorVue } from '@rudra/vue';
 * </script>
 * ```
 */

import { defineComponent, ref, onMounted, onUnmounted, h, type PropType } from 'vue';
import { S1Editor, Toolbars } from '@rudra/editor';
import type { EditorOptions, ToolbarConfig, Format } from '@rudra/editor';

export const S1EditorVue = defineComponent({
  name: 'S1Editor',

  props: {
    theme: { type: String as PropType<'default' | 'dark' | 'minimal'>, default: 'default' },
    toolbar: { type: [String, Object, Boolean] as PropType<keyof typeof Toolbars | ToolbarConfig | false>, default: 'standard' },
    readOnly: { type: Boolean, default: false },
    spellcheck: { type: Boolean, default: true },
  },

  emits: ['ready', 'change', 'save', 'error'],

  setup(props, { emit, expose }) {
    const containerRef = ref<HTMLElement | null>(null);
    const editorRef = ref<S1Editor | null>(null);

    onMounted(async () => {
      if (!containerRef.value) return;

      const resolvedToolbar = props.toolbar === false
        ? false
        : typeof props.toolbar === 'string'
          ? Toolbars[props.toolbar as keyof typeof Toolbars]
          : props.toolbar;

      try {
        const editor = await S1Editor.create(containerRef.value, {
          theme: props.theme,
          toolbar: resolvedToolbar || undefined,
          readOnly: props.readOnly,
          spellcheck: props.spellcheck,
          onReady: () => emit('ready'),
          onChange: (e) => emit('change', e),
          onError: (e) => emit('error', e),
        });

        editorRef.value = editor;
      } catch (err) {
        emit('error', err);
      }
    });

    onUnmounted(() => {
      editorRef.value?.destroy();
      editorRef.value = null;
    });

    expose({
      editor: editorRef,
      open: (data: ArrayBuffer) => editorRef.value?.open(data),
      openUrl: (url: string) => editorRef.value?.openUrl(url),
      createNew: () => editorRef.value?.createNew(),
      exportDocument: (format: Format) => editorRef.value?.exportDocument(format),
    });

    return () => h('div', {
      ref: containerRef,
      style: { width: '100%', height: '100%' },
    });
  },
});

export { S1Editor, Toolbars } from '@rudra/editor';
export type { EditorOptions, Format, ToolbarConfig } from '@rudra/editor';
