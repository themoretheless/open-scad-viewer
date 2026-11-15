import { inspectRushGraphOffsetDeviation } from '../rushGraphOffsetDeviation'
import {
  readMiterAffineReplayRouting,
  readNativeSweepPlanarCapFaces,
  inspectNurbsBrep,
  tessellateNurbsBrep,
} from '../../kernel/geometry/brep'
import { packNativeGeometrySnapshot } from '../../kernel/nativeGeometrySnapshot'
import type { StationResolver, Value } from '../rushGraphNurbsKernel'
import { createOwnNurbsResolver } from './createOwnNurbsResolver'
import type { BodyResolver, Mesh, MiterResolver, OwnNurbsRequest, SweepResolver } from '../rushGraphNurbsKernel'
import { exportDirectStepV9 } from '../../kernel/cadNurbsStep'
import { evaluateNurbsCurve, nurbsCurveBounds } from '../../kernel/nurbsCurve'
import { evaluateNurbsSurface, nurbsSurfaceBounds } from '../../kernel/nurbsSurface'
import { certifyNurbsCurveFoundation, certifyNurbsSurfaceFoundation } from '../../kernel/nurbsFoundation'
import { compileRushGraphNurbs } from '../rushGraphNurbsCompiler'
import { createNativeGeometryArtifact } from '../../core/nativeGeometry'
import { exportMeshFormat, meshExportBase64 } from '../../exchange/meshExportFormats'
import { exportNurbsStl, tessellateNurbsSurface } from '../../kernel/geometry/tessellation'
import { tessellateNurbsPatches } from '../../kernel/geometry/reconstruction'
import { stringifyMeshJson } from '../../exchange/meshJson'
import { tessellateSubdivision } from '../../kernel/geometry/subdivision'
import type { NurbsCurve } from '../../kernel/nurbsCurve'

type CompiledNurbs = ReturnType<typeof compileRushGraphNurbs>
type ResolvedNurbsMap = Map<string, Value>
type NurbsNodeMap = Map<string, CompiledNurbs['resolved_document']['nodes'][number]>
type ResolverState = ReturnType<typeof createOwnNurbsResolver>

function computeMeshBounds(root: Extract<Value, { kind: 'mesh' }>) {
  if (root.data.positions.length === 0) return null
  const bounds = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] }
  for (let i = 0; i < root.data.positions.length; i++) {
    const axis = i % 3
    const v = root.data.positions[i]
    bounds.min[axis] = Math.min(bounds.min[axis], v)
    bounds.max[axis] = Math.max(bounds.max[axis], v)
  }
  return bounds
}

function computePatchBounds(root: Extract<Value, { kind: 'patches' }>) {
  if (root.data.patches.length === 0) return null
  const bounds = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] }
  for (const patch of root.data.patches) {
    const surfaceBounds = nurbsSurfaceBounds(patch)
    for (let axis = 0; axis < 3; axis++) {
      bounds.min[axis] = Math.min(bounds.min[axis], surfaceBounds.min[axis])
      bounds.max[axis] = Math.max(bounds.max[axis], surfaceBounds.max[axis])
    }
  }
  return bounds
}

function buildNurbsReport(
  compiled: CompiledNurbs,
  root: Value,
  cache: ResolvedNurbsMap,
  constructionReports: ResolverState['constructionReports'],
  reconstructionReports: ResolverState['reconstructionReports'],
) {
  const definitions = Object.fromEntries(
    [...cache].filter(([, v]) => v.kind !== 'mesh').map(([id, v]) => [id, { kind: v.kind, ...v.data }]),
  )
  const extendedGeometry = compiled.document.nodes.some(
    n =>
      n.op.startsWith('polygon_') ||
      n.op === 'subdivision' ||
      n.op === 'triangle_mesh' ||
      n.op.startsWith('mesh_to_') ||
      n.op === 'mesh_fit_nurbs' ||
      n.op.startsWith('sdf_'),
  )
  const foundationCertificate =
    root.kind === 'curve'
      ? certifyNurbsCurveFoundation(root.data)
      : root.kind === 'surface'
        ? certifyNurbsSurfaceFoundation(root.data)
        : null
  return {
    ...(Object.keys(constructionReports).length ? { construction: constructionReports } : {}),
    ...(Object.keys(reconstructionReports).length ? { reconstruction: reconstructionReports } : {}),
    bounds_scope: foundationCertificate
      ? 'outward-rounded rational span hulls'
      : root.kind === 'mesh'
        ? 'derived mesh vertices'
        : 'conservative control hull',
    kernel: extendedGeometry ? 'own-rust-geometry' : 'own-rust-nurbs',
    polygon_core: 'own-rust-polygons',
    geometry_authority: extendedGeometry ? 'source geometry definitions' : 'rational control data',
    root: compiled.document.root,
    root_kind: root.kind,
    bounds:
      root.kind === 'curve'
        ? nurbsCurveBounds(root.data)
        : root.kind === 'surface'
          ? nurbsSurfaceBounds(root.data)
          : root.kind === 'patches'
            ? computePatchBounds(root)
            : root.kind === 'mesh'
              ? computeMeshBounds(root)
              : null,
    foundation_certificate: foundationCertificate,
    foundation_successor: foundationCertificate
      ? {
          version: 'nurbs-ss/1',
          curve_projection: 'Bernstein stationary isolation',
          surface_projection: '2d outward hull subdivision with Krawczyk/interval uniqueness',
          normal_regularity: 'recursive sub-knot-cell homogeneous cone localization',
          simplification: 'adaptive Lipschitz rollback certified',
          periodic_editing: 'wrapped cyclic collocation with C0/C1/C2 seam evidence',
          reparameterization:
            'piecewise positive rational monotone with nested/periodic exact control-net materialization',
          fitting: 'ranked bounded approximate-only with cloud Hausdorff enclosure',
          intersection:
            'certified general CC/CS/SS with Bernstein/Krawczyk isolation, BranchGraph, CoedgeTrim maps; Boolean mutation deferred',
        }
      : null,
    definitions,
    mesh: root.kind === 'mesh' ? root.data.report : null,
    printability: 'unknown',
    error_bound_certified: foundationCertificate !== null && Object.keys(constructionReports).length === 0,
  }
}

function exportOwnNurbsArtifact<Base extends Record<string, unknown>>(
  root: Value,
  compiled: CompiledNurbs,
  request: OwnNurbsRequest,
  base: Base,
) {
  if (request.format === 'json')
    return {
      ...base,
      artifact: { format: 'json', mime_type: 'application/json', text: stringifyMeshJson(compiled.document, 2) },
    }
  if (request.format === 'step') {
    if (root.kind !== 'brep')
      throw new Error('STEP export requires an exact B-rep root; mesh/curve/surface STEP conversion is not supported.')
    inspectNurbsBrep(root.data)
    const text = exportDirectStepV9(root.data).text
    if (new TextEncoder().encode(text).byteLength > 4 * 1024 * 1024) throw new Error('STEP export exceeds 4 MiB.')
    return { ...base, artifact: { format: 'step', mime_type: 'model/step', extension: 'step', text } }
  }
  if (!request.format || root.kind !== 'mesh')
    throw new Error('STL export requires a closed tessellated mesh. Export JSON to retain native NURBS definitions.')
  if (request.format !== 'stl') {
    const artifact = exportMeshFormat(root.data, request.format)
    return {
      ...base,
      artifact: {
        format: request.format,
        mime_type: artifact.mimeType,
        extension: artifact.extension,
        base64: meshExportBase64(artifact.data),
      },
    }
  }
  const text = exportNurbsStl(root.data)
  if (text.length > 4 * 1024 * 1024) throw new Error('STL export exceeds 4 MiB.')
  return { ...base, artifact: { format: 'stl', mime_type: 'model/stl', text } }
}

function extractSweepEvidence(sourceConstruction: object | undefined) {
  if (!sourceConstruction || !('volume' in sourceConstruction)) return undefined
  return {
    volume: sourceConstruction.volume,
    profileSmoothness: 'profileSmoothness' in sourceConstruction ? sourceConstruction.profileSmoothness : null,
    boundaryCertificate: 'boundaryCertificate' in sourceConstruction ? sourceConstruction.boundaryCertificate : null,
    certifiedErrorUpper: 'certifiedErrorUpper' in sourceConstruction ? sourceConstruction.certifiedErrorUpper : null,
    retainedCapDecomposition:
      'retainedCapDecomposition' in sourceConstruction ? sourceConstruction.retainedCapDecomposition : null,
    retainedDecomposition:
      'retainedDecomposition' in sourceConstruction ? sourceConstruction.retainedDecomposition : null,
    retainedWallErrorUpper:
      'retainedWallErrorUpper' in sourceConstruction ? sourceConstruction.retainedWallErrorUpper : null,
    capParallelism: 'capParallelism' in sourceConstruction ? sourceConstruction.capParallelism : null,
    capProjection: 'capProjection' in sourceConstruction ? sourceConstruction.capProjection : null,
    idealCapDomains: 'idealCapDomains' in sourceConstruction ? sourceConstruction.idealCapDomains : null,
    filledCapErrorUpper: 'filledCapErrorUpper' in sourceConstruction ? sourceConstruction.filledCapErrorUpper : null,
    boundaryErrorUpper: 'boundaryErrorUpper' in sourceConstruction ? sourceConstruction.boundaryErrorUpper : null,
    boundaryErrorWithinBudget:
      'boundaryErrorWithinBudget' in sourceConstruction ? sourceConstruction.boundaryErrorWithinBudget : null,
    boundaryErrorBudget:
      'boundaryErrorWithinBudget' in sourceConstruction && 'budget' in sourceConstruction
        ? sourceConstruction.budget
        : null,
    sectionCorrection: 'sectionCorrection' in sourceConstruction ? sourceConstruction.sectionCorrection : null,
    endpointContourErrorUpper:
      'endpointContourErrorUpper' in sourceConstruction ? sourceConstruction.endpointContourErrorUpper : null,
    authoredFramesApplied:
      'authoredFramesApplied' in sourceConstruction ? sourceConstruction.authoredFramesApplied : false,
    orientationGuideApplied:
      'orientationGuideApplied' in sourceConstruction ? sourceConstruction.orientationGuideApplied : false,
    affineLawsApplied: 'affineLawsApplied' in sourceConstruction ? sourceConstruction.affineLawsApplied : false,
    profileRegularityCertified:
      'profileRegularityCertified' in sourceConstruction ? sourceConstruction.profileRegularityCertified : false,
    wallRegularityCertified:
      'wallRegularityCertified' in sourceConstruction ? sourceConstruction.wallRegularityCertified : false,
    continuousBound: 'continuousBound' in sourceConstruction ? sourceConstruction.continuousBound : false,
  }
}

function buildOwnNurbsDisplayResult<Base extends Record<string, unknown>>(
  compiled: CompiledNurbs,
  nodes: NurbsNodeMap,
  root: Value,
  resolver: ResolverState,
  needCurve: (key: string) => NurbsCurve,
  display: NonNullable<OwnNurbsRequest['display']>,
  base: Base,
) {
  const { segments, subdivisionLevels } = display
  if (
    !Number.isInteger(segments) ||
    segments < 1 ||
    segments > 32 ||
    !Number.isInteger(subdivisionLevels) ||
    subdivisionLevels < 0 ||
    subdivisionLevels > 4
  )
    throw new Error('Invalid display tessellation policy')
  let mesh: Mesh | undefined = root.kind === 'mesh' ? root.data : undefined
  if (root.kind === 'surface') mesh = tessellateNurbsSurface(root.data, { segmentsU: segments, segmentsV: segments })
  if (root.kind === 'brep') mesh = tessellateNurbsBrep(root.data, segments)
  if (root.kind === 'subdivision') mesh = tessellateSubdivision(root.data, subdivisionLevels)
  if (root.kind === 'patches') mesh = tessellateNurbsPatches(root.data, segments)
  let sourceNode = compiled.document.root
  let sourceValue = root
  const rootNode = nodes.get(sourceNode)!
  // Only known display conversions preserve source face correspondence.
  // Boolean/thicken/edit results own their mesh; do not invent CAD lineage.
  if (
    ['tessellate', 'brep_tessellate', 'subdivision_tessellate', 'sdf_tessellate', 'nurbs_patches_tessellate'].includes(
      rootNode.op,
    ) &&
    'input' in rootNode
  ) {
    sourceNode = rootNode.input
    sourceValue = resolver.get(sourceNode)
  }
  if (rootNode.op === 'subdivision')
    sourceValue = { kind: 'subdivision', data: { vertices: rootNode.vertices, faces: rootNode.faces } }
  const boundary =
    rootNode.op === 'tessellate' && (rootNode.trim || rootNode.trim_curves)
      ? {
          trim: rootNode.trim ?? null,
          trimCurves: rootNode.trim_curves
            ? { outer: needCurve(rootNode.trim_curves.outer), holes: rootNode.trim_curves.holes.map(needCurve) }
            : null,
        }
      : null
  const sourceConstruction = resolver.constructionReports[sourceNode]
  const sweepEvidence = extractSweepEvidence(sourceConstruction)
  const replayOwner = resolver.exactMiterPlacements.get(sourceNode)?.body
  const sweepMiterReplay = replayOwner ? readMiterAffineReplayRouting(replayOwner) : null
  const sweepPlanarCapFaces = sourceValue.kind === 'brep' ? readNativeSweepPlanarCapFaces(sourceValue.data) : undefined
  const nativeGeometry = createNativeGeometryArtifact(
    sourceNode,
    sourceValue.kind,
    {
      geometry: sourceValue.data,
      boundary,
      ...(sweepMiterReplay ? { sweepMiterReplay } : {}),
      ...(sweepPlanarCapFaces !== undefined ? { sweepPlanarCapFaces } : {}),
      ...(sourceConstruction && 'stationReconstruction' in sourceConstruction
        ? { sweepStationReconstruction: sourceConstruction.stationReconstruction }
        : {}),
      ...(sourceConstruction && 'sweepMiterReplay' in sourceConstruction && sourceConstruction.sweepMiterReplay
        ? { sweepMiterReplay: sourceConstruction.sweepMiterReplay }
        : {}),
      ...(sweepEvidence ? { sweepEvidence } : {}),
      ...(sourceValue.kind === 'brep' && sourceConstruction && 'bodyBoundaryEvidence' in sourceConstruction
        ? { sweepBodyBoundaryEvidence: sourceConstruction.bodyBoundaryEvidence }
        : {}),
      ...(sourceValue.kind === 'patches' &&
      sourceConstruction &&
      'method' in sourceConstruction &&
      sourceConstruction.method === 'progressive-fourfold-section-refinement'
        ? { sweepPatchEvidence: sourceConstruction }
        : {}),
    },
    compiled.document,
    packNativeGeometrySnapshot,
  )
  return { ...base, ...(mesh ? { mesh } : {}), nativeGeometry }
}

export function buildOwnNurbsResolved(
  document: unknown,
  request: OwnNurbsRequest,
  resolveSweep?: SweepResolver,
  resolveBody?: BodyResolver,
  resolveMiter?: MiterResolver,
  resolveStations?: StationResolver,
) {
  if (!['build', 'evaluate', 'export'].includes(request.action) || (request.evaluations?.length ?? 0) > 64)
    throw new Error('Invalid NURBS request or evaluation budget exceeded.')
  const compiled = compileRushGraphNurbs(document)
  const nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n]))
  const resolver = createOwnNurbsResolver(nodes, request, resolveSweep, resolveBody, resolveMiter, resolveStations)
  const needCurve = (key: string) => {
    const result = resolver.get(key)
    if (result.kind !== 'curve') throw new Error(`Expected curve at ${key}.`)
    return result.data
  }
  const root = resolver.get(compiled.document.root)
  const report = buildNurbsReport(
    compiled,
    root,
    resolver.cache,
    resolver.constructionReports,
    resolver.reconstructionReports,
  )
  const evaluations = (request.evaluations ?? []).map(q => {
    const v = resolver.get(q.node)
    if (v.kind === 'curve') return { node: q.node, ...evaluateNurbsCurve(v.data, q.u) }
    if (v.kind === 'surface' && q.v !== undefined) return { node: q.node, ...evaluateNurbsSurface(v.data, q.u, q.v) }
    throw new Error('Evaluation requires a curve or surface, and v for a surface.')
  })
  const offsetDeviation = request.offsetDeviation
    ? inspectRushGraphOffsetDeviation(request.offsetDeviation, resolver.get)
    : undefined
  const base = {
    ...(offsetDeviation ? { offsetDeviation } : {}),
    ok: true,
    document_sha256: compiled.document_sha256,
    execution_target: 'own-nurbs',
    automatic_fallback: false,
    report,
    evaluations,
  }
  if (request.action === 'export') return exportOwnNurbsArtifact(root, compiled, request, base)
  if (request.action === 'build' && request.display)
    return buildOwnNurbsDisplayResult(compiled, nodes, root, resolver, needCurve, request.display, base)
  return { ...base, ...(request.action === 'build' && root.kind === 'mesh' ? { mesh: root.data } : {}) }
}
