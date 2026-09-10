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

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value))
}
