import { expect, it } from 'vitest'
import { exportMeshFormat, MESH_EXPORT_FORMATS } from '../src/exchange/meshExportFormats'
import { buildOwnNurbs } from '../src/language/rushGraphNurbsKernel'
import { RUSH_GRAPH_NURBS_SURFACE_EXAMPLE } from '../src/language/rushGraphNurbs'
import { OpenScadProject } from '../src/language/openScadProject'
import { parseOpenScad3mf, parseOpenScadAmf, parseOpenScadStl, parseOpenScadOff } from '../src/exchange/openScadImport'
const built = buildOwnNurbs(RUSH_GRAPH_NURBS_SURFACE_EXAMPLE, { action: 'build' })
if (!('mesh' in built) || !built.mesh) throw new Error('Missing mesh')
const mesh = built.mesh
function file(data: Uint8Array, extension: string) {
  const p = new OpenScadProject({
    entrypoint: 'main.scad',
    files: [
      { kind: 'source', path: 'main.scad', source: '' },
      { kind: 'blob', path: `mesh.${extension}`, data },
    ],
  })
  return p.read(`mesh.${extension}`)!
}
it.each(['stl', 'stl_binary', '3mf', 'amf', 'off'] as const)(
  'roundtrips %s through the existing independent importer',
  async format => {
    const out = exportMeshFormat(mesh, format),
      input = file(out.data, out.extension)
    const decoded =
      format === '3mf'
        ? await parseOpenScad3mf(input)
        : format === 'amf'
          ? await parseOpenScadAmf(input)
          : format === 'off'
            ? parseOpenScadOff(input)
            : parseOpenScadStl(input)
    expect(decoded.triangleCount).toBe(mesh.indices.length / 3)
    const bounds = (values: ArrayLike<number>) =>
      Array.from({ length: 3 }, (_, a) => {
        const v = Array.from(values).filter((_, i) => i % 3 === a)
        return [Math.min(...v), Math.max(...v)]
      })
    const before = bounds(mesh.positions),
      after = bounds(decoded.vertices)
    before.forEach((v, i) => v.forEach((n, j) => expect(after[i][j]).toBeCloseTo(n, 5)))
  },
)
it('writes correct OBJ indexing and PLY counts, deterministically', () => {
  const obj = new TextDecoder().decode(exportMeshFormat(mesh, 'obj').data).split('\n')
  expect(obj.filter(l => l.startsWith('v '))).toHaveLength(mesh.positions.length / 3)
  const faces = obj.filter(l => l.startsWith('f '))
  expect(faces).toHaveLength(mesh.indices.length / 3)
  expect(faces[0]).toBe(
    'f ' +
      mesh.indices
        .slice(0, 3)
        .map(i => i + 1)
        .join(' '),
  )
  const ply = new TextDecoder().decode(exportMeshFormat(mesh, 'ply').data)
  expect(ply).toContain(`element vertex ${mesh.positions.length / 3}`)
  expect(ply).toContain(`element face ${mesh.indices.length / 3}`)
  expect(exportMeshFormat(mesh, '3mf').data).toEqual(exportMeshFormat(mesh, '3mf').data)
})
it.each(MESH_EXPORT_FORMATS)('exposes %s through the own kernel artifact', format => {
  const out = buildOwnNurbs(RUSH_GRAPH_NURBS_SURFACE_EXAMPLE, { action: 'export', format })
  expect('artifact' in out && out.artifact?.format).toBe(format)
})
it('rejects an open printing mesh but exports it as OBJ', () => {
  const open = { ...mesh, indices: mesh.indices.slice(3) }
  expect(() => exportMeshFormat(open, '3mf')).toThrow('closed')
  expect(() => exportMeshFormat(open, 'amf')).toThrow('closed')
  expect(new TextDecoder().decode(exportMeshFormat(open, 'obj').data)).toContain('v ')
})
it('preserves mirrored world transforms when exporting the existing scene', async () => {
  const { HeadlessGeometryService } = await import('../src/mcp/geometryService')
  const { flattenExportMeshes } = await import('../src/exchange/meshExportAdapter')
  const result = await new HeadlessGeometryService().compile(
    'translate([10,20,30])mirror([1,0,0])cube([2,3,4]);',
    'full',
  )
  const flat = flattenExportMeshes(result.meshes)
  expect(flat.report.signedVolumeMm3).toBeCloseTo(24)
  expect(Math.min(...flat.positions.filter((_, i) => i % 3 === 0))).toBe(8)
  expect(exportMeshFormat(flat, '3mf').data.length).toBeGreaterThan(100)
})
it('preserves separate 3MF objects and world placement through lossless compressed export', async () => {
  const { HeadlessGeometryService } = await import('../src/mcp/geometryService')
  const { flattenExportMeshes } = await import('../src/exchange/meshExportAdapter')
  const { exportMeshFormatCompressed } = await import('../src/exchange/meshExportFormats')
  const result = await new HeadlessGeometryService().compile(
    'cube(2);translate([10,20,30])mirror([1,0,0])cube([2,3,4]);',
    'full',
  )
  const flat = flattenExportMeshes(result.meshes)
  expect(flat.parts).toHaveLength(2)
  const stored = exportMeshFormat(flat, '3mf')
  const xml = new TextDecoder().decode(stored.data)
  expect(xml.match(/<object id=/g)).toHaveLength(2)
  expect(xml.match(/<item objectid=/g)).toHaveLength(2)
  expect(xml).toContain('objectid="2"')
  const out = await exportMeshFormatCompressed(flat, '3mf')
  expect(out.data.length).toBeLessThan(stored.data.length)
  const decoded = await parseOpenScad3mf(file(out.data, '3mf'))
  const original = await parseOpenScad3mf(file(stored.data, '3mf'))
  expect(Array.from(decoded.vertices)).toEqual(Array.from(original.vertices))
  expect(Array.from(decoded.triangles)).toEqual(Array.from(original.triangles))
  expect(decoded.triangleCount).toBe(flat.indices.length / 3)
  expect(Math.max(...Array.from(decoded.vertices).filter((_, i) => i % 3 === 2))).toBe(34)
})
it('keeps touching scene parts independent instead of rejecting a welded aggregate', async () => {
  const { HeadlessGeometryService } = await import('../src/mcp/geometryService')
  const { flattenExportMeshes } = await import('../src/exchange/meshExportAdapter')
  const result = await new HeadlessGeometryService().compile('cube(2);translate([2,0,0])cube(2);', 'full')
  const flat = flattenExportMeshes(result.meshes)
  expect(flat.parts).toHaveLength(2)
  expect(new TextDecoder().decode(exportMeshFormat(flat, '3mf').data).match(/<object id=/g)).toHaveLength(2)
})

it('admits compressible 3MF documents larger than the final artifact limit', async () => {
  const { exportMeshFormatCompressed } = await import('../src/exchange/meshExportFormats')
  const { export3mfModelInKernel } = await import('../src/kernel/geometry/meshArtifactExport')
  const large = { ...mesh, positions: [...mesh.positions, ...new Array<number>(270000).fill(Math.PI)] }
  expect(export3mfModelInKernel(large).length).toBeGreaterThan(4 * 1024 * 1024)
  expect(() => exportMeshFormat(large, '3mf')).toThrow('4 MiB')
  const compressed = await exportMeshFormatCompressed(large, '3mf')
  expect(compressed.data.length).toBeLessThan(4 * 1024 * 1024)
})

it('validates the actual exported 3MF parts and their cumulative resource budget', async () => {
  const { export3mfModelInKernel } = await import('../src/kernel/geometry/meshArtifactExport')
  const invalid = { ...mesh, indices: [0, 1, 999999] }
  expect(() => export3mfModelInKernel({ ...mesh, parts: [mesh, invalid] })).toThrow('Malformed')
  const oversized = { ...mesh, positions: [...mesh.positions, ...new Array<number>(899976).fill(0)] }
  expect(() => export3mfModelInKernel({ ...mesh, parts: [oversized, oversized] })).toThrow('budget')
})

it.each([false, true])('writes consistent native ZIP headers and CRC with compression=%s', async compressed => {
  const { inflateRawSync, crc32 } = await import('node:zlib')
  const { export3mfInKernel } = await import('../src/kernel/geometry/meshArtifactExport')
  const data = export3mfInKernel(mesh, compressed)
  expect(export3mfInKernel(mesh, compressed)).toEqual(data)
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength)
  const end = data.length - 22
  expect(view.getUint32(end, true)).toBe(0x06054b50)
  expect(view.getUint16(end + 8, true)).toBe(3)
  expect(view.getUint16(end + 10, true)).toBe(3)
  const directory = view.getUint32(end + 16, true)
  expect(directory + view.getUint32(end + 12, true)).toBe(end)
  let cursor = directory
  for (const path of ['[Content_Types].xml', '_rels/.rels', '3D/3dmodel.model']) {
    expect(view.getUint32(cursor, true)).toBe(0x02014b50)
    const local = view.getUint32(cursor + 42, true)
    expect(view.getUint32(local, true)).toBe(0x04034b50)
    expect(view.getUint16(local + 6, true)).toBe(0x800)
    expect(view.getUint16(local + 8, true)).toBe(compressed ? 8 : 0)
    expect(view.getUint16(cursor + 10, true)).toBe(compressed ? 8 : 0)
    const nameLength = view.getUint16(local + 26, true)
    expect(new TextDecoder().decode(data.subarray(local + 30, local + 30 + nameLength))).toBe(path)
    expect(view.getUint16(cursor + 28, true)).toBe(nameLength)
    expect(new TextDecoder().decode(data.subarray(cursor + 46, cursor + 46 + nameLength))).toBe(path)
    const size = view.getUint32(local + 18, true)
    expect(view.getUint32(cursor + 20, true)).toBe(size)
    const payload = data.subarray(local + 30 + nameLength, local + 30 + nameLength + size)
    const decoded = compressed ? inflateRawSync(payload) : payload
    expect(decoded.length).toBe(view.getUint32(local + 22, true))
    expect(decoded.length).toBe(view.getUint32(cursor + 24, true))
    expect(crc32(decoded)).toBe(view.getUint32(local + 14, true))
    expect(crc32(decoded)).toBe(view.getUint32(cursor + 16, true))
    cursor += 46 + nameLength
  }
  expect(cursor).toBe(end)
})

it('preserves OpenSCAD module groups, object names, and colors across 3MF, OBJ, and AMF exports', async () => {
  const { inflateRawSync } = await import('node:zlib')
  const { HeadlessGeometryService } = await import('../src/mcp/geometryService')
  const { exportMesh } = await import('../src/exchange/meshConvert')
  const { traceExportSceneParts } = await import('../src/kernel/geometryBuildEngine')
  const { flattenExportMeshes } = await import('../src/exchange/meshExportAdapter')

  const source = `
module bracket() {
  color([0.1, 0.2, 0.3]) cube([10, 4, 2]);
  color([0.1, 0.2, 0.3]) translate([0, 0, 2]) cylinder(r = 1.5, h = 4);
}
module pin() {
  color([0.9, 0.5, 0.1]) cylinder(r = 1, h = 5);
}
bracket();
translate([25, 0, 0]) pin();
`
  const built = await new HeadlessGeometryService().compile(source, 'full')

  const art3mf = await exportMesh(built.meshes, '3mf', {
    quality: 'viewport',
    source,
    title: 'named-assembly',
  })
  const view = new DataView(art3mf.data.buffer, art3mf.data.byteOffset, art3mf.data.byteLength)
  const dirOff = view.getUint32(art3mf.data.length - 6, true)
  const count = view.getUint16(art3mf.data.length - 12, true)
  let cursor = dirOff
  let modelXml = ''
  for (let i = 0; i < count; i++) {
    const method = view.getUint16(cursor + 10, true)
    const compSize = view.getUint32(cursor + 20, true)
    const nameLen = view.getUint16(cursor + 28, true)
    const extraLen = view.getUint16(cursor + 30, true)
    const commentLen = view.getUint16(cursor + 32, true)
    const localOff = view.getUint32(cursor + 42, true)
    const entryName = new TextDecoder().decode(art3mf.data.subarray(cursor + 46, cursor + 46 + nameLen))
    if (entryName === '3D/3dmodel.model') {
      const lNameLen = view.getUint16(localOff + 26, true)
      const lExtraLen = view.getUint16(localOff + 28, true)
      const start = localOff + 30 + lNameLen + lExtraLen
      const payload = art3mf.data.subarray(start, start + compSize)
      modelXml = new TextDecoder().decode(method === 8 ? inflateRawSync(payload) : payload)
    }
    cursor += 46 + nameLen + extraLen + commentLen
  }
  expect(modelXml).toContain('<metadata name="Title">named-assembly</metadata>')
  expect(modelXml).toContain('<basematerials id="5">')
  expect(modelXml).toContain('displaycolor="#1A334D"')
  expect(modelXml).toContain('displaycolor="#E6801A"')
  expect(modelXml).toContain('name="bracket_cube"')
  expect(modelXml).toContain('name="bracket_cylinder"')
  expect(modelXml).toContain('name="pin"')
  expect(modelXml).toContain(
    '<object id="4" name="bracket" type="model"><components><component objectid="1"/><component objectid="2"/></components></object>',
  )
  expect(modelXml).toContain('<build><item objectid="4"/><item objectid="3"/></build>')

  const decoded3mf = await parseOpenScad3mf(file(art3mf.data, '3mf'))
  const expectedTris = built.meshes.reduce((sum, m) => sum + m.indices.length / 3, 0)
  expect(decoded3mf.triangleCount).toBe(expectedTris)

  const traces = await traceExportSceneParts(source)
  const flat = flattenExportMeshes(built.meshes, traces, 'named-assembly')
  const objText = new TextDecoder().decode(exportMeshFormat(flat, 'obj').data)
  expect(objText).toContain('g bracket')
  expect(objText).toContain('o bracket_cube')
  expect(objText).toContain('o bracket_cylinder')
  expect(objText).toContain('g pin')
  expect(objText).toContain('o pin')

  const amfText = new TextDecoder().decode(exportMeshFormat(flat, 'amf').data)
  expect(amfText).toContain('<metadata type="name">bracket_cube</metadata>')
  expect(amfText).toContain('<metadata type="name">bracket_cylinder</metadata>')
  expect(amfText).toContain('<metadata type="name">pin</metadata>')
  expect(amfText).toContain(
    '<constellation id="4"><metadata type="name">bracket</metadata><instance objectid="1"/><instance objectid="2"/></constellation>',
  )
  const decodedAmf = await parseOpenScadAmf(file(new TextEncoder().encode(amfText), 'amf'))
  expect(decodedAmf.triangleCount).toBe(expectedTris)
})
