import type {
  CurveMeasurement,
  FaceDistanceResult,
  PointMeasurement,
  ShellDistanceResult,
} from '../../solidMeasurements'
import { finite, vector, type MainSolidExpectation } from '../../mainSolidProtocol'
import type { NurbsCurveDistance } from '../../nurbsCurve'
import type { NurbsSurfaceDistance } from '../../nurbsSurface'
import { arrayOf, distanceWitnessConsistent } from '../mainSolidExpectation'
import { isNonnegativeFinite, isOrderedInterval } from './predicates'

function validBoundedConvergence(
  converged: boolean,
  reason: string,
  lo: number,
  hi: number | null,
  toleranceMm: number,
  nonConvergedReasons: readonly string[],
): boolean {
  if (converged) {
    return reason === 'tolerance' && hi !== null && hi - lo <= toleranceMm
  }
  return (
    nonConvergedReasons.includes(reason) &&
    (hi === null || hi - lo > toleranceMm) &&
    (reason !== 'empty-domain' || hi === null)
  )
}

export function validShellDistanceResult(
  job: Extract<MainSolidExpectation, { kind: 'shellDistance' }>,
  value: unknown,
): boolean {
  const v = value as ShellDistanceResult
  if (
    v.method !== 'interval-trimmed-face-pairs' ||
    v.scope !== 'boundary-shells-bounded-joins' ||
    v.containment !== 'not-classified' ||
    v.toleranceMm !== job.toleranceMm ||
    v.toleranceUv !== job.toleranceUv ||
    v.maxCells !== job.maxCells ||
    v.maxDomainCells !== job.maxDomainCells ||
    v.pairs !== job.domains[0].length * job.domains[1].length ||
    !Number.isInteger(v.evaluatedPairs) ||
    v.evaluatedPairs < 0 ||
    v.evaluatedPairs > v.pairs ||
    !Number.isInteger(v.cells) ||
    v.cells < 0 ||
    v.cells > job.maxCells ||
    !Number.isInteger(v.domainCells) ||
    v.domainCells < 0 ||
    v.domainCells > job.maxDomainCells ||
    !Array.isArray(v.distanceIntervalMm) ||
    v.distanceIntervalMm.length !== 2 ||
    !isNonnegativeFinite(v.distanceIntervalMm[0]) ||
    typeof v.converged !== 'boolean'
  ) {
    return false
  }
  const [lo, hi] = v.distanceIntervalMm
  if (hi === null) {
    if (
      v.faces !== null ||
      v.parameters !== null ||
      v.points !== null ||
      v.pointEnclosures !== null ||
      v.converged
    ) {
      return false
    }
  } else if (
    !finite(hi) ||
    hi < lo ||
    !arrayOf(v.faces, 2, f => Number.isInteger(f) && Number(f) >= 0) ||
    !v.faces!.every((f, i) => !!job.domains[i][f]) ||
    !arrayOf(v.parameters, 2, uv => arrayOf(uv, 2, finite)) ||
    !v.parameters!.every((uv, i) =>
      uv.every(
        (t, k) =>
          t >= job.domains[i][v.faces![i]][k][0] && t <= job.domains[i][v.faces![i]][k][1],
      ),
    ) ||
    !arrayOf(v.points, 2, vector) ||
    !arrayOf(v.pointEnclosures, 2, p => arrayOf(p, 3, isOrderedInterval)) ||
    v.evaluatedPairs === 0 ||
    v.cells === 0 ||
    v.domainCells === 0 ||
    !distanceWitnessConsistent(v.points!, v.pointEnclosures!, lo, hi)
  ) {
    return false
  }
  return validBoundedConvergence(v.converged, v.reason, lo, hi, job.toleranceMm, [
    'work-limit',
    'domain-work-limit',
    'pair-resolution-limit',
    'empty-domain',
  ])
}

export function validFaceDistanceResult(
  job: Extract<MainSolidExpectation, { kind: 'faceDistance' }>,
  value: unknown,
): boolean {
  const v = value as FaceDistanceResult
  if (
    v.method !== 'interval-trimmed-surface-subdivision' ||
    v.scope !== 'trimmed-surfaces-bounded-joins' ||
    v.toleranceMm !== job.toleranceMm ||
    v.toleranceUv !== job.toleranceUv ||
    v.maxCells !== job.maxCells ||
    v.maxDomainCells !== job.maxDomainCells ||
    !Number.isInteger(v.cells) ||
    v.cells < 1 ||
    v.cells > job.maxCells ||
    !Number.isInteger(v.domainCells) ||
    v.domainCells < 1 ||
    v.domainCells > job.maxDomainCells ||
    !Array.isArray(v.distanceIntervalMm) ||
    v.distanceIntervalMm.length !== 2 ||
    !isNonnegativeFinite(v.distanceIntervalMm[0]) ||
    typeof v.converged !== 'boolean'
  ) {
    return false
  }
  const [lo, hi] = v.distanceIntervalMm
  if (hi === null) {
    if (
      v.parameters !== null ||
      v.points !== null ||
      v.pointEnclosures !== null ||
      v.converged
    ) {
      return false
    }
  } else if (
    !finite(hi) ||
    hi < lo ||
    !arrayOf(v.parameters, 2, uv => arrayOf(uv, 2, finite)) ||
    !v.parameters!.every((uv, i) =>
      uv.every(
        (t, k) =>
          !!job.domains[i][k] && t >= job.domains[i][k][0] && t <= job.domains[i][k][1],
      ),
    ) ||
    !arrayOf(v.points, 2, vector) ||
    !arrayOf(v.pointEnclosures, 2, p => arrayOf(p, 3, isOrderedInterval)) ||
    !distanceWitnessConsistent(v.points!, v.pointEnclosures!, lo, hi)
  ) {
    return false
  }
  return validBoundedConvergence(v.converged, v.reason, lo, hi, job.toleranceMm, [
    'work-limit',
    'domain-work-limit',
    'precision-limit',
    'empty-domain',
  ])
}

export function validSurfaceDistanceResult(
  job: Extract<MainSolidExpectation, { kind: 'surfaceDistance' }>,
  value: unknown,
): boolean {
  const v = value as NurbsSurfaceDistance
  return (
    v.method === 'interval-tensor-de-boor-pair-subdivision' &&
    v.scope === 'untrimmed-surfaces' &&
    v.toleranceMm === job.toleranceMm &&
    v.maxCells === job.maxCells &&
    Number.isInteger(v.cells) &&
    v.cells >= 1 &&
    v.cells <= job.maxCells &&
    isOrderedInterval(v.distanceIntervalMm) &&
    v.distanceIntervalMm[0] >= 0 &&
    arrayOf(v.parameters, 2, uv => arrayOf(uv, 2, finite)) &&
    v.parameters.every((uv, i) =>
      uv.every((t, k) => t >= job.domains[i][k][0] && t <= job.domains[i][k][1]),
    ) &&
    arrayOf(v.points, 2, vector) &&
    arrayOf(v.pointEnclosures, 2, p => arrayOf(p, 3, isOrderedInterval)) &&
    distanceWitnessConsistent(
      v.points,
      v.pointEnclosures,
      v.distanceIntervalMm[0],
      v.distanceIntervalMm[1],
    ) &&
    typeof v.converged === 'boolean' &&
    validBoundedConvergence(
      v.converged,
      v.reason,
      v.distanceIntervalMm[0],
      v.distanceIntervalMm[1],
      job.toleranceMm,
      ['work-limit', 'precision-limit'],
    )
  )
}

export function validCurveDistanceResult(
  job: Extract<MainSolidExpectation, { kind: 'curveDistance' }>,
  value: unknown,
): boolean {
  const v = value as NurbsCurveDistance
  return (
    [2, 3].includes(job.dimension) &&
    v.method === 'interval-de-boor-pair-subdivision' &&
    v.toleranceMm === job.toleranceMm &&
    v.maxCells === job.maxCells &&
    Number.isInteger(v.cells) &&
    v.cells >= 1 &&
    v.cells <= job.maxCells &&
    isOrderedInterval(v.distanceIntervalMm) &&
    v.distanceIntervalMm[0] >= 0 &&
    arrayOf(v.parameters, 2, finite) &&
    v.parameters.every((t, i) => t >= job.domains[i][0] && t <= job.domains[i][1]) &&
    arrayOf(v.points, 2, p => arrayOf(p, job.dimension, finite)) &&
    arrayOf(v.pointEnclosures, 2, p => arrayOf(p, job.dimension, isOrderedInterval)) &&
    typeof v.converged === 'boolean' &&
    validBoundedConvergence(
      v.converged,
      v.reason,
      v.distanceIntervalMm[0],
      v.distanceIntervalMm[1],
      job.toleranceMm,
      ['work-limit', 'precision-limit'],
    )
  )
}

export function validMeasureVerticesResult(value: unknown): boolean {
  const v = value as PointMeasurement
  return vector(v.a) && vector(v.b) && vector(v.deltaMm) && isNonnegativeFinite(v.distanceMm)
}

export function validMeasureEdgeResult(value: unknown): boolean {
  const v = value as CurveMeasurement
  return (
    vector(v.point) &&
    finite(v.parameter) &&
    v.parameter >= 0 &&
    v.parameter <= 1 &&
    isNonnegativeFinite(v.curvaturePerMm) &&
    (v.radiusMm === null || (finite(v.radiusMm) && v.radiusMm > 0))
  )
}
