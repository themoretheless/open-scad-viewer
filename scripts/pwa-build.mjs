// Build-time PWA hardening: a Content-Security-Policy for the built shell and a
// service-worker CACHE_VERSION derived from the emitted files.
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { join, relative } from 'node:path'

export const CACHE_VERSION_PLACEHOLDER = '__CACHE_VERSION__'
export const SERVICE_WORKER_FILE = 'sw.js'

// Scripts only from the origin; WASM compilation needs 'wasm-unsafe-eval'. Styles
// keep 'unsafe-inline' because components set inline style attributes. Workers
// are same-origin module workers; downloads use blob: URLs.
export const CONTENT_SECURITY_POLICY = [
  "default-src 'self'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "worker-src 'self' blob:",
  "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
  "font-src 'self' data: https://fonts.gstatic.com",
  "img-src 'self' data: blob:",
  "connect-src 'self' data: blob: https://fonts.googleapis.com https://fonts.gstatic.com",
  "manifest-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
].join('; ')

/**
 * Content hash of a build: sorted `[path, bytes]` pairs, so any changed, added,
 * removed or renamed file yields a new version. The service worker itself is excluded.
 */
export function deriveCacheVersion(entries) {
  const hash = createHash('sha256')
  for (const [path, bytes] of [...entries].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
    if (path === SERVICE_WORKER_FILE) continue
    hash.update(path).update('\0').update(createHash('sha256').update(bytes).digest()).update('\0')
  }
  return hash.digest('hex').slice(0, 16)
}

export function readBuildEntries(root) {
  const entries = []
  const walk = directory => {
    for (const name of readdirSync(directory)) {
      const path = join(directory, name)
      if (statSync(path).isDirectory()) walk(path)
      else entries.push([relative(root, path).replaceAll('\\', '/'), readFileSync(path)])
    }
  }
  walk(root)
  return entries
}

/** Replaces the single placeholder in the service worker source; refuses anything else. */
export function stampServiceWorker(source, version) {
  const occurrences = source.split(CACHE_VERSION_PLACEHOLDER).length - 1
  if (occurrences !== 1)
    throw new Error(`${SERVICE_WORKER_FILE} must contain ${CACHE_VERSION_PLACEHOLDER} exactly once`)
  return source.replace(CACHE_VERSION_PLACEHOLDER, version)
}

/** Inline (non-src) <script> elements in an HTML document. */
export function findInlineScripts(html) {
  return [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)]
    .filter(([, attributes, body]) => !/\bsrc\s*=/i.test(attributes) || body.trim() !== '')
    .map(([element]) => element)
}

export function pwaBuildPlugin() {
  let outDir = 'dist'
  return {
    name: 'pwa-build',
    apply: 'build',
    configResolved(config) {
      outDir = join(config.root, config.build.outDir)
    },
    transformIndexHtml: {
      order: 'post',
      handler: () => [
        {
          tag: 'meta',
          attrs: { 'http-equiv': 'Content-Security-Policy', content: CONTENT_SECURITY_POLICY },
          injectTo: 'head-prepend',
        },
      ],
    },
    closeBundle() {
      const swPath = join(outDir, SERVICE_WORKER_FILE)
      const version = deriveCacheVersion(readBuildEntries(outDir))
      writeFileSync(swPath, stampServiceWorker(readFileSync(swPath, 'utf8'), version))
    },
  }
}
