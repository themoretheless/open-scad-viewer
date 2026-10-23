// Service worker for OpenSCAD 3D Viewer (PWA offline support).
//
// CACHE_VERSION: bump on each deploy. The cache name is derived from it so
// that the `activate` handler can delete every cache that doesn't match the
// current version, evicting stale shells/assets from previous deploys.
const CACHE_VERSION = 'v2'
const CACHE_NAME = `openscad-viewer-${CACHE_VERSION}`

// Network timeout (ms) for navigation requests. If the network hangs longer
// than this we fall back to the cached shell instead of leaving the page
// blank/spinning.
const NETWORK_TIMEOUT_MS = 5000

// Static shell URLs to precache. We intentionally precache ONLY the app shell
// here. The hashed JS/CSS bundles (e.g. /assets/index-abc123.js) have
// build-time hash filenames that are unknown when this SW is authored, so they
// can't be listed statically. Instead they are populated into the runtime
// cache on the first online visit by the cache-first asset strategy in the
// fetch handler below, which makes the app offline-capable after first load.
const SHELL_URLS = [
  '/',
  '/index.html',
  '/manifest.json',
]

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then(async (cache) => {
      // Cache each URL individually so a single 404 (e.g. a missing
      // /manifest.json) doesn't abort the whole install, unlike
      // cache.addAll() which rejects atomically.
      await Promise.allSettled(
        SHELL_URLS.map((url) => cache.add(url).catch(() => {}))
      )
    })
  )
  // Tradeoff: skipWaiting() activates the new SW immediately rather than
  // waiting for all tabs to close. This gives faster updates, but a new SW can
  // take control of an already-running page, which could in theory serve
  // assets that don't match the JS the page loaded. For this simple app the
  // faster-update benefit outweighs that risk; if mid-session asset mismatch
  // becomes a problem, gate this behind a postMessage-driven update flow.
  self.skipWaiting()
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      // Delete every cache that isn't the current versioned cache.
      Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k)))
    )
  )
  // Take control of open pages so the activated SW handles their fetches.
  self.clients.claim()
})

// Fetch with a timeout so a hanging connection doesn't block the fallback.
function fetchWithTimeout(request, timeoutMs) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Network timeout')), timeoutMs)
    fetch(request).then(
      (response) => {
        clearTimeout(timer)
        resolve(response)
      },
      (err) => {
        clearTimeout(timer)
        reject(err)
      }
    )
  })
}

// Network-first strategy for navigations (the HTML document). Trying the
// network first ensures deploys take effect on the next visit rather than the
// one after. Falls back to the cached response (and finally the cached shell)
// when offline.
async function handleNavigation(request) {
  const cache = await caches.open(CACHE_NAME)
  try {
    const response = await fetchWithTimeout(request, NETWORK_TIMEOUT_MS)
    if (response && response.status === 200) {
      cache.put(request, response.clone())
    }
    return response
  } catch {
    // Network failed or timed out: try the cached document, then fall back to
    // the SPA shell so the app still loads offline.
    const cached = await cache.match(request)
    if (cached) return cached
    const shell = await cache.match('/index.html')
    if (shell) return shell
    return Response.error()
  }
}

// Cache-first (stale-while-revalidate) for hashed static assets. Serves the
// cached copy immediately and refreshes it in the background.
async function handleAsset(request) {
  const cache = await caches.open(CACHE_NAME)
  const cached = await cache.match(request)
  const fetched = fetch(request)
    .then((response) => {
      if (response && response.status === 200 && response.type === 'basic') {
        cache.put(request, response.clone())
      }
      return response
    })
    .catch(() => cached)
  return cached || fetched
}

self.addEventListener('fetch', (event) => {
  const request = event.request
  if (request.method !== 'GET') return

  // Navigation requests (the HTML document) use network-first.
  if (request.mode === 'navigate' || request.destination === 'document') {
    event.respondWith(handleNavigation(request))
    return
  }

  // Only handle same-origin assets; let the browser deal with cross-origin.
  const url = new URL(request.url)
  if (url.origin !== self.location.origin) return

  event.respondWith(handleAsset(request))
})
