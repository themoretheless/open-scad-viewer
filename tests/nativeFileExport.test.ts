import { expect, it, vi } from 'vitest'
import * as kernel from '../src/services/geometry/kernel'
import { buildBinaryStl, buildObj } from '../src/services/meshExport'
import type { MeshData } from '../src/core/mesh'

function mesh(): MeshData {
  // Export consumes only these three buffers.
  return { vertices: new Float32Array([0,0,0,0,0,1, 1,0,0,0,0,1, 0,1,0,0,0,1]),
    indices: new Uint32Array([0,1,2]), transform: new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]),
  } as MeshData
}

it('poisons raw transport refusal and never reuses a finished session handle', () => {
  const id = kernel.callGeometryRust<number>('mesh_export_file',{ action:'begin',format:'obj',name:'' })
  const runtime = kernel.kernelRuntime()
  expect(() => kernel.decodeNurbsResult(runtime.takeResponse(runtime.exports.abi_export_append(id,0,64*1024*1024/4+1,0,0,0,0)))).toThrow('transport limit')
  expect(() => kernel.callGeometryRust('mesh_export_file',{action:'finish',handle:id})).toThrow('failed export session')
  const next = kernel.callGeometryRust<number>('mesh_export_file',{action:'begin',format:'obj',name:''})
  try { expect(next).toBeGreaterThan(id) }
  finally { kernel.callGeometryRust('mesh_export_file',{action:'dispose',handle:next}) }
})

it('aborts failed multi-mesh exports without exhausting native session slots', () => {
  const valid = mesh(), invalid = mesh()
  invalid.indices[2] = 99
  for (let i = 0; i < 12; i++) expect(() => buildObj([valid,invalid])).toThrow('out-of-range')
  expect(buildObj([valid,valid])).toBe('# Exported by OpenSCAD Viewer\no result_1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\no result_2\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 4 5 6\n')
})

it('enforces the cumulative scene triangle ceiling including degenerate input', () => {
  const large = mesh(); large.indices = new Uint32Array(750000*3)
  expect(() => buildBinaryStl([large,mesh()])).toThrowError(expect.objectContaining({code:'too-many-triangles'}))
  expect(buildBinaryStl([mesh()]).length).toBe(134)
})

it('copies the committed file before freeing the native artifact', () => {
  const real = kernel.kernelRuntime()
  const release = vi.fn((handle: number) => real.exports.abi_array_free(handle))
  const runtime = vi.spyOn(kernel,'kernelRuntime').mockImplementation(() => ({...real,exports:{...real.exports,abi_array_free:release}}))
  try {
    const data = buildBinaryStl([mesh()],'Я😀'.repeat(80))
    expect(release).toHaveBeenCalledTimes(1)
    expect(real.exports.abi_array_field(release.mock.calls[0][0],1)).toBe(0)
    expect(new DataView(data.buffer).getUint32(80,true)).toBe(1)
    expect(() => new TextDecoder('utf-8',{fatal:true}).decode(data.subarray(0,80))).not.toThrow()
  } finally {runtime.mockRestore()}
})
