import type { GeometryQuality } from '../core/build'
import type {
  GeometryBuildPurpose,
  GeometryEngineRegistrySnapshot,
  GeometryExecutionDescriptor,
} from '../core/geometryExecution'
import type { MeshData, MeshSourceReference } from '../core/mesh'
import {
  attachGeometryExecutionToError,
  defaultGeometryBuildEngine,
  GeometryBuildEngine,
  type GeometryBuildResult,
} from '../services/geometryBuildEngine'
import { buildBinaryStl, buildObj } from '../services/meshExport'
import { inspectMesh } from '../services/meshInspection'
import { transformPoint } from '../services/math3d'
import {
  extractCustomizerParameters,
  replaceCustomizerValue,
  type CustomizerParameter,
  type CustomizerValue,
} from '../services/scadCustomizer'
import {
  MAX_STORED_ARTIFACT_BYTES,
  validateModelSource,
  type ArtifactFormat,
} from './modelStore'

export const DEFAULT_MAX_ARTIFACT_BYTES = 4 * 1024 * 1024
export const MAX_ARTIFACT_BYTES = MAX_STORED_ARTIFACT_BYTES
export const MAX_ANALYSIS_OBJECTS = 64
export const MAX_ANALYSIS_SOURCES_PER_OBJECT = 16
export const MAX_ANALYSIS_PARAMETERS = 128
export const MAX_ANALYSIS_OPTIONS = 64
export const MAX_ANALYSIS_WARNINGS = 32
export const MAX_ANALYSIS_TEXT_LENGTH = 256
export const MAX_ANALYSIS_WARNING_LENGTH = 512
export const MAX_PENDING_GEOMETRY_JOBS = 8

let pendingGeometryJobs = 0

export interface SourceSummary {
  label: string
  start: number
  end: number
  operationId: string | null
  instanceId: string | null
  triangles: number
}

export interface ObjectAnalysis {
  index: number
  entityId: string | null
  vertices: number
  triangles: number
  bounds: { min: [number, number, number]; max: [number, number, number] } | null
  dimensions: [number, number, number]
  center: [number, number, number] | null
  color: [number, number, number, number]
  topology: {
    boundary: number
    crease: number
    nonManifold: number
    degenerate: number
  }
  sources: SourceSummary[]
  sourcesTruncated: boolean
}

export interface CustomizerParameterSummary {
  name: string
  label: string
  value: CustomizerValue
  min: number | null
  max: number | null
  step: number | null
  options: CustomizerValue[] | null
}

export interface GeometryAnalysis {
  execution: GeometryExecutionDescriptor
  quality: GeometryQuality
  reduced: boolean
  durationMs: number
  meshCount: number
  vertexCount: number
  triangleCount: number
  volume: number
  surfaceArea: number
  bounds: { min: [number, number, number]; max: [number, number, number] } | null
  topology: {
    boundary: number
    crease: number
    nonManifold: number
    degenerate: number
  }
  warnings: string[]
  parameters: CustomizerParameterSummary[]
  parametersTruncated: boolean
  objects: ObjectAnalysis[]
  objectsTruncated: boolean
  detailsTruncated: boolean
}

export interface CompiledGeometry {
  analysis: GeometryAnalysis
  meshes: MeshData[]
}

export interface GeometryExport {
  analysis: GeometryAnalysis
  data: Uint8Array
  format: ArtifactFormat
  mimeType: string
}

export interface CustomizeResult {
  source: string
  applied: string[]
  parameters: CustomizerParameterSummary[]
  parametersTruncated: boolean
}

export class ArtifactSizeError extends RangeError {
  constructor(readonly estimatedBytes: number, readonly maxBytes: number) {
    super(`Export is estimated at ${estimatedBytes.toLocaleString()} bytes, above the ${maxBytes.toLocaleString()}-byte limit`)
    this.name = 'ArtifactSizeError'
  }
}

export class InvalidGeometryError extends RangeError {
  constructor(message: string) {
    super(message)
    this.name = 'InvalidGeometryError'
  }
}

export class GeometryBusyError extends Error {
  constructor(readonly retryAfterMs = 250) {
    super(`Geometry queue is full; retry after ${retryAfterMs} ms`)
    this.name = 'GeometryBusyError'
  }
}

export class GeometryDeadlineExceededError extends Error {
  constructor(readonly deadlineMs: number) {
    super(`Geometry evaluation exceeded its ${deadlineMs.toLocaleString()} ms deadline`)
    this.name = 'GeometryDeadlineExceededError'
  }
}

export class CustomizerValueError extends RangeError {
  constructor(message: string) {
    super(message)
    this.name = 'CustomizerValueError'
  }
}

function boundedText(value: string, maxLength = MAX_ANALYSIS_TEXT_LENGTH): string {
  if (value.length <= maxLength) return value
  let end = maxLength - 1
  const last = value.charCodeAt(end - 1)
  if (last >= 0xD800 && last <= 0xDBFF) end--
  return `${value.slice(0, end)}…`
}

function validateFiniteGeometry(result: GeometryBuildResult['result']): void {
  if (!Number.isFinite(result.volume) || result.volume < 0) {
    throw new InvalidGeometryError('Compiled geometry has a non-finite or negative volume')
  }
  if (!Number.isFinite(result.surfaceArea) || result.surfaceArea < 0) {
    throw new InvalidGeometryError('Compiled geometry has a non-finite or negative surface area')
  }
  for (const [meshIndex, mesh] of result.meshes.entries()) {
    if ([...mesh.transform, ...mesh.color].some(value => !Number.isFinite(value))) {
      throw new InvalidGeometryError(`Mesh ${meshIndex} has a non-finite transform or color`)
    }
    const vertexCount = Math.floor(mesh.vertices.length / 6)
    for (let offset = 0; offset + 5 < mesh.vertices.length; offset += 6) {
      const values = mesh.vertices.subarray(offset, offset + 6)
      if ([...values].some(value => !Number.isFinite(value))) {
        throw new InvalidGeometryError(`Mesh ${meshIndex} has a non-finite vertex`)
      }
      const world = transformPoint(mesh.transform, [values[0], values[1], values[2]])
      if (!world.every(Number.isFinite)) {
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

function stableSourceKey(source: MeshSourceReference): string {
  return source.instanceId ?? source.operationId ?? `${source.id}:${source.start}:${source.end}`
}

function summarizeSources(mesh: MeshData): { sources: SourceSummary[]; truncated: boolean } {
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

function boundedCustomizerValue(value: CustomizerValue): CustomizerValue {
  return typeof value === 'string' ? boundedText(value) : value
}

function parameterSummary(parameter: CustomizerParameter): {
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

function summarizeResult(
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

function validateCustomizerValue(parameter: CustomizerParameter, value: CustomizerValue): void {
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

function validateArtifactLimit(maxBytes: number): void {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > MAX_ARTIFACT_BYTES) {
    throw new RangeError(`maxBytes must be an integer between 1 and ${MAX_ARTIFACT_BYTES}`)
  }
}

function estimatedExportBytes(meshes: readonly MeshData[], format: ArtifactFormat): number {
  const triangles = meshes.reduce((sum, mesh) => sum + Math.floor(mesh.indices.length / 3), 0)
  if (format === 'stl') return 84 + triangles * 50
  const vertices = meshes.reduce((sum, mesh) => sum + Math.floor(mesh.vertices.length / 6), 0)
  // A deliberately conservative bound for the decimal lines emitted by buildObj.
  return 64 + meshes.length * 64 + vertices * 100 + triangles * 50
}

export interface McpGeometryService {
  capabilities(): Promise<GeometryEngineRegistrySnapshot>
  planSource(
    source: string,
    quality: GeometryQuality,
    purpose: GeometryBuildPurpose,
  ): GeometryExecutionDescriptor
  compile(
    source: string,
    quality?: GeometryQuality,
    signal?: AbortSignal,
    purpose?: GeometryBuildPurpose,
  ): Promise<CompiledGeometry>
  analyze(source: string, quality?: GeometryQuality, signal?: AbortSignal): Promise<GeometryAnalysis>
  export(
    source: string,
    format: ArtifactFormat,
    options?: { maxBytes?: number; signal?: AbortSignal; name?: string },
  ): Promise<GeometryExport>
  customize(source: string, values: Record<string, CustomizerValue>): CustomizeResult
}

/**
 * Optional Node-host execution boundary. The default remains an in-process
 * engine for deterministic unit tests; the production stdio entrypoint injects
 * a disposable-worker runtime so synchronous kernel calls cannot block MCP.
 */
export interface McpGeometryBuildRuntime {
  capabilities(): Promise<GeometryEngineRegistrySnapshot>
  build(
    source: string,
    quality: GeometryQuality,
    purpose: GeometryBuildPurpose,
    signal?: AbortSignal,
  ): Promise<GeometryBuildResult>
}

export class HeadlessGeometryService implements McpGeometryService {
  constructor(
    private readonly engine: GeometryBuildEngine = defaultGeometryBuildEngine,
    private readonly runtime?: McpGeometryBuildRuntime,
  ) {}

  capabilities(): Promise<GeometryEngineRegistrySnapshot> {
    return this.runtime?.capabilities() ?? this.engine.capabilities()
  }

  planSource(
    source: string,
    quality: GeometryQuality,
    purpose: GeometryBuildPurpose,
  ): GeometryExecutionDescriptor {
    return this.engine.planSource(source, { quality, purpose })
  }

  async compile(
    source: string,
    quality: GeometryQuality = 'full',
    signal?: AbortSignal,
    purpose: GeometryBuildPurpose = quality === 'preview' ? 'preview' : 'full',
  ): Promise<CompiledGeometry> {
    validateModelSource(source)
    const plannedExecution = this.engine.planSource(source, { quality, purpose })
    if (signal?.aborted) {
      // AbortSignal.reason belongs to the signal, not to this invocation. A
      // caller may intentionally reuse an already-aborted signal for sources
      // routed to different engines, so throwing/tagging that shared object
      // would leak the first call's execution descriptor into later calls.
      const error = new DOMException('Build cancelled', 'AbortError')
      attachGeometryExecutionToError(error, plannedExecution)
      throw error
    }
    if (pendingGeometryJobs >= MAX_PENDING_GEOMETRY_JOBS) {
      const error = new GeometryBusyError()
      attachGeometryExecutionToError(error, plannedExecution)
      throw error
    }
    pendingGeometryJobs++
    try {
      const startedAt = performance.now()
      let built: GeometryBuildResult
      try {
        built = this.runtime
          ? await this.runtime.build(source, quality, purpose, signal)
          : await this.engine.buildSource(
              source,
              { quality, purpose },
              { shouldAbort: () => signal?.aborted ?? false },
            )
      } catch (error) {
        attachGeometryExecutionToError(error, plannedExecution)
        throw error
      }
      const durationMs = performance.now() - startedAt
      try {
        return {
          analysis: summarizeResult(source, built.result, durationMs, built.execution),
          meshes: built.result.meshes,
        }
      } catch (error) {
        attachGeometryExecutionToError(error, built.execution)
        throw error
      }
    } finally {
      pendingGeometryJobs--
    }
  }

  async analyze(
    source: string,
    quality: GeometryQuality = 'full',
    signal?: AbortSignal,
  ): Promise<GeometryAnalysis> {
    return (await this.compile(source, quality, signal, 'analysis')).analysis
  }

  async export(
    source: string,
    format: ArtifactFormat,
    options: { maxBytes?: number; signal?: AbortSignal; name?: string } = {},
  ): Promise<GeometryExport> {
    const maxBytes = options.maxBytes ?? DEFAULT_MAX_ARTIFACT_BYTES
    validateArtifactLimit(maxBytes)
    const compiled = await this.compile(source, 'full', options.signal, 'export')
    try {
      const estimate = estimatedExportBytes(compiled.meshes, format)
      if (estimate > maxBytes) throw new ArtifactSizeError(estimate, maxBytes)
      const data = format === 'stl'
        ? buildBinaryStl(compiled.meshes, options.name)
        : new TextEncoder().encode(buildObj(compiled.meshes))
      if (data.byteLength > maxBytes) throw new ArtifactSizeError(data.byteLength, maxBytes)
      return {
        analysis: compiled.analysis,
        data,
        format,
        mimeType: format === 'stl' ? 'model/stl' : 'model/obj',
      }
    } catch (error) {
      attachGeometryExecutionToError(error, compiled.analysis.execution)
      throw error
    }
  }

  customize(source: string, values: Record<string, CustomizerValue>): CustomizeResult {
    const routeBefore = this.engine.planSource(source, { quality: 'preview', purpose: 'preview' })
    const parameters = extractCustomizerParameters(source)
    const byName = new Map(parameters.map(parameter => [parameter.name, parameter]))
    const replacements = Object.entries(values).map(([name, value]) => {
      const parameter = byName.get(name)
      if (!parameter) throw new CustomizerValueError(`Unknown Customizer parameter ${name}`)
      validateCustomizerValue(parameter, value)
      return { parameter, value }
    }).sort((left, right) => right.parameter.valueStart - left.parameter.valueStart)

    let updated = source
    for (const replacement of replacements) {
      updated = replaceCustomizerValue(updated, replacement.parameter, replacement.value)
    }
    try {
      validateModelSource(updated)
    } catch (error) {
      if (error instanceof TypeError || error instanceof RangeError) {
        throw new CustomizerValueError(error.message)
      }
      throw error
    }
    const updatedParameters = extractCustomizerParameters(updated)
    const routeAfter = this.engine.planSource(updated, { quality: 'preview', purpose: 'preview' })
    if (routeAfter.languageContract !== routeBefore.languageContract
      || routeAfter.engineClass !== routeBefore.engineClass
      || routeAfter.requiredCapabilities.join('\0') !== routeBefore.requiredCapabilities.join('\0')) {
      throw new CustomizerValueError('Customizer replacements cannot change the source engine-routing contract')
    }
    const summaries = updatedParameters.slice(0, MAX_ANALYSIS_PARAMETERS).map(parameterSummary)
    return {
      source: updated,
      applied: replacements.map(replacement => replacement.parameter.name).sort(),
      parameters: summaries.map(parameter => parameter.summary),
      parametersTruncated: updatedParameters.length > MAX_ANALYSIS_PARAMETERS
        || summaries.some(parameter => parameter.truncated),
    }
  }
}
