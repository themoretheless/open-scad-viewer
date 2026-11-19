import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'
import { compileRushFrontend } from '../src/language/rushFrontend'

const source = readFileSync('examples/rush/directional-travel.r', 'utf8')

it('returns the measured intervals in the editor scene while preserving the source geometry', async () => {
  const result = await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(result.meshes.length).toBe(1)
  const rows = result.measurements ?? []
  expect(rows.map(row => row.operation)).toEqual(['curve_measure_directional_travel','curve_measure_directional_travel_batch'])
  const single = JSON.parse(rows[0]!.report)
  expect(single.reverseStatus).toBe('exceeds')
  expect(single.parameterInterval).toEqual([0.25, 0.75])
  for (const endpoint of single.reverseBoundsMm) expect(endpoint).toBeCloseTo(4, 8)
  const batch = JSON.parse(rows[1]!.report)
  expect(batch.intervals.map((row: { reverseStatus:string }) => row.reverseStatus)).toEqual(['within','within'])
  for (const [i, length] of [2,6].entries()) {
    for (const endpoint of batch.intervals[i].forwardBoundsMm) expect(endpoint).toBeCloseTo(length, 8)
  }
})

it.each([
  ['axis_vector:[-1,0,0]', 'axis_vector:[0,0,0]'],
  ['reverse_limit:3mm', 'reverse_limit:-1mm'],
  ['interval:[0.25,0.75]', 'interval:[0.75,0.25]'],
  ['max_cells:128', 'max_cells:0'],
])('refuses invalid measurement input %s', async (from, to) => {
  await expect(async () => buildTextNurbsScene(compileRushFrontend(source.replace(from, to)).document)).rejects.toThrow()
})
