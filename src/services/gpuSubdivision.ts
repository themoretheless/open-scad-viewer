/**
 * WebGPU Compute Catmull–Clark Subdivision & Brush Sculpting Accelerator.
 *
 * Accelerates Catmull–Clark cage refinement and interactive sculpting brushes:
 * 1. Face point pass (mode 0), Edge point pass (mode 1), and Vertex point pass (mode 2)
 *    matching `crates/subdivision-core` interior `(F + 2R + (n-3)P)/n` and boundary `(6P + B0 + B1)/8` rules.
 * 2. Brush / Sculpting pass (mode 3) for Grab, Inflate, and Pinch with Smoothstep / Linear / Constant falloff.
 * 3. Deterministic CPU fallback matching `subdivision_core::Cage::subdivide`.
 */

import { runGpuCompute } from './webgpuCompute'
import type { SubdivisionCage, SubdivisionRefined } from './geometry/subdivision'
import type { GeometryBrush } from './geometryEditing'

export const CATMULL_CLARK_WGSL = /* wgsl */`
struct SubdivParams {
  mode: u32,
  num_items: u32,
  brush_falloff: u32,
  brush_kind: u32,
  brush_cx: f32,
  brush_cy: f32,
  brush_cz: f32,
  brush_radius: f32,
  brush_dx: f32,
  brush_dy: f32,
  brush_dz: f32,
  brush_strength: f32,
};

@group(0) @binding(0) var<uniform> params: SubdivParams;
@group(0) @binding(1) var<storage, read> in_positions: array<f32>;
@group(0) @binding(2) var<storage, read> aux_positions: array<f32>;
@group(0) @binding(3) var<storage, read> stencil_meta: array<u32>;
@group(0) @binding(4) var<storage, read> stencil_indices: array<u32>;
@group(0) @binding(5) var<storage, read_write> out_positions: array<f32>;

fn get_pos(buf_idx: u32) -> vec3<f32> {
  let b = buf_idx * 3u;
  return vec3<f32>(in_positions[b + 0u], in_positions[b + 1u], in_positions[b + 2u]);
}

fn get_aux(buf_idx: u32) -> vec3<f32> {
  let b = buf_idx * 3u;
  return vec3<f32>(aux_positions[b + 0u], aux_positions[b + 1u], aux_positions[b + 2u]);
}

fn write_out(idx: u32, p: vec3<f32>) {
  let b = idx * 3u;
  out_positions[b + 0u] = p.x;
  out_positions[b + 1u] = p.y;
  out_positions[b + 2u] = p.z;
}

fn eval_falloff(dist: f32, radius: f32, kind: u32) -> f32 {
  if (dist >= radius || radius <= 1e-9) {
    return 0.0;
  }
  if (kind == 2u) {
    return 1.0;
  }
  let u = 1.0 - dist / radius;
  if (kind == 1u) {
    return u;
  }
  return u * u * (3.0 - 2.0 * u);
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let idx = id.x;
  if (idx >= params.num_items) {
    return;
  }

  if (params.mode == 0u) {
    let start = stencil_meta[idx * 2u + 0u];
    let count = stencil_meta[idx * 2u + 1u];
    var sum = vec3<f32>(0.0, 0.0, 0.0);
    if (count > 0u) {
      let inv_n = 1.0 / f32(count);
      for (var i = 0u; i < count; i++) {
        let v_idx = stencil_indices[start + i];
        sum += get_pos(v_idx) * inv_n;
      }
    }
    write_out(idx, sum);
    return;
  }

  if (params.mode == 1u) {
    let v0 = stencil_meta[idx * 4u + 0u];
    let v1 = stencil_meta[idx * 4u + 1u];
    let f0 = stencil_meta[idx * 4u + 2u];
    let f1 = stencil_meta[idx * 4u + 3u];

    let p0 = get_pos(v0);
    let p1 = get_pos(v1);
    if (f1 == 0xFFFFFFFFu) {
      write_out(idx, (p0 + p1) * 0.5);
    } else {
      let fp0 = get_aux(f0);
      let fp1 = get_aux(f1);
      write_out(idx, (p0 + p1 + fp0 + fp1) * 0.25);
    }
    return;
  }

  if (params.mode == 2u) {
    let p = get_pos(idx);
    let nbr_off = stencil_meta[idx * 6u + 0u];
    let nbr_cnt = stencil_meta[idx * 6u + 1u];
    let face_off = stencil_meta[idx * 6u + 2u];
    let face_cnt = stencil_meta[idx * 6u + 3u];
    let b0 = stencil_meta[idx * 6u + 4u];
    let b1 = stencil_meta[idx * 6u + 5u];

    if (b0 != 0xFFFFFFFFu && b1 != 0xFFFFFFFFu) {
      let pb0 = get_pos(b0);
      let pb1 = get_pos(b1);
      write_out(idx, (6.0 * p + pb0 + pb1) * 0.125);
    } else if (nbr_cnt > 0u && face_cnt > 0u) {
      var f_avg = vec3<f32>(0.0, 0.0, 0.0);
      let inv_f = 1.0 / f32(face_cnt);
      for (var i = 0u; i < face_cnt; i++) {
        let fi = stencil_indices[face_off + i];
        f_avg += get_aux(fi) * inv_f;
      }

      var r_avg = vec3<f32>(0.0, 0.0, 0.0);
      let inv_n = 1.0 / f32(nbr_cnt);
      for (var i = 0u; i < nbr_cnt; i++) {
        let ni = stencil_indices[nbr_off + i];
        r_avg += 0.5 * (p + get_pos(ni)) * inv_n;
      }

      let n_f = f32(nbr_cnt);
      write_out(idx, (f_avg + 2.0 * r_avg + (n_f - 3.0) * p) / n_f);
    } else {
      write_out(idx, p);
    }
    return;
  }

  let p = get_pos(idx);
  let center = vec3<f32>(params.brush_cx, params.brush_cy, params.brush_cz);
  let diff = p - center;
  let dist = length(diff);
  let w = eval_falloff(dist, params.brush_radius, params.brush_falloff);

  if (w <= 0.0) {
    write_out(idx, p);
    return;
  }

  if (params.brush_kind == 0u) {
    let disp = vec3<f32>(params.brush_dx, params.brush_dy, params.brush_dz);
    write_out(idx, p + w * disp);
  } else if (params.brush_kind == 1u) {
    let n = get_aux(idx);
    write_out(idx, p + (w * params.brush_strength) * n);
  } else {
    write_out(idx, p - (w * params.brush_strength) * diff);
  }
}
`

function edgeKey(a: number, b: number): string {
  return a < b ? `${a}:${b}` : `${b}:${a}`
}

interface EdgeUse {
  a: number
  b: number
  faces: number[]
  edgeIndex: number
}

function buildSubdivUniform(
  mode: number,
  numItems: number,
  brushFalloff = 0,
  brushKind = 0,
  center: [number, number, number] = [0, 0, 0],
  radius = 1,
  disp: [number, number, number] = [0, 0, 0],
  strength = 0
): ArrayBuffer {
  const buf = new ArrayBuffer(48)
  const v = new DataView(buf)
  v.setUint32(0, mode, true)
  v.setUint32(4, numItems, true)
  v.setUint32(8, brushFalloff, true)
  v.setUint32(12, brushKind, true)
  v.setFloat32(16, center[0], true)
  v.setFloat32(20, center[1], true)
  v.setFloat32(24, center[2], true)
  v.setFloat32(28, radius, true)
  v.setFloat32(32, disp[0], true)
  v.setFloat32(36, disp[1], true)
  v.setFloat32(40, disp[2], true)
  v.setFloat32(44, strength, true)
  return buf
}

/**
 * Exact CPU reference implementation of Catmull–Clark refinement matching `subdivision_core::Cage::subdivide`.
 */
export function referenceRefineSubdivision(cage: SubdivisionCage, levels = 2): SubdivisionRefined {
  if (!Number.isInteger(levels) || levels < 0 || levels > 5) {
    throw new Error('Subdivision levels must be 0..5')
  }
  if (
    cage.vertices.length === 0 ||
    cage.vertices.length > 100_000 ||
    cage.faces.length === 0 ||
    cage.faces.length > 25_000
  ) {
    throw new Error('Invalid control cage or budget exceeded')
  }

  let currentVertices = cage.vertices.map((p) => [p[0]!, p[1]!, p[2]!])
  let currentFaces = cage.faces.map((f) => [...f])
  let faceIds = cage.faces.map((_, idx) => idx)

  for (let step = 0; step < levels; step++) {
    let cornerSum = 0
    for (let i = 0; i < currentFaces.length; i++) cornerSum += currentFaces[i]!.length
    if (cornerSum > 25_000) {
      throw new Error('Subdivision face budget exceeded')
    }

    const edgeMap = new Map<string, EdgeUse>()
    const edgeList: EdgeUse[] = []

    for (let fi = 0; fi < currentFaces.length; fi++) {
      const f = currentFaces[fi]!
      const flen = f.length
      for (let i = 0; i < flen; i++) {
        const u = f[i]!
        const w = f[(i + 1) % flen]!
        const a = u < w ? u : w
        const b = u < w ? w : u
        const key = `${a}:${b}`
        let entry = edgeMap.get(key)
        if (!entry) {
          entry = { a, b, faces: [], edgeIndex: edgeList.length }
          edgeMap.set(key, entry)
          edgeList.push(entry)
        }
        entry.faces.push(fi)
      }
    }

    // 1. Face points
    const facePoints: [number, number, number][] = new Array(currentFaces.length)
    for (let fi = 0; fi < currentFaces.length; fi++) {
      const f = currentFaces[fi]!
      const inv = 1 / f.length
      let sx = 0, sy = 0, sz = 0
      for (let i = 0; i < f.length; i++) {
        const p = currentVertices[f[i]!]!
        sx += p[0]! * inv
        sy += p[1]! * inv
        sz += p[2]! * inv
      }
      facePoints[fi] = [sx, sy, sz]
    }

    // Build vertex adjacency
    const vertexEdges: EdgeUse[][] = Array.from({ length: currentVertices.length }, () => [])
    for (let ei = 0; ei < edgeList.length; ei++) {
      const e = edgeList[ei]!
      vertexEdges[e.a]!.push(e)
      vertexEdges[e.b]!.push(e)
    }

    // 2. Updated vertex points
    const nextVertices: number[][] = new Array(
      currentVertices.length + edgeList.length + currentFaces.length
    )

    for (let v = 0; v < currentVertices.length; v++) {
      const p = currentVertices[v]!
      const adj = vertexEdges[v]!
      const boundaryNbrs: number[] = []
      for (let i = 0; i < adj.length; i++) {
        const e = adj[i]!
        if (e.faces.length === 1) {
          boundaryNbrs.push(e.a === v ? e.b : e.a)
        }
      }

      if (boundaryNbrs.length === 2) {
        const b0 = currentVertices[boundaryNbrs[0]!]!
        const b1 = currentVertices[boundaryNbrs[1]!]!
        nextVertices[v] = [
          (6 * p[0]! + b0[0]! + b1[0]!) / 8,
          (6 * p[1]! + b0[1]! + b1[1]!) / 8,
          (6 * p[2]! + b0[2]! + b1[2]!) / 8,
        ]
      } else {
        const fSet = new Set<number>()
        let rx = 0, ry = 0, rz = 0
        const n = adj.length
        const invN = 1 / n
        for (let i = 0; i < n; i++) {
          const e = adj[i]!
          const pa = currentVertices[e.a]!
          const pb = currentVertices[e.b]!
          rx += 0.5 * (pa[0]! + pb[0]!) * invN
          ry += 0.5 * (pa[1]! + pb[1]!) * invN
          rz += 0.5 * (pa[2]! + pb[2]!) * invN
          for (let j = 0; j < e.faces.length; j++) {
            fSet.add(e.faces[j]!)
          }
        }
        let fx = 0, fy = 0, fz = 0
        const invF = 1 / fSet.size
        for (const fi of fSet) {
          const fp = facePoints[fi]!
          fx += fp[0] * invF
          fy += fp[1] * invF
          fz += fp[2] * invF
        }
        nextVertices[v] = [
          (fx + 2 * rx + (n - 3) * p[0]!) / n,
          (fy + 2 * ry + (n - 3) * p[1]!) / n,
          (fz + 2 * rz + (n - 3) * p[2]!) / n,
        ]
      }
    }

    // 3. Edge points
    const edgeOffset = currentVertices.length
    for (let ei = 0; ei < edgeList.length; ei++) {
      const e = edgeList[ei]!
      const pa = currentVertices[e.a]!
      const pb = currentVertices[e.b]!
      if (e.faces.length === 2) {
        const fp0 = facePoints[e.faces[0]!]!
        const fp1 = facePoints[e.faces[1]!]!
        nextVertices[edgeOffset + ei] = [
          (pa[0]! + pb[0]! + fp0[0] + fp1[0]) * 0.25,
          (pa[1]! + pb[1]! + fp0[1] + fp1[1]) * 0.25,
          (pa[2]! + pb[2]! + fp0[2] + fp1[2]) * 0.25,
        ]
      } else {
        nextVertices[edgeOffset + ei] = [
          (pa[0]! + pb[0]!) * 0.5,
          (pa[1]! + pb[1]!) * 0.5,
          (pa[2]! + pb[2]!) * 0.5,
        ]
      }
    }

    // 4. Append face points
    const faceOffset = edgeOffset + edgeList.length
    for (let fi = 0; fi < currentFaces.length; fi++) {
      nextVertices[faceOffset + fi] = facePoints[fi]!
    }

    // 5. Assemble subdivided quad faces
    const nextFaces: number[][] = []
    const nextIds: number[] = []
    for (let fi = 0; fi < currentFaces.length; fi++) {
      const f = currentFaces[fi]!
      const flen = f.length
      for (let i = 0; i < flen; i++) {
        const curr = f[i]!
        const next = f[(i + 1) % flen]!
        const prev = f[(i + flen - 1) % flen]!
        const eNext = edgeOffset + edgeMap.get(edgeKey(curr, next))!.edgeIndex
        const ePrev = edgeOffset + edgeMap.get(edgeKey(prev, curr))!.edgeIndex
        nextFaces.push([curr, eNext, faceOffset + fi, ePrev])
        nextIds.push(faceIds[fi]!)
      }
    }

    currentVertices = nextVertices
    currentFaces = nextFaces
    faceIds = nextIds
  }

  return {
    cage: { vertices: currentVertices, faces: currentFaces },
    faceIds,
  }
}

/**
 * Refines a Catmull–Clark control cage on WebGPU Compute across `levels` iterations.
 * Falls back to deterministic CPU refinement when WebGPU is unavailable.
 */
export async function runGpuSubdivisionRefine(
  cage: SubdivisionCage,
  levels = 2
): Promise<SubdivisionRefined> {
  if (typeof navigator === 'undefined' || !navigator.gpu || levels === 0) {
    return referenceRefineSubdivision(cage, levels)
  }

  try {
    let currentVertices = cage.vertices.map((p) => [p[0]!, p[1]!, p[2]!])
    let currentFaces = cage.faces.map((f) => [...f])
    let faceIds = cage.faces.map((_, idx) => idx)

    for (let step = 0; step < levels; step++) {
      const numV = currentVertices.length
      const numF = currentFaces.length
      const inPos = new Float32Array(numV * 3)
      for (let i = 0; i < numV; i++) {
        const p = currentVertices[i]!
        inPos[i * 3 + 0] = p[0]!
        inPos[i * 3 + 1] = p[1]!
        inPos[i * 3 + 2] = p[2]!
      }

      const edgeMap = new Map<string, EdgeUse>()
      const edgeList: EdgeUse[] = []
      const faceMeta = new Uint32Array(numF * 2)
      const faceCornerList: number[] = []

      for (let fi = 0; fi < numF; fi++) {
        const f = currentFaces[fi]!
        faceMeta[fi * 2 + 0] = faceCornerList.length
        faceMeta[fi * 2 + 1] = f.length
        for (let i = 0; i < f.length; i++) {
          const u = f[i]!
          const w = f[(i + 1) % f.length]!
          faceCornerList.push(u)
          const key = edgeKey(u, w)
          let entry = edgeMap.get(key)
          if (!entry) {
            entry = { a: Math.min(u, w), b: Math.max(u, w), faces: [], edgeIndex: edgeList.length }
            edgeMap.set(key, entry)
            edgeList.push(entry)
          }
          entry.faces.push(fi)
        }
      }

      // Pass 0: Face points on GPU
      const faceOutBytes = numF * 3 * 4
      const faceRes = await runGpuCompute({
        wgsl: CATMULL_CLARK_WGSL,
        entryPoint: 'main',
        dispatches: [
          {
            buffers: [
              { binding: 0, data: buildSubdivUniform(0, numF), uniform: true },
              { binding: 1, data: inPos },
              { binding: 2, data: new Float32Array(4) },
              { binding: 3, data: faceMeta },
              { binding: 4, data: new Uint32Array(faceCornerList) },
              { binding: 5, data: new Float32Array(numF * 3), output: true, outputBytes: faceOutBytes },
            ],
            outputBytes: faceOutBytes,
            workgroups: [Math.ceil(numF / 256), 1, 1],
          },
        ],
      })
      const facePosBuf = faceRes[0]
      if (!facePosBuf) return referenceRefineSubdivision(cage, levels)

      // Pass 1: Edge points on GPU
      const numE = edgeList.length
      const edgeMeta = new Uint32Array(numE * 4)
      for (let ei = 0; ei < numE; ei++) {
        const e = edgeList[ei]!
        edgeMeta[ei * 4 + 0] = e.a
        edgeMeta[ei * 4 + 1] = e.b
        edgeMeta[ei * 4 + 2] = e.faces[0] ?? 0
        edgeMeta[ei * 4 + 3] = e.faces.length === 2 ? e.faces[1]! : 0xFFFFFFFF
      }
      const edgeOutBytes = numE * 3 * 4
      const edgeRes = await runGpuCompute({
        wgsl: CATMULL_CLARK_WGSL,
        entryPoint: 'main',
        dispatches: [
          {
            buffers: [
              { binding: 0, data: buildSubdivUniform(1, numE), uniform: true },
              { binding: 1, data: inPos },
              { binding: 2, data: facePosBuf },
              { binding: 3, data: edgeMeta },
              { binding: 4, data: new Uint32Array([0]) },
              { binding: 5, data: new Float32Array(numE * 3), output: true, outputBytes: edgeOutBytes },
            ],
            outputBytes: edgeOutBytes,
            workgroups: [Math.ceil(numE / 256), 1, 1],
          },
        ],
      })
      const edgePosBuf = edgeRes[0]
      if (!edgePosBuf) return referenceRefineSubdivision(cage, levels)

      // Pass 2: Vertex points on GPU
      const vertexEdges: EdgeUse[][] = Array.from({ length: numV }, () => [])
      for (let ei = 0; ei < numE; ei++) {
        const e = edgeList[ei]!
        vertexEdges[e.a]!.push(e)
        vertexEdges[e.b]!.push(e)
      }
      const vertMeta = new Uint32Array(numV * 6)
      const vertStencilIndices: number[] = []
      for (let v = 0; v < numV; v++) {
        const adj = vertexEdges[v]!
        const nbrs: number[] = []
        const boundary: number[] = []
        const fSet = new Set<number>()
        for (let i = 0; i < adj.length; i++) {
          const e = adj[i]!
          const other = e.a === v ? e.b : e.a
          nbrs.push(other)
          if (e.faces.length === 1) boundary.push(other)
          for (let j = 0; j < e.faces.length; j++) fSet.add(e.faces[j]!)
        }
        const nbrOff = vertStencilIndices.length
        for (const n of nbrs) vertStencilIndices.push(n)
        const faceOff = vertStencilIndices.length
        for (const fi of fSet) vertStencilIndices.push(fi)

        vertMeta[v * 6 + 0] = nbrOff
        vertMeta[v * 6 + 1] = nbrs.length
        vertMeta[v * 6 + 2] = faceOff
        vertMeta[v * 6 + 3] = fSet.size
        vertMeta[v * 6 + 4] = boundary.length === 2 ? boundary[0]! : 0xFFFFFFFF
        vertMeta[v * 6 + 5] = boundary.length === 2 ? boundary[1]! : 0xFFFFFFFF
      }

      const vertOutBytes = numV * 3 * 4
      const vertRes = await runGpuCompute({
        wgsl: CATMULL_CLARK_WGSL,
        entryPoint: 'main',
        dispatches: [
          {
            buffers: [
              { binding: 0, data: buildSubdivUniform(2, numV), uniform: true },
              { binding: 1, data: inPos },
              { binding: 2, data: facePosBuf },
              { binding: 3, data: vertMeta },
              { binding: 4, data: new Uint32Array(vertStencilIndices.length ? vertStencilIndices : [0]) },
              { binding: 5, data: new Float32Array(numV * 3), output: true, outputBytes: vertOutBytes },
            ],
            outputBytes: vertOutBytes,
            workgroups: [Math.ceil(numV / 256), 1, 1],
          },
        ],
      })
      const vertPosBuf = vertRes[0]
      if (!vertPosBuf) return referenceRefineSubdivision(cage, levels)

      const nextVertices: number[][] = []
      for (let i = 0; i < numV; i++) {
        nextVertices.push([vertPosBuf[i * 3]!, vertPosBuf[i * 3 + 1]!, vertPosBuf[i * 3 + 2]!])
      }
      for (let i = 0; i < numE; i++) {
        nextVertices.push([edgePosBuf[i * 3]!, edgePosBuf[i * 3 + 1]!, edgePosBuf[i * 3 + 2]!])
      }
      for (let i = 0; i < numF; i++) {
        nextVertices.push([facePosBuf[i * 3]!, facePosBuf[i * 3 + 1]!, facePosBuf[i * 3 + 2]!])
      }

      const edgeOffset = numV
      const faceOffset = numV + numE
      const nextFaces: number[][] = []
      const nextIds: number[] = []
      for (let fi = 0; fi < numF; fi++) {
        const f = currentFaces[fi]!
        const flen = f.length
        for (let i = 0; i < flen; i++) {
          const curr = f[i]!
          const next = f[(i + 1) % flen]!
          const prev = f[(i + flen - 1) % flen]!
          const eNext = edgeOffset + edgeMap.get(edgeKey(curr, next))!.edgeIndex
          const ePrev = edgeOffset + edgeMap.get(edgeKey(prev, curr))!.edgeIndex
          nextFaces.push([curr, eNext, faceOffset + fi, ePrev])
          nextIds.push(faceIds[fi]!)
        }
      }

      currentVertices = nextVertices
      currentFaces = nextFaces
      faceIds = nextIds
    }

    return {
      cage: { vertices: currentVertices, faces: currentFaces },
      faceIds,
    }
  } catch {
    return referenceRefineSubdivision(cage, levels)
  }
}

/**
 * Applies a localized smoothstep brush displacement to a subdivision control cage.
 */
export function referenceBrushSubdivision(
  cage: SubdivisionCage,
  brush: GeometryBrush
): SubdivisionCage {
  const [cx, cy, cz] = brush.center
  const [dx, dy, dz] = brush.displacement
  const r = brush.radius
  if (!(r > 0) || !Number.isFinite(r) || ![cx, cy, cz, dx, dy, dz].every(Number.isFinite)) {
    throw new Error('Invalid subdivision brush')
  }

  const nextVertices = cage.vertices.map((p) => {
    const px = p[0]!, py = p[1]!, pz = p[2]!
    const dist = Math.hypot(px - cx, py - cy, pz - cz)
    if (dist >= r) return [px, py, pz]
    const u = 1 - dist / r
    const w = u * u * (3 - 2 * u)
    return [px + w * dx, py + w * dy, pz + w * dz]
  })

  return {
    vertices: nextVertices,
    faces: cage.faces.map((f) => [...f]),
  }
}

/**
 * Applies a brush displacement on WebGPU Compute with CPU fallback.
 */
export async function runGpuBrushSubdivision(
  cage: SubdivisionCage,
  brush: GeometryBrush
): Promise<SubdivisionCage> {
  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return referenceBrushSubdivision(cage, brush)
  }

  try {
    const numV = cage.vertices.length
    const inPos = new Float32Array(numV * 3)
    for (let i = 0; i < numV; i++) {
      const p = cage.vertices[i]!
      inPos[i * 3 + 0] = p[0]!
      inPos[i * 3 + 1] = p[1]!
      inPos[i * 3 + 2] = p[2]!
    }
    const outBytes = numV * 3 * 4
    const res = await runGpuCompute({
      wgsl: CATMULL_CLARK_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            {
              binding: 0,
              data: buildSubdivUniform(
                3,
                numV,
                0,
                0,
                [brush.center[0]!, brush.center[1]!, brush.center[2]!],
                brush.radius,
                [brush.displacement[0]!, brush.displacement[1]!, brush.displacement[2]!],
                1
              ),
              uniform: true,
            },
            { binding: 1, data: inPos },
            { binding: 2, data: new Float32Array(4) },
            { binding: 3, data: new Uint32Array([0]) },
            { binding: 4, data: new Uint32Array([0]) },
            { binding: 5, data: new Float32Array(numV * 3), output: true, outputBytes: outBytes },
          ],
          outputBytes: outBytes,
          workgroups: [Math.ceil(numV / 256), 1, 1],
        },
      ],
    })
    const outBuf = res[0]
    if (outBuf && outBuf.length >= numV * 3) {
      const vertices: number[][] = new Array(numV)
      for (let i = 0; i < numV; i++) {
        vertices[i] = [outBuf[i * 3]!, outBuf[i * 3 + 1]!, outBuf[i * 3 + 2]!]
      }
      return {
        vertices,
        faces: cage.faces.map((f) => [...f]),
      }
    }
  } catch {
    // Fallback to CPU reference
  }

  return referenceBrushSubdivision(cage, brush)
}
