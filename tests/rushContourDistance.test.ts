import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source = readFileSync('examples/rush/curvature-contour-distance.r','utf8')
async function measure(text: string) {
  const scene = await buildTextNurbsScene(compileRushFrontend(text).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('encloses a known separation between contours sampled on different grids', async () => {
  const result = await measure(source)
  const [lower,upper] = result.distanceBoundsMm
  expect(lower).toBeLessThanOrEqual(1)
  expect(upper).toBeGreaterThanOrEqual(1)
  expect(upper-lower).toBeLessThan(0.01)
  expect(result.upperSegments).toHaveLength(2)
  expect(result.comparedPairs+result.prunedPairs).toBe(4)
})
it('detects coincident branches and refuses an exhausted shared work budget', async () => {
  const sameSurface = source.replace(/^target = .*\n/m, '').replace('target_geometry:target','target_geometry:panel')
  const coincident = await measure(sameSurface)
  expect(coincident.distanceBoundsMm[0]).toBe(0)
  await expect(measure(source.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})
