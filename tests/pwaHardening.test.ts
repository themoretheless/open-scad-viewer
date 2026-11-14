import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import { describe, expect, it } from 'vitest'
import {
  CACHE_VERSION_PLACEHOLDER,
  CONTENT_SECURITY_POLICY,
  deriveCacheVersion,
  findInlineScripts,
  stampServiceWorker,
} from '../scripts/pwa-build.mjs'

const root = new URL('../', import.meta.url)
const swSource = readFileSync(new URL('public/sw.js', root), 'utf8')

/** Insertion-ordered fake of the Cache API, keyed by URL. */
class FakeCache {
  entries = new Map<string, unknown>()
  async keys() {
    return [...this.entries.keys()]
  }
  async delete(key: string) {
    return this.entries.delete(key)
  }
  async put(key: string, value: unknown) {
    this.entries.set(key, value)
  }
}

type FakeResponse = { status: number; type: string; headers: { get(name: string): string | null } }
const response = (bytes?: number, type = 'basic', status = 200): FakeResponse => ({
  status,
  type,
  headers: { get: name => (name === 'content-length' && bytes !== undefined ? String(bytes) : null) },
})

interface ServiceWorkerScope {
  trimCache(cache: FakeCache, max: number): Promise<void>
  putBounded(cache: FakeCache, request: string, response: FakeResponse): Promise<void>
  isCacheable(response: FakeResponse): boolean
  MAX_CACHE_ENTRIES: number
  MAX_ENTRY_BYTES: number
  CACHE_NAME: string
}

function loadServiceWorker(source = swSource): ServiceWorkerScope {
  const context = createContext({ self: { addEventListener() {} } })
  runInContext(source, context)
  return runInContext(
    '({trimCache, putBounded, isCacheable, MAX_CACHE_ENTRIES, MAX_ENTRY_BYTES, CACHE_NAME})',
    context,
  ) as ServiceWorkerScope
}

describe('service worker cache bound', () => {
  it('evicts the oldest entries beyond the limit', async () => {
    const sw = loadServiceWorker()
    const cache = new FakeCache()
    for (let i = 0; i < 5; i++) await cache.put(`/a${i}`, i)
    await sw.trimCache(cache, 3)
    expect(await cache.keys()).toEqual(['/a2', '/a3', '/a4'])
  })

  it('keeps at most MAX_CACHE_ENTRIES and treats a refreshed entry as newest', async () => {
    const sw = loadServiceWorker()
    const cache = new FakeCache()
    for (let i = 0; i < sw.MAX_CACHE_ENTRIES; i++) await sw.putBounded(cache, `/a${i}`, response(10))
    await sw.putBounded(cache, '/a0', response(10))
    await sw.putBounded(cache, '/new', response(10))
    const keys = await cache.keys()
    expect(keys).toHaveLength(sw.MAX_CACHE_ENTRIES)
    expect(keys).not.toContain('/a1')
    expect(keys.slice(-2)).toEqual(['/a0', '/new'])
  })

  it('does not cache oversized, failed or foreign responses', () => {
    const sw = loadServiceWorker()
    expect(sw.isCacheable(response(sw.MAX_ENTRY_BYTES + 1))).toBe(false)
    expect(sw.isCacheable(response(10, 'basic', 404))).toBe(false)
    expect(sw.isCacheable(response(10, 'error', 200))).toBe(false)
    expect(sw.isCacheable(response(undefined))).toBe(true)
    expect(sw.isCacheable(response(10, 'cors'))).toBe(true)
  })
})

describe('CACHE_VERSION derivation', () => {
  const build = (): [string, Buffer][] => [
    ['index.html', Buffer.from('<html>')],
    ['assets/index-abc.js', Buffer.from('console.log(1)')],
  ]

  it('is deterministic and independent of file order', () => {
    expect(deriveCacheVersion(build())).toBe(deriveCacheVersion(build().reverse()))
    expect(deriveCacheVersion(build())).toMatch(/^[0-9a-f]{16}$/)
  })

  it('changes when content, names or the file set change, but not for sw.js itself', () => {
    const base = deriveCacheVersion(build())
    const edited = build()
    edited[1] = ['assets/index-abc.js', Buffer.from('console.log(2)')]
    expect(deriveCacheVersion(edited)).not.toBe(base)
    expect(deriveCacheVersion([...build(), ['icons/x.png', Buffer.from('x')]])).not.toBe(base)
    expect(deriveCacheVersion([build()[0], ['assets/index-def.js', build()[1][1]]])).not.toBe(base)
    expect(deriveCacheVersion([...build(), ['sw.js', Buffer.from('anything')]])).toBe(base)
  })

  it('stamps the service worker source exactly once', () => {
    expect(swSource.split(CACHE_VERSION_PLACEHOLDER)).toHaveLength(2)
    const stamped = stampServiceWorker(swSource, '0123456789abcdef')
    expect(loadServiceWorker(stamped).CACHE_NAME).toBe('openscad-viewer-0123456789abcdef')
    expect(() => stampServiceWorker(stamped, 'x')).toThrow()
  })
})

describe('shell security and manifest', () => {
  it('index.html has no inline script and the CSP allows no inline script', () => {
    const html = readFileSync(new URL('index.html', root), 'utf8')
    expect(findInlineScripts(html)).toEqual([])
    expect(findInlineScripts('<script>alert(1)</script><script type="module" src="/a.js"></script>')).toHaveLength(1)
    const scriptSrc = CONTENT_SECURITY_POLICY.split('; ').find(d => d.startsWith('script-src'))
    expect(scriptSrc).toBe("script-src 'self' 'wasm-unsafe-eval'")
  })

  it('manifest lists existing PNG icons with matching sizes, including a maskable one', () => {
    const manifest = JSON.parse(readFileSync(new URL('public/manifest.json', root), 'utf8')) as {
      icons: { src: string; sizes: string; type: string; purpose: string }[]
    }
    for (const icon of manifest.icons) {
      const png = readFileSync(new URL(`public${icon.src}`, root))
      expect(icon.type).toBe('image/png')
      expect(`${png.readUInt32BE(16)}x${png.readUInt32BE(20)}`).toBe(icon.sizes)
    }
    expect(manifest.icons.map(icon => icon.sizes)).toEqual(expect.arrayContaining(['192x192', '512x512']))
    expect(manifest.icons.some(icon => icon.purpose === 'maskable')).toBe(true)
  })
})
