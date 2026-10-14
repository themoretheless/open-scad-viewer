import { CURRENT_MESH_MANIFEST, OFFICIAL_LANGUAGE_SUMMARY, executionCommonShape, officialRunInputShape, officialToolErrorResultSchema, sourceSelectorFields, toolErrorResultSchema, utf8Encoder } from '../createServer'
import type { OfficialToolRunInput } from '../createServer'
import type { GeometryAnalysis } from '../geometryService'
import type { GeometryEngineRegistrySnapshot, GeometryExecutionDescriptor } from '../../core/geometryExecution'
import type { IndependentArtifactSummary } from '../independentArtifactCache'
import { OFFICIAL_OPENSCAD_EXPORT_FORMATS, OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES, officialOpenScadProjectPathsConflict } from '../officialOpenScadRuntimeProtocol'
import { OPENSCAD_PROJECT_MAX_TOTAL_BYTES } from '../../services/openScadProject'
import type { OfficialArtifactSummary } from '../officialArtifactCache'
import type { OfficialOpenScadCapabilities, OfficialOpenScadCheckResult, OfficialOpenScadExportResult, OfficialOpenScadRunOptions } from '../officialOpenScadRuntimeService'
import { ResourceNotFoundError } from '@modelcontextprotocol/server'
import { validateModelId } from '../modelStore'
import type { StoredModel, StoredModelRevision } from '../modelStore'
import { createHash } from 'node:crypto'
import { staticEngineManifestToWire } from '../engineManifest'
import { z } from 'zod/v4'

export function officialToolInputSchema<T extends z.ZodRawShape>(extra: T) {
  return z.object({ ...officialRunInputShape, ...extra }).strict().superRefine((input, context) => {
    const project = input as unknown as {
      source: string
      files: Array<{ path: string; text?: string; data_base64?: string }>
    }
    const paths = new Set<string>()
    let bytes = utf8Encoder.encode(project.source).byteLength
    for (const [index, file] of project.files.entries()) {
      if ([...paths].some(path => officialOpenScadProjectPathsConflict(path, file.path))) {
        context.addIssue({
          code: 'custom',
          path: ['files', index, 'path'],
          message: 'Project file paths must not duplicate or contain one another',
        })
      }
      paths.add(file.path)
      bytes += file.text !== undefined
        ? utf8Encoder.encode(file.text).byteLength
        : Buffer.from(file.data_base64!, 'base64').byteLength
    }
    if (bytes > OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES) {
      context.addIssue({
        code: 'custom',
        path: ['files'],
        message: `Project must not exceed ${OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES} bytes`,
      })
    }
  })
}

export function meshExecutionSchemaForManifest(
  manifest: typeof CURRENT_MESH_MANIFEST,
) {
  return z.object({
    ...executionCommonShape,
    language_contract: z.literal('legacy/current'),
    engine_class: z.literal(manifest.engineClass),
    engine_key: z.literal(manifest.engineKey),
    kernel_fingerprint: z.literal(manifest.kernelFingerprint),
    semantic_program_version: z.literal(manifest.semanticProgramVersion),
    capability_manifest_version: z.literal(manifest.capabilityManifestVersion),
    manifest_digest: z.literal(manifest.manifestDigest),
    representation: z.literal('mesh'),
    evidence: z.enum(['planned', 'runtime']),
    effective_limits: z.object({
      sourceCharacters: z.literal(250_000),
      triangles: z.literal(750_000),
    }),
  })
}

export function validateIndependentProjectInput(
  input: {
    source: string
    files: Array<{ path: string; text?: string; data_base64?: string }>
  },
  context: z.core.$RefinementCtx,
): void {
  const paths = new Set<string>(['main.scad'])
  let bytes = utf8Encoder.encode(input.source).byteLength
  for (const [index, file] of input.files.entries()) {
    if (paths.has(file.path)) {
      context.addIssue({
        code: 'custom',
        path: ['files', index, 'path'],
        message: file.path === 'main.scad'
          ? 'main.scad is reserved for the source entrypoint'
          : 'Project file paths must be unique',
      })
    }
    paths.add(file.path)
    bytes += file.text !== undefined
      ? utf8Encoder.encode(file.text).byteLength
      : Buffer.from(file.data_base64!, 'base64').byteLength
  }
  if (bytes > OPENSCAD_PROJECT_MAX_TOTAL_BYTES) {
    context.addIssue({
      code: 'custom',
      path: ['files'],
      message: `Project must not exceed ${OPENSCAD_PROJECT_MAX_TOTAL_BYTES} bytes`,
    })
  }
}

export function toolOutputSchema<T extends z.ZodType>(success: T) {
  return z.union([success, toolErrorResultSchema])
}

export function officialToolOutputSchema<T extends z.ZodType>(success: T) {
  return z.union([success, officialToolErrorResultSchema])
}

export function selectorSchema<T extends z.ZodRawShape>(extra: T) {
  return z.union([
    z.object({
      ...extra,
      source: sourceSelectorFields.source.unwrap(),
      model_id: z.never().optional(),
      revision: z.never().optional(),
    }).strict(),
    z.object({
      ...extra,
      source: z.never().optional(),
      model_id: sourceSelectorFields.model_id.unwrap(),
      revision: sourceSelectorFields.revision,
    }).strict(),
  ])
}

export function sha256(data: string | Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

export function modelUri(id: string): string {
  return `openscad://models/${encodeURIComponent(id)}`
}

export function modelRevisionUri(id: string, revision: number): string {
  return `${modelUri(id)}/revisions/${revision}`
}

export function buildUri(id: string): string {
  return `openscad://builds/${encodeURIComponent(id)}`
}

export function artifactUri(id: string): string {
  return `openscad://artifacts/${encodeURIComponent(id)}`
}

export function independentArtifactUri(id: string): string {
  return `openscad://independent-artifacts/${id}`
}

export function officialArtifactUri(id: string): string {
  return `openscad://official-artifacts/${id}`
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
