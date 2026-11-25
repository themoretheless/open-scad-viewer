import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const example = readFileSync('examples/rush/curvature-biarc.r','utf8')
async function reports(source: string) {
  const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return scene.measurements!.map(entry => JSON.parse(entry.report))
}

it('reconstructs an offset straight line and exposes the fitted arc in the editor', async () => {
  const data = await reports(example)
  const path = data.find(row => row.sites)
  const arc = data.find(row => row.arcs)
  expect(path.lengthMm).toBeCloseTo(4)
  expect(path.sites.at(-1)[0]).toBeCloseTo(5)
  expect(path.sites.at(-1)[1]).toBeCloseTo(2)
  expect(arc.pieces).toBeGreaterThan(0)
  expect(arc.maxSampledErrorMm).toBeLessThan(0.001)
  await expect(reports(example.replace('piece:0','piece:999'))).rejects.toThrow()
})

it('integrates signed curvature in inverse millimetres and rejects reversed station order', async () => {
  const data = await reports(example.replaceAll('0/1mm','0.5/1mm'))
  const end = data.find(row => row.sites).sites.at(-1)
  expect(end[0]).toBeCloseTo(1 + 2 * Math.sin(2), 4)
  expect(end[1]).toBeCloseTo(2 + 2 * (1 - Math.cos(2)), 4)
  await expect(reports(example.replace('5mm,6mm','5mm,4mm'))).rejects.toThrow()
})
