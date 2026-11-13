import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// Прокси делает API и WebSocket same-origin, поэтому CORS на сервере не нужен.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/api': 'http://localhost:3001',
      '/ws': { target: 'ws://localhost:3001', ws: true },
    },
  },
});
