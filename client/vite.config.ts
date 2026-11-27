import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import gzipPlugin from 'rollup-plugin-gzip';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    gzipPlugin(),
    VitePWA({
      registerType: 'autoUpdate',
      workbox: {
        globPatterns: ['**/*.{js,css,html,svg,png,ico,txt}'],
        runtimeCaching: [
          {
            urlPattern: /^https:\/\/.*\.(jpg|jpeg|png|gif|webp|svg|mp4|webm)$/i,
            handler: 'CacheFirst',
            options: {
              cacheName: 'media-cache',
              expiration: { maxEntries: 200, maxAgeSeconds: 30 * 24 * 60 * 60 },
            },
          },
          {
            urlPattern: ({ request }) => request.destination === 'font',
            handler: 'CacheFirst',
            options: {
              cacheName: 'fonts-cache',
              expiration: { maxEntries: 10, maxAgeSeconds: 365 * 24 * 60 * 60 },
            },
          },
        ],
      },
      manifest: false,
      devOptions: {
        enabled: false,
      },
    }),
  ],
  base: '/',
  // Для GitHub Pages укажите полный адрес WebSocket-сервера, иначе подключается к тому же хосту.
  // Пример: VITE_WS_URL=https://chat.example.com
    server: {
     proxy: {
       '/api': 'http://localhost:3030',
       '/ws': { target: 'ws://localhost:3030', ws: true },
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
