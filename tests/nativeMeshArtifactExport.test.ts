import { expect, it, vi } from 'vitest'
import * as kernel from '../src/services/geometry/kernel'
import { exportMeshArtifactInKernel } from '../src/services/geometry/meshArtifactExport'

const mesh = { positions: [0,0,0,2,0,0,0,3,0], indices: [0,1,2] }

it('copies native file bytes before releasing their result handle', () => {
  const real = kernel.kernelRuntime()
  const release = vi.fn((handle: number) => real.exports.abi_array_free(handle))
  const spy = vi.spyOn(kernel, 'kernelRuntime').mockImplementation(() => ({ ...real, exports: { ...real.exports, abi_array_free: release } }))
  try {
    const bytes = exportMeshArtifactInKernel(mesh, 'obj')
    expect(release).toHaveBeenCalledTimes(1)
    expect(real.exports.abi_array_field(release.mock.calls[0][0], 1)).toBe(0)
    exportMeshArtifactInKernel(mesh, 'ply')
    expect(new TextDecoder().decode(bytes)).toBe('# ModelGraph; units: millimeter\nv 0 0 0\nv 2 0 0\nv 0 3 0\nf 1 2 3\n')
    expect(() => exportMeshArtifactInKernel(mesh, 'amf')).toThrow('closed')
    expect(release).toHaveBeenCalledTimes(2)
  } finally { spy.mockRestore() }
})

it('refuses invalid topology in native admission for every non-3MF format', () => {
  for (const format of ['stl','stl_binary','obj','ply','off','amf'] as const) {
    expect(() => exportMeshArtifactInKernel({ ...mesh, indices: [0,1,9] }, format)).toThrow('Malformed')
    expect(() => exportMeshArtifactInKernel({ ...mesh, indices: [0,0,1] }, format)).toThrow('topology')
  }
})

it('preserves finite binary64 coordinates in text formats and bounds native output', () => {
  const positions = [1e-7,1e21,-0, Math.PI,Number.MIN_VALUE,-Number.MAX_VALUE]
  const obj = new TextDecoder().decode(exportMeshArtifactInKernel({ positions, indices: [] }, 'obj'))
  const decoded = obj.split('\n').filter(line => line.startsWith('v ')).flatMap(line => line.slice(2).split(' ').map(Number))
  expect(decoded).toEqual(positions.map(value => value === 0 ? 0 : value))
  expect(obj).toContain('1e-7 1e+21 0')
  // Unreferenced vertices are legal and preserved, but cannot bypass artifact admission.
  const large = { positions: new Array<number>(900_000).fill(Math.PI), indices: [] }
  expect(() => exportMeshArtifactInKernel(large, 'obj')).toThrow('4 MiB')
  expect(new TextDecoder().decode(exportMeshArtifactInKernel(mesh, 'off'))).toContain('3 0 1 2')
})
