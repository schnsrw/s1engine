// s1 Editor Service Worker — offline caching with stale-while-revalidate strategy
const CACHE_NAME = 's1-editor-v1';
const ASSETS_TO_CACHE = [
  '/',
  '/index.html',
];

// Install: pre-cache the app shell
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(ASSETS_TO_CACHE);
    })
  );
  self.skipWaiting();
});

// Activate: clean up old caches
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((k) => k !== CACHE_NAME)
          .map((k) => caches.delete(k))
      )
    )
  );
  self.clients.claim();
});

// Fetch: stale-while-revalidate for static assets, skip non-GET and API routes
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  // Skip non-GET requests
  if (event.request.method !== 'GET') return;

  // Skip health/metrics/API endpoints
  if (url.pathname.startsWith('/health') || url.pathname.startsWith('/metrics')) return;

  // Skip WebSocket upgrade requests
  if (event.request.headers.get('upgrade') === 'websocket') return;

  // Skip room/admin API endpoints
  if (url.pathname.startsWith('/rooms') || url.pathname.startsWith('/admin')) return;

  event.respondWith(
    caches.match(event.request).then((cached) => {
      const fetchPromise = fetch(event.request)
        .then((response) => {
          // Only cache successful responses
          if (response && response.ok) {
            const clone = response.clone();
            caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
          }
          return response;
        })
        .catch(() => {
          // Return cached version if available, otherwise a proper error response
          if (cached) return cached;
          return new Response('Offline', { status: 503, statusText: 'Service Unavailable' });
        });

      // Return cached version immediately, update in background
      return cached || fetchPromise;
    })
  );
});
