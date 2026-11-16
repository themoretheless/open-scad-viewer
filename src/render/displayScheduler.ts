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
 *   a redraw once the finer level lands. Deferred placeholder levels are first
 *   produced by a `DeferredLevelLoader` (the display level worker).
 */
import type { MeshLodLevel } from '../core/mesh'
import { selectDisplayRepresentation, type DisplayRepresentation } from '../kernel/geometry/displayPolicy'
import type { GMesh } from './types'
import { isDeferredLevel, levelByteSize } from './deferredLevels'
import { ByteLruCache, GpuResidencyCache, type ResidentResource } from './gpuResidencyCache'
import { pixelsPerUnit, type ViewerLodCamera } from './viewerLod'

export interface DisplayBudget {
  /** Byte budget for the levels admitted in one plan. */
  readonly budgetBytes: number
  /** Entities projecting to at most this many pixels stay on the coarsest level. */
  readonly targetErrorPixels: number
  /**
   * Draw a 'bounding-box' entity as its world box outline instead of its
   * coarsest level. Off keeps the coarsest certified level on screen.
   */
  readonly boundingBoxProxies?: boolean
}

export const DEFAULT_DISPLAY_BUDGET: DisplayBudget = {
  budgetBytes: 256 * 1024 * 1024,
  targetErrorPixels: 24,
  boundingBoxProxies: false,
}

const MEGABYTE = 1024 * 1024
/** Selectable display budgets in megabytes; the UI offers exactly these. */
export const DISPLAY_BUDGET_MEGABYTES = [64, 128, 256, 512, 1024] as const

/** Budget for a requested size in megabytes, snapped to the nearest offered step. */
export function displayBudgetFor(megabytes: number, boundingBoxProxies = false): DisplayBudget {
  const requested = Number.isFinite(megabytes) ? megabytes : DEFAULT_DISPLAY_BUDGET.budgetBytes / MEGABYTE
  let best: number = DISPLAY_BUDGET_MEGABYTES[0]
  for (const step of DISPLAY_BUDGET_MEGABYTES) {
    if (Math.abs(step - requested) < Math.abs(best - requested)) best = step
  }
  return {
    budgetBytes: best * MEGABYTE,
    targetErrorPixels: DEFAULT_DISPLAY_BUDGET.targetErrorPixels,
    boundingBoxProxies,
  }
}

export const sameDisplayBudget = (a: DisplayBudget, b: DisplayBudget) =>
  a.budgetBytes === b.budgetBytes &&
  a.targetErrorPixels === b.targetErrorPixels &&
  !!a.boundingBoxProxies === !!b.boundingBoxProxies

/** The twelve edges of an axis-aligned box as corner index pairs (bit 0 = x, 1 = y, 2 = z). */
const BOX_EDGES: readonly (readonly [number, number])[] = [
  [0, 1],
  [2, 3],
  [4, 5],
  [6, 7],
  [0, 2],
  [1, 3],
  [4, 6],
  [5, 7],
  [0, 4],
  [1, 5],
  [2, 6],
  [3, 7],
]

/**
 * Line-list vertices (xyz + rgba, the selection overlay layout) outlining the
 * world bounds of every mesh shown as a bounding-box proxy.
 */
export function boundingBoxLineVertices(meshes: readonly GMesh[], color: readonly number[]): number[] {
  const values: number[] = []
  for (const mesh of meshes) {
    if (!mesh.visible || !mesh.lod?.boxed) continue
    const { min, max } = mesh.worldBounds
    for (const edge of BOX_EDGES) {
      for (const corner of edge) {
        values.push(corner & 1 ? max[0] : min[0], corner & 2 ? max[1] : min[1], corner & 4 ? max[2] : min[2], ...color)
      }
    }
  }
  return values
}

export type DisplayQualityPreset = 'low' | 'medium' | 'high'
export const DISPLAY_QUALITY_PRESETS: readonly DisplayQualityPreset[] = ['low', 'medium', 'high']

/** Budget presets offered by the display-quality control; medium is the default budget. */
export const DISPLAY_BUDGET_PRESETS: Readonly<Record<DisplayQualityPreset, DisplayBudget>> = {
  low: { budgetBytes: 64 * 1024 * 1024, targetErrorPixels: 48 },
  medium: DEFAULT_DISPLAY_BUDGET,
  high: { budgetBytes: 1024 * 1024 * 1024, targetErrorPixels: 8 },
}

export const isDisplayQualityPreset = (value: unknown): value is DisplayQualityPreset =>
  value === 'low' || value === 'medium' || value === 'high'

export interface DisplayPlanEntry {
  readonly representation: DisplayRepresentation
  /** Finest level index (levels are coarse to fine) the mesh may bind. */
  readonly maxLevel: number
  readonly projectedPixels: number
}

/** Upload size of a level; deferred placeholders report the size their summary promises. */
export const levelBytes = (level: MeshLodLevel) => levelByteSize(level)

/** Maps a representation onto the coarse-to-fine level list of one mesh. */
export function representationCap(representation: DisplayRepresentation, levelCount: number): number {
  if (representation === 'bounding-box') return 0
  if (representation === 'preview') return Math.max(0, levelCount - 2)
  return levelCount - 1
}

/**
 * Pick a representation for every visible LOD mesh. 'bounding-box' caps the
 * mesh at its coarsest certified level; with `boundingBoxProxies` the renderer
 * draws the box outline in place of that level.
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

export { ByteLruCache }

export interface LevelBuffers {
  readonly vb: GPUBuffer
  readonly ib: GPUBuffer
}

interface CachedLevel extends LevelBuffers, ResidentResource {
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

/** Produces the arrays of deferred placeholder levels (see `onDemandLevels`). */
export interface DeferredLevelLoader {
  load(mesh: GMesh, level: number): Promise<void>
  /** Abort a different level still being produced for this mesh's geometry. */
  cancelStale(mesh: GMesh, keep: number): void
}

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

const LEVEL_PREFIX = 'lod:'
const cacheKey = (geometry: string, level: number) => `${LEVEL_PREFIX}${geometry}#${level}`
/** Geometry of a level key, or null for keys owned by another residency client. */
const geometryOf = (key: string) =>
  key.startsWith(LEVEL_PREFIX) ? key.slice(LEVEL_PREFIX.length, key.lastIndexOf('#')) : null

/**
 * Uploads LOD level buffers on idle callbacks, one level per callback, and
 * shares them between instances of the same asset through a byte-bounded LRU.
 */
export class ProgressiveLevelStore {
  private readonly residency: GpuResidencyCache
  private readonly cache: ByteLruCache<ResidentResource>
  private readonly pending = new Map<string, { generation: number; source: MeshLodLevel }>()
  private readonly queue: (() => void)[] = []
  private bound = new Set<string>()
  private generation = 0
  private draining = false

  /** A number creates a private residency budget; a shared cache puts levels under its one budget. */
  constructor(
    budget: number | GpuResidencyCache,
    private readonly onReady: () => void = () => {},
    private readonly schedule: IdleScheduler = defaultIdleScheduler,
    private readonly deferred: DeferredLevelLoader | null = null,
  ) {
    this.residency = typeof budget === 'number' ? new GpuResidencyCache(budget) : budget
    this.cache = this.residency.lru
    this.residency.pin(key => this.bound.has(key))
  }

  get cachedBytes() {
    return this.cache.bytesWhere(key => geometryOf(key) !== null)
  }

  state(mesh: GMesh, level: number): LevelState {
    const key = cacheKey(geometryKey(mesh), level)
    if (this.pending.has(key)) return 'pending'
    const cached = this.cache.has(key) ? (this.cache.get(key) as CachedLevel | undefined) : undefined
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
    this.deferred?.cancelStale(mesh, target)
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
    const own = (key: string) => {
      const geometry = geometryOf(key)
      return geometry !== null && match(geometry)
    }
    this.cache.invalidate(own)
    for (const key of [...this.pending.keys()]) if (own(key)) this.pending.delete(key)
  }

  private ready(mesh: GMesh, level: number): CachedLevel | undefined {
    const geometry = geometryKey(mesh)
    const key = cacheKey(geometry, level)
    const cached = this.cache.get(key) as CachedLevel | undefined
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
    if (isDeferredLevel(source)) {
      if (!this.deferred) {
        this.pending.delete(key)
        return
      }
      // The worker fills the shared level list in place; upload the produced level on success.
      this.deferred.load(mesh, level).then(
        () => {
          if (this.pending.get(key) !== ticket) return
          this.pending.delete(key)
          const produced = mesh.lod?.levels[level]
          if (produced && !isDeferredLevel(produced)) this.request(dev, mesh, level)
        },
        () => {
          if (this.pending.get(key) === ticket) this.pending.delete(key)
        },
      )
      return
    }
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
      const level: CachedLevel = {
        vb,
        ib,
        source,
        destroy: () => {
          vb.destroy()
          ib.destroy()
        },
      }
      this.cache.set(key, level, levelBytes(source))
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
