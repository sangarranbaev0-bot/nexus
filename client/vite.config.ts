import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: '/nexus/',
  // Для GitHub Pages укажите полный адрес WebSocket-сервера, иначе подключается к тому же хосту.
  // Пример: VITE_WS_URL=https://chat.example.com
  server: {
    proxy: {
      '/api': 'http://localhost:3001',
      '/ws': { target: 'ws://localhost:3001', ws: true },
    },
  },
});
