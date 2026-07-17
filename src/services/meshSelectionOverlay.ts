import { transformPoint, type Mat4, type Vec3 } from './math3d'

export interface FaceOverlayGeometry {
  triangles: Float32Array
  boundaryLines: Float32Array
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
): FaceOverlayGeometry {
  if (!triangleVertexIndices(indices, triangleIndex) || maxFaceTriangles < 1) return EMPTY_FACE_OVERLAY

  const triangleCount = Math.floor(indices.length / 3)
  const faceTriangles: number[] = []
  if (faceId !== null && faceIds.length >= triangleCount) {
    for (let triangle = 0; triangle < triangleCount; triangle++) {
      if (faceIds[triangle] !== faceId) continue
      faceTriangles.push(triangle)
      if (faceTriangles.length > maxFaceTriangles) break
    }
  }
  if (!faceTriangles.length || faceTriangles.length > maxFaceTriangles) {
    faceTriangles.length = 0
    faceTriangles.push(triangleIndex)
  }

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
