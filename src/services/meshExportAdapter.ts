import type { MeshData } from '../core/mesh';
import { flattenGroupGeometry } from './meshFlatten';
import { inspectNurbsMesh, type NurbsMesh } from './nurbsTessellation';
function flattenGroup(meshes: readonly MeshData[]): NurbsMesh {
    const { positions, indices } = flattenGroupGeometry(meshes);
    const report = inspectNurbsMesh(positions, indices);
    return { positions, indices, report: { ...report, construction: 'sampled_surface', errorBoundCertified: false, selfIntersectionStatus: 'not_checked' } };
}

/** Preserve scene objects for 3MF while keeping the flat mesh for legacy formats. */
export function flattenExportMeshes(meshes: readonly MeshData[]): NurbsMesh & { parts: NurbsMesh[] } {
    const combined = flattenGroup(meshes);
    return { ...combined, parts: meshes.filter(m => m.indices.length > 0).map(m => flattenGroup([m])) };
}
