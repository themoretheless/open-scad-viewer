/**
 * WebGPU Compute Multi-Plane Mesh Slicer & Cross-Section Contour Extractor.
 *
 * Slices 3D triangle meshes across 1 or N parallel planes in a single GPU Compute pass:
 * 1. O(1) per-triangle layer interval culling [l_start .. l_end].
 * 2. Exact edge-plane linear interpolation.
 * 3. Canonical counter-clockwise contour winding around the slicing plane normal.
 * 4. Exact cross-sectional area evaluation via Stokes' / Green's theorem.
 */

import { runGpuCompute } from './webgpuCompute'

export interface SlicerOptions {
  /** Slicing plane normal vector (default: [0, 0, 1] for Z-slicing) */
  planeNormal?: [number, number, number]
  /** Distance d of the first slicing plane (n . p = d) */
  firstLayerDistance: number
  /** Step between consecutive parallel layers (default: 0.2) */
  layerStep?: number
  /** Number of parallel layers to slice simultaneously (default: 1) */
  numLayers?: number
  /** Float stride per vertex in vertices buffer (default: 3) */
  strideFloats?: number
  /** Float offset of XYZ position in vertex stride (default: 0) */
  posOffset?: number
  /** Maximum segments to allocate in output buffer */
  maxSegments?: number
}

export interface SliceSegment {
  p0: [number, number, number]
  p1: [number, number, number]
  layerIndex: number
  triangleIndex: number
}

export interface SlicerResult {
  /** Raw packed segments: 8 floats per segment [x0, y0, z0, layerIdx, x1, y1, z1, triIdx] */
  rawSegments: Float32Array
  /** Parsed structured segments */
  segments: SliceSegment[]
  /** Number of emitted contour segments */
  segmentCount: number
}

export const MESH_SLICER_WGSL = /* wgsl */`
struct SlicerParams {
  total_triangles: u32,
  num_layers: u32,
  stride_floats: u32,
  pos_offset: u32,
  plane_nx: f32,
  plane_ny: f32,
  plane_nz: f32,
  first_layer_d: f32,
  layer_step: f32,
  max_segments: u32,
  _pad0: u32,
  _pad1: u32,
};

@group(0) @binding(0) var<uniform> params: SlicerParams;
@group(0) @binding(1) var<storage, read> vertices: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> segment_counter: array<atomic<u32>, 1>;
@group(0) @binding(4) var<storage, read_write> out_segments: array<f32>;

fn get_pos(v_idx: u32) -> vec3<f32> {
  let base = v_idx * params.stride_floats + params.pos_offset;
  return vec3<f32>(
    vertices[base + 0u],
    vertices[base + 1u],
    vertices[base + 2u]
  );
}

fn interp_edge(p0: vec3<f32>, s0: f32, p1: vec3<f32>, s1: f32) -> vec3<f32> {
  let denom = s1 - s0;
  if (abs(denom) < 1e-9) {
    return (p0 + p1) * 0.5;
  }
  let t = clamp(-s0 / denom, 0.0, 1.0);
  return p0 + t * (p1 - p0);
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let tri_idx = id.x;
  if (tri_idx >= params.total_triangles) {
    return;
  }

  let i0 = indices[tri_idx * 3u + 0u];
  let i1 = indices[tri_idx * 3u + 1u];
  let i2 = indices[tri_idx * 3u + 2u];

  let p0 = get_pos(i0);
  let p1 = get_pos(i1);
  let p2 = get_pos(i2);

  let plane_n = normalize(vec3<f32>(params.plane_nx, params.plane_ny, params.plane_nz));

  let d0 = dot(plane_n, p0);
  let d1 = dot(plane_n, p1);
  let d2 = dot(plane_n, p2);

  let d_min = min(d0, min(d1, d2));
  let d_max = max(d0, max(d1, d2));

  var l_start = 0;
  var l_end = i32(params.num_layers) - 1;

  if (params.layer_step > 1e-7 && params.num_layers > 1u) {
    l_start = max(0, i32(ceil((d_min - params.first_layer_d) / params.layer_step)));
    l_end = min(i32(params.num_layers) - 1, i32(floor((d_max - params.first_layer_d) / params.layer_step)));
  } else {
    if (params.first_layer_d < d_min || params.first_layer_d > d_max) {
      return;
    }
    l_start = 0;
    l_end = 0;
  }

  if (l_start > l_end) {
    return;
  }

  let tri_n = cross(p1 - p0, p2 - p0);
  let ccw_dir = cross(plane_n, tri_n);

  let pts = array<vec3<f32>, 3>(p0, p1, p2);

  for (var layer = l_start; layer <= l_end; layer++) {
    let plane_d = params.first_layer_d + f32(layer) * params.layer_step;

    let s0 = d0 - plane_d;
    let s1 = d1 - plane_d;
    let s2 = d2 - plane_d;
    let s_arr = array<f32, 3>(s0, s1, s2);

    var neg_idx: array<u32, 3>;
    var pos_idx: array<u32, 3>;
    var neg_len = 0u;
    var pos_len = 0u;

    for (var k = 0u; k < 3u; k++) {
      if (s_arr[k] < 0.0) {
        neg_idx[neg_len] = k;
        neg_len++;
      } else {
        pos_idx[pos_len] = k;
        pos_len++;
      }
    }

    if (neg_len == 0u || pos_len == 0u) {
      continue;
    }

    var solo = 0u;
    var pair0 = 0u;
    var pair1 = 0u;

    if (neg_len == 1u) {
      solo = neg_idx[0];
      pair0 = pos_idx[0];
      pair1 = pos_idx[1];
    } else {
      solo = pos_idx[0];
      pair0 = neg_idx[0];
      pair1 = neg_idx[1];
    }

    var q1 = interp_edge(pts[solo], s_arr[solo], pts[pair0], s_arr[pair0]);
    var q2 = interp_edge(pts[solo], s_arr[solo], pts[pair1], s_arr[pair1]);

    if (dot(ccw_dir, q2 - q1) < 0.0) {
      let tmp = q1;
      q1 = q2;
      q2 = tmp;
    }

    if (length(q2 - q1) < 1e-7) {
      continue;
    }

    let slot = atomicAdd(&segment_counter[0], 1u);
    if (slot < params.max_segments) {
      let off = slot * 8u;
      out_segments[off + 0u] = q1.x;
      out_segments[off + 1u] = q1.y;
      out_segments[off + 2u] = q1.z;
      out_segments[off + 3u] = f32(layer);
      out_segments[off + 4u] = q2.x;
      out_segments[off + 5u] = q2.y;
      out_segments[off + 6u] = q2.z;
      out_segments[off + 7u] = f32(tri_idx);
    }
  }
}
`

function normalizeVec3(v: [number, number, number]): [number, number, number] {
  const len = Math.hypot(v[0], v[1], v[2])
  if (len < 1e-12) return [0, 0, 1]
  return [v[0] / len, v[1] / len, v[2] / len]
}

function buildSlicerUniform(
  totalTriangles: number,
  numLayers: number,
  strideFloats: number,
  posOffset: number,
  planeNormal: [number, number, number],
  firstLayerDistance: number,
  layerStep: number,
  maxSegments: number
): ArrayBuffer {
  const buffer = new ArrayBuffer(48)
  const view = new DataView(buffer)

  const n = normalizeVec3(planeNormal)

  view.setUint32(0, totalTriangles, true)
  view.setUint32(4, numLayers, true)
  view.setUint32(8, strideFloats, true)
  view.setUint32(12, posOffset, true)

  view.setFloat32(16, n[0], true)
  view.setFloat32(20, n[1], true)
  view.setFloat32(24, n[2], true)
  view.setFloat32(28, firstLayerDistance, true)

  view.setFloat32(32, layerStep, true)
  view.setUint32(36, maxSegments, true)
  view.setUint32(40, 0, true)
  view.setUint32(44, 0, true)

  return buffer
}

function unpackSegments(raw: Float32Array, count: number): SliceSegment[] {
  const segments: SliceSegment[] = []
  for (let i = 0; i < count; i++) {
    const off = i * 8
    segments.push({
      p0: [raw[off + 0]!, raw[off + 1]!, raw[off + 2]!],
      layerIndex: Math.round(raw[off + 3]!),
      p1: [raw[off + 4]!, raw[off + 5]!, raw[off + 6]!],
      triangleIndex: Math.round(raw[off + 7]!),
    })
  }
  return segments
}

/**
 * Slices a mesh across 1 or N parallel planes on WebGPU Compute.
 */
export async function runGpuMeshSlicer(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SlicerOptions
): Promise<SlicerResult> {
  const totalTriangles = Math.floor(indices.length / 3)
  const numLayers = Math.max(1, opts.numLayers ?? 1)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0
  const planeNormal = opts.planeNormal ?? [0, 0, 1]
  const layerStep = opts.layerStep ?? 0.2
  const maxSegments = opts.maxSegments ?? Math.max(1024, totalTriangles * Math.min(numLayers, 16))

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return referenceMeshSlicer(vertices, indices, opts)
  }

  try {
    const uniformData = buildSlicerUniform(
      totalTriangles,
      numLayers,
      strideFloats,
      posOffset,
      planeNormal,
      opts.firstLayerDistance,
      layerStep,
      maxSegments
    )

    const counterInit = new Uint32Array([0])
    const segOutBytes = maxSegments * 8 * 4

    const results = await runGpuCompute({
      wgsl: MESH_SLICER_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformData, uniform: true },
            { binding: 1, data: vertices },
            { binding: 2, data: indices },
            { binding: 3, data: counterInit, output: true, outputBytes: 4 },
            { binding: 4, data: new Float32Array(maxSegments * 8), output: true, outputBytes: segOutBytes },
          ],
          outputBytes: segOutBytes,
          workgroups: [Math.ceil(totalTriangles / 256), 1, 1],
        },
      ],
    })

    const count = Math.min(results[0]?.[0] ?? 0, maxSegments)
    const rawOut = results[1]
    if (rawOut) {
      const sliced = rawOut.slice(0, count * 8)
      return {
        rawSegments: sliced,
        segments: unpackSegments(sliced, count),
        segmentCount: count,
      }
    }
  } catch {
    // Fallback to CPU reference when WebGPU adapter is unavailable
  }

  return referenceMeshSlicer(vertices, indices, opts)
}

/**
 * Exact CPU reference implementation of multi-plane mesh slicing.
 */
export function referenceMeshSlicer(
  vertices: Float32Array,
  indices: Uint32Array,
  opts: SlicerOptions
): SlicerResult {
  const totalTriangles = Math.floor(indices.length / 3)
  const numLayers = Math.max(1, opts.numLayers ?? 1)
  const strideFloats = opts.strideFloats ?? 3
  const posOffset = opts.posOffset ?? 0
  const planeN = normalizeVec3(opts.planeNormal ?? [0, 0, 1])
  const firstD = opts.firstLayerDistance
  const layerStep = opts.layerStep ?? 0.2

  function getPos(vIdx: number): [number, number, number] {
    const base = vIdx * strideFloats + posOffset
    return [vertices[base + 0]!, vertices[base + 1]!, vertices[base + 2]!]
  }

  function dot3(a: [number, number, number], b: [number, number, number]): number {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
  }

  function cross3(a: [number, number, number], b: [number, number, number]): [number, number, number] {
    return [
      a[1] * b[2] - a[2] * b[1],
      a[2] * b[0] - a[0] * b[2],
      a[0] * b[1] - a[1] * b[0],
    ]
  }

  function interp(
    p0: [number, number, number],
    s0: number,
    p1: [number, number, number],
    s1: number
  ): [number, number, number] {
    const denom = s1 - s0
    if (Math.abs(denom) < 1e-9) {
      return [(p0[0] + p1[0]) * 0.5, (p0[1] + p1[1]) * 0.5, (p0[2] + p1[2]) * 0.5]
    }
    const t = Math.max(0, Math.min(1, -s0 / denom))
    return [
      p0[0] + t * (p1[0] - p0[0]),
      p0[1] + t * (p1[1] - p0[1]),
      p0[2] + t * (p1[2] - p0[2]),
    ]
  }

  const rawList: number[] = []
  const segments: SliceSegment[] = []

  for (let t = 0; t < totalTriangles; t++) {
    const i0 = indices[t * 3 + 0]!
    const i1 = indices[t * 3 + 1]!
    const i2 = indices[t * 3 + 2]!

    const p0 = getPos(i0)
    const p1 = getPos(i1)
    const p2 = getPos(i2)

    const d0 = dot3(planeN, p0)
    const d1 = dot3(planeN, p1)
    const d2 = dot3(planeN, p2)

    const dMin = Math.min(d0, d1, d2)
    const dMax = Math.max(d0, d1, d2)

    let lStart = 0
    let lEnd = numLayers - 1

    if (layerStep > 1e-7 && numLayers > 1) {
      lStart = Math.max(0, Math.ceil((dMin - firstD) / layerStep))
      lEnd = Math.min(numLayers - 1, Math.floor((dMax - firstD) / layerStep))
    } else {
      if (firstD < dMin || firstD > dMax) continue
      lStart = 0
      lEnd = 0
    }

    if (lStart > lEnd) continue

    const e1: [number, number, number] = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]]
    const e2: [number, number, number] = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]]
    const triN = cross3(e1, e2)
    const ccwDir = cross3(planeN, triN)

    const pts: [number, number, number][] = [p0, p1, p2]

    for (let layer = lStart; layer <= lEnd; layer++) {
      const planeD = firstD + layer * layerStep
      const sArr = [d0 - planeD, d1 - planeD, d2 - planeD]

      const neg: number[] = []
      const pos: number[] = []
      for (let k = 0; k < 3; k++) {
        if (sArr[k]! < 0) neg.push(k)
        else pos.push(k)
      }

      if (neg.length === 0 || pos.length === 0) continue

      const solo = neg.length === 1 ? neg[0]! : pos[0]!
      const pair0 = neg.length === 1 ? pos[0]! : neg[0]!
      const pair1 = neg.length === 1 ? pos[1]! : neg[1]!

      let q1 = interp(pts[solo]!, sArr[solo]!, pts[pair0]!, sArr[pair0]!)
      let q2 = interp(pts[solo]!, sArr[solo]!, pts[pair1]!, sArr[pair1]!)

      const segDir: [number, number, number] = [q2[0] - q1[0], q2[1] - q1[1], q2[2] - q1[2]]
      if (dot3(ccwDir, segDir) < 0) {
        const tmp = q1
        q1 = q2
        q2 = tmp
      }

      const segLen = Math.hypot(q2[0] - q1[0], q2[1] - q1[1], q2[2] - q1[2])
      if (segLen < 1e-7) continue

      rawList.push(q1[0], q1[1], q1[2], layer, q2[0], q2[1], q2[2], t)
      segments.push({
        p0: q1,
        p1: q2,
        layerIndex: layer,
        triangleIndex: t,
      })
    }
  }

  return {
    rawSegments: new Float32Array(rawList),
    segments,
    segmentCount: segments.length,
  }
}

/**
 * Computes the exact signed 2D cross-sectional area of a slice layer
 * via Green's / Stokes' theorem: Area = 0.5 * sum((q1 x q2) . planeNormal).
 */
export function computeSliceArea(
  segments: readonly SliceSegment[],
  planeNormal: [number, number, number] = [0, 0, 1],
  layerIndex?: number
): number {
  const n = normalizeVec3(planeNormal)
  let doubleArea = 0.0

  for (const seg of segments) {
    if (layerIndex !== undefined && seg.layerIndex !== layerIndex) continue
    const [x1, y1, z1] = seg.p0
    const [x2, y2, z2] = seg.p1

    // Cross product q1 x q2
    const cx = y1 * z2 - z1 * y2
    const cy = z1 * x2 - x1 * z2
    const cz = x1 * y2 - y1 * x2

    doubleArea += cx * n[0] + cy * n[1] + cz * n[2]
  }

  return 0.5 * doubleArea
}

/**
 * Groups emitted segments by their layer index.
 */
export function groupSegmentsByLayer(result: SlicerResult): Map<number, SliceSegment[]> {
  const map = new Map<number, SliceSegment[]>()
  for (const seg of result.segments) {
    let list = map.get(seg.layerIndex)
    if (!list) {
      list = []
      map.set(seg.layerIndex, list)
    }
    list.push(seg)
  }
  return map
}
