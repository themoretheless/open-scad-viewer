import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const example = (name: string) => readFileSync('examples/rush/exact-' + name + '.r', 'utf8')
const restriction = example('curve-restriction')

it.each([
  ['factor-curve', 'curve_cancel_factor'],
  ['factor-panel', 'surface_cancel_factor'],
])('cancels the common factor while preserving the rational parameter law of %s', async (name, op) => {
  const { document } = compileRushFrontend(example(name))
  const id = (type: string) => document.nodes.find(n => n.op === type)!.id
  const evaluations = [0, 0.125, 0.5, 0.875, 1].map(u => ({
    node: id(op), u, ...(name === 'factor-panel' ? { v: 0.375 } : {}),
  }))
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations })
  expect(result.report.definitions[id('curve_normalize_weights')]).toMatchObject({ weights: [0.25, 0.5, 1] })
  expect(result.report.definitions[id(op)]).toMatchObject(name === 'factor-curve' ? { degree: 1 } : { degreeU: 1 })
  evaluations.forEach(({ u }, index) => {
    const point = result.evaluations[index]!.point
    expect(point[0]).toBeCloseTo(2 * u / (1 + u), 12)
    expect(point[2]).toBeCloseTo(name === 'factor-panel' ? 0.75 : 0, 12)
  })
})

it('retains the original clock through elevation, split, restriction and partition', async () => {
  const { document } = compileRushFrontend(restriction)
  const id = (op: string) => document.nodes.find(n => n.op === op)!.id
  const result = await buildOwnNurbsAsync(document, {
    action: 'evaluate', evaluations: [{ node: id('curve_exact_partition'), u: 0.8125 }],
  })
  expect(result.evaluations[0]!.point).toEqual([6.5, 0, 0])
  for (const [op, start, end] of [
    ['curve_exact_elevate', 0, 1], ['curve_exact_split', 0.5, 1],
    ['curve_exact_restrict', 0.625, 1], ['curve_exact_partition', 0.75, 0.875],
  ] as const) {
    expect(result.report.definitions[id(op)]).toMatchObject({ degree: 2, knots: [start,start,start,end,end,end] })
  }
})

it.each([
  [restriction, 'cuts:[0.75,0.875]', 'cuts:[0.875,0.75]', /increase/i],
  [restriction, '0.875],part:1', '0.875],part:3', /index outside/i],
  [example('factor-curve'), 'max_work:1000000', 'max_work:1', /ResourceLimit/],
  [example('factor-panel'), 'weights:[1,2,4]', 'weights:[1,1,1]', /NoCommonFactor/],
])('preserves native refusals for %s', async (source, from, to, message) => {
  const { document } = compileRushFrontend(String(source).replace(String(from), String(to)))
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(message as RegExp)
})
