import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// In dev, the Rust server (port 7070) owns /api and /ws; Vite proxies to it.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': 'http://127.0.0.1:7070',
      '/ws': { target: 'ws://127.0.0.1:7070', ws: true },
    },
  },
});
