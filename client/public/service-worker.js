self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", () => {
  self.clients.claim();
});

self.addEventListener("push", (event) => {
  const data = event.data ? event.data.json() : {};
  const title = data.title || "NexusChat";
  const body = data.body || "";
  event.waitUntil(
    self.registration.showNotification(title, { body, icon: "/icon-192.png" })
  );
});

self.addEventListener("notificationclick", (event) => {
  event.preventDefault();
  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
      const client = clients[0];
      if (client) return client.focus();
      return self.clients.openWindow("/channels");
    })
  );
});
