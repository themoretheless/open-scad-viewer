import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { lstat, readFile, realpath, stat } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { defaultCacheRoot } from './officialRuntimeCacheRoot'
import {
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  OFFICIAL_OPENSCAD_PINNED_INTEGRITY,
  isOfficialOpenScadRuntimeManifest,
  matchesOfficialOpenScadRuntimeIntegrity,
  sha256Buffer,
  type OfficialOpenScadRuntimeIntegrity,
  type OfficialOpenScadRuntimeManifest,
} from './officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS,
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES,
  OFFICIAL_OPENSCAD_MAX_DEFINES,
  OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
  OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
  OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
  OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES,
  OFFICIAL_OPENSCAD_PROTOCOL_VERSION,
  isOfficialOpenScadWireRequest,
  isOfficialOpenScadWireTerminal,
  isWellFormedOfficialOpenScadText,
  normalizeOfficialOpenScadProjectPath,
  officialOpenScadProjectPathsConflict,
  type OfficialOpenScadBackend,
  type OfficialOpenScadExperimentalFeature,
  type OfficialOpenScadExportFormat,
  type OfficialOpenScadLogs,
  type OfficialOpenScadWireRequest,
} from './officialOpenScadRuntimeProtocol'
import { boundedInteger, buildRequest, capabilitiesShape, classifyUnavailable, compactChildError, defaultChildFactory, defaultRunnerPath, installationSignature, permissionModelAvailable, unavailableCapabilities, verifyInstallation } from './officialOpenScadRuntimeService/defaultRunnerPath'

export const MAX_MANIFEST_BYTES = 16 * 1024
export const MAX_RUNTIME_BYTES = 64 * 1024 * 1024
const MAX_CHILD_STDOUT_BYTES = 9 * 1024 * 1024
const MAX_CHILD_STDERR_BYTES = 8 * 1024
export const MAX_FONT_BYTES = 2 * 1024 * 1024
export const MAX_FONT_LICENSE_BYTES = 64 * 1024
const DEFAULT_MAX_CONCURRENT_JOBS = 1
const MAX_CONCURRENT_JOBS = 4
export const SETUP_COMMAND = 'npm run setup:openscad'
const FORMAT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPORT_FORMATS)
export const EXPERIMENT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)
export const UTF8 = new TextEncoder()

export interface OfficialOpenScadProjectFileInput {
  readonly path: string
  readonly data: string | Uint8Array
}

export interface OfficialOpenScadRunOptions {
  readonly files?: readonly OfficialOpenScadProjectFileInput[]
  readonly experimentalFeatures?: readonly OfficialOpenScadExperimentalFeature[]
  readonly defines?: readonly string[]
  readonly time?: number
  readonly backend?: OfficialOpenScadBackend
  readonly hardWarnings?: boolean
  readonly checkParameters?: boolean
  readonly checkParameterRanges?: boolean
  readonly signal?: AbortSignal
  readonly timeoutMs?: number
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
  | 'not-installed'
  | 'manifest-invalid'
  | 'runtime-integrity-failed'
  | 'permission-model-unavailable'

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

export type OfficialOpenScadChildFactory =
  (input: OfficialOpenScadChildFactoryInput) => ChildProcessWithoutNullStreams

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

interface ActiveChild {
  readonly child: ChildProcessWithoutNullStreams
  readonly joined: Promise<void>
  stopReason: 'cancelled' | 'deadline' | 'closed' | 'output-limit' | null
}

export type NodePermissionFlag = '--permission' | '--experimental-permission'

/**
 * Production boundary for the opt-in official runtime. Every call uses a new
 * permission-model process and resolves only after that process has joined.
 */
export class OfficialOpenScadRuntimeSupervisor implements OfficialOpenScadRuntimeService {
  private readonly cacheRoot: string
  private readonly runnerPath: string
  private readonly maxConcurrentJobs: number
  private readonly childFactory: OfficialOpenScadChildFactory
  private readonly expectedIntegrity: OfficialOpenScadRuntimeIntegrity
  private readonly active = new Map<number, ActiveChild>()
  private admittedJobs = 0
  private nextJobId = 1
  private jobsStarted = 0
  private jobsJoined = 0
  private closing = false
  private closed = false
  private closePromise: Promise<void> | null = null
  private verifiedInstallationCache: {
    signature: string
    paths: string[]
    installation: VerifiedInstallation
  } | null = null

  constructor(options: OfficialOpenScadRuntimeSupervisorOptions = {}) {
    this.cacheRoot = resolve(options.cacheRoot ?? defaultCacheRoot())
    this.runnerPath = resolve(options.runnerPath ?? defaultRunnerPath())
    this.maxConcurrentJobs = boundedInteger(
      options.maxConcurrentJobs ?? DEFAULT_MAX_CONCURRENT_JOBS,
      'maxConcurrentJobs',
      1,
      MAX_CONCURRENT_JOBS,
    )
    this.childFactory = options.childFactory ?? defaultChildFactory
    this.expectedIntegrity = options.testOnlyExpectedIntegrity ?? OFFICIAL_OPENSCAD_PINNED_INTEGRITY
  }

  snapshot(): OfficialOpenScadSupervisorSnapshot {
    return Object.freeze({
      activeJobs: this.active.size,
      jobsStarted: this.jobsStarted,
      jobsJoined: this.jobsJoined,
      closing: this.closing,
      closed: this.closed,
    })
  }

  /**
   * Memoized installation verification. The expensive SHA-256 pass over the
   * runtime and font files runs once per supervisor; later calls only re-stat
   * the verified files and fall back to full verification when any of them
   * changed (size/mtime) or disappeared. Failed verifications are never
   * cached, and the cacheRoot is fixed per supervisor, so a cache can never
   * outlive the installation it verified.
   */
  private async verifiedInstallationMemoized(): Promise<VerifiedInstallation> {
    const cached = this.verifiedInstallationCache
    if (cached && (await installationSignature(cached.paths)) === cached.signature) {
      return cached.installation
    }
    this.verifiedInstallationCache = null
    const installation = await verifyInstallation(this.cacheRoot, this.expectedIntegrity)
    const paths = [
      join(this.cacheRoot, OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME),
      installation.runtimePath,
      installation.fontPath,
      join(this.cacheRoot, installation.manifest.fontLicenseFilename),
    ]
    const signature = await installationSignature(paths)
    if (signature !== null) this.verifiedInstallationCache = { signature, paths, installation }
    return installation
  }

  async capabilities(): Promise<OfficialOpenScadCapabilities> {
    if (!permissionModelAvailable()) return unavailableCapabilities('permission-model-unavailable')
    try {
      const installation = await this.verifiedInstallationMemoized()
      return capabilitiesShape(
        true,
        null,
        installation.manifest.runtimeVersion,
        installation.manifest.runtimeSha256,
        installation.manifest.fontSha256,
        installation.manifest.fontLicenseSha256,
      )
    } catch (error) {
      return unavailableCapabilities(classifyUnavailable(error))
    }
  }

  async check(
    source: string,
    options: OfficialOpenScadRunOptions = {},
  ): Promise<OfficialOpenScadCheckResult> {
    const jobId = this.reserveJobId()
    try {
      const request = buildRequest(jobId, 'check', source, null, options)
      const terminal = await this.run(request, options)
      return Object.freeze({
        runtimeVersion: terminal.runtimeVersion,
        sourceSha256: terminal.sourceSha256,
        csgSha256: terminal.output.sha256,
        csgBytes: terminal.output.byteLength,
        durationMs: terminal.durationMs,
        logs: terminal.logs,
        experimentalFeatures: request.options.experimentalFeatures,
      })
    } finally {
      this.admittedJobs -= 1
    }
  }

  async export(
    source: string,
    format: OfficialOpenScadExportFormat,
    options: OfficialOpenScadRunOptions = {},
  ): Promise<OfficialOpenScadExportResult> {
    if (typeof format !== 'string' || !FORMAT_SET.has(format)) {
      throw new TypeError(`Unsupported official OpenSCAD export format: ${String(format)}`)
    }
    const jobId = this.reserveJobId()
    try {
      const request = buildRequest(jobId, 'export', source, format, options)
      const terminal = await this.run(request, options)
      const data = Buffer.from(terminal.output.dataBase64!, 'base64')
      return Object.freeze({
        runtimeVersion: terminal.runtimeVersion,
        sourceSha256: terminal.sourceSha256,
        format,
        mimeType: terminal.output.mimeType,
        fileName: `model.${format}`,
        data: new Uint8Array(data),
        sha256: terminal.output.sha256,
        durationMs: terminal.durationMs,
        logs: terminal.logs,
        experimentalFeatures: request.options.experimentalFeatures,
      })
    } finally {
      this.admittedJobs -= 1
    }
  }

  close(): Promise<void> {
    this.closePromise ??= (async () => {
      this.closing = true
      const active = [...this.active.values()]
      for (const child of active) {
        child.stopReason ??= 'closed'
        child.child.kill('SIGKILL')
      }
      await Promise.allSettled(active.map(child => child.joined))
      this.closed = true
      this.closing = false
    })()
    return this.closePromise
  }

  private reserveJobId(): number {
    if (this.closing || this.closed) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_CLOSED',
        'Official OpenSCAD runtime supervisor is closed',
        null,
      )
    }
    if (this.admittedJobs >= this.maxConcurrentJobs) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_BUSY',
        'Official OpenSCAD runtime supervisor is at its concurrency limit',
        null,
      )
    }
    const jobId = this.nextJobId++
    if (!Number.isSafeInteger(this.nextJobId)) this.nextJobId = 1
    this.admittedJobs += 1
    return jobId
  }

  private async run(request: OfficialOpenScadWireRequest, options: OfficialOpenScadRunOptions) {
    if (options.signal?.aborted) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_CANCELLED',
        'Official OpenSCAD execution was cancelled before admission',
        request.jobId,
      )
    }
    if (!permissionModelAvailable()) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_UNAVAILABLE',
        'The current Node.js runtime does not provide the permission model',
        request.jobId,
      )
    }
    let installation: VerifiedInstallation
    try {
      installation = await this.verifiedInstallationMemoized()
    } catch (error) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_UNAVAILABLE',
        `Official OpenSCAD runtime is unavailable (${classifyUnavailable(error)}); run ${SETUP_COMMAND}`,
        request.jobId,
        { cause: error },
      )
    }
    if (this.closing || this.closed) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_CLOSED',
        'Official OpenSCAD runtime supervisor is closed',
        request.jobId,
      )
    }
    if (options.signal?.aborted) {
      throw new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_CANCELLED',
        'Official OpenSCAD execution was cancelled before child admission',
        request.jobId,
      )
    }
    const timeoutMs = boundedInteger(
      options.timeoutMs ?? OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS,
      'timeoutMs',
      1,
      OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
    )
    return this.spawnAndJoin(
      request,
      installation.runtimePath,
      installation.fontPath,
      installation.manifest.fontSha256,
      timeoutMs,
      options.signal,
    )
  }

  private spawnAndJoin(
    request: OfficialOpenScadWireRequest,
    runtimePath: string,
    fontPath: string,
    fontSha256: string,
    timeoutMs: number,
    signal: AbortSignal | undefined,
  ): Promise<Extract<import('./officialOpenScadRuntimeProtocol').OfficialOpenScadWireTerminal, { status: 'succeeded' }>> {
    let child: ChildProcessWithoutNullStreams
    try {
      child = this.childFactory({ runnerPath: this.runnerPath, runtimePath, fontPath, fontSha256 })
    } catch (error) {
      return Promise.reject(new OfficialOpenScadSupervisorError(
        'E_OFFICIAL_OPENSCAD_CHILD_CRASH',
        'Unable to start the official OpenSCAD child process',
        request.jobId,
        { cause: error },
      ))
    }
    this.jobsStarted += 1
    let resolveJoined!: () => void
    const joined = new Promise<void>(resolve => { resolveJoined = resolve })
    const active: ActiveChild = { child, joined, stopReason: null }
    this.active.set(request.jobId, active)

    return new Promise((resolvePromise, rejectPromise) => {
      const stdout: Buffer[] = []
      let stdoutBytes = 0
      let stderrBytes = 0
      let spawnError: unknown = null
      let finished = false
      const deadline = setTimeout(() => {
        active.stopReason ??= 'deadline'
        child.kill('SIGKILL')
      }, timeoutMs)
      const onAbort = () => {
        active.stopReason ??= 'cancelled'
        child.kill('SIGKILL')
      }
      signal?.addEventListener('abort', onAbort, { once: true })

      child.stdout.on('data', (chunk: Buffer | string) => {
        const data = Buffer.from(chunk)
        stdoutBytes += data.byteLength
        if (stdoutBytes > MAX_CHILD_STDOUT_BYTES) {
          active.stopReason ??= 'output-limit'
          child.kill('SIGKILL')
          return
        }
        stdout.push(data)
      })
      child.stderr.on('data', (chunk: Buffer | string) => {
        // Drain forever, retain nothing, and keep only a saturating byte count.
        stderrBytes = Math.min(MAX_CHILD_STDERR_BYTES, stderrBytes + Buffer.byteLength(chunk))
      })
      child.on('error', error => { spawnError = error })
      child.stdin.on('error', () => undefined)

      const finish = (exitCode: number | null, exitSignal: NodeJS.Signals | null) => {
        if (finished) return
        finished = true
        clearTimeout(deadline)
        signal?.removeEventListener('abort', onAbort)
        this.active.delete(request.jobId)
        this.jobsJoined += 1
        resolveJoined()

        if (active.stopReason === 'cancelled') {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_CANCELLED',
            'Official OpenSCAD execution was cancelled',
            request.jobId,
          ))
          return
        }
        if (active.stopReason === 'deadline') {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_DEADLINE',
            `Official OpenSCAD execution exceeded ${timeoutMs} ms`,
            request.jobId,
          ))
          return
        }
        if (active.stopReason === 'closed') {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_CLOSED',
            'Official OpenSCAD execution was stopped during shutdown',
            request.jobId,
          ))
          return
        }
        if (active.stopReason === 'output-limit') {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_PROTOCOL',
            'Official OpenSCAD child exceeded its output protocol limit',
            request.jobId,
          ))
          return
        }
        if (spawnError !== null) {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_CHILD_CRASH',
            compactChildError(spawnError),
            request.jobId,
            { cause: spawnError },
          ))
          return
        }
        let value: unknown
        try {
          value = JSON.parse(Buffer.concat(stdout, stdoutBytes).toString('utf8'))
        } catch (error) {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_PROTOCOL',
            `Official OpenSCAD child returned invalid JSON (exit ${String(exitCode)}, signal ${String(exitSignal)})`,
            request.jobId,
            { cause: error },
          ))
          return
        }
        if (!isOfficialOpenScadWireTerminal(value, request)
          || value.runtimeVersion !== OFFICIAL_OPENSCAD_RUNTIME_VERSION) {
          rejectPromise(new OfficialOpenScadSupervisorError(
            'E_OFFICIAL_OPENSCAD_PROTOCOL',
            'Official OpenSCAD child returned an invalid or mismatched terminal',
            request.jobId,
          ))
          return
        }
        if (value.status === 'failed') {
          rejectPromise(new OfficialOpenScadRemoteError(
            value.error.code,
            value.error.message,
            request.jobId,
            value.logs,
            value.durationMs,
          ))
          return
        }
        resolvePromise(value)
      }
      child.once('close', finish)
      child.stdin.end(JSON.stringify(request))
    })
  }
}
