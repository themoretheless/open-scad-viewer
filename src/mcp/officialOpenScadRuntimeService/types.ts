import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import type {
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  OfficialOpenScadRuntimeIntegrity,
  OfficialOpenScadRuntimeManifest,
} from '../officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  type OfficialOpenScadBackend,
  type OfficialOpenScadExperimentalFeature,
  type OfficialOpenScadExportFormat,
  type OfficialOpenScadLogs,
} from '../officialOpenScadRuntimeProtocol'

export const MAX_MANIFEST_BYTES = 16 * 1024
export const MAX_RUNTIME_BYTES = 64 * 1024 * 1024
export const MAX_CHILD_STDOUT_BYTES = 9 * 1024 * 1024
export const MAX_CHILD_STDERR_BYTES = 8 * 1024
export const MAX_FONT_BYTES = 2 * 1024 * 1024
export const MAX_FONT_LICENSE_BYTES = 64 * 1024
export const DEFAULT_MAX_CONCURRENT_JOBS = 1
export const MAX_CONCURRENT_JOBS = 4
export const SETUP_COMMAND = 'npm run setup:openscad'
export const FORMAT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPORT_FORMATS)
export const EXPERIMENT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)
export const UTF8 = new TextEncoder()

export interface OfficialOpenScadProjectFileInput {
  readonly path: string
  readonly data: string | Uint8Array
}

export interface OfficialOpenScadRunOptions {
  readonly files?: readonly OfficialOpenScadProjectFileInput[] | undefined
  readonly experimentalFeatures?: readonly OfficialOpenScadExperimentalFeature[] | undefined
  readonly defines?: readonly string[] | undefined
  readonly time?: number | undefined
  readonly backend?: OfficialOpenScadBackend | undefined
  readonly hardWarnings?: boolean | undefined
  readonly checkParameters?: boolean | undefined
  readonly checkParameterRanges?: boolean | undefined
  readonly signal?: AbortSignal | undefined
  readonly timeoutMs?: number | undefined
}

export interface OfficialOpenScadCheckResult {
  readonly runtimeVersion: string
  readonly sourceSha256: string
  readonly csgSha256: string
  readonly csgBytes: number
  readonly durationMs: number
  readonly logs: OfficialOpenScadLogs
  readonly experimentalFeatures: readonly OfficialOpenScadExperimentalFeature[]
}

export interface OfficialOpenScadExportResult {
  readonly runtimeVersion: string
  readonly sourceSha256: string
  readonly format: OfficialOpenScadExportFormat
  readonly mimeType: string
  readonly fileName: string
  readonly data: Uint8Array
  readonly sha256: string
  readonly durationMs: number
  readonly logs: OfficialOpenScadLogs
  readonly experimentalFeatures: readonly OfficialOpenScadExperimentalFeature[]
}

export type OfficialOpenScadUnavailableReason =
  'not-installed' | 'manifest-invalid' | 'runtime-integrity-failed' | 'permission-model-unavailable'

export interface OfficialOpenScadCapabilities {
  readonly available: boolean
  readonly unavailableReason: OfficialOpenScadUnavailableReason | null
  readonly expectedRuntimeVersion: typeof OFFICIAL_OPENSCAD_RUNTIME_VERSION
  readonly runtimeVersion: typeof OFFICIAL_OPENSCAD_RUNTIME_VERSION | null
  readonly archiveSha256: typeof OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256
  readonly runtimeSha256: string | null
  readonly patchVersion: typeof OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION
  readonly defaultFont: {
    readonly family: typeof OFFICIAL_OPENSCAD_FONT_FAMILY
    readonly filename: typeof OFFICIAL_OPENSCAD_FONT_FILENAME
    readonly sha256: string | null
    readonly licenseFilename: typeof OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME
    readonly licenseSha256: string | null
  }
  readonly isolation: {
    readonly filesystem: 'MEMFS'
    readonly nodePermissionModel: true
    readonly scadHostFileRead: false
    readonly scadHostFileWrite: false
    readonly networkApiExposed: false
    readonly networkSandboxEnforced: false
    readonly wasmMemoryLimitEnforced: false
    readonly processModel: 'one-shot'
  }
  readonly formats: readonly OfficialOpenScadExportFormat[]
  readonly experimentalFeatures: readonly OfficialOpenScadExperimentalFeature[]
  readonly defaults: {
    readonly backend: 'Manifold'
    readonly hardWarnings: true
    readonly checkParameters: true
    readonly checkParameterRanges: true
    readonly experimentsEnabled: false
  }
  readonly limits: {
    readonly sourceBytes: number
    readonly projectFiles: number
    readonly projectFileBytes: number
    readonly projectBytes: number
    readonly wireRequestBytes: number
    readonly outputBytes: number
    readonly logBytes: number
    readonly logEntries: number
    readonly timeoutMs: number
  }
  readonly setupCommand: typeof SETUP_COMMAND
}

export interface OfficialOpenScadRuntimeService {
  capabilities(): Promise<OfficialOpenScadCapabilities>
  check(source: string, options?: OfficialOpenScadRunOptions): Promise<OfficialOpenScadCheckResult>
  export(
    source: string,
    format: OfficialOpenScadExportFormat,
    options?: OfficialOpenScadRunOptions,
  ): Promise<OfficialOpenScadExportResult>
  close(): Promise<void>
}

export type OfficialOpenScadSupervisorErrorCode =
  | 'E_OFFICIAL_OPENSCAD_UNAVAILABLE'
  | 'E_OFFICIAL_OPENSCAD_BUSY'
  | 'E_OFFICIAL_OPENSCAD_CANCELLED'
  | 'E_OFFICIAL_OPENSCAD_DEADLINE'
  | 'E_OFFICIAL_OPENSCAD_PROTOCOL'
  | 'E_OFFICIAL_OPENSCAD_REQUEST_LIMIT'
  | 'E_OFFICIAL_OPENSCAD_CHILD_CRASH'
  | 'E_OFFICIAL_OPENSCAD_CLOSED'

export class OfficialOpenScadSupervisorError extends Error {
  constructor(
    readonly code: OfficialOpenScadSupervisorErrorCode,
    message: string,
    readonly jobId: number | null,
    options: { cause?: unknown } = {},
  ) {
    super(message, options)
    this.name = 'OfficialOpenScadSupervisorError'
  }
}

export class OfficialOpenScadRemoteError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly jobId: number,
    readonly logs: OfficialOpenScadLogs,
    readonly durationMs: number,
  ) {
    super(message)
    this.name = 'OfficialOpenScadRemoteError'
  }
}

export interface OfficialOpenScadRuntimeSupervisorOptions {
  readonly cacheRoot?: string
  readonly runnerPath?: string
  readonly maxConcurrentJobs?: number
  readonly childFactory?: OfficialOpenScadChildFactory
  /** Test seam for synthetic runtime fixtures. Production callers must omit this. */
  readonly testOnlyExpectedIntegrity?: OfficialOpenScadRuntimeIntegrity
}

export interface OfficialOpenScadChildFactoryInput {
  readonly runnerPath: string
  readonly runtimePath: string
  readonly fontPath: string
  readonly fontSha256: string
}

export type OfficialOpenScadChildFactory = (input: OfficialOpenScadChildFactoryInput) => ChildProcessWithoutNullStreams

export interface OfficialOpenScadSupervisorSnapshot {
  readonly activeJobs: number
  readonly jobsStarted: number
  readonly jobsJoined: number
  readonly closing: boolean
  readonly closed: boolean
}

export interface VerifiedInstallation {
  readonly manifest: OfficialOpenScadRuntimeManifest
  readonly runtimePath: string
  readonly fontPath: string
}

export interface ActiveChild {
  readonly child: ChildProcessWithoutNullStreams
  readonly joined: Promise<void>
  stopReason: 'cancelled' | 'deadline' | 'closed' | 'output-limit' | null
}

export type NodePermissionFlag = '--permission' | '--experimental-permission'
