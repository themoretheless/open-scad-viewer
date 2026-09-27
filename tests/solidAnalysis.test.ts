import { describe, expect, it } from 'vitest'
import { CadGeometryKernel } from '../src/services/cadGeometryKernel'
import { loadCadKernelOps } from '../src/services/cadKernelOps'
import { analyzeSolidCooperativelyInKernel, analyzeSolidInKernel } from '../src/services/geometry/meshAnalysis'
import { decodeNurbsResult, kernelRuntime } from '../src/services/geometry/kernel'
import { buildMeshBvh } from '../src/services/meshBvh'
import { extractSemanticEdges } from '../src/services/meshTopology'

function expectSameBytes(actual: ArrayBufferView, expected: ArrayBufferView) {
  expect(Buffer.compare(
    Buffer.from(actual.buffer, actual.byteOffset, actual.byteLength),
    Buffer.from(expected.buffer, expected.byteOffset, expected.byteLength),
  )).toBe(0)
}

describe('retained solid analysis', () => {
  it.each(['cube', 'sphere', 'dense-sphere', 'hole', 'empty', 'transformed'])(
    'matches separate display, BVH and edge calls byte for byte: %s', async fixture => {
      const session = await new CadGeometryKernel().openSession()
      try {
        const { CadSolid } = session.module
        const solid = fixture === 'cube' ? CadSolid.cube([2, 3, 4], true)
          : fixture === 'sphere' ? CadSolid.sphere(3, 24)
          : fixture === 'dense-sphere' ? CadSolid.sphere(30, 128)
          : fixture === 'hole' ? CadSolid.cube([8, 8, 2], true).subtract(CadSolid.cylinder(4, 1, 1, 16, true))
          : fixture === 'empty' ? CadSolid.union([])
          : CadSolid.cube([2, 3, 4], true).scale([-1, 2, 0.25]).rotate([30, 40, 50]).translate([1e5, -5, 3])
        const mesh = solid.calculateNormals(0, 52.5).getMesh()
        const bvh = buildMeshBvh(mesh.vertProperties, mesh.triVerts)
        const edges = extractSemanticEdges(mesh.vertProperties, mesh.triVerts, {
          mergeFromVert: mesh.mergeFromVert, mergeToVert: mesh.mergeToVert,
        })
        const actual = analyzeSolidInKernel(solid.handle)
        const pairs = [
          [actual.mesh.vertices, mesh.vertProperties], [actual.mesh.indices, mesh.triVerts],
          [actual.mesh.mergeFrom, mesh.mergeFromVert], [actual.mesh.mergeTo, mesh.mergeToVert],
          [actual.mesh.faceIds, mesh.faceID], [actual.bvh.bounds, bvh.bounds],
          [actual.bvh.nodes, bvh.nodes], [actual.bvh.triangles, bvh.triangles],
          [actual.semanticEdges.indices, edges.indices],
        ] as const
        for (const [value, reference] of pairs) expectSameBytes(value, reference)
        expect(actual.bvh.nodeCount).toBe(bvh.nodeCount)
        expect(actual.bvh.vertexStride).toBe(bvh.vertexStride)
        expect(actual.bvh.leafSize).toBe(bvh.leafSize)
        expect(actual.semanticEdges.diagnostics).toEqual(edges.diagnostics)

        const buffers = pairs.map(([value]) => value.buffer)
        expect(new Set(buffers).size).toBe(buffers.length)
        for (const [value] of pairs) {
          expect(value.byteOffset).toBe(0)
          expect(value.byteLength).toBe(value.buffer.byteLength)
        }
        // Result leases are already freed. Source deletion, memory growth and
        // transfer must not change the independently owned publication arrays.
        solid.delete()
        kernelRuntime().memory.grow(1)
        const transferred = structuredClone(actual, { transfer: buffers })
        expect(buffers.every(buffer => buffer.byteLength === 0)).toBe(true)
        expectSameBytes(transferred.mesh.vertices, mesh.vertProperties)
        expectSameBytes(transferred.mesh.indices, mesh.triVerts)
        expectSameBytes(transferred.bvh.bounds, bvh.bounds)
        expectSameBytes(transferred.semanticEdges.indices, edges.indices)
      } finally { session.dispose() }
    },
  )

  it.each(['cube', 'sphere', 'empty'])('cooperative analysis preserves exclusive result buffers: %s', async fixture => {
    const session = await new CadGeometryKernel().openSession()
    try {
      const {CadSolid} = session.module
      const solid = fixture === 'cube' ? CadSolid.cube([2,3,4], true)
        : fixture === 'sphere' ? CadSolid.sphere(3, 128) : CadSolid.union([])
      const expected = analyzeSolidInKernel(solid.handle)
      let checkpoints = 0
      const actual = await analyzeSolidCooperativelyInKernel(solid.handle, async () => { checkpoints++ })
      expect(actual).toEqual(expected)
      if (fixture === 'sphere') expect(checkpoints).toBeGreaterThan(10)
      expect(actual.mesh.vertices.buffer).not.toBe(expected.mesh.vertices.buffer)
      solid.delete()
      kernelRuntime().memory.grow(1)
      expect(actual).toEqual(expected)
    } finally { session.dispose() }
  })

  it('cancels after multiple BVH and edge steps and releases capacity for later analyses', async () => {
    const session = await new CadGeometryKernel().openSession()
    try {
      const solid = session.module.CadSolid.sphere(3, 128)
      const cancellation = new Error('cancel during BVH')
      for (let run = 0; run < 5; run++) {
        let checkpoints = 0
        await expect(analyzeSolidCooperativelyInKernel(solid.handle, async () => {
          if (++checkpoints === [4, 80, 200, 240, 300][run]) throw cancellation
        })).rejects.toBe(cancellation)
        expect(checkpoints).toBe([4, 80, 200, 240, 300][run])
      }
      const result = await analyzeSolidCooperativelyInKernel(solid.handle, async () => {})
      expect(result).toEqual(analyzeSolidInKernel(solid.handle))
    } finally { session.dispose() }
  })

  it('cancels a completed analysis before copying and releases the result lease', async () => {
    const session = await new CadGeometryKernel().openSession()
    try {
      const solid = session.module.CadSolid.cube(2)
      let completionCheckpoints = 0
      await analyzeSolidCooperativelyInKernel(solid.handle, async () => { completionCheckpoints++ })
      const cancel = new Error('cancel before publication')
      for (let run = 0; run < 5; run++) {
        let checkpoints = 0
        // The final checkpoint follows result completion but precedes copying.
        await expect(analyzeSolidCooperativelyInKernel(solid.handle, async () => {
          if (++checkpoints === completionCheckpoints) throw cancel
        })).rejects.toBe(cancel)
      }
      const result = await analyzeSolidCooperativelyInKernel(solid.handle, async () => {})
      expect(result.mesh.indices).toHaveLength(36)
    } finally { session.dispose() }
  })

  it('resumable ABI rejects stale jobs without cancelling newer jobs', async () => {
    const session = await new CadGeometryKernel().openSession()
    try {
      const solid = session.module.CadSolid.sphere(3, 128)
      const {exports: wasm, takeResponse} = kernelRuntime()
      const start = () => decodeNurbsResult<number>(takeResponse(wasm.abi_solid_analysis_start(solid.handle, .6, .8, 8)))
      const a = start(), b = start()
      try {
        expect(start).toThrow('job budget')
        expect(decodeNurbsResult(takeResponse(wasm.abi_solid_analysis_step(a)))).toBe(0)
        wasm.abi_solid_analysis_cancel(a)
        const c = start()
        try {
          expect(c).toBeGreaterThan(b)
          wasm.abi_solid_analysis_cancel(a)
          expect(() => decodeNurbsResult(takeResponse(wasm.abi_solid_analysis_step(a)))).toThrow('Unknown or completed')
          expect(decodeNurbsResult(takeResponse(wasm.abi_solid_analysis_step(c)))).toBe(0)
        } finally { wasm.abi_solid_analysis_cancel(c) }
      } finally { wasm.abi_solid_analysis_cancel(a); wasm.abi_solid_analysis_cancel(b) }
    } finally { session.dispose() }
  })

  it('preserves metrics and provenance while producing independent analyses', async () => {
    const ops = await loadCadKernelOps()
    const solid = ops.box([2, 3, 4], true)
    try {
      const first = ops.analyzeSolid(solid)
      const second = ops.analyzeSolid(solid)
      expect(first.volume).toBe(24)
      expect(first.surfaceArea).toBe(52)
      expect(first.mesh.runOriginalID).toEqual(new Uint32Array([ops.originalId(solid)!]))
      expect(first.mesh.runIndex).toEqual(new Uint32Array([0, 36]))
      expect(first.mesh.runFlags).toEqual(new Uint8Array([0]))
      first.mesh.vertProperties.fill(0)
      first.bvh.bounds.fill(0)
      expect(second.mesh.vertProperties.some(value => value !== 0)).toBe(true)
      expect(second.bvh.bounds.some(value => value !== 0)).toBe(true)
    } finally { ops.delete(solid) }
    expect(() => ops.analyzeSolid(solid)).toThrow('Foreign or deleted')
  })

  it('rejects invalid ABI parameters and deleted handles without poisoning later calls', async () => {
    const session = await new CadGeometryKernel().openSession()
    try {
      const solid = session.module.CadSolid.cube(1)
      const { exports: wasm, takeResponse } = kernelRuntime()
      for (const [normal, edge, leaf] of [[NaN, 1, 8], [1, Infinity, 8], [1, 1, 0], [1, 1, 65]]) {
        expect(() => decodeNurbsResult(takeResponse(wasm.abi_analyze_solid(solid.handle, normal, edge, leaf))))
          .toThrow('Invalid solid analysis parameters')
      }
      expect(analyzeSolidInKernel(solid.handle).mesh.indices).toHaveLength(36)
      solid.delete()
      expect(() => analyzeSolidInKernel(solid.handle)).toThrow('Unknown or deleted')
      expect(() => analyzeSolidInKernel(0)).toThrow('Invalid CAD handle')
    } finally { session.dispose() }
  })
})
