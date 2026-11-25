import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const source = readFileSync('examples/rush/edit-deviation.r', 'utf8')

async function measure(program: string) {
  const scene = await buildTextNurbsScene(compileRushFrontend(program).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(row => [row.operation, JSON.parse(row.report)]))
}

it('encloses a known three millimetre translation over both whole domains', async () => {
  const reports = await measure(source)
  const curve = reports.get('curve_edit_deviation')
  expect(curve.converged).toBe(true)
  expect(curve.boundsMm[0]).toBeLessThanOrEqual(3)
  expect(curve.boundsMm[1]).toBeGreaterThanOrEqual(3)
  expect(curve.boundsMm[1] - curve.boundsMm[0]).toBeLessThanOrEqual(0.01)
  const surface = reports.get('surface_edit_deviation')
  expect(surface.upperMm).toBeGreaterThanOrEqual(3)
  expect(surface.upperMm).toBeCloseTo(3, 10)
})

it('bounds identical curves and gives an exact zero for identical surfaces', async () => {
  const reports = await measure(source.replaceAll('3mm', '0mm'))
  expect(reports.get('curve_edit_deviation').boundsMm[1]).toBeLessThanOrEqual(0.01)
  expect(reports.get('surface_edit_deviation').upperMm).toBe(0)
})
