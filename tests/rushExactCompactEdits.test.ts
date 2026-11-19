import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/exact-compact-strip.r', 'utf8')

it('preserves the strip and materializes the requested knot and degree edits', async () => {
  const { document } = compileRushFrontend(source)
  const id = (op: string) => document.nodes.find(n => n.op === op)!.id
  const result = await buildOwnNurbsAsync(document, {
    action: 'evaluate',
    evaluations: [{ node: id('surface_exact_reduce_degree'), u: 0.375, v: 0.625 }],
  })
  expect(result.evaluations[0]!.point).toEqual([3, 0, 2.5])
  const definitions = result.report.definitions
  expect(definitions[id('curve_exact_insert_knot')]).toMatchObject({ knots: [0,0,0,0.5,1,1,1] })
  expect(definitions[id('curve_exact_remove_knot')]).toMatchObject({ knots: [0,0,0,1,1,1] })
  expect(definitions[id('curve_exact_reduce_degree')]).toMatchObject({ degree: 1 })
  expect(definitions[id('surface_exact_insert_knot')]).toMatchObject({ knotsU: [0,0,0.5,1,1] })
  expect(definitions[id('surface_exact_remove_knot')]).toMatchObject({ knotsU: [0,0,1,1] })
  expect(definitions[id('surface_exact_reduce_degree')]).toMatchObject({ degreeV: 1 })
})

it.each(['curve', 'surface'].flatMap(kind =>
  ['insert_knot', 'remove_knot', 'reduce_degree'].map(edit => kind + '_exact_' + edit),
))('propagates the native work limit for %s', async op => {
  const start = source.indexOf(op + '(')
  expect(start).toBeGreaterThan(0)
  const limited = source.slice(0, start) + source.slice(start).replace('max_work:1000000', 'max_work:1')
  const { document } = compileRushFrontend(limited)
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(/refused/i)
})

it('refuses to straighten a genuinely curved quadratic', async () => {
  const { document } = compileRushFrontend(source.replace('[4mm,0,0]', '[4mm,1mm,0]'))
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(/refused/i)
})
