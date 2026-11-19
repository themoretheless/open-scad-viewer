import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/periodic-weighted-strip.r', 'utf8')

it('moves the curve and surface seams without changing the sampled wrapped positions', async () => {
  const { document } = compileRushFrontend(source)
  const evaluations = [
    ['curve_edit_weights', 2.375], ['curve_relocate_seam', 8.375],
    ['surface_edit_weights', 3.375], ['surface_relocate_seam', 9.375],
  ].map(([op, u]) => ({ node: document.nodes.find(n => n.op === op)!.id, u: Number(u), ...(String(op).startsWith('surface') ? { v: 0.625 } : {}) }))
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations })
  for (const i of [0, 2]) {
    result.evaluations[i]!.point.forEach((coordinate, axis) => {
      expect(result.evaluations[i + 1]!.point[axis]).toBeCloseTo(coordinate, 10)
    })
  }
})

it.each([
  ['[[0,3]]', '[[0,3],[6,3]]', /Duplicate logical curve weight/],
  ['[[2,1,3]]', '[[0,1,3],[6,1,3]]', /Duplicate logical surface weight/],
  ['max_cells:20000', 'max_cells:1', /budget|cells|limit/i],
] as const)('rejects conflicting aliases or insufficient verification work for %s', async (from, to, message) => {
  const { document } = compileRushFrontend(source.replace(from, to))
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(message)
})

it('applies the edited control weights in both rational denominators', async () => {
  const { document } = compileRushFrontend(`// @rush/1
line = bezier_curve(points:[[0,0,0],[8mm,0,0]])
weighted = line.curve_edit_weights(edits:[[1,3]])
panel = weighted.surface_extrude(vector:[0,0,4mm])
show panel.surface_edit_weights(edits:[[1,1,5]]).tessellate(8,8)`)
  const evaluations = [
    { node: document.nodes.find(n => n.op === 'curve_edit_weights')!.id, u: 0.5 },
    { node: document.nodes.find(n => n.op === 'surface_edit_weights')!.id, u: 0.5, v: 0.5 },
  ]
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations })
  expect(result.evaluations[0]!.point).toEqual([6, 0, 0])
  const point = result.evaluations[1]!.point
  expect(point[0]).toBeCloseTo(8 * (3 + 5) / 10, 12)
  expect(point[2]).toBeCloseTo(4 * (1 + 5) / 10, 12)
})
