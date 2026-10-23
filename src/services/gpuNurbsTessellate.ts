/**
 * WebGPU Compute NURBS & B-Spline Surface Tessellator.
 *
 * Evaluates tensor-product NURBS surfaces entirely on GPU Compute units:
 * 1. Parallel Cox-de Boor basis function and first-derivative evaluation.
 * 2. Homogeneous projection and exact analytical surface normal extraction.
 * 3. Water-tight vertex and triangle index generation directly in VRAM.
 */

import { runGpuCompute } from './webgpuCompute'
import { generateQuadIndices, referenceNurbsTessellate, referenceNurbsTessellateAsync, type GpuNurbsSurface, type GpuNurbsTessellationResult } from './gpuNurbsReference'
import type { CooperativeAsyncOptions } from './cooperativeAsync'
export * from './gpuNurbsReference'

import NURBS_TESSELLATE_WGSL from '../../crates/compute-core/shaders/nurbs_tessellate.wgsl?raw'
export { NURBS_TESSELLATE_WGSL }


const nurbsUniformBuffer = new ArrayBuffer(64)
const nurbsUniformView = new DataView(nurbsUniformBuffer)

function buildNurbsUniform(
  surface: GpuNurbsSurface,
  samplesU: number,
  samplesV: number
): ArrayBuffer {
  const ku = surface.knotsU
  const kv = surface.knotsV
  const uMin = surface.domainU?.[0] ?? ku[surface.degreeU] ?? 0
  const uMax = surface.domainU?.[1] ?? ku[surface.numCpU] ?? 1
  const vMin = surface.domainV?.[0] ?? kv[surface.degreeV] ?? 0
  const vMax = surface.domainV?.[1] ?? kv[surface.numCpV] ?? 1

  const isRational = surface.weights && surface.weights.length > 0 ? 1 : 0
  const totalVertices = samplesU * samplesV
  const totalQuads = (samplesU - 1) * (samplesV - 1)

  nurbsUniformView.setUint32(0, surface.degreeU, true)
  nurbsUniformView.setUint32(4, surface.degreeV, true)
  nurbsUniformView.setUint32(8, surface.numCpU, true)
  nurbsUniformView.setUint32(12, surface.numCpV, true)

  nurbsUniformView.setUint32(16, surface.knotsU.length, true)
  nurbsUniformView.setUint32(20, surface.knotsV.length, true)
  nurbsUniformView.setUint32(24, samplesU, true)
  nurbsUniformView.setUint32(28, samplesV, true)

  nurbsUniformView.setFloat32(32, uMin, true)
  nurbsUniformView.setFloat32(36, uMax, true)
  nurbsUniformView.setFloat32(40, vMin, true)
  nurbsUniformView.setFloat32(44, vMax, true)

  nurbsUniformView.setUint32(48, isRational, true)
  nurbsUniformView.setUint32(52, totalVertices, true)
  nurbsUniformView.setUint32(56, totalQuads, true)
  nurbsUniformView.setUint32(60, 0, true) // pad

  return nurbsUniformBuffer
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
  samplesV = 32,
  options: CooperativeAsyncOptions = {}
): Promise<GpuNurbsTessellationResult> {
  const totalVertices = samplesU * samplesV
  const totalQuads = (samplesU - 1) * (samplesV - 1)
  const triangleCount = totalQuads * 2

  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return totalVertices >= 1024
      ? referenceNurbsTessellateAsync(surface, samplesU, samplesV, options)
      : referenceNurbsTessellate(surface, samplesU, samplesV)
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
            { binding: 4, data: new Float32Array(0), output: true, outputBytes: vertOutputBytes },
            { binding: 5, data: new Uint32Array(0), output: true, outputBytes: indexOutputBytes },
          ],
          outputBytes: vertOutputBytes,
          workgroups: [Math.ceil(samplesU / 16), Math.ceil(samplesV / 16), 1],
        },
      ],
    })

    const rawVerts = results[0]
    const rawIndices = results[1]
    if (rawVerts && rawVerts.length >= totalVertices * 8) {
      const indexCount = totalQuads * 6
      const indices =
        rawIndices && rawIndices.byteLength >= indexOutputBytes
          ? new Uint32Array(rawIndices.buffer, rawIndices.byteOffset, indexCount).slice()
          : generateQuadIndices(samplesU, samplesV)
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

  return totalVertices >= 1024
    ? referenceNurbsTessellateAsync(surface, samplesU, samplesV, options)
    : referenceNurbsTessellate(surface, samplesU, samplesV)
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
