import { describe, expect, it } from 'vitest'
import {
  decomposeNurbsCurve, elevateNurbsCurve, evaluateNurbsCurve, insertNurbsKnot,
  nurbsBasisDerivatives, nurbsCurveBounds, reverseNurbsCurve, splitNurbsCurve,
  trimNurbsCurve, validateNurbsCurve, type NurbsCurve,
} from '../src/services/nurbsCurve'

const circle: NurbsCurve = {
  degree: 2, knots: [0, 0, 0, 1, 1, 1],
  controlPoints: [[1, 0, 0], [1, 1, 0], [0, 1, 0]], weights: [1, Math.SQRT1_2, 1],
}

function near(actual: number[], expected: number[], digits = 11) {
  expect(actual).toHaveLength(expected.length)
  actual.forEach((value, i) => expect(value).toBeCloseTo(expected[i], digits))
}

function equalGeometry(a: NurbsCurve, b: NurbsCurve, start = b.knots[b.degree], end = b.knots[b.controlPoints.length]) {
  for (let i = 0; i <= 50; i++) {
    const u = start + (end - start) * i / 50
    near(evaluateNurbsCurve(a, u).point, evaluateNurbsCurve(b, u).point)
  }
}

describe('own NURBS curve mathematics', () => {
  it('evaluates an exact rational quarter circle and its analytic first/second derivatives', () => {
    const result = evaluateNurbsCurve(circle, 0.5)
    near(result.point, [Math.SQRT1_2, Math.SQRT1_2, 0])
    // The rational quadratic circle parametrization at t=.5 has these closed forms.
    const velocity = 4 / (2 + Math.SQRT2)
    near(result.d1!, [-velocity, velocity, 0])
    near(result.d2!, [-Math.SQRT2 * velocity ** 2, -Math.SQRT2 * velocity ** 2, 0])
    expect(result.derivative_status).toBe('available')
    near(evaluateNurbsCurve(circle, 0).point, [1, 0, 0])
    near(evaluateNurbsCurve(circle, 1).point, [0, 1, 0])
  })

  it('supports nonclamped natural domains and partitions unity including endpoints', () => {
    const curve: NurbsCurve = { degree: 2, knots: [-2, -1, 0, 1, 2, 3, 4, 5],
      controlPoints: [[0, 0], [2, 4], [5, 3], [6, 1], [8, -1]], weights: [1, 1, 1, 1, 1] }
    near(evaluateNurbsCurve(curve, 0).point, [1, 2])
    near(evaluateNurbsCurve(curve, 3).point, [7, 0])
    for (const u of [0, 0.4, 1, 1.5, 2.4, 3]) {
      const basis = nurbsBasisDerivatives(2, curve.knots, 5, u)
      expect(basis.basis.reduce((sum, value) => sum + value, 0)).toBeCloseTo(1, 13)
      expect(basis.d1.reduce((sum, value) => sum + value, 0)).toBeCloseTo(0, 13)
      expect(basis.d2.reduce((sum, value) => sum + value, 0)).toBeCloseTo(0, 13)
    }
    equalGeometry(curve, trimNurbsCurve(curve, 0, 3))
    equalGeometry(curve, insertNurbsKnot(curve, 3, 2))
    equalGeometry(curve, insertNurbsKnot(curve, 0, 2))
  })

  it('inserts knots in homogeneous space without mutating its input', () => {
    const original = JSON.stringify(circle)
    const refined = insertNurbsKnot(circle, 0.25, 2)
    expect(refined.controlPoints).toHaveLength(5)
    expect(refined.knots.filter(knot => knot === 0.25)).toHaveLength(2)
    equalGeometry(circle, refined)
    expect(JSON.stringify(circle)).toBe(original)
    expect(() => insertNurbsKnot(refined, 0.25)).toThrow(/multiplicity/)
  })

  it('raises degree, decomposes rational spans, and returns independent editable control data', () => {
    const curve = insertNurbsKnot(circle, 0.25)
    const elevated = elevateNurbsCurve(curve, 5)
    expect(elevated.degree).toBe(5)
    expect(elevated.controlPoints).toHaveLength(11)
    equalGeometry(curve, elevated)
    const pieces = decomposeNurbsCurve(curve)
    expect(pieces.map(piece => piece.domain)).toEqual([[0, 0.25], [0.25, 1]])
    for (const piece of pieces) {
      expect(piece.curve.controlPoints).toHaveLength(3)
      equalGeometry(curve, piece.curve)
    }
    elevated.controlPoints[0][0] = 99
    expect(circle.controlPoints[0][0]).toBe(1)
  })

  it('trims and splits at original parameters rather than reparametrizing them', () => {
    const refined = insertNurbsKnot(circle, 0.5)
    const [left, right] = splitNurbsCurve(refined, 0.3)
    expect(left.knots.slice(-3)).toEqual([0.3, 0.3, 0.3])
    expect(right.knots.slice(0, 3)).toEqual([0.3, 0.3, 0.3])
    equalGeometry(circle, left)
    equalGeometry(circle, right)
    equalGeometry(circle, trimNurbsCurve(refined, 0.15, 0.9))
    expect(() => splitNurbsCurve(circle, 0)).toThrow(/strictly/)
    expect(() => trimNurbsCurve(circle, 0.8, 0.2)).toThrow(/subdomain/)
  })

  it('reverses geometry, derivatives and a nonzero parameter domain', () => {
    const offset = { ...circle, knots: circle.knots.map(knot => knot + 7) }
    const reverse = reverseNurbsCurve(offset)
    for (const u of [7, 7.1, 7.45, 8]) {
      const a = evaluateNurbsCurve(offset, 15 - u)
      const b = evaluateNurbsCurve(reverse, u)
      near(a.point, b.point)
      near(a.d1!.map(value => -value), b.d1!)
      near(a.d2!, b.d2!)
    }
  })

  it('refuses two-sided derivatives at low-continuity knots', () => {
    const curve: NurbsCurve = { degree: 1, knots: [0, 0, 1, 2, 2],
      controlPoints: [[0, 0], [1, 0], [1, 2]], weights: [1, 1, 1] }
    const corner = evaluateNurbsCurve(curve, 1)
    near(corner.point, [1, 0])
    expect(corner).toMatchObject({ d1: null, d2: null, derivative_status: 'insufficient_continuity', continuity: 0 })
    near(evaluateNurbsCurve(curve, 0.5).d2!, [0, 0])
    expect(evaluateNurbsCurve(curve, 0).derivative_side).toBe('right')
    expect(evaluateNurbsCurve(curve, 2).derivative_side).toBe('left')
  })

  it('validates explicit periodic wrapping and evaluates a continuous seam', () => {
    const curve: NurbsCurve = { degree: 2, knots: [-2, -1, 0, 1, 2, 3, 4, 5, 6],
      controlPoints: [[1, 0], [0, 1], [-1, 0], [0, -1], [1, 0], [0, 1]],
      weights: [1, 1, 1, 1, 1, 1], periodic: true }
    validateNurbsCurve(curve)
    const first = evaluateNurbsCurve(curve, 0), last = evaluateNurbsCurve(curve, 4)
    near(first.point, last.point)
    near(first.d1!, last.d1!)
    expect(first).toMatchObject({ continuity: 1, derivative_status: 'insufficient_continuity', d2: null })
    expect(last).toMatchObject({ continuity: 1, derivative_status: 'insufficient_continuity', d2: null })
    equalGeometry(curve, insertNurbsKnot(curve, 1.5))
    expect(insertNurbsKnot(curve, 1.5).periodic).toBe(false)
    expect(reverseNurbsCurve(curve).periodic).toBe(true)
    expect(() => validateNurbsCurve({ ...curve, controlPoints: [...curve.controlPoints.slice(0, -1), [9, 9]] })).toThrow(/repeat/)
    const tiny = { ...curve, knots: curve.knots.map(knot => knot * 1e-20) }
    validateNurbsCurve(tiny)
    tiny.knots[1] = -0.5e-20
    expect(() => validateNurbsCurve(tiny)).toThrow(/exterior knots/)
  })

  it('keeps derivative jets invariant under large exactly representable translations', () => {
    const shifted = { ...circle, controlPoints: circle.controlPoints.map(point => point.map(value => value + 1e8)) }
    for (const u of [0, 0.1, 0.5, 0.8, 1]) {
      const original = evaluateNurbsCurve(circle, u), translated = evaluateNurbsCurve(shifted, u)
      near(translated.d1!, original.d1!, 13)
      near(translated.d2!, original.d2!, 13)
      translated.point.forEach((value, axis) => expect(value).toBe(original.point[axis] + 1e8))
    }
  })

  it('elevates to degree 25 and preserves rational geometry near endpoints', () => {
    const high = elevateNurbsCurve(circle, 25)
    expect(high.controlPoints).toHaveLength(26)
    for (const u of [0, 1e-10, 1e-5, 0.3, 0.6, 1 - 1e-10, 1]) {
      const a = evaluateNurbsCurve(circle, u), b = evaluateNurbsCurve(high, u)
      near(a.point, b.point, 12)
      near(a.d1!, b.d1!, 11)
      near(a.d2!, b.d2!, 9)
    }
  })

  it('bounds positive rational curves by their control hull and rejects invalid or excessive inputs', () => {
    expect(nurbsCurveBounds(circle)).toEqual({ min: [0, 0, 0], max: [1, 1, 0] })
    for (const bad of [0, -1, NaN, Infinity]) {
      expect(() => validateNurbsCurve({ ...circle, weights: [1, bad, 1] })).toThrow()
    }
    expect(() => validateNurbsCurve({ ...circle, knots: [0, 0, 0, -1, 1, 1] })).toThrow(/nondecreasing/)
    expect(() => evaluateNurbsCurve(circle, 1.1)).toThrow(/domain/)
    expect(() => elevateNurbsCurve(circle, 1)).toThrow(/Elevation/)
    const many: NurbsCurve = { degree: 1, knots: [0, ...Array.from({ length: 30 }, (_, i) => i), 29],
      controlPoints: Array.from({ length: 30 }, (_, i) => [i, i % 2]), weights: new Array(30).fill(1) }
    expect(() => elevateNurbsCurve(many, 25)).toThrow(/256/)
  })
})
