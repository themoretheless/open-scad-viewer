/**
 * WebGPU Compute Semantic Edges & Crease Angle Detection.
 *
 * Classifies edges into boundary edges and sharp crease edges in parallel on GPU:
 * 1. Computes facet normals for incident triangles.
 * 2. Evaluates dihedral dot product in parallel workgroups.
 * 3. Emits line vertex index pairs [v0, v1] directly into VRAM for wireframe/edge rendering.
 */

import { runGpuCompute } from './webgpuCompute'
import { forEachChunkedAsync, type CooperativeAsyncOptions } from './cooperativeAsync'

export interface SemanticEdgesOptions extends CooperativeAsyncOptions {
  creaseAngleDegrees?: number
  creaseDotThreshold?: number
  strideFloats?: number
  posOffset?: number
  maxEdges?: number
}

export interface SemanticEdgesResult {
  /** Line indices: flat pairs [v0, v1, v0, v1, ...] (stride 2) */
  indices: Uint32Array
  edgeCount: number
}

import SEMANTIC_EDGES_WGSL from '../../crates/compute-core/shaders/semantic_edges.wgsl?raw'
import { packUndirectedEdgeKey } from './math3d'
export { SEMANTIC_EDGES_WGSL }

/**
 * Builds edge adjacency from triangle index buffer.
 * Output is packed as [v0, v1, tri0, tri1] (stride 4 uint32s).
 */
export function buildMeshEdgeAdjacency(indices: Uint32Array): Uint32Array {
  const triCount = (indices.length / 3) | 0
  // Map packed edge key -> offset in flat buffer (stride 4 uint32s)
  const edgeIndexMap = new Map<number | string, number>()
  const buf = new Uint32Array(triCount * 3 * 4)
  let ptr = 0

  for (let t = 0; t < triCount; t++) {
    const base = t * 3
    const i0 = indices[base]!
    const i1 = indices[base + 1]!
    const i2 = indices[base + 2]!

    for (let e = 0; e < 3; e++) {
      const va = e === 0 ? i0 : e === 1 ? i1 : i2
      const vb = e === 0 ? i1 : e === 1 ? i2 : i0
      const minV = va < vb ? va : vb
      const maxV = va < vb ? vb : va
      const key = packUndirectedEdgeKey(minV, maxV)

      const existingOff = edgeIndexMap.get(key)
      if (existingOff === undefined) {
        edgeIndexMap.set(key, ptr)
        buf[ptr] = minV
        buf[ptr + 1] = maxV
        buf[ptr + 2] = t
        buf[ptr + 3] = 0xffffffff
        ptr += 4
      } else {
        buf[existingOff + 3] = t
      }
    }
  }

  return buf.slice(0, ptr)
}


const semanticEdgesUniformBuffer = new ArrayBuffer(32)
const semanticEdgesUniformView = new DataView(semanticEdgesUniformBuffer)

function buildParamsUniform(
  totalEdges: number,
  totalTriangles: number,
  threshold: number,
  strideFloats: number,
  posOffset: number,
  maxOutputEdges: number
): ArrayBuffer {
  semanticEdgesUniformView.setUint32(0, totalEdges, true)
  semanticEdgesUniformView.setUint32(4, totalTriangles, true)
  semanticEdgesUniformView.setFloat32(8, threshold, true)
  semanticEdgesUniformView.setUint32(12, strideFloats, true)

  semanticEdgesUniformView.setUint32(16, posOffset, true)
  semanticEdgesUniformView.setUint32(20, maxOutputEdges, true)
  semanticEdgesUniformView.setUint32(24, 0, true)
  semanticEdgesUniformView.setUint32(28, 0, true)

  return semanticEdgesUniformBuffer
}

/**
 * Extracts semantic and crease edges using WebGPU Compute.
 */
export async function runGpuSemanticEdges(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SemanticEdgesOptions = {}
): Promise<SemanticEdgesResult> {
  const edgeAdjacency = buildMeshEdgeAdjacency(indices)
  const totalEdges = edgeAdjacency.length / 4
  const totalTriangles = Math.floor(indices.length / 3)

  const creaseAngle = opts.creaseAngleDegrees ?? 30.0
  const threshold = opts.creaseDotThreshold ?? Math.cos((creaseAngle * Math.PI) / 180.0)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0
  const maxEdges = opts.maxEdges ?? totalEdges

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return totalEdges >= 1024
      ? referenceSemanticEdgesAsync(vertices, indices, opts, edgeAdjacency)
      : referenceSemanticEdges(vertices, indices, opts, edgeAdjacency)
  }

  try {
    const uniformData = buildParamsUniform(
      totalEdges,
      totalTriangles,
      threshold,
      strideFloats,
      posOffset,
      maxEdges
    )

    const counterInit = new Uint32Array([0])

    const results = await runGpuCompute({
      wgsl: SEMANTIC_EDGES_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformData, uniform: true },
            { binding: 1, data: vertices },
            { binding: 2, data: indices },
            { binding: 3, data: edgeAdjacency },
            { binding: 4, data: counterInit, output: true, outputBytes: 4 },
            { binding: 5, data: new Uint32Array(0), output: true, outputBytes: maxEdges * 2 * 4 },
          ],
          outputBytes: maxEdges * 2 * 4,
          workgroups: [Math.ceil(totalEdges / 256), 1, 1],
        },
      ],
    })

    const count = Math.min(results[0]?.[0] ?? 0, maxEdges)
    const rawIndices = results[1]
    if (rawIndices) {
      const edgeIndices = new Uint32Array(count * 2)
      for (let i = 0; i < count * 2; i++) {
        edgeIndices[i] = rawIndices[i]!
      }
      return { indices: edgeIndices, edgeCount: count }
    }
  } catch {
    // Fallback if WebGPU compute is not available
  }

  return totalEdges >= 1024
    ? referenceSemanticEdgesAsync(vertices, indices, opts, edgeAdjacency)
    : referenceSemanticEdges(vertices, indices, opts, edgeAdjacency)
}

function computeTriNormalInto(
  tIdx: number,
  vertices: Float32Array,
  indices: Uint32Array,
  strideFloats: number,
  posOffset: number,
  out: Float64Array,
  outOffset: number
): void {
  const b = tIdx * 3
  const b0 = indices[b]! * strideFloats + posOffset
  const b1 = indices[b + 1]! * strideFloats + posOffset
  const b2 = indices[b + 2]! * strideFloats + posOffset

  const p0x = vertices[b0]!, p0y = vertices[b0 + 1]!, p0z = vertices[b0 + 2]!
  const d1x = vertices[b1]! - p0x
  const d1y = vertices[b1 + 1]! - p0y
  const d1z = vertices[b1 + 2]! - p0z
  const d2x = vertices[b2]! - p0x
  const d2y = vertices[b2 + 1]! - p0y
  const d2z = vertices[b2 + 2]! - p0z

  let nx = d1y * d2z - d1z * d2y
  let ny = d1z * d2x - d1x * d2z
  let nz = d1x * d2y - d1y * d2x

  const lenSq = nx * nx + ny * ny + nz * nz
  if (lenSq > 1e-14) {
    const invLen = 1.0 / Math.sqrt(lenSq)
    nx *= invLen; ny *= invLen; nz *= invLen
  } else {
    nx = 0; ny = 0; nz = 1
  }
  out[outOffset] = nx
  out[outOffset + 1] = ny
  out[outOffset + 2] = nz
}

/**
 * CPU reference evaluator for semantic edge extraction.
 */
export function referenceSemanticEdges(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SemanticEdgesOptions = {},
  precomputedAdjacency?: Uint32Array
): SemanticEdgesResult {
  const edgeAdjacency = precomputedAdjacency ?? buildMeshEdgeAdjacency(indices)
  const totalEdges = edgeAdjacency.length / 4

  const creaseAngle = opts.creaseAngleDegrees ?? 30.0
  const threshold = opts.creaseDotThreshold ?? Math.cos((creaseAngle * Math.PI) / 180.0)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0

  const nScratch = new Float64Array(6)
  const resultEdges: number[] = []

  for (let e = 0; e < totalEdges; e++) {
    const base = e * 4
    const v0 = edgeAdjacency[base]!
    const v1 = edgeAdjacency[base + 1]!
    const tri0 = edgeAdjacency[base + 2]!
    const tri1 = edgeAdjacency[base + 3]!

    if (tri1 === 0xffffffff) {
      // Boundary edge
      resultEdges.push(v0, v1)
    } else {
      // Manifold interior edge: check dihedral angle
      computeTriNormalInto(tri0, vertices, indices, strideFloats, posOffset, nScratch, 0)
      computeTriNormalInto(tri1, vertices, indices, strideFloats, posOffset, nScratch, 3)
      const dot =
        nScratch[0]! * nScratch[3]! +
        nScratch[1]! * nScratch[4]! +
        nScratch[2]! * nScratch[5]!

      if (dot < threshold) {
        resultEdges.push(v0, v1)
      }
    }
  }

  return {
    indices: new Uint32Array(resultEdges),
    edgeCount: resultEdges.length / 2,
  }
}

/**
 * Cooperative non-blocking async CPU reference evaluator for semantic edge extraction.
 */
export async function referenceSemanticEdgesAsync(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SemanticEdgesOptions = {},
  precomputedAdjacency?: Uint32Array
): Promise<SemanticEdgesResult> {
  const edgeAdjacency = precomputedAdjacency ?? buildMeshEdgeAdjacency(indices)
  const totalEdges = edgeAdjacency.length / 4

  const creaseAngle = opts.creaseAngleDegrees ?? 30.0
  const threshold = opts.creaseDotThreshold ?? Math.cos((creaseAngle * Math.PI) / 180.0)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0

  const nScratch = new Float64Array(6)
  const resultEdges: number[] = []

  await forEachChunkedAsync(
    totalEdges,
    e => {
      const base = e * 4
      const v0 = edgeAdjacency[base]!
      const v1 = edgeAdjacency[base + 1]!
      const tri0 = edgeAdjacency[base + 2]!
      const tri1 = edgeAdjacency[base + 3]!

      if (tri1 === 0xffffffff) {
        resultEdges.push(v0, v1)
      } else {
        computeTriNormalInto(tri0, vertices, indices, strideFloats, posOffset, nScratch, 0)
        computeTriNormalInto(tri1, vertices, indices, strideFloats, posOffset, nScratch, 3)
        const dot =
          nScratch[0]! * nScratch[3]! +
          nScratch[1]! * nScratch[4]! +
          nScratch[2]! * nScratch[5]!
        if (dot < threshold) {
          resultEdges.push(v0, v1)
        }
      }
    },
    opts
  )

  return {
    indices: new Uint32Array(resultEdges),
    edgeCount: resultEdges.length / 2,
  }
}
