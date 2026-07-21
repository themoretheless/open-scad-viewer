import { transformPoint, type Mat4, type Vec3 } from './math3d'
import type { MeshProvenanceRun } from '../core/mesh'

export interface FaceOverlayGeometry {
  triangles: Float32Array
  boundaryLines: Float32Array
}

/**
 * CSR (compressed sparse row) mapping from kernel face id to the triangles
 * that carry it: the triangles of face `f` are
 * `triangles[offsets[f] .. offsets[f + 1])`, ascending. Built once per mesh so
 * a face-mode hover costs O(face) instead of an O(scene) faceIds scan.
 */
export interface FaceTriangleIndex {
  /** Triangle count the index was built for; guards against stale indices. */
  triangleCount: number
  offsets: Uint32Array
  triangles: Uint32Array
}

export interface SourceOverlayGeometry extends FaceOverlayGeometry {
  triangleCount: number
  truncated: boolean
}

interface EdgeUse {
  from: number
  to: number
  count: number
}

const EMPTY_FACE_OVERLAY: FaceOverlayGeometry = {
  triangles: new Float32Array(),
  boundaryLines: new Float32Array(),
}

const EMPTY_SOURCE_OVERLAY: SourceOverlayGeometry = {
  triangles: new Float32Array(),
  boundaryLines: new Float32Array(),
  triangleCount: 0,
  truncated: false,
}

/** Hard ceiling for transient source-highlight geometry, independent of callers. */
export const MAX_SOURCE_OVERLAY_TRIANGLES = 20_000

function triangleVertexIndices(indices: Uint32Array, triangleIndex: number): [number, number, number] | null {
  const offset = triangleIndex * 3
  if (!Number.isInteger(triangleIndex) || triangleIndex < 0 || offset + 2 >= indices.length) return null
  return [indices[offset], indices[offset + 1], indices[offset + 2]]
}

function worldVertex(vertices: Float32Array, transform: Mat4, vertexIndex: number): Vec3 | null {
  const offset = vertexIndex * 6
  if (!Number.isInteger(vertexIndex) || vertexIndex < 0 || offset + 2 >= vertices.length) return null
  const point: Vec3 = [vertices[offset], vertices[offset + 1], vertices[offset + 2]]
  if (!point.every(Number.isFinite)) return null
  const world = transformPoint(transform, point)
  return world.every(Number.isFinite) ? world : null
}

/**
 * Builds the faceId → triangle CSR index with one counting-sort pass.
 * Returns null when no index can be built (empty mesh, short faceIds array) or
 * when the id space is so sparse that the offsets table would dwarf the mesh;
 * callers fall back to the direct scan in that case.
 */
export function buildFaceTriangleIndex(
  faceIds: Uint32Array,
  triangleCount: number,
): FaceTriangleIndex | null {
  const count = Number.isInteger(triangleCount) ? triangleCount : 0
  if (count < 1 || faceIds.length < count) return null

  let maxId = 0
  for (let triangle = 0; triangle < count; triangle++) {
    const id = faceIds[triangle]
    if (id > maxId) maxId = id
  }
  // Manifold face ids are bounded by the halfedge count (3 × triangles); a
  // pathologically sparse id space would make the offsets table quadratic.
  if (maxId > count * 4 + 1024) return null

  const offsets = new Uint32Array(maxId + 2)
  for (let triangle = 0; triangle < count; triangle++) offsets[faceIds[triangle] + 1]++
  for (let id = 0; id <= maxId; id++) offsets[id + 1] += offsets[id]

  const triangles = new Uint32Array(count)
  const cursor = new Uint32Array(maxId + 1)
  for (let triangle = 0; triangle < count; triangle++) {
    const id = faceIds[triangle]
    triangles[offsets[id] + cursor[id]++] = triangle
  }
  return { triangleCount: count, offsets, triangles }
}

function provenanceRunBounds(run: MeshProvenanceRun, triangleCount: number): [number, number] | null {
  if (!Number.isFinite(run.triangleStart) || !Number.isFinite(run.triangleEnd)) return null
  const start = Math.max(0, Math.min(triangleCount, Math.floor(run.triangleStart)))
  const end = Math.max(start, Math.min(triangleCount, Math.floor(run.triangleEnd)))
  return end > start ? [start, end] : null
}

/**
 * Builds a filled face and its topological boundary from a picked triangle.
 * If the kernel face is unusually large, it intentionally falls back to the
 * picked triangle so an inspection gesture can never allocate an unbounded
 * transient GPU buffer.
 */
export function buildFaceOverlayGeometry(
  vertices: Float32Array,
  indices: Uint32Array,
  faceIds: Uint32Array,
  transform: Mat4,
  triangleIndex: number,
  faceId: number | null,
  maxFaceTriangles = 20_000,
  faceIndex: FaceTriangleIndex | null = null,
): FaceOverlayGeometry {
  if (!triangleVertexIndices(indices, triangleIndex) || maxFaceTriangles < 1) return EMPTY_FACE_OVERLAY

  const triangleCount = Math.floor(indices.length / 3)
  let faceTriangles: ArrayLike<number> & Iterable<number> = []
  if (faceId !== null && faceIds.length >= triangleCount) {
    if (faceIndex && faceIndex.triangleCount === triangleCount) {
      // O(face) path: the CSR rows hold the same ascending triangle order the
      // direct scan produces, so the generated geometry is identical.
      const start = faceId + 1 < faceIndex.offsets.length ? faceIndex.offsets[faceId] : 0
      const end = faceId + 1 < faceIndex.offsets.length ? faceIndex.offsets[faceId + 1] : 0
      if (end > start && end - start <= maxFaceTriangles) {
        faceTriangles = faceIndex.triangles.subarray(start, end)
      }
    } else {
      const scanned: number[] = []
      for (let triangle = 0; triangle < triangleCount; triangle++) {
        if (faceIds[triangle] !== faceId) continue
        scanned.push(triangle)
        if (scanned.length > maxFaceTriangles) break
      }
      if (scanned.length && scanned.length <= maxFaceTriangles) faceTriangles = scanned
    }
  }
  if (!faceTriangles.length) faceTriangles = [triangleIndex]

  const trianglePositions: number[] = []
  const edges = new Map<string, EdgeUse>()
  for (const triangle of faceTriangles) {
    const vertexIndices = triangleVertexIndices(indices, triangle)
    if (!vertexIndices) continue
    const points = vertexIndices.map(vertex => worldVertex(vertices, transform, vertex))
    if (points.some(point => point === null)) continue
    for (const point of points as Vec3[]) trianglePositions.push(...point)

    for (let edge = 0; edge < 3; edge++) {
      const from = vertexIndices[edge]
      const to = vertexIndices[(edge + 1) % 3]
      const low = Math.min(from, to)
      const high = Math.max(from, to)
      const key = `${low}:${high}`
      const existing = edges.get(key)
      if (existing) existing.count++
      else edges.set(key, { from, to, count: 1 })
    }
  }

  const boundaryPositions: number[] = []
  for (const edge of edges.values()) {
    if (edge.count !== 1) continue
    const from = worldVertex(vertices, transform, edge.from)
    const to = worldVertex(vertices, transform, edge.to)
    if (from && to) boundaryPositions.push(...from, ...to)
  }

  return {
    triangles: new Float32Array(trianglePositions),
    boundaryLines: new Float32Array(boundaryPositions),
  }
}

/** Resolves the nearest picked triangle corner in world space. */
export function pointOverlayPosition(
  vertices: Float32Array,
  indices: Uint32Array,
  transform: Mat4,
  triangleIndex: number,
  barycentric: Vec3,
): Vec3 | null {
  const vertexIndices = triangleVertexIndices(indices, triangleIndex)
  if (!vertexIndices || barycentric.length !== 3 || !barycentric.every(Number.isFinite)) return null
  let corner = 0
  if (barycentric[1] > barycentric[corner]) corner = 1
  if (barycentric[2] > barycentric[corner]) corner = 2
  return worldVertex(vertices, transform, vertexIndices[corner])
}

/**
 * Builds the surviving world-space triangles and topological boundary for one
 * source operation. The matching triangle count is determined and capped
 * before output storage is allocated, so malformed provenance or a very broad
 * source operation cannot create an unbounded transient allocation.
 */
export function buildSourceOverlayGeometry(
  vertices: Float32Array,
  indices: Uint32Array,
  provenance: readonly MeshProvenanceRun[],
  transform: Mat4,
  sourceId: number,
  maxTriangles = MAX_SOURCE_OVERLAY_TRIANGLES,
): SourceOverlayGeometry {
  if (!Number.isInteger(sourceId) || sourceId < 0 || maxTriangles < 1) return EMPTY_SOURCE_OVERLAY

  const triangleCount = Math.floor(indices.length / 3)
  const triangleLimit = Math.min(MAX_SOURCE_OVERLAY_TRIANGLES, Math.max(0, Math.floor(maxTriangles)))
  if (!triangleCount || !triangleLimit) return EMPTY_SOURCE_OVERLAY

  // Count first and saturate at limit + 1. No output-sized allocation occurs
  // until the hard cap is known.
  let matchingTriangles = 0
  for (const run of provenance) {
    if (run.source?.id !== sourceId) continue
    const bounds = provenanceRunBounds(run, triangleCount)
    if (!bounds) continue
    const [start, end] = bounds
    matchingTriangles = Math.min(triangleLimit + 1, matchingTriangles + end - start)
  }
  if (!matchingTriangles) return EMPTY_SOURCE_OVERLAY

  const allocationTriangles = Math.min(triangleLimit, matchingTriangles)
  const trianglePositions = new Float32Array(allocationTriangles * 9)
  const edges = new Map<string, EdgeUse>()
  let writtenTriangles = 0

  outer: for (const run of provenance) {
    if (run.source?.id !== sourceId) continue
    const bounds = provenanceRunBounds(run, triangleCount)
    if (!bounds) continue
    const [start, end] = bounds
    for (let triangle = start; triangle < end; triangle++) {
      if (writtenTriangles >= allocationTriangles) break outer
      const vertexIndices = triangleVertexIndices(indices, triangle)
      if (!vertexIndices) continue
      const points = vertexIndices.map(vertex => worldVertex(vertices, transform, vertex))
      if (points.some(point => point === null)) continue

      let target = writtenTriangles * 9
      for (const point of points as Vec3[]) {
        trianglePositions[target++] = point[0]
        trianglePositions[target++] = point[1]
        trianglePositions[target++] = point[2]
      }
      writtenTriangles++

      for (let edge = 0; edge < 3; edge++) {
        const from = vertexIndices[edge]
        const to = vertexIndices[(edge + 1) % 3]
        const low = Math.min(from, to)
        const high = Math.max(from, to)
        const key = `${low}:${high}`
        const existing = edges.get(key)
        if (existing) existing.count++
        else edges.set(key, { from, to, count: 1 })
      }
    }
  }

  let boundaryCapacity = 0
  for (const edge of edges.values()) {
    if (edge.count === 1) boundaryCapacity += 6
  }
  const boundaryPositions = new Float32Array(boundaryCapacity)
  let boundaryOffset = 0
  for (const edge of edges.values()) {
    if (edge.count !== 1) continue
    const from = worldVertex(vertices, transform, edge.from)
    const to = worldVertex(vertices, transform, edge.to)
    if (!from || !to) continue
    boundaryPositions.set(from, boundaryOffset)
    boundaryPositions.set(to, boundaryOffset + 3)
    boundaryOffset += 6
  }

  return {
    triangles: writtenTriangles === allocationTriangles
      ? trianglePositions
      : trianglePositions.slice(0, writtenTriangles * 9),
    boundaryLines: boundaryOffset === boundaryPositions.length
      ? boundaryPositions
      : boundaryPositions.slice(0, boundaryOffset),
    triangleCount: writtenTriangles,
    truncated: matchingTriangles > triangleLimit,
  }
}
