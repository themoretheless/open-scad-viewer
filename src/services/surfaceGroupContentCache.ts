/** FIFO reuse across publications, bounded independently of live scene ownership. */
export class SurfaceGroupContentCache {
  private readonly entries = new Map<string, { ids: Uint32Array; bytes: number }>()
  private bytes = 0
  private readonly maxEntries: number
  private readonly maxBytes: number

  constructor(limits = { maxEntries: 128, maxBytes: 8 * 1024 * 1024 }) {
    for (const value of [limits.maxEntries, limits.maxBytes]) {
      if (!Number.isSafeInteger(value) || value < 1) throw new RangeError('Invalid surface cache limit')
    }
    this.maxEntries = limits.maxEntries
    this.maxBytes = limits.maxBytes
  }

  get size() { return this.entries.size }
  /** Backing-buffer bytes plus UTF-16 key storage; not a JavaScript heap/RSS bound. */
  get retainedBytes() { return this.bytes }

  getOrCompute(key: string, compute: () => Uint32Array): Uint32Array {
    const existing = this.entries.get(key)
    if (existing) return existing.ids
    const ids = compute()
    const bytes = ids.buffer.byteLength + key.length * 2
    // A failed computation or oversized value must preserve useful cached entries.
    if (bytes > this.maxBytes) return ids
    while (this.entries.size >= this.maxEntries || this.bytes + bytes > this.maxBytes) {
      const oldest = this.entries.keys().next().value!
      this.bytes -= this.entries.get(oldest)!.bytes
      this.entries.delete(oldest)
    }
    this.entries.set(key, { ids, bytes })
    this.bytes += bytes
    return ids
  }
}
