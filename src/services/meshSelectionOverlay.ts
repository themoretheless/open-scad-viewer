import { packUndirectedEdgeKey, transformPoint, type Mat4, type Vec3 } from './math3d'
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

const SCRATCH_TRI_PTS = new Float64Array(9)

function triangleVertexIndices(indices: Uint32Array, triangleIndex: number): [number, number, number] | null {
  const offset = triangleIndex * 3
  if (!Number.isInteger(triangleIndex) || triangleIndex < 0 || offset + 2 >= indices.length) return null
  return [indices[offset], indices[offset + 1], indices[offset + 2]]
}

function worldVertex(vertices: Float32Array, transform: Mat4, vertexIndex: number): Vec3 | null {
  const offset = vertexIndex * 6
  if (!Number.isInteger(vertexIndex) || vertexIndex < 0 || offset + 2 >= vertices.length) return null
  const point: Vec3 = [vertices[offset], vertices[offset + 1], vertices[offset + 2]]
  if (!Number.isFinite(point[0]) || !Number.isFinite(point[1]) || !Number.isFinite(point[2])) return null
  const world = transformPoint(transform, point)
  return Number.isFinite(world[0]) && Number.isFinite(world[1]) && Number.isFinite(world[2]) ? world : null
}

function writeWorldVertex(
  vertices: Float32Array,
  transform: Mat4,
  vertexIndex: number,
  out: Float64Array,
  outOffset: number,
): boolean {
  const offset = vertexIndex * 6
  if (!Number.isInteger(vertexIndex) || vertexIndex < 0 || offset + 2 >= vertices.length) return false
  const x = vertices[offset]
  const y = vertices[offset + 1]
  const z = vertices[offset + 2]
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) return false
  const w = transform[12] * x + transform[13] * y + transform[14] * z + transform[15]
  const iw = Number.isFinite(w) && Math.abs(w) > 1e-12 ? 1 / w : 1
  const wx = (transform[0] * x + transform[1] * y + transform[2] * z + transform[3]) * iw
  const wy = (transform[4] * x + transform[5] * y + transform[6] * z + transform[7]) * iw
  const wz = (transform[8] * x + transform[9] * y + transform[10] * z + transform[11]) * iw
  if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) return false
  out[outOffset] = wx
  out[outOffset + 1] = wy
  out[outOffset + 2] = wz
  return true
}

function recordEdge(edges: Map<number | string, EdgeUse>, from: number, to: number): void {
  const key = packUndirectedEdgeKey(from, to)
  const existing = edges.get(key)
  if (existing) existing.count++
  else edges.set(key, { from, to, count: 1 })
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

  const trianglePositions = new Float32Array(faceTriangles.length * 9)
  let writtenPositions = 0
  const edges = new Map<number | string, EdgeUse>()
  for (let idx = 0; idx < faceTriangles.length; idx++) {
    const triangle = faceTriangles[idx]
    const triOffset = triangle * 3
    if (!Number.isInteger(triangle) || triangle < 0 || triOffset + 2 >= indices.length) continue
    const i0 = indices[triOffset]
    const i1 = indices[triOffset + 1]
    const i2 = indices[triOffset + 2]
    if (
      !writeWorldVertex(vertices, transform, i0, SCRATCH_TRI_PTS, 0) ||
      !writeWorldVertex(vertices, transform, i1, SCRATCH_TRI_PTS, 3) ||
      !writeWorldVertex(vertices, transform, i2, SCRATCH_TRI_PTS, 6)
    ) {
      continue
    }
    for (let k = 0; k < 9; k++) {
      trianglePositions[writtenPositions++] = SCRATCH_TRI_PTS[k]
    }

    recordEdge(edges, i0, i1)
    recordEdge(edges, i1, i2)
    recordEdge(edges, i2, i0)
  }

  let boundaryCapacity = 0
  for (const edge of edges.values()) {
    if (edge.count === 1) boundaryCapacity += 6
  }
  const boundaryPositions = new Float32Array(boundaryCapacity)
  let boundaryOffset = 0
  for (const edge of edges.values()) {
    if (edge.count !== 1) continue
    if (
      !writeWorldVertex(vertices, transform, edge.from, SCRATCH_TRI_PTS, 0) ||
      !writeWorldVertex(vertices, transform, edge.to, SCRATCH_TRI_PTS, 3)
    ) {
      continue
    }
    for (let k = 0; k < 6; k++) {
      boundaryPositions[boundaryOffset + k] = SCRATCH_TRI_PTS[k]
    }
    boundaryOffset += 6
  }

  return {
    triangles: writtenPositions === trianglePositions.length ? trianglePositions : trianglePositions.slice(0, writtenPositions),
    boundaryLines: boundaryOffset === boundaryPositions.length ? boundaryPositions : boundaryPositions.slice(0, boundaryOffset),
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
  if (!vertexIndices || barycentric.length !== 3 || !Number.isFinite(barycentric[0]) || !Number.isFinite(barycentric[1]) || !Number.isFinite(barycentric[2])) return null
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
  const edges = new Map<number | string, EdgeUse>()
  let writtenTriangles = 0

  outer: for (const run of provenance) {
    if (run.source?.id !== sourceId) continue
    const bounds = provenanceRunBounds(run, triangleCount)
    if (!bounds) continue
    const [start, end] = bounds
    for (let triangle = start; triangle < end; triangle++) {
      if (writtenTriangles >= allocationTriangles) break outer
      const triOffset = triangle * 3
      if (!Number.isInteger(triangle) || triangle < 0 || triOffset + 2 >= indices.length) continue
      const i0 = indices[triOffset]
      const i1 = indices[triOffset + 1]
      const i2 = indices[triOffset + 2]
      if (
        !writeWorldVertex(vertices, transform, i0, SCRATCH_TRI_PTS, 0) ||
        !writeWorldVertex(vertices, transform, i1, SCRATCH_TRI_PTS, 3) ||
        !writeWorldVertex(vertices, transform, i2, SCRATCH_TRI_PTS, 6)
      ) {
        continue
      }

      const target = writtenTriangles * 9
      for (let k = 0; k < 9; k++) {
        trianglePositions[target + k] = SCRATCH_TRI_PTS[k]
      }
      writtenTriangles++

      recordEdge(edges, i0, i1)
      recordEdge(edges, i1, i2)
      recordEdge(edges, i2, i0)
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
    if (
      !writeWorldVertex(vertices, transform, edge.from, SCRATCH_TRI_PTS, 0) ||
      !writeWorldVertex(vertices, transform, edge.to, SCRATCH_TRI_PTS, 3)
    ) {
      continue
    }
    for (let k = 0; k < 6; k++) {
      boundaryPositions[boundaryOffset + k] = SCRATCH_TRI_PTS[k]
    }
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
