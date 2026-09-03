import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import {
  OFFICIAL_OPENSCAD_RUNTIME_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  createOfficialOpenScadRuntimeManifest,
} from '../src/mcp/officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
  OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
} from '../src/mcp/officialOpenScadRuntimeProtocol'
import {
  OfficialOpenScadRemoteError,
  OfficialOpenScadRuntimeSupervisor,
  OfficialOpenScadSupervisorError,
} from '../src/mcp/officialOpenScadRuntimeService'

const stubRuntimeUrl = new URL('./fixtures/official-openscad-runtime-stub.cjs', import.meta.url)
const temporaryRoots: string[] = []

async function createRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'official-openscad-service-'))
  temporaryRoots.push(root)
  return root
}

async function installStub(root: string) {
  const runtime = await readFile(stubRuntimeUrl)
  const font = Buffer.from('fixture Basic Regular font')
  const fontLicense = Buffer.from('fixture OFL license')
  const manifest = createOfficialOpenScadRuntimeManifest(runtime, font, fontLicense)
  await writeFile(join(root, OFFICIAL_OPENSCAD_RUNTIME_FILENAME), runtime)
  await writeFile(join(root, OFFICIAL_OPENSCAD_FONT_FILENAME), font)
  await writeFile(join(root, OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME), fontLicense)
  await writeFile(
    join(root, OFFICIAL_OPENSCAD_RUNTIME_MANIFEST_FILENAME),
    `${JSON.stringify(manifest)}\n`,
  )
  return {
    runtime,
    expectedIntegrity: {
      runtimeSha256: manifest.runtimeSha256,
      fontSha256: manifest.fontSha256,
      fontLicenseSha256: manifest.fontLicenseSha256,
    },
  }
}

afterEach(async () => {
  await Promise.all(temporaryRoots.splice(0).map(root => rm(root, { recursive: true, force: true })))
})

describe('official OpenSCAD runtime supervisor', () => {
  it('reports opt-in installation state and verifies the runtime hash without executing it', async () => {
    const root = await createRoot()
    const missingSupervisor = new OfficialOpenScadRuntimeSupervisor({ cacheRoot: root })
    await expect(missingSupervisor.capabilities()).resolves.toMatchObject({
      available: false,
      unavailableReason: 'not-installed',
      expectedRuntimeVersion: '2026.09.01',
      runtimeVersion: null,
      setupCommand: 'npm run setup:openscad',
      isolation: {
        filesystem: 'MEMFS',
        nodePermissionModel: true,
        scadHostFileRead: false,
        scadHostFileWrite: false,
        networkApiExposed: false,
        networkSandboxEnforced: false,
        wasmMemoryLimitEnforced: false,
        processModel: 'one-shot',
      },
    })
    await missingSupervisor.close()

    const installation = await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({
      cacheRoot: root,
      testOnlyExpectedIntegrity: installation.expectedIntegrity,
    })
    const available = await supervisor.capabilities()
    expect(available).toMatchObject({
      available: true,
      unavailableReason: null,
      runtimeVersion: '2026.09.01',
      patchVersion: 2,
      defaultFont: { family: 'Basic', filename: 'Basic-Regular.ttf' },
      defaults: { backend: 'Manifold', experimentsEnabled: false },
    })
    expect(available.runtimeSha256).toMatch(/^[0-9a-f]{64}$/)

    await writeFile(
      join(root, OFFICIAL_OPENSCAD_RUNTIME_FILENAME),
      Buffer.concat([installation.runtime, Buffer.from('tamper')]),
    )
    await expect(supervisor.capabilities()).resolves.toMatchObject({
      available: false,
      unavailableReason: 'runtime-integrity-failed',
    })
    expect(supervisor.snapshot()).toMatchObject({ jobsStarted: 0, jobsJoined: 0 })
    await supervisor.close()
  })

  it('does not trust a self-consistent manifest with unpinned runtime and asset digests', async () => {
    const root = await createRoot()
    await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({ cacheRoot: root })

    await expect(supervisor.capabilities()).resolves.toMatchObject({
      available: false,
      unavailableReason: 'manifest-invalid',
    })
    expect(supervisor.snapshot()).toMatchObject({ jobsStarted: 0, jobsJoined: 0 })
    await supervisor.close()
  })

  it('runs semantic CSG checks and strict exports in fresh permission-model children', async () => {
    const root = await createRoot()
    const installation = await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({
      cacheRoot: root,
      testOnlyExpectedIntegrity: installation.expectedIntegrity,
    })

    const checked = await supervisor.check(
      'include <lib/helpers.scad>; function f(x) = x * x; cube(f(size));',
      { files: [{ path: 'lib/helpers.scad', data: 'module helper() {}' }] },
    )
    expect(checked).toMatchObject({
      runtimeVersion: '2026.09.01',
      csgBytes: expect.any(Number),
      experimentalFeatures: [],
      logs: { truncated: false },
    })
    expect(checked.csgBytes).toBeGreaterThan(0)

    const exported = await supervisor.export('cube(size);', 'csg', {
      files: [{ path: 'data/raw.bin', data: new Uint8Array([0, 1, 2, 255]) }],
      defines: ['size=7'],
      time: 0.25,
      backend: 'CGAL',
      hardWarnings: false,
      checkParameters: false,
      checkParameterRanges: false,
      experimentalFeatures: ['textmetrics', 'vector-swizzle'],
    })
    const childEvidence = JSON.parse(new TextDecoder().decode(exported.data)) as {
      args: string[]
      projectFiles: string[]
      rawFs: boolean
      environment: Record<string, string>
    }
    expect(exported).toMatchObject({
      format: 'csg',
      mimeType: 'text/x-openscad-csg',
      fileName: 'model.csg',
      experimentalFeatures: ['textmetrics', 'vector-swizzle'],
    })
    expect(childEvidence.args).toEqual(expect.arrayContaining([
      '--backend=CGAL',
      '--check-parameters=false',
      '--check-parameter-ranges=false',
      '--enable=textmetrics',
      '--enable=vector-swizzle',
      'size=7',
      '$t=0.25',
      '--export-format=csg',
    ]))
    expect(childEvidence.args).not.toContain('--hardwarnings')
    expect(childEvidence.projectFiles).toEqual(expect.arrayContaining([
      '/project/main.scad',
      '/project/data/raw.bin',
    ]))
    expect(childEvidence.rawFs).toBe(false)
    expect(childEvidence.environment).toEqual({
      FONTCONFIG_FILE: '/project/fonts/fonts.conf',
      FONTCONFIG_PATH: '/project/fonts',
      HOME: '/project/home',
    })
    expect(childEvidence.projectFiles).toEqual(expect.arrayContaining([
      '/project/fonts/fonts.conf',
      '/project/fonts/Basic-Regular.ttf',
    ]))
    expect(supervisor.snapshot()).toEqual({
      activeJobs: 0,
      jobsStarted: 2,
      jobsJoined: 2,
      closing: false,
      closed: false,
    })
    await supervisor.close()
  })

  it('keeps experiments off and semantic warnings fatal by default', async () => {
    const root = await createRoot()
    const installation = await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({
      cacheRoot: root,
      testOnlyExpectedIntegrity: installation.expectedIntegrity,
    })

    const exported = await supervisor.export('cube(1);', 'csg')
    const evidence = JSON.parse(new TextDecoder().decode(exported.data)) as { args: string[] }
    expect(evidence.args.some(argument => argument.startsWith('--enable='))).toBe(false)
    expect(evidence.args).toContain('--hardwarnings')
    expect(evidence.args).toContain('--backend=Manifold')

    let failure: unknown
    try {
      await supervisor.check('UNKNOWN_MODULE;')
    } catch (error) {
      failure = error
    }
    expect(failure).toBeInstanceOf(OfficialOpenScadRemoteError)
    expect(failure).toMatchObject({
      code: 'E_OPENSCAD_COMPILE',
      logs: { stderr: [expect.stringContaining('unknown module')] },
    })
    await supervisor.close()
  })

  it('denies host filesystem reads in the child permission boundary', async () => {
    const root = await createRoot()
    const installation = await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({
      cacheRoot: root,
      testOnlyExpectedIntegrity: installation.expectedIntegrity,
    })

    await expect(supervisor.export('TRY_HOST_READ;', 'csg')).rejects.toMatchObject({
      code: 'ERR_ACCESS_DENIED',
      message: expect.stringMatching(/access|permission|restricted/i),
    })
    expect(supervisor.snapshot()).toMatchObject({ jobsStarted: 1, jobsJoined: 1, activeJobs: 0 })
    await supervisor.close()
  })

  it('hard-kills and joins non-cooperative children on deadline and cancellation', async () => {
    const root = await createRoot()
    const installation = await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({
      cacheRoot: root,
      testOnlyExpectedIntegrity: installation.expectedIntegrity,
    })

    await expect(supervisor.check('HANG_FOREVER;', { timeoutMs: 40 })).rejects.toMatchObject({
      code: 'E_OFFICIAL_OPENSCAD_DEADLINE',
      jobId: 1,
    })
    expect(supervisor.snapshot()).toMatchObject({ activeJobs: 0, jobsJoined: 1 })

    const controller = new AbortController()
    const cancelled = supervisor.check('HANG_FOREVER;', {
      signal: controller.signal,
      timeoutMs: 2_000,
    })
    setTimeout(() => controller.abort(), 40)
    await expect(cancelled).rejects.toMatchObject({
      code: 'E_OFFICIAL_OPENSCAD_CANCELLED',
      jobId: 2,
    })
    expect(supervisor.snapshot()).toMatchObject({ activeJobs: 0, jobsStarted: 2, jobsJoined: 2 })
    await supervisor.close()
  })

  it('bounds admission, project paths, source size, and retained logs', async () => {
    const root = await createRoot()
    const installation = await installStub(root)
    const supervisor = new OfficialOpenScadRuntimeSupervisor({
      cacheRoot: root,
      maxConcurrentJobs: 1,
      testOnlyExpectedIntegrity: installation.expectedIntegrity,
    })

    await expect(supervisor.check('cube(1);', {
      files: [{ path: '../host.scad', data: 'cube(1);' }],
    })).rejects.toThrow(/path|segments/)
    await expect(supervisor.check('cube(1);', {
      files: [
        { path: 'lib', data: 'not a directory' },
        { path: 'lib/part.scad', data: 'cube(1);' },
      ],
    })).rejects.toThrow(/conflicting/i)
    await expect(supervisor.check('cube(1);', {
      files: [{ path: 'home/cache.dat', data: 'reserved runtime state' }],
    })).rejects.toThrow(/reserved/i)
    await expect(supervisor.check('x'.repeat(1_048_577))).rejects.toThrow(/exceeds/)
    expect(supervisor.snapshot()).toMatchObject({ jobsStarted: 0, jobsJoined: 0 })

    const maximumFile = new Uint8Array(OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES)
    const remainingFile = new Uint8Array(
      OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES
        - OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES
        - OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
    )
    await expect(supervisor.check('\0'.repeat(OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES), {
      files: [
        { path: 'large-a.bin', data: maximumFile },
        { path: 'large-b.bin', data: remainingFile },
      ],
    })).rejects.toMatchObject({ code: 'E_OFFICIAL_OPENSCAD_REQUEST_LIMIT' })
    expect(supervisor.snapshot()).toMatchObject({ jobsStarted: 0, jobsJoined: 0 })

    const controller = new AbortController()
    const active = supervisor.check('HANG_FOREVER;', { signal: controller.signal, timeoutMs: 2_000 })
    await expect(supervisor.check('cube(1);')).rejects.toBeInstanceOf(OfficialOpenScadSupervisorError)
    controller.abort()
    await expect(active).rejects.toMatchObject({ code: 'E_OFFICIAL_OPENSCAD_CANCELLED' })

    const noisy = await supervisor.export('NOISY_RUNTIME;', 'csg')
    expect(noisy.logs.truncated).toBe(true)
    expect(noisy.logs.stdout.length + noisy.logs.stderr.length).toBeLessThanOrEqual(128)
    const retainedBytes = [...noisy.logs.stdout, ...noisy.logs.stderr]
      .reduce((sum, value) => sum + Buffer.byteLength(value), 0)
    expect(retainedBytes).toBeLessThanOrEqual(64 * 1024)
    await supervisor.close()
  })
})
