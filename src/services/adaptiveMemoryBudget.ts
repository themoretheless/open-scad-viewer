/** Approximate browser hints; these do not measure free system RAM or GPU/WASM use. */
export interface MemoryHints { deviceGiB?: number; heapLimit?: number; heapUsed?: number }
export function readMemoryHints(): MemoryHints {
  const nav = typeof navigator === 'undefined' ? undefined : navigator as Navigator & { deviceMemory?: number }
  const heap = typeof performance === 'undefined' ? undefined : (performance as Performance & {
    memory?: { jsHeapSizeLimit: number; usedJSHeapSize: number }
  }).memory
  return { deviceGiB: nav?.deviceMemory, heapLimit: heap?.jsHeapSizeLimit, heapUsed: heap?.usedJSHeapSize }
}
export function adaptiveCacheBytes(base: number, hints = readMemoryHints()): number {
  let scale = 1
  const { deviceGiB, heapLimit, heapUsed } = hints
  if (deviceGiB !== undefined && Number.isFinite(deviceGiB) && deviceGiB > 0)
    scale = Math.max(.25, Math.min(4, deviceGiB / 2))
  let budget = base * scale
  if (heapLimit !== undefined && heapUsed !== undefined && Number.isFinite(heapLimit)
      && Number.isFinite(heapUsed) && heapLimit > 0 && heapUsed >= 0 && heapUsed <= heapLimit) {
    // All caches using this policy share the same 72 MB baseline proportionally.
    // Reserve 25% of the heap and use only a quarter of remaining headroom.
    const headroom = Math.max(0, heapLimit * .75 - heapUsed)
    budget = Math.min(budget, headroom / 4 * base / 72_000_000)
  }
  return Math.max(1, Math.floor(budget))
}
