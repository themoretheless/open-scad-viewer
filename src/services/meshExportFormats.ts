import { export3mfInKernel, exportMeshArtifactInKernel } from './geometry/meshArtifactExport';
import { type NurbsMesh } from './geometry/tessellation';
export const MESH_EXPORT_FORMATS = ['stl', 'stl_binary', '3mf', 'obj', 'ply', 'off', 'amf'] as const;
export type MeshExportFormat = typeof MESH_EXPORT_FORMATS[number];
type ExportMesh = NurbsMesh & { parts?: NurbsMesh[] };
function exportUnchecked(mesh: ExportMesh, format: MeshExportFormat, compressed = false) {
    if (!MESH_EXPORT_FORMATS.includes(format))
        throw new Error('Unsupported mesh export format.');
    if (format !== '3mf') {
        const data = exportMeshArtifactInKernel(mesh, format);
        const mimeType = format === 'stl' || format === 'stl_binary' ? 'model/stl'
            : format === 'obj' ? 'model/obj' : format === 'amf' ? 'application/amf+xml' : 'application/octet-stream';
        return { data, mimeType, extension: format === 'stl_binary' ? 'stl' : format };
    }
    const data = export3mfInKernel(mesh, compressed);
    return { data, mimeType: 'model/3mf', extension: '3mf' };
}
export function meshExportBase64(data: Uint8Array) { let binary = ''; for (let i = 0; i < data.length; i += 8192)
    binary += String.fromCharCode(...data.subarray(i, i + 8192)); return btoa(binary); }

function checkArtifact<T extends { data: Uint8Array }>(artifact: T): T {
    if (artifact.data.length > 4 * 1024 * 1024) throw new Error('Export exceeds 4 MiB.');
    return artifact;
}
export function exportMeshFormat(mesh: ExportMesh, format: MeshExportFormat) {
    return checkArtifact(exportUnchecked(mesh, format));
}
/** Rust owns model serialization, CRC32, OPC ZIP and lossless DEFLATE. */
export async function exportMeshFormatCompressed(mesh: ExportMesh, format: MeshExportFormat) {
    return checkArtifact(exportUnchecked(mesh, format, true));
}
