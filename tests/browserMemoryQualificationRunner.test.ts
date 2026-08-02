import { describe, expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  BROWSER_MEMORY_CONTRACT,
  BrowserMemoryLateSettlementQuarantine,
  BrowserMemoryQualificationError,
  acquireBrowserMemoryResource,
  assertExecutableBrowserBinary,
  buildBrowserMemoryRecord,
  collectBrowserMemorySamples,
  evaluateBrowserMemoryBudget,
  finalizeQualificationCleanup,
  findBrowserRootPid,
  installBrowserMemoryNetworkIsolation,
  isBrowserMemoryNetworkUrlAllowed,
  isBrowserMemoryWebSocketUrlAllowed,
  loadPlaywright,
  ordinaryLeastSquaresSlope,
  parseBrowserMemoryArguments,
  sumProcessTreeRss,
  withBrowserJobTimeout,
} from '../scripts/run-manifold-g1-memory-qualification.mjs'
import {
  QUALIFICATION_PLAYWRIGHT_PACKAGE,
  inspectQualificationPlaywrightPackage,
} from '../scripts/qualificationPlaywrightPackage.mjs'

function processRecord(
  pid: number,
  parentPid: number,
  rssBytes: number,
  command: string,
) {
  return Object.freeze({ pid, parentPid, rssBytes, command })
}

describe('G1 actual-browser memory qualification runner', () => {
  it('matches the frozen plan row without closing actual-run evidence', () => {
    const plan = JSON.parse(readFileSync(resolve(
      import.meta.dirname,
      '../docs/qualification/semantic-manifold-g1-plan-v3.json',
    ), 'utf8')) as {
      matrix: Array<{ id: string; harness: { command: string }; evidenceState: string }>
      resourceBudgets: {
        memoryExperiments: Array<{
          id: string
          warmupJobs: number
          measuredJobsPerEnvironment: number
          sampleEveryJobs: number
          cleanRuns: number
          gcPassesPerSample: number
          slopeBytesPerJobMax: { rss: number }
          endpointDriftBytesMax: { rss: number }
        }>
      }
      unresolvedRows: Array<{ id: string }>
    }
    const row = plan.matrix.find(item => item.id === 'browser-memory-slope')
    const budget = plan.resourceBudgets.memoryExperiments
      .find(item => item.id === 'browser-worker-cycle')
    expect(row).toMatchObject({
      evidenceState: 'not-executed-clean-post-freeze',
      harness: {
        command: 'node scripts/run-manifold-g1-memory-qualification.mjs --surface browser --browser <chromium|firefox|webkit> --cycles 500 --warmup 50 --sample-every 25 --run-index <1|2|3>',
      },
    })
    expect(budget).toMatchObject({
      warmupJobs: BROWSER_MEMORY_CONTRACT.warmupJobs,
      measuredJobsPerEnvironment: BROWSER_MEMORY_CONTRACT.measuredJobs,
      sampleEveryJobs: BROWSER_MEMORY_CONTRACT.sampleEveryJobs,
      cleanRuns: BROWSER_MEMORY_CONTRACT.cleanRunsRequired,
      gcPassesPerSample: 0,
      slopeBytesPerJobMax: { rss: BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax },
      endpointDriftBytesMax: { rss: BROWSER_MEMORY_CONTRACT.endpointDriftRssBytesMax },
    })
    expect(plan.unresolvedRows.map(item => item.id)).not.toContain('u06-browser-memory-probe')
  })

  it('accepts only the frozen one-engine browser contract', () => {
    expect(parseBrowserMemoryArguments([
      '--surface', 'browser',
      '--browser', 'chromium',
      '--warmup', '50',
      '--cycles', '500',
      '--sample-every', '25',
      '--run-index', '2',
    ])).toEqual({
      surface: 'browser',
      browser: 'chromium',
      warmupJobs: 50,
      measuredJobs: 500,
      sampleEveryJobs: 25,
      runIndex: 2,
    })
    expect(() => parseBrowserMemoryArguments([
      '--surface', 'browser', '--browser', 'all',
    ])).toThrowError(expect.objectContaining({ code: 'E_ARGUMENT' }))
    expect(() => parseBrowserMemoryArguments([
      '--surface', 'browser', '--browser', 'firefox', '--cycles', '499',
    ])).toThrow('parameters are frozen')
    expect(() => parseBrowserMemoryArguments([
      '--surface', 'browser', '--browser', 'webkit', '--run-index', '4',
    ])).toThrow('--run-index must be between 1 and 3')
    expect(() => parseBrowserMemoryArguments([
      '--surface', 'browser', '--browser', 'webkit',
    ])).toThrow('--run-index must be between 1 and 3')
  })

  it('fails closed on every non-local network transport and blocks service-worker bypass', async () => {
    const origin = 'http://127.0.0.1:41731'
    expect(isBrowserMemoryNetworkUrlAllowed(`${origin}/fixture.ts`, origin)).toBe(true)
    expect(isBrowserMemoryNetworkUrlAllowed(`blob:${origin}/worker`, origin)).toBe(true)
    expect(isBrowserMemoryNetworkUrlAllowed('data:text/plain,local', origin)).toBe(true)
    for (const blocked of [
      'https://127.0.0.1:41731/fixture.ts',
      'http://localhost:41731/fixture.ts',
      'http://127.0.0.1:41732/fixture.ts',
      'file:///tmp/fixture.ts',
      'about:blank',
      'wss://127.0.0.1:41731/hmr',
    ]) expect(isBrowserMemoryNetworkUrlAllowed(blocked, origin)).toBe(false)
    expect(isBrowserMemoryWebSocketUrlAllowed('ws://127.0.0.1:41731/hmr', origin)).toBe(true)
    expect(isBrowserMemoryWebSocketUrlAllowed('wss://127.0.0.1:41731/hmr', origin)).toBe(false)

    let requestHandler: ((route: any) => Promise<void>) | undefined
    let webSocketHandler: ((route: any) => Promise<void>) | undefined
    const context = {
      route: async (_pattern: string, handler: (route: any) => Promise<void>) => { requestHandler = handler },
      routeWebSocket: async (_pattern: string, handler: (route: any) => Promise<void>) => {
        webSocketHandler = handler
      },
    }
    const policy = await installBrowserMemoryNetworkIsolation(context, origin)
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
    expect(policy).toMatchObject({ serviceWorkers: 'block' })
    expect(policy.violations).toHaveLength(2)
  })

  it('fails closed when Playwright or the selected browser binary is missing', async () => {
    await expect(loadPlaywright(async () => {
      throw new Error('fixture package missing')
    })).rejects.toMatchObject({
      name: 'BrowserMemoryQualificationError',
      code: 'E_PLAYWRIGHT_UNAVAILABLE',
      details: {
        qualificationPackage: 'tools/browser-qualification',
        installCommand: 'npm ci --prefix tools/browser-qualification --ignore-scripts --include=dev --registry=https://registry.npmjs.org/ --audit=false --fund=false',
      },
    })
    await expect(assertExecutableBrowserBinary(
      '/definitely-not-installed/manifold-g1-browser',
    )).rejects.toMatchObject({
      name: 'BrowserMemoryQualificationError',
      code: 'E_BROWSER_BINARY_UNAVAILABLE',
    })
  })

  it('requires the exact frozen Playwright package version', async () => {
    const browserType = {
      launchPersistentContext: async () => {},
      executablePath: () => '/fixture/browser',
    }
    const playwright = {
      chromium: browserType,
      firefox: browserType,
      webkit: browserType,
    }
    const packageLoader = (version: string) => async () => ({
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
    })
    await expect(loadPlaywright(packageLoader('1.62.0'))).rejects.toMatchObject({
      code: 'E_PLAYWRIGHT_VERSION',
      details: { observedVersion: '1.62.0' },
    })
    await expect(loadPlaywright(packageLoader('1.62.1')))
      .resolves.toMatchObject({ playwright: { chromium: browserType } })
    await expect(loadPlaywright(async () => ({
      playwright,
      manifest: { name: 'playwright', version: '1.62.1' },
      packageMetadata: {},
    }))).rejects.toMatchObject({ code: 'E_PLAYWRIGHT_PACKAGE_METADATA' })
  })

  it('uses the immutable isolated qualification package instead of the product graph', async () => {
    expect(QUALIFICATION_PLAYWRIGHT_PACKAGE).toMatchObject({
      packageManager: 'npm@10.9.8',
      nodeEngine: '>=20.19.0',
      playwrightVersion: '1.62.1',
      relativeRoot: 'tools/browser-qualification',
      installCommand: 'npm ci --prefix tools/browser-qualification --ignore-scripts --include=dev --registry=https://registry.npmjs.org/ --audit=false --fund=false',
      browserProvisionEnvironment: 'ubuntu-linux-posix',
      packageJsonSha256: '639d9e0c47a413b3d87e86fc0ef6e6aae546034810d475c5f58ff9ce8835b0e4',
      packageLockSha256: '0a2e7f09703398ff090367d55e1f39b88b363e5267126edf699a2e78fedc471f',
      licenseManifestPath: 'docs/qualification/playwright-license-manifest-v1.json',
      licenseManifestSha256: 'ca6b12d581fdb7added7c5b7e144a8b5b20ae946417ec6e663a8bda348df1b6a',
    })
    await expect(inspectQualificationPlaywrightPackage()).resolves.toMatchObject({
      packageRoot: 'tools/browser-qualification',
      playwrightVersion: '1.62.1',
    })
  })

  it('bounds a wedged page job with a typed outer timeout', async () => {
    const quarantine = new BrowserMemoryLateSettlementQuarantine()
    await expect(withBrowserJobTimeout(
      () => new Promise(() => {}),
      10,
      quarantine,
    )).rejects.toMatchObject({
      name: 'BrowserMemoryQualificationError',
      code: 'E_BROWSER_JOB_TIMEOUT',
      details: { timeoutMs: 10, quarantined: true, hardKillJoinClaim: false },
    })
    await expect(quarantine.drain(10)).resolves.toEqual([
      expect.objectContaining({
        code: 'E_LATE_SETTLEMENT_UNJOINED',
        details: expect.objectContaining({ externalSupervisorRequiredForHardKillAndJoin: true }),
      }),
    ])
  })

  it('cleans late resource acquisitions and reports unjoined acquisitions without a kill claim', async () => {
    let resolveAcquisition!: (value: { close: () => Promise<void> }) => void
    const acquisition = new Promise<{ close: () => Promise<void> }>((resolvePromise) => {
      resolveAcquisition = resolvePromise
    })
    const quarantine = new BrowserMemoryLateSettlementQuarantine()
    const close = vi.fn(async () => {})
    await expect(acquireBrowserMemoryResource(
      () => acquisition,
      {
        label: 'fixture context',
        timeoutMs: 10,
        quarantine,
        cleanupLate: (resource) => resource.close(),
      },
    )).rejects.toMatchObject({
      code: 'E_RESOURCE_ACQUISITION_TIMEOUT',
      details: { lateSettlementCleanupArmed: true, hardKillJoinClaim: false },
    })
    resolveAcquisition({ close })
    await expect(quarantine.drain(50)).resolves.toEqual([])
    expect(close).toHaveBeenCalledOnce()
  })

  it('fails closed on cleanup and preserves primary then cleanup error order', async () => {
    const primary = new BrowserMemoryQualificationError('E_PRIMARY', 'primary failure')
    const contextFailure = new Error('context close failed')
    const serverFailure = new Error('server close failed')
    let aggregate: unknown
    try {
      await finalizeQualificationCleanup(primary, [
        {
          label: 'Playwright context',
          code: 'E_BROWSER_CONTEXT_CLEANUP',
          run: async () => { throw contextFailure },
        },
        {
          label: 'Vite server',
          code: 'E_VITE_CLEANUP',
          run: async () => { throw serverFailure },
        },
      ])
    } catch (error) {
      aggregate = error
    }
    expect(aggregate).toBeInstanceOf(AggregateError)
    expect((aggregate as AggregateError).errors).toEqual([
      primary,
      expect.objectContaining({ code: 'E_BROWSER_CONTEXT_CLEANUP' }),
      expect.objectContaining({ code: 'E_VITE_CLEANUP' }),
    ])

    await expect(finalizeQualificationCleanup(null, [{
      label: 'Playwright context',
      code: 'E_BROWSER_CONTEXT_CLEANUP',
      run: async () => { throw contextFailure },
    }])).rejects.toMatchObject({
      code: 'E_BROWSER_CONTEXT_CLEANUP',
      details: { cause: 'context close failed' },
    })
    await expect(finalizeQualificationCleanup(null, [
      { label: 'context', code: 'E_CONTEXT', run: async () => {} },
      { label: 'server', code: 'E_SERVER', run: async () => {} },
    ])).resolves.toBeUndefined()

    await expect(finalizeQualificationCleanup(null, [{
      label: 'wedged context',
      code: 'E_CONTEXT_CLEANUP',
      run: async () => await new Promise(() => {}),
    }], { timeoutMs: 10 })).rejects.toMatchObject({
      code: 'E_CONTEXT_CLEANUP',
      details: {
        cleanupDeadlineMs: 10,
        hardKillJoinClaim: false,
        externalSupervisorRequiredForHardKillAndJoin: true,
      },
    })
  })

  it('sums the full marked browser process tree and excludes siblings', () => {
    const marker = '/tmp/manifold-g1-chromium-profile'
    const table = new Map([
      [1, processRecord(1, 0, 1_000, 'node qualification-runner')],
      [2, processRecord(2, 1, 2_000, 'pre-existing helper')],
      [10, processRecord(10, 1, 10_000, `chromium --user-data-dir=${marker}`)],
      [11, processRecord(11, 10, 11_000, 'chromium renderer')],
      [12, processRecord(12, 11, 12_000, 'chromium worker utility')],
      [20, processRecord(20, 1, 20_000, 'unrelated new process')],
      [21, processRecord(21, 20, 21_000, 'unrelated descendant')],
    ])
    const browserRoot = findBrowserRootPid(table, new Set([1, 2]), 1, marker)
    expect(browserRoot).toBe(10)
    expect(sumProcessTreeRss(table, browserRoot)).toEqual({
      rssBytes: 33_000,
      processCount: 3,
    })
  })

  it('runs 50 warmups and 500 measured jobs but samples only after every 25th settlement', async () => {
    const calls: Array<{ phase: string; job: number }> = []
    const samples = await collectBrowserMemorySamples({
      warmupJobs: 50,
      measuredJobs: 500,
      sampleEveryJobs: 25,
    }, async call => {
      calls.push(call)
    }, async job => ({
      rssBytes: 100_000_000 + job * 1_000,
      processCount: 7,
    }))
    expect(calls).toHaveLength(550)
    expect(calls.slice(0, 50)).toEqual(
      Array.from({ length: 50 }, (_, index) => ({ phase: 'warmup', job: index + 1 })),
    )
    expect(samples).toHaveLength(20)
    expect(samples.map(sample => sample.job)).toEqual(
      Array.from({ length: 20 }, (_, index) => (index + 1) * 25),
    )
    expect(ordinaryLeastSquaresSlope(samples)).toBeCloseTo(1_000, 8)
  })

  it('applies 128 KiB/job OLS and 64 MiB endpoint budgets independently', () => {
    const exactSlope = Array.from({ length: 20 }, (_, index) => ({
      job: (index + 1) * 25,
      rssBytes: 200_000_000 + (index + 1) * 25 * BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax,
      processCount: 4,
    }))
    expect(evaluateBrowserMemoryBudget(exactSlope)).toMatchObject({
      slopeRssBytesPerJob: BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax,
      slopePassed: true,
      endpointPassed: true,
      passed: true,
    })

    const slopeFailure = exactSlope.map((sample, index) => ({
      ...sample,
      rssBytes: 200_000_000 + (index + 1) * 25
        * (BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax + 1),
    }))
    expect(evaluateBrowserMemoryBudget(slopeFailure)).toMatchObject({
      slopePassed: false,
      passed: false,
    })

    const endpointFailure = exactSlope.map((sample, index) => ({
      ...sample,
      rssBytes: index === exactSlope.length - 1
        ? exactSlope[0].rssBytes + BROWSER_MEMORY_CONTRACT.endpointDriftRssBytesMax + 1
        : exactSlope[0].rssBytes,
    }))
    expect(evaluateBrowserMemoryBudget(endpointFailure)).toMatchObject({
      endpointPassed: false,
      passed: false,
    })
  })

  it('emits only a single-run fragment and never claims three clean runs or qualification', () => {
    const config = parseBrowserMemoryArguments([
      '--surface', 'browser', '--browser', 'webkit', '--run-index', '1',
    ])
    const samples = [
      { job: 25, rssBytes: 100_000_000, processCount: 3 },
      { job: 50, rssBytes: 100_001_000, processCount: 3 },
    ]
    const record = buildBrowserMemoryRecord({
      config,
      qualificationPackage: {
        packageRoot: 'tools/browser-qualification',
        packageJsonPath: 'tools/browser-qualification/package.json',
        packageLockPath: 'tools/browser-qualification/package-lock.json',
        packageJsonSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256,
        packageLockSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256,
        licenseManifestPath: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestPath,
        licenseManifestSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256,
      },
      executablePath: '/frozen/webkit',
      executableSha256: 'a'.repeat(64),
      browserRootPid: 42,
      networkIsolation: {
        exactOrigin: 'http://127.0.0.1:41731',
        serviceWorkers: 'block',
        allowedRequestTransports: [
          'exact-http-loopback-origin',
          'same-origin-blob-browser-local',
          'data-browser-local',
        ],
        allowedWebSocketTransport: 'exact-ws-loopback-origin',
        violations: [],
      },
      samples,
      startedAt: '2026-08-01T00:00:00.000Z',
      finishedAt: '2026-08-01T00:01:00.000Z',
    })
    expect(record).toMatchObject({
      status: 'single-clean-run-passed',
      qualificationClaim: 'none',
      browser: 'webkit',
      environment: {
        playwrightVersion: '1.62.1',
        qualificationPackage: {
          packageJsonSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256,
          packageLockSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256,
          licenseManifestSha256: QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256,
        },
      },
      orchestration: {
        cleanRunsRequired: 3,
        cleanRunsRepresentedByThisInvocation: 1,
        runIndex: 1,
        aggregateQualificationClaimAllowed: false,
        hardKillJoinClaim: false,
        externalSupervisorRequiredForHardKillAndJoin: true,
      },
      work: {
        jobTimeoutMs: 15_000,
      },
      sampling: {
        forcedBrowserGc: false,
        rssScope: 'browser-root-plus-all-live-descendants',
        slopeMethod: 'ordinary-least-squares using measured job ordinal as x',
        pageErrorLimit: 32,
      },
      containment: {
        networkIsolation: {
          exactOrigin: 'http://127.0.0.1:41731',
          serviceWorkers: 'block',
          violationCount: 0,
        },
        cleanupDeadlineMs: 5_000,
      },
      budget: {
        slopeRssBytesPerJobMax: 128 * 1024,
        endpointDriftRssBytesMax: 64 * 1024 * 1024,
      },
    })
  })

  it('uses typed failures for invalid sample sets', async () => {
    expect(() => ordinaryLeastSquaresSlope([{ job: 25, rssBytes: 1, processCount: 1 }]))
      .toThrowError(BrowserMemoryQualificationError)
    await expect(collectBrowserMemorySamples({
      warmupJobs: 1,
      measuredJobs: 2,
      sampleEveryJobs: 1,
    }, async () => {}, async () => ({ rssBytes: -1, processCount: 0 })))
      .rejects.toMatchObject({ code: 'E_SAMPLE_SET' })
  })
})
