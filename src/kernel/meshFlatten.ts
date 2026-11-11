import type { MeshData } from '../core/mesh';

import { callGeometryRust } from './geometry/kernel';
import { normalizePolygonMesh, type PolygonMesh } from './geometry/polygon';

/** Rust owns placement, reflected winding and exact-coordinate welding.
 * Scene-owned typed views are encoded directly by valueBinaryCodec (wire-identical to the
 * equivalent plain arrays), so no temporary number[] copy of vertices/indices/transform
 * is materialized here. The decoded plain-array response is boxed into typed views once,
 * here, and flows into DirectBody.mesh as the canonical PolygonMesh representation. */
export function flattenGroupGeometry(meshes: readonly Pick<MeshData, 'vertices' | 'indices' | 'transform'>[]): PolygonMesh {
    return normalizePolygonMesh(callGeometryRust<PolygonMesh>('scene_flatten', { meshes: meshes.map(mesh => ({
        vertices: mesh.vertices, indices: mesh.indices, transform: mesh.transform,
    })) }));
}
/** A body record for the direct-modeling document, from scene mesh data. */
export function sceneBody(mesh: MeshData, index: number) {
    const m = flattenGroupGeometry([mesh]);
    return { id: String(index), name: `Body ${index + 1}`, mesh: { positions: m.positions, indices: m.indices } };
}
