import type { MeshData } from '../core/mesh'

import { prepareKernelScene, type PreparedVrMesh } from './vrKernel'
export type VrMesh = PreparedVrMesh

/** Select the visible publication; Rust owns transforms, bounds and VR scale. */
export function prepareVrScene(meshes: readonly Pick<MeshData, 'vertices' | 'indices' | 'transform' | 'color' | 'material'>[], visibility: readonly boolean[], isolated: boolean, selectedIndex: number | null): VrMesh[] {
  return prepareKernelScene(meshes.flatMap((mesh, index) => {
    if (visibility[index] === false || (isolated && selectedIndex !== index) || !mesh.indices.length) return []
    const base = mesh.material?.baseColor ?? [1, 1, 1]
    return [{ vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform,
      color: mesh.color.map((v, i) => i < 3 ? v * base[i] : v) }]
  }))
}

/** Adapter for Solid/Mesh workspaces, whose published polygon coordinates are world-space. */
export function prepareVrPolygons(meshes: readonly { positions: ArrayLike<number>; indices: ArrayLike<number>; color?: readonly number[] }[]): VrMesh[] {
  return prepareVrScene(meshes.map(mesh => {
    const vertices = new Float32Array(mesh.positions.length * 2)
    for (let i = 0; i < mesh.positions.length; i++) vertices[Math.floor(i / 3) * 6 + i % 3] = mesh.positions[i]
    return {
      vertices, indices: new Uint32Array(mesh.indices),
      transform: new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]),
      color: [...(mesh.color ?? [0.6, 0.7, 0.85]).slice(0, 3), 1] as [number, number, number, number],
    }
  }), [], false, null)
}
