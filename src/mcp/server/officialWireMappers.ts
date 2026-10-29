// Official reference-runtime wire shapes; the common mapper facade reexports
// these functions so existing server callers retain the same public API.
import type { OfficialArtifactSummary } from '../officialArtifactCache'
import type {
  OfficialOpenScadCapabilities,
  OfficialOpenScadCheckResult,
  OfficialOpenScadExportResult,
  OfficialOpenScadRunOptions,
} from '../officialOpenScadRuntimeService'
import {
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
} from '../officialOpenScadRuntimeProtocol'
import { OFFICIAL_LANGUAGE_SUMMARY, officialArtifactUri } from './contracts'

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
