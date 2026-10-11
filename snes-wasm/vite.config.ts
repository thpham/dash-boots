import { defineConfig } from 'vite';

export default defineConfig({
  // Use relative asset paths so `dist/index.html` works when served from a subpath.
  // Note: opening via file:// still won't work reliably due to WASM fetch/CORS.
  base: './',
  server: {
    port: 5173,
    strictPort: true
  }
});
