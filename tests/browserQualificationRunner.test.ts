import { spawnSync } from 'node:child_process'
import { readdirSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { describe, expect, it, vi } from 'vitest'

import {
  BoundedQualificationLog,
  BrowserLateSettlementQuarantine,
  BrowserQualificationRunnerError,
  EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES,
  EXPECTED_PLAYWRIGHT_VERSION,
  REQUIRED_PLAYWRIGHT_DEPENDENCY,
  acquireBrowserResource,
  buildQualificationBundle,
  installQualificationNetworkIsolation,
  isQualificationNetworkUrlAllowed,
  isQualificationWebSocketUrlAllowed,
  parseBrowserQualificationArguments,
  preflightPlaywright,
  runBrowserQualification,
  startQualificationPreview,
  validateBuildArtifactByteBudget,
  validateQualificationPayload,
} from '../scripts/run-browser-qualification.mjs'
import {
  QUALIFICATION_PLAYWRIGHT_PACKAGE,
  assertIsolatedPackageResolution,
  assertIsolatedPlaywrightResolution,
  inspectQualificationPlaywrightPackage,
} from '../scripts/qualificationPlaywrightPackage.mjs'

const repositoryRoot = resolve(import.meta.dirname, '..')
const runnerPath = resolve(repositoryRoot, 'scripts/run-browser-qualification.mjs')

function fakeBrowserType(name = 'chromium') {
  return {
    name: () => name,
    executablePath: () => '/managed/browser',
    launch: vi.fn(),
  }
}

function fakeQualificationPackage(playwright: object, version = EXPECTED_PLAYWRIGHT_VERSION) {
  return {
    playwright,
    manifest: { name: 'playwright', version },
    packageMetadata: {
      packageRoot: 'tools/browser-qualification',
      packageJsonPath: 'tools/browser-qualification/package.json',
      packageLockPath: 'tools/browser-qualification/package-lock.json',
      packageJsonSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256,
      packageLockSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256,
      licenseManifestPath: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestPath,
      licenseManifestSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256,
      playwrightVersion: version,
    },
  }
}

describe('actual-browser qualification runner', () => {
  it('admits exactly one named Playwright engine and bounded orchestration options', () => {
    expect(parseBrowserQualificationArguments([
      '--browser', 'firefox', '--clean-runs', '1', '--timeout-ms', '90000',
    ])).toEqual({
      browser: 'firefox',
      cleanRuns: 1,
      engineTimeoutMs: 90_000,
      help: false,
    })

    for (const arguments_ of [
      [],
      ['--browser', 'chrome'],
      ['--browser', 'chromium', '--browser', 'webkit'],
      ['--browser', 'chromium', '--clean-runs', '0'],
      ['--browser', 'chromium', '--clean-runs', '3'],
      ['--browser', 'chromium', '--timeout-ms', '999'],
      ['--browser', 'chromium', '--unknown', 'value'],
    ]) {
      expect(() => parseBrowserQualificationArguments(arguments_)).toThrowError(
        expect.objectContaining({ code: 'E_BROWSER_QUALIFICATION_USAGE' }),
      )
    }
  })

  it('allows only exact loopback HTTP, same-origin blob/data, and exact loopback WS', async () => {
    const origin = 'http://127.0.0.1:43123'
    for (const allowed of [
      `${origin}/entry`,
      `blob:${origin}/worker-id`,
      'data:text/plain,qualification-local',
    ]) expect(isQualificationNetworkUrlAllowed(allowed, origin)).toBe(true)
    for (const blocked of [
      'https://127.0.0.1:43123/entry',
      'http://localhost:43123/entry',
      'http://127.0.0.1:43124/entry',
      'blob:http://127.0.0.1:43124/worker-id',
      'file:///tmp/qualification',
      'javascript:alert(1)',
      'ws://127.0.0.1:43123/socket',
    ]) expect(isQualificationNetworkUrlAllowed(blocked, origin)).toBe(false)
    expect(isQualificationWebSocketUrlAllowed('ws://127.0.0.1:43123/hmr', origin)).toBe(true)
    expect(isQualificationWebSocketUrlAllowed('wss://127.0.0.1:43123/hmr', origin)).toBe(false)
    expect(isQualificationWebSocketUrlAllowed('ws://localhost:43123/hmr', origin)).toBe(false)

    let requestHandler: ((route: any) => Promise<void>) | undefined
    let webSocketHandler: ((route: any) => Promise<void>) | undefined
    const context = {
      route: vi.fn(async (_pattern, handler) => { requestHandler = handler }),
      routeWebSocket: vi.fn(async (_pattern, handler) => { webSocketHandler = handler }),
    }
    const policy = await installQualificationNetworkIsolation(context, origin)
    const abort = vi.fn(async () => {})
    await requestHandler?.({
      request: () => ({ url: () => 'https://example.com/exfiltrate' }),
      continue: vi.fn(async () => {}),
      abort,
    })
    const close = vi.fn(async () => {})
    await webSocketHandler?.({
      url: () => 'wss://example.com/socket',
      connectToServer: vi.fn(),
      close,
    })
    expect(abort).toHaveBeenCalledWith('blockedbyclient')
    expect(close).toHaveBeenCalledWith({ code: 1008, reason: 'qualification-network-policy' })
    expect(policy.violations).toEqual([
      { channel: 'request', url: 'https://example.com/exfiltrate' },
      { channel: 'websocket', url: 'wss://example.com/socket' },
    ])
  })

  it('quarantines and cleans a resource that settles after acquisition timeout', async () => {
    let resolveAcquisition!: (value: { close: () => Promise<void> }) => void
    const acquisition = new Promise<{ close: () => Promise<void> }>((resolvePromise) => {
      resolveAcquisition = resolvePromise
    })
    const timeout = new BrowserQualificationRunnerError(
      'E_BROWSER_ENGINE_TIMEOUT',
      'browser-launch',
      'fixture timeout',
      { timeoutMs: 10 },
    )
    const fakeDeadline = { run: async () => { throw timeout } }
    const quarantine = new BrowserLateSettlementQuarantine()
    const close = vi.fn(async () => {})
    await expect(acquireBrowserResource(
      fakeDeadline,
      'browser-launch',
      () => acquisition,
      quarantine,
      (resource) => resource.close(),
    )).rejects.toMatchObject({
      code: 'E_BROWSER_ENGINE_TIMEOUT',
      details: {
        lateSettlementCleanupArmed: true,
        hardKillJoinClaim: false,
      },
    })
    resolveAcquisition({ close })
    await expect(quarantine.drain(50)).resolves.toEqual([])
    expect(close).toHaveBeenCalledOnce()
  })

  it('bounds retained browser and Vite diagnostics by entry and byte budgets', () => {
    const logs = new BoundedQualificationLog(2, 5_000)
    logs.add('info', 'page-console', 'x'.repeat(20_000), { value: 'y'.repeat(20_000) })
    logs.add('warn', 'vite', 'second')
    logs.add('error', 'page-error', 'must be dropped')

    const snapshot = logs.snapshot()
    expect(snapshot.entries).toHaveLength(2)
    expect(snapshot.droppedEntries).toBe(1)
    expect(snapshot.retainedBytes).toBeLessThanOrEqual(5_000)
    expect(Buffer.byteLength(JSON.stringify(snapshot.entries), 'utf8')).toBeLessThan(5_100)
  })

  it('fails preflight with typed dependency, version, and managed-binary errors', async () => {
    await expect(preflightPlaywright('chromium', {
      loadQualificationPackage: async () => { throw new Error('missing') },
    })).rejects.toMatchObject({
      name: 'BrowserQualificationRunnerError',
      code: 'E_PLAYWRIGHT_PREFLIGHT_MISSING',
      phase: 'preflight',
      details: {
        requiredDependency: REQUIRED_PLAYWRIGHT_DEPENDENCY,
        qualificationPackage: 'tools/browser-qualification',
        installCommand: 'npm ci --prefix tools/browser-qualification --ignore-scripts --include=dev --registry=https://registry.npmjs.org/ --audit=false --fund=false',
      },
    })

    await expect(preflightPlaywright('chromium', {
      loadQualificationPackage: async () => fakeQualificationPackage({
        chromium: fakeBrowserType(),
      }, '1.61.0'),
    })).rejects.toMatchObject({
      code: 'E_PLAYWRIGHT_PREFLIGHT_VERSION',
      details: {
        requiredDependency: 'playwright@1.62.1',
        installedVersion: '1.61.0',
      },
    })

    await expect(preflightPlaywright('chromium', {
      loadQualificationPackage: async () => fakeQualificationPackage({
        chromium: fakeBrowserType(),
      }),
      accessExecutable: async () => { throw new Error('ENOENT') },
    })).rejects.toMatchObject({
      code: 'E_PLAYWRIGHT_BROWSER_BINARY_MISSING',
      phase: 'preflight',
      details: {
        browser: 'chromium',
        executablePath: '/managed/browser',
        provisionCommand: 'tools/browser-qualification/node_modules/.bin/playwright install chromium',
        provisionEnvironment: 'ubuntu-linux-posix',
      },
    })
  })

  it('binds the requested engine to the exact pinned Playwright package and executable digest', async () => {
    const browserType = fakeBrowserType('webkit')
    const loadedPackage = fakeQualificationPackage({ webkit: browserType })
    const preflight = await preflightPlaywright('webkit', {
      loadQualificationPackage: async () => loadedPackage,
      accessExecutable: async () => {},
      hashExecutable: async () => 'a'.repeat(64),
    })

    expect(preflight.browserType).toBe(browserType)
    expect(preflight.metadata).toEqual({
      version: '1.62.1',
      requiredDependency: 'playwright@1.62.1',
      browser: 'webkit',
      managedBinary: true,
      executablePath: '/managed/browser',
      executableSha256: 'a'.repeat(64),
      qualificationPackage: loadedPackage.packageMetadata,
    })
  })

  it('binds Playwright to the isolated qualification-only manifest and lock', async () => {
    await expect(inspectQualificationPlaywrightPackage()).resolves.toEqual({
      packageRoot: 'tools/browser-qualification',
      packageJsonPath: 'tools/browser-qualification/package.json',
      packageLockPath: 'tools/browser-qualification/package-lock.json',
      packageJsonSha256: '639d9e0c47a413b3d87e86fc0ef6e6aae546034810d475c5f58ff9ce8835b0e4',
      packageLockSha256: '0a2e7f09703398ff090367d55e1f39b88b363e5267126edf699a2e78fedc471f',
      licenseManifestPath: 'docs/qualification/playwright-license-manifest-v1.json',
      licenseManifestSha256: 'ca6b12d581fdb7added7c5b7e144a8b5b20ae946417ec6e663a8bda348df1b6a',
      playwrightVersion: '1.62.1',
    })
    await expect(assertIsolatedPlaywrightResolution(resolve(
      repositoryRoot,
      'node_modules/playwright/index.js',
    ))).rejects.toMatchObject({
      code: 'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE',
    })
    await expect(assertIsolatedPackageResolution('playwright-core', resolve(
      repositoryRoot,
      'node_modules/playwright-core/index.js',
    ))).rejects.toMatchObject({
      code: 'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE',
    })
  })

  it('rejects a dependency-root symlink that escapes the isolated package', async () => {
    const lexicalPackageRoot = resolve(repositoryRoot, 'tools/browser-qualification')
    const lexicalNodeModulesRoot = join(lexicalPackageRoot, 'node_modules')
    const lexicalDependencyRoot = join(lexicalNodeModulesRoot, 'playwright')
    const lexicalEntry = join(lexicalDependencyRoot, 'index.js')
    const canonicalPackageRoot = resolve(repositoryRoot, 'canonical/browser-qualification')
    const escapedDependencyRoot = resolve(repositoryRoot, 'escaped/playwright')
    const canonical = async (path: string) => {
      const normalized = resolve(path)
      if (normalized === lexicalPackageRoot) return canonicalPackageRoot
      if (normalized === lexicalNodeModulesRoot) return join(canonicalPackageRoot, 'node_modules')
      if (normalized === lexicalDependencyRoot) return escapedDependencyRoot
      if (normalized === lexicalEntry) return join(escapedDependencyRoot, 'index.js')
      throw new Error(`Unexpected canonicalization path: ${path}`)
    }
    await expect(assertIsolatedPackageResolution('playwright', lexicalEntry, {
      realpath: canonical,
    })).rejects.toMatchObject({
      code: 'E_QUALIFICATION_PLAYWRIGHT_RESOLUTION_ESCAPE',
      details: { packageName: 'playwright' },
    })
  })

  it('requires the exact frozen check names, order, uniqueness, and pass state', () => {
    const exactChecks = EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES.map((name) => ({
      name,
      passed: true,
    }))
    expect(validateQualificationPayload('passed', {
      status: 'passed',
      checks: exactChecks,
    })).toMatchObject({ status: 'passed', checks: exactChecks })

    const missing = exactChecks.slice(0, -1)
    const duplicate = exactChecks.map((check) => ({ ...check }))
    duplicate[duplicate.length - 1] = { ...duplicate[0] }
    const reordered = exactChecks.map((check) => ({ ...check }))
    ;[reordered[0], reordered[1]] = [reordered[1], reordered[0]]

    for (const checks of [missing, duplicate, reordered]) {
      expect(() => validateQualificationPayload('passed', {
        status: 'passed',
        checks,
      })).toThrowError(expect.objectContaining({
        code: 'E_BROWSER_RESULT_COVERAGE',
        phase: 'result',
      }))
    }
    const failed = exactChecks.map((check) => ({ ...check }))
    failed[5].passed = false
    expect(() => validateQualificationPayload('passed', {
      status: 'passed',
      checks: failed,
    })).toThrowError(expect.objectContaining({ code: 'E_BROWSER_QUALIFICATION_FAILED' }))

    expect(() => validateQualificationPayload('passed', {
      status: 'passed',
      checks: exactChecks,
      unexpected: true,
    })).toThrowError(expect.objectContaining({ code: 'E_BROWSER_RESULT_INVALID' }))
    expect(() => validateQualificationPayload('passed', {
      status: 'passed',
      checks: exactChecks.map((check, index) => (
        index === 0 ? { ...check, unexpected: true } : check
      )),
    })).toThrowError(expect.objectContaining({ code: 'E_BROWSER_RESULT_INVALID' }))
  })

  it('caps aggregate Vite output bytes independently of the file-count cap', () => {
    expect(validateBuildArtifactByteBudget([
      { path: 'a.js', bytes: 4 * 1024 * 1024 },
      { path: 'b.wasm', bytes: 4 * 1024 * 1024 },
    ])).toBe(8 * 1024 * 1024)
    expect(() => validateBuildArtifactByteBudget([
      { path: 'a.js', bytes: 4 * 1024 * 1024 },
      { path: 'b.wasm', bytes: 4 * 1024 * 1024 + 1 },
    ])).toThrowError(expect.objectContaining({
      code: 'E_BROWSER_BUILD_ARTIFACT_BYTES_LIMIT',
      details: { maximumBytes: 8 * 1024 * 1024, observedBytes: 8 * 1024 * 1024 + 1 },
    }))
  })

  it('builds only the isolated Vite qualification entry and emits module Worker assets', async () => {
    const evidence = await buildQualificationBundle()

    expect(evidence.configPath).toBe('vite.qualification.config.ts')
    expect(evidence.outputPath).toBe('tmp/browser-qualification-dist')
    expect(evidence.entryPath).toBe('tests/fixtures/browser-qualification.html')
    expect(evidence.totalBytes).toBe(
      evidence.artifacts.reduce((sum: number, artifact: { bytes: number }) => sum + artifact.bytes, 0),
    )
    expect(evidence.artifacts.find((artifact: { path: string }) => (
      artifact.path === evidence.entryPath
    ))).toMatchObject({ bytes: expect.any(Number), sha256: expect.stringMatching(/^[a-f0-9]{64}$/) })
    expect(evidence.workerArtifacts.length).toBeGreaterThan(0)
    expect(evidence.workerArtifacts.every((path: string) => path.startsWith('assets/'))).toBe(true)
  })

  it('configures every preview as a fresh loopback-only ephemeral Vite server', async () => {
    let capturedConfig: Record<string, unknown> | undefined
    const close = vi.fn(async () => {})
    const fakeVite = {
      createLogger: () => ({
        info: vi.fn(), warn: vi.fn(), warnOnce: vi.fn(), error: vi.fn(), clearScreen: vi.fn(),
      }),
      preview: vi.fn(async (config: Record<string, unknown>) => {
        capturedConfig = config
        return {
          httpServer: {
            address: () => ({ address: '127.0.0.1', family: 'IPv4', port: 43123 }),
          },
          close,
        }
      }),
    }

    const preview = await startQualificationPreview({ vite: fakeVite })
    expect(preview.origin).toBe('http://127.0.0.1:43123')
    expect(preview.entryUrl).toBe(
      'http://127.0.0.1:43123/tests/fixtures/browser-qualification.html',
    )
    expect(capturedConfig).toMatchObject({
      configFile: resolve(repositoryRoot, 'vite.qualification.config.ts'),
      logLevel: 'silent',
      build: { outDir: resolve(repositoryRoot, 'tmp/browser-qualification-dist') },
      preview: { host: '127.0.0.1', port: 0, strictPort: true, open: false },
    })
    await preview.server.close()
    expect(close).toHaveBeenCalledOnce()
  })

  it('enforces one total engine deadline before any browser can launch', async () => {
    const evidence = await runBrowserQualification({
      browser: 'chromium',
      cleanRuns: 1,
      engineTimeoutMs: 10,
    }, {
      preflightPlaywright: async () => await new Promise(() => {}),
    })

    expect(evidence).toMatchObject({
      qualificationOnly: true,
      status: 'failed',
      browser: 'chromium',
      build: null,
      runs: [],
      error: {
        code: 'E_BROWSER_ENGINE_TIMEOUT',
        phase: 'playwright-preflight',
      },
    })
  })

  it('uses one fresh preview/context/page/browser fragment and closes every resource', async () => {
    const actualVite = await import('vite')
    const pageClose = vi.fn(async () => {})
    const contextClose = vi.fn(async () => {})
    const browserClose = vi.fn(async () => {})
    const previewClose = vi.fn(async () => {})
    const locatorSelectors: string[] = []
    let previewNumber = 0

    function createPage() {
      const listeners = new Map<string, Array<(value: unknown) => void>>()
      const page = {
        on: vi.fn((event: string, listener: (value: unknown) => void) => {
          const eventListeners = listeners.get(event) ?? []
          eventListeners.push(listener)
          listeners.set(event, eventListeners)
        }),
        locator: vi.fn((selector: string) => {
          locatorSelectors.push(selector)
          return {
            waitFor: vi.fn(async () => {}),
            count: vi.fn(async () => 1),
            evaluate: vi.fn(async () => ({
              status: 'passed',
              byteLength: 79,
              text: JSON.stringify({
                status: 'passed',
                checks: EXPECTED_BROWSER_QUALIFICATION_CHECK_NAMES.map((name) => ({
                  name,
                  passed: true,
                })),
              }),
            })),
          }
        }),
        waitForFunction: vi.fn(async () => {}),
        goto: vi.fn(async (entryUrl: string) => {
          const origin = new URL(entryUrl).origin
          const workerName = readdirSync(resolve(
            repositoryRoot,
            'tmp/browser-qualification-dist/assets',
          )).find((name) => name.includes('.worker-') && name.endsWith('.js'))
          if (workerName === undefined) throw new Error('missing built Worker fixture')
          const workerUrl = origin + '/assets/' + workerName
          for (const listener of listeners.get('worker') ?? []) listener({ url: () => workerUrl })
          for (const listener of listeners.get('response') ?? []) {
            listener({
              url: () => workerUrl,
              status: () => 200,
              headers: () => ({ 'content-type': 'text/javascript' }),
              request: () => ({ resourceType: () => 'script' }),
            })
          }
          return {
            ok: () => true,
            status: () => 200,
            headers: () => ({ 'content-type': 'text/html; charset=utf-8' }),
          }
        }),
        close: pageClose,
      }
      return page
    }

    const context = {
      route: vi.fn(async () => {}),
      routeWebSocket: vi.fn(async () => {}),
      newPage: vi.fn(async () => createPage()),
      close: contextClose,
    }
    const browser = {
      browserType: () => ({ name: () => 'chromium' }),
      version: () => 'fake-browser-version',
      newContext: vi.fn(async () => context),
      close: browserClose,
    }
    const browserType = {
      launch: vi.fn(async () => browser),
    }
    const vite = {
      ...actualVite,
      preview: vi.fn(async () => {
        previewNumber += 1
        return {
          httpServer: {
            address: () => ({ address: '127.0.0.1', family: 'IPv4', port: 45_000 + previewNumber }),
          },
          close: previewClose,
        }
      }),
    }

    const evidence = await runBrowserQualification({
      browser: 'chromium',
      cleanRuns: 1,
      engineTimeoutMs: 30_000,
    }, {
      vite,
      preflightPlaywright: async () => ({
        browserType,
        metadata: {
          version: '1.62.1',
          requiredDependency: 'playwright@1.62.1',
          browser: 'chromium',
          managedBinary: true,
          executablePath: '/managed/browser',
          executableSha256: 'a'.repeat(64),
        },
      }),
    })

    expect(evidence.status).toBe('passed')
    expect(evidence).toMatchObject({
      qualificationOnly: true,
      qualificationClaim: 'none',
      cleanRunFragment: 1,
    })
    expect(evidence.runs).toHaveLength(1)
    expect(evidence.runs.every((run: { status: string }) => run.status === 'passed')).toBe(true)
    expect(vite.preview).toHaveBeenCalledOnce()
    expect(browser.newContext).toHaveBeenCalledOnce()
    expect(browser.newContext).toHaveBeenCalledWith({ serviceWorkers: 'block' })
    expect(context.route).toHaveBeenCalledOnce()
    expect(context.routeWebSocket).toHaveBeenCalledOnce()
    expect(context.newPage).toHaveBeenCalledOnce()
    expect(locatorSelectors).toEqual([
      '[data-testid="qualification-result"]',
    ])
    expect(pageClose).toHaveBeenCalledOnce()
    expect(contextClose).toHaveBeenCalledOnce()
    expect(previewClose).toHaveBeenCalledOnce()
    expect(browserClose).toHaveBeenCalledOnce()
  })

  it('emits a single bounded JSON failure record for invalid CLI input', () => {
    const child = spawnSync(process.execPath, [runnerPath, '--browser', 'chrome'], {
      cwd: repositoryRoot,
      encoding: 'utf8',
      timeout: 5_000,
    })

    expect(child.status).toBe(1)
    expect(child.stdout.trim().split('\n')).toHaveLength(1)
    expect(JSON.parse(child.stdout)).toMatchObject({
      schemaVersion: 'browser-qualification-evidence-v1',
      qualificationOnly: true,
      status: 'failed',
      error: { code: 'E_BROWSER_QUALIFICATION_USAGE', phase: 'arguments' },
    })
    expect(Buffer.byteLength(child.stdout, 'utf8')).toBeLessThan(16_384)
    expect(child.stderr.trim().split('\n')).toHaveLength(1)
    expect(JSON.parse(child.stderr)).toMatchObject({ event: 'browser-qualification-failed' })
  })

  it('uses a dedicated typed runner error contract', () => {
    const error = new BrowserQualificationRunnerError('E_TEST', 'test', 'failure')
    expect(error).toMatchObject({
      name: 'BrowserQualificationRunnerError',
      code: 'E_TEST',
      phase: 'test',
      message: 'failure',
    })
  })
})
