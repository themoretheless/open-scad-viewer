import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const code = readFileSync('examples/rush/original-geometry-distances.r','utf8')
const run = (source: string) => buildTextNurbsScene(compileRushFrontend(source).document)

it('encloses the independently known clearance for all five original-geometry queries', async () => {
  const scene = await run(code)
  expect(scene.meshes).toHaveLength(1)
  const measurements = scene.measurements!
  expect(measurements.map(m=>m.operation).sort()).toEqual([
    'curve_nearest_point','curve_pair_distance','curve_surface_distance','surface_nearest_point','surface_pair_distance',
  ].sort())
  for (const m of measurements) {
    const r = JSON.parse(m.report)
    expect(r.converged).toBe(true)
    expect(r.distanceIntervalMm[0]).toBeLessThanOrEqual(3)
    expect(r.distanceIntervalMm[1]).toBeGreaterThanOrEqual(3)
    expect(r.distanceIntervalMm[1]-r.distanceIntervalMm[0]).toBeLessThanOrEqual(0.001)
    if (m.operation === 'surface_nearest_point') expect(r.point).toEqual([2,1,0])
    if (m.operation === 'curve_nearest_point') expect(r.point).toEqual([2,0,0])
  }
})

it('rejects wrong geometry families and a point dimension inconsistent with its curve', async () => {
  await expect(run(code.replace('target_geometry:upper,','target_geometry:upper_wall,'))).rejects.toThrow(/curve/i)
  await expect(run(code.replace('probe_point:[2mm,0mm,3mm]','probe_point:[2mm,0mm]'))).rejects.toThrow(/dimension/i)
})
