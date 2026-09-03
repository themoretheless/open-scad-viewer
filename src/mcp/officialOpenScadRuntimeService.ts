import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { lstat, readFile, realpath } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  isOfficialOpenScadRuntimeManifest,
  sha256Buffer,
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
  OFFICIAL_OPENSCAD_PROTOCOL_VERSION,
  isOfficialOpenScadWireRequest,
  isOfficialOpenScadWireTerminal,
  isWellFormedOfficialOpenScadText,
  normalizeOfficialOpenScadProjectPath,
  type OfficialOpenScadBackend,
  type OfficialOpenScadExperimentalFeature,
  type OfficialOpenScadExportFormat,
  type OfficialOpenScadLogs,
  type OfficialOpenScadWireRequest,
} from './officialOpenScadRuntimeProtocol'

const MAX_MANIFEST_BYTES = 16 * 1024
const MAX_RUNTIME_BYTES = 64 * 1024 * 1024
const MAX_CHILD_STDOUT_BYTES = 9 * 1024 * 1024
const MAX_CHILD_STDERR_BYTES = 8 * 1024
const MAX_FONT_BYTES = 2 * 1024 * 1024
const MAX_FONT_LICENSE_BYTES = 64 * 1024
const DEFAULT_MAX_CONCURRENT_JOBS = 1
const MAX_CONCURRENT_JOBS = 4
const SETUP_COMMAND = 'npm run setup:openscad'
const FORMAT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPORT_FORMATS)
const EXPERIMENT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)
const UTF8 = new TextEncoder()

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

interface VerifiedInstallation {
  readonly manifest: OfficialOpenScadRuntimeManifest
  readonly runtimePath: string
  readonly fontPath: string
}

interface ActiveChild {
  readonly child: ChildProcessWithoutNullStreams
  readonly joined: Promise<void>
  stopReason: 'cancelled' | 'deadline' | 'closed' | 'output-limit' | null
}

function defaultCacheRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '../..', '.open-scad-runtime')
}

function defaultRunnerPath(): string {
  return fileURLToPath(new URL('./officialOpenScadRuntimeRunner.mjs', import.meta.url))
}

function permissionModelAvailable(): boolean {
  return process.allowedNodeEnvironmentFlags.has('--experimental-permission')
    || process.allowedNodeEnvironmentFlags.has('--permission')
}

function defaultChildFactory(input: OfficialOpenScadChildFactoryInput): ChildProcessWithoutNullStreams {
  return spawn(process.execPath, [
    '--experimental-permission',
    '--max-old-space-size=512',
    `--allow-fs-read=${input.runnerPath}`,
    `--allow-fs-read=${input.runtimePath}`,
    `--allow-fs-read=${input.fontPath}`,
    input.runnerPath,
    input.runtimePath,
    input.fontPath,
    input.fontSha256,
  ], {
    env: { NODE_NO_WARNINGS: '1' },
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
  })
}

function boundedInteger(value: number, label: string, minimum: number, maximum: number): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}

function unavailableCapabilities(
  reason: OfficialOpenScadUnavailableReason,
  runtimeSha256: string | null = null,
): OfficialOpenScadCapabilities {
  return capabilitiesShape(false, reason, null, runtimeSha256, null, null)
}

function capabilitiesShape(
  available: boolean,
  unavailableReason: OfficialOpenScadUnavailableReason | null,
  runtimeVersion: typeof OFFICIAL_OPENSCAD_RUNTIME_VERSION | null,
  runtimeSha256: string | null,
  fontSha256: string | null,
  fontLicenseSha256: string | null,
): OfficialOpenScadCapabilities {
  return Object.freeze({
    available,
    unavailableReason,
    expectedRuntimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
    runtimeVersion,
    archiveSha256: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
    runtimeSha256,
    patchVersion: OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
    defaultFont: Object.freeze({
      family: OFFICIAL_OPENSCAD_FONT_FAMILY,
      filename: OFFICIAL_OPENSCAD_FONT_FILENAME,
      sha256: fontSha256,
      licenseFilename: OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
      licenseSha256: fontLicenseSha256,
    }),
    isolation: Object.freeze({
      filesystem: 'MEMFS' as const,
      nodePermissionModel: true as const,
      scadHostFileRead: false as const,
      scadHostFileWrite: false as const,
      networkApiExposed: false as const,
      networkSandboxEnforced: false as const,
      wasmMemoryLimitEnforced: false as const,
      processModel: 'one-shot' as const,
    }),
    formats: OFFICIAL_OPENSCAD_EXPORT_FORMATS,
    experimentalFeatures: OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
    defaults: Object.freeze({
      backend: 'Manifold' as const,
      hardWarnings: true as const,
      checkParameters: true as const,
      checkParameterRanges: true as const,
      experimentsEnabled: false as const,
    }),
    limits: Object.freeze({
      sourceBytes: OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
      projectFiles: OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
      projectFileBytes: OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
      projectBytes: OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
      outputBytes: OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
      logBytes: OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
      logEntries: OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
      timeoutMs: OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
    }),
    setupCommand: SETUP_COMMAND,
  })
}

async function readBoundedFile(path: string, maximumBytes: number, label: string): Promise<Buffer> {
  const details = await lstat(path)
  if (details.isSymbolicLink() || !details.isFile() || !Number.isSafeInteger(details.size) || details.size <= 0
    || details.size > maximumBytes) {
    throw new Error(`${label} is missing or exceeds ${maximumBytes} bytes`)
  }
  const data = await readFile(path)
  if (data.byteLength !== details.size || data.byteLength > maximumBytes) {
    throw new Error(`${label} changed while it was being verified`)
  }
  return data
}

async function verifyInstallation(cacheRoot: string): Promise<VerifiedInstallation> {
  const manifestPath = join(cacheRoot, OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME)
  const manifestBytes = await readBoundedFile(manifestPath, MAX_MANIFEST_BYTES, 'Official OpenSCAD manifest')
  let manifest: unknown
  try {
    manifest = JSON.parse(manifestBytes.toString('utf8'))
  } catch (error) {
    throw Object.assign(new Error('Official OpenSCAD manifest is not valid JSON', { cause: error }), {
      verificationStage: 'manifest',
    })
  }
  if (!isOfficialOpenScadRuntimeManifest(manifest)) {
    throw Object.assign(new Error('Official OpenSCAD manifest does not match the pinned runtime'), {
      verificationStage: 'manifest',
    })
  }
  const runtimePath = join(cacheRoot, manifest.runtimeFilename)
  const fontPath = join(cacheRoot, manifest.fontFilename)
  const fontLicensePath = join(cacheRoot, manifest.fontLicenseFilename)
  const runtimeBytes = await readBoundedFile(runtimePath, MAX_RUNTIME_BYTES, 'Official OpenSCAD runtime')
  if (sha256Buffer(runtimeBytes) !== manifest.runtimeSha256) {
    throw Object.assign(new Error('Official OpenSCAD runtime checksum does not match its manifest'), {
      verificationStage: 'runtime',
    })
  }
  const fontBytes = await readBoundedFile(fontPath, MAX_FONT_BYTES, 'Official OpenSCAD default font')
  if (sha256Buffer(fontBytes) !== manifest.fontSha256) {
    throw Object.assign(new Error('Official OpenSCAD default font checksum does not match its manifest'), {
      verificationStage: 'runtime',
    })
  }
  const fontLicenseBytes = await readBoundedFile(
    fontLicensePath,
    MAX_FONT_LICENSE_BYTES,
    'Official OpenSCAD default font license',
  )
  if (sha256Buffer(fontLicenseBytes) !== manifest.fontLicenseSha256) {
    throw Object.assign(new Error('Official OpenSCAD default font license checksum does not match its manifest'), {
      verificationStage: 'runtime',
    })
  }
  return {
    manifest,
    runtimePath: await realpath(runtimePath),
    fontPath: await realpath(fontPath),
  }
}

function classifyUnavailable(error: unknown): OfficialOpenScadUnavailableReason {
  if (error && typeof error === 'object' && 'code' in error
    && (error.code === 'ENOENT' || error.code === 'ENOTDIR')) return 'not-installed'
  if (error && typeof error === 'object' && 'verificationStage' in error
    && error.verificationStage === 'runtime') return 'runtime-integrity-failed'
  return 'manifest-invalid'
}

function validateSource(source: string): void {
  if (typeof source !== 'string' || !isWellFormedOfficialOpenScadText(source)) {
    throw new TypeError('Official OpenSCAD source must be well-formed text')
  }
  if (UTF8.encode(source).byteLength > OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES) {
    throw new RangeError(`Official OpenSCAD source exceeds ${OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES} UTF-8 bytes`)
  }
}

function validateDefines(defines: readonly string[]): void {
  if (!Array.isArray(defines) || defines.length > OFFICIAL_OPENSCAD_MAX_DEFINES) {
    throw new RangeError(`Official OpenSCAD accepts at most ${OFFICIAL_OPENSCAD_MAX_DEFINES} definitions`)
  }
  let bytes = 0
  for (const define of defines) {
    if (typeof define !== 'string' || !isWellFormedOfficialOpenScadText(define)
      || define.includes('\0') || /[\r\n]/.test(define)
      || !/^[$A-Za-z_][$A-Za-z0-9_]*\s*=/.test(define)) {
      throw new TypeError('Each OpenSCAD definition must be a single-line name=value expression')
    }
    bytes += UTF8.encode(define).byteLength
  }
  if (bytes > OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES) {
    throw new RangeError(`OpenSCAD definitions exceed ${OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES} UTF-8 bytes`)
  }
}

function encodeFiles(sourceBytes: number, files: readonly OfficialOpenScadProjectFileInput[]) {
  if (!Array.isArray(files) || files.length > OFFICIAL_OPENSCAD_MAX_PROJECT_FILES) {
    throw new RangeError(`Official OpenSCAD accepts at most ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILES} project files`)
  }
  const paths = new Set<string>()
  let totalBytes = sourceBytes
  return files.map(file => {
    if (!file || typeof file !== 'object') throw new TypeError('Official OpenSCAD project file must be an object')
    const path = normalizeOfficialOpenScadProjectPath(file.path)
    if (paths.has(path)) throw new TypeError(`Duplicate OpenSCAD project path: ${path}`)
    paths.add(path)
    let data: Uint8Array
    if (typeof file.data === 'string') {
      if (!isWellFormedOfficialOpenScadText(file.data)) throw new TypeError(`Project file ${path} is not well-formed text`)
      data = UTF8.encode(file.data)
    } else if (file.data instanceof Uint8Array) {
      data = new Uint8Array(file.data)
    } else {
      throw new TypeError(`Project file ${path} data must be text or Uint8Array`)
    }
    if (data.byteLength > OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES) {
      throw new RangeError(`Project file ${path} exceeds ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES} bytes`)
    }
    totalBytes += data.byteLength
    if (totalBytes > OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES) {
      throw new RangeError(`OpenSCAD project exceeds ${OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES} bytes`)
    }
    return Object.freeze({
      path,
      dataBase64: Buffer.from(data).toString('base64'),
      sha256: sha256Buffer(data),
    })
  })
}

function buildRequest(
  jobId: number,
  operation: 'check' | 'export',
  source: string,
  format: OfficialOpenScadExportFormat | null,
  options: OfficialOpenScadRunOptions,
): OfficialOpenScadWireRequest {
  validateSource(source)
  const sourceBytes = UTF8.encode(source).byteLength
  const features = [...(options.experimentalFeatures ?? [])]
  const seenFeatures = new Set<string>()
  for (const feature of features) {
    if (typeof feature !== 'string' || !EXPERIMENT_SET.has(feature) || seenFeatures.has(feature)) {
      throw new TypeError(`Unsupported or duplicate OpenSCAD experimental feature: ${String(feature)}`)
    }
    seenFeatures.add(feature)
  }
  const defines = [...(options.defines ?? [])]
  validateDefines(defines)
  const time = options.time ?? null
  if (time !== null && (!Number.isFinite(time) || time < 0 || time > 1)) {
    throw new RangeError('OpenSCAD time must be a finite number between 0 and 1')
  }
  const backend = options.backend ?? 'Manifold'
  if (backend !== 'Manifold' && backend !== 'CGAL') throw new TypeError('OpenSCAD backend must be Manifold or CGAL')
  const request: OfficialOpenScadWireRequest = Object.freeze({
    protocolVersion: OFFICIAL_OPENSCAD_PROTOCOL_VERSION,
    jobId,
    operation,
    source,
    sourceSha256: sha256Buffer(source),
    files: encodeFiles(sourceBytes, options.files ?? []),
    format,
    options: Object.freeze({
      experimentalFeatures: Object.freeze(features),
      defines: Object.freeze(defines),
      time,
      backend,
      hardWarnings: options.hardWarnings ?? true,
      checkParameters: options.checkParameters ?? true,
      checkParameterRanges: options.checkParameterRanges ?? true,
    }),
  })
  if (!isOfficialOpenScadWireRequest(request)) throw new TypeError('Official OpenSCAD request failed protocol validation')
  return request
}

function compactChildError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error)
  return raw.replace(/[\r\n\u2028\u2029]+/g, ' ').trim().slice(0, 512)
    || 'Official OpenSCAD child failed'
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
  private readonly active = new Map<number, ActiveChild>()
  private admittedJobs = 0
  private nextJobId = 1
  private jobsStarted = 0
  private jobsJoined = 0
  private closing = false
  private closed = false
  private closePromise: Promise<void> | null = null

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

  async capabilities(): Promise<OfficialOpenScadCapabilities> {
    if (!permissionModelAvailable()) return unavailableCapabilities('permission-model-unavailable')
    try {
      const installation = await verifyInstallation(this.cacheRoot)
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
      installation = await verifyInstallation(this.cacheRoot)
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
