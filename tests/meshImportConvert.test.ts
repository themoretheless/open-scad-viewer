import { describe, expect, it } from 'vitest'
import { convertMeshFile, MESH_EXPORT_FORMATS } from '../src/services/meshConvert'
import { exportMeshFormat } from '../src/services/meshExportFormats'
import {
  detectMeshImportFormat,
  importMeshFile,
  MESH_IMPORT_ACCEPT,
  MESH_IMPORT_FORMATS,
  MeshImportError,
  stripMeshExtension,
} from '../src/services/meshImport'
import { buildOwnNurbs } from '../src/services/modelGraphNurbsKernel'
import { MODELGRAPH_NURBS_SURFACE_EXAMPLE } from '../src/services/modelGraphNurbs'

const built = buildOwnNurbs(MODELGRAPH_NURBS_SURFACE_EXAMPLE, { action: 'build' })
if (!('mesh' in built) || !built.mesh) throw new Error('Missing mesh')
const closedMesh = built.mesh
const encoder = new TextEncoder()

/** Unit cube as a closed indexed mesh (8 vertices, 12 triangles). */
const CUBE_POSITIONS = [0,0,0, 1,0,0, 1,1,0, 0,1,0, 0,0,1, 1,0,1, 1,1,1, 0,1,1]
const CUBE_QUADS = [[0,3,2,1],[4,5,6,7],[0,1,5,4],[1,2,6,5],[2,3,7,6],[3,0,4,7]]
const CUBE_TRIANGLES = CUBE_QUADS.flatMap(([a,b,c,d]) => [a,b,c, a,c,d])

function cubeObj(): string {
  const v = Array.from({ length: 8 }, (_, i) => `v ${CUBE_POSITIONS[i*3]} ${CUBE_POSITIONS[i*3+1]} ${CUBE_POSITIONS[i*3+2]}`)
  const f = CUBE_QUADS.map(q => `f ${q.map(i => `${i+1}/1/${i+1}`).join(' ')}`)
  return ['# cube', 'mtllib cube.mtl', 'o cube', ...v, 'vn 0 0 1', 'vt 0 0', 'usemtl red', 's off', ...f, ''].join('\n')
}

function cubePlyAscii(): string {
  return [
    'ply', 'format ascii 1.0', 'comment test cube',
    'element vertex 8', 'property float x', 'property float y', 'property float z', 'property uchar red',
    'element face 6', 'property list uchar int vertex_indices',
    'end_header',
    ...Array.from({ length: 8 }, (_, i) => `${CUBE_POSITIONS[i*3]} ${CUBE_POSITIONS[i*3+1]} ${CUBE_POSITIONS[i*3+2]} 255`),
    ...CUBE_QUADS.map(q => `4 ${q.join(' ')}`),
    '',
  ].join('\n')
}

function cubePlyBinary(littleEndian: boolean): Uint8Array {
  const header = encoder.encode([
    'ply', `format ${littleEndian ? 'binary_little_endian' : 'binary_big_endian'} 1.0`,
    'element vertex 8', 'property double x', 'property double y', 'property double z',
    'element face 12', 'property list uchar uint vertex_indices', 'property int flags',
    'end_header', '',
  ].join('\n'))
  const body = new ArrayBuffer(8 * 24 + 12 * (1 + 12 + 4))
  const view = new DataView(body)
  let o = 0
  for (let i = 0; i < 24; i++, o += 8) view.setFloat64(o, CUBE_POSITIONS[i], littleEndian)
  for (let t = 0; t < 12; t++) {
    view.setUint8(o, 3); o += 1
    for (let k = 0; k < 3; k++, o += 4) view.setUint32(o, CUBE_TRIANGLES[t*3+k], littleEndian)
    view.setInt32(o, -1, littleEndian); o += 4
  }
  const out = new Uint8Array(header.byteLength + body.byteLength)
  out.set(header); out.set(new Uint8Array(body), header.byteLength)
  return out
}

function expectCube(mesh: { positions: number[]; indices: number[]; triangleCount: number; vertexCount: number }) {
  expect(mesh.triangleCount).toBe(12)
  expect(mesh.vertexCount).toBe(8)
  const xs = mesh.positions.filter((_, i) => i % 3 === 0)
  expect(Math.min(...xs)).toBe(0)
  expect(Math.max(...xs)).toBe(1)
}

describe('meshImport', () => {
  it('detects formats by extension and by content', () => {
    expect(detectMeshImportFormat('Part.STL')).toBe('stl')
    expect(detectMeshImportFormat('a.amf.gz')).toBe('amf')
    expect(detectMeshImportFormat('model.3MF')).toBe('3mf')
    expect(detectMeshImportFormat('unknown', encoder.encode(cubeObj()))).toBe('obj')
    expect(detectMeshImportFormat('unknown', encoder.encode(cubePlyAscii()))).toBe('ply')
    expect(detectMeshImportFormat('unknown', encoder.encode('OFF\n0 0 0\n'))).toBe('off')
    expect(detectMeshImportFormat('unknown', encoder.encode('solid a\nendsolid a\n'))).toBe('stl')
    expect(detectMeshImportFormat('unknown', exportMeshFormat(closedMesh, '3mf').data)).toBe('3mf')
    expect(detectMeshImportFormat('unknown', exportMeshFormat(closedMesh, 'stl_binary').data)).toBe('stl')
    expect(detectMeshImportFormat('unknown', encoder.encode('hello world'))).toBeNull()
    expect(stripMeshExtension('bracket.amf.gz')).toBe('bracket')
    expect(stripMeshExtension('bracket.obj')).toBe('bracket')
    for (const format of MESH_IMPORT_FORMATS) expect(MESH_IMPORT_ACCEPT).toContain(`.${format}`)
  })

  it('imports OBJ with polygon faces, texture/normal slots and negative indices', async () => {
    const mesh = await importMeshFile('cube.obj', encoder.encode(cubeObj()))
    expect(mesh.format).toBe('obj')
    expectCube(mesh)
    const negative = ['v 0 0 0', 'v 1 0 0', 'v 0 1 0', 'f -3 -2 -1'].join('\n')
    const tri = await importMeshFile('t.obj', encoder.encode(negative))
    expect(tri.indices).toEqual([0, 1, 2])
  })

  it('imports ASCII and both binary PLY layouts, skipping extra properties', async () => {
    expectCube(await importMeshFile('cube.ply', encoder.encode(cubePlyAscii())))
    expectCube(await importMeshFile('cube.ply', cubePlyBinary(true)))
    expectCube(await importMeshFile('cube.ply', cubePlyBinary(false)))
  })

  it('welds STL triangle soup into a closed indexed mesh', async () => {
    const stl = exportMeshFormat(closedMesh, 'stl_binary').data
    const mesh = await importMeshFile('mesh.stl', stl)
    expect(mesh.triangleCount).toBe(closedMesh.indices.length / 3)
    expect(mesh.sourceVertexCount).toBe(mesh.triangleCount * 3)
    expect(mesh.vertexCount).toBe(closedMesh.positions.length / 3)
    const unwelded = await importMeshFile('mesh.stl', stl, { weld: false })
    expect(unwelded.vertexCount).toBe(unwelded.triangleCount * 3)
  })

  it.each(MESH_EXPORT_FORMATS)('round-trips the kernel %s writer through the importer', async format => {
    const out = exportMeshFormat(closedMesh, format)
    const mesh = await importMeshFile(`mesh.${out.extension}`, out.data)
    expect(mesh.triangleCount).toBe(closedMesh.indices.length / 3)
    expect(mesh.vertexCount).toBe(closedMesh.positions.length / 3)
  })

  it('reports typed errors for bad input', async () => {
    await expect(importMeshFile('x.gltf', new Uint8Array(16))).rejects.toMatchObject({ code: 'unsupported-format' })
    await expect(importMeshFile('x.obj', new Uint8Array(21_000_000))).rejects.toMatchObject({ code: 'too-large' })
    await expect(importMeshFile('x.obj', encoder.encode('v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n'))).rejects.toMatchObject({ code: 'invalid-data' })
    await expect(importMeshFile('x.obj', encoder.encode('v 0 0 0\n'))).rejects.toMatchObject({ code: 'empty' })
    await expect(importMeshFile('x.ply', encoder.encode('ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nend_header\n0\n'))).rejects.toBeInstanceOf(MeshImportError)
    await expect(importMeshFile('x.ply', encoder.encode('ply\nformat binary_little_endian 1.0\nelement vertex 8\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n'))).rejects.toMatchObject({ code: 'invalid-data' })
    await expect(importMeshFile('x.stl', encoder.encode('garbage'))).rejects.toMatchObject({ code: 'invalid-data', format: 'stl' })
    await expect(importMeshFile('x.obj', new Uint8Array([0xff, 0xfe, 0x00]))).rejects.toMatchObject({ code: 'encoding' })
  })
})

describe('meshConvert', () => {
  const inputs: Array<[string, () => Uint8Array]> = [
    ['cube.obj', () => encoder.encode(cubeObj())],
    ['cube.ply', () => cubePlyBinary(true)],
    ['mesh.stl', () => exportMeshFormat(closedMesh, 'stl_binary').data],
    ['mesh.off', () => exportMeshFormat(closedMesh, 'off').data],
    ['mesh.amf', () => exportMeshFormat(closedMesh, 'amf').data],
    ['mesh.3mf', () => exportMeshFormat(closedMesh, '3mf').data],
  ]

  it.each(inputs.flatMap(([name, data]) => MESH_EXPORT_FORMATS.map(target => [name, target, data] as const)))(
    'converts %s to %s',
    async (name, target, data) => {
      const result = await convertMeshFile(name, data(), target)
      expect(result.format).toBe(target)
      expect(result.fileName).toBe(`${name.replace(/\.[^.]+$/u, '')}.${result.extension}`)
      expect(result.data.byteLength).toBeGreaterThan(0)
      const back = await importMeshFile(result.fileName, result.data)
      expect(back.triangleCount).toBe(result.source.triangleCount)
      expect(back.vertexCount).toBe(result.source.vertexCount)
    },
  )

  it('refuses printing formats for open meshes but still writes PLY/OBJ', async () => {
    const open = ['v 0 0 0', 'v 1 0 0', 'v 0 1 0', 'f 1 2 3'].join('\n')
    await expect(convertMeshFile('tri.obj', encoder.encode(open), '3mf')).rejects.toThrow(/closed/u)
    await expect(convertMeshFile('tri.obj', encoder.encode(open), 'amf')).rejects.toThrow(/closed/u)
    const ply = await convertMeshFile('tri.obj', encoder.encode(open), 'ply')
    expect(new TextDecoder().decode(ply.data)).toContain('element face 1')
    const obj = await convertMeshFile('tri.ply', ply.data, 'obj')
    expect(new TextDecoder().decode(obj.data)).toContain('f 1 2 3')
  })

  it('rejects unknown targets and sanitises output names', async () => {
    await expect(convertMeshFile('cube.obj', encoder.encode(cubeObj()), 'gltf' as never)).rejects.toThrow(/Unsupported target/u)
    const result = await convertMeshFile('../we/ird.obj', encoder.encode(cubeObj()), 'ply')
    expect(result.fileName).toBe('.._we_ird.ply')
  })
})
