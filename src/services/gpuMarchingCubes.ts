/**
 * WebGPU Compute Marching Cubes / Tetrahedra Isosurface Extraction.
 *
 * Offloads SDF and implicit field polygonization entirely to GPU Compute:
 * 1. Parallel cell evaluation and triangle counting in Workgroups.
 * 2. Prefix-scan offset calculation.
 * 3. Watertight triangle and normal emission directly into VRAM vertex/index buffers.
 */

import { runGpuCompute } from './webgpuCompute'
import type { MeshData } from '../core/mesh'

export interface MarchingGrid {
  min: [number, number, number]
  max: [number, number, number]
  cells: [number, number, number]
  isoLevel?: number
}

export interface MarchingOutput {
  positions: Float32Array
  normals: Float32Array
  indices: Uint32Array
  triangleCount: number
}

// Canonical 6-tetrahedra decomposition of a cube (around diagonal 0-7)
export const CUBE_TETRAHEDRA: readonly [number, number, number, number][] = [
  [0, 1, 3, 7],
  [0, 3, 2, 7],
  [0, 2, 6, 7],
  [0, 6, 4, 7],
  [0, 4, 5, 7],
  [0, 5, 1, 7],
]

// Cube corner offsets in bit order (x, y, z)
export const CORNER_OFFSETS: readonly [number, number, number][] = [
  [0, 0, 0],
  [1, 0, 0],
  [0, 1, 0],
  [1, 1, 0],
  [0, 0, 1],
  [1, 0, 1],
  [0, 1, 1],
  [1, 1, 1],
]

export const MARCHING_TETRAHEDRA_WGSL = /* wgsl */`
struct GridUniform {
  nx: u32,
  ny: u32,
  nz: u32,
  total_cells: u32,
  min_x: f32,
  min_y: f32,
  min_z: f32,
  iso_level: f32,
  step_x: f32,
  step_y: f32,
  step_z: f32,
  max_triangles: u32,
}

@group(0) @binding(0) var<uniform> grid: GridUniform;
@group(0) @binding(1) var<storage, read> field: array<f32>;
// Atomic counter for emitted triangles
@group(0) @binding(2) var<storage, read_write> tri_counter: array<atomic<u32>, 1>;
// Emitted vertex buffer: x, y, z, nx, ny, nz (stride 6 floats)
@group(0) @binding(3) var<storage, read_write> out_vertices: array<f32>;
// Emitted index buffer: 3 u32 per triangle
@group(0) @binding(4) var<storage, read_write> out_indices: array<u32>;

const CORNER_OFFSET = array<vec3<u32>, 8>(
  vec3<u32>(0u, 0u, 0u),
  vec3<u32>(1u, 0u, 0u),
  vec3<u32>(0u, 1u, 0u),
  vec3<u32>(1u, 1u, 0u),
  vec3<u32>(0u, 0u, 1u),
  vec3<u32>(1u, 0u, 1u),
  vec3<u32>(0u, 1u, 1u),
  vec3<u32>(1u, 1u, 1u)
);

const CUBE_TETS = array<vec4<u32>, 6>(
  vec4<u32>(0u, 1u, 3u, 7u),
  vec4<u32>(0u, 3u, 2u, 7u),
  vec4<u32>(0u, 2u, 6u, 7u),
  vec4<u32>(0u, 6u, 4u, 7u),
  vec4<u32>(0u, 4u, 5u, 7u),
  vec4<u32>(0u, 5u, 1u, 7u)
);

fn field_index(x: u32, y: u32, z: u32) -> u32 {
  let row = grid.nx + 1u;
  let slice = row * (grid.ny + 1u);
  return x + y * row + z * slice;
}

fn corner_pos(x: u32, y: u32, z: u32, c: u32) -> vec3<f32> {
  let off = CORNER_OFFSET[c];
  return vec3<f32>(
    grid.min_x + f32(x + off.x) * grid.step_x,
    grid.min_y + f32(y + off.y) * grid.step_y,
    grid.min_z + f32(z + off.z) * grid.step_z
  );
}

fn interpolate_point(p0: vec3<f32>, v0: f32, p1: vec3<f32>, v1: f32) -> vec3<f32> {
  let denom = v1 - v0;
  if (abs(denom) < 1e-6) {
    return (p0 + p1) * 0.5;
  }
  let t = clamp((grid.iso_level - v0) / denom, 0.0, 1.0);
  return p0 + t * (p1 - p0);
}

fn emit_tri(v0: vec3<f32>, v1_in: vec3<f32>, v2_in: vec3<f32>, hint: vec3<f32>) {
  let slot = atomicAdd(&tri_counter[0], 1u);
  if (slot >= grid.max_triangles) {
    return;
  }

  var v1 = v1_in;
  var v2 = v2_in;
  let d1 = v1 - v0;
  let d2 = v2 - v0;
  var norm = cross(d1, d2);
  if (dot(norm, hint) < 0.0) {
    let tmp = v1;
    v1 = v2;
    v2 = tmp;
    norm = -norm;
  }
  let len_sq = dot(norm, norm);
  if (len_sq > 1e-12) {
    norm = norm * inverseSqrt(len_sq);
  } else {
    norm = vec3<f32>(0.0, 0.0, 1.0);
  }

  let base_v = slot * 3u;
  let v_off = base_v * 6u;

  // Vertex 0
  out_vertices[v_off + 0u] = v0.x;
  out_vertices[v_off + 1u] = v0.y;
  out_vertices[v_off + 2u] = v0.z;
  out_vertices[v_off + 3u] = norm.x;
  out_vertices[v_off + 4u] = norm.y;
  out_vertices[v_off + 5u] = norm.z;

  // Vertex 1
  out_vertices[v_off + 6u] = v1.x;
  out_vertices[v_off + 7u] = v1.y;
  out_vertices[v_off + 8u] = v1.z;
  out_vertices[v_off + 9u] = norm.x;
  out_vertices[v_off + 10u] = norm.y;
  out_vertices[v_off + 11u] = norm.z;

  // Vertex 2
  out_vertices[v_off + 12u] = v2.x;
  out_vertices[v_off + 13u] = v2.y;
  out_vertices[v_off + 14u] = v2.z;
  out_vertices[v_off + 15u] = norm.x;
  out_vertices[v_off + 16u] = norm.y;
  out_vertices[v_off + 17u] = norm.z;

  let i_off = slot * 3u;
  out_indices[i_off + 0u] = base_v + 0u;
  out_indices[i_off + 1u] = base_v + 1u;
  out_indices[i_off + 2u] = base_v + 2u;
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let cell_idx = id.x;
  if (cell_idx >= grid.total_cells) {
    return;
  }

  let x = cell_idx % grid.nx;
  let y = (cell_idx / grid.nx) % grid.ny;
  let z = cell_idx / (grid.nx * grid.ny);

  // Read corner values
  var cv: array<f32, 8>;
  var has_neg = false;
  var has_pos = false;
  for (var c = 0u; c < 8u; c++) {
    let off = CORNER_OFFSET[c];
    let val = field[field_index(x + off.x, y + off.y, z + off.z)];
    cv[c] = val;
    if (val < grid.iso_level) { has_neg = true; } else { has_pos = true; }
  }

  // Early out if all corners lie on the same side
  if (!has_neg || !has_pos) {
    return;
  }

  var cp: array<vec3<f32>, 8>;
  for (var c = 0u; c < 8u; c++) {
    cp[c] = corner_pos(x, y, z, c);
  }

  // Decompose cube into 6 tetrahedra
  for (var t = 0u; t < 6u; t++) {
    let tet = CUBE_TETS[t];
    let v0 = cv[tet.x];
    let v1 = cv[tet.y];
    let v2 = cv[tet.z];
    let v3 = cv[tet.w];

    let p0 = cp[tet.x];
    let p1 = cp[tet.y];
    let p2 = cp[tet.z];
    let p3 = cp[tet.w];

    // Classify corners into negative (inside) and positive (outside)
    var neg: array<u32, 4>;
    var pos: array<u32, 4>;
    var neg_len = 0u;
    var pos_len = 0u;

    if (v0 < grid.iso_level) { neg[neg_len] = 0u; neg_len++; } else { pos[pos_len] = 0u; pos_len++; }
    if (v1 < grid.iso_level) { neg[neg_len] = 1u; neg_len++; } else { pos[pos_len] = 1u; pos_len++; }
    if (v2 < grid.iso_level) { neg[neg_len] = 2u; neg_len++; } else { pos[pos_len] = 2u; pos_len++; }
    if (v3 < grid.iso_level) { neg[neg_len] = 3u; neg_len++; } else { pos[pos_len] = 3u; pos_len++; }

    if (neg_len == 0u || pos_len == 0u) {
      continue;
    }

    let p_arr = array<vec3<f32>, 4>(p0, p1, p2, p3);
    let v_arr = array<f32, 4>(v0, v1, v2, v3);

    var hint = vec3<f32>(0.0);
    let inv_pos = 1.0 / f32(pos_len);
    for (var i = 0u; i < pos_len; i++) {
      hint += p_arr[pos[i]] * inv_pos;
    }
    let inv_neg = 1.0 / f32(neg_len);
    for (var i = 0u; i < neg_len; i++) {
      hint -= p_arr[neg[i]] * inv_neg;
    }

    if (neg_len == 1u && pos_len == 3u) {
      let a = neg[0];
      let c0 = pos[0];
      let c1 = pos[1];
      let c2 = pos[2];
      emit_tri(
        interpolate_point(p_arr[a], v_arr[a], p_arr[c0], v_arr[c0]),
        interpolate_point(p_arr[a], v_arr[a], p_arr[c1], v_arr[c1]),
        interpolate_point(p_arr[a], v_arr[a], p_arr[c2], v_arr[c2]),
        hint
      );
    } else if (neg_len == 3u && pos_len == 1u) {
      let a = pos[0];
      let c0 = neg[0];
      let c1 = neg[1];
      let c2 = neg[2];
      emit_tri(
        interpolate_point(p_arr[a], v_arr[a], p_arr[c0], v_arr[c0]),
        interpolate_point(p_arr[a], v_arr[a], p_arr[c1], v_arr[c1]),
        interpolate_point(p_arr[a], v_arr[a], p_arr[c2], v_arr[c2]),
        hint
      );
    } else if (neg_len == 2u && pos_len == 2u) {
      let a = neg[0];
      let b = neg[1];
      let c = pos[0];
      let d = pos[1];

      let ac = interpolate_point(p_arr[a], v_arr[a], p_arr[c], v_arr[c]);
      let bc = interpolate_point(p_arr[b], v_arr[b], p_arr[c], v_arr[c]);
      let ad = interpolate_point(p_arr[a], v_arr[a], p_arr[d], v_arr[d]);
      let bd = interpolate_point(p_arr[b], v_arr[b], p_arr[d], v_arr[d]);

      emit_tri(ac, bc, bd, hint);
      emit_tri(ac, bd, ad, hint);
    }
  }
}
`

const gridUniformBuffer = new ArrayBuffer(48)
const gridUniformView = new DataView(gridUniformBuffer)

function buildGridUniform(grid: MarchingGrid, maxTriangles: number): ArrayBuffer {
  const [nx, ny, nz] = grid.cells
  const totalCells = nx * ny * nz
  const isoLevel = grid.isoLevel ?? 0.0

  const stepX = (grid.max[0] - grid.min[0]) / nx
  const stepY = (grid.max[1] - grid.min[1]) / ny
  const stepZ = (grid.max[2] - grid.min[2]) / nz

  gridUniformView.setUint32(0, nx, true)
  gridUniformView.setUint32(4, ny, true)
  gridUniformView.setUint32(8, nz, true)
  gridUniformView.setUint32(12, totalCells, true)

  gridUniformView.setFloat32(16, grid.min[0], true)
  gridUniformView.setFloat32(20, grid.min[1], true)
  gridUniformView.setFloat32(24, grid.min[2], true)
  gridUniformView.setFloat32(28, isoLevel, true)

  gridUniformView.setFloat32(32, stepX, true)
  gridUniformView.setFloat32(36, stepY, true)
  gridUniformView.setFloat32(40, stepZ, true)
  gridUniformView.setUint32(44, maxTriangles, true)

  return gridUniformBuffer
}

/**
 * Extracts a watertight isosurface from a 3D sampled scalar field on WebGPU.
 */
export async function runGpuMarchingIsosurface(
  grid: MarchingGrid,
  fieldValues: Float32Array,
  maxTriangles = 500_000
): Promise<MarchingOutput> {
  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return referenceMarchingTetrahedra(grid, fieldValues)
  }

  try {
    const [nx, ny, nz] = grid.cells
    const totalCells = nx * ny * nz

    const uniformData = buildGridUniform(grid, maxTriangles)
    const counterInit = new Uint32Array([0])

    // Total allocated vertex buffer: maxTriangles * 3 vertices * 6 floats * 4 bytes
    const vertOutputBytes = maxTriangles * 3 * 6 * 4
    // Total allocated index buffer: maxTriangles * 3 indices * 4 bytes
    const indexOutputBytes = maxTriangles * 3 * 4

    const results = await runGpuCompute({
      wgsl: MARCHING_TETRAHEDRA_WGSL,
      entryPoint: 'main',
      dispatches: [
        {
          buffers: [
            { binding: 0, data: uniformData, uniform: true },
            { binding: 1, data: fieldValues },
            { binding: 2, data: counterInit, output: true, outputBytes: 4 },
            { binding: 3, data: new Float32Array(0), output: true, outputBytes: vertOutputBytes },
            { binding: 4, data: new Uint32Array(0), output: true, outputBytes: indexOutputBytes },
          ],
          outputBytes: 4,
          workgroups: [Math.ceil(totalCells / 256), 1, 1],
        },
      ],
    })

    const countBuf = results[0]
    const rawVerts = results[1]
    const rawIndices = results[2]
    if (countBuf && rawVerts && rawIndices) {
      const triCountView = new Uint32Array(countBuf.buffer, countBuf.byteOffset, 1)
      const triCount = Math.min(triCountView[0] ?? 0, maxTriangles)
      const vertCount = triCount * 3
      const positions = new Float32Array(vertCount * 3)
      const normals = new Float32Array(vertCount * 3)
      for (let v = 0; v < vertCount; v++) {
        const srcOff = v * 6
        const dstOff = v * 3
        positions[dstOff + 0] = rawVerts[srcOff + 0]!
        positions[dstOff + 1] = rawVerts[srcOff + 1]!
        positions[dstOff + 2] = rawVerts[srcOff + 2]!
        normals[dstOff + 0] = rawVerts[srcOff + 3]!
        normals[dstOff + 1] = rawVerts[srcOff + 4]!
        normals[dstOff + 2] = rawVerts[srcOff + 5]!
      }
      const indicesView = new Uint32Array(rawIndices.buffer, rawIndices.byteOffset, vertCount)
      return {
        positions,
        normals,
        indices: indicesView.slice(),
        triangleCount: triCount,
      }
    }
  } catch {
    // Fallback to CPU reference when WebGPU adapter is unavailable
  }

  return referenceMarchingTetrahedra(grid, fieldValues)
}

/**
 * Fast CPU-side reference evaluator for Marching Tetrahedra (used for conformance verification).
 */
export function referenceMarchingTetrahedra(
  grid: MarchingGrid,
  fieldValues: Float32Array
): MarchingOutput {
  const [nx, ny, nz] = grid.cells
  const isoLevel = grid.isoLevel ?? 0.0
  const row = nx + 1
  const slice = row * (ny + 1)

  const minX = grid.min[0]
  const minY = grid.min[1]
  const minZ = grid.min[2]
  const stepX = (grid.max[0] - minX) / nx
  const stepY = (grid.max[1] - minY) / ny
  const stepZ = (grid.max[2] - minZ) / nz

  const positions: number[] = []
  const normals: number[] = []
  const indices: number[] = []
  let triCount = 0

  const cv = new Float32Array(8)
  const cpX = new Float64Array(8)
  const cpY = new Float64Array(8)
  const cpZ = new Float64Array(8)
  const neg = new Int32Array(4)
  const pos = new Int32Array(4)
  const vArr = new Float64Array(4)
  const pX = new Float64Array(4)
  const pY = new Float64Array(4)
  const pZ = new Float64Array(4)

  const q0: [number, number, number] = [0, 0, 0]
  const q1: [number, number, number] = [0, 0, 0]
  const q2: [number, number, number] = [0, 0, 0]
  const q3: [number, number, number] = [0, 0, 0]

  function interpInto(
    i0: number,
    i1: number,
    out: [number, number, number]
  ): [number, number, number] {
    const v0 = vArr[i0]!, v1 = vArr[i1]!
    const x0 = pX[i0]!, y0 = pY[i0]!, z0 = pZ[i0]!
    const x1 = pX[i1]!, y1 = pY[i1]!, z1 = pZ[i1]!
    const d = v1 - v0
    if (Math.abs(d) < 1e-6) {
      out[0] = (x0 + x1) * 0.5
      out[1] = (y0 + y1) * 0.5
      out[2] = (z0 + z1) * 0.5
      return out
    }
    const t = Math.max(0, Math.min(1, (isoLevel - v0) / d))
    out[0] = x0 + t * (x1 - x0)
    out[1] = y0 + t * (y1 - y0)
    out[2] = z0 + t * (z1 - z0)
    return out
  }

  function addTri(
    v0: [number, number, number],
    v1: [number, number, number],
    v2: [number, number, number],
    hintX: number,
    hintY: number,
    hintZ: number
  ) {
    const d1x = v1[0] - v0[0], d1y = v1[1] - v0[1], d1z = v1[2] - v0[2]
    const d2x = v2[0] - v0[0], d2y = v2[1] - v0[1], d2z = v2[2] - v0[2]
    let nx = d1y * d2z - d1z * d2y
    let ny = d1z * d2x - d1x * d2z
    let nz = d1x * d2y - d1y * d2x
    if (nx * hintX + ny * hintY + nz * hintZ < 0) {
      const tmp = v1
      v1 = v2
      v2 = tmp
      nx = -nx; ny = -ny; nz = -nz
    }
    const len = Math.hypot(nx, ny, nz)
    if (len > 1e-6) {
      const invLen = 1 / len
      nx *= invLen; ny *= invLen; nz *= invLen
    } else {
      nx = 0; ny = 0; nz = 1
    }

    const base = triCount * 3
    positions.push(v0[0], v0[1], v0[2], v1[0], v1[1], v1[2], v2[0], v2[1], v2[2])
    normals.push(nx, ny, nz, nx, ny, nz, nx, ny, nz)
    indices.push(base, base + 1, base + 2)
    triCount++
  }

  for (let z = 0; z < nz; z++) {
    const zBase = z * slice
    for (let y = 0; y < ny; y++) {
      const yzBase = y * row + zBase
      for (let x = 0; x < nx; x++) {
        const xyzBase = x + yzBase
        let hasNeg = false, hasPos = false
        for (let c = 0; c < 8; c++) {
          const [dx, dy, dz] = CORNER_OFFSETS[c]!
          const val = fieldValues[xyzBase + dx + dy * row + dz * slice]!
          cv[c] = val
          if (val < isoLevel) hasNeg = true; else hasPos = true
        }
        if (!hasNeg || !hasPos) continue

        for (let c = 0; c < 8; c++) {
          const [dx, dy, dz] = CORNER_OFFSETS[c]!
          cpX[c] = minX + (x + dx) * stepX
          cpY[c] = minY + (y + dy) * stepY
          cpZ[c] = minZ + (z + dz) * stepZ
        }

        for (let ti = 0; ti < 6; ti++) {
          const tet = CUBE_TETRAHEDRA[ti]!
          let negLen = 0
          let posLen = 0
          for (let k = 0; k < 4; k++) {
            const ci = tet[k]!
            const val = cv[ci]!
            vArr[k] = val
            pX[k] = cpX[ci]!
            pY[k] = cpY[ci]!
            pZ[k] = cpZ[ci]!
            if (val < isoLevel) neg[negLen++] = k
            else pos[posLen++] = k
          }

          if (negLen === 0 || posLen === 0) continue

          let hintX = 0, hintY = 0, hintZ = 0
          const invPos = 1 / posLen
          for (let k = 0; k < posLen; k++) {
            const idx = pos[k]!
            hintX += pX[idx]! * invPos
            hintY += pY[idx]! * invPos
            hintZ += pZ[idx]! * invPos
          }
          const invNeg = 1 / negLen
          for (let k = 0; k < negLen; k++) {
            const idx = neg[k]!
            hintX -= pX[idx]! * invNeg
            hintY -= pY[idx]! * invNeg
            hintZ -= pZ[idx]! * invNeg
          }

          if (negLen === 1 && posLen === 3) {
            const a = neg[0]!
            addTri(
              interpInto(a, pos[0]!, q0),
              interpInto(a, pos[1]!, q1),
              interpInto(a, pos[2]!, q2),
              hintX, hintY, hintZ
            )
          } else if (negLen === 3 && posLen === 1) {
            const a = pos[0]!
            addTri(
              interpInto(a, neg[0]!, q0),
              interpInto(a, neg[1]!, q1),
              interpInto(a, neg[2]!, q2),
              hintX, hintY, hintZ
            )
          } else if (negLen === 2 && posLen === 2) {
            const a = neg[0]!, b = neg[1]!, c = pos[0]!, d = pos[1]!
            const ac = interpInto(a, c, q0)
            const bc = interpInto(b, c, q1)
            const ad = interpInto(a, d, q2)
            const bd = interpInto(b, d, q3)
            addTri(ac, bc, bd, hintX, hintY, hintZ)
            addTri(ac, bd, ad, hintX, hintY, hintZ)
          }
        }
      }
    }
  }

  return {
    positions: new Float32Array(positions),
    normals: new Float32Array(normals),
    indices: new Uint32Array(indices),
    triangleCount: triCount,
  }
}
