import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'

const original = readFileSync('examples/skadis-box/skadis-dovetail.scad', 'utf8')
const compact = readFileSync('examples/skadis-box/skadis-dovetail.modelgraph.scad', 'utf8')

it.each([0, 1, 2])('preserves SKADIS geometry for output %i', async part => {
  const source = compact.replace('param part = 0', `param part = ${part}`)
  const compiled = compileModelGraphText(source)
  expect(compiled.document).toMatchObject({ language: 'modelgraph/1', segments: 40 })
  expect(compiled.source).not.toMatch(/module\s+hook/)
  const reference = await parseOpenSCAD(original.replace('part = 0;', `part = ${part};`), { quality: 'full' })
  const result = await parseOpenSCAD(source, { quality: 'full' })
  expect(result.meshes.length).toBe(part === 0 ? 3 : 1)
  expect(result.volume).toBeCloseTo(reference.volume, 7)
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
