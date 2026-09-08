import { describe, expect, it } from 'vitest'
import { evaluateNurbsCurve } from '../src/services/nurbsCurve'
import {
  type NurbsSurface,
  validateNurbsSurface,
  evaluateNurbsSurface,
  createNurbsSurfaceEvaluator,
  insertNurbsSurfaceKnot,
  elevateNurbsSurface,
  reverseNurbsSurface,
  trimNurbsSurface,
  isoNurbsCurve,
  nurbsSurfaceBounds,
} from '../src/services/nurbsSurface'

function weightedPatch(): NurbsSurface {
  return {
    degreeU: 2, degreeV: 2,
    knotsU: [0, 0, 0, 1, 1, 1], knotsV: [0, 0, 0, 1, 1, 1],
    controlPoints: [[[0, 0, 0], [0, 10, 0], [0, 20, 0]], [[10, 0, 0], [10, 10, 8], [10, 20, 0]], [[20, 0, 0], [20, 10, 0], [20, 20, 0]]],
    weights: [[1, 1, 1], [1, 2, 1], [1, 1, 1]],
  }
}
function plane(): NurbsSurface {
  return {
    degreeU: 1, degreeV: 1, knotsU: [2, 2, 5, 5], knotsV: [-1, -1, 3, 3],
    controlPoints: [[[0, 0, 3], [0, 20, 3]], [[20, 0, 3], [20, 20, 3]]], weights: [[1, 1], [1, 1]],
  }
}
function close(actual: number[] | null, expected: number[], digits = 9) {
  expect(actual).not.toBeNull()
  expect(actual).toHaveLength(expected.length)
  expected.forEach((value, i) => expect(actual![i]).toBeCloseTo(value, digits))
}

describe('own tensor-product rational NURBS surface arithmetic', () => {
  it('computes a weighted biquadratic point and analytic first/second quotient jets', () => {
    const result = evaluateNurbsSurface(weightedPatch(), 0.5, 0.5)
    close(result.point, [10, 10, 3.2])
    close(result.du, [16, 0, 0]); close(result.dv, [0, 16, 0])
    close(result.duu, [0, 0, -20.48]); close(result.duv, [0, 0, 0]); close(result.dvv, [0, 0, -20.48])
    close(result.normal, [0, 0, 1])
    expect(result.meanCurvature).toBeCloseTo(-0.08, 10)
    expect(result.gaussianCurvature).toBeCloseTo(0.0064, 10)
    expect(result.derivative_status).toBe('available')
  })

  it('handles unequal non-unit domains and one-sided endpoint jets', () => {
    const result = evaluateNurbsSurface(plane(), 3.5, 1)
    close(result.point, [10, 10, 3]); close(result.du, [20 / 3, 0, 0]); close(result.dv, [0, 5, 0])
    close(result.duu, [0, 0, 0]); close(result.dvv, [0, 0, 0])
    expect(result.gaussianCurvature).toBe(0); expect(result.meanCurvature).toBe(0)
    expect(result.domainU).toEqual([2, 5]); expect(result.domainV).toEqual([-1, 3])
    expect(evaluateNurbsSurface(plane(), 2, 3)).toMatchObject({ derivative_side_u: 'right', derivative_side_v: 'left' })
    expect(() => evaluateNurbsSurface(plane(), 1, 1)).toThrow()
  })

  it('preserves a rational circular cylinder and reports signed principal invariants', () => {
    const radius = 7, height = 11
    const surface: NurbsSurface = {
      degreeU: 2, degreeV: 1, knotsU: [0, 0, 0, 1, 1, 1], knotsV: [0, 0, 1, 1],
      controlPoints: [[[radius, 0, 0], [radius, 0, height]], [[radius, radius, 0], [radius, radius, height]], [[0, radius, 0], [0, radius, height]]],
      weights: [[1, 1], [Math.SQRT1_2, Math.SQRT1_2], [1, 1]],
    }
    for (const u of [0, 0.2, 0.5, 0.9, 1]) {
      const result = evaluateNurbsSurface(surface, u, 0.3)
      expect(Math.hypot(result.point[0], result.point[1])).toBeCloseTo(radius, 10)
      expect(result.point[2]).toBeCloseTo(height * 0.3, 10)
      close(result.normal, [result.point[0] / radius, result.point[1] / radius, 0])
      expect(result.gaussianCurvature).toBeCloseTo(0, 10)
      expect(result.meanCurvature).toBeCloseTo(-1 / (2 * radius), 10)
    }
  })

  it('is invariant under common weight scaling and covariant under affine coordinate changes', () => {
    const surface = weightedPatch(), transformed = weightedPatch()
    transformed.weights = transformed.weights.map(row => row.map(weight => weight * 1024))
    transformed.controlPoints = transformed.controlPoints.map(row => row.map(([x, y, z]) => [3 * x + y + 5, 2 * y - 7, z + 4]))
    const a = evaluateNurbsSurface(surface, 0.27, 0.63), b = evaluateNurbsSurface(transformed, 0.27, 0.63)
    close(b.point, [3 * a.point[0] + a.point[1] + 5, 2 * a.point[1] - 7, a.point[2] + 4])
    close(b.du, [3 * a.du![0] + a.du![1], 2 * a.du![1], a.du![2]])
  })

  it('keeps derivative and curvature precision after a large exact translation', () => {
    const surface = weightedPatch(), translated = weightedPatch(), offset = [1e8, -2e8, 3e8]
    translated.controlPoints = translated.controlPoints.map(row => row.map(point => point.map((coordinate, axis) => coordinate + offset[axis])))
    for (const [u, v] of [[0.5, 0.5], [0.2131, 0.4872], [0.97, 0.03]]) {
      const a = evaluateNurbsSurface(surface, u, v), b = evaluateNurbsSurface(translated, u, v)
      expect(b.point).toEqual(a.point.map((coordinate, axis) => coordinate + offset[axis]))
      for (const jet of ['du', 'dv', 'duu', 'duv', 'dvv', 'normal'] as const) close(b[jet], a[jet]!, 12)
      expect(b.meanCurvature).toBeCloseTo(a.meanCurvature!, 14)
      expect(b.gaussianCurvature).toBeCloseTo(a.gaussianCurvature!, 14)
    }
  })

  it('distinguishes knot continuity loss from singular surface charts', () => {
    const crease: NurbsSurface = {
      degreeU: 1, degreeV: 1, knotsU: [0, 0, 0.5, 1, 1], knotsV: [0, 0, 1, 1],
      controlPoints: [[[0, 0, 0], [0, 1, 0]], [[1, 0, 0], [1, 1, 0]], [[2, 0, 1], [2, 1, 1]]], weights: [[1, 1], [1, 1], [1, 1]],
    }
    const atKnot = evaluateNurbsSurface(crease, 0.5, 0.4)
    expect(atKnot.derivative_status).toBe('insufficient_continuity')
    expect(atKnot.du).toBeNull(); expect(atKnot.duu).toBeNull(); expect(atKnot.duv).toBeNull()
    close(atKnot.dv, [0, 1, 0]); expect(atKnot.normal).toBeNull(); expect(atKnot.gaussianCurvature).toBeNull()
    const collapsed = plane()
    collapsed.controlPoints = [[[0, 0, 0], [1, 0, 0]], [[2, 0, 0], [3, 0, 0]]]
    expect(evaluateNurbsSurface(collapsed, 3, 0)).toMatchObject({ derivative_status: 'singular', normal: null, gaussianCurvature: null, meanCurvature: null })
  })

  it('retains first jets and normals at C1 knots while refusing two-sided curvature', () => {
    const surface: NurbsSurface = {
      degreeU: 2, degreeV: 1, knotsU: [0, 0, 0, 0.5, 1, 1, 1], knotsV: [0, 0, 1, 1],
      controlPoints: [[[0, 0, 0], [0, 1, 0]], [[1, 0, 0], [1, 1, 0]], [[2, 0, 1], [2, 1, 1]], [[3, 0, 0], [3, 1, 0]]],
      weights: [[1, 1], [1, 1], [1, 1], [1, 1]],
    }
    const result = evaluateNurbsSurface(surface, 0.5, 0.3)
    expect(result.derivative_status).toBe('insufficient_continuity')
    expect(result.du).not.toBeNull(); expect(result.duv).not.toBeNull(); expect(result.normal).not.toBeNull()
    expect(result.duu).toBeNull(); expect(result.meanCurvature).toBeNull(); expect(result.gaussianCurvature).toBeNull()
  })

  it('keeps conservative positive-weight control bounds and rejects malformed data', () => {
    expect(nurbsSurfaceBounds(weightedPatch())).toEqual({ min: [0, 0, 0], max: [20, 20, 8] })
    const ragged = weightedPatch(); ragged.controlPoints[1].pop()
    expect(() => validateNurbsSurface(ragged)).toThrow('rectangular')
    const badWeights = weightedPatch(); badWeights.weights[1][1] = 0
    expect(() => validateNurbsSurface(badWeights)).toThrow()
    const badKnots = weightedPatch(); badKnots.knotsU[3] = -1
    expect(() => validateNurbsSurface(badKnots)).toThrow()
    const wrongDimension = weightedPatch(); wrongDimension.controlPoints[0][0] = [0, 0]
    expect(() => validateNurbsSurface(wrongDimension)).toThrow('three')
    const poorConditioning = plane(); poorConditioning.weights = [[1e-12, 1], [1, 1e12]]
    expect(() => validateNurbsSurface(poorConditioning)).toThrow('conditioning')
  })

  it('captures an immutable validated evaluator for tessellation', () => {
    const surface = weightedPatch(), evaluate = createNurbsSurfaceEvaluator(surface)
    const before = evaluate(0.5, 0.5)
    surface.controlPoints[1][1][2] = 999
    surface.weights[1][1] = 3
    expect(evaluate(0.5, 0.5)).toEqual(before)
    evaluate.dispose()
    evaluate.dispose()
    expect(() => evaluate(0.5, 0.5)).toThrow('disposed')
  })
})

describe('representation-preserving surface edits', () => {
  const samples = [[0.12, 0.17], [0.47, 0.81], [0.91, 0.34]]
  it('inserts knots along either axis and elevates degree without modifying the input', () => {
    const surface = weightedPatch(), initial = JSON.stringify(surface)
    const refined = insertNurbsSurfaceKnot(insertNurbsSurfaceKnot(surface, 'u', 0.3), 'v', 0.6, 2)
    const elevated = elevateNurbsSurface(elevateNurbsSurface(surface, 'u', 3), 'v', 4)
    expect(elevated.degreeU).toBe(3); expect(elevated.degreeV).toBe(4)
    for (const [u, v] of samples) {
      const source = evaluateNurbsSurface(surface, u, v)
      for (const result of [refined, elevated]) {
        const actual = evaluateNurbsSurface(result, u, v)
        close(actual.point, source.point); close(actual.du, source.du!); close(actual.duv, source.duv!, 8)
      }
    }
    expect(JSON.stringify(surface)).toBe(initial)
  })

  it('restricts the parameter rectangle while preserving parameter correspondence', () => {
    const surface = weightedPatch(), trimmed = trimNurbsSurface(surface, [0.1, 0.95, 0.15, 0.9])
    for (const [u, v] of samples) close(evaluateNurbsSurface(trimmed, u, v).point, evaluateNurbsSurface(surface, u, v).point)
    expect(evaluateNurbsSurface(trimmed, 0.1, 0.9)).toMatchObject({ domainU: [0.1, 0.95], domainV: [0.15, 0.9] })
    expect(() => trimNurbsSurface(surface, 0.8, 0.2, 0, 1)).toThrow('increasing')
    expect(() => trimNurbsSurface(surface, -0.1, 1, 0, 1)).toThrow()
  })

  it('reverses one parameter axis and flips orientation once', () => {
    const surface = weightedPatch(), reversed = reverseNurbsSurface(surface, 'u')
    const before = evaluateNurbsSurface(surface, 0.8, 0.3), after = evaluateNurbsSurface(reversed, 0.2, 0.3)
    close(after.point, before.point); close(after.du, before.du!.map(value => -value)); close(after.normal, before.normal!.map(value => -value))
    expect(after.meanCurvature).toBeCloseTo(-before.meanCurvature!, 10)
    expect(after.gaussianCurvature).toBeCloseTo(before.gaussianCurvature!, 10)
  })

  it('extracts exact rational iso-curves in either parameter direction', () => {
    const surface = weightedPatch()
    for (const direction of ['u', 'v'] as const) {
      const curve = isoNurbsCurve(surface, direction, 0.27)
      for (const t of [0, 0.12, 0.4, 0.87, 1]) {
        const point = direction === 'u' ? evaluateNurbsSurface(surface, 0.27, t).point : evaluateNurbsSurface(surface, t, 0.27).point
        close(evaluateNurbsCurve(curve, t).point, point)
      }
    }
  })

  it('preserves explicit periodic seams and their first derivatives', () => {
    const ring = [[1, 0], [0, 1], [-1, 0], [0, -1], [1, 0], [0, 1]]
    const surface: NurbsSurface = {
      degreeU: 2, degreeV: 1, knotsU: [0, 1, 2, 3, 4, 5, 6, 7, 8], knotsV: [0, 0, 1, 1],
      controlPoints: ring.map(([x, y]) => [[x, y, 0], [x, y, 3]]), weights: ring.map(() => [1, 1]), periodicU: true,
    }
    const first = evaluateNurbsSurface(surface, 2, 0.4), last = evaluateNurbsSurface(surface, 6, 0.4)
    close(first.point, last.point); close(first.du, last.du!); close(first.normal, last.normal!)
    expect(first).toMatchObject({ derivative_status: 'insufficient_continuity', duu: null, gaussianCurvature: null, meanCurvature: null })
    expect(last).toMatchObject({ derivative_status: 'insufficient_continuity', duu: null, gaussianCurvature: null, meanCurvature: null })
    expect(isoNurbsCurve(surface, 'v', 0.4).periodic).toBe(true)
    const reversed = reverseNurbsSurface(surface, 'u')
    close(evaluateNurbsSurface(reversed, 2.3, 0.4).point, evaluateNurbsSurface(surface, 5.7, 0.4).point)
  })
})
