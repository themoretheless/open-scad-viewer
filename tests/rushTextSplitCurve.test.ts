import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/split-nurbs-span.r', 'utf8')
it.each([0, 1])('preserves the rational curve on split side %i in its original domain', async part => {
  const { document } = compileRushFrontend(source.replace('part:1', 'part:' + part))
  const nodes = ['curve', 'curve_bezier_span', 'curve_split'].map(op => document.nodes.find(n => n.op === op)!)
  const u = part ? 2.5 : 1.5
  const result = await buildOwnNurbsAsync(document, {
    action: 'evaluate', evaluations: nodes.map(node => ({ node: node.id, u })),
  })
  const t = (u - 1) / 2
  const expected = [(2 * (1-t) + 5*t)/(2-t), (4 * (1-t) + 4*t)/(2-t), 0]
  for (const sample of result.evaluations)
    expected.forEach((value, axis) => expect(sample.point[axis]).toBeCloseTo(value, 12))
  expect(result.report.definitions[nodes[2]!.id]).toMatchObject({ knots: part ? [2,2,3,3] : [1,1,2,2] })
  expect((await buildOwnNurbsAsync(document, { action: 'build' })).report.mesh!.triangleCount).toBeGreaterThan(0)
})
it('refuses a missing span or a split outside the retained interval', async () => {
  for (const invalid of [source.replace('index:1','index:2'), source.replace('parameter:2','parameter:0.5')])
    await expect(buildOwnNurbsAsync(compileRushFrontend(invalid).document, { action: 'build' })).rejects.toThrow()
})
