import type { MeshData } from '../core/mesh';

import { callGeometryRust } from './geometry/kernel';

/** Rust owns placement, reflected winding and exact-coordinate welding. */
export function flattenGroupGeometry(meshes: readonly Pick<MeshData, 'vertices' | 'indices' | 'transform'>[]): { positions: number[]; indices: number[] } {
    return callGeometryRust('scene_flatten', { meshes: meshes.map(mesh => ({
        vertices: Array.from(mesh.vertices), indices: Array.from(mesh.indices), transform: Array.from(mesh.transform),
    })) });
}
/** A body record for the direct-modeling document, from scene mesh data. */
export function sceneBody(mesh: MeshData, index: number) {
    const m = flattenGroupGeometry([mesh]);
    return { id: String(index), name: `Body ${index + 1}`, mesh: { positions: m.positions, indices: m.indices } };
}
