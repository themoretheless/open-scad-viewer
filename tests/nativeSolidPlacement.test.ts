import { expect, it, vi } from 'vitest'
import * as kernel from '../src/services/geometry/kernel'
import { placeSolidMeshInKernel } from '../src/services/geometry/meshAnalysis'

const vertices = () => new Float32Array([0,0,0,0,0,1, 1,0,0,0,0,1, 0,1,0,0,0,1])
const matrix = () => new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1])

it('uploads offset views once, returns f64 positions and reverses reflected winding', () => {
  const points = new Float32Array([99, ...vertices(), 99]).subarray(1,19)
  const indices = new Uint32Array([99,0,1,2,99]).subarray(1,4)
  const storage = new Float32Array([99,...matrix(),99])
  const transform = storage.subarray(1,17)
  transform[0] = -2; transform[3] = 0.1
  const result = placeSolidMeshInKernel(points, indices, transform)!
  expect(result.indices).toEqual([0,2,1])
  const expected = -2 + transform[3]
  expect(result.positions[3]).toBe(expected)
  expect(result.positions[3]).not.toBe(Math.fround(expected))
  points.fill(99); indices.fill(99); transform.fill(99)
  expect(result.positions[3]).toBe(expected)
  expect(result.indices).toEqual([0,2,1])
})

it('frees raw inputs and the result on success, and raw inputs on native refusal', () => {
  const real = kernel.kernelRuntime()
  const freeArray = vi.fn((handle: number) => real.exports.abi_array_free(handle))
  const freeInput = vi.fn((ptr: number, len: number) => real.exports.abi_free(ptr,len))
  const runtime = vi.spyOn(kernel,'kernelRuntime').mockImplementation(() => ({ ...real,
    exports: { ...real.exports, abi_array_free: freeArray, abi_free: freeInput },
  }))
  try {
    const points = vertices(), indices = new Uint32Array([0,1,2]), transform = matrix()
    expect(placeSolidMeshInKernel(points,indices,transform)).not.toBeNull()
    expect(freeArray).toHaveBeenCalledTimes(1)
    expect(freeInput).toHaveBeenCalledTimes(3)
    const handle = freeArray.mock.calls[0][0]
    expect(real.exports.abi_array_field(handle,1)).toBe(0)
    transform[0] = 0
    expect(() => placeSolidMeshInKernel(points,indices,transform)).toThrow('singular scene transform')
    expect(freeArray).toHaveBeenCalledTimes(1)
    expect(freeInput).toHaveBeenCalledTimes(6)
  } finally { runtime.mockRestore() }
})

it('keeps empty conversions empty and refuses malformed triangles and coordinate overflow', () => {
  expect(placeSolidMeshInKernel(new Float32Array(),new Uint32Array(),matrix())).toBeNull()
  expect(() => placeSolidMeshInKernel(vertices(),new Uint32Array([0,1,99]),matrix())).toThrow('valid vertex indices')
  expect(() => placeSolidMeshInKernel(vertices(),new Uint32Array([0,1,2,0]),matrix())).toThrow('complete triangles')
  const transform = matrix(); transform[3] = 1e7
  expect(() => placeSolidMeshInKernel(vertices(),new Uint32Array([0,1,2]),transform)).toThrow('coordinate bounds')
})
