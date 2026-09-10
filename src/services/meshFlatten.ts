import type { MeshData } from '../core/mesh';

/** Kernel-free mesh flattening for non-export paths (the export report needs
 * inspectNurbsMesh and lives in meshExportAdapter). Same transform math. */
export function flattenGroupGeometry(meshes: readonly MeshData[]): { positions: number[]; indices: number[] } {
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
            const a = map[mesh.indices[i]!]!, b = map[mesh.indices[i + 1]!]!, c = map[mesh.indices[i + 2]!]!;
            indices.push(a, ...(determinant < 0 ? [c, b] : [b, c]));
        }
    }
    return { positions, indices };
}
/** A body record for the direct-modeling document, from scene mesh data. */
export function sceneBody(mesh: MeshData, index: number) {
    const m = flattenGroupGeometry([mesh]);
    return { id: String(index), name: `Body ${index + 1}`, mesh: { positions: m.positions, indices: m.indices } };
}
