import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildTextNurbsScene } from '../src/language/rushFrontendScene'

const program = readFileSync('examples/rush/axis-motion.r', 'utf8')

async function reports(source: string) {
  const scene = await buildTextNurbsScene(compileRushFrontend(source).document)
  return scene.measurements!.map(row => JSON.parse(row.report))
}

it('classifies limits, uniform slowdown, duration and deadline under two independent axes', async () => {
  const [limits, one, many, duration, deadline] = await reports(program)
  expect(limits.status).toBe('exceeds')
  for (const pair of [one.slowdownBounds, many.slowdownBounds, duration.durationBounds, many.axisBounds[0]]) {
    expect(pair[0]).toBeLessThanOrEqual(2)
    expect(pair[1]).toBeGreaterThanOrEqual(2)
    expect(pair[1]-pair[0]).toBeLessThan(1e-8)
  }
  expect(many.axisBounds[1]).toEqual([1,1])
  expect(deadline.axisStatuses).toEqual(['exceeds','within'])
  expect(deadline.status).toBe('exceeds')
  expect((await reports(program.replace('deadline:1.5','deadline:3'))).at(-1).status).toBe('within')
})

it('keeps an unbounded slowdown explicit when positive speed has a zero limit', async () => {
  const rows = await reports(program.replaceAll('[4,10,10]', '[0,10,10]'))
  expect(rows[1].slowdownBounds[1]).toBe('unbounded')
  expect(rows[3].durationBounds[1]).toBe('unbounded')
})

it.each([
  ['directions:[[1,0,0],[0,1,0]]','directions:[[1,0,0]]'],
  ['deadline:1.5','deadline:0'],
  ['tolerances:[0.001,0.001,0.001]','tolerances:[-1,0.001,0.001]'],
])('refuses inconsistent or invalid motion requests %s', async (from, to) => {
  await expect(reports(program.replace(from, to))).rejects.toThrow()
})

it('refuses a shared budget too small to evaluate both axes', async () => {
  const source = program.replace(/(curve_axis_motion_multi_slowdown\([^\n]*)max_cells:192/, '$1max_cells:1')
  expect(source).not.toBe(program)
  await expect(reports(source)).rejects.toThrow(/budget|cells/i)
})
