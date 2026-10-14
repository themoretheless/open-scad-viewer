import { spawn } from 'node:child_process'
import type { ChildProcessWithoutNullStreams } from 'node:child_process'
import { EXPERIMENT_SET, MAX_FONT_BYTES, MAX_FONT_LICENSE_BYTES, MAX_MANIFEST_BYTES, MAX_RUNTIME_BYTES, OfficialOpenScadSupervisorError, SETUP_COMMAND, UTF8 } from '../officialOpenScadRuntimeService'
import type { NodePermissionFlag, OfficialOpenScadCapabilities, OfficialOpenScadChildFactoryInput, OfficialOpenScadProjectFileInput, OfficialOpenScadRunOptions, OfficialOpenScadUnavailableReason, VerifiedInstallation } from '../officialOpenScadRuntimeService'
import { OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES, OFFICIAL_OPENSCAD_EXPORT_FORMATS, OFFICIAL_OPENSCAD_MAX_DEFINES, OFFICIAL_OPENSCAD_MAX_DEFINE_BYTES, OFFICIAL_OPENSCAD_MAX_LOG_BYTES, OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES, OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES, OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES, OFFICIAL_OPENSCAD_MAX_PROJECT_FILES, OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES, OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES, OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS, OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES, OFFICIAL_OPENSCAD_PROTOCOL_VERSION, isOfficialOpenScadWireRequest, isWellFormedOfficialOpenScadText, normalizeOfficialOpenScadProjectPath, officialOpenScadProjectPathsConflict } from '../officialOpenScadRuntimeProtocol'
import type { OfficialOpenScadExportFormat, OfficialOpenScadWireRequest } from '../officialOpenScadRuntimeProtocol'
import { OFFICIAL_OPENSCAD_FONT_FAMILY, OFFICIAL_OPENSCAD_FONT_FILENAME, OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME, OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256, OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME, OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION, OFFICIAL_OPENSCAD_RUNTIME_VERSION, isOfficialOpenScadRuntimeManifest, matchesOfficialOpenScadRuntimeIntegrity, sha256Buffer } from '../officialOpenScadRuntimePatch'
import type { OfficialOpenScadRuntimeIntegrity } from '../officialOpenScadRuntimePatch'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'
import { lstat, readFile, realpath, stat } from 'node:fs/promises'

export function defaultRunnerPath(): string {
  return fileURLToPath(new URL('../officialOpenScadRuntimeRunner.mjs', import.meta.url))
}

export function nodePermissionFlag(): NodePermissionFlag | null {
  if (process.allowedNodeEnvironmentFlags.has('--permission')) return '--permission'
  if (process.allowedNodeEnvironmentFlags.has('--experimental-permission')) {
    return '--experimental-permission'
  }
  return null
}

export function permissionModelAvailable(): boolean {
  return nodePermissionFlag() !== null
}

export function defaultChildFactory(input: OfficialOpenScadChildFactoryInput): ChildProcessWithoutNullStreams {
  const permissionFlag = nodePermissionFlag()
  if (permissionFlag === null) {
    throw new Error('The current Node.js runtime does not provide the permission model')
  }
  return spawn(process.execPath, [
    permissionFlag,
    '--max-old-space-size=512',
    `--allow-fs-read=${input.runnerPath}`,
    `--allow-fs-read=${input.runtimePath}`,
    `--allow-fs-read=${input.fontPath}`,
    input.runnerPath,
    input.runtimePath,
    input.fontPath,
    input.fontSha256,
  ], {
    env: { NODE_NO_WARNINGS: '1', ...(process.env.ELECTRON_RUN_AS_NODE ? { ELECTRON_RUN_AS_NODE: process.env.ELECTRON_RUN_AS_NODE } : {}) },
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
  })
}

export function boundedInteger(value: number, label: string, minimum: number, maximum: number): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}

export function unavailableCapabilities(
  reason: OfficialOpenScadUnavailableReason,
  runtimeSha256: string | null = null,
): OfficialOpenScadCapabilities {
  return capabilitiesShape(false, reason, null, runtimeSha256, null, null)
}

export function capabilitiesShape(
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
      wireRequestBytes: OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES,
      outputBytes: OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
      logBytes: OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
      logEntries: OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
      timeoutMs: OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
    }),
    setupCommand: SETUP_COMMAND,
  })
}

export async function readBoundedFile(path: string, maximumBytes: number, label: string): Promise<Buffer> {
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

export async function verifyInstallation(
  cacheRoot: string,
  expectedIntegrity: OfficialOpenScadRuntimeIntegrity,
): Promise<VerifiedInstallation> {
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
  if (!matchesOfficialOpenScadRuntimeIntegrity(manifest, expectedIntegrity)) {
    throw Object.assign(new Error('Official OpenSCAD manifest does not match the pinned runtime integrity'), {
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

/** Cheap change detector for already-verified installation files. */
export async function installationSignature(paths: readonly string[]): Promise<string | null> {
  try {
    const entries = await Promise.all(paths.map(path => stat(path)))
    return entries.map(entry => `${entry.size}:${entry.mtimeMs}`).join('|')
  } catch {
    return null
  }
}

export function classifyUnavailable(error: unknown): OfficialOpenScadUnavailableReason {  if (error && typeof error === 'object' && 'code' in error
    && (error.code === 'ENOENT' || error.code === 'ENOTDIR')) return 'not-installed'
  if (error && typeof error === 'object' && 'verificationStage' in error
    && error.verificationStage === 'runtime') return 'runtime-integrity-failed'
  return 'manifest-invalid'
}

export function validateSource(source: string): void {
  if (typeof source !== 'string' || !isWellFormedOfficialOpenScadText(source)) {
    throw new TypeError('Official OpenSCAD source must be well-formed text')
  }
  if (UTF8.encode(source).byteLength > OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES) {
    throw new RangeError(`Official OpenSCAD source exceeds ${OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES} UTF-8 bytes`)
  }
}

export function validateDefines(defines: readonly string[]): void {
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

export function encodeFiles(sourceBytes: number, files: readonly OfficialOpenScadProjectFileInput[]) {
  if (!Array.isArray(files) || files.length > OFFICIAL_OPENSCAD_MAX_PROJECT_FILES) {
    throw new RangeError(`Official OpenSCAD accepts at most ${OFFICIAL_OPENSCAD_MAX_PROJECT_FILES} project files`)
  }
  const paths = new Set<string>()
  let totalBytes = sourceBytes
  return files.map(file => {
    if (!file || typeof file !== 'object') throw new TypeError('Official OpenSCAD project file must be an object')
    const path = normalizeOfficialOpenScadProjectPath(file.path)
    if ([...paths].some(existing => officialOpenScadProjectPathsConflict(existing, path))) {
      throw new TypeError(`Conflicting OpenSCAD project path: ${path}`)
    }
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

export function buildRequest(
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
  const wireBytes = Buffer.byteLength(JSON.stringify(request))
  if (wireBytes > OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES) {
    throw new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_REQUEST_LIMIT',
      `Official OpenSCAD serialized request exceeds ${OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES} bytes`,
      jobId,
    )
  }
  return request
}

export function compactChildError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error)
  return raw.replace(/[\r\n\u2028\u2029]+/g, ' ').trim().slice(0, 512)
    || 'Official OpenSCAD child failed'
}
