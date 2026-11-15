import { join, resolve } from 'node:path'
import {
  OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  OFFICIAL_OPENSCAD_PINNED_INTEGRITY,
  type OfficialOpenScadRuntimeIntegrity,
} from './officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS,
  OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  type OfficialOpenScadExportFormat,
  type OfficialOpenScadWireRequest,
} from './officialOpenScadRuntimeProtocol'
import {
  boundedInteger,
  buildRequest,
  capabilitiesShape,
  classifyUnavailable,
  defaultCacheRoot,
  defaultChildFactory,
  defaultRunnerPath,
  installationSignature,
  permissionModelAvailable,
  unavailableCapabilities,
  verifyInstallation,
} from './officialOpenScadRuntimeService/defaultCacheRoot'
import { spawnAndJoinOfficialChild } from './officialOpenScadRuntimeService/spawnAndJoinChild'
import {
  DEFAULT_MAX_CONCURRENT_JOBS,
  EXPERIMENT_SET,
  FORMAT_SET,
  MAX_CONCURRENT_JOBS,
  MAX_FONT_BYTES,
  MAX_FONT_LICENSE_BYTES,
  MAX_MANIFEST_BYTES,
  MAX_RUNTIME_BYTES,
  OfficialOpenScadRemoteError,
  OfficialOpenScadSupervisorError,
  SETUP_COMMAND,
  UTF8,
  type ActiveChild,
  type NodePermissionFlag,
  type OfficialOpenScadCapabilities,
  type OfficialOpenScadCheckResult,
  type OfficialOpenScadChildFactory,
  type OfficialOpenScadChildFactoryInput,
  type OfficialOpenScadExportResult,
  type OfficialOpenScadProjectFileInput,
  type OfficialOpenScadRunOptions,
  type OfficialOpenScadRuntimeService,
  type OfficialOpenScadRuntimeSupervisorOptions,
  type OfficialOpenScadSupervisorErrorCode,
  type OfficialOpenScadSupervisorSnapshot,
  type OfficialOpenScadUnavailableReason,
  type VerifiedInstallation,
} from './officialOpenScadRuntimeService/types'

export {
  EXPERIMENT_SET,
  MAX_FONT_BYTES,
  MAX_FONT_LICENSE_BYTES,
  MAX_MANIFEST_BYTES,
  MAX_RUNTIME_BYTES,
  OfficialOpenScadRemoteError,
  OfficialOpenScadSupervisorError,
  SETUP_COMMAND,
  UTF8,
}
export type {
  NodePermissionFlag,
  OfficialOpenScadCapabilities,
  OfficialOpenScadCheckResult,
  OfficialOpenScadChildFactory,
  OfficialOpenScadChildFactoryInput,
  OfficialOpenScadExportResult,
  OfficialOpenScadProjectFileInput,
  OfficialOpenScadRunOptions,
  OfficialOpenScadRuntimeService,
  OfficialOpenScadRuntimeSupervisorOptions,
  OfficialOpenScadSupervisorErrorCode,
  OfficialOpenScadSupervisorSnapshot,
  OfficialOpenScadUnavailableReason,
  VerifiedInstallation,
}

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

  async check(source: string, options: OfficialOpenScadRunOptions = {}): Promise<OfficialOpenScadCheckResult> {
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
    return spawnAndJoinOfficialChild({
      request,
      runnerPath: this.runnerPath,
      runtimePath: installation.runtimePath,
      fontPath: installation.fontPath,
      fontSha256: installation.manifest.fontSha256,
      timeoutMs,
      signal: options.signal,
      childFactory: this.childFactory,
      activeMap: this.active,
      onStarted: () => {
        this.jobsStarted += 1
      },
      onJoined: () => {
        this.jobsJoined += 1
      },
    })
  }
}
