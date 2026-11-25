import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbs } from '../src/language/rushGraphNurbsKernel'
import { opticalLens } from '../src/kernel/nurbsConstructors'

const example = (name: string) => readFileSync('examples/rush/' + name + '.r', 'utf8')
const build = (code: string) => buildOwnNurbs(compileRushFrontend(code).document, { action: 'build' })

it('preserves the major cap and its elliptic spherical profile in the rendered mesh', () => {
  const mesh = build(example('optical-dome')).mesh!
  expect(mesh.indices.length).toBeGreaterThan(0)
  let top = -Infinity
  for (let i = 0; i < mesh.positions.length; i += 3) {
    const x = mesh.positions[i]!, y = mesh.positions[i + 1]!, z = mesh.positions[i + 2]!
    // R=(a²+h²)/(2h)=4.1 and the sphere center lies 0.9 mm above the rim.
    expect(x*x + (4*y/3)**2 + (z-0.9)**2).toBeCloseTo(4.1**2, 4)
    top = Math.max(top, z)
  }
  expect(top).toBeCloseTo(5)
})

it('builds a lens with its barrel, and omits the barrel at zero rim thickness', () => {
  expect(build(example('optical-lens')).mesh!.indices.length).toBeGreaterThan(0)
  const [upper, lower, barrel] = opticalLens([0, 0, 0], [4, 3], [1, 2], 4)
  expect(barrel!.controlPoints[0]).toEqual(lower!.controlPoints.at(-1))
  expect(barrel!.controlPoints[1]).toEqual(upper!.controlPoints.at(-1))
  expect(opticalLens([0, 0, 0], [4, 3], [1, 2], 3)).toHaveLength(2)
  expect(() => build(example('optical-lens').replace('thickness:4mm', 'thickness:2mm'))).toThrow(/thickness/i)
})
