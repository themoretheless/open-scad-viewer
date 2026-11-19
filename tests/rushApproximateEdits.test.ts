import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/simplify-nurbs-strip.r', 'utf8')

it('preserves the planar strip through six tolerance-controlled simplifications', async () => {
  const { document } = compileRushFrontend(source)
  const result = await buildOwnNurbsAsync(document, { action: 'build' })
  expect(result.report.mesh!.triangleCount).toBeGreaterThan(0)
  const node = document.nodes.find(n => n.op === 'surface_rebuild')!.id
  const evaluations = [[0, 0], [0.25, 0.75], [1, 1]].map(([u, v]) => ({ node, u: u!, v: v! }))
  const surface = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations })
  surface.evaluations.forEach(({ point }, index) => {
    const { u, v } = evaluations[index]!
    expect(point[0]).toBeCloseTo(8 * u, 10)
    expect(point[1]).toBeCloseTo(0, 10)
    expect(point[2]).toBeCloseTo(4 * v, 10)
  })
})

it('reports refusal when a curved profile cannot be straightened within zero deviation', async () => {
  const text = source.replace('[4mm,0,0]', '[4mm,3mm,0]').replace('degree:1,max_deviation:0.01mm', 'degree:1,max_deviation:0mm')
  const { document } = compileRushFrontend(text)
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(/deviation budget/)
})

it.each([
  ['curve_remove_knot(parameter:0.4,max_deviation:1mm)', 'curve', /absent/],
  ['curve_reduce_degree(degree:1,max_deviation:0mm)', 'curve', /deviation budget/],
  ['curve_rebuild(degree:1,control_count:2,max_deviation:0mm)', 'curve', /deviation budget/],
  ['surface_remove_knot(direction:"u",parameter:0.4,max_deviation:1mm)', 'surface', /absent/],
  ['surface_reduce_degree(direction:"u",degree:1,max_deviation:0mm)', 'surface', /deviation budget/],
  ['surface_rebuild(direction:"u",degree:1,control_count:2,max_deviation:0mm)', 'surface', /deviation budget/],
] as const)('propagates the native refusal from %s', async (operation, kind, error) => {
  const extrusion = 'surface_extrude(vector:[0,0,4mm])'
  const edit = kind === 'curve' ? operation + '.' + extrusion : extrusion + '.' + operation
  const { document } = compileRushFrontend('// @rush/1\nprofile = bezier_curve(points:[[0,0,0],[4mm,3mm,0],[8mm,0,0]])\nshow profile.' + edit + '.tessellate(8,8)')
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(error)
})
