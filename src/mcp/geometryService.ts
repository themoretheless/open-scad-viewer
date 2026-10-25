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
import { estimatedExportBytes, parameterSummary, summarizeResult, validateArtifactLimit, validateCustomizerValue } from './geometryService/boundedText'

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
    super(`Export is estimated at ${estimatedBytes.toLocaleString('en-US')} bytes, above the ${maxBytes.toLocaleString('en-US')}-byte limit`)
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
    super(`Geometry evaluation exceeded its ${deadlineMs.toLocaleString('en-US')} ms deadline`)
    this.name = 'GeometryDeadlineExceededError'
  }
}

export class CustomizerValueError extends RangeError {
  constructor(message: string) {
    super(message)
    this.name = 'CustomizerValueError'
  }
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
        if (!this.runtime) await this.engine.initializeSource(source, { quality, purpose }, signal)
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
