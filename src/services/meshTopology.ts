/**
 * Extracts meaningful, renderable edges from a triangle mesh.
 *
 * Manifold meshes may contain several "property vertices" at the same geometric
 * vertex (for example, one per sharp normal). The optional merge arrays restore
 * that topology. When they are absent we fall back to exact-position welding.
 */

import type { MeshTopologyDiagnostics } from '../core/mesh'

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
const LARGE_WELD_VERTEX_THRESHOLD = 65_536
const RADIX_BITS = 16
const RADIX_SIZE = 1 << RADIX_BITS
const RADIX_MASK = RADIX_SIZE - 1
const SMALL_EDGE_SORT_THRESHOLD = 65_536

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

  const parent = new Uint32Array(vertexCount)
  for (let i = 0; i < vertexCount; i++) parent[i] = i

  if (mergeFrom && mergeTo) {
    for (let i = 0; i < mergeFrom.length; i++) {
      const from = mergeFrom[i]
      const to = mergeTo[i]
      validateVertexIndex(from, vertexCount, 'mergeFromVert')
      validateVertexIndex(to, vertexCount, 'mergeToVert')
      union(parent, from, to)
    }
  }

  const weldCoincident = options.weldCoincidentVertices ?? mergeFrom === undefined
  if (weldCoincident) weldExactPositions(vertices, parent)

  // Fully compress roots so representative IDs and output ordering are stable.
  for (let i = 0; i < vertexCount; i++) parent[i] = find(parent, i)

  // Store one compact occurrence for each triangle edge. A JS Map with an
  // object per unique edge becomes the dominant memory cost on production-size
  // meshes; these buffers have a fixed upper bound of 20 bytes per occurrence
  // (endpoints, normal storage and radix order) and avoid per-object overhead.
  const maxEdgeOccurrences = triangleIndices.length
  const edgeA = new Uint32Array(maxEdgeOccurrences)
  const edgeB = new Uint32Array(maxEdgeOccurrences)
  const faceNormals = new Float32Array(triangleIndices.length)
  const occurrenceOrder = new Uint32Array(maxEdgeOccurrences)
  let edgeCount = 0
  let degenerate = 0

  for (let i = 0; i < triangleIndices.length; i += 3) {
    const i0 = triangleIndices[i]
    const i1 = triangleIndices[i + 1]
    const i2 = triangleIndices[i + 2]
    const v0 = parent[i0]
    const v1 = parent[i1]
    const v2 = parent[i2]

    if (v0 === v1 || v1 === v2 || v2 === v0) {
      degenerate++
      continue
    }

    const p0 = i0 * VERTEX_STRIDE
    const p1 = i1 * VERTEX_STRIDE
    const p2 = i2 * VERTEX_STRIDE
    const e10x = vertices[p1] - vertices[p0]
    const e10y = vertices[p1 + 1] - vertices[p0 + 1]
    const e10z = vertices[p1 + 2] - vertices[p0 + 2]
    const e20x = vertices[p2] - vertices[p0]
    const e20y = vertices[p2 + 1] - vertices[p0 + 1]
    const e20z = vertices[p2 + 2] - vertices[p0 + 2]
    const crossX = e10y * e20z - e10z * e20y
    const crossY = e10z * e20x - e10x * e20z
    const crossZ = e10x * e20y - e10y * e20x
    const twiceArea = Math.hypot(crossX, crossY, crossZ)
    const edgeScale = (
      e10x * e10x + e10y * e10y + e10z * e10z
      + e20x * e20x + e20y * e20y + e20z * e20z
    )
    const areaEpsilon = edgeScale * Number.EPSILON * 16

    if (!Number.isFinite(twiceArea) || twiceArea <= areaEpsilon) {
      degenerate++
      continue
    }

    const nx = crossX / twiceArea
    const ny = crossY / twiceArea
    const nz = crossZ / twiceArea
    faceNormals[i] = nx
    faceNormals[i + 1] = ny
    faceNormals[i + 2] = nz
    edgeCount = addEdgeOccurrence(edgeA, edgeB, occurrenceOrder, edgeCount, i, v0, v1)
    edgeCount = addEdgeOccurrence(edgeA, edgeB, occurrenceOrder, edgeCount, i + 1, v1, v2)
    edgeCount = addEdgeOccurrence(edgeA, edgeB, occurrenceOrder, edgeCount, i + 2, v2, v0)
  }

  const requestedCreaseAngle = options.creaseAngleDegrees ?? DEFAULT_CREASE_ANGLE_DEGREES
  if (!Number.isFinite(requestedCreaseAngle)) {
    throw new RangeError('creaseAngleDegrees must be finite')
  }
  const creaseAngle = clamp(requestedCreaseAngle, 0, 180)
  const creaseDotThreshold = Math.cos(creaseAngle * Math.PI / 180)
  const sortedOccurrences = radixSortEdgeOccurrences(edgeA, edgeB, occurrenceOrder, edgeCount)
  let boundary = 0
  let crease = 0
  let nonManifold = 0
  let outputEdgeCount = 0

  let groupStart = 0
  while (groupStart < edgeCount) {
    const firstOccurrence = sortedOccurrences[groupStart]
    const a = edgeA[firstOccurrence]
    const b = edgeB[firstOccurrence]
    let groupEnd = groupStart + 1
    while (groupEnd < edgeCount) {
      const occurrence = sortedOccurrences[groupEnd]
      if (edgeA[occurrence] !== a || edgeB[occurrence] !== b) break
      groupEnd++
    }
    const kind = classifyEdgeGroup(
      sortedOccurrences,
      groupStart,
      groupEnd,
      faceNormals,
      creaseDotThreshold,
    )
    if (kind !== 0) outputEdgeCount++
    if (kind === 1) boundary++
    else if (kind === 2) crease++
    else if (kind === 3) nonManifold++
    groupStart = groupEnd
  }

  // Allocate the exact line-list size. A second linear group scan is cheaper
  // than keeping a worst-case JS number array or copying an oversized buffer.
  const output = new Uint32Array(outputEdgeCount * 2)
  let outputOffset = 0
  groupStart = 0
  while (groupStart < edgeCount) {
    const firstOccurrence = sortedOccurrences[groupStart]
    const a = edgeA[firstOccurrence]
    const b = edgeB[firstOccurrence]
    let groupEnd = groupStart + 1
    while (groupEnd < edgeCount) {
      const occurrence = sortedOccurrences[groupEnd]
      if (edgeA[occurrence] !== a || edgeB[occurrence] !== b) break
      groupEnd++
    }
    if (classifyEdgeGroup(sortedOccurrences, groupStart, groupEnd, faceNormals, creaseDotThreshold) !== 0) {
      output[outputOffset++] = a
      output[outputOffset++] = b
    }
    groupStart = groupEnd
  }

  return {
    indices: output,
    diagnostics: { boundary, crease, nonManifold, degenerate },
  }
}

function addEdgeOccurrence(
  edgeA: Uint32Array,
  edgeB: Uint32Array,
  occurrenceOrder: Uint32Array,
  edgeCount: number,
  occurrence: number,
  first: number,
  second: number,
): number {
  edgeA[occurrence] = Math.min(first, second)
  edgeB[occurrence] = Math.max(first, second)
  occurrenceOrder[edgeCount] = occurrence
  return edgeCount + 1
}

/** Stable LSD radix sort by the (a, b) Uint32 pair. */
function radixSortEdgeOccurrences(
  edgeA: Uint32Array,
  edgeB: Uint32Array,
  occurrenceOrder: Uint32Array,
  edgeCount: number,
): Uint32Array {
  let current: Uint32Array = occurrenceOrder.subarray(0, edgeCount)
  if (edgeCount < 2) return current
  let next: Uint32Array = new Uint32Array(edgeCount)
  // Tiny bodies otherwise allocate and scan 65,536 buckets for just a handful
  // of edges. Eight-bit digits need more stable passes but only 1 KiB of
  // counters; keep the existing four-pass path for larger occurrence streams.
  const radixBits = edgeCount < SMALL_EDGE_SORT_THRESHOLD ? 8 : RADIX_BITS
  const radixSize = 1 << radixBits
  const radixMask = radixSize - 1
  const counts = new Uint32Array(radixSize)

  // b is the secondary key and therefore sorted first.
  for (const values of [edgeB, edgeA]) {
    for (let shift = 0; shift < 32; shift += radixBits) {
      counts.fill(0)
      for (let index = 0; index < edgeCount; index++) {
        counts[(values[current[index]] >>> shift) & radixMask]++
      }
      let offset = 0
      for (let digit = 0; digit < radixSize; digit++) {
        const count = counts[digit]
        counts[digit] = offset
        offset += count
      }
      for (let index = 0; index < edgeCount; index++) {
        const occurrence = current[index]
        const digit = (values[occurrence] >>> shift) & radixMask
        next[counts[digit]++] = occurrence
      }
      const swap = current
      current = next
      next = swap
    }
  }
  return current
}

/** 0 = hidden, 1 = boundary, 2 = crease, 3 = non-manifold. */
function classifyEdgeGroup(
  sortedOccurrences: Uint32Array,
  groupStart: number,
  groupEnd: number,
  faceNormals: Float32Array,
  creaseDotThreshold: number,
): 0 | 1 | 2 | 3 {
  const incidentFaces = groupEnd - groupStart
  if (incidentFaces === 1) return 1
  if (incidentFaces > 2) return 3

  const firstFace = sortedOccurrences[groupStart] - (sortedOccurrences[groupStart] % 3)
  const secondFace = sortedOccurrences[groupStart + 1] - (sortedOccurrences[groupStart + 1] % 3)
  const dot = clamp(
    faceNormals[firstFace] * faceNormals[secondFace]
      + faceNormals[firstFace + 1] * faceNormals[secondFace + 1]
      + faceNormals[firstFace + 2] * faceNormals[secondFace + 2],
    -1,
    1,
  )
  // A tiny margin prevents Float32 noise from turning a nominally coplanar
  // triangulation diagonal into an edge. The comparison intentionally means
  // "angle strictly greater than threshold".
  return dot < creaseDotThreshold - 1e-7 ? 2 : 0
}

function weldExactPositions(vertices: Float32Array, parent: Uint32Array) {
  if (parent.length >= LARGE_WELD_VERTEX_THRESHOLD) {
    weldExactPositionsTyped(vertices, parent)
    return
  }
  const firstAtPosition = new Map<string, number>()
  for (let vertex = 0; vertex < parent.length; vertex++) {
    const offset = vertex * VERTEX_STRIDE
    const x = vertices[offset]
    const y = vertices[offset + 1]
    const z = vertices[offset + 2]
    // Invalid coordinates are handled as degenerate faces. Do not let all NaN
    // vertices collapse into one accidental topological vertex.
    if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) continue
    const key = `${normaliseZero(x)},${normaliseZero(y)},${normaliseZero(z)}`
    const first = firstAtPosition.get(key)
    if (first === undefined) firstAtPosition.set(key, vertex)
    else union(parent, first, vertex)
  }
}

/**
 * Exact-position welding for large meshes without one JS string and Map entry
 * per vertex. Six stable 16-bit radix passes group identical Float32 xyz bits;
 * -0 is canonicalized to +0 and non-finite vertices remain unwelded.
 */
function weldExactPositionsTyped(vertices: Float32Array, parent: Uint32Array) {
  const vertexCount = parent.length
  let current = new Uint32Array(vertexCount)
  let next = new Uint32Array(vertexCount)
  for (let vertex = 0; vertex < vertexCount; vertex++) current[vertex] = vertex

  const bits = new Uint32Array(vertices.buffer, vertices.byteOffset, vertices.length)
  const counts = new Uint32Array(RADIX_SIZE)
  // z, y, x makes x the primary key after stable LSD sorting.
  for (const coordinate of [2, 1, 0]) {
    for (let shift = 0; shift < 32; shift += RADIX_BITS) {
      counts.fill(0)
      for (let index = 0; index < vertexCount; index++) {
        const value = canonicalFloatBits(bits[current[index] * VERTEX_STRIDE + coordinate])
        counts[(value >>> shift) & RADIX_MASK]++
      }
      let offset = 0
      for (let digit = 0; digit < RADIX_SIZE; digit++) {
        const count = counts[digit]
        counts[digit] = offset
        offset += count
      }
      for (let index = 0; index < vertexCount; index++) {
        const vertex = current[index]
        const value = canonicalFloatBits(bits[vertex * VERTEX_STRIDE + coordinate])
        next[counts[(value >>> shift) & RADIX_MASK]++] = vertex
      }
      const swap = current
      current = next
      next = swap
    }
  }

  let first = current[0]
  for (let index = 1; index < vertexCount; index++) {
    const candidate = current[index]
    if (sameFinitePosition(vertices, first, candidate)) union(parent, first, candidate)
    else first = candidate
  }
}

function sameFinitePosition(vertices: Float32Array, first: number, second: number) {
  const firstOffset = first * VERTEX_STRIDE
  const secondOffset = second * VERTEX_STRIDE
  const x = vertices[firstOffset]
  const y = vertices[firstOffset + 1]
  const z = vertices[firstOffset + 2]
  return Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(z)
    && x === vertices[secondOffset]
    && y === vertices[secondOffset + 1]
    && z === vertices[secondOffset + 2]
}

function canonicalFloatBits(value: number) {
  return (value & 0x7fff_ffff) === 0 ? 0 : value
}

function find(parent: Uint32Array, value: number): number {
  let root = value
  while (parent[root] !== root) root = parent[root]
  while (parent[value] !== value) {
    const next = parent[value]
    parent[value] = root
    value = next
  }
  return root
}

function union(parent: Uint32Array, first: number, second: number) {
  const firstRoot = find(parent, first)
  const secondRoot = find(parent, second)
  if (firstRoot === secondRoot) return
  // Keeping the smallest property index as root makes line indices stable.
  if (firstRoot < secondRoot) parent[secondRoot] = firstRoot
  else parent[firstRoot] = secondRoot
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

function normaliseZero(value: number) {
  return Object.is(value, -0) ? 0 : value
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value))
}
