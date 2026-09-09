import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { setModelGraphParameters } from '../src/services/modelGraph'
import type { MeshData } from '../src/core/mesh'

const original = readFileSync('examples/skadis-box/skadis-dovetail.scad', 'utf8')
const compact = readFileSync('examples/skadis-box/skadis-dovetail.modelgraph.scad', 'utf8')

function bounds(mesh: MeshData) {
  const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity]
  for (let i = 0; i < mesh.vertices.length; i += 6) {
    for (let axis = 0; axis < 3; axis++) {
      const row = axis * 4
      const value = mesh.transform[row]! * mesh.vertices[i]! + mesh.transform[row + 1]! * mesh.vertices[i + 1]!
        + mesh.transform[row + 2]! * mesh.vertices[i + 2]! + mesh.transform[row + 3]!
      min[axis] = Math.min(min[axis]!, value)
      max[axis] = Math.max(max[axis]!, value)
    }
  }
  return [...min, ...max]
}

function expectBounds(actual: MeshData[], expected: MeshData[]) {
  expect(actual).toHaveLength(expected.length)
  actual.forEach((mesh, i) => bounds(mesh).forEach((value, axis) => expect(value).toBeCloseTo(bounds(expected[i]!)[axis]!, 5)))
}

it.each([0, 1, 2])('preserves SKADIS geometry for output %i', async part => {
  const source = compact.replace('param part = 0', `param part = ${part}`)
  const compiled = compileModelGraphText(source)
  expect(compiled.document).toMatchObject({ language: 'modelgraph/1', segments: 40 })
  expect(compiled.source).not.toMatch(/module\s+hook/)
  const reference = await parseOpenSCAD(original.replace('part = 0;', `part = ${part};`), { quality: 'full' })
  const result = await parseOpenSCAD(source, { quality: 'full' })
  expect(result.meshes.length).toBe(part === 0 ? 3 : 1)
  expect(result.volume).toBeCloseTo(reference.volume, 7)
  expectBounds(result.meshes, reference.meshes)
  expect(result.meshes.map(mesh => mesh.topology)).toEqual(reference.meshes.map(mesh => mesh.topology))
})

it('preserves configurable fit, mount placement and disabled ridge', async () => {
  const changes = { width: 160, height: 100, fit: 0.35, hook_count: 3, hook_spacing_steps: 1, mount_top_offset: 20 }
  let source = compact
  let reference = original.replace('friction_ridge_enabled = true;', 'friction_ridge_enabled = false;')
  source = source.replace('param friction_ridge_enabled = 1', 'param friction_ridge_enabled = 0')
  for (const [name, value] of Object.entries(changes)) {
    source = source.replace(new RegExp(`param ${name} = [0-9.]+`), `param ${name} = ${value}`)
    reference = reference.replace(new RegExp(`^${name} = [0-9.]+;`, 'm'), `${name} = ${value};`)
  }
  const a = await parseOpenSCAD(reference, { quality: 'full' })
  const b = await parseOpenSCAD(source, { quality: 'full' })
  expect(b.meshes.length).toBe(4)
  expect(b.volume).toBeCloseTo(a.volume, 7)
  expectBounds(b.meshes, a.meshes)
})

it.each([
  { width: 160, height: 100, hook_count: 3, hook_spacing_steps: 1, fit: 0.35, mount_top_offset: 20, friction_ridge_enabled: 0 },
  { width: 72, depth: 30, height: 50, wall: 2, floor_thickness: 2, corner_radius: 3, hook_spacing_steps: 1, fit: 0.15 },
])('recalculates the rounded shell and shared mounts after parameter updates: %j', async changes => {
  const compiled = compileModelGraphText(compact)
  const updated = setModelGraphParameters(compiled.document, compiled.document_sha256,
    Object.entries(changes).map(([id, value]) => ({ id, value })))
  let source = original
  for (const [name, value] of Object.entries(changes)) {
    source = source.replace(new RegExp(`^${name} = [^;]+;`, 'm'), `${name} = ${name === 'friction_ridge_enabled' ? Boolean(value) : value};`)
  }
  const reference = await parseOpenSCAD(source, { quality: 'full' }).catch(error => { throw new Error('Original OpenSCAD failed', { cause: error }) })
  const result = await parseOpenSCAD(updated.source, { quality: 'full' })
  expect(result.volume).toBeCloseTo(reference.volume, 7)
  expectBounds(result.meshes, reference.meshes)
})

it.each(['hook_count', 'hook_spacing_steps', 'part', 'friction_ridge_enabled'])('rejects fractional %s', name => {
  const value = name === 'part' || name === 'friction_ridge_enabled' ? 0.5 : 2.5
  expect(() => compileModelGraphText(compact.replace(new RegExp(`param ${name} = [0-9.]+`), `param ${name} = ${value}`))).toThrow()
})

it('rejects a corner radius smaller than the wall', () => {
  expect(() => compileModelGraphText(compact.replace('param wall = 3', 'param wall = 6'))).toThrow()
})

it('checks polygon, hull, sharp offsets and tessellation settings', () => {
  const source = '// @modelgraph-text/1\nsegments 40\nshow hull(polygon([[0,0],[4,0],[0,4]]), circle(1)).offset(delta: 0.2).extrude(2)'
  expect(compileModelGraphText(source).source).toContain('offset(delta=0.2)')
  expect(compileModelGraphText(source).source).toContain('hull(){')
  for (const bad of ['11', '129', '40.5', '40mm']) {
    expect(() => compileModelGraphText(source.replace('segments 40', `segments ${bad}`))).toThrow()
  }
  expect(() => compileModelGraphText(source.replace('segments 40', 'segments 40\nsegments 48'))).toThrow()
  expect(() => compileModelGraphText(source.replace('delta: 0.2', 'delta: 0.2, distance: 1'))).toThrow()
})


it('reports the required hook width and builds the narrow box with 40 mm spacing', async () => {
  const narrow = compact.replace('param width = 120', 'param width = 72.2399111440207')
    .replace('param depth = 75', 'param depth = 68.4089226212514')
  expect(() => compileModelGraphText(narrow)).toThrow(/Box too narrow for hooks.*72\.2399111440207.*112/)
  const result = await parseOpenSCAD(narrow.replace('param hook_spacing_steps = 2', 'param hook_spacing_steps = 1'), { quality: 'full' })
  expect(result.meshes).toHaveLength(3)
  expect(result.volume).toBeGreaterThan(0)
})
