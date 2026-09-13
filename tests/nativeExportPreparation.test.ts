import { expect, it, vi } from 'vitest'
import * as kernel from '../src/services/geometry/kernel'
import { prepareExportMeshInKernel } from '../src/services/geometry/meshAnalysis'

it('copies raw subarray views and releases inputs/results on native success and refusal', () => {
  const points = new Float32Array([99,0,0,0,0,0,1, 1,0,0,0,0,1, 0,1,0,0,0,1,99]).subarray(1,19)
  const indices = new Uint32Array([99,0,1,2,99]).subarray(1,4)
  const matrix = new Float32Array([-1,0,0,2,0,1,0,3,0,0,1,4,0,0,0,1])
  const real = kernel.kernelRuntime()
  const release = vi.fn((handle: number) => real.exports.abi_array_free(handle))
  const free = vi.fn((ptr: number, len: number) => real.exports.abi_free(ptr,len))
  const runtime = vi.spyOn(kernel,'kernelRuntime').mockImplementation(() => ({ ...real, exports: { ...real.exports, abi_array_free: release, abi_free: free } }))
  try {
    const result = prepareExportMeshInKernel(points,indices,matrix,false)
    expect(Array.from(result.positions)).toEqual([2,3,4,1,3,4,2,4,4])
    expect(Array.from(result.indices)).toEqual([0,2,1])
    // Reflected edge arithmetic preserves the IEEE negative zero in normal Y.
    expect(Array.from(result.normals)).toEqual([0,-0,1])
    expect(release).toHaveBeenCalledTimes(1)
    expect(free).toHaveBeenCalledTimes(3)
    expect(real.exports.abi_array_field(release.mock.calls[0][0],1)).toBe(0)
    indices[2] = 99
    expect(() => prepareExportMeshInKernel(points,indices,matrix,false)).toThrow('out-of-range')
    expect(release).toHaveBeenCalledTimes(1)
    expect(free).toHaveBeenCalledTimes(6)
    points.fill(99)
    expect(result.positions[0]).toBe(2)
  } finally { runtime.mockRestore() }
})
