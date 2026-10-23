import { ResourceNotFoundError } from '@modelcontextprotocol/server'
import type {
  GeometryEngineRegistrySnapshot,
  GeometryExecutionDescriptor,
} from '../../core/geometryExecution'
import {
  geometryExecutionForError,
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
} from '../../services/geometryBuildEngine'
import { buildObj } from '../../services/meshExport'
import type { GeometryAnalysis } from '../geometryService'
import type {
  IndependentArtifactFormat,
  IndependentArtifactSummary,
} from '../independentArtifactCache'
import { staticEngineManifestToWire } from '../engineManifest'
import {
  validateModelId,
  type ModelStore,
  type StoredBuild,
  type StoredModel,
  type StoredModelRevision,
} from '../modelStore'
import {
  ModelNotFoundError,
  publicToolError,
} from '../publicError'
import type { OfficialArtifactSummary } from '../officialArtifactCache'
import {
  OfficialOpenScadRemoteError,
  type OfficialOpenScadCapabilities,
  type OfficialOpenScadCheckResult,
  type OfficialOpenScadExportResult,
  type OfficialOpenScadRunOptions,
} from '../officialOpenScadRuntimeService'
import {
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
} from '../officialOpenScadRuntimeProtocol'
import {
  buildUri,
  independentArtifactUri,
  modelRevisionUri,
  modelUri,
  OFFICIAL_LANGUAGE_SUMMARY,
  officialArtifactUri,
  sha256,
  truncateWellFormed,
} from './contracts'

export interface OfficialToolRunInput {
  source: string
  files: Array<{
    path: string
    text?: string
    data_base64?: string
  }>
  experimental_features: Array<typeof OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES[number]>
  defines: string[]
  time?: number
  backend: 'Manifold' | 'CGAL'
  hard_warnings: boolean
  check_parameters: boolean
  check_parameter_ranges: boolean
  timeout_ms: number
}

export function officialRunOptions(
  input: OfficialToolRunInput,
  signal: AbortSignal | undefined,
): OfficialOpenScadRunOptions {
  return {
    files: input.files.map(file => ({
      path: file.path,
      data: file.text !== undefined
        ? file.text
        : new Uint8Array(Buffer.from(file.data_base64!, 'base64')),
    })),
    experimentalFeatures: input.experimental_features,
    defines: input.defines,
    time: input.time,
    backend: input.backend,
    hardWarnings: input.hard_warnings,
    checkParameters: input.check_parameters,
    checkParameterRanges: input.check_parameter_ranges,
    timeoutMs: input.timeout_ms,
    signal,
  }
}

export function officialCapabilitiesToWire(capabilities: OfficialOpenScadCapabilities) {
  return {
    provider_role: 'qualification-oracle' as const,
    available: capabilities.available,
    unavailable_reason: capabilities.unavailableReason,
    expected_runtime_version: capabilities.expectedRuntimeVersion,
    runtime_version: capabilities.runtimeVersion,
    archive_sha256: capabilities.archiveSha256,
    runtime_sha256: capabilities.runtimeSha256,
    patch_version: capabilities.patchVersion,
    default_font: {
      family: capabilities.defaultFont.family,
      filename: capabilities.defaultFont.filename,
      sha256: capabilities.defaultFont.sha256,
      license_filename: capabilities.defaultFont.licenseFilename,
      license_sha256: capabilities.defaultFont.licenseSha256,
    },
    isolation: {
      filesystem: capabilities.isolation.filesystem,
      node_permission_model: capabilities.isolation.nodePermissionModel,
      scad_host_file_read: capabilities.isolation.scadHostFileRead,
      scad_host_file_write: capabilities.isolation.scadHostFileWrite,
      network_api_exposed: capabilities.isolation.networkApiExposed,
      network_sandbox_enforced: capabilities.isolation.networkSandboxEnforced,
      wasm_memory_limit_enforced: capabilities.isolation.wasmMemoryLimitEnforced,
      process_model: capabilities.isolation.processModel,
    },
    formats: capabilities.formats,
    experimental_features: capabilities.experimentalFeatures,
    defaults: {
      backend: capabilities.defaults.backend,
      hard_warnings: capabilities.defaults.hardWarnings,
      check_parameters: capabilities.defaults.checkParameters,
      check_parameter_ranges: capabilities.defaults.checkParameterRanges,
      experiments_enabled: capabilities.defaults.experimentsEnabled,
    },
    limits: {
      source_bytes: capabilities.limits.sourceBytes,
      project_files: capabilities.limits.projectFiles,
      project_file_bytes: capabilities.limits.projectFileBytes,
      project_bytes: capabilities.limits.projectBytes,
      wire_request_bytes: capabilities.limits.wireRequestBytes,
      output_bytes: capabilities.limits.outputBytes,
      log_bytes: capabilities.limits.logBytes,
      log_entries: capabilities.limits.logEntries,
      timeout_ms: capabilities.limits.timeoutMs,
    },
    setup_command: capabilities.setupCommand,
  }
}

export function officialCheckToWire(result: OfficialOpenScadCheckResult) {
  return {
    provider_role: 'qualification-oracle' as const,
    language_contract: OFFICIAL_LANGUAGE_SUMMARY,
    runtime_version: result.runtimeVersion,
    source_sha256: result.sourceSha256,
    csg_sha256: result.csgSha256,
    csg_bytes: result.csgBytes,
    duration_ms: result.durationMs,
    logs: result.logs,
    experimental_features: result.experimentalFeatures,
  }
}

export function officialExportToWire(result: OfficialOpenScadExportResult) {
  return {
    provider_role: 'qualification-oracle' as const,
    language_contract: OFFICIAL_LANGUAGE_SUMMARY,
    runtime_version: result.runtimeVersion,
    source_sha256: result.sourceSha256,
    format: result.format,
    duration_ms: result.durationMs,
    logs: result.logs,
    experimental_features: result.experimentalFeatures,
  }
}

export function officialArtifactToWire(
  artifact: OfficialArtifactSummary,
  format: typeof OFFICIAL_OPENSCAD_EXPORT_FORMATS[number],
) {
  return {
    id: artifact.id,
    resource_uri: officialArtifactUri(artifact.id),
    file_name: artifact.fileName,
    format,
    mime_type: artifact.mimeType,
    sha256: artifact.sha256,
    byte_length: artifact.byteLength,
  }
}

export function independentArtifactToWire(artifact: IndependentArtifactSummary) {
  return {
    id: artifact.id,
    resource_uri: independentArtifactUri(artifact.id),
    file_name: artifact.fileName,
    format: artifact.format,
    mime_type: artifact.mimeType,
    sha256: artifact.sha256,
    byte_length: artifact.byteLength,
  }
}

export function resourceVariable(uri: URL, value: string | string[] | undefined): string {
  if (typeof value !== 'string') throw new ResourceNotFoundError(uri.href)
  try {
    return decodeURIComponent(value)
  } catch {
    throw new ResourceNotFoundError(uri.href)
  }
}

export function resourceId(uri: URL, value: string | string[] | undefined): string {
  const id = resourceVariable(uri, value)
  try { validateModelId(id) } catch { throw new ResourceNotFoundError(uri.href) }
  return id
}

export function modelToWire(model: StoredModel) {
  return {
    id: model.id,
    name: model.name,
    revision: model.revision,
    created_at: model.createdAt,
    updated_at: model.updatedAt,
    resource_uri: modelUri(model.id),
  }
}

export function modelRevisionToWire(model: StoredModelRevision) {
  return {
    id: model.id,
    name: model.name,
    revision: model.revision,
    created_at: model.createdAt,
    updated_at: model.createdAt,
    resource_uri: modelRevisionUri(model.id, model.revision),
  }
}

export function executionToWire(execution: GeometryExecutionDescriptor) {
  return {
    language_contract: execution.languageContract,
    required_capabilities: execution.requiredCapabilities,
    engine_class: execution.engineClass,
    engine_key: execution.engineKey,
    kernel_fingerprint: execution.kernelFingerprint,
    semantic_program_version: execution.semanticProgramVersion,
    capability_manifest_version: execution.capabilityManifestVersion,
    manifest_digest: execution.manifestDigest,
    purpose: execution.purpose,
    quality: execution.quality,
    representation: execution.representation,
    evidence: execution.evidence,
    effective_limits: execution.effectiveLimits,
    automatic_fallback: execution.automaticFallback,
  }
}

export function engineRegistryToWire(registry: GeometryEngineRegistrySnapshot) {
  return {
    contract_version: registry.contractVersion,
    source_directed_routing: registry.sourceDirectedRouting,
    automatic_fallback: registry.automaticFallback,
    routes: registry.routes.map(route => ({
      language_contract: route.languageContract,
      engine_class: route.engineClass,
      fallback: route.fallback,
    })),
    engines: registry.engines.map(engine => ({
      ...staticEngineManifestToWire(engine),
      availability: engine.availability,
      unavailable_reason: engine.unavailableReason,
    })),
  }
}

export function analysisToWire(analysis: GeometryAnalysis) {
  return {
    execution: executionToWire(analysis.execution),
    quality: analysis.quality,
    reduced: analysis.reduced,
    duration_ms: analysis.durationMs,
    mesh_count: analysis.meshCount,
    vertex_count: analysis.vertexCount,
    triangle_count: analysis.triangleCount,
    volume: analysis.volume,
    surface_area: analysis.surfaceArea,
    bounds: analysis.bounds,
    topology: {
      boundary: analysis.topology.boundary,
      crease: analysis.topology.crease,
      non_manifold: analysis.topology.nonManifold,
      degenerate: analysis.topology.degenerate,
    },
    warnings: analysis.warnings,
    parameters: analysis.parameters,
    parameters_truncated: analysis.parametersTruncated,
    objects: analysis.objects.map(object => ({
      index: object.index,
      entity_id: object.entityId,
      vertices: object.vertices,
      triangles: object.triangles,
      bounds: object.bounds,
      dimensions: object.dimensions,
      center: object.center,
      color: object.color,
      topology: {
        boundary: object.topology.boundary,
        crease: object.topology.crease,
        non_manifold: object.topology.nonManifold,
        degenerate: object.topology.degenerate,
      },
      sources: object.sources.map(source => ({
        label: source.label,
        start: source.start,
        end: source.end,
        operation_id: source.operationId,
        instance_id: source.instanceId,
        triangles: source.triangles,
      })),
      sources_truncated: object.sourcesTruncated,
    })),
    objects_truncated: analysis.objectsTruncated,
    details_truncated: analysis.detailsTruncated,
  }
}

export function buildToWire(build: StoredBuild) {
  return {
    id: build.id,
    model_id: build.modelId,
    model_revision: build.modelRevision,
    source_sha256: build.sourceSha256,
    source_attestation: build.sourceAttestation,
    execution: executionToWire(build.execution),
    quality: build.quality,
    status: build.status,
    duration_ms: build.durationMs,
    warnings: build.warnings,
    metrics: build.metrics ? {
      mesh_count: build.metrics.meshCount,
      triangle_count: build.metrics.triangleCount,
      volume: build.metrics.volume,
      surface_area: build.metrics.surfaceArea,
      reduced: build.metrics.reduced,
    } : null,
    error: build.error,
    created_at: build.createdAt,
    resource_uri: buildUri(build.id),
  }
}

export function textResult<T extends object>(structuredContent: T) {
  const serialized = JSON.stringify(structuredContent)
  return {
    content: [{ type: 'text' as const, text: serialized }],
    structuredContent: structuredContent as Record<string, unknown>,
  }
}

export function executionForFailure(error: unknown): GeometryExecutionDescriptor | undefined {
  return geometryExecutionForError(error)
    ?? (error instanceof GeometryEngineUnavailableError
    || error instanceof GeometryCapabilityUnavailableError
    ? error.execution
    : undefined)
}

export function errorResult(error: unknown, extra: Record<string, unknown> = {}) {
  const problem = publicToolError(error)
  if (problem.internal) {
    console.error(`MCP tool failure (${problem.error.correlation_id}):`, error)
  }
  const execution = executionForFailure(error)
  const structured = {
    ...extra,
    ...(execution && extra.execution === undefined
      ? { execution: executionToWire(execution) }
      : {}),
    error: problem.error,
  }
  return {
    content: [{ type: 'text' as const, text: JSON.stringify(structured, null, 2) }],
    structuredContent: structured,
    isError: true as const,
  }
}

export function officialErrorResult(error: unknown) {
  const exposesCompilerDiagnostics = error instanceof OfficialOpenScadRemoteError
    && (error.code === 'E_OPENSCAD_COMPILE'
      || error.code === 'E_OPENSCAD_NO_OUTPUT'
      || error.code === 'E_OPENSCAD_OUTPUT_LIMIT')
  return exposesCompilerDiagnostics
    ? errorResult(error, {
        official_logs: error.logs,
        duration_ms: error.durationMs,
      })
    : errorResult(error)
}

export interface SelectedModel {
  id: string
  name: string
  source: string
  revision: number
}

export async function resolveSource(
  store: ModelStore,
  input: { source?: string; model_id?: string; revision?: number },
): Promise<{ source: string; model: SelectedModel | null }> {
  if (input.source !== undefined) return { source: input.source, model: null }
  const model = input.revision === undefined
    ? await store.getModel(input.model_id!)
    : await store.getModelRevision(input.model_id!, input.revision)
  if (!model) throw new ModelNotFoundError(input.model_id!, input.revision)
  return { source: model.source, model }
}

export function analysisDimensions(analysis: GeometryAnalysis): [number, number, number] {
  if (!analysis.bounds) return [0, 0, 0]
  return analysis.bounds.max.map((value, index) => (
    value - analysis.bounds!.min[index]
  )) as [number, number, number]
}

export function comparisonSide(
  selected: Awaited<ReturnType<typeof resolveSource>>,
  analysis: GeometryAnalysis,
) {
  return {
    source_sha256: sha256(selected.source),
    model_id: selected.model?.id ?? null,
    model_revision: selected.model?.revision ?? null,
    execution: executionToWire(analysis.execution),
    mesh_count: analysis.meshCount,
    triangle_count: analysis.triangleCount,
    volume: analysis.volume,
    surface_area: analysis.surfaceArea,
    bounds: analysis.bounds,
    dimensions: analysisDimensions(analysis),
    topology: {
      boundary: analysis.topology.boundary,
      crease: analysis.topology.crease,
      non_manifold: analysis.topology.nonManifold,
      degenerate: analysis.topology.degenerate,
    },
    warnings: analysis.warnings,
  }
}

export function comparisonDelta(
  left: ReturnType<typeof comparisonSide>,
  right: ReturnType<typeof comparisonSide>,
) {
  return {
    mesh_count: right.mesh_count - left.mesh_count,
    triangle_count: right.triangle_count - left.triangle_count,
    volume: right.volume - left.volume,
    surface_area: right.surface_area - left.surface_area,
    dimensions: right.dimensions.map((value, index) => (
      value - left.dimensions[index]
    )) as [number, number, number],
    topology: {
      boundary: right.topology.boundary - left.topology.boundary,
      crease: right.topology.crease - left.topology.crease,
      non_manifold: right.topology.non_manifold - left.topology.non_manifold,
      degenerate: right.topology.degenerate - left.topology.degenerate,
    },
  }
}

export function sameExecutionContext(
  left: GeometryExecutionDescriptor,
  right: GeometryExecutionDescriptor,
): boolean {
  const stableLimits = (limits: Record<string, number>) => JSON.stringify(
    Object.entries(limits).sort(([leftKey], [rightKey]) => leftKey.localeCompare(rightKey)),
  )
  return left.languageContract === right.languageContract
    && left.engineClass === right.engineClass
    && left.engineKey === right.engineKey
    && left.kernelFingerprint === right.kernelFingerprint
    && left.semanticProgramVersion === right.semanticProgramVersion
    && left.capabilityManifestVersion === right.capabilityManifestVersion
    && left.manifestDigest === right.manifestDigest
    && left.requiredCapabilities.join('\0') === right.requiredCapabilities.join('\0')
    && left.purpose === right.purpose
    && left.quality === right.quality
    && left.representation === right.representation
    && left.automaticFallback === right.automaticFallback
    && stableLimits(left.effectiveLimits) === stableLimits(right.effectiveLimits)
}

export function safeFileName(name: string, format: 'stl' | 'obj'): string {
  const extension = `.${format}`
  const trimmed = name.trim().replace(/[/\\\0]/g, '_') || `model${extension}`
  const base = trimmed.toLowerCase().endsWith(extension) ? trimmed.slice(0, -extension.length) : trimmed
  return `${truncateWellFormed(base, 255 - extension.length)}${extension}`
}

export function estimatedIndependentExportBytes(
  meshes: Parameters<typeof buildObj>[0],
  format: IndependentArtifactFormat,
): number {
  const triangles = meshes.reduce(
    (total, mesh) => total + Math.floor(mesh.indices.length / 3),
    0,
  )
  if (format === 'stl') return 84 + triangles * 50
  const vertices = meshes.reduce(
    (total, mesh) => total + Math.floor(mesh.vertices.length / 6),
    0,
  )
  return 64 + meshes.length * 64 + vertices * 100 + triangles * 50
}

export function throwIfIndependentExportAborted(signal: AbortSignal | undefined): void {
  if (signal?.aborted) throw new DOMException('OpenSCAD export was cancelled', 'AbortError')
}
