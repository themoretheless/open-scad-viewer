import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source = readFileSync('examples/rush/cst-measured-sides.r','utf8')
async function build(text: string) {
  return buildTextNurbsScene(compileRushFrontend(text).document)
}

it('fits labeled cambered samples in a rotated and translated chord frame',async()=>{
  const scene = await build(source)
  expect(scene.meshes).toHaveLength(1)
  const report = JSON.parse(scene.measurements![0]!.report)
  for(const value of report.upperCoefficients) expect(value).toBeCloseTo(0.4,5)
  for(const value of report.lowerCoefficients) expect(value).toBeCloseTo(0.2,5)
  expect(report.maxOrdinateResidualMm).toBeLessThan(1e-6)
  expect(report.maxOutOfPlaneMm).toBeLessThan(1e-12)
})
it('rejects a thickness direction parallel to the authored chord',async()=>{
  await expect(build(source.replace('ordinate_direction:[1,0,0]','ordinate_direction:[0,1,0]'))).rejects.toThrow()
})
