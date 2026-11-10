import { validG2RunoutDiagnosticReport, validG2RunoutResult } from '../../geometricG2'
import {
  finite,
  isNumericSequence,
  vector,
  type MainSolidExpectation,
  type MainSolidResults,
} from '../../mainSolidProtocol'
import type { SolidBooleanResult } from '../../../kernel/solidBoolean'
import type { SolidBrepToolResult } from '../../../kernel/solidBrepTool'
import { arrayOf, numericSequenceOf } from '../mainSolidExpectation'
import { isNonnegativeFinite, isSafeNonnegativeCount, validDirectDocument } from './predicates'

export function validPartialAnnularPreviewResult(
  job: Extract<MainSolidExpectation, { kind: 'partialAnnularPreview' }>,
  value: unknown,
): boolean {
  const v = value as MainSolidResults['partialAnnularPreview']
  const q = v.evidence?.qualification
  const mesh = v.body?.mesh
  if (
    v.body?.id !== job.id ||
    !job.bodyIdentity ||
    v.body.brep?.topologyIds?.bodies.length !== 1 ||
    v.body.brep.topologyIds.bodies[0] !== job.bodyIdentity ||
    !mesh ||
    q?.status !== 'preview-only' ||
    q.commitAllowed !== false ||
    q.boundaryIntersectionProof !== 'unqualified' ||
    q.transitionContinuityProof !== 'unqualified' ||
    !isNumericSequence(mesh.positions) ||
    !isNumericSequence(mesh.indices) ||
    !mesh.positions.length ||
    mesh.positions.length % 3 ||
    !mesh.indices.length ||
    mesh.indices.length % 3
  ) {
    return false
  }
  return (
    numericSequenceOf(mesh.positions, mesh.positions.length, finite) &&
    numericSequenceOf(
      mesh.indices,
      mesh.indices.length,
      i => Number.isSafeInteger(i) && Number(i) >= 0 && Number(i) < mesh.positions.length / 3,
    )
  )
}

export function validBrepToolResult(
  job: Extract<MainSolidExpectation, { kind: 'brepTool' }>,
  value: unknown,
): boolean {
  const v = value as SolidBrepToolResult
  if (v.id !== job.id || v.kind !== job.operation) return false
  if (v.kind === 'display') {
    const d = v.diagnostics
    return (
      !!d &&
      !!d.report &&
      (['boundaryEdges', 'nonManifoldEdges', 'orientationConflicts', 'degenerateTriangles'] as const).every(
        k => isSafeNonnegativeCount(d.report[k]),
      ) &&
      Array.isArray(d.defectLines) &&
      d.defectLines.every(
        l => typeof l.kind === 'string' && Array.isArray(l.points) && l.points.every(vector),
      ) &&
      !!d.section &&
      vector(d.section.normal) &&
      finite(d.section.offsetMm) &&
      Array.isArray(d.section.contours) &&
      d.section.contours.every(
        c =>
          Array.isArray(c.points) &&
          c.points.every(vector) &&
          Array.isArray(c.sourceTriangles),
      ) &&
      Array.isArray(d.section.collapsedSegmentTriangles) &&
      typeof d.sectionError === 'string' &&
      typeof d.boundaryError === 'string'
    )
  }
  if (v.kind === 'mesh') {
    return validDirectDocument(v.document) && v.document.bodies.some(b => b.id === v.id)
  }
  if (v.kind !== 'mass' || !v.mass) return false
  const m = v.mass
  return (
    isNonnegativeFinite(m.surfaceAreaMm2) &&
    finite(m.signedVolumeMm3) &&
    vector(m.centroid) &&
    arrayOf(m.inertiaMm5, 3, vector) &&
    arrayOf(m.conservativeBounds, 2, vector) &&
    isNonnegativeFinite(m.areaErrorEstimateMm2) &&
    isNonnegativeFinite(m.volumeErrorEstimateMm3) &&
    isSafeNonnegativeCount(m.evaluations) &&
    m.status === 'converged_estimate' &&
    m.solidGeometryStatus === 'not_certified'
  )
}

export function validSolidBooleanResult(value: unknown): boolean {
  const v = value as SolidBooleanResult
  return (
    validDirectDocument(v.document) &&
    typeof v.exact === 'boolean' &&
    isNonnegativeFinite(v.toleranceMm) &&
    (v.resultId === null ||
      (typeof v.resultId === 'string' && v.document.bodies.some(b => b.id === v.resultId)))
  )
}

function validAcceptedDocumentEnvelope(value: unknown): boolean {
  const v = value as MainSolidResults['curveMatch' | 'surfaceMatch' | 'seamPrepare']
  const r = v.report
  return (
    validDirectDocument(v.document) &&
    !!r &&
    typeof r.accepted === 'boolean' &&
    typeof r.reason === 'string' &&
    r.accepted === (r.reason === 'accepted')
  )
}

export function validCurveMatchResult(value: unknown): boolean {
  if (!validAcceptedDocumentEnvelope(value)) return false
  const c = (value as MainSolidResults['curveMatch']).report
  return (
    c.method === 'outward-endpoint-handle-wedge' &&
    isNonnegativeFinite(c.positionErrorUpper) &&
    isNonnegativeFinite(c.maxAngleDegrees) &&
    (c.sineAngleUpper === null || isNonnegativeFinite(c.sineAngleUpper)) &&
    (c.angleDegreesUpper === null || isNonnegativeFinite(c.angleDegreesUpper)) &&
    typeof c.regularityCertified === 'boolean' &&
    typeof c.orientationCertified === 'boolean' &&
    (!c.accepted ||
      (c.regularityCertified &&
        c.orientationCertified &&
        c.angleDegreesUpper !== null &&
        c.angleDegreesUpper <= c.maxAngleDegrees))
  )
}

export function validSurfaceMatchResult(value: unknown): boolean {
  if (!validAcceptedDocumentEnvelope(value)) return false
  const c = (value as MainSolidResults['surfaceMatch']).report
  const b = c.errorBounds
  return (
    [1, 2].includes(c.continuityOrder) &&
    finite(c.normalScale) &&
    c.normalScale > 0 &&
    isNonnegativeFinite(c.errorUpper) &&
    isNonnegativeFinite(c.maxError) &&
    c.method === 'homogeneous-normalized-boundary-jets' &&
    c.seamBasis === 'exact-affine-knot-correspondence' &&
    !!b &&
    b.wholeSeam === true &&
    b.normalizedParameters === true &&
    isNonnegativeFinite(b.positionUpper) &&
    isNonnegativeFinite(b.firstDerivativeUpper) &&
    (b.secondDerivativeUpper === null || isNonnegativeFinite(b.secondDerivativeUpper)) &&
    (b.mixedDerivativeUpper === null || isNonnegativeFinite(b.mixedDerivativeUpper)) &&
    (!c.accepted ||
      (c.regularityCertified &&
        c.tangentialSmoothnessCertified &&
        c.errorUpper <= c.maxError))
  )
}

export function validSeamPrepareResult(value: unknown): boolean {
  if (!validAcceptedDocumentEnvelope(value)) return false
  const v = value as MainSolidResults['seamPrepare']
  const c = v.report
  const b = v.basis
  return (
    isNonnegativeFinite(c.budget) &&
    !!b &&
    Number.isInteger(b.degree) &&
    b.degree >= 1 &&
    Number.isInteger(b.controlCount) &&
    b.controlCount > b.degree &&
    b.normalizedSeam === true &&
    typeof b.editedReversed === 'boolean' &&
    arrayOf(b.referenceDomain, 2, finite) &&
    arrayOf(b.editedDomain, 2, finite) &&
    (!c.accepted ||
      (c.wholeSurface === true &&
        isNonnegativeFinite(c.referenceErrorUpper) &&
        isNonnegativeFinite(c.editedErrorUpper) &&
        c.referenceErrorUpper! <= c.budget &&
        c.editedErrorUpper! <= c.budget))
  )
}

export function validSurfaceBuildResult(
  job: Extract<MainSolidExpectation, { kind: 'surfaceBuild' }>,
  value: unknown,
): boolean {
  const v = value as MainSolidResults['surfaceBuild']
  const r = v.report
  if (
    job.runoutId &&
    !v.document &&
    v.runoutReport !== undefined &&
    !validG2RunoutDiagnosticReport(v.runoutReport)
  ) {
    return false
  }
  if (job.runoutId && v.document) {
    const patches = (v.document.surfaces ?? [])
      .filter(s => s.id === job.runoutId || s.id.startsWith(job.runoutId + ':'))
      .map(s => s.surface)
    if (
      !validG2RunoutResult({ patches, report: v.runoutReport }, 100000, 1000000, 256) ||
      !v.runoutReport?.withinTolerance
    ) {
      return false
    }
  }
  if (
    typeof v.error !== 'string' ||
    (v.document === null ? v.error.length === 0 : v.error !== '' || !validDirectDocument(v.document))
  ) {
    return false
  }
  if (r === null) return true
  if (
    !r ||
    typeof r.accepted !== 'boolean' ||
    !isNonnegativeFinite(r.sampledControlDeviation) ||
    !isNonnegativeFinite(r.budget) ||
    !Number.isInteger(r.stations) ||
    r.stations <= 0 ||
    !Number.isInteger(r.sections) ||
    r.sections <= 0 ||
    typeof r.continuousBound !== 'boolean' ||
    r.method !== 'double-reflection-fourfold-section-refinement'
  ) {
    return false
  }
  const certOk =
    r.continuousCertificate === undefined
      ? r.continuousBound === false && r.accepted === (r.sampledControlDeviation <= r.budget)
      : !!r.continuousCertificate &&
        r.continuousCertificate.scope === 'matched-parameter-profile-deviation' &&
        typeof r.continuousCertificate.method === 'string' &&
        (r.continuousCertificate.reason === null ||
          typeof r.continuousCertificate.reason === 'string') &&
        r.continuousCertificate.regularityCertified === false &&
        r.continuousCertificate.globalEmbeddingCertified === false &&
        r.continuousCertificate.seamSmoothnessCertified === false &&
        Number.isInteger(r.continuousCertificate.cells) &&
        r.continuousCertificate.cells >= 0 &&
        Number.isInteger(r.continuousCertificate.maxCells) &&
        r.continuousCertificate.maxCells >= r.continuousCertificate.cells &&
        r.continuousCertificate.maxCells <= 100000 &&
        r.continuousBound === (r.continuousCertificate.errorUpper !== null) &&
        (r.continuousCertificate.errorUpper === null ||
          isNonnegativeFinite(r.continuousCertificate.errorUpper)) &&
        r.continuousCertificate.withinBudget ===
          (r.continuousBound && r.continuousCertificate.errorUpper! <= r.budget) &&
        r.accepted === r.continuousCertificate.withinBudget
  const seamOk =
    !r.seamContinuity ||
    !['G1', 'G2'].includes(r.seamContinuity) ||
    (!!r.seamCertificate &&
      r.seamCertificate.certified === true &&
      r.seamCertificate.exact === true &&
      r.seamCertificate.regularityCertified === true &&
      r.seamCertificate.order === (r.seamContinuity === 'G2' ? 2 : 1) &&
      r.seamCertificate.scope === 'represented-closing-strip-jets' &&
      r.seamCertificate.method === 'dyadic-cubic-closing-strips-exact-predicate' &&
      Number.isInteger(r.seamCertificate.work) &&
      r.seamCertificate.work >= 0 &&
      r.seamCertificate.work <= 1000000)
  return certOk && seamOk && (!v.document || r.accepted)
}

export function validNurbsRefitResult(value: unknown): boolean {
  const v = value as MainSolidResults['nurbsRefit']
  const c = v.certificate
  return (
    validDirectDocument(v.document) &&
    !!c &&
    /^nurbs-foundation\/[1-5]$/.test(c.version) &&
    typeof c.accepted === 'boolean' &&
    typeof c.rolledBack === 'boolean' &&
    !(c.accepted && c.rolledBack) &&
    (['geometryErrorUpper', 'hausdorffErrorUpper', 'dataSiteErrorUpper', 'budget'] as const).every(
      key => {
        const n = c[key]
        return n === undefined || isNonnegativeFinite(n)
      },
    ) &&
    !!c.evidence &&
    !!c.evidence.toleranceIdentity &&
    typeof c.evidence.toleranceIdentity.canonical === 'string'
  )
}
