import type { DisplayMesh } from '../../solidDisplayCache'
import {
  finite,
  isNumericSequence,
  vector,
  type MainSolidExpectation,
  type MainSolidResults,
} from '../../mainSolidProtocol'
import type { PolygonMesh } from '../../geometry/polygon'
import type { KernelManifoldMetrics } from '../../geometry/manifoldAnalysis'
import type { MeshBooleanResult, MeshRepairResult } from '../../meshEditing'
import type { ImportedMesh } from '../../../exchange/meshImport'
import type { MeshConvertResult, MeshExportArtifact } from '../../meshConvert'
import { convertedMeshFileName, MESH_IMPORT_FORMATS } from '../../meshFormats'
import type { SurfaceBoundaryReport } from '../../solidSurfaceDiagnostics'
import { arrayOf, numericSequenceOf } from '../mainSolidExpectation'
import { validManifoldRepair, validManifoldReport, validRepairMesh } from './manifoldRepair'
import { isNonnegativeFinite, isSafeIntegerValue, isSafeNonnegativeCount } from './predicates'

export function validSurfaceMeshResult(
  job: Extract<MainSolidExpectation, { kind: 'surfaceMesh' }>,
  value: unknown,
): boolean {
  const m = value as PolygonMesh
  if (
    ![job.u, job.v].every(n => Number.isInteger(n) && n >= 2 && n <= 64) ||
    !isNumericSequence(m.positions) ||
    !isNumericSequence(m.indices)
  ) {
    return false
  }
  const vertices = m.positions.length / 3
  const triangles = m.indices.length / 3
  return (
    Number.isInteger(vertices) &&
    vertices > 0 &&
    vertices <= (job.u + 1) * (job.v + 1) + job.u * job.v &&
    Number.isInteger(triangles) &&
    triangles <= job.u * job.v * 4 &&
    numericSequenceOf(m.positions, m.positions.length, finite) &&
    numericSequenceOf(
      m.indices,
      m.indices.length,
      i => Number.isSafeInteger(i) && Number(i) >= 0 && Number(i) < vertices,
    )
  )
}

export function validSurfaceBoundaryResult(
  job: Extract<MainSolidExpectation, { kind: 'surfaceBoundary' }>,
  value: unknown,
): boolean {
  const v = value as SurfaceBoundaryReport
  const angle = (a: unknown) => a === null || (finite(a) && a >= 0 && a <= 90)
  return (
    Number.isInteger(job.samples) &&
    job.samples >= 2 &&
    job.samples <= 257 &&
    v.samplingOnly === true &&
    isNonnegativeFinite(v.maxGapMm) &&
    angle(v.maxTangentPlaneAngleDeg) &&
    Number.isInteger(v.undefinedNormals) &&
    v.undefinedNormals >= 0 &&
    v.undefinedNormals <= job.samples &&
    Number.isInteger(v.worstGapSample) &&
    v.worstGapSample >= 0 &&
    v.worstGapSample < job.samples &&
    typeof v.sampledWithinTolerance === 'boolean' &&
    arrayOf(v.samples, job.samples, item => {
      const s = item as SurfaceBoundaryReport['samples'][number]
      return (
        !!s &&
        finite(s.t) &&
        s.t >= 0 &&
        s.t <= 1 &&
        vector(s.a) &&
        vector(s.b) &&
        isNonnegativeFinite(s.gapMm) &&
        angle(s.tangentPlaneAngleDeg)
      )
    })
  )
}

export function validManifoldMetricsResult(value: unknown): boolean {
  const v = value as KernelManifoldMetrics
  return (
    [v.vertexCount, v.triangleCount, v.edgeCount, v.componentCount].every(
      isSafeNonnegativeCount,
    ) &&
    isSafeIntegerValue(v.eulerCharacteristic) &&
    finite(v.signedVolume) &&
    isNonnegativeFinite(v.surfaceArea) &&
    typeof v.watertight === 'boolean' &&
    Array.isArray(v.components) &&
    v.components.length === v.componentCount &&
    v.components.every(
      c =>
        [c.vertexCount, c.triangleCount, c.edgeCount, c.boundaryEdges].every(
          isSafeNonnegativeCount,
        ) &&
        isSafeIntegerValue(c.eulerCharacteristic) &&
        (c.genus === null || isSafeNonnegativeCount(c.genus)) &&
        finite(c.signedVolume) &&
        isNonnegativeFinite(c.surfaceArea),
    )
  )
}

export function validMeshManifoldResult(
  job: Extract<MainSolidExpectation, { kind: 'meshManifold' }>,
  value: unknown,
): boolean {
  const entries = value as MainSolidResults['meshManifold']
  return (
    Array.isArray(entries) &&
    entries.length === job.objects.length &&
    entries.every((entry, index) => {
      const object = job.objects[index]
      return (
        !!object &&
        entry?.id === object.id &&
        validManifoldReport(entry.report, object.vertices, object.triangles)
      )
    })
  )
}

export function validMeshRepairResult(
  job: Extract<MainSolidExpectation, { kind: 'meshRepair' }>,
  value: unknown,
): boolean {
  const result = value as MeshRepairResult
  const vertices = result.mesh?.positions.length / 3
  const triangles = result.mesh?.indices.length / 3
  return (
    validRepairMesh(result.mesh) &&
    validManifoldReport(result.before, job.vertices, job.triangles) &&
    validManifoldReport(result.after, vertices, triangles) &&
    (result.repair === null
      ? result.before.isManifold && result.after.isManifold
      : validManifoldRepair(result.repair, result.mesh, result.after))
  )
}

export function validMeshBooleanResult(
  job: Extract<MainSolidExpectation, { kind: 'meshBoolean' }>,
  value: unknown,
): boolean {
  const result = value as MeshBooleanResult
  const report = result.report
  const triangles = result.mesh?.indices.length / 3
  return (
    validRepairMesh(result.mesh) &&
    !!report &&
    report.operation === job.operation &&
    arrayOf(report.operandRepaired, 2, item => typeof item === 'boolean') &&
    isNonnegativeFinite(report.toleranceMm) &&
    isSafeNonnegativeCount(report.work) &&
    isSafeNonnegativeCount(report.fragments) &&
    report.weldEpsilonMm === job.weldEpsilonMm &&
    report.relativeTolerance === job.relativeTolerance &&
    arrayOf(report.inputTriangles, 2, isSafeNonnegativeCount) &&
    report.inputTriangles.every((count, index) => count === job.triangles[index]) &&
    report.outputTriangles === triangles &&
    typeof report.isManifold === 'boolean'
  )
}

export function validMeshImportResult(
  job: Extract<MainSolidExpectation, { kind: 'meshImport' }>,
  value: unknown,
): boolean {
  const result = value as ImportedMesh
  const manifold = result.manifold
  const before = manifold?.before
  const after = manifold?.after
  const repair = manifold?.repair
  const vertices = result.positions?.length / 3
  const triangles = result.indices?.length / 3
  if (
    !validRepairMesh(result) ||
    !manifold ||
    manifold.mode !== job.repair ||
    !before ||
    !after ||
    !(MESH_IMPORT_FORMATS as readonly string[]).includes(result.format) ||
    result.vertexCount !== vertices ||
    result.triangleCount !== triangles ||
    !isSafeNonnegativeCount(result.sourceVertexCount) ||
    result.sourceVertexCount < before.vertexCount ||
    !isSafeNonnegativeCount(result.degenerateTriangles) ||
    !validManifoldReport(before, before.vertexCount, before.triangleCount) ||
    !validManifoldReport(after, vertices, triangles)
  ) {
    return false
  }
  if (repair === null) return job.repair === 'none' || before.isManifold
  const counts = [
    repair.weldedVertices,
    repair.removedDegenerateTriangles,
    repair.flippedTriangles,
    repair.splitVertices,
    repair.filledHoles,
    repair.filledTriangles,
  ]
  return (
    job.repair !== 'none' &&
    !before.isManifold &&
    repair.mode === (job.repair === 'full' ? 1 : 0) &&
    counts.every(isSafeNonnegativeCount) &&
    repair.isManifold === after.isManifold &&
    repair.isManifoldWithBoundary === after.isManifoldWithBoundary
  )
}

export function validMeshConvertResult(
  job: Extract<MainSolidExpectation, { kind: 'meshConvert' }>,
  value: unknown,
): boolean {
  const result = value as MeshConvertResult
  const source = result.source
  const extension = job.target === 'stl_binary' ? 'stl' : job.target
  return (
    result.data instanceof Uint8Array &&
    result.data.byteLength > 0 &&
    typeof result.mimeType === 'string' &&
    result.mimeType.length > 0 &&
    result.format === job.target &&
    result.extension === extension &&
    result.fileName === convertedMeshFileName(job.fileName, extension) &&
    !!source &&
    source.fileName === job.fileName &&
    source.byteLength === job.byteLength &&
    (MESH_IMPORT_FORMATS as readonly string[]).includes(source.format) &&
    [
      source.triangleCount,
      source.vertexCount,
      source.sourceVertexCount,
      source.degenerateTriangles,
    ].every(isSafeNonnegativeCount)
  )
}

export function validMeshExportResult(
  job: Extract<MainSolidExpectation, { kind: 'meshExport' }>,
  value: unknown,
): boolean {
  const result = value as MeshExportArtifact
  const extension = job.target === 'stl_binary' ? 'stl' : job.target
  return (
    result.data instanceof Uint8Array &&
    result.data.byteLength > 0 &&
    typeof result.mimeType === 'string' &&
    result.mimeType.length > 0 &&
    result.extension === extension
  )
}

export function validDisplayMeshResult(
  job: Extract<MainSolidExpectation, { kind: 'displayMesh' }>,
  value: unknown,
): boolean {
  const v = value as DisplayMesh
  const m = v.mesh
  if (
    !m ||
    !isNumericSequence(m.positions) ||
    !isNumericSequence(m.indices) ||
    m.positions.length % 3 ||
    m.indices.length % 3
  ) {
    return false
  }
  const count = m.indices.length / 3
  const vertices = m.positions.length / 3
  return (
    numericSequenceOf(m.positions, m.positions.length, finite) &&
    numericSequenceOf(
      m.indices,
      m.indices.length,
      i => Number.isSafeInteger(i) && Number(i) >= 0 && Number(i) < vertices,
    ) &&
    (m.uv === undefined || numericSequenceOf(m.uv, vertices * 2, finite)) &&
    arrayOf(v.normals, count, vector) &&
    (v.closed == null || arrayOf(v.closed, count, x => typeof x === 'boolean')) &&
    (v.workClosed == null ||
      arrayOf(v.workClosed, job.triangles, x => typeof x === 'boolean')) &&
    (v.map === null
      ? count === job.triangles
      : count <= 4000 &&
        arrayOf(
          v.map,
          count,
          i => Number.isSafeInteger(i) && Number(i) >= 0 && Number(i) < job.triangles,
        ))
  )
}

export function validMeshContactsResult(
  job: Extract<MainSolidExpectation, { kind: 'meshContacts' }>,
  value: unknown,
): boolean {
  const v = value as MainSolidResults['meshContacts']
  if (
    v.scope !== 'display-mesh-all-contacts' ||
    v.relativeTolerance !== 1e-9 ||
    typeof v.complete !== 'boolean' ||
    v.maxWork !== job.maxWork ||
    v.maxContacts !== job.maxContacts ||
    !Number.isInteger(v.work) ||
    v.work < 0 ||
    v.work > job.maxWork ||
    (v.complete ? v.stopReason !== null : !['work-limit', 'contact-limit'].includes(v.stopReason ?? '')) ||
    !Array.isArray(v.contacts) ||
    v.contacts.length > job.maxContacts ||
    !Array.isArray(v.triangleIds) ||
    !Array.isArray(v.lines)
  ) {
    return false
  }
  if (
    !v.contacts.every(
      (c, i) =>
        !!c &&
        arrayOf(
          c.triangles,
          2,
          t => Number.isInteger(t) && Number(t) >= 0 && Number(t) < job.triangles,
        ) &&
        c.triangles[0] < c.triangles[1] &&
        vector(c.point) &&
        finite(c.toleranceMm) &&
        c.toleranceMm > 0 &&
        Number.isInteger(c.sharedVertices) &&
        c.sharedVertices >= 0 &&
        c.sharedVertices <= 3 &&
        (i === 0 ||
          v.contacts[i - 1].triangles[0] < c.triangles[0] ||
          (v.contacts[i - 1].triangles[0] === c.triangles[0] &&
            v.contacts[i - 1].triangles[1] < c.triangles[1])),
    )
  ) {
    return false
  }
  const ids = [...new Set(v.contacts.flatMap(c => c.triangles))].sort((a, b) => a - b)
  return (
    JSON.stringify(v.contact) === JSON.stringify(v.contacts[0] ?? null) &&
    v.triangleIds.length === ids.length &&
    v.triangleIds.every((id, i) => id === ids[i]) &&
    arrayOf(v.lines, ids.length, line => arrayOf(line, 4, vector))
  )
}
