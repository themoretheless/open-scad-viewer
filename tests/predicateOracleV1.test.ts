import { describe, expect, it } from 'vitest'
import {
  addInterval,
  classifyResidual,
  cmpRational,
  compareSquaredDistanceExact,
  compareSquaredDistanceFilter,
  mulInterval,
  orient2dExact,
  orient2dFilter,
  orient3dExact,
  orient3dFilter,
  outwardInterval,
  rational,
  rationalFromNumber,
  subInterval,
  type BigRational,
  type Interval,
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

// Candidate correctness regressions; these do not amend frozen qualification
// artifacts or assert that a production predicate implementation is qualified.
describe('predicate oracle arithmetic boundaries', () => {
  const r = rationalFromNumber
  const min = Number.MIN_VALUE
  const max = Number.MAX_VALUE

  function contains(enclosure: Interval, value: BigRational) {
    expect(Number.isNaN(enclosure.lo) || Number.isNaN(enclosure.hi)).toBe(false)
    expect(enclosure.lo).toBeLessThanOrEqual(enclosure.hi)
    if (enclosure.lo !== -Infinity) expect(cmpRational(r(enclosure.lo), value)).toBeLessThanOrEqual(0)
    if (enclosure.hi !== Infinity) expect(cmpRational(r(enclosure.hi), value)).toBeGreaterThanOrEqual(0)
  }

  it('steps outward at zero, subnormal values and both finite extremes', () => {
    expect(outwardInterval(0)).toEqual({ lo: -min, hi: min })
    expect(outwardInterval(-0)).toEqual({ lo: -min, hi: min })
    expect(outwardInterval(min)).toEqual({ lo: 0, hi: 2 * min })
    expect(outwardInterval(-min)).toEqual({ lo: -2 * min, hi: -0 })
    expect(outwardInterval(max).hi).toBe(Infinity)
    expect(outwardInterval(-max).lo).toBe(-Infinity)
    for (const value of [0, min, -min, 2 ** -1022, -(2 ** -1022), 1, -1, max, -max]) {
      contains(outwardInterval(value), r(value))
    }
    for (const value of [NaN, Infinity, -Infinity]) expect(() => outwardInterval(value)).toThrow()
  })

  it('encloses each rounded arithmetic operation, including overflow and underflow', () => {
    const pairs: readonly (readonly [Interval, Interval])[] = [
      [{ lo: min, hi: min }, { lo: min, hi: min }],
      [{ lo: -min, hi: min }, { lo: -min, hi: min }],
      [{ lo: max, hi: max }, { lo: max, hi: max }],
      [{ lo: max, hi: max }, { lo: -max, hi: -max }],
      [{ lo: 1, hi: 1 }, { lo: 2 ** -54, hi: 2 ** -54 }],
      [{ lo: -(2 ** 600), hi: 2 ** 600 }, { lo: -(2 ** -600), hi: 2 ** -600 }],
    ]
    for (const [a, b] of pairs) {
      const sum = addInterval(a, b)
      const difference = subInterval(a, b)
      const product = mulInterval(a, b)
      for (const x of [a.lo, a.hi].map(r)) {
        for (const y of [b.lo, b.hi].map(r)) {
          contains(sum, rational(x.n * y.d + y.n * x.d, x.d * y.d))
          contains(difference, rational(x.n * y.d - y.n * x.d, x.d * y.d))
          contains(product, rational(x.n * y.n, x.d * y.d))
        }
      }
    }
    const whole = { lo: -Infinity, hi: Infinity }
    expect(mulInterval({ lo: 0, hi: 0 }, whole)).toEqual(whole)
    expect(subInterval(whole, whole)).toEqual(whole)
    expect(addInterval(whole, whole)).toEqual(whole)
  })

  it('does not report a zero orient2d sign after determinant underflow', () => {
    for (const scale of [min, 2 ** -600]) {
      expect(orient2dFilter(0, 0, scale, 0, 0, scale)).toBe('indeterminate')
      expect(orient2dExact(r(0), r(0), r(scale), r(0), r(0), r(scale))).toBe(1)
    }
  })

  it('checks 2D signs against independent rational arithmetic over extreme and cancelling coordinates', () => {
    const cases: readonly (readonly [number, number, number, number, number, number])[] = [
      [0, 0, min, 0, 0, min],
      [0, 0, 2 ** -600, 0, 0, 2 ** -600],
      [0, 0, 2 ** 600, 0, 0, 2 ** 600],
      [0, 0, max, 0, 0, max],
      [max, max, -max, max, max, -max],
      [0, 0, 2 ** 900, 0, 0, 2 ** -900],
      [2 ** 52, 2 ** 52, 2 ** 52 + 1, 2 ** 52 + 1, 2 ** 52 + 2, 2 ** 52 + 3],
      [0, 0, 2 ** -600, 2 ** -600, 2 ** -599, 2 ** -599],
    ]
    for (const coordinates of cases) {
      const [ax, ay, bx, by, cx, cy] = coordinates
      const exact = orient2dExact(r(ax), r(ay), r(bx), r(by), r(cx), r(cy))
      const filtered = orient2dFilter(...coordinates)
      if (filtered !== 'indeterminate') expect(filtered).toBe(exact)
      expect(orient2dExact(r(ax), r(ay), r(cx), r(cy), r(bx), r(by))).toBe(exact === 0 ? 0 : -exact)
    }
  })

  it('checks 3D handedness at subnormal, overflow and cancellation limits', () => {
    type Point = readonly [number, number, number]
    const cases: readonly (readonly [Point, Point, Point, Point, -1 | 0 | 1])[] = [
      [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1], 1],
      [[0, 0, 0], [min, 0, 0], [0, min, 0], [0, 0, min], 1],
      [[0, 0, 0], [2 ** -400, 0, 0], [0, 2 ** -400, 0], [0, 0, 2 ** -400], 1],
      [[0, 0, 0], [2 ** 400, 0, 0], [0, 2 ** 400, 0], [0, 0, 2 ** 400], 1],
      [[max, max, max], [-max, max, max], [max, -max, max], [max, max, -max], -1],
      [[0, 0, 0], [2 ** 900, 0, 0], [0, 2 ** -900, 0], [0, 0, min], 1],
      [[0, 0, 0], [max, 0, 0], [0, max, 0], [max, max, 0], 0],
    ]
    const point = (p: Point) => [r(p[0]), r(p[1]), r(p[2])] as const
    for (const [a, b, c, d, expected] of cases) {
      const exact = orient3dExact(point(a), point(b), point(c), point(d))
      expect(exact).toBe(expected)
      const filtered = orient3dFilter(a, b, c, d)
      if (filtered !== 'indeterminate') expect(filtered).toBe(exact)
      expect(orient3dExact(point(a), point(b), point(d), point(c))).toBe(exact === 0 ? 0 : -exact)
    }
    expect(orient3dFilter([0, 0, 0], [min, 0, 0], [0, min, 0], [0, 0, min])).toBe('indeterminate')
  })

  it('compares squared distances with exact rational radii and dimensional checks', () => {
    const origin = [r(0), r(0)]
    expect(compareSquaredDistanceExact(origin, [r(3), r(4)], r(5))).toBe(0)
    expect(compareSquaredDistanceExact(origin, [r(3), r(4)], r(4))).toBe(1)
    expect(compareSquaredDistanceExact(origin, [r(3), r(4)], r(6))).toBe(-1)
    expect(compareSquaredDistanceExact(origin, [rational(1, 3), r(0)], rational(1, 3))).toBe(0)
    expect(compareSquaredDistanceExact([r(0), r(0), r(0)], [r(1), r(2), r(2)], r(3))).toBe(0)
    expect(() => compareSquaredDistanceExact(origin, [r(0)], r(1))).toThrow(/matching/)
    expect(() => compareSquaredDistanceExact(origin, origin, r(-1))).toThrow(/nonnegative/)
    expect(() => compareSquaredDistanceFilter([0, 0], [0, 0, 0], 1)).toThrow(/matching/)
    expect(() => compareSquaredDistanceFilter([0, 0], [0, 0], -1)).toThrow(/nonnegative/)
    for (const radius of [NaN, Infinity]) expect(() => compareSquaredDistanceFilter([0, 0], [0, 0], radius)).toThrow()
  })

  it('keeps distance filters conservative around exact equality and exponent extremes', () => {
    const cases: readonly (readonly [readonly number[], readonly number[], number, -1 | 0 | 1])[] = [
      [[0, 0], [3, 4], 5, 0],
      [[0, 0], [1 + Number.EPSILON, 0], 1, 1],
      [[0, 0], [1 - Number.EPSILON, 0], 1, -1],
      [[0, 0], [min, 0], 0, 1],
      [[0, 0], [2 ** -600, 2 ** -600], 2 ** -600, 1],
      [[0, 0], [2 ** 600, 0], 2 ** 600, 0],
      [[max, 0], [-max, 0], max, 1],
      [[0, 0, 0], [1, 2, 2], 3, 0],
    ]
    for (const [p, q, radius, expected] of cases) {
      const exact = compareSquaredDistanceExact(p.map(r), q.map(r), r(radius))
      expect(exact).toBe(expected)
      const filtered = compareSquaredDistanceFilter(p, q, radius)
      if (filtered !== 'indeterminate') expect(filtered).toBe(exact)
    }
    expect(compareSquaredDistanceFilter([0, 0], [min, 0], 0)).toBe('indeterminate')
    expect(compareSquaredDistanceFilter([0, 0], [3, 4], 4)).toBe(1)
    expect(compareSquaredDistanceFilter([0, 0], [3, 4], 6)).toBe(-1)
  })

  it('refuses nonfinite tolerance profiles and leaves boundary residuals indeterminate', () => {
    expect(() => classifyResidual(0, 1, Infinity)).toThrow(/invalid tolerance/)
    expect(() => classifyResidual(0, NaN, 1)).toThrow(/invalid tolerance/)
    expect(classifyResidual(1, 1, 2)).toBe('Indeterminate')
    expect(classifyResidual(2, 1, 2)).toBe('Indeterminate')
    expect(classifyResidual(Infinity, 1, 2)).toBe('Indeterminate')
  })
})
