import type { MeshData } from '../core/mesh'
import { exportSceneInKernel } from './geometry/meshFileExport'
import { GeometryKernelError } from './geometry/kernel'

export const MAX_EXPORT_TRIANGLES = 750_000

export class MeshExportError extends Error {
  constructor(readonly code: 'invalid-mesh' | 'too-many-triangles', message: string) {
    super(message)
    this.name = 'MeshExportError'
  }
}

function exportScene(meshes: readonly MeshData[], format: 'stl' | 'obj', name: string) {
  try { return exportSceneInKernel(meshes,format,name) }
  catch (error) {
    const code = error instanceof GeometryKernelError && error.code === 'MESH_EXPORT_TOO_MANY_TRIANGLES' ? 'too-many-triangles' : 'invalid-mesh'
    throw new MeshExportError(code,error instanceof Error ? error.message : String(error))
  }
}

/** Ready-to-save standard binary STL, generated entirely by Rust. */
export function buildBinaryStl(meshes: readonly MeshData[], name = 'OpenSCAD Viewer'): Uint8Array {
  return exportScene(meshes,'stl',name)
}

/** Ready-to-save OBJ; the host only decodes the native UTF-8 artifact. */
export function buildObj(meshes: readonly MeshData[]): string {
  return new TextDecoder().decode(exportScene(meshes,'obj',''))
}
