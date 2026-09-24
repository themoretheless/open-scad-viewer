/** Scene export transport. Rust owns preparation, serialization and commit. */
import type { MeshData } from '../../core/mesh'
import { callGeometryRust, decodeNurbsResult, GeometryKernelError, kernelRuntime, kernelVertexFormat } from './kernel'

function append(handle: number, mesh: MeshData): void {
  const { exports: wasm, takeResponse } = kernelRuntime()
  const fmt = kernelVertexFormat(mesh.vertices)
  const matrixView = kernelVertexFormat(mesh.transform) === fmt ? mesh.transform : fmt === 1 ? new Float64Array(mesh.transform) : new Float32Array(mesh.transform)
  const allocations: Array<[number, number]> = []
  const upload = (view: Float32Array | Float64Array | Uint32Array) => {
    if (!view.byteLength) return 0
    const ptr = wasm.abi_export_alloc(view.byteLength)
    if (!ptr) throw new GeometryKernelError('GEOMETRY_RESOURCE_LIMIT', 'Mesh export exceeds transport limit')
    allocations.push([ptr, view.byteLength])
    new Uint8Array(kernelRuntime().memory.buffer, ptr, view.byteLength).set(new Uint8Array(view.buffer, view.byteOffset, view.byteLength))
    return ptr
  }
  try {
    const vp = upload(mesh.vertices), ip = upload(mesh.indices), mp = upload(matrixView)
    decodeNurbsResult(takeResponse(wasm.abi_export_append(handle,vp,mesh.vertices.length,ip,mesh.indices.length,mp,matrixView.length,fmt)))
  } finally { for (const [ptr, length] of allocations) wasm.abi_free(ptr,length) }
}

export function exportSceneInKernel(meshes: readonly MeshData[], format: 'stl' | 'obj', name: string): Uint8Array {
  const handle = callGeometryRust<number>('mesh_export_file', { action: 'begin', format, name })
  let result = 0
  const { exports: wasm } = kernelRuntime()
  try {
    for (const mesh of meshes) append(handle,mesh)
    result = callGeometryRust<number>('mesh_export_file', { action: 'finish', handle })
    const ptr = wasm.abi_array_field(result,0), len = wasm.abi_array_field(result,1)
    return new Uint8Array(kernelRuntime().memory.buffer,ptr,len).slice()
  } finally {
    if (result) wasm.abi_array_free(result)
    callGeometryRust('mesh_export_file', { action: 'dispose', handle })
  }
}
