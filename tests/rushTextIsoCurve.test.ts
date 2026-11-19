import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/iso-curve-strip.r', 'utf8')

it.each([
  ['u', [2.5, 6, 0.75]],
  ['v', [7.5, 2, 0.75]],
] as const)('fixes the %s coordinate and builds the section in the editor pipeline', async (axis, expected) => {
  const { document } = compileRushFrontend(source.replace('"u"', JSON.stringify(axis)))
  const section = document.nodes.find(n => n.op === 'iso_curve')!
  const evaluated = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations: [{ node: section.id, u: 0.75 }] })
  expected.forEach((value, i) => expect(evaluated.evaluations[0]!.point[i]).toBeCloseTo(value, 12))
  const built = await buildOwnNurbsAsync(document, { action: 'build' })
  expect(built.report.mesh!.triangleCount).toBeGreaterThan(0)
})

it('rejects invalid directions and dimensioned parameters', () => {
  expect(() => compileRushFrontend(source.replace('"u"', '"z"'))).toThrow()
  expect(() => compileRushFrontend(source.replace('0.25', '0.25mm'))).toThrow()
})

it('places an extracted rational section with a shear and translation without changing its weights', async () => {
  const program = source.replace('section.surface_extrude',
    'section.transform(matrix:[[1,2,0,3mm],[0,1,0,-2mm],[0,0,1,1mm],[0,0,0,1]]).surface_extrude')
    .replace('bezier_surface(points:', 'bezier_surface(weights:[[1,2],[1,2]],points:')
  const { document } = compileRushFrontend(program)
  const section = document.nodes.find(n => n.op === 'iso_curve')!
  const placed = document.nodes.find(n => n.op === 'transform')!
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate',
    evaluations: [section, placed].map(node => ({ node: node.id, u: 0.75 })) })
  const [x, y, z] = result.evaluations[0]!.point
  ;[x! + 2 * y! + 3, y! - 2, z! + 1].forEach((value, i) =>
    expect(result.evaluations[1]!.point[i]).toBeCloseTo(value, 12))
  for (const node of [section, placed]) expect(result.report.definitions[node.id]).toMatchObject({ weights: [1, 2] })
  const built = await buildOwnNurbsAsync(document, { action: 'build' })
  expect(built.report.mesh!.triangleCount).toBeGreaterThan(0)
})
