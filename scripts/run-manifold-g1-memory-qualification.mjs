#!/usr/bin/env node

import { createReadStream, constants as fsConstants } from 'node:fs'
import { access, mkdtemp, readFile, readdir, rm } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import {
  QUALIFICATION_PLAYWRIGHT_PACKAGE,
  loadQualificationPlaywrightPackage,
} from './qualificationPlaywrightPackage.mjs'

export const BROWSER_MEMORY_CONTRACT = Object.freeze({
  surface: 'browser',
  browsers: Object.freeze(['chromium', 'firefox', 'webkit']),
  warmupJobs: 50,
  measuredJobs: 500,
  sampleEveryJobs: 25,
  cleanRunsRequired: 3,
  slopeRssBytesPerJobMax: 128 * 1024,
  endpointDriftRssBytesMax: 64 * 1024 * 1024,
  playwrightVersion: QUALIFICATION_PLAYWRIGHT_PACKAGE.playwrightVersion,
  jobTimeoutMs: 15_000,
  pageErrorLimit: 32,
  forcedBrowserGc: false,
  samplingPoint: 'after Worker terminate request, terminal settlement, two requestAnimationFrame callbacks, and an event-loop drain',
})

const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const qualificationPagePath = '/tests/fixtures/browser-memory-qualification.html'
const RESOURCE_ACQUISITION_TIMEOUT_MS = 30_000
const CLEANUP_TIMEOUT_MS = 5_000
const LATE_SETTLEMENT_DRAIN_TIMEOUT_MS = 5_000
const MAX_NETWORK_URL_CHARACTERS = 2_048

export class BrowserMemoryQualificationError extends Error {
  constructor(code, message, details = {}) {
    super(message)
    this.name = 'BrowserMemoryQualificationError'
    this.code = code
    this.details = Object.freeze({ ...details })
  }
}

function exactLoopbackOrigin(origin) {
  try {
    const parsed = new URL(origin)
    return parsed.origin === origin
      && parsed.protocol === 'http:'
      && parsed.hostname === '127.0.0.1'
      && parsed.port.length > 0
      && parsed.username.length === 0
      && parsed.password.length === 0
  } catch {
    return false
  }
}

export function isBrowserMemoryNetworkUrlAllowed(rawUrl, exactOrigin) {
  if (!exactLoopbackOrigin(exactOrigin)) return false
  try {
    const parsed = new URL(rawUrl)
    if (parsed.protocol === 'http:') {
      return parsed.origin === exactOrigin
        && parsed.username.length === 0
        && parsed.password.length === 0
    }
    if (parsed.protocol === 'blob:') return parsed.origin === exactOrigin
    return parsed.protocol === 'data:'
  } catch {
    return false
  }
}

export function isBrowserMemoryWebSocketUrlAllowed(rawUrl, exactOrigin) {
  if (!exactLoopbackOrigin(exactOrigin)) return false
  try {
    const expected = new URL(exactOrigin)
    const parsed = new URL(rawUrl)
    return parsed.protocol === 'ws:'
      && parsed.hostname === expected.hostname
      && parsed.port === expected.port
      && parsed.username.length === 0
      && parsed.password.length === 0
  } catch {
    return false
  }
}

function argumentError(message) {
  return new BrowserMemoryQualificationError('E_ARGUMENT', message)
}

function consumeValue(argv, index, flag) {
  const value = argv[index + 1]
  if (value === undefined || value.startsWith('--')) throw argumentError(`${flag} requires a value`)
  return value
}

function exactPositiveInteger(value, flag) {
  if (!/^[1-9][0-9]*$/.test(value)) throw argumentError(`${flag} must be a positive integer`)
  const parsed = Number(value)
  if (!Number.isSafeInteger(parsed)) throw argumentError(`${flag} is outside the safe integer range`)
  return parsed
}

export function parseBrowserMemoryArguments(argv) {
  const seen = new Set()
  let surface
  let browser
  let warmupJobs = BROWSER_MEMORY_CONTRACT.warmupJobs
  let measuredJobs = BROWSER_MEMORY_CONTRACT.measuredJobs
  let sampleEveryJobs = BROWSER_MEMORY_CONTRACT.sampleEveryJobs
  let runIndex = null

  for (let index = 0; index < argv.length; index++) {
    const flag = argv[index]
    if (!['--surface', '--browser', '--warmup', '--cycles', '--sample-every', '--run-index'].includes(flag)) {
      throw argumentError(`Unknown argument: ${flag}`)
    }
    if (seen.has(flag)) throw argumentError(`Duplicate argument: ${flag}`)
    seen.add(flag)
    const value = consumeValue(argv, index, flag)
    index++
    if (flag === '--surface') surface = value
    else if (flag === '--browser') browser = value
    else if (flag === '--warmup') warmupJobs = exactPositiveInteger(value, flag)
    else if (flag === '--cycles') measuredJobs = exactPositiveInteger(value, flag)
    else if (flag === '--sample-every') sampleEveryJobs = exactPositiveInteger(value, flag)
    else runIndex = exactPositiveInteger(value, flag)
  }

  if (surface !== BROWSER_MEMORY_CONTRACT.surface) {
    throw argumentError('--surface must be browser')
  }
  if (!BROWSER_MEMORY_CONTRACT.browsers.includes(browser)) {
    throw argumentError('--browser must be exactly one of chromium, firefox, or webkit')
  }
  if (warmupJobs !== BROWSER_MEMORY_CONTRACT.warmupJobs
    || measuredJobs !== BROWSER_MEMORY_CONTRACT.measuredJobs
    || sampleEveryJobs !== BROWSER_MEMORY_CONTRACT.sampleEveryJobs) {
    throw argumentError('Browser memory parameters are frozen at --warmup 50 --cycles 500 --sample-every 25')
  }
  if (runIndex === null || runIndex > BROWSER_MEMORY_CONTRACT.cleanRunsRequired) {
    throw argumentError('--run-index must be between 1 and 3')
  }

  return Object.freeze({
    surface,
    browser,
    warmupJobs,
    measuredJobs,
    sampleEveryJobs,
    runIndex,
  })
}

export async function loadPlaywright(packageLoader = loadQualificationPlaywrightPackage) {
  let loadedPackage
  try {
    loadedPackage = await packageLoader()
  } catch (cause) {
    throw new BrowserMemoryQualificationError(
      'E_PLAYWRIGHT_UNAVAILABLE',
      'The isolated exact Playwright package is required for browser memory qualification',
      {
        qualificationPackage: QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot,
        installCommand: QUALIFICATION_PLAYWRIGHT_PACKAGE.installCommand,
        causeCode: typeof cause?.code === 'string' ? cause.code : null,
        cause: cause instanceof Error ? cause.message : String(cause),
      },
    )
  }
  const playwright = loadedPackage.playwright
  if (playwright === null || typeof playwright !== 'object') {
    throw new BrowserMemoryQualificationError(
      'E_PLAYWRIGHT_UNAVAILABLE',
      'The Playwright module did not expose browser types',
    )
  }
  const manifest = loadedPackage.manifest
  if (manifest?.name !== 'playwright' || manifest?.version !== BROWSER_MEMORY_CONTRACT.playwrightVersion) {
    throw new BrowserMemoryQualificationError(
      'E_PLAYWRIGHT_VERSION',
      `Browser memory qualification requires Playwright ${BROWSER_MEMORY_CONTRACT.playwrightVersion}`,
      { observedName: manifest?.name ?? null, observedVersion: manifest?.version ?? null },
    )
  }
  for (const browser of BROWSER_MEMORY_CONTRACT.browsers) {
    if (typeof playwright[browser]?.launchPersistentContext !== 'function'
      || typeof playwright[browser]?.executablePath !== 'function') {
      throw new BrowserMemoryQualificationError(
        'E_PLAYWRIGHT_UNAVAILABLE',
        `The Playwright module does not expose ${browser}`,
      )
    }
  }
  const packageMetadata = loadedPackage.packageMetadata
  if (packageMetadata === null || typeof packageMetadata !== 'object'
      || packageMetadata.packageRoot !== QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot
      || packageMetadata.packageJsonPath !== `${QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot}/package.json`
      || packageMetadata.packageLockPath !== `${QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot}/package-lock.json`
      || packageMetadata.licenseManifestPath !== QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestPath
      || packageMetadata.playwrightVersion !== BROWSER_MEMORY_CONTRACT.playwrightVersion
      || packageMetadata.packageJsonSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256
      || packageMetadata.packageLockSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256
      || packageMetadata.licenseManifestSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256
      || !/^[a-f0-9]{64}$/.test(packageMetadata.packageJsonSha256)
      || !/^[a-f0-9]{64}$/.test(packageMetadata.packageLockSha256)
      || !/^[a-f0-9]{64}$/.test(packageMetadata.licenseManifestSha256)) {
    throw new BrowserMemoryQualificationError(
      'E_PLAYWRIGHT_PACKAGE_METADATA',
      'The isolated Playwright package metadata does not match its frozen manifest and lock',
    )
  }
  return Object.freeze({ playwright, packageMetadata: Object.freeze({ ...packageMetadata }) })
}

export async function assertExecutableBrowserBinary(path) {
  if (typeof path !== 'string' || path.length === 0) {
    throw new BrowserMemoryQualificationError(
      'E_BROWSER_BINARY_UNAVAILABLE',
      'Playwright returned an empty browser executable path',
    )
  }
  try {
    await access(path, fsConstants.R_OK | fsConstants.X_OK)
  } catch (cause) {
    throw new BrowserMemoryQualificationError(
      'E_BROWSER_BINARY_UNAVAILABLE',
      `The pinned Playwright browser binary is unavailable: ${path}`,
      { path, cause: cause instanceof Error ? cause.message : String(cause) },
    )
  }
}

async function sha256File(path) {
  const hash = createHash('sha256')
  await new Promise((resolvePromise, reject) => {
    const input = createReadStream(path)
    input.on('data', chunk => hash.update(chunk))
    input.once('error', reject)
    input.once('end', resolvePromise)
  })
  return hash.digest('hex')
}

function statusInteger(text, name) {
  const match = new RegExp(`^${name}:\\s+([0-9]+)(?:\\s+kB)?$`, 'm').exec(text)
  return match === null ? null : Number(match[1])
}

export async function readLinuxProcessTable(procRoot = '/proc') {
  let entries
  try {
    entries = await readdir(procRoot, { withFileTypes: true })
  } catch (cause) {
    throw new BrowserMemoryQualificationError(
      'E_PROCESS_TREE_UNAVAILABLE',
      `Cannot read Linux process table at ${procRoot}`,
      { cause: cause instanceof Error ? cause.message : String(cause) },
    )
  }
  const table = new Map()
  await Promise.all(entries
    .filter(entry => entry.isDirectory() && /^[1-9][0-9]*$/.test(entry.name))
    .map(async entry => {
      const directory = join(procRoot, entry.name)
      try {
        const [status, rawCommand] = await Promise.all([
          readFile(join(directory, 'status'), 'utf8'),
          readFile(join(directory, 'cmdline')),
        ])
        const pid = statusInteger(status, 'Pid')
        const parentPid = statusInteger(status, 'PPid')
        const rssKiB = statusInteger(status, 'VmRSS')
        if (pid === null || parentPid === null || rssKiB === null) return
        table.set(pid, Object.freeze({
          pid,
          parentPid,
          rssBytes: rssKiB * 1024,
          command: rawCommand.toString('utf8').replaceAll('\0', ' ').trim(),
        }))
      } catch {
        // Processes can exit between readdir and readFile. A live browser root
        // is checked separately, so ignoring only this vanished record is safe.
      }
    }))
  return table
}

export function processTreePids(table, rootPid) {
  if (!(table instanceof Map) || !table.has(rootPid)) {
    throw new BrowserMemoryQualificationError(
      'E_PROCESS_TREE_LOST',
      `Browser root process ${rootPid} is absent from the process table`,
    )
  }
  const result = new Set([rootPid])
  let changed = true
  while (changed) {
    changed = false
    for (const process of table.values()) {
      if (!result.has(process.pid) && result.has(process.parentPid)) {
        result.add(process.pid)
        changed = true
      }
    }
  }
  return result
}

export function sumProcessTreeRss(table, rootPid) {
  const pids = processTreePids(table, rootPid)
  let rssBytes = 0
  for (const pid of pids) rssBytes += table.get(pid).rssBytes
  return Object.freeze({ rssBytes, processCount: pids.size })
}

export function findBrowserRootPid(table, baselinePids, ownerPid, marker) {
  const ownerTree = processTreePids(table, ownerPid)
  const newPids = new Set([...ownerTree].filter(pid => pid !== ownerPid && !baselinePids.has(pid)))
  const roots = [...newPids].filter(pid => !newPids.has(table.get(pid).parentPid))
  const matching = roots.filter(rootPid => [...processTreePids(table, rootPid)]
    .some(pid => table.get(pid).command.includes(marker)))
  if (matching.length !== 1) {
    throw new BrowserMemoryQualificationError(
      'E_BROWSER_PROCESS_AMBIGUOUS',
      `Expected one marked browser process tree, found ${matching.length}`,
      { roots, marker },
    )
  }
  return matching[0]
}

export function ordinaryLeastSquaresSlope(samples) {
  if (!Array.isArray(samples) || samples.length < 2) {
    throw new BrowserMemoryQualificationError('E_SAMPLE_SET', 'OLS requires at least two samples')
  }
  const meanJob = samples.reduce((sum, sample) => sum + sample.job, 0) / samples.length
  const meanRss = samples.reduce((sum, sample) => sum + sample.rssBytes, 0) / samples.length
  let covariance = 0
  let variance = 0
  for (const sample of samples) {
    const centeredJob = sample.job - meanJob
    covariance += centeredJob * (sample.rssBytes - meanRss)
    variance += centeredJob * centeredJob
  }
  if (variance === 0) throw new BrowserMemoryQualificationError('E_SAMPLE_SET', 'OLS job ordinals have zero variance')
  return covariance / variance
}

export function evaluateBrowserMemoryBudget(samples) {
  const slopeRssBytesPerJob = ordinaryLeastSquaresSlope(samples)
  const endpointDriftRssBytes = samples.at(-1).rssBytes - samples[0].rssBytes
  return Object.freeze({
    slopeRssBytesPerJob,
    endpointDriftRssBytes,
    slopePassed: slopeRssBytesPerJob <= BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax,
    endpointPassed: endpointDriftRssBytes <= BROWSER_MEMORY_CONTRACT.endpointDriftRssBytesMax,
    passed: slopeRssBytesPerJob <= BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax
      && endpointDriftRssBytes <= BROWSER_MEMORY_CONTRACT.endpointDriftRssBytesMax,
  })
}

export function withBrowserJobTimeout(
  operation,
  timeoutMs = BROWSER_MEMORY_CONTRACT.jobTimeoutMs,
  quarantine = undefined,
) {
  if (typeof operation !== 'function') throw new TypeError('Browser job operation must be a function')
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1) throw new RangeError('Browser job timeout must be positive')
  return new Promise((resolvePromise, reject) => {
    let settled = false
    const operationPromise = Promise.resolve().then(operation)
    const timer = setTimeout(() => {
      if (settled) return
      settled = true
      quarantine?.trackPending('browser-worker-job', operationPromise)
      reject(new BrowserMemoryQualificationError(
        'E_BROWSER_JOB_TIMEOUT',
        `Browser memory Worker job did not settle within ${timeoutMs} ms`,
        {
          timeoutMs,
          quarantined: quarantine !== undefined,
          lateSettlementCleanupRequired: true,
          hardKillJoinClaim: false,
          externalSupervisorRequiredForHardKillAndJoin: true,
        },
      ))
    }, timeoutMs)
    operationPromise.then(value => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        resolvePromise(value)
      }, error => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        reject(error)
      })
  })
}

export class BrowserMemoryLateSettlementQuarantine {
  constructor() {
    this.pending = []
    this.labels = []
  }

  trackPending(label, operation) {
    this.labels.push(label)
    this.pending.push(Promise.resolve(operation).then(() => undefined, () => undefined))
  }

  trackAcquisition(label, acquisition, cleanupLate) {
    this.labels.push(label)
    this.pending.push(Promise.resolve(acquisition).then(
      (resource) => cleanupAction({
        label: 'late ' + label,
        code: 'E_LATE_RESOURCE_CLEANUP',
        run: () => cleanupLate(resource),
      }, CLEANUP_TIMEOUT_MS).then((error) => {
        if (error !== undefined) throw error
      }),
      () => undefined,
    ))
  }

  async drain(timeoutMs = LATE_SETTLEMENT_DRAIN_TIMEOUT_MS) {
    if (this.pending.length === 0) return []
    let timer
    try {
      const settled = await Promise.race([
        Promise.allSettled(this.pending),
        new Promise((resolvePromise) => {
          timer = setTimeout(() => resolvePromise(null), timeoutMs)
        }),
      ])
      if (settled === null) {
        return [new BrowserMemoryQualificationError(
          'E_LATE_SETTLEMENT_UNJOINED',
          'A timed-out browser operation did not settle within quarantine drain',
          {
            operations: [...this.labels],
            quarantineDrainDeadlineMs: timeoutMs,
            quarantined: true,
            hardKillJoinClaim: false,
            externalSupervisorRequiredForHardKillAndJoin: true,
          },
        )]
      }
      return settled
        .filter((result) => result.status === 'rejected')
        .map((result) => result.reason)
    } finally {
      if (timer !== undefined) clearTimeout(timer)
    }
  }
}

export async function acquireBrowserMemoryResource(
  operation,
  {
    label,
    timeoutMs = RESOURCE_ACQUISITION_TIMEOUT_MS,
    quarantine,
    cleanupLate,
  },
) {
  if (!(quarantine instanceof BrowserMemoryLateSettlementQuarantine)
      || typeof cleanupLate !== 'function') {
    throw new TypeError('Timed browser resource acquisition requires quarantine and late cleanup')
  }
  const acquisition = Promise.resolve().then(operation)
  let timer
  try {
    return await Promise.race([
      acquisition,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new BrowserMemoryQualificationError(
          'E_RESOURCE_ACQUISITION_TIMEOUT',
          `${label} acquisition exceeded ${timeoutMs} ms`,
          {
            resource: label,
            timeoutMs,
            quarantined: true,
            lateSettlementCleanupArmed: true,
            hardKillJoinClaim: false,
            externalSupervisorRequiredForHardKillAndJoin: true,
          },
        )), timeoutMs)
      }),
    ])
  } catch (error) {
    if (error instanceof BrowserMemoryQualificationError
        && error.code === 'E_RESOURCE_ACQUISITION_TIMEOUT') {
      quarantine.trackAcquisition(label, acquisition, cleanupLate)
    }
    throw error
  } finally {
    if (timer !== undefined) clearTimeout(timer)
  }
}

async function withBrowserMemoryOperationTimeout(operation, label, timeoutMs, quarantine) {
  const pending = Promise.resolve().then(operation)
  let timer
  try {
    return await Promise.race([
      pending,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new BrowserMemoryQualificationError(
          'E_OPERATION_TIMEOUT',
          `${label} did not settle within ${timeoutMs} ms`,
          {
            operation: label,
            timeoutMs,
            quarantined: true,
            hardKillJoinClaim: false,
            externalSupervisorRequiredForHardKillAndJoin: true,
          },
        )), timeoutMs)
      }),
    ])
  } catch (error) {
    if (error instanceof BrowserMemoryQualificationError && error.code === 'E_OPERATION_TIMEOUT') {
      quarantine.trackPending(label, pending)
    }
    throw error
  } finally {
    if (timer !== undefined) clearTimeout(timer)
  }
}

function cleanupFailure(action, cause, timeoutMs) {
  return new BrowserMemoryQualificationError(
    action.code,
    `${action.label} cleanup failed`,
    {
      cause: cause instanceof Error ? cause.message : String(cause),
      cleanupDeadlineMs: timeoutMs,
      quarantined: true,
      hardKillJoinClaim: false,
      externalSupervisorRequiredForHardKillAndJoin: true,
    },
  )
}

async function cleanupAction(action, timeoutMs) {
  let timer
  try {
    await Promise.race([
      Promise.resolve().then(action.run),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(
          `${action.label} cleanup exceeded ${timeoutMs} ms`,
        )), timeoutMs)
      }),
    ])
    return undefined
  } catch (cause) {
    return cleanupFailure(action, cause, timeoutMs)
  } finally {
    if (timer !== undefined) clearTimeout(timer)
  }
}

export async function finalizeQualificationCleanup(primaryError, actions, options = {}) {
  const timeoutMs = options.timeoutMs ?? CLEANUP_TIMEOUT_MS
  const cleanupErrors = []
  for (const action of actions) {
    const error = await cleanupAction(action, timeoutMs)
    if (error !== undefined) cleanupErrors.push(error)
  }
  if (options.quarantine !== undefined) {
    cleanupErrors.push(...await options.quarantine.drain(
      options.quarantineDrainTimeoutMs ?? LATE_SETTLEMENT_DRAIN_TIMEOUT_MS,
    ))
  }
  if (primaryError !== null && cleanupErrors.length === 0) throw primaryError
  if (primaryError === null && cleanupErrors.length === 1) throw cleanupErrors[0]
  if (primaryError !== null || cleanupErrors.length > 0) {
    throw new AggregateError(
      primaryError === null ? cleanupErrors : [primaryError, ...cleanupErrors],
      primaryError === null
        ? 'Browser memory qualification cleanup failed'
        : 'Browser memory qualification failed and cleanup also failed',
    )
  }
}

export async function collectBrowserMemorySamples(config, runJob, readSample) {
  for (let job = 1; job <= config.warmupJobs; job++) await runJob({ phase: 'warmup', job })
  const samples = []
  for (let job = 1; job <= config.measuredJobs; job++) {
    await runJob({ phase: 'measured', job })
    if (job % config.sampleEveryJobs !== 0) continue
    const sample = await readSample(job)
    if (!Number.isSafeInteger(sample.rssBytes) || sample.rssBytes < 0
      || !Number.isSafeInteger(sample.processCount) || sample.processCount < 1) {
      throw new BrowserMemoryQualificationError('E_SAMPLE_SET', `Invalid process-tree RSS sample for job ${job}`)
    }
    samples.push(Object.freeze({
      job,
      rssBytes: sample.rssBytes,
      processCount: sample.processCount,
    }))
  }
  const expectedSamples = config.measuredJobs / config.sampleEveryJobs
  if (!Number.isInteger(expectedSamples) || samples.length !== expectedSamples) {
    throw new BrowserMemoryQualificationError(
      'E_SAMPLE_SET',
      `Expected ${expectedSamples} samples, collected ${samples.length}`,
    )
  }
  return Object.freeze(samples)
}

function immediate() {
  return new Promise(resolvePromise => setImmediate(resolvePromise))
}

async function waitForBrowserRoot(baselinePids, marker) {
  let lastError
  for (let attempt = 0; attempt < 100; attempt++) {
    const table = await readLinuxProcessTable()
    try {
      return findBrowserRootPid(table, baselinePids, process.pid, marker)
    } catch (error) {
      lastError = error
      await new Promise(resolvePromise => setTimeout(resolvePromise, 50))
    }
  }
  throw lastError ?? new BrowserMemoryQualificationError(
    'E_BROWSER_PROCESS_AMBIGUOUS',
    'Browser process tree did not become observable',
  )
}

async function startViteServer(quarantine) {
  let vite
  try {
    vite = await import('vite')
  } catch (cause) {
    throw new BrowserMemoryQualificationError(
      'E_VITE_UNAVAILABLE',
      'Vite is required for actual Worker transformation',
      { cause: cause instanceof Error ? cause.message : String(cause) },
    )
  }
  const server = await acquireBrowserMemoryResource(
    () => vite.createServer({
      root: repositoryRoot,
      configFile: false,
      appType: 'mpa',
      logLevel: 'silent',
      server: {
        host: '127.0.0.1',
        port: 0,
        strictPort: false,
      },
    }),
    {
      label: 'Vite server',
      quarantine,
      cleanupLate: (lateServer) => lateServer.close(),
    },
  )
  try {
    await withBrowserMemoryOperationTimeout(
      () => server.listen(),
      'Vite server listen',
      RESOURCE_ACQUISITION_TIMEOUT_MS,
      quarantine,
    )
    const address = server.httpServer?.address()
    if (address === null || typeof address !== 'object'
        || address.address !== '127.0.0.1' || address.port <= 0) {
      throw new BrowserMemoryQualificationError('E_VITE_STARTUP', 'Vite did not expose a TCP address')
    }
    return { server, url: `http://127.0.0.1:${address.port}${qualificationPagePath}` }
  } catch (error) {
    await finalizeQualificationCleanup(error, [{
      label: 'Vite server',
      code: 'E_VITE_CLEANUP',
      run: () => server.close(),
    }])
  }
}

export async function installBrowserMemoryNetworkIsolation(context, exactOrigin) {
  if (!exactLoopbackOrigin(exactOrigin)
      || typeof context?.route !== 'function'
      || typeof context?.routeWebSocket !== 'function') {
    throw new BrowserMemoryQualificationError(
      'E_NETWORK_ISOLATION_UNAVAILABLE',
      'Exact HTTP and WebSocket routing is required before browser memory navigation',
      { exactOrigin },
    )
  }
  const violations = []
  const recordViolation = (channel, url) => {
    if (violations.length < BROWSER_MEMORY_CONTRACT.pageErrorLimit) {
      const boundedUrl = String(url).slice(0, MAX_NETWORK_URL_CHARACTERS)
      violations.push(Object.freeze({ channel, url: boundedUrl }))
    }
  }
  await context.route('**/*', async (route) => {
    const requestUrl = route.request().url()
    if (isBrowserMemoryNetworkUrlAllowed(requestUrl, exactOrigin)) await route.continue()
    else {
      recordViolation('request', requestUrl)
      await route.abort('blockedbyclient')
    }
  })
  await context.routeWebSocket('**/*', async (webSocketRoute) => {
    const webSocketUrl = webSocketRoute.url()
    if (isBrowserMemoryWebSocketUrlAllowed(webSocketUrl, exactOrigin)) {
      webSocketRoute.connectToServer()
    } else {
      recordViolation('websocket', webSocketUrl)
      await webSocketRoute.close({ code: 1008, reason: 'qualification-network-policy' })
    }
  })
  return Object.freeze({
    exactOrigin,
    serviceWorkers: 'block',
    allowedRequestTransports: Object.freeze([
      'exact-http-loopback-origin',
      'same-origin-blob-browser-local',
      'data-browser-local',
    ]),
    allowedWebSocketTransport: 'exact-ws-loopback-origin',
    violations,
  })
}

export function buildBrowserMemoryRecord({
  config,
  qualificationPackage,
  executablePath,
  executableSha256,
  browserRootPid,
  networkIsolation,
  samples,
  startedAt,
  finishedAt,
}) {
  if (!Number.isSafeInteger(config?.runIndex)
      || config.runIndex < 1 || config.runIndex > BROWSER_MEMORY_CONTRACT.cleanRunsRequired) {
    throw argumentError('--run-index must be between 1 and 3')
  }
  if (qualificationPackage === null || typeof qualificationPackage !== 'object'
      || qualificationPackage.packageRoot !== QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot
      || qualificationPackage.packageJsonPath !== `${QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot}/package.json`
      || qualificationPackage.packageLockPath !== `${QUALIFICATION_PLAYWRIGHT_PACKAGE.relativeRoot}/package-lock.json`
      || qualificationPackage.licenseManifestPath !== QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestPath
      || qualificationPackage.packageJsonSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageJsonSha256
      || qualificationPackage.packageLockSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.packageLockSha256
      || qualificationPackage.licenseManifestSha256 !== QUALIFICATION_PLAYWRIGHT_PACKAGE.licenseManifestSha256) {
    throw new BrowserMemoryQualificationError(
      'E_PLAYWRIGHT_PACKAGE_METADATA',
      'Browser memory evidence requires the frozen qualification package SHA metadata',
    )
  }
  if (networkIsolation === null || typeof networkIsolation !== 'object'
      || !exactLoopbackOrigin(networkIsolation.exactOrigin)
      || networkIsolation.serviceWorkers !== 'block'
      || !Array.isArray(networkIsolation.violations)
      || networkIsolation.violations.length !== 0) {
    throw new BrowserMemoryQualificationError(
      'E_NETWORK_POLICY_VIOLATION',
      'Browser memory evidence requires a zero-violation exact-loopback network policy',
    )
  }
  const budget = evaluateBrowserMemoryBudget(samples)
  return Object.freeze({
    schema: 'manifold-g1-browser-memory-probe',
    version: 1,
    status: budget.passed ? 'single-clean-run-passed' : 'single-clean-run-failed',
    qualificationClaim: 'none',
    browser: config.browser,
    environment: {
      platform: process.platform,
      architecture: process.arch,
      nodeVersion: process.version,
      playwrightVersion: BROWSER_MEMORY_CONTRACT.playwrightVersion,
      qualificationPackage: {
        packageRoot: qualificationPackage.packageRoot,
        packageJsonPath: qualificationPackage.packageJsonPath,
        packageLockPath: qualificationPackage.packageLockPath,
        packageJsonSha256: qualificationPackage.packageJsonSha256,
        packageLockSha256: qualificationPackage.packageLockSha256,
        licenseManifestPath: qualificationPackage.licenseManifestPath,
        licenseManifestSha256: qualificationPackage.licenseManifestSha256,
      },
      browserExecutablePath: executablePath,
      browserExecutableSha256: executableSha256,
      browserRootPid,
    },
    orchestration: {
      cleanRunsRequired: BROWSER_MEMORY_CONTRACT.cleanRunsRequired,
      cleanRunsRepresentedByThisInvocation: 1,
      runIndex: config.runIndex,
      aggregateQualificationClaimAllowed: false,
      hardKillJoinClaim: false,
      externalSupervisorRequiredForHardKillAndJoin: true,
    },
    work: {
      warmupJobs: config.warmupJobs,
      measuredJobs: config.measuredJobs,
      sampleEveryJobs: config.sampleEveryJobs,
      sampleCount: samples.length,
      jobTimeoutMs: BROWSER_MEMORY_CONTRACT.jobTimeoutMs,
    },
    sampling: {
      forcedBrowserGc: BROWSER_MEMORY_CONTRACT.forcedBrowserGc,
      point: BROWSER_MEMORY_CONTRACT.samplingPoint,
      rssScope: 'browser-root-plus-all-live-descendants',
      slopeMethod: 'ordinary-least-squares using measured job ordinal as x',
      pageErrorLimit: BROWSER_MEMORY_CONTRACT.pageErrorLimit,
    },
    containment: {
      networkIsolation: {
        exactOrigin: networkIsolation.exactOrigin,
        serviceWorkers: networkIsolation.serviceWorkers,
        allowedRequestTransports: networkIsolation.allowedRequestTransports,
        allowedWebSocketTransport: networkIsolation.allowedWebSocketTransport,
        violationCount: networkIsolation.violations.length,
      },
      resourceAcquisitionDeadlineMs: RESOURCE_ACQUISITION_TIMEOUT_MS,
      cleanupDeadlineMs: CLEANUP_TIMEOUT_MS,
      lateSettlementDrainDeadlineMs: LATE_SETTLEMENT_DRAIN_TIMEOUT_MS,
    },
    budget: {
      slopeRssBytesPerJobMax: BROWSER_MEMORY_CONTRACT.slopeRssBytesPerJobMax,
      endpointDriftRssBytesMax: BROWSER_MEMORY_CONTRACT.endpointDriftRssBytesMax,
    },
    observed: budget,
    samples,
    startedAt,
    finishedAt,
  })
}

export async function runActualBrowserMemoryQualification(config) {
  const loadedPlaywright = await loadPlaywright()
  const playwright = loadedPlaywright.playwright
  const browserType = playwright[config.browser]
  let executablePath
  try {
    executablePath = browserType.executablePath()
  } catch (cause) {
    throw new BrowserMemoryQualificationError(
      'E_BROWSER_BINARY_UNAVAILABLE',
      `Playwright could not resolve the pinned ${config.browser} executable`,
      { cause: cause instanceof Error ? cause.message : String(cause) },
    )
  }
  await assertExecutableBrowserBinary(executablePath)
  if (process.platform !== 'linux') {
    throw new BrowserMemoryQualificationError(
      'E_PROCESS_TREE_UNAVAILABLE',
      'Frozen browser memory evidence requires Linux /proc process-tree RSS',
      { platform: process.platform },
    )
  }
  const executableSha256 = await sha256File(executablePath)
  await readLinuxProcessTable()

  const startedAt = new Date().toISOString()
  const userDataDirectory = await mkdtemp(join(tmpdir(), `manifold-g1-${config.browser}-`))
  const quarantine = new BrowserMemoryLateSettlementQuarantine()
  let viteServer
  let context
  let record
  let primaryError = null
  try {
    const vite = await startViteServer(quarantine)
    viteServer = vite.server
    const exactOrigin = new URL(vite.url).origin
    const baselineTable = await readLinuxProcessTable()
    const baselinePids = processTreePids(baselineTable, process.pid)
    try {
      context = await acquireBrowserMemoryResource(
        () => browserType.launchPersistentContext(userDataDirectory, {
          headless: true,
          serviceWorkers: 'block',
          timeout: RESOURCE_ACQUISITION_TIMEOUT_MS,
        }),
        {
          label: 'Playwright persistent context',
          quarantine,
          cleanupLate: (lateContext) => lateContext.close(),
        },
      )
    } catch (cause) {
      if (cause instanceof BrowserMemoryQualificationError) throw cause
      throw new BrowserMemoryQualificationError(
        'E_BROWSER_LAUNCH',
        `Failed to launch the pinned ${config.browser} binary`,
        { cause: cause instanceof Error ? cause.message : String(cause) },
      )
    }
    const networkIsolation = await withBrowserMemoryOperationTimeout(
      () => installBrowserMemoryNetworkIsolation(context, exactOrigin),
      'browser network-isolation installation',
      RESOURCE_ACQUISITION_TIMEOUT_MS,
      quarantine,
    )
    const browserRootPid = await waitForBrowserRoot(baselinePids, userDataDirectory)
    const page = context.pages()[0] ?? await acquireBrowserMemoryResource(
      () => context.newPage(),
      {
        label: 'Playwright page',
        quarantine,
        cleanupLate: (latePage) => latePage.close(),
      },
    )
    const pageErrors = []
    let pageErrorOverflow = 0
    page.on('pageerror', error => {
      if (pageErrors.length < BROWSER_MEMORY_CONTRACT.pageErrorLimit) {
        pageErrors.push(String(error.message).slice(0, MAX_NETWORK_URL_CHARACTERS))
      }
      else pageErrorOverflow++
    })
    await page.goto(vite.url, { waitUntil: 'networkidle', timeout: 30_000 })
    await page.waitForFunction(() => (
      document.querySelector('[data-testid="memory-qualification-status"]')?.getAttribute('data-status') === 'ready'
    ), undefined, { timeout: 30_000 })
    if (networkIsolation.violations.length > 0) {
      throw new BrowserMemoryQualificationError(
        'E_NETWORK_POLICY_VIOLATION',
        'Browser memory page attempted a forbidden network transport',
        { violations: [...networkIsolation.violations] },
      )
    }

    const runJob = async () => {
      const result = await withBrowserJobTimeout(
        () => page.evaluate(async () => {
          const controller = globalThis.__manifoldPlanMemoryQualification
          if (controller === undefined) throw new Error('Browser memory qualification controller is unavailable')
          return controller.runJob()
        }),
        BROWSER_MEMORY_CONTRACT.jobTimeoutMs,
        quarantine,
      )
      if (result.workersStarted !== result.workersTerminated) {
        throw new BrowserMemoryQualificationError(
          'E_WORKER_LIFECYCLE',
          `Worker epoch ${result.workerEpoch} was not terminated before settlement`,
        )
      }
      if (pageErrors.length > 0) {
        throw new BrowserMemoryQualificationError(
          'E_BROWSER_PAGE',
          'Browser page reported an uncaught error',
          { pageErrors: [...pageErrors], pageErrorOverflow },
        )
      }
      if (networkIsolation.violations.length > 0) {
        throw new BrowserMemoryQualificationError(
          'E_NETWORK_POLICY_VIOLATION',
          'Browser memory job attempted a forbidden network transport',
          { violations: [...networkIsolation.violations] },
        )
      }
    }
    const readSample = async job => {
      await immediate()
      const table = await readLinuxProcessTable()
      const sample = sumProcessTreeRss(table, browserRootPid)
      return { job, ...sample }
    }
    const samples = await collectBrowserMemorySamples(config, runJob, readSample)
    if (networkIsolation.violations.length > 0) {
      throw new BrowserMemoryQualificationError(
        'E_NETWORK_POLICY_VIOLATION',
        'Browser memory run attempted a forbidden network transport',
        { violations: [...networkIsolation.violations] },
      )
    }
    record = buildBrowserMemoryRecord({
      config,
      qualificationPackage: loadedPlaywright.packageMetadata,
      executablePath,
      executableSha256,
      browserRootPid,
      networkIsolation,
      samples,
      startedAt,
      finishedAt: new Date().toISOString(),
    })
  } catch (error) {
    primaryError = error
  }
  await finalizeQualificationCleanup(primaryError, [
    ...(context === undefined ? [] : [{
      label: 'Playwright context',
      code: 'E_BROWSER_CONTEXT_CLEANUP',
      run: () => context.close(),
    }]),
    ...(viteServer === undefined ? [] : [{
      label: 'Vite server',
      code: 'E_VITE_CLEANUP',
      run: () => viteServer.close(),
    }]),
    {
      label: 'temporary browser profile',
      code: 'E_TEMP_CLEANUP',
      run: () => rm(userDataDirectory, { recursive: true, force: true }),
    },
  ], { quarantine })
  if (record === undefined) {
    throw new BrowserMemoryQualificationError('E_UNEXPECTED', 'Browser memory qualification produced no record')
  }
  return record
}

function serializedError(error) {
  if (error instanceof AggregateError) {
    return {
      name: error.name,
      code: 'E_AGGREGATE',
      message: error.message,
      details: { errors: [...error.errors].map(serializedError) },
    }
  }
  if (error instanceof BrowserMemoryQualificationError) {
    return {
      name: error.name,
      code: error.code,
      message: error.message,
      details: error.details,
    }
  }
  return {
    name: error instanceof Error ? error.name : 'UnknownError',
    code: 'E_UNEXPECTED',
    message: error instanceof Error ? error.message : String(error),
    details: {},
  }
}

async function main() {
  try {
    const config = parseBrowserMemoryArguments(process.argv.slice(2))
    const record = await runActualBrowserMemoryQualification(config)
    process.stdout.write(`${JSON.stringify(record)}\n`)
    if (record.status !== 'single-clean-run-passed') process.exitCode = 1
  } catch (error) {
    process.stderr.write(`${JSON.stringify({
      schema: 'manifold-g1-browser-memory-probe-error',
      version: 1,
      status: 'failed-closed',
      qualificationClaim: 'none',
      error: serializedError(error),
    })}\n`)
    process.exitCode = 2
  }
}

const invokedPath = process.argv[1] === undefined ? null : pathToFileURL(resolve(process.argv[1])).href
if (invokedPath === import.meta.url) await main()
