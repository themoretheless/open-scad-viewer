import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/edited-nurbs-strip.r', 'utf8')

it('evaluates the trimmed and reversed profile and surface from editable Rush text', async () => {
  const { document } = compileRushFrontend(source)
  const curve = document.nodes.find(n => n.op === 'curve_edit')!
  const surface = document.nodes.find(n => n.op === 'surface_edit')!
  const result = await buildOwnNurbsAsync(document, {
    action: 'evaluate', evaluations: [{ node: curve.id, u: 0.1 }, { node: surface.id, u: 0.2, v: 0.25 }],
  })
  expect(result.report.definitions[curve.id]).toMatchObject({ degree: 2, knots: expect.arrayContaining([0.5]) })
  expect(result.report.definitions[surface.id]).toMatchObject({ degreeV: 2, knotsV: expect.arrayContaining([0.5]) })
  for (const [i, expected] of [[9, 0, 0], [8, 6, 0]].entries()) {
    expected.forEach((value, axis) => expect(result.evaluations[i]!.point[axis]).toBeCloseTo(value, 10))
  }
  const built = await buildOwnNurbsAsync(document, { action: 'build' })
  expect(built.report.mesh!.triangleCount).toBeGreaterThan(0)
})

it('rejects a dimensioned knot and an out-of-domain trim', async () => {
  expect(() => compileRushFrontend(source.replace('value:0.5','value:0.5mm'))).toThrow()
  await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('0.1,0.9','0.1,2')).document,
    { action: 'build' })).rejects.toThrow()
})
