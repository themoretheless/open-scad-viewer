import { validCurveOffsetDiagnostics } from '../../curveOffsetDiagnostics'
import { validateProfileIntersectionReport } from '../../../kernel/geometry/profileIntersections'
import {
  finite,
  vector,
  type MainSolidExpectation,
  type MainSolidResults,
} from '../../mainSolidProtocol'
import type { NurbsCurve } from '../../../kernel/nurbsCurve'
import { arrayOf } from '../mainSolidExpectation'
import { isNonnegativeFinite, validDirectDocument } from './predicates'

export function validFaceSketchResult(value: unknown): boolean {
  const v = value as MainSolidResults['faceSketch']
  return (
    !!v.plane &&
    vector(v.plane.origin) &&
    vector(v.plane.u) &&
    vector(v.plane.v) &&
    vector(v.normal) &&
    Array.isArray(v.outline) &&
    v.outline.length > 0 &&
    v.outline.every(
      loop => Array.isArray(loop) && loop.length >= 3 && loop.every(p => arrayOf(p, 2, finite)),
    )
  )
}

export function validBodyEdgesResult(
  job: Extract<MainSolidExpectation, { kind: 'bodyEdges' }>,
  value: unknown,
): boolean {
  return (
    Array.isArray(value) &&
    (job.exact ? value.length === job.edges : value.length <= job.edges) &&
    value.every(
      (e, i) =>
        e &&
        e.index === i &&
        typeof e.id === 'string' &&
        e.id.length > 0 &&
        [e.a, e.b].every(n => Number.isSafeInteger(n) && n >= 0 && n < job.vertices) &&
        Array.isArray(e.points) &&
        e.points.length >= 2 &&
        e.points.every(vector),
    )
  )
}

export function validCurveChainInspectionResult(
  job: Extract<MainSolidExpectation, { kind: 'curveChainInspection' }>,
  value: unknown,
): boolean {
  return validCurveOffsetDiagnostics(value, job.segments) && value.checks <= job.maxPairs
}

export function validCurveDisplayResult(
  job: Extract<MainSolidExpectation, { kind: 'curveDisplay' }>,
  value: unknown,
): boolean {
  return (
    [2, 3].includes(job.dimension) &&
    Array.isArray(value) &&
    value.length === 49 &&
    value.every(p => Array.isArray(p) && p.length === job.dimension && p.every(finite))
  )
}

export function validProfileDisplayResult(
  job: Extract<MainSolidExpectation, { kind: 'profileDisplay' }>,
  value: unknown,
): boolean {
  return (
    Array.isArray(value) &&
    value.length === job.counts.length &&
    value.every(
      (loop, i) =>
        Array.isArray(loop) &&
        loop.length === job.counts[i] &&
        loop.every(p => Array.isArray(p) && p.length === 2 && p.every(finite)),
    )
  )
}

export function validTrimmedCurveOffsetResult(
  job: Extract<MainSolidExpectation, { kind: 'trimmedCurveOffset' }>,
  value: unknown,
): boolean {
  const v = value as MainSolidResults['trimmedCurveOffset']
  const r = v.report
  if (
    !validDirectDocument(v.document) ||
    !r ||
    r.accepted !== true ||
    r.method !== 'represented-bevel-offset-fill/1' ||
    r.fillRule !== job.fillRule ||
    r.regionTrimmed !== true ||
    r.topologyScope !== 'represented-reconstructed-chord-graph' ||
    r.regionTopologyCertified !== false ||
    r.originalOffsetTopologyCertified !== false ||
    !isNonnegativeFinite(r.sourceWireErrorUpperMm) ||
    r.sourceWireErrorUpperMm > job.toleranceMm ||
    !isNonnegativeFinite(r.intersectionConstructionErrorUpperMm) ||
    r.intersectionConstructionErrorUpperMm > job.intersectionToleranceMm ||
    !Array.isArray(v.loopIds) ||
    v.loopIds.length > 128
  ) {
    return false
  }
  const used = new Set<string>()
  const equalCoords = (a: number[], b: number[]) =>
    a.length === b.length && a.every((x, i) => x === b[i])
  return v.loopIds.every(ids => {
    if (!Array.isArray(ids) || !ids.length || ids.length > 128) return false
    const curves: NurbsCurve[] = []
    for (const id of ids) {
      if (typeof id !== 'string' || !id.startsWith(job.createdId + ':') || used.has(id)) {
        return false
      }
      const item = v.document.curves?.find(c => c.id === id)
      if (!item || item.curve.degree !== 1 || item.curve.periodic === true) return false
      used.add(id)
      curves.push(item.curve)
    }
    for (let i = 1; i < curves.length; i++) {
      if (!equalCoords(curves[i - 1]!.controlPoints.at(-1)!, curves[i]!.controlPoints[0]!)) {
        return false
      }
    }
    return equalCoords(curves[0]!.controlPoints[0]!, curves.at(-1)!.controlPoints.at(-1)!)
  })
}

export function validCurveOffsetResult(value: unknown): boolean {
  const v = value as MainSolidResults['curveOffset']
  const r = v.report
  return (
    validDirectDocument(v.document) &&
    !!r &&
    r.accepted === true &&
    (r.method === 'outward-rational-jets-chord-bound/1'
      ? r.wholeCurve === true
      : r.method === 'outward-source-offset-bevel-wire/1' &&
        r.wholeCurve === false &&
        r.wholeWire === true &&
        r.regionTrimmed === false) &&
    typeof r.closed === 'boolean' &&
    r.regionTopologyCertified === false &&
    r.offsetRegularityCertified === false &&
    isNonnegativeFinite(r.errorUpperMm) &&
    finite(r.toleranceMm) &&
    r.toleranceMm > 0 &&
    r.errorUpperMm <= r.toleranceMm &&
    Array.isArray(r.cells) &&
    r.cells.length <= 65536 &&
    r.cells.every(
      c =>
        !!c &&
        arrayOf(c.domain, 2, finite) &&
        c.domain[0] < c.domain[1] &&
        isNonnegativeFinite(c.errorUpperMm) &&
        c.errorUpperMm <= r.errorUpperMm,
    ) &&
    (r.wholeWire !== true ||
      (r.cells.length > 0 &&
        r.cells.every((c, i) => {
          const role = c.source
          return (
            c.domain[0] === i &&
            c.domain[1] === i + 1 &&
            !!role &&
            (role.kind === 'source-offset'
              ? arrayOf(role.domain, 2, finite) && role.domain[0] < role.domain[1]
              : role.kind === 'bevel' && finite(role.sourceKnot))
          )
        }))) &&
    (r.cells.length
      ? validCurveOffsetDiagnostics(r.chainDiagnostics, r.cells.length)
      : r.chainDiagnostics === null)
  )
}

export function validProfileIntersectionsResult(
  job: Extract<MainSolidExpectation, { kind: 'profileIntersections' }>,
  value: unknown,
): boolean {
  try {
    validateProfileIntersectionReport(value, job.loops, job.options)
    return true
  } catch {
    return false
  }
}

export function validProfilePrepareResult(
  job: Extract<MainSolidExpectation, { kind: 'profilePrepare' }>,
  value: unknown,
): boolean {
  const v = value as MainSolidResults['profilePrepare']
  const r = v.report
  const point2d = (p: unknown) => arrayOf(p, 2, finite)
  if (r?.diagnosticLoops !== undefined) {
    if (
      r.accepted ||
      r.reason !== 'invalid-contour' ||
      !Array.isArray(r.diagnosticLoops) ||
      r.diagnosticLoops.length !== 1 ||
      !Array.isArray(r.diagnosticLoops[0]) ||
      r.diagnosticLoops[0].length < 2 ||
      r.diagnosticLoops[0].length > 254 ||
      !r.curveSources ||
      r.curveSources.length !== r.diagnosticLoops[0].length
    ) {
      return false
    }
    if (r.intersections) {
      try {
        validateProfileIntersectionReport(r.intersections, r.diagnosticLoops, {
          toleranceMm: 1e-7,
          maxPairs: 128,
          maxBoxes: 8192,
        })
      } catch {
        return false
      }
    } else if (
      typeof r.intersectionDiagnosticError !== 'string' ||
      !r.intersectionDiagnosticError
    ) {
      return false
    }
  } else if (
    r?.intersections !== undefined ||
    r?.intersectionDiagnosticError !== undefined ||
    r?.diagnosticDisplay !== undefined
  ) {
    return false
  }
  if (
    r?.diagnosticDisplay !== undefined &&
    (!r.intersections ||
      !Array.isArray(r.diagnosticDisplay) ||
      r.diagnosticDisplay.length > 254 ||
      r.diagnosticDisplay.some(
        s =>
          !s ||
          !Number.isInteger(s.curve) ||
          s.curve < 0 ||
          s.curve >= r.diagnosticLoops![0].length ||
          !['intersection', 'overlap', 'unproven'].includes(s.kind) ||
          !Array.isArray(s.points) ||
          s.points.length < 2 ||
          s.points.length > 49 ||
          !s.points.every(point2d),
      ) ||
      new Set(r.diagnosticDisplay.map(s => s.curve)).size !== r.diagnosticDisplay.length)
  ) {
    return false
  }
  return (
    validDirectDocument(v.document) &&
    typeof v.id === 'string' &&
    !!v.plane &&
    vector(v.plane.origin) &&
    vector(v.plane.u) &&
    vector(v.plane.v) &&
    !!r &&
    typeof r.accepted === 'boolean' &&
    ['accepted', 'endpoint-topology', 'disconnected', 'invalid-contour'].includes(r.reason) &&
    (r.projectionMaxDeviationMm === undefined ||
      (isNonnegativeFinite(r.projectionMaxDeviationMm) && r.projectionMaxDeviationMm <= 1e-7)) &&
    r.accepted === (r.reason === 'accepted') &&
    Array.isArray(r.points) &&
    r.points.every(point2d) &&
    Array.isArray(r.defects) &&
    r.defects.every(
      d =>
        d &&
        Number.isInteger(d.chain) &&
        d.chain >= 0 &&
        ['start', 'end'].includes(d.end) &&
        point2d(d.point) &&
        ['gap', 'ambiguous'].includes(d.kind) &&
        Array.isArray(d.candidates) &&
        d.candidates.every(i => Number.isInteger(i) && i >= 0),
    ) &&
    Array.isArray(r.connectors) &&
    r.connectors.every(c => c && point2d(c.a) && point2d(c.b)) &&
    (r.curveSources === undefined ||
      (Array.isArray(r.curveSources) &&
        r.curveSources.length <= 512 &&
        r.curveSources.every(
          s =>
            s &&
            Number.isInteger(s.chain) &&
            s.chain >= 0 &&
            typeof s.reversed === 'boolean' &&
            (!job.chainSegments || s.chain < job.chainSegments.length) &&
            (s.connector === false
              ? Number.isInteger(s.segment) &&
                s.segment >= 0 &&
                (!job.chainSegments || s.segment < job.chainSegments[s.chain])
              : s.connector === true &&
                ['start', 'end'].includes(s.end) &&
                Number.isInteger(s.nextChain) &&
                s.nextChain >= 0 &&
                (!job.chainSegments || s.nextChain < job.chainSegments.length)),
        ) &&
        (!r.profile ||
          (Array.isArray(r.profile.loops) &&
            r.profile.loops.every(Array.isArray) &&
            r.curveSources.length ===
              r.profile.loops.reduce((n, loop) => n + loop.length, 0))))) &&
    (!r.segmentDefect ||
      (['intersection', 'overlap', 'degenerate', 'unproven'].includes(r.segmentDefect.kind) &&
        Array.isArray(r.segmentDefect.segments) &&
        r.segmentDefect.segments.every(
          s => Number.isInteger(s.index) && s.index >= 0 && point2d(s.a) && point2d(s.b),
        )))
  )
}
