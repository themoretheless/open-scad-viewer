/**
 * Placeholder display levels: certified counts and errors known from a level
 * summary, arrays not produced yet. Kept apart from the producers so the LOD
 * binder and the scheduler can recognise placeholders without import cycles.
 */
import type { MeshLodLevel } from '../core/mesh'
import type { DisplayLevelSummaryEntry } from '../kernel/displayLevelService'
import { buildMeshBvh } from '../kernel/meshBvh'

const deferred = new WeakMap<MeshLodLevel, number>()
const EMPTY_BVH = buildMeshBvh(new Float32Array(0), new Uint32Array(0))

/** True for a placeholder whose arrays have not been produced yet. */
export const isDeferredLevel = (level: MeshLodLevel | undefined) => !!level && deferred.has(level)

/** Upload size of a level; placeholders report the size their summary promises. */
export function levelByteSize(level: MeshLodLevel): number {
  return deferred.get(level) ?? level.vertices.byteLength + level.indices.byteLength
}

export function placeholderLevel(entry: DisplayLevelSummaryEntry): MeshLodLevel {
  const level: MeshLodLevel = {
    vertices: new Float32Array(0),
    indices: new Uint32Array(0),
    edgeIndices: new Uint32Array(0),
    faceIds: new Uint32Array(0),
    bvh: EMPTY_BVH,
    provenance: [],
    triangleCount: entry.triangleCount,
    errorUpper: entry.errorUpper,
  }
  deferred.set(level, entry.vertexCount * 6 * 4 + entry.triangleCount * 3 * 4)
  return level
}
