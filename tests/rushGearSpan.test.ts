import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const example = readFileSync('examples/rush/gear-span-inspection.r', 'utf8')
const build = (code: string) => buildTextNurbsScene(compileRushFrontend(code).document)

it('recovers the authored thinning and shows a closed spur gear', async () => {
  const scene = await build(example)
  const model = JSON.parse(scene.meshes[0]!.nativeGeometry!.geometryJson).geometry
  expect(model.shells[0].closed).toBe(true)
  const report = JSON.parse(scene.measurements![0]!.report)
  const angle = 20 * Math.PI / 180
  const base = 24 * Math.cos(angle)
  const roll = Math.PI / 64 + Math.tan(angle) - angle - 0.1 / 48 + 3 * Math.PI / 32
  const contactRadius = base * Math.hypot(1, roll)
  const width = 2 * contactRadius * Math.sin(Math.atan(roll))
  expect(report.counts).toContain(4)
  expect(report.nominalWidthMm[0]).toBeLessThanOrEqual(width)
  expect(report.nominalWidthMm[1]).toBeGreaterThanOrEqual(width)
  expect(report.inferredThinningMm[0]).toBeCloseTo(0.1 - 0.001 / Math.cos(angle), 9)
  expect(report.inferredThinningMm[1]).toBeCloseTo(0.1 + 0.001 / Math.cos(angle), 9)
})

it('lists feasible spans before any measurement has been supplied', async () => {
  const scene = await build(example.replace(/span_count:4,measured_span_mm:\[[^\]]+\]/, 'inspect_spans:true'))
  const report = JSON.parse(scene.measurements![0]!.report)
  expect(report.counts).toContain(4)
  expect(report.nominalWidthMm).toBeNull()
  expect(report.inferredThinningMm).toBeNull()
})

it('rejects invalid observations and gear families outside the calculation domain', async () => {
  await expect(build(example.replace('span_count:4', 'helix_angle:12,span_count:4'))).rejects.toThrow(/external spur/i)
  await expect(build(example.replace('span_count:4,', ''))).rejects.toThrow(/tooth count/i)
  await expect(build(example.replace(/measured_span_mm:\[[^\]]+\]/, 'measured_span_mm:[100mm,101mm]'))).rejects.toThrow(/contact|thinning/i)
})
