/**
 * Coarse display levels built lazily for plain triangle meshes that were
 * published without `lods` and have no B-rep snapshot for the display level
 * worker. Such a mesh above `minTriangles` gets a two-level LOD state: a
 * deferred placeholder for a vertex-clustered coarse level and its own base
 * geometry as the finest level. The coarse arrays are built only when the
 * scheduler targets level 0, i.e. when the display budget or the camera
 * distance asks for it, on an idle callback.
 *
 * Error bound: every vertex moves to the mean of its grid cell, so it moves by
 * at most the cell diagonal, and every point of a kept triangle moves by at most
 * the same amount (affine interpolation of vertex displacements). The level
 * reports the cell diagonal as `errorUpper`.
 */
import type { MeshLodLevel } from '../core/mesh'
import { buildMeshBvh } from '../kernel/meshBvh'
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

/** Bounds of the interleaved position/normal vertices of a mesh. */
function bounds(vertices: Float32Array): { min: [number, number, number]; extent: number } | null {
  const min: [number, number, number] = [Infinity, Infinity, Infinity]
  const max: [number, number, number] = [-Infinity, -Infinity, -Infinity]
  for (let offset = 0; offset + 2 < vertices.length; offset += STRIDE) {
    for (let axis = 0; axis < 3; axis++) {
      const value = vertices[offset + axis]!
      if (value < min[axis]!) min[axis] = value
      if (value > max[axis]!) max[axis] = value
    }
  }
  const extent = Math.max(max[0] - min[0], max[1] - min[1], max[2] - min[2])
  return Number.isFinite(extent) && extent > 0 ? { min, extent } : null
}

/** Grid cells per axis that roughly yield `targetTriangles` on a closed surface. */
export const clusterResolution = (targetTriangles: number) => Math.max(2, Math.round(Math.sqrt(targetTriangles / 12)))

/**
 * Vertex-clustering decimation on a uniform grid with `resolution` cells along
 * the longest axis. Returns null when nothing collapses.
 */
export function clusterLevel(
  base: Pick<MeshLodLevel, 'vertices' | 'indices' | 'edgeIndices' | 'faceIds' | 'provenance'>,
  resolution: number,
): MeshLodLevel | null {
  const box = bounds(base.vertices)
  if (!box) return null
  const cell = box.extent / resolution
  const vertexCount = Math.floor(base.vertices.length / STRIDE)
  const cellOf = new Map<number, number>()
  const remap = new Uint32Array(vertexCount)
  const sums: number[] = []
  const cellIndex = (value: number, axis: number) =>
    Math.min(resolution - 1, Math.max(0, Math.floor((value - box.min[axis]!) / cell)))
  for (let vertex = 0; vertex < vertexCount; vertex++) {
    const offset = vertex * STRIDE
    const key =
      (cellIndex(base.vertices[offset]!, 0) * (resolution + 1) + cellIndex(base.vertices[offset + 1]!, 1)) *
        (resolution + 1) +
      cellIndex(base.vertices[offset + 2]!, 2)
    let cluster = cellOf.get(key)
    if (cluster === undefined) {
      cluster = cellOf.size
      cellOf.set(key, cluster)
      sums.push(0, 0, 0, 0, 0, 0, 0)
    }
    remap[vertex] = cluster
    const at = cluster * 7
    for (let component = 0; component < STRIDE; component++) sums[at + component]! += base.vertices[offset + component]!
    sums[at + 6]! += 1
  }
  const indices: number[] = []
  const faceIds: number[] = []
  const seen = new Set<string>()
  for (let triangle = 0; triangle * 3 + 2 < base.indices.length; triangle++) {
    const a = remap[base.indices[triangle * 3]!]!
    const b = remap[base.indices[triangle * 3 + 1]!]!
    const c = remap[base.indices[triangle * 3 + 2]!]!
    if (a === b || b === c || a === c) continue
    // Same oriented triangle from several source triangles: keep one.
    const low = Math.min(a, b, c)
    const key = low === a ? `${a},${b},${c}` : low === b ? `${b},${c},${a}` : `${c},${a},${b}`
    if (seen.has(key)) continue
    seen.add(key)
    indices.push(a, b, c)
    faceIds.push(base.faceIds[triangle] ?? 0)
  }
  const baseTriangles = Math.floor(base.indices.length / 3)
  if (indices.length / 3 >= baseTriangles || indices.length === 0) return null
  const vertices = new Float32Array(cellOf.size * STRIDE)
  for (let cluster = 0; cluster < cellOf.size; cluster++) {
    const at = cluster * 7
    const count = sums[at + 6]!
    for (let axis = 0; axis < 3; axis++) vertices[cluster * STRIDE + axis] = sums[at + axis]! / count
    const nx = sums[at + 3]!
    const ny = sums[at + 4]!
    const nz = sums[at + 5]!
    const length = Math.hypot(nx, ny, nz) || 1
    vertices[cluster * STRIDE + 3] = nx / length
    vertices[cluster * STRIDE + 4] = ny / length
    vertices[cluster * STRIDE + 5] = nz / length
  }
  const edges: number[] = []
  const edgeSeen = new Set<number>()
  for (let edge = 0; edge * 2 + 1 < base.edgeIndices.length; edge++) {
    const a = remap[base.edgeIndices[edge * 2]!]
    const b = remap[base.edgeIndices[edge * 2 + 1]!]
    if (a === undefined || b === undefined || a === b) continue
    const key = Math.min(a, b) * cellOf.size + Math.max(a, b)
    if (edgeSeen.has(key)) continue
    edgeSeen.add(key)
    edges.push(a, b)
  }
  const triangleCount = indices.length / 3
  const indexArray = new Uint32Array(indices)
  const first = base.provenance[0]
  return {
    vertices,
    indices: indexArray,
    edgeIndices: new Uint32Array(edges),
    faceIds: new Uint32Array(faceIds),
    bvh: buildMeshBvh(vertices, indexArray),
    provenance: [
      {
        triangleStart: 0,
        triangleEnd: triangleCount,
        backside: first?.backside ?? false,
        source: first?.source ?? null,
      },
    ],
    triangleCount,
    errorUpper: cell * Math.sqrt(3),
  }
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

  constructor(
    private readonly options: ClusterLevelOptions = DEFAULT_CLUSTER_LEVEL_OPTIONS,
    private readonly schedule: IdleScheduler = defaultIdleScheduler,
  ) {}

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
      const box = bounds(mesh.vertices)
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

  /** Build the coarse level on an idle callback; the store uploads it once it resolves. */
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
    return new Promise((resolve, reject) => {
      this.schedule(() => {
        if (!isDeferredLevel(entry.levels[0])) return resolve()
        const built = clusterLevel(entry.levels[1]!, entry.resolution)
        if (!built) return giveUp().catch(reject)
        entry.levels[0] = built
        resolve()
      })
    })
  }

  /** Building is a single synchronous slice; there is nothing in flight to cancel. */
  cancelStale(): void {}
}
