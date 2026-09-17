import {describe, expect, test} from 'vitest'
import {
  approximateNurbsCurveCertified,
  certifyNurbsCurveFoundation,
  certifyNurbsSurfaceFoundation,
  interpolateNurbsPolylineCertified,
  projectPointToNurbsCurveCertified,
  reparameterizeNurbsCurveExact,
} from '../src/services/nurbsFoundation'
import {evaluateNurbsCurve, type NurbsCurve} from '../src/services/nurbsCurve'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'

const arc: NurbsCurve = {
  degree: 2,
  knots: [0, 0, 0, 1, 1, 1],
  controlPoints: [[1, 0, 0], [1, 1, 0], [0, 1, 0]],
  weights: [1, Math.SQRT1_2, 1],
}

describe('NURBS foundation product boundary', () => {
  test('exposes deterministic positive outward certificates through WASM', () => {
    const certificate = certifyNurbsCurveFoundation(arc)
    expect(certificate.version).toBe('nurbs-foundation/1')
    expect(certificate.spans[0].denominatorLower).toBeGreaterThan(0)
    expect(certificate.spans[0].regularity.classification).toBe('certified_regular')
  })

  test('certifies planar surface normal regularity', () => {
    const certificate = certifyNurbsSurfaceFoundation({
      degreeU: 1, degreeV: 1,
      knotsU: [0, 0, 1, 1], knotsV: [0, 0, 1, 1],
      controlPoints: [[[0, 0, 0], [0, 1, 0]], [[1, 0, 0], [1, 1, 0]]],
      weights: [[1, 2], [3, 4]],
    })
    expect(certificate.cells[0].normalRegularity.classification).toBe('certified_planar_regular')
  })

  test('reports projection uniqueness and bounded approximation', () => {
    const line: NurbsCurve = {
      degree: 1, knots: [0, 0, 1, 1],
      controlPoints: [[0, 0], [2, 0]], weights: [1, 1],
    }
    expect(projectPointToNurbsCurveCertified(line, [0.5, 1]).status).toBe('unique')
    expect(approximateNurbsCurveCertified(arc).certificate.hausdorffErrorUpper).toBeGreaterThan(0)
    expect(interpolateNurbsPolylineCertified([[0, 0], [1, 2]]).certificate.dataSiteErrorUpper).toBe(0)
  })

  test('preserves periodic geometry under exact monotone remapping', () => {
    const periodic: NurbsCurve = {
      degree: 2,
      knots: [0, 1, 2, 3, 4, 5, 6, 7, 8],
      controlPoints: [[1, 0], [0, 1], [-1, 0], [0, -1], [1, 0], [0, 1]],
      weights: [1, 1, 1, 1, 1, 1],
      periodic: true,
    }
    const mapped = reparameterizeNurbsCurveExact(periodic, [-3, 5]).curve
    expect(mapped.periodic).toBe(true)
    expect(evaluateNurbsCurve(mapped, 1).point).toEqual(evaluateNurbsCurve(periodic, 4).point)
  })

  test('ModelGraph product reports carry the foundation certificate', () => {
    const built = buildOwnNurbs({
      language: 'modelgraph/nurbs-1', units: 'mm', parameters: [],
      nodes: [{id: 'arc', op: 'curve', degree: 2, knots: arc.knots,
        control_points: arc.controlPoints, weights: arc.weights, periodic: false}],
      root: 'arc',
    }, {action: 'build'})
    expect(built.report.error_bound_certified).toBe(true)
    expect(built.report.foundation_certificate.version).toBe('nurbs-foundation/1')
  })
})
