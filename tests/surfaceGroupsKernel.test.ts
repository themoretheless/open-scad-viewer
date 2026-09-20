import {expect, it} from 'vitest'
import {CadGeometryKernel} from '../src/services/cadGeometryKernel'
import {surfaceGroupsInKernel} from '../src/services/geometry/meshAnalysis'
import {kernelRuntime} from '../src/services/geometry/kernel'
import {inferSurfaceIds} from '../src/services/meshSurfaceGroups'

it('matches host groups for real display meshes and owns detached result buffers', async () => {
  const session = await new CadGeometryKernel().openSession()
  try {
    const {CadSolid} = session.module
    for (const solid of [CadSolid.cube(3), CadSolid.cylinder(4, 2, 2, 48), CadSolid.sphere(3, 32)]) {
      const mesh = solid.calculateNormals(0, 52.5).getMesh()
      for (const angle of [0, 15, 30, 60]) {
        const expected = inferSurfaceIds(mesh.vertProperties, mesh.triVerts, angle)
        const actual = surfaceGroupsInKernel(mesh.vertProperties, mesh.triVerts, 6, angle)
        expect(actual).toEqual(expected)
        kernelRuntime().memory.grow(1)
        expect(actual).toEqual(expected)
        const transferred = structuredClone(actual, {transfer: [actual.buffer]})
        expect(transferred).toEqual(expected)
        expect(actual.byteLength).toBe(0)
      }
      solid.delete()
    }
  } finally { session.dispose() }
})

it('handles subarray uploads, empty meshes and recovery after invalid input', async () => {
  const session = await new CadGeometryKernel().openSession()
  try {
    const vertices = new Float32Array([99, 0,0,0,0,0,1, 1,0,0,0,0,1, 0,1,0,0,0,1, 99]).subarray(1, 19)
    const indices = new Uint32Array([99, 0, 1, 2, 99]).subarray(1, 4)
    expect(surfaceGroupsInKernel(vertices, indices)).toEqual(new Uint32Array([0]))
    for (let i = 0; i < 20; i++) {
      expect(() => surfaceGroupsInKernel(vertices, new Uint32Array([0, 1, 3]))).toThrow('Invalid triangle index')
      expect(() => surfaceGroupsInKernel(vertices, indices, 6, NaN)).toThrow('Invalid surface grouping')
      expect(surfaceGroupsInKernel(vertices, indices)).toEqual(new Uint32Array([0]))
    }
    expect(surfaceGroupsInKernel(new Float32Array(), new Uint32Array())).toEqual(new Uint32Array())
    vertices[0] = Infinity
    expect(() => surfaceGroupsInKernel(vertices, indices)).toThrow('Nonfinite mesh position')
  } finally { session.dispose() }
})
