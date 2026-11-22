import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import gzipPlugin from 'rollup-plugin-gzip';

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    gzipPlugin(),
  ],
  base: '/nexus/',
  // Для GitHub Pages укажите полный адрес WebSocket-сервера, иначе подключается к тому же хосту.
  // Пример: VITE_WS_URL=https://chat.example.com
  server: {
    proxy: {
      '/api': 'http://localhost:3001',
      '/ws': { target: 'ws://localhost:3001', ws: true },
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules/react') || id.includes('node_modules/react-dom')) return 'vendor';
          if (id.includes('node_modules/react-router')) return 'router';
          if (id.includes('node_modules/react-virtuoso')) return 'virtuoso';
          if (id.includes('node_modules/zustand')) return 'store';
        },
      },
    },
    chunkSizeWarningLimit: 150,
  },
});
