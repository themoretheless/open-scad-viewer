/**
 * WebGPU Compute NURBS & B-Spline Surface Tessellator.
 *
 * Evaluates tensor-product NURBS surfaces entirely on GPU Compute units:
 * 1. Parallel Cox-de Boor basis function and first-derivative evaluation.
 * 2. Homogeneous projection and exact analytical surface normal extraction.
 * 3. Water-tight vertex and triangle index generation directly in VRAM.
 */

import { runGpuCompute } from './webgpuCompute'

export interface GpuNurbsSurface {
  degreeU: number
  degreeV: number
  knotsU: number[] | Float32Array
  knotsV: number[] | Float32Array
  /** Array of 3D control points [x, y, z] in row-major order: index = x + y * numCpU */
  controlPoints: Array<[number, number, number]> | Float32Array
  /** Optional weights for rational NURBS (default 1.0 for polynomial B-spline) */
  weights?: number[] | Float32Array
  numCpU: number
  numCpV: number
  domainU?: [number, number]
  domainV?: [number, number]
}

export interface GpuNurbsTessellationResult {
  /** Interleaved vertex buffer: stride 8 floats [x, y, z, nx, ny, nz, u, v] */
  vertices: Float32Array
  /** Triangle indices */
  indices: Uint32Array
  vertexCount: number
  triangleCount: number
}

export const NURBS_TESSELLATE_WGSL = /* wgsl */`
struct NurbsUniform {
  degree_u: u32,
  degree_v: u32,
  num_cp_u: u32,
  num_cp_v: u32,
  num_knots_u: u32,
  num_knots_v: u32,
  samples_u: u32,
  samples_v: u32,
  u_min: f32,
  u_max: f32,
  v_min: f32,
  v_max: f32,
  is_rational: u32,
  total_vertices: u32,
  total_quads: u32,
  _pad: u32,
};

@group(0) @binding(0) var<uniform> params: NurbsUniform;
@group(0) @binding(1) var<storage, read> knots_u: array<f32>;
@group(0) @binding(2) var<storage, read> knots_v: array<f32>;
@group(0) @binding(3) var<storage, read> control_points: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> out_vertices: array<f32>;
@group(0) @binding(5) var<storage, read_write> out_indices: array<u32>;

fn find_knot_span_u(u: f32) -> u32 {
  let p = params.degree_u;
  let n = params.num_cp_u - 1u;
  if (u >= knots_u[n + 1u]) {
    return n;
  }
  if (u <= knots_u[p]) {
    return p;
  }
  var low = p;
  var high = n + 1u;
  var mid = (low + high) / 2u;
  while (u < knots_u[mid] || u >= knots_u[mid + 1u]) {
    if (u < knots_u[mid]) {
      high = mid;
    } else {
      low = mid;
    }
    mid = (low + high) / 2u;
  }
  return mid;
}

fn find_knot_span_v(v: f32) -> u32 {
  let q = params.degree_v;
  let m = params.num_cp_v - 1u;
  if (v >= knots_v[m + 1u]) {
    return m;
  }
  if (v <= knots_v[q]) {
    return q;
  }
  var low = q;
  var high = m + 1u;
  var mid = (low + high) / 2u;
  while (v < knots_v[mid] || v >= knots_v[mid + 1u]) {
    if (v < knots_v[mid]) {
      high = mid;
    } else {
      low = mid;
    }
    mid = (low + high) / 2u;
  }
  return mid;
}

struct BasisDeriv {
  basis: array<f32, 8>,
  ders: array<f32, 8>,
};

fn eval_basis_u(span: u32, u: f32) -> BasisDeriv {
  var res: BasisDeriv;
  let p = params.degree_u;
  var ndu: array<f32, 64>;
  var left: array<f32, 8>;
  var right: array<f32, 8>;

  ndu[0] = 1.0;
  for (var j = 1u; j <= p; j++) {
    left[j] = u - knots_u[span + 1u - j];
    right[j] = knots_u[span + j] - u;
    var saved = 0.0;
    for (var r = 0u; r < j; r++) {
      let denom = right[r + 1u] + left[j - r];
      ndu[j * 8u + r] = denom;
      var temp = 0.0;
      if (abs(denom) > 1e-12) {
        temp = ndu[r * 8u + (j - 1u)] / denom;
      }
      ndu[r * 8u + j] = saved + right[r + 1u] * temp;
      saved = left[j - r] * temp;
    }
    ndu[j * 8u + j] = saved;
  }

  for (var j = 0u; j <= p; j++) {
    res.basis[j] = ndu[j * 8u + p];
  }

  let fp = f32(p);
  for (var r = 0u; r <= p; r++) {
    var d = 0.0;
    if (r == 0u) {
      let den = ndu[p * 8u + 0u];
      if (abs(den) > 1e-12) {
        d = -fp * ndu[0u * 8u + (p - 1u)] / den;
      }
    } else if (r == p) {
      let den = ndu[p * 8u + (p - 1u)];
      if (abs(den) > 1e-12) {
        d = fp * ndu[(p - 1u) * 8u + (p - 1u)] / den;
      }
    } else {
      let den1 = ndu[p * 8u + (r - 1u)];
      let den2 = ndu[p * 8u + r];
      let t1 = select(0.0, fp * ndu[(r - 1u) * 8u + (p - 1u)] / den1, abs(den1) > 1e-12);
      let t2 = select(0.0, fp * ndu[r * 8u + (p - 1u)] / den2, abs(den2) > 1e-12);
      d = t1 - t2;
    }
    res.ders[r] = d;
  }

  return res;
}

fn eval_basis_v(span: u32, v: f32) -> BasisDeriv {
  var res: BasisDeriv;
  let q = params.degree_v;
  var ndu: array<f32, 64>;
  var left: array<f32, 8>;
  var right: array<f32, 8>;

  ndu[0] = 1.0;
  for (var j = 1u; j <= q; j++) {
    left[j] = v - knots_v[span + 1u - j];
    right[j] = knots_v[span + j] - v;
    var saved = 0.0;
    for (var r = 0u; r < j; r++) {
      let denom = right[r + 1u] + left[j - r];
      ndu[j * 8u + r] = denom;
      var temp = 0.0;
      if (abs(denom) > 1e-12) {
        temp = ndu[r * 8u + (j - 1u)] / denom;
      }
      ndu[r * 8u + j] = saved + right[r + 1u] * temp;
      saved = left[j - r] * temp;
    }
    ndu[j * 8u + j] = saved;
  }

  for (var j = 0u; j <= q; j++) {
    res.basis[j] = ndu[j * 8u + q];
  }

  let fq = f32(q);
  for (var r = 0u; r <= q; r++) {
    var d = 0.0;
    if (r == 0u) {
      let den = ndu[q * 8u + 0u];
      if (abs(den) > 1e-12) {
        d = -fq * ndu[0u * 8u + (q - 1u)] / den;
      }
    } else if (r == q) {
      let den = ndu[q * 8u + (q - 1u)];
      if (abs(den) > 1e-12) {
        d = fq * ndu[(q - 1u) * 8u + (q - 1u)] / den;
      }
    } else {
      let den1 = ndu[q * 8u + (r - 1u)];
      let den2 = ndu[q * 8u + r];
      let t1 = select(0.0, fq * ndu[(r - 1u) * 8u + (q - 1u)] / den1, abs(den1) > 1e-12);
      let t2 = select(0.0, fq * ndu[r * 8u + (q - 1u)] / den2, abs(den2) > 1e-12);
      d = t1 - t2;
    }
    res.ders[r] = d;
  }

  return res;
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let iu = id.x;
  let iv = id.y;

  if (iu < params.samples_u && iv < params.samples_v) {
    let tu = f32(iu) / f32(params.samples_u - 1u);
    let tv = f32(iv) / f32(params.samples_v - 1u);

    let u = params.u_min + tu * (params.u_max - params.u_min);
    let v = params.v_min + tv * (params.v_max - params.v_min);

    let span_u = find_knot_span_u(u);
    let span_v = find_knot_span_v(v);

    let bu = eval_basis_u(span_u, u);
    let bv = eval_basis_v(span_v, v);

    let p = params.degree_u;
    let q = params.degree_v;

    var pt_homo = vec4<f32>(0.0);
    var du_homo = vec4<f32>(0.0);
    var dv_homo = vec4<f32>(0.0);

    for (var r = 0u; r <= p; r++) {
      let cp_x = span_u - p + r;
      let nu = bu.basis[r];
      let dnu = bu.ders[r];

      for (var s = 0u; s <= q; s++) {
        let cp_y = span_v - q + s;
        let mv = bv.basis[s];
        let dmv = bv.ders[s];

        let cp_idx = cp_x + cp_y * params.num_cp_u;
        var cp = control_points[cp_idx];

        if (params.is_rational == 0u) {
          cp.w = 1.0;
        } else {
          cp = vec4<f32>(cp.x * cp.w, cp.y * cp.w, cp.z * cp.w, cp.w);
        }

        let w_pt = nu * mv;
        let w_du = dnu * mv;
        let w_dv = nu * dmv;

        pt_homo += cp * w_pt;
        du_homo += cp * w_du;
        dv_homo += cp * w_dv;
      }
    }

    var pos: vec3<f32>;
    var du: vec3<f32>;
    var dv: vec3<f32>;

    if (params.is_rational == 0u || abs(pt_homo.w) < 1e-12) {
      pos = pt_homo.xyz;
      du = du_homo.xyz;
      dv = dv_homo.xyz;
    } else {
      let inv_w = 1.0 / pt_homo.w;
      pos = pt_homo.xyz * inv_w;
      du = (du_homo.xyz - pos * du_homo.w) * inv_w;
      dv = (dv_homo.xyz - pos * dv_homo.w) * inv_w;
    }

    var normal = cross(du, dv);
    let len = length(normal);
    if (len > 1e-7) {
      normal = normal / len;
    } else {
      normal = vec3<f32>(0.0, 0.0, 1.0);
    }

    let v_idx = iu + iv * params.samples_u;
    let v_off = v_idx * 8u;

    out_vertices[v_off + 0u] = pos.x;
    out_vertices[v_off + 1u] = pos.y;
    out_vertices[v_off + 2u] = pos.z;
    out_vertices[v_off + 3u] = normal.x;
    out_vertices[v_off + 4u] = normal.y;
    out_vertices[v_off + 5u] = normal.z;
    out_vertices[v_off + 6u] = tu;
    out_vertices[v_off + 7u] = tv;
  }

  if (iu < (params.samples_u - 1u) && iv < (params.samples_v - 1u)) {
    let q_idx = iu + iv * (params.samples_u - 1u);
    let i_off = q_idx * 6u;

    let v00 = iu + iv * params.samples_u;
    let v10 = (iu + 1u) + iv * params.samples_u;
    let v01 = iu + (iv + 1u) * params.samples_u;
    let v11 = (iu + 1u) + (iv + 1u) * params.samples_u;

    out_indices[i_off + 0u] = v00;
    out_indices[i_off + 1u] = v10;
    out_indices[i_off + 2u] = v01;

    out_indices[i_off + 3u] = v10;
    out_indices[i_off + 4u] = v11;
    out_indices[i_off + 5u] = v01;
  }
}
`

function buildNurbsUniform(
  surface: GpuNurbsSurface,
  samplesU: number,
  samplesV: number
): ArrayBuffer {
  const buffer = new ArrayBuffer(64)
  const view = new DataView(buffer)

  const ku = surface.knotsU
  const kv = surface.knotsV
  const uMin = surface.domainU?.[0] ?? (Array.isArray(ku) ? ku[surface.degreeU] : ku[surface.degreeU]) ?? 0
  const uMax = surface.domainU?.[1] ?? (Array.isArray(ku) ? ku[surface.numCpU] : ku[surface.numCpU]) ?? 1
  const vMin = surface.domainV?.[0] ?? (Array.isArray(kv) ? kv[surface.degreeV] : kv[surface.degreeV]) ?? 0
  const vMax = surface.domainV?.[1] ?? (Array.isArray(kv) ? kv[surface.numCpV] : kv[surface.numCpV]) ?? 1

  const isRational = surface.weights && surface.weights.length > 0 ? 1 : 0
  const totalVertices = samplesU * samplesV
  const totalQuads = (samplesU - 1) * (samplesV - 1)

  view.setUint32(0, surface.degreeU, true)
  view.setUint32(4, surface.degreeV, true)
  view.setUint32(8, surface.numCpU, true)
  view.setUint32(12, surface.numCpV, true)

  view.setUint32(16, surface.knotsU.length, true)
  view.setUint32(20, surface.knotsV.length, true)
  view.setUint32(24, samplesU, true)
  view.setUint32(28, samplesV, true)

  view.setFloat32(32, uMin, true)
  view.setFloat32(36, uMax, true)
  view.setFloat32(40, vMin, true)
  view.setFloat32(44, vMax, true)

  view.setUint32(48, isRational, true)
  view.setUint32(52, totalVertices, true)
  view.setUint32(56, totalQuads, true)
  view.setUint32(60, 0, true) // pad

  return buffer
}

function packControlPoints(surface: GpuNurbsSurface): Float32Array {
  const count = surface.numCpU * surface.numCpV
  const packed = new Float32Array(count * 4)

  const cp = surface.controlPoints
  const isFlat = cp instanceof Float32Array
  const weights = surface.weights

  for (let i = 0; i < count; i++) {
    let x = 0, y = 0, z = 0
    if (isFlat) {
      x = cp[i * 3 + 0] ?? 0
      y = cp[i * 3 + 1] ?? 0
      z = cp[i * 3 + 2] ?? 0
    } else {
      const pt = (cp as Array<[number, number, number]>)[i]
      if (pt) {
        x = pt[0]
        y = pt[1]
        z = pt[2]
      }
    }
    const w = weights ? (weights[i] ?? 1.0) : 1.0
    packed[i * 4 + 0] = x
    packed[i * 4 + 1] = y
    packed[i * 4 + 2] = z
    packed[i * 4 + 3] = w
  }

  return packed
}

/**
 * Evaluates a NURBS surface into a watertight triangle mesh using WebGPU Compute.
 */
export async function runGpuNurbsTessellate(
  surface: GpuNurbsSurface,
  samplesU = 32,
  samplesV = 32
): Promise<GpuNurbsTessellationResult> {
  const totalVertices = samplesU * samplesV
  const totalQuads = (samplesU - 1) * (samplesV - 1)
  const triangleCount = totalQuads * 2

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return referenceNurbsTessellate(surface, samplesU, samplesV)
  }

  try {
    const uniformData = buildNurbsUniform(surface, samplesU, samplesV)
    const knotsU = surface.knotsU instanceof Float32Array ? surface.knotsU : new Float32Array(surface.knotsU)
    const knotsV = surface.knotsV instanceof Float32Array ? surface.knotsV : new Float32Array(surface.knotsV)
    const cpPacked = packControlPoints(surface)

    const vertOutputBytes = totalVertices * 8 * 4
    const indexOutputBytes = totalQuads * 6 * 4

    const results = await runGpuCompute({
      wgsl: NURBS_TESSELLATE_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformData, uniform: true },
            { binding: 1, data: knotsU },
            { binding: 2, data: knotsV },
            { binding: 3, data: cpPacked },
            { binding: 4, data: new Float32Array(totalVertices * 8), output: true, outputBytes: vertOutputBytes },
            { binding: 5, data: new Uint32Array(totalQuads * 6), output: true, outputBytes: indexOutputBytes },
          ],
          outputBytes: vertOutputBytes,
          workgroups: [Math.ceil(samplesU / 16), Math.ceil(samplesV / 16), 1],
        },
      ],
    })

    const rawVerts = results[0]
    if (rawVerts && rawVerts.length >= totalVertices * 8) {
      const indices = generateQuadIndices(samplesU, samplesV)
      return {
        vertices: rawVerts.slice(0, totalVertices * 8),
        indices,
        vertexCount: totalVertices,
        triangleCount,
      }
    }
  } catch {
    // Fallback if WebGPU compute fails or device is unsupported
  }

  return referenceNurbsTessellate(surface, samplesU, samplesV)
}

/**
 * Fast quad index generator for a regular (samplesU x samplesV) parametric grid.
 */
export function generateQuadIndices(samplesU: number, samplesV: number): Uint32Array {
  const totalQuads = (samplesU - 1) * (samplesV - 1)
  const indices = new Uint32Array(totalQuads * 6)
  let ptr = 0

  for (let iv = 0; iv < samplesV - 1; iv++) {
    for (let iu = 0; iu < samplesU - 1; iu++) {
      const v00 = iu + iv * samplesU
      const v10 = (iu + 1) + iv * samplesU
      const v01 = iu + (iv + 1) * samplesU
      const v11 = (iu + 1) + (iv + 1) * samplesU

      // Tri 1: CCW
      indices[ptr++] = v00
      indices[ptr++] = v10
      indices[ptr++] = v01

      // Tri 2: CCW
      indices[ptr++] = v10
      indices[ptr++] = v11
      indices[ptr++] = v01
    }
  }

  return indices
}

/**
 * CPU-side reference evaluator for NURBS surface tessellation.
 * Used for mathematical verification, unit testing, and fallback.
 */
export function referenceNurbsTessellate(
  surface: GpuNurbsSurface,
  samplesU = 32,
  samplesV = 32
): GpuNurbsTessellationResult {
  const p = surface.degreeU
  const q = surface.degreeV
  const ku = surface.knotsU
  const kv = surface.knotsV
  const numCpU = surface.numCpU
  const numCpV = surface.numCpV

  const uMin = surface.domainU?.[0] ?? ku[p] ?? 0
  const uMax = surface.domainU?.[1] ?? ku[numCpU] ?? 1
  const vMin = surface.domainV?.[0] ?? kv[q] ?? 0
  const vMax = surface.domainV?.[1] ?? kv[numCpV] ?? 1

  const isRational = surface.weights && surface.weights.length > 0
  const cp = surface.controlPoints
  const isFlatCp = cp instanceof Float32Array
  const weights = surface.weights

  function getCp(x: number, y: number): [number, number, number, number] {
    const idx = x + y * numCpU
    let px = 0, py = 0, pz = 0
    if (isFlatCp) {
      px = cp[idx * 3 + 0] ?? 0
      py = cp[idx * 3 + 1] ?? 0
      pz = cp[idx * 3 + 2] ?? 0
    } else {
      const pt = (cp as Array<[number, number, number]>)[idx]
      if (pt) { px = pt[0]; py = pt[1]; pz = pt[2] }
    }
    const w = weights ? (weights[idx] ?? 1.0) : 1.0
    return [px, py, pz, w]
  }

  function findSpan(n: number, deg: number, u: number, knots: number[] | Float32Array): number {
    if (u >= knots[n + 1]!) return n
    if (u <= knots[deg]!) return deg
    let low = deg
    let high = n + 1
    let mid = Math.floor((low + high) / 2)
    while (u < knots[mid]! || u >= knots[mid + 1]!) {
      if (u < knots[mid]!) high = mid
      else low = mid
      mid = Math.floor((low + high) / 2)
    }
    return mid
  }

  function basisFunsDers(
    span: number,
    u: number,
    deg: number,
    knots: number[] | Float32Array
  ): { basis: number[]; ders: number[] } {
    const ndu: number[][] = Array.from({ length: deg + 1 }, () => new Array(deg + 1).fill(0))
    const left = new Array(deg + 1).fill(0)
    const right = new Array(deg + 1).fill(0)

    ndu[0]![0] = 1.0
    for (let j = 1; j <= deg; j++) {
      left[j] = u - knots[span + 1 - j]!
      right[j] = knots[span + j]! - u
      let saved = 0.0
      for (let r = 0; r < j; r++) {
        const denom = right[r + 1]! + left[j - r]!
        ndu[j]![r] = denom
        const temp = Math.abs(denom) > 1e-12 ? ndu[r]![j - 1]! / denom : 0.0
        ndu[r]![j] = saved + right[r + 1]! * temp
        saved = left[j - r]! * temp
      }
      ndu[j]![j] = saved
    }

    const basis: number[] = new Array(deg + 1)
    for (let j = 0; j <= deg; j++) {
      basis[j] = ndu[j]![deg]!
    }

    const ders: number[] = new Array(deg + 1).fill(0)
    for (let r = 0; r <= deg; r++) {
      let d = 0.0
      if (r === 0) {
        const den = ndu[deg]![0]!
        if (Math.abs(den) > 1e-12) d = -deg * ndu[0]![deg - 1]! / den
      } else if (r === deg) {
        const den = ndu[deg]![deg - 1]!
        if (Math.abs(den) > 1e-12) d = deg * ndu[deg - 1]![deg - 1]! / den
      } else {
        const den1 = ndu[deg]![r - 1]!
        const den2 = ndu[deg]![r]!
        const t1 = Math.abs(den1) > 1e-12 ? deg * ndu[r - 1]![deg - 1]! / den1 : 0.0
        const t2 = Math.abs(den2) > 1e-12 ? deg * ndu[r]![deg - 1]! / den2 : 0.0
        d = t1 - t2
      }
      ders[r] = d
    }

    return { basis, ders }
  }

  const totalVertices = samplesU * samplesV
  const vertices = new Float32Array(totalVertices * 8)

  for (let iv = 0; iv < samplesV; iv++) {
    const tv = iv / (samplesV - 1)
    const v = vMin + tv * (vMax - vMin)
    const spanV = findSpan(numCpV - 1, q, v, kv)
    const bv = basisFunsDers(spanV, v, q, kv)

    for (let iu = 0; iu < samplesU; iu++) {
      const tu = iu / (samplesU - 1)
      const u = uMin + tu * (uMax - uMin)
      const spanU = findSpan(numCpU - 1, p, u, ku)
      const bu = basisFunsDers(spanU, u, p, ku)

      let ptX = 0, ptY = 0, ptZ = 0, ptW = 0
      let duX = 0, duY = 0, duZ = 0, duW = 0
      let dvX = 0, dvY = 0, dvZ = 0, dvW = 0

      for (let r = 0; r <= p; r++) {
        const cpx = spanU - p + r
        const nu = bu.basis[r]!
        const dnu = bu.ders[r]!

        for (let s = 0; s <= q; s++) {
          const cpy = spanV - q + s
          const mv = bv.basis[s]!
          const dmv = bv.ders[s]!

          const cpVal = getCp(cpx, cpy)
          let cx = cpVal[0], cy = cpVal[1], cz = cpVal[2]
          const cw = cpVal[3]

          if (isRational) {
            cx *= cw
            cy *= cw
            cz *= cw
          }

          const wpt = nu * mv
          const wdu = dnu * mv
          const wdv = nu * dmv

          ptX += cx * wpt; ptY += cy * wpt; ptZ += cz * wpt; ptW += cw * wpt
          duX += cx * wdu; duY += cy * wdu; duZ += cz * wdu; duW += cw * wdu
          dvX += cx * wdv; dvY += cy * wdv; dvZ += cz * wdv; dvW += cw * wdv
        }
      }

      let posX = ptX, posY = ptY, posZ = ptZ
      let tanUX = duX, tanUY = duY, tanUZ = duZ
      let tanVX = dvX, tanVY = dvY, tanVZ = dvZ

      if (isRational && Math.abs(ptW) > 1e-12) {
        const invW = 1.0 / ptW
        posX = ptX * invW
        posY = ptY * invW
        posZ = ptZ * invW
        tanUX = (duX - posX * duW) * invW
        tanUY = (duY - posY * duW) * invW
        tanUZ = (duZ - posZ * duW) * invW
        tanVX = (dvX - posX * dvW) * invW
        tanVY = (dvY - posY * dvW) * invW
        tanVZ = (dvZ - posZ * dvW) * invW
      }

      let nx = tanUY * tanVZ - tanUZ * tanVY
      let ny = tanUZ * tanVX - tanUX * tanVZ
      let nz = tanUX * tanVY - tanUY * tanVX
      const len = Math.hypot(nx, ny, nz)
      if (len > 1e-7) {
        nx /= len; ny /= len; nz /= len
      } else {
        nx = 0; ny = 0; nz = 1
      }

      const vIdx = iu + iv * samplesU
      const off = vIdx * 8
      vertices[off + 0] = posX
      vertices[off + 1] = posY
      vertices[off + 2] = posZ
      vertices[off + 3] = nx
      vertices[off + 4] = ny
      vertices[off + 5] = nz
      vertices[off + 6] = tu
      vertices[off + 7] = tv
    }
  }

  const indices = generateQuadIndices(samplesU, samplesV)

  return {
    vertices,
    indices,
    vertexCount: totalVertices,
    triangleCount: (samplesU - 1) * (samplesV - 1) * 2,
  }
}

/**
 * Creates a flat bilinear patch (degree 1x1).
 */
export function createBilinearNurbsPatch(
  p00: [number, number, number],
  p10: [number, number, number],
  p01: [number, number, number],
  p11: [number, number, number]
): GpuNurbsSurface {
  return {
    degreeU: 1,
    degreeV: 1,
    knotsU: [0, 0, 1, 1],
    knotsV: [0, 0, 1, 1],
    controlPoints: [p00, p10, p01, p11],
    numCpU: 2,
    numCpV: 2,
  }
}

/**
 * Creates an exact rational 90-degree cylindrical NURBS patch.
 */
export function createCylinderQuarterNurbsPatch(radius = 1.0, height = 2.0): GpuNurbsSurface {
  // Quadratic in U (quarter circle: w = [1, cos(45°), 1] = [1, sqrt(2)/2, 1])
  // Linear in V (height)
  const w1 = Math.SQRT1_2
  return {
    degreeU: 2,
    degreeV: 1,
    knotsU: [0, 0, 0, 1, 1, 1],
    knotsV: [0, 0, 1, 1],
    numCpU: 3,
    numCpV: 2,
    controlPoints: [
      // v = 0 (z = 0)
      [radius, 0, 0],
      [radius, radius, 0],
      [0, radius, 0],
      // v = 1 (z = height)
      [radius, 0, height],
      [radius, radius, height],
      [0, radius, height],
    ],
    weights: [
      1.0, w1, 1.0,
      1.0, w1, 1.0,
    ],
  }
}
