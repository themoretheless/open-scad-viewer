import { isBondedSolidResult } from '../bondedSolidProtocol'
import { validExpressionShell } from '../expressionShell'
import { validExpressionInspection } from '../expressionShellInspection'
import { validFrozenGraphBody, validFrozenGraphDisplay } from '../frozenGraphBody'
import { validG2RunoutResult } from '../geometricG2'
import { isNominalLatticeGraph } from '../../fabrication/latticeGraphProtocol'
import type { MainSolidExpectation } from '../mainSolidProtocol'
import {
  isAxialScreeningRows,
  isStructuralProfileCatalog,
  isThermalStrengthResult,
  isTrussFieldMeshes,
  isTrussTopologyResponse,
  isValidatedPrintProfile,
} from '../../app/mechanicsProtocol'
import { validContactQualification } from '../nurbsOffsetContactQualification'
import { validOffsetDeviation } from '../nurbsOffsetDeviation'
import { validEnvelopeFit } from '../nurbsOffsetEnvelopeFit'
import { validProfileBridgeChordReport } from '../nurbsProfileBridgeChords'
import { validBoundaryAgreement } from '../solidBoundaryAgreement'
import { validSolidDistance } from '../solidDistance'
import { validFaceContacts } from '../solidFaceContacts'
import { validMaterial } from '../solidMaterialVolume'
import { validMaterialWall } from '../solidMaterialWall'
import { validSelfIntersection } from '../solidSelfIntersection'
import { validBodySnapGeometry, validSketchSnapGeometry } from '../solidSnapProtocol'
import { validSolidTopology } from '../solidTopologyProtocol'
import { validWholeWall } from '../solidWholeWall'
import { validSourceBody } from '../sourceBody'
import { validSourceGraphG2Preview } from '../sourceGraphG2Preview'
import {
  validCurveDistanceResult,
  validFaceDistanceResult,
  validMeasureEdgeResult,
  validMeasureVerticesResult,
  validShellDistanceResult,
  validSurfaceDistanceResult,
} from './results/distance'
import {
  validAnimationFramesResult,
  validAnimationSampleResult,
  validDrawingPdfResult,
  validDrawingSheetsResult,
  validInspectResult,
  validStructuralSectionsResult,
  validTrussResult,
} from './results/mechanicsAndMedia'
import {
  validDisplayMeshResult,
  validManifoldMetricsResult,
  validMeshBooleanResult,
  validMeshContactsResult,
  validMeshConvertResult,
  validMeshExportResult,
  validMeshImportResult,
  validMeshManifoldResult,
  validMeshRepairResult,
  validSurfaceBoundaryResult,
  validSurfaceMeshResult,
} from './results/meshAndManifold'
import { validMinkowskiResult } from './results/minkowski'
import {
  validBrepToolResult,
  validCurveMatchResult,
  validNurbsRefitResult,
  validPartialAnnularPreviewResult,
  validSeamPrepareResult,
  validSolidBooleanResult,
  validSurfaceBuildResult,
  validSurfaceMatchResult,
} from './results/nurbsConstruction'
import { validOffsetContact } from './results/offsetContact'
import { validOffsetEnvelope } from './results/offsetEnvelope'
import { isObjectRecord, validDirectDocument } from './results/predicates'
import {
  validBodyEdgesResult,
  validCurveChainInspectionResult,
  validCurveDisplayResult,
  validCurveOffsetResult,
  validFaceSketchResult,
  validProfileDisplayResult,
  validProfileIntersectionsResult,
  validProfilePrepareResult,
  validTrimmedCurveOffsetResult,
} from './results/profileAndCurve'

type ResultValidatorMap = {
  [K in MainSolidExpectation['kind']]: (
    job: Extract<MainSolidExpectation, { kind: K }>,
    value: unknown,
  ) => boolean
}

const PRE_OBJECT_KINDS = new Set<MainSolidExpectation['kind']>([
  'expressionShellRestore',
  'expressionShellInspect',
  'sourceBodyRestore',
  'profileBridgeChords',
])

const directDocumentEntry = (_job: unknown, value: unknown): boolean =>
  validDirectDocument(value)

const RESULT_VALIDATORS: ResultValidatorMap = {
  expressionShellRestore: validExpressionShell,
  expressionShellInspect: validExpressionInspection,
  sourceBodyRestore: validSourceBody,
  profileBridgeChords: validProfileBridgeChordReport,
  frozenGraphBodyDisplay: (job, value) =>
    validFrozenGraphDisplay(value, job.expected, job.segments),
  frozenGraphBodyRestore: (job, value) => validFrozenGraphBody(value, job.expected),
  sourceGraphG2Preview: (job, value) => validSourceGraphG2Preview(value, job.options),
  g2RunoutSurface: (job, value) =>
    validG2RunoutResult(
      value,
      job.maxCells,
      job.maxExactWork,
      job.maxPatches,
      job.tolerances,
    ),
  minkowski3d: (job, value) => validMinkowskiResult(job, value, validDirectDocument),
  animationFrames: (_job, value) => validAnimationFramesResult(value),
  animationSample: (_job, value) => validAnimationSampleResult(value),
  drawingPdf: (_job, value) => validDrawingPdfResult(value),
  drawingSheets: (_job, value) => validDrawingSheetsResult(value),
  offsetContactQualification: validContactQualification,
  offsetEnvelopeFit: validEnvelopeFit,
  offsetEnvelope: (job, value) => validOffsetEnvelope(job, value, mainSolidResult),
  offsetDeviation: (job, value) => validOffsetDeviation(job.options, value),
  offsetContactTangent: (job, value) => validOffsetContact(job, value, mainSolidResult),
  offsetSourceBoundary: (job, value) => validOffsetContact(job, value, mainSolidResult),
  trimmedOffsetContactBand: (job, value) => validOffsetContact(job, value, mainSolidResult),
  offsetContactBand: (job, value) => validOffsetContact(job, value, mainSolidResult),
  wholeWall: validWholeWall,
  materialWall: validMaterialWall,
  materialSegment: validMaterial,
  materialChord: validMaterial,
  solidDistance: validSolidDistance,
  selfIntersection: validSelfIntersection,
  faceContacts: validFaceContacts,
  boundaryAgreement: validBoundaryAgreement,
  shellDistance: validShellDistanceResult,
  faceDistance: validFaceDistanceResult,
  surfaceDistance: validSurfaceDistanceResult,
  curveDistance: validCurveDistanceResult,
  sketchSnaps: (job, value) =>
    validSketchSnapGeometry(value, job.maxPoints, job.maxSegments, job.maxCircles),
  bodySnaps: (job, value) => validBodySnapGeometry(value, job.maxPoints, job.maxSegments),
  faceSketch: (_job, value) => validFaceSketchResult(value),
  bodyEdges: validBodyEdgesResult,
  topology: (job, value) => validSolidTopology(value, job.vertices, job.triangles),
  curveChainInspection: validCurveChainInspectionResult,
  curveDisplay: validCurveDisplayResult,
  profileDisplay: validProfileDisplayResult,
  surfaceMesh: validSurfaceMeshResult,
  surfaceBoundary: validSurfaceBoundaryResult,
  measureVertices: (_job, value) => validMeasureVerticesResult(value),
  manifoldMetrics: (_job, value) => validManifoldMetricsResult(value),
  meshManifold: validMeshManifoldResult,
  meshRepair: validMeshRepairResult,
  meshBoolean: validMeshBooleanResult,
  meshImport: validMeshImportResult,
  meshConvert: validMeshConvertResult,
  meshExport: validMeshExportResult,
  measureEdge: (_job, value) => validMeasureEdgeResult(value),
  partialAnnularPreview: validPartialAnnularPreviewResult,
  displayMesh: validDisplayMeshResult,
  brepTool: validBrepToolResult,
  boolean: (_job, value) => validSolidBooleanResult(value),
  curveMatch: (_job, value) => validCurveMatchResult(value),
  surfaceMatch: (_job, value) => validSurfaceMatchResult(value),
  seamPrepare: (_job, value) => validSeamPrepareResult(value),
  trimmedCurveOffset: validTrimmedCurveOffsetResult,
  curveOffset: (_job, value) => validCurveOffsetResult(value),
  surfaceBuild: validSurfaceBuildResult,
  nurbsRefit: (_job, value) => validNurbsRefitResult(value),
  profileIntersections: validProfileIntersectionsResult,
  profilePrepare: validProfilePrepareResult,
  meshContacts: validMeshContactsResult,
  bondedSolid: (job, value) => isBondedSolidResult(value, job.nodes, job.tets, job.bonds),
  latticeGraph: (_job, value) => isNominalLatticeGraph(value),
  structuralSections: validStructuralSectionsResult,
  trussField: (job, value) => isTrussFieldMeshes(value, job.members),
  trussScreening: (job, value) =>
    isAxialScreeningRows(value, job.members, job.responses),
  printStrengthProfile: (job, value) => isValidatedPrintProfile(value, job.profile),
  thermalStrength: (_job, value) => isThermalStrengthResult(value),
  trussTopology: (job, value) =>
    isTrussTopologyResponse(value, job.nodes, job.members, job.cases),
  structuralProfiles: (_job, value) => isStructuralProfileCatalog(value),
  truss: validTrussResult,
  inspect: (_job, value) => validInspectResult(value),
  restoreDocument: directDocumentEntry,
  primitive: directDocumentEntry,
  rushGraphImport: directDocumentEntry,
  nurbsEdit: directDocumentEntry,
  pointEdit: directDocumentEntry,
  sketchEdit: directDocumentEntry,
  sceneEdit: directDocumentEntry,
  profileEdit: directDocumentEntry,
  bodyEdit: directDocumentEntry,
  revolve: directDocumentEntry,
  extrusion: directDocumentEntry,
  main: directDocumentEntry,
  cad: directDocumentEntry,
  transaction: directDocumentEntry,
}

/** Admit the result for this request, not merely any object with a result field. */
export function mainSolidResult(job: MainSolidExpectation, value: unknown): boolean {
  if (!PRE_OBJECT_KINDS.has(job.kind) && !isObjectRecord(value)) return false
  const validator = RESULT_VALIDATORS[job.kind] as
    | ((expectation: MainSolidExpectation, candidate: unknown) => boolean)
    | undefined
  return typeof validator === 'function' ? validator(job, value) : false
}
