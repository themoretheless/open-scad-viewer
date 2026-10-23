/**
 * WebGPU Compute Marching Cubes / Tetrahedra Isosurface Extraction.
 *
 * Offloads SDF and implicit field polygonization entirely to GPU Compute:
 * 1. Parallel cell evaluation and triangle counting in Workgroups.
 * 2. Prefix-scan offset calculation.
 * 3. Watertight triangle and normal emission directly into VRAM vertex/index buffers.
 */

import MARCHING_TETRAHEDRA_WGSL from '../../crates/compute-core/shaders/marching_tetrahedra.wgsl?raw'
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

export { MARCHING_TETRAHEDRA_WGSL }

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
