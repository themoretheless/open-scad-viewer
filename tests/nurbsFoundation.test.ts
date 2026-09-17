import {describe, expect, test} from 'vitest'
import {
  approximateNurbsCurveCertified,
  certifyNurbsCurveFoundation,
  certifyNurbsSurfaceFoundation,
  interpolateNurbsPolylineCertified,
  projectPointToNurbsCurveCertified,
  projectPointToNurbsSurfaceCertified,
  removeNurbsCurveKnotCertified,
  certifyNurbsReparameterization,
  editPeriodicNurbsCurveCertified,
  evaluateReparameterizedNurbsCurve,
  fitNurbsCurveCertified,
  fitNurbsSurfaceCertified,
  splitPeriodicNurbsCurveCertified,
  materializeReparameterizedNurbsCurve,
  fitNurbsCurveCloudCertified,
  fitNurbsSurfaceCloudCertified,
  intersectNurbsCurveCurveCertified,
  intersectNurbsCurveSurfaceCertified,
  intersectNurbsSurfaceSurfaceCertified,
  verifyNurbsSurfaceSurfaceCoverage,
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

  test('isolates rational stationary candidates and covers surface boundaries', () => {
    const projected = projectPointToNurbsCurveCertified(arc, [0.8, 0.2, 0])
    expect(projected.version).toBe('nurbs-foundation/3')
    expect(projected.coverage?.endpointsIncluded).toBe(true)
    const surface = {
      degreeU: 1, degreeV: 1, knotsU: [0, 0, 1, 1], knotsV: [0, 0, 1, 1],
      controlPoints: [[[0, 0, 0], [0, 1, 0]], [[1, 0, 0], [1, 1, 1]]],
      weights: [[1, 1], [1, 1]],
    }
    const result = projectPointToNurbsSurfaceCertified(surface, [2, 0, 0.5])
    expect(result.version).toBe('nurbs-foundation/4')
    expect(result.coverage.complete).toBe(true)
    expect(result.boundaryReductions).toHaveLength(4)
    expect(certifyNurbsSurfaceFoundation(surface).cells[0].normalRegularity.classification).toBe('certified_regular')
  })

  test('rolls certified removal back when the global budget is exceeded', () => {
    const inserted = {
      degree: 2, knots: [0, 0, 0, 0.5, 1, 1, 1],
      controlPoints: [[1, 0, 0], [1, 0.4142135623730951, 0], [0.4142135623730951, 1, 0], [0, 1, 0]],
      weights: [1, 0.8535533905932737, 0.8535533905932737, 1],
      periodic: false,
    }
    const result = removeNurbsCurveKnotCertified(inserted, 0.5, 0)
    expect(result.certificate.rolledBack).toBe(true)
    expect(result.curve).toEqual(inserted)
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

  test('preserves wrapped periodic storage and certifies seam crossings', () => {
    const periodic: NurbsCurve = {
      degree: 2, knots: [0,1,2,3,4,5,6,7,8],
      controlPoints: [[1,0],[0,1],[-1,0],[0,-1],[1,0],[0,1]],
      weights: [1,1,1,1,1,1], periodic: true,
    }
    const edited = editPeriodicNurbsCurveCertified(periodic, 'insert', {u:3.5,maxError:10})
    expect(edited.curve.periodic).toBe(true)
    expect((edited.certificate as any).seam.c0.certified).toBe(true)
    expect(splitPeriodicNurbsCurveCertified(periodic, 4).curves).toHaveLength(2)
  })

  test('certifies nonlinear rational maps and keeps fits approximate', () => {
    const mapping = {pieces:[{domain:[0,1] as [number,number],range:[0,1] as [number,number],
      controlValues:[0,0.2,1],weights:[1,0.75,1]}]}
    expect(certifyNurbsReparameterization(mapping).classification).toBe('certified_strictly_monotone')
    const evaluation = evaluateReparameterizedNurbsCurve(arc,mapping,0.5)
    expect(evaluation.sourceParameter).toBeGreaterThan(0)
    const fit = fitNurbsCurveCertified([[0,0],[1,1],[2,0],[3,1]],3)
    expect((fit.certificate as any).classification).toBe('approximate_fit')
    expect((fit.certificate as any).fittedToExactPromotion).toBe(false)
    const surfaceFit = fitNurbsSurfaceCertified([[[0,0,0],[0,1,0]],[[1,0,0],[1,1,1]]])
    expect((surfaceFit.certificate as any).classification).toBe('approximate_fit')
  })

  test('keeps transformed scales and mutated certificates outside authority', () => {
    const transformed: NurbsCurve = {...arc,
      controlPoints: arc.controlPoints.map(([x,y,z]) => [1e5 + 1e3*x, -2e5 + 1e3*y, 7 + z])}
    const projected = projectPointToNurbsCurveCertified(transformed,[100800,-199800,7])
    expect(projected.globalDistanceUpper).toBeGreaterThanOrEqual(0)
    ;(projected.coverage as any).endpointsIncluded = false
    expect(projectPointToNurbsCurveCertified(transformed,[100800,-199800,7]).coverage?.endpointsIncluded).toBe(true)
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
    expect(built.report.foundation_successor.version).toBe('nurbs-ss/1')
  })

  test('materializes exact nonlinear maps and cloud Hausdorff fits', () => {
    const mapping = {pieces:[{domain:[0,1] as [number,number],range:[0,1] as [number,number],
      controlValues:[0,0.5,1],weights:[1,1,1]}]}
    const composed = materializeReparameterizedNurbsCurve(arc, mapping)
    expect(composed.certificate.version).toBe('nurbs-foundation/5')
    expect((composed.certificate as any).exact).toBe(true)
    const left = evaluateNurbsCurve(composed.curve, 0.5).point
    const right = evaluateNurbsCurve(arc, 0.5).point
    expect(left.every((value, index) => Math.abs(value - right[index]) < 1e-12)).toBe(true)
    const cloud = fitNurbsCurveCloudCertified([[0,0],[0.5,0.4],[1,0],[1.5,0.3],[2,0]], 4)
    expect((cloud.certificate as any).classification).toBe('approximate_cloud_fit')
    expect((cloud.certificate as any).hausdorffErrorUpper)
      .toBeGreaterThanOrEqual((cloud.certificate as any).dataSiteErrorUpper)
    const surfaceCloud = fitNurbsSurfaceCloudCertified(
      [[0,0,0],[1,0,0],[0,1,0],[1,1,0.2],[0.5,0.5,0.1]], 3, 3)
    expect((surfaceCloud.certificate as any).fittedToExactPromotion).toBe(false)
    const planar = projectPointToNurbsSurfaceCertified({
      degreeU:1, degreeV:1, knotsU:[0,0,1,1], knotsV:[0,0,1,1],
      controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]], weights:[[1,1],[1,1]],
    }, [0.25, 0.4, 1])
    expect(planar.status).toBe('unique')
  })

  test('certifies general curve/curve and curve/surface intersections', () => {
    const a = {degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[1,1,0]],weights:[1,1]}
    const b = {degree:1,knots:[0,0,1,1],controlPoints:[[0,1,0],[1,0,0]],weights:[1,1]}
    const cc = intersectNurbsCurveCurveCertified(a, b)
    expect(cc.version).toBe('nurbs-foundation/5')
    expect(cc.coverage.complete).toBe(true)
    expect(cc.components.some(c => c.kind === 'point')).toBe(true)
    const plane = {
      degreeU:1, degreeV:1, knotsU:[0,0,1,1], knotsV:[0,0,1,1],
      controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]], weights:[[1,1],[1,1]],
    }
    const piercing = {degree:1,knots:[0,0,1,1],controlPoints:[[0.25,0.4,-1],[0.25,0.4,1]],weights:[1,1]}
    const cs = intersectNurbsCurveSurfaceCertified(piercing, plane)
    expect(cs.kind).toBe('curve_surface')
    expect(cs.coverage.complete).toBe(true)
    expect(cs.components.some(c => c.kind === 'point')).toBe(true)
    const onPlane = {degree:1,knots:[0,0,1,1],controlPoints:[[0.1,0.2,0],[0.8,0.7,0]],weights:[1,1]}
    const overlap = intersectNurbsCurveSurfaceCertified(onPlane, plane)
    expect(overlap.components.some(c => c.kind === 'overlap' && (c as any).coedgeTrim)).toBe(true)
  })

  test('certifies general surface/surface intersection without graph-patch iso', () => {
    const xy = {
      degreeU:1, degreeV:1, knotsU:[0,0,1,1], knotsV:[0,0,1,1],
      controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]], weights:[[1,1],[1,1]],
    }
    const xz = {
      degreeU:1, degreeV:1, knotsU:[0,0,1,1], knotsV:[0,0,1,1],
      controlPoints:[[[0,0,0],[0,0,1]],[[1,0,0],[1,0,1]]], weights:[[1,1],[1,1]],
    }
    const ss = intersectNurbsSurfaceSurfaceCertified(xy, xz)
    expect(ss.version).toBe('nurbs-ss/1')
    expect(ss.kind).toBe('surface_surface')
    expect(ss.coverage.complete).toBe(true)
    expect(ss.booleanMutationAuthority).toBe(false)
    expect(ss.topologyAuthority.granted).toBe(false)
    expect(ss.components.some(c => c.kind === 'curve')).toBe(true)
    expect(ss.branchGraph.components.length).toBeGreaterThan(0)
    const audit = verifyNurbsSurfaceSurfaceCoverage(ss)
    expect(audit.complete).toBe(true)
    const self = intersectNurbsSurfaceSurfaceCertified(xy, xy)
    expect(self.components.some(c => c.kind === 'overlap')).toBe(true)
  })
})
