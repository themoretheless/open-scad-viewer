import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/split-bezier-panel.r', 'utf8')

it.each([
  ['u', 0, 0.125, 0.75],
  ['u', 1, 0.625, 0.75],
  ['v', 0, 0.625, 0.625],
  ['v', 1, 0.625, 0.875],
] as const)('retains the original coordinates on %s side %i', async (direction, part, u, v) => {
  const program = source.replace('part:1', 'part:' + part)
    .replace('direction:"u"', 'direction:"' + direction + '"')
    .replace('parameter:0.25', direction === 'v' ? 'parameter:0.75' : 'parameter:0.25')
  const { document } = compileRushFrontend(program)
  const node = document.nodes.find(n => n.op === 'surface_split')!.id
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations: [{ node, u, v }] })
  const point = result.evaluations[0]!.point
  for (const [i, expected] of [10 * v, 8 * u, 4 * u * v].entries()) {
    expect(point[i]).toBeCloseTo(expected, 12)
  }
})

it.each([
  ['index:1', 'index:2', /Bezier patch index/],
  ['parameter:0.25', 'parameter:0', /strictly inside/],
] as const)('rejects invalid %s', async (field, invalid, message) => {
  const { document } = compileRushFrontend(source.replace(field, invalid))
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(message)
})
