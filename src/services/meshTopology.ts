/**
 * Extracts meaningful, renderable edges from a triangle mesh.
 *
 * Manifold meshes may contain several "property vertices" at the same geometric
 * vertex (for example, one per sharp normal). The optional merge arrays restore
 * that topology. When they are absent we fall back to exact-position welding.
 *
 * The computation runs in the Rust geometry kernel
 * (crates/polygon-core/src/edges.rs) through the raw-buffer binding in
 * services/geometry/meshAnalysis.ts. This module keeps the validation and
 * error contracts (RangeError/TypeError) plus the crease-angle handling.
 */

import type { MeshTopologyDiagnostics } from '../core/mesh'
import { clamp } from './math3d'
import { extractSemanticEdgesInKernel } from './geometry/meshAnalysis'

export type { MeshTopologyDiagnostics } from '../core/mesh'

export interface SemanticEdgeOptions {
  /** Minimum angle between adjacent face normals that is rendered as a crease. */
  creaseAngleDegrees?: number
  /** Manifold Mesh.mergeFromVert. Must be supplied together with mergeToVert. */
  mergeFromVert?: ArrayLike<number>
  /** Manifold Mesh.mergeToVert. Must be supplied together with mergeFromVert. */
  mergeToVert?: ArrayLike<number>
  /**
   * Weld vertices with exactly equal Float32 positions in addition to merge
   * pairs. Defaults to true only when merge arrays are not supplied.
   */
  weldCoincidentVertices?: boolean
}

export interface SemanticEdgesResult {
  /** Pairs of vertex indices suitable for a GPU line-list index buffer. */
  indices: Uint32Array
  diagnostics: MeshTopologyDiagnostics
}

const VERTEX_STRIDE = 6
const DEFAULT_CREASE_ANGLE_DEGREES = 30

/**
 * Returns boundary, crease and non-manifold edges, while removing coplanar
 * triangulation diagonals. Degenerate triangles do not contribute topology.
 */
export function extractSemanticEdges(
  vertices: Float32Array,
  triangleIndices: Uint32Array,
  options: SemanticEdgeOptions = {},
): SemanticEdgesResult {
  if (vertices.length % VERTEX_STRIDE !== 0) {
    throw new RangeError(`Expected position/normal vertices with stride ${VERTEX_STRIDE}`)
  }
  if (triangleIndices.length % 3 !== 0) {
    throw new RangeError('Triangle index count must be divisible by 3')
  }

  const vertexCount = vertices.length / VERTEX_STRIDE
  validateIndices(triangleIndices, vertexCount)

  const mergeFrom = options.mergeFromVert
  const mergeTo = options.mergeToVert
  if ((mergeFrom === undefined) !== (mergeTo === undefined)) {
    throw new TypeError('mergeFromVert and mergeToVert must be supplied together')
  }
  if (mergeFrom && mergeTo && mergeFrom.length !== mergeTo.length) {
    throw new RangeError('mergeFromVert and mergeToVert must have equal lengths')
  }
  if (mergeFrom && mergeTo) {
    for (let i = 0; i < mergeFrom.length; i++) {
      validateVertexIndex(mergeFrom[i], vertexCount, 'mergeFromVert')
      validateVertexIndex(mergeTo[i], vertexCount, 'mergeToVert')
    }
  }

  const weldCoincident = options.weldCoincidentVertices ?? mergeFrom === undefined

  const requestedCreaseAngle = options.creaseAngleDegrees ?? DEFAULT_CREASE_ANGLE_DEGREES
  if (!Number.isFinite(requestedCreaseAngle)) {
    throw new RangeError('creaseAngleDegrees must be finite')
  }
  const creaseAngle = clamp(requestedCreaseAngle, 0, 180)
  // Computed here so the classification constant keeps the exact JavaScript
  // Math.cos result; the kernel compares against it verbatim.
  const creaseDotThreshold = Math.cos(creaseAngle * Math.PI / 180)

  return extractSemanticEdgesInKernel(
    vertices,
    triangleIndices,
    mergeFrom,
    mergeTo,
    weldCoincident,
    creaseDotThreshold,
  )
}


function validateIndices(indices: Uint32Array, vertexCount: number) {
  for (let i = 0; i < indices.length; i++) {
    validateVertexIndex(indices[i], vertexCount, 'triangleIndices')
  }
}

function validateVertexIndex(index: number, vertexCount: number, label: string) {
  if (!Number.isInteger(index) || index < 0 || index >= vertexCount) {
    throw new RangeError(`${label} contains out-of-range vertex index ${index}`)
  }
}

/**
 * Hardware-accelerated semantic edge extraction via WebGPU compute shaders
 * (`gpuSemanticEdges.ts`), with automatic fallback to `extractSemanticEdges`
 * when WebGPU is unavailable or Manifold property-vertex merge arrays are supplied.
 */
export async function extractSemanticEdgesGpu(
  vertices: Float32Array,
  triangleIndices: Uint32Array,
  options: SemanticEdgeOptions = {},
): Promise<SemanticEdgesResult> {
  if (typeof navigator !== 'undefined' && navigator.gpu
    && options.mergeFromVert === undefined && options.mergeToVert === undefined && !(options.weldCoincidentVertices ?? true)) {
    const requestedCreaseAngle = options.creaseAngleDegrees ?? DEFAULT_CREASE_ANGLE_DEGREES
    if (
      vertices.length % VERTEX_STRIDE === 0 &&
      triangleIndices.length % 3 === 0 &&
      Number.isFinite(requestedCreaseAngle)
    ) {
      const { runGpuSemanticEdges } = await import('./gpuSemanticEdges')
      const gpuResult = await runGpuSemanticEdges(vertices, triangleIndices, {
        strideFloats: VERTEX_STRIDE,
        creaseAngleDegrees: clamp(requestedCreaseAngle, 0, 180),
      })
      return {
        indices: gpuResult.indices,
        diagnostics: {
          boundary: 0,
          crease: gpuResult.edgeCount,
          nonManifold: 0,
          degenerate: 0,
        },
      }
    }
  }
  return extractSemanticEdges(vertices, triangleIndices, options)
}

/**
 * Extracts sharp feature and boundary edges from a position/index triangle mesh
 * where adjacent face normals differ by more than `creaseAngleDeg` (default 30°).
 * Coincident vertices with identical coordinates are welded automatically.
 */
export function extractSharpFeatureEdges(
  mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> },
  creaseAngleDeg = DEFAULT_CREASE_ANGLE_DEGREES,
): Array<[[number, number, number], [number, number, number]]> {
  const { positions, indices } = mesh
  if (positions.length % 3 !== 0) {
    throw new RangeError('Expected mesh positions with stride 3')
  }
  if (indices.length % 3 !== 0) {
    throw new RangeError('Triangle index count must be divisible by 3')
  }
  if (!Number.isFinite(creaseAngleDeg)) {
    throw new RangeError('creaseAngleDeg must be finite')
  }
  const vertexCount = positions.length / 3
  const clampedAngle = clamp(creaseAngleDeg, 0, 180)
  const cosThreshold = Math.cos((clampedAngle * Math.PI) / 180)

  // Map unique 3D positions to canonical welded vertex indices
  const posKeyToCanonical = new Map<string, number>()
  const canonicalCoords: Array<[number, number, number]> = []
  const vertexToCanonical = new Int32Array(vertexCount)
  for (let v = 0; v < vertexCount; v++) {
    const x = Math.fround(positions[v * 3]!)
    const y = Math.fround(positions[v * 3 + 1]!)
    const z = Math.fround(positions[v * 3 + 2]!)
    const key = `${x === 0 ? 0 : x},${y === 0 ? 0 : y},${z === 0 ? 0 : z}`
    let canon = posKeyToCanonical.get(key)
    if (canon === undefined) {
      canon = canonicalCoords.length
      posKeyToCanonical.set(key, canon)
      canonicalCoords.push([positions[v * 3]!, positions[v * 3 + 1]!, positions[v * 3 + 2]!])
    }
    vertexToCanonical[v] = canon
  }

  interface EdgeEntry {
    u: number
    v: number
    normals: Array<[number, number, number]>
  }
  const canonicalCount = Math.max(1, canonicalCoords.length)
  const edgeMap = new Map<number, EdgeEntry>()

  for (let i = 0; i < indices.length; i += 3) {
    const raw0 = indices[i]!
    const raw1 = indices[i + 1]!
    const raw2 = indices[i + 2]!
    if (raw0 < 0 || raw0 >= vertexCount || raw1 < 0 || raw1 >= vertexCount || raw2 < 0 || raw2 >= vertexCount) {
      throw new RangeError('mesh.indices contains out-of-range vertex index')
    }
    const c0 = vertexToCanonical[raw0]!
    const c1 = vertexToCanonical[raw1]!
    const c2 = vertexToCanonical[raw2]!
    if (c0 === c1 || c1 === c2 || c2 === c0) continue

    const p0 = canonicalCoords[c0]!
    const p1 = canonicalCoords[c1]!
    const p2 = canonicalCoords[c2]!
    const ax = p1[0] - p0[0]
    const ay = p1[1] - p0[1]
    const az = p1[2] - p0[2]
    const bx = p2[0] - p0[0]
    const by = p2[1] - p0[1]
    const bz = p2[2] - p0[2]
    const nx = ay * bz - az * by
    const ny = az * bx - ax * bz
    const nz = ax * by - ay * bx
    const len = Math.hypot(nx, ny, nz)
    if (len <= 1e-12) continue
    const normal: [number, number, number] = [nx / len, ny / len, nz / len]

    const addEdgeNormal = (a: number, b: number) => {
      const u = a < b ? a : b
      const v = a < b ? b : a
      const edgeKey = u * canonicalCount + v
      let entry = edgeMap.get(edgeKey)
      if (!entry) {
        entry = { u, v, normals: [] }
        edgeMap.set(edgeKey, entry)
      }
      entry.normals.push(normal)
    }
    addEdgeNormal(c0, c1)
    addEdgeNormal(c1, c2)
    addEdgeNormal(c2, c0)
  }

  const sharpEdges: Array<[[number, number, number], [number, number, number]]> = []
  for (const entry of edgeMap.values()) {
    const { u, v, normals } = entry
    let isSharp = false
    if (normals.length === 1 || normals.length > 2) {
      // Boundary edge (1 incident triangle) or non-manifold edge (>2 incident triangles)
      isSharp = true
    } else if (normals.length === 2) {
      const n0 = normals[0]!
      const n1 = normals[1]!
      const dot = n0[0] * n1[0] + n0[1] * n1[1] + n0[2] * n1[2]
      // Strictly greater than creaseAngleDeg -> dot < cosThreshold - 1e-7
      if (dot < cosThreshold - 1e-7) {
        isSharp = true
      }
    }
    if (isSharp) {
      const pA = canonicalCoords[u]!
      const pB = canonicalCoords[v]!
      sharpEdges.push([
        [pA[0], pA[1], pA[2]],
        [pB[0], pB[1], pB[2]],
      ])
    }
  }

  return sharpEdges
}
