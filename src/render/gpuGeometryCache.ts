/**
 * Base geometry buffers keyed by geometry asset id, retained across scene
 * publications. Assets referenced by the live scene are pinned; assets that
 * left the scene stay resident (and are reused if they come back) until the
 * shared `GpuResidencyCache` budget evicts them. Entries own their buffers:
 * scene teardown must skip anything `owns` reports.
 */
import { sameTypedArray } from './drawSorting'
import type { GpuResidencyCache, ResidentResource } from './gpuResidencyCache'

interface CachedGeometry extends ResidentResource {
  readonly vb: GPUBuffer
  readonly ib: GPUBuffer
  readonly vertices: Float32Array
  readonly indices: Uint32Array
}

const PREFIX = 'geometry:'

export class GpuGeometryCache {
  private live = new Set<string>()
  private adoptedInStage: string[] | null = null
  private readonly owned = new Map<GPUBuffer, string>()
  private readonly unpin: () => void

  constructor(
    private readonly residency: GpuResidencyCache,
    private readonly onEvict: (entry: { vb: GPUBuffer; ib: GPUBuffer }) => void = () => {},
  ) {
    this.unpin = residency.pin(key => key.startsWith(PREFIX) && this.live.has(key.slice(PREFIX.length)))
  }

  get size() {
    return this.owned.size / 2
  }

  get bytes() {
    return this.residency.lru.bytesWhere(key => key.startsWith(PREFIX))
  }

  has(assetId: string) {
    return this.residency.lru.has(PREFIX + assetId)
  }

  owns(buffer: GPUBuffer) {
    return this.owned.has(buffer)
  }

  /** Buffers for an asset whose cached bytes equal `vertices`/`indices`, or null. */
  lookup(assetId: string, vertices: Float32Array, indices: Uint32Array): { vb: GPUBuffer; ib: GPUBuffer } | null {
    const entry = this.residency.lru.get(PREFIX + assetId) as CachedGeometry | undefined
    if (!entry) return null
    const same =
      (entry.vertices === vertices && entry.indices === indices) ||
      (sameTypedArray(entry.vertices, vertices) && sameTypedArray(entry.indices, indices))
    return same ? entry : null
  }

  /**
   * Run one publication's staging with `incoming` pinned next to the live set,
   * so buffers adopted during it cannot evict each other. A failed staging
   * restores the previous live set; `publish` commits a successful one.
   */
  stage<T>(incoming: Iterable<string>, staging: () => T): T {
    const previous = this.live
    this.live = new Set([...previous, ...incoming])
    const adopted: string[] = []
    this.adoptedInStage = adopted
    try {
      return staging()
    } catch (error) {
      // Buffers uploaded by the failed staging are released, not retained.
      this.live = previous
      for (const key of adopted) this.residency.lru.delete(key)
      throw error
    } finally {
      this.adoptedInStage = null
    }
  }

  /** Take ownership of freshly uploaded buffers; a no-op if the asset is already cached. */
  adopt(assetId: string, vertices: Float32Array, indices: Uint32Array, vb: GPUBuffer, ib: GPUBuffer) {
    const key = PREFIX + assetId
    if (this.residency.lru.has(key)) return
    this.live.add(assetId)
    const entry: CachedGeometry = {
      vb,
      ib,
      vertices,
      indices,
      destroy: () => {
        this.owned.delete(vb)
        this.owned.delete(ib)
        this.onEvict({ vb, ib })
        vb.destroy()
        ib.destroy()
      },
    }
    this.owned.set(vb, assetId)
    this.owned.set(ib, assetId)
    this.adoptedInStage?.push(key)
    this.residency.lru.set(key, entry, vertices.byteLength + indices.byteLength)
  }

  /** Commit the live asset set of the published scene and evict over budget. */
  publish(assetIds: Iterable<string>) {
    this.live = new Set(assetIds)
    this.residency.lru.trim()
  }

  dispose() {
    this.live = new Set()
    this.residency.lru.invalidate(key => key.startsWith(PREFIX))
    this.unpin()
  }
}
