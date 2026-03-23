import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  root: '.',
  publicDir: 'public',
  server: {
    port: 3000,
    open: true,
    fs: {
      allow: [
        resolve(__dirname, '..'),
      ],
    },
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
      input: {
        main: resolve(__dirname, 'index.html'),
        admin: resolve(__dirname, 'admin.html'),
      },
      output: {
        manualChunks(id) {
          // Keep WASM bindings in their own chunk
          if (id.includes('wasm-pkg') || id.includes('packages/wasm/dist')) {
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
