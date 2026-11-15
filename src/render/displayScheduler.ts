/**
 * Adaptive display planning for LOD meshes.
 *
 * - `planDisplayCaps` runs `selectDisplayRepresentation` per entity, largest
 *   projected size first, so near entities claim the byte budget before far
 *   ones. The chosen representation caps how fine a level the native
 *   `meshLod` selector may bind; it never picks a level on its own.
 * - `ByteLruCache` holds uploaded level buffers keyed by geometry asset id and
 *   level, evicting least recently used entries once the byte budget is hit.
 * - `ProgressiveLevelStore` uploads a requested level off the frame (idle
 *   callback), keeps showing the best coarser level that is ready and asks for
 *   a redraw once the finer level lands.
 */
import type { MeshLodLevel } from '../core/mesh'
import { selectDisplayRepresentation, type DisplayRepresentation } from '../kernel/geometry/displayPolicy'
import type { GMesh } from './types'
import { pixelsPerUnit, type ViewerLodCamera } from './viewerLod'

export interface DisplayBudget {
  /** Byte budget for the levels admitted in one plan. */
  readonly budgetBytes: number
  /** Entities projecting to at most this many pixels stay on the coarsest level. */
  readonly targetErrorPixels: number
}

export const DEFAULT_DISPLAY_BUDGET: DisplayBudget = { budgetBytes: 256 * 1024 * 1024, targetErrorPixels: 24 }

export interface DisplayPlanEntry {
  readonly representation: DisplayRepresentation
  /** Finest level index (levels are coarse to fine) the mesh may bind. */
  readonly maxLevel: number
  readonly projectedPixels: number
}

export const levelBytes = (level: MeshLodLevel) => level.vertices.byteLength + level.indices.byteLength

/** Maps a representation onto the coarse-to-fine level list of one mesh. */
export function representationCap(representation: DisplayRepresentation, levelCount: number): number {
  if (representation === 'bounding-box') return 0
  if (representation === 'preview') return Math.max(0, levelCount - 2)
  return levelCount - 1
}

/**
 * Pick a representation for every visible LOD mesh. No GPU bounding-box draw
 * path exists, so 'bounding-box' degrades to the coarsest certified level.
 */
export function planDisplayCaps(
  meshes: readonly GMesh[],
  camera: ViewerLodCamera,
  budget: DisplayBudget = DEFAULT_DISPLAY_BUDGET,
): Map<GMesh, DisplayPlanEntry> {
  const candidates: { mesh: GMesh; levels: readonly MeshLodLevel[]; projectedPixels: number }[] = []
  for (const mesh of meshes) {
    if (!mesh.lod || !mesh.visible) continue
    const { center, radius } = mesh.worldBounds
    candidates.push({
      mesh,
      levels: mesh.lod.levels,
      projectedPixels: 2 * radius * pixelsPerUnit(camera, center, radius),
    })
  }
  candidates.sort((a, b) => b.projectedPixels - a.projectedPixels)
  const plan = new Map<GMesh, DisplayPlanEntry>()
  let admittedBytes = 0
  for (const { mesh, levels, projectedPixels } of candidates) {
    const representation = selectDisplayRepresentation({
      projectedPixels,
      targetErrorPixels: budget.targetErrorPixels,
      admittedBytes,
      budgetBytes: budget.budgetBytes,
      exactAvailable: levels.length > 1,
      previewBytes: levelBytes(levels[representationCap('preview', levels.length)]),
      exactBytes: levelBytes(levels[levels.length - 1]),
    })
    const maxLevel = representationCap(representation, levels.length)
    admittedBytes += levelBytes(levels[maxLevel])
    plan.set(mesh, { representation, maxLevel, projectedPixels })
  }
  return plan
}

interface CacheEntry<V> {
  value: V
  bytes: number
}

/** Least-recently-used map bounded by total bytes; pinned entries are never evicted. */
export class ByteLruCache<V> {
  private readonly entries = new Map<string, CacheEntry<V>>()
  private total = 0

  constructor(
    readonly budgetBytes: number,
    private readonly onEvict: (key: string, value: V) => void = () => {},
    private readonly pinned: (key: string) => boolean = () => false,
  ) {}

  get bytes() {
    return this.total
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

  /** Drop every entry whose key matches; returns how many were removed. */
  invalidate(match: (key: string) => boolean): number {
    let removed = 0
    for (const key of [...this.entries.keys()]) if (match(key) && this.delete(key)) removed++
    return removed
  }

  /** Evict oldest unpinned entries until the budget holds; `keep` is the entry just inserted. */
  private evict(keep: string) {
    for (const key of [...this.entries.keys()]) {
      if (this.total <= this.budgetBytes) return
      if (key !== keep && !this.pinned(key)) this.delete(key)
    }
  }
}

export interface LevelBuffers {
  readonly vb: GPUBuffer
  readonly ib: GPUBuffer
}

interface CachedLevel extends LevelBuffers {
  /** Level data the buffers were uploaded from; a different object means the geometry changed. */
  readonly source: MeshLodLevel
}

export type IdleScheduler = (task: () => void) => void

export const defaultIdleScheduler: IdleScheduler = task => {
  const idle = (globalThis as { requestIdleCallback?: (cb: () => void, opts?: { timeout: number }) => number })
    .requestIdleCallback
  if (idle) idle(task, { timeout: 100 })
  else setTimeout(task, 0)
}

export type LevelState = 'missing' | 'pending' | 'ready'

export interface ResolvedLevel {
  readonly level: number
  readonly buffers: LevelBuffers
}

const meshKeys = new WeakMap<GMesh, string>()
let nextMeshKey = 0

/** Cache identity of a mesh's geometry: the shared asset id, or a per-mesh key for unshared geometry. */
export function geometryKey(mesh: GMesh): string {
  if (mesh.assetId) return mesh.assetId
  let key = meshKeys.get(mesh)
  if (!key) meshKeys.set(mesh, (key = `mesh:${nextMeshKey++}`))
  return key
}

const cacheKey = (geometry: string, level: number) => `${geometry}#${level}`
const geometryOf = (key: string) => key.slice(0, key.lastIndexOf('#'))

/**
 * Uploads LOD level buffers on idle callbacks, one level per callback, and
 * shares them between instances of the same asset through a byte-bounded LRU.
 */
export class ProgressiveLevelStore {
  private readonly cache: ByteLruCache<CachedLevel>
  private readonly pending = new Map<string, { generation: number; source: MeshLodLevel }>()
  private readonly queue: (() => void)[] = []
  private bound = new Set<string>()
  private generation = 0
  private draining = false

  constructor(
    budgetBytes: number,
    private readonly onReady: () => void = () => {},
    private readonly schedule: IdleScheduler = defaultIdleScheduler,
  ) {
    this.cache = new ByteLruCache<CachedLevel>(
      budgetBytes,
      (_key, value) => {
        value.vb.destroy()
        value.ib.destroy()
      },
      key => this.bound.has(key),
    )
  }

  get cachedBytes() {
    return this.cache.bytes
  }

  state(mesh: GMesh, level: number): LevelState {
    const key = cacheKey(geometryKey(mesh), level)
    if (this.pending.has(key)) return 'pending'
    const cached = this.cache.has(key) ? this.cache.get(key) : undefined
    return cached && cached.source === mesh.lod?.levels[level] ? 'ready' : 'missing'
  }

  /**
   * Level buffers to bind for `target`, or null to keep the current binding.
   * A missing target is requested asynchronously; meanwhile the finest ready
   * level not finer than the target is offered.
   */
  resolve(dev: GPUDevice, mesh: GMesh, target: number): ResolvedLevel | null {
    const lod = mesh.lod
    if (!lod || !lod.levels[target]) return null
    const ready = this.ready(mesh, target)
    if (ready) return { level: target, buffers: ready }
    this.request(dev, mesh, target)
    for (let level = target - 1; level >= 0; level--) {
      const buffers = this.ready(mesh, level)
      if (!buffers) continue
      const current = lod.current
      // A coarser fallback is useful when nothing finer is bound or the bound level is too fine.
      if (level !== current && (current === null || current > target || level > current)) return { level, buffers }
      return null
    }
    return null
  }

  /** Keys bound by the given meshes are pinned against eviction. */
  setBound(meshes: readonly GMesh[]) {
    const bound = new Set<string>()
    for (const mesh of meshes) {
      if (mesh.lod && mesh.lod.current !== null) bound.add(cacheKey(geometryKey(mesh), mesh.lod.current))
    }
    this.bound = bound
  }

  /** Drop cached and pending levels of geometry no longer present in `meshes`. */
  retain(meshes: readonly GMesh[]) {
    const live = new Set(meshes.filter(mesh => mesh.lod).map(geometryKey))
    this.invalidateWhere(geometry => !live.has(geometry))
  }

  /** Drop every cached and pending level of one geometry, e.g. after its asset changed. */
  invalidate(geometry: string) {
    this.invalidateWhere(candidate => candidate === geometry)
  }

  dispose() {
    this.bound = new Set()
    this.invalidateWhere(() => true)
  }

  private invalidateWhere(match: (geometry: string) => boolean) {
    this.cache.invalidate(key => match(geometryOf(key)))
    for (const key of [...this.pending.keys()]) if (match(geometryOf(key))) this.pending.delete(key)
  }

  private ready(mesh: GMesh, level: number): CachedLevel | undefined {
    const geometry = geometryKey(mesh)
    const key = cacheKey(geometry, level)
    const cached = this.cache.get(key)
    if (!cached) return undefined
    if (cached.source === mesh.lod?.levels[level]) return cached
    // Same asset id with different level data: the geometry changed underneath.
    if (!this.bound.has(key)) this.invalidate(geometry)
    return undefined
  }

  private request(dev: GPUDevice, mesh: GMesh, level: number) {
    const key = cacheKey(geometryKey(mesh), level)
    const source = mesh.lod!.levels[level]
    const queued = this.pending.get(key)
    if (queued && queued.source === source) return
    const ticket = { generation: ++this.generation, source }
    this.pending.set(key, ticket)
    this.queue.push(() => {
      if (this.pending.get(key) !== ticket) return
      this.pending.delete(key)
      const vb = dev.createBuffer({
        size: source.vertices.byteLength,
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
      })
      const ib = dev.createBuffer({
        size: source.indices.byteLength,
        usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
      })
      dev.queue.writeBuffer(vb, 0, source.vertices)
      dev.queue.writeBuffer(ib, 0, source.indices)
      this.cache.set(key, { vb, ib, source }, levelBytes(source))
      this.onReady()
    })
    this.drain()
  }

  /** One upload per idle callback keeps each slice of work small. */
  private drain() {
    if (this.draining || !this.queue.length) return
    this.draining = true
    this.schedule(() => {
      this.draining = false
      const task = this.queue.shift()
      try {
        task?.()
      } finally {
        this.drain()
      }
    })
  }
}
