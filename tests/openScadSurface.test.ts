import { describe, expect, it } from 'vitest'
import { OpenScadProject } from '../src/services/openScadProject'
import {
  OPENSCAD_SURFACE_MAX_TRIANGLES,
  OpenScadSurfaceDataError,
  decodeOpenScadSurfacePng,
  loadOpenScadSurfaceHeightMap,
  parseOpenScadSurfaceDat,
  prepareOpenScadSurfaceAssets,
  triangulateOpenScadSurface,
} from '../src/services/openScadSurface'

const RGBA_PNG_2X2 = 'iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAGUlEQVR4nGNgAIL///8LMjQ0NCg6ODgYAgAyLgWhbk/ABgAAAABJRU5ErkJggg=='
const GRAYSCALE16_PNG_2X2 = 'iVBORw0KGgoAAAANSUhEUgAAAAIAAAACEAAAAAAHTY67AAAAEklEQVR4nGNg+M/IwPD/fwMDABACA3/NNNLvAAAAAElFTkSuQmCC'

function bytes(base64: string): Uint8Array {
  return new Uint8Array(Buffer.from(base64, 'base64'))
}

function datFile(source: string) {
  const project = new OpenScadProject({
    entrypoint: 'heightmap.dat',
    files: [{ kind: 'source', path: 'heightmap.dat', source }],
  })
  return project.readEntrypoint()
}

describe('independent OpenSCAD surface assets', () => {
  it('parses whitespace DAT rows, comments, base depth, and center triangulation', () => {
    const map = parseOpenScadSurfaceDat(datFile([
      '# terrain',
      ' -1\t0  1 ',
      '',
      ' 2 3\t4',
    ].join('\n')))

    expect(map).toMatchObject({
      rows: 2,
      columns: 3,
      minimum: -1,
      base: -2,
      format: 'dat',
    })
    expect(Array.from(map.values)).toEqual([-1, 0, 1, 2, 3, 4])

    const mesh = triangulateOpenScadSurface(map, true)
    expect(mesh.triangleCount).toBe(26)
    expect(mesh.triangles).toHaveLength(26 * 3)
    expect(mesh.vertices).toHaveLength(15 * 3)
    const positions = Array.from({ length: mesh.vertices.length / 3 }, (_, index) => [
      mesh.vertices[index * 3],
      mesh.vertices[index * 3 + 1],
      mesh.vertices[index * 3 + 2],
    ])
    expect(Math.min(...positions.map(position => position[0]))).toBe(-1)
    expect(Math.max(...positions.map(position => position[0]))).toBe(1)
    expect(Math.min(...positions.map(position => position[1]))).toBe(-0.5)
    expect(Math.max(...positions.map(position => position[1]))).toBe(0.5)
    expect(Math.min(...positions.map(position => position[2]))).toBe(-2)
    expect(Math.max(...positions.map(position => position[2]))).toBe(4)
  })

  it('decodes RGBA PNG luminance, ignores alpha, flips Y, scales heights, and preserves 2021 invert', async () => {
    const png = bytes(RGBA_PNG_2X2)
    const decoded = await decodeOpenScadSurfacePng(png)
    expect(decoded).toMatchObject({ width: 2, height: 2 })
    ;[0, 255, 128, 64].forEach((expected, index) => {
      expect(decoded.pixels[index]).toBeCloseTo(expected, 12)
    })

    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '' },
        { kind: 'blob', path: 'assets/heightmap.png', data: png },
      ],
    })
    const prepared = await prepareOpenScadSurfaceAssets(project)
    const normal = loadOpenScadSurfaceHeightMap(
      project,
      prepared,
      'main.scad',
      'assets/heightmap.png',
      false,
    ).map
    const inverted = loadOpenScadSurfaceHeightMap(
      project,
      prepared,
      'main.scad',
      'assets/heightmap.png',
      true,
    ).map
    const scale = 100 / 255

    ;[128 * scale, 64 * scale, 0, 255 * scale].forEach((expected, index) => {
      expect(normal.values[index]).toBeCloseTo(expected, 12)
    })
    ;[
      (1 - 128) * scale,
      (1 - 64) * scale,
      scale,
      (1 - 255) * scale,
    ].forEach((expected, index) => {
      expect(inverted.values[index]).toBeCloseTo(expected, 12)
    })
    expect(normal.base).toBe(-1)
    expect(inverted.base).toBeCloseTo((1 - 255) * scale - 1, 12)
  })

  it('narrows 16-bit PNG channels exactly like the 2021 RGBA8 decoder', async () => {
    const decoded = await decodeOpenScadSurfacePng(bytes(GRAYSCALE16_PNG_2X2))

    // 0x00ff must become 0, not round up to 1; each channel contributes its
    // high byte exactly as lodepng's OpenSCAD 2021 RGBA8 target does.
    expect(Array.from(decoded.pixels)).toEqual([0, 1, 255, 128])
  })

  it('rejects corrupt, ragged, oversized, and over-triangulated inputs with typed codes', async () => {
    const corruptPng = bytes(RGBA_PNG_2X2)
    corruptPng[corruptPng.length - 1] ^= 1
    await expect(decodeOpenScadSurfacePng(corruptPng)).rejects.toMatchObject({
      name: 'OpenScadSurfaceDataError',
      code: 'E_SURFACE_PNG_INVALID',
    })

    expect(() => parseOpenScadSurfaceDat(datFile('0 1\n2'))).toThrow(
      expect.objectContaining({ code: 'E_SURFACE_DAT_INVALID' }),
    )
    const invalidProject = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '' },
        { kind: 'source', path: 'assets/ragged.dat', source: '0 1\n2' },
      ],
    })
    const invalidPrepared = await prepareOpenScadSurfaceAssets(invalidProject)
    expect(() => loadOpenScadSurfaceHeightMap(
      invalidProject,
      invalidPrepared,
      'main.scad',
      'assets/ragged.dat',
      false,
    )).toThrow(expect.objectContaining({
      code: 'E_SURFACE_DAT_INVALID',
      assetPath: 'assets/ragged.dat',
    }))
    expect(() => parseOpenScadSurfaceDat(datFile(
      `${Array.from({ length: 4_097 }, () => '0').join(' ')}\n${Array.from({ length: 4_097 }, () => '0').join(' ')}`,
    ))).toThrow(expect.objectContaining({
      code: 'E_SURFACE_DIMENSION_LIMIT',
      limit: 4_096,
      actual: 4_097,
    }))

    const overLimitMap = {
      rows: 500,
      columns: 500,
      values: new Float64Array(250_000),
      minimum: 0,
      base: -1,
      format: 'dat' as const,
    }
    expect(() => triangulateOpenScadSurface(overLimitMap, false)).toThrow(
      expect.objectContaining({
        name: OpenScadSurfaceDataError.name,
        code: 'E_SURFACE_TRIANGLE_LIMIT',
        limit: OPENSCAD_SURFACE_MAX_TRIANGLES,
      }),
    )
  })
})
