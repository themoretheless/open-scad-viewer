import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import {
  OPENSCAD_2021_01_BUILTIN_MODULES,
  OPENSCAD_2021_01_SMOKE_FIXTURES,
} from '../src/core/openScad2021Contract'
import { OpenScadProject } from '../src/services/openScadProject'
import { parseOpenSCAD, parseOpenScadProject } from '../src/services/openscadParser'
import { OpenScadImportPositionedError } from '../src/services/openScadImport'
import { OpenScadSurfaceError } from '../src/services/openScadSurface'
import { OpenScadTextPositionedError } from '../src/services/openScadText'

const RGBA_PNG_2X2 = 'iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAGUlEQVR4nGNgAIL///8LMjQ0NCg6ODgYAgAyLgWhbk/ABgAAAABJRU5ErkJggg=='
const BASIC_TTF = new Uint8Array(readFileSync(
  new URL('./fixtures/Basic-Regular.ttf', import.meta.url),
))

function bounds(vertices: Float32Array): { min: number[]; max: number[] } {
  const min = [Infinity, Infinity, Infinity]
  const max = [-Infinity, -Infinity, -Infinity]
  for (let offset = 0; offset < vertices.length; offset += 6) {
    for (let axis = 0; axis < 3; axis++) {
      min[axis] = Math.min(min[axis], vertices[offset + axis])
      max[axis] = Math.max(max[axis], vertices[offset + axis])
    }
  }
  return { min, max }
}

describe('independent OpenSCAD project evaluation', () => {
  it('evaluates include state and use definitions through the shared independent engine', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: 'include <config.scad> use <lib/part.scad> make_part(size);',
        },
        { kind: 'source', path: 'config.scad', source: 'size = 3;' },
        {
          kind: 'source',
          path: 'lib/part.scad',
          source: 'sphere(100); module make_part(edge) { cube(edge); }',
        },
      ],
    })

    const result = await parseOpenScadProject(project, { quality: 'full' })

    expect(result).toMatchObject({
      quality: 'full',
      reduced: false,
      meshes: [expect.any(Object)],
      warnings: [],
    })
    expect(result.volume).toBeCloseTo(27, 5)
    expect(result.surfaceArea).toBeCloseTo(54, 5)
  })

  it('requires a project VFS when surface() is called through the direct source parser', async () => {
    await expect(parseOpenSCAD('surface(file = "heightmap.dat");', {
      languageProfile: 'openscad/stable-2021.01',
      quality: 'full',
    })).rejects.toMatchObject({
      name: 'OpenScadSurfaceError',
      code: 'E_SURFACE_PROJECT_REQUIRED',
      details: { sourcePath: '<inline>' },
      line: 1,
      column: 1,
    })
  })

  it('requires a project VFS when import() is called through the direct source parser', async () => {
    await expect(parseOpenSCAD('import(file = "shape.svg");', {
      languageProfile: 'openscad/stable-2021.01',
      quality: 'full',
    })).rejects.toMatchObject({
      name: 'OpenScadImportPositionedError',
      importCode: 'E_IMPORT_PROJECT_REQUIRED',
      details: { sourcePath: '<inline>', specifier: '' },
      line: 1,
      column: 1,
    })
  })

  it('requires a project VFS when text() is called through the direct source parser', async () => {
    await expect(parseOpenSCAD('text("A", font = "Basic");', {
      languageProfile: 'openscad/stable-2021.01',
      quality: 'full',
    })).rejects.toMatchObject({
      name: 'OpenScadTextPositionedError',
      textCode: 'E_TEXT_PROJECT_REQUIRED',
      details: { sourcePath: '<inline>' },
      line: 1,
      column: 1,
    })
  })

  it('executes the exact canonical text smoke through the direct project evaluator', async () => {
    const smoke = OPENSCAD_2021_01_BUILTIN_MODULES.find(entry => entry.name === 'text')!.smoke
    const fixture = OPENSCAD_2021_01_SMOKE_FIXTURES.find(entry => entry.id === 'fontconfig-default-font')!
    const result = await parseOpenScadProject(new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: smoke.source },
        { kind: 'blob', path: fixture.path, data: BASIC_TTF },
      ],
    }), { quality: 'full' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes[0].topology).toMatchObject({
      boundary: 0,
      nonManifold: 0,
      degenerate: 0,
    })
  })

  it('resolves text font paths relative to their defining module source', async () => {
    const result = await parseOpenScadProject(new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/label.scad> label();' },
        {
          kind: 'source',
          path: 'lib/label.scad',
          source: 'module label() { linear_extrude(height = 2) text("AV", size = 5, font = "../fonts/Basic-Regular.ttf", halign = "center", valign = "center", spacing = 1.2, direction = "ltr", language = "en", script = "Latn", $fn = 24); }',
        },
        { kind: 'blob', path: 'fonts/Basic-Regular.ttf', data: BASIC_TTF },
      ],
    }), { quality: 'full' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes[0].topology).toMatchObject({ boundary: 0, nonManifold: 0 })
  })

  it('reports missing text fonts at the call in their defining source', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/label.scad> label();' },
        {
          kind: 'source',
          path: 'lib/label.scad',
          source: 'module label() {\n  text("A", font = "../fonts/missing.ttf");\n}',
        },
      ],
    })

    try {
      await parseOpenScadProject(project, { quality: 'full' })
      throw new Error('Expected missing text font diagnostic')
    } catch (error) {
      expect(error).toBeInstanceOf(OpenScadTextPositionedError)
      expect(error).toMatchObject({
        textCode: 'E_TEXT_FONT_NOT_FOUND',
        line: 2,
        column: 3,
        details: {
          sourcePath: 'lib/label.scad',
          font: '../fonts/missing.ttf',
          fontPath: 'fonts/missing.ttf',
        },
      })
    }
  })

  it('executes the exact canonical import smoke through the direct project evaluator', async () => {
    const smoke = OPENSCAD_2021_01_BUILTIN_MODULES.find(entry => entry.name === 'import')!.smoke
    const fixture = OPENSCAD_2021_01_SMOKE_FIXTURES.find(entry => entry.id === 'import-square-svg')!
    if (fixture.provisioning !== 'inline-text') throw new Error('Expected inline SVG fixture')
    const result = await parseOpenScadProject(new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: smoke.source },
        { kind: 'source', path: fixture.path, source: fixture.text },
      ],
    }), { quality: 'full' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes[0].topology).toMatchObject({ boundary: 0, nonManifold: 0 })
  })

  it('resolves import assets relative to the defining module source', async () => {
    const fixture = OPENSCAD_2021_01_SMOKE_FIXTURES.find(entry => entry.id === 'import-square-svg')!
    if (fixture.provisioning !== 'inline-text') throw new Error('Expected inline SVG fixture')
    const result = await parseOpenScadProject(new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/part.scad> imported_part();' },
        {
          kind: 'source',
          path: 'lib/part.scad',
          source: 'module imported_part() { linear_extrude(height = 2) import(file = "../assets/part.svg"); }',
        },
        { kind: 'source', path: 'assets/part.svg', source: fixture.text },
      ],
    }), { quality: 'full' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes[0].topology).toMatchObject({ boundary: 0, nonManifold: 0 })
  })

  it('reports missing import assets at the call in their defining source', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/part.scad> imported_part();' },
        {
          kind: 'source',
          path: 'lib/part.scad',
          source: 'module imported_part() {\n  import(file = "../assets/missing.svg");\n}',
        },
      ],
    })

    try {
      await parseOpenScadProject(project, { quality: 'full' })
      throw new Error('Expected missing import asset diagnostic')
    } catch (error) {
      expect(error).toBeInstanceOf(OpenScadImportPositionedError)
      expect(error).toMatchObject({
        importCode: 'E_IMPORT_FILE_MISSING',
        line: 2,
        column: 3,
        details: {
          sourcePath: 'lib/part.scad',
          specifier: '../assets/missing.svg',
          assetPath: 'assets/missing.svg',
          format: 'svg',
        },
      })
    }
  })

  it('executes the exact canonical surface smoke through the direct project evaluator', async () => {
    const smoke = OPENSCAD_2021_01_BUILTIN_MODULES.find(entry => entry.name === 'surface')!.smoke
    const fixture = OPENSCAD_2021_01_SMOKE_FIXTURES.find(entry => entry.id === 'heightmap-dat')!
    if (fixture.provisioning !== 'inline-text') throw new Error('Expected inline DAT fixture')
    const result = await parseOpenScadProject(new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: smoke.source },
        { kind: 'source', path: fixture.path, source: fixture.text },
      ],
    }), { quality: 'full' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes[0].topology).toMatchObject({ boundary: 0, nonManifold: 0 })
  })

  it('resolves DAT paths relative to the defining source and produces a closed manifold', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/terrain.scad> terrain();' },
        {
          kind: 'source',
          path: 'lib/terrain.scad',
          source: 'module terrain() { surface(file = "../assets/terrain.dat", center = true, invert = true, convexity = 7); }',
        },
        { kind: 'source', path: 'assets/terrain.dat', source: '0\t1\n2  3\n' },
      ],
    })

    const result = await parseOpenScadProject(project, { quality: 'full' })

    expect(result.meshes).toHaveLength(1)
    expect(result.volume).toBeCloseTo(2.5, 5)
    expect(bounds(result.meshes[0].vertices)).toEqual({
      min: [-0.5, -0.5, -1],
      max: [0.5, 0.5, 3],
    })
    expect(result.meshes[0].topology).toMatchObject({
      boundary: 0,
      nonManifold: 0,
      degenerate: 0,
    })
  })

  it('evaluates PNG scale and historical invert semantics through project blobs', async () => {
    const data = new Uint8Array(Buffer.from(RGBA_PNG_2X2, 'base64'))
    const makeProject = (invert: boolean) => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        {
          kind: 'source',
          path: 'main.scad',
          source: `surface(file = "assets/heightmap.png", center = true, invert = ${invert});`,
        },
        { kind: 'blob', path: 'assets/heightmap.png', data },
      ],
    })

    const normal = await parseOpenScadProject(makeProject(false), { quality: 'full' })
    const inverted = await parseOpenScadProject(makeProject(true), { quality: 'full' })
    const scale = 100 / 255

    expect(bounds(normal.meshes[0].vertices)).toEqual({
      min: [-0.5, -0.5, -1],
      max: [0.5, 0.5, 100],
    })
    const invertedBounds = bounds(inverted.meshes[0].vertices)
    expect(invertedBounds.min[0]).toBe(-0.5)
    expect(invertedBounds.min[1]).toBe(-0.5)
    expect(invertedBounds.min[2]).toBeCloseTo((1 - 255) * scale - 1, 5)
    expect(invertedBounds.max[2]).toBeCloseTo(scale, 5)
    expect(normal.volume).toBeGreaterThan(0)
    expect(inverted.volume).toBeGreaterThan(0)
  })

  it('reports missing surface assets at the call in their defining source', async () => {
    const project = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: 'use <lib/terrain.scad> terrain();' },
        {
          kind: 'source',
          path: 'lib/terrain.scad',
          source: 'module terrain() {\n  surface(file = "../assets/missing.dat");\n}',
        },
      ],
    })

    try {
      await parseOpenScadProject(project, { quality: 'full' })
      throw new Error('Expected missing surface asset diagnostic')
    } catch (error) {
      expect(error).toBeInstanceOf(OpenScadSurfaceError)
      expect(error).toMatchObject({
        code: 'E_SURFACE_FILE_MISSING',
        line: 2,
        column: 3,
        details: {
          sourcePath: 'lib/terrain.scad',
          specifier: '../assets/missing.dat',
          assetPath: 'assets/missing.dat',
        },
      })
    }
  })
})
