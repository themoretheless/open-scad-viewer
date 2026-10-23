import { export3mfInKernel, exportMeshArtifactInKernel } from './geometry/meshArtifactExport';
import { type NurbsMesh } from './geometry/tessellation';
import { MESH_EXPORT_FORMATS, type MeshExportFormat } from './meshFormats';
export { MESH_EXPORT_FORMATS, type MeshExportFormat };
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
export function meshExportBase64(data: Uint8Array) {
    const nodeBuffer = (globalThis as { Buffer?: { from(buf: ArrayBufferLike, byteOffset: number, byteLength: number): { toString(enc: 'base64'): string } } }).Buffer;
    if (nodeBuffer) {
        return nodeBuffer.from(data.buffer, data.byteOffset, data.byteLength).toString('base64');
    }
    const chunks: string[] = [];
    for (let i = 0; i < data.length; i += 8192) {
        chunks.push(String.fromCharCode.apply(null, data.subarray(i, i + 8192) as unknown as number[]));
    }
    return btoa(chunks.join(''));
}

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
