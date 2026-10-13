/**
 * WebGPU Compute Semantic Edges & Crease Angle Detection.
 *
 * Classifies edges into boundary edges and sharp crease edges in parallel on GPU:
 * 1. Computes facet normals for incident triangles.
 * 2. Evaluates dihedral dot product in parallel workgroups.
 * 3. Emits line vertex index pairs [v0, v1] directly into VRAM for wireframe/edge rendering.
 */

import { runGpuCompute } from './webgpuCompute'

export interface SemanticEdgesOptions {
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
export { SEMANTIC_EDGES_WGSL }


/**
 * Builds edge adjacency from triangle index buffer.
 * Output is packed as [v0, v1, tri0, tri1] (stride 4 uint32s).
 */
const EDGE_KEY_STRIDE = 67108864 // 2^26

export function buildMeshEdgeAdjacency(indices: Uint32Array): Uint32Array {
  const triCount = (indices.length / 3) | 0
  // Map packed edge key -> offset in flat buffer (stride 4 uint32s)
  const edgeIndexMap = new Map<number, number>()
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
      const key = minV * EDGE_KEY_STRIDE + maxV

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


function buildParamsUniform(
  totalEdges: number,
  totalTriangles: number,
  threshold: number,
  strideFloats: number,
  posOffset: number,
  maxOutputEdges: number
): ArrayBuffer {
  const buffer = new ArrayBuffer(32)
  const view = new DataView(buffer)

  view.setUint32(0, totalEdges, true)
  view.setUint32(4, totalTriangles, true)
  view.setFloat32(8, threshold, true)
  view.setUint32(12, strideFloats, true)

  view.setUint32(16, posOffset, true)
  view.setUint32(20, maxOutputEdges, true)
  view.setUint32(24, 0, true)
  view.setUint32(28, 0, true)

  return buffer
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
    return referenceSemanticEdges(vertices, indices, opts)
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
    const outIndicesBuffer = new Uint32Array(maxEdges * 2)

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
            { binding: 5, data: outIndicesBuffer, output: true, outputBytes: maxEdges * 2 * 4 },
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

  return referenceSemanticEdges(vertices, indices, opts)
}

/**
 * CPU reference evaluator for semantic edge extraction.
 */
export function referenceSemanticEdges(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SemanticEdgesOptions = {}
): SemanticEdgesResult {
  const edgeAdjacency = buildMeshEdgeAdjacency(indices)
  const totalEdges = edgeAdjacency.length / 4

  const creaseAngle = opts.creaseAngleDegrees ?? 30.0
  const threshold = opts.creaseDotThreshold ?? Math.cos((creaseAngle * Math.PI) / 180.0)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0

  function getPos(vIdx: number): [number, number, number] {
    const base = vIdx * strideFloats + posOffset
    return [vertices[base + 0]!, vertices[base + 1]!, vertices[base + 2]!]
  }

  function getTriNormal(tIdx: number): [number, number, number] {
    const i0 = indices[tIdx * 3 + 0]!
    const i1 = indices[tIdx * 3 + 1]!
    const i2 = indices[tIdx * 3 + 2]!

    const p0 = getPos(i0)
    const p1 = getPos(i1)
    const p2 = getPos(i2)

    const d1x = p1[0] - p0[0], d1y = p1[1] - p0[1], d1z = p1[2] - p0[2]
    const d2x = p2[0] - p0[0], d2y = p2[1] - p0[1], d2z = p2[2] - p0[2]

    let nx = d1y * d2z - d1z * d2y
    let ny = d1z * d2x - d1x * d2z
    let nz = d1x * d2y - d1y * d2x

    const len = Math.hypot(nx, ny, nz)
    if (len > 1e-7) {
      nx /= len; ny /= len; nz /= len
    } else {
      nx = 0; ny = 0; nz = 1
    }
    return [nx, ny, nz]
  }

  const resultEdges: number[] = []

  for (let e = 0; e < totalEdges; e++) {
    const v0 = edgeAdjacency[e * 4 + 0]!
    const v1 = edgeAdjacency[e * 4 + 1]!
    const tri0 = edgeAdjacency[e * 4 + 2]!
    const tri1 = edgeAdjacency[e * 4 + 3]!

    if (tri1 === 0xffffffff) {
      // Boundary edge
      resultEdges.push(v0, v1)
    } else {
      // Manifold interior edge: check dihedral angle
      const n0 = getTriNormal(tri0)
      const n1 = getTriNormal(tri1)
      const dot = n0[0] * n1[0] + n0[1] * n1[1] + n0[2] * n1[2]

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
