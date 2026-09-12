import { describe, expect, it } from 'vitest'
import {
  classifyResidual,
  orient2dExact,
  orient2dFilter,
  orient3dExact,
  rational,
  rationalFromNumber,
} from './support/referencePredicateOracleV1'

describe('G0.7 independent predicate oracle', () => {
  it('orient2dExact matches CCW / CW / collinear', () => {
    const z = rational(0)
    expect(
      orient2dExact(z, z, rational(1), z, rational(0), rational(1)),
    ).toBe(1)
    expect(
      orient2dExact(z, z, rational(0), rational(1), rational(1), z),
    ).toBe(-1)
    expect(
      orient2dExact(z, z, rational(2), z, rational(4), z),
    ).toBe(0)
  })

  it('orient2dFilter agrees on easy cases and stays indeterminate near zero', () => {
    expect(orient2dFilter(0, 0, 1, 0, 0, 1)).toBe(1)
    expect(orient2dFilter(0, 0, 0, 1, 1, 0)).toBe(-1)
    // Nearly collinear — filter may be indeterminate; exact resolves.
    const ax = 0
    const ay = 0
    const bx = 1
    const by = 0
    const cx = 0.5
    const cy = Number.EPSILON
    const filtered = orient2dFilter(ax, ay, bx, by, cx, cy)
    const exact = orient2dExact(
      rationalFromNumber(ax),
      rationalFromNumber(ay),
      rationalFromNumber(bx),
      rationalFromNumber(by),
      rationalFromNumber(cx),
      rationalFromNumber(cy),
    )
    if (filtered !== 'indeterminate') expect(filtered).toBe(exact)
    expect(exact).toBe(1)
  })

  it('orient3dExact signs a right-handed tetrahedron', () => {
    const o = [rational(0), rational(0), rational(0)] as const
    const x = [rational(1), rational(0), rational(0)] as const
    const y = [rational(0), rational(1), rational(0)] as const
    const z = [rational(0), rational(0), rational(1)] as const
    expect(orient3dExact(o, x, y, z)).toBe(1)
    expect(orient3dExact(o, x, z, y)).toBe(-1)
  })

  it('model classifier keeps gray band as Indeterminate', () => {
    expect(classifyResidual(1e-9, 1e-6, 1e-3)).toBe('Coincident')
    expect(classifyResidual(1e-2, 1e-6, 1e-3)).toBe('Separate')
    expect(classifyResidual(5e-4, 1e-6, 1e-3)).toBe('Indeterminate')
  })

  it('never coerces Indeterminate to falsey success', () => {
    const gray = classifyResidual(5e-4, 1e-6, 1e-3)
    expect(gray === 'Indeterminate').toBe(true)
    expect(gray === 'Coincident').toBe(false)
    expect(gray === 'Separate').toBe(false)
  })
})
