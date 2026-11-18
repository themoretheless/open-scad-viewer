// The offline shell is a production concern. In development it would serve stale
// modules from its cache, so any registration left over from a build is removed.
// Kept as a module (not an inline script) so the Content-Security-Policy needs no 'unsafe-inline'.
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    if (import.meta.env.DEV) {
      navigator.serviceWorker
        .getRegistrations()
        .then(registrations => registrations.forEach(registration => void registration.unregister()))
        .catch(() => {})
      return
    }
    navigator.serviceWorker.register('/sw.js').catch(() => {})
  })
}

export {}
