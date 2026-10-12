import type { ArtifactFormat } from '../modelStore'
import { extractCustomizerParameters } from '../../services/scadCustomizer'
import type { CustomizerParameter, CustomizerValue } from '../../services/scadCustomizer'
import { CustomizerValueError, InvalidGeometryError, MAX_ANALYSIS_OBJECTS, MAX_ANALYSIS_OPTIONS, MAX_ANALYSIS_PARAMETERS, MAX_ANALYSIS_SOURCES_PER_OBJECT, MAX_ANALYSIS_TEXT_LENGTH, MAX_ANALYSIS_WARNINGS, MAX_ANALYSIS_WARNING_LENGTH, MAX_ARTIFACT_BYTES } from '../geometryService'
import type { CustomizerParameterSummary, GeometryAnalysis, ObjectAnalysis, SourceSummary } from '../geometryService'
import type { GeometryBuildResult } from '../../services/geometryBuildEngine'
import type { GeometryExecutionDescriptor } from '../../core/geometryExecution'
import type { MeshData, MeshSourceReference } from '../../core/mesh'
import { inspectMesh } from '../../services/meshInspection'

export function boundedText(value: string, maxLength = MAX_ANALYSIS_TEXT_LENGTH): string {
  if (value.length <= maxLength) return value
  let end = maxLength - 1
  const last = value.charCodeAt(end - 1)
  if (last >= 0xD800 && last <= 0xDBFF) end--
  return `${value.slice(0, end)}…`
}

export function validateFiniteGeometry(result: GeometryBuildResult['result']): void {
  if (!Number.isFinite(result.volume) || result.volume < 0) {
    throw new InvalidGeometryError('Compiled geometry has a non-finite or negative volume')
  }
  if (!Number.isFinite(result.surfaceArea) || result.surfaceArea < 0) {
    throw new InvalidGeometryError('Compiled geometry has a non-finite or negative surface area')
  }
  for (const [meshIndex, mesh] of result.meshes.entries()) {
    let transformColorFinite = true
    for (let i = 0; i < mesh.transform.length; i++) {
      if (!Number.isFinite(mesh.transform[i])) { transformColorFinite = false; break }
    }
    if (transformColorFinite) {
      for (let i = 0; i < mesh.color.length; i++) {
        if (!Number.isFinite(mesh.color[i])) { transformColorFinite = false; break }
      }
    }
    if (!transformColorFinite) {
      throw new InvalidGeometryError(`Mesh ${meshIndex} has a non-finite transform or color`)
    }
    const vertexCount = Math.floor(mesh.vertices.length / 6)
    const vertices = mesh.vertices
    const m = mesh.transform
    for (let offset = 0; offset + 5 < vertices.length; offset += 6) {
      const x = vertices[offset]
      const y = vertices[offset + 1]
      const z = vertices[offset + 2]
      if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)
        || !Number.isFinite(vertices[offset + 3]) || !Number.isFinite(vertices[offset + 4])
        || !Number.isFinite(vertices[offset + 5])) {
        throw new InvalidGeometryError(`Mesh ${meshIndex} has a non-finite vertex`)
      }
      // Scalar equivalent of transformPoint(mesh.transform, [x, y, z]) with the
      // same homogeneous-division fallback, checked without allocating Vec3.
      const w = m[12] * x + m[13] * y + m[14] * z + m[15]
      const iw = Number.isFinite(w) && Math.abs(w) > 1e-12 ? 1 / w : 1
      if (!Number.isFinite((m[0] * x + m[1] * y + m[2] * z + m[3]) * iw)
        || !Number.isFinite((m[4] * x + m[5] * y + m[6] * z + m[7]) * iw)
        || !Number.isFinite((m[8] * x + m[9] * y + m[10] * z + m[11]) * iw)) {
        throw new InvalidGeometryError(`Mesh ${meshIndex} has a non-finite world-space vertex`)
      }
    }
    for (const index of mesh.indices) {
      if (index >= vertexCount) {
        throw new InvalidGeometryError(`Mesh ${meshIndex} contains an out-of-range triangle index`)
      }
    }
  }
}

export function stableSourceKey(source: MeshSourceReference): string {
  return source.instanceId ?? source.operationId ?? `${source.id}:${source.start}:${source.end}`
}

export function summarizeSources(mesh: MeshData): { sources: SourceSummary[]; truncated: boolean } {
  const sources = new Map<string, SourceSummary>()
  let truncated = false
  for (const run of mesh.provenance) {
    if (!run.source) continue
    const key = stableSourceKey(run.source)
    const triangles = Math.max(0, run.triangleEnd - run.triangleStart)
    const existing = sources.get(key)
    if (existing) {
      existing.triangles += triangles
      continue
    }
    if (sources.size >= MAX_ANALYSIS_SOURCES_PER_OBJECT) {
      truncated = true
      continue
    }
    const label = boundedText(run.source.label)
    const operationId = run.source.operationId ? boundedText(run.source.operationId) : null
    const instanceId = run.source.instanceId ? boundedText(run.source.instanceId) : null
    if (label !== run.source.label
      || operationId !== (run.source.operationId ?? null)
      || instanceId !== (run.source.instanceId ?? null)) truncated = true
    sources.set(key, {
      label,
      start: run.source.start,
      end: run.source.end,
      operationId,
      instanceId,
      triangles,
    })
  }
  return {
    sources: [...sources.values()].sort((left, right) => left.start - right.start || left.end - right.end),
    truncated,
  }
}

export function boundedCustomizerValue(value: CustomizerValue): CustomizerValue {
  return typeof value === 'string' ? boundedText(value) : value
}

export function parameterSummary(parameter: CustomizerParameter): {
  summary: CustomizerParameterSummary
  truncated: boolean
} {
  const options = parameter.options
    ? parameter.options.slice(0, MAX_ANALYSIS_OPTIONS).map(boundedCustomizerValue)
    : null
  const summary = {
    name: boundedText(parameter.name),
    label: boundedText(parameter.label),
    value: boundedCustomizerValue(parameter.value),
    min: parameter.min ?? null,
    max: parameter.max ?? null,
    step: parameter.step ?? null,
    options,
  }
  return {
    summary,
    truncated: summary.name !== parameter.name
      || summary.label !== parameter.label
      || summary.value !== parameter.value
      || (parameter.options !== undefined && (
        parameter.options.length > MAX_ANALYSIS_OPTIONS
        || parameter.options.some((option, index) => index < MAX_ANALYSIS_OPTIONS && options?.[index] !== option)
      )),
  }
}

export function summarizeResult(
  source: string,
  result: GeometryBuildResult['result'],
  durationMs: number,
  execution: GeometryExecutionDescriptor,
): GeometryAnalysis {
  validateFiniteGeometry(result)
  const inspected = result.meshes.map((mesh, index) => ({ mesh, inspection: inspectMesh(mesh, index) }))
  let detailsTruncated = false
  const objects = inspected.slice(0, MAX_ANALYSIS_OBJECTS).map(({ mesh, inspection }): ObjectAnalysis => {
    const sourceSummary = summarizeSources(mesh)
    const entityId = mesh.entityId ? boundedText(mesh.entityId) : null
    if (sourceSummary.truncated || entityId !== (mesh.entityId ?? null)) detailsTruncated = true
    return {
      index: inspection.index,
      entityId,
      vertices: inspection.vertices,
      triangles: inspection.triangles,
      bounds: inspection.bounds ? {
        min: [...inspection.bounds.min],
        max: [...inspection.bounds.max],
      } : null,
      dimensions: [...inspection.dimensions],
      center: inspection.center ? [...inspection.center] : null,
      color: [...mesh.color],
      topology: { ...mesh.topology },
      sources: sourceSummary.sources,
      sourcesTruncated: sourceSummary.truncated,
    }
  })
  const bounded = inspected.filter(entry => entry.inspection.bounds !== null)
  const bounds = bounded.length ? {
    min: [
      Math.min(...bounded.map(entry => entry.inspection.bounds!.min[0])),
      Math.min(...bounded.map(entry => entry.inspection.bounds!.min[1])),
      Math.min(...bounded.map(entry => entry.inspection.bounds!.min[2])),
    ] as [number, number, number],
    max: [
      Math.max(...bounded.map(entry => entry.inspection.bounds!.max[0])),
      Math.max(...bounded.map(entry => entry.inspection.bounds!.max[1])),
      Math.max(...bounded.map(entry => entry.inspection.bounds!.max[2])),
    ] as [number, number, number],
  } : null
  const topology = inspected.reduce((total, { mesh }) => ({
    boundary: total.boundary + mesh.topology.boundary,
    crease: total.crease + mesh.topology.crease,
    nonManifold: total.nonManifold + mesh.topology.nonManifold,
    degenerate: total.degenerate + mesh.topology.degenerate,
  }), { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 })
  const parameters = extractCustomizerParameters(source)
  const parameterSummaries = parameters.slice(0, MAX_ANALYSIS_PARAMETERS).map(parameterSummary)
  const warnings = result.warnings.slice(0, MAX_ANALYSIS_WARNINGS).map(warning => (
    boundedText(warning, MAX_ANALYSIS_WARNING_LENGTH)
  ))
  detailsTruncated ||= inspected.length > MAX_ANALYSIS_OBJECTS
    || parameters.length > MAX_ANALYSIS_PARAMETERS
    || parameterSummaries.some(parameter => parameter.truncated)
    || result.warnings.length > MAX_ANALYSIS_WARNINGS
    || warnings.some((warning, index) => warning !== result.warnings[index])

  return {
    execution,
    quality: result.quality,
    reduced: result.reduced,
    durationMs,
    meshCount: inspected.length,
    vertexCount: inspected.reduce((sum, entry) => sum + entry.inspection.vertices, 0),
    triangleCount: inspected.reduce((sum, entry) => sum + entry.inspection.triangles, 0),
    volume: result.volume,
    surfaceArea: result.surfaceArea,
    bounds,
    topology,
    warnings,
    parameters: parameterSummaries.map(parameter => parameter.summary),
    parametersTruncated: parameters.length > MAX_ANALYSIS_PARAMETERS
      || parameterSummaries.some(parameter => parameter.truncated),
    objects,
    objectsTruncated: inspected.length > MAX_ANALYSIS_OBJECTS,
    detailsTruncated,
  }
}

export function validateCustomizerValue(parameter: CustomizerParameter, value: CustomizerValue): void {
  if (typeof value !== typeof parameter.value || (typeof value === 'number' && !Number.isFinite(value))) {
    throw new CustomizerValueError(`Parameter ${parameter.name} expects a ${typeof parameter.value}`)
  }
  if (typeof value === 'number') {
    if (parameter.min !== undefined && value < parameter.min) {
      throw new CustomizerValueError(`Parameter ${parameter.name} must be at least ${parameter.min}`)
    }
    if (parameter.max !== undefined && value > parameter.max) {
      throw new CustomizerValueError(`Parameter ${parameter.name} must be at most ${parameter.max}`)
    }
  }
  if (parameter.options && !parameter.options.some(option => Object.is(option, value))) {
    throw new CustomizerValueError(`Parameter ${parameter.name} must be one of its declared options`)
  }
}

export function validateArtifactLimit(maxBytes: number): void {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > MAX_ARTIFACT_BYTES) {
    throw new RangeError(`maxBytes must be an integer between 1 and ${MAX_ARTIFACT_BYTES}`)
  }
}

export function estimatedExportBytes(meshes: readonly MeshData[], format: ArtifactFormat): number {
  const triangles = meshes.reduce((sum, mesh) => sum + Math.floor(mesh.indices.length / 3), 0)
  if (format === 'stl') return 84 + triangles * 50
  const vertices = meshes.reduce((sum, mesh) => sum + Math.floor(mesh.vertices.length / 6), 0)
  // A deliberately conservative bound for the decimal lines emitted by buildObj.
  return 64 + meshes.length * 64 + vertices * 100 + triangles * 50
}
