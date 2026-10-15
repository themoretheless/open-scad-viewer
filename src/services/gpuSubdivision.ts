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

import CATMULL_CLARK_WGSL from '../../crates/compute-core/shaders/catmull_clark.wgsl?raw'
import { packUndirectedEdgeKey } from './math3d'
export { CATMULL_CLARK_WGSL }
import { referenceRefineSubdivision } from './gpuSubdivisionReference'
export { referenceRefineSubdivision } from './gpuSubdivisionReference'

const edgeKey = packUndirectedEdgeKey

interface EdgeUse {
  a: number
  b: number
  faces: number[]
  edgeIndex: number
}

const subdivUniformBuffer = new ArrayBuffer(48)
const subdivUniformView = new DataView(subdivUniformBuffer)
const dummyFloat4 = new Float32Array(4)
const dummyUint1 = new Uint32Array([0])
const emptyFloat0 = new Float32Array(0)

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
  subdivUniformView.setUint32(0, mode, true)
  subdivUniformView.setUint32(4, numItems, true)
  subdivUniformView.setUint32(8, brushFalloff, true)
  subdivUniformView.setUint32(12, brushKind, true)
  subdivUniformView.setFloat32(16, center[0], true)
  subdivUniformView.setFloat32(20, center[1], true)
  subdivUniformView.setFloat32(24, center[2], true)
  subdivUniformView.setFloat32(28, radius, true)
  subdivUniformView.setFloat32(32, disp[0], true)
  subdivUniformView.setFloat32(36, disp[1], true)
  subdivUniformView.setFloat32(40, disp[2], true)
  subdivUniformView.setFloat32(44, strength, true)
  return subdivUniformBuffer
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

      const edgeMap = new Map<number | string, EdgeUse>()
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
              { binding: 2, data: dummyFloat4 },
              { binding: 3, data: faceMeta },
              { binding: 4, data: new Uint32Array(faceCornerList) },
              { binding: 5, data: emptyFloat0, output: true, outputBytes: faceOutBytes },
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
              { binding: 4, data: dummyUint1 },
              { binding: 5, data: emptyFloat0, output: true, outputBytes: edgeOutBytes },
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
              { binding: 4, data: vertStencilIndices.length ? new Uint32Array(vertStencilIndices) : dummyUint1 },
              { binding: 5, data: emptyFloat0, output: true, outputBytes: vertOutBytes },
            ],
            outputBytes: vertOutBytes,
            workgroups: [Math.ceil(numV / 256), 1, 1],
          },
        ],
      })
      const vertPosBuf = vertRes[0]
      if (!vertPosBuf) return referenceRefineSubdivision(cage, levels)

      const nextVertices: number[][] = new Array(numV + numE + numF)
      for (let i = 0; i < numV; i++) {
        nextVertices[i] = [vertPosBuf[i * 3]!, vertPosBuf[i * 3 + 1]!, vertPosBuf[i * 3 + 2]!]
      }
      for (let i = 0; i < numE; i++) {
        nextVertices[numV + i] = [edgePosBuf[i * 3]!, edgePosBuf[i * 3 + 1]!, edgePosBuf[i * 3 + 2]!]
      }
      for (let i = 0; i < numF; i++) {
        nextVertices[numV + numE + i] = [facePosBuf[i * 3]!, facePosBuf[i * 3 + 1]!, facePosBuf[i * 3 + 2]!]
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

  const rSq = r * r
  const invR = 1 / r
  const nextVertices = cage.vertices.map((p) => {
    const px = p[0]!, py = p[1]!, pz = p[2]!
    const vx = px - cx, vy = py - cy, vz = pz - cz
    const distSq = vx * vx + vy * vy + vz * vz
    if (distSq >= rSq) return [px, py, pz]
    const dist = Math.sqrt(distSq)
    const u = 1 - dist * invR
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
            { binding: 2, data: dummyFloat4 },
            { binding: 3, data: dummyUint1 },
            { binding: 4, data: dummyUint1 },
            { binding: 5, data: emptyFloat0, output: true, outputBytes: outBytes },
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
