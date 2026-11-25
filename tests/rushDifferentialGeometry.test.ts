import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const source = readFileSync('examples/rush/curvature-and-torsion.r','utf8')
const build = (code: string) => buildTextNurbsScene(compileRushFrontend(code).document)
function contains(bounds: number[], value: number) {
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
}

it('encloses curvature, torsion and both oriented wall curvatures for an extruded parabola', async () => {
  const scene = await build(source)
  expect(scene.meshes).toHaveLength(1)
  const reports = new Map(scene.measurements!.map(m => [m.operation,JSON.parse(m.report)]))
  const curve = reports.get('curve_curvature_torsion'), wall = reports.get('surface_principal_curvatures')
  contains(curve.curvaturePerMm, Math.SQRT1_2)
  contains(curve.torsionPerMm, 0)
  expect(curve.side).toBe('TwoSided')
  expect(curve.frameStatus).toBe('Available')
  contains(wall.principalPerMm[0], -Math.SQRT1_2)
  contains(wall.principalPerMm[1], 0)
  contains(wall.gaussianPerMm2, 0)
  expect(wall.directionStatus).toBe('Available')
  expect(wall.directions).toHaveLength(2)
})

it('retains undefined torsion and refuses a derivative side outside the domain', async () => {
  const scene = await build(source.replace('[1mm,1mm,0mm]', '[1mm,0mm,0mm]'))
  const c = JSON.parse(scene.measurements!.find(m=>m.operation==='curve_curvature_torsion')!.report)
  expect(c.torsionPerMm).toBeNull()
  expect(c.torsionStatus).toBe('CurvatureNotSeparatedFromZero')
  await expect(build(source.replace('parameter:0.5,query_side:"automatic"', 'parameter:0,query_side:"left"'))).rejects.toThrow(/side/i)
})
