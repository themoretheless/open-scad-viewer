// Service worker for OpenSCAD 3D Viewer (PWA offline support).
//
// CACHE_VERSION is stamped at build time by scripts/pwa-build.mjs with a hash
// of every emitted file, so each build that changes content gets a new cache
// name and the `activate` handler deletes caches from previous deploys.
const CACHE_VERSION = '__CACHE_VERSION__'
const CACHE_NAME = `openscad-viewer-${CACHE_VERSION}`

// Runtime cache bounds. Entries beyond MAX_CACHE_ENTRIES are evicted oldest
// first (Cache.keys() returns insertion order, and a refreshed entry is
// re-inserted). Responses that declare more than MAX_ENTRY_BYTES are served
// but not cached, so one large download cannot take over the quota.
const MAX_CACHE_ENTRIES = 120
const MAX_ENTRY_BYTES = 16 * 1024 * 1024

// Cross-origin hosts whose responses are cached (the Google Fonts stylesheet
// and font files referenced by index.html). Everything else cross-origin is
// left to the browser.
const CACHEABLE_CROSS_ORIGINS = ['https://fonts.googleapis.com', 'https://fonts.gstatic.com']

// Network timeout (ms) for navigation requests. If the network hangs longer
// than this we fall back to the cached shell instead of leaving the page
// blank/spinning.
const NETWORK_TIMEOUT_MS = 5000

// Static shell URLs to precache. Hashed bundles are populated into the runtime
// cache on first online visit by the asset strategy below.
const SHELL_URLS = ['/', '/index.html', '/manifest.json', '/icons/icon-192.png', '/icons/icon-512.png']

self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(CACHE_NAME).then(async cache => {
      // Cache each URL individually so a single 404 doesn't abort the install,
      // unlike cache.addAll() which rejects atomically.
      await Promise.allSettled(SHELL_URLS.map(url => cache.add(url).catch(() => {})))
    }),
  )
  // No unconditional skipWaiting(): a new worker waits until every tab running
  // the previous build is closed, so a page never gets assets from a different
  // build mid-session. A page may opt in by posting {type: 'SKIP_WAITING'}.
})

self.addEventListener('message', event => {
  if (event.data && event.data.type === 'SKIP_WAITING') self.skipWaiting()
})

self.addEventListener('activate', event => {
  event.waitUntil(
    caches
      .keys()
      .then(keys => Promise.all(keys.filter(k => k !== CACHE_NAME).map(k => caches.delete(k))))
      // Control pages that loaded without a worker (first install).
      .then(() => self.clients.claim()),
  )
})

// Response size when the server declares it; unknown sizes are admitted.
function declaredBytes(response) {
  const length = Number(response.headers.get('content-length'))
  return Number.isFinite(length) && length > 0 ? length : 0
}

function isCacheable(response) {
  if (!response || declaredBytes(response) > MAX_ENTRY_BYTES) return false
  // Opaque responses (no-cors font stylesheet) have status 0 and are allowed
  // only from the listed font hosts, which is the only place they reach here.
  if (response.type === 'opaque') return true
  return response.status === 200 && (response.type === 'basic' || response.type === 'cors')
}

// Deletes the oldest entries until at most `maxEntries` remain.
async function trimCache(cache, maxEntries) {
  const keys = await cache.keys()
  const excess = keys.length - maxEntries
  for (let i = 0; i < excess; i++) await cache.delete(keys[i])
}

// Stores a response as the newest entry, then enforces the entry bound.
async function putBounded(cache, request, response) {
  if (!isCacheable(response)) return
  await cache.delete(request)
  await cache.put(request, response)
  await trimCache(cache, MAX_CACHE_ENTRIES)
}

// Fetch with a timeout so a hanging connection doesn't block the fallback.
function fetchWithTimeout(request, timeoutMs) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Network timeout')), timeoutMs)
    fetch(request).then(
      response => {
        clearTimeout(timer)
        resolve(response)
      },
      err => {
        clearTimeout(timer)
        reject(err)
      },
    )
  })
}

// Network-first strategy for navigations (the HTML document), so deploys take
// effect on the next visit. Falls back to the cached document, then the shell.
async function handleNavigation(request) {
  const cache = await caches.open(CACHE_NAME)
  try {
    const response = await fetchWithTimeout(request, NETWORK_TIMEOUT_MS)
    if (response && response.status === 200) putBounded(cache, request, response.clone()).catch(() => {})
    return response
  } catch {
    const cached = await cache.match(request)
    if (cached) return cached
    const shell = await cache.match('/index.html')
    if (shell) return shell
    return Response.error()
  }
}

// Stale-while-revalidate for static assets: serve the cached copy immediately
// and refresh it in the background.
async function handleAsset(request) {
  const cache = await caches.open(CACHE_NAME)
  const cached = await cache.match(request)
  const fetched = fetch(request)
    .then(response => {
      putBounded(cache, request, response.clone()).catch(() => {})
      return response
    })
    .catch(() => cached || Response.error())
  return cached || fetched
}

self.addEventListener('fetch', event => {
  const request = event.request
  if (request.method !== 'GET') return

  if (request.mode === 'navigate' || request.destination === 'document') {
    event.respondWith(handleNavigation(request))
    return
  }

  const url = new URL(request.url)
  if (url.origin !== self.location.origin && !CACHEABLE_CROSS_ORIGINS.includes(url.origin)) return

  event.respondWith(handleAsset(request))
})
