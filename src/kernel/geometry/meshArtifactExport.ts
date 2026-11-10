/** Native mesh artifact transport. The host only copies the committed byte buffer. */
import { callGeometryRust, kernelRuntime } from './kernel'
type Mesh = { positions: ArrayLike<number>; indices: ArrayLike<number> }

function copyArtifact(result: number): Uint8Array {
  const { exports: wasm } = kernelRuntime()
  try {
    const ptr = wasm.abi_array_field(result, 0), length = wasm.abi_array_field(result, 1)
    return new Uint8Array(kernelRuntime().memory.buffer, ptr, length).slice()
  } finally { wasm.abi_array_free(result) }
}

export function export3mfModelInKernel(mesh: Mesh & { parts?: Mesh[] }): Uint8Array {
  const geometry = (part: Mesh) => ({ positions: part.positions, indices: part.indices })
  return copyArtifact(callGeometryRust<number>('mesh_export_3mf_model', {
    mesh: geometry(mesh), parts: (mesh.parts ?? []).map(geometry),
  }))
}

export function export3mfInKernel(mesh: Mesh & { parts?: Mesh[] }, compressed: boolean): Uint8Array {
  const geometry = (part: Mesh) => ({ positions: part.positions, indices: part.indices })
  return copyArtifact(callGeometryRust<number>('mesh_export_3mf', {
    mesh: geometry(mesh), parts: (mesh.parts ?? []).map(geometry), compressed,
  }))
}

export function exportMeshArtifactInKernel(
  mesh: { positions: ArrayLike<number>; indices: ArrayLike<number> },
  format: 'stl' | 'stl_binary' | 'obj' | 'ply' | 'off' | 'amf',
): Uint8Array {
  const result = callGeometryRust<number>('mesh_export_format', {
    mesh: { positions: mesh.positions, indices: mesh.indices }, format,
  })
  return copyArtifact(result)
}
