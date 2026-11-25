import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source = readFileSync('examples/rush/curvature-contour-section.r','utf8')
async function section(text: string) {
  const scene = await buildTextNurbsScene(compileRushFrontend(text).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('localizes the plane crossing in original transverse knot coordinates', async () => {
  const {events} = await section(source)
  expect(events).toHaveLength(1)
  expect(events[0]).toMatchObject({segment:0,status:'crossing'})
  expect(events[0].transverse[0]).toBeLessThanOrEqual(7.5)
  expect(events[0].transverse[1]).toBeGreaterThanOrEqual(7.5)
})
it('certifies separation and rejects an invalid plane', async () => {
  expect((await section(source.replace('0.25mm,max_work','2mm,max_work'))).events).toEqual([])
  await expect(section(source.replace('plane_normal:[0,1,0]','plane_normal:[0,0,0]'))).rejects.toThrow()
})
