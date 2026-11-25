import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbs } from '../src/language/rushGraphNurbsKernel'
import { corrugatedBellows } from '../src/kernel/nurbsConstructors'
import { evaluateNurbsSurface } from '../src/kernel/nurbsSurface'

const source = readFileSync('examples/rush/corrugated-bellows.r', 'utf8')
const build = (text: string) => buildOwnNurbs(compileRushFrontend(text).document, { action: 'build' })

it('retains both inner and outer skins across a native patch split and both annular ends', () => {
  const mesh = build(source).mesh!
  expect(mesh.indices.length).toBeGreaterThan(0)
  const patches = corrugatedBellows([0, 0, 0], 8, 2, 24, 1, 6)
  expect(patches).toHaveLength(6)
  for (const u of [0, 1/12, 0.35, 10/12, 0.95, 1]) {
    const i = u <= 10/12 ? 0 : 1
    const a = evaluateNurbsSurface(patches[i]!, u, 0).point
    const b = evaluateNurbsSurface(patches[i + 2]!, u, 0).point
    expect(a[0]! - b[0]!).toBeCloseTo(1, 10)
    expect(a[2]).toBeCloseTo(24 * u, 10)
    expect(b[2]).toBeCloseTo(24 * u, 10)
  }
  for (const [i, z] of [[4, 0], [5, 24]] as const) {
    const p = evaluateNurbsSurface(patches[i]!, 0.5, 0).point
    expect(p[0]).toBeCloseTo(9.5, 10)
    expect(p[2]).toBeCloseTo(z, 10)
  }
})

it('rejects a collapsed cavity or dimensioned period count', () => {
  expect(() => build(source.replace('thickness:1mm', 'thickness:6mm'))).toThrow(/radius/i)
  expect(() => build(source.replace('periods:6', 'periods:6mm'))).toThrow()
})
