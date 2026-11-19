import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const program = readFileSync('examples/rush/axial-derivative-peaks.r', 'utf8')

it('bounds all three derivatives of 8t³ and encloses their maximizers', async () => {
  const scene = await buildTextNurbsScene(compileRushFrontend(program).document)
  expect(scene.measurements).toHaveLength(6)
  for (const [index, row] of scene.measurements!.entries()) {
    const report = JSON.parse(row.report)
    const order = Math.floor(index / 2) + 1
    const exact = order === 1 ? 24 : 48
    expect(report.order).toBe(order)
    expect(report.peakBounds[0]).toBeLessThanOrEqual(exact)
    expect(report.peakBounds[1]).toBeGreaterThanOrEqual(exact)
    expect(report.peakBounds[1] - report.peakBounds[0]).toBeLessThan(0.011)
    if (index % 2 === 0) expect(report.parameterIntervals).toBeNull()
    else {
      expect(report.parameterIntervals.at(-1)[1]).toBe(1)
      if (order === 3) expect(report.parameterIntervals).toEqual([[0, 1]])
    }
  }
})

it.each([
  ['order:1', 'order:4'],
  ['parameter_tolerance:0.05', 'parameter_tolerance:0'],
  ['interval:[0,1]', 'interval:[0,2]'],
  ['axis_vector:[1,0,0]', 'axis_vector:[0,0,0]'],
])('rejects invalid peak measurement arguments %s', async (from, to) => {
  await expect(async () => buildTextNurbsScene(compileRushFrontend(program.replace(from, to)).document)).rejects.toThrow()
})
