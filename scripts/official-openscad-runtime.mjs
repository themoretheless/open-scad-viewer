import { randomUUID } from 'node:crypto'
import { lstat, mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises'
import { dirname, isAbsolute, join, relative, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { inflateRawSync } from 'node:zlib'
import {
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256,
  OFFICIAL_OPENSCAD_FONT_LICENSE_URL,
  OFFICIAL_OPENSCAD_FONT_SHA256,
  OFFICIAL_OPENSCAD_FONT_URL,
  OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  createOfficialOpenScadRuntimeManifest,
  isOfficialOpenScadRuntimeManifest,
  patchOfficialOpenScadRuntimeSource,
  sha256Buffer,
} from '../src/mcp/officialOpenScadRuntimePatch.ts'

export {
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256,
  OFFICIAL_OPENSCAD_FONT_LICENSE_URL,
  OFFICIAL_OPENSCAD_FONT_SHA256,
  OFFICIAL_OPENSCAD_FONT_URL,
  OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  createOfficialOpenScadRuntimeManifest,
  isOfficialOpenScadRuntimeManifest,
  patchOfficialOpenScadRuntimeSource,
  sha256Buffer,
}

const ARCHIVE_ENTRY = 'openscad.js'
const CACHE_DIRECTORY = '.open-scad-runtime'
const MAXIMUM_ARCHIVE_BYTES = 32 * 1024 * 1024
const MAXIMUM_RUNTIME_BYTES = 64 * 1024 * 1024
const MAXIMUM_MANIFEST_BYTES = 16 * 1024
const MAXIMUM_FONT_BYTES = 2 * 1024 * 1024
const MAXIMUM_FONT_LICENSE_BYTES = 64 * 1024
const DOWNLOAD_TIMEOUT_MS = 120_000

/** Shared, human-readable metadata for setup output and tests. */
export const OFFICIAL_OPENSCAD_RUNTIME = Object.freeze({
  schemaVersion: 2,
  name: 'OpenSCAD',
  version: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  sourceUrl: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL,
  archiveSha256: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  archiveEntry: ARCHIVE_ENTRY,
  runtimeFilename: OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
  manifestFile: OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  patchVersion: OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  fontUrl: OFFICIAL_OPENSCAD_FONT_URL,
  fontSha256: OFFICIAL_OPENSCAD_FONT_SHA256,
  fontFilename: OFFICIAL_OPENSCAD_FONT_FILENAME,
  fontLicenseUrl: OFFICIAL_OPENSCAD_FONT_LICENSE_URL,
  fontLicenseSha256: OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256,
  fontLicenseFilename: OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  license: 'GPL-2.0-or-later',
  licenseUrl: 'https://github.com/openscad/openscad/blob/master/COPYING',
  cacheDirectory: CACHE_DIRECTORY,
  maximumArchiveBytes: MAXIMUM_ARCHIVE_BYTES,
  maximumRuntimeBytes: MAXIMUM_RUNTIME_BYTES,
})

const EOCD_SIGNATURE = 0x06054b50
const CENTRAL_DIRECTORY_SIGNATURE = 0x02014b50
const LOCAL_FILE_SIGNATURE = 0x04034b50
const MAX_ZIP_COMMENT_BYTES = 65_535

function assertBufferRange(buffer, offset, length, context) {
  if (
    !Number.isSafeInteger(offset)
    || !Number.isSafeInteger(length)
    || offset < 0
    || length < 0
    || offset + length > buffer.length
  ) {
    throw new Error(`Invalid ZIP ${context}: byte range is outside the archive`)
  }
}

function readUInt16(buffer, offset, context) {
  assertBufferRange(buffer, offset, 2, context)
  return buffer.readUInt16LE(offset)
}

function readUInt32(buffer, offset, context) {
  assertBufferRange(buffer, offset, 4, context)
  return buffer.readUInt32LE(offset)
}

function findEndOfCentralDirectory(buffer) {
  const minimumOffset = Math.max(0, buffer.length - 22 - MAX_ZIP_COMMENT_BYTES)
  for (let offset = buffer.length - 22; offset >= minimumOffset; offset -= 1) {
    if (readUInt32(buffer, offset, 'end-of-central-directory signature') !== EOCD_SIGNATURE) continue
    const commentLength = readUInt16(buffer, offset + 20, 'end-of-central-directory comment length')
    if (offset + 22 + commentLength === buffer.length) return offset
  }
  throw new Error('Invalid ZIP archive: end-of-central-directory record was not found')
}

const CRC32_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let index = 0; index < table.length; index += 1) {
    let value = index
    for (let bit = 0; bit < 8; bit += 1) {
      value = (value & 1) === 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
    }
    table[index] = value >>> 0
  }
  return table
})()

function crc32(bytes) {
  let value = 0xffffffff
  for (const byte of bytes) value = CRC32_TABLE[(value ^ byte) & 0xff] ^ (value >>> 8)
  return (value ^ 0xffffffff) >>> 0
}

/** Verify the exact official artifact before parsing attacker-controlled ZIP metadata. */
export function verifyOfficialOpenScadArchive(archive) {
  if (!Buffer.isBuffer(archive)) throw new TypeError('Official OpenSCAD archive must be a Buffer')
  if (archive.length === 0) throw new Error('Official OpenSCAD archive is empty')
  if (archive.length > MAXIMUM_ARCHIVE_BYTES) {
    throw new Error(`Official OpenSCAD archive exceeds ${MAXIMUM_ARCHIVE_BYTES} bytes`)
  }
  const actualSha256 = sha256Buffer(archive)
  if (actualSha256 !== OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256) {
    throw new Error(
      `Official OpenSCAD archive checksum mismatch: expected ${OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256}, received ${actualSha256}`,
    )
  }
  return actualSha256
}

/** Extract exactly one named top-level file in memory; archive paths are never interpreted. */
export function extractSingleZipEntry(archive, expectedEntry, maximumOutputBytes = MAXIMUM_RUNTIME_BYTES) {
  if (!Buffer.isBuffer(archive)) throw new TypeError('ZIP archive must be a Buffer')
  if (
    typeof expectedEntry !== 'string'
    || expectedEntry.length === 0
    || expectedEntry.includes('/')
    || expectedEntry.includes('\\')
    || expectedEntry === '.'
    || expectedEntry === '..'
  ) {
    throw new Error('Expected ZIP entry must be a single safe file name')
  }
  if (!Number.isSafeInteger(maximumOutputBytes) || maximumOutputBytes <= 0) {
    throw new Error('Maximum ZIP output size must be a positive integer')
  }

  const endOffset = findEndOfCentralDirectory(archive)
  const diskNumber = readUInt16(archive, endOffset + 4, 'disk number')
  const centralDirectoryDisk = readUInt16(archive, endOffset + 6, 'central-directory disk')
  const entriesOnDisk = readUInt16(archive, endOffset + 8, 'entries on disk')
  const totalEntries = readUInt16(archive, endOffset + 10, 'total entries')
  const centralDirectoryBytes = readUInt32(archive, endOffset + 12, 'central-directory size')
  const centralDirectoryOffset = readUInt32(archive, endOffset + 16, 'central-directory offset')

  if (diskNumber !== 0 || centralDirectoryDisk !== 0 || entriesOnDisk !== totalEntries) {
    throw new Error('Unsupported multi-disk ZIP archive')
  }
  if (totalEntries !== 1) {
    throw new Error(`Official OpenSCAD ZIP must contain exactly one entry; received ${totalEntries}`)
  }
  if (centralDirectoryOffset + centralDirectoryBytes !== endOffset) {
    throw new Error('Invalid ZIP archive: central-directory bounds do not match')
  }
  assertBufferRange(archive, centralDirectoryOffset, centralDirectoryBytes, 'central directory')

  const centralOffset = centralDirectoryOffset
  if (readUInt32(archive, centralOffset, 'central-directory header') !== CENTRAL_DIRECTORY_SIGNATURE) {
    throw new Error('Invalid ZIP archive: central-directory header was not found')
  }
  const flags = readUInt16(archive, centralOffset + 8, 'entry flags')
  const method = readUInt16(archive, centralOffset + 10, 'compression method')
  const expectedCrc32 = readUInt32(archive, centralOffset + 16, 'entry CRC-32')
  const compressedBytes = readUInt32(archive, centralOffset + 20, 'compressed size')
  const uncompressedBytes = readUInt32(archive, centralOffset + 24, 'uncompressed size')
  const fileNameBytes = readUInt16(archive, centralOffset + 28, 'file-name length')
  const extraBytes = readUInt16(archive, centralOffset + 30, 'extra-field length')
  const commentBytes = readUInt16(archive, centralOffset + 32, 'entry-comment length')
  const entryDisk = readUInt16(archive, centralOffset + 34, 'entry disk')
  const localOffset = readUInt32(archive, centralOffset + 42, 'local-header offset')
  const centralRecordBytes = 46 + fileNameBytes + extraBytes + commentBytes

  if (centralRecordBytes !== centralDirectoryBytes) {
    throw new Error('Official OpenSCAD ZIP central directory contains trailing or truncated data')
  }
  assertBufferRange(archive, centralOffset, centralRecordBytes, 'central-directory entry')
  if ((flags & 0x1) !== 0) throw new Error('Encrypted ZIP entries are not supported')
  if (entryDisk !== 0) throw new Error('Unsupported multi-disk ZIP entry')
  if (method !== 0 && method !== 8) throw new Error(`Unsupported ZIP compression method ${method}`)
  if (uncompressedBytes > maximumOutputBytes) {
    throw new Error(`ZIP entry exceeds ${maximumOutputBytes} uncompressed bytes`)
  }

  const entryName = archive.subarray(centralOffset + 46, centralOffset + 46 + fileNameBytes).toString('utf8')
  if (entryName !== expectedEntry) {
    throw new Error(`Official OpenSCAD ZIP entry must be ${expectedEntry}; received ${entryName}`)
  }
  if (readUInt32(archive, localOffset, 'local-file header') !== LOCAL_FILE_SIGNATURE) {
    throw new Error('Invalid ZIP archive: local-file header was not found')
  }

  const localFlags = readUInt16(archive, localOffset + 6, 'local entry flags')
  const localMethod = readUInt16(archive, localOffset + 8, 'local compression method')
  const localCrc32 = readUInt32(archive, localOffset + 14, 'local CRC-32')
  const localCompressedBytes = readUInt32(archive, localOffset + 18, 'local compressed size')
  const localUncompressedBytes = readUInt32(archive, localOffset + 22, 'local uncompressed size')
  const localNameBytes = readUInt16(archive, localOffset + 26, 'local file-name length')
  const localExtraBytes = readUInt16(archive, localOffset + 28, 'local extra-field length')
  const localNameStart = localOffset + 30
  const dataOffset = localNameStart + localNameBytes + localExtraBytes

  assertBufferRange(archive, localNameStart, localNameBytes, 'local file name')
  assertBufferRange(archive, dataOffset, compressedBytes, 'compressed entry data')
  const localName = archive.subarray(localNameStart, localNameStart + localNameBytes).toString('utf8')
  if (localName !== expectedEntry || localFlags !== flags || localMethod !== method) {
    throw new Error('Invalid ZIP archive: local and central entry metadata differ')
  }
  if (
    (flags & 0x08) === 0
    && (localCrc32 !== expectedCrc32
      || localCompressedBytes !== compressedBytes
      || localUncompressedBytes !== uncompressedBytes)
  ) {
    throw new Error('Invalid ZIP archive: local and central entry integrity metadata differ')
  }
  if (dataOffset + compressedBytes > centralDirectoryOffset) {
    throw new Error('Invalid ZIP archive: entry data overlaps the central directory')
  }

  const compressed = archive.subarray(dataOffset, dataOffset + compressedBytes)
  let extracted
  try {
    extracted = method === 0
      ? Buffer.from(compressed)
      : inflateRawSync(compressed, { maxOutputLength: maximumOutputBytes })
  } catch (error) {
    throw new Error(
      `Unable to decompress official OpenSCAD ZIP entry: ${error instanceof Error ? error.message : String(error)}`,
    )
  }
  if (extracted.length !== uncompressedBytes) {
    throw new Error(`ZIP entry size mismatch: expected ${uncompressedBytes}, received ${extracted.length}`)
  }
  if (crc32(extracted) !== expectedCrc32) throw new Error('ZIP entry CRC-32 mismatch')
  return extracted
}

export function patchOfficialOpenScadRuntime(runtime) {
  if (!Buffer.isBuffer(runtime) && typeof runtime !== 'string') {
    throw new TypeError('Official OpenSCAD runtime must be a Buffer or string')
  }
  let source = runtime
  if (Buffer.isBuffer(runtime)) {
    source = runtime.toString('utf8')
    if (!Buffer.from(source, 'utf8').equals(runtime)) {
      throw new Error('Official OpenSCAD runtime is not valid UTF-8 JavaScript')
    }
  }
  return patchOfficialOpenScadRuntimeSource(source)
}

async function downloadBoundedArtifact(fetchImpl, url, maximumBytes, label) {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), DOWNLOAD_TIMEOUT_MS)
  try {
    const response = await fetchImpl(url, {
      redirect: 'follow',
      signal: controller.signal,
    })
    if (!response.ok) throw new Error(`${label} download failed with HTTP ${response.status}`)
    const contentLength = response.headers.get('content-length')
    if (contentLength !== null) {
      const declaredLength = Number(contentLength)
      if (!Number.isSafeInteger(declaredLength) || declaredLength < 0) {
        throw new Error(`${label} download returned an invalid Content-Length`)
      }
      if (declaredLength > maximumBytes) {
        throw new Error(`${label} download exceeds ${maximumBytes} bytes`)
      }
    }
    if (!response.body) throw new Error(`${label} download did not include a response body`)

    const chunks = []
    let totalBytes = 0
    for await (const chunk of response.body) {
      const bytes = Buffer.from(chunk)
      totalBytes += bytes.length
      if (totalBytes > maximumBytes) {
        throw new Error(`${label} download exceeds ${maximumBytes} bytes`)
      }
      chunks.push(bytes)
    }
    return Buffer.concat(chunks, totalBytes)
  } finally {
    clearTimeout(timeout)
  }
}

async function downloadOfficialOpenScadArchive(fetchImpl) {
  return downloadBoundedArtifact(
    fetchImpl,
    OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_URL,
    MAXIMUM_ARCHIVE_BYTES,
    'Official OpenSCAD archive',
  )
}

function verifyPinnedAsset(bytes, expectedSha256, label) {
  const actualSha256 = sha256Buffer(bytes)
  if (actualSha256 !== expectedSha256) {
    throw new Error(`${label} checksum mismatch: expected ${expectedSha256}, received ${actualSha256}`)
  }
}

export function resolveOfficialOpenScadCacheRoot(cacheRoot) {
  if (cacheRoot !== undefined) return resolve(cacheRoot)
  const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  return join(repositoryRoot, CACHE_DIRECTORY)
}

function assertPathInside(directory, path, label) {
  const relativePath = relative(directory, path)
  if (
    relativePath === ''
    || relativePath === '..'
    || relativePath.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`)
    || isAbsolute(relativePath)
  ) {
    throw new Error(`${label} must resolve inside the official OpenSCAD cache`)
  }
}

function isFileSystemErrorWithCode(error, code) {
  return error instanceof Error && 'code' in error && error.code === code
}

async function assertDirectoryWithoutSymlink(path, label, create) {
  if (create) await mkdir(path, { recursive: true })
  let metadata
  try {
    metadata = await lstat(path)
  } catch (error) {
    if (!create && isFileSystemErrorWithCode(error, 'ENOENT')) throw error
    throw new Error(`Unable to inspect ${label}: ${error instanceof Error ? error.message : String(error)}`)
  }
  if (metadata.isSymbolicLink() || !metadata.isDirectory()) {
    throw new Error(`${label} must be a real directory, not a symlink or another file type`)
  }
}

async function readBoundedRegularFile(path, maximumBytes, label) {
  const metadata = await lstat(path)
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new Error(`${label} must be a regular file, not a symlink or another file type`)
  }
  if (metadata.size <= 0) throw new Error(`${label} is empty`)
  if (metadata.size > maximumBytes) throw new Error(`${label} exceeds ${maximumBytes} bytes`)
  return readFile(path)
}

function installationPaths(cacheRoot) {
  const runtimePath = join(cacheRoot, OFFICIAL_OPENSCAD_RUNTIME_FILENAME)
  const manifestPath = join(cacheRoot, OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME)
  const fontPath = join(cacheRoot, OFFICIAL_OPENSCAD_FONT_FILENAME)
  const fontLicensePath = join(cacheRoot, OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME)
  assertPathInside(cacheRoot, runtimePath, 'Runtime path')
  assertPathInside(cacheRoot, manifestPath, 'Manifest path')
  assertPathInside(cacheRoot, fontPath, 'Font path')
  assertPathInside(cacheRoot, fontLicensePath, 'Font license path')
  return { runtimePath, manifestPath, fontPath, fontLicensePath }
}

function parseAndValidateManifest(rawManifest) {
  let manifest
  try {
    manifest = JSON.parse(rawManifest)
  } catch (error) {
    throw new Error(
      `Official OpenSCAD runtime manifest is invalid JSON: ${error instanceof Error ? error.message : String(error)}`,
    )
  }
  if (!isOfficialOpenScadRuntimeManifest(manifest)) {
    throw new Error('Official OpenSCAD runtime manifest does not match the pinned runtime schema')
  }
  return manifest
}

export async function verifyInstalledOfficialOpenScadRuntime(options = {}) {
  const cacheRoot = resolveOfficialOpenScadCacheRoot(options.cacheRoot)
  const { manifestPath, runtimePath, fontPath, fontLicensePath } = installationPaths(cacheRoot)
  await assertDirectoryWithoutSymlink(cacheRoot, 'Official OpenSCAD cache', false)
  const manifestBytes = await readBoundedRegularFile(
    manifestPath,
    MAXIMUM_MANIFEST_BYTES,
    'Official OpenSCAD runtime manifest',
  )
  const manifest = parseAndValidateManifest(manifestBytes.toString('utf8'))

  const expectedRuntimePath = join(cacheRoot, manifest.runtimeFilename)
  if (expectedRuntimePath !== runtimePath) {
    throw new Error('Official OpenSCAD runtime manifest resolves to an unexpected runtime path')
  }
  const runtime = await readBoundedRegularFile(runtimePath, MAXIMUM_RUNTIME_BYTES, 'Official OpenSCAD runtime')
  const actualRuntimeSha256 = sha256Buffer(runtime)
  if (actualRuntimeSha256 !== manifest.runtimeSha256) {
    throw new Error(
      `Official OpenSCAD runtime checksum mismatch: expected ${manifest.runtimeSha256}, received ${actualRuntimeSha256}`,
    )
  }
  if (join(cacheRoot, manifest.fontFilename) !== fontPath
    || join(cacheRoot, manifest.fontLicenseFilename) !== fontLicensePath) {
    throw new Error('Official OpenSCAD manifest resolves to unexpected font asset paths')
  }
  const font = await readBoundedRegularFile(fontPath, MAXIMUM_FONT_BYTES, 'Official OpenSCAD default font')
  const fontLicense = await readBoundedRegularFile(
    fontLicensePath,
    MAXIMUM_FONT_LICENSE_BYTES,
    'Official OpenSCAD default font license',
  )
  verifyPinnedAsset(font, manifest.fontSha256, 'Official OpenSCAD default font')
  verifyPinnedAsset(fontLicense, manifest.fontLicenseSha256, 'Official OpenSCAD default font license')
  return { cacheRoot, manifestPath, runtimePath, fontPath, fontLicensePath, manifest }
}

export async function getOfficialOpenScadRuntimeStatus(options = {}) {
  const cacheRoot = resolveOfficialOpenScadCacheRoot(options.cacheRoot)
  const { manifestPath } = installationPaths(cacheRoot)
  try {
    await lstat(manifestPath)
  } catch (error) {
    if (isFileSystemErrorWithCode(error, 'ENOENT')) return { status: 'missing', cacheRoot, manifestPath }
    return {
      status: 'invalid',
      cacheRoot,
      manifestPath,
      error: error instanceof Error ? error.message : String(error),
    }
  }
  try {
    return { status: 'installed', ...(await verifyInstalledOfficialOpenScadRuntime({ cacheRoot })) }
  } catch (error) {
    return {
      status: 'invalid',
      cacheRoot,
      manifestPath,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

export async function installOfficialOpenScadRuntime(options = {}) {
  const cacheRoot = resolveOfficialOpenScadCacheRoot(options.cacheRoot)
  const existing = await getOfficialOpenScadRuntimeStatus({ cacheRoot })
  if (existing.status === 'installed') return { ...existing, installState: 'already-installed' }

  const fetchImpl = options.fetchImpl ?? globalThis.fetch
  if (typeof fetchImpl !== 'function') throw new Error('This Node.js runtime does not provide fetch')
  const [archive, font, fontLicense] = await Promise.all([
    downloadOfficialOpenScadArchive(fetchImpl),
    downloadBoundedArtifact(
      fetchImpl,
      OFFICIAL_OPENSCAD_FONT_URL,
      MAXIMUM_FONT_BYTES,
      'Official OpenSCAD default font',
    ),
    downloadBoundedArtifact(
      fetchImpl,
      OFFICIAL_OPENSCAD_FONT_LICENSE_URL,
      MAXIMUM_FONT_LICENSE_BYTES,
      'Official OpenSCAD default font license',
    ),
  ])
  verifyOfficialOpenScadArchive(archive)
  verifyPinnedAsset(font, OFFICIAL_OPENSCAD_FONT_SHA256, 'Official OpenSCAD default font')
  verifyPinnedAsset(
    fontLicense,
    OFFICIAL_OPENSCAD_FONT_LICENSE_SHA256,
    'Official OpenSCAD default font license',
  )
  const originalRuntime = extractSingleZipEntry(archive, ARCHIVE_ENTRY)
  const patchedRuntime = patchOfficialOpenScadRuntime(originalRuntime)
  const manifest = createOfficialOpenScadRuntimeManifest(patchedRuntime, font, fontLicense)
  const { runtimePath, manifestPath, fontPath, fontLicensePath } = installationPaths(cacheRoot)

  await assertDirectoryWithoutSymlink(cacheRoot, 'Official OpenSCAD cache', true)
  const nonce = `${process.pid}-${randomUUID()}`
  const runtimeTemporaryPath = join(cacheRoot, `.${OFFICIAL_OPENSCAD_RUNTIME_FILENAME}.${nonce}.tmp`)
  const manifestTemporaryPath = join(cacheRoot, `.${OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME}.${nonce}.tmp`)
  const fontTemporaryPath = join(cacheRoot, `.${OFFICIAL_OPENSCAD_FONT_FILENAME}.${nonce}.tmp`)
  const fontLicenseTemporaryPath = join(cacheRoot, `.${OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME}.${nonce}.tmp`)
  assertPathInside(cacheRoot, runtimeTemporaryPath, 'Temporary runtime path')
  assertPathInside(cacheRoot, manifestTemporaryPath, 'Temporary manifest path')
  assertPathInside(cacheRoot, fontTemporaryPath, 'Temporary font path')
  assertPathInside(cacheRoot, fontLicenseTemporaryPath, 'Temporary font license path')

  try {
    await writeFile(runtimeTemporaryPath, patchedRuntime, { encoding: 'utf8', flag: 'wx', mode: 0o600 })
    await writeFile(fontTemporaryPath, font, { flag: 'wx', mode: 0o600 })
    await writeFile(fontLicenseTemporaryPath, fontLicense, { flag: 'wx', mode: 0o600 })
    await writeFile(manifestTemporaryPath, `${JSON.stringify(manifest, null, 2)}\n`, {
      encoding: 'utf8',
      flag: 'wx',
      mode: 0o600,
    })
    await rename(runtimeTemporaryPath, runtimePath)
    await rename(fontTemporaryPath, fontPath)
    await rename(fontLicenseTemporaryPath, fontLicensePath)
    await rename(manifestTemporaryPath, manifestPath)
  } finally {
    await Promise.all([
      rm(runtimeTemporaryPath, { force: true }),
      rm(manifestTemporaryPath, { force: true }),
      rm(fontTemporaryPath, { force: true }),
      rm(fontLicenseTemporaryPath, { force: true }),
    ])
  }

  const verified = await verifyInstalledOfficialOpenScadRuntime({ cacheRoot })
  return { status: 'installed', ...verified, installState: existing.status === 'invalid' ? 'repaired' : 'installed' }
}

function printUsage(log) {
  log(`Usage:
  npm run setup:openscad
  npm run verify:openscad-runtime
  npm run status:openscad-runtime

Direct invocation (tsx loader required):
  node --import tsx scripts/official-openscad-runtime.mjs install|verify|status

The install command explicitly downloads the pinned official OpenSCAD ${OFFICIAL_OPENSCAD_RUNTIME_VERSION}
WebAssembly Node snapshot, verifies its SHA-256 checksum, and installs a patched
MEMFS-only CommonJS runtime in ${CACHE_DIRECTORY}/. Normal builds and tests never download it.`)
}

export async function runOfficialOpenScadRuntimeCli(argv, options = {}) {
  const log = options.log ?? console.log
  const errorLog = options.errorLog ?? console.error
  const cacheRoot = options.cacheRoot
  const command = argv.length === 1 ? argv[0].replace(/^--/u, '') : undefined

  if (command === 'help' || (argv.length === 1 && argv[0] === '-h')) {
    printUsage(log)
    return 0
  }
  if (!command || !['install', 'verify', 'status'].includes(command)) {
    printUsage(log)
    return 2
  }
  if (command === 'install') {
    const result = await installOfficialOpenScadRuntime({ cacheRoot, fetchImpl: options.fetchImpl })
    const verb = result.installState === 'already-installed'
      ? 'Already verified'
      : result.installState === 'repaired'
        ? 'Repaired and verified'
        : 'Installed and verified'
    log(`${verb} official OpenSCAD ${result.manifest.runtimeVersion} runtime at ${result.runtimePath}`)
    log(`Pinned source archive SHA-256 ${result.manifest.archiveSha256}`)
    log(`Installed runtime SHA-256 ${result.manifest.runtimeSha256}`)
    return 0
  }
  if (command === 'verify') {
    const result = await verifyInstalledOfficialOpenScadRuntime({ cacheRoot })
    log(`Verified official OpenSCAD ${result.manifest.runtimeVersion} runtime at ${result.runtimePath}`)
    log(`Installed runtime SHA-256 ${result.manifest.runtimeSha256}`)
    return 0
  }

  const status = await getOfficialOpenScadRuntimeStatus({ cacheRoot })
  if (status.status === 'installed') {
    log(`Official OpenSCAD ${status.manifest.runtimeVersion} runtime is installed and verified at ${status.runtimePath}`)
    return 0
  }
  if (status.status === 'missing') {
    errorLog(`Official OpenSCAD runtime is not installed in ${status.cacheRoot}`)
    return 1
  }
  errorLog(`Official OpenSCAD runtime installation is invalid: ${status.error}`)
  return 1
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : undefined
if (invokedPath === import.meta.url) {
  runOfficialOpenScadRuntimeCli(process.argv.slice(2)).then(
    exitCode => { process.exitCode = exitCode },
    error => {
      console.error(error instanceof Error ? error.message : String(error))
      process.exitCode = 1
    },
  )
}
