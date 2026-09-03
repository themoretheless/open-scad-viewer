import { sha256Buffer } from './officialOpenScadRuntimePatch'

export const OFFICIAL_OPENSCAD_PROTOCOL_VERSION = 1 as const
export const OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES = 1_048_576
export const OFFICIAL_OPENSCAD_MAX_PROJECT_FILES = 128
export const OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES = 4 * 1024 * 1024
export const OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES = 6 * 1024 * 1024
export const OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES = 6 * 1024 * 1024
export const OFFICIAL_OPENSCAD_MAX_LOG_BYTES = 64 * 1024
export const OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES = 128
export const OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES = 2 * 1024
export const OFFICIAL_OPENSCAD_MAX_DEFINES = 64
export const OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES = 4 * 1024
export const OFFICIAL_OPENSCAD_DEFAULT_TIMEOUT_MS = 30_000
export const OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS = 120_000

export const OFFICIAL_OPENSCAD_EXPORT_FORMATS = Object.freeze([
  'stl',
  'off',
  'wrl',
  '3mf',
  'csg',
  'dxf',
  'svg',
] as const)

export type OfficialOpenScadExportFormat = typeof OFFICIAL_OPENSCAD_EXPORT_FORMATS[number]

/** Language-affecting experiments in the pinned snapshot. Host-integration features are excluded. */
export const OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES = Object.freeze([
  'roof',
  'lazy-union',
  'vertex-object-renderers-indexing',
  'textmetrics',
  'import-function',
  'object-function',
  'predictible-output',
  'vector-swizzle',
  'discretization-by-error',
  'ai-features',
  'unicode-identifiers',
] as const)

export type OfficialOpenScadExperimentalFeature =
  typeof OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES[number]
export type OfficialOpenScadBackend = 'Manifold' | 'CGAL'

export interface OfficialOpenScadWireFile {
  readonly path: string
  readonly dataBase64: string
  readonly sha256: string
}

export interface OfficialOpenScadWireOptions {
  readonly experimentalFeatures: readonly OfficialOpenScadExperimentalFeature[]
  readonly defines: readonly string[]
  readonly time: number | null
  readonly backend: OfficialOpenScadBackend
  readonly hardWarnings: boolean
  readonly checkParameters: boolean
  readonly checkParameterRanges: boolean
}

export interface OfficialOpenScadWireRequest {
  readonly protocolVersion: typeof OFFICIAL_OPENSCAD_PROTOCOL_VERSION
  readonly jobId: number
  readonly operation: 'check' | 'export'
  readonly source: string
  readonly sourceSha256: string
  readonly files: readonly OfficialOpenScadWireFile[]
  readonly format: OfficialOpenScadExportFormat | null
  readonly options: OfficialOpenScadWireOptions
}

export interface OfficialOpenScadLogs {
  readonly stdout: readonly string[]
  readonly stderr: readonly string[]
  readonly truncated: boolean
}

interface OfficialOpenScadTerminalBase {
  readonly protocolVersion: typeof OFFICIAL_OPENSCAD_PROTOCOL_VERSION
  readonly jobId: number
  readonly operation: 'check' | 'export'
  readonly sourceSha256: string
  readonly runtimeVersion: string
  readonly durationMs: number
  readonly logs: OfficialOpenScadLogs
}

export interface OfficialOpenScadWireSuccess extends OfficialOpenScadTerminalBase {
  readonly status: 'succeeded'
  readonly output: {
    readonly format: OfficialOpenScadExportFormat | 'csg'
    readonly mimeType: string
    readonly byteLength: number
    readonly sha256: string
    readonly dataBase64: string | null
  }
}

export interface OfficialOpenScadWireFailure extends OfficialOpenScadTerminalBase {
  readonly status: 'failed'
  readonly error: {
    readonly code: string
    readonly name: string
    readonly message: string
  }
}

export type OfficialOpenScadWireTerminal =
  | OfficialOpenScadWireSuccess
  | OfficialOpenScadWireFailure

const REQUEST_KEYS = [
  'protocolVersion', 'jobId', 'operation', 'source', 'sourceSha256', 'files',
  'format', 'options',
] as const
const FILE_KEYS = ['path', 'dataBase64', 'sha256'] as const
const OPTIONS_KEYS = [
  'experimentalFeatures', 'defines', 'time', 'backend', 'hardWarnings',
  'checkParameters', 'checkParameterRanges',
] as const
const TERMINAL_BASE_KEYS = [
  'protocolVersion', 'jobId', 'operation', 'sourceSha256', 'runtimeVersion',
  'durationMs', 'logs', 'status',
] as const
const LOG_KEYS = ['stdout', 'stderr', 'truncated'] as const
const OUTPUT_KEYS = ['format', 'mimeType', 'byteLength', 'sha256', 'dataBase64'] as const
const ERROR_KEYS = ['code', 'name', 'message'] as const
const UTF8 = new TextEncoder()
const FORMAT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPORT_FORMATS)
const EXPERIMENT_SET = new Set<string>(OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES)

function record(value: unknown): Record<string, unknown> | null {
  if (value === null || Array.isArray(value) || typeof value !== 'object') return null
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
    ? value as Record<string, unknown>
    : null
}

function exactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const actual = Object.keys(value)
  return actual.length === keys.length
    && actual.every(key => keys.includes(key))
    && keys.every(key => Object.hasOwn(value, key))
}

function positiveSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) > 0
}

function nonNegativeSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}

export function isWellFormedOfficialOpenScadText(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

export function normalizeOfficialOpenScadProjectPath(value: string): string {
  if (typeof value !== 'string' || value.length === 0 || value.length > 1_024
    || value !== value.normalize('NFC') || value.includes('\\') || value.includes('\0')
    || value.startsWith('/') || !isWellFormedOfficialOpenScadText(value)) {
    throw new TypeError('Project file path must be a normalized relative POSIX path')
  }
  const segments = value.split('/')
  if (segments.some(segment => segment.length === 0 || segment === '.' || segment === '..')) {
    throw new TypeError('Project file path must not contain empty, dot, or parent segments')
  }
  if (value === 'main.scad' || value === 'fonts/Basic-Regular.ttf'
    || value === 'fonts/fonts.conf' || value.startsWith('__open_scad_result.')) {
    throw new TypeError('Project file path is reserved by the official OpenSCAD runner')
  }
  return value
}

function strictBase64(value: unknown): Buffer | null {
  if (typeof value !== 'string' || value.length % 4 !== 0
    || value.length > Math.ceil(OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES / 3) * 4) return null
  const paddingIndex = value.indexOf('=')
  if (paddingIndex >= 0 && (paddingIndex < value.length - 2
    || !/^={1,2}$/u.test(value.slice(paddingIndex)))) return null
  const dataEnd = paddingIndex < 0 ? value.length : paddingIndex
  for (let index = 0; index < dataEnd; index += 1) {
    const code = value.charCodeAt(index)
    const allowed = (code >= 0x41 && code <= 0x5a) || (code >= 0x61 && code <= 0x7a)
      || (code >= 0x30 && code <= 0x39) || code === 0x2b || code === 0x2f
    if (!allowed) return null
  }
  const decoded = Buffer.from(value, 'base64')
  return decoded.toString('base64') === value ? decoded : null
}

function validSha(value: unknown): value is string {
  return typeof value === 'string' && /^[0-9a-f]{64}$/.test(value)
}

function isFormat(value: unknown): value is OfficialOpenScadExportFormat {
  return typeof value === 'string' && FORMAT_SET.has(value)
}

function validDefines(value: unknown): value is readonly string[] {
  if (!Array.isArray(value) || value.length > OFFICIAL_OPENSCAD_MAX_DEFINES) return false
  let bytes = 0
  for (const define of value) {
    if (typeof define !== 'string' || !isWellFormedOfficialOpenScadText(define)
      || define.includes('\0') || /[\r\n]/.test(define)
      || !/^[$A-Za-z_][$A-Za-z0-9_]*\s*=/.test(define)) return false
    bytes += UTF8.encode(define).byteLength
    if (bytes > OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES) return false
  }
  return true
}

function validOptions(value: unknown): value is OfficialOpenScadWireOptions {
  const options = record(value)
  if (options === null || !exactKeys(options, OPTIONS_KEYS)
    || !Array.isArray(options.experimentalFeatures)
    || options.experimentalFeatures.length > OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES.length
    || !validDefines(options.defines)
    || (options.time !== null && (typeof options.time !== 'number'
      || !Number.isFinite(options.time) || options.time < 0 || options.time > 1))
    || (options.backend !== 'Manifold' && options.backend !== 'CGAL')
    || typeof options.hardWarnings !== 'boolean'
    || typeof options.checkParameters !== 'boolean'
    || typeof options.checkParameterRanges !== 'boolean') return false
  const seen = new Set<string>()
  return options.experimentalFeatures.every(feature => {
    if (typeof feature !== 'string' || !EXPERIMENT_SET.has(feature) || seen.has(feature)) return false
    seen.add(feature)
    return true
  })
}

export function isOfficialOpenScadWireRequest(value: unknown): value is OfficialOpenScadWireRequest {
  const request = record(value)
  if (request === null || !exactKeys(request, REQUEST_KEYS)
    || request.protocolVersion !== OFFICIAL_OPENSCAD_PROTOCOL_VERSION
    || !positiveSafeInteger(request.jobId)
    || (request.operation !== 'check' && request.operation !== 'export')
    || typeof request.source !== 'string' || !isWellFormedOfficialOpenScadText(request.source)
    || UTF8.encode(request.source).byteLength > OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES
    || !validSha(request.sourceSha256) || sha256Buffer(request.source) !== request.sourceSha256
    || !Array.isArray(request.files) || request.files.length > OFFICIAL_OPENSCAD_MAX_PROJECT_FILES
    || !validOptions(request.options)) return false
  if ((request.operation === 'check' && request.format !== null)
    || (request.operation === 'export' && !isFormat(request.format))) return false

  const paths = new Set<string>()
  let projectBytes = UTF8.encode(request.source).byteLength
  for (const valueFile of request.files) {
    const file = record(valueFile)
    if (file === null || !exactKeys(file, FILE_KEYS) || typeof file.path !== 'string'
      || !validSha(file.sha256)) return false
    try {
      if (normalizeOfficialOpenScadProjectPath(file.path) !== file.path || paths.has(file.path)) return false
    } catch {
      return false
    }
    const data = strictBase64(file.dataBase64)
    if (data === null || data.byteLength > OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES
      || sha256Buffer(data) !== file.sha256) return false
    paths.add(file.path)
    projectBytes += data.byteLength
    if (projectBytes > OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES) return false
  }
  return true
}

function validLogEntries(value: unknown): value is readonly string[] {
  if (!Array.isArray(value) || value.length > OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES) return false
  return value.every(entry => typeof entry === 'string'
    && isWellFormedOfficialOpenScadText(entry)
    && UTF8.encode(entry).byteLength <= OFFICIAL_OPENSCAD_MAX_LOG_ENTRY_BYTES)
}

function validLogs(value: unknown): value is OfficialOpenScadLogs {
  const logs = record(value)
  if (logs === null || !exactKeys(logs, LOG_KEYS)
    || !validLogEntries(logs.stdout) || !validLogEntries(logs.stderr)
    || typeof logs.truncated !== 'boolean') return false
  if (logs.stdout.length + logs.stderr.length > OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES) return false
  const bytes = [...logs.stdout, ...logs.stderr]
    .reduce((total, entry) => total + UTF8.encode(entry).byteLength, 0)
  return bytes <= OFFICIAL_OPENSCAD_MAX_LOG_BYTES
}

function validTerminalBase(value: Record<string, unknown>, request: OfficialOpenScadWireRequest): boolean {
  return value.protocolVersion === OFFICIAL_OPENSCAD_PROTOCOL_VERSION
    && value.jobId === request.jobId
    && value.operation === request.operation
    && value.sourceSha256 === request.sourceSha256
    && typeof value.runtimeVersion === 'string'
    && value.runtimeVersion.length > 0 && value.runtimeVersion.length <= 64
    && typeof value.durationMs === 'number' && Number.isFinite(value.durationMs)
    && value.durationMs >= 0 && value.durationMs <= OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS + 5_000
    && validLogs(value.logs)
}

export function isOfficialOpenScadWireTerminal(
  value: unknown,
  request: OfficialOpenScadWireRequest,
): value is OfficialOpenScadWireTerminal {
  const terminal = record(value)
  if (terminal === null || !validTerminalBase(terminal, request)) return false
  if (terminal.status === 'failed') {
    if (!exactKeys(terminal, [...TERMINAL_BASE_KEYS, 'error'])) return false
    const error = record(terminal.error)
    return error !== null && exactKeys(error, ERROR_KEYS)
      && typeof error.code === 'string' && /^[A-Z][A-Z0-9_]{0,63}$/.test(error.code)
      && typeof error.name === 'string' && error.name.length > 0 && error.name.length <= 128
      && typeof error.message === 'string' && error.message.length > 0
      && UTF8.encode(error.message).byteLength <= 4_096
      && isWellFormedOfficialOpenScadText(error.message)
  }
  if (terminal.status !== 'succeeded'
    || !exactKeys(terminal, [...TERMINAL_BASE_KEYS, 'output'])) return false
  const output = record(terminal.output)
  if (output === null || !exactKeys(output, OUTPUT_KEYS)
    || (request.operation === 'check' ? output.format !== 'csg' : output.format !== request.format)
    || typeof output.mimeType !== 'string' || output.mimeType.length === 0 || output.mimeType.length > 128
    || output.mimeType !== officialOpenScadMimeType(output.format as OfficialOpenScadExportFormat | 'csg')
    || !nonNegativeSafeInteger(output.byteLength) || output.byteLength > OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES
    || !validSha(output.sha256)) return false
  if (request.operation === 'check') return output.dataBase64 === null
  const data = strictBase64(output.dataBase64)
  return data !== null && data.byteLength === output.byteLength
    && sha256Buffer(data) === output.sha256
}

export function officialOpenScadMimeType(format: OfficialOpenScadExportFormat | 'csg'): string {
  switch (format) {
    case 'stl': return 'model/stl'
    case 'off': return 'model/vnd.off'
    case 'wrl': return 'model/vrml'
    case '3mf': return 'model/3mf'
    case 'dxf': return 'image/vnd.dxf'
    case 'svg': return 'image/svg+xml'
    case 'csg': return 'text/x-openscad-csg'
  }
}
