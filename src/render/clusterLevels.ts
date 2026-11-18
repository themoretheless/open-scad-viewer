/**
 * Coarse display levels built lazily for plain triangle meshes that were
 * published without `lods` and have no B-rep snapshot for the display level
 * worker. Such a mesh above `minTriangles` gets a two-level LOD state: a
 * deferred placeholder for a vertex-clustered coarse level and its own base
 * geometry as the finest level. The coarse arrays are built only when the
 * scheduler targets level 0, i.e. when the display budget or the camera
 * distance asks for it, in the display level worker when one is available and
 * otherwise on an idle callback. Both paths run the same `clusterArrays`.
 *
 * Error bound: every vertex moves to the mean of its grid cell, so it moves by
 * at most the cell diagonal, and every point of a kept triangle moves by at most
 * the same amount (affine interpolation of vertex displacements). The level
 * reports the cell diagonal as `errorUpper`.
 */
import type { MeshLodLevel } from '../core/mesh'
import {
  clusterArrays,
  clusterBounds,
  clusterResolution,
  type ClusterInput,
  type ClusterPayload,
} from '../kernel/clusterDecimation'
import { isDeferredLevel, placeholderLevel } from './deferredLevels'
import { defaultIdleScheduler, type DeferredLevelLoader, type IdleScheduler } from './displayScheduler'
import { createMeshLodState } from './meshLod'
import type { GMesh } from './types'

const STRIDE = 6

export interface ClusterLevelOptions {
  /** Meshes with fewer triangles keep their single published level. */
  readonly minTriangles: number
  /** Target triangle count of the coarse level as a fraction of the base. */
  readonly ratio: number
}

export const DEFAULT_CLUSTER_LEVEL_OPTIONS: ClusterLevelOptions = { minTriangles: 50_000, ratio: 1 / 8 }

export { clusterArrays, clusterResolution, type ClusterInput, type ClusterPayload }

/**
 * Builds clustered arrays off the main thread (the display level worker). A
 * rejection that is not an abort falls back to the main-thread build.
 */
export type ClusterBuilder = (base: ClusterInput, resolution: number) => Promise<ClusterPayload | null>

/** Attach the base mesh's source run to clustered arrays. */
export function withClusterProvenance(payload: ClusterPayload, provenance: MeshLodLevel['provenance']): MeshLodLevel {
  const first = provenance[0]
  return {
    ...payload,
    provenance: [
      {
        triangleStart: 0,
        triangleEnd: payload.triangleCount,
        backside: first?.backside ?? false,
        source: first?.source ?? null,
      },
    ],
  }
}

/**
 * Vertex-clustering decimation on a uniform grid with `resolution` cells along
 * the longest axis. Returns null when nothing collapses.
 */
export function clusterLevel(
  base: Pick<MeshLodLevel, 'vertices' | 'indices' | 'edgeIndices' | 'faceIds' | 'provenance'>,
  resolution: number,
): MeshLodLevel | null {
  const payload = clusterArrays(base, resolution)
  return payload && withClusterProvenance(payload, base.provenance)
}

/** The base geometry of a mesh as its finest display level (shares the arrays). */
function baseLevel(mesh: GMesh): MeshLodLevel {
  return {
    vertices: mesh.vertices,
    indices: mesh.indices,
    edgeIndices: mesh.edgeIndices,
    faceIds: mesh.faceIds,
    bvh: mesh.bvh,
    provenance: mesh.provenance,
    triangleCount: Math.floor(mesh.indices.length / 3),
    errorUpper: 0,
  }
}

interface ClusterEntry {
  /** Coarse-to-fine: [clustered placeholder or built level, base geometry]. */
  readonly levels: MeshLodLevel[]
  readonly resolution: number
}

/** Attaches lazily clustered coarse levels to large plain meshes and builds them on demand. */
export class ClusterLevelSource implements DeferredLevelLoader {
  private readonly owned = new WeakMap<GMesh, ClusterEntry>()
  /** Instances of one asset share vertex arrays; they share one level list so the level cache sees one source. */
  private readonly shared = new WeakMap<Float32Array, ClusterEntry>()
  private readonly failed = new WeakSet<Float32Array>()
  private builder: ClusterBuilder | null = null

  constructor(
    private readonly options: ClusterLevelOptions = DEFAULT_CLUSTER_LEVEL_OPTIONS,
    private readonly schedule: IdleScheduler = defaultIdleScheduler,
  ) {}

  /** Build in a worker when one is available; null builds on the main thread in an idle slice. */
  setBuilder(builder: ClusterBuilder | null) {
    this.builder = builder
  }

  /** Meshes this source may give levels to; B-rep snapshots go to the display level worker instead. */
  eligible(mesh: GMesh): boolean {
    return (
      !mesh.lod &&
      mesh.nativeGeometry?.kind !== 'brep' &&
      mesh.indices.length / 3 >= this.options.minTriangles &&
      !this.failed.has(mesh.vertices)
    )
  }

  owns(mesh: GMesh): boolean {
    return this.owned.has(mesh)
  }

  /** Give eligible meshes a placeholder coarse level; returns meshes that gained LOD state. */
  attach(meshes: readonly GMesh[]): GMesh[] {
    const attached: GMesh[] = []
    for (const mesh of meshes) {
      if (!this.eligible(mesh)) continue
      const known = this.shared.get(mesh.vertices)
      if (known && known.levels[1]!.indices === mesh.indices) {
        const lod = createMeshLodState(mesh, known.levels)
        if (!lod) continue
        mesh.lod = lod
        this.owned.set(mesh, known)
        attached.push(mesh)
        continue
      }
      const base = baseLevel(mesh)
      const target = Math.max(1, Math.floor(base.triangleCount * this.options.ratio))
      const resolution = clusterResolution(target)
      const box = clusterBounds(mesh.vertices)
      if (!box) {
        this.failed.add(mesh.vertices)
        continue
      }
      const estimate = Math.min(base.triangleCount - 1, 12 * resolution * resolution)
      const placeholder = placeholderLevel({
        triangleCount: estimate,
        vertexCount: Math.min(Math.floor(mesh.vertices.length / STRIDE), 6 * resolution * resolution),
        errorUpper: (box.extent / resolution) * Math.sqrt(3),
      })
      const levels = [placeholder, base]
      const lod = createMeshLodState(mesh, levels)
      if (!lod) continue
      mesh.lod = lod
      const entry = { levels, resolution }
      this.owned.set(mesh, entry)
      this.shared.set(mesh.vertices, entry)
      attached.push(mesh)
    }
    return attached
  }

  /**
   * Build the coarse level in the worker when a builder is set, else (or when
   * the worker fails) on an idle callback; the store uploads it once it resolves.
   */
  load(mesh: GMesh, level: number): Promise<void> {
    const entry = this.owned.get(mesh)
    if (!entry || level !== 0) return Promise.reject(new RangeError('No clustered level to build'))
    if (!isDeferredLevel(entry.levels[0])) return Promise.resolve()
    const giveUp = () => {
      // Nothing collapses: drop the LOD state (only the base was ever bound) and stop offering one.
      this.failed.add(mesh.vertices)
      this.owned.delete(mesh)
      this.shared.delete(mesh.vertices)
      if (mesh.lod?.levels === entry.levels) mesh.lod = undefined
      return Promise.reject(new Error('Clustering did not reduce the mesh'))
    }
    if (this.failed.has(mesh.vertices)) return giveUp()
    const land = (built: MeshLodLevel | null): Promise<void> => {
      if (!isDeferredLevel(entry.levels[0])) return Promise.resolve()
      if (!built) return giveUp()
      entry.levels[0] = built
      return Promise.resolve()
    }
    const onMainThread = () =>
      new Promise<void>((resolve, reject) => {
        this.schedule(() => {
          if (!isDeferredLevel(entry.levels[0])) return resolve()
          land(clusterLevel(entry.levels[1]!, entry.resolution)).then(resolve, reject)
        })
      })
    const builder = this.builder
    if (!builder) return onMainThread()
    const base = entry.levels[1]!
    let request: Promise<ClusterPayload | null>
    try {
      request = builder(base, entry.resolution)
    } catch {
      return onMainThread()
    }
    return request.then(
      payload => land(payload && withClusterProvenance(payload, base.provenance)),
      // Worker unavailable or crashed: build here instead.
      () => onMainThread(),
    )
  }

  /** A build is never superseded by another level of the same mesh; there is nothing to cancel. */
  cancelStale(): void {}
}
