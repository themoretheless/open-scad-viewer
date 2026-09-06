import type { MeshData } from '../core/mesh';
import { inspectNurbsMesh, type NurbsMesh } from './nurbsTessellation';
function flattenGroup(meshes: readonly MeshData[]): NurbsMesh {
    if (meshes.reduce((n, m) => n + m.indices.length / 3, 0) > 100000)
        throw new Error('Mesh export exceeds 100000 triangles.');
    const positions: number[] = [], indices: number[] = [], vertices = new Map<string, number>();
    for (const mesh of meshes) {
        const t = mesh.transform, map: number[] = [];
        for (let i = 0; i < mesh.vertices.length; i += 6) {
            const p = [0, 1, 2].map(r => t[r * 4 + 3] + t[r * 4] * mesh.vertices[i] + t[r * 4 + 1] * mesh.vertices[i + 1] + t[r * 4 + 2] * mesh.vertices[i + 2]);
            const key = p.join(',');
            let j = vertices.get(key);
            if (j === undefined) {
                j = positions.length / 3;
                positions.push(...p);
                vertices.set(key, j);
            }
            map.push(j);
        }
        const determinant = t[0] * (t[5] * t[10] - t[6] * t[9]) - t[1] * (t[4] * t[10] - t[6] * t[8]) + t[2] * (t[4] * t[9] - t[5] * t[8]);
        for (let i = 0; i < mesh.indices.length; i += 3) {
            const a = map[mesh.indices[i]], b = map[mesh.indices[i + 1]], c = map[mesh.indices[i + 2]];
            indices.push(a, ...(determinant < 0 ? [c, b] : [b, c]));
        }
    }
    const report = inspectNurbsMesh(positions, indices);
    return { positions, indices, report: { ...report, construction: 'sampled_surface', errorBoundCertified: false, selfIntersectionStatus: 'not_checked' } };
}

/** Preserve scene objects for 3MF while keeping the flat mesh for legacy formats. */
export function flattenExportMeshes(meshes: readonly MeshData[]): NurbsMesh & { parts: NurbsMesh[] } {
    const combined = flattenGroup(meshes);
    return { ...combined, parts: meshes.filter(m => m.indices.length > 0).map(m => flattenGroup([m])) };
}
