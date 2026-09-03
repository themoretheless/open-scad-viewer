import { readFileSync } from 'node:fs'
import { deflateRawSync, gzipSync } from 'node:zlib'
import { describe, expect, it } from 'vitest'
import differential from './fixtures/open-scad-import-2021.json'
import { OpenScadProject, type OpenScadProjectFileInput } from '../src/services/openScadProject'
import {
  OPENSCAD_IMPORT_CAPABILITIES,
  OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
  OpenScadImportDataError,
  loadOpenScadImport,
  loadPreparedOpenScadImport,
  parseOpenScad3mf,
  parseOpenScadAmf,
  parseOpenScadDxf,
  parseOpenScadDxfQueryMetadata,
  parseOpenScadOff,
  parseOpenScadStl,
  parseOpenScadSvg,
  prepareOpenScadImportAssets,
  queryPreparedOpenScadDxfCross,
  queryPreparedOpenScadDxfDimension,
  type OpenScadImportGeometry2D,
  type OpenScadImportGeometry3D,
} from '../src/services/openScadImport'

const UTF8 = new TextEncoder()
const LEGACY_DIMENSIONED_DXF = readFileSync(
  new URL('./fixtures/legacy-dimensioned.dxf', import.meta.url),
  'utf8',
)

function project(extraFiles: readonly OpenScadProjectFileInput[]): OpenScadProject {
  return new OpenScadProject({
    entrypoint: 'models/main.scad',
    files: [
      { kind: 'source', path: 'models/main.scad', source: '' },
      ...extraFiles,
    ],
  })
}

function sourceFile(path: string, source: string) {
  return project([{ kind: 'source', path, source }]).read(path)!
}

function blobFile(path: string, data: Uint8Array) {
  return project([{ kind: 'blob', path, data }]).read(path)!
}

function bounds2(geometry: OpenScadImportGeometry2D): { minimum: number[]; maximum: number[] } {
  const points = geometry.regions.flatMap(region => region.contours.flat())
  return {
    minimum: [Math.min(...points.map(point => point[0])), Math.min(...points.map(point => point[1]))],
    maximum: [Math.max(...points.map(point => point[0])), Math.max(...points.map(point => point[1]))],
  }
}

function bounds3(geometry: OpenScadImportGeometry3D): { minimum: number[]; maximum: number[] } {
  const points = Array.from({ length: geometry.vertices.length / 3 }, (_, index) => [
    geometry.vertices[index * 3],
    geometry.vertices[index * 3 + 1],
    geometry.vertices[index * 3 + 2],
  ])
  return {
    minimum: [0, 1, 2].map(axis => Math.min(...points.map(point => point[axis]))),
    maximum: [0, 1, 2].map(axis => Math.max(...points.map(point => point[axis]))),
  }
}

const CRC_TABLE = new Uint32Array(256)
for (let index = 0; index < CRC_TABLE.length; index++) {
  let value = index
  for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
  CRC_TABLE[index] = value >>> 0
}

function crc32(data: Uint8Array): number {
  let value = 0xffffffff
  for (const byte of data) value = CRC_TABLE[(value ^ byte) & 0xff] ^ (value >>> 8)
  return (value ^ 0xffffffff) >>> 0
}

function zip(
  entries: readonly { name: string; source: string; deflate?: boolean }[],
  options: { readonly zip64?: boolean; readonly comment?: Uint8Array } = {},
): Uint8Array {
  const encoded = entries.map(entry => {
    const name = UTF8.encode(entry.name)
    const data = UTF8.encode(entry.source)
    const compressed = entry.deflate ? new Uint8Array(deflateRawSync(data)) : data
    return { name, data, compressed, method: entry.deflate ? 8 : 0, crc: crc32(data), offset: 0 }
  })
  const localExtraSize = options.zip64 ? 20 : 0
  const centralExtraSize = options.zip64 ? 28 : 0
  const localSize = encoded.reduce((sum, entry) => sum + 30 + entry.name.length + localExtraSize + entry.compressed.length, 0)
  const centralSize = encoded.reduce((sum, entry) => sum + 46 + entry.name.length + centralExtraSize, 0)
  const zip64Trailer = options.zip64 ? 76 : 0
  const comment = options.comment ?? new Uint8Array()
  const output = new Uint8Array(localSize + centralSize + zip64Trailer + 22 + comment.length)
  const view = new DataView(output.buffer)
  const u64 = (offset: number, value: number): void => view.setBigUint64(offset, BigInt(value), true)
  let offset = 0
  for (const entry of encoded) {
    entry.offset = offset
    view.setUint32(offset, 0x04034b50, true); offset += 4
    view.setUint16(offset, options.zip64 ? 45 : 20, true); offset += 2
    view.setUint16(offset, 0x0800, true); offset += 2
    view.setUint16(offset, entry.method, true); offset += 2
    view.setUint32(offset, 0, true); offset += 4
    view.setUint32(offset, entry.crc, true); offset += 4
    view.setUint32(offset, options.zip64 ? 0xffffffff : entry.compressed.length, true); offset += 4
    view.setUint32(offset, options.zip64 ? 0xffffffff : entry.data.length, true); offset += 4
    view.setUint16(offset, entry.name.length, true); offset += 2
    view.setUint16(offset, localExtraSize, true); offset += 2
    output.set(entry.name, offset); offset += entry.name.length
    if (options.zip64) {
      view.setUint16(offset, 0x0001, true); offset += 2
      view.setUint16(offset, 16, true); offset += 2
      u64(offset, entry.data.length); offset += 8
      u64(offset, entry.compressed.length); offset += 8
    }
    output.set(entry.compressed, offset); offset += entry.compressed.length
  }
  const centralOffset = offset
  for (const entry of encoded) {
    view.setUint32(offset, 0x02014b50, true); offset += 4
    view.setUint16(offset, options.zip64 ? 45 : 20, true); offset += 2
    view.setUint16(offset, options.zip64 ? 45 : 20, true); offset += 2
    view.setUint16(offset, 0x0800, true); offset += 2
    view.setUint16(offset, entry.method, true); offset += 2
    view.setUint32(offset, 0, true); offset += 4
    view.setUint32(offset, entry.crc, true); offset += 4
    view.setUint32(offset, options.zip64 ? 0xffffffff : entry.compressed.length, true); offset += 4
    view.setUint32(offset, options.zip64 ? 0xffffffff : entry.data.length, true); offset += 4
    view.setUint16(offset, entry.name.length, true); offset += 2
    view.setUint16(offset, centralExtraSize, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint32(offset, 0, true); offset += 4
    view.setUint32(offset, options.zip64 ? 0xffffffff : entry.offset, true); offset += 4
    output.set(entry.name, offset); offset += entry.name.length
    if (options.zip64) {
      view.setUint16(offset, 0x0001, true); offset += 2
      view.setUint16(offset, 24, true); offset += 2
      u64(offset, entry.data.length); offset += 8
      u64(offset, entry.compressed.length); offset += 8
      u64(offset, entry.offset); offset += 8
    }
  }
  const centralLength = offset - centralOffset
  if (options.zip64) {
    const zip64Offset = offset
    view.setUint32(offset, 0x06064b50, true); offset += 4
    u64(offset, 44); offset += 8
    view.setUint16(offset, 45, true); offset += 2
    view.setUint16(offset, 45, true); offset += 2
    view.setUint32(offset, 0, true); offset += 4
    view.setUint32(offset, 0, true); offset += 4
    u64(offset, encoded.length); offset += 8
    u64(offset, encoded.length); offset += 8
    u64(offset, centralLength); offset += 8
    u64(offset, centralOffset); offset += 8
    view.setUint32(offset, 0x07064b50, true); offset += 4
    view.setUint32(offset, 0, true); offset += 4
    u64(offset, zip64Offset); offset += 8
    view.setUint32(offset, 1, true); offset += 4
  }
  view.setUint32(offset, 0x06054b50, true); offset += 4
  view.setUint32(offset, 0, true); offset += 4
  view.setUint16(offset, options.zip64 ? 0xffff : encoded.length, true); offset += 2
  view.setUint16(offset, options.zip64 ? 0xffff : encoded.length, true); offset += 2
  view.setUint32(offset, options.zip64 ? 0xffffffff : centralLength, true); offset += 4
  view.setUint32(offset, options.zip64 ? 0xffffffff : centralOffset, true); offset += 4
  view.setUint16(offset, comment.length, true); offset += 2
  output.set(comment, offset)
  return output
}

function binaryStl(): Uint8Array {
  const output = new Uint8Array(84 + 50)
  const view = new DataView(output.buffer)
  view.setUint32(80, 1, true)
  const coordinates = [0, 0, 0, 1, 0, 0, 0, 1, 0]
  coordinates.forEach((value, index) => view.setFloat32(96 + index * 4, value, true))
  return output
}

const TRIANGLE_3MF = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<model unit="inch" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">',
  '<resources>',
  '<object id="1" type="model"><mesh><vertices>',
  '<vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="0"/>',
  '</vertices><triangles><triangle v1="0" v2="1" v3="2"/></triangles></mesh></object>',
  '<object id="2" type="model"><components><component objectid="1" transform="1 0 0 0 1 0 0 0 1 2 3 4"/></components></object>',
  '</resources><build><item objectid="2"/></build></model>',
].join('')

const START_PART_RELS = [
  '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">',
  '<Relationship Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/model.model"/>',
  '</Relationships>',
].join('')

function threeMfEntries(model = TRIANGLE_3MF, deflate = false) {
  return [
    { name: '_rels/.rels', source: START_PART_RELS, deflate },
    { name: '3D/model.model', source: model, deflate },
  ]
}

describe('independent OpenSCAD 2021.01 import assets', () => {
  it('publishes six unconditional formats and an explicit CGAL-only NEF3 exclusion', () => {
    expect(OPENSCAD_IMPORT_CAPABILITIES).toEqual({
      profile: 'OpenSCAD-2021.01',
      formats: ['svg', 'dxf', 'stl', 'off', 'amf', '3mf'],
      conditionalExclusions: [{
        format: 'nef3',
        requires: 'CGAL',
        diagnostic: 'E_IMPORT_NEF3_UNAVAILABLE',
      }],
    })
  })

  it('matches captured OpenSCAD 2021.01 SVG viewport, unit, and Y-axis bounds', () => {
    expect(differential.profile).toBe('OpenSCAD-2021.01')
    for (const fixture of differential.cases) {
      const geometry = parseOpenScadSvg(sourceFile('asset.svg', fixture.source))
      const actual = bounds2(geometry)
      actual.minimum.forEach((value, index) => expect(value).toBeCloseTo(fixture.minimum[index], 8))
      actual.maximum.forEach((value, index) => expect(value).toBeCloseTo(fixture.maximum[index], 8))
    }
  })

  it('supports SVG primitives, compound paths, every path command, transforms, and strokes', () => {
    const source = [
      '<svg width="100mm" height="100mm" viewBox="0 0 100 100">',
      '<g transform="translate(2 3) rotate(5)">',
      '<path fill-rule="evenodd" d="M0 0 L30 0 H40 V20 C40 30 30 30 30 20 S20 10 20 20 Q20 30 10 30 T0 20 A10 10 0 0 1 0 0 Z M5 5 l5 0 0 5 -5 0z"/>',
      '<rect x="50" y="5" width="20" height="10" rx="2"/>',
      '<circle cx="80" cy="10" r="5"/><ellipse cx="80" cy="30" rx="8" ry="4"/>',
      '<polygon points="50,30 60,30 55,40"/>',
      '<polyline fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" points="5,50 20,60 30,50"/>',
      '<line stroke="black" stroke-width="2" stroke-linecap="square" x1="40" y1="50" x2="60" y2="50"/>',
      '</g></svg>',
    ].join('')
    const geometry = parseOpenScadSvg(sourceFile('asset.svg', source))
    expect(geometry.dimension).toBe(2)
    expect(geometry.regions.length).toBeGreaterThanOrEqual(10)
    expect(geometry.regions[0].contours).toHaveLength(2)
    expect(geometry.pointCount).toBeGreaterThan(200)
  })

  it('rejects external/active SVG features, DTDs, malformed path data and XML depth', () => {
    const invalid = [
      '<!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><svg><path d="M0 0L1 0L0 1z"/></svg>',
      '<svg><image href="https://example.test/a.png"/></svg>',
      '<svg><path d="M0 0 R 1 1"/></svg>',
      `<svg>${'<g>'.repeat(65)}<rect width="1" height="1"/>${'</g>'.repeat(65)}</svg>`,
    ]
    for (const source of invalid) {
      expect(() => parseOpenScadSvg(sourceFile('bad.svg', source))).toThrow(
        expect.objectContaining({ name: OpenScadImportDataError.name }),
      )
    }
  })

  it('imports ASCII DXF layers, closed/open chains, curves, and transformed BLOCK inserts', async () => {
    const dxf = [
      '0','SECTION','2','BLOCKS',
      '0','BLOCK','2','PART','10','1','20','1',
      '0','LWPOLYLINE','8','0','70','1','10','1','20','1','10','3','20','1','10','1','20','3',
      '0','ENDBLK','0','ENDSEC',
      '0','SECTION','2','ENTITIES',
      '0','INSERT','8','inserted','2','PART','10','10','20','20','41','2','42','3','50','90',
      '0','LWPOLYLINE','8','cut','70','1','10','1','20','2','42','1','10','5','20','2','10','5','20','6','10','1','20','6',
      '0','LINE','8','open','10','0','20','0','11','4','21','0',
      '0','LINE','8','open','10','4','20','0','11','2','21','3',
      '0','ARC','8','arc','10','20','20','20','40','3','50','0','51','180',
      '0','CIRCLE','8','circle','10','30','20','30','40','2',
      '0','ENDSEC','0','EOF',
    ].join('\n')
    const parsed = parseOpenScadDxf(sourceFile('asset.dxf', dxf))
    expect(new Set(parsed.regions.map(region => region.layer))).toEqual(
      new Set(['inserted', 'cut', 'circle', 'open', 'arc']),
    )
    expect(parsed.regions.find(region => region.layer === 'cut')?.contours[0]).toHaveLength(4)

    const bundle = project([{ kind: 'source', path: 'assets/asset.dxf', source: dxf }])
    const prepared = await prepareOpenScadImportAssets(bundle)
    const transformed = loadPreparedOpenScadImport(
      bundle,
      prepared,
      'models/main.scad',
      '../assets/asset.dxf',
      { layer: 'cut', origin: [1, 2], scale: 2, center: false },
    )
    expect(transformed.dimension).toBe(2)
    if (transformed.dimension !== 2) throw new Error('Expected 2D import')
    expect(bounds2(transformed)).toEqual({ minimum: [0, 0], maximum: [8, 8] })
  })

  it('preserves bounded DXF query metadata independently from filled import geometry', async () => {
    const bundle = project([{
      kind: 'source',
      path: 'fixtures/legacy-dimensioned.dxf',
      source: LEGACY_DIMENSIONED_DXF,
    }])
    const direct = parseOpenScadDxfQueryMetadata(bundle.read('fixtures/legacy-dimensioned.dxf')!)
    expect(direct.dimensions.map(dimension => dimension.name)).toEqual([
      'block_raw',
      'contract_dimension',
      'aligned',
      'angular',
      'diameter',
      'unsupported',
      'ordinate_x',
    ])
    expect(direct.lineSegments).toHaveLength(5)
    expect(direct.recordCount).toBe(12)
    expect(Object.isFrozen(direct.lineSegments[0].start)).toBe(true)

    const prepared = await prepareOpenScadImportAssets(bundle)
    expect(prepared.assets.get('fixtures/legacy-dimensioned.dxf')).toMatchObject({
      name: OpenScadImportDataError.name,
      code: 'E_IMPORT_EMPTY',
    })
    expect(prepared.dxfQueryMetadata.get('fixtures/legacy-dimensioned.dxf')).toMatchObject({
      recordCount: 12,
    })
  })

  it('evaluates 2021.01 dxf_dim coordinate groups, transforms, selection, and warnings', async () => {
    const bundle = project([{
      kind: 'source',
      path: 'fixtures/legacy-dimensioned.dxf',
      source: LEGACY_DIMENSIONED_DXF,
    }])
    const prepared = await prepareOpenScadImportAssets(bundle)
    const query = (name: string, options: { origin?: readonly [number, number]; scale?: number } = {}) => (
      queryPreparedOpenScadDxfDimension(
        bundle,
        prepared,
        'models/main.scad',
        '../fixtures/legacy-dimensioned.dxf',
        { layer: name === 'block_raw' ? 'block_dims' : 'dims', name, ...options },
      )
    )

    expect(query('contract_dimension')).toEqual({ value: 6, diagnostics: [] })
    expect(query('aligned', { origin: [1, 2], scale: 2 })).toEqual({ value: 10, diagnostics: [] })
    expect(query('angular').value).toBeCloseTo(90, 12)
    expect(query('diameter', { scale: 2 }).value).toBe(20)
    expect(query('ordinate_x', { origin: [1, 2], scale: 2 })).toEqual({ value: 16, diagnostics: [] })
    // The 2021 stream parser leaves DIMENSION coordinates inside BLOCKS raw.
    expect(query('block_raw', { origin: [100, 200], scale: 3 })).toEqual({ value: 7, diagnostics: [] })

    expect(query('unsupported')).toMatchObject({
      value: undefined,
      diagnostics: [{ severity: 'warning', code: 'OPENSCAD_DXF_DIMENSION_TYPE_UNSUPPORTED' }],
    })
    expect(query('missing')).toMatchObject({
      value: undefined,
      diagnostics: [{ severity: 'warning', code: 'OPENSCAD_DXF_DIMENSION_NOT_FOUND' }],
    })
  })

  it('builds dxf_cross paths after layer filtering and intersects the first two eligible paths', async () => {
    const bundle = project([{
      kind: 'source',
      path: 'fixtures/legacy-dimensioned.dxf',
      source: LEGACY_DIMENSIONED_DXF,
    }])
    const prepared = await prepareOpenScadImportAssets(bundle)
    const cross = queryPreparedOpenScadDxfCross(
      bundle,
      prepared,
      'models/main.scad',
      '../fixtures/legacy-dimensioned.dxf',
      { layer: 'contract_cross' },
    )
    // Infinite lines intersect outside the extent of the first source segment.
    expect(cross).toEqual({ value: [3, 1], diagnostics: [] })
    expect(queryPreparedOpenScadDxfCross(
      bundle,
      prepared,
      'models/main.scad',
      '../fixtures/legacy-dimensioned.dxf',
      { layer: 'contract_cross', origin: [1, -1], scale: 2 },
    )).toEqual({ value: [4, 4], diagnostics: [] })
    // A parallel first pair terminates the 2021.01 search; later paths are ignored.
    expect(queryPreparedOpenScadDxfCross(
      bundle,
      prepared,
      'models/main.scad',
      '../fixtures/legacy-dimensioned.dxf',
      { layer: 'parallel' },
    )).toMatchObject({
      value: undefined,
      diagnostics: [{ severity: 'warning', code: 'OPENSCAD_DXF_CROSS_NOT_FOUND' }],
    })

    const layerJoinDxf = [
      '0','SECTION','2','ENTITIES',
      '0','LINE','8','a','10','0','20','0','11','1','21','0',
      '0','LINE','8','b','10','1','20','0','11','2','21','0',
      '0','LINE','8','c','10','0','20','1','11','2','21','1',
      '0','LINE','8','d','10','3','20','0','11','3','21','2',
      '0','ENDSEC','0','EOF',
    ].join('\n')
    const joinedBundle = project([{ kind: 'source', path: 'assets/joined.dxf', source: layerJoinDxf }])
    const joinedPrepared = await prepareOpenScadImportAssets(joinedBundle)
    expect(queryPreparedOpenScadDxfCross(
      joinedBundle,
      joinedPrepared,
      'models/main.scad',
      '../assets/joined.dxf',
    )).toEqual({ value: [3, 1], diagnostics: [] })
  })

  it('prepares legacy import_stl/import_off/import_dxf decoders for neutral extensions', async () => {
    const asciiStl = [
      'solid t',
      'facet normal 0 0 1',
      'outer loop',
      'vertex 0 0 0',
      'vertex 1 0 0',
      'vertex 0 1 0',
      'endloop',
      'endfacet',
      'endsolid t',
    ].join('\n')
    const off = ['OFF', '3 1 0', '0 0 0', '1 0 0', '0 1 0', '3 0 1 2'].join('\n')
    const dxf = [
      '0','SECTION','2','ENTITIES',
      '0','LWPOLYLINE','8','0','70','1',
      '10','0','20','0','10','2','20','0','10','0','20','2',
      '0','ENDSEC','0','EOF',
    ].join('\n')
    const bundle = project([
      { kind: 'source', path: 'assets/mesh.stldata', source: asciiStl },
      { kind: 'source', path: 'assets/mesh.offdata', source: off },
      { kind: 'source', path: 'assets/shape.dxfdata', source: dxf },
    ])
    const prepared = await prepareOpenScadImportAssets(bundle, {
      forcedAssets: [
        { path: 'assets/mesh.stldata', format: 'stl' },
        { path: 'assets/mesh.offdata', format: 'off' },
        { path: 'assets/shape.dxfdata', format: 'dxf' },
      ],
    })
    expect(prepared.assets.size).toBe(0)
    expect(prepared.forcedAssets.size).toBe(3)

    const loadedStl = loadPreparedOpenScadImport(
      bundle, prepared, 'models/main.scad', '../assets/mesh.stldata', { forcedFormat: 'stl' },
    )
    const loadedOff = loadPreparedOpenScadImport(
      bundle, prepared, 'models/main.scad', '../assets/mesh.offdata', { forcedFormat: 'off' },
    )
    const loadedDxf = loadPreparedOpenScadImport(
      bundle, prepared, 'models/main.scad', '../assets/shape.dxfdata', { forcedFormat: 'dxf' },
    )
    expect(loadedStl).toMatchObject({ dimension: 3, format: 'stl', triangleCount: 1 })
    expect(loadedOff).toMatchObject({ dimension: 3, format: 'off', triangleCount: 1 })
    expect(loadedDxf).toMatchObject({ dimension: 2, format: 'dxf' })
    await expect(loadOpenScadImport(
      bundle,
      'models/main.scad',
      '../assets/mesh.offdata',
      { forcedFormat: 'off' },
    )).resolves.toMatchObject({ dimension: 3, format: 'off', triangleCount: 1 })
  })

  it('decodes binary and grammar-checked ASCII STL and rejects Float32-collapsed faces', () => {
    const binary = parseOpenScadStl(blobFile('mesh.stl', binaryStl()))
    const ascii = parseOpenScadStl(sourceFile('mesh.stl', [
      'solid t',
      'facet normal 0 0 1',
      'outer loop',
      'vertex 0 0 0',
      'vertex 1 0 0',
      'vertex 0 1 0',
      'endloop',
      'endfacet',
      'endsolid t',
    ].join('\n')))
    expect(binary.triangleCount).toBe(1)
    expect(Array.from(ascii.vertices)).toEqual(Array.from(binary.vertices))
    expect(() => parseOpenScadStl(sourceFile('bad.stl', [
      'solid t',
      'facet normal 0 0 1',
      'outer loop',
      'vertex 16777216 0 0',
      'vertex 16777217 0 0',
      'vertex 16777216 1 0',
      'endloop',
      'endfacet',
      'endsolid t',
    ].join('\n')))).toThrow(expect.objectContaining({ code: 'E_IMPORT_EMPTY' }))
    expect(() => parseOpenScadStl(sourceFile('bad.stl', 'solid t\nvertex 0 0 0\nendsolid t'))).toThrow(
      expect.objectContaining({ code: 'E_IMPORT_INVALID_DATA' }),
    )
  })

  it('imports OFF/COFF polygon faces with optional per-line colors and concave triangulation', () => {
    const off = parseOpenScadOff(sourceFile('mesh.off', [
      'COFF 5 1 0',
      '0 0 0 255 0 0 255',
      '2 0 0 255 0 0 255',
      '2 2 0 255 0 0 255',
      '1 1 0 255 0 0 255',
      '0 2 0 255 0 0 255',
      '5 0 1 2 3 4 0 255 0 255',
    ].join('\n')))
    expect(off.triangleCount).toBe(3)
    expect(bounds3(off)).toEqual({ minimum: [0, 0, 0], maximum: [2, 2, 0] })
  })

  it('imports AMF units, all volumes and constellation transforms', async () => {
    const amfSource = [
      '<amf unit="inch">',
      '<object id="7"><mesh><vertices>',
      '<vertex><coordinates><x>0</x><y>0</y><z>0</z></coordinates></vertex>',
      '<vertex><coordinates><x>1</x><y>0</y><z>0</z></coordinates></vertex>',
      '<vertex><coordinates><x>0</x><y>1</y><z>0</z></coordinates></vertex>',
      '<vertex><coordinates><x>0</x><y>0</y><z>1</z></coordinates></vertex>',
      '</vertices>',
      '<volume><triangle><v1>0</v1><v2>1</v2><v3>2</v3></triangle></volume>',
      '<volume><triangle><v1>0</v1><v2>3</v2><v3>1</v3></triangle></volume>',
      '</mesh></object>',
      '<constellation id="9"><instance objectid="7"><deltax>2</deltax></instance></constellation>',
      '</amf>',
    ].join('')
    const amf = await parseOpenScadAmf(sourceFile('mesh.amf', amfSource))
    expect(amf.triangleCount).toBe(2)
    expect(bounds3(amf)).toEqual({ minimum: [50.79999923706055, 0, 0], maximum: [76.19999694824219, 25.399999618530273, 25.399999618530273] })

    const zipped = await parseOpenScadAmf(blobFile('mesh.amf', zip([{ name: 'scene.amf', source: amfSource, deflate: true }])))
    const gzipped = await parseOpenScadAmf(blobFile('mesh.amf', new Uint8Array(gzipSync(UTF8.encode(amfSource)))))
    expect(Array.from(zipped.vertices)).toEqual(Array.from(amf.vertices))
    expect(Array.from(gzipped.triangles)).toEqual(Array.from(amf.triangles))
  })

  it('expands nested AMF constellations once, retains standalone roots, and rejects cycles', async () => {
    const mesh = (id: string, x: number) => [
      `<object id="${id}"><mesh><vertices>`,
      `<vertex><coordinates><x>${x}</x><y>0</y><z>0</z></coordinates></vertex>`,
      `<vertex><coordinates><x>${x + 1}</x><y>0</y><z>0</z></coordinates></vertex>`,
      `<vertex><coordinates><x>${x}</x><y>1</y><z>0</z></coordinates></vertex>`,
      '</vertices><volume><triangle><v1>0</v1><v2>1</v2><v3>2</v3></triangle></volume></mesh></object>',
    ].join('')
    const nested = [
      '<amf unit="millimeter">',
      mesh('1', 0),
      mesh('2', 20),
      '<constellation id="3"><instance objectid="1"><deltax>5</deltax></instance></constellation>',
      '<constellation id="4"><instance objectid="3"><deltay>7</deltay></instance></constellation>',
      '</amf>',
    ].join('')
    const geometry = await parseOpenScadAmf(sourceFile('nested.amf', nested))
    expect(geometry.triangleCount).toBe(2)
    expect(bounds3(geometry)).toEqual({ minimum: [5, 0, 0], maximum: [21, 8, 0] })

    await expect(parseOpenScadAmf(sourceFile('cycle.amf', [
      '<amf>',
      '<constellation id="1"><instance objectid="2"/></constellation>',
      '<constellation id="2"><instance objectid="1"/></constellation>',
      '</amf>',
    ].join('')))).rejects.toMatchObject({ code: 'E_IMPORT_INVALID_DATA' })
  })

  it('imports stored and deflated 3MF core packages with units and component transforms', async () => {
    for (const deflate of [false, true]) {
      const data = zip(threeMfEntries(TRIANGLE_3MF, deflate))
      const geometry = await parseOpenScad3mf(blobFile('mesh.3mf', data))
      expect(geometry.triangleCount).toBe(1)
      const bounds = bounds3(geometry)
      expect(bounds.minimum[0]).toBeCloseTo(50.8, 4)
      expect(bounds.minimum[1]).toBeCloseTo(76.2, 4)
      expect(bounds.minimum[2]).toBeCloseTo(101.6, 4)
      expect(bounds.maximum[0]).toBeCloseTo(76.2, 4)
      expect(bounds.maximum[1]).toBeCloseTo(101.6, 4)
    }
  })

  it('accepts bounded ZIP64 3MF and skips EOCD-shaped bytes inside the real comment', async () => {
    for (const deflate of [false, true]) {
      const geometry = await parseOpenScad3mf(blobFile(
        'zip64.3mf',
        zip(threeMfEntries(TRIANGLE_3MF, deflate), { zip64: true }),
      ))
      expect(geometry.triangleCount).toBe(1)
    }

    const fakeEnd = new Uint8Array(22)
    new DataView(fakeEnd.buffer).setUint32(0, 0x06054b50, true)
    const geometry = await parseOpenScad3mf(blobFile(
      'comment.3mf',
      zip(threeMfEntries(), { comment: fakeEnd }),
    ))
    expect(geometry.triangleCount).toBe(1)

    const oversized = zip(threeMfEntries(), { zip64: true })
    const oversizedView = new DataView(oversized.buffer)
    let central = 0
    while (central + 4 <= oversized.byteLength && oversizedView.getUint32(central, true) !== 0x02014b50) central++
    const nameLength = oversizedView.getUint16(central + 28, true)
    oversizedView.setBigUint64(central + 46 + nameLength + 4, 1n << 63n, true)
    await expect(parseOpenScad3mf(blobFile('oversized.3mf', oversized))).rejects.toMatchObject({
      code: 'E_IMPORT_LIMIT',
    })
  })

  it('uses exact namespace-aware StartPart routing and ignores vendor decoys', async () => {
    const decoyRels = [
      '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">',
      '<Relationship Id="vendor" Type="urn:vendor/3dmodel" Target="/3D/decoy.model"/>',
      '<Relationship Id="core" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/model.model"/>',
      '</Relationships>',
    ].join('')
    const geometry = await parseOpenScad3mf(blobFile('routed.3mf', zip([
      { name: '_rels/.rels', source: decoyRels },
      { name: '3D/decoy.model', source: '<vendor/>' },
      { name: '3D/model.model', source: TRIANGLE_3MF },
    ])))
    expect(geometry.triangleCount).toBe(1)

    const duplicateStart = START_PART_RELS.replace('</Relationships>', [
      '<Relationship Id="rel1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/model.model"/>',
      '</Relationships>',
    ].join(''))
    await expect(parseOpenScad3mf(blobFile('duplicate.3mf', zip([
      { name: '_rels/.rels', source: duplicateStart },
      { name: '3D/model.model', source: TRIANGLE_3MF },
    ])))).rejects.toMatchObject({ code: 'E_IMPORT_INVALID_DATA' })
  })

  it('resolves 3MF core namespaces and ST_ResourceID values, not lexical spellings', async () => {
    const model = [
      '<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:x="urn:vendor" x:unit="inch">',
      '<resources>',
      '<x:object id="999"><x:mesh/></x:object>',
      '<object id="01"><mesh><vertices>',
      '<vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="0"/>',
      '</vertices><triangles><triangle v1="00" v2="01" v3="02"/></triangles></mesh></object>',
      '<object id="2"><components><component objectid="+1"/></components></object>',
      '</resources><build><item objectid="02"/></build></model>',
    ].join('')
    const geometry = await parseOpenScad3mf(blobFile('ids.3mf', zip(threeMfEntries(model))))
    expect(bounds3(geometry)).toEqual({ minimum: [0, 0, 0], maximum: [1, 1, 0] })

    const duplicate = model.replace('<object id="2">', '<object id="1">')
    await expect(parseOpenScad3mf(blobFile('duplicate-id.3mf', zip(threeMfEntries(duplicate))))).rejects.toMatchObject({
      code: 'E_IMPORT_INVALID_DATA',
    })
    for (const id of ['0', '-1', '1.5', '2147483648']) {
      const invalid = model.replace('id="01"', `id="${id}"`)
      await expect(parseOpenScad3mf(blobFile('invalid-id.3mf', zip(threeMfEntries(invalid))))).rejects.toMatchObject({
        code: 'E_IMPORT_INVALID_DATA',
      })
    }
    const wrongNamespace = model.replace(
      'http://schemas.microsoft.com/3dmanufacturing/core/2015/02',
      'urn:vendor:fake-core',
    )
    await expect(parseOpenScad3mf(blobFile('wrong-ns.3mf', zip(threeMfEntries(wrongNamespace))))).rejects.toMatchObject({
      code: 'E_IMPORT_INVALID_DATA',
    })
  })

  it('resolves imports relative to authored source and returns contextual path/format/NEF3 errors', async () => {
    const bundle = project([{ kind: 'source', path: 'assets/shape.svg', source: '<svg><rect width="10" height="10"/></svg>' }])
    const imported = await loadOpenScadImport(bundle, 'models/main.scad', '../assets/shape.svg')
    expect(imported).toMatchObject({ dimension: 2, format: 'svg' })

    await expect(loadOpenScadImport(bundle, 'models/main.scad', '../../escape.svg')).rejects.toMatchObject({
      code: 'E_IMPORT_PATH_INVALID',
      sourcePath: 'models/main.scad',
      specifier: '../../escape.svg',
    })
    await expect(loadOpenScadImport(bundle, 'models/main.scad', '../assets/missing.stl')).rejects.toMatchObject({
      code: 'E_IMPORT_FILE_MISSING',
      sourcePath: 'models/main.scad',
      assetPath: 'assets/missing.stl',
    })
    await expect(loadOpenScadImport(bundle, 'models/main.scad', '../assets/model.nef3')).rejects.toMatchObject({
      code: 'E_IMPORT_NEF3_UNAVAILABLE',
      assetPath: 'assets/model.nef3',
    })
    await expect(loadOpenScadImport(bundle, 'models/main.scad', '../assets/file.obj')).rejects.toMatchObject({
      code: 'E_IMPORT_FORMAT_UNSUPPORTED',
      assetPath: 'assets/file.obj',
    })
  })

  it('bounds aggregate expanded 3MF data across eager project preparation', async () => {
    const padding = 'x'.repeat(Math.ceil(OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES / 2))
    const paddedModel = TRIANGLE_3MF.replace('</model>', `<!--${padding}--></model>`)
    const bomb = zip(threeMfEntries(paddedModel, true))
    expect(bomb.byteLength).toBeLessThan(20_000)
    const bundle = project([
      { kind: 'blob', path: 'assets/a.3mf', data: bomb },
      { kind: 'blob', path: 'assets/b.3mf', data: bomb },
    ])
    const prepared = await prepareOpenScadImportAssets(bundle)
    expect(prepared.assets.get('assets/a.3mf')).toMatchObject({ dimension: 3, format: '3mf' })
    expect(prepared.assets.get('assets/b.3mf')).toMatchObject({
      name: OpenScadImportDataError.name,
      code: 'E_IMPORT_LIMIT',
      limit: OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
    })
  }, 30_000)
})
