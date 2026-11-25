import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const source = readFileSync('examples/rush/curve-and-surface-metrology.r', 'utf8')
const run = (code: string) => buildTextNurbsScene(compileRushFrontend(code).document)
function encloses(bounds: number[], value: number) {
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
}

it('measures the original diagonal, half-length station, equal divisions and extruded area', async () => {
  const scene = await run(source)
  expect(scene.meshes).toHaveLength(1)
  const reports = new Map(scene.measurements!.map(m => [m.operation, JSON.parse(m.report)]))
  const length = reports.get('curve_length_bounds')
  encloses(length.boundsMm, 10)
  expect(length.withinTolerance).toBe(true)
  expect(length.errorUpperMm).toBeLessThanOrEqual(0.001)
  const station = reports.get('curve_station_at_length')
  encloses(station.parameterBounds, 0.5)
  expect(station.pointMm[0]).toBeCloseTo(3, 3)
  expect(station.pointMm[1]).toBeCloseTo(4, 3)
  const division = reports.get('curve_divide_by_length')
  expect(division.points).toHaveLength(5)
  for (let i=0; i<=4; i++) encloses(division.points[i].parameterBounds, i/4)
  const area = reports.get('surface_area_bounds')
  encloses(area.boundsMm2, 20)
  expect(area.errorUpperMm2).toBeLessThanOrEqual(0.001)
})

it('refuses incompatible length and area units and stations outside the source curve', async () => {
  expect(() => compileRushFrontend(source.replace('0.001mm*1mm','0.001mm'))).toThrow()
  expect(() => compileRushFrontend(source.replace('distance_mm:5mm','distance_mm:5deg'))).toThrow()
  await expect(run(source.replace('distance_mm:5mm', 'distance_mm:15mm'))).rejects.toThrow()
})
