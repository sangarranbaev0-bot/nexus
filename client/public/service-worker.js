const CACHE_NAME = "nexus-v2";
const PRECACHE_URLS = ["/", "/offline.html"];

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
  ).then(() => self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;
  const url = new URL(event.request.url);
  // API и WS — только сеть
  if (url.pathname.startsWith("/api/") || url.pathname.startsWith("/ws")) {
    event.respondWith(fetch(event.request));
    return;
  }
  event.respondWith(
    caches.match(event.request).then((cached) => {
      const fetchPromise = fetch(event.request).then((networkResponse) => {
        if (networkResponse && networkResponse.status === 200) {
          const responseClone = networkResponse.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, responseClone));
        }
        return networkResponse;
      }).catch(() => {
        // При отсутствии сети возвращаем офлайн-страницу для навигаций
        if (event.request.destination === "document") {
          return caches.match("/offline.html");
        }
      });
      return cached || fetchPromise;
    }),
  );
});

self.addEventListener("push", (event) => {
  const data = event.data ? event.data.json() : {};
  const title = data.title || "NexusChat";
  const body = data.body || "";
  const icon = data.icon || "/icon-192.png";
  event.waitUntil(
    self.registration.showNotification(title, { body, icon, badge: "/icon-192.png" }),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.preventDefault();
  event.notification.close();
  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
      const client = clients.find((c) => !c.closed);
      if (client) return client.focus();
      return self.clients.openWindow("/");
    }),
  );
});

// Periodic background sync — обновлять непрочитанные при возвращении в сеть
self.addEventListener("periodicsync", (event) => {
  if (event.tag === "sync-unread") {
    event.waitUntil(
      fetch("/api/notifications/unread-count").catch(() => {}),
    );
  }
});
