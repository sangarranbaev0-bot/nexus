const CACHE_NAME = "nexus-v1";
const PRECACHE_URLS = ["/nexus/"];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(PRECACHE_URLS)),
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((names) =>
      Promise.all(names.filter((n) => n !== CACHE_NAME).map((n) => caches.delete(n))),
    ),
  );
  self.clients.claim();
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;
  const url = new URL(event.request.url);
  // Никогда не кэшируем API-запросы.
  if (url.pathname.startsWith("/api/") || url.pathname.startsWith("/ws")) return;
  event.respondWith(
    caches.match(event.request).then((cached) => {
      const fetchPromise = fetch(event.request).then((networkResponse) => {
        if (networkResponse && networkResponse.status === 200) {
          const responseClone = networkResponse.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, responseClone));
        }
        return networkResponse;
      });
      return cached || fetchPromise;
    }),
  );
});

self.addEventListener("push", (event) => {
  const data = event.data ? event.data.json() : {};
  const title = data.title || "NexusChat";
  const body = data.body || "";
  event.waitUntil(
    self.registration.showNotification(title, { body, icon: "/nexus/icon-192.png" }),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.preventDefault();
  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
      const client = clients[0];
      if (client) return client.focus();
      return self.clients.openWindow("/nexus/channels");
    }),
  );
});
