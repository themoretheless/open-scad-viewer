/**
 * Vertex-clustering decimation of interleaved position/normal triangle meshes.
 * Pure array code shared by the main thread and the display level worker, so
 * both produce byte-identical coarse levels.
 */
import type { MeshLodLevel } from '../core/mesh'
import { buildMeshBvh } from './meshBvh'

const STRIDE = 6

/** Input arrays of a clustered level. */
export type ClusterInput = Pick<MeshLodLevel, 'vertices' | 'indices' | 'edgeIndices' | 'faceIds'>
/** A clustered level without provenance (the caller attaches its own source run). */
export type ClusterPayload = Omit<MeshLodLevel, 'provenance'>

/** Bounds of the interleaved position/normal vertices of a mesh. */
export function clusterBounds(vertices: Float32Array): { min: [number, number, number]; extent: number } | null {
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
export function clusterArrays(base: ClusterInput, resolution: number): ClusterPayload | null {
  const box = clusterBounds(base.vertices)
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
  return {
    vertices,
    indices: indexArray,
    edgeIndices: new Uint32Array(edges),
    faceIds: new Uint32Array(faceIds),
    bvh: buildMeshBvh(vertices, indexArray),
    triangleCount,
    errorUpper: cell * Math.sqrt(3),
  }
}
