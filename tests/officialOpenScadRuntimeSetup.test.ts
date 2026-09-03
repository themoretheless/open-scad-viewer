import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { deflateRawSync } from 'node:zlib'
import { afterEach, describe, expect, it, vi } from 'vitest'
import * as runtimeContract from '../src/mcp/officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_RUNTIME,
  createOfficialOpenScadRuntimeManifest,
  extractSingleZipEntry,
  getOfficialOpenScadRuntimeStatus,
  installOfficialOpenScadRuntime,
  patchOfficialOpenScadRuntime,
  runOfficialOpenScadRuntimeCli,
  verifyInstalledOfficialOpenScadRuntime,
  verifyOfficialOpenScadArchive,
} from '../scripts/official-openscad-runtime.mjs'

const fixtureUrl = new URL('./fixtures/official-openscad-runtime.js.txt', import.meta.url)
const temporaryDirectories: string[] = []

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map(path => rm(path, { recursive: true, force: true })))
})

async function temporaryDirectory(): Promise<string> {
  const path = await mkdtemp(join(tmpdir(), 'open-scad-runtime-test-'))
  temporaryDirectories.push(path)
  return path
}

async function fixtureRuntime(): Promise<Buffer> {
  const fixture = await readFile(fixtureUrl, 'utf8')
  return Buffer.from(fixture.padEnd(1_000_001, ' '), 'utf8')
}

function crc32(bytes: Buffer): number {
  let value = 0xffffffff
  for (const byte of bytes) {
    value ^= byte
    for (let bit = 0; bit < 8; bit += 1) {
      value = (value & 1) === 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
    }
  }
  return (value ^ 0xffffffff) >>> 0
}

function createZipFixture(name: string, contents: Buffer, method: 0 | 8 = 8): Buffer {
  const nameBytes = Buffer.from(name)
  const compressed = method === 8 ? deflateRawSync(contents) : contents
  const checksum = crc32(contents)
  const local = Buffer.alloc(30 + nameBytes.length)
  local.writeUInt32LE(0x04034b50, 0)
  local.writeUInt16LE(20, 4)
  local.writeUInt16LE(0, 6)
  local.writeUInt16LE(method, 8)
  local.writeUInt32LE(checksum, 14)
  local.writeUInt32LE(compressed.length, 18)
  local.writeUInt32LE(contents.length, 22)
  local.writeUInt16LE(nameBytes.length, 26)
  nameBytes.copy(local, 30)

  const central = Buffer.alloc(46 + nameBytes.length)
  central.writeUInt32LE(0x02014b50, 0)
  central.writeUInt16LE(20, 4)
  central.writeUInt16LE(20, 6)
  central.writeUInt16LE(0, 8)
  central.writeUInt16LE(method, 10)
  central.writeUInt32LE(checksum, 16)
  central.writeUInt32LE(compressed.length, 20)
  central.writeUInt32LE(contents.length, 24)
  central.writeUInt16LE(nameBytes.length, 28)
  nameBytes.copy(central, 46)

  const end = Buffer.alloc(22)
  end.writeUInt32LE(0x06054b50, 0)
  end.writeUInt16LE(1, 8)
  end.writeUInt16LE(1, 10)
  end.writeUInt32LE(central.length, 12)
  end.writeUInt32LE(local.length + compressed.length, 16)
  return Buffer.concat([local, compressed, central, end])
}

async function writeVerifiedFixtureInstallation(cacheRoot: string) {
  const original = await fixtureRuntime()
  const patched = patchOfficialOpenScadRuntime(original)
  const font = Buffer.from('fixture Basic font')
  const fontLicense = Buffer.from('fixture OFL license')
  const manifest = createOfficialOpenScadRuntimeManifest(patched, font, fontLicense)
  const runtimePath = join(cacheRoot, runtimeContract.OFFICIAL_OPENSCAD_RUNTIME_FILENAME)
  const manifestPath = join(cacheRoot, runtimeContract.OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME)
  const fontPath = join(cacheRoot, runtimeContract.OFFICIAL_OPENSCAD_FONT_FILENAME)
  const fontLicensePath = join(cacheRoot, runtimeContract.OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME)
  await mkdir(cacheRoot, { recursive: true })
  await writeFile(runtimePath, patched)
  await writeFile(fontPath, font)
  await writeFile(fontLicensePath, fontLicense)
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
  return { fontPath, fontLicensePath, manifest, manifestPath, original, patched, runtimePath }
}

describe('official OpenSCAD runtime setup', () => {
  it('shares the exact pinned artifact, patch, filename, and manifest contract with the MCP adapter', async () => {
    const original = await fixtureRuntime()
    const patchedBySetup = patchOfficialOpenScadRuntime(original)
    const patchedByAdapterContract = runtimeContract.patchOfficialOpenScadRuntimeSource(original.toString('utf8'))
    const manifest = createOfficialOpenScadRuntimeManifest(patchedBySetup)

    expect(OFFICIAL_OPENSCAD_RUNTIME).toMatchObject({
      version: '2026.09.01',
      sourceUrl: 'https://files.openscad.org/snapshots/OpenSCAD-2026.09.01-WebAssembly-node.zip',
      archiveSha256: '82054dfb4911686de0ee3ea36771dbf81f3d014c3460c8ea069ab4f933f6d888',
      archiveEntry: 'openscad.js',
      runtimeFilename: 'openscad.patched.cjs',
      manifestFile: 'runtime-manifest.json',
      patchVersion: 2,
    })
    expect(patchedBySetup).toBe(patchedByAdapterContract)
    expect(manifest).toEqual(runtimeContract.createOfficialOpenScadRuntimeManifest(patchedByAdapterContract))
    expect(runtimeContract.isOfficialOpenScadRuntimeManifest(manifest)).toBe(true)
  })

  it.each([0, 8] as const)('extracts one bounded CRC-checked entry using ZIP method %i', method => {
    const contents = Buffer.from('official runtime fixture')
    const archive = createZipFixture('openscad.js', contents, method)

    expect(extractSingleZipEntry(archive, 'openscad.js')).toEqual(contents)
    expect(() => extractSingleZipEntry(archive, 'other.js')).toThrow(/must be other\.js/)
  })

  it('rejects corrupt entry bytes and contradictory local metadata', () => {
    const archive = createZipFixture('openscad.js', Buffer.from('official runtime fixture'))
    const corrupt = Buffer.from(archive)
    const dataOffset = 30 + Buffer.byteLength('openscad.js')
    corrupt[dataOffset] ^= 0xff
    expect(() => extractSingleZipEntry(corrupt, 'openscad.js')).toThrow(/decompress|CRC-32|size mismatch/i)

    const contradictory = Buffer.from(archive)
    contradictory.writeUInt32LE(123, 18)
    expect(() => extractSingleZipEntry(contradictory, 'openscad.js')).toThrow(/integrity metadata differ/)
  })

  it('rejects unsafe ZIP shapes, entry names, and output sizes', () => {
    expect(() => extractSingleZipEntry(Buffer.alloc(0), '../openscad.js')).toThrow(/single safe file name/)
    expect(() => extractSingleZipEntry(createZipFixture('../openscad.js', Buffer.from('x')), 'openscad.js')).toThrow(/must be openscad\.js/)
    expect(() => extractSingleZipEntry(createZipFixture('openscad.js', Buffer.from('oversized')), 'openscad.js', 2)).toThrow(/exceeds 2/)

    const extraEntryClaim = createZipFixture('openscad.js', Buffer.from('x'))
    extraEntryClaim.writeUInt16LE(2, extraEntryClaim.length - 12)
    extraEntryClaim.writeUInt16LE(2, extraEntryClaim.length - 14)
    expect(() => extractSingleZipEntry(extraEntryClaim, 'openscad.js')).toThrow(/exactly one entry/)
  })

  it('applies the canonical MEMFS/NODERAWFS patch deterministically and fail-closed', async () => {
    const fixture = await fixtureRuntime()
    const first = patchOfficialOpenScadRuntime(fixture)
    const second = patchOfficialOpenScadRuntime(Buffer.from(fixture))

    expect(first).toBe(second)
    expect(first).toContain('open-scad-viewer-official-runtime-patch-v2')
    expect(first).toContain('var Module=globalThis.__OPENSCAD_MODULE__')
    expect(first).toContain('var ENV=Module["environment"]||{}')
    expect(first).toContain('Module["useNodeRawFS"]===true')
    expect(() => patchOfficialOpenScadRuntime(first)).toThrow(/already patched/)
    const missingSentinel = fixture.toString('utf8')
      .replace('NODEFS.staticInit()', 'NODEFS.changed()')
      .padEnd(1_000_001, ' ')
    expect(() => patchOfficialOpenScadRuntime(missingSentinel)).toThrow(/sentinel occurred 0 times/)
  })

  it('fails a checksum mismatch before attempting ZIP extraction', () => {
    const archive = createZipFixture('not-openscad.js', Buffer.from('not the pinned artifact'))
    expect(() => verifyOfficialOpenScadArchive(archive)).toThrow(/checksum mismatch/)
  })

  it('verifies the flat canonical manifest and colocated CommonJS runtime', async () => {
    const cacheRoot = await temporaryDirectory()
    const installation = await writeVerifiedFixtureInstallation(cacheRoot)

    expect(installation.manifest).toMatchObject({
      schemaVersion: 2,
      runtimeVersion: '2026.09.01',
      runtimeFilename: 'openscad.patched.cjs',
      patchVersion: 2,
      fontFilename: 'Basic-Regular.ttf',
      fontLicenseFilename: 'Basic-OFL.txt',
    })
    await expect(verifyInstalledOfficialOpenScadRuntime({ cacheRoot })).resolves.toMatchObject({
      cacheRoot,
      manifestPath: installation.manifestPath,
      runtimePath: installation.runtimePath,
      fontPath: installation.fontPath,
      fontLicensePath: installation.fontLicensePath,
      manifest: installation.manifest,
    })
    await expect(getOfficialOpenScadRuntimeStatus({ cacheRoot })).resolves.toMatchObject({ status: 'installed' })

    await writeFile(installation.runtimePath, 'tampered')
    await expect(verifyInstalledOfficialOpenScadRuntime({ cacheRoot })).rejects.toThrow(/checksum mismatch/)
    await expect(getOfficialOpenScadRuntimeStatus({ cacheRoot })).resolves.toMatchObject({ status: 'invalid' })
  })

  it('rejects unexpected manifest fields and symlinked runtime files', async () => {
    const cacheRoot = await temporaryDirectory()
    const installation = await writeVerifiedFixtureInstallation(cacheRoot)
    await writeFile(installation.manifestPath, `${JSON.stringify({ ...installation.manifest, runtimePath: '../escape.cjs' })}\n`)
    await expect(verifyInstalledOfficialOpenScadRuntime({ cacheRoot })).rejects.toThrow(/pinned runtime schema/)

    await writeFile(installation.manifestPath, `${JSON.stringify(installation.manifest)}\n`)
    const target = join(await temporaryDirectory(), 'target.cjs')
    await writeFile(target, installation.patched)
    await rm(installation.runtimePath)
    await symlink(target, installation.runtimePath)
    await expect(verifyInstalledOfficialOpenScadRuntime({ cacheRoot })).rejects.toThrow(/regular file, not a symlink/)
  })

  it('makes repeated install idempotent without downloading an already verified runtime', async () => {
    const cacheRoot = await temporaryDirectory()
    const installation = await writeVerifiedFixtureInstallation(cacheRoot)
    const fetchImpl = vi.fn(() => { throw new Error('network must not be used') })

    await expect(installOfficialOpenScadRuntime({ cacheRoot, fetchImpl })).resolves.toMatchObject({
      installState: 'already-installed',
      runtimePath: installation.runtimePath,
    })
    expect(fetchImpl).not.toHaveBeenCalled()
    await expect(verifyInstalledOfficialOpenScadRuntime({ cacheRoot })).resolves.toMatchObject({
      runtimePath: installation.runtimePath,
    })
  })

  it('reports missing, installed, and invalid states through the CLI contract', async () => {
    const cacheRoot = join(await temporaryDirectory(), 'cache')
    const log = vi.fn()
    const errorLog = vi.fn()

    await expect(runOfficialOpenScadRuntimeCli(['status'], { cacheRoot, log, errorLog })).resolves.toBe(1)
    expect(errorLog).toHaveBeenLastCalledWith(expect.stringContaining('not installed'))

    await writeVerifiedFixtureInstallation(cacheRoot)
    await expect(runOfficialOpenScadRuntimeCli(['verify'], { cacheRoot, log, errorLog })).resolves.toBe(0)
    await expect(runOfficialOpenScadRuntimeCli(['--status'], { cacheRoot, log, errorLog })).resolves.toBe(0)
    expect(log).toHaveBeenCalledWith(expect.stringContaining('installed and verified'))

    await writeFile(join(cacheRoot, runtimeContract.OFFICIAL_OPENSCAD_RUNTIME_FILENAME), 'tampered')
    await expect(runOfficialOpenScadRuntimeCli(['status'], { cacheRoot, log, errorLog })).resolves.toBe(1)
    expect(errorLog).toHaveBeenLastCalledWith(expect.stringContaining('installation is invalid'))
  })

  it('returns help and usage errors without touching the filesystem', async () => {
    const log = vi.fn()
    await expect(runOfficialOpenScadRuntimeCli(['--help'], { log })).resolves.toBe(0)
    await expect(runOfficialOpenScadRuntimeCli([], { log })).resolves.toBe(2)
    await expect(runOfficialOpenScadRuntimeCli(['unknown'], { log })).resolves.toBe(2)
    expect(log).toHaveBeenCalledWith(expect.stringContaining('npm run setup:openscad'))
  })
})
