import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  publicDir: 'public',
  server: {
    port: 3000,
    open: true,
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  build: {
    outDir: 'dist',
    target: 'esnext',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Keep WASM bindings in their own chunk
          if (id.includes('wasm-pkg')) {
            return 'wasm';
          }
          // Vendor chunk for node_modules (KaTeX, pdfjs-dist, etc.)
          if (id.includes('node_modules')) {
            return 'vendor';
          }
        },
      },
    },
  },
  assetsInclude: ['**/*.wasm'],
  optimizeDeps: {
    exclude: ['s1engine_wasm'],
  },
  worker: {
    format: 'es',
  },
});
