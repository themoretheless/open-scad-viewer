import { exportNurbsStl, inspectNurbsMesh, type NurbsMesh } from './geometry/tessellation';
export const MESH_EXPORT_FORMATS = ['stl', 'stl_binary', '3mf', 'obj', 'ply', 'off', 'amf'] as const;
export type MeshExportFormat = typeof MESH_EXPORT_FORMATS[number];
const utf8 = (s: string) => new TextEncoder().encode(s);
function crc32(bytes: Uint8Array) { let crc = 0xffffffff; for (const b of bytes) {
    crc ^= b;
    for (let i = 0; i < 8; i++)
        crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
} return (crc ^ 0xffffffff) >>> 0; }
/** Deterministic OPC ZIP, stored entries, UTF-8 names; no compression dependency. */
function zip(files: Array<[string, string]>, compressed?: Uint8Array[]) {
    const chunks: Uint8Array[] = [], central: Uint8Array[] = [];
    let offset = 0;
    for (const [entryIndex, [path, text]] of files.entries()) {
        const name = utf8(path), data = utf8(text), crc = crc32(data), local = new Uint8Array(30 + name.length), v = new DataView(local.buffer);
        v.setUint32(0, 0x04034b50, true);
        v.setUint16(4, 20, true);
        v.setUint16(6, 0x800, true);
        v.setUint16(12, 33, true);
        v.setUint32(14, crc, true);
        v.setUint32(18, data.length, true);
        v.setUint32(22, data.length, true);
        v.setUint16(26, name.length, true);
        local.set(name, 30);
        const payload = compressed?.[entryIndex] ?? data;
        v.setUint16(8, compressed ? 8 : 0, true);
        v.setUint32(18, payload.length, true);
        const c = new Uint8Array(46 + name.length), d = new DataView(c.buffer);
        d.setUint32(0, 0x02014b50, true);
        d.setUint16(4, 20, true);
        d.setUint16(6, 20, true);
        d.setUint16(8, 0x800, true);
        d.setUint16(14, 33, true);
        d.setUint32(16, crc, true);
        d.setUint16(10, compressed ? 8 : 0, true);
        d.setUint32(20, payload.length, true);
        d.setUint32(24, data.length, true);
        d.setUint16(28, name.length, true);
        d.setUint32(42, offset, true);
        c.set(name, 46);
        chunks.push(local, payload);
        central.push(c);
        offset += local.length + payload.length;
    }
    const directorySize = central.reduce((n, b) => n + b.length, 0), end = new Uint8Array(22), v = new DataView(end.buffer);
    v.setUint32(0, 0x06054b50, true);
    v.setUint16(8, files.length, true);
    v.setUint16(10, files.length, true);
    v.setUint32(12, directorySize, true);
    v.setUint32(16, offset, true);
    const result = new Uint8Array(offset + directorySize + 22);
    let cursor = 0;
    for (const c of [...chunks, ...central, end]) {
        result.set(c, cursor);
        cursor += c.length;
    }
    return result;
}
type ExportMesh = NurbsMesh & { parts?: NurbsMesh[] };
function exportUnchecked(mesh: ExportMesh, format: MeshExportFormat) {
    if (!MESH_EXPORT_FORMATS.includes(format))
        throw new Error('Unsupported mesh export format.');
    if (mesh.indices.length / 3 > 100000)
        throw new Error('Mesh export exceeds 100000 triangles.');
    const topology = inspectNurbsMesh(mesh.positions, mesh.indices);
    if (format !== '3mf' && (topology.degenerateTriangles || topology.nonManifoldEdges || topology.orientationConflicts))
        throw new Error('Mesh has invalid or inconsistent topology.');
    if (['stl', 'stl_binary', 'amf'].includes(format) && (!topology.closed || topology.signedVolumeMm3 <= 0))
        throw new Error('Printing export requires a closed oriented mesh with positive volume.');
    const points = Array.from({ length: mesh.positions.length / 3 }, (_, i) => mesh.positions.slice(i * 3, i * 3 + 3)), faces = Array.from({ length: mesh.indices.length / 3 }, (_, i) => mesh.indices.slice(i * 3, i * 3 + 3));
    let data: Uint8Array, mimeType: string, extension: string = format === 'stl_binary' ? 'stl' : format;
    if (format === 'stl') {
        data = utf8(exportNurbsStl(mesh));
        mimeType = 'model/stl';
    }
    else if (format === 'stl_binary') {
        if (inspectNurbsMesh(mesh.positions.map(Math.fround), mesh.indices).degenerateTriangles)
            throw new Error('Binary STL precision would collapse triangles; use ASCII STL or 3MF.');
        data = new Uint8Array(84 + faces.length * 50);
        data.set(utf8('ModelGraph mesh; coordinates in millimeters'));
        const view = new DataView(data.buffer);
        view.setUint32(80, faces.length, true);
        faces.forEach((f, i) => { const [a, b, c] = f.map(j => points[j]), u = b.map((v, k) => v - a[k]), v = c.map((v, k) => v - a[k]), n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]], len = Math.hypot(...n); [...n.map(x => x / len), ...a, ...b, ...c].forEach((x, j) => { if (!Number.isFinite(Math.fround(x)))
            throw new Error('STL float32 range exceeded.'); view.setFloat32(84 + i * 50 + j * 4, x, true); }); });
        mimeType = 'model/stl';
    }
    else if (format === 'obj') {
        data = utf8('# ModelGraph; units: millimeter\n' + points.map(p => 'v ' + p.join(' ')).join('\n') + '\n' + faces.map(f => 'f ' + f.map(i => i + 1).join(' ')).join('\n') + '\n');
        mimeType = 'model/obj';
    }
    else if (format === 'ply') {
        data = utf8(`ply\nformat ascii 1.0\ncomment units millimeter\nelement vertex ${points.length}\nproperty double x\nproperty double y\nproperty double z\nelement face ${faces.length}\nproperty list uchar int vertex_indices\nend_header\n` + points.map(p => p.join(' ')).join('\n') + '\n' + faces.map(f => '3 ' + f.join(' ')).join('\n') + '\n');
        mimeType = 'application/octet-stream';
    }
    else if (format === 'off') {
        data = utf8(`OFF\n${points.length} ${faces.length} 0\n` + points.map(p => p.join(' ')).join('\n') + '\n' + faces.map(f => '3 ' + f.join(' ')).join('\n') + '\n');
        mimeType = 'application/octet-stream';
    }
    else if (format === 'amf') {
        data = utf8('<?xml version="1.0" encoding="UTF-8"?><amf unit="millimeter" version="1.1"><object id="0"><mesh><vertices>' + points.map(p => `<vertex><coordinates><x>${p[0]}</x><y>${p[1]}</y><z>${p[2]}</z></coordinates></vertex>`).join('') + '</vertices><volume>' + faces.map(f => `<triangle><v1>${f[0]}</v1><v2>${f[1]}</v2><v3>${f[2]}</v3></triangle>`).join('') + '</volume></mesh></object></amf>');
        mimeType = 'application/amf+xml';
    }
    else {
        const parts = mesh.parts?.length ? mesh.parts : [mesh];
        const objects = parts.map((part, i) => {
            const report = inspectNurbsMesh(part.positions, part.indices);
            if (!report.closed || report.signedVolumeMm3 <= 0 || report.degenerateTriangles || report.nonManifoldEdges || report.orientationConflicts)
                throw new Error('Each 3MF object must be a closed oriented mesh with positive volume.');
            const vertices = Array.from({length: part.positions.length / 3}, (_, j) => `<vertex x="${part.positions[j*3]}" y="${part.positions[j*3+1]}" z="${part.positions[j*3+2]}"/>`).join('');
            const triangles = Array.from({length: part.indices.length / 3}, (_, j) => `<triangle v1="${part.indices[j*3]}" v2="${part.indices[j*3+1]}" v3="${part.indices[j*3+2]}"/>`).join('');
            return `<object id="${i+1}" name="Part ${i+1}" type="model"><mesh><vertices>${vertices}</vertices><triangles>${triangles}</triangles></mesh></object>`;
        }).join('');
        const model = '<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><resources>' + objects + '</resources><build>' + parts.map((_, i) => `<item objectid="${i+1}"/>`).join('') + '</build></model>';
        data = zip([['[Content_Types].xml', '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>'], ['_rels/.rels', '<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>'], ['3D/3dmodel.model', model]]);
        mimeType = 'model/3mf';
    }
    return { data, mimeType, extension };
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
/** Standard lossless ZIP DEFLATE, using the browser/Node native codec. */
export async function exportMeshFormatCompressed(mesh: ExportMesh, format: MeshExportFormat) {
    if (format !== '3mf') return exportMeshFormat(mesh, format);
    const artifact = exportUnchecked(mesh, format);
    const view = new DataView(artifact.data.buffer, artifact.data.byteOffset, artifact.data.byteLength);
    const files: Array<[string, string]> = [], compressed: Uint8Array[] = [];
    const decoder = new TextDecoder();
    let offset = 0;
    while (view.getUint32(offset, true) === 0x04034b50) {
        const size = view.getUint32(offset + 18, true), nameLength = view.getUint16(offset + 26, true);
        const start = offset + 30 + nameLength;
        const bytes = artifact.data.slice(start, start + size);
        files.push([decoder.decode(artifact.data.subarray(offset + 30, start)), decoder.decode(bytes)]);
        const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate-raw'));
        compressed.push(new Uint8Array(await new Response(stream).arrayBuffer()));
        offset = start + size;
    }
    return checkArtifact({ ...artifact, data: zip(files, compressed) });
}
