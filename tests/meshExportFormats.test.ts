import { expect, it } from 'vitest';
import { exportMeshFormat, MESH_EXPORT_FORMATS } from '../src/services/meshExportFormats';
import { buildOwnNurbs } from '../src/services/modelGraphNurbsKernel';
import { MODELGRAPH_NURBS_SURFACE_EXAMPLE } from '../src/services/modelGraphNurbs';
import { OpenScadProject } from '../src/services/openScadProject';
import { parseOpenScad3mf, parseOpenScadAmf, parseOpenScadStl, parseOpenScadOff } from '../src/services/openScadImport';
const built = buildOwnNurbs(MODELGRAPH_NURBS_SURFACE_EXAMPLE, { action: 'build' });
if (!('mesh' in built) || !built.mesh)
    throw new Error('Missing mesh');
const mesh = built.mesh;
function file(data: Uint8Array, extension: string) { const p = new OpenScadProject({ entrypoint: 'main.scad', files: [{ kind: 'source', path: 'main.scad', source: '' }, { kind: 'blob', path: `mesh.${extension}`, data }] }); return p.read(`mesh.${extension}`)!; }
it.each(['stl', 'stl_binary', '3mf', 'amf', 'off'] as const)('roundtrips %s through the existing independent importer', async (format) => {
    const out = exportMeshFormat(mesh, format), input = file(out.data, out.extension);
    const decoded = format === '3mf' ? await parseOpenScad3mf(input) : format === 'amf' ? await parseOpenScadAmf(input) : format === 'off' ? parseOpenScadOff(input) : parseOpenScadStl(input);
    expect(decoded.triangleCount).toBe(mesh.indices.length / 3);
    const bounds = (values: ArrayLike<number>) => Array.from({ length: 3 }, (_, a) => { const v = Array.from(values).filter((_, i) => i % 3 === a); return [Math.min(...v), Math.max(...v)]; });
    const before = bounds(mesh.positions), after = bounds(decoded.vertices);
    before.forEach((v, i) => v.forEach((n, j) => expect(after[i][j]).toBeCloseTo(n, 5)));
});
it('writes correct OBJ indexing and PLY counts, deterministically', () => {
    const obj = new TextDecoder().decode(exportMeshFormat(mesh, 'obj').data).split('\n');
    expect(obj.filter(l => l.startsWith('v '))).toHaveLength(mesh.positions.length / 3);
    const faces = obj.filter(l => l.startsWith('f '));
    expect(faces).toHaveLength(mesh.indices.length / 3);
    expect(faces[0]).toBe('f ' + mesh.indices.slice(0, 3).map(i => i + 1).join(' '));
    const ply = new TextDecoder().decode(exportMeshFormat(mesh, 'ply').data);
    expect(ply).toContain(`element vertex ${mesh.positions.length / 3}`);
    expect(ply).toContain(`element face ${mesh.indices.length / 3}`);
    expect(exportMeshFormat(mesh, '3mf').data).toEqual(exportMeshFormat(mesh, '3mf').data);
});
it.each(MESH_EXPORT_FORMATS)('exposes %s through the own kernel artifact', format => {
    const out = buildOwnNurbs(MODELGRAPH_NURBS_SURFACE_EXAMPLE, { action: 'export', format });
    expect('artifact' in out && out.artifact?.format).toBe(format);
});
it('rejects an open printing mesh but exports it as OBJ', () => {
    const open = { ...mesh, indices: mesh.indices.slice(3) };
    expect(() => exportMeshFormat(open, '3mf')).toThrow('closed');
    expect(() => exportMeshFormat(open, 'amf')).toThrow('closed');
    expect(new TextDecoder().decode(exportMeshFormat(open, 'obj').data)).toContain('v ');
});
it('preserves mirrored world transforms when exporting the existing scene', async () => {
    const { HeadlessGeometryService } = await import('../src/mcp/geometryService');
    const { flattenExportMeshes } = await import('../src/services/meshExportAdapter');
    const result = await new HeadlessGeometryService().compile('translate([10,20,30])mirror([1,0,0])cube([2,3,4]);', 'full');
    const flat = flattenExportMeshes(result.meshes);
    expect(flat.report.signedVolumeMm3).toBeCloseTo(24);
    expect(Math.min(...flat.positions.filter((_, i) => i % 3 === 0))).toBe(8);
    expect(exportMeshFormat(flat, '3mf').data.length).toBeGreaterThan(100);
});
it('preserves separate 3MF objects and world placement through lossless compressed export', async () => {
    const { HeadlessGeometryService } = await import('../src/mcp/geometryService');
    const { flattenExportMeshes } = await import('../src/services/meshExportAdapter');
    const { exportMeshFormatCompressed } = await import('../src/services/meshExportFormats');
    const result = await new HeadlessGeometryService().compile('cube(2);translate([10,20,30])mirror([1,0,0])cube([2,3,4]);', 'full');
    const flat = flattenExportMeshes(result.meshes);
    expect(flat.parts).toHaveLength(2);
    const stored = exportMeshFormat(flat, '3mf');
    const xml = new TextDecoder().decode(stored.data);
    expect(xml.match(/<object id=/g)).toHaveLength(2);
    expect(xml.match(/<item objectid=/g)).toHaveLength(2);
    expect(xml).toContain('objectid="2"');
    const out = await exportMeshFormatCompressed(flat, '3mf');
    expect(out.data.length).toBeLessThan(stored.data.length);
    const decoded = await parseOpenScad3mf(file(out.data, '3mf'));
    const original = await parseOpenScad3mf(file(stored.data, '3mf'));
    expect(Array.from(decoded.vertices)).toEqual(Array.from(original.vertices));
    expect(Array.from(decoded.triangles)).toEqual(Array.from(original.triangles));
    expect(decoded.triangleCount).toBe(flat.indices.length / 3);
    expect(Math.max(...Array.from(decoded.vertices).filter((_, i) => i % 3 === 2))).toBe(34);
});
it('keeps touching scene parts independent instead of rejecting a welded aggregate', async () => {
    const { HeadlessGeometryService } = await import('../src/mcp/geometryService');
    const { flattenExportMeshes } = await import('../src/services/meshExportAdapter');
    const result = await new HeadlessGeometryService().compile('cube(2);translate([2,0,0])cube(2);', 'full');
    const flat = flattenExportMeshes(result.meshes);
    expect(flat.parts).toHaveLength(2);
    expect(new TextDecoder().decode(exportMeshFormat(flat, '3mf').data).match(/<object id=/g)).toHaveLength(2);
});
