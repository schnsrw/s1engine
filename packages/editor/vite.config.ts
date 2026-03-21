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
      external: ['@rudra/wasm', '@rudra/sdk'],
      output: {
        globals: {
          '@rudra/wasm': 'S1EngineWasm',
          '@rudra/sdk': 'S1EngineSdk',
        },
      },
    },
    outDir: 'dist',
    sourcemap: true,
  },
});
