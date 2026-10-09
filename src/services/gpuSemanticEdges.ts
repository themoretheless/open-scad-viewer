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

export const SEMANTIC_EDGES_WGSL = /* wgsl */`
struct EdgeParams {
  total_edges: u32,
  total_triangles: u32,
  crease_dot_threshold: f32,
  stride_floats: u32,
  pos_offset: u32,
  max_output_edges: u32,
  _pad1: u32,
  _pad2: u32,
};

struct EdgeInput {
  v0: u32,
  v1: u32,
  tri0: u32,
  tri1: u32,
};

@group(0) @binding(0) var<uniform> params: EdgeParams;
@group(0) @binding(1) var<storage, read> vertices: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read> edges: array<EdgeInput>;
@group(0) @binding(4) var<storage, read_write> edge_counter: array<atomic<u32>, 1>;
@group(0) @binding(5) var<storage, read_write> out_edge_indices: array<u32>;

fn get_vertex_pos(v_idx: u32) -> vec3<f32> {
  let base = v_idx * params.stride_floats + params.pos_offset;
  return vec3<f32>(
    vertices[base + 0u],
    vertices[base + 1u],
    vertices[base + 2u]
  );
}

fn compute_triangle_normal(tri_idx: u32) -> vec3<f32> {
  let i0 = indices[tri_idx * 3u + 0u];
  let i1 = indices[tri_idx * 3u + 1u];
  let i2 = indices[tri_idx * 3u + 2u];

  let p0 = get_vertex_pos(i0);
  let p1 = get_vertex_pos(i1);
  let p2 = get_vertex_pos(i2);

  let d1 = p1 - p0;
  let d2 = p2 - p0;
  let n = cross(d1, d2);
  let len = length(n);
  if (len > 1e-7) {
    return n / len;
  }
  return vec3<f32>(0.0, 0.0, 1.0);
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let edge_idx = id.x;
  if (edge_idx >= params.total_edges) {
    return;
  }

  let edge = edges[edge_idx];
  var is_semantic = false;

  if (edge.tri1 == 0xFFFFFFFFu) {
    // 1. Boundary edge (only 1 incident triangle)
    is_semantic = true;
  } else {
    // 2. Manifold interior edge: inspect dihedral angle
    let n0 = compute_triangle_normal(edge.tri0);
    let n1 = compute_triangle_normal(edge.tri1);
    let dot_prod = dot(n0, n1);

    if (dot_prod < params.crease_dot_threshold) {
      is_semantic = true;
    }
  }

  if (is_semantic) {
    let slot = atomicAdd(&edge_counter[0], 1u);
    if (slot < params.max_output_edges) {
      let out_off = slot * 2u;
      out_edge_indices[out_off + 0u] = edge.v0;
      out_edge_indices[out_off + 1u] = edge.v1;
    }
  }
}
`

/**
 * Builds edge adjacency from triangle index buffer.
 * Output is packed as [v0, v1, tri0, tri1] (stride 4 uint32s).
 */
export function buildMeshEdgeAdjacency(indices: Uint32Array): Uint32Array {
  const triCount = Math.floor(indices.length / 3)
  // Map canonical edge (min, max) -> [tri0, tri1]
  const edgeMap = new Map<bigint, { v0: number; v1: number; tri0: number; tri1: number }>()

  for (let t = 0; t < triCount; t++) {
    const i0 = indices[t * 3 + 0]!
    const i1 = indices[t * 3 + 1]!
    const i2 = indices[t * 3 + 2]!

    const triEdges: [number, number][] = [
      [i0, i1],
      [i1, i2],
      [i2, i0],
    ]

    for (const [va, vb] of triEdges) {
      const minV = Math.min(va, vb)
      const maxV = Math.max(va, vb)
      // Pack 2x 32-bit uints into 64-bit bigint key
      const key = (BigInt(minV) << 32n) | BigInt(maxV)

      const existing = edgeMap.get(key)
      if (!existing) {
        edgeMap.set(key, { v0: minV, v1: maxV, tri0: t, tri1: 0xffffffff })
      } else {
        existing.tri1 = t
      }
    }
  }

  const result = new Uint32Array(edgeMap.size * 4)
  let ptr = 0
  for (const edge of edgeMap.values()) {
    result[ptr++] = edge.v0
    result[ptr++] = edge.v1
    result[ptr++] = edge.tri0
    result[ptr++] = edge.tri1
  }

  return result
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
