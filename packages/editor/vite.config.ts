import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'S1Editor',
      formats: ['es', 'umd'],
      fileName: (format) => `s1-editor.${format === 'es' ? 'js' : 'umd.cjs'}`,
    },
    rollupOptions: {
      external: ['@s1engine/wasm', '@s1engine/sdk'],
      output: {
        globals: {
          '@s1engine/wasm': 'S1EngineWasm',
          '@s1engine/sdk': 'S1EngineSdk',
        },
      },
    },
    outDir: 'dist',
    sourcemap: true,
  },
});
