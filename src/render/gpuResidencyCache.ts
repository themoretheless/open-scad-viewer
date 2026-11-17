/**
 * One byte budget for GPU buffers that outlive a scene publication: LOD level
 * buffers from `ProgressiveLevelStore` and base geometry buffers from
 * `GpuGeometryCache` share a single least-recently-used cache. Each client
 * owns a key namespace and pins the keys it is currently drawing; unpinned
 * entries are retained until the budget forces their eviction.
 */

interface CacheEntry<V> {
  value: V
  bytes: number
}

/** Least-recently-used map bounded by total bytes; pinned entries are never evicted. */
export class ByteLruCache<V> {
  private readonly entries = new Map<string, CacheEntry<V>>()
  private total = 0

  constructor(
    private budget: number,
    private readonly onEvict: (key: string, value: V) => void = () => {},
    private readonly pinned: (key: string) => boolean = () => false,
  ) {}

  get bytes() {
    return this.total
  }

  get budgetBytes() {
    return this.budget
  }

  /** Change the byte budget; shrinking evicts unpinned entries at once. */
  setBudget(bytes: number) {
    this.budget = bytes
    this.trim()
  }

  get size() {
    return this.entries.size
  }

  keys(): string[] {
    return [...this.entries.keys()]
  }

  has(key: string) {
    return this.entries.has(key)
  }

  get(key: string): V | undefined {
    const entry = this.entries.get(key)
    if (!entry) return undefined
    this.entries.delete(key)
    this.entries.set(key, entry)
    return entry.value
  }

  set(key: string, value: V, bytes: number) {
    this.delete(key)
    this.entries.set(key, { value, bytes })
    this.total += bytes
    this.evict(key)
  }

  delete(key: string): boolean {
    const entry = this.entries.get(key)
    if (!entry) return false
    this.entries.delete(key)
    this.total -= entry.bytes
    this.onEvict(key, entry.value)
    return true
  }

  /** Total bytes of the entries whose key matches. */
  bytesWhere(match: (key: string) => boolean): number {
    let total = 0
    for (const [key, entry] of this.entries) if (match(key)) total += entry.bytes
    return total
  }

  /** Re-run eviction, e.g. after entries were unpinned. */
  trim() {
    this.evict('')
  }

  /** Drop every entry whose key matches; returns how many were removed. */
  invalidate(match: (key: string) => boolean): number {
    let removed = 0
    for (const key of [...this.entries.keys()]) if (match(key) && this.delete(key)) removed++
    return removed
  }

  /** Evict oldest unpinned entries until the budget holds; `keep` is the entry just inserted. */
  private evict(keep: string) {
    for (const key of [...this.entries.keys()]) {
      if (this.total <= this.budget) return
      if (key !== keep && !this.pinned(key)) this.delete(key)
    }
  }
}

/** A cached GPU allocation; eviction calls `destroy`. */
export interface ResidentResource {
  destroy(): void
}

export class GpuResidencyCache {
  private readonly pins = new Set<(key: string) => boolean>()
  readonly lru: ByteLruCache<ResidentResource>

  constructor(budgetBytes: number) {
    this.lru = new ByteLruCache<ResidentResource>(
      budgetBytes,
      (_key, value) => value.destroy(),
      key => {
        for (const pinned of this.pins) if (pinned(key)) return true
        return false
      },
    )
  }

  /** Register a pin predicate; the returned function removes it. */
  pin(predicate: (key: string) => boolean): () => void {
    this.pins.add(predicate)
    return () => {
      this.pins.delete(predicate)
    }
  }

  get bytes() {
    return this.lru.bytes
  }

  get budgetBytes() {
    return this.lru.budgetBytes
  }

  /** Follow the display plan budget; shrinking evicts unpinned buffers at once. */
  setBudget(bytes: number) {
    this.lru.setBudget(bytes)
  }

  /** Destroy every entry regardless of pins. */
  dispose() {
    this.lru.invalidate(() => true)
  }
}
